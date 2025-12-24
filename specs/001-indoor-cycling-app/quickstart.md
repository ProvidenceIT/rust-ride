# Developer Quickstart: RustRide

**Feature Branch**: `001-indoor-cycling-app`
**Date**: 2025-12-24

## Prerequisites

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.75+ stable | Programming language |
| Cargo | (bundled) | Package manager |
| Git | 2.x | Version control |

### Platform-Specific Requirements

#### Windows 10/11
- Visual Studio Build Tools or VS 2019+ with C++ workload
- Windows 10 SDK
- No additional BLE setup required (uses Windows.Devices.Bluetooth)

#### macOS 11+
- Xcode Command Line Tools: `xcode-select --install`
- Bluetooth permission: Add `NSBluetoothAlwaysUsageDescription` to Info.plist

#### Linux (Ubuntu 20.04+, Fedora 35+)
```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libdbus-1-dev libudev-dev

# Fedora
sudo dnf install gcc pkg-config dbus-devel systemd-devel

# BlueZ (usually pre-installed)
sudo apt install bluez libbluetooth-dev
```

### BLE Hardware

For development and testing, you'll need:
- Bluetooth LE adapter (built-in or USB dongle)
- At minimum one of:
  - BLE smart trainer (Wahoo, Tacx, Elite with FTMS support)
  - BLE heart rate monitor
  - BLE power meter

For testing without hardware, see "Sensor Simulator" section below.

---

## Quick Setup

### 1. Clone and Build

```bash
# Clone repository
git clone https://github.com/ProvidenceIT/rust-ride.git
cd rust-ride

# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release
```

### 2. Run the Application

```bash
# Debug mode
cargo run

# Release mode
cargo run --release
```

### 3. Run Tests

```bash
# All tests
cargo test

# Specific module tests
cargo test --lib sensors
cargo test --lib workouts
cargo test --lib metrics

# With output
cargo test -- --nocapture
```

---

## Project Structure

```
rust-ride/
├── Cargo.toml              # Dependencies and metadata
├── src/
│   ├── main.rs             # Entry point
│   ├── app.rs              # egui application state
│   ├── lib.rs              # Library exports
│   ├── sensors/            # BLE sensor management
│   ├── workouts/           # Workout parsing and execution
│   ├── recording/          # Ride recording and export
│   ├── metrics/            # Training metric calculations
│   ├── storage/            # SQLite and config persistence
│   └── ui/                 # egui screens and widgets
├── tests/
│   ├── unit/               # Unit tests
│   ├── integration/        # Integration tests
│   └── fixtures/           # Test data files
├── specs/                  # Feature specifications
└── .specify/               # SpecKit configuration
```

---

## Development Workflow

### Feature Development

1. Create a feature branch from `main`
2. Implement feature with TDD approach
3. Run `cargo fmt` and `cargo clippy` before committing
4. Create PR for review

### Code Style

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy -- -D warnings

# Check before commit
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Trace level for BLE
RUST_LOG=btleplug=trace cargo run

# Specific module logging
RUST_LOG=rustride::sensors=debug cargo run
```

---

## Testing Without Hardware

### Sensor Simulator

For development without physical sensors, use the mock sensor module:

```rust
// In tests or development mode
use rustride::sensors::mock::{MockTrainer, MockHeartRate};

let trainer = MockTrainer::new(MockTrainerConfig {
    initial_power: 200,
    power_variance: 10,
    cadence: 90,
    speed_kmh: 30.0,
});

let hr_monitor = MockHeartRate::new(MockHRConfig {
    base_hr: 140,
    hr_variance: 5,
});
```

### Sample Workout Files

Test workout files are in `tests/fixtures/workouts/`:
- `sweet_spot.zwo` - Sweet spot intervals
- `vo2max_intervals.zwo` - VO2max workout
- `endurance.mrc` - Long endurance ride

### Sample Ride Data

Test ride data in `tests/fixtures/rides/`:
- `1_hour_steady.json` - 1-hour steady state ride
- `intervals_workout.json` - Interval workout with samples

---

## Database

### Location

The SQLite database is stored at:
- **Windows**: `%APPDATA%\rustride\data.db`
- **macOS**: `~/Library/Application Support/rustride/data.db`
- **Linux**: `~/.local/share/rustride/data.db`

### Development Reset

```bash
# Remove database to start fresh
# Windows
del %APPDATA%\rustride\data.db

