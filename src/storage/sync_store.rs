//! Sync storage operations for upload queue persistence and sync record tracking.
//!
//! Provides persistence for:
//! - Upload queue entries (pending uploads that survive app restart)
//! - Sync records (upload history and status tracking)
//!
//! This module handles offline scenarios by storing uploads in a persistent
//! queue that can be processed when connectivity is restored. It also provides
//! comprehensive tracking of upload status (Pending, Uploading, Completed, Failed),
//! external activity IDs, and error messages.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::storage::database::DatabaseError;

// ========== Sync Record Types ==========

/// Upload/sync status for tracking progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncRecordStatus {
    /// Upload is queued and waiting to be processed
    Pending,
    /// Upload is currently in progress
    Uploading,
    /// Upload completed successfully
    Completed,
    /// Upload failed (may be retried or permanently failed)
    Failed,
}

impl SyncRecordStatus {
    /// Convert to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncRecordStatus::Pending => "pending",
            SyncRecordStatus::Uploading => "uploading",
            SyncRecordStatus::Completed => "completed",
            SyncRecordStatus::Failed => "failed",
        }
    }

    /// Parse from database string representation.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => SyncRecordStatus::Pending,
            "uploading" => SyncRecordStatus::Uploading,
            "completed" => SyncRecordStatus::Completed,
            "failed" => SyncRecordStatus::Failed,
            _ => SyncRecordStatus::Pending, // Default to pending for unknown values
        }
    }
}

impl std::fmt::Display for SyncRecordStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Stored sync record for tracking upload history.
/// Represents a single upload attempt from a ride to an external platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSyncRecord {
    /// Unique ID for the sync record
    pub id: Uuid,
    /// Platform sync configuration ID (references platform_syncs table)
    pub platform_sync_id: Uuid,
    /// Ride being synced
    pub ride_id: Uuid,
    /// External activity ID from the platform (e.g., Strava activity ID)
    pub external_activity_id: Option<String>,
    /// External activity URL (e.g., https://strava.com/activities/123456)
    pub external_activity_url: Option<String>,
    /// Current status of the sync
    pub status: SyncRecordStatus,
    /// Error message if sync failed
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: i32,
    /// When the upload was successfully completed
    pub uploaded_at: Option<String>,
    /// When the record was created
    pub created_at: String,
}

/// Sync store for persisting upload queue and sync records.
pub struct SyncStore<'a> {
    conn: &'a Connection,
}

/// Stored upload queue entry (pending upload that survives app restart).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUploadQueueEntry {
    /// Unique ID for the queue entry
    pub id: Uuid,
    /// Ride ID being uploaded
    pub ride_id: Uuid,
    /// Target platform (e.g., "Strava", "GarminConnect")
    pub platform: String,
    /// Path to the stored FIT file
    pub fit_file_path: String,
    /// Activity name (optional)
    pub activity_name: Option<String>,
    /// Current status (pending, uploading, completed, failed)
    pub status: String,
    /// Error message if failed
    pub error_message: Option<String>,
    /// External activity ID from platform
    pub external_activity_id: Option<String>,
    /// External activity URL
    pub external_activity_url: Option<String>,
    /// Number of retry attempts
    pub retry_count: i32,
    /// Next retry time (for backoff)
    pub next_retry_at: Option<String>,
    /// When the entry was created
    pub created_at: String,
    /// When upload completed
    pub completed_at: Option<String>,
}

impl StoredUploadQueueEntry {
    /// Check if entry is ready to be processed
    pub fn is_ready_to_process(&self) -> bool {
        if self.status != "pending" {
            return false;
        }

        if let Some(next_retry) = &self.next_retry_at {
            if let Ok(retry_time) = DateTime::parse_from_rfc3339(next_retry) {
                return Utc::now() >= retry_time.with_timezone(&Utc);
            }
        }

        true
    }
}

