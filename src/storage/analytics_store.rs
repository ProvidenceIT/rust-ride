//! Analytics data storage operations.
//!
//! Provides persistence for:
//! - Power Duration Curve (PDC) points
//! - Critical Power (CP) models
//! - FTP estimates
//! - Daily training load (ATL/CTL/TSB)
//! - VO2max estimates
//! - Rider profiles

use chrono::{NaiveDate, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::metrics::analytics::vo2max::{FitnessLevel, Vo2maxMethod};
use crate::metrics::analytics::{
    CpModel, DailyLoad, FtpConfidence, FtpEstimate, FtpMethod, PdcPoint, PowerDurationCurve,
    PowerProfile, RiderType, Vo2maxResult,
};
use crate::storage::database::DatabaseError;

/// Analytics store for persisting training analytics data.
pub struct AnalyticsStore<'a> {
    conn: &'a Connection,
}

impl<'a> AnalyticsStore<'a> {
    /// Create a new analytics store with the given connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== PDC Storage (T010) ==========

    /// Load the Power Duration Curve for a user.
    pub fn load_pdc(&self, user_id: &Uuid) -> Result<PowerDurationCurve, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT duration_secs, power_watts FROM pdc_points
                 WHERE user_id = ?1 ORDER BY duration_secs",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(PdcPoint {
                    duration_secs: row.get(0)?,
                    power_watts: row.get(1)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut points = Vec::new();
        for row in rows {
            points.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }

