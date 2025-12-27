# Data Model: Hardware Integration

**Feature Branch**: `007-hardware-integration`
**Date**: 2025-12-26

This document defines the data entities for the Hardware Integration feature, extending the existing RustRide data model.

---

## Entity Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SENSOR DOMAIN                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  Sensor (extended)                                                           │
│    ├── AntDongle                                                            │
│    ├── AntChannel                                                           │
│    ├── DualProtocolBinding                                                  │
│    ├── CyclingDynamicsData                                                  │
│    ├── SmO2Reading                                                          │
│    ├── MotionSample                                                         │
│    └── SensorFusionConfig                                                   │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              TRAINER DOMAIN                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  InclineConfig                                                               │
│    └── GradientState                                                        │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              AUDIO DOMAIN                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  AudioConfig                                                                 │
│    ├── AlertType                                                            │
│    └── CueTemplate                                                          │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           INTEGRATION DOMAIN                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  MqttConfig                                                                  │
│    └── FanProfile                                                           │
│  StreamingConfig                                                             │
│    └── StreamingSession                                                     │
│  WeatherConfig                                                               │
│    └── WeatherData                                                          │
│  PlatformSync                                                                │
│    └── SyncRecord                                                           │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              HID DOMAIN                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  HidDevice                                                                   │
│    └── ButtonMapping                                                        │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              VIDEO DOMAIN                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  VideoSync                                                                   │
│    └── SyncPoint                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Sensor Domain

### Sensor (Extended)

Extends the existing `Sensor` entity to support additional protocols.

```rust
pub enum SensorProtocol {
    Ble,
    AntPlus,
}

pub struct Sensor {
    pub id: Uuid,
    pub name: String,
    pub sensor_type: SensorType,
    pub protocol: SensorProtocol,         // NEW
    pub device_id: String,                 // BLE address or ANT+ device number
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,     // NEW - for duplicate detection
    pub connection_status: ConnectionStatus,
    pub signal_strength: Option<i8>,
    pub last_seen: DateTime<Utc>,
    pub preferred_protocol: Option<SensorProtocol>, // NEW - user preference for dual-protocol
}

pub enum SensorType {
    PowerMeter,
    HeartRate,
    Cadence,
    Speed,
    SmartTrainer,
    SmO2,           // NEW
    Imu,            // NEW
}
```

### AntDongle

Represents a connected ANT+ USB dongle.

```rust
pub struct AntDongle {
    pub id: Uuid,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub max_channels: u8,                  // Typically 8
    pub firmware_version: Option<String>,
    pub status: DongleStatus,
}

pub enum DongleStatus {
    Connected,
    Disconnected,
    Initializing,
    Error(String),
}
```

### AntChannel

Represents an ANT+ communication channel.

```rust
pub struct AntChannel {
    pub channel_number: u8,
    pub channel_type: AntChannelType,
    pub device_type: u8,                   // ANT+ device type code
    pub device_number: u16,                // Device ID
    pub transmission_type: u8,
    pub status: ChannelStatus,
    pub sensor_id: Option<Uuid>,           // Link to Sensor entity
}

pub enum AntChannelType {
    BidirectionalSlave,                    // For receiving data
    BidirectionalMaster,                   // For sending control
}

pub enum ChannelStatus {
    Unassigned,
    Assigned,
    Searching,
    Tracking,
    Closed,
}
```

### DualProtocolBinding

Tracks sensors that broadcast on both BLE and ANT+.

```rust
pub struct DualProtocolBinding {
    pub id: Uuid,
    pub ble_sensor_id: Uuid,
    pub ant_sensor_id: Uuid,
    pub matched_by: MatchMethod,
    pub preferred_protocol: SensorProtocol,
    pub created_at: DateTime<Utc>,
}

pub enum MatchMethod {
    SerialNumber,
    ManufacturerId,
    UserConfirmed,
}
```

### CyclingDynamicsData

