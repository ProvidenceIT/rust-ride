# Contract: Sensor Control Module

**Module**: `src/sensors/`
**Feature**: 002-3d-world-features
**Status**: Extension of existing module

## Purpose

Complete the BLE sensor management functionality by implementing actual discovery, connection, and disconnection operations. The module already has the structure; this contract defines the completion of TODO placeholders.

## Public Interface

### SensorManager

```rust
impl SensorManager {
    /// Start BLE discovery for fitness devices
    /// Scans for FTMS, Cycling Power, and Heart Rate services
    /// Discovered devices sent via crossbeam channel to UI
    pub async fn start_discovery(&mut self) -> Result<(), SensorError>;

    /// Stop active BLE discovery
    pub async fn stop_discovery(&mut self) -> Result<(), SensorError>;

    /// Connect to a discovered sensor by device ID
    /// Establishes connection, discovers services, subscribes to notifications
    pub async fn connect(&mut self, device_id: &str) -> Result<(), SensorError>;

    /// Disconnect from a connected sensor
    pub async fn disconnect(&mut self, device_id: &str) -> Result<(), SensorError>;

    /// Get current connection status for all known sensors
    pub fn get_sensor_states(&self) -> Vec<SensorState>;

    /// Check if any trainer is connected and ready for ERG mode
    pub fn has_controllable_trainer(&self) -> bool;
}
```

### SensorState

```rust
pub struct SensorState {
    pub device_id: String,
    pub name: String,
    pub sensor_type: SensorType,
    pub status: ConnectionStatus,
    pub signal_strength: Option<i16>,  // RSSI in dBm
    pub last_seen: Instant,
}

pub enum ConnectionStatus {
    Discovered,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Error(String),
}
```

### Channel Messages

```rust
/// Messages sent from sensor manager to UI
pub enum SensorEvent {
    DeviceDiscovered(SensorState),
    DeviceConnected { device_id: String },
    DeviceDisconnected { device_id: String },
    ConnectionFailed { device_id: String, error: String },
    DataReceived(SensorData),
}

/// Commands sent from UI to sensor manager
pub enum SensorCommand {
    StartDiscovery,
    StopDiscovery,
    Connect(String),      // device_id
    Disconnect(String),   // device_id
    SetErgTarget(u16),    // watts
}
```

## Dependencies

- `btleplug` - BLE operations
- `crossbeam::channel` - Thread-safe message passing
- `tokio` - Async runtime for BLE operations

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    #[error("Bluetooth adapter not found")]
    AdapterNotFound,

    #[error("Bluetooth is disabled")]
    BluetoothDisabled,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Service discovery failed: {0}")]
    ServiceDiscoveryFailed(String),

    #[error("Already scanning")]
    AlreadyScanning,

    #[error("BLE error: {0}")]
    BleError(#[from] btleplug::Error),
}
```

## Thread Model

```
┌─────────────────┐     SensorCommand      ┌──────────────────┐
│   UI Thread     │ ───────────────────►   │  Tokio Runtime   │
│   (egui)        │                        │  (BLE ops)       │
│                 │ ◄───────────────────   │                  │
└─────────────────┘     SensorEvent        └──────────────────┘
```

## Implementation Notes

1. **Discovery timeout**: 30 seconds, then auto-stop
2. **Connection timeout**: 10 seconds per device
3. **Auto-reconnect**: Attempt reconnection on unexpected disconnect (3 retries)
4. **Service filter**: Only show devices with FTMS, Cycling Power, or Heart Rate services
5. **Signal strength**: Update RSSI every 2 seconds during discovery

## Testing Requirements

- Unit tests for state machine transitions
- Integration tests with mock BLE adapter
- Manual testing with real hardware required for release
