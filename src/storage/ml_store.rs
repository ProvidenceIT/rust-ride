//! ML prediction storage operations.
//!
//! T009: Create MlStore for ML prediction CRUD operations

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::ml::types::{MlError, PredictionSource, PredictionType};

/// Store for ML predictions and related data.
pub struct MlStore<'a> {
    conn: &'a Connection,
}

impl<'a> MlStore<'a> {
    /// Create a new ML store with a database connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== ML Predictions ==========

    /// Store a prediction in the cache.
    pub fn store_prediction<T: Serialize>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
        payload: &T,
        confidence: f32,
        source: PredictionSource,
    ) -> Result<Uuid, MlError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::hours(prediction_type.cache_expiry_hours() as i64);
        let payload_json = serde_json::to_string(payload)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO ml_predictions
             (id, user_id, prediction_type, payload, confidence, created_at, expires_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
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

    /// Get the latest prediction of a type for a user.
    pub fn get_prediction<T: DeserializeOwned>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
    ) -> Result<Option<CachedPrediction<T>>, MlError> {
        let result: Option<(String, f32, String, String, String)> = self
            .conn
            .query_row(
                "SELECT payload, confidence, created_at, expires_at, source
                 FROM ml_predictions
                 WHERE user_id = ?1 AND prediction_type = ?2
                 ORDER BY created_at DESC LIMIT 1",
                params![user_id.to_string(), format!("{:?}", prediction_type)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;

        match result {
            Some((payload_json, confidence, created_str, expires_str, source_str)) => {
                let payload: T = serde_json::from_str(&payload_json)?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| MlError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc);
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| MlError::DatabaseError(e.to_string()))?
                    .with_timezone(&Utc);
                let source = parse_prediction_source(&source_str);
                let is_expired = Utc::now() > expires_at;

                Ok(Some(CachedPrediction {
                    payload,
                    confidence,
                    created_at,
                    expires_at,
                    source,
                    is_expired,
                }))
            }
            None => Ok(None),
        }
    }

    /// Delete expired predictions.
    pub fn cleanup_expired_predictions(&self) -> Result<usize, MlError> {
        let now = Utc::now().to_rfc3339();
        let deleted = self.conn.execute(
            "DELETE FROM ml_predictions WHERE expires_at < ?1",
            params![now],
        )?;
        Ok(deleted)
    }

    // ========== Fatigue States ==========

    /// Store a fatigue state reading.
    pub fn store_fatigue_state(&self, state: &FatigueStateRecord) -> Result<i64, MlError> {
        self.conn.execute(
            "INSERT INTO fatigue_states
             (ride_id, timestamp, aerobic_decoupling_score, power_variability_index,
              hrv_fatigue_indicator, alert_triggered, alert_dismissed, cooldown_expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                state.ride_id.to_string(),
                state.timestamp.to_rfc3339(),
                state.aerobic_decoupling_score,
                state.power_variability_index,
                state.hrv_fatigue_indicator,
                state.alert_triggered as i32,
                state.alert_dismissed as i32,
                state.cooldown_expires_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get fatigue states for a ride.
    pub fn get_fatigue_states(&self, ride_id: Uuid) -> Result<Vec<FatigueStateRecord>, MlError> {
        let mut stmt = self.conn.prepare(
            "SELECT ride_id, timestamp, aerobic_decoupling_score, power_variability_index,
                    hrv_fatigue_indicator, alert_triggered, alert_dismissed, cooldown_expires_at
             FROM fatigue_states WHERE ride_id = ?1 ORDER BY timestamp",
        )?;

        let rows = stmt.query_map(params![ride_id.to_string()], |row| {
            let ride_id_str: String = row.get(0)?;
            let timestamp_str: String = row.get(1)?;
            let cooldown_str: Option<String> = row.get(7)?;

            Ok(FatigueStateRecord {
                ride_id: Uuid::parse_str(&ride_id_str).unwrap_or_default(),
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|t| t.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                aerobic_decoupling_score: row.get(2)?,
                power_variability_index: row.get(3)?,
                hrv_fatigue_indicator: row.get(4)?,
                alert_triggered: row.get::<_, i32>(5)? != 0,
                alert_dismissed: row.get::<_, i32>(6)? != 0,
                cooldown_expires_at: cooldown_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|t| t.with_timezone(&Utc))
                        .ok()
                }),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| MlError::DatabaseError(e.to_string()))
    }

    /// Update alert dismissal state.
    pub fn dismiss_fatigue_alert(
        &self,
        ride_id: Uuid,
        cooldown_minutes: u32,
    ) -> Result<(), MlError> {
        let cooldown_expires = Utc::now() + Duration::minutes(cooldown_minutes as i64);
        self.conn.execute(
            "UPDATE fatigue_states
             SET alert_dismissed = 1, cooldown_expires_at = ?1
             WHERE ride_id = ?2 AND alert_triggered = 1 AND alert_dismissed = 0",
            params![cooldown_expires.to_rfc3339(), ride_id.to_string()],
        )?;
        Ok(())
    }

    // ========== Workout Recommendations ==========

    /// Store a workout recommendation.
    pub fn store_recommendation(&self, rec: &WorkoutRecommendationRecord) -> Result<(), MlError> {
        self.conn.execute(
            "INSERT INTO workout_recommendations
             (id, user_id, workout_id, workout_source, suitability_score, reasoning,
              target_energy_systems, expected_tss, goal_id, training_gap, recommended_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                rec.id.to_string(),
                rec.user_id.to_string(),
                rec.workout_id.to_string(),
                rec.workout_source,
                rec.suitability_score,
                rec.reasoning,
                serde_json::to_string(&rec.target_energy_systems)?,
                rec.expected_tss,
                rec.goal_id.map(|id| id.to_string()),
                rec.training_gap,
                rec.recommended_at.to_rfc3339(),
                rec.status,
            ],
        )?;
        Ok(())
    }

    /// Get pending recommendations for a user.
    pub fn get_pending_recommendations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<WorkoutRecommendationRecord>, MlError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, workout_id, workout_source, suitability_score, reasoning,
                    target_energy_systems, expected_tss, goal_id, training_gap, recommended_at, status
             FROM workout_recommendations
             WHERE user_id = ?1 AND status = 'pending'
             ORDER BY suitability_score DESC",
        )?;

        let rows = stmt.query_map(params![user_id.to_string()], parse_recommendation_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| MlError::DatabaseError(e.to_string()))
    }

    /// Update recommendation status.
    pub fn update_recommendation_status(
        &self,
        id: Uuid,
        status: &str,
        completed_at: Option<DateTime<Utc>>,
    ) -> Result<(), MlError> {
        self.conn.execute(
            "UPDATE workout_recommendations SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![
                status,
                completed_at.map(|t| t.to_rfc3339()),
                id.to_string()
            ],
        )?;
        Ok(())
    }
}

