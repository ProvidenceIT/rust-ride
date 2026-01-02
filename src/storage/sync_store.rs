//! Sync storage operations for upload queue persistence.
//!
//! Provides persistence for:
//! - Upload queue entries (pending uploads that survive app restart)
//! - Sync records (upload history and status tracking)
//!
//! This module handles offline scenarios by storing uploads in a persistent
//! queue that can be processed when connectivity is restored.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::storage::database::DatabaseError;

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
}
