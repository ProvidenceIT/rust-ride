# Data Model: 3D Virtual World & Complete Feature Implementation

**Feature**: 002-3d-world-features
**Date**: 2025-12-24

## Entity Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   UserProfile   │────▶│      Ride       │────▶│   RideSample    │
│                 │     │                 │     │                 │
│ - ftp           │     │ - started_at    │     │ - timestamp     │
│ - max_hr        │     │ - duration      │     │ - power         │
│ - weight        │     │ - samples[]     │     │ - hr            │
│ - theme         │     │ - world_id      │     │ - cadence       │
│ - units         │     │ - route_id      │     │ - speed         │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │
        │
        ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│     Avatar      │     │  VirtualWorld   │────▶│      Route      │
│                 │     │                 │     │                 │
│ - jersey_color  │     │ - name          │     │ - name          │
│ - bike_style    │     │ - theme         │     │ - distance      │
│ - body_type     │     │ - routes[]      │     │ - elevation     │
└─────────────────┘     │ - assets_path   │     │ - waypoints[]   │
                        └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐
│    Workout      │────▶│ WorkoutSegment  │
│                 │     │                 │
│ - name          │     │ - type          │
│ - duration      │     │ - duration      │
│ - segments[]    │     │ - power_target  │
│ - source_file   │     │ - cadence       │
└─────────────────┘     └─────────────────┘
```

---

## Core Entities

### UserProfile

Represents user configuration and personal metrics.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | UUID | Yes | Unique identifier | Auto-generated |
| name | String | Yes | Display name | 1-100 chars |
| ftp | u16 | Yes | Functional Threshold Power (watts) | 50-500 |
| max_hr | u8 | Yes | Maximum heart rate (bpm) | 100-220 |
| weight_kg | f32 | Yes | Body weight in kilograms | 30.0-200.0 |
| height_cm | u16 | No | Height in centimeters | 100-250 |
| birth_year | u16 | No | Year of birth | 1920-2020 |
| theme | Theme | Yes | UI theme preference | Light \| Dark |
| units | UnitSystem | Yes | Measurement units | Metric \| Imperial |
| created_at | DateTime | Yes | Profile creation time | - |
| updated_at | DateTime | Yes | Last modification time | - |

**State Transitions**: None (static configuration)

---

### Avatar

User's virtual representation in the 3D world.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | UUID | Yes | Unique identifier | Auto-generated |
| user_id | UUID | Yes | Foreign key to UserProfile | Must exist |
| jersey_color | Color | Yes | Primary jersey color | RGB value |
| jersey_secondary | Color | No | Secondary jersey color | RGB value |
| bike_style | BikeStyle | Yes | Bike model selection | Road \| TT \| Gravel |
| helmet_color | Color | No | Helmet color | RGB value |
| updated_at | DateTime | Yes | Last modification time | - |

**Enums**:
```rust
enum BikeStyle {
    RoadBike,
    TTBike,
    GravelBike,
}
```

---

### Ride

A completed or in-progress training session.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | UUID | Yes | Unique identifier | Auto-generated |
| user_id | UUID | Yes | Foreign key to UserProfile | Must exist |
| started_at | DateTime | Yes | Ride start timestamp | - |
| ended_at | DateTime | No | Ride end timestamp (null if in progress) | > started_at |
| duration_seconds | u32 | Yes | Total ride duration | - |
| distance_meters | f64 | Yes | Total distance traveled | >= 0 |
| avg_power | u16 | No | Average power (watts) | - |
| max_power | u16 | No | Maximum power (watts) | - |
| normalized_power | u16 | No | Normalized Power (watts) | - |
| avg_hr | u8 | No | Average heart rate (bpm) | - |
| max_hr | u8 | No | Maximum heart rate (bpm) | - |
| avg_cadence | u8 | No | Average cadence (rpm) | - |
| avg_speed_kmh | f32 | No | Average speed (km/h) | - |
| max_speed_kmh | f32 | No | Maximum speed (km/h) | - |
| tss | f32 | No | Training Stress Score | - |
| intensity_factor | f32 | No | Intensity Factor (NP/FTP) | - |
| calories | u32 | No | Estimated calories burned | - |
| world_id | String | No | Virtual world used (if 3D mode) | - |
| route_id | String | No | Route ridden (if 3D mode) | - |
| workout_id | UUID | No | Associated workout (if structured) | - |
| notes | String | No | User notes about the ride | Max 2000 chars |
| created_at | DateTime | Yes | Record creation time | - |

**State Transitions**:
```
[Not Started] → [Recording] → [Completed]
                     ↓
               [Paused] → [Recording]
