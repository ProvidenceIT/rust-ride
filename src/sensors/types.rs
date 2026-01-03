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
    /// Smart trainer (alias for compatibility)
    SmartTrainer,
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
    /// Cadence sensor (alias)
    CadenceSensor,
    /// Muscle oxygen sensor (SmO2)
    SmO2,
    /// Inertial measurement unit (motion tracking)
    Imu,
}

impl std::fmt::Display for SensorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorType::Trainer => write!(f, "Smart Trainer"),
            SensorType::SmartTrainer => write!(f, "Smart Trainer"),
            SensorType::PowerMeter => write!(f, "Power Meter"),
            SensorType::HeartRate => write!(f, "Heart Rate"),
            SensorType::Cadence => write!(f, "Cadence"),
            SensorType::CadenceSensor => write!(f, "Cadence Sensor"),
            SensorType::Speed => write!(f, "Speed"),
            SensorType::SpeedCadence => write!(f, "Speed/Cadence"),
            SensorType::SmO2 => write!(f, "Muscle Oxygen"),
            SensorType::Imu => write!(f, "Motion Sensor"),
        }
    }
}

/// High-level sensor protocol (BLE vs ANT+)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorProtocol {
    /// Bluetooth Low Energy
    Ble,
    /// ANT+ wireless protocol
    AntPlus,
}

impl std::fmt::Display for SensorProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorProtocol::Ble => write!(f, "BLE"),
            SensorProtocol::AntPlus => write!(f, "ANT+"),
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
    /// ANT+ Heart Rate profile
    AntHeartRate,
    /// ANT+ Cycling Power profile
    AntPower,
    /// ANT+ FE-C (Fitness Equipment Control)
    AntFec,
    /// ANT+ Speed/Cadence profile
    AntSpeedCadence,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::BleFtms => write!(f, "FTMS"),
            Protocol::BleCyclingPower => write!(f, "Cycling Power"),
            Protocol::BleHeartRate => write!(f, "Heart Rate"),
            Protocol::BleCsc => write!(f, "Cycling Speed/Cadence"),
            Protocol::AntHeartRate => write!(f, "ANT+ HR"),
            Protocol::AntPower => write!(f, "ANT+ Power"),
            Protocol::AntFec => write!(f, "ANT+ FE-C"),
            Protocol::AntSpeedCadence => write!(f, "ANT+ S/C"),
        }
    }
}

impl Protocol {
    /// Get the high-level sensor protocol
    pub fn sensor_protocol(&self) -> SensorProtocol {
        match self {
            Protocol::BleFtms
            | Protocol::BleCyclingPower
            | Protocol::BleHeartRate
            | Protocol::BleCsc => SensorProtocol::Ble,
            Protocol::AntHeartRate
            | Protocol::AntPower
            | Protocol::AntFec
            | Protocol::AntSpeedCadence => SensorProtocol::AntPlus,
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
    /// Primary sensor failover occurred.
    /// When the primary sensor for a data type disconnects and a secondary
    /// sensor is available, the secondary is automatically promoted to primary.
    FailoverActivated {
        /// The data type that experienced failover (e.g., Power, HeartRate)
        data_type: String,
        /// The device ID of the sensor that disconnected (former primary)
        from_device_id: String,
        /// The name of the sensor that disconnected
        from_sensor_name: String,
        /// The device ID of the sensor that was promoted to primary
        to_device_id: String,
        /// The name of the sensor that was promoted to primary
        to_sensor_name: String,
    },
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
    /// Progressive timeout configuration for discovery
    pub progressive_timeout: ProgressiveTimeoutConfig,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            discovery_timeout_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            reconnect_delay_secs: 2,
            progressive_timeout: ProgressiveTimeoutConfig::default(),
        }
    }
}

