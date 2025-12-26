# Data Model: 3D World & Content

**Feature Branch**: `005-3d-world-content`
**Date**: 2025-12-25

## Entity Definitions

### Route

A rideable path through a virtual world with GPS coordinates, elevation profile, and metadata.

```rust
/// Source type for the route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteSource {
    /// Imported from GPX/FIT/TCX file
    Imported,
    /// Pre-built famous cycling route
    Famous,
    /// Procedurally generated
    Generated,
    /// User-created with world editor
    Custom,
}

/// Route metadata and waypoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedRoute {
    /// Unique identifier
    pub id: Uuid,
    /// User who imported/owns this route
    pub user_id: Uuid,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Route source type
    pub source: RouteSource,
    /// Original file path (if imported)
    pub source_file: Option<String>,
    /// Total distance in meters
    pub total_distance_meters: f64,
    /// Total elevation gain in meters
    pub elevation_gain_meters: f64,
    /// Total elevation loss in meters
    pub elevation_loss_meters: f64,
    /// Maximum gradient percentage
    pub max_gradient_percent: f32,
    /// Average gradient percentage
    pub avg_gradient_percent: f32,
    /// Minimum elevation in meters
    pub min_elevation_meters: f32,
    /// Maximum elevation in meters
    pub max_elevation_meters: f32,
    /// Estimated completion time in seconds (based on FTP)
    pub estimated_time_seconds: Option<u32>,
    /// Difficulty rating (1-10)
    pub difficulty_rating: Option<u8>,
    /// Route tags for filtering
    pub tags: Vec<String>,
    /// Whether route is publicly shared
    pub is_public: bool,
    /// When imported/created
    pub created_at: DateTime<Utc>,
    /// Last modification time
    pub updated_at: DateTime<Utc>,
}

/// A point along a route with full position and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteWaypoint {
    /// Route this waypoint belongs to
    pub route_id: Uuid,
    /// Sequence number (0-indexed)
    pub sequence: u32,
    /// Latitude in degrees
    pub latitude: f64,
    /// Longitude in degrees
    pub longitude: f64,
    /// Elevation in meters (may be fetched from API)
    pub elevation_meters: f32,
    /// Distance from route start in meters
    pub distance_from_start_meters: f64,
    /// Gradient at this point as percentage
    pub gradient_percent: f32,
    /// Original timestamp from GPS (if available)
    pub timestamp: Option<DateTime<Utc>>,
}
```

**Validation Rules**:
- `name` must be 1-200 characters
- `total_distance_meters` must be > 0 and <= 500,000 (500km limit)
- `gradient_percent` must be -50.0 to +50.0 (reasonable limits)
- `difficulty_rating` must be 1-10 if present

**State Transitions**: N/A (immutable after import, deletable)

---

### Segment

A defined portion of a route used for timed efforts and leaderboard competition.

```rust
/// A timed segment on a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique identifier
    pub id: Uuid,
    /// Route this segment belongs to
    pub route_id: Uuid,
    /// Display name
    pub name: String,
    /// Start distance from route start (meters)
    pub start_distance_meters: f64,
    /// End distance from route start (meters)
    pub end_distance_meters: f64,
    /// Segment length in meters
    pub length_meters: f64,
    /// Elevation gain over segment
    pub elevation_gain_meters: f32,
    /// Average gradient over segment
    pub avg_gradient_percent: f32,
    /// Category (HC, 1, 2, 3, 4, Sprint, None)
    pub category: Option<SegmentCategory>,
    /// When created
    pub created_at: DateTime<Utc>,
}

/// Climbing category (Tour de France style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentCategory {
    /// Hors Categorie (beyond category)
    HC,
    /// Category 1 - hardest regular climb
    Cat1,
    /// Category 2
    Cat2,
    /// Category 3
    Cat3,
    /// Category 4 - easiest climb
    Cat4,
    /// Sprint segment (flat/downhill)
    Sprint,
}

/// A user's recorded time on a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentTime {
    /// Unique identifier
    pub id: Uuid,
    /// Segment this time is for
    pub segment_id: Uuid,
    /// User who recorded this time
    pub user_id: Uuid,
    /// Ride during which this was recorded
    pub ride_id: Uuid,
    /// Time in seconds (to 0.1s precision)
    pub time_seconds: f64,
    /// Average power during segment (if available)
    pub avg_power_watts: Option<u16>,
    /// Average heart rate during segment
    pub avg_heart_rate: Option<u8>,
    /// FTP at time of effort (for relative comparison)
    pub ftp_at_effort: u16,
    /// Whether this is user's personal best
    pub is_personal_best: bool,
    /// When recorded
    pub recorded_at: DateTime<Utc>,
}
```