```

---

### RideSample

Individual data point captured during a ride (1-second intervals).

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | i64 | Yes | Auto-increment ID | - |
| ride_id | UUID | Yes | Foreign key to Ride | Must exist |
| timestamp_offset | u32 | Yes | Seconds from ride start | - |
| power_watts | u16 | No | Instantaneous power | 0-2500 |
| heart_rate_bpm | u8 | No | Instantaneous heart rate | 30-250 |
| cadence_rpm | u8 | No | Instantaneous cadence | 0-200 |
| speed_kmh | f32 | No | Instantaneous speed | 0-120 |
| distance_delta_m | f32 | No | Distance since last sample | >= 0 |
| position_x | f32 | No | 3D world X position | - |
| position_y | f32 | No | 3D world Y position | - |
| position_z | f32 | No | 3D world Z position (elevation) | - |

**Storage**: Stored in database as individual rows. Loaded in batches for export.

---

### Autosave

Temporary storage for crash recovery.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | i32 | Yes | Primary key (always 1) | - |
| ride_id | UUID | Yes | ID for the recovered ride | - |
| started_at | DateTime | Yes | When ride started | - |
| last_sample_at | DateTime | Yes | Last recorded sample time | - |
| samples_json | String | Yes | JSON array of RideSample | Valid JSON |
| workout_id | UUID | No | Workout in progress | - |
| workout_elapsed | u32 | No | Seconds into workout | - |
| world_id | String | No | Virtual world | - |
| route_id | String | No | Route | - |
| position_on_route | f32 | No | Distance along route (meters) | - |
| updated_at | DateTime | Yes | Last autosave time | - |

**Lifecycle**: Single row, replaced every 30 seconds during ride, deleted on clean exit.

---

### Workout

Structured training plan imported from file.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | UUID | Yes | Unique identifier | Auto-generated |
| name | String | Yes | Workout name | 1-200 chars |
| description | String | No | Workout description | Max 2000 chars |
| author | String | No | Workout author | - |
| duration_seconds | u32 | Yes | Total workout duration | > 0 |
| tss_estimate | f32 | No | Estimated TSS | - |
| intensity_factor | f32 | No | Estimated IF | - |
| source_format | WorkoutFormat | Yes | Original file format | ZWO \| MRC |
| source_path | String | No | Original file path | - |
| tags | Vec<String> | No | Categorization tags | - |
| created_at | DateTime | Yes | Import timestamp | - |
| last_used_at | DateTime | No | Last execution time | - |

**Enums**:
```rust
enum WorkoutFormat {
    ZWO,    // Zwift workout
    MRC,    // ErgDB/TrainerRoad format
}
```

---

### WorkoutSegment

Individual interval within a workout.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | i64 | Yes | Auto-increment ID | - |
| workout_id | UUID | Yes | Foreign key to Workout | Must exist |
| sequence | u16 | Yes | Order within workout | >= 0 |
| segment_type | SegmentType | Yes | Type of interval | - |
| duration_seconds | u32 | Yes | Segment duration | > 0 |
| power_start | f32 | Yes | Starting power (% FTP) | 0.0-2.0 |
| power_end | f32 | Yes | Ending power (% FTP) | 0.0-2.0 |
| cadence_target | u8 | No | Target cadence (rpm) | 40-150 |
| cadence_low | u8 | No | Min cadence (rpm) | - |
| cadence_high | u8 | No | Max cadence (rpm) | - |
| text | String | No | On-screen text/instruction | Max 500 chars |

**Enums**:
```rust
enum SegmentType {
    Warmup,
    Cooldown,
    SteadyState,
    Intervals,
    Ramp,
    FreeRide,
}
```

---

## 3D World Entities

### VirtualWorld

Definition of a 3D environment.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | String | Yes | Unique identifier | Lowercase alphanumeric |
| name | String | Yes | Display name | 1-100 chars |
| description | String | No | World description | Max 500 chars |
| theme | WorldTheme | Yes | Visual theme category | - |
| preview_image | String | Yes | Path to preview thumbnail | Valid path |
| assets_path | String | Yes | Path to world assets | Valid directory |
| default_route | String | Yes | Default route ID | Must exist in routes |
| routes | Vec<Route> | Yes | Available routes | At least 1 |
| ambient_lighting | AmbientConfig | Yes | Lighting configuration | - |
| time_of_day | TimeOfDay | Yes | Default time setting | - |

**Enums**:
```rust
enum WorldTheme {
    Countryside,    // Rolling hills, farms, forests
    Mountains,      // Alpine, switchbacks, snow peaks
    Coastal,        // Ocean views, beaches, palm trees
    Urban,          // City streets, buildings
    Desert,         // Arid landscape, canyons
}

