# Data Model: Competitive Feature Gaps

**Feature**: 010-competitive-features
**Date**: 2025-12-28

## Entity Relationship Diagram

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│  Achievement    │       │ UserAchievement │       │    UserXP       │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ key: String PK  │◄──────│ achievement_key │       │ user_id: i64 PK │
│ name: String    │       │ user_id: i64    │       │ total_xp: u64   │
│ description     │       │ earned_at: DateTime│    │ current_level   │
│ category        │       │ ride_id: Option │       │ updated_at      │
│ tier            │       └─────────────────┘       └─────────────────┘
│ xp_value: u32   │                                          │
│ target: Option  │                                          │
│ is_secret: bool │                                          ▼
└─────────────────┘                                 ┌─────────────────┐
                                                    │   UserReward    │
┌─────────────────┐       ┌─────────────────┐       ├─────────────────┤
│   GPXRoute      │       │ GradientSettings│       │ user_id: i64    │
├─────────────────┤       ├─────────────────┤       │ reward_type     │
│ id: Uuid PK     │       │ user_id: i64 PK │       │ reward_id       │
│ name: String    │       │ difficulty_pct  │       │ unlocked_at     │
│ source_file     │       │ max_gradient    │       └─────────────────┘
│ total_distance  │       │ min_gradient    │
│ total_elevation │       │ smoothing_secs  │
│ points: Vec     │       └─────────────────┘
│ gradients: Vec  │
│ imported_at     │       ┌─────────────────┐       ┌─────────────────┐
└─────────────────┘       │  PowerProfile   │       │PowerProfilePoint│
                          ├─────────────────┤       ├─────────────────┤
┌─────────────────┐       │ user_id: i64    │◄──────│ profile_id      │
│ TrainingPlan    │       │ profile_type    │       │ duration_secs   │
├─────────────────┤       │ recorded_at     │       │ power_watts     │
│ id: Uuid PK     │       │ is_current      │       │ achieved_at     │
│ name: String    │       └─────────────────┘       │ ride_id         │
│ discipline      │                                 └─────────────────┘
│ duration_weeks  │
│ workouts_per_wk │       ┌─────────────────┐
│ description     │       │PlanAssignment   │
│ difficulty_lvl  │       ├─────────────────┤
└────────┬────────┘       │ user_id: i64 PK │
         │                │ plan_id: Uuid   │
         ▼                │ started_at      │
┌─────────────────┐       │ current_week    │
│PlanWorkout      │       │ completed_workouts│
├─────────────────┤       │ status          │
│ plan_id: Uuid   │       └─────────────────┘
│ week_number     │
│ day_of_week     │
│ workout_id      │
│ is_optional     │
└─────────────────┘
```

---

## Entities

### 1. Achievement (Extended)

Extends existing `world/achievements/definitions.rs::Achievement`.

```rust
pub struct Achievement {
    /// Unique identifier (e.g., "distance_100km")
    pub key: String,
    /// Display name
    pub name: String,
    /// Description of how to earn
    pub description: String,
    /// Category for grouping
    pub category: AchievementCategory,
    /// Difficulty tier
    pub tier: AchievementTier,
    /// Target value for progress tracking (optional)
    pub target: Option<f64>,
    /// Whether hidden until earned
    pub is_secret: bool,
    /// NEW: XP awarded on completion
    pub xp_value: u32,
}

pub enum AchievementCategory {
    Distance,
    Climbing,
    Consistency,
    Competition,
    Exploration,
    Training,
    Special,
    Power,      // NEW: Power-related achievements
}

