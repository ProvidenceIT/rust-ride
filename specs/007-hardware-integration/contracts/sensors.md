# Contract: Sensors Module (Extended)

**Module**: `src/sensors/`
**Feature**: Hardware Integration
**Date**: 2025-12-26

This contract defines the extended sensor module API for ANT+, cycling dynamics, SmO2, IMU, and sensor fusion.

---

## ANT+ Submodule (`src/sensors/ant/`)

### Dongle Manager

```rust
/// Manages ANT+ USB dongle detection and lifecycle
pub trait AntDongleManager: Send + Sync {
    /// Scan for connected ANT+ dongles
    async fn scan_dongles(&self) -> Result<Vec<AntDongle>, SensorError>;

    /// Initialize a specific dongle for use
    async fn initialize_dongle(&self, dongle_id: &Uuid) -> Result<(), SensorError>;

    /// Get current dongle status
    fn get_dongle_status(&self, dongle_id: &Uuid) -> Option<DongleStatus>;

    /// Close and release a dongle
    async fn release_dongle(&self, dongle_id: &Uuid) -> Result<(), SensorError>;

    /// Subscribe to dongle connection events
    fn subscribe_dongle_events(&self) -> broadcast::Receiver<DongleEvent>;
}

pub enum DongleEvent {
    Connected(AntDongle),
    Disconnected(Uuid),
    Error { dongle_id: Uuid, error: String },
}
```

### Channel Manager

```rust
/// Manages ANT+ communication channels
pub trait AntChannelManager: Send + Sync {
    /// Allocate a channel for a device type
    async fn allocate_channel(
        &self,
        dongle_id: &Uuid,
        device_type: AntDeviceType,
    ) -> Result<AntChannel, SensorError>;

    /// Start searching on a channel
    async fn start_search(&self, channel: &AntChannel) -> Result<(), SensorError>;

    /// Close a channel
    async fn close_channel(&self, channel_number: u8) -> Result<(), SensorError>;

    /// Get available channel count
    fn available_channels(&self, dongle_id: &Uuid) -> u8;

    /// Subscribe to channel data
    fn subscribe_channel_data(&self, channel_number: u8) -> broadcast::Receiver<AntMessage>;
}

pub enum AntDeviceType {
    Power,        // ANT+ PWR
    HeartRate,    // ANT+ HRM
    FitnessEquipment, // ANT+ FE-C
    Cadence,
    SpeedCadence,
}

pub struct AntMessage {
    pub channel: u8,
    pub device_number: u16,
    pub payload: Vec<u8>,
    pub timestamp: Instant,
}
```

### Profile Parsers

```rust
/// Parse ANT+ power meter data (PWR profile)
pub trait PowerProfileParser {
    fn parse_power_page(&self, data: &[u8]) -> Result<PowerData, ParseError>;
    fn parse_calibration_page(&self, data: &[u8]) -> Result<CalibrationData, ParseError>;
}

pub struct PowerData {
    pub instant_power: u16,
    pub accumulated_power: u32,
    pub event_count: u8,
    pub cadence: Option<u8>,
    pub left_right_balance: Option<LeftRightBalance>,
}

/// Parse ANT+ heart rate data (HRM profile)
pub trait HrProfileParser {
    fn parse_hr_page(&self, data: &[u8]) -> Result<HrData, ParseError>;
}

pub struct HrData {
    pub heart_rate: u8,
    pub rr_interval: Option<u16>,
    pub event_time: u16,
}

/// Parse ANT+ FE-C trainer data
pub trait FecProfileParser {
    fn parse_general_fe_data(&self, data: &[u8]) -> Result<FeData, ParseError>;
    fn parse_specific_trainer_data(&self, data: &[u8]) -> Result<TrainerData, ParseError>;
}

pub struct FeData {
    pub equipment_type: u8,
    pub elapsed_time: u8,
    pub distance: u8,
    pub speed: u16,
    pub heart_rate: Option<u8>,
}

pub struct TrainerData {
    pub instant_power: u16,
    pub accumulated_power: u32,
    pub cadence: u8,
    pub status: TrainerStatus,
}
```

### Dual Protocol Detection

```rust
/// Detect and manage sensors broadcasting on both BLE and ANT+
pub trait DualProtocolDetector: Send + Sync {
    /// Check if a newly discovered sensor matches an existing one
    async fn check_duplicate(
        &self,
        new_sensor: &Sensor,
    ) -> Option<DualProtocolBinding>;

    /// Create a binding between BLE and ANT+ instances
    async fn create_binding(
        &self,
        ble_sensor_id: Uuid,
        ant_sensor_id: Uuid,
        match_method: MatchMethod,
    ) -> Result<DualProtocolBinding, SensorError>;

    /// Get user's preferred protocol for a bound sensor
    fn get_preferred_protocol(&self, binding_id: &Uuid) -> SensorProtocol;

    /// Set user's preferred protocol
    async fn set_preferred_protocol(
        &self,
        binding_id: &Uuid,
        protocol: SensorProtocol,
    ) -> Result<(), SensorError>;
}
```

---

## Incline Control (`src/sensors/incline.rs`)