Extended power meter data for L/R balance and pedaling metrics.

```rust
pub struct CyclingDynamicsData {
    pub timestamp: DateTime<Utc>,
    pub sensor_id: Uuid,
    pub left_right_balance: Option<LeftRightBalance>,
    pub pedal_smoothness: Option<PedalSmoothness>,
    pub torque_effectiveness: Option<TorqueEffectiveness>,
    pub power_phase: Option<PowerPhase>,
}

pub struct LeftRightBalance {
    pub left_percent: f32,                 // 0-100
    pub right_percent: f32,                // 0-100
}

pub struct PedalSmoothness {
    pub left: f32,                         // 0-100%
    pub right: f32,                        // 0-100%
    pub combined: f32,
}

pub struct TorqueEffectiveness {
    pub left: f32,                         // 0-100%
    pub right: f32,                        // 0-100%
}

pub struct PowerPhase {
    pub left_start: f32,                   // Degrees (0-360)
    pub left_end: f32,
    pub left_peak: f32,
    pub right_start: f32,
    pub right_end: f32,
    pub right_peak: f32,
}
```

### SmO2Reading

Muscle oxygen saturation data.

```rust
pub struct SmO2Reading {
    pub timestamp: DateTime<Utc>,
    pub sensor_id: Uuid,
    pub smo2_percent: f32,                 // 0-100%
    pub thb: Option<f32>,                  // Total hemoglobin (g/dL)
    pub sensor_location: Option<MuscleLocation>,
}

pub enum MuscleLocation {
    LeftVastusLateralis,
    RightVastusLateralis,
    LeftGastrocnemius,
    RightGastrocnemius,
    Other(String),
}
```

### MotionSample

IMU/motion sensor data.

```rust
pub struct MotionSample {
    pub timestamp: DateTime<Utc>,
    pub sensor_id: Uuid,
    pub acceleration: Vector3,             // m/s² (x, y, z)
    pub gyroscope: Option<Vector3>,        // rad/s (x, y, z)
    pub orientation: Option<Quaternion>,
    pub tilt_pitch: Option<f32>,           // Degrees
    pub tilt_roll: Option<f32>,            // Degrees
}

pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
```

### SensorFusionConfig

Configuration for multi-sensor data fusion.

```rust
pub struct SensorFusionConfig {
    pub id: Uuid,
    pub metric_type: MetricType,
    pub primary_sensor_id: Uuid,
    pub secondary_sensor_id: Option<Uuid>,
    pub fusion_algorithm: FusionAlgorithm,
    pub alpha: f32,                        // Weighting factor (0-1)
    pub dropout_threshold_ms: u32,         // Fallback trigger
}

pub enum MetricType {
    Cadence,
    Power,
    HeartRate,
}

pub enum FusionAlgorithm {
    ComplementaryFilter,
    WeightedAverage,
    PrimaryWithFallback,
}
```

---

## Trainer Domain

### InclineConfig

User configuration for gradient simulation.

```rust
pub struct InclineConfig {
    pub enabled: bool,
    pub intensity_percent: u8,             // 50-150%, default 100
    pub max_gradient: f32,                 // Cap gradient at this value
    pub smoothing_enabled: bool,
    pub smoothing_window_seconds: f32,     // Gradient transition smoothing
    pub rider_weight_kg: f32,
    pub bike_weight_kg: f32,
}
```

### GradientState

Runtime state for gradient control.

```rust
pub struct GradientState {
    pub current_gradient: f32,             // Actual gradient being sent
    pub target_gradient: f32,              // Requested gradient from route
    pub effective_gradient: f32,           // After intensity scaling
    pub last_command_sent: DateTime<Utc>,
}
```

---

## Audio Domain

### AudioConfig

User preferences for audio cues.

