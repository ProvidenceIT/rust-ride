//! Database operations using rusqlite.
//!
//! T009: Implement Database struct with connection and migration
//! T080: Implement workout CRUD operations
//! T099: Implement ride CRUD in database
//! T100: Implement ride_samples bulk insert
//! T115: Implement UserProfile CRUD in database
//! T145: Implement sensor CRUD in database

use crate::metrics::zones::{HRZones, PowerZones};
use crate::recording::types::{Ride, RideSample};
use crate::sensors::types::{Protocol, SavedSensor, SensorType};
use crate::storage::config::{Theme, Units, UserProfile};
use crate::storage::schema::{CURRENT_VERSION, SCHEMA, SCHEMA_VERSION_TABLE};
use crate::workouts::types::{Workout, WorkoutFormat, WorkoutSegment};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Database wrapper for SQLite operations.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path.
    pub fn open(path: &PathBuf) -> Result<Self, DatabaseError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DatabaseError::IoError(e.to_string()))?;
        }

        let conn =
            Connection::open(path).map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Initialize the database schema.
    fn initialize(&self) -> Result<(), DatabaseError> {
        // Create schema version table
        self.conn
            .execute_batch(SCHEMA_VERSION_TABLE)
            .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

        // Check current version
        let current_version = self.get_schema_version()?;

        if current_version < CURRENT_VERSION {
            self.migrate(current_version)?;
        }

        Ok(())
    }

    /// Get the current schema version.
    fn get_schema_version(&self) -> Result<i32, DatabaseError> {
        let result: SqliteResult<i32> = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(version) => Ok(version),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Run database migrations.
    fn migrate(&self, from_version: i32) -> Result<(), DatabaseError> {
        if from_version < 1 {
            // Initial schema
            self.conn
                .execute_batch(SCHEMA)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?, datetime('now'))",
                    [CURRENT_VERSION],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version {}", CURRENT_VERSION);
        }

        // Future migrations would go here:
        // if from_version < 2 { ... }

        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Execute a query and return the number of rows affected.
    pub fn execute(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<usize, DatabaseError> {
        self.conn
            .execute(sql, params)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))
    }

    /// Begin a transaction.
    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, DatabaseError> {
        self.conn
            .transaction()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))
    }

    // ========== Workout CRUD Operations (T080) ==========

    /// Insert a new workout into the database.
    pub fn insert_workout(&self, workout: &Workout) -> Result<(), DatabaseError> {
        let segments_json = serde_json::to_string(&workout.segments)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let tags_json = if workout.tags.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&workout.tags)
                    .map_err(|e| DatabaseError::SerializationError(e.to_string()))?,
            )
        };

        let source_format = workout
            .source_format
            .map(|f| format!("{:?}", f).to_lowercase());

        self.conn
            .execute(
                "INSERT INTO workouts (id, name, description, author, source_file, source_format,
                 segments_json, total_duration_seconds, estimated_tss, estimated_if, tags_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    workout.id.to_string(),
                    workout.name,
                    workout.description,
                    workout.author,
                    workout.source_file,
                    source_format,
                    segments_json,
                    workout.total_duration_seconds,
                    workout.estimated_tss,
                    workout.estimated_if,
                    tags_json,
                    workout.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a workout by ID.
    pub fn get_workout(&self, id: &Uuid) -> Result<Option<Workout>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description, author, source_file, source_format,
                 segments_json, total_duration_seconds, estimated_tss, estimated_if,
                 tags_json, created_at FROM workouts WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(WorkoutRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                author: row.get(3)?,
                source_file: row.get(4)?,
                source_format: row.get(5)?,
                segments_json: row.get(6)?,
                total_duration_seconds: row.get(7)?,
                estimated_tss: row.get(8)?,
                estimated_if: row.get(9)?,
                tags_json: row.get(10)?,
                created_at: row.get(11)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_workout()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all workouts, optionally filtered by search query.
    pub fn list_workouts(&self, search: Option<&str>) -> Result<Vec<Workout>, DatabaseError> {
        let sql = match search {
            Some(_) => {
                "SELECT id, name, description, author, source_file, source_format,
                 segments_json, total_duration_seconds, estimated_tss, estimated_if,
                 tags_json, created_at FROM workouts
                 WHERE name LIKE ?1 OR description LIKE ?1
                 ORDER BY created_at DESC"
            }
            None => {
                "SELECT id, name, description, author, source_file, source_format,
                 segments_json, total_duration_seconds, estimated_tss, estimated_if,
                 tags_json, created_at FROM workouts ORDER BY created_at DESC"
            }
        };

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<WorkoutRow> {
            Ok(WorkoutRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                author: row.get(3)?,
                source_file: row.get(4)?,
                source_format: row.get(5)?,
                segments_json: row.get(6)?,
                total_duration_seconds: row.get(7)?,
                estimated_tss: row.get(8)?,
                estimated_if: row.get(9)?,
                tags_json: row.get(10)?,
                created_at: row.get(11)?,
            })
        };

        let mut workouts = Vec::new();

        if let Some(query) = search {
            let pattern = format!("%{}%", query);
            let rows = stmt
                .query_map(params![pattern], map_row)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            for row in rows {
                let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
                workouts.push(row.into_workout()?);
            }
        } else {
            let rows = stmt
                .query_map([], map_row)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            for row in rows {
                let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
                workouts.push(row.into_workout()?);
            }
        }

        Ok(workouts)
    }

    /// Update an existing workout.
    pub fn update_workout(&self, workout: &Workout) -> Result<(), DatabaseError> {
        let segments_json = serde_json::to_string(&workout.segments)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let tags_json = if workout.tags.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&workout.tags)
                    .map_err(|e| DatabaseError::SerializationError(e.to_string()))?,
            )
        };

        let source_format = workout
            .source_format
            .map(|f| format!("{:?}", f).to_lowercase());

        let rows_affected = self
            .conn
            .execute(
                "UPDATE workouts SET name = ?2, description = ?3, author = ?4,
                 source_file = ?5, source_format = ?6, segments_json = ?7,
                 total_duration_seconds = ?8, estimated_tss = ?9, estimated_if = ?10,
                 tags_json = ?11 WHERE id = ?1",
                params![
                    workout.id.to_string(),
                    workout.name,
                    workout.description,
                    workout.author,
                    workout.source_file,
                    source_format,
                    segments_json,
                    workout.total_duration_seconds,
                    workout.estimated_tss,
                    workout.estimated_if,
                    tags_json,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Workout {}", workout.id)));
        }

        Ok(())
    }

    /// Delete a workout by ID.
    pub fn delete_workout(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM workouts WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Workout {}", id)));
        }

        Ok(())
    }

    /// Count workouts in the database.
    pub fn count_workouts(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ========== Ride CRUD Operations (T099, T100) ==========

    /// Insert a new ride into the database.
    pub fn insert_ride(&self, ride: &Ride) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO rides (id, user_id, workout_id, started_at, ended_at,
                 duration_seconds, distance_meters, avg_power, max_power, normalized_power,
                 intensity_factor, tss, avg_hr, max_hr, avg_cadence, calories, ftp_at_ride,
                 notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    ride.id.to_string(),
                    ride.user_id.to_string(),
                    ride.workout_id.map(|id| id.to_string()),
                    ride.started_at.to_rfc3339(),
                    ride.ended_at.map(|dt| dt.to_rfc3339()),
                    ride.duration_seconds,
                    ride.distance_meters,
                    ride.avg_power,
                    ride.max_power,
                    ride.normalized_power,
                    ride.intensity_factor,
                    ride.tss,
                    ride.avg_hr,
                    ride.max_hr,
                    ride.avg_cadence,
                    ride.calories,
                    ride.ftp_at_ride,
                    ride.notes,
                    ride.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Insert ride samples in bulk.
    pub fn insert_ride_samples(
        &mut self,
        ride_id: &Uuid,
        samples: &[RideSample],
    ) -> Result<(), DatabaseError> {
        if samples.is_empty() {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO ride_samples (ride_id, elapsed_seconds, power_watts, cadence_rpm,
                     heart_rate_bpm, speed_kmh, distance_meters, calories, resistance_level,
                     target_power, trainer_grade)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                )
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            for sample in samples {
                stmt.execute(params![
                    ride_id.to_string(),
                    sample.elapsed_seconds,
                    sample.power_watts,
                    sample.cadence_rpm,
                    sample.heart_rate_bpm,
                    sample.speed_kmh,
                    sample.distance_meters,
                    sample.calories,
                    sample.resistance_level,
                    sample.target_power,
                    sample.trainer_grade,
                ])
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a ride by ID.
    pub fn get_ride(&self, id: &Uuid) -> Result<Option<Ride>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, workout_id, started_at, ended_at, duration_seconds,
                 distance_meters, avg_power, max_power, normalized_power, intensity_factor,
                 tss, avg_hr, max_hr, avg_cadence, calories, ftp_at_ride, notes, created_at
                 FROM rides WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(RideRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                workout_id: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                duration_seconds: row.get(5)?,
                distance_meters: row.get(6)?,
                avg_power: row.get(7)?,
                max_power: row.get(8)?,
                normalized_power: row.get(9)?,
                intensity_factor: row.get(10)?,
                tss: row.get(11)?,
                avg_hr: row.get(12)?,
                max_hr: row.get(13)?,
                avg_cadence: row.get(14)?,
                calories: row.get(15)?,
                ftp_at_ride: row.get(16)?,
                notes: row.get(17)?,
                created_at: row.get(18)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_ride()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get ride samples by ride ID.
    pub fn get_ride_samples(&self, ride_id: &Uuid) -> Result<Vec<RideSample>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT elapsed_seconds, power_watts, cadence_rpm, heart_rate_bpm,
                 speed_kmh, distance_meters, calories, resistance_level, target_power, trainer_grade
                 FROM ride_samples WHERE ride_id = ?1 ORDER BY elapsed_seconds",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![ride_id.to_string()], |row| {
                Ok(RideSample {
                    elapsed_seconds: row.get(0)?,
                    power_watts: row.get(1)?,
                    cadence_rpm: row.get(2)?,
                    heart_rate_bpm: row.get(3)?,
                    speed_kmh: row.get(4)?,
                    distance_meters: row.get(5)?,
                    calories: row.get(6)?,
                    resistance_level: row.get(7)?,
                    target_power: row.get(8)?,
                    trainer_grade: row.get(9)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut samples = Vec::new();
        for row in rows {
            samples.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }

        Ok(samples)
    }

    /// Get a ride with its samples.
    pub fn get_ride_with_samples(
        &self,
        id: &Uuid,
    ) -> Result<Option<(Ride, Vec<RideSample>)>, DatabaseError> {
        let ride = self.get_ride(id)?;
        match ride {
            Some(ride) => {
                let samples = self.get_ride_samples(id)?;
                Ok(Some((ride, samples)))
            }
            None => Ok(None),
        }
    }

    /// List all rides for a user, ordered by date descending.
    pub fn list_rides(
        &self,
        user_id: &Uuid,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Ride>, DatabaseError> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, workout_id, started_at, ended_at, duration_seconds,
                 distance_meters, avg_power, max_power, normalized_power, intensity_factor,
                 tss, avg_hr, max_hr, avg_cadence, calories, ftp_at_ride, notes, created_at
                 FROM rides WHERE user_id = ?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string(), limit, offset], |row| {
                Ok(RideRow {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    workout_id: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    duration_seconds: row.get(5)?,
                    distance_meters: row.get(6)?,
                    avg_power: row.get(7)?,
                    max_power: row.get(8)?,
                    normalized_power: row.get(9)?,
                    intensity_factor: row.get(10)?,
                    tss: row.get(11)?,
                    avg_hr: row.get(12)?,
                    max_hr: row.get(13)?,
                    avg_cadence: row.get(14)?,
                    calories: row.get(15)?,
                    ftp_at_ride: row.get(16)?,
                    notes: row.get(17)?,
                    created_at: row.get(18)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rides = Vec::new();
        for row in rows {
            let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            rides.push(row.into_ride()?);
        }

        Ok(rides)
    }

    /// Delete a ride by ID (cascades to samples).
    pub fn delete_ride(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM rides WHERE id = ?1", params![id.to_string()])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Ride {}", id)));
        }

        Ok(())
    }

    /// Count rides for a user.
    pub fn count_rides(&self, user_id: &Uuid) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM rides WHERE user_id = ?1",
                params![user_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ========== Autosave Operations ==========

    /// Save ride data for crash recovery.
    pub fn save_autosave(&self, ride: &Ride, samples: &[RideSample]) -> Result<(), DatabaseError> {
        let ride_json = serde_json::to_string(ride)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        let samples_json = serde_json::to_string(samples)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO autosave (id, ride_json, samples_json, saved_at)
                 VALUES (1, ?1, ?2, datetime('now'))",
                params![ride_json, samples_json],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Check if autosave data exists.
    pub fn has_autosave(&self) -> Result<bool, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM autosave", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count > 0)
    }

    /// Load autosave data for crash recovery.
    pub fn load_autosave(&self) -> Result<Option<(Ride, Vec<RideSample>)>, DatabaseError> {
        let result: Result<(String, String), _> = self.conn.query_row(
            "SELECT ride_json, samples_json FROM autosave WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((ride_json, samples_json)) => {
                let ride: Ride = serde_json::from_str(&ride_json)
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let samples: Vec<RideSample> = serde_json::from_str(&samples_json)
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                Ok(Some((ride, samples)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Clear autosave data.
    pub fn clear_autosave(&self) -> Result<(), DatabaseError> {
        self.conn
            .execute("DELETE FROM autosave", [])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ========== User Profile CRUD Operations (T115) ==========

    /// Insert a new user profile into the database.
    pub fn insert_user(&self, profile: &UserProfile) -> Result<(), DatabaseError> {
        let power_zones_json = serde_json::to_string(&profile.power_zones)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let hr_zones_json = profile
            .hr_zones
            .as_ref()
            .map(|zones| {
                serde_json::to_string(zones)
                    .map_err(|e| DatabaseError::SerializationError(e.to_string()))
            })
            .transpose()?;

        self.conn
            .execute(
                "INSERT INTO users (id, name, ftp, max_hr, resting_hr, weight_kg, height_cm,
                 power_zones_json, hr_zones_json, units, theme, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    profile.id.to_string(),
                    profile.name,
                    profile.ftp,
                    profile.max_hr,
                    profile.resting_hr,
                    profile.weight_kg,
                    profile.height_cm,
                    power_zones_json,
                    hr_zones_json,
                    format!("{:?}", profile.units).to_lowercase(),
                    format!("{:?}", profile.theme).to_lowercase(),
                    profile.created_at.to_rfc3339(),
                    profile.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a user profile by ID.
    pub fn get_user(&self, id: &Uuid) -> Result<Option<UserProfile>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, ftp, max_hr, resting_hr, weight_kg, height_cm,
                 power_zones_json, hr_zones_json, units, theme, created_at, updated_at
                 FROM users WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(UserProfileRow {
                id: row.get(0)?,
                name: row.get(1)?,
                ftp: row.get(2)?,
                max_hr: row.get(3)?,
                resting_hr: row.get(4)?,
                weight_kg: row.get(5)?,
                height_cm: row.get(6)?,
                power_zones_json: row.get(7)?,
                hr_zones_json: row.get(8)?,
                units: row.get(9)?,
                theme: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_user_profile()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all user profiles.
    pub fn list_users(&self) -> Result<Vec<UserProfile>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, ftp, max_hr, resting_hr, weight_kg, height_cm,
                 power_zones_json, hr_zones_json, units, theme, created_at, updated_at
                 FROM users ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(UserProfileRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    ftp: row.get(2)?,
                    max_hr: row.get(3)?,
                    resting_hr: row.get(4)?,
                    weight_kg: row.get(5)?,
                    height_cm: row.get(6)?,
                    power_zones_json: row.get(7)?,
                    hr_zones_json: row.get(8)?,
                    units: row.get(9)?,
                    theme: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut profiles = Vec::new();
        for row in rows {
            let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            profiles.push(row.into_user_profile()?);
        }

        Ok(profiles)
    }

    /// Get the first (default) user profile.
    pub fn get_default_user(&self) -> Result<Option<UserProfile>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, ftp, max_hr, resting_hr, weight_kg, height_cm,
                 power_zones_json, hr_zones_json, units, theme, created_at, updated_at
                 FROM users ORDER BY created_at ASC LIMIT 1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row([], |row| {
            Ok(UserProfileRow {
                id: row.get(0)?,
                name: row.get(1)?,
                ftp: row.get(2)?,
                max_hr: row.get(3)?,
                resting_hr: row.get(4)?,
                weight_kg: row.get(5)?,
                height_cm: row.get(6)?,
                power_zones_json: row.get(7)?,
                hr_zones_json: row.get(8)?,
                units: row.get(9)?,
                theme: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_user_profile()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Update an existing user profile.
    pub fn update_user(&self, profile: &UserProfile) -> Result<(), DatabaseError> {
        let power_zones_json = serde_json::to_string(&profile.power_zones)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let hr_zones_json = profile
            .hr_zones
            .as_ref()
            .map(|zones| {
                serde_json::to_string(zones)
                    .map_err(|e| DatabaseError::SerializationError(e.to_string()))
            })
            .transpose()?;

        let rows_affected = self
            .conn
            .execute(
                "UPDATE users SET name = ?2, ftp = ?3, max_hr = ?4, resting_hr = ?5,
                 weight_kg = ?6, height_cm = ?7, power_zones_json = ?8, hr_zones_json = ?9,
                 units = ?10, theme = ?11, updated_at = ?12 WHERE id = ?1",
                params![
                    profile.id.to_string(),
                    profile.name,
                    profile.ftp,
                    profile.max_hr,
                    profile.resting_hr,
                    profile.weight_kg,
                    profile.height_cm,
                    power_zones_json,
                    hr_zones_json,
                    format!("{:?}", profile.units).to_lowercase(),
                    format!("{:?}", profile.theme).to_lowercase(),
                    profile.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("User {}", profile.id)));
        }

        Ok(())
    }

    /// Delete a user profile by ID.
    pub fn delete_user(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM users WHERE id = ?1", params![id.to_string()])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("User {}", id)));
        }

        Ok(())
    }

    /// Count users in the database.
    pub fn count_users(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Get or create a default user profile.
    pub fn get_or_create_default_user(&self) -> Result<UserProfile, DatabaseError> {
        if let Some(user) = self.get_default_user()? {
            return Ok(user);
        }

        // Create a default user
        let profile = UserProfile::default();
        self.insert_user(&profile)?;
        Ok(profile)
    }

    // ========== Sensor CRUD Operations (T145) ==========

    /// Insert a new saved sensor into the database.
    pub fn insert_sensor(&self, sensor: &SavedSensor) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO sensors (id, user_id, device_id, name, sensor_type, protocol,
                 last_seen_at, is_primary, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    sensor.id.to_string(),
                    sensor.user_id.to_string(),
                    sensor.device_id,
                    sensor.name,
                    format!("{:?}", sensor.sensor_type).to_lowercase(),
                    format!("{:?}", sensor.protocol).to_lowercase(),
                    sensor.last_seen_at.map(|dt| dt.to_rfc3339()),
                    sensor.is_primary as i32,
                    sensor.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a sensor by ID.
    pub fn get_sensor(&self, id: &Uuid) -> Result<Option<SavedSensor>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, device_id, name, sensor_type, protocol,
                 last_seen_at, is_primary, created_at FROM sensors WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(SensorRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                device_id: row.get(2)?,
                name: row.get(3)?,
                sensor_type: row.get(4)?,
                protocol: row.get(5)?,
                last_seen_at: row.get(6)?,
                is_primary: row.get(7)?,
                created_at: row.get(8)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_saved_sensor()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get a sensor by device ID for a user.
    pub fn get_sensor_by_device_id(
        &self,
        user_id: &Uuid,
        device_id: &str,
    ) -> Result<Option<SavedSensor>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, device_id, name, sensor_type, protocol,
                 last_seen_at, is_primary, created_at FROM sensors
                 WHERE user_id = ?1 AND device_id = ?2",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![user_id.to_string(), device_id], |row| {
            Ok(SensorRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                device_id: row.get(2)?,
                name: row.get(3)?,
                sensor_type: row.get(4)?,
                protocol: row.get(5)?,
                last_seen_at: row.get(6)?,
                is_primary: row.get(7)?,
                created_at: row.get(8)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_saved_sensor()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all sensors for a user.
    pub fn list_sensors(&self, user_id: &Uuid) -> Result<Vec<SavedSensor>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, device_id, name, sensor_type, protocol,
                 last_seen_at, is_primary, created_at FROM sensors
                 WHERE user_id = ?1 ORDER BY is_primary DESC, name ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(SensorRow {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    device_id: row.get(2)?,
                    name: row.get(3)?,
                    sensor_type: row.get(4)?,
                    protocol: row.get(5)?,
                    last_seen_at: row.get(6)?,
                    is_primary: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut sensors = Vec::new();
        for row in rows {
            let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            sensors.push(row.into_saved_sensor()?);
        }

        Ok(sensors)
    }

    /// Update a sensor's last seen timestamp.
    pub fn update_sensor_last_seen(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE sensors SET last_seen_at = ?2 WHERE id = ?1",
                params![id.to_string(), now],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Sensor {}", id)));
        }

        Ok(())
    }

    /// Set a sensor as primary for its type (and unset others).
    pub fn set_sensor_primary(
        &self,
        user_id: &Uuid,
        sensor_id: &Uuid,
        sensor_type: SensorType,
    ) -> Result<(), DatabaseError> {
        let type_str = format!("{:?}", sensor_type).to_lowercase();

        // Unset all sensors of this type as primary
        self.conn
            .execute(
                "UPDATE sensors SET is_primary = 0 WHERE user_id = ?1 AND sensor_type = ?2",
                params![user_id.to_string(), type_str],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Set the specified sensor as primary
        self.conn
            .execute(
                "UPDATE sensors SET is_primary = 1 WHERE id = ?1",
                params![sensor_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Delete a sensor by ID.
    pub fn delete_sensor(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM sensors WHERE id = ?1", params![id.to_string()])
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Sensor {}", id)));
        }

        Ok(())
    }

    /// Count sensors for a user.
    pub fn count_sensors(&self, user_id: &Uuid) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sensors WHERE user_id = ?1",
                params![user_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }
}

/// Intermediate struct for reading workout rows from database.
struct WorkoutRow {
    id: String,
    name: String,
    description: Option<String>,
    author: Option<String>,
    source_file: Option<String>,
    source_format: Option<String>,
    segments_json: String,
    total_duration_seconds: u32,
    estimated_tss: Option<f32>,
    estimated_if: Option<f32>,
    tags_json: Option<String>,
    created_at: String,
}

/// Intermediate struct for reading ride rows from database.
struct RideRow {
    id: String,
    user_id: String,
    workout_id: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    duration_seconds: u32,
    distance_meters: f64,
    avg_power: Option<u16>,
    max_power: Option<u16>,
    normalized_power: Option<u16>,
    intensity_factor: Option<f32>,
    tss: Option<f32>,
    avg_hr: Option<u8>,
    max_hr: Option<u8>,
    avg_cadence: Option<u8>,
    calories: u32,
    ftp_at_ride: u16,
    notes: Option<String>,
    created_at: String,
}

impl RideRow {
    fn into_ride(self) -> Result<Ride, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let user_id = Uuid::parse_str(&self.user_id).map_err(|e| {
            DatabaseError::DeserializationError(format!("Invalid user UUID: {}", e))
        })?;

        let workout_id = self
            .workout_id
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid workout UUID: {}", e))
            })?;

        let started_at = DateTime::parse_from_rfc3339(&self.started_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid start date: {}", e))
            })?;

        let ended_at = self
            .ended_at
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid end date: {}", e)))?;

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid created date: {}", e))
            })?;

        Ok(Ride {
            id,
            user_id,
            workout_id,
            started_at,
            ended_at,
            duration_seconds: self.duration_seconds,
            distance_meters: self.distance_meters,
            avg_power: self.avg_power,
            max_power: self.max_power,
            normalized_power: self.normalized_power,
            intensity_factor: self.intensity_factor,
            tss: self.tss,
            avg_hr: self.avg_hr,
            max_hr: self.max_hr,
            avg_cadence: self.avg_cadence,
            calories: self.calories,
            ftp_at_ride: self.ftp_at_ride,
            notes: self.notes,
            created_at,
        })
    }
}

impl WorkoutRow {
    fn into_workout(self) -> Result<Workout, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let segments: Vec<WorkoutSegment> =
            serde_json::from_str(&self.segments_json).map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid segments JSON: {}", e))
            })?;

        let tags: Vec<String> = match self.tags_json {
            Some(json) => serde_json::from_str(&json).map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid tags JSON: {}", e))
            })?,
            None => Vec::new(),
        };

        let source_format = self
            .source_format
            .and_then(|s| match s.to_lowercase().as_str() {
                "zwo" => Some(WorkoutFormat::Zwo),
                "mrc" => Some(WorkoutFormat::Mrc),
                "fit" => Some(WorkoutFormat::Fit),
                "native" => Some(WorkoutFormat::Native),
                _ => None,
            });

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid date: {}", e)))?;

        Ok(Workout {
            id,
            name: self.name,
            description: self.description,
            author: self.author,
            source_file: self.source_file,
            source_format,
            segments,
            total_duration_seconds: self.total_duration_seconds,
            estimated_tss: self.estimated_tss,
            estimated_if: self.estimated_if,
            tags,
            created_at,
        })
    }
}

/// Intermediate struct for reading user profile rows from database.
struct UserProfileRow {
    id: String,
    name: String,
    ftp: u16,
    max_hr: Option<u8>,
    resting_hr: Option<u8>,
    weight_kg: f32,
    height_cm: Option<u16>,
    power_zones_json: String,
    hr_zones_json: Option<String>,
    units: String,
    theme: String,
    created_at: String,
    updated_at: String,
}

impl UserProfileRow {
    fn into_user_profile(self) -> Result<UserProfile, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let power_zones: PowerZones =
            serde_json::from_str(&self.power_zones_json).map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid power zones JSON: {}", e))
            })?;

        let hr_zones: Option<HRZones> = self
            .hr_zones_json
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid HR zones JSON: {}", e))
            })?;

        let units = match self.units.to_lowercase().as_str() {
            "imperial" => Units::Imperial,
            _ => Units::Metric,
        };

        let theme = match self.theme.to_lowercase().as_str() {
            "light" => Theme::Light,
            _ => Theme::Dark,
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid created date: {}", e))
            })?;

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid updated date: {}", e))
            })?;

        Ok(UserProfile {
            id,
            name: self.name,
            ftp: self.ftp,
            max_hr: self.max_hr,
            resting_hr: self.resting_hr,
            weight_kg: self.weight_kg,
            height_cm: self.height_cm,
            power_zones,
            hr_zones,
            units,
            theme,
            created_at,
            updated_at,
        })
    }
}

/// Intermediate struct for reading sensor rows from database.
struct SensorRow {
    id: String,
    user_id: String,
    device_id: String,
    name: String,
    sensor_type: String,
    protocol: String,
    last_seen_at: Option<String>,
    is_primary: i32,
    created_at: String,
}

impl SensorRow {
    fn into_saved_sensor(self) -> Result<SavedSensor, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let user_id = Uuid::parse_str(&self.user_id).map_err(|e| {
            DatabaseError::DeserializationError(format!("Invalid user UUID: {}", e))
        })?;

        let sensor_type = match self.sensor_type.to_lowercase().as_str() {
            "trainer" => SensorType::Trainer,
            "powermeter" => SensorType::PowerMeter,
            "heartrate" => SensorType::HeartRate,
            "cadence" => SensorType::Cadence,
            "speed" => SensorType::Speed,
            "speedcadence" => SensorType::SpeedCadence,
            _ => {
                return Err(DatabaseError::DeserializationError(format!(
                    "Unknown sensor type: {}",
                    self.sensor_type
                )))
            }
        };

        let protocol = match self.protocol.to_lowercase().as_str() {
            "bleftms" => Protocol::BleFtms,
            "blecyclingpower" => Protocol::BleCyclingPower,
            "bleheartrate" => Protocol::BleHeartRate,
            "blecsc" => Protocol::BleCsc,
            _ => {
                return Err(DatabaseError::DeserializationError(format!(
                    "Unknown protocol: {}",
                    self.protocol
                )))
            }
        };

        let last_seen_at = self
            .last_seen_at
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid last_seen_at date: {}", e))
            })?;

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                DatabaseError::DeserializationError(format!("Invalid created_at date: {}", e))
            })?;

        Ok(SavedSensor {
            id,
            user_id,
            device_id: self.device_id,
            name: self.name,
            sensor_type,
            protocol,
            last_seen_at,
            is_primary: self.is_primary != 0,
            created_at,
        })
    }
}

