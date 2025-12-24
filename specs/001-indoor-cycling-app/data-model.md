# Data Model: RustRide Indoor Cycling Application

**Feature Branch**: `001-indoor-cycling-app`
**Date**: 2025-12-24

## Entity Relationship Overview

```
┌─────────────────┐
│   UserProfile   │
└────────┬────────┘
         │ 1
         │
         │ owns many
         ▼
┌─────────────────┐     ┌─────────────────┐
│     Sensor      │     │     Workout     │
└─────────────────┘     └────────┬────────┘
                                 │ 1
                                 │
                                 │ contains many
                                 ▼
                        ┌─────────────────┐
                        │ WorkoutSegment  │
                        └─────────────────┘

┌─────────────────┐
│      Ride       │──────────────────────┐
└────────┬────────┘                      │ 0..1
         │ 1                             │
         │                               │ based on
         │ contains many                 ▼
         ▼                      ┌─────────────────┐
┌─────────────────┐             │     Workout     │
│   RideSample    │             └─────────────────┘
└─────────────────┘
```

---

## Core Entities

### UserProfile

Represents the cyclist with their physiological data and application preferences.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | Primary key | Unique identifier |
| name | String | Required, 1-100 chars | Display name |
| ftp | u16 | 50-600, default 200 | Functional Threshold Power in watts |
| max_hr | Option<u8> | 100-220 | Maximum heart rate in bpm |
| resting_hr | Option<u8> | 30-100 | Resting heart rate in bpm |
| weight_kg | f32 | 30.0-200.0 | Weight in kilograms |
| height_cm | Option<u16> | 100-250 | Height in centimeters |
| power_zones | PowerZones | - | Power training zones |
| hr_zones | Option<HRZones> | - | Heart rate training zones |
| units | Units | Default: Metric | Unit preference (Metric/Imperial) |
| theme | Theme | Default: Dark | UI theme preference |
| created_at | DateTime | Required | Profile creation timestamp |
| updated_at | DateTime | Required | Last modification timestamp |

**Derived Fields:**
- Power zones auto-calculated from FTP using Coggan 7-zone model
- HR zones auto-calculated from max_hr and resting_hr using Karvonen formula

**Validation Rules:**
- FTP must be between 50 and 600 watts
- Weight must be positive and reasonable (30-200 kg)
- max_hr must be greater than resting_hr

---

### PowerZones

Power training zones based on percentage of FTP.

| Field | Type | Description |
|-------|------|-------------|
| z1_recovery | ZoneRange | 0-55% FTP |
| z2_endurance | ZoneRange | 56-75% FTP |
| z3_tempo | ZoneRange | 76-90% FTP |
| z4_threshold | ZoneRange | 91-105% FTP |
| z5_vo2max | ZoneRange | 106-120% FTP |
| z6_anaerobic | ZoneRange | 121-150% FTP |
| z7_neuromuscular | ZoneRange | >150% FTP |
| custom | bool | Whether zones are user-customized |

**ZoneRange:**
| Field | Type | Description |
|-------|------|-------------|
| min_percent | u8 | Minimum % of FTP |
| max_percent | u8 | Maximum % of FTP |
| color | Color | Display color (RGBA) |
| name | String | Zone name |

---

### HRZones

Heart rate training zones.

| Field | Type | Description |
|-------|------|-------------|
| z1_recovery | HRZoneRange | Zone 1 (50-60% HRR) |
| z2_aerobic | HRZoneRange | Zone 2 (60-70% HRR) |
| z3_tempo | HRZoneRange | Zone 3 (70-80% HRR) |
| z4_threshold | HRZoneRange | Zone 4 (80-90% HRR) |
| z5_maximum | HRZoneRange | Zone 5 (90-100% HRR) |
| custom | bool | Whether zones are user-customized |

**HRZoneRange:**
| Field | Type | Description |
|-------|------|-------------|
| min_bpm | u8 | Minimum heart rate |
| max_bpm | u8 | Maximum heart rate |
| color | Color | Display color |
| name | String | Zone name |

---

### Sensor

Represents a BLE fitness sensor device.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | Primary key | Unique identifier |
| user_id | UUID | Foreign key → UserProfile | Owner |
| device_id | String | Required | BLE device address/identifier |
| name | String | Required | User-friendly name |
| sensor_type | SensorType | Required | Type of sensor |
| protocol | Protocol | Required | Communication protocol |
| last_seen_at | Option<DateTime> | - | Last successful connection |
| is_primary | bool | Default: false | Primary data source for type |
| created_at | DateTime | Required | First pairing timestamp |

**SensorType Enum:**
- `Trainer` - Smart trainer with FTMS support
- `PowerMeter` - Standalone power meter
- `HeartRate` - Heart rate monitor
- `Cadence` - Cadence sensor
- `Speed` - Speed sensor
- `SpeedCadence` - Combined speed/cadence sensor