```rust
pub struct AudioConfig {
    pub enabled: bool,
    pub volume: u8,                        // 0-100
    pub voice: Option<String>,             // System voice name
    pub alerts: Vec<AlertConfig>,
}

pub struct AlertConfig {
    pub alert_type: AlertType,
    pub enabled: bool,
    pub custom_message: Option<String>,
}

pub enum AlertType {
    IntervalStart,
    IntervalEnd,
    ZoneChange,
    HalfwayPoint,
    FinalMinute,
    WorkoutComplete,
    LapMarker,
    PowerTarget,
    CadenceTarget,
    HeartRateZone,
}
```

### CueTemplate

Pre-defined or custom audio cue templates.

```rust
pub struct CueTemplate {
    pub id: Uuid,
    pub name: String,
    pub message_template: String,          // "Starting {interval_name} at {target_power}W"
    pub is_builtin: bool,
}
```

---

## Integration Domain

### MqttConfig

MQTT broker connection settings.

```rust
pub struct MqttConfig {
    pub enabled: bool,
    pub broker_host: String,
    pub broker_port: u16,                  // Default 1883, 8883 for TLS
    pub use_tls: bool,
    pub username: Option<String>,
    pub password: Option<String>,          // Stored securely
    pub client_id: String,
    pub fan_profiles: Vec<FanProfile>,
}
```

### FanProfile

Fan speed control mapping.

```rust
pub struct FanProfile {
    pub id: Uuid,
    pub name: String,
    pub topic: String,                     // e.g., "home/fan/living_room/set"
    pub control_metric: ControlMetric,
    pub zone_mappings: Vec<ZoneMapping>,
    pub off_on_stop: bool,                 // Turn off when ride ends
}

pub enum ControlMetric {
    Power,
    HeartRate,
}

pub struct ZoneMapping {
    pub zone: u8,                          // 1-7
    pub fan_speed: u8,                     // 0-100 or 0-3 for discrete levels
}
```

### StreamingConfig

External display streaming settings.

```rust
pub struct StreamingConfig {
    pub enabled: bool,
    pub port: u16,                         // Default 8080
    pub require_pin: bool,                 // Default true
    pub current_pin: Option<String>,       // 6-digit PIN
    pub pin_expiry_minutes: u32,           // PIN validity period
    pub update_interval_ms: u32,           // Default 1000
    pub metrics_to_stream: Vec<StreamMetric>,
}

pub enum StreamMetric {
    Power,
    HeartRate,
    Cadence,
    Speed,
    Distance,
    ElapsedTime,
    CurrentInterval,
    ZoneName,
    Gradient,
}
```

### StreamingSession

Active streaming client session.

```rust
pub struct StreamingSession {
    pub id: Uuid,
    pub client_ip: String,
    pub authenticated: bool,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub user_agent: Option<String>,
}
```

### WeatherConfig

Weather display settings.

```rust
pub struct WeatherConfig {
    pub enabled: bool,
    pub api_key: Option<String>,           // User's OpenWeatherMap API key
    pub latitude: f64,
    pub longitude: f64,
    pub units: WeatherUnits,
    pub refresh_interval_minutes: u32,     // Default 30
}

pub enum WeatherUnits {
    Metric,
    Imperial,
}
```

### WeatherData

Cached weather information.

```rust
pub struct WeatherData {
    pub temperature: f32,
    pub feels_like: f32,
    pub humidity: u8,                      // 0-100%
    pub conditions: String,                // "Sunny", "Cloudy", etc.
    pub icon_code: String,                 // For display
    pub wind_speed: f32,
    pub wind_direction: u16,               // Degrees
    pub fetched_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}
```

### PlatformSync

Fitness platform sync configuration.

```rust
pub struct PlatformSync {
    pub id: Uuid,
    pub platform: SyncPlatform,
    pub enabled: bool,
    pub auto_sync: bool,                   // Sync automatically after ride
    pub last_sync: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
}

pub enum SyncPlatform {
    GarminConnect,
    Strava,
    AppleHealth,
    GoogleFit,
}

pub enum SyncStatus {
    NotConfigured,
    Authorized,
    TokenExpired,
    Error(String),
}
```

