//! Daemon state management.
//!
//! Defines the state machine and data structures for the daemon process.
//!
//! T033: Integration with existing sensor management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::sensors::{ConnectionState as BleConnectionState, SensorState as BleSensorState, SensorType as BleSensorType};

/// Daemon lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonStatus {
    /// Daemon is initializing
    Starting,
    /// Daemon is fully operational
    Running,
    /// Running but with limited functionality (e.g., no BLE adapter)
    Degraded,
    /// Graceful shutdown in progress
    ShuttingDown,
    /// Daemon has stopped
    Stopped,
}

/// Information about the current daemon state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonState {
    /// Process ID
    pub pid: u32,
    /// When the daemon started
    pub started_at: DateTime<Utc>,
    /// Current status
    pub status: DaemonStatus,
    /// Current active session (ride or workout)
    pub active_session: Option<SessionInfo>,
    /// Connected sensors
    pub connected_sensors: Vec<SensorInfo>,
    /// Whether BLE adapter is available
    pub ble_adapter_available: bool,
    /// Path to config file
    pub config_path: Option<PathBuf>,
    /// Path to socket file
    pub socket_path: Option<PathBuf>,
    /// Path to log file
    pub log_path: Option<PathBuf>,
    /// T072: Incomplete rides detected on startup that can be recovered
    #[serde(default)]
    pub incomplete_rides: Vec<super::IncompleteRideInfo>,
}

impl DaemonState {
    /// Create a new daemon state
    pub fn new() -> Self {
        Self {
            pid: std::process::id(),
            started_at: Utc::now(),
            status: DaemonStatus::Starting,
            active_session: None,
            connected_sensors: Vec::new(),
            ble_adapter_available: false,
            config_path: None,
            socket_path: None,
            log_path: None,
            incomplete_rides: Vec::new(),
        }
    }

    /// Calculate uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        (Utc::now() - self.started_at).num_seconds().max(0) as u64
    }

    /// Mark daemon as running
    pub fn set_running(&mut self) {
        self.status = DaemonStatus::Running;
    }

    /// Mark daemon as degraded (e.g., no BLE)
    pub fn set_degraded(&mut self) {
        self.status = DaemonStatus::Degraded;
    }

    /// Initiate shutdown
    pub fn initiate_shutdown(&mut self) {
        self.status = DaemonStatus::ShuttingDown;
    }

    /// T033: Update connected sensors from SensorManager states
    pub fn update_sensors_from_ble(&mut self, sensor_states: Vec<BleSensorState>) {
        self.connected_sensors = sensor_states
            .iter()
            .filter(|s| s.connection_state == BleConnectionState::Connected)
            .map(SensorInfo::from_ble_sensor_state)
            .collect();
    }

    /// T033: Update BLE adapter availability
    pub fn set_ble_adapter_available(&mut self, available: bool) {
        self.ble_adapter_available = available;
        if !available && self.status == DaemonStatus::Running {
            self.status = DaemonStatus::Degraded;
        }
    }

    /// T033: Add a sensor to connected list
    pub fn add_sensor(&mut self, sensor: SensorInfo) {
        // Remove existing entry with same ID if present
        self.connected_sensors.retain(|s| s.id != sensor.id);
        self.connected_sensors.push(sensor);
    }

    /// T033: Remove a sensor from connected list
    pub fn remove_sensor(&mut self, sensor_id: &str) {
        self.connected_sensors.retain(|s| s.id != sensor_id);
    }

    /// T033: Update a sensor's connection status
    pub fn update_sensor_status(&mut self, sensor_id: &str, status: ConnectionStatus) {
        if let Some(sensor) = self.connected_sensors.iter_mut().find(|s| s.id == sensor_id) {
            sensor.connection_status = status.clone();
            sensor.last_seen = Utc::now();
        }
        // Remove disconnected sensors from the list
        if status == ConnectionStatus::Disconnected {
            self.connected_sensors.retain(|s| s.id != sensor_id);
        }
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of session (ride or workout)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    /// Unstructured free ride
    FreeRide,
    /// Structured workout from file
    Workout { path: PathBuf },
}

/// Information about an active session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Type of session
    pub session_type: SessionType,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// Workout details (if applicable)
    pub workout_info: Option<WorkoutInfo>,
    /// Current live metrics
    pub current_metrics: LiveMetrics,
    /// Whether session is paused
    pub is_paused: bool,
}

impl SessionInfo {
    /// Calculate elapsed seconds
    pub fn elapsed_seconds(&self) -> u64 {
        (Utc::now() - self.started_at).num_seconds().max(0) as u64
    }
}

