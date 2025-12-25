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

-- Avatar customization table
CREATE TABLE IF NOT EXISTS avatars (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    jersey_color TEXT NOT NULL DEFAULT '#FF0000',
    jersey_secondary TEXT,
    bike_style TEXT NOT NULL DEFAULT 'road_bike',
    helmet_color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id)
);

-- Power Duration Curve points table
CREATE TABLE IF NOT EXISTS pdc_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    duration_secs INTEGER NOT NULL,
    power_watts INTEGER NOT NULL,
    achieved_at TEXT NOT NULL,
    ride_id TEXT REFERENCES rides(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, duration_secs)
);

CREATE INDEX IF NOT EXISTS idx_pdc_points_user_id ON pdc_points(user_id);
CREATE INDEX IF NOT EXISTS idx_pdc_points_duration ON pdc_points(user_id, duration_secs);

-- Critical Power model history table
CREATE TABLE IF NOT EXISTS cp_models (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cp_watts INTEGER NOT NULL,
    w_prime_joules INTEGER NOT NULL,
    r_squared REAL NOT NULL,
    calculated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cp_models_user_id ON cp_models(user_id);
CREATE INDEX IF NOT EXISTS idx_cp_models_current ON cp_models(user_id, is_current);

-- FTP estimates table
CREATE TABLE IF NOT EXISTS ftp_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ftp_watts INTEGER NOT NULL,
    method TEXT NOT NULL,
    confidence TEXT NOT NULL,
    supporting_data_json TEXT,
    detected_at TEXT NOT NULL,
    accepted INTEGER NOT NULL DEFAULT 0,
    accepted_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ftp_estimates_user_id ON ftp_estimates(user_id);
CREATE INDEX IF NOT EXISTS idx_ftp_estimates_accepted ON ftp_estimates(user_id, accepted);

-- Daily training load table
CREATE TABLE IF NOT EXISTS daily_training_load (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    tss REAL NOT NULL,
    atl REAL NOT NULL,
    ctl REAL NOT NULL,
    tsb REAL NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, date)
);

CREATE INDEX IF NOT EXISTS idx_daily_training_load_user_date ON daily_training_load(user_id, date);

-- VO2max estimates table
CREATE TABLE IF NOT EXISTS vo2max_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vo2max REAL NOT NULL,
    method TEXT NOT NULL,
    classification TEXT NOT NULL,
    estimated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vo2max_estimates_user_id ON vo2max_estimates(user_id);
CREATE INDEX IF NOT EXISTS idx_vo2max_estimates_current ON vo2max_estimates(user_id, is_current);

-- Rider analytics profile table
CREATE TABLE IF NOT EXISTS rider_profiles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rider_type TEXT NOT NULL,
    neuromuscular_pct REAL NOT NULL,
    anaerobic_pct REAL NOT NULL,
    vo2max_pct REAL NOT NULL,
    threshold_pct REAL NOT NULL DEFAULT 100.0,
    calculated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, is_current) -- Only one current profile per user
);

CREATE INDEX IF NOT EXISTS idx_rider_profiles_user_id ON rider_profiles(user_id);
"#;

/// SQL for schema version tracking (migrations)
pub const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

/// Current schema version
pub const CURRENT_VERSION: i32 = 2;

/// SQL for migration from v1 to v2 (analytics tables)
pub const MIGRATION_V1_TO_V2: &str = r#"
-- Power Duration Curve points table
CREATE TABLE IF NOT EXISTS pdc_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    duration_secs INTEGER NOT NULL,
    power_watts INTEGER NOT NULL,
    achieved_at TEXT NOT NULL,
    ride_id TEXT REFERENCES rides(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, duration_secs)
);

CREATE INDEX IF NOT EXISTS idx_pdc_points_user_id ON pdc_points(user_id);
CREATE INDEX IF NOT EXISTS idx_pdc_points_duration ON pdc_points(user_id, duration_secs);

-- Critical Power model history table
CREATE TABLE IF NOT EXISTS cp_models (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cp_watts INTEGER NOT NULL,
    w_prime_joules INTEGER NOT NULL,
    r_squared REAL NOT NULL,
    calculated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cp_models_user_id ON cp_models(user_id);
CREATE INDEX IF NOT EXISTS idx_cp_models_current ON cp_models(user_id, is_current);

-- FTP estimates table
CREATE TABLE IF NOT EXISTS ftp_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ftp_watts INTEGER NOT NULL,
    method TEXT NOT NULL,
    confidence TEXT NOT NULL,
    supporting_data_json TEXT,
    detected_at TEXT NOT NULL,
    accepted INTEGER NOT NULL DEFAULT 0,
    accepted_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ftp_estimates_user_id ON ftp_estimates(user_id);
CREATE INDEX IF NOT EXISTS idx_ftp_estimates_accepted ON ftp_estimates(user_id, accepted);

-- Daily training load table
CREATE TABLE IF NOT EXISTS daily_training_load (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    tss REAL NOT NULL,
    atl REAL NOT NULL,
    ctl REAL NOT NULL,
    tsb REAL NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, date)
);

CREATE INDEX IF NOT EXISTS idx_daily_training_load_user_date ON daily_training_load(user_id, date);

-- VO2max estimates table
CREATE TABLE IF NOT EXISTS vo2max_estimates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vo2max REAL NOT NULL,
    method TEXT NOT NULL,
    classification TEXT NOT NULL,
    estimated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vo2max_estimates_user_id ON vo2max_estimates(user_id);
CREATE INDEX IF NOT EXISTS idx_vo2max_estimates_current ON vo2max_estimates(user_id, is_current);

-- Rider analytics profile table
CREATE TABLE IF NOT EXISTS rider_profiles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rider_type TEXT NOT NULL,
    neuromuscular_pct REAL NOT NULL,
    anaerobic_pct REAL NOT NULL,
    vo2max_pct REAL NOT NULL,
    threshold_pct REAL NOT NULL DEFAULT 100.0,
    calculated_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rider_profiles_user_id ON rider_profiles(user_id);
"#;