/// Database errors.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::config::UserProfile;
    use crate::workouts::types::{PowerTarget, SegmentType, WorkoutSegment};

    /// Create a test user with the specified ID (for ride foreign key tests).
    fn create_test_user_with_id(user_id: Uuid) -> UserProfile {
        let mut profile = UserProfile::default();
        profile.id = user_id;
        profile.name = "Test User".to_string();
        profile
    }

    #[test]
    fn test_create_in_memory_database() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let version = db.get_schema_version().expect("Failed to get version");
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_tables_created() {
        let db = Database::open_in_memory().expect("Failed to create database");

        // Check that tables exist
        let tables: Vec<String> = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"sensors".to_string()));
        assert!(tables.contains(&"workouts".to_string()));
        assert!(tables.contains(&"rides".to_string()));
        assert!(tables.contains(&"ride_samples".to_string()));
        assert!(tables.contains(&"autosave".to_string()));
    }

    fn create_test_workout(name: &str) -> Workout {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300,
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: Some("Warmup".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 1200,
                power_target: PowerTarget::percent_ftp(90),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::Cooldown,
                duration_seconds: 300,
                power_target: PowerTarget::percent_ftp(40),
                cadence_target: None,
                text_event: None,
            },
        ];

        let mut workout = Workout::new(name.to_string(), segments);
        workout.description = Some("A test workout".to_string());
        workout.author = Some("Test Author".to_string());
        workout.source_format = Some(WorkoutFormat::Zwo);
        workout.tags = vec!["test".to_string(), "threshold".to_string()];
        workout.calculate_estimates(200);
        workout
    }

    #[test]
    fn test_workout_insert_and_get() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let workout = create_test_workout("Test Workout");
        let workout_id = workout.id;

        // Insert workout
        db.insert_workout(&workout)
            .expect("Failed to insert workout");

        // Retrieve workout
        let retrieved = db
            .get_workout(&workout_id)
            .expect("Failed to get workout")
            .expect("Workout not found");

        assert_eq!(retrieved.id, workout.id);
        assert_eq!(retrieved.name, "Test Workout");
        assert_eq!(retrieved.description, Some("A test workout".to_string()));
        assert_eq!(retrieved.author, Some("Test Author".to_string()));
        assert_eq!(retrieved.source_format, Some(WorkoutFormat::Zwo));
        assert_eq!(retrieved.segments.len(), 3);
        assert_eq!(retrieved.tags.len(), 2);
        assert!(retrieved.tags.contains(&"test".to_string()));
        assert!(retrieved.tags.contains(&"threshold".to_string()));
        assert_eq!(retrieved.total_duration_seconds, 1800);
    }

    #[test]
    fn test_workout_list_all() {
        let db = Database::open_in_memory().expect("Failed to create database");

        // Insert multiple workouts
        db.insert_workout(&create_test_workout("Workout One"))
            .unwrap();
        db.insert_workout(&create_test_workout("Workout Two"))
            .unwrap();
        db.insert_workout(&create_test_workout("Workout Three"))
            .unwrap();

        // List all
        let workouts = db.list_workouts(None).expect("Failed to list workouts");
        assert_eq!(workouts.len(), 3);
    }

    #[test]
    fn test_workout_list_with_search() {
        let db = Database::open_in_memory().expect("Failed to create database");

        db.insert_workout(&create_test_workout("Sweet Spot Training"))
            .unwrap();
        db.insert_workout(&create_test_workout("VO2 Max Intervals"))
            .unwrap();
        db.insert_workout(&create_test_workout("Endurance Ride"))
            .unwrap();

        // Search for "spot"
        let workouts = db
            .list_workouts(Some("spot"))
            .expect("Failed to list workouts");
        assert_eq!(workouts.len(), 1);
        assert_eq!(workouts[0].name, "Sweet Spot Training");

        // Search for "ride" (case insensitive via SQL LIKE)
        let workouts = db
            .list_workouts(Some("Ride"))
            .expect("Failed to list workouts");
        assert_eq!(workouts.len(), 1);
    }

    #[test]
    fn test_workout_update() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let mut workout = create_test_workout("Original Name");
        let workout_id = workout.id;

        db.insert_workout(&workout)
            .expect("Failed to insert workout");

        // Update the workout
        workout.name = "Updated Name".to_string();
        workout.description = Some("Updated description".to_string());
        workout.tags.push("updated".to_string());

        db.update_workout(&workout)
            .expect("Failed to update workout");

        // Verify update
        let retrieved = db
            .get_workout(&workout_id)
            .expect("Failed to get workout")
            .expect("Workout not found");

        assert_eq!(retrieved.name, "Updated Name");
        assert_eq!(
            retrieved.description,
            Some("Updated description".to_string())
        );
        assert!(retrieved.tags.contains(&"updated".to_string()));
    }

    #[test]
    fn test_workout_delete() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let workout = create_test_workout("To Delete");
        let workout_id = workout.id;

        db.insert_workout(&workout)
            .expect("Failed to insert workout");
        assert_eq!(db.count_workouts().unwrap(), 1);

        // Delete the workout
        db.delete_workout(&workout_id)
            .expect("Failed to delete workout");
        assert_eq!(db.count_workouts().unwrap(), 0);

        // Verify it's gone
        let result = db.get_workout(&workout_id).expect("Failed to query");
        assert!(result.is_none());
    }

    #[test]
    fn test_workout_delete_not_found() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let fake_id = Uuid::new_v4();

        let result = db.delete_workout(&fake_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DatabaseError::NotFound(_)));
    }

    #[test]
    fn test_workout_count() {
        let db = Database::open_in_memory().expect("Failed to create database");

        assert_eq!(db.count_workouts().unwrap(), 0);

        db.insert_workout(&create_test_workout("One")).unwrap();
        assert_eq!(db.count_workouts().unwrap(), 1);

        db.insert_workout(&create_test_workout("Two")).unwrap();
        assert_eq!(db.count_workouts().unwrap(), 2);
    }

    #[test]
    fn test_workout_segment_types_roundtrip() {
        let db = Database::open_in_memory().expect("Failed to create database");

        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(60),
                ),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: 600,
                power_target: PowerTarget::absolute(250),
                cadence_target: None,
                text_event: Some("Go hard!".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::FreeRide,
                duration_seconds: 300,
                power_target: PowerTarget::percent_ftp(0),
                cadence_target: None,
                text_event: None,
            },
        ];

        let mut workout = Workout::new("Segment Test".to_string(), segments);
        workout.source_format = Some(WorkoutFormat::Mrc);
        let workout_id = workout.id;

        db.insert_workout(&workout).expect("Failed to insert");

        let retrieved = db
            .get_workout(&workout_id)
            .expect("Failed to get")
            .expect("Not found");

        assert_eq!(retrieved.segments.len(), 3);
        assert_eq!(retrieved.segments[0].segment_type, SegmentType::Warmup);
        assert_eq!(retrieved.segments[1].segment_type, SegmentType::Intervals);
        assert_eq!(retrieved.segments[2].segment_type, SegmentType::FreeRide);

        // Check range power target
        if let PowerTarget::Range { start, end } = &retrieved.segments[0].power_target {
            if let PowerTarget::PercentFtp { percent } = start.as_ref() {
                assert_eq!(*percent, 40);
            } else {
                panic!("Expected PercentFtp for start");
            }
            if let PowerTarget::PercentFtp { percent } = end.as_ref() {
                assert_eq!(*percent, 60);
            } else {
                panic!("Expected PercentFtp for end");
            }
        } else {
            panic!("Expected Range power target");
        }

        // Check absolute power target
        if let PowerTarget::Absolute { watts } = &retrieved.segments[1].power_target {
            assert_eq!(*watts, 250);
        } else {
            panic!("Expected Absolute power target");
        }

        // Check text event
        assert_eq!(
            retrieved.segments[1].text_event,
            Some("Go hard!".to_string())
        );

        // Check source format
        assert_eq!(retrieved.source_format, Some(WorkoutFormat::Mrc));
    }

    // ========== Ride CRUD Tests ==========

    fn create_test_ride(user_id: Uuid) -> Ride {
        let mut ride = Ride::new(user_id, 250);
        ride.ended_at = Some(Utc::now());
        ride.duration_seconds = 3600;
        ride.distance_meters = 30000.0;
        ride.avg_power = Some(200);
        ride.max_power = Some(350);
        ride.normalized_power = Some(210);
        ride.intensity_factor = Some(0.84);
        ride.tss = Some(70.0);
        ride.avg_hr = Some(145);
        ride.max_hr = Some(175);
        ride.avg_cadence = Some(88);
        ride.calories = 720;
        ride.notes = Some("Test ride".to_string());
        ride
    }

    fn create_test_samples(count: usize) -> Vec<RideSample> {
        (0..count)
            .map(|i| RideSample {
                elapsed_seconds: i as u32,
                power_watts: Some(200 + (i % 50) as u16),
                cadence_rpm: Some(85 + (i % 10) as u8),
                heart_rate_bpm: Some(140 + (i % 20) as u8),
                speed_kmh: Some(30.0 + (i % 5) as f32),
                distance_meters: i as f64 * 8.33,
                calories: (i as f64 * 0.2) as u32,
                resistance_level: None,
                target_power: Some(200),
                trainer_grade: None,
            })
            .collect()
    }

    #[test]
    fn test_ride_insert_and_get() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        let ride = create_test_ride(user_id);
        let ride_id = ride.id;

        db.insert_ride(&ride).expect("Failed to insert ride");

        let retrieved = db
            .get_ride(&ride_id)
            .expect("Failed to get ride")
            .expect("Ride not found");

        assert_eq!(retrieved.id, ride.id);
        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.duration_seconds, 3600);
        assert_eq!(retrieved.distance_meters, 30000.0);
        assert_eq!(retrieved.avg_power, Some(200));
        assert_eq!(retrieved.max_power, Some(350));
        assert_eq!(retrieved.ftp_at_ride, 250);
    }

    #[test]
    fn test_ride_samples_insert_and_get() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        let ride = create_test_ride(user_id);
        let ride_id = ride.id;

        db.insert_ride(&ride).expect("Failed to insert ride");

        let samples = create_test_samples(60);
        db.insert_ride_samples(&ride_id, &samples)
            .expect("Failed to insert samples");

        let retrieved_samples = db
            .get_ride_samples(&ride_id)
            .expect("Failed to get samples");

        assert_eq!(retrieved_samples.len(), 60);
        assert_eq!(retrieved_samples[0].elapsed_seconds, 0);
        assert_eq!(retrieved_samples[59].elapsed_seconds, 59);
    }

    #[test]
    fn test_ride_with_samples() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        let ride = create_test_ride(user_id);
        let ride_id = ride.id;

        db.insert_ride(&ride).expect("Failed to insert ride");

        let samples = create_test_samples(30);
        db.insert_ride_samples(&ride_id, &samples)
            .expect("Failed to insert samples");

        let result = db
            .get_ride_with_samples(&ride_id)
            .expect("Failed to get ride");
        assert!(result.is_some());

        let (retrieved_ride, retrieved_samples) = result.unwrap();
        assert_eq!(retrieved_ride.id, ride_id);
        assert_eq!(retrieved_samples.len(), 30);
    }

    #[test]
    fn test_ride_list() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        // Insert 3 rides
        for _ in 0..3 {
            let ride = create_test_ride(user_id);
            db.insert_ride(&ride).expect("Failed to insert ride");
        }

        let rides = db
            .list_rides(&user_id, None, None)
            .expect("Failed to list rides");
        assert_eq!(rides.len(), 3);
    }

    #[test]
    fn test_ride_delete() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        let ride = create_test_ride(user_id);
        let ride_id = ride.id;

        db.insert_ride(&ride).expect("Failed to insert ride");
        assert_eq!(db.count_rides(&user_id).unwrap(), 1);

        db.delete_ride(&ride_id).expect("Failed to delete ride");
        assert_eq!(db.count_rides(&user_id).unwrap(), 0);
    }

    #[test]
    fn test_ride_delete_cascades_samples() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // Insert user first (foreign key constraint)
        db.insert_user(&create_test_user_with_id(user_id))
            .expect("Failed to insert user");

        let ride = create_test_ride(user_id);
        let ride_id = ride.id;

        db.insert_ride(&ride).expect("Failed to insert ride");
        db.insert_ride_samples(&ride_id, &create_test_samples(100))
            .expect("Failed to insert samples");

        // Verify samples exist
        let samples = db
            .get_ride_samples(&ride_id)
            .expect("Failed to get samples");
        assert_eq!(samples.len(), 100);

        // Delete ride
        db.delete_ride(&ride_id).expect("Failed to delete ride");

        // Samples should be gone
        let samples = db
            .get_ride_samples(&ride_id)
            .expect("Failed to get samples");
        assert_eq!(samples.len(), 0);
    }

    // ========== Autosave Tests ==========

    #[test]
    fn test_autosave_roundtrip() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();
        let ride = create_test_ride(user_id);
        let samples = create_test_samples(30);

        // Initially no autosave
        assert!(!db.has_autosave().unwrap());

        // Save autosave
        db.save_autosave(&ride, &samples)
            .expect("Failed to save autosave");
        assert!(db.has_autosave().unwrap());

        // Load autosave
        let result = db.load_autosave().expect("Failed to load autosave");
        assert!(result.is_some());

        let (recovered_ride, recovered_samples) = result.unwrap();
        assert_eq!(recovered_ride.id, ride.id);
        assert_eq!(recovered_samples.len(), 30);
    }

    #[test]
    fn test_autosave_clear() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();
        let ride = create_test_ride(user_id);
        let samples = create_test_samples(10);

        db.save_autosave(&ride, &samples)
            .expect("Failed to save autosave");
        assert!(db.has_autosave().unwrap());

        db.clear_autosave().expect("Failed to clear autosave");
        assert!(!db.has_autosave().unwrap());
    }

    #[test]
    fn test_autosave_overwrite() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let user_id = Uuid::new_v4();

        // First save
        let ride1 = create_test_ride(user_id);
        let samples1 = create_test_samples(10);
        db.save_autosave(&ride1, &samples1)
            .expect("Failed to save first autosave");

        // Second save (overwrite)
        let ride2 = create_test_ride(user_id);
        let samples2 = create_test_samples(20);
        db.save_autosave(&ride2, &samples2)
            .expect("Failed to save second autosave");

        // Should get the second one
        let (recovered_ride, recovered_samples) = db.load_autosave().unwrap().unwrap();
        assert_eq!(recovered_ride.id, ride2.id);
        assert_eq!(recovered_samples.len(), 20);
    }

    // ========== User Profile CRUD Tests ==========

    fn create_test_user(name: &str) -> UserProfile {
        let mut profile = UserProfile::new(name.to_string());
        profile.ftp = 250;
        profile.weight_kg = 70.0;
        profile.max_hr = Some(180);
        profile.resting_hr = Some(50);
        profile.height_cm = Some(175);
        profile.set_heart_rate(Some(180), Some(50));
        profile
    }

    #[test]
    fn test_user_insert_and_get() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let profile = create_test_user("Test Cyclist");
        let user_id = profile.id;

        db.insert_user(&profile).expect("Failed to insert user");

        let retrieved = db
            .get_user(&user_id)
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(retrieved.id, profile.id);
        assert_eq!(retrieved.name, "Test Cyclist");
        assert_eq!(retrieved.ftp, 250);
        assert_eq!(retrieved.weight_kg, 70.0);
        assert_eq!(retrieved.max_hr, Some(180));
        assert_eq!(retrieved.resting_hr, Some(50));
        assert!(retrieved.hr_zones.is_some());
    }

    #[test]
    fn test_user_list() {
        let db = Database::open_in_memory().expect("Failed to create database");

        db.insert_user(&create_test_user("User One")).unwrap();
        db.insert_user(&create_test_user("User Two")).unwrap();
        db.insert_user(&create_test_user("User Three")).unwrap();

        let users = db.list_users().expect("Failed to list users");
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_user_get_default() {
        let db = Database::open_in_memory().expect("Failed to create database");

        // Initially no users
        let result = db.get_default_user().expect("Failed to query");
        assert!(result.is_none());

        // Insert some users
        let user1 = create_test_user("First User");
        let user2 = create_test_user("Second User");

        db.insert_user(&user1).unwrap();
        db.insert_user(&user2).unwrap();

        // Should return the first one inserted
        let default = db
            .get_default_user()
            .expect("Failed to get default")
            .expect("No default user");

        assert_eq!(default.id, user1.id);
    }

    #[test]
    fn test_user_update() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let mut profile = create_test_user("Original Name");
        let user_id = profile.id;

        db.insert_user(&profile).expect("Failed to insert user");

        // Update the profile
        profile.name = "Updated Name".to_string();
        let _ = profile.set_ftp(300);
        profile.weight_kg = 75.0;

        db.update_user(&profile).expect("Failed to update user");

        // Verify update
        let retrieved = db
            .get_user(&user_id)
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(retrieved.name, "Updated Name");
        assert_eq!(retrieved.ftp, 300);
        assert_eq!(retrieved.weight_kg, 75.0);
    }

    #[test]
    fn test_user_delete() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let profile = create_test_user("To Delete");
        let user_id = profile.id;

        db.insert_user(&profile).expect("Failed to insert user");
        assert_eq!(db.count_users().unwrap(), 1);

        db.delete_user(&user_id).expect("Failed to delete user");
        assert_eq!(db.count_users().unwrap(), 0);

        let result = db.get_user(&user_id).expect("Failed to query");
        assert!(result.is_none());
    }

    #[test]
    fn test_user_get_or_create_default() {
        let db = Database::open_in_memory().expect("Failed to create database");

        // Initially no users, should create one
        let profile1 = db
            .get_or_create_default_user()
            .expect("Failed to get or create");
        assert_eq!(db.count_users().unwrap(), 1);

        // Should return the same one on subsequent calls
        let profile2 = db
            .get_or_create_default_user()
            .expect("Failed to get or create");
        assert_eq!(db.count_users().unwrap(), 1);
        assert_eq!(profile1.id, profile2.id);
    }

    #[test]
    fn test_user_zones_roundtrip() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let profile = create_test_user("Zone Test");
        let user_id = profile.id;

        db.insert_user(&profile).expect("Failed to insert user");

        let retrieved = db
            .get_user(&user_id)
            .expect("Failed to get user")
            .expect("User not found");

        // Verify power zones are preserved
        assert_eq!(retrieved.power_zones.z4_threshold.name, "Threshold");

        // Verify HR zones are preserved
        let hr_zones = retrieved.hr_zones.expect("HR zones should be present");
        assert_eq!(hr_zones.z1_recovery.name, "Recovery");
    }
}
