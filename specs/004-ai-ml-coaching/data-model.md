# Data Model: AI & Machine Learning Coaching

**Feature Branch**: `004-ai-ml-coaching`
**Date**: 2025-12-25
**Status**: Complete

## Overview

Entity definitions for ML coaching features, extending the existing RustRide data model with training goals, ML predictions, and recommendation tracking.

---

## New Entities

### TrainingGoal

Represents a rider's training objective.

```rust
pub struct TrainingGoal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub goal_type: GoalType,
    pub title: String,
    pub description: Option<String>,
    pub target_date: Option<NaiveDate>,      // For event goals
    pub target_metric: Option<TargetMetric>, // e.g., target CTL, target FTP
    pub priority: u8,                        // 1 = highest priority
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum GoalType {
    // General fitness
    ImproveEndurance,
    LoseWeight,
    GetFaster,

    // Event-focused
    Race { event_type: EventType },
    CenturyRide,
    GranFondo,
    TimeTrial,

    // Energy system
    ImproveVo2max,
    BuildThreshold,
    DevelopSprint,
}

pub enum EventType {
    RoadRace,
    Criterium,
    GranFondo,
    TimeTrial,
    Triathlon,
    Other(String),
}

pub struct TargetMetric {
    pub metric_type: MetricType,
    pub target_value: f32,
    pub current_value: Option<f32>,
}

pub enum MetricType {
    Ctl,
    Ftp,
    Vo2max,
    Weight,
}

pub enum GoalStatus {
    Active,
    Completed,
    Abandoned,
    OnHold,
}
```

**Relationships**:
- Belongs to User (many-to-one)
- Referenced by WorkoutRecommendation

**Validation**:
- User can have multiple active goals (FR-029)
- Event goals require target_date
- Priority must be unique per user

---

### MlPrediction

Base entity for cached ML predictions from cloud.

```rust
pub struct MlPrediction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub prediction_type: PredictionType,
    pub payload: serde_json::Value,  // Type-specific prediction data
    pub confidence: f32,             // 0.0 - 1.0
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub source: PredictionSource,
}

pub enum PredictionType {
    FtpPrediction,
    FatigueState,
    PerformanceForecast,
    DifficultyEstimate,
    CadenceAnalysis,
    AdaptationModel,
}

pub enum PredictionSource {
    Cloud,
    LocalFallback,
    Cached,
}
```

**Cache Expiry**:
- FtpPrediction: 7 days
- FatigueState: 24 hours
- PerformanceForecast: 24 hours
- DifficultyEstimate: 1 hour
- CadenceAnalysis: 7 days
- AdaptationModel: 7 days

---

### FtpPrediction (extends MlPrediction payload)

```rust
pub struct FtpPredictionPayload {
    pub predicted_ftp: u16,
    pub confidence_level: FtpConfidence,  // Reuse from existing analytics
    pub method_used: FtpMethod,
    pub supporting_efforts: Vec<SupportingEffort>,
    pub confidence_interval: (u16, u16),  // low, high
    pub differs_from_current: bool,
    pub difference_percent: f32,
}

pub struct SupportingEffort {
    pub ride_id: Uuid,
    pub duration_secs: u32,
    pub power_watts: u16,
    pub ride_date: NaiveDate,
}

pub enum FtpMethod {
    ExtendedDuration,
    TwentyMinute,
    CriticalPower,
    MlRefined,
}
```

---

### FatigueState

Real-time fatigue indicators during a ride.

```rust
pub struct FatigueState {
    pub ride_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub aerobic_decoupling_score: f32,   // HR drift percentage
    pub power_variability_index: f32,    // CV above baseline
    pub hrv_fatigue_indicator: Option<f32>,
    pub alert_triggered: bool,
    pub alert_dismissed: bool,
    pub dismissed_at: Option<DateTime<Utc>>,
    pub cooldown_expires_at: Option<DateTime<Utc>>,
}
```

**State Transitions**:
- `alert_triggered: false` → `true` when thresholds exceeded
- `alert_dismissed: true` sets `cooldown_expires_at` to now + 5-10 min
- After cooldown, `alert_dismissed` resets if fatigue persists

---

### WorkoutRecommendation

A recommended workout with reasoning.

