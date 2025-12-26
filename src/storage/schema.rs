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

/// SQL for migration from v2 to v3 (ML coaching tables)
pub const MIGRATION_V2_TO_V3: &str = r#"
-- Training Goals table
CREATE TABLE IF NOT EXISTS training_goals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    goal_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    target_date TEXT,
    target_metric_type TEXT,
    target_metric_value REAL,
    target_metric_current REAL,
    priority INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_training_goals_user_id ON training_goals(user_id);
CREATE INDEX IF NOT EXISTS idx_training_goals_status ON training_goals(user_id, status);

-- ML Prediction Cache table
CREATE TABLE IF NOT EXISTS ml_predictions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    prediction_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    confidence REAL NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    source TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ml_predictions_user_type ON ml_predictions(user_id, prediction_type);
CREATE INDEX IF NOT EXISTS idx_ml_predictions_expires ON ml_predictions(expires_at);

-- Workout Recommendations table
CREATE TABLE IF NOT EXISTS workout_recommendations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workout_id TEXT NOT NULL,
    workout_source TEXT NOT NULL,
    suitability_score REAL NOT NULL,
    reasoning TEXT NOT NULL,
    target_energy_systems TEXT NOT NULL,
    expected_tss REAL NOT NULL,
    goal_id TEXT REFERENCES training_goals(id) ON DELETE SET NULL,
    training_gap TEXT,
    recommended_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_workout_recommendations_user ON workout_recommendations(user_id);
CREATE INDEX IF NOT EXISTS idx_workout_recommendations_status ON workout_recommendations(user_id, status);

-- Performance Projections table
CREATE TABLE IF NOT EXISTS performance_projections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    projected_at TEXT NOT NULL,
    forecast_weeks INTEGER NOT NULL,
    data_points TEXT NOT NULL,
    plateau_detected INTEGER NOT NULL DEFAULT 0,
    detraining_risk TEXT NOT NULL,
    event_readiness TEXT
);

CREATE INDEX IF NOT EXISTS idx_performance_projections_user ON performance_projections(user_id);