/// Configuration for progressive discovery timeout.
///
/// Progressive timeout starts with an aggressive initial scan period,
/// then extends the scan if sensors are still being discovered. This
/// balances fast discovery when sensors are readily available with
/// longer scans when sensors are slow to respond.
///
/// Power meters may take longer to advertise due to sleep mode. When
/// a saved power meter is expected but not found, discovery can extend
/// up to `power_meter_max_secs` (default: 45 seconds).
#[derive(Debug, Clone)]
pub struct ProgressiveTimeoutConfig {
    /// Initial aggressive scan period in seconds (default: 10s)
    pub initial_scan_secs: u64,
    /// Extension period when sensors are still being found (default: 5s)
    pub extension_period_secs: u64,
    /// Maximum total discovery time in seconds (default: 30s)
    pub max_total_secs: u64,
    /// Maximum discovery time when waiting for power meters (default: 45s)
    /// Power meters may take longer to advertise due to sleep mode.
    pub power_meter_max_secs: u64,
    /// Time window for detecting sensor activity (default: 3s)
    /// If a sensor is discovered within this window before timeout,
    /// the scan is extended.
    pub activity_window_secs: u64,
    /// Minimum time since last discovery before stopping early (default: 5s)
    /// If no sensors are found for this duration, scan may stop early.
    pub idle_threshold_secs: u64,
    /// Whether progressive timeout is enabled (default: true)
    pub enabled: bool,
}

impl Default for ProgressiveTimeoutConfig {
    fn default() -> Self {
        Self {
            initial_scan_secs: 10,
            extension_period_secs: 5,
            max_total_secs: 30,
            power_meter_max_secs: 45,
            activity_window_secs: 3,
            idle_threshold_secs: 5,
            enabled: true,
        }
    }
}

impl ProgressiveTimeoutConfig {
    /// Create a fast scan configuration for quick discovery.
    pub fn fast() -> Self {
        Self {
            initial_scan_secs: 5,
            extension_period_secs: 3,
            max_total_secs: 15,
            power_meter_max_secs: 30, // Shorter extension for fast mode
            activity_window_secs: 2,
            idle_threshold_secs: 3,
            enabled: true,
        }
    }

    /// Create a thorough scan configuration for finding all sensors.
    pub fn thorough() -> Self {
        Self {
            initial_scan_secs: 15,
            extension_period_secs: 10,
            max_total_secs: 45,
            power_meter_max_secs: 60, // Longer extension for thorough mode
            activity_window_secs: 5,
            idle_threshold_secs: 8,
            enabled: true,
        }
    }