/// A cached prediction with metadata.
#[derive(Debug, Clone)]
pub struct CachedPrediction<T> {
    /// The prediction payload
    pub payload: T,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// When the prediction was created
    pub created_at: DateTime<Utc>,
    /// When the prediction expires
    pub expires_at: DateTime<Utc>,
    /// Source of the prediction
    pub source: PredictionSource,
    /// Whether the prediction has expired
    pub is_expired: bool,
}

impl<T> CachedPrediction<T> {
    /// Check if the prediction is still fresh (not expired).
    pub fn is_fresh(&self) -> bool {
        !self.is_expired
    }

    /// Get age of the prediction.
    pub fn age(&self) -> Duration {
        Utc::now() - self.created_at
    }
}

/// Fatigue state database record.
#[derive(Debug, Clone)]
pub struct FatigueStateRecord {
    pub ride_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub aerobic_decoupling_score: f32,
    pub power_variability_index: f32,
    pub hrv_fatigue_indicator: Option<f32>,
    pub alert_triggered: bool,
    pub alert_dismissed: bool,
    pub cooldown_expires_at: Option<DateTime<Utc>>,
}

/// Workout recommendation database record.
#[derive(Debug, Clone)]
pub struct WorkoutRecommendationRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workout_id: Uuid,
    pub workout_source: String,
    pub suitability_score: f32,
    pub reasoning: String,
    pub target_energy_systems: Vec<String>,
    pub expected_tss: f32,
    pub goal_id: Option<Uuid>,
    pub training_gap: Option<String>,
    pub recommended_at: DateTime<Utc>,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
}