        Ok(PowerDurationCurve::from_points(points))
    }

    /// Save or update PDC points for a user.
    /// Uses INSERT OR REPLACE to update existing points at the same duration.
    pub fn save_pdc_points(
        &self,
        user_id: &Uuid,
        points: &[PdcPoint],
        ride_id: Option<&Uuid>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let ride_id_str = ride_id.map(|id| id.to_string());

        for point in points {
            self.conn
                .execute(
                    r#"
                    INSERT INTO pdc_points (user_id, duration_secs, power_watts, achieved_at, ride_id, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?4)
                    ON CONFLICT(user_id, duration_secs) DO UPDATE SET
                        power_watts = CASE WHEN excluded.power_watts > power_watts THEN excluded.power_watts ELSE power_watts END,
                        achieved_at = CASE WHEN excluded.power_watts > power_watts THEN excluded.achieved_at ELSE achieved_at END,
                        ride_id = CASE WHEN excluded.power_watts > power_watts THEN excluded.ride_id ELSE ride_id END
                    "#,
                    params![
                        user_id.to_string(),
                        point.duration_secs,
                        point.power_watts,
                        now,
                        ride_id_str,
                    ],
                )
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        }

        Ok(())
    }

    // ========== CP Model Storage (T011) ==========

    /// Load the current CP model for a user.
    pub fn load_current_cp_model(&self, user_id: &Uuid) -> Result<Option<CpModel>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT cp_watts, w_prime_joules, r_squared FROM cp_models
             WHERE user_id = ?1 AND is_current = 1",
            params![user_id.to_string()],
            |row| {
                Ok(CpModel {
                    cp: row.get(0)?,
                    w_prime: row.get(1)?,
                    r_squared: row.get(2)?,
                })
            },
        );

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save a new CP model for a user (marks as current, unsets previous).
    pub fn save_cp_model(&self, user_id: &Uuid, model: &CpModel) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        // Unset previous current model
        self.conn
            .execute(
                "UPDATE cp_models SET is_current = 0 WHERE user_id = ?1 AND is_current = 1",
                params![user_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Insert new model
        self.conn
            .execute(
                "INSERT INTO cp_models (id, user_id, cp_watts, w_prime_joules, r_squared, calculated_at, is_current, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?6)",
                params![
                    id.to_string(),
                    user_id.to_string(),
                    model.cp,
                    model.w_prime,
                    model.r_squared,
                    now,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ========== FTP Estimate Storage (T012) ==========

    /// Load the most recently accepted FTP for a user.
    pub fn load_accepted_ftp(&self, user_id: &Uuid) -> Result<Option<u16>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT ftp_watts FROM ftp_estimates
             WHERE user_id = ?1 AND accepted = 1
             ORDER BY accepted_at DESC LIMIT 1",
            params![user_id.to_string()],
            |row| row.get(0),
        );

        match result {
            Ok(ftp) => Ok(Some(ftp)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save a new FTP estimate (not yet accepted).
    pub fn save_ftp_estimate(
        &self,
        user_id: &Uuid,
        estimate: &FtpEstimate,
    ) -> Result<Uuid, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        let method = match estimate.method {
            FtpMethod::TwentyMinute => "twenty_minute",
            FtpMethod::ExtendedDuration => "extended_duration",
            FtpMethod::CriticalPower => "critical_power",
        };

        let confidence = match estimate.confidence {
            FtpConfidence::High => "high",
            FtpConfidence::Medium => "medium",
            FtpConfidence::Low => "low",
        };

        let supporting_data_json = serde_json::to_string(&estimate.supporting_data)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO ftp_estimates (id, user_id, ftp_watts, method, confidence, supporting_data_json, detected_at, accepted, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?7)",
                params![
                    id.to_string(),
                    user_id.to_string(),
                    estimate.ftp_watts,
                    method,
                    confidence,
                    supporting_data_json,
                    now,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(id)
    }

    /// Accept an FTP estimate (user confirms the detected FTP).
    pub fn accept_ftp_estimate(&self, estimate_id: &Uuid) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();

        let rows_affected = self
            .conn
            .execute(
                "UPDATE ftp_estimates SET accepted = 1, accepted_at = ?2 WHERE id = ?1",
                params![estimate_id.to_string(), now],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!(
                "FTP estimate {}",
                estimate_id
            )));
        }

        Ok(())
    }

    // ========== Training Load Storage (T013) ==========

    /// Load daily training load for a specific date.
    pub fn load_daily_load(
        &self,
        user_id: &Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyLoad>, DatabaseError> {
        let date_str = date.format("%Y-%m-%d").to_string();

        let result = self.conn.query_row(
            "SELECT tss, atl, ctl, tsb FROM daily_training_load
             WHERE user_id = ?1 AND date = ?2",
            params![user_id.to_string(), date_str],
            |row| {
                Ok(DailyLoad {
                    tss: row.get(0)?,
                    atl: row.get(1)?,
                    ctl: row.get(2)?,
                    tsb: row.get(3)?,
                })
            },
        );

        match result {
            Ok(load) => Ok(Some(load)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Load training load history for a date range.
    pub fn load_training_load_history(
        &self,
        user_id: &Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, DailyLoad)>, DatabaseError> {
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let end_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT date, tss, atl, ctl, tsb FROM daily_training_load
                 WHERE user_id = ?1 AND date >= ?2 AND date <= ?3
                 ORDER BY date",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string(), start_str, end_str], |row| {
                let date_str: String = row.get(0)?;
                Ok((
                    date_str,
                    DailyLoad {
                        tss: row.get(1)?,
                        atl: row.get(2)?,
                        ctl: row.get(3)?,
                        tsb: row.get(4)?,
                    },
                ))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (date_str, load) = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            results.push((date, load));
        }

        Ok(results)
    }

    /// Save or update daily training load.
    pub fn save_daily_load(
        &self,
        user_id: &Uuid,
        date: NaiveDate,
        load: &DailyLoad,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let date_str = date.format("%Y-%m-%d").to_string();

        self.conn
            .execute(
                r#"
                INSERT INTO daily_training_load (user_id, date, tss, atl, ctl, tsb, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(user_id, date) DO UPDATE SET
                    tss = excluded.tss,
                    atl = excluded.atl,
                    ctl = excluded.ctl,
                    tsb = excluded.tsb
                "#,
                params![
                    user_id.to_string(),
                    date_str,
                    load.tss,
                    load.atl,
                    load.ctl,
                    load.tsb,
                    now,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ========== VO2max Storage (T014) ==========

    /// Load the current VO2max estimate for a user.
    pub fn load_current_vo2max(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<Vo2maxResult>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT vo2max, method, classification FROM vo2max_estimates
             WHERE user_id = ?1 AND is_current = 1",
            params![user_id.to_string()],
            |row| {
                let vo2max: f32 = row.get(0)?;
                let method_str: String = row.get(1)?;
                let classification_str: String = row.get(2)?;
                Ok((vo2max, method_str, classification_str))
            },
        );

        match result {
            Ok((vo2max, method_str, classification_str)) => {
                let method = match method_str.as_str() {
                    "five_minute_power" => Vo2maxMethod::FiveMinutePower,
                    "ftp_based" => Vo2maxMethod::FtpBased,
                    "critical_power_based" => Vo2maxMethod::CriticalPowerBased,
                    _ => Vo2maxMethod::FtpBased,
                };

                let classification = match classification_str.as_str() {
                    "untrained" => FitnessLevel::Untrained,
                    "recreational" => FitnessLevel::Recreational,
                    "trained" => FitnessLevel::Trained,
                    "well_trained" => FitnessLevel::WellTrained,
                    "elite" => FitnessLevel::Elite,
                    "world_class" => FitnessLevel::WorldClass,
                    _ => FitnessLevel::Recreational,
                };

                Ok(Some(Vo2maxResult {
                    vo2max,
                    method,
                    classification,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save a new VO2max estimate (marks as current, unsets previous).
    pub fn save_vo2max(&self, user_id: &Uuid, result: &Vo2maxResult) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        let method = match result.method {
            Vo2maxMethod::FiveMinutePower => "five_minute_power",
            Vo2maxMethod::FtpBased => "ftp_based",
            Vo2maxMethod::CriticalPowerBased => "critical_power_based",
        };

        let classification = match result.classification {
            FitnessLevel::Untrained => "untrained",
            FitnessLevel::Recreational => "recreational",
            FitnessLevel::Trained => "trained",
            FitnessLevel::WellTrained => "well_trained",
            FitnessLevel::Elite => "elite",
            FitnessLevel::WorldClass => "world_class",
        };

        // Unset previous current
        self.conn
            .execute(
                "UPDATE vo2max_estimates SET is_current = 0 WHERE user_id = ?1 AND is_current = 1",
                params![user_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Insert new
        self.conn
            .execute(
                "INSERT INTO vo2max_estimates (id, user_id, vo2max, method, classification, estimated_at, is_current, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?6)",
                params![
                    id.to_string(),
                    user_id.to_string(),
                    result.vo2max,
                    method,
                    classification,
                    now,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ========== Rider Profile Storage (T015) ==========

    /// Load the current rider profile for a user.
    pub fn load_rider_profile(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<(RiderType, PowerProfile)>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT rider_type, neuromuscular_pct, anaerobic_pct, vo2max_pct, threshold_pct
             FROM rider_profiles WHERE user_id = ?1 AND is_current = 1",
            params![user_id.to_string()],
            |row| {
                let rider_type_str: String = row.get(0)?;
                Ok((
                    rider_type_str,
                    PowerProfile {
                        neuromuscular: row.get(1)?,
                        anaerobic: row.get(2)?,
                        vo2max: row.get(3)?,
                        threshold: row.get(4)?,
                    },
                ))
            },
        );

        match result {
            Ok((rider_type_str, profile)) => {
                let rider_type = match rider_type_str.as_str() {
                    "sprinter" => RiderType::Sprinter,
                    "pursuiter" => RiderType::Pursuiter,
                    "time_trialist" => RiderType::TimeTrialist,
                    "all_rounder" => RiderType::AllRounder,
                    _ => RiderType::Unknown,
                };
                Ok(Some((rider_type, profile)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save a new rider profile (marks as current).
    pub fn save_rider_profile(
        &self,
        user_id: &Uuid,
        rider_type: RiderType,
        profile: &PowerProfile,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4();

        let rider_type_str = match rider_type {
            RiderType::Sprinter => "sprinter",
            RiderType::Pursuiter => "pursuiter",
            RiderType::TimeTrialist => "time_trialist",
            RiderType::AllRounder => "all_rounder",
            RiderType::Unknown => "unknown",
        };

        // Delete previous current (UNIQUE constraint on user_id, is_current)
        self.conn
            .execute(
                "DELETE FROM rider_profiles WHERE user_id = ?1 AND is_current = 1",
                params![user_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Insert new
        self.conn
            .execute(
                "INSERT INTO rider_profiles (id, user_id, rider_type, neuromuscular_pct, anaerobic_pct, vo2max_pct, threshold_pct, calculated_at, is_current, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?8)",
                params![
                    id.to_string(),
                    user_id.to_string(),
                    rider_type_str,
                    profile.neuromuscular,
                    profile.anaerobic,
                    profile.vo2max,
                    profile.threshold,
                    now,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ========== Aggregate Queries (T016, T017) ==========

    /// Aggregate daily TSS from rides for a date range.
    pub fn aggregate_daily_tss(
        &self,
        user_id: &Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, f32)>, DatabaseError> {
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let end_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT date(started_at) as ride_date, SUM(COALESCE(tss, 0)) as total_tss
                FROM rides
                WHERE user_id = ?1
                  AND date(started_at) >= ?2
                  AND date(started_at) <= ?3
                GROUP BY date(started_at)
                ORDER BY ride_date
                "#,
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string(), start_str, end_str], |row| {
                let date_str: String = row.get(0)?;
                let tss: f32 = row.get(1)?;
                Ok((date_str, tss))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (date_str, tss) = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            results.push((date, tss));
        }

        Ok(results)
    }

    /// Load power samples for a ride (for MMP extraction).
    pub fn load_ride_power_samples(&self, ride_id: &Uuid) -> Result<Vec<u16>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT COALESCE(power_watts, 0) FROM ride_samples
                 WHERE ride_id = ?1 ORDER BY elapsed_seconds",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string()], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut samples = Vec::new();
        for row in rows {
            samples.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }

        Ok(samples)
    }

    /// Get the most recent daily load for calculating next day's values.
    pub fn load_most_recent_daily_load(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<(NaiveDate, DailyLoad)>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT date, tss, atl, ctl, tsb FROM daily_training_load
             WHERE user_id = ?1
             ORDER BY date DESC LIMIT 1",
            params![user_id.to_string()],
            |row| {
                let date_str: String = row.get(0)?;
                Ok((
                    date_str,
                    DailyLoad {
                        tss: row.get(1)?,
                        atl: row.get(2)?,
                        ctl: row.get(3)?,
                        tsb: row.get(4)?,
                    },
                ))
            },
        );

        match result {
            Ok((date_str, load)) => {
                let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                Ok(Some((date, load)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::Database;

    fn setup_db() -> Database {
        Database::open_in_memory().expect("Failed to create test database")
    }

    fn create_test_user(db: &Database) -> Uuid {
        let user_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        db.connection()
            .execute(
                "INSERT INTO users (id, name, ftp, weight_kg, power_zones_json, units, theme, created_at, updated_at)
                 VALUES (?1, 'Test User', 250, 70.0, '{}', 'metric', 'dark', ?2, ?2)",
                params![user_id.to_string(), now],
            )
            .expect("Failed to create test user");
        user_id
    }

    #[test]
    fn test_pdc_save_and_load() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        let points = vec![
            PdcPoint {
                duration_secs: 60,
                power_watts: 400,
            },
            PdcPoint {
                duration_secs: 300,
                power_watts: 320,
            },
            PdcPoint {
                duration_secs: 1200,
                power_watts: 280,
            },
        ];

        store
            .save_pdc_points(&user_id, &points, None)
            .expect("Failed to save PDC points");

        let pdc = store.load_pdc(&user_id).expect("Failed to load PDC");
        assert_eq!(pdc.len(), 3);
        assert_eq!(pdc.power_at(60), Some(400));
        assert_eq!(pdc.power_at(300), Some(320));
        assert_eq!(pdc.power_at(1200), Some(280));
    }

    #[test]
    fn test_pdc_update_only_improves() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        // Initial save
        store
            .save_pdc_points(
                &user_id,
                &[PdcPoint {
                    duration_secs: 60,
                    power_watts: 400,
                }],
                None,
            )
            .expect("Failed to save");

        // Try to save lower power - should not update
        store
            .save_pdc_points(
                &user_id,
                &[PdcPoint {
                    duration_secs: 60,
                    power_watts: 350,
                }],
                None,
            )
            .expect("Failed to save");

        let pdc = store.load_pdc(&user_id).expect("Failed to load");
        assert_eq!(pdc.power_at(60), Some(400)); // Still 400

        // Save higher power - should update
        store
            .save_pdc_points(
                &user_id,
                &[PdcPoint {
                    duration_secs: 60,
                    power_watts: 450,
                }],
                None,
            )
            .expect("Failed to save");

        let pdc = store.load_pdc(&user_id).expect("Failed to load");
        assert_eq!(pdc.power_at(60), Some(450)); // Now 450
    }

    #[test]
    fn test_cp_model_save_and_load() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        let model = CpModel {
            cp: 250,
            w_prime: 20000,
            r_squared: 0.98,
        };

        store
            .save_cp_model(&user_id, &model)
            .expect("Failed to save CP model");

        let loaded = store
            .load_current_cp_model(&user_id)
            .expect("Failed to load")
            .expect("No model found");

        assert_eq!(loaded.cp, 250);
        assert_eq!(loaded.w_prime, 20000);
        assert!((loaded.r_squared - 0.98).abs() < 0.01);
    }

    #[test]
    fn test_daily_load_save_and_load() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let load = DailyLoad {
            tss: 75.0,
            atl: 60.0,
            ctl: 50.0,
            tsb: -10.0,
        };

        store
            .save_daily_load(&user_id, date, &load)
            .expect("Failed to save");

        let loaded = store
            .load_daily_load(&user_id, date)
            .expect("Failed to load")
            .expect("No load found");

        assert!((loaded.tss - 75.0).abs() < 0.1);
        assert!((loaded.atl - 60.0).abs() < 0.1);
        assert!((loaded.ctl - 50.0).abs() < 0.1);
        assert!((loaded.tsb - -10.0).abs() < 0.1);
    }

    #[test]
    fn test_vo2max_save_and_load() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        let result = Vo2maxResult {
            vo2max: 55.0,
            method: Vo2maxMethod::FtpBased,
            classification: FitnessLevel::Trained,
        };

        store
            .save_vo2max(&user_id, &result)
            .expect("Failed to save");

        let loaded = store
            .load_current_vo2max(&user_id)
            .expect("Failed to load")
            .expect("No result found");

        assert!((loaded.vo2max - 55.0).abs() < 0.1);
        assert_eq!(loaded.method, Vo2maxMethod::FtpBased);
        assert_eq!(loaded.classification, FitnessLevel::Trained);
    }

    #[test]
    fn test_rider_profile_save_and_load() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let store = AnalyticsStore::new(db.connection());

        let profile = PowerProfile {
            neuromuscular: 180.0,
            anaerobic: 125.0,
            vo2max: 95.0,
            threshold: 100.0,
        };

        store
            .save_rider_profile(&user_id, RiderType::AllRounder, &profile)
            .expect("Failed to save");

        let (loaded_type, loaded_profile) = store
            .load_rider_profile(&user_id)
            .expect("Failed to load")
            .expect("No profile found");

        assert_eq!(loaded_type, RiderType::AllRounder);
        assert!((loaded_profile.neuromuscular - 180.0).abs() < 0.1);
        assert!((loaded_profile.anaerobic - 125.0).abs() < 0.1);
    }
}