    /// Create a disabled configuration (uses fixed timeout).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// State tracking for progressive discovery timeout.
///
/// This struct tracks the discovery progress and determines when to
/// extend or stop scanning based on sensor activity.
#[derive(Debug, Clone)]
pub struct ProgressiveTimeoutState {
    /// When discovery started
    pub started_at: std::time::Instant,
    /// When the current timeout phase started
    pub phase_started_at: std::time::Instant,
    /// When a sensor was last discovered
    pub last_discovery_at: Option<std::time::Instant>,
    /// Number of sensors discovered so far
    pub sensors_discovered: usize,
    /// Current scan phase
    pub phase: DiscoveryPhase,
    /// Number of extensions applied
    pub extensions_count: u32,
}

impl ProgressiveTimeoutState {
    /// Create a new timeout state starting now.
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            started_at: now,
            phase_started_at: now,
            last_discovery_at: None,
            sensors_discovered: 0,
            phase: DiscoveryPhase::Initial,
            extensions_count: 0,
        }
    }

    /// Record that a sensor was discovered.
    pub fn record_discovery(&mut self) {
        self.last_discovery_at = Some(std::time::Instant::now());
        self.sensors_discovered += 1;
    }

    /// Get time elapsed since discovery started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Get time since last sensor discovery.
    pub fn time_since_last_discovery(&self) -> Option<std::time::Duration> {
        self.last_discovery_at.map(|t| t.elapsed())
    }

    /// Check if a sensor was recently discovered.
    pub fn has_recent_activity(&self, window_secs: u64) -> bool {
        self.last_discovery_at.map_or(false, |t| {
            t.elapsed() < std::time::Duration::from_secs(window_secs)
        })
    }

    /// Check if discovery has been idle (no recent discoveries).
    pub fn is_idle(&self, threshold_secs: u64) -> bool {
        // If we've never discovered anything, check time since start
        match self.last_discovery_at {
            Some(t) => t.elapsed() >= std::time::Duration::from_secs(threshold_secs),
            None => self.started_at.elapsed() >= std::time::Duration::from_secs(threshold_secs),
        }
    }

    /// Calculate the decision for the current state.
    pub fn calculate_decision(&self, config: &ProgressiveTimeoutConfig) -> TimeoutDecision {
        if !config.enabled {
            // Progressive timeout disabled, use fixed timeout
            if self.elapsed() >= std::time::Duration::from_secs(config.max_total_secs) {
                return TimeoutDecision::Stop {
                    reason: StopReason::MaxTimeReached,
                };
            }
            return TimeoutDecision::Continue;
        }

        let elapsed = self.elapsed();
        let max_duration = std::time::Duration::from_secs(config.max_total_secs);

        // Always stop if we've reached max time
        if elapsed >= max_duration {
            return TimeoutDecision::Stop {
                reason: StopReason::MaxTimeReached,
            };
        }

        match self.phase {
            DiscoveryPhase::Initial => {
                let initial_duration = std::time::Duration::from_secs(config.initial_scan_secs);

                if elapsed >= initial_duration {
                    // Initial phase complete, decide whether to extend
                    if self.has_recent_activity(config.activity_window_secs) {
                        // Recent activity - extend the scan
                        TimeoutDecision::Extend
                    } else if self.sensors_discovered > 0 && self.is_idle(config.idle_threshold_secs) {
                        // Found some sensors but been idle - stop early
                        TimeoutDecision::Stop {
                            reason: StopReason::IdleTimeout,
                        }
                    } else if self.sensors_discovered == 0 {
                        // No sensors found yet - extend to look for more
                        TimeoutDecision::Extend
                    } else {
                        // Continue in initial phase
                        TimeoutDecision::Continue
                    }
                } else {
                    TimeoutDecision::Continue
                }
            }
            DiscoveryPhase::Extended => {
                let extension_duration = std::time::Duration::from_secs(config.extension_period_secs);
                let phase_elapsed = self.phase_started_at.elapsed();

                if phase_elapsed >= extension_duration {
                    // Extension period complete
                    if self.has_recent_activity(config.activity_window_secs) {
                        // Still finding sensors - extend again if not at max
                        if elapsed + extension_duration <= max_duration {
                            TimeoutDecision::Extend
                        } else {
                            TimeoutDecision::Stop {
                                reason: StopReason::MaxTimeReached,
                            }
                        }
                    } else {
                        // No recent activity - stop
                        TimeoutDecision::Stop {
                            reason: StopReason::IdleTimeout,
                        }
                    }
                } else {
                    TimeoutDecision::Continue
                }
            }
            DiscoveryPhase::Completed => {
                TimeoutDecision::Stop {
                    reason: StopReason::Completed,
                }
            }
        }
    }

    /// Apply an extension to the timeout.
    pub fn apply_extension(&mut self) {
        self.phase = DiscoveryPhase::Extended;
        self.phase_started_at = std::time::Instant::now();
        self.extensions_count += 1;
    }

    /// Mark discovery as completed.
    pub fn mark_completed(&mut self) {
        self.phase = DiscoveryPhase::Completed;
    }
}

impl Default for ProgressiveTimeoutState {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovery phase for progressive timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryPhase {
    /// Initial aggressive scan period
    Initial,
    /// Extended scan period (after initial)
    Extended,
    /// Discovery completed
    Completed,
}

impl std::fmt::Display for DiscoveryPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryPhase::Initial => write!(f, "Initial Scan"),
            DiscoveryPhase::Extended => write!(f, "Extended Scan"),
            DiscoveryPhase::Completed => write!(f, "Completed"),
        }
    }
}

/// Decision from progressive timeout calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutDecision {
    /// Continue scanning in current phase
    Continue,
    /// Extend scanning with a new phase
    Extend,
    /// Stop scanning
    Stop { reason: StopReason },
}

