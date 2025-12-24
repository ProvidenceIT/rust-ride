# Contract: Sensor Manager

**Module**: `src/sensors/`
**Responsibility**: BLE sensor discovery, connection management, and data streaming

## Public Interface

### SensorManager

Main entry point for sensor operations.

```rust
pub struct SensorManager {
    // Internal state
}

impl SensorManager {
    /// Create a new sensor manager
    /// Initializes BLE adapter and loads saved sensor pairings
    pub async fn new(config: SensorConfig) -> Result<Self, SensorError>;

    /// Start scanning for nearby BLE sensors
    /// Emits DiscoveredSensor events via the event channel
    pub async fn start_discovery(&self) -> Result<(), SensorError>;

    /// Stop the active scan
    pub async fn stop_discovery(&self) -> Result<(), SensorError>;

    /// Connect to a specific sensor
    /// Returns a handle for the connection
    pub async fn connect(&self, sensor_id: &SensorId) -> Result<SensorConnection, SensorError>;

    /// Disconnect from a sensor
    pub async fn disconnect(&self, sensor_id: &SensorId) -> Result<(), SensorError>;

    /// Get connection status for all known sensors
    pub fn get_sensor_states(&self) -> Vec<SensorState>;

    /// Get the event receiver for sensor events
    pub fn events(&self) -> Receiver<SensorEvent>;

    /// Save a sensor pairing for auto-reconnect
    pub async fn save_pairing(&self, sensor: &DiscoveredSensor, name: String) -> Result<Sensor, SensorError>;

    /// Remove a saved sensor pairing
    pub async fn remove_pairing(&self, sensor_id: &SensorId) -> Result<(), SensorError>;

    /// Get all saved sensor pairings
    pub fn get_saved_sensors(&self) -> Vec<Sensor>;

    /// Attempt to reconnect to all saved sensors
    pub async fn reconnect_saved(&self) -> Vec<Result<SensorConnection, SensorError>>;
}
```

### Types

```rust
/// Configuration for sensor manager
pub struct SensorConfig {
    /// How long to scan before auto-stopping (None = indefinite)
    pub scan_timeout: Option<Duration>,
    /// How often to attempt reconnection
    pub reconnect_interval: Duration,
    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            scan_timeout: Some(Duration::from_secs(30)),
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }
}

/// Unique identifier for a sensor
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SensorId(pub String); // BLE address

/// A discovered but not yet paired sensor
pub struct DiscoveredSensor {
    pub id: SensorId,
    pub name: Option<String>,
    pub sensor_type: SensorType,
    pub protocol: Protocol,
    pub rssi: Option<i8>,
}

/// Current state of a sensor
pub struct SensorState {
    pub sensor: Sensor,
    pub connection_state: ConnectionState,
    pub battery_level: Option<u8>,
    pub last_data: Option<Instant>,
}

/// Events emitted by the sensor manager
pub enum SensorEvent {
    /// New sensor discovered during scan
    SensorDiscovered(DiscoveredSensor),

    /// Sensor connection established
    Connected { sensor_id: SensorId },

    /// Sensor disconnected (planned or unexpected)
    Disconnected { sensor_id: SensorId, reason: DisconnectReason },

    /// Reconnection attempt started
    Reconnecting { sensor_id: SensorId, attempt: u32 },

    /// New data received from sensor
    DataReceived { sensor_id: SensorId, reading: SensorReading },

    /// Battery level updated
    BatteryUpdated { sensor_id: SensorId, level: u8 },

    /// Scan started
    ScanStarted,

    /// Scan stopped
    ScanStopped,

    /// Error occurred
    Error { sensor_id: Option<SensorId>, error: SensorError },
}

pub enum DisconnectReason {
    UserRequested,
    SignalLost,
    DevicePoweredOff,
    Error(SensorError),
}

pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
}
```

### SensorConnection

Handle for an active sensor connection.

```rust
pub struct SensorConnection {
    // Internal state
}

impl SensorConnection {
    /// Get the sensor ID
    pub fn sensor_id(&self) -> &SensorId;

    /// Get the sensor type
    pub fn sensor_type(&self) -> SensorType;

    /// Check if connection is still active
    pub fn is_connected(&self) -> bool;

    /// Subscribe to data notifications
    /// Returns a receiver for SensorReading events
    pub fn subscribe(&self) -> Receiver<SensorReading>;

    /// For trainers: Set ERG mode target power
    pub async fn set_target_power(&self, watts: u16) -> Result<(), SensorError>;

    /// For trainers: Set resistance level (0-100%)
    pub async fn set_resistance(&self, percent: u8) -> Result<(), SensorError>;

    /// For trainers: Start/stop training session
    pub async fn set_training_active(&self, active: bool) -> Result<(), SensorError>;

    /// For trainers: Trigger spindown calibration
    pub async fn start_calibration(&self) -> Result<CalibrationHandle, SensorError>;

    /// Get current battery level if available
    pub async fn get_battery_level(&self) -> Result<Option<u8>, SensorError>;
}
```

### Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    #[error("BLE adapter not found")]
    AdapterNotFound,

    #[error("BLE adapter error: {0}")]
    AdapterError(String),

    #[error("Sensor not found: {0}")]
    SensorNotFound(SensorId),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection lost")]
    ConnectionLost,

    #[error("Operation not supported by this sensor")]
    NotSupported,

    #[error("Write operation failed: {0}")]
    WriteFailed(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Calibration failed: {0}")]
    CalibrationFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```

## Dependencies

- `btleplug` - BLE communication
- `tokio` - Async runtime
- `crossbeam` - Channel communication
- `rusqlite` - Sensor persistence

## Threading Model

```
┌─────────────────────────────┐
│    SensorManager (main)     │
│  - Manages connections      │
│  - Coordinates discovery    │
└────────────┬────────────────┘
             │
    ┌────────┴────────┐
    │                 │
    ▼                 ▼
┌─────────┐    ┌─────────────┐
│ Scanner │    │ Connections │
│ (tokio) │    │ (per sensor)│
└────┬────┘    └──────┬──────┘
     │                │
     │   SensorEvent  │
     └───────┬────────┘
             │
             ▼
┌─────────────────────────────┐
│      Event Channel          │
│  (crossbeam::unbounded)     │
└─────────────────────────────┘
             │
             ▼
         UI Thread
```

## Behavioral Requirements

1. **Discovery**: Scan filters by FTMS, CPS, and HRS service UUIDs
2. **Auto-reconnect**: On unexpected disconnect, attempt reconnection up to `max_reconnect_attempts`
3. **Data rate**: Emit `DataReceived` events at sensor's native rate (~1Hz for most)
4. **Multi-sensor**: Support simultaneous connections to multiple sensors
5. **Primary selection**: When multiple sensors provide same data type, use the one marked `is_primary`