### SyncRecord

Record of sync attempts.

```rust
pub struct SyncRecord {
    pub id: Uuid,
    pub ride_id: Uuid,
    pub platform: SyncPlatform,
    pub status: SyncRecordStatus,
    pub external_id: Option<String>,       // ID on external platform
    pub attempted_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

pub enum SyncRecordStatus {
    Pending,
    Uploading,
    Success,
    Failed,
    Retrying,
}
```

---

## HID Domain

### HidDevice

Detected USB HID device.

```rust
pub struct HidDevice {
    pub id: Uuid,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub is_enabled: bool,
}
```

### ButtonMapping

Mapping between HID button and application action.

```rust
pub struct ButtonMapping {
    pub id: Uuid,
    pub device_id: Uuid,
    pub button_code: u8,                   // HID usage code
    pub action: ButtonAction,
    pub label: Option<String>,             // User-defined label
}

pub enum ButtonAction {
    AddLapMarker,
    PauseResume,
    SkipInterval,
    EndRide,
    VolumeUp,
    VolumeDown,
    MuteAudio,
    ToggleFan,
    Custom(String),                        // For extensibility
}
```

---

## Video Domain

### VideoSync

Video file linked to a route.

```rust
pub struct VideoSync {
    pub id: Uuid,
    pub route_id: Uuid,
    pub video_path: PathBuf,
    pub duration_seconds: f32,
    pub resolution: VideoResolution,
    pub sync_points: Vec<SyncPoint>,
    pub playback_speed_range: (f32, f32),  // e.g., (0.5, 2.0)
}

pub struct VideoResolution {
    pub width: u32,
    pub height: u32,
}
```

### SyncPoint

Manual sync point for video-to-route alignment.

```rust
pub struct SyncPoint {
    pub video_time_seconds: f32,
    pub route_distance_meters: f32,
}
```

---

## Database Schema Extensions

### New Tables

```sql
-- ANT+ Dongles
CREATE TABLE ant_dongles (
    id TEXT PRIMARY KEY,
    vendor_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    serial_number TEXT,
    max_channels INTEGER DEFAULT 8,
    firmware_version TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Dual Protocol Bindings
CREATE TABLE dual_protocol_bindings (
    id TEXT PRIMARY KEY,
    ble_sensor_id TEXT NOT NULL REFERENCES sensors(id),
    ant_sensor_id TEXT NOT NULL REFERENCES sensors(id),
    matched_by TEXT NOT NULL,
    preferred_protocol TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Fan Profiles
CREATE TABLE fan_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    topic TEXT NOT NULL,
    control_metric TEXT NOT NULL,
    zone_mappings TEXT NOT NULL,  -- JSON array
    off_on_stop INTEGER DEFAULT 1,
    created_at TEXT NOT NULL
);

-- HID Devices
CREATE TABLE hid_devices (
    id TEXT PRIMARY KEY,
    vendor_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    manufacturer TEXT,
    product_name TEXT,
    serial_number TEXT,
    is_enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL
);

-- Button Mappings
CREATE TABLE button_mappings (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES hid_devices(id),
    button_code INTEGER NOT NULL,
    action TEXT NOT NULL,
    label TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(device_id, button_code)
);

-- Streaming Sessions
CREATE TABLE streaming_sessions (
    id TEXT PRIMARY KEY,
    client_ip TEXT NOT NULL,
    authenticated INTEGER DEFAULT 0,
    connected_at TEXT NOT NULL,
    last_activity TEXT NOT NULL,
    user_agent TEXT
);

-- Platform Syncs
CREATE TABLE platform_syncs (
    id TEXT PRIMARY KEY,
    platform TEXT NOT NULL UNIQUE,
    enabled INTEGER DEFAULT 0,
    auto_sync INTEGER DEFAULT 0,
    last_sync TEXT,
    sync_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sync Records
CREATE TABLE sync_records (
    id TEXT PRIMARY KEY,
    ride_id TEXT NOT NULL REFERENCES rides(id),
    platform TEXT NOT NULL,
    status TEXT NOT NULL,
    external_id TEXT,
    attempted_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);

-- Video Syncs
CREATE TABLE video_syncs (
    id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL REFERENCES routes(id),
    video_path TEXT NOT NULL,
    duration_seconds REAL NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    sync_points TEXT NOT NULL,  -- JSON array
    speed_min REAL DEFAULT 0.5,
    speed_max REAL DEFAULT 2.0,
    created_at TEXT NOT NULL
);

-- Extend existing sensors table
ALTER TABLE sensors ADD COLUMN protocol TEXT DEFAULT 'ble';
ALTER TABLE sensors ADD COLUMN serial_number TEXT;
ALTER TABLE sensors ADD COLUMN preferred_protocol TEXT;
```

