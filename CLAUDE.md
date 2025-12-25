# RustRide - Claude Code Context

## Project Overview

RustRide is an open-source, self-hosted indoor cycling training application built in Rust. It provides BLE sensor connectivity (smart trainers, power meters, HR monitors), structured workout execution with ERG mode control, real-time metrics display, ride recording, and export to standard formats (.fit, .tcx).

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust stable (1.75+) |
| GUI Framework | egui + eframe |
| BLE Communication | btleplug |
| Async Runtime | tokio |
| Database | SQLite via rusqlite |
| Configuration | TOML via toml crate |
| Logging | tracing |
| Serialization | serde |

## Project Structure

```
src/
├── main.rs              # Application entry point
├── app.rs               # egui application state
├── lib.rs               # Library exports
├── sensors/             # BLE sensor management (btleplug)
├── workouts/            # Workout parsing and execution
├── recording/           # Ride recording and file export
├── metrics/             # Training metric calculations
├── storage/             # SQLite and config persistence
└── ui/                  # egui screens and widgets
    ├── screens/         # Main application screens
    └── widgets/         # Reusable UI components

tests/
├── unit/               # Unit tests
├── integration/        # Integration tests
└── fixtures/           # Test data (workouts, rides)

specs/                  # Feature specifications
├── 001-indoor-cycling-app/
│   ├── spec.md         # Feature specification
│   ├── plan.md         # Implementation plan
│   ├── research.md     # Technology research
│   ├── data-model.md   # Entity definitions
│   ├── quickstart.md   # Developer setup
│   └── contracts/      # Module API contracts
```

## Key Modules

### sensors/
BLE sensor discovery and connection using btleplug. Supports:
- FTMS (Fitness Machine Service) for smart trainers
- Cycling Power Service for power meters
- Heart Rate Service for HR monitors

### workouts/
Workout file parsing (.zwo, .mrc) and execution engine. Manages:
- Workout state machine (start, pause, resume, skip)
- ERG mode power target calculations
- Interval timing and transitions

### recording/
Ride data capture and export. Features:
- 1-second sample recording
- Auto-save for crash recovery (30-second intervals)
- Export to TCX, FIT, GPX, CSV formats

### metrics/
Training metric calculations:
- Normalized Power (NP)
- Training Stress Score (TSS)
- Intensity Factor (IF)
- Power and HR zone calculations

### storage/
Data persistence:
- SQLite for rides, workouts, sensors
- TOML for user configuration
- Auto-migrations on startup

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test               # Run all tests
cargo test sensors       # Test specific module

# Run
cargo run                # Debug mode
cargo run --release      # Release mode

# Code quality
cargo fmt                # Format code
cargo clippy             # Lint
```

## Important Patterns

### Thread Communication
- Use `crossbeam::channel` for sensor data (BLE → UI)
- Use `Arc<Mutex<T>>` for shared state (ride samples)
- Call `ctx.request_repaint()` from background threads to wake UI

### BLE Protocol
- FTMS Control Point writes for ERG mode: `[0x05, watts_lo, watts_hi]`
- Always request control (`[0x00]`) before setting targets
- Handle automatic reconnection on signal loss

### Metrics Calculations
- Normalized Power: 30s rolling average → 4th power → mean → 4th root
- TSS = (hours × IF²) × 100
- Filter power spikes > 2000W as noise

## Configuration Locations

- **Windows**: `%APPDATA%\rustride\`
- **macOS**: `~/Library/Application Support/rustride/`
- **Linux**: `~/.config/rustride/` (config), `~/.local/share/rustride/` (data)

## Current Feature: 001-indoor-cycling-app

MVP implementation covering:
- P1: Connect smart trainer and start free ride
- P2: Execute structured workouts with ERG mode
- P3: Record and export ride data
- P4: Real-time training metrics display
- P5: User profile and training zones
- P6: Ride history browsing
- P7: Multi-sensor support

See `specs/001-indoor-cycling-app/` for full specifications.
