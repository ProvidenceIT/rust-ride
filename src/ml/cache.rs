//! ML prediction cache with SQLite backing.
//!
//! T013: Create MlCache struct
//! T014: Implement cache expiry logic

use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use super::types::{MlError, PredictionSource, PredictionType};

/// SQLite-backed prediction cache.
pub struct MlCache<'a> {
    conn: &'a Connection,
}

impl<'a> MlCache<'a> {
    /// Create a new ML cache with a database connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Store a prediction with automatic expiry based on type.
    pub fn store<T: Serialize>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
        payload: &T,
        confidence: f32,
        source: PredictionSource,
    ) -> Result<Uuid, MlError> {
        let expires_in = Duration::hours(prediction_type.cache_expiry_hours() as i64);
        self.store_with_expiry(user_id, prediction_type, payload, confidence, source, expires_in)
    }

    /// Store a prediction with custom expiry duration.
    pub fn store_with_expiry<T: Serialize>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
        payload: &T,
        confidence: f32,
        source: PredictionSource,
        expires_in: Duration,
    ) -> Result<Uuid, MlError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + expires_in;
        let payload_json = serde_json::to_string(payload)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO ml_predictions
             (id, user_id, prediction_type, payload, confidence, created_at, expires_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id.to_string(),
                user_id.to_string(),
                format!("{:?}", prediction_type),
                payload_json,
                confidence,
                now.to_rfc3339(),
                expires_at.to_rfc3339(),
                format!("{:?}", source),
            ],
        )?;

        Ok(id)
    }

    /// Get a cached prediction if not expired.
    ///
    /// Returns None if no prediction exists or if expired (unless include_stale is true).
    pub fn get<T: DeserializeOwned>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
    ) -> Result<Option<CachedPrediction<T>>, MlError> {
        self.get_with_stale(user_id, prediction_type, false)
    }

    /// Get a cached prediction, optionally including stale (expired) predictions.
    pub fn get_with_stale<T: DeserializeOwned>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
        include_stale: bool,
    ) -> Result<Option<CachedPrediction<T>>, MlError> {
        use rusqlite::OptionalExtension;

        let result: Option<(String, f32, String, String, String)> = self
            .conn
            .query_row(
                "SELECT payload, confidence, created_at, expires_at, source
                 FROM ml_predictions
                 WHERE user_id = ?1 AND prediction_type = ?2
                 ORDER BY created_at DESC LIMIT 1",
                rusqlite::params![user_id.to_string(), format!("{:?}", prediction_type)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;

        match result {
            Some((payload_json, confidence, created_str, expires_str, source_str)) => {
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| MlError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc);

                let is_stale = Utc::now() > expires_at;

                // Return None if expired and we don't want stale data
                if is_stale && !include_stale {
                    return Ok(None);
                }

                let payload: T = serde_json::from_str(&payload_json)?;
                let cached_at = DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| MlError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc);
                let source = parse_source(&source_str);

                Ok(Some(CachedPrediction {
                    payload,
                    confidence,
                    cached_at,
                    expires_at,
                    source,
                    is_stale,
                }))
            }
            None => Ok(None),
        }
    }

    /// Check if a valid (non-expired) cache entry exists.
    pub fn has_valid(&self, user_id: Uuid, prediction_type: PredictionType) -> Result<bool, MlError> {
        let now = Utc::now().to_rfc3339();

        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM ml_predictions
             WHERE user_id = ?1 AND prediction_type = ?2 AND expires_at > ?3",
            rusqlite::params![user_id.to_string(), format!("{:?}", prediction_type), now],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Invalidate all predictions of a type for a user.
    pub fn invalidate(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
    ) -> Result<usize, MlError> {
        let deleted = self.conn.execute(
            "DELETE FROM ml_predictions WHERE user_id = ?1 AND prediction_type = ?2",
            rusqlite::params![user_id.to_string(), format!("{:?}", prediction_type)],
        )?;
        Ok(deleted)
    }

    /// Clean up expired predictions.
    pub fn cleanup_expired(&self) -> Result<usize, MlError> {
        let now = Utc::now().to_rfc3339();
        let deleted = self.conn.execute(
            "DELETE FROM ml_predictions WHERE expires_at < ?1",
            rusqlite::params![now],
        )?;

        if deleted > 0 {
            tracing::info!("Cleaned up {} expired ML predictions", deleted);
        }

        Ok(deleted)
    }

    /// Get cache statistics.
    pub fn stats(&self, user_id: Uuid) -> Result<CacheStats, MlError> {
        let now = Utc::now().to_rfc3339();

        let total: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM ml_predictions WHERE user_id = ?1",
            rusqlite::params![user_id.to_string()],
            |row| row.get(0),
        )?;

        let valid: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM ml_predictions WHERE user_id = ?1 AND expires_at > ?2",
            rusqlite::params![user_id.to_string(), now],
            |row| row.get(0),
        )?;

        let expired: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM ml_predictions WHERE user_id = ?1 AND expires_at <= ?2",
            rusqlite::params![user_id.to_string(), now],
            |row| row.get(0),
        )?;

        Ok(CacheStats {
            total_entries: total as usize,
            valid_entries: valid as usize,
            expired_entries: expired as usize,
        })
    }

    /// Get the last update time for a prediction type.
    pub fn last_updated(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
    ) -> Result<Option<DateTime<Utc>>, MlError> {
        use rusqlite::OptionalExtension;

        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT created_at FROM ml_predictions
                 WHERE user_id = ?1 AND prediction_type = ?2
                 ORDER BY created_at DESC LIMIT 1",
                rusqlite::params![user_id.to_string(), format!("{:?}", prediction_type)],
                |row| row.get(0),
            )
            .optional()?;

        match result {
            Some(created_str) => {
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| MlError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(Some(created_at))
            }
            None => Ok(None),
        }
    }
}