```rust
/// Control smart trainer incline/slope mode
pub trait InclineController: Send + Sync {
    /// Set target gradient (will be scaled by user intensity setting)
    async fn set_gradient(&self, gradient_percent: f32) -> Result<(), SensorError>;

    /// Get current gradient state
    fn get_gradient_state(&self) -> GradientState;

    /// Enable/disable incline mode
    async fn set_enabled(&self, enabled: bool) -> Result<(), SensorError>;

    /// Update configuration
    fn update_config(&self, config: InclineConfig);

    /// Calculate effective gradient with user settings
    fn calculate_effective_gradient(&self, raw_gradient: f32, config: &InclineConfig) -> f32;
}

impl InclineController {
    /// FTMS slope command format
    /// Gradient is sent as signed 16-bit: gradient * 100
    /// Range: -20.00% to +20.00% (-2000 to +2000)
    pub fn build_slope_command(gradient_percent: f32) -> [u8; 3] {
        let value = (gradient_percent * 100.0).clamp(-2000.0, 2000.0) as i16;
        [0x11, (value & 0xFF) as u8, ((value >> 8) & 0xFF) as u8]
    }
}
```

---

## Cycling Dynamics (`src/sensors/dynamics.rs`)

```rust
/// Parse and provide cycling dynamics data
pub trait CyclingDynamicsProvider: Send + Sync {
    /// Check if sensor supports cycling dynamics
    fn supports_dynamics(&self, sensor_id: &Uuid) -> bool;

    /// Get latest dynamics data
    fn get_current_dynamics(&self, sensor_id: &Uuid) -> Option<CyclingDynamicsData>;

    /// Subscribe to dynamics updates
    fn subscribe_dynamics(&self, sensor_id: &Uuid) -> broadcast::Receiver<CyclingDynamicsData>;

    /// Get session averages
    fn get_session_averages(&self) -> DynamicsAverages;
}

pub struct DynamicsAverages {
    pub avg_left_balance: f32,
    pub avg_right_balance: f32,
    pub avg_left_smoothness: f32,
    pub avg_right_smoothness: f32,
    pub avg_left_torque_eff: f32,
    pub avg_right_torque_eff: f32,
    pub sample_count: u32,
}
```

---

## SmO2 Sensors (`src/sensors/smo2.rs`)

```rust
/// Muscle oxygen sensor interface
pub trait SmO2Provider: Send + Sync {
    /// Discover SmO2 sensors
    async fn discover_smo2_sensors(&self) -> Result<Vec<Sensor>, SensorError>;

    /// Connect to SmO2 sensor
    async fn connect(&self, sensor_id: &Uuid) -> Result<(), SensorError>;

    /// Get current reading
    fn get_current_reading(&self, sensor_id: &Uuid) -> Option<SmO2Reading>;

    /// Subscribe to readings
    fn subscribe_readings(&self, sensor_id: &Uuid) -> broadcast::Receiver<SmO2Reading>;

    /// Set muscle location label
    async fn set_location(&self, sensor_id: &Uuid, location: MuscleLocation) -> Result<(), SensorError>;
}
```

---

## IMU/Motion Sensors (`src/sensors/imu.rs`)

```rust
/// Motion/IMU sensor interface
pub trait MotionProvider: Send + Sync {
    /// Discover motion sensors
    async fn discover_motion_sensors(&self) -> Result<Vec<Sensor>, SensorError>;

    /// Connect to motion sensor
    async fn connect(&self, sensor_id: &Uuid) -> Result<(), SensorError>;

    /// Get current motion data
    fn get_current_motion(&self, sensor_id: &Uuid) -> Option<MotionSample>;

    /// Subscribe to motion updates
    fn subscribe_motion(&self, sensor_id: &Uuid) -> broadcast::Receiver<MotionSample>;

    /// Calibrate sensor (zero point)
    async fn calibrate(&self, sensor_id: &Uuid) -> Result<(), SensorError>;
}
```

---

## Sensor Fusion (`src/sensors/fusion.rs`)

```rust
/// Multi-sensor data fusion
pub trait SensorFusion: Send + Sync {
    /// Configure fusion for a metric
    fn configure_fusion(&self, config: SensorFusionConfig);

    /// Get fused value for a metric
    fn get_fused_value(&self, metric: MetricType) -> Option<f64>;

    /// Get fusion diagnostics (which sensors contributing, weights)
    fn get_diagnostics(&self, metric: MetricType) -> FusionDiagnostics;

    /// Handle sensor dropout
    fn handle_dropout(&self, sensor_id: &Uuid);

    /// Handle sensor recovery
    fn handle_recovery(&self, sensor_id: &Uuid);
}

pub struct FusionDiagnostics {
    pub primary_sensor_id: Uuid,
    pub primary_contributing: bool,
    pub primary_weight: f32,
    pub secondary_sensor_id: Option<Uuid>,
    pub secondary_contributing: bool,
    pub secondary_weight: f32,
    pub fused_value: Option<f64>,
    pub confidence: f32,
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    #[error("ANT+ dongle not found")]
    DongleNotFound,

    #[error("No channels available")]
    NoChannelsAvailable,

    #[error("Device not found: {0}")]
    DeviceNotFound(Uuid),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("USB error: {0}")]
    UsbError(#[from] rusb::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout")]
    Timeout,
}
```

---

## Events (via crossbeam channel)

```rust
pub enum SensorEvent {
    // ANT+ events
    DongleConnected(AntDongle),
    DongleDisconnected(Uuid),
    ChannelOpened { dongle_id: Uuid, channel: u8 },
    ChannelClosed { dongle_id: Uuid, channel: u8 },

    // Data events
    PowerUpdate { sensor_id: Uuid, power: u16 },
    HrUpdate { sensor_id: Uuid, hr: u8 },
    CadenceUpdate { sensor_id: Uuid, cadence: u8 },
    DynamicsUpdate { sensor_id: Uuid, data: CyclingDynamicsData },
    SmO2Update { sensor_id: Uuid, data: SmO2Reading },
    MotionUpdate { sensor_id: Uuid, data: MotionSample },

    // Control events
    GradientChanged { gradient: f32 },
    DualProtocolDetected { ble_id: Uuid, ant_id: Uuid },
}
```
