//! Database operations using rusqlite.
//!
//! T009: Implement Database struct with connection and migration
//! T080: Implement workout CRUD operations
//! T099: Implement ride CRUD in database
//! T100: Implement ride_samples bulk insert
//! T115: Implement UserProfile CRUD in database
//! T126: Implement save_avatar database operation
//! T127: Implement get_avatar database operation
//! T145: Implement sensor CRUD in database

use crate::metrics::zones::{HRZones, PowerZones};
use crate::recording::types::{Ride, RideSample};
use crate::sensors::types::{Protocol, SavedSensor, SensorType};
use crate::storage::config::{Theme, Units, UserProfile};
use crate::storage::schema::{
    CURRENT_VERSION, MIGRATION_V1_TO_V2, MIGRATION_V2_TO_V3, MIGRATION_V5_TO_V6,
    MIGRATION_V6_TO_V7, SCHEMA, SCHEMA_VERSION_TABLE,
};
use crate::workouts::types::{Workout, WorkoutFormat, WorkoutSegment};
use crate::world::avatar::{AvatarConfig, BikeStyle};
use crate::world::route::{RouteSource, StoredRoute, StoredWaypoint, SurfaceType};
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

            // Record version 1
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 1");
        }

        // Migration v1 -> v2: Add analytics tables
        if from_version < 2 {
            self.conn
                .execute_batch(MIGRATION_V1_TO_V2)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 2
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (2, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 2 (analytics tables)");
        }

        // Migration v2 -> v3: Add ML coaching tables
        if from_version < 3 {
            self.conn
                .execute_batch(MIGRATION_V2_TO_V3)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 3
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (3, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 3 (ML coaching tables)");
        }

        // Migration v3 -> v4: Add 3D World & Content tables
        if from_version < 4 {
            self.conn
                .execute_batch(crate::storage::schema::MIGRATION_V3_TO_V4)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 4
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (4, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 4 (3D World & Content tables)");
        }

        // Migration v4 -> v5: Add Social & Multiplayer tables
        if from_version < 5 {
            self.conn
                .execute_batch(crate::storage::schema::MIGRATION_V4_TO_V5)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 5
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (5, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 5 (Social & Multiplayer tables)");
        }

        // Migration v5 -> v6: Add Hardware Integration tables
        if from_version < 6 {
            self.conn
                .execute_batch(MIGRATION_V5_TO_V6)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 6
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (6, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 6 (Hardware Integration tables)");
        }

        // Migration v6 -> v7: Add UX & Accessibility tables
        if from_version < 7 {
            self.conn
                .execute_batch(MIGRATION_V6_TO_V7)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version 7
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (7, datetime('now'))",
                    [],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version 7 (UX & Accessibility tables)");
        }

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
                    // T049: Dynamics fields (not stored in ride_samples table yet)
                    left_right_balance: None,
                    left_torque_effectiveness: None,
                    right_torque_effectiveness: None,
                    left_pedal_smoothness: None,
                    right_pedal_smoothness: None,
                    // T130: Power phase fields (not stored in ride_samples table yet)
                    left_power_phase_start: None,
                    left_power_phase_end: None,
                    left_power_phase_peak: None,
                    right_power_phase_start: None,
                    right_power_phase_end: None,
                    right_power_phase_peak: None,
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

    // ============= Avatar CRUD Operations (T126-T127) =============

    /// Save or update avatar configuration for a user.
    pub fn save_avatar(&self, user_id: &Uuid, config: &AvatarConfig) -> Result<(), DatabaseError> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let jersey_color = format!(
            "#{:02X}{:02X}{:02X}",
            config.jersey_color[0], config.jersey_color[1], config.jersey_color[2]
        );
        let jersey_secondary = config
            .jersey_secondary
            .map(|c| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
        let helmet_color = config
            .helmet_color
            .map(|c| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
        let bike_style = match config.bike_style {
            BikeStyle::RoadBike => "road_bike",
            BikeStyle::TimeTrial => "time_trial",
            BikeStyle::Gravel => "gravel",
        };

        // Use INSERT OR REPLACE to upsert (user_id is UNIQUE)
        self.conn
            .execute(
                r#"
                INSERT INTO avatars (id, user_id, jersey_color, jersey_secondary, bike_style, helmet_color, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                ON CONFLICT(user_id) DO UPDATE SET
                    jersey_color = excluded.jersey_color,
                    jersey_secondary = excluded.jersey_secondary,
                    bike_style = excluded.bike_style,
                    helmet_color = excluded.helmet_color,
                    updated_at = excluded.updated_at
                "#,
                params![
                    id.to_string(),
                    user_id.to_string(),
                    jersey_color,
                    jersey_secondary,
                    bike_style,
                    helmet_color,
                    now
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get avatar configuration for a user.
    pub fn get_avatar(&self, user_id: &Uuid) -> Result<Option<AvatarConfig>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT jersey_color, jersey_secondary, bike_style, helmet_color FROM avatars WHERE user_id = ?1",
            params![user_id.to_string()],
            |row| {
                let jersey_color: String = row.get(0)?;
                let jersey_secondary: Option<String> = row.get(1)?;
                let bike_style: String = row.get(2)?;
                let helmet_color: Option<String> = row.get(3)?;
                Ok((jersey_color, jersey_secondary, bike_style, helmet_color))
            },
        );

        match result {
            Ok((jersey_color, jersey_secondary, bike_style, helmet_color)) => {
                let jersey_color = parse_hex_color(&jersey_color).unwrap_or([255, 0, 0]);
                let jersey_secondary = jersey_secondary.as_ref().and_then(|s| parse_hex_color(s));
                let helmet_color = helmet_color.as_ref().and_then(|s| parse_hex_color(s));
                let bike_style = match bike_style.as_str() {
                    "time_trial" => BikeStyle::TimeTrial,
                    "gravel" => BikeStyle::Gravel,
                    _ => BikeStyle::RoadBike,
                };

                Ok(Some(AvatarConfig {
                    jersey_color,
                    bike_style,
                    jersey_secondary,
                    helmet_color,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Delete avatar configuration for a user.
    pub fn delete_avatar(&self, user_id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM avatars WHERE user_id = ?1",
                params![user_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    // ============= Route CRUD Operations (T019-T020) =============

    /// Insert a new stored route into the database.
    pub fn insert_route(&self, route: &StoredRoute) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO imported_routes (id, name, description, source, distance_meters,
                 elevation_gain_meters, max_elevation_meters, min_elevation_meters,
                 avg_gradient_percent, max_gradient_percent, source_file, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    route.id.to_string(),
                    route.name,
                    route.description,
                    route.source.to_string(),
                    route.distance_meters,
                    route.elevation_gain_meters,
                    route.max_elevation_meters,
                    route.min_elevation_meters,
                    route.avg_gradient_percent,
                    route.max_gradient_percent,
                    route.source_file,
                    route.created_at.to_rfc3339(),
                    route.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a stored route by ID.
    pub fn get_route(&self, id: &Uuid) -> Result<Option<StoredRoute>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description, source, distance_meters, elevation_gain_meters,
                 max_elevation_meters, min_elevation_meters, avg_gradient_percent,
                 max_gradient_percent, source_file, created_at, updated_at
                 FROM imported_routes WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(RouteRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                source: row.get(3)?,
                distance_meters: row.get(4)?,
                elevation_gain_meters: row.get(5)?,
                max_elevation_meters: row.get(6)?,
                min_elevation_meters: row.get(7)?,
                avg_gradient_percent: row.get(8)?,
                max_gradient_percent: row.get(9)?,
                source_file: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_stored_route()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all stored routes, optionally filtered by source.
    pub fn list_routes(
        &self,
        source: Option<RouteSource>,
    ) -> Result<Vec<StoredRoute>, DatabaseError> {
        let sql = match source {
            Some(_) => {
                "SELECT id, name, description, source, distance_meters, elevation_gain_meters,
                 max_elevation_meters, min_elevation_meters, avg_gradient_percent,
                 max_gradient_percent, source_file, created_at, updated_at
                 FROM imported_routes WHERE source = ?1 ORDER BY created_at DESC"
            }
            None => {
                "SELECT id, name, description, source, distance_meters, elevation_gain_meters,
                 max_elevation_meters, min_elevation_meters, avg_gradient_percent,
                 max_gradient_percent, source_file, created_at, updated_at
                 FROM imported_routes ORDER BY created_at DESC"
            }
        };

        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<RouteRow> {
            Ok(RouteRow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                source: row.get(3)?,
                distance_meters: row.get(4)?,
                elevation_gain_meters: row.get(5)?,
                max_elevation_meters: row.get(6)?,
                min_elevation_meters: row.get(7)?,
                avg_gradient_percent: row.get(8)?,
                max_gradient_percent: row.get(9)?,
                source_file: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        };

        let mut routes = Vec::new();

        if let Some(src) = source {
            let rows = stmt
                .query_map(params![src.to_string()], map_row)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            for row in rows {
                let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
                routes.push(row.into_stored_route()?);
            }
        } else {
            let rows = stmt
                .query_map([], map_row)
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            for row in rows {
                let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
                routes.push(row.into_stored_route()?);
            }
        }

        Ok(routes)
    }

    /// Update an existing stored route.
    pub fn update_route(&self, route: &StoredRoute) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE imported_routes SET name = ?2, description = ?3, source = ?4,
                 distance_meters = ?5, elevation_gain_meters = ?6, max_elevation_meters = ?7,
                 min_elevation_meters = ?8, avg_gradient_percent = ?9, max_gradient_percent = ?10,
                 source_file = ?11, updated_at = ?12 WHERE id = ?1",
                params![
                    route.id.to_string(),
                    route.name,
                    route.description,
                    route.source.to_string(),
                    route.distance_meters,
                    route.elevation_gain_meters,
                    route.max_elevation_meters,
                    route.min_elevation_meters,
                    route.avg_gradient_percent,
                    route.max_gradient_percent,
                    route.source_file,
                    route.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Route {}", route.id)));
        }

        Ok(())
    }

    /// Delete a stored route by ID (cascades to waypoints).
    pub fn delete_route(&self, id: &Uuid) -> Result<(), DatabaseError> {
        // First delete waypoints (foreign key cascade may not be enabled)
        self.conn
            .execute(
                "DELETE FROM route_waypoints WHERE route_id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM imported_routes WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Route {}", id)));
        }

        Ok(())
    }

    /// Count routes in the database.
    pub fn count_routes(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM imported_routes", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ============= Route Waypoints CRUD Operations (T020) =============

    /// Insert waypoints for a route in bulk.
    pub fn insert_route_waypoints(
        &mut self,
        waypoints: &[StoredWaypoint],
    ) -> Result<(), DatabaseError> {
        if waypoints.is_empty() {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO route_waypoints (id, route_id, sequence, latitude, longitude,
                     elevation_meters, distance_from_start, gradient_percent, surface_type)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            for wp in waypoints {
                let surface_str = match wp.surface_type {
                    SurfaceType::Asphalt => "asphalt",
                    SurfaceType::Concrete => "concrete",
                    SurfaceType::Cobblestone => "cobblestone",
                    SurfaceType::Gravel => "gravel",
                    SurfaceType::Dirt => "dirt",
                };

                stmt.execute(params![
                    wp.id.to_string(),
                    wp.route_id.to_string(),
                    wp.sequence,
                    wp.latitude,
                    wp.longitude,
                    wp.elevation_meters,
                    wp.distance_from_start,
                    wp.gradient_percent,
                    surface_str,
                ])
                .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))?;

        Ok(())
    }

    /// Get all waypoints for a route, ordered by sequence.
    pub fn get_route_waypoints(
        &self,
        route_id: &Uuid,
    ) -> Result<Vec<StoredWaypoint>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, route_id, sequence, latitude, longitude, elevation_meters,
                 distance_from_start, gradient_percent, surface_type
                 FROM route_waypoints WHERE route_id = ?1 ORDER BY sequence",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![route_id.to_string()], |row| {
                Ok(WaypointRow {
                    id: row.get(0)?,
                    route_id: row.get(1)?,
                    sequence: row.get(2)?,
                    latitude: row.get(3)?,
                    longitude: row.get(4)?,
                    elevation_meters: row.get(5)?,
                    distance_from_start: row.get(6)?,
                    gradient_percent: row.get(7)?,
                    surface_type: row.get(8)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut waypoints = Vec::new();
        for row in rows {
            let row = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
            waypoints.push(row.into_stored_waypoint()?);
        }

        Ok(waypoints)
    }

    /// Get a route with all its waypoints.
    pub fn get_route_with_waypoints(
        &self,
        id: &Uuid,
    ) -> Result<Option<(StoredRoute, Vec<StoredWaypoint>)>, DatabaseError> {
        let route = self.get_route(id)?;
        match route {
            Some(route) => {
                let waypoints = self.get_route_waypoints(id)?;
                Ok(Some((route, waypoints)))
            }
            None => Ok(None),
        }
    }

    /// Delete all waypoints for a route.
    pub fn delete_route_waypoints(&self, route_id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM route_waypoints WHERE route_id = ?1",
                params![route_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Count waypoints for a route.
    pub fn count_route_waypoints(&self, route_id: &Uuid) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM route_waypoints WHERE route_id = ?1",
                params![route_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ========== Landmark CRUD Operations (T089) ==========

    /// Insert a new landmark into the database.
    pub fn insert_landmark(
        &self,
        landmark: &crate::world::landmarks::Landmark,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO landmarks (id, route_id, landmark_type, name, description,
                 latitude, longitude, elevation_meters, distance_meters, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    landmark.id.to_string(),
                    landmark.route_id.map(|id| id.to_string()),
                    format!("{:?}", landmark.landmark_type).to_lowercase(),
                    landmark.name,
                    landmark.description,
                    landmark.latitude,
                    landmark.longitude,
                    landmark.elevation_meters,
                    landmark.distance_meters,
                    landmark.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a landmark by ID.
    pub fn get_landmark(
        &self,
        id: &Uuid,
    ) -> Result<Option<crate::world::landmarks::Landmark>, DatabaseError> {
        use crate::world::landmarks::{Landmark, LandmarkType};

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, route_id, landmark_type, name, description,
                 latitude, longitude, elevation_meters, distance_meters, created_at
                 FROM landmarks WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let route_id_str: Option<String> = row.get(1)?;
            let type_str: String = row.get(2)?;
            let name: String = row.get(3)?;
            let description: Option<String> = row.get(4)?;
            let latitude: f64 = row.get(5)?;
            let longitude: f64 = row.get(6)?;
            let elevation: f32 = row.get(7)?;
            let distance: Option<f64> = row.get(8)?;
            let created_str: String = row.get(9)?;

            Ok((
                id_str,
                route_id_str,
                type_str,
                name,
                description,
                latitude,
                longitude,
                elevation,
                distance,
                created_str,
            ))
        });

        match result {
            Ok((
                id_str,
                route_id_str,
                type_str,
                name,
                description,
                latitude,
                longitude,
                elevation,
                distance,
                created_str,
            )) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let route_id = route_id_str
                    .map(|s| Uuid::parse_str(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let landmark_type = match type_str.as_str() {
                    "summit" => LandmarkType::Summit,
                    "viewpoint" => LandmarkType::Viewpoint,
                    "town" => LandmarkType::Town,
                    "historic" => LandmarkType::Historic,
                    "sprint" => LandmarkType::Sprint,
                    "feedzone" => LandmarkType::FeedZone,
                    "waterfountain" => LandmarkType::WaterFountain,
                    "restarea" => LandmarkType::RestArea,
                    _ => LandmarkType::Custom,
                };
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

                Ok(Some(Landmark {
                    id,
                    route_id,
                    landmark_type,
                    name,
                    description,
                    latitude,
                    longitude,
                    elevation_meters: elevation,
                    distance_meters: distance,
                    created_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all landmarks for a route.
    pub fn get_route_landmarks(
        &self,
        route_id: &Uuid,
    ) -> Result<Vec<crate::world::landmarks::Landmark>, DatabaseError> {
        use crate::world::landmarks::{Landmark, LandmarkType};

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, route_id, landmark_type, name, description,
                 latitude, longitude, elevation_meters, distance_meters, created_at
                 FROM landmarks WHERE route_id = ?1 ORDER BY distance_meters",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![route_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let route_id_str: Option<String> = row.get(1)?;
                let type_str: String = row.get(2)?;
                let name: String = row.get(3)?;
                let description: Option<String> = row.get(4)?;
                let latitude: f64 = row.get(5)?;
                let longitude: f64 = row.get(6)?;
                let elevation: f32 = row.get(7)?;
                let distance: Option<f64> = row.get(8)?;
                let created_str: String = row.get(9)?;

                Ok((
                    id_str,
                    route_id_str,
                    type_str,
                    name,
                    description,
                    latitude,
                    longitude,
                    elevation,
                    distance,
                    created_str,
                ))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut landmarks = Vec::new();
        for row in rows {
            let (
                id_str,
                route_id_str,
                type_str,
                name,
                description,
                latitude,
                longitude,
                elevation,
                distance,
                created_str,
            ) = row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let route_id = route_id_str
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let landmark_type = match type_str.as_str() {
                "summit" => LandmarkType::Summit,
                "viewpoint" => LandmarkType::Viewpoint,
                "town" => LandmarkType::Town,
                "historic" => LandmarkType::Historic,
                "sprint" => LandmarkType::Sprint,
                "feedzone" => LandmarkType::FeedZone,
                "waterfountain" => LandmarkType::WaterFountain,
                "restarea" => LandmarkType::RestArea,
                _ => LandmarkType::Custom,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

            landmarks.push(Landmark {
                id,
                route_id,
                landmark_type,
                name,
                description,
                latitude,
                longitude,
                elevation_meters: elevation,
                distance_meters: distance,
                created_at,
            });
        }

        Ok(landmarks)
    }

    /// Delete a landmark by ID.
    pub fn delete_landmark(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM landmarks WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Landmark {}", id)));
        }

        Ok(())
    }

    /// Count landmarks for a route.
    pub fn count_route_landmarks(&self, route_id: &Uuid) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM landmarks WHERE route_id = ?1",
                params![route_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ========== Landmark Discovery CRUD Operations (T090) ==========

    /// Insert a new landmark discovery.
    pub fn insert_landmark_discovery(
        &self,
        discovery: &crate::world::landmarks::discovery::LandmarkDiscovery,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "INSERT INTO landmark_discoveries (id, user_id, landmark_id, ride_id, discovered_at, screenshot_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    discovery.id.to_string(),
                    discovery.user_id.to_string(),
                    discovery.landmark_id.to_string(),
                    discovery.ride_id.to_string(),
                    discovery.discovered_at.to_rfc3339(),
                    discovery.screenshot_path,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get all discoveries for a user.
    pub fn get_user_discoveries(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<crate::world::landmarks::discovery::LandmarkDiscovery>, DatabaseError> {
        use crate::world::landmarks::discovery::LandmarkDiscovery;

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, landmark_id, ride_id, discovered_at, screenshot_path
                 FROM landmark_discoveries WHERE user_id = ?1 ORDER BY discovered_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let user_id_str: String = row.get(1)?;
                let landmark_id_str: String = row.get(2)?;
                let ride_id_str: String = row.get(3)?;
                let discovered_str: String = row.get(4)?;
                let screenshot: Option<String> = row.get(5)?;

                Ok((
                    id_str,
                    user_id_str,
                    landmark_id_str,
                    ride_id_str,
                    discovered_str,
                    screenshot,
                ))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut discoveries = Vec::new();
        for row in rows {
            let (id_str, user_id_str, landmark_id_str, ride_id_str, discovered_str, screenshot) =
                row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let user_id = Uuid::parse_str(&user_id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let landmark_id = Uuid::parse_str(&landmark_id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let ride_id = Uuid::parse_str(&ride_id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let discovered_at = DateTime::parse_from_rfc3339(&discovered_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

            discoveries.push(LandmarkDiscovery {
                id,
                user_id,
                landmark_id,
                ride_id,
                discovered_at,
                screenshot_path: screenshot,
            });
        }

        Ok(discoveries)
    }

    /// Check if a user has discovered a specific landmark.
    pub fn has_discovered_landmark(
        &self,
        user_id: &Uuid,
        landmark_id: &Uuid,
    ) -> Result<bool, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM landmark_discoveries WHERE user_id = ?1 AND landmark_id = ?2",
                params![user_id.to_string(), landmark_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count > 0)
    }

    /// Count total discoveries for a user.
    pub fn count_user_discoveries(&self, user_id: &Uuid) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM landmark_discoveries WHERE user_id = ?1",
                params![user_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    // ========== Layout Profile CRUD Operations (T018-T019) ==========

    /// Insert a new layout profile.
    pub fn insert_layout_profile(
        &self,
        profile: &crate::ui::layout::LayoutProfile,
    ) -> Result<(), DatabaseError> {
        let layout_json = serde_json::to_string(&profile.widgets)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO layout_profiles (id, name, layout_json, is_default, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    profile.id.to_string(),
                    profile.name,
                    layout_json,
                    profile.is_default as i32,
                    profile.created_at.to_rfc3339(),
                    profile.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get a layout profile by ID.
    pub fn get_layout_profile(
        &self,
        id: &Uuid,
    ) -> Result<Option<crate::ui::layout::LayoutProfile>, DatabaseError> {
        use crate::ui::layout::{LayoutProfile, WidgetPlacement};

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, layout_json, is_default, created_at, updated_at
                 FROM layout_profiles WHERE id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            let layout_json: String = row.get(2)?;
            let is_default: i32 = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;

            Ok((id_str, name, layout_json, is_default, created_str, updated_str))
        });

        match result {
            Ok((id_str, name, layout_json, is_default, created_str, updated_str)) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let widgets: Vec<WidgetPlacement> = serde_json::from_str(&layout_json)
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

                Ok(Some(LayoutProfile {
                    id,
                    name,
                    widgets,
                    is_default: is_default != 0,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Get all layout profiles.
    pub fn list_layout_profiles(&self) -> Result<Vec<crate::ui::layout::LayoutProfile>, DatabaseError> {
        use crate::ui::layout::{LayoutProfile, WidgetPlacement};

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, layout_json, is_default, created_at, updated_at
                 FROM layout_profiles ORDER BY created_at ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let name: String = row.get(1)?;
                let layout_json: String = row.get(2)?;
                let is_default: i32 = row.get(3)?;
                let created_str: String = row.get(4)?;
                let updated_str: String = row.get(5)?;

                Ok((id_str, name, layout_json, is_default, created_str, updated_str))
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut profiles = Vec::new();
        for row in rows {
            let (id_str, name, layout_json, is_default, created_str, updated_str) =
                row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let widgets: Vec<WidgetPlacement> = serde_json::from_str(&layout_json)
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

            profiles.push(LayoutProfile {
                id,
                name,
                widgets,
                is_default: is_default != 0,
                created_at,
                updated_at,
            });
        }

        Ok(profiles)
    }

    /// Update an existing layout profile.
    pub fn update_layout_profile(
        &self,
        profile: &crate::ui::layout::LayoutProfile,
    ) -> Result<(), DatabaseError> {
        let layout_json = serde_json::to_string(&profile.widgets)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let rows_affected = self
            .conn
            .execute(
                "UPDATE layout_profiles SET name = ?2, layout_json = ?3, is_default = ?4, updated_at = ?5
                 WHERE id = ?1",
                params![
                    profile.id.to_string(),
                    profile.name,
                    layout_json,
                    profile.is_default as i32,
                    profile.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Layout profile {}", profile.id)));
        }

        Ok(())
    }

    /// Delete a layout profile by ID.
    pub fn delete_layout_profile(&self, id: &Uuid) -> Result<(), DatabaseError> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM layout_profiles WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        if rows_affected == 0 {
            return Err(DatabaseError::NotFound(format!("Layout profile {}", id)));
        }

        Ok(())
    }

    /// Count layout profiles.
    pub fn count_layout_profiles(&self) -> Result<usize, DatabaseError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM layout_profiles", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(count as usize)
    }

    /// Get or create default layout profile.
    pub fn get_or_create_default_layout(&self) -> Result<crate::ui::layout::LayoutProfile, DatabaseError> {
        use crate::ui::layout::LayoutProfile;

        // Check if any profiles exist
        let profiles = self.list_layout_profiles()?;
        if let Some(default) = profiles.into_iter().find(|p| p.is_default) {
            return Ok(default);
        }

        // Create and insert default profile
        let profile = LayoutProfile::default_layout();
        self.insert_layout_profile(&profile)?;
        Ok(profile)
    }

    // ========== Onboarding State CRUD Operations (T020-T021) ==========

    /// Get onboarding state.
    pub fn get_onboarding_state(
        &self,
    ) -> Result<Option<crate::onboarding::OnboardingState>, DatabaseError> {
        use crate::onboarding::{OnboardingState, OnboardingStep};

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, current_step, completed, skipped_at, completed_steps, started_at
                 FROM onboarding_state WHERE id = 1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row([], |row| {
            let _id: i32 = row.get(0)?;
            let current_step: i32 = row.get(1)?;
            let completed: i32 = row.get(2)?;
            let skipped_at_str: Option<String> = row.get(3)?;
            let completed_steps_json: String = row.get(4)?;
            let _started_at_str: String = row.get(5)?;

            Ok((current_step, completed, skipped_at_str, completed_steps_json))
        });

        match result {
            Ok((current_step_idx, completed, skipped_at_str, completed_steps_json)) => {
                let steps = OnboardingStep::all();
                let current_step = steps.get(current_step_idx as usize)
                    .copied()
                    .unwrap_or(OnboardingStep::Welcome);
                let skipped = skipped_at_str.is_some();
                let completed_steps: Vec<OnboardingStep> = serde_json::from_str(&completed_steps_json)
                    .unwrap_or_default();

                Ok(Some(OnboardingState {
                    completed: completed != 0,
                    current_step,
                    skipped,
                    completed_steps,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save or update onboarding state (upsert).
    pub fn save_onboarding_state(
        &self,
        state: &crate::onboarding::OnboardingState,
    ) -> Result<(), DatabaseError> {
        let completed_steps_json = serde_json::to_string(&state.completed_steps)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        let skipped_at = if state.skipped {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        self.conn
            .execute(
                "INSERT INTO onboarding_state (id, current_step, completed, skipped_at, completed_steps, started_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                 current_step = excluded.current_step,
                 completed = excluded.completed,
                 skipped_at = excluded.skipped_at,
                 completed_steps = excluded.completed_steps",
                params![
                    state.current_step.index() as i32,
                    state.completed as i32,
                    skipped_at,
                    completed_steps_json,
                    Utc::now().to_rfc3339(), // started_at only set on insert
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get or create initial onboarding state.
    pub fn get_or_create_onboarding_state(
        &self,
    ) -> Result<crate::onboarding::OnboardingState, DatabaseError> {
        if let Some(state) = self.get_onboarding_state()? {
            return Ok(state);
        }

        let state = crate::onboarding::OnboardingState::default();
        self.save_onboarding_state(&state)?;
        Ok(state)
    }

    // ========== User Preferences CRUD Operations (T022-T023) ==========

    /// Get user preferences.
    pub fn get_user_preferences(
        &self,
    ) -> Result<Option<crate::storage::config::UserPreferences>, DatabaseError> {
        use crate::storage::config::{
            AccessibilitySettings, AudioCueSettings, DisplayMode, FlowModeConfig,
            LocaleSettings, ThemePreference, UserPreferences,
        };

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, theme_preference, accessibility_json, audio_json, display_mode,
                 flow_mode_json, locale_json, active_layout_id
                 FROM user_preferences WHERE id = 1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let result = stmt.query_row([], |row| {
            let _id: i32 = row.get(0)?;
            let theme_pref_str: String = row.get(1)?;
            let accessibility_json: String = row.get(2)?;
            let audio_json: String = row.get(3)?;
            let display_mode_str: String = row.get(4)?;
            let flow_mode_json: String = row.get(5)?;
            let locale_json: String = row.get(6)?;
            let active_layout_id_str: Option<String> = row.get(7)?;

            Ok((
                theme_pref_str,
                accessibility_json,
                audio_json,
                display_mode_str,
                flow_mode_json,
                locale_json,
                active_layout_id_str,
            ))
        });

        match result {
            Ok((
                theme_pref_str,
                accessibility_json,
                audio_json,
                display_mode_str,
                flow_mode_json,
                locale_json,
                active_layout_id_str,
            )) => {
                let theme_preference = match theme_pref_str.as_str() {
                    "light" => ThemePreference::Light,
                    "dark" => ThemePreference::Dark,
                    _ => ThemePreference::FollowSystem,
                };
                let accessibility: AccessibilitySettings = serde_json::from_str(&accessibility_json)
                    .unwrap_or_default();
                let audio: AudioCueSettings = serde_json::from_str(&audio_json)
                    .unwrap_or_default();
                let display_mode = match display_mode_str.as_str() {
                    "tv_mode" => DisplayMode::TvMode,
                    "flow_mode" => DisplayMode::FlowMode,
                    _ => DisplayMode::Normal,
                };
                let flow_mode: FlowModeConfig = serde_json::from_str(&flow_mode_json)
                    .unwrap_or_default();
                let locale: LocaleSettings = serde_json::from_str(&locale_json)
                    .unwrap_or_default();
                let active_layout_id = active_layout_id_str
                    .map(|s| Uuid::parse_str(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::DeserializationError(e.to_string()))?;

                Ok(Some(UserPreferences {
                    theme_preference,
                    accessibility,
                    audio,
                    display_mode,
                    flow_mode,
                    locale,
                    active_layout_id,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Save or update user preferences (upsert).
    pub fn save_user_preferences(
        &self,
        prefs: &crate::storage::config::UserPreferences,
    ) -> Result<(), DatabaseError> {
        let theme_pref_str = match prefs.theme_preference {
            crate::storage::config::ThemePreference::FollowSystem => "follow_system",
            crate::storage::config::ThemePreference::Light => "light",
            crate::storage::config::ThemePreference::Dark => "dark",
        };
        let accessibility_json = serde_json::to_string(&prefs.accessibility)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        let audio_json = serde_json::to_string(&prefs.audio)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        let display_mode_str = match prefs.display_mode {
            crate::storage::config::DisplayMode::Normal => "normal",
            crate::storage::config::DisplayMode::TvMode => "tv_mode",
            crate::storage::config::DisplayMode::FlowMode => "flow_mode",
        };
        let flow_mode_json = serde_json::to_string(&prefs.flow_mode)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
        let locale_json = serde_json::to_string(&prefs.locale)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO user_preferences (id, theme_preference, accessibility_json, audio_json,
                 display_mode, flow_mode_json, locale_json, active_layout_id)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                 theme_preference = excluded.theme_preference,
                 accessibility_json = excluded.accessibility_json,
                 audio_json = excluded.audio_json,
                 display_mode = excluded.display_mode,
                 flow_mode_json = excluded.flow_mode_json,
                 locale_json = excluded.locale_json,
                 active_layout_id = excluded.active_layout_id",
                params![
                    theme_pref_str,
                    accessibility_json,
                    audio_json,
                    display_mode_str,
                    flow_mode_json,
                    locale_json,
                    prefs.active_layout_id.map(|id| id.to_string()),
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Get or create default user preferences.
    pub fn get_or_create_user_preferences(
        &self,
    ) -> Result<crate::storage::config::UserPreferences, DatabaseError> {
        if let Some(prefs) = self.get_user_preferences()? {
            return Ok(prefs);
        }

        let prefs = crate::storage::config::UserPreferences::default();
        self.save_user_preferences(&prefs)?;
        Ok(prefs)
    }
}

/// Parse a hex color string (e.g., "#FF0000") to RGB array.
fn parse_hex_color(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
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
            // T049: Dynamics averages (not in current DB schema)
            avg_left_balance: None,
            avg_left_torque_eff: None,
            avg_right_torque_eff: None,
            avg_left_smoothness: None,
            avg_right_smoothness: None,
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

/// Intermediate struct for reading route rows from database.
struct RouteRow {
    id: String,
    name: String,
    description: Option<String>,
    source: String,
    distance_meters: f64,
    elevation_gain_meters: f32,
    max_elevation_meters: f32,
    min_elevation_meters: f32,
    avg_gradient_percent: f32,
    max_gradient_percent: f32,
    source_file: Option<String>,
    created_at: String,
    updated_at: String,
}

impl RouteRow {
    fn into_stored_route(self) -> Result<StoredRoute, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let source: RouteSource = self.source.parse().map_err(|e: String| {
            DatabaseError::DeserializationError(format!("Invalid route source: {}", e))
        })?;

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

        Ok(StoredRoute {
            id,
            name: self.name,
            description: self.description,
            source,
            distance_meters: self.distance_meters,
            elevation_gain_meters: self.elevation_gain_meters,
            max_elevation_meters: self.max_elevation_meters,
            min_elevation_meters: self.min_elevation_meters,
            avg_gradient_percent: self.avg_gradient_percent,
            max_gradient_percent: self.max_gradient_percent,
            source_file: self.source_file,
            created_at,
            updated_at,
        })
    }
}

/// Intermediate struct for reading waypoint rows from database.
struct WaypointRow {
    id: String,
    route_id: String,
    sequence: u32,
    latitude: f64,
    longitude: f64,
    elevation_meters: f32,
    distance_from_start: f32,
    gradient_percent: f32,
    surface_type: String,
}

impl WaypointRow {
    fn into_stored_waypoint(self) -> Result<StoredWaypoint, DatabaseError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DatabaseError::DeserializationError(format!("Invalid UUID: {}", e)))?;

        let route_id = Uuid::parse_str(&self.route_id).map_err(|e| {
            DatabaseError::DeserializationError(format!("Invalid route UUID: {}", e))
        })?;

        let surface_type = match self.surface_type.to_lowercase().as_str() {
            "asphalt" => SurfaceType::Asphalt,
            "concrete" => SurfaceType::Concrete,
            "cobblestone" => SurfaceType::Cobblestone,
            "gravel" => SurfaceType::Gravel,
            "dirt" => SurfaceType::Dirt,
            _ => SurfaceType::Asphalt, // Default to asphalt
        };

        Ok(StoredWaypoint {
            id,
            route_id,
            sequence: self.sequence,
            latitude: self.latitude,
            longitude: self.longitude,
            elevation_meters: self.elevation_meters,
            distance_from_start: self.distance_from_start,
            gradient_percent: self.gradient_percent,
            surface_type,
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
        UserProfile {
            id: user_id,
            name: "Test User".to_string(),
            ..Default::default()
        }
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
                left_right_balance: None,
                left_torque_effectiveness: None,
                right_torque_effectiveness: None,
                left_pedal_smoothness: None,
                right_pedal_smoothness: None,
                left_power_phase_start: None,
                left_power_phase_end: None,
                left_power_phase_peak: None,
                right_power_phase_start: None,
                right_power_phase_end: None,
                right_power_phase_peak: None,
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

    // ========== Route CRUD Tests (T019-T020) ==========

    fn create_test_stored_route(name: &str) -> StoredRoute {
        let mut route = StoredRoute::new(name.to_string(), RouteSource::Gpx);
        route.description = Some("Test route description".to_string());
        route.distance_meters = 25000.0;
        route.elevation_gain_meters = 500.0;
        route.max_elevation_meters = 800.0;
        route.min_elevation_meters = 300.0;
        route.avg_gradient_percent = 2.0;
        route.max_gradient_percent = 12.0;
        route.source_file = Some("/test/route.gpx".to_string());
        route
    }

    fn create_test_waypoints(route_id: Uuid, count: u32) -> Vec<StoredWaypoint> {
        (0..count)
            .map(|i| {
                StoredWaypoint::new(
                    route_id,
                    i,
                    45.0 + (i as f64 * 0.001),
                    -122.0 + (i as f64 * 0.001),
                    100.0 + (i as f32 * 10.0),
                    i as f32 * 100.0,
                )
                .with_gradient(if i % 3 == 0 { 5.0 } else { 0.0 })
            })
            .collect()
    }

    #[test]
    fn test_route_insert_and_get() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("Test GPX Route");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");

        let retrieved = db
            .get_route(&route_id)
            .expect("Failed to get route")
            .expect("Route not found");

        assert_eq!(retrieved.id, route.id);
        assert_eq!(retrieved.name, "Test GPX Route");
        assert_eq!(retrieved.source, RouteSource::Gpx);
        assert_eq!(retrieved.distance_meters, 25000.0);
        assert_eq!(retrieved.elevation_gain_meters, 500.0);
    }

    #[test]
    fn test_route_list_all() {
        let db = Database::open_in_memory().expect("Failed to create database");

        db.insert_route(&create_test_stored_route("Route One"))
            .unwrap();

        let mut route2 = create_test_stored_route("Route Two");
        route2.source = RouteSource::Fit;
        db.insert_route(&route2).unwrap();

        let mut route3 = create_test_stored_route("Route Three");
        route3.source = RouteSource::Tcx;
        db.insert_route(&route3).unwrap();

        // List all
        let routes = db.list_routes(None).expect("Failed to list routes");
        assert_eq!(routes.len(), 3);
    }

    #[test]
    fn test_route_list_by_source() {
        let db = Database::open_in_memory().expect("Failed to create database");

        db.insert_route(&create_test_stored_route("GPX Route 1"))
            .unwrap();
        db.insert_route(&create_test_stored_route("GPX Route 2"))
            .unwrap();

        let mut fit_route = create_test_stored_route("FIT Route");
        fit_route.source = RouteSource::Fit;
        db.insert_route(&fit_route).unwrap();

        // List only GPX routes
        let gpx_routes = db
            .list_routes(Some(RouteSource::Gpx))
            .expect("Failed to list routes");
        assert_eq!(gpx_routes.len(), 2);

        // List only FIT routes
        let fit_routes = db
            .list_routes(Some(RouteSource::Fit))
            .expect("Failed to list routes");
        assert_eq!(fit_routes.len(), 1);
    }

    #[test]
    fn test_route_update() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let mut route = create_test_stored_route("Original Name");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");

        // Update the route
        route.name = "Updated Name".to_string();
        route.description = Some("Updated description".to_string());
        route.distance_meters = 30000.0;

        db.update_route(&route).expect("Failed to update route");

        // Verify update
        let retrieved = db
            .get_route(&route_id)
            .expect("Failed to get route")
            .expect("Route not found");

        assert_eq!(retrieved.name, "Updated Name");
        assert_eq!(
            retrieved.description,
            Some("Updated description".to_string())
        );
        assert_eq!(retrieved.distance_meters, 30000.0);
    }

    #[test]
    fn test_route_delete() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("To Delete");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");
        assert_eq!(db.count_routes().unwrap(), 1);

        db.delete_route(&route_id).expect("Failed to delete route");
        assert_eq!(db.count_routes().unwrap(), 0);

        let result = db.get_route(&route_id).expect("Failed to query");
        assert!(result.is_none());
    }

    #[test]
    fn test_waypoints_insert_and_get() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("Route with Waypoints");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");

        let waypoints = create_test_waypoints(route_id, 10);
        db.insert_route_waypoints(&waypoints)
            .expect("Failed to insert waypoints");

        let retrieved = db
            .get_route_waypoints(&route_id)
            .expect("Failed to get waypoints");

        assert_eq!(retrieved.len(), 10);
        assert_eq!(retrieved[0].sequence, 0);
        assert_eq!(retrieved[9].sequence, 9);
    }

    #[test]
    fn test_route_with_waypoints() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("Full Route");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");

        let waypoints = create_test_waypoints(route_id, 5);
        db.insert_route_waypoints(&waypoints)
            .expect("Failed to insert waypoints");

        let result = db
            .get_route_with_waypoints(&route_id)
            .expect("Failed to get route");
        assert!(result.is_some());

        let (retrieved_route, retrieved_waypoints) = result.unwrap();
        assert_eq!(retrieved_route.id, route_id);
        assert_eq!(retrieved_waypoints.len(), 5);
    }

    #[test]
    fn test_route_delete_cascades_waypoints() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("Route to Delete");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");
        db.insert_route_waypoints(&create_test_waypoints(route_id, 20))
            .expect("Failed to insert waypoints");

        // Verify waypoints exist
        assert_eq!(db.count_route_waypoints(&route_id).unwrap(), 20);

        // Delete route
        db.delete_route(&route_id).expect("Failed to delete route");

        // Waypoints should be gone
        assert_eq!(db.count_route_waypoints(&route_id).unwrap(), 0);
    }

    #[test]
    fn test_waypoint_surface_types() {
        let mut db = Database::open_in_memory().expect("Failed to create database");
        let route = create_test_stored_route("Surface Test Route");
        let route_id = route.id;

        db.insert_route(&route).expect("Failed to insert route");

        let waypoints = vec![
            StoredWaypoint::new(route_id, 0, 45.0, -122.0, 100.0, 0.0)
                .with_surface(SurfaceType::Asphalt),
            StoredWaypoint::new(route_id, 1, 45.001, -122.001, 110.0, 100.0)
                .with_surface(SurfaceType::Gravel),
            StoredWaypoint::new(route_id, 2, 45.002, -122.002, 120.0, 200.0)
                .with_surface(SurfaceType::Cobblestone),
        ];

        db.insert_route_waypoints(&waypoints)
            .expect("Failed to insert waypoints");

        let retrieved = db
            .get_route_waypoints(&route_id)
            .expect("Failed to get waypoints");

        assert_eq!(retrieved[0].surface_type, SurfaceType::Asphalt);
        assert_eq!(retrieved[1].surface_type, SurfaceType::Gravel);
        assert_eq!(retrieved[2].surface_type, SurfaceType::Cobblestone);
    }
}