```rust
pub struct WorkoutRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workout_id: Uuid,
    pub workout_source: WorkoutSource,
    pub suitability_score: f32,          // 0.0 - 1.0
    pub reasoning: String,
    pub target_energy_systems: Vec<EnergySystem>,
    pub expected_tss: f32,
    pub goal_alignment: Option<Uuid>,    // TrainingGoal.id
    pub training_gap: Option<String>,    // e.g., "No VO2max in 8 days"
    pub recommended_at: DateTime<Utc>,
    pub status: RecommendationStatus,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum WorkoutSource {
    BuiltIn,
    UserImport,
}

pub enum EnergySystem {
    Neuromuscular,
    Anaerobic,
    Vo2max,
    Threshold,
    SweetSpot,
    Endurance,
    Recovery,
}

pub enum RecommendationStatus {
    Pending,
    Accepted,
    Declined,
    Completed,
    Expired,
}
```

**Relationships**:
- Belongs to User (many-to-one)
- References Workout (many-to-one)
- Optionally references TrainingGoal

---

### PerformanceProjection

CTL/fitness trend forecast.

```rust
pub struct PerformanceProjection {
    pub id: Uuid,
    pub user_id: Uuid,
    pub projected_at: DateTime<Utc>,
    pub forecast_weeks: u8,
    pub data_points: Vec<ProjectedCtl>,
    pub plateau_detected: bool,
    pub detraining_risk: DetrainingRisk,
    pub event_readiness: Option<EventReadiness>,
}

pub struct ProjectedCtl {
    pub date: NaiveDate,
    pub projected_ctl: f32,
    pub confidence_low: f32,
    pub confidence_high: f32,
}

pub enum DetrainingRisk {
    None,
    Low,
    Medium,
    High,
}

pub struct EventReadiness {
    pub goal_id: Uuid,
    pub target_ctl: f32,
    pub projected_ctl_at_event: f32,
    pub gap: f32,
    pub recommendation: String,
}
```

---

### DifficultyEstimate

Personalized workout difficulty score.

```rust
pub struct DifficultyEstimate {
    pub workout_id: Uuid,
    pub user_id: Uuid,
    pub base_difficulty: f32,           // 1.0 - 10.0 scale
    pub fatigue_adjustment: f32,        // + or - based on current state
    pub final_difficulty: f32,          // Displayed to user
    pub calculated_at: DateTime<Utc>,
    pub factors: DifficultyFactors,
}

pub struct DifficultyFactors {
    pub user_ftp: u16,
    pub workout_if: f32,                // Intensity Factor
    pub workout_tss: f32,
    pub current_atl: f32,               // Acute training load
    pub recent_hard_days: u8,           // Days since last hard effort
}
```

---

### CadenceAnalysis

Optimal cadence insights.

```rust
pub struct CadenceAnalysis {
    pub user_id: Uuid,
    pub analyzed_at: DateTime<Utc>,
    pub optimal_range: (u8, u8),        // RPM min, max
    pub efficiency_by_zone: Vec<CadenceEfficiency>,
    pub degradation_pattern: Option<DegradationPattern>,
    pub recommendation: String,
}

pub struct CadenceEfficiency {
    pub cadence_band: (u8, u8),         // e.g., 80-85 RPM
    pub efficiency_score: f32,          // 0.0 - 1.0
    pub sample_count: u32,
}

pub struct DegradationPattern {
    pub onset_minutes: u32,             // Minutes into ride
    pub variability_increase: f32,      // Percentage increase
    pub recommendation: String,
}
```

---

### AdaptationModel

Personalized recovery and load parameters.

```rust
pub struct AdaptationModel {
    pub user_id: Uuid,
    pub updated_at: DateTime<Utc>,
    pub recovery_rate: f32,             // Days to recover from hard effort
    pub optimal_ctl_range: (f32, f32),  // Sustainable CTL
    pub optimal_acwr_range: (f32, f32), // Safe ACWR band
    pub training_response: TrainingResponse,
    pub confidence: ModelConfidence,
}

pub struct TrainingResponse {
    pub ctl_sensitivity: f32,           // CTL gain per TSS
    pub fatigue_sensitivity: f32,       // ATL gain per TSS
    pub recovery_days_per_hard: f32,    // Days needed after hard effort
}

pub enum ModelConfidence {
    Insufficient,                       // < 4 weeks data
    Low,                                // 4-6 weeks data
    Medium,                             // 6-12 weeks data
    High,                               // 12+ weeks data
}
```

