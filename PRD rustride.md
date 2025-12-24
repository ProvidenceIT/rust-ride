# RustRide - Product Requirements Document

**Version:** 1.0  
**Date:** December 2025  
**Author:** Providence IT  
**Status:** Draft

---

## Executive Summary

RustRide is an open-source, self-hosted indoor cycling training application built in Rust. It provides a lightweight alternative to Zwift, focusing on essential training features without the subscription cost. The MVP prioritizes sensor connectivity, structured workouts, and ride data recording over gamification and social features.

---

## Problem Statement

Current indoor cycling platforms like Zwift require ongoing subscriptions (€17.99/month), are resource-heavy, and include many features unnecessary for focused training. Cyclists who want a straightforward, cost-effective training tool lack good self-hosted alternatives.

**Target User:** Cyclists who want structured indoor training with their smart trainer without monthly fees.

---

## Goals & Non-Goals

### Goals (MVP)
- Connect to ANT+ and Bluetooth smart trainers and sensors
- Display real-time cycling metrics (power, cadence, heart rate, speed)
- Support ERG mode for structured workouts
- Import and execute structured workouts (.zwo, .fit, .mrc formats)
- Record rides and export to standard formats (.fit, .tcx)
- Provide a simple, functional UI that doesn't require a gaming GPU
- Run on Linux, Windows, and macOS

### Non-Goals (MVP)
- 3D virtual worlds or avatars
- Multiplayer/group rides
- Social features (following, kudos, leaderboards)
- Route simulation with gradient changes (SIM mode)
- Mobile apps
- Cloud sync or online accounts

---

## Core Features

### 1. Sensor Connectivity

**Priority:** Critical

| Sensor Type | Protocol | Data Points |
|-------------|----------|-------------|
| Smart Trainer | ANT+ FE-C, BLE FTMS | Power, Speed, Cadence |
| Power Meter | ANT+, BLE | Power, Cadence |
| Heart Rate | ANT+, BLE | Heart Rate |
| Cadence Sensor | ANT+, BLE | Cadence |
| Speed Sensor | ANT+, BLE | Speed |

**Requirements:**
- Auto-discovery of nearby sensors
- Manual sensor pairing with device ID
- Simultaneous connection to multiple sensors
- Connection status indicators
- Automatic reconnection on signal loss
- Sensor battery level display (where supported)

**Technical Notes:**
- Use `btleplug` crate for BLE on all platforms
- Use `ant-rs` or custom ANT+ USB stick implementation
- Support common ANT+ USB sticks (Garmin, Suunto)

### 2. Real-Time Metrics Display

**Priority:** Critical

**Required Metrics:**
- Current Power (watts)
- 3-second Average Power
- Cadence (rpm)
- Heart Rate (bpm)
- Speed (km/h)
- Distance (km)
- Elapsed Time
- Calories (estimated)

**Derived Metrics:**
- Normalized Power (NP)
- Intensity Factor (IF) - requires FTP setting
- Training Stress Score (TSS) - requires FTP setting
- Power Zones (configurable)
- Heart Rate Zones (configurable)

**Display Requirements:**
- Large, readable numbers
- Color-coded zone indicators
- Configurable dashboard layout
- Full-screen mode support

### 3. ERG Mode Control

**Priority:** Critical

**Requirements:**
- Set target power on smart trainer
- Smooth power transitions (configurable ramp rate)
- Cadence-independent resistance (true ERG)
- Manual power adjustment (+/- 5W, +/- 10W buttons)
- ERG mode toggle (switch to free ride)
- Power smoothing/averaging options

**Calibration:**
- Support trainer spindown calibration
- Store calibration values per trainer

### 4. Structured Workouts

**Priority:** Critical

**Workout File Support:**
- `.zwo` (Zwift workout format) - import
- `.mrc/.erg` (TrainerRoad/generic format) - import
- `.fit` (Garmin workout format) - import/export
- Native JSON format - full support

