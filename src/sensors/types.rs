//! Sensor types and enums for BLE fitness sensors.
//!
//! T012: Define SensorType, Protocol, ConnectionState enums
//! T021: Define SensorError enum
//! T028: Define DiscoveredSensor, SensorState, SensorEvent types
//! T029: Define SensorConfig struct

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

/// Type of fitness sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    /// Smart trainer with FTMS support
    Trainer,
    /// Standalone power meter
    PowerMeter,
    /// Heart rate monitor
    HeartRate,
    /// Cadence sensor
    Cadence,
    /// Speed sensor
    Speed,
    /// Combined speed/cadence sensor
    SpeedCadence,
}

impl std::fmt::Display for SensorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorType::Trainer => write!(f, "Smart Trainer"),
            SensorType::PowerMeter => write!(f, "Power Meter"),
            SensorType::HeartRate => write!(f, "Heart Rate"),
            SensorType::Cadence => write!(f, "Cadence"),
            SensorType::Speed => write!(f, "Speed"),
            SensorType::SpeedCadence => write!(f, "Speed/Cadence"),
        }
    }
}

/// BLE communication protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    /// BLE Fitness Machine Service (0x1826)
    BleFtms,
    /// BLE Cycling Power Service (0x1818)
    BleCyclingPower,
    /// BLE Heart Rate Service (0x180D)
    BleHeartRate,
    /// BLE Cycling Speed and Cadence (0x1816)
    BleCsc,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::BleFtms => write!(f, "FTMS"),
            Protocol::BleCyclingPower => write!(f, "Cycling Power"),
            Protocol::BleHeartRate => write!(f, "Heart Rate"),
            Protocol::BleCsc => write!(f, "Cycling Speed/Cadence"),
        }
    }
}

/// Connection state of a sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// Not connected
    #[default]
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Active connection
    Connected,
    /// Auto-reconnect in progress
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting..."),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting => write!(f, "Reconnecting..."),
        }
    }
}

/// A sensor discovered during BLE scanning.
#[derive(Debug, Clone)]
pub struct DiscoveredSensor {
    /// BLE device address/identifier
    pub device_id: String,
    /// User-friendly name (from BLE advertisement)
    pub name: String,
    /// Detected sensor type
    pub sensor_type: SensorType,
    /// Communication protocol
    pub protocol: Protocol,
    /// Signal strength (RSSI)
    pub signal_strength: Option<i16>,
    /// When the sensor was last seen
    pub last_seen: Instant,
}

/// Runtime state of a connected sensor.
#[derive(Debug, Clone)]
pub struct SensorState {
    /// Unique identifier (from database)
    pub id: Uuid,
    /// BLE device address/identifier
    pub device_id: String,
    /// User-friendly name
    pub name: String,
    /// Type of sensor
    pub sensor_type: SensorType,
    /// Communication protocol
    pub protocol: Protocol,
    /// Current connection state
    pub connection_state: ConnectionState,
    /// Signal strength (RSSI)
    pub signal_strength: Option<i16>,
    /// Battery level percentage (0-100)
    pub battery_level: Option<u8>,
    /// When data was last received
    pub last_data_at: Option<Instant>,
    /// Is this the primary source for its data type
    pub is_primary: bool,
}

/// Live data reading from a sensor.
#[derive(Debug, Clone)]
pub struct SensorReading {
    /// Source sensor ID
    pub sensor_id: Uuid,
    /// Reading timestamp
    pub timestamp: Instant,
    /// Power reading in watts
    pub power_watts: Option<u16>,
    /// Cadence in RPM
    pub cadence_rpm: Option<u8>,
    /// Heart rate in BPM
    pub heart_rate_bpm: Option<u8>,
    /// Speed in km/h
    pub speed_kmh: Option<f32>,
    /// Distance increment in meters
    pub distance_delta_m: Option<f32>,
}

/// Events from the sensor system.
#[derive(Debug, Clone)]
pub enum SensorEvent {
    /// A new sensor was discovered during scanning
    Discovered(DiscoveredSensor),
    /// Sensor connection state changed
    ConnectionChanged {
        device_id: String,
        state: ConnectionState,
    },
    /// New data received from sensor
    Data(SensorReading),
    /// Scan started
    ScanStarted,
    /// Scan stopped
    ScanStopped,
    /// Error occurred
    Error(String),
}

/// Configuration for the sensor manager.
#[derive(Debug, Clone)]
pub struct SensorConfig {
    /// Timeout for discovery scan in seconds
    pub discovery_timeout_secs: u64,
    /// Timeout for connection attempt in seconds
    pub connection_timeout_secs: u64,
    /// Whether to auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Delay between reconnection attempts in seconds
    pub reconnect_delay_secs: u64,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            discovery_timeout_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 2,
        }
    }
}

/// A sensor saved in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSensor {
    /// Unique identifier in the database
    pub id: Uuid,
    /// User ID who owns this sensor
    pub user_id: Uuid,
    /// BLE device address/identifier
    pub device_id: String,
    /// User-friendly name
    pub name: String,
    /// Type of sensor
    pub sensor_type: SensorType,
    /// Communication protocol
    pub protocol: Protocol,
    /// When the sensor was last seen online
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Is this the primary source for its data type
    pub is_primary: bool,
    /// When the sensor was first added
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl SavedSensor {
    /// Create a new saved sensor from a discovered sensor.
    pub fn from_discovered(discovered: &DiscoveredSensor, user_id: Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            device_id: discovered.device_id.clone(),
            name: discovered.name.clone(),
            sensor_type: discovered.sensor_type,
            protocol: discovered.protocol,
            last_seen_at: Some(now),
            is_primary: false,
            created_at: now,
        }
    }
}

/// Errors that can occur in the sensor system.
#[derive(Debug, Error)]
pub enum SensorError {
    /// BLE adapter not found or unavailable
    #[error("Bluetooth adapter not found")]
    AdapterNotFound,

    /// BLE is not enabled on the system
    #[error("Bluetooth is disabled")]
    BluetoothDisabled,

    /// Failed to start BLE scanning
    #[error("Failed to start scanning: {0}")]
    ScanFailed(String),

    /// Sensor not found with given device ID
    #[error("Sensor not found: {0}")]
    SensorNotFound(String),

    /// Connection to sensor failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Connection timed out
    #[error("Connection timed out")]
    ConnectionTimeout,

    /// Sensor disconnected unexpectedly
    #[error("Sensor disconnected: {0}")]
    Disconnected(String),

    /// Failed to subscribe to sensor notifications
    #[error("Failed to subscribe to notifications: {0}")]
    SubscriptionFailed(String),

    /// Failed to write to sensor characteristic
    #[error("Write failed: {0}")]
    WriteFailed(String),

    /// Unsupported sensor or protocol
    #[error("Unsupported sensor type or protocol")]
    Unsupported,

    /// Permission denied for Bluetooth access
    #[error("Bluetooth permission denied")]
    PermissionDenied,

    /// Generic BLE error
    #[error("BLE error: {0}")]
    BleError(String),
}