enum TimeOfDay {
    Dawn,
    Morning,
    Noon,
    Afternoon,
    Sunset,
    Night,
}
```

---

### Route

A path through a virtual world.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | String | Yes | Unique identifier | Lowercase alphanumeric |
| world_id | String | Yes | Parent world | Must exist |
| name | String | Yes | Route name | 1-100 chars |
| distance_meters | f32 | Yes | Total route length | > 0 |
| elevation_gain_meters | f32 | Yes | Total climbing | >= 0 |
| elevation_loss_meters | f32 | Yes | Total descending | >= 0 |
| min_elevation_meters | f32 | Yes | Lowest point | - |
| max_elevation_meters | f32 | Yes | Highest point | >= min |
| difficulty | Difficulty | Yes | Route difficulty rating | - |
| is_loop | bool | Yes | Whether route forms a loop | - |
| waypoints | Vec<Waypoint> | Yes | Path definition | At least 2 |
| preview_image | String | No | Route preview thumbnail | - |

**Enums**:
```rust
enum Difficulty {
    Easy,       // Flat or gentle gradients
    Moderate,   // Some climbing
    Hard,       // Significant climbing
    Extreme,    // Mountain stages
}
```

---

### Waypoint

Point along a route defining the path.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| position | Vec3 | Yes | 3D coordinates (x, y, z) | - |
| distance_from_start | f32 | Yes | Meters from route start | >= 0 |
| gradient_percent | f32 | Yes | Grade at this point | -50 to +50 |
| surface_type | SurfaceType | No | Road surface | - |
| scenery_trigger | String | No | Scenery event ID | - |

**Enums**:
```rust
enum SurfaceType {
    Asphalt,
    Concrete,
    Cobblestone,
    Gravel,
    Dirt,
}
```

---

## Sensor Entities (Existing, Extended)

### SensorState

Runtime state of a connected sensor.

| Field | Type | Required | Description | Validation |
|-------|------|----------|-------------|------------|
| id | UUID | Yes | Runtime identifier | - |
| device_id | String | Yes | BLE device ID | - |
| name | String | Yes | Display name | - |
| sensor_type | SensorType | Yes | Type classification | - |
| protocol | Protocol | Yes | Communication protocol | - |
| connection_state | ConnectionState | Yes | Current state | - |
| signal_strength | i16 | No | RSSI value | - |
| battery_level | u8 | No | Battery percentage | 0-100 |
| last_data_at | Instant | No | Last data received | - |
| is_primary | bool | Yes | Primary data source | - |

**State Transitions**:
```
[Disconnected] ⟷ [Connecting] ⟷ [Connected]
                      ↓
               [Reconnecting]