**Validation Rules**:
- `start_distance_meters` < `end_distance_meters`
- `length_meters` = `end_distance_meters` - `start_distance_meters`
- `time_seconds` must be > 0

**Relationships**:
- Segment belongs to Route (many-to-one)
- SegmentTime belongs to Segment, User, Ride (many-to-one each)

---

### Weather State

Current environmental conditions for the virtual world.

```rust
/// Weather condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WeatherType {
    #[default]
    Clear,
    Cloudy,
    Rain,
    HeavyRain,
    Fog,
    Snow,
}

/// Time of day period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeOfDay {
    Dawn,
    #[default]
    Day,
    Dusk,
    Night,
}

/// Complete weather state for the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherState {
    /// Current weather type
    pub weather: WeatherType,
    /// Weather transition progress (0.0 = previous, 1.0 = current)
    pub transition_progress: f32,
    /// Previous weather (for transitions)
    pub previous_weather: Option<WeatherType>,
    /// Current time of day
    pub time_of_day: TimeOfDay,
    /// Exact time (0.0-24.0 hours)
    pub time_hours: f32,
    /// Whether time progresses realistically
    pub realistic_time: bool,
    /// Visibility distance in meters (affected by fog/rain)
    pub visibility_meters: f32,
    /// Wind speed in km/h (for visual effects)
    pub wind_speed_kmh: f32,
    /// Wind direction in degrees (0 = north)
    pub wind_direction_degrees: f32,
}
```

**State Transitions**:
- Weather: Clear ↔ Cloudy ↔ Rain ↔ HeavyRain, Clear ↔ Fog, Clear ↔ Snow
- TimeOfDay: Dawn → Day → Dusk → Night → Dawn (cyclic)

---

### NPC Cyclist

AI-controlled virtual rider.

```rust
/// NPC difficulty level relative to user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NpcDifficulty {
    /// 50% of user FTP
    Easy,
    #[default]
    /// 80% of user FTP
    Medium,
    /// 100% of user FTP
    MatchUser,
    /// 110% of user FTP
    Hard,
    /// 130% of user FTP
    VeryHard,
}

/// Runtime state of an NPC cyclist
#[derive(Debug, Clone)]
pub struct NpcCyclist {
    /// Unique identifier for this NPC instance
    pub id: u32,
    /// Current position on route (distance from start)
    pub distance_meters: f64,
    /// Current speed in m/s
    pub speed_mps: f32,
    /// Target power based on difficulty
    pub target_power_watts: u16,
    /// Current simulated power (with variation)
    pub current_power_watts: u16,
    /// Visual appearance (jersey color index)
    pub appearance_index: u8,
    /// Whether NPC has been passed by user
    pub passed_by_user: bool,
    /// Whether user is currently drafting this NPC
    pub user_drafting: bool,
}

/// NPC system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcSettings {
    /// Whether NPCs are enabled
    pub enabled: bool,
    /// Number of NPCs to spawn
    pub count: u8,
    /// Difficulty level
    pub difficulty: NpcDifficulty,
    /// Whether to show NPC names
    pub show_names: bool,
}
```

---

### Landmark

Point of interest on a route.

