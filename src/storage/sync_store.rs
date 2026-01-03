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

    /// Reset a failed upload for manual retry.
    /// This clears the error state and sets status back to pending for immediate processing.
    /// Returns true if the entry was reset, false if not found or not in failed state.
    pub fn reset_for_retry(&self, id: &Uuid) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE upload_queue SET status = 'pending', error_message = NULL, next_retry_at = NULL WHERE id = ?1 AND status = 'failed'",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Get all failed upload queue entries.
    pub fn get_failed_entries(&self) -> Result<Vec<StoredUploadQueueEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ride_id, platform, fit_file_path, activity_name, status,
                        error_message, external_activity_id, external_activity_url,
                        retry_count, next_retry_at, created_at, completed_at
                 FROM upload_queue
                 WHERE status = 'failed'
                 ORDER BY created_at DESC",
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

    /// Get the retry count for a specific entry.
    pub fn get_retry_count(&self, id: &Uuid) -> Result<Option<i32>, DatabaseError> {
        match self
            .conn
            .query_row(
                "SELECT retry_count FROM upload_queue WHERE id = ?1",
                params![id.to_string()],
                |row| row.get::<_, i32>(0),
            ) {
            Ok(count) => Ok(Some(count)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Increment retry count for an entry without changing status.
    pub fn increment_retry_count(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE upload_queue SET retry_count = retry_count + 1 WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
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

// ========== Platform Sync Configuration ==========

/// Stored platform sync configuration (from platform_syncs table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlatformSync {
    /// Unique ID for the platform sync record
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Platform name (e.g., "Strava", "GarminConnect")
    pub platform: String,
    /// Whether this platform is enabled
    pub is_enabled: bool,
    /// Whether to auto-upload after ride completion
    pub auto_upload: bool,
    /// Athlete ID from the platform
    pub athlete_id: Option<String>,
    /// Last sync timestamp
    pub last_sync_at: Option<String>,
    /// When the record was created
    pub created_at: String,
    /// When the record was last updated
    pub updated_at: String,
}

impl<'a> SyncStore<'a> {
    // ========== Platform Sync Configuration Operations ==========

    /// Get platform sync configuration by user and platform.
    pub fn get_platform_sync(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<Option<StoredPlatformSync>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, platform, is_enabled, auto_upload, athlete_id,
                        last_sync_at, created_at, updated_at
                 FROM platform_syncs
                 WHERE user_id = ?1 AND platform = ?2",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(
                params![user_id.to_string(), platform],
                Self::map_platform_sync_row,
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(sync)) => Ok(Some(sync)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get platform sync configuration by ID.
    pub fn get_platform_sync_by_id(
        &self,
        id: &Uuid,
    ) -> Result<Option<StoredPlatformSync>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, platform, is_enabled, auto_upload, athlete_id,
                        last_sync_at, created_at, updated_at
                 FROM platform_syncs
                 WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id.to_string()], Self::map_platform_sync_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(sync)) => Ok(Some(sync)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get all platform syncs for a user.
    pub fn get_platform_syncs_by_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<StoredPlatformSync>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, platform, is_enabled, auto_upload, athlete_id,
                        last_sync_at, created_at, updated_at
                 FROM platform_syncs
                 WHERE user_id = ?1
                 ORDER BY platform ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], Self::map_platform_sync_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut syncs = Vec::new();
        for row in rows {
            syncs.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(syncs)
    }

    /// Create or update a platform sync configuration.
    pub fn upsert_platform_sync(&self, sync: &StoredPlatformSync) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO platform_syncs (id, user_id, platform, is_enabled, auto_upload,
                    athlete_id, last_sync_at, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(user_id, platform) DO UPDATE SET
                    is_enabled = excluded.is_enabled,
                    auto_upload = excluded.auto_upload,
                    athlete_id = excluded.athlete_id,
                    last_sync_at = excluded.last_sync_at,
                    updated_at = excluded.updated_at
                "#,
                params![
                    sync.id.to_string(),
                    sync.user_id.to_string(),
                    sync.platform,
                    sync.is_enabled,
                    sync.auto_upload,
                    sync.athlete_id,
                    sync.last_sync_at,
                    sync.created_at,
                    sync.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Update the auto_upload setting for a platform.
    pub fn update_platform_auto_upload(
        &self,
        user_id: &Uuid,
        platform: &str,
        auto_upload: bool,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE platform_syncs SET auto_upload = ?1, updated_at = ?2 WHERE user_id = ?3 AND platform = ?4",
                params![auto_upload, now, user_id.to_string(), platform],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Update the is_enabled setting for a platform.
    pub fn update_platform_enabled(
        &self,
        user_id: &Uuid,
        platform: &str,
        is_enabled: bool,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE platform_syncs SET is_enabled = ?1, updated_at = ?2 WHERE user_id = ?3 AND platform = ?4",
                params![is_enabled, now, user_id.to_string(), platform],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Update both enabled and auto_upload settings for a platform.
    pub fn update_platform_config(
        &self,
        user_id: &Uuid,
        platform: &str,
        is_enabled: bool,
        auto_upload: bool,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE platform_syncs SET is_enabled = ?1, auto_upload = ?2, updated_at = ?3 WHERE user_id = ?4 AND platform = ?5",
                params![is_enabled, auto_upload, now, user_id.to_string(), platform],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Update the last sync timestamp for a platform.
    pub fn update_platform_last_sync(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE platform_syncs SET last_sync_at = ?1, updated_at = ?1 WHERE user_id = ?2 AND platform = ?3",
                params![now, user_id.to_string(), platform],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Delete a platform sync configuration.
    pub fn delete_platform_sync(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM platform_syncs WHERE user_id = ?1 AND platform = ?2",
                params![user_id.to_string(), platform],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Check if auto_upload is enabled for a platform.
    pub fn is_auto_upload_enabled(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<bool, DatabaseError> {
        match self
            .conn
            .query_row(
                "SELECT auto_upload FROM platform_syncs WHERE user_id = ?1 AND platform = ?2",
                params![user_id.to_string(), platform],
                |row| row.get::<_, bool>(0),
            ) {
            Ok(auto_upload) => Ok(auto_upload),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Check if a platform is enabled for a user.
    pub fn is_platform_enabled(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<bool, DatabaseError> {
        match self
            .conn
            .query_row(
                "SELECT is_enabled FROM platform_syncs WHERE user_id = ?1 AND platform = ?2",
                params![user_id.to_string(), platform],
                |row| row.get::<_, bool>(0),
            ) {
            Ok(is_enabled) => Ok(is_enabled),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Helper function to map a row to StoredPlatformSync.
    fn map_platform_sync_row(row: &rusqlite::Row) -> rusqlite::Result<StoredPlatformSync> {
        Ok(StoredPlatformSync {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
            platform: row.get(2)?,
            is_enabled: row.get(3)?,
            auto_upload: row.get(4)?,
            athlete_id: row.get(5)?,
            last_sync_at: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }
}

// ========== TrainingPeaks Workout Sync Tracking (T017) ==========

/// Summary of TrainingPeaks workout sync status.
/// Provides an overview of the current workout sync state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPeaksWorkoutSyncSummary {
    /// Total number of workouts synced from TrainingPeaks
    pub total_synced: usize,
    /// Number of workouts synced in the current date range (last 30 days)
    pub synced_in_range: usize,
    /// When workouts were last synced from TrainingPeaks
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Whether auto-sync is enabled for workout plans
    pub auto_sync_enabled: bool,
    /// Earliest scheduled workout date in synced workouts
    pub earliest_scheduled_date: Option<String>,
    /// Latest scheduled workout date in synced workouts
    pub latest_scheduled_date: Option<String>,
}

impl Default for TrainingPeaksWorkoutSyncSummary {
    fn default() -> Self {
        Self {
            total_synced: 0,
            synced_in_range: 0,
            last_sync_at: None,
            auto_sync_enabled: false,
            earliest_scheduled_date: None,
            latest_scheduled_date: None,
        }
    }
}

/// Individual TrainingPeaks workout sync entry.
/// Represents a single workout that has been synced from TrainingPeaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPeaksWorkoutSyncEntry {
    /// Unique ID for the sync entry
    pub id: Uuid,
    /// External workout ID from TrainingPeaks
    pub external_workout_id: i64,
    /// Local workout ID in our database
    pub local_workout_id: Uuid,
    /// Scheduled date from TrainingPeaks (YYYY-MM-DD format)
    pub scheduled_date: Option<String>,
    /// When this workout was synced
    pub synced_at: DateTime<Utc>,
    /// Last modified date from TrainingPeaks (for detecting updates)
    pub last_modified: Option<String>,
    /// Hash of workout content (for detecting changes)
    pub sync_hash: Option<String>,
}

/// TrainingPeaks workout sync configuration.
/// Settings for automatic workout plan syncing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPeaksWorkoutSyncConfig {
    /// Whether automatic workout sync is enabled
    pub auto_sync_enabled: bool,
    /// Number of days to look ahead for scheduled workouts
    pub lookahead_days: i32,
    /// Number of days to look back for scheduled workouts
    pub lookback_days: i32,
    /// Sync interval in hours (how often to check for new workouts)
    pub sync_interval_hours: i32,
    /// Only sync cycling workouts (filter out running, swimming, etc.)
    pub cycling_only: bool,
    /// Last successful sync timestamp
    pub last_sync_at: Option<String>,
}

impl Default for TrainingPeaksWorkoutSyncConfig {
    fn default() -> Self {
        Self {
            auto_sync_enabled: true,
            lookahead_days: 14,
            lookback_days: 7,
            sync_interval_hours: 6,
            cycling_only: true,
            last_sync_at: None,
        }
    }
}

impl<'a> SyncStore<'a> {
    // ========== TrainingPeaks Workout Sync Operations ==========

    /// Get the workout sync summary for TrainingPeaks.
    pub fn get_trainingpeaks_workout_sync_summary(&self) -> Result<TrainingPeaksWorkoutSyncSummary, DatabaseError> {
        // Get total synced count
        let total_synced: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM trainingpeaks_workout_sync",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Get count in current range (last 30 days from now)
        let thirty_days_ago = (Utc::now() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
        let synced_in_range: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM trainingpeaks_workout_sync WHERE scheduled_date >= ?1",
                params![thirty_days_ago],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Get last sync time
        let last_sync_at: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(synced_at) FROM trainingpeaks_workout_sync",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let last_sync_datetime = last_sync_at.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        // Get date range
        let earliest_date: Option<String> = self
            .conn
            .query_row(
                "SELECT MIN(scheduled_date) FROM trainingpeaks_workout_sync WHERE scheduled_date IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let latest_date: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(scheduled_date) FROM trainingpeaks_workout_sync WHERE scheduled_date IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        Ok(TrainingPeaksWorkoutSyncSummary {
            total_synced: total_synced as usize,
            synced_in_range: synced_in_range as usize,
            last_sync_at: last_sync_datetime,
            auto_sync_enabled: false, // Set from config
            earliest_scheduled_date: earliest_date,
            latest_scheduled_date: latest_date,
        })
    }

    /// Get all TrainingPeaks workout sync entries.
    pub fn get_all_trainingpeaks_workout_syncs(&self) -> Result<Vec<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                        synced_at, last_modified, sync_hash
                 FROM trainingpeaks_workout_sync
                 ORDER BY synced_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_workout_sync_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Get TrainingPeaks workout sync entries for a date range.
    pub fn get_trainingpeaks_workout_syncs_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                        synced_at, last_modified, sync_hash
                 FROM trainingpeaks_workout_sync
                 WHERE scheduled_date >= ?1 AND scheduled_date <= ?2
                 ORDER BY scheduled_date ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![start_date, end_date], Self::map_workout_sync_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Get a TrainingPeaks workout sync entry by external workout ID.
    pub fn get_trainingpeaks_workout_sync_by_external_id(
        &self,
        external_workout_id: i64,
    ) -> Result<Option<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                        synced_at, last_modified, sync_hash
                 FROM trainingpeaks_workout_sync
                 WHERE external_workout_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![external_workout_id], Self::map_workout_sync_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get a TrainingPeaks workout sync entry by local workout ID.
    pub fn get_trainingpeaks_workout_sync_by_local_id(
        &self,
        local_workout_id: &Uuid,
    ) -> Result<Option<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                        synced_at, last_modified, sync_hash
                 FROM trainingpeaks_workout_sync
                 WHERE local_workout_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![local_workout_id.to_string()], Self::map_workout_sync_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Check if a workout has been synced from TrainingPeaks.
    pub fn is_trainingpeaks_workout_synced(&self, external_workout_id: i64) -> Result<bool, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
                params![external_workout_id],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count > 0)
    }

    /// Get the local workout ID for a synced TrainingPeaks workout.
    pub fn get_local_workout_id_for_trainingpeaks(
        &self,
        external_workout_id: i64,
    ) -> Result<Option<Uuid>, DatabaseError> {
        let result: Result<String, rusqlite::Error> = self.conn.query_row(
            "SELECT local_workout_id FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
            params![external_workout_id],
            |row| row.get(0),
        );

        match result {
            Ok(id_str) => {
                let uuid = Uuid::parse_str(&id_str)
                    .map_err(|e| DatabaseError::QueryFailed(format!("Invalid UUID: {}", e)))?;
                Ok(Some(uuid))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get the external workout ID for a local workout.
    pub fn get_external_workout_id_for_local(
        &self,
        local_workout_id: &Uuid,
    ) -> Result<Option<i64>, DatabaseError> {
        let result: Result<i64, rusqlite::Error> = self.conn.query_row(
            "SELECT external_workout_id FROM trainingpeaks_workout_sync WHERE local_workout_id = ?1",
            params![local_workout_id.to_string()],
            |row| row.get(0),
        );

        match result {
            Ok(external_id) => Ok(Some(external_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Record a workout sync from TrainingPeaks.
    pub fn record_trainingpeaks_workout_sync(
        &self,
        entry: &TrainingPeaksWorkoutSyncEntry,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO trainingpeaks_workout_sync
                    (id, external_workout_id, local_workout_id, scheduled_date, synced_at, last_modified, sync_hash)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(external_workout_id) DO UPDATE SET
                    local_workout_id = excluded.local_workout_id,
                    scheduled_date = excluded.scheduled_date,
                    synced_at = excluded.synced_at,
                    last_modified = excluded.last_modified,
                    sync_hash = excluded.sync_hash
                "#,
                params![
                    entry.id.to_string(),
                    entry.external_workout_id,
                    entry.local_workout_id.to_string(),
                    entry.scheduled_date,
                    entry.synced_at.to_rfc3339(),
                    entry.last_modified,
                    entry.sync_hash,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Update the sync hash for a workout sync entry.
    pub fn update_trainingpeaks_workout_sync_hash(
        &self,
        external_workout_id: i64,
        sync_hash: &str,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE trainingpeaks_workout_sync SET sync_hash = ?1, synced_at = ?2 WHERE external_workout_id = ?3",
                params![sync_hash, now, external_workout_id],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Update the last modified date for a workout sync entry.
    pub fn update_trainingpeaks_workout_last_modified(
        &self,
        external_workout_id: i64,
        last_modified: &str,
    ) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE trainingpeaks_workout_sync SET last_modified = ?1 WHERE external_workout_id = ?2",
                params![last_modified, external_workout_id],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Delete a TrainingPeaks workout sync entry.
    pub fn delete_trainingpeaks_workout_sync(&self, external_workout_id: i64) -> Result<bool, DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
                params![external_workout_id],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected > 0)
    }

    /// Clear all TrainingPeaks workout sync entries.
    pub fn clear_all_trainingpeaks_workout_syncs(&self) -> Result<usize, DatabaseError> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM trainingpeaks_workout_sync", [])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(rows_affected)
    }

    /// Get the last sync time for TrainingPeaks workouts.
    pub fn get_trainingpeaks_last_workout_sync_time(&self) -> Result<Option<DateTime<Utc>>, DatabaseError> {
        let result: Result<String, rusqlite::Error> = self.conn.query_row(
            "SELECT MAX(synced_at) FROM trainingpeaks_workout_sync",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(timestamp_str) => {
                let datetime = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::QueryFailed(format!("Invalid timestamp: {}", e)))?;
                Ok(Some(datetime))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => {
                // SQLite returns NULL for MAX on empty table, which causes an error
                if e.to_string().contains("NullPointer") || e.to_string().contains("Null") {
                    Ok(None)
                } else {
                    Err(DatabaseError::QueryFailed(e.to_string()))
                }
            }
        }
    }

    /// Get the count of synced TrainingPeaks workouts.
    pub fn get_trainingpeaks_workout_sync_count(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM trainingpeaks_workout_sync",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Get the count of synced TrainingPeaks workouts in a date range.
    pub fn get_trainingpeaks_workout_sync_count_in_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM trainingpeaks_workout_sync WHERE scheduled_date >= ?1 AND scheduled_date <= ?2",
                params![start_date, end_date],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Check if a sync is due based on last sync time and interval.
    pub fn is_trainingpeaks_workout_sync_due(&self, sync_interval_hours: i32) -> Result<bool, DatabaseError> {
        match self.get_trainingpeaks_last_workout_sync_time()? {
            Some(last_sync) => {
                let next_sync = last_sync + chrono::Duration::hours(sync_interval_hours as i64);
                Ok(Utc::now() >= next_sync)
            }
            None => Ok(true), // Never synced, so sync is due
        }
    }

    /// Get recently synced workouts (within the last N days).
    pub fn get_recently_synced_trainingpeaks_workouts(
        &self,
        days: i32,
    ) -> Result<Vec<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                        synced_at, last_modified, sync_hash
                 FROM trainingpeaks_workout_sync
                 WHERE synced_at >= ?1
                 ORDER BY synced_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![cutoff], Self::map_workout_sync_entry_row)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Get upcoming scheduled workouts from TrainingPeaks.
    pub fn get_upcoming_trainingpeaks_workouts(
        &self,
        days_ahead: i32,
    ) -> Result<Vec<TrainingPeaksWorkoutSyncEntry>, DatabaseError> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let end_date = (Utc::now() + chrono::Duration::days(days_ahead as i64))
            .format("%Y-%m-%d")
            .to_string();

        self.get_trainingpeaks_workout_syncs_by_date_range(&today, &end_date)
    }

    /// Helper function to map a row to TrainingPeaksWorkoutSyncEntry.
    fn map_workout_sync_entry_row(row: &rusqlite::Row) -> rusqlite::Result<TrainingPeaksWorkoutSyncEntry> {
        let id_str: String = row.get(0)?;
        let local_workout_id_str: String = row.get(2)?;
        let synced_at_str: String = row.get(4)?;

        Ok(TrainingPeaksWorkoutSyncEntry {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            external_workout_id: row.get(1)?,
            local_workout_id: Uuid::parse_str(&local_workout_id_str).unwrap_or_default(),
            scheduled_date: row.get(3)?,
            synced_at: DateTime::parse_from_rfc3339(&synced_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_modified: row.get(5)?,
            sync_hash: row.get(6)?,
        })
    }
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

    // ========== Retry Functionality Tests ==========

    #[test]
    fn test_reset_for_retry() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        let entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test.fit".to_string(),
            activity_name: None,
            status: "failed".to_string(),
            error_message: Some("Network error".to_string()),
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 2,
            next_retry_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();

        // Reset for retry
        let reset = store.reset_for_retry(&entry.id).unwrap();
        assert!(reset);

        // Verify status was reset
        let loaded = store.get_queue_entry(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.status, "pending");
        assert!(loaded.error_message.is_none());
        assert!(loaded.next_retry_at.is_none());
        // retry_count should NOT be changed by reset_for_retry
        assert_eq!(loaded.retry_count, 2);
    }

    #[test]
    fn test_reset_for_retry_only_failed() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        // Create a pending entry (not failed)
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

        // Try to reset - should return false since not in failed state
        let reset = store.reset_for_retry(&entry.id).unwrap();
        assert!(!reset);
    }

    #[test]
    fn test_get_failed_entries() {
        let conn = setup_test_db();
        let store = SyncStore::new(&conn);

        // Add pending entry
        let pending_entry = StoredUploadQueueEntry {
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

        // Add failed entry
        let failed_entry = StoredUploadQueueEntry {
            id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: "Strava".to_string(),
            fit_file_path: "/tmp/test2.fit".to_string(),
            activity_name: None,
            status: "failed".to_string(),
            error_message: Some("Max retries exceeded".to_string()),
            external_activity_id: None,
            external_activity_url: None,
            retry_count: 5,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&pending_entry).unwrap();
        store.add_to_queue(&failed_entry).unwrap();

        // Get failed entries
        let failed = store.get_failed_entries().unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].id, failed_entry.id);
        assert_eq!(failed[0].error_message, Some("Max retries exceeded".to_string()));
    }

    #[test]
    fn test_get_retry_count() {
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
            retry_count: 3,
            next_retry_at: None,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        store.add_to_queue(&entry).unwrap();

        // Get retry count
        let count = store.get_retry_count(&entry.id).unwrap();
        assert_eq!(count, Some(3));

        // Non-existent entry
        let count = store.get_retry_count(&Uuid::new_v4()).unwrap();
        assert_eq!(count, None);
    }

    #[test]
    fn test_increment_retry_count() {
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

        // Increment retry count
        store.increment_retry_count(&entry.id).unwrap();

        // Verify incremented
        let count = store.get_retry_count(&entry.id).unwrap();
        assert_eq!(count, Some(1));

        // Increment again
        store.increment_retry_count(&entry.id).unwrap();
        let count = store.get_retry_count(&entry.id).unwrap();
        assert_eq!(count, Some(2));
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        // Test the backoff calculation logic matches service.rs constants
        let base_delay: i64 = 30; // BASE_RETRY_DELAY_SECS

        // First retry: 30 * 2^0 = 30 seconds
        assert_eq!(base_delay * 2_i64.pow(0), 30);

        // Second retry: 30 * 2^1 = 60 seconds
        assert_eq!(base_delay * 2_i64.pow(1), 60);

        // Third retry: 30 * 2^2 = 120 seconds (2 minutes)
        assert_eq!(base_delay * 2_i64.pow(2), 120);

        // Fourth retry: 30 * 2^3 = 240 seconds (4 minutes)
        assert_eq!(base_delay * 2_i64.pow(3), 240);

        // Fifth retry: 30 * 2^4 = 480 seconds (8 minutes)
        assert_eq!(base_delay * 2_i64.pow(4), 480);
    }

    // ========== Platform Sync Configuration Tests ==========

    fn setup_test_db_with_platform_syncs() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create minimal schema for platform_syncs tests
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS platform_syncs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 0,
                auto_upload INTEGER NOT NULL DEFAULT 0,
                access_token_encrypted TEXT,
                refresh_token_encrypted TEXT,
                token_expires_at TEXT,
                scopes_json TEXT,
                athlete_id TEXT,
                last_sync_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(user_id, platform)
            );

            CREATE INDEX IF NOT EXISTS idx_platform_syncs_user ON platform_syncs(user_id);
            "#,
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_upsert_and_get_platform_sync() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: Some("12345".to_string()),
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.platform, "Strava");
        assert!(loaded.is_enabled);
        assert!(loaded.auto_upload);
        assert_eq!(loaded.athlete_id, Some("12345".to_string()));
    }

    #[test]
    fn test_get_platform_sync_not_found() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_update_platform_auto_upload() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // Create initial record with auto_upload = false
        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        // Verify initial state
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(!loaded.auto_upload);

        // Update auto_upload to true
        let updated = store.update_platform_auto_upload(&user_id, "Strava", true).unwrap();
        assert!(updated);

        // Verify update
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(loaded.auto_upload);

        // Update back to false
        store.update_platform_auto_upload(&user_id, "Strava", false).unwrap();
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(!loaded.auto_upload);
    }

    #[test]
    fn test_update_platform_auto_upload_not_found() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();

        // Try to update non-existent record
        let updated = store.update_platform_auto_upload(&user_id, "Strava", true).unwrap();
        assert!(!updated);
    }

    #[test]
    fn test_update_platform_enabled() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: false,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        // Update is_enabled to true
        let updated = store.update_platform_enabled(&user_id, "Strava", true).unwrap();
        assert!(updated);

        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(loaded.is_enabled);
    }

    #[test]
    fn test_update_platform_config() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: false,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        // Update both settings at once
        let updated = store.update_platform_config(&user_id, "Strava", true, true).unwrap();
        assert!(updated);

        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(loaded.is_enabled);
        assert!(loaded.auto_upload);
    }

    #[test]
    fn test_is_auto_upload_enabled() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // No record exists
        assert!(!store.is_auto_upload_enabled(&user_id, "Strava").unwrap());

        // Create record with auto_upload = true
        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();
        assert!(store.is_auto_upload_enabled(&user_id, "Strava").unwrap());
    }

    #[test]
    fn test_is_platform_enabled() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // No record exists
        assert!(!store.is_platform_enabled(&user_id, "Strava").unwrap());

        // Create enabled record
        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();
        assert!(store.is_platform_enabled(&user_id, "Strava").unwrap());
    }

    #[test]
    fn test_get_platform_syncs_by_user() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // Add two platform syncs for the same user
        let strava_sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let garmin_sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "GarminConnect".to_string(),
            is_enabled: false,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&strava_sync).unwrap();
        store.upsert_platform_sync(&garmin_sync).unwrap();

        let syncs = store.get_platform_syncs_by_user(&user_id).unwrap();
        assert_eq!(syncs.len(), 2);

        // Different user should have no syncs
        let other_user = Uuid::new_v4();
        let syncs = store.get_platform_syncs_by_user(&other_user).unwrap();
        assert!(syncs.is_empty());
    }

    #[test]
    fn test_delete_platform_sync() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        // Verify it exists
        assert!(store.get_platform_sync(&user_id, "Strava").unwrap().is_some());

        // Delete it
        let deleted = store.delete_platform_sync(&user_id, "Strava").unwrap();
        assert!(deleted);

        // Verify it's gone
        assert!(store.get_platform_sync(&user_id, "Strava").unwrap().is_none());

        // Deleting again returns false
        let deleted = store.delete_platform_sync(&user_id, "Strava").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_update_platform_last_sync() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        let sync = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        store.upsert_platform_sync(&sync).unwrap();

        // Verify no last_sync_at initially
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(loaded.last_sync_at.is_none());

        // Update last_sync
        let updated = store.update_platform_last_sync(&user_id, "Strava").unwrap();
        assert!(updated);

        // Verify last_sync_at is now set
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        assert!(loaded.last_sync_at.is_some());
    }

    #[test]
    fn test_platform_sync_upsert_conflict() {
        let conn = setup_test_db_with_platform_syncs();
        let store = SyncStore::new(&conn);

        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        // Create initial record
        let sync1 = StoredPlatformSync {
            id: Uuid::new_v4(),
            user_id,
            platform: "Strava".to_string(),
            is_enabled: false,
            auto_upload: false,
            athlete_id: None,
            last_sync_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        store.upsert_platform_sync(&sync1).unwrap();

        // Upsert with different ID but same user_id + platform
        let later = Utc::now().to_rfc3339();
        let sync2 = StoredPlatformSync {
            id: Uuid::new_v4(), // Different ID
            user_id,
            platform: "Strava".to_string(),
            is_enabled: true,
            auto_upload: true,
            athlete_id: Some("99999".to_string()),
            last_sync_at: None,
            created_at: later.clone(),
            updated_at: later,
        };

        store.upsert_platform_sync(&sync2).unwrap();

        // Should have updated the existing record
        let loaded = store.get_platform_sync(&user_id, "Strava").unwrap().unwrap();
        // The id should be the original one (sync1.id) since we're doing ON CONFLICT UPDATE
        assert_eq!(loaded.id, sync1.id);
        // But the values should be updated
        assert!(loaded.is_enabled);
        assert!(loaded.auto_upload);
        assert_eq!(loaded.athlete_id, Some("99999".to_string()));
    }

    // ========== TrainingPeaks Workout Sync Tests (T017) ==========

    fn setup_test_db_with_trainingpeaks_workout_sync() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS trainingpeaks_workout_sync (
                id TEXT PRIMARY KEY,
                external_workout_id INTEGER NOT NULL UNIQUE,
                local_workout_id TEXT NOT NULL,
                scheduled_date TEXT,
                synced_at TEXT NOT NULL,
                last_modified TEXT,
                sync_hash TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_tp_workout_sync_external ON trainingpeaks_workout_sync(external_workout_id);
            CREATE INDEX IF NOT EXISTS idx_tp_workout_sync_local ON trainingpeaks_workout_sync(local_workout_id);
            CREATE INDEX IF NOT EXISTS idx_tp_workout_sync_date ON trainingpeaks_workout_sync(scheduled_date);
            "#,
        )
        .unwrap();

        conn
    }

    fn create_test_workout_sync_entry(external_id: i64, scheduled_date: Option<&str>) -> TrainingPeaksWorkoutSyncEntry {
        TrainingPeaksWorkoutSyncEntry {
            id: Uuid::new_v4(),
            external_workout_id: external_id,
            local_workout_id: Uuid::new_v4(),
            scheduled_date: scheduled_date.map(|s| s.to_string()),
            synced_at: Utc::now(),
            last_modified: None,
            sync_hash: None,
        }
    }

    #[test]
    fn test_record_and_get_trainingpeaks_workout_sync() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, Some("2026-01-15"));

        // Record the sync
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Get by external ID
        let loaded = store.get_trainingpeaks_workout_sync_by_external_id(12345).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.external_workout_id, 12345);
        assert_eq!(loaded.local_workout_id, entry.local_workout_id);
        assert_eq!(loaded.scheduled_date, Some("2026-01-15".to_string()));
    }

    #[test]
    fn test_is_trainingpeaks_workout_synced() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Initially not synced
        assert!(!store.is_trainingpeaks_workout_synced(12345).unwrap());

        // Record sync
        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Now synced
        assert!(store.is_trainingpeaks_workout_synced(12345).unwrap());

        // Different ID still not synced
        assert!(!store.is_trainingpeaks_workout_synced(99999).unwrap());
    }

    #[test]
    fn test_get_local_workout_id_for_trainingpeaks() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Get local ID
        let local_id = store.get_local_workout_id_for_trainingpeaks(12345).unwrap();
        assert!(local_id.is_some());
        assert_eq!(local_id.unwrap(), entry.local_workout_id);

        // Non-existent returns None
        let missing = store.get_local_workout_id_for_trainingpeaks(99999).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_external_workout_id_for_local() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Get external ID
        let external_id = store.get_external_workout_id_for_local(&entry.local_workout_id).unwrap();
        assert!(external_id.is_some());
        assert_eq!(external_id.unwrap(), 12345);

        // Non-existent returns None
        let missing = store.get_external_workout_id_for_local(&Uuid::new_v4()).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_all_trainingpeaks_workout_syncs() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Initially empty
        let all = store.get_all_trainingpeaks_workout_syncs().unwrap();
        assert!(all.is_empty());

        // Add entries
        let entry1 = create_test_workout_sync_entry(1001, Some("2026-01-10"));
        let entry2 = create_test_workout_sync_entry(1002, Some("2026-01-15"));
        store.record_trainingpeaks_workout_sync(&entry1).unwrap();
        store.record_trainingpeaks_workout_sync(&entry2).unwrap();

        // Get all
        let all = store.get_all_trainingpeaks_workout_syncs().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_trainingpeaks_workout_syncs_by_date_range() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Add entries with different dates
        let entry1 = create_test_workout_sync_entry(1001, Some("2026-01-10"));
        let entry2 = create_test_workout_sync_entry(1002, Some("2026-01-15"));
        let entry3 = create_test_workout_sync_entry(1003, Some("2026-01-20"));

        store.record_trainingpeaks_workout_sync(&entry1).unwrap();
        store.record_trainingpeaks_workout_sync(&entry2).unwrap();
        store.record_trainingpeaks_workout_sync(&entry3).unwrap();

        // Query mid-range
        let range = store.get_trainingpeaks_workout_syncs_by_date_range("2026-01-12", "2026-01-18").unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].external_workout_id, 1002);

        // Query all
        let all = store.get_trainingpeaks_workout_syncs_by_date_range("2026-01-01", "2026-01-31").unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_trainingpeaks_workout_sync_by_local_id() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, Some("2026-01-15"));
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Get by local ID
        let loaded = store.get_trainingpeaks_workout_sync_by_local_id(&entry.local_workout_id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.external_workout_id, 12345);

        // Non-existent returns None
        let missing = store.get_trainingpeaks_workout_sync_by_local_id(&Uuid::new_v4()).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_trainingpeaks_workout_sync_hash() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Update hash
        let updated = store.update_trainingpeaks_workout_sync_hash(12345, "abc123hash").unwrap();
        assert!(updated);

        // Verify
        let loaded = store.get_trainingpeaks_workout_sync_by_external_id(12345).unwrap().unwrap();
        assert_eq!(loaded.sync_hash, Some("abc123hash".to_string()));

        // Non-existent returns false
        let not_updated = store.update_trainingpeaks_workout_sync_hash(99999, "test").unwrap();
        assert!(!not_updated);
    }

    #[test]
    fn test_update_trainingpeaks_workout_last_modified() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Update last_modified
        let updated = store.update_trainingpeaks_workout_last_modified(12345, "2026-01-15T10:00:00Z").unwrap();
        assert!(updated);

        // Verify
        let loaded = store.get_trainingpeaks_workout_sync_by_external_id(12345).unwrap().unwrap();
        assert_eq!(loaded.last_modified, Some("2026-01-15T10:00:00Z".to_string()));
    }

    #[test]
    fn test_delete_trainingpeaks_workout_sync() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        let entry = create_test_workout_sync_entry(12345, None);
        store.record_trainingpeaks_workout_sync(&entry).unwrap();

        // Verify exists
        assert!(store.is_trainingpeaks_workout_synced(12345).unwrap());

        // Delete
        let deleted = store.delete_trainingpeaks_workout_sync(12345).unwrap();
        assert!(deleted);

        // Verify deleted
        assert!(!store.is_trainingpeaks_workout_synced(12345).unwrap());

        // Deleting again returns false
        let deleted_again = store.delete_trainingpeaks_workout_sync(12345).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_clear_all_trainingpeaks_workout_syncs() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Add multiple entries
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1001, None)).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1002, None)).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1003, None)).unwrap();

        assert_eq!(store.get_trainingpeaks_workout_sync_count().unwrap(), 3);

        // Clear all
        let cleared = store.clear_all_trainingpeaks_workout_syncs().unwrap();
        assert_eq!(cleared, 3);

        // Verify empty
        assert_eq!(store.get_trainingpeaks_workout_sync_count().unwrap(), 0);
    }

    #[test]
    fn test_get_trainingpeaks_workout_sync_count() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Initially zero
        assert_eq!(store.get_trainingpeaks_workout_sync_count().unwrap(), 0);

        // Add entries
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1001, None)).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1002, None)).unwrap();

        assert_eq!(store.get_trainingpeaks_workout_sync_count().unwrap(), 2);
    }

    #[test]
    fn test_get_trainingpeaks_workout_sync_count_in_range() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Add entries with different dates
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1001, Some("2026-01-10"))).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1002, Some("2026-01-15"))).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1003, Some("2026-01-20"))).unwrap();

        // Count in mid-range
        let count = store.get_trainingpeaks_workout_sync_count_in_range("2026-01-12", "2026-01-18").unwrap();
        assert_eq!(count, 1);

        // Count all
        let count = store.get_trainingpeaks_workout_sync_count_in_range("2026-01-01", "2026-01-31").unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_is_trainingpeaks_workout_sync_due() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // With no syncs, should be due
        assert!(store.is_trainingpeaks_workout_sync_due(6).unwrap());

        // Add a recent sync
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1001, None)).unwrap();

        // Should not be due yet (we just synced)
        assert!(!store.is_trainingpeaks_workout_sync_due(6).unwrap());
    }

    #[test]
    fn test_get_trainingpeaks_workout_sync_summary() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Empty summary
        let summary = store.get_trainingpeaks_workout_sync_summary().unwrap();
        assert_eq!(summary.total_synced, 0);
        assert_eq!(summary.synced_in_range, 0);
        assert!(summary.last_sync_at.is_none());
        assert!(summary.earliest_scheduled_date.is_none());
        assert!(summary.latest_scheduled_date.is_none());

        // Add entries
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1001, Some("2026-01-10"))).unwrap();
        store.record_trainingpeaks_workout_sync(&create_test_workout_sync_entry(1002, Some("2026-01-20"))).unwrap();

        let summary = store.get_trainingpeaks_workout_sync_summary().unwrap();
        assert_eq!(summary.total_synced, 2);
        assert!(summary.last_sync_at.is_some());
        assert_eq!(summary.earliest_scheduled_date, Some("2026-01-10".to_string()));
        assert_eq!(summary.latest_scheduled_date, Some("2026-01-20".to_string()));
    }

    #[test]
    fn test_upsert_trainingpeaks_workout_sync() {
        let conn = setup_test_db_with_trainingpeaks_workout_sync();
        let store = SyncStore::new(&conn);

        // Initial sync
        let entry1 = create_test_workout_sync_entry(12345, Some("2026-01-15"));
        store.record_trainingpeaks_workout_sync(&entry1).unwrap();

        // Update with same external ID
        let mut entry2 = create_test_workout_sync_entry(12345, Some("2026-01-16"));
        entry2.sync_hash = Some("updated_hash".to_string());
        store.record_trainingpeaks_workout_sync(&entry2).unwrap();

        // Should still have only 1 entry
        assert_eq!(store.get_trainingpeaks_workout_sync_count().unwrap(), 1);

        // Should have updated values
        let loaded = store.get_trainingpeaks_workout_sync_by_external_id(12345).unwrap().unwrap();
        assert_eq!(loaded.scheduled_date, Some("2026-01-16".to_string()));
        assert_eq!(loaded.sync_hash, Some("updated_hash".to_string()));
    }

    #[test]
    fn test_trainingpeaks_workout_sync_config_defaults() {
        let config = TrainingPeaksWorkoutSyncConfig::default();
        assert!(config.auto_sync_enabled);
        assert_eq!(config.lookahead_days, 14);
        assert_eq!(config.lookback_days, 7);
        assert_eq!(config.sync_interval_hours, 6);
        assert!(config.cycling_only);
        assert!(config.last_sync_at.is_none());
    }

    #[test]
    fn test_trainingpeaks_workout_sync_summary_defaults() {
        let summary = TrainingPeaksWorkoutSyncSummary::default();
        assert_eq!(summary.total_synced, 0);
        assert_eq!(summary.synced_in_range, 0);
        assert!(summary.last_sync_at.is_none());
        assert!(!summary.auto_sync_enabled);
        assert!(summary.earliest_scheduled_date.is_none());
        assert!(summary.latest_scheduled_date.is_none());
    }
}