**Protocol Enum:**
- `BleFtms` - BLE Fitness Machine Service
- `BleCyclingPower` - BLE Cycling Power Service
- `BleHeartRate` - BLE Heart Rate Service
- `BleCsc` - BLE Cycling Speed and Cadence

**Runtime State (not persisted):**
| Field | Type | Description |
|-------|------|-------------|
| connection_state | ConnectionState | Connected/Disconnecting/Disconnected |
| signal_strength | Option<i8> | RSSI value |
| battery_level | Option<u8> | Battery percentage (0-100) |
| last_data_at | Option<Instant> | Last data received |

---

### Workout

Represents a structured training session definition.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | Primary key | Unique identifier |
| name | String | Required, 1-200 chars | Workout name |
| description | Option<String> | Max 2000 chars | Workout description |
| author | Option<String> | Max 100 chars | Workout creator |
| source_file | Option<String> | - | Original import file path |
| source_format | Option<WorkoutFormat> | - | Original file format |
| segments | Vec<WorkoutSegment> | Min 1 segment | Ordered list of segments |
| total_duration_seconds | u32 | Calculated | Total workout duration |
| estimated_tss | Option<f32> | Calculated | Estimated Training Stress Score |
| estimated_if | Option<f32> | Calculated | Estimated Intensity Factor |
| tags | Vec<String> | - | User-defined tags |
| created_at | DateTime | Required | Import/creation timestamp |

**WorkoutFormat Enum:**
- `Zwo` - Zwift workout format (.zwo)
- `Mrc` - TrainerRoad/generic format (.mrc, .erg)
- `Fit` - Garmin workout format (.fit)
- `Native` - RustRide native format (JSON)

**Validation Rules:**
- Workout must have at least one segment
- Total duration must equal sum of segment durations
- Estimated TSS calculated as: (duration_hours × IF² × 100)

---

### WorkoutSegment

Represents a portion of a workout with specific power/cadence targets.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| segment_type | SegmentType | Required | Type of segment |
| duration_seconds | u32 | > 0 | Segment duration |
| power_target | PowerTarget | Required | Power target specification |
| cadence_target | Option<CadenceTarget> | - | Optional cadence target |
| text_event | Option<String> | Max 200 chars | On-screen text message |

**SegmentType Enum:**
- `Warmup` - Gradual power increase
- `Cooldown` - Gradual power decrease
- `SteadyState` - Constant power
- `Intervals` - Repeating on/off blocks
- `FreeRide` - No ERG target
- `Ramp` - Linear power change

**PowerTarget Enum:**
| Variant | Fields | Description |
|---------|--------|-------------|
| Absolute | watts: u16 | Fixed wattage |
| PercentFtp | percent: u8 | Percentage of user's FTP |
| Range | start: PowerTarget, end: PowerTarget | For ramps |

**CadenceTarget:**
| Field | Type | Description |
|-------|------|-------------|
| min_rpm | u8 | Minimum target cadence |
| max_rpm | u8 | Maximum target cadence |

---

### Ride

Represents a completed or in-progress training session.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | UUID | Primary key | Unique identifier |
| user_id | UUID | Foreign key → UserProfile | Rider |
| workout_id | Option<UUID> | Foreign key → Workout | Associated workout (if any) |
| started_at | DateTime | Required | Ride start timestamp |
| ended_at | Option<DateTime> | - | Ride end timestamp |
| duration_seconds | u32 | Calculated | Active riding time |
| distance_meters | f64 | ≥ 0 | Total distance |
| avg_power | Option<u16> | - | Average power in watts |
| max_power | Option<u16> | - | Maximum power in watts |
| normalized_power | Option<u16> | - | Normalized Power (NP) |
| intensity_factor | Option<f32> | - | IF = NP / FTP |
| tss | Option<f32> | - | Training Stress Score |
| avg_hr | Option<u8> | - | Average heart rate |
| max_hr | Option<u8> | - | Maximum heart rate |
| avg_cadence | Option<u8> | - | Average cadence |
| calories | u32 | ≥ 0 | Estimated calories burned |
| ftp_at_ride | u16 | - | User's FTP at time of ride |
| notes | Option<String> | Max 2000 chars | User notes |
| created_at | DateTime | Required | Record creation timestamp |

**Calculated Fields:**
- `normalized_power`: 30-second rolling average, 4th power average
- `intensity_factor`: NP / FTP
- `tss`: (duration_seconds / 3600) × (NP / FTP)² × 100
- `calories`: Based on power and duration (kJ ≈ kcal for cycling)

**State Transitions:**
```
[Not Started] → [In Progress] → [Paused] → [In Progress] → [Completed]
                      ↓                           ↓
                 [Discarded]                [Discarded]
```

---

### RideSample