```rust
/// Landmark type for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandmarkType {
    /// Mountain summit
    Summit,
    /// Historical monument
    Monument,
    /// Scenic viewpoint
    Viewpoint,
    /// Town or village
    Town,
    /// Sprint point
    SprintPoint,
    /// Kilometer marker
    KmMarker,
    /// Custom/other
    Custom,
}

/// A discoverable point of interest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    /// Unique identifier
    pub id: Uuid,
    /// Route this landmark is on
    pub route_id: Uuid,
    /// Display name
    pub name: String,
    /// Description / historical info
    pub description: Option<String>,
    /// Landmark type
    pub landmark_type: LandmarkType,
    /// Distance from route start (meters)
    pub distance_meters: f64,
    /// Elevation at landmark
    pub elevation_meters: f32,
    /// Image asset path (optional)
    pub image_path: Option<String>,
    /// Achievement awarded on discovery (optional)
    pub achievement_id: Option<Uuid>,
}

/// Record of user discovering a landmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandmarkDiscovery {
    /// Unique identifier
    pub id: Uuid,
    /// Landmark discovered
    pub landmark_id: Uuid,
    /// User who discovered it
    pub user_id: Uuid,
    /// Ride during which it was discovered
    pub ride_id: Uuid,
    /// When discovered
    pub discovered_at: DateTime<Utc>,
}
```

---

### Achievement

Accomplishment with criteria and progress tracking.

```rust
/// Achievement category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AchievementCategory {
    /// Distance-based (ride X km)
    Distance,
    /// Elevation-based (climb X meters)
    Climbing,
    /// Segment-based (complete X segments)
    Segments,
    /// Landmark-based (discover X landmarks)
    Exploration,
    /// Time-based (ride for X hours)
    Endurance,
    /// Social (pass X NPCs)
    Social,
    /// Collection (collect X items)
    Collection,
    /// Special/hidden achievements
    Special,
}

/// Achievement rarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AchievementRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// Achievement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Description of how to earn
    pub description: String,
    /// Category
    pub category: AchievementCategory,
    /// Rarity level
    pub rarity: AchievementRarity,
    /// Badge image path
    pub badge_path: String,
    /// Target value to complete (e.g., 1000 for "Ride 1000km")
    pub target_value: u32,
    /// Whether this is a hidden achievement
    pub is_hidden: bool,
    /// Points awarded
    pub points: u32,
}

/// User progress toward an achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementProgress {
    /// Unique identifier
    pub id: Uuid,
    /// Achievement this progress is for
    pub achievement_id: Uuid,
    /// User
    pub user_id: Uuid,
    /// Current progress value
    pub current_value: u32,
    /// Whether completed
    pub is_complete: bool,
    /// When completed (if complete)
    pub completed_at: Option<DateTime<Utc>>,
    /// When progress was last updated
    pub updated_at: DateTime<Utc>,
}
```

---

### Collectible

In-world item that can be picked up.

```rust
/// Collectible type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectibleType {
    /// Star for points
    Star,
    /// Powerup (visual only)
    Powerup,
    /// Jersey unlock
    Jersey,
    /// Badge component
    Badge,
    /// Hidden item
    Hidden,
}

/// A collectible item in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collectible {
    /// Unique identifier
    pub id: Uuid,
    /// Route this collectible is on
    pub route_id: Uuid,
    /// Collectible type
    pub collectible_type: CollectibleType,
    /// Distance from route start (meters)
    pub distance_meters: f64,
    /// Offset from center of road (meters, negative=left)
    pub lateral_offset_meters: f32,
    /// Points awarded
    pub points: u32,
    /// Achievement unlocked on collection (optional)
    pub achievement_id: Option<Uuid>,
}

/// Record of user collecting an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiblePickup {
    /// Unique identifier
    pub id: Uuid,
    /// Collectible picked up
    pub collectible_id: Uuid,
    /// User who collected it
    pub user_id: Uuid,
    /// Ride during which it was collected
    pub ride_id: Uuid,
    /// When collected
    pub collected_at: DateTime<Utc>,
}
```

---

### Difficulty Modifier

Configuration for adjusting route difficulty.

```rust
/// Difficulty modifier settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyModifier {
    /// Gradient scaling factor (0.0 = flat, 1.0 = original, 2.0 = doubled)
    pub gradient_scale: f32,
    /// Whether adaptive scaling is enabled
    pub adaptive_enabled: bool,
    /// Target difficulty (if adaptive, normalizes to this TSS/hour)
    pub adaptive_target_tss_per_hour: Option<f32>,
    /// Apply to descents as well (or keep descents unmodified)
    pub apply_to_descents: bool,
}

impl Default for DifficultyModifier {
    fn default() -> Self {
        Self {
            gradient_scale: 1.0,
            adaptive_enabled: false,
            adaptive_target_tss_per_hour: None,
            apply_to_descents: true,
        }
    }
}
```

