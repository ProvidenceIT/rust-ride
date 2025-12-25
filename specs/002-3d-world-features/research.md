# Research: 3D Virtual World & Complete Feature Implementation

**Feature**: 002-3d-world-features
**Date**: 2025-12-24
**Status**: Complete

## 1. 3D Rendering in Rust with egui

### Decision: wgpu + egui integration

**Rationale**: wgpu is the standard Rust graphics library, providing cross-platform GPU access (Vulkan, Metal, DX12, OpenGL). It integrates well with egui through the `egui_wgpu` backend that eframe already uses internally.

**Alternatives Considered**:

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| wgpu (direct) | Full control, egui already uses it | More code to write | **Selected** |
| bevy | Full game engine, ECS | Heavy, would replace egui entirely | Rejected |
| rend3 | Higher-level wgpu wrapper | Less mature, smaller community | Rejected |
| three-d | Easy 3D abstractions | Less control over rendering | Considered |

**Implementation Approach**:
1. Use `eframe::egui_wgpu::RenderState` to access wgpu device/queue
2. Create custom render pass for 3D scene before egui UI pass
3. Render 3D to texture, display in egui `Image` widget for HUD overlay
4. Share wgpu context between egui and 3D renderer

**Key Dependencies**:
```toml
wgpu = "0.20"           # GPU abstraction (already via eframe)
glam = "0.27"           # 3D math (vectors, matrices, quaternions)
bytemuck = "1.14"       # Safe casting for GPU buffers
```

---

## 2. Asset Pipeline

### Decision: GLTF format with embedded textures

**Rationale**: GLTF 2.0 is the standard 3D interchange format, well-supported by Blender and other tools. The `gltf` crate provides robust loading in Rust.

**Alternatives Considered**:

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| GLTF (.glb binary) | Single file, fast loading | Larger file size | **Selected** |
| OBJ + MTL | Simple, widely supported | No animations, multiple files | Rejected |
| FBX | Industry standard | Complex, proprietary | Rejected |
| Custom format | Optimized for our needs | Development overhead | Rejected |

**Asset Structure**:
```
assets/
├── models/
│   ├── cyclist.glb          # Avatar with cycling animation
│   ├── bikes/
│   │   ├── road_bike.glb
│   │   ├── tt_bike.glb
│   │   └── gravel_bike.glb
│   └── environment/
│       ├── tree_01.glb
│       ├── building_01.glb
│       └── road_segment.glb
├── textures/
│   ├── terrain/
│   │   ├── grass.png
│   │   ├── asphalt.png
│   │   └── dirt.png
│   └── sky/
│       └── hdri_sky.hdr
└── worlds/
    ├── countryside.json      # World definition + route data
    ├── mountains.json
    └── coastal.json
```

**Key Dependencies**:
```toml
gltf = "1.4"            # GLTF loading
image = "0.25"          # Texture loading
```

**Asset Bundling**:
- Development: Load from `assets/` directory relative to executable
- Release: Embed using `include_bytes!` or distribute as separate archive
- Total size budget: <500MB for all worlds combined

---

## 3. Physics Model: Power to Speed

### Decision: Simplified cycling physics model

**Rationale**: Real cycling physics involves complex aerodynamics, but for indoor training visualization, a simplified model provides responsive, intuitive feedback.

**Physics Equations**:

```
Power (W) = Force × Velocity
Force = F_gravity + F_rolling + F_air

Where:
- F_gravity = mass × g × sin(gradient)
- F_rolling = Crr × mass × g × cos(gradient)
- F_air = 0.5 × ρ × CdA × v²

Solving for velocity given power:
v = solve(P = v × (mg×sin(θ) + Crr×mg×cos(θ) + 0.5×ρ×CdA×v²))
```

**Default Parameters**:
| Parameter | Symbol | Default Value | Notes |
|-----------|--------|---------------|-------|
| Air density | ρ | 1.225 kg/m³ | Sea level |
| CdA (drag area) | CdA | 0.32 m² | Hoods position |
| Rolling resistance | Crr | 0.004 | Road tires |
| Gravity | g | 9.81 m/s² | Standard |
| Rider mass | m | From profile | User configurable |

**Implementation**:
```rust
fn calculate_speed(power_watts: f32, mass_kg: f32, gradient_percent: f32) -> f32 {
    // Newton-Raphson iteration to solve cubic equation
    // Returns speed in m/s
}
```

**Simplifications**:
- No wind (virtual environment has no weather)
- Constant CdA (no position changes)
- Instant power application (no drivetrain lag)
- No drafting (single rider only)

---

## 4. Native File Dialogs

### Decision: rfd (Rusty File Dialogs)

**Rationale**: rfd provides native file dialogs on all platforms with a simple async API. It's well-maintained and used by many Rust GUI applications.

**Alternatives Considered**:

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| rfd | Native look, async, cross-platform | External dependency | **Selected** |
| nfd2 | Native dialogs | Less active maintenance | Rejected |
| tinyfiledialogs | Minimal | No async, blocking | Rejected |
| Custom egui dialog | Consistent UI | Non-native feel, more work | Rejected |

**Usage Pattern**:
```rust
use rfd::AsyncFileDialog;

async fn pick_workout_file() -> Option<PathBuf> {
    let file = AsyncFileDialog::new()
        .add_filter("Workouts", &["zwo", "mrc"])
        .set_title("Import Workout")
        .pick_file()
        .await?;

    Some(file.path().to_path_buf())
}
```

**Key Dependency**:
```toml
rfd = "0.14"            # Native file dialogs
```

---

## 5. BLE Integration Completion

