# Research: RustRide Technology Decisions

**Feature Branch**: `001-indoor-cycling-app`
**Date**: 2025-12-24

## 1. BLE Communication Library

### Decision: `btleplug` v0.11.8

### Rationale
- Only mature cross-platform Rust BLE library with active maintenance
- Latest version 0.11.8 (April 2025) with continuous updates throughout 2024-2025
- Async-first design built on tokio runtime
- Supports Windows 10+, macOS, Linux (BlueZ), iOS, and Android
- 1,030+ GitHub stars, 69 contributors, proven in production

### Alternatives Considered
| Alternative | Reason Rejected |
|-------------|-----------------|
| `bluster` | Peripheral/server mode only, not for connecting to trainers |
| `ble-peripheral-rust` | Also peripheral-only |
| Platform-specific SDKs | Would require separate implementations per platform |

### Implementation Notes
```toml
[dependencies]
btleplug = { version = "0.11", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
uuid = "1.0"
```

**Cross-Platform Considerations:**
- macOS: Requires `NSBluetoothAlwaysUsageDescription` in Info.plist
- Linux: Requires BlueZ and appropriate permissions
- Windows: Works with Windows 10+ built-in BLE stack

---

## 2. BLE Fitness Protocols

### 2.1 Fitness Machine Service (FTMS) - Smart Trainers

**Service UUID**: `0x1826`

**Key Characteristics:**
| Characteristic | UUID | Operations | Purpose |
|---------------|------|------------|---------|
| Indoor Bike Data | `0x2AD2` | NOTIFY | Power, cadence, speed (~1/sec) |
| Fitness Machine Control Point | `0x2AD9` | WRITE, INDICATE | ERG mode control |
| Fitness Machine Feature | `0x2ACC` | READ | Supported features |
| Training Status | `0x2AD3` | READ, NOTIFY | Current training state |

**ERG Mode Control Sequence:**
```rust
// 1. Request control
write(&control_point, &[0x00], WriteType::WithResponse);

// 2. Start training session
write(&control_point, &[0x07], WriteType::WithResponse);

// 3. Set target power (OpCode 0x05)
let target_watts: u16 = 250;
let mut data = vec![0x05]; // OpCode
data.extend_from_slice(&target_watts.to_le_bytes());
write(&control_point, &data, WriteType::WithResponse);
// 250 watts = [0x05, 0xFA, 0x00]
```

**Indoor Bike Data Parsing:**
- First 2 bytes: Flags field (indicates which data fields present)
- Variable length based on flags
- Little-endian encoding for all values

### 2.2 Cycling Power Service - Power Meters

**Service UUID**: `0x1818`

**Key Characteristics:**
| Characteristic | UUID | Operations | Purpose |
|---------------|------|------------|---------|
| Cycling Power Measurement | `0x2A63` | NOTIFY | Power data |
| Cycling Power Feature | `0x2A65` | READ | Capabilities |

**Measurement Format:**
```rust
// [flags: u16][instantaneous_power: u16][optional fields...]
let flags = u16::from_le_bytes([data[0], data[1]]);
let power_watts = u16::from_le_bytes([data[2], data[3]]);
```

### 2.3 Heart Rate Service

**Service UUID**: `0x180D`

**Key Characteristics:**
| Characteristic | UUID | Operations | Purpose |
|---------------|------|------------|---------|
| Heart Rate Measurement | `0x2A37` | NOTIFY | HR data |
| Body Sensor Location | `0x2A38` | READ | Sensor location |

**Measurement Format:**
```rust
let flags = data[0];
let hr_format_u16 = (flags & 0x01) != 0;

let heart_rate = if hr_format_u16 {
    u16::from_le_bytes([data[1], data[2]])
} else {
    data[1] as u16
};
```

---

## 3. GUI Framework

### Decision: `egui` v0.33.2 + `eframe`

### Rationale
- Immediate mode GUI: Simple mental model, efficient for real-time data
- Small binary size (<5MB)
- Cross-platform (Windows, macOS, Linux)
- 60fps capable with minimal CPU usage (1-2ms per frame)
- Active development with frequent releases

### Alternatives Considered
| Alternative | Reason Rejected |
|-------------|-----------------|
| Tauri (web UI) | Higher memory footprint, JavaScript overhead |
| Iced | Retained mode complexity for real-time updates |
| GTK/Qt bindings | Heavy dependencies, complex cross-platform builds |
| Native platform UIs | Separate implementations per platform |

### Real-Time Data Patterns

**Thread Communication:**
```rust
// Use crossbeam channels for sensor data
use crossbeam::channel::{unbounded, Receiver, Sender};

// BLE thread sends data
sensor_tx.send(data).unwrap();
ctx.request_repaint(); // Wake UI thread

// UI thread receives (non-blocking)
while let Ok(data) = self.sensor_rx.try_recv() {
    self.current_metrics = data;
}
```

**tokio Integration:**
- Spawn tokio runtime in separate thread
- Use channels between tokio and egui threads
- `egui::Context` is `Send + Sync`, can call `request_repaint()` from any thread