---

### World Seed

For procedurally generated worlds.

```rust
/// Biome type for procedural generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Biome {
    #[default]
    Countryside,
    Alpine,
    Desert,
    Coastal,
    Forest,
}

/// Configuration for procedural world generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeed {
    /// Seed value (text or numeric)
    pub seed: String,
    /// Numeric seed derived from string
    pub seed_value: u64,
    /// Selected biome
    pub biome: Biome,
    /// Route length to generate (meters)
    pub target_length_meters: f64,
    /// Maximum gradient for generated terrain
    pub max_gradient_percent: f32,
    /// Elevation variation scale
    pub elevation_scale: f32,
}

impl WorldSeed {
    pub fn from_string(seed: &str, biome: Biome, length: f64) -> Self {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let seed_value = hasher.finish();

        Self {
            seed: seed.to_string(),
            seed_value,
            biome,
            target_length_meters: length,
            max_gradient_percent: 15.0,
            elevation_scale: 1.0,
        }
    }
}
```

---

### Drafting State

Visual drafting indicator state.

```rust
/// Drafting detection result
#[derive(Debug, Clone)]
pub struct DraftingState {
    /// Whether currently in draft zone
    pub is_drafting: bool,
    /// ID of cyclist being drafted (NPC or player in multiplayer)
    pub drafting_behind: Option<u32>,
    /// Calculated benefit percentage (0-30%)
    pub benefit_percent: f32,
    /// Time spent drafting in current session (seconds)
    pub total_draft_time_seconds: f32,
    /// Estimated energy saved (kJ)
    pub energy_saved_kj: f32,
}
```

---

## Entity Relationships

```
User (existing)
  ├── ImportedRoute (one-to-many)
  │     ├── RouteWaypoint (one-to-many)
  │     ├── Segment (one-to-many)
  │     │     └── SegmentTime (one-to-many, per user)
  │     ├── Landmark (one-to-many)
  │     │     └── LandmarkDiscovery (one-to-many, per user)
  │     └── Collectible (one-to-many)
  │           └── CollectiblePickup (one-to-many, per user)
  ├── AchievementProgress (one-to-many)
  └── Settings
        ├── NpcSettings
        ├── WeatherPreferences
        └── DifficultyModifier

Achievement (standalone)
  └── AchievementProgress (one-to-many)
```

## Database Schema (SQLite)

