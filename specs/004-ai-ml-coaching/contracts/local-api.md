# Local ML Module API Contract

**Feature Branch**: `004-ai-ml-coaching`
**Module**: `src/ml/`

## Overview

Internal Rust API for ML coaching features. Wraps cloud API calls with local caching, offline handling, and real-time analysis.

---

## Module Structure

```rust
// src/ml/mod.rs
pub mod client;           // Cloud API client
pub mod cache;            // Local prediction cache
pub mod ftp_prediction;   // FTP prediction wrapper
pub mod fatigue_detection;// Real-time fatigue analysis
pub mod workout_recommend;// Workout recommendations
pub mod performance_forecast; // CTL forecasting
pub mod difficulty;       // Workout difficulty estimation
pub mod cadence_analysis; // Cadence optimization
pub mod adaptation;       // Training load adaptation
```

---

## Core Types

### MlClient

Cloud API client with offline queue support.

```rust
pub struct MlClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    offline_queue: Arc<Mutex<VecDeque<QueuedRequest>>>,
}

impl MlClient {
    pub fn new(api_key: String) -> Self;

    /// Send prediction request, queue if offline
    pub async fn request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: impl Serialize,
    ) -> Result<T, MlError>;

    /// Process queued requests when back online
    pub async fn flush_queue(&self) -> Result<usize, MlError>;

    /// Check if currently online
    pub fn is_online(&self) -> bool;
}
```

### MlCache

SQLite-backed prediction cache.

```rust
pub struct MlCache {
    db: Connection,
}

impl MlCache {
    pub fn new(db: Connection) -> Self;

    /// Store prediction with expiry
    pub fn store<T: Serialize>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
        payload: &T,
        expires_in: Duration,
    ) -> Result<(), MlError>;

    /// Retrieve cached prediction if not expired
    pub fn get<T: DeserializeOwned>(
        &self,
        user_id: Uuid,
        prediction_type: PredictionType,
    ) -> Option<CachedPrediction<T>>;

    /// Clean expired predictions
    pub fn cleanup_expired(&self) -> Result<usize, MlError>;
}

pub struct CachedPrediction<T> {
    pub payload: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_stale: bool,  // True if past expiry but still usable
}
```

---

## FTP Prediction API

```rust
// src/ml/ftp_prediction.rs

pub struct FtpPredictor {
    client: Arc<MlClient>,
    cache: Arc<MlCache>,
}

impl FtpPredictor {
    /// Request FTP prediction after ride completion
    /// Returns cached value if cloud unavailable
    pub async fn predict(
        &self,
        user_id: Uuid,
        rides: &[RideSummary],
        current_ftp: u16,
    ) -> Result<FtpPredictionResult, MlError>;

    /// Check if prediction differs significantly from current FTP
    pub fn should_notify(&self, prediction: &FtpPredictionResult, current_ftp: u16) -> bool {
        prediction.differs_from_current && prediction.difference_percent.abs() > 3.0
    }
}

pub struct FtpPredictionResult {
    pub predicted_ftp: u16,
    pub confidence: FtpConfidence,
    pub method_used: FtpMethod,
    pub supporting_efforts: Vec<SupportingEffort>,
    pub differs_from_current: bool,
    pub difference_percent: f32,
    pub source: PredictionSource,  // Cloud, Cached, LocalFallback
}
```

---

## Fatigue Detection API

```rust
// src/ml/fatigue_detection.rs

pub struct FatigueDetector {
    client: Arc<MlClient>,
    baseline: AthleteBaseline,
}

impl FatigueDetector {
    /// Analyze fatigue from recent ride samples (5-min window)
    /// Can run locally or request cloud analysis
    pub async fn analyze(
        &self,
        ride_id: Uuid,
        samples: &[RideSample],
        target_power: Option<u16>,
    ) -> Result<FatigueAnalysis, MlError>;

    /// Calculate aerobic decoupling locally (no cloud needed)
    pub fn aerobic_decoupling(&self, samples: &[RideSample]) -> f32;

    /// Calculate power variability index locally
    pub fn power_variability_index(&self, samples: &[RideSample]) -> f32;

    /// Check if alert should trigger based on thresholds
    pub fn should_alert(&self, analysis: &FatigueAnalysis) -> bool;

    /// Manage alert dismissal with cooldown
    pub fn dismiss_alert(&self, state: &mut FatigueState);
    pub fn is_in_cooldown(&self, state: &FatigueState) -> bool;
}

pub struct FatigueAnalysis {
    pub aerobic_decoupling_score: f32,
    pub power_variability_index: f32,
    pub hrv_indicator: Option<f32>,
    pub alert_triggered: bool,
    pub severity: FatigueSeverity,
    pub message: String,
    pub confidence: f32,
}

pub enum FatigueSeverity {
    None,
    Mild,
    Moderate,
    Severe,
}

pub struct AthleteBaseline {
    pub resting_hr: u8,
    pub max_hr: u8,
    pub typical_decoupling: f32,
    pub typical_variability: f32,
}
```

---

## Workout Recommendation API