---

### BuiltInWorkout

Curated workout in the built-in library.

```rust
pub struct BuiltInWorkout {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub category: WorkoutCategory,
    pub energy_systems: Vec<EnergySystem>,
    pub goal_alignment: Vec<GoalType>,
    pub difficulty_tier: DifficultyTier,
    pub duration_minutes: u16,
    pub base_tss: f32,
    pub segments: Vec<WorkoutSegment>,  // Reuse existing type
    pub created_at: DateTime<Utc>,
}

pub enum WorkoutCategory {
    Recovery,
    Endurance,
    SweetSpot,
    Threshold,
    Vo2max,
    Sprint,
    Mixed,
}

pub enum DifficultyTier {
    Easy,
    Moderate,
    Hard,
    VeryHard,
}
```

**Initial Library Size**: ~80 workouts covering all categories and goal types.

---

## Extended Existing Entities

### User (extend)

```rust
// Add to existing User struct
pub struct UserMlSettings {
    pub cloud_sync_enabled: bool,
    pub fatigue_alert_enabled: bool,
    pub fatigue_alert_cooldown_minutes: u8,  // 5-10
    pub recommendation_auto_refresh: bool,
}
```

---

## Database Schema Additions

```sql
-- Training Goals
CREATE TABLE training_goals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
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

-- ML Prediction Cache
CREATE TABLE ml_predictions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    prediction_type TEXT NOT NULL,
    payload TEXT NOT NULL,  -- JSON
    confidence REAL NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    source TEXT NOT NULL
);

CREATE INDEX idx_ml_predictions_user_type ON ml_predictions(user_id, prediction_type);
CREATE INDEX idx_ml_predictions_expires ON ml_predictions(expires_at);

-- Workout Recommendations
CREATE TABLE workout_recommendations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    workout_id TEXT NOT NULL,
    workout_source TEXT NOT NULL,
    suitability_score REAL NOT NULL,
    reasoning TEXT NOT NULL,
    target_energy_systems TEXT NOT NULL,  -- JSON array
    expected_tss REAL NOT NULL,
    goal_id TEXT REFERENCES training_goals(id),
    training_gap TEXT,
    recommended_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    completed_at TEXT
);

-- Performance Projections
CREATE TABLE performance_projections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    projected_at TEXT NOT NULL,
    forecast_weeks INTEGER NOT NULL,
    data_points TEXT NOT NULL,  -- JSON array
    plateau_detected INTEGER NOT NULL DEFAULT 0,
    detraining_risk TEXT NOT NULL,
    event_readiness TEXT  -- JSON
);

-- Built-in Workout Library
CREATE TABLE builtin_workouts (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    energy_systems TEXT NOT NULL,  -- JSON array
    goal_alignment TEXT NOT NULL,  -- JSON array
    difficulty_tier TEXT NOT NULL,
    duration_minutes INTEGER NOT NULL,
    base_tss REAL NOT NULL,
    segments TEXT NOT NULL,  -- JSON array
    created_at TEXT NOT NULL
);

-- Fatigue States (per-ride tracking)
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

CREATE INDEX idx_fatigue_states_ride ON fatigue_states(ride_id);
```

---

## Entity Relationships

```
User
 ├── TrainingGoal (1:N)
 ├── MlPrediction (1:N, cached)
 ├── WorkoutRecommendation (1:N)
 ├── PerformanceProjection (1:N)
 ├── CadenceAnalysis (1:1, latest)
 └── AdaptationModel (1:1, latest)

TrainingGoal
 └── WorkoutRecommendation (1:N, via goal_alignment)

Workout (existing)
 ├── WorkoutRecommendation (1:N)
 └── DifficultyEstimate (1:N, per user)

Ride (existing)
 └── FatigueState (1:N, time series during ride)

BuiltInWorkout
 └── WorkoutRecommendation (1:N, when source = BuiltIn)
```

---

## Migration Strategy

1. Add new tables without dropping existing data
2. Schema version increment: 2 → 3
3. Seed builtin_workouts table with initial 80 workouts on first run
4. Create indexes for query performance
