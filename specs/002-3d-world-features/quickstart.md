# Developer Quickstart: 3D Virtual World & Complete Feature Implementation

**Feature**: 002-3d-world-features
**Date**: 2025-12-24

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| OS | Windows 10, macOS 12, Ubuntu 22.04 | Windows 11, macOS 14, Ubuntu 24.04 |
| CPU | 4 cores, 2.5 GHz | 8 cores, 3.5 GHz |
| RAM | 8 GB | 16 GB |
| GPU | OpenGL 3.3 / Vulkan 1.0 | Vulkan 1.2 / Metal |
| Storage | 2 GB free | 5 GB free |

### Development Tools

```bash
# Rust toolchain (1.75+)
rustup update stable
rustc --version  # Should be 1.75.0 or newer

# Verify cargo is available
cargo --version
```

### Platform-Specific Dependencies

**Windows**:
- Visual Studio 2019+ Build Tools (C++ workload)
- No additional dependencies required

**macOS**:
```bash
# Xcode command line tools
xcode-select --install
```

**Linux (Ubuntu/Debian)**:
```bash
# Required system libraries
sudo apt-get update
sudo apt-get install -y \
    libdbus-1-dev \
    pkg-config \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libssl-dev \
    libvulkan-dev
```

## Repository Setup

```bash
# Clone repository (if not already)
git clone https://github.com/your-org/rust-ride.git
cd rust-ride

# Switch to feature branch
git checkout 002-3d-world-features

# Fetch dependencies and verify build
cargo build

# Run tests
cargo test
```

## Project Structure Overview

```
rust-ride/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # Application state
│   ├── sensors/             # BLE sensor management
│   ├── workouts/            # Workout parsing/execution
│   ├── recording/           # Ride recording/export
│   ├── metrics/             # Training calculations
│   ├── storage/             # Database and config
│   ├── ui/                  # egui screens/widgets
│   └── world/               # NEW: 3D virtual world
├── assets/                  # NEW: 3D models/textures
├── specs/002-3d-world-features/
│   ├── spec.md              # Feature specification
│   ├── plan.md              # Implementation plan
│   ├── research.md          # Technology decisions
│   ├── data-model.md        # Entity definitions
│   ├── contracts/           # Module interfaces
│   └── quickstart.md        # This file
└── tests/
```

## Development Commands

### Building

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized, for performance testing)
cargo build --release

# Check without building (faster feedback)
cargo check
```

### Running

```bash
# Debug mode
cargo run

# Release mode (required for 3D performance)
cargo run --release

# With logging
RUST_LOG=debug cargo run
```

### Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test sensors
cargo test workouts
cargo test world

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Full check (format + lint + test)
cargo fmt && cargo clippy && cargo test
```

## Key Development Patterns

### Thread Communication (Sensor → UI)

```rust
use crossbeam::channel::{unbounded, Receiver, Sender};

// Create channel pair
let (tx, rx): (Sender<SensorData>, Receiver<SensorData>) = unbounded();

// Send from async BLE thread
tx.send(SensorData { power: 250, .. }).unwrap();

// Receive in UI thread (non-blocking)
if let Ok(data) = rx.try_recv() {
    // Update UI with sensor data
}

// Request UI repaint from background thread
ctx.request_repaint();
```

### BLE Protocol (ERG Mode)

```rust
// FTMS Control Point characteristic
const FTMS_CONTROL_POINT: Uuid = uuid!("00002ad9-0000-1000-8000-00805f9b34fb");

// Request control before sending targets
peripheral.write(&control_char, &[0x00], WriteType::WithResponse).await?;

// Set ERG mode power target
let watts: u16 = 200;
let command = [0x05, (watts & 0xFF) as u8, (watts >> 8) as u8];
peripheral.write(&control_char, &command, WriteType::WithResponse).await?;
```

### wgpu Integration with egui

```rust
// Access wgpu from eframe
fn setup(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    if let Some(render_state) = frame.wgpu_render_state() {
        let device = &render_state.device;
        let queue = &render_state.queue;

        // Initialize 3D renderer
        self.world = World3D::new(render_state, world_def, route_def, avatar)?;
    }
}

// Render 3D to egui
fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    // Update world with power data
    self.world.update(self.current_power, ctx.input(|i| i.stable_dt));

    // Render and display
    egui::CentralPanel::default().show(ctx, |ui| {
        let texture_id = self.world.render();
        ui.image(texture_id, ui.available_size());
    });
}
```

### Database Operations

```rust
use rusqlite::{Connection, params};

// Open database
let conn = Connection::open(db_path)?;

// Run query
let rides: Vec<RideSummary> = conn.prepare("SELECT * FROM rides ORDER BY started_at DESC")?
    .query_map([], |row| {
        Ok(RideSummary {
            id: row.get("id")?,
            started_at: row.get("started_at")?,
            // ...
        })
    })?
    .collect::<Result<_, _>>()?;
```

## New Dependencies for This Feature

Add to `Cargo.toml`:

```toml
[dependencies]
# 3D Math
glam = "0.27"

# Model loading
gltf = "1.4"

# GPU buffer casting
bytemuck = { version = "1.14", features = ["derive"] }

# Image loading (textures)
image = { version = "0.25", default-features = false, features = ["png"] }

# Native file dialogs
rfd = "0.14"
```

## Feature Flags

```toml
[features]
default = ["3d"]
3d = []           # Enable 3D world rendering
mock-ble = []     # Use mock BLE for testing without hardware
```

```bash
# Build without 3D (for low-spec testing)
cargo build --no-default-features

# Build with mock BLE
cargo build --features mock-ble
```

## Testing Hardware

### BLE Sensors

For testing BLE functionality, you need:
- Smart trainer with FTMS support (e.g., Wahoo KICKR, Tacx NEO)
- Optional: Heart rate monitor with BLE
- Optional: Cadence sensor with BLE

Without hardware, use `--features mock-ble` for simulated sensors.

### GPU

For 3D testing:
- Any GPU with OpenGL 3.3+ or Vulkan 1.0+
- Integrated graphics (Intel/AMD) sufficient for development
- Discrete GPU recommended for performance testing

Check GPU capability:
```bash
# Linux
vulkaninfo | head -20

# Windows PowerShell
dxdiag
```

## Common Issues

### Linux: "Permission denied" for BLE

```bash
# Add user to bluetooth group
sudo usermod -a -G bluetooth $USER

# Restart bluetooth service
sudo systemctl restart bluetooth

# Log out and back in
```

### macOS: BLE not discovering devices

System Preferences → Security & Privacy → Privacy → Bluetooth
Ensure terminal/IDE is allowed.

### Windows: Visual Studio build tools missing

Download and install Visual Studio Build Tools:
https://visualstudio.microsoft.com/visual-cpp-build-tools/

Select "Desktop development with C++" workload.

### GPU: "Adapter not found"

```bash
# Check GPU drivers are up to date
# Linux:
sudo ubuntu-drivers autoinstall

# Windows: Update via Device Manager or vendor website
```

## Next Steps

1. Read `spec.md` for full feature requirements
2. Review `contracts/` for module interfaces
3. Check `data-model.md` for entity definitions
4. Start with P1 tasks (Sensor Control, Ride Persistence)
5. Run `/speckit.tasks` to generate task list

## Resources

- [wgpu Documentation](https://docs.rs/wgpu)
- [egui Documentation](https://docs.rs/egui)
- [btleplug Documentation](https://docs.rs/btleplug)
- [GLTF Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- [FTMS Protocol Specification](https://www.bluetooth.com/specifications/specs/fitness-machine-service-1-0/)