Represents a single data point (1-second resolution) during a ride.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | i64 | Primary key, auto-increment | Unique identifier |
| ride_id | UUID | Foreign key → Ride | Parent ride |
| elapsed_seconds | u32 | ≥ 0 | Seconds since ride start |
| power_watts | Option<u16> | 0-2000 | Instantaneous power |
| cadence_rpm | Option<u8> | 0-200 | Cadence |
| heart_rate_bpm | Option<u8> | 30-220 | Heart rate |
| speed_kmh | Option<f32> | 0-100 | Speed in km/h |
| distance_meters | f64 | Cumulative | Distance from start |
| calories | u32 | Cumulative | Calories from start |
| resistance_level | Option<u8> | 0-100 | Trainer resistance % |
| target_power | Option<u16> | - | ERG target power |
| trainer_grade | Option<f32> | -20 to 20 | Simulated gradient % |

**Validation Rules:**
- power_watts filtered: values > 2000 treated as noise
- elapsed_seconds must be monotonically increasing
- distance_meters must be monotonically increasing

**Performance Considerations:**
- ~3,600 samples per hour of riding
- Index on ride_id for efficient retrieval
- Consider compression for long-term storage

---

## Enums

### Units
| Value | Description |
|-------|-------------|
| Metric | km/h, kg, km |
| Imperial | mph, lbs, miles |

### Theme
| Value | Description |
|-------|-------------|
| Dark | Dark mode (default) |
| Light | Light mode |

### ConnectionState
| Value | Description |
|-------|-------------|
| Disconnected | Not connected |
| Connecting | Connection in progress |
| Connected | Active connection |
| Reconnecting | Auto-reconnect in progress |

---

## Database Schema (SQLite)

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ftp INTEGER NOT NULL DEFAULT 200,
    max_hr INTEGER,
    resting_hr INTEGER,
    weight_kg REAL NOT NULL,
    height_cm INTEGER,
    power_zones_json TEXT NOT NULL,
    hr_zones_json TEXT,
    units TEXT NOT NULL DEFAULT 'metric',
    theme TEXT NOT NULL DEFAULT 'dark',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sensors table
CREATE TABLE sensors (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sensor_type TEXT NOT NULL,
    protocol TEXT NOT NULL,
    last_seen_at TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, device_id)
);

-- Workouts table
CREATE TABLE workouts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    author TEXT,
    source_file TEXT,
    source_format TEXT,
    segments_json TEXT NOT NULL,
    total_duration_seconds INTEGER NOT NULL,
    estimated_tss REAL,
    estimated_if REAL,
    tags_json TEXT,
    created_at TEXT NOT NULL
);

-- Rides table
CREATE TABLE rides (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    workout_id TEXT REFERENCES workouts(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL,
    distance_meters REAL NOT NULL,
    avg_power INTEGER,
    max_power INTEGER,
    normalized_power INTEGER,
    intensity_factor REAL,
    tss REAL,
    avg_hr INTEGER,
    max_hr INTEGER,
    avg_cadence INTEGER,
    calories INTEGER NOT NULL,
    ftp_at_ride INTEGER NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_rides_user_id ON rides(user_id);
CREATE INDEX idx_rides_started_at ON rides(started_at);

-- Ride samples table
CREATE TABLE ride_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    elapsed_seconds INTEGER NOT NULL,
    power_watts INTEGER,
    cadence_rpm INTEGER,
    heart_rate_bpm INTEGER,
    speed_kmh REAL,
    distance_meters REAL NOT NULL,
    calories INTEGER NOT NULL,
    resistance_level INTEGER,
    target_power INTEGER,
    trainer_grade REAL
);

CREATE INDEX idx_ride_samples_ride_id ON ride_samples(ride_id);

-- Auto-save table (crash recovery)
CREATE TABLE autosave (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ride_json TEXT NOT NULL,
    samples_json TEXT NOT NULL,
    saved_at TEXT NOT NULL
);
```

---

## Real-Time Data Structures

### SensorReading

Live data from sensors (not persisted directly).

| Field | Type | Description |
|-------|------|-------------|
| timestamp | Instant | Reading timestamp |
| sensor_id | UUID | Source sensor |
| power_watts | Option<u16> | Power reading |
| cadence_rpm | Option<u8> | Cadence reading |
| heart_rate_bpm | Option<u8> | Heart rate reading |
| speed_kmh | Option<f32> | Speed reading |
| distance_delta_m | Option<f32> | Distance increment |

### AggregatedMetrics

Combined metrics from all sensors for display.

| Field | Type | Description |
|-------|------|-------------|
| timestamp | Instant | Aggregation timestamp |
| power_instant | Option<u16> | Current power |
| power_3s_avg | Option<u16> | 3-second rolling average |
| power_30s_avg | Option<u16> | 30-second rolling average (for NP) |
| cadence | Option<u8> | Current cadence |
| heart_rate | Option<u8> | Current heart rate |
| speed | Option<f32> | Current speed |
| distance | f64 | Total distance |
| elapsed_time | Duration | Total ride time |
| calories | u32 | Total calories |
| power_zone | Option<u8> | Current power zone (1-7) |
| hr_zone | Option<u8> | Current HR zone (1-5) |
| normalized_power | Option<u16> | Running NP calculation |
| tss | Option<f32> | Running TSS calculation |
