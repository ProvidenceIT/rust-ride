# Quickstart: AI & Machine Learning Coaching

**Feature Branch**: `004-ai-ml-coaching`
**Date**: 2025-12-25

## Prerequisites

- Rust 1.75+ installed
- RustRide base application building successfully
- SQLite database initialized (schema version 2)
- (Optional) Cloud API key for ML inference

## Quick Setup

```bash
# Clone and switch to feature branch
git checkout 004-ai-ml-coaching

# Add new dependencies to Cargo.toml
# reqwest = { version = "0.11", features = ["json"] }

# Build
cargo build

# Run tests
cargo test ml
cargo test goals

# Run application
cargo run
```

## New Module Structure

```
src/
├── ml/                      # ML coaching module
│   ├── mod.rs               # Module exports
│   ├── client.rs            # Cloud API client
│   ├── cache.rs             # Prediction cache
│   ├── ftp_prediction.rs    # FTP prediction
│   ├── fatigue_detection.rs # Real-time fatigue
│   ├── workout_recommend.rs # Recommendations
│   ├── performance_forecast.rs
│   ├── difficulty.rs
│   ├── cadence_analysis.rs
│   └── adaptation.rs
├── goals/                   # Training goals
│   ├── mod.rs
│   ├── types.rs
│   └── manager.rs
└── workouts/
    └── library.rs           # Built-in workouts
```

## Database Migration

Schema version 2 → 3. New tables:

```sql
-- Run automatically on startup via schema.rs
training_goals
ml_predictions
workout_recommendations
performance_projections
builtin_workouts
fatigue_states
```

## Key Integration Points

### 1. Post-Ride Trigger

Extend `src/metrics/analytics/triggers.rs`:

```rust
use crate::ml::MlCoordinator;

impl AnalyticsTriggers {
    pub async fn run_ml_triggers(
        &self,
        ride: &Ride,
        samples: &[RideSample],
    ) -> MlTriggerResult {
        let ml = MlCoordinator::new(&self.config);

        // FTP prediction update
        let ftp_result = ml.update_ftp_prediction(ride.user_id).await;

        // Workout recommendations refresh
        let recommendations = ml.refresh_recommendations(ride.user_id).await;

        // Performance forecast update
        let forecast = ml.update_forecast(ride.user_id).await;

        MlTriggerResult { ftp_result, recommendations, forecast }
    }
}
```

### 2. Real-Time Fatigue During Ride

Extend `src/ui/screens/ride.rs`:

```rust
use crate::ml::FatigueDetector;

impl RideScreen {
    fn check_fatigue(&mut self, samples: &[RideSample]) {
        if samples.len() >= 300 { // 5 minutes
            let analysis = self.fatigue_detector.analyze_local(samples);
            if analysis.alert_triggered && !self.fatigue_state.is_in_cooldown() {
                self.show_fatigue_alert(&analysis);
            }
        }
    }
}
```

### 3. Goals Screen

New screen at `src/ui/screens/goals.rs`:

```rust
pub struct GoalsScreen {
    goals: Vec<TrainingGoal>,
    manager: GoalManager,
}

impl GoalsScreen {
    pub fn render(&mut self, ui: &mut Ui) {
        // Goal list with add/edit/delete
        // Progress toward each goal
        // Linked recommendations
    }
}
```

### 4. AI Insights Screen

New screen at `src/ui/screens/insights.rs`:

```rust
pub struct InsightsScreen {
    ftp_prediction: Option<FtpPredictionResult>,
    forecast: Option<PerformanceProjection>,
    recommendations: Vec<WorkoutRecommendation>,
    cadence_analysis: Option<CadenceAnalysis>,
}
```

## Testing Scenarios

### Scenario 1: FTP Prediction

```rust
#[test]
fn test_ftp_prediction_with_sufficient_data() {
    let rides = generate_test_rides(10, FTP_250);
    let predictor = FtpPredictor::new_test();

    let result = predictor.predict_local(&rides, 250);

    assert!(result.confidence == FtpConfidence::High);
    assert!((result.predicted_ftp as i32 - 250).abs() < 15);
}
```

### Scenario 2: Fatigue Detection

```rust
#[test]
fn test_aerobic_decoupling_detection() {
    // Simulate HR drift: power constant, HR increases
    let samples = generate_samples_with_hr_drift(power: 200, hr_start: 140, hr_end: 160);
    let detector = FatigueDetector::new_test();

    let analysis = detector.analyze_local(&samples);

    assert!(analysis.aerobic_decoupling_score > 0.10);
    assert!(analysis.alert_triggered);
}
```

### Scenario 3: Workout Recommendations

```rust
#[test]
fn test_recommendations_respect_acwr() {
    let high_load = DailyLoad { atl: 100.0, ctl: 70.0, ..default() }; // ACWR = 1.43
    let recommender = WorkoutRecommender::new_test();

    let recs = recommender.recommend_local(&[goal_vo2max()], &high_load, 60);

    // Should prioritize recovery due to high ACWR
    assert!(recs[0].energy_systems.contains(&EnergySystem::Recovery));
}
```

## Configuration

Add to `config.toml`:

```toml
[ml]
cloud_enabled = true
cloud_api_url = "https://api.rustride.io/v1"
api_key = ""  # Set via environment or UI
cache_cleanup_days = 7

[fatigue]
alert_enabled = true
cooldown_minutes = 7
decoupling_threshold = 0.10
variability_threshold = 1.40

[recommendations]
auto_refresh = true
default_duration_minutes = 60
```

## Offline Mode

When cloud is unavailable:

1. **FTP Prediction**: Uses local FtpDetector from analytics module (already implemented)
2. **Fatigue Detection**: Runs entirely locally (aerobic decoupling, PVI)
3. **Recommendations**: Returns cached recommendations with "last updated" warning
4. **Forecasting**: Uses local EWMA projection (no plateau detection)

## Built-In Workout Library

Initial seeding: 80 workouts across categories:

| Category | Count | Example |
|----------|-------|---------|
| Recovery | 10 | "Easy Spin 30min" |
| Endurance | 15 | "Endurance 90min Z2" |
| Sweet Spot | 15 | "Sweet Spot 2x20min" |
| Threshold | 15 | "Threshold 3x10min" |
| VO2max | 15 | "VO2max 5x4min" |
| Sprint | 10 | "Sprint 6x30s" |

Stored in `builtin_workouts` table, seeded on first run.

## Development Workflow

1. **Phase 1**: Implement local-only features (fatigue detection, difficulty estimation)
2. **Phase 2**: Add cloud client with mock server for testing
3. **Phase 3**: Implement goals module and recommendations
4. **Phase 4**: Add forecasting and adaptation engine
5. **Phase 5**: Polish UI and integrate all components

## Common Issues

### "Insufficient data" errors
- Ensure at least 5 rides with power data exist
- Check rides have PDC points calculated

### Cloud timeout
- Verify API key is set
- Check network connectivity
- Fallback to cached predictions

### Fatigue alerts too frequent
- Increase `cooldown_minutes` in config
- Adjust `decoupling_threshold` higher

## Next Steps

After setup:
1. Run `/speckit.tasks` to generate implementation tasks
2. Implement Phase 1 (local fatigue detection)
3. Set up mock cloud server for development
4. Build UI components incrementally