/// A cached prediction with metadata.
#[derive(Debug, Clone)]
pub struct CachedPrediction<T> {
    /// The prediction payload
    pub payload: T,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// When the prediction was cached
    pub cached_at: DateTime<Utc>,
    /// When the prediction expires
    pub expires_at: DateTime<Utc>,
    /// Source of the prediction
    pub source: PredictionSource,
    /// Whether the prediction has expired (stale)
    pub is_stale: bool,
}

impl<T> CachedPrediction<T> {
    /// Check if the prediction is still fresh (not stale).
    pub fn is_fresh(&self) -> bool {
        !self.is_stale
    }

    /// Get the age of the cached prediction.
    pub fn age(&self) -> Duration {
        Utc::now() - self.cached_at
    }

    /// Get human-readable age description.
    pub fn age_description(&self) -> String {
        let age = self.age();
        if age.num_hours() < 1 {
            format!("{} minutes ago", age.num_minutes())
        } else if age.num_days() < 1 {
            format!("{} hours ago", age.num_hours())
        } else {
            format!("{} days ago", age.num_days())
        }
    }

    /// Get time until expiry (negative if already expired).
    pub fn time_until_expiry(&self) -> Duration {
        self.expires_at - Utc::now()
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cached entries
    pub total_entries: usize,
    /// Number of valid (non-expired) entries
    pub valid_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
}

fn parse_source(s: &str) -> PredictionSource {
    match s {
        "Cloud" => PredictionSource::Cloud,
        "Cached" => PredictionSource::Cached,
        _ => PredictionSource::LocalFallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE ml_predictions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                prediction_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                confidence REAL NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                source TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        (file, conn)
    }

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    struct TestPayload {
        value: i32,
    }

    #[test]
    fn test_store_and_retrieve() {
        let (_file, conn) = setup_test_db();
        let cache = MlCache::new(&conn);
        let user_id = Uuid::new_v4();

        let payload = TestPayload { value: 42 };
        cache
            .store(
                user_id,
                PredictionType::FtpPrediction,
                &payload,
                0.9,
                PredictionSource::Cloud,
            )
            .unwrap();

        let cached: Option<CachedPrediction<TestPayload>> = cache
            .get(user_id, PredictionType::FtpPrediction)
            .unwrap();

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.payload.value, 42);
        assert_eq!(cached.confidence, 0.9);
        assert!(!cached.is_stale);
    }

    #[test]
    fn test_expired_cache() {
        let (_file, conn) = setup_test_db();
        let cache = MlCache::new(&conn);
        let user_id = Uuid::new_v4();

        let payload = TestPayload { value: 42 };

        // Store with negative expiry (already expired)
        cache
            .store_with_expiry(
                user_id,
                PredictionType::FtpPrediction,
                &payload,
                0.9,
                PredictionSource::Cloud,
                Duration::hours(-1),
            )
            .unwrap();

        // Should not return expired cache by default
        let cached: Option<CachedPrediction<TestPayload>> = cache
            .get(user_id, PredictionType::FtpPrediction)
            .unwrap();
        assert!(cached.is_none());

        // Should return if we ask for stale data
        let cached: Option<CachedPrediction<TestPayload>> = cache
            .get_with_stale(user_id, PredictionType::FtpPrediction, true)
            .unwrap();
        assert!(cached.is_some());
        assert!(cached.unwrap().is_stale);
    }

    #[test]
    fn test_has_valid() {
        let (_file, conn) = setup_test_db();
        let cache = MlCache::new(&conn);
        let user_id = Uuid::new_v4();

        assert!(!cache.has_valid(user_id, PredictionType::FtpPrediction).unwrap());

        cache
            .store(
                user_id,
                PredictionType::FtpPrediction,
                &TestPayload { value: 1 },
                0.9,
                PredictionSource::Cloud,
            )
            .unwrap();

        assert!(cache.has_valid(user_id, PredictionType::FtpPrediction).unwrap());
    }

    #[test]
    fn test_invalidate() {
        let (_file, conn) = setup_test_db();
        let cache = MlCache::new(&conn);
        let user_id = Uuid::new_v4();

        cache
            .store(
                user_id,
                PredictionType::FtpPrediction,
                &TestPayload { value: 1 },
                0.9,
                PredictionSource::Cloud,
            )
            .unwrap();

        assert!(cache.has_valid(user_id, PredictionType::FtpPrediction).unwrap());

        cache.invalidate(user_id, PredictionType::FtpPrediction).unwrap();

        assert!(!cache.has_valid(user_id, PredictionType::FtpPrediction).unwrap());
    }

    #[test]
    fn test_stats() {
        let (_file, conn) = setup_test_db();
        let cache = MlCache::new(&conn);
        let user_id = Uuid::new_v4();

        let stats = cache.stats(user_id).unwrap();
        assert_eq!(stats.total_entries, 0);

        cache
            .store(
                user_id,
                PredictionType::FtpPrediction,
                &TestPayload { value: 1 },
                0.9,
                PredictionSource::Cloud,
            )
            .unwrap();

        let stats = cache.stats(user_id).unwrap();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }
}