/// Details about a running workout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutInfo {
    /// Workout name
    pub name: String,
    /// Path to workout file
    pub file_path: PathBuf,
    /// Total workout duration in seconds
    pub total_duration_seconds: u64,
    /// Current interval index (0-based)
    pub current_interval_index: usize,
    /// Total number of intervals
    pub total_intervals: usize,
    /// Name of current interval
    pub current_interval_name: String,
    /// Seconds into current interval
    pub interval_elapsed_seconds: u64,
    /// Seconds remaining in current interval
    pub interval_remaining_seconds: u64,
    /// Current ERG target power in watts
    pub target_power_watts: u16,
    /// Target power as percentage of FTP
    pub target_power_percent_ftp: f32,
}

/// Real-time sensor readings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveMetrics {
    /// Current power in watts
    pub power_watts: Option<u16>,
    /// Current heart rate in bpm
    pub heart_rate_bpm: Option<u8>,
    /// Current cadence in rpm
    pub cadence_rpm: Option<u8>,
    /// Current speed in km/h
    pub speed_kmh: Option<f32>,
    /// Total distance in km
    pub distance_km: f32,
    /// Estimated calories burned
    pub calories: u32,
    /// Rolling normalized power in watts
    pub normalized_power: Option<u16>,
    /// Average power in watts
    pub average_power: Option<u16>,
    /// Average heart rate in bpm
    pub average_heart_rate: Option<u8>,
}

/// Information about a sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorInfo {
    /// Unique sensor identifier (BLE address)
    pub id: String,
    /// Display name
    pub name: String,
    /// Type of sensor
    pub sensor_type: SensorType,
    /// Connection status
    pub connection_status: ConnectionStatus,
    /// Signal strength in dBm
    pub signal_strength_dbm: Option<i8>,
    /// Battery level percentage
    pub battery_percent: Option<u8>,
    /// Last time data was received
    pub last_seen: DateTime<Utc>,
}

/// Types of sensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    /// FTMS-compatible smart trainer
    SmartTrainer,
    /// Cycling power meter
    PowerMeter,
    /// Heart rate monitor
    HeartRateMonitor,
    /// Cadence-only sensor
    CadenceSensor,
    /// Speed-only sensor
    SpeedSensor,
    /// Combined speed/cadence sensor
    SpeedCadenceSensor,
}

/// Sensor connection states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Sensor discovered but not connected
    Discovered,
    /// Connection in progress
    Connecting,
    /// Actively connected
    Connected,
    /// Lost connection, attempting reconnect
    Reconnecting,
    /// Intentionally disconnected
    Disconnected,
    /// Connection failed
    Failed { reason: String },
}

impl From<BleConnectionState> for ConnectionStatus {
    fn from(state: BleConnectionState) -> Self {
        match state {
            BleConnectionState::Disconnected => ConnectionStatus::Disconnected,
            BleConnectionState::Connecting => ConnectionStatus::Connecting,
            BleConnectionState::Connected => ConnectionStatus::Connected,
            BleConnectionState::Reconnecting => ConnectionStatus::Reconnecting,
        }
    }
}

impl From<BleSensorType> for SensorType {
    fn from(st: BleSensorType) -> Self {
        match st {
            BleSensorType::Trainer | BleSensorType::SmartTrainer => SensorType::SmartTrainer,
            BleSensorType::PowerMeter => SensorType::PowerMeter,
            BleSensorType::HeartRate => SensorType::HeartRateMonitor,
            BleSensorType::Cadence => SensorType::CadenceSensor,
            BleSensorType::Speed => SensorType::SpeedSensor,
            BleSensorType::SpeedCadence => SensorType::SpeedCadenceSensor,
        }
    }
}

impl SensorInfo {
    /// Create SensorInfo from an existing sensor's SensorState
    pub fn from_ble_sensor_state(state: &BleSensorState) -> Self {
        SensorInfo {
            id: state.device_id.clone(),
            name: state.name.clone(),
            sensor_type: state.sensor_type.into(),
            connection_status: state.connection_state.into(),
            signal_strength_dbm: state.signal_strength.map(|s| s as i8),
            battery_percent: state.battery_level,
            last_seen: Utc::now(),
        }
    }
}

/// Information about a recoverable interrupted session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInfo {
    /// Database ID of incomplete ride
    pub ride_id: i64,
    /// When the ride started
    pub started_at: DateTime<Utc>,
    /// Last recorded sample time
    pub last_sample_at: DateTime<Utc>,
    /// Duration before interruption
    pub duration_seconds: u64,
    /// Number of samples recorded
    pub sample_count: usize,
    /// Session type
    pub session_type: SessionType,
}