**Workout Components:**
- Warmup blocks (ramp from X to Y watts)
- Steady-state intervals (hold X watts for Y time)
- Interval sets (repeat N times)
- Recovery blocks
- Cooldown blocks (ramp down)
- Free ride sections

**Workout Player:**
- Visual workout profile graph
- Current interval highlight
- Time remaining in interval
- Time remaining in workout
- Next interval preview
- Pause/resume functionality
- Skip interval option
- Extend interval option (+30s, +1min)

**Workout Builder (Post-MVP):**
- Visual drag-and-drop builder
- Save custom workouts
- Workout library management

### 5. Free Ride Mode

**Priority:** High

**Requirements:**
- Ride without structured workout
- Manual resistance control (0-100%)
- Optional: Simulation mode with virtual gradient
- Target power mode (hold steady power)
- Set ride duration or distance goals

### 6. Ride Recording

**Priority:** Critical

**Data Captured (1-second resolution):**
- Timestamp
- Power (watts)
- Cadence (rpm)
- Heart Rate (bpm)
- Speed (km/h)
- Distance (cumulative)
- Calories (cumulative)
- Trainer resistance level
- Temperature (if available)

**Export Formats:**
- `.fit` (Garmin) - primary format
- `.tcx` (Garmin Training Center XML)
- `.gpx` (with extensions for power/hr)
- `.csv` (raw data export)

**Auto-Save:**
- Continuous saving during ride (crash recovery)
- Configurable auto-save interval (default: 30s)

### 7. User Profile & Settings

**Priority:** High

**Profile Settings:**
- FTP (Functional Threshold Power)
- Max Heart Rate
- Resting Heart Rate
- Weight (kg)
- Height (cm)
- Power Zones (auto-calculated or custom)
- Heart Rate Zones (auto-calculated or custom)

**Application Settings:**
- Units (metric/imperial)
- UI theme (light/dark)
- Audio cues (interval start/end, targets)
- Dashboard layout customization
- Default export format
- Auto-upload destinations (post-MVP)

### 8. Ride History

**Priority:** Medium

**Requirements:**
- List of past rides with summary stats
- Ride detail view with charts
- Power curve analysis
- Filter/search rides by date
- Delete rides
- Re-export rides

**Summary Stats per Ride:**
- Date/time
- Duration
- Distance
- Average/Max Power
- Normalized Power
- Average/Max HR
- Average Cadence
- TSS
- Calories

---

## Technical Architecture

### Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (stable) |
| GUI Framework | `egui` + `eframe` (immediate mode, cross-platform) |
| BLE | `btleplug` |
| ANT+ | Custom implementation or `libant` bindings |
| FIT Parsing | `fit-rs` or custom parser |
| Data Storage | SQLite via `rusqlite` |
| Config | TOML via `toml` crate |
| Async Runtime | `tokio` |
| Logging | `tracing` |
| Serialization | `serde` |

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        RustRide                              │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   GUI       │  │  Workout    │  │   Ride Recording    │  │
│  │  (egui)     │  │   Engine    │  │      Engine         │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│  ┌──────┴────────────────┴─────────────────────┴──────────┐ │
│  │                    Core Engine                          │ │
│  │  - Metrics aggregation                                  │ │
│  │  - Zone calculations                                    │ │
│  │  - TSS/NP/IF calculations                              │ │
│  └──────────────────────────┬─────────────────────────────┘ │
│                             │                                │
│  ┌──────────────────────────┴─────────────────────────────┐ │
│  │                 Sensor Manager                          │ │
│  │  ┌─────────────┐              ┌─────────────┐          │ │
│  │  │  BLE Stack  │              │  ANT+ Stack │          │ │
│  │  │ (btleplug)  │              │   (USB)     │          │ │
│  │  └─────────────┘              └─────────────┘          │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                 Storage Layer                           │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │ │
│  │  │  SQLite  │  │  Config  │  │  File Export/Import  │  │ │
│  │  │  (rides) │  │  (TOML)  │  │  (.fit, .zwo, etc)   │  │ │
│  │  └──────────┘  └──────────┘  └──────────────────────────┘│ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Data Models