impl<'a> SyncStore<'a> {
    /// Create a new sync store with the given connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the upload queue table if it doesn't exist.
    /// This extends the base schema with additional columns for queue management.
    pub fn init_upload_queue_table(&self) -> Result<(), DatabaseError> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS upload_queue (
                    id TEXT PRIMARY KEY,
                    ride_id TEXT NOT NULL,
                    platform TEXT NOT NULL,
                    fit_file_path TEXT NOT NULL,
                    activity_name TEXT,
                    status TEXT NOT NULL DEFAULT 'pending',
                    error_message TEXT,
                    external_activity_id TEXT,
                    external_activity_url TEXT,
                    retry_count INTEGER NOT NULL DEFAULT 0,
                    next_retry_at TEXT,
                    created_at TEXT NOT NULL,
                    completed_at TEXT,
                    UNIQUE(ride_id, platform)
                );

                CREATE INDEX IF NOT EXISTS idx_upload_queue_status ON upload_queue(status);
                CREATE INDEX IF NOT EXISTS idx_upload_queue_platform ON upload_queue(platform);
                CREATE INDEX IF NOT EXISTS idx_upload_queue_ride ON upload_queue(ride_id);
                CREATE INDEX IF NOT EXISTS idx_upload_queue_next_retry ON upload_queue(next_retry_at);
                "#,
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Upload Queue Operations ==========