pub enum AchievementTier {
    Bronze,     // 100 XP base
    Silver,     // 250 XP base
    Gold,       // 500 XP base
    Diamond,    // 1000 XP base
    Legendary,  // 2500 XP base
}
```

**Validation Rules**:
- `key` must be unique, lowercase, alphanumeric with underscores
- `xp_value` defaults to tier base value if not specified
- Secret achievements get 1.5x XP multiplier

---

### 2. UserAchievement

Tracks earned achievements per user.

```rust
pub struct UserAchievement {
    /// Achievement key reference
    pub achievement_key: String,
    /// User who earned it (default: 1 for single-user app)
    pub user_id: i64,
    /// When the achievement was earned
    pub earned_at: DateTime<Utc>,
    /// Ride that triggered the achievement (if applicable)
    pub ride_id: Option<Uuid>,
    /// Progress at time of earning (for cumulative achievements)
    pub progress_value: Option<f64>,
}
```

**Validation Rules**:
- Composite primary key: `(user_id, achievement_key)`
- `earned_at` must be non-future
- `ride_id` references `rides.id` if present

---

### 3. UserXP

Tracks experience points and level for a user.

```rust
pub struct UserXP {
    /// User identifier
    pub user_id: i64,
    /// Total accumulated XP
    pub total_xp: u64,
    /// Current career level (1-50)
    pub current_level: u32,
    /// XP needed for next level
    pub xp_to_next_level: u64,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl UserXP {
    /// Calculate XP required for a given level
    pub fn xp_for_level(level: u32) -> u64 {
        if level <= 1 { return 0; }
        // Exponential curve: 1000 * 1.15^(level-1)
        (1000.0 * 1.15_f64.powi(level as i32 - 1)) as u64
    }

    /// Calculate cumulative XP needed to reach a level
    pub fn cumulative_xp_for_level(level: u32) -> u64 {
        (1..level).map(Self::xp_for_level).sum()
    }
}
```

**Validation Rules**:
- `current_level` range: 1-50
- `total_xp` must be consistent with `current_level`
- Single row per user

---

### 4. UserReward

Tracks unlocked cosmetic rewards.

```rust
pub struct UserReward {
    /// User identifier
    pub user_id: i64,
    /// Type of reward
    pub reward_type: RewardType,
    /// Specific reward identifier
    pub reward_id: String,
    /// When unlocked
    pub unlocked_at: DateTime<Utc>,
    /// Level at which it was unlocked
    pub unlocked_at_level: u32,
}

pub enum RewardType {
    JerseyColor,
    BikeFrame,
    UiTheme,
    AccentColor,
    Badge,
}
```

**Validation Rules**:
- Composite primary key: `(user_id, reward_type, reward_id)`
- `unlocked_at_level` must match level requirements

---

### 5. GradientSettings

User preferences for gradient simulation.

```rust
pub struct GradientSettings {
    /// User identifier
    pub user_id: i64,
    /// Trainer difficulty multiplier (0-100%)
    pub difficulty_percent: u8,
    /// Maximum positive gradient (default: 15%)
    pub max_gradient: f32,
    /// Maximum negative gradient (default: -15%)
    pub min_gradient: f32,
    /// Smoothing window in seconds (default: 3)
    pub smoothing_secs: u8,
    /// Rolling resistance coefficient (default: 0.004)
    pub rolling_resistance: f32,
}
```

**Validation Rules**:
- `difficulty_percent` range: 0-100
- `max_gradient` range: 0-25
- `min_gradient` range: -25-0
- `smoothing_secs` range: 0-10

---

### 6. GPXRoute

Parsed and cached GPX route with gradient data.

```rust
pub struct GPXRoute {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Original file path
    pub source_file: PathBuf,
    /// Total route distance in meters
    pub total_distance_m: f64,
    /// Total elevation gain in meters
    pub total_elevation_m: f64,
    /// Parsed GPS points
    pub points: Vec<RoutePoint>,
    /// Pre-calculated gradients between points
    pub gradients: Vec<GradientSegment>,
    /// When imported
    pub imported_at: DateTime<Utc>,
}

pub struct RoutePoint {
    /// Distance from start in meters
    pub distance_m: f64,
    /// Elevation in meters
    pub elevation_m: f32,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
}

pub struct GradientSegment {
    /// Start distance in meters
    pub start_distance_m: f64,
    /// End distance in meters
    pub end_distance_m: f64,
    /// Gradient percentage
    pub gradient_percent: f32,
}
```

**Validation Rules**:
- At least 2 points required
- Points must have elevation data (or default to 0)
- Gradients calculated on import, not stored in DB

---

### 7. PowerProfile

Stores power duration curve snapshots.

```rust
pub struct PowerProfile {
    /// Profile identifier
    pub id: i64,
    /// User identifier
    pub user_id: i64,
    /// Type: "current" (90-day rolling) or "lifetime"
    pub profile_type: PowerProfileType,
    /// When this snapshot was recorded
    pub recorded_at: DateTime<Utc>,
    /// Whether this is the active current profile
    pub is_current: bool,
    /// Power points at standard durations
    pub points: Vec<PowerProfilePoint>,
}

pub enum PowerProfileType {
    Current,   // Rolling 90-day window
    Lifetime,  // All-time bests
}

pub struct PowerProfilePoint {
    /// Duration in seconds
    pub duration_secs: u32,
    /// Best average power at this duration
    pub power_watts: u16,
    /// When this power was achieved
    pub achieved_at: DateTime<Utc>,
    /// Ride where this was achieved
    pub ride_id: Option<Uuid>,
}
```

**Validation Rules**:
- Standard durations: 5, 15, 30, 60, 180, 300, 600, 1200, 3600 seconds
- Only one `is_current = true` per user per type
- `current` profile points must be within 90 days

---

### 8. TrainingPlan

Pre-built training plan definitions.

```rust
pub struct TrainingPlan {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Target discipline
    pub discipline: Discipline,
    /// Plan duration in weeks
    pub duration_weeks: u8,
    /// Recommended workouts per week
    pub workouts_per_week: u8,
    /// Plan description
    pub description: String,
    /// Difficulty level
    pub difficulty: DifficultyLevel,
    /// Ordered list of weekly workouts
    pub workouts: Vec<PlanWorkout>,
}

pub enum Discipline {
    Road,
    Gravel,
    Triathlon,
    MTB,
    GeneralFitness,
}

pub enum DifficultyLevel {
    Beginner,
    Intermediate,
    Advanced,
}

pub struct PlanWorkout {
    /// Week number (1-based)
    pub week: u8,
    /// Day within week (1=Monday, 7=Sunday)
    pub day_of_week: u8,
    /// Reference to workout definition
    pub workout_id: Uuid,
    /// Whether workout can be skipped
    pub is_optional: bool,
    /// Alternative workout if primary not available
    pub alternative_id: Option<Uuid>,
}
```

**Validation Rules**:
- `duration_weeks` range: 1-52
- `workouts_per_week` range: 1-7
- `day_of_week` range: 1-7

---

### 9. PlanAssignment

Tracks user's active training plan progress.

```rust
pub struct PlanAssignment {
    /// User identifier
    pub user_id: i64,
    /// Assigned plan
    pub plan_id: Uuid,
    /// When the plan was started
    pub started_at: DateTime<Utc>,
    /// Current week (1-based)
    pub current_week: u8,
    /// Completed workout count
    pub completed_workouts: u32,
    /// Skipped workout count
    pub skipped_workouts: u32,
    /// Assignment status
    pub status: PlanStatus,
    /// Available training days (bitmask: Mon=1, Tue=2, Wed=4...)
    pub available_days: u8,
}

pub enum PlanStatus {
    Active,
    Paused,
    Completed,
    Abandoned,
}
```

**Validation Rules**:
- One active assignment per user
- `current_week` <= plan's `duration_weeks`
- `available_days` must have at least 1 day set

---

## Database Schema (SQLite)

```sql
-- Achievement XP (extends conceptual model, actual achievements are in code)
-- No table needed - achievements are defined in Rust code

-- User achievements (earned)
CREATE TABLE user_achievements (
    user_id INTEGER NOT NULL,
    achievement_key TEXT NOT NULL,
    earned_at TEXT NOT NULL,
    ride_id TEXT,
    progress_value REAL,
    PRIMARY KEY (user_id, achievement_key)
);

-- User XP and level
CREATE TABLE user_xp (
    user_id INTEGER PRIMARY KEY,
    total_xp INTEGER NOT NULL DEFAULT 0,
    current_level INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);

-- Unlocked rewards
CREATE TABLE user_rewards (
    user_id INTEGER NOT NULL,
    reward_type TEXT NOT NULL,
    reward_id TEXT NOT NULL,
    unlocked_at TEXT NOT NULL,
    unlocked_at_level INTEGER NOT NULL,
    PRIMARY KEY (user_id, reward_type, reward_id)
);

-- Gradient settings
CREATE TABLE gradient_settings (
    user_id INTEGER PRIMARY KEY,
    difficulty_percent INTEGER NOT NULL DEFAULT 100,
    max_gradient REAL NOT NULL DEFAULT 15.0,
    min_gradient REAL NOT NULL DEFAULT -15.0,
    smoothing_secs INTEGER NOT NULL DEFAULT 3,
    rolling_resistance REAL NOT NULL DEFAULT 0.004
);

-- Power profiles
CREATE TABLE power_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    profile_type TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE power_profile_points (
    profile_id INTEGER NOT NULL,
    duration_secs INTEGER NOT NULL,
    power_watts INTEGER NOT NULL,
    achieved_at TEXT NOT NULL,
    ride_id TEXT,
    PRIMARY KEY (profile_id, duration_secs),
    FOREIGN KEY (profile_id) REFERENCES power_profiles(id) ON DELETE CASCADE
);

-- Training plan assignments
CREATE TABLE plan_assignments (
    user_id INTEGER PRIMARY KEY,
    plan_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    current_week INTEGER NOT NULL DEFAULT 1,
    completed_workouts INTEGER NOT NULL DEFAULT 0,
    skipped_workouts INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    available_days INTEGER NOT NULL DEFAULT 127
);

-- Indexes for common queries
CREATE INDEX idx_user_achievements_user ON user_achievements(user_id);
CREATE INDEX idx_power_profiles_user_current ON power_profiles(user_id, is_current);
CREATE INDEX idx_power_profile_points_achieved ON power_profile_points(achieved_at);
```

---

## State Transitions

### Achievement Progress States
```
[Not Started] → [In Progress] → [Earned]
     │                │
     └────────────────┘ (cumulative achievements)
```

### Plan Assignment States
```
[Active] ←→ [Paused]
    │           │
    ▼           ▼
[Completed] [Abandoned]
```

### Power Profile Update Flow
```
[Ride Saved] → [Calculate MMP] → [Update Current Profile]
                                        │
                                        ▼
                              [Check Lifetime Bests]
                                        │
                                        ▼
                              [Trigger Achievement Check]
```