```rust
// User Profile
struct UserProfile {
    id: Uuid,
    ftp: u16,                    // watts
    max_hr: u8,                  // bpm
    resting_hr: u8,              // bpm
    weight_kg: f32,
    height_cm: u16,
    power_zones: PowerZones,
    hr_zones: HeartRateZones,
}

// Power Zones (default: Coggan 7-zone)
struct PowerZones {
    z1_recovery: (u16, u16),     // % of FTP range
    z2_endurance: (u16, u16),
    z3_tempo: (u16, u16),
    z4_threshold: (u16, u16),
    z5_vo2max: (u16, u16),
    z6_anaerobic: (u16, u16),
    z7_neuromuscular: (u16, u16),
}

// Real-time sensor data
struct SensorData {
    timestamp: DateTime<Utc>,
    power_watts: Option<u16>,
    cadence_rpm: Option<u8>,
    heart_rate_bpm: Option<u8>,
    speed_kmh: Option<f32>,
}

// Ride record (1-second samples)
struct RideSample {
    elapsed_seconds: u32,
    power_watts: Option<u16>,
    cadence_rpm: Option<u8>,
    heart_rate_bpm: Option<u8>,
    speed_kmh: Option<f32>,
    distance_meters: f64,
    calories: u32,
}

// Ride summary
struct Ride {
    id: Uuid,
    started_at: DateTime<Utc>,
    duration_seconds: u32,
    distance_meters: f64,
    avg_power: u16,
    max_power: u16,
    normalized_power: u16,
    avg_hr: Option<u8>,
    max_hr: Option<u8>,
    avg_cadence: u8,
    tss: f32,
    calories: u32,
    workout_id: Option<Uuid>,
    samples: Vec<RideSample>,
}

// Workout definition
struct Workout {
    id: Uuid,
    name: String,
    description: Option<String>,
    author: Option<String>,
    segments: Vec<WorkoutSegment>,
    total_duration_seconds: u32,
}

enum WorkoutSegment {
    SteadyState {
        duration_seconds: u32,
        power_target: PowerTarget,
        cadence_target: Option<CadenceTarget>,
    },
    Ramp {
        duration_seconds: u32,
        power_start: PowerTarget,
        power_end: PowerTarget,
    },
    Intervals {
        repeat: u8,
        on_segment: Box<WorkoutSegment>,
        off_segment: Box<WorkoutSegment>,
    },
    FreeRide {
        duration_seconds: u32,
    },
}

enum PowerTarget {
    Absolute(u16),           // Fixed watts
    PercentFtp(u8),          // % of FTP
}
```

### Database Schema

```sql
-- Users table (support multiple profiles)
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ftp INTEGER NOT NULL DEFAULT 200,
    max_hr INTEGER,
    resting_hr INTEGER,
    weight_kg REAL NOT NULL,
    height_cm INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Rides table
CREATE TABLE rides (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    workout_id TEXT REFERENCES workouts(id),
    started_at TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    distance_meters REAL NOT NULL,
    avg_power INTEGER,
    max_power INTEGER,
    normalized_power INTEGER,
    avg_hr INTEGER,
    max_hr INTEGER,
    avg_cadence INTEGER,
    tss REAL,
    calories INTEGER,
    created_at TEXT NOT NULL
);

-- Ride samples (1-second data points)
CREATE TABLE ride_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    elapsed_seconds INTEGER NOT NULL,
    power_watts INTEGER,
    cadence_rpm INTEGER,
    heart_rate_bpm INTEGER,
    speed_kmh REAL,
    distance_meters REAL,
    calories INTEGER
);

CREATE INDEX idx_ride_samples_ride_id ON ride_samples(ride_id);

-- Workouts table
CREATE TABLE workouts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    author TEXT,
    source_file TEXT,
    segments_json TEXT NOT NULL,  -- JSON serialized segments
    total_duration_seconds INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- Paired sensors
CREATE TABLE sensors (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sensor_type TEXT NOT NULL,  -- 'trainer', 'power', 'hr', 'cadence', 'speed'
    protocol TEXT NOT NULL,      -- 'ble', 'ant+'
    last_seen_at TEXT,
    created_at TEXT NOT NULL
);
```