    /// Add an entry to the upload queue.
    pub fn add_to_queue(&self, entry: &StoredUploadQueueEntry) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO upload_queue (id, ride_id, platform, fit_file_path, activity_name,
                    status, error_message, external_activity_id, external_activity_url,
                    retry_count, next_retry_at, created_at, completed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(ride_id, platform) DO UPDATE SET
                    fit_file_path = excluded.fit_file_path,
                    activity_name = excluded.activity_name,
                    status = excluded.status,
                    error_message = excluded.error_message,
                    external_activity_id = excluded.external_activity_id,
                    external_activity_url = excluded.external_activity_url,
                    retry_count = excluded.retry_count,
                    next_retry_at = excluded.next_retry_at,
                    completed_at = excluded.completed_at
                "#,
                params![
                    entry.id.to_string(),
                    entry.ride_id.to_string(),
                    entry.platform,
                    entry.fit_file_path,
                    entry.activity_name,
                    entry.status,
                    entry.error_message,
                    entry.external_activity_id,
                    entry.external_activity_url,
                    entry.retry_count,
                    entry.next_retry_at,
                    entry.created_at,
                    entry.completed_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get an upload queue entry by ID.
    pub fn get_queue_entry(&self, id: &Uuid) -> Result<Option<StoredUploadQueueEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, platform, fit_file_path, activity_name, status,
                        error_message, external_activity_id, external_activity_url,
                        retry_count, next_retry_at, created_at, completed_at
                 FROM upload_queue WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id.to_string()], Self::map_queue_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get all pending upload queue entries (ready to process).
    pub fn get_pending_entries(&self) -> Result<Vec<StoredUploadQueueEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, platform, fit_file_path, activity_name, status,
                        error_message, external_activity_id, external_activity_url,
                        retry_count, next_retry_at, created_at, completed_at
                 FROM upload_queue
                 WHERE status = 'pending'
                 ORDER BY created_at ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_queue_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Get pending entries for a specific platform.
    pub fn get_pending_entries_for_platform(
        &self,
        platform: &str,
    ) -> Result<Vec<StoredUploadQueueEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, platform, fit_file_path, activity_name, status,
                        error_message, external_activity_id, external_activity_url,
                        retry_count, next_retry_at, created_at, completed_at
                 FROM upload_queue
                 WHERE status = 'pending' AND platform = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![platform], Self::map_queue_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Get all entries for a specific ride.
    pub fn get_entries_for_ride(&self, ride_id: &Uuid) -> Result<Vec<StoredUploadQueueEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, platform, fit_file_path, activity_name, status,
                        error_message, external_activity_id, external_activity_url,
                        retry_count, next_retry_at, created_at, completed_at
                 FROM upload_queue
                 WHERE ride_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string()], Self::map_queue_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Update queue entry status.
    pub fn update_status(
        &self,
        id: &Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let completed_at = if status == "completed" {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        self.conn
            .execute(
                "UPDATE upload_queue SET status = ?1, error_message = ?2, completed_at = ?3 WHERE id = ?4",
                params![status, error_message, completed_at, id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Mark entry as uploading.
    pub fn mark_uploading(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.update_status(id, "uploading", None)
    }

    /// Mark entry as completed with external activity info.
    pub fn mark_completed(
        &self,
        id: &Uuid,
        external_activity_id: Option<&str>,
        external_activity_url: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let completed_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE upload_queue SET status = 'completed', external_activity_id = ?1,
                 external_activity_url = ?2, completed_at = ?3 WHERE id = ?4",
                params![external_activity_id, external_activity_url, completed_at, id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Mark entry as failed with retry info.
    pub fn mark_failed(
        &self,
        id: &Uuid,
        error_message: &str,
        next_retry_at: Option<DateTime<Utc>>,
    ) -> Result<(), DatabaseError> {
        let next_retry_str = next_retry_at.map(|t| t.to_rfc3339());
        self.conn
            .execute(
                "UPDATE upload_queue SET status = 'pending', error_message = ?1,
                 retry_count = retry_count + 1, next_retry_at = ?2 WHERE id = ?3",
                params![error_message, next_retry_str, id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Mark entry as permanently failed (no more retries).
    pub fn mark_permanently_failed(&self, id: &Uuid, error_message: &str) -> Result<(), DatabaseError> {
        self.update_status(id, "failed", Some(error_message))
    }

    /// Cancel a pending upload.
    pub fn cancel_entry(&self, id: &Uuid) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE upload_queue SET status = 'cancelled' WHERE id = ?1 AND status = 'pending'",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Delete a queue entry (and optionally clean up the FIT file).
    pub fn delete_entry(&self, id: &Uuid) -> Result<Option<String>, DatabaseError> {
        // First get the fit file path to return for cleanup
        let fit_path = self.get_queue_entry(id)?.map(|e| e.fit_file_path);

        self.conn
            .execute(
                "DELETE FROM upload_queue WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(fit_path)
    }

    /// Clean up old completed/failed entries older than the specified days.
    pub fn cleanup_old_entries(&self, days_old: i32) -> Result<usize, DatabaseError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_old as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM upload_queue WHERE status IN ('completed', 'failed', 'cancelled')
                 AND created_at < ?1",
                params![cutoff_str],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(rows_affected)
    }

    /// Get count of pending entries.
    pub fn get_pending_count(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM upload_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Get count of pending entries for a platform.
    pub fn get_pending_count_for_platform(&self, platform: &str) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM upload_queue WHERE status = 'pending' AND platform = ?1",
                params![platform],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Helper function to map a row to StoredUploadQueueEntry.
    fn map_queue_entry_row(row: &rusqlite::Row) -> rusqlite::Result<StoredUploadQueueEntry> {
        Ok(StoredUploadQueueEntry {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            ride_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
            platform: row.get(2)?,
            fit_file_path: row.get(3)?,
            activity_name: row.get(4)?,
            status: row.get(5)?,
            error_message: row.get(6)?,
            external_activity_id: row.get(7)?,
            external_activity_url: row.get(8)?,
            retry_count: row.get(9)?,
            next_retry_at: row.get(10)?,
            created_at: row.get(11)?,
            completed_at: row.get(12)?,
        })
    }

    // ========== Sync Record Operations ==========

    /// Create a new sync record for tracking an upload.
    pub fn create_sync_record(&self, record: &StoredSyncRecord) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO sync_records (id, platform_sync_id, ride_id, external_activity_id,
                    status, error_message, uploaded_at, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(platform_sync_id, ride_id) DO UPDATE SET
                    external_activity_id = excluded.external_activity_id,
                    status = excluded.status,
                    error_message = excluded.error_message,
                    uploaded_at = excluded.uploaded_at
                "#,
                params![
                    record.id.to_string(),
                    record.platform_sync_id.to_string(),
                    record.ride_id.to_string(),
                    record.external_activity_id,
                    record.status.as_str(),
                    record.error_message,
                    record.uploaded_at,
                    record.created_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get a sync record by ID.
    pub fn get_sync_record(&self, id: &Uuid) -> Result<Option<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id.to_string()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get all sync records for a specific ride.
    /// This provides the sync history showing uploads to all platforms for a ride.
    pub fn get_sync_records_by_ride(&self, ride_id: &Uuid) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records WHERE ride_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Get all sync records for a platform sync configuration.
    pub fn get_sync_records_by_platform(&self, platform_sync_id: &Uuid) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records WHERE platform_sync_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![platform_sync_id.to_string()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Get sync records by status.
    pub fn get_sync_records_by_status(&self, status: SyncRecordStatus) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records WHERE status = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![status.as_str()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Get pending sync records for a platform (ready for upload/retry).
    pub fn get_pending_sync_records(&self, platform_sync_id: &Uuid) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records
                 WHERE platform_sync_id = ?1 AND status = 'pending'
                 ORDER BY created_at ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![platform_sync_id.to_string()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Get failed sync records for a platform (candidates for retry).
    pub fn get_failed_sync_records(&self, platform_sync_id: &Uuid) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records
                 WHERE platform_sync_id = ?1 AND status = 'failed'
                 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![platform_sync_id.to_string()], Self::map_sync_record_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Update sync record status.
    pub fn update_sync_record_status(
        &self,
        id: &Uuid,
        status: SyncRecordStatus,
        error_message: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE sync_records SET status = ?1, error_message = ?2 WHERE id = ?3",
                params![status.as_str(), error_message, id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Mark sync record as uploading.
    pub fn mark_sync_uploading(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.update_sync_record_status(id, SyncRecordStatus::Uploading, None)
    }

    /// Mark sync record as completed with external activity info.
    pub fn mark_sync_completed(
        &self,
        id: &Uuid,
        external_activity_id: Option<&str>,
        external_activity_url: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let uploaded_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                r#"
                UPDATE sync_records
                SET status = 'completed',
                    external_activity_id = ?1,
                    uploaded_at = ?2,
                    error_message = NULL
                WHERE id = ?3
                "#,
                params![external_activity_id, uploaded_at, id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Note: The schema doesn't have external_activity_url column in sync_records table,
        // but we track it in the upload_queue. For sync_records, we store the activity ID
        // and clients can construct the URL from the platform + activity_id if needed.
        // If URL storage is required, a schema migration should be added.

        Ok(())
    }

    /// Mark sync record as failed with error message.
    pub fn mark_sync_failed(&self, id: &Uuid, error_message: &str) -> Result<(), DatabaseError> {
        self.update_sync_record_status(id, SyncRecordStatus::Failed, Some(error_message))
    }

    /// Delete a sync record.
    pub fn delete_sync_record(&self, id: &Uuid) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM sync_records WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Get sync record count by status.
    pub fn get_sync_record_count_by_status(&self, status: SyncRecordStatus) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sync_records WHERE status = ?1",
                params![status.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Check if a ride has already been synced to a platform.
    pub fn is_ride_synced(
        &self,
        platform_sync_id: &Uuid,
        ride_id: &Uuid,
    ) -> Result<bool, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sync_records
                 WHERE platform_sync_id = ?1 AND ride_id = ?2 AND status = 'completed'",
                params![platform_sync_id.to_string(), ride_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count > 0)
    }

    /// Get the most recent sync record for a ride-platform combination.
    pub fn get_latest_sync_record(
        &self,
        platform_sync_id: &Uuid,
        ride_id: &Uuid,
    ) -> Result<Option<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records
                 WHERE platform_sync_id = ?1 AND ride_id = ?2
                 ORDER BY created_at DESC
                 LIMIT 1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(
                params![platform_sync_id.to_string(), ride_id.to_string()],
                Self::map_sync_record_row,
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Helper function to map a row to StoredSyncRecord.
    fn map_sync_record_row(row: &rusqlite::Row) -> rusqlite::Result<StoredSyncRecord> {
        let status_str: String = row.get(4)?;
        Ok(StoredSyncRecord {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            platform_sync_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
            ride_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_default(),
            external_activity_id: row.get(3)?,
            external_activity_url: None, // Not stored in current schema
            status: SyncRecordStatus::from_str(&status_str),
            error_message: row.get(5)?,
            retry_count: 0, // Not stored in current schema, tracked via upload_queue
            uploaded_at: row.get(6)?,
            created_at: row.get(7)?,
        })
    }
}

/// Directory for storing pending upload FIT files.
pub fn get_upload_queue_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustRide")
        .join("upload_queue");

    // Create directory if it doesn't exist
    if !base.exists() {
        let _ = std::fs::create_dir_all(&base);
    }

    base
}

/// Save FIT data to the upload queue directory.
pub fn save_fit_for_queue(ride_id: &Uuid, platform: &str, fit_data: &[u8]) -> Result<PathBuf, std::io::Error> {
    let dir = get_upload_queue_dir();
    let filename = format!("{}_{}.fit", ride_id, platform.to_lowercase());
    let path = dir.join(filename);

    std::fs::write(&path, fit_data)?;
    Ok(path)
}

/// Load FIT data from a queue file.
pub fn load_fit_from_queue(path: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

/// Delete a FIT file from the queue directory.
pub fn delete_fit_from_queue(path: &str) -> Result<(), std::io::Error> {
    let path = PathBuf::from(path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let store = SyncStore::new(&conn);
        store.init_upload_queue_table().unwrap();
        conn
    }

    #[test]
    fn test_add_and_get_queue_entry() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: Some("Morning Ride".to_string()),
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();

        let loaded = store.get_queue_entry(&entry.id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.platform, "Strava");
        assert_eq!(loaded.activity_name, Some("Morning Ride".to_string()));
    }

    #[test]
    fn test_get_pending_entries() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        // Add two entries
        let entry1 = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test1.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        let mut entry2 = entry1.clone();
        entry2.id = Uuid::new_v4();
        entry2.ride_id = Uuid::new_v4();
        entry2.status = "completed".to_string();

        store.add_to_queue(&entry1).unwrap();
        store.add_to_queue(&entry2).unwrap();

        let pending = store.get_pending_entries().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, entry1.id);
    }

    #[test]
    fn test_update_status() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();
        store.mark_uploading(&entry.id).unwrap();

        let loaded = store.get_queue_entry(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.status, "uploading");
    }

    #[test]
    fn test_mark_completed() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();
        store.mark_completed(&entry.id, Some("123456"), Some("https://strava.com/activities/123456")).unwrap();

        let loaded = store.get_queue_entry(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.status, "completed");
        assert_eq!(loaded.external_activity_id, Some("123456".to_string()));
        assert!(loaded.completed_at.is_some());
    }

    #[test]
    fn test_mark_failed_with_retry() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();

        let next_retry = Utc::now() + chrono::Duration::minutes(5);
        store.mark_failed(&entry.id, "Network error", Some(next_retry)).unwrap();

        let loaded = store.get_queue_entry(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.status, "pending"); // Still pending for retry
        assert_eq!(loaded.retry_count, 1);
        assert!(loaded.next_retry_at.is_some());
        assert_eq!(loaded.error_message, Some("Network error".to_string()));
    }

    #[test]
    fn test_cancel_entry() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();
        let cancelled = store.cancel_entry(&entry.id).unwrap();
        assert!(cancelled);

        let loaded = store.get_queue_entry(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.status, "cancelled");
    }

    #[test]
    fn test_pending_count() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        assert_eq!(store.get_pending_count().unwrap(), 0);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();
        assert_eq!(store.get_pending_count().unwrap(), 1);
        assert_eq!(store.get_pending_count_for_platform("Strava").unwrap(), 1);
        assert_eq!(store.get_pending_count_for_platform("GarminConnect").unwrap(), 0);
    }

    #[test]
    fn test_is_ready_to_process() {
        // Entry without next_retry_at should be ready
        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };
        assert!(entry.is_ready_to_process());

        // Entry with future retry time should not be ready
        let mut entry_future = entry.clone();
        entry_future.next_retry_at = Some((Utc::now() + chrono::Duration::hours(1)).to_rfc3339());
        assert!(!entry_future.is_ready_to_process());

        // Entry with past retry time should be ready
        let mut entry_past = entry.clone();
        entry_past.next_retry_at = Some((Utc::now() - chrono::Duration::hours(1)).to_rfc3339());
        assert!(entry_past.is_ready_to_process());

        // Non-pending entry should not be ready
        let mut entry_completed = entry.clone();
        entry_completed.status = "completed".to_string();
        assert!(!entry_completed.is_ready_to_process());
    }

    #[test]
    fn test_get_entries_for_ride() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let ride_id = Uuid::new_v4();

        // Add entry for Strava
        let entry1 = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id,
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test1.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };
        store.add_to_queue(&entry1).unwrap();

        // Add entry for GarminConnect (different ride)
        let entry2 = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(), // Different ride
            platform: "GarminConnect".to_string(),
            fit_file_path: "/tmp/test2.fit".to_string(),
            activity_name: None,
            status: "pending".to_string(),
            error_message: None,
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 0,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };
        store.add_to_queue(&entry2).unwrap();

        let entries = store.get_entries_for_ride(&ride_id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].platform, "Strava");
    }

    // ========== Sync Record Tests ==========

    fn setup_test_db_with_sync_records() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let store = SyncStore::new(&conn);
        store.init_upload_queue_table().unwrap();

        // Create minimal schema for sync_records tests
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                ftp INTEGER NOT NULL DEFAULT 200,
                weight_kg REAL NOT NULL,
                power_zones_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rides (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                distance_meters REAL NOT NULL,
                calories INTEGER NOT NULL,
                ftp_at_ride INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS platform_syncs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 0,
                auto_upload INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(user_id, platform)
            );

            CREATE TABLE IF NOT EXISTS sync_records (
                id TEXT PRIMARY KEY,
                platform_sync_id TEXT NOT NULL,
                ride_id TEXT NOT NULL,
                external_activity_id TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                error_message TEXT,
                uploaded_at TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(platform_sync_id, ride_id)
            );

            CREATE INDEX IF NOT EXISTS idx_sync_records_platform ON sync_records(platform_sync_id);
            CREATE INDEX IF NOT EXISTS idx_sync_records_ride ON sync_records(ride_id);
            CREATE INDEX IF NOT EXISTS idx_sync_records_status ON sync_records(status);
            "#,
        ).unwrap();

        conn
    }

    fn create_test_sync_dependencies(conn: &Connection) -> (Uuid, Uuid, Uuid) {
        let user_id = Uuid::new_v4();
        let ride_id = Uuid::new_v4();
        let platform_sync_id = Uuid::new_v4();

        conn.execute(
            "INSERT INTO users (id, name, ftp, weight_kg, power_zones_json, created_at, updated_at)
             VALUES (?1, 'Test', 200, 70.0, '{}', datetime('now'), datetime('now'))",
            params![user_id.to_string()],
        ).unwrap();

        conn.execute(
            "INSERT INTO rides (id, user_id, started_at, duration_seconds, distance_meters, calories, ftp_at_ride, created_at)
             VALUES (?1, ?2, datetime('now'), 3600, 30000.0, 500, 200, datetime('now'))",
            params![ride_id.to_string(), user_id.to_string()],
        ).unwrap();

        conn.execute(
            "INSERT INTO platform_syncs (id, user_id, platform, is_enabled, auto_upload, created_at, updated_at)
             VALUES (?1, ?2, 'Strava', 1, 1, datetime('now'), datetime('now'))",
            params![platform_sync_id.to_string(), user_id.to_string()],
        ).unwrap();

        (user_id, ride_id, platform_sync_id)
    }

    #[test]
    fn test_sync_record_status_as_str() {
        assert_eq!(SyncRecordStatus::Pending.as_str(), "pending");
        assert_eq!(SyncRecordStatus::Uploading.as_str(), "uploading");
        assert_eq!(SyncRecordStatus::Completed.as_str(), "completed");
        assert_eq!(SyncRecordStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_sync_record_status_from_str() {
        assert_eq!(SyncRecordStatus::from_str("pending"), SyncRecordStatus::Pending);
        assert_eq!(SyncRecordStatus::from_str("PENDING"), SyncRecordStatus::Pending);
        assert_eq!(SyncRecordStatus::from_str("uploading"), SyncRecordStatus::Uploading);
        assert_eq!(SyncRecordStatus::from_str("completed"), SyncRecordStatus::Completed);
        assert_eq!(SyncRecordStatus::from_str("failed"), SyncRecordStatus::Failed);
        // Unknown values default to pending
        assert_eq!(SyncRecordStatus::from_str("unknown"), SyncRecordStatus::Pending);
    }

    #[test]
    fn test_create_and_get_sync_record() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        let record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record).unwrap();

        let loaded = store.get_sync_record(&record.id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.platform_sync_id, platform_sync_id);
        assert_eq!(loaded.ride_id, ride_id);
        assert_eq!(loaded.status, SyncRecordStatus::Pending);
    }

    #[test]
    fn test_get_sync_records_by_ride() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (user_id, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // Add a second platform sync
        let platform_sync_id_2 = Uuid::new_v4();
        conn.execute(
            "INSERT INTO platform_syncs (id, user_id, platform, is_enabled, auto_upload, created_at, updated_at)
             VALUES (?1, ?2, 'TrainingPeaks', 1, 1, datetime('now'), datetime('now'))",
            params![platform_sync_id_2.to_string(), user_id.to_string()],
        ).unwrap();

        // Create sync records for both platforms
        let record1 = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Completed,
            error_message: None,
            retry_count: 0,
            uploaded_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        };

        let record2 = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id: platform_sync_id_2,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record1).unwrap();
        store.create_sync_record(&record2).unwrap();

        // Get all sync records for this ride
        let records = store.get_sync_records_by_ride(&ride_id).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_get_sync_records_by_platform() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (user_id, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // Add a second ride
        let ride_id_2 = Uuid::new_v4();
        conn.execute(
            "INSERT INTO rides (id, user_id, started_at, duration_seconds, distance_meters, calories, ftp_at_ride, created_at)
             VALUES (?1, ?2, datetime('now'), 1800, 15000.0, 250, 200, datetime('now'))",
            params![ride_id_2.to_string(), user_id.to_string()],
        ).unwrap();

        let record1 = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: Some("123456".to_string()),
            external_activity_url: None,
            status: SyncRecordStatus::Completed,
            error_message: None,
            retry_count: 0,
            uploaded_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        };

        let record2 = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id: ride_id_2,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record1).unwrap();
        store.create_sync_record(&record2).unwrap();

        let records = store.get_sync_records_by_platform(&platform_sync_id).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_update_sync_record_status() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        let record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record).unwrap();

        // Update to uploading
        store.mark_sync_uploading(&record.id).unwrap();
        let loaded = store.get_sync_record(&record.id).unwrap().unwrap();
        assert_eq!(loaded.status, SyncRecordStatus::Uploading);

        // Update to completed
        store.mark_sync_completed(&record.id, Some("12345"), None).unwrap();
        let loaded = store.get_sync_record(&record.id).unwrap().unwrap();
        assert_eq!(loaded.status, SyncRecordStatus::Completed);
        assert_eq!(loaded.external_activity_id, Some("12345".to_string()));
        assert!(loaded.uploaded_at.is_some());
    }

