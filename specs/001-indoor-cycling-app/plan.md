# Implementation Plan: RustRide Indoor Cycling Application

**Branch**: `001-indoor-cycling-app` | **Date**: 2025-12-24 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-indoor-cycling-app/spec.md`

## Summary

RustRide is an open-source, self-hosted indoor cycling training application built in Rust. The MVP provides BLE sensor connectivity (smart trainers, power meters, HR monitors), structured workout execution with ERG mode control, real-time metrics display, ride recording, and export to standard formats (.fit, .tcx). The application uses egui for a lightweight cross-platform GUI and SQLite for local data persistence.

## Technical Context

**Language/Version**: Rust stable (1.75+)
**Primary Dependencies**:
- `egui` + `eframe` (GUI framework, immediate mode)
- `btleplug` (BLE communication)
- `tokio` (async runtime)
- `rusqlite` (SQLite bindings)
- `serde` + `toml` (configuration)
- `quick-xml` (workout file parsing)
- `tracing` (logging)

**Storage**: SQLite via `rusqlite` (rides, workouts, sensors) + TOML files (user config)
**Testing**: `cargo test` with unit tests, integration tests for BLE mocking
**Target Platform**: Windows 10/11, macOS 11+, Linux (Ubuntu 20.04+, Fedora 35+)
**Project Type**: Single desktop application with modular library structure
**Performance Goals**:
- <200 MB memory during active ride
- <10% CPU during active ride
- 60 fps UI rendering
- 1-second metric update rate

**Constraints**:
- Fully offline-capable (no internet required)
- <5 second startup time
- <30 second data loss on crash (auto-save)
- Cross-platform BLE support

**Scale/Scope**: Single-user desktop application, ~50 screens/views, unlimited ride history

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The constitution template is not yet configured for this project. The following standard gates apply:

| Gate | Status | Notes |
|------|--------|-------|
| Library-First Design | PASS | Core logic (sensors, workouts, metrics) separated from GUI |
| Test-First Development | PASS | TDD approach for core calculations and sensor protocols |
| Simplicity | PASS | No over-engineering; MVP scope well-defined in Out of Scope |
| Cross-Platform | PASS | Using proven cross-platform libraries (egui, btleplug, rusqlite) |

## Project Structure

### Documentation (this feature)

```text
specs/001-indoor-cycling-app/
├── plan.md              # This file
├── research.md          # Phase 0: Technology decisions
├── data-model.md        # Phase 1: Entity definitions
├── quickstart.md        # Phase 1: Developer setup guide
├── contracts/           # Phase 1: Internal API contracts
│   ├── sensor-manager.md
│   ├── workout-engine.md
│   ├── ride-recorder.md
│   └── metrics-calculator.md
└── tasks.md             # Phase 2: Implementation tasks
```

### Source Code (repository root)

```text
src/
├── main.rs              # Application entry point
├── app.rs               # egui application state
├── lib.rs               # Library exports
│
├── sensors/             # BLE sensor management
│   ├── mod.rs
│   ├── manager.rs       # Sensor discovery, connection, reconnection
│   ├── ftms.rs          # FTMS protocol (smart trainers)
│   ├── cycling_power.rs # Cycling Power Service
│   ├── heart_rate.rs    # Heart Rate Service
│   └── types.rs         # Sensor data types
│
├── workouts/            # Workout engine
│   ├── mod.rs
│   ├── engine.rs        # Workout execution state machine
│   ├── parser_zwo.rs    # .zwo file parser
│   ├── parser_mrc.rs    # .mrc/.erg file parser
│   └── types.rs         # Workout segment types
│
├── recording/           # Ride recording
│   ├── mod.rs
│   ├── recorder.rs      # Real-time data capture
│   ├── exporter_fit.rs  # FIT file export
│   ├── exporter_tcx.rs  # TCX file export
│   └── types.rs         # Ride/sample types
│
├── metrics/             # Training metrics
│   ├── mod.rs
│   ├── calculator.rs    # NP, TSS, IF calculations
│   ├── zones.rs         # Power/HR zone logic
│   └── smoothing.rs     # 3-second averaging, noise filtering
│
├── storage/             # Data persistence
│   ├── mod.rs
│   ├── database.rs      # SQLite operations
│   ├── config.rs        # TOML config loading
│   └── schema.rs        # Database migrations
│
└── ui/                  # egui user interface
    ├── mod.rs
    ├── screens/
    │   ├── home.rs
    │   ├── ride.rs
    │   ├── workout_library.rs
    │   ├── sensor_setup.rs
    │   ├── ride_history.rs
    │   ├── ride_detail.rs
    │   └── settings.rs
    ├── widgets/
    │   ├── metric_display.rs
    │   ├── workout_graph.rs
    │   ├── zone_indicator.rs
    │   └── sensor_status.rs
    └── theme.rs

tests/
├── unit/
│   ├── metrics_test.rs
│   ├── zones_test.rs
│   ├── workout_parser_test.rs
│   └── fit_export_test.rs
├── integration/
│   ├── sensor_mock.rs
│   ├── workout_execution_test.rs
│   └── ride_recording_test.rs
└── fixtures/
    ├── workouts/        # Sample .zwo, .mrc files
    └── rides/           # Sample ride data
```

**Structure Decision**: Single Rust project with modular library structure. Core logic (sensors, workouts, metrics, recording) is separated from UI for testability. Each module has clear responsibility and can be unit tested independently.

## Complexity Tracking

No constitution violations requiring justification.

---

## Phase 0: Research Summary

See [research.md](./research.md) for detailed findings.

### Key Technology Decisions

| Area | Decision | Rationale |
|------|----------|-----------|
| BLE Library | `btleplug` | Only mature cross-platform Rust BLE library; async-first design |
| GUI Framework | `egui` + `eframe` | Immediate mode, <5MB binary, 60fps capable, cross-platform |
| FIT Export | `fit-sdk-rs` or custom | Need write support; evaluate existing crates |
| Workout Parsing | `quick-xml` | Standard XML parser for .zwo files |
| Database | `rusqlite` with bundled SQLite | Zero-config, single-file database |
| Async Runtime | `tokio` | Required by btleplug; multi-threaded runtime |

### Research Tasks Completed

1. BLE FTMS protocol specification and btleplug implementation patterns
2. FIT file format and Rust crate availability for write operations
3. egui state management patterns for real-time updating displays
4. Cross-platform BLE permissions and setup requirements
5. Workout file format specifications (.zwo, .mrc/.erg)

---

## Phase 1: Design Artifacts

### Data Model

See [data-model.md](./data-model.md) for complete entity definitions.

**Core Entities**:
- UserProfile (FTP, zones, preferences)
- Sensor (device info, connection state)
- Workout (segments, metadata)
- WorkoutSegment (power targets, duration)
- Ride (summary stats, samples)
- RideSample (1-second data points)

### Internal Contracts

See [contracts/](./contracts/) for module interface definitions.

**Module Boundaries**:
- `SensorManager`: Discovery, connection, data streaming
- `WorkoutEngine`: State machine, ERG target calculation
- `RideRecorder`: Sample capture, auto-save, export
- `MetricsCalculator`: Real-time and post-ride calculations

### Developer Quickstart

See [quickstart.md](./quickstart.md) for setup instructions.

---

## Next Steps

Run `/speckit.tasks` to generate the implementation task list.
