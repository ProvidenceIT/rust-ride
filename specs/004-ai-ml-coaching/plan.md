# Implementation Plan: AI & Machine Learning Coaching

**Branch**: `004-ai-ml-coaching` | **Date**: 2025-12-25 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-ai-ml-coaching/spec.md`

## Summary

Implement AI/ML-powered coaching features for RustRide including FTP prediction from workout history, real-time fatigue detection during rides, adaptive workout recommendations based on training goals and load, performance trend forecasting, workout difficulty estimation, enhanced rider profiling, cadence analysis, and personalized training load adaptation. The system uses cloud-based ML inference with local caching for offline viewing, triggered post-ride for prediction updates.

## Technical Context

**Language/Version**: Rust stable (1.75+)
**Primary Dependencies**: egui/eframe (GUI), tokio (async), serde (serialization), reqwest (HTTP client for cloud API), chrono (datetime)
**Storage**: SQLite via rusqlite (local cache), Cloud API (ML inference)
**Testing**: cargo test (unit + integration tests)
**Target Platform**: Windows, macOS, Linux desktop application
**Project Type**: Single desktop application with cloud backend integration
**Performance Goals**: 2 second insight delivery (SC-009), 5 second cloud inference (SC-011), 60fps UI
**Constraints**: Graceful offline degradation, post-ride prediction updates, dismissible alerts with cooldown
**Scale/Scope**: Single-user desktop app, 8 user stories across P1-P4 priorities

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| Rust-First | PASS | All client code in Rust per existing codebase |
| Test-First | PASS | Unit tests required for all ML wrappers and prediction logic |
| Simplicity | PASS | Builds on existing analytics module; cloud offloads ML complexity |
| Self-Hosted | REVIEW | Cloud dependency noted; local caching preserves core functionality offline |

**Gate Decision**: PROCEED - Cloud dependency justified by ML inference requirements; local caching maintains self-hosted spirit for viewing predictions.

## Project Structure

### Documentation (this feature)

```text
specs/004-ai-ml-coaching/
├── plan.md              # This file
├── research.md          # Phase 0 output - ML approaches, cloud API design
├── data-model.md        # Phase 1 output - entities, relationships
├── quickstart.md        # Phase 1 output - developer setup
├── contracts/           # Phase 1 output - API contracts
│   ├── cloud-api.md     # Cloud ML service contract
│   ├── prediction-api.md # Local prediction interface
│   └── recommendation-api.md # Workout recommendation interface
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── ml/                      # NEW: ML coaching module
│   ├── mod.rs               # Module exports
│   ├── client.rs            # Cloud API client (reqwest)
│   ├── cache.rs             # Local prediction cache
│   ├── ftp_prediction.rs    # FTP prediction wrapper
│   ├── fatigue_detection.rs # Real-time fatigue analysis
│   ├── workout_recommend.rs # Workout recommendations
│   ├── performance_forecast.rs # CTL trend forecasting
│   ├── difficulty.rs        # Workout difficulty estimation
│   ├── cadence_analysis.rs  # Cadence optimization
│   └── adaptation.rs        # Training load adaptation
├── goals/                   # NEW: Training goals module
│   ├── mod.rs               # Module exports
│   ├── types.rs             # Goal type definitions
│   └── manager.rs           # Goal lifecycle management
├── workouts/
│   └── library.rs           # NEW: Built-in workout library
├── metrics/analytics/       # EXISTING: Extend with ML triggers
│   └── triggers.rs          # Update to call ML predictions post-ride
├── storage/
│   └── ml_store.rs          # NEW: ML prediction storage
├── ui/
│   ├── screens/
│   │   ├── goals.rs         # NEW: Training goals screen
│   │   └── insights.rs      # NEW: AI insights screen
│   └── widgets/
│       ├── fatigue_alert.rs # NEW: Fatigue warning widget
│       ├── recommendation_card.rs # NEW: Workout recommendation
│       └── forecast_chart.rs # NEW: Performance forecast

tests/
├── unit/
│   └── ml/                  # ML module unit tests
├── integration/
│   └── ml_integration_test.rs # End-to-end ML flow tests
└── fixtures/
    └── ml/                  # Test data for ML predictions
```

**Structure Decision**: Extends existing single-project structure with new `ml/` and `goals/` modules. Leverages existing `metrics/analytics/` for data inputs. Cloud integration via dedicated client module.

## Complexity Tracking

| Aspect | Justification | Alternative Rejected |
|--------|--------------|---------------------|
| Cloud ML backend | Complex ML models (gradient boosting, anomaly detection) exceed local compute budget | On-device ML rejected: model size, training data, update cycles too complex |
| New `goals/` module | Training goals are domain concept distinct from workouts/metrics | Embedding in workouts rejected: goals span multiple workouts |
| Built-in workout library | Recommendations need curated workout pool | User imports only rejected: cold start problem for new users |