**Chart Performance:**
- egui_plot handles ~7,200 points (2-hour ride) well
- For ride history, use min-max downsampling to ~1,000 display points
- Cache plot data using egui's `FrameCache` system

---

## 4. FIT File Export

### Decision: Dual approach - TCX primary, FIT secondary

### Rationale
- No mature Rust crate for FIT file writing from scratch
- TCX is XML-based, easier to implement and debug
- Both formats accepted by Strava and Garmin Connect
- TCX doesn't require manufacturer ID registration

### Primary: TCX Export

**Implementation:**
```toml
[dependencies]
quick-xml = { version = "0.31", features = ["serialize"] }
serde = { version = "1.0", features = ["derive"] }
```

**TCX Structure for Cycling:**
```xml
<TrainingCenterDatabase>
  <Activities>
    <Activity Sport="Biking">
      <Lap StartTime="...">
        <Track>
          <Trackpoint>
            <Time>2024-12-24T10:00:00Z</Time>
            <HeartRateBpm><Value>145</Value></HeartRateBpm>
            <Cadence>90</Cadence>
            <Extensions>
              <TPX xmlns="..."><Watts>250</Watts></TPX>
            </Extensions>
          </Trackpoint>
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>
```

### Secondary: FIT Export (Post-MVP)

**Options:**
1. Use `fit-rust` crate if it supports creation (needs testing)
2. Implement minimal FIT encoder for cycling activities
3. Reference: [Garmin FIT SDK](https://developer.garmin.com/fit/)

### Alternatives Considered
| Format | Status |
|--------|--------|
| FIT (primary) | Deferred - no good write crate |
| GPX | Supported but lacks power extension support in `gpx` crate |
| CSV | Easy to implement, useful for data analysis |

---

## 5. Workout File Parsing

### Decision: `quick-xml` for .zwo, custom parser for .mrc/.erg

### .zwo Format (Zwift)
- XML-based format
- Power targets as decimal fractions of FTP (0.88 = 88%)
- Use `quick-xml` with serde deserialization

```rust
#[derive(Deserialize)]
struct ZwoWorkout {
    name: String,
    workout: Vec<ZwoSegment>,
}

#[derive(Deserialize)]
enum ZwoSegment {
    Warmup { duration: u32, power_low: f32, power_high: f32 },
    SteadyState { duration: u32, power: f32 },
    Cooldown { duration: u32, power_low: f32, power_high: f32 },
    // ...
}
```

### .mrc/.erg Format (TrainerRoad)
- Line-based text format
- Custom parser (simple state machine)
- Format: `duration_minutes power_percent` pairs

---

## 6. Data Persistence

### Decision: `rusqlite` with bundled SQLite

### Rationale
- Zero-config, single-file database
- Embedded (no external dependencies)
- Bundled SQLite avoids system library version issues
- Perfect for local desktop application

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

### Schema Design
- Rides table: Summary stats
- Ride samples: 1-second data points (indexed by ride_id)
- Workouts: Parsed workout definitions
- Sensors: Remembered device pairings
- Users: Profile settings

---

## 7. Async Runtime

### Decision: `tokio` (multi-threaded runtime)

### Rationale
- Required by `btleplug`
- Industry standard for async Rust
- Multi-threaded runtime for BLE scanning
- Well-documented, mature ecosystem

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "macros"] }
```

### Threading Model
1. **Main thread**: egui UI (60fps when active)
2. **tokio threads**: BLE sensor communication
3. **Background thread**: SQLite writes (auto-save every 30s)

**Communication:**
- BLE → UI: `crossbeam::channel` (non-blocking)
- UI ← Database: `Arc<Vec<Ride>>` (cheap cloning)
- Recording buffer: `Arc<Mutex<Vec<RideSample>>>`

---

## 8. Configuration

### Decision: TOML configuration files

```toml
[dependencies]
toml = "0.8"
directories = "5.0"
```

### Configuration Structure
- User profile: FTP, zones, weight, height
- Application settings: Theme, units, audio cues
- Sensor pairings: Device IDs and names

### Storage Locations
- Windows: `%APPDATA%\rustride\`
- macOS: `~/Library/Application Support/rustride/`
- Linux: `~/.config/rustride/`

Use `directories` crate for cross-platform paths.

---

## 9. Logging

### Decision: `tracing` + `tracing-subscriber`

### Rationale
- Structured logging with spans
- Easy filtering by module
- Multiple output formats (console, file)
- Standard in Rust ecosystem

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## 10. Reference Projects

### Jet Ordaneza's Cycling Trainer
- Rust + btleplug + Tauri
- Supports Wahoo Kickr, .zwo workouts
- Good reference for FTMS implementation
- GitHub: Check author's profile

### pycycling (Python)
- Reference implementation for BLE fitness protocols
- Useful for protocol verification

### Dependency Summary

```toml
[dependencies]
# Core
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "macros"] }
serde = { version = "1.0", features = ["derive"] }

# BLE
btleplug = { version = "0.11", features = ["serde"] }
uuid = "1.0"

# GUI
eframe = "0.33"
egui_plot = "0.33"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }
toml = "0.8"
directories = "5.0"

# File Formats
quick-xml = { version = "0.31", features = ["serialize"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
crossbeam = "0.8"
```