---

## User Interface

### Main Screens

#### 1. Home Screen
- Quick-start buttons: Free Ride, Workouts, Ride History
- Last ride summary card
- Weekly training summary (TSS, time, distance)
- Connected sensors status

#### 2. Sensor Setup Screen
- Discovered sensors list
- Paired sensors list
- Sensor signal strength indicator
- Pairing/unpairing controls
- Calibration trigger (for trainers)

#### 3. Workout Library Screen
- List of available workouts
- Import workout button
- Workout preview (duration, TSS estimate, profile graph)
- Filter by duration, type, intensity

#### 4. Ride Screen (Active Ride)
```
┌─────────────────────────────────────────────────────────────┐
│  [Pause]  [End Ride]                      Elapsed: 00:45:32 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│    ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   │
│    │  POWER  │   │ CADENCE │   │   HR    │   │  SPEED  │   │
│    │   245   │   │   92    │   │   152   │   │  32.4   │   │
│    │  watts  │   │   rpm   │   │   bpm   │   │  km/h   │   │
│    │ [Z4]    │   │         │   │  [Z3]   │   │         │   │
│    └─────────┘   └─────────┘   └─────────┘   └─────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │          WORKOUT PROGRESS BAR / GRAPH                   ││
│  │  [====current====>                                    ] ││
│  │  Interval: 3/8  |  Target: 250W  |  Remaining: 2:30    ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│    Distance: 24.3 km    Calories: 485    NP: 238W          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 5. Ride Summary Screen (Post-Ride)
- Ride statistics summary
- Power/HR/Cadence charts over time
- Power distribution histogram
- Export options
- Save/discard controls

#### 6. Settings Screen
- User profile settings
- Power/HR zone configuration
- Application preferences
- Sensor management link
- Data management (export all, clear data)

### UI/UX Requirements

- **Responsive:** Works on 1080p and above
- **Keyboard shortcuts:** Space (pause), Escape (end), +/- (adjust power)
- **High contrast:** Readable while sweating on the bike
- **Minimal clicks:** Start riding in ≤3 clicks from launch
- **No internet required:** Fully offline-capable

---

## Development Roadmap

### Phase 1: Foundation (Weeks 1-4)

**Week 1-2: Project Setup & BLE**
- [ ] Project structure and build setup
- [ ] BLE sensor discovery and connection
- [ ] Heart rate monitor support
- [ ] Basic egui window with sensor list

**Week 3-4: Trainer Control**
- [ ] BLE FTMS trainer connection
- [ ] ERG mode control (set target power)
- [ ] Read power/cadence/speed from trainer
- [ ] Basic ride screen with live metrics

### Phase 2: Core Training (Weeks 5-8)

**Week 5-6: Ride Recording**
- [ ] SQLite database setup
- [ ] Ride recording engine (1s samples)
- [ ] Auto-save with crash recovery
- [ ] Basic ride history list

**Week 7-8: Workout Engine**
- [ ] Workout data model
- [ ] .zwo file parser
- [ ] Workout player with ERG targets
- [ ] Workout progress visualization

### Phase 3: Polish & Export (Weeks 9-12)

**Week 9-10: File Formats**
- [ ] .fit file export
- [ ] .tcx file export
- [ ] .mrc/.erg workout import
- [ ] User profile and zone configuration

**Week 11-12: UI Polish**
- [ ] Dashboard customization
- [ ] Ride summary with charts
- [ ] Dark/light themes
- [ ] Keyboard shortcuts

### Phase 4: Extended Connectivity (Post-MVP)

- [ ] ANT+ support via USB dongle
- [ ] ANT+ FE-C trainer control
- [ ] Multiple simultaneous sensors
- [ ] Trainer calibration support

---

## Success Criteria (MVP)

| Metric | Target |
|--------|--------|
| Connect to BLE trainer | < 30 seconds |
| Start workout from launch | < 3 clicks |
| Ride data loss on crash | < 30 seconds |
| Memory usage | < 200 MB |
| CPU usage during ride | < 10% |
| Supported trainers | Wahoo, Tacx, Elite (FTMS) |
| Export compatibility | Strava, Garmin Connect |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| BLE cross-platform issues | High | Test early on all platforms; use proven `btleplug` |
| ANT+ USB complexity | Medium | Defer to post-MVP; BLE covers most modern trainers |
| Workout format parsing | Medium | Start with .zwo only; add formats incrementally |
| Performance during rides | High | Profile early; use efficient data structures |

---

## Future Enhancements (Post-MVP)

1. **Virtual Routes:** Simple 2D route display with gradient simulation (SIM mode)
2. **Strava/Garmin Auto-Upload:** OAuth integration for automatic ride uploads
3. **Training Plans:** Multi-week structured training programs
4. **Video Integration:** Sync with YouTube cycling videos
5. **Power Analysis:** Power curve, peak powers, FTP estimation
6. **Multiplayer:** LAN-based group rides (no central server)
7. **Mobile Companion:** Phone app for remote control
8. **Voice Announcements:** Interval cues via text-to-speech

---

## Appendix A: Supported Sensor Protocols

### BLE FTMS (Fitness Machine Service)
- UUID: `0x1826`
- Used by: Wahoo, Elite, Saris, most modern trainers
- Features: Power, speed, cadence, resistance control

### BLE Cycling Power Service
- UUID: `0x1818`
- Used by: Power meters, some trainers
- Features: Power, cadence (if supported)

### BLE Heart Rate Service
- UUID: `0x180D`
- Used by: All BLE HR monitors
- Features: Heart rate, RR intervals

### ANT+ FE-C (Fitness Equipment Control)
- Device Type: 17
- Used by: Wahoo, Tacx, Elite trainers
- Features: Full trainer control, power, speed, cadence

---

## Appendix B: .zwo Workout Format Reference

```xml
<workout_file>
    <name>Sweet Spot 2x20</name>
    <description>Two 20-minute sweet spot intervals</description>
    <workout>
        <Warmup Duration="600" PowerLow="0.40" PowerHigh="0.70"/>
        <SteadyState Duration="1200" Power="0.88"/>
        <SteadyState Duration="300" Power="0.50"/>
        <SteadyState Duration="1200" Power="0.88"/>
        <Cooldown Duration="600" PowerLow="0.70" PowerHigh="0.40"/>
    </workout>
</workout_file>
```

Power values are expressed as decimal fractions of FTP (0.88 = 88% FTP).

---

## Appendix C: Useful Crates

| Crate | Purpose | Notes |
|-------|---------|-------|
| `btleplug` | BLE communication | Cross-platform, async |
| `egui` | GUI framework | Immediate mode, fast |
| `eframe` | egui native wrapper | Easy setup |
| `tokio` | Async runtime | Mature, well-documented |
| `rusqlite` | SQLite bindings | Bundled SQLite option |
| `serde` | Serialization | JSON, TOML support |
| `quick-xml` | XML parsing | For .zwo files |
| `chrono` | Date/time | Timestamp handling |
| `uuid` | UUID generation | Record IDs |
| `tracing` | Logging | Structured logging |
| `anyhow` | Error handling | Convenient error types |
| `directories` | Platform paths | Config/data directories |

---

*Document End*