# Implementation Plan: Training Science & Analytics

**Branch**: `003-training-analytics` | **Date**: 2025-12-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-training-analytics/spec.md`

## Summary

Implement advanced training analytics features including Power Duration Curve (PDC), Critical Power/W' modeling, automatic FTP detection, ACWR training load tracking, VO2max estimation, rider type classification, and Sweet Spot training recommendations. Extends existing `src/metrics/` module which already provides NP/TSS/IF calculations, power zones, and rolling averages.

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**: egui/eframe (GUI), rusqlite (storage), serde (serialization), chrono (datetime)
**Storage**: SQLite via rusqlite (existing database.rs, schema.rs)
**Testing**: cargo test (existing tests/ directory structure)
**Target Platform**: Windows, macOS, Linux desktop (cross-platform)
**Project Type**: Single desktop application with library
**Performance Goals**: PDC chart renders < 3 seconds; real-time metrics update every 1-5 seconds
**Constraints**: Must work offline; all calculations local; no cloud dependencies
**Scale/Scope**: Single-user application; thousands of ride samples per ride; hundreds of historical rides

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project does not have a configured constitution (template only). Standard best practices apply:

| Principle | Status | Notes |
|-----------|--------|-------|
| Simplicity | PASS | Extends existing metrics module; no new architectural patterns |
| Test-First | PASS | Unit tests for all calculation modules |
| Library-First | PASS | All analytics as library functions, UI separate |
| No Over-Engineering | PASS | Uses existing patterns (RollingAverage, PowerFilter) |

### Post-Design Re-evaluation (Phase 1 Complete)

| Principle | Status | Verification |
|-----------|--------|--------------|
| Simplicity | PASS | 7 new files in `analytics/` submodule, each <500 LOC estimated |
| Test-First | PASS | Test structure defined, reference values documented in research.md |
| Library-First | PASS | All calculations in library, storage/UI layers separate |
| No Over-Engineering | PASS | Reuses existing `RollingAverage`, `PowerFilter`; no new patterns |
| Data Model | PASS | 6 new tables, clear relationships, migration strategy defined |
| API Contracts | PASS | All module interfaces documented with examples |

## Project Structure

### Documentation (this feature)

```text
specs/003-training-analytics/
├── plan.md              # This file
├── research.md          # Phase 0 output - algorithm research
├── data-model.md        # Phase 1 output - new entities
├── quickstart.md        # Phase 1 output - setup guide
├── contracts/           # Phase 1 output - module APIs
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs                    # Application entry point
├── app.rs                     # egui application state
├── lib.rs                     # Library exports (add analytics)
├── metrics/
│   ├── mod.rs                 # Add new submodule exports
│   ├── calculator.rs          # [EXISTS] Real-time NP/TSS/IF
│   ├── smoothing.rs           # [EXISTS] Rolling averages, power filter
│   ├── zones.rs               # [EXISTS] Power/HR zones
│   ├── analytics/             # [NEW] Advanced analytics submodule
│   │   ├── mod.rs             # Analytics module root
│   │   ├── pdc.rs             # Power Duration Curve
│   │   ├── critical_power.rs  # CP/W' model fitting
│   │   ├── ftp_detection.rs   # Auto FTP estimation
│   │   ├── training_load.rs   # ATL/CTL/ACWR
│   │   ├── vo2max.rs          # VO2max estimation
│   │   ├── rider_type.rs      # Rider classification
│   │   └── sweet_spot.rs      # Workout recommendations
├── storage/
│   ├── schema.rs              # [MODIFY] Add new tables
│   ├── database.rs            # [MODIFY] Add analytics queries
│   └── analytics_store.rs     # [NEW] Analytics persistence
└── ui/
    ├── screens/
    │   ├── analytics_screen.rs    # [NEW] Analytics dashboard
    │   └── ...
    └── widgets/
        ├── pdc_chart.rs           # [NEW] PDC visualization
        ├── training_load_widget.rs # [NEW] ACWR display
        └── ...

tests/
├── unit/
│   └── analytics/             # [NEW] Analytics unit tests
│       ├── pdc_tests.rs
│       ├── cp_model_tests.rs
│       ├── ftp_detection_tests.rs
│       └── training_load_tests.rs
├── integration/
│   └── analytics_integration.rs # [NEW] End-to-end analytics
└── fixtures/
    └── analytics/             # [NEW] Test ride data
        ├── sample_pdc.json
        └── sample_rides.json
```

**Structure Decision**: Single desktop application structure. New analytics module added under `src/metrics/analytics/` to group related advanced calculations. Follows existing pattern of feature modules (sensors/, workouts/, recording/).

## Complexity Tracking

No constitution violations requiring justification. Implementation uses existing patterns and extends current module structure.