-- Built-in Workout Library table
CREATE TABLE IF NOT EXISTS builtin_workouts (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    energy_systems TEXT NOT NULL,
    goal_alignment TEXT NOT NULL,
    difficulty_tier TEXT NOT NULL,
    duration_minutes INTEGER NOT NULL,
    base_tss REAL NOT NULL,
    segments TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_builtin_workouts_category ON builtin_workouts(category);
CREATE INDEX IF NOT EXISTS idx_builtin_workouts_difficulty ON builtin_workouts(difficulty_tier);

-- Fatigue States table (per-ride tracking)
CREATE TABLE IF NOT EXISTS fatigue_states (
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

CREATE INDEX IF NOT EXISTS idx_fatigue_states_ride ON fatigue_states(ride_id);
"#;

/// SQL for migration from v3 to v4 (3D World & Content tables)
pub const MIGRATION_V3_TO_V4: &str = r#"
-- Imported routes table
CREATE TABLE IF NOT EXISTS imported_routes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source TEXT NOT NULL,
    distance_meters REAL NOT NULL DEFAULT 0,
    elevation_gain_meters REAL NOT NULL DEFAULT 0,
    max_elevation_meters REAL NOT NULL DEFAULT 0,
    min_elevation_meters REAL NOT NULL DEFAULT 0,
    avg_gradient_percent REAL NOT NULL DEFAULT 0,
    max_gradient_percent REAL NOT NULL DEFAULT 0,
    source_file TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_imported_routes_name ON imported_routes(name);
CREATE INDEX IF NOT EXISTS idx_imported_routes_source ON imported_routes(source);

-- Route waypoints table
CREATE TABLE IF NOT EXISTS route_waypoints (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL REFERENCES imported_routes(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    elevation_meters REAL NOT NULL,
    distance_from_start REAL NOT NULL,
    gradient_percent REAL NOT NULL,
    surface_type TEXT NOT NULL DEFAULT 'asphalt'
);

CREATE INDEX IF NOT EXISTS idx_route_waypoints_route ON route_waypoints(route_id);
CREATE INDEX IF NOT EXISTS idx_route_waypoints_sequence ON route_waypoints(route_id, sequence);

-- Segments table (timed portions of routes)
CREATE TABLE IF NOT EXISTS segments (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL REFERENCES imported_routes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    start_distance_meters REAL NOT NULL,
    end_distance_meters REAL NOT NULL,
    length_meters REAL NOT NULL,
    elevation_gain_meters REAL NOT NULL,
    avg_gradient_percent REAL NOT NULL,
    category TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_segments_route ON segments(route_id);

-- Segment times (leaderboard entries)
CREATE TABLE IF NOT EXISTS segment_times (
    id TEXT PRIMARY KEY,
    segment_id TEXT NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    time_seconds REAL NOT NULL,
    avg_power_watts INTEGER,
    avg_heart_rate INTEGER,
    ftp_at_effort INTEGER NOT NULL,
    is_personal_best INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_segment_times_segment ON segment_times(segment_id);
CREATE INDEX IF NOT EXISTS idx_segment_times_user ON segment_times(user_id);
CREATE INDEX IF NOT EXISTS idx_segment_times_leaderboard ON segment_times(segment_id, time_seconds);

-- Landmarks table
CREATE TABLE IF NOT EXISTS landmarks (
    id TEXT PRIMARY KEY,
    route_id TEXT REFERENCES imported_routes(id) ON DELETE CASCADE,
    landmark_type TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    elevation_meters REAL NOT NULL,
    distance_meters REAL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_landmarks_route ON landmarks(route_id);

-- Landmark discoveries (user progress)
CREATE TABLE IF NOT EXISTS landmark_discoveries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    landmark_id TEXT NOT NULL REFERENCES landmarks(id) ON DELETE CASCADE,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    discovered_at TEXT NOT NULL,
    screenshot_path TEXT,
    UNIQUE(user_id, landmark_id)
);

CREATE INDEX IF NOT EXISTS idx_landmark_discoveries_user ON landmark_discoveries(user_id);

-- Achievements table
CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    tier TEXT NOT NULL,
    icon TEXT NOT NULL,
    is_secret INTEGER NOT NULL DEFAULT 0,
    target_value REAL
);

CREATE INDEX IF NOT EXISTS idx_achievements_category ON achievements(category);

-- Achievement progress table
CREATE TABLE IF NOT EXISTS achievement_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id TEXT NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    current_value REAL NOT NULL DEFAULT 0,
    is_unlocked INTEGER NOT NULL DEFAULT 0,
    unlocked_at TEXT,
    UNIQUE(user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_achievement_progress_user ON achievement_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_achievement_progress_unlocked ON achievement_progress(user_id, is_unlocked);

-- Collectibles table
CREATE TABLE IF NOT EXISTS collectibles (
    id TEXT PRIMARY KEY,
    route_id TEXT REFERENCES imported_routes(id) ON DELETE CASCADE,
    collectible_type TEXT NOT NULL,
    distance_meters REAL NOT NULL,
    position_x REAL NOT NULL,
    position_y REAL NOT NULL,
    position_z REAL NOT NULL,
    respawns INTEGER NOT NULL DEFAULT 1,
    respawn_time_seconds INTEGER
);

CREATE INDEX IF NOT EXISTS idx_collectibles_route ON collectibles(route_id);

-- Collected items (user collections)
CREATE TABLE IF NOT EXISTS collected_items (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    collectible_id TEXT NOT NULL REFERENCES collectibles(id) ON DELETE CASCADE,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    points INTEGER NOT NULL,
    collected_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_collected_items_user ON collected_items(user_id);

-- Custom routes (user-created)
CREATE TABLE IF NOT EXISTS custom_routes (
    id TEXT PRIMARY KEY,
    author_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_loop INTEGER NOT NULL DEFAULT 0,
    points_json TEXT NOT NULL,
    objects_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_custom_routes_author ON custom_routes(author_id);
"#;

/// SQL for schema version tracking (migrations)
pub const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

/// Current schema version
pub const CURRENT_VERSION: i32 = 5;

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

/// SQL for migration from v4 to v5 (Social & Multiplayer tables)
pub const MIGRATION_V4_TO_V5: &str = r#"
-- Rider profile (extends user concept for social features)
CREATE TABLE IF NOT EXISTS riders (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_id TEXT,
    bio TEXT,
    ftp INTEGER,
    total_distance_km REAL DEFAULT 0,
    total_time_hours REAL DEFAULT 0,
    sharing_enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Clubs
CREATE TABLE IF NOT EXISTS clubs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    join_code TEXT UNIQUE NOT NULL,
    admin_rider_id TEXT NOT NULL REFERENCES riders(id),
    total_distance_km REAL DEFAULT 0,
    total_time_hours REAL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- Club memberships
CREATE TABLE IF NOT EXISTS club_memberships (
    id TEXT PRIMARY KEY,
    club_id TEXT NOT NULL REFERENCES clubs(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    joined_at TEXT NOT NULL,
    left_at TEXT
);

-- Badges (seeded on first run)
CREATE TABLE IF NOT EXISTS badges (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    icon TEXT NOT NULL,
    category TEXT NOT NULL,
    criteria_type TEXT NOT NULL,
    criteria_value REAL NOT NULL
);

-- Earned badges
CREATE TABLE IF NOT EXISTS rider_badges (
    id TEXT PRIMARY KEY,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    badge_id TEXT NOT NULL REFERENCES badges(id),
    unlocked_at TEXT NOT NULL,
    UNIQUE(rider_id, badge_id)
);

-- Social segments (distinct from route segments in v4)
CREATE TABLE IF NOT EXISTS social_segments (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    name TEXT NOT NULL,
    start_distance_m REAL NOT NULL,
    end_distance_m REAL NOT NULL,
    category TEXT NOT NULL,
    elevation_gain_m REAL DEFAULT 0
);

-- Social segment efforts
CREATE TABLE IF NOT EXISTS social_segment_efforts (
    id TEXT PRIMARY KEY,
    segment_id TEXT NOT NULL REFERENCES social_segments(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    ride_id TEXT,
    elapsed_time_ms INTEGER NOT NULL,
    avg_power_watts INTEGER,
    avg_hr_bpm INTEGER,
    recorded_at TEXT NOT NULL,
    imported INTEGER DEFAULT 0,
    import_source_name TEXT
);

-- Challenges
CREATE TABLE IF NOT EXISTS challenges (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    goal_type TEXT NOT NULL,
    goal_value REAL NOT NULL,
    duration_days INTEGER NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    created_by_rider_id TEXT REFERENCES riders(id),
    created_at TEXT NOT NULL
);

-- Challenge progress
CREATE TABLE IF NOT EXISTS challenge_progress (
    id TEXT PRIMARY KEY,
    challenge_id TEXT NOT NULL REFERENCES challenges(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    current_value REAL DEFAULT 0,
    completed INTEGER DEFAULT 0,
    completed_at TEXT,
    last_updated TEXT NOT NULL,
    UNIQUE(challenge_id, rider_id)
);

-- Workout ratings
CREATE TABLE IF NOT EXISTS workout_ratings (
    id TEXT PRIMARY KEY,
    workout_id TEXT NOT NULL,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    rating INTEGER NOT NULL CHECK(rating >= 1 AND rating <= 5),
    review_text TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(workout_id, rider_id)
);

-- Race events
CREATE TABLE IF NOT EXISTS race_events (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    world_id TEXT NOT NULL,
    route_id TEXT NOT NULL,
    distance_km REAL NOT NULL,
    scheduled_start TEXT NOT NULL,
    status TEXT NOT NULL,
    organizer_rider_id TEXT NOT NULL REFERENCES riders(id),
    created_at TEXT NOT NULL
);

-- Race participants
CREATE TABLE IF NOT EXISTS race_participants (
    id TEXT PRIMARY KEY,
    race_id TEXT NOT NULL REFERENCES race_events(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    status TEXT NOT NULL,
    finish_time_ms INTEGER,
    finish_position INTEGER,
    joined_at TEXT NOT NULL,
    disconnected_at TEXT
);

-- Group rides
CREATE TABLE IF NOT EXISTS group_rides (
    id TEXT PRIMARY KEY,
    host_rider_id TEXT NOT NULL REFERENCES riders(id),
    name TEXT,
    world_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    max_participants INTEGER DEFAULT 10
);

-- Group ride participants
CREATE TABLE IF NOT EXISTS group_ride_participants (
    id TEXT PRIMARY KEY,
    group_ride_id TEXT NOT NULL REFERENCES group_rides(id),
    rider_id TEXT NOT NULL REFERENCES riders(id),
    joined_at TEXT NOT NULL,
    left_at TEXT
);

-- Chat messages
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    group_ride_id TEXT NOT NULL REFERENCES group_rides(id),
    sender_rider_id TEXT NOT NULL REFERENCES riders(id),
    message_text TEXT NOT NULL,
    sent_at TEXT NOT NULL
);

-- Activity summaries
CREATE TABLE IF NOT EXISTS activity_summaries (
    id TEXT PRIMARY KEY,
    ride_id TEXT,
    rider_id TEXT NOT NULL REFERENCES riders(id),
    rider_name TEXT NOT NULL,
    distance_km REAL NOT NULL,
    duration_minutes INTEGER NOT NULL,
    avg_power_watts INTEGER,
    elevation_gain_m REAL DEFAULT 0,
    world_id TEXT,
    recorded_at TEXT NOT NULL,
    shared INTEGER DEFAULT 1
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_social_segment_efforts_segment ON social_segment_efforts(segment_id);
CREATE INDEX IF NOT EXISTS idx_social_segment_efforts_rider ON social_segment_efforts(rider_id);
CREATE INDEX IF NOT EXISTS idx_challenge_progress_challenge ON challenge_progress(challenge_id);
CREATE INDEX IF NOT EXISTS idx_workout_ratings_workout ON workout_ratings(workout_id);
CREATE INDEX IF NOT EXISTS idx_activity_summaries_rider ON activity_summaries(rider_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_group_ride ON chat_messages(group_ride_id);
CREATE INDEX IF NOT EXISTS idx_group_ride_participants_ride ON group_ride_participants(group_ride_id);
CREATE INDEX IF NOT EXISTS idx_race_participants_race ON race_participants(race_id);
"#;