### Extended Ride Samples (for cycling dynamics, SmO2, motion)

```sql
-- Cycling Dynamics Samples (linked to ride samples)
CREATE TABLE cycling_dynamics_samples (
    id TEXT PRIMARY KEY,
    ride_sample_id TEXT NOT NULL REFERENCES ride_samples(id),
    left_balance REAL,
    right_balance REAL,
    left_smoothness REAL,
    right_smoothness REAL,
    left_torque_eff REAL,
    right_torque_eff REAL
);

-- SmO2 Samples
CREATE TABLE smo2_samples (
    id TEXT PRIMARY KEY,
    ride_sample_id TEXT NOT NULL REFERENCES ride_samples(id),
    sensor_id TEXT NOT NULL REFERENCES sensors(id),
    smo2_percent REAL NOT NULL,
    thb REAL,
    muscle_location TEXT
);

-- Motion Samples
CREATE TABLE motion_samples (
    id TEXT PRIMARY KEY,
    ride_sample_id TEXT NOT NULL REFERENCES ride_samples(id),
    sensor_id TEXT NOT NULL REFERENCES sensors(id),
    accel_x REAL,
    accel_y REAL,
    accel_z REAL,
    gyro_x REAL,
    gyro_y REAL,
    gyro_z REAL,
    tilt_pitch REAL,
    tilt_roll REAL
);
```

---

## Validation Rules

| Entity | Rule | Constraint |
|--------|------|------------|
| InclineConfig | intensity_percent | 50 ≤ value ≤ 150 |
| InclineConfig | max_gradient | -30 ≤ value ≤ 30 |
| AudioConfig | volume | 0 ≤ value ≤ 100 |
| FanProfile | zone_mappings.fan_speed | 0 ≤ value ≤ 100 |
| StreamingConfig | port | 1024 ≤ value ≤ 65535 |
| StreamingConfig | current_pin | 6-digit numeric string |
| WeatherConfig | latitude | -90 ≤ value ≤ 90 |
| WeatherConfig | longitude | -180 ≤ value ≤ 180 |
| LeftRightBalance | left_percent + right_percent | = 100 |
| SmO2Reading | smo2_percent | 0 ≤ value ≤ 100 |

---

## State Transitions

### SyncStatus
```
NotConfigured → Authorized (after OAuth)
Authorized → TokenExpired (after token expires)
TokenExpired → Authorized (after refresh)
Authorized → Error (on API failure)
Error → Authorized (after retry)
```

### ChannelStatus
```
Unassigned → Assigned (channel allocated)
Assigned → Searching (looking for device)
Searching → Tracking (device found)
Tracking → Searching (signal lost)
Tracking → Closed (user disconnect)
Searching → Closed (timeout)
```

### DongleStatus
```
Disconnected → Initializing (USB connected)
Initializing → Connected (ready)
Initializing → Error (driver issue)
Connected → Disconnected (USB removed)
Error → Disconnected (reset)
```