/// Reason for stopping discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Maximum time reached
    MaxTimeReached,
    /// Idle timeout (no recent discoveries)
    IdleTimeout,
    /// Discovery completed normally
    Completed,
    /// User requested stop
    UserRequested,
}

/// Summary of discovery progress for UI display.
#[derive(Debug, Clone)]
pub struct DiscoveryProgress {
    /// Current discovery phase
    pub phase: DiscoveryPhase,
    /// Time elapsed since discovery started
    pub elapsed: std::time::Duration,
    /// Number of sensors discovered so far
    pub sensors_discovered: usize,
    /// Number of extensions applied
    pub extensions_count: u32,
    /// Whether discovery is still active
    pub is_active: bool,
}

impl DiscoveryProgress {
    /// Get a human-readable status string.
    pub fn status_text(&self) -> String {
        if !self.is_active {
            return format!(
                "Completed: {} sensor{}",
                self.sensors_discovered,
                if self.sensors_discovered == 1 { "" } else { "s" }
            );
        }

        let elapsed_secs = self.elapsed.as_secs();
        let phase_text = match self.phase {
            DiscoveryPhase::Initial => "Scanning",
            DiscoveryPhase::Extended => "Extended scan",
            DiscoveryPhase::Completed => "Complete",
        };

        format!(
            "{} ({:.0}s): {} sensor{}",
            phase_text,
            elapsed_secs,
            self.sensors_discovered,
            if self.sensors_discovered == 1 { "" } else { "s" }
        )
    }

    /// Get progress as a percentage (approximate based on typical timing).
    pub fn progress_percent(&self, config: &ProgressiveTimeoutConfig) -> f32 {
        if !self.is_active {
            return 100.0;
        }

        let elapsed_secs = self.elapsed.as_secs_f32();
        let max_secs = config.max_total_secs as f32;

        (elapsed_secs / max_secs * 100.0).min(99.0)
    }
}

/// Result of parallel BLE/ANT+ discovery.
///
/// Indicates which protocols were successfully started and any errors.
#[derive(Debug, Clone, Default)]
pub struct ParallelDiscoveryResult {
    /// Whether BLE discovery was successfully started
    pub ble_started: bool,
    /// Whether ANT+ discovery was successfully started
    pub ant_started: bool,
    /// Error message if BLE failed to start
    pub ble_error: Option<String>,
    /// Error message if ANT+ failed to start
    pub ant_error: Option<String>,
}

impl ParallelDiscoveryResult {
    /// Check if at least one protocol started successfully.
    pub fn any_started(&self) -> bool {
        self.ble_started || self.ant_started
    }

    /// Check if both protocols started successfully.
    pub fn all_started(&self) -> bool {
        self.ble_started && self.ant_started
    }

    /// Check if there were any errors.
    pub fn has_errors(&self) -> bool {
        self.ble_error.is_some() || self.ant_error.is_some()
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

/// Commands that can be sent to the sensor manager from the UI.
#[derive(Debug, Clone)]
pub enum SensorCommand {
    /// Start BLE discovery
    StartDiscovery,
    /// Stop BLE discovery
    StopDiscovery,
    /// Connect to a sensor by device ID
    Connect(String),
    /// Disconnect from a sensor by device ID
    Disconnect(String),
    /// Set ERG mode power target in watts
    SetErgTarget(u16),
    /// Set simulation mode gradient
    SetSimGrade(f32),
}

/// Connection status for display in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Sensor discovered but not connected
    Discovered,
    /// Connection attempt in progress
    Connecting,
    /// Successfully connected
    Connected,
    /// Disconnection in progress
    Disconnecting,
    /// Not connected
    Disconnected,
    /// Connection attempt failed
    Error,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Discovered => write!(f, "Found"),
            ConnectionStatus::Connecting => write!(f, "Connecting..."),
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Disconnecting => write!(f, "Disconnecting..."),
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Error => write!(f, "Error"),
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

    /// Data parsing error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// ANT+ dongle not found
    #[error("ANT+ dongle not found")]
    DongleNotFound,

    /// No ANT+ channels available
    #[error("No ANT+ channels available")]
    NoChannelsAvailable,

    /// Protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}