```

---

## Database Schema

### Tables

```sql
-- User Profile
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ftp INTEGER NOT NULL DEFAULT 200,
    max_hr INTEGER NOT NULL DEFAULT 180,
    weight_kg REAL NOT NULL DEFAULT 70.0,
    height_cm INTEGER,
    birth_year INTEGER,
    theme TEXT NOT NULL DEFAULT 'dark',
    units TEXT NOT NULL DEFAULT 'metric',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Avatar Customization
CREATE TABLE avatars (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    jersey_color TEXT NOT NULL DEFAULT '#FF0000',
    jersey_secondary TEXT,
    bike_style TEXT NOT NULL DEFAULT 'road_bike',
    helmet_color TEXT,
    updated_at TEXT NOT NULL
);

-- Rides
CREATE TABLE rides (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    distance_meters REAL NOT NULL DEFAULT 0,
    avg_power INTEGER,
    max_power INTEGER,
    normalized_power INTEGER,
    avg_hr INTEGER,
    max_hr INTEGER,
    avg_cadence INTEGER,
    avg_speed_kmh REAL,
    max_speed_kmh REAL,
    tss REAL,
    intensity_factor REAL,
    calories INTEGER,
    world_id TEXT,
    route_id TEXT,
    workout_id TEXT REFERENCES workouts(id),
    notes TEXT,
    created_at TEXT NOT NULL
);

-- Ride Samples
CREATE TABLE ride_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ride_id TEXT NOT NULL REFERENCES rides(id) ON DELETE CASCADE,
    timestamp_offset INTEGER NOT NULL,
    power_watts INTEGER,
    heart_rate_bpm INTEGER,
    cadence_rpm INTEGER,
    speed_kmh REAL,
    distance_delta_m REAL,
    position_x REAL,
    position_y REAL,
    position_z REAL
);

CREATE INDEX idx_ride_samples_ride_id ON ride_samples(ride_id);

-- Autosave (single row)
CREATE TABLE autosave (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ride_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_sample_at TEXT NOT NULL,
    samples_json TEXT NOT NULL,
    workout_id TEXT,
    workout_elapsed INTEGER,
    world_id TEXT,
    route_id TEXT,
    position_on_route REAL,
    updated_at TEXT NOT NULL
);

-- Workouts
CREATE TABLE workouts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    author TEXT,
    duration_seconds INTEGER NOT NULL,
    tss_estimate REAL,
    intensity_factor REAL,
    source_format TEXT NOT NULL,
    source_path TEXT,
    tags TEXT,  -- JSON array
    created_at TEXT NOT NULL,
    last_used_at TEXT
);

-- Workout Segments
CREATE TABLE workout_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workout_id TEXT NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    segment_type TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    power_start REAL NOT NULL,
    power_end REAL NOT NULL,
    cadence_target INTEGER,
    cadence_low INTEGER,
    cadence_high INTEGER,
    text TEXT
);

CREATE INDEX idx_workout_segments_workout_id ON workout_segments(workout_id);
```

### File-Based Data (Not in Database)

Virtual worlds and routes are loaded from JSON files in the `assets/worlds/` directory:

```json
// assets/worlds/countryside.json
{
  "id": "countryside",
  "name": "Rolling Countryside",
  "description": "Gentle hills and pastoral scenery",
  "theme": "countryside",
  "preview_image": "countryside_preview.png",
  "assets_path": "assets/worlds/countryside/",
  "default_route": "farm_loop",
  "time_of_day": "morning",
  "routes": [
    {
      "id": "farm_loop",
      "name": "Farm Loop",
      "distance_meters": 12500,
      "elevation_gain_meters": 120,
      "elevation_loss_meters": 120,
      "difficulty": "easy",
      "is_loop": true,
      "waypoints": [...]
    }
  ]
}
```