# macOS/Linux
rm ~/Library/Application\ Support/rustride/data.db  # macOS
rm ~/.local/share/rustride/data.db                  # Linux
```

### Schema Migrations

Migrations are embedded in the binary and run automatically on startup.

---

## Configuration

### User Config File

Located at:
- **Windows**: `%APPDATA%\rustride\config.toml`
- **macOS**: `~/Library/Application Support/rustride/config.toml`
- **Linux**: `~/.config/rustride/config.toml`

### Example Config

```toml
[profile]
name = "Cyclist"
ftp = 250
max_hr = 185
resting_hr = 55
weight_kg = 75.0

[preferences]
units = "metric"  # or "imperial"
theme = "dark"    # or "light"

[recording]
autosave_interval_secs = 30

[sensors]
reconnect_attempts = 10
scan_timeout_secs = 30
```

---

## Common Development Tasks

### Adding a New Sensor Type

1. Add protocol implementation in `src/sensors/`
2. Register service UUID in `SensorManager::SCAN_FILTER`
3. Add data parsing in appropriate protocol module
4. Update `SensorType` enum
5. Add tests in `tests/unit/sensors/`

### Adding a Workout Format

1. Create parser in `src/workouts/parser_*.rs`
2. Implement `parse_*()` function
3. Register format in `WorkoutFormat` enum
4. Add test files in `tests/fixtures/workouts/`
5. Add tests in `tests/unit/workout_parser_test.rs`

### Adding an Export Format

1. Create exporter in `src/recording/exporter_*.rs`
2. Implement `export_*()` function
3. Add format to export options in UI
4. Add tests validating output format

---

## Useful Commands

```bash
# Check compilation without building
cargo check

# Build with all features
cargo build --all-features

# Generate documentation
cargo doc --open

# Profile build for performance
cargo build --release --features profiling

# Watch for changes and rebuild
cargo watch -x run

# Benchmark (requires nightly for some benches)
cargo bench
```

---

## Troubleshooting

### BLE Issues

**"Adapter not found"**
- Ensure Bluetooth is enabled in OS settings
- On Linux, check BlueZ is running: `systemctl status bluetooth`

**"Permission denied" (Linux)**
- Add user to `bluetooth` group: `sudo usermod -aG bluetooth $USER`
- Or run with sudo (not recommended for regular development)

**"Sensor not found"**
- Ensure sensor is in pairing mode
- Check sensor is not connected to another device
- Try power cycling the sensor

### Build Issues

**Windows: "MSVC not found"**
- Install Visual Studio Build Tools with C++ workload

**macOS: "SDK not found"**
- Run `xcode-select --install`

**Linux: "pkg-config not found"**
```bash
sudo apt install pkg-config  # Ubuntu/Debian
sudo dnf install pkg-config  # Fedora
```

### Runtime Issues

**"Database locked"**
- Only one instance of RustRide can run at a time
- Close other instances or kill orphaned processes

**"Font not loading"**
- egui includes embedded fonts, no system fonts required
- If issues persist, check `RUST_LOG=egui=debug` output

---

## Resources

### Documentation
- [egui Documentation](https://docs.rs/egui)
- [btleplug Documentation](https://docs.rs/btleplug)
- [rusqlite Documentation](https://docs.rs/rusqlite)
- [tokio Documentation](https://docs.rs/tokio)

### BLE Specifications
- [FTMS Specification](https://www.bluetooth.com/specifications/specs/fitness-machine-service-1-0/)
- [Cycling Power Service](https://www.bluetooth.com/specifications/specs/cycling-power-service-1-1/)
- [Heart Rate Service](https://www.bluetooth.com/specifications/specs/heart-rate-service/)

### Project Specifications
- [Feature Spec](./spec.md)
- [Data Model](./data-model.md)
- [Research Notes](./research.md)
- [API Contracts](./contracts/)