    #[test]
    fn test_mark_sync_failed() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        let record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Uploading,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record).unwrap();
        store.mark_sync_failed(&record.id, "Network timeout").unwrap();

        let loaded = store.get_sync_record(&record.id).unwrap().unwrap();
        assert_eq!(loaded.status, SyncRecordStatus::Failed);
        assert_eq!(loaded.error_message, Some("Network timeout".to_string()));
    }

    #[test]
    fn test_get_pending_sync_records() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (user_id, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // Add a second ride
        let ride_id_2 = Uuid::new_v4();
        conn.execute(
            "INSERT INTO rides (id, user_id, started_at, duration_seconds, distance_meters, calories, ftp_at_ride, created_at)
             VALUES (?1, ?2, datetime('now'), 1800, 15000.0, 250, 200, datetime('now'))",
            params![ride_id_2.to_string(), user_id.to_string()],
        ).unwrap();

        // Create a pending and completed record
        let pending_record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        let completed_record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id: ride_id_2,
            external_activity_id: Some("123".to_string()),
            external_activity_url: None,
            status: SyncRecordStatus::Completed,
            error_message: None,
            retry_count: 0,
            uploaded_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&pending_record).unwrap();
        store.create_sync_record(&completed_record).unwrap();

        let pending = store.get_pending_sync_records(&platform_sync_id).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, pending_record.id);
    }

    #[test]
    fn test_is_ride_synced() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // Initially not synced
        assert!(!store.is_ride_synced(&platform_sync_id, &ride_id).unwrap());

        // Add a pending record - still not considered synced
        let record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };
        store.create_sync_record(&record).unwrap();
        assert!(!store.is_ride_synced(&platform_sync_id, &ride_id).unwrap());

        // Mark as completed - now synced
        store.mark_sync_completed(&record.id, Some("123"), None).unwrap();
        assert!(store.is_ride_synced(&platform_sync_id, &ride_id).unwrap());
    }

    #[test]
    fn test_delete_sync_record() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        let record = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&record).unwrap();

        // Verify it exists
        assert!(store.get_sync_record(&record.id).unwrap().is_some());

        // Delete it
        let deleted = store.delete_sync_record(&record.id).unwrap();
        assert!(deleted);

        // Verify it's gone
        assert!(store.get_sync_record(&record.id).unwrap().is_none());

        // Deleting again returns false
        let deleted_again = store.delete_sync_record(&record.id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_get_sync_record_count_by_status() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (user_id, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // Add a second ride
        let ride_id_2 = Uuid::new_v4();
        conn.execute(
            "INSERT INTO rides (id, user_id, started_at, duration_seconds, distance_meters, calories, ftp_at_ride, created_at)
             VALUES (?1, ?2, datetime('now'), 1800, 15000.0, 250, 200, datetime('now'))",
            params![ride_id_2.to_string(), user_id.to_string()],
        ).unwrap();

        // Initially no records
        assert_eq!(store.get_sync_record_count_by_status(SyncRecordStatus::Pending).unwrap(), 0);

        // Add records with different statuses
        let pending = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        let completed = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id: ride_id_2,
            external_activity_id: Some("123".to_string()),
            external_activity_url: None,
            status: SyncRecordStatus::Completed,
            error_message: None,
            retry_count: 0,
            uploaded_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        };

        store.create_sync_record(&pending).unwrap();
        store.create_sync_record(&completed).unwrap();

        assert_eq!(store.get_sync_record_count_by_status(SyncRecordStatus::Pending).unwrap(), 1);
        assert_eq!(store.get_sync_record_count_by_status(SyncRecordStatus::Completed).unwrap(), 1);
        assert_eq!(store.get_sync_record_count_by_status(SyncRecordStatus::Failed).unwrap(), 0);
    }

    #[test]
    fn test_get_latest_sync_record() {
        let conn = setup_test_db_with_sync_records();
        let store = SyncStore::new(&conn);
        let (_, ride_id, platform_sync_id) = create_test_sync_dependencies(&conn);

        // No records yet
        let latest = store.get_latest_sync_record(&platform_sync_id, &ride_id).unwrap();
        assert!(latest.is_none());

        // Add first record
        let record1 = StoredSyncRecord {
            id: Uuid::new_v4(),
            platform_sync_id,
            ride_id,
            external_activity_id: None,
            external_activity_url: None,
            status: SyncRecordStatus::Pending,
            error_message: None,
            retry_count: 0,
            uploaded_at: None,
            created_at: "2024-01-01T10:00:00Z".to_string(),
        };
        store.create_sync_record(&record1).unwrap();

        let latest = store.get_latest_sync_record(&platform_sync_id, &ride_id).unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, record1.id);
    }
}
