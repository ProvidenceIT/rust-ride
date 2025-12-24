//! Database schema definitions for RustRide.
//!
//! T008: Define database schema SQL

/// SQL schema for creating all database tables.
pub const SCHEMA: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ftp INTEGER NOT NULL DEFAULT 200,
    max_hr INTEGER,
    resting_hr INTEGER,
    weight_kg REAL NOT NULL,
    height_cm INTEGER,
    power_zones_json TEXT NOT NULL,
    hr_zones_json TEXT,
    units TEXT NOT NULL DEFAULT 'metric',
    theme TEXT NOT NULL DEFAULT 'dark',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sensors table
CREATE TABLE IF NOT EXISTS sensors (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sensor_type TEXT NOT NULL,
    protocol TEXT NOT NULL,
    last_seen_at TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, device_id)
);

-- Workouts table
CREATE TABLE IF NOT EXISTS workouts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    author TEXT,
    source_file TEXT,
    source_format TEXT,
    segments_json TEXT NOT NULL,
    total_duration_seconds INTEGER NOT NULL,
    estimated_tss REAL,
    estimated_if REAL,
    tags_json TEXT,
    created_at TEXT NOT NULL
);

-- Rides table
CREATE TABLE IF NOT EXISTS rides (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    workout_id TEXT REFERENCES workouts(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL,
    distance_meters REAL NOT NULL,
    avg_power INTEGER,
    max_power INTEGER,
    normalized_power INTEGER,
    intensity_factor REAL,
    tss REAL,
    avg_hr INTEGER,
    max_hr INTEGER,
    avg_cadence INTEGER,
    calories INTEGER NOT NULL,
    ftp_at_ride INTEGER NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rides_user_id ON rides(user_id);
CREATE INDEX IF NOT EXISTS idx_rides_started_at ON rides(started_at);

-- Ride samples table
CREATE TABLE IF NOT EXISTS ride_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    elapsed_seconds INTEGER NOT NULL,
    power_watts INTEGER,
    cadence_rpm INTEGER,
    heart_rate_bpm INTEGER,
    speed_kmh REAL,
    distance_meters REAL NOT NULL,
    calories INTEGER NOT NULL,
    resistance_level INTEGER,
    target_power INTEGER,
    trainer_grade REAL
);

CREATE INDEX IF NOT EXISTS idx_ride_samples_ride_id ON ride_samples(ride_id);

-- Auto-save table (crash recovery)
CREATE TABLE IF NOT EXISTS autosave (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ride_json TEXT NOT NULL,
    samples_json TEXT NOT NULL,
    saved_at TEXT NOT NULL
);
"#;

/// SQL for schema version tracking (migrations)
pub const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

/// Current schema version
pub const CURRENT_VERSION: i32 = 1;