fn parse_prediction_source(s: &str) -> PredictionSource {
    match s {
        "Cloud" => PredictionSource::Cloud,
        "Cached" => PredictionSource::Cached,
        _ => PredictionSource::LocalFallback,
    }
}

fn parse_recommendation_row(row: &rusqlite::Row) -> rusqlite::Result<WorkoutRecommendationRecord> {
    let id_str: String = row.get(0)?;
    let user_id_str: String = row.get(1)?;
    let workout_id_str: String = row.get(2)?;
    let energy_systems_json: String = row.get(6)?;
    let goal_id_str: Option<String> = row.get(8)?;
    let recommended_at_str: String = row.get(10)?;

    Ok(WorkoutRecommendationRecord {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        user_id: Uuid::parse_str(&user_id_str).unwrap_or_default(),
        workout_id: Uuid::parse_str(&workout_id_str).unwrap_or_default(),
        workout_source: row.get(3)?,
        suitability_score: row.get(4)?,
        reasoning: row.get(5)?,
        target_energy_systems: serde_json::from_str(&energy_systems_json).unwrap_or_default(),
        expected_tss: row.get(7)?,
        goal_id: goal_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        training_gap: row.get(9)?,
        recommended_at: DateTime::parse_from_rfc3339(&recommended_at_str)
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        status: row.get(11)?,
        completed_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        // Create minimal schema for testing
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
            CREATE TABLE fatigue_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ride_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                aerobic_decoupling_score REAL NOT NULL,
                power_variability_index REAL NOT NULL,
                hrv_fatigue_indicator REAL,
                alert_triggered INTEGER NOT NULL DEFAULT 0,
                alert_dismissed INTEGER NOT NULL DEFAULT 0,
                cooldown_expires_at TEXT
            );
            "#,
        )
        .unwrap();

        (file, conn)
    }

    #[test]
    fn test_store_and_get_prediction() {
        let (_file, conn) = setup_test_db();
        let store = MlStore::new(&conn);
        let user_id = Uuid::new_v4();

        #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
        struct TestPayload {
            ftp: u16,
        }

        let payload = TestPayload { ftp: 280 };
        store
            .store_prediction(
                user_id,
                PredictionType::FtpPrediction,
                &payload,
                0.9,
                PredictionSource::Cloud,
            )
            .unwrap();

        let cached: Option<CachedPrediction<TestPayload>> = store
            .get_prediction(user_id, PredictionType::FtpPrediction)
            .unwrap();

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.payload.ftp, 280);
        assert_eq!(cached.confidence, 0.9);
        assert!(!cached.is_expired);
    }

    #[test]
    fn test_fatigue_state_storage() {
        let (_file, conn) = setup_test_db();
        let store = MlStore::new(&conn);
        let ride_id = Uuid::new_v4();

        let state = FatigueStateRecord {
            ride_id,
            timestamp: Utc::now(),
            aerobic_decoupling_score: 0.12,
            power_variability_index: 1.35,
            hrv_fatigue_indicator: None,
            alert_triggered: true,
            alert_dismissed: false,
            cooldown_expires_at: None,
        };

        store.store_fatigue_state(&state).unwrap();

        let states = store.get_fatigue_states(ride_id).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].aerobic_decoupling_score, 0.12);
        assert!(states[0].alert_triggered);
    }
}