```rust
// src/ml/workout_recommend.rs

pub struct WorkoutRecommender {
    client: Arc<MlClient>,
    cache: Arc<MlCache>,
    library: Arc<WorkoutLibrary>,
}

impl WorkoutRecommender {
    /// Get personalized workout recommendations
    pub async fn recommend(
        &self,
        user_id: Uuid,
        goals: &[TrainingGoal],
        current_load: &DailyLoad,
        available_minutes: u16,
        recently_completed: &[Uuid],
    ) -> Result<Vec<WorkoutRecommendation>, MlError>;

    /// Get recommendations for a specific goal
    pub async fn recommend_for_goal(
        &self,
        user_id: Uuid,
        goal: &TrainingGoal,
        current_load: &DailyLoad,
    ) -> Result<Vec<WorkoutRecommendation>, MlError>;

    /// Mark recommendation as completed/declined
    pub fn update_status(
        &self,
        recommendation_id: Uuid,
        status: RecommendationStatus,
    ) -> Result<(), MlError>;
}

pub struct WorkoutLibrary {
    builtin: Vec<BuiltInWorkout>,
    user_imports: Vec<Workout>,
}

impl WorkoutLibrary {
    pub fn search(
        &self,
        energy_systems: &[EnergySystem],
        max_duration: u16,
        difficulty_range: (f32, f32),
    ) -> Vec<&dyn WorkoutRef>;
}
```

---

## Performance Forecasting API

```rust
// src/ml/performance_forecast.rs

pub struct PerformanceForecaster {
    client: Arc<MlClient>,
    cache: Arc<MlCache>,
}

impl PerformanceForecaster {
    /// Generate CTL forecast for next N weeks
    pub async fn forecast(
        &self,
        user_id: Uuid,
        ctl_history: &[(NaiveDate, DailyLoad)],
        forecast_weeks: u8,
        target_event: Option<&TrainingGoal>,
    ) -> Result<PerformanceProjection, MlError>;

    /// Detect if athlete is plateauing
    pub fn detect_plateau(&self, projection: &PerformanceProjection) -> bool;

    /// Assess detraining risk based on recent activity
    pub fn assess_detraining_risk(
        &self,
        ctl_history: &[(NaiveDate, DailyLoad)],
    ) -> DetrainingRisk;

    /// Calculate event readiness gap
    pub fn event_gap(
        &self,
        projection: &PerformanceProjection,
        goal: &TrainingGoal,
    ) -> Option<EventReadiness>;
}
```

---

## Difficulty Estimation API

```rust
// src/ml/difficulty.rs

pub struct DifficultyEstimator {
    cache: Arc<MlCache>,
}

impl DifficultyEstimator {
    /// Calculate personalized difficulty for a workout
    pub fn estimate(
        &self,
        workout: &Workout,
        user_ftp: u16,
        current_fatigue: &DailyLoad,
    ) -> DifficultyEstimate;

    /// Adjust difficulty based on current fatigue state
    pub fn apply_fatigue_adjustment(
        &self,
        base_difficulty: f32,
        current_atl: f32,
        recent_hard_days: u8,
    ) -> f32;
}
```

---

## Cadence Analysis API

```rust
// src/ml/cadence_analysis.rs

pub struct CadenceAnalyzer {
    client: Arc<MlClient>,
    cache: Arc<MlCache>,
}

impl CadenceAnalyzer {
    /// Analyze optimal cadence from ride history
    pub async fn analyze(
        &self,
        user_id: Uuid,
        ride_samples: &[RideCadenceSamples],
    ) -> Result<CadenceAnalysis, MlError>;

    /// Detect technique degradation pattern
    pub fn detect_degradation(
        &self,
        samples: &[RideSample],
    ) -> Option<DegradationPattern>;
}
```

---

## Adaptation Engine API

```rust
// src/ml/adaptation.rs

pub struct AdaptationEngine {
    client: Arc<MlClient>,
    cache: Arc<MlCache>,
}

impl AdaptationEngine {
    /// Learn individual recovery patterns from history
    pub async fn learn_patterns(
        &self,
        user_id: Uuid,
        training_history: &[(NaiveDate, DailyLoad, Option<f32>)], // date, load, performance
    ) -> Result<AdaptationModel, MlError>;

    /// Get personalized load recommendation
    pub fn recommend_load(
        &self,
        model: &AdaptationModel,
        current_load: &DailyLoad,
    ) -> LoadRecommendation;

    /// Check if model has sufficient confidence
    pub fn has_sufficient_data(&self, model: &AdaptationModel) -> bool {
        matches!(model.confidence, ModelConfidence::Medium | ModelConfidence::High)
    }
}

pub struct LoadRecommendation {
    pub suggested_tss: f32,
    pub intensity_focus: EnergySystem,
    pub recovery_needed: bool,
    pub reasoning: String,
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MlError {
    #[error("Cloud API error: {0}")]
    ApiError(String),

    #[error("Network unavailable")]
    Offline,

    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
```

---

## Usage Example

```rust
// Post-ride trigger integration
async fn on_ride_complete(ride: &Ride, samples: &[RideSample]) {
    let ml = MlCoordinator::new(config);

    // Trigger all post-ride ML updates
    let ftp_result = ml.ftp_predictor.predict(
        ride.user_id,
        &recent_rides,
        current_ftp,
    ).await;

    if ml.ftp_predictor.should_notify(&ftp_result, current_ftp) {
        ui.show_ftp_notification(&ftp_result);
    }

    // Update recommendations
    let recommendations = ml.recommender.recommend(
        ride.user_id,
        &active_goals,
        &current_load,
        60, // default available time
        &[ride.id],
    ).await;

    cache.store_recommendations(&recommendations);
}
```