```sql
-- Routes table
CREATE TABLE IF NOT EXISTS routes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    source TEXT NOT NULL,
    source_file TEXT,
    total_distance_meters REAL NOT NULL,
    elevation_gain_meters REAL NOT NULL,
    elevation_loss_meters REAL NOT NULL,
    max_gradient_percent REAL NOT NULL,
    avg_gradient_percent REAL NOT NULL,
    min_elevation_meters REAL NOT NULL,
    max_elevation_meters REAL NOT NULL,
    estimated_time_seconds INTEGER,
    difficulty_rating INTEGER,
    tags_json TEXT,
    is_public INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_routes_user_id ON routes(user_id);
CREATE INDEX idx_routes_source ON routes(source);

-- Route waypoints table
CREATE TABLE IF NOT EXISTS route_waypoints (
    route_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    elevation_meters REAL NOT NULL,
    distance_from_start_meters REAL NOT NULL,
    gradient_percent REAL NOT NULL,
    timestamp TEXT,
    PRIMARY KEY (route_id, sequence),
    FOREIGN KEY (route_id) REFERENCES routes(id) ON DELETE CASCADE
);

-- Segments table
CREATE TABLE IF NOT EXISTS segments (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL,
    name TEXT NOT NULL,
    start_distance_meters REAL NOT NULL,
    end_distance_meters REAL NOT NULL,
    length_meters REAL NOT NULL,
    elevation_gain_meters REAL NOT NULL,
    avg_gradient_percent REAL NOT NULL,
    category TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (route_id) REFERENCES routes(id) ON DELETE CASCADE
);

CREATE INDEX idx_segments_route_id ON segments(route_id);

-- Segment times table
CREATE TABLE IF NOT EXISTS segment_times (
    id TEXT PRIMARY KEY,
    segment_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    ride_id TEXT NOT NULL,
    time_seconds REAL NOT NULL,
    avg_power_watts INTEGER,
    avg_heart_rate INTEGER,
    ftp_at_effort INTEGER NOT NULL,
    is_personal_best INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (segment_id) REFERENCES segments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (ride_id) REFERENCES rides(id) ON DELETE CASCADE
);

CREATE INDEX idx_segment_times_segment_id ON segment_times(segment_id);
CREATE INDEX idx_segment_times_user_id ON segment_times(user_id);
CREATE INDEX idx_segment_times_time ON segment_times(time_seconds);

-- Landmarks table
CREATE TABLE IF NOT EXISTS landmarks (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    landmark_type TEXT NOT NULL,
    distance_meters REAL NOT NULL,
    elevation_meters REAL NOT NULL,
    image_path TEXT,
    achievement_id TEXT,
    FOREIGN KEY (route_id) REFERENCES routes(id) ON DELETE CASCADE
);

CREATE INDEX idx_landmarks_route_id ON landmarks(route_id);

-- Landmark discoveries table
CREATE TABLE IF NOT EXISTS landmark_discoveries (
    id TEXT PRIMARY KEY,
    landmark_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    ride_id TEXT NOT NULL,
    discovered_at TEXT NOT NULL,
    FOREIGN KEY (landmark_id) REFERENCES landmarks(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (ride_id) REFERENCES rides(id) ON DELETE CASCADE,
    UNIQUE (landmark_id, user_id)
);

-- Achievements table
CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    rarity TEXT NOT NULL,
    badge_path TEXT NOT NULL,
    target_value INTEGER NOT NULL,
    is_hidden INTEGER NOT NULL DEFAULT 0,
    points INTEGER NOT NULL DEFAULT 0
);

-- Achievement progress table
CREATE TABLE IF NOT EXISTS achievement_progress (
    id TEXT PRIMARY KEY,
    achievement_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    current_value INTEGER NOT NULL DEFAULT 0,
    is_complete INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (achievement_id) REFERENCES achievements(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE (achievement_id, user_id)
);

CREATE INDEX idx_achievement_progress_user_id ON achievement_progress(user_id);

-- Collectibles table
CREATE TABLE IF NOT EXISTS collectibles (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL,
    collectible_type TEXT NOT NULL,
    distance_meters REAL NOT NULL,
    lateral_offset_meters REAL NOT NULL DEFAULT 0,
    points INTEGER NOT NULL DEFAULT 0,
    achievement_id TEXT,
    FOREIGN KEY (route_id) REFERENCES routes(id) ON DELETE CASCADE
);

CREATE INDEX idx_collectibles_route_id ON collectibles(route_id);

-- Collectible pickups table
CREATE TABLE IF NOT EXISTS collectible_pickups (
    id TEXT PRIMARY KEY,
    collectible_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    ride_id TEXT NOT NULL,
    collected_at TEXT NOT NULL,
    FOREIGN KEY (collectible_id) REFERENCES collectibles(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (ride_id) REFERENCES rides(id) ON DELETE CASCADE,
    UNIQUE (collectible_id, user_id)
);
```

## Data Volume Assumptions

| Entity | Expected Volume | Storage Estimate |
|--------|-----------------|------------------|
| Routes | 100-500 per user | ~100KB per route (with waypoints) |
| Waypoints | 1000-10000 per route | ~50 bytes per waypoint |
| Segments | 1-10 per route | ~500 bytes per segment |
| Segment Times | 100-1000 per user | ~100 bytes per time |
| Landmarks | 0-50 per route | ~500 bytes per landmark |
| Achievements | 50+ (system-wide) | ~200 bytes per achievement |
| Achievement Progress | 50 per user | ~50 bytes per progress |
| Collectibles | 10-100 per route | ~50 bytes per collectible |