### Decision: Complete btleplug implementation with async channels

**Rationale**: The existing btleplug integration has TODOs for actual BLE operations. We'll complete these using the established crossbeam channel pattern for UI communication.

**Current State Analysis**:
- `SensorManager::start_discovery()` - Needs to trigger `adapter.start_scan()`
- `SensorManager::connect()` - Needs actual peripheral connection
- `SensorManager::disconnect()` - Needs peripheral disconnection
- UI buttons exist but call empty implementations

**Implementation Plan**:

1. **Start Scanning**:
```rust
pub async fn start_discovery(&mut self) -> Result<(), SensorError> {
    let adapter = self.adapter.as_ref().ok_or(AdapterNotFound)?;

    // Already implemented - just need to wire up UI button
    adapter.start_scan(ScanFilter { services: vec![FTMS_UUID, ...] }).await?;

    // Spawn event listener (already exists)
    self.spawn_discovery_listener();
    Ok(())
}
```

2. **Connect to Sensor**:
```rust
pub async fn connect(&mut self, device_id: &str) -> Result<(), SensorError> {
    // Find peripheral, connect, discover services, subscribe
    // Already implemented - just wire to UI
}
```

3. **Disconnect**:
```rust
pub async fn disconnect(&mut self, device_id: &str) -> Result<(), SensorError> {
    if let Some(peripheral) = self.connected.remove(device_id) {
        peripheral.disconnect().await?;
    }
    Ok(())
}
```

**UI Integration Pattern**:
- UI calls method that sends command to async runtime via channel
- Async runtime executes BLE operation
- Result sent back via crossbeam channel
- UI polls channel each frame for updates

---

## 6. Crash Recovery Implementation

### Decision: SQLite autosave table with 30-second intervals

**Rationale**: The database already has autosave support planned. Implementing it provides resilience against crashes during long rides.

**Schema**:
```sql
CREATE TABLE IF NOT EXISTS autosave (
    id INTEGER PRIMARY KEY,
    ride_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_sample_at TEXT NOT NULL,
    samples_json TEXT NOT NULL,  -- JSON array of samples
    workout_id TEXT,             -- If workout in progress
    workout_elapsed INTEGER,     -- Seconds into workout
    updated_at TEXT NOT NULL
);

-- Only one autosave row exists at a time
```

**Recovery Flow**:
1. App startup: Check `SELECT * FROM autosave LIMIT 1`
2. If row exists: Show recovery dialog
3. User chooses "Recover": Load samples, create Ride, navigate to summary
4. User chooses "Discard": `DELETE FROM autosave`
5. During ride: `INSERT OR REPLACE` every 30 seconds

---

## 7. Light Theme Colors

### Decision: Material Design-inspired light palette

**Rationale**: Consistent with dark theme's Material Design influence, light theme uses complementary colors with sufficient contrast for outdoor/bright room use.

**Color Palette**:
```rust
pub fn light_colors() -> ThemeColors {
    ThemeColors {
        background: Color32::from_rgb(250, 250, 252),
        surface: Color32::from_rgb(255, 255, 255),
        primary: Color32::from_rgb(25, 118, 210),      // Blue 700
        secondary: Color32::from_rgb(156, 39, 176),    // Purple 500
        success: Color32::from_rgb(46, 125, 50),       // Green 700
        warning: Color32::from_rgb(245, 124, 0),       // Orange 700
        error: Color32::from_rgb(211, 47, 47),         // Red 700
        text_primary: Color32::from_rgb(33, 33, 33),
        text_secondary: Color32::from_rgb(117, 117, 117),
        border: Color32::from_rgb(224, 224, 224),
    }
}
```

---

## 8. Power Distribution Chart

### Decision: Histogram with zone coloring using egui_plot

**Rationale**: egui_plot (already a dependency) supports bar charts. Power distribution visualizes time spent in each zone.

**Implementation**:
```rust
fn render_power_distribution(ui: &mut Ui, samples: &[RideSample], ftp: u16) {
    let zones = PowerZones::from_ftp(ftp);
    let mut zone_seconds = [0u32; 7];

    for sample in samples {
        if let Some(power) = sample.power_watts {
            let zone = zones.get_zone(power);
            zone_seconds[(zone - 1) as usize] += 1;
        }
    }

    // Render as colored bar chart
    Plot::new("power_dist")
        .show(ui, |plot_ui| {
            for (i, seconds) in zone_seconds.iter().enumerate() {
                let bar = Bar::new(i as f64, *seconds as f64)
                    .fill(zone_color(i + 1));
                plot_ui.bar(bar);
            }
        });
}
```

---

## Summary of Technology Decisions

| Area | Decision | Key Crate |
|------|----------|-----------|
| 3D Rendering | wgpu via eframe | wgpu 0.20 |
| 3D Math | glam vectors/matrices | glam 0.27 |
| Model Loading | GLTF binary format | gltf 1.4 |
| Textures | PNG/HDR via image | image 0.25 |
| File Dialogs | Native via rfd | rfd 0.14 |
| Physics | Simplified cycling model | (custom) |
| BLE | Complete btleplug impl | btleplug 0.11 |
| Recovery | SQLite autosave table | rusqlite 0.31 |

## New Cargo.toml Dependencies

```toml
# 3D Rendering (wgpu accessed via eframe internals)
glam = "0.27"           # 3D math
gltf = "1.4"            # Model loading
bytemuck = { version = "1.14", features = ["derive"] }  # GPU buffer casting

# Image loading
image = { version = "0.25", default-features = false, features = ["png"] }

# File dialogs
rfd = "0.14"
```
