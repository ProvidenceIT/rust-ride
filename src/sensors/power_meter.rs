//! Power meter wake-up detection and user prompting.
//!
//! Power meters often enter a low-power sleep mode when not in use. To wake
//! up, users typically need to pedal briefly. This module detects when a
//! saved power meter is expected but not discovered, and generates prompts
//! for the user to wake up the power meter.
//!
//! Key features:
//! - Tracks expected power meters from cache/saved sensors
//! - Detects when expected power meters are not found during discovery
//! - Generates user-friendly wake-up hints
//! - Supports multiple power meters
//! - Extended discovery time for power meters (up to 45 seconds)

use crate::sensors::cache::SensorCache;
use crate::sensors::types::{DiscoveredSensor, Protocol, SensorType};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Default time to wait before showing wake-up hint (10 seconds into discovery).
pub const DEFAULT_WAKE_UP_HINT_DELAY_SECS: u64 = 10;

/// Default grace period after discovery before considering power meter missing (5 seconds).
pub const DEFAULT_GRACE_PERIOD_SECS: u64 = 5;

/// Default extended discovery time for power meters (45 seconds).
/// Power meters may take longer to advertise due to sleep mode.
pub const DEFAULT_EXTENDED_DISCOVERY_SECS: u64 = 45;

/// Standard discovery time without power meter extension (30 seconds).
pub const DEFAULT_STANDARD_DISCOVERY_SECS: u64 = 30;

/// Minimum elapsed time before considering extended discovery (15 seconds).
/// We only extend after the initial scan period has passed.
pub const EXTENDED_DISCOVERY_THRESHOLD_SECS: u64 = 15;

/// Wake-up hint for a power meter that needs user action to activate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeUpHint {
    /// Device ID of the expected power meter.
    pub device_id: String,
    /// Name of the expected power meter.
    pub name: String,
    /// Protocol of the power meter (BLE or ANT+).
    pub protocol: Protocol,
    /// Type of hint to display.
    pub hint_type: WakeUpHintType,
    /// Whether this hint has been shown to the user.
    pub shown: bool,
    /// When the hint was generated.
    pub generated_at: Instant,
}

impl WakeUpHint {
    /// Create a new wake-up hint.
    pub fn new(device_id: String, name: String, protocol: Protocol, hint_type: WakeUpHintType) -> Self {
        Self {
            device_id,
            name,
            protocol,
            hint_type,
            shown: false,
            generated_at: Instant::now(),
        }
    }

    /// Get a user-friendly message for this hint.
    pub fn message(&self) -> String {
        match self.hint_type {
            WakeUpHintType::PedalToWake => {
                format!(
                    "Waiting for {} - try pedaling briefly to wake it up",
                    self.name
                )
            }
            WakeUpHintType::CheckBattery => {
                format!(
                    "{} not found - please check the battery or try pedaling",
                    self.name
                )
            }
            WakeUpHintType::MoveSensor => {
                format!(
                    "{} not responding - try moving the pedals or crank arms",
                    self.name
                )
            }
            WakeUpHintType::ExtendedSearch => {
                format!(
                    "Still searching for {} - pedaling may help wake it up",
                    self.name
                )
            }
        }
    }

    /// Get a short hint text for compact display.
    pub fn short_message(&self) -> String {
        match self.hint_type {
            WakeUpHintType::PedalToWake => "Pedal to wake up".to_string(),
            WakeUpHintType::CheckBattery => "Check battery".to_string(),
            WakeUpHintType::MoveSensor => "Move pedals/cranks".to_string(),
            WakeUpHintType::ExtendedSearch => "Pedal to wake".to_string(),
        }
    }

    /// Mark this hint as shown to the user.
    pub fn mark_shown(&mut self) {
        self.shown = true;
    }
}

/// Type of wake-up hint to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeUpHintType {
    /// Standard hint: pedal to wake up the power meter.
    PedalToWake,
    /// Power meter not found - check battery or pedal.
    CheckBattery,
    /// Try moving the sensor (crank-based power meters).
    MoveSensor,
    /// Extended search in progress - pedaling may help.
    ExtendedSearch,
}

impl std::fmt::Display for WakeUpHintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WakeUpHintType::PedalToWake => write!(f, "Pedal to Wake"),
            WakeUpHintType::CheckBattery => write!(f, "Check Battery"),
            WakeUpHintType::MoveSensor => write!(f, "Move Sensor"),
            WakeUpHintType::ExtendedSearch => write!(f, "Extended Search"),
        }
    }
}

/// Configuration for power meter wake-up detection.
#[derive(Debug, Clone)]
pub struct PowerMeterWakeUpConfig {
    /// Delay before showing first wake-up hint (default: 10 seconds).
    pub hint_delay: Duration,
    /// Grace period after discovery ends before showing missing hint (default: 5 seconds).
    pub grace_period: Duration,
    /// Whether wake-up detection is enabled (default: true).
    pub enabled: bool,
    /// Maximum number of hints to generate per discovery session.
    pub max_hints_per_session: usize,
    /// Time between repeated hints for the same device (default: 30 seconds).
    pub hint_repeat_interval: Duration,
}

impl Default for PowerMeterWakeUpConfig {
    fn default() -> Self {
        Self {
            hint_delay: Duration::from_secs(DEFAULT_WAKE_UP_HINT_DELAY_SECS),
            grace_period: Duration::from_secs(DEFAULT_GRACE_PERIOD_SECS),
            enabled: true,
            max_hints_per_session: 5,
            hint_repeat_interval: Duration::from_secs(30),
        }
    }
}

impl PowerMeterWakeUpConfig {
    /// Create a configuration with aggressive hinting (shorter delays).
    pub fn aggressive() -> Self {
        Self {
            hint_delay: Duration::from_secs(5),
            grace_period: Duration::from_secs(3),
            enabled: true,
            max_hints_per_session: 10,
            hint_repeat_interval: Duration::from_secs(20),
        }
    }

    /// Create a configuration with relaxed hinting (longer delays).
    pub fn relaxed() -> Self {
        Self {
            hint_delay: Duration::from_secs(15),
            grace_period: Duration::from_secs(10),
            enabled: true,
            max_hints_per_session: 3,
            hint_repeat_interval: Duration::from_secs(60),
        }
    }

    /// Create a disabled configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// Configuration for extended power meter discovery.
///
/// Power meters often take longer to advertise than other sensors because
/// they enter a deep sleep mode to conserve battery. This configuration
/// allows the discovery process to extend beyond the standard timeout
/// when a saved power meter is expected but not yet found.
#[derive(Debug, Clone)]
pub struct ExtendedPowerMeterDiscoveryConfig {
    /// Whether extended discovery for power meters is enabled (default: true).
    pub enabled: bool,
    /// Standard discovery timeout in seconds (default: 30s).
    pub standard_timeout_secs: u64,
    /// Extended discovery timeout when power meters are expected (default: 45s).
    pub extended_timeout_secs: u64,
    /// Minimum time to wait before extending discovery (default: 15s).
    /// Extended discovery only kicks in after this initial period.
    pub extension_threshold_secs: u64,
}

impl Default for ExtendedPowerMeterDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            standard_timeout_secs: DEFAULT_STANDARD_DISCOVERY_SECS,
            extended_timeout_secs: DEFAULT_EXTENDED_DISCOVERY_SECS,
            extension_threshold_secs: EXTENDED_DISCOVERY_THRESHOLD_SECS,
        }
    }
}

impl ExtendedPowerMeterDiscoveryConfig {
    /// Create a disabled configuration (no extended discovery).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Create a configuration with aggressive extension (longer timeout).
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            standard_timeout_secs: 30,
            extended_timeout_secs: 60,
            extension_threshold_secs: 10,
        }
    }

    /// Create a configuration with minimal extension.
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            standard_timeout_secs: 30,
            extended_timeout_secs: 35,
            extension_threshold_secs: 20,
        }
    }

    /// Get the additional time added by extended discovery.
    pub fn extension_time(&self) -> Duration {
        Duration::from_secs(
            self.extended_timeout_secs.saturating_sub(self.standard_timeout_secs),
        )
    }

    /// Check if extension should be active based on elapsed time.
    pub fn should_extend(&self, elapsed: Duration) -> bool {
        self.enabled && elapsed.as_secs() >= self.extension_threshold_secs
    }
}

/// Result of extended discovery decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtendedDiscoveryDecision {
    /// Use standard discovery timeout.
    UseStandardTimeout,
    /// Extend discovery to find power meters.
    ExtendForPowerMeters {
        /// Names of the power meters we're waiting for.
        waiting_for: Vec<String>,
        /// The extended timeout in seconds.
        extended_timeout_secs: u64,
    },
    /// Extended discovery is disabled.
    Disabled,
}

/// Expected power meter information from cache.
#[derive(Debug, Clone)]
pub struct ExpectedPowerMeter {
    /// Device ID of the expected power meter.
    pub device_id: String,
    /// Name of the expected power meter.
    pub name: String,
    /// Protocol (BLE or ANT+).
    pub protocol: Protocol,
    /// When this expectation was registered.
    pub expected_since: Instant,
    /// Number of hints generated for this device.
    pub hint_count: u32,
    /// When the last hint was generated.
    pub last_hint_at: Option<Instant>,
}

impl ExpectedPowerMeter {
    /// Create a new expected power meter entry.
    pub fn new(device_id: String, name: String, protocol: Protocol) -> Self {
        Self {
            device_id,
            name,
            protocol,
            expected_since: Instant::now(),
            hint_count: 0,
            last_hint_at: None,
        }
    }

    /// Check if enough time has passed to show a hint.
    pub fn can_show_hint(&self, config: &PowerMeterWakeUpConfig) -> bool {
        // Check if we've waited long enough since expectation
        if self.expected_since.elapsed() < config.hint_delay {
            return false;
        }

        // Check if enough time has passed since last hint
        if let Some(last_hint) = self.last_hint_at {
            if last_hint.elapsed() < config.hint_repeat_interval {
                return false;
            }
        }

        true
    }

    /// Record that a hint was generated.
    pub fn record_hint(&mut self) {
        self.hint_count += 1;
        self.last_hint_at = Some(Instant::now());
    }
}

/// Detection result for power meter wake-up checks.
#[derive(Debug, Clone)]
pub struct WakeUpDetectionResult {
    /// Power meters that were expected but not found.
    pub missing: Vec<String>,
    /// Power meters that were expected and found.
    pub found: Vec<String>,
    /// Generated hints for missing power meters.
    pub hints: Vec<WakeUpHint>,
    /// Whether all expected power meters were found.
    pub all_found: bool,
}

impl WakeUpDetectionResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            missing: Vec::new(),
            found: Vec::new(),
            hints: Vec::new(),
            all_found: true,
        }
    }

    /// Check if there are any missing power meters.
    pub fn has_missing(&self) -> bool {
        !self.missing.is_empty()
    }

    /// Check if there are any hints to show.
    pub fn has_hints(&self) -> bool {
        !self.hints.is_empty()
    }

    /// Get the number of missing power meters.
    pub fn missing_count(&self) -> usize {
        self.missing.len()
    }
}

/// Power meter wake-up detector.
///
/// Tracks expected power meters from cache and generates wake-up hints
/// when they are not found during discovery.
#[derive(Debug)]
pub struct PowerMeterWakeUpDetector {
    /// Configuration.
    config: PowerMeterWakeUpConfig,
    /// Extended discovery configuration.
    extended_discovery_config: ExtendedPowerMeterDiscoveryConfig,
    /// Expected power meters (device_id -> ExpectedPowerMeter).
    expected: HashMap<String, ExpectedPowerMeter>,
    /// Device IDs of power meters found during current discovery.
    found: HashSet<String>,
    /// When the current discovery session started.
    discovery_started_at: Option<Instant>,
    /// Whether discovery is currently active.
    discovery_active: bool,
    /// Total hints generated in current session.
    session_hint_count: usize,
    /// Generated hints (device_id -> WakeUpHint).
    hints: HashMap<String, WakeUpHint>,
    /// Whether extended discovery has been triggered this session.
    extended_discovery_triggered: bool,
}

impl PowerMeterWakeUpDetector {
    /// Create a new power meter wake-up detector.
    pub fn new() -> Self {
        Self::with_config(PowerMeterWakeUpConfig::default())
    }

    /// Create a new detector with custom configuration.
    pub fn with_config(config: PowerMeterWakeUpConfig) -> Self {
        Self {
            config,
            extended_discovery_config: ExtendedPowerMeterDiscoveryConfig::default(),
            expected: HashMap::new(),
            found: HashSet::new(),
            discovery_started_at: None,
            discovery_active: false,
            session_hint_count: 0,
            hints: HashMap::new(),
            extended_discovery_triggered: false,
        }
    }

    /// Create a new detector with custom extended discovery configuration.
    pub fn with_extended_config(
        config: PowerMeterWakeUpConfig,
        extended_config: ExtendedPowerMeterDiscoveryConfig,
    ) -> Self {
        Self {
            config,
            extended_discovery_config: extended_config,
            expected: HashMap::new(),
            found: HashSet::new(),
            discovery_started_at: None,
            discovery_active: false,
            session_hint_count: 0,
            hints: HashMap::new(),
            extended_discovery_triggered: false,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &PowerMeterWakeUpConfig {
        &self.config
    }

    /// Get the extended discovery configuration.
    pub fn extended_discovery_config(&self) -> &ExtendedPowerMeterDiscoveryConfig {
        &self.extended_discovery_config
    }

    /// Set the configuration.
    pub fn set_config(&mut self, config: PowerMeterWakeUpConfig) {
        self.config = config;
    }

    /// Set the extended discovery configuration.
    pub fn set_extended_discovery_config(&mut self, config: ExtendedPowerMeterDiscoveryConfig) {
        self.extended_discovery_config = config;
    }

    /// Load expected power meters from sensor cache.
    ///
    /// This should be called at the start of discovery to register
    /// all saved power meters as expected.
    pub fn load_from_cache(&mut self, cache: &SensorCache) {
        let power_meters = cache.sensors_of_type(SensorType::PowerMeter);

        for sensor in power_meters {
            if !sensor.is_stale() {
                self.register_expected(
                    sensor.device_id.clone(),
                    sensor.name.clone(),
                    sensor.protocol,
                );
            }
        }

        tracing::debug!(
            "Loaded {} expected power meters from cache",
            self.expected.len()
        );
    }

    /// Register an expected power meter.
    pub fn register_expected(&mut self, device_id: String, name: String, protocol: Protocol) {
        if !self.expected.contains_key(&device_id) {
            tracing::debug!("Registered expected power meter: {} ({})", name, device_id);
            self.expected.insert(
                device_id.clone(),
                ExpectedPowerMeter::new(device_id, name, protocol),
            );
        }
    }

    /// Start a new discovery session.
    pub fn start_discovery(&mut self) {
        self.found.clear();
        self.hints.clear();
        self.session_hint_count = 0;
        self.discovery_started_at = Some(Instant::now());
        self.discovery_active = true;
        self.extended_discovery_triggered = false;

        // Reset hint counts for all expected power meters
        for expected in self.expected.values_mut() {
            expected.hint_count = 0;
            expected.last_hint_at = None;
            expected.expected_since = Instant::now();
        }

        tracing::debug!(
            "Started discovery session with {} expected power meters",
            self.expected.len()
        );
    }

    /// Stop the discovery session.
    pub fn stop_discovery(&mut self) {
        self.discovery_active = false;
        tracing::debug!(
            "Stopped discovery session: found {}/{} power meters",
            self.found.len(),
            self.expected.len()
        );
    }

    /// Record that a power meter was discovered.
    pub fn record_discovered(&mut self, sensor: &DiscoveredSensor) {
        if sensor.sensor_type == SensorType::PowerMeter {
            self.found.insert(sensor.device_id.clone());

            // Remove any hint for this device since it's now found
            self.hints.remove(&sensor.device_id);

            tracing::debug!(
                "Power meter discovered: {} ({})",
                sensor.name,
                sensor.device_id
            );
        }
    }

    /// Check if a specific power meter is expected but not found.
    pub fn is_missing(&self, device_id: &str) -> bool {
        self.expected.contains_key(device_id) && !self.found.contains(device_id)
    }

    /// Get the list of missing power meter device IDs.
    pub fn get_missing(&self) -> Vec<&str> {
        self.expected
            .keys()
            .filter(|id| !self.found.contains(*id))
            .map(|s| s.as_str())
            .collect()
    }

    /// Get the number of expected power meters.
    pub fn expected_count(&self) -> usize {
        self.expected.len()
    }

    /// Get the number of found power meters.
    pub fn found_count(&self) -> usize {
        self.found.len()
    }

    /// Get the number of missing power meters.
    pub fn missing_count(&self) -> usize {
        self.expected.len().saturating_sub(self.found.len())
    }

    /// Check if all expected power meters have been found.
    pub fn all_found(&self) -> bool {
        self.expected.keys().all(|id| self.found.contains(id))
    }

    /// Check if we have any expected power meters.
    pub fn has_expected(&self) -> bool {
        !self.expected.is_empty()
    }

    /// Check for wake-up hints and generate them if needed.
    ///
    /// Returns hints for missing power meters that haven't been shown yet.
    /// This should be called periodically during discovery.
    pub fn check_for_hints(&mut self) -> Vec<WakeUpHint> {
        if !self.config.enabled || !self.discovery_active {
            return Vec::new();
        }

        // Check if we've hit the session limit
        if self.session_hint_count >= self.config.max_hints_per_session {
            return Vec::new();
        }

        let mut new_hints = Vec::new();

        for (device_id, expected) in self.expected.iter_mut() {
            // Skip if already found
            if self.found.contains(device_id) {
                continue;
            }

            // Skip if we can't show a hint yet
            if !expected.can_show_hint(&self.config) {
                continue;
            }

            // Skip if we already have an unshown hint for this device
            if let Some(hint) = self.hints.get(device_id) {
                if !hint.shown {
                    continue;
                }
            }

            // Determine hint type based on context
            let hint_type = if expected.hint_count == 0 {
                WakeUpHintType::PedalToWake
            } else if expected.hint_count < 3 {
                WakeUpHintType::ExtendedSearch
            } else {
                WakeUpHintType::CheckBattery
            };

            let hint = WakeUpHint::new(
                device_id.clone(),
                expected.name.clone(),
                expected.protocol,
                hint_type,
            );

            expected.record_hint();
            new_hints.push(hint.clone());
            self.hints.insert(device_id.clone(), hint);
        }

        self.session_hint_count += new_hints.len();
        new_hints
    }

    /// Get all current hints.
    pub fn get_hints(&self) -> Vec<&WakeUpHint> {
        self.hints.values().collect()
    }

    /// Get unshown hints.
    pub fn get_unshown_hints(&self) -> Vec<&WakeUpHint> {
        self.hints.values().filter(|h| !h.shown).collect()
    }

    /// Mark a hint as shown.
    pub fn mark_hint_shown(&mut self, device_id: &str) {
        if let Some(hint) = self.hints.get_mut(device_id) {
            hint.mark_shown();
        }
    }

    /// Mark all hints as shown.
    pub fn mark_all_hints_shown(&mut self) {
        for hint in self.hints.values_mut() {
            hint.shown = true;
        }
    }

    /// Get a full detection result with all state.
    pub fn get_detection_result(&self) -> WakeUpDetectionResult {
        let missing: Vec<String> = self
            .expected
            .keys()
            .filter(|id| !self.found.contains(*id))
            .cloned()
            .collect();

        let found: Vec<String> = self
            .expected
            .keys()
            .filter(|id| self.found.contains(*id))
            .cloned()
            .collect();

        let hints: Vec<WakeUpHint> = self.hints.values().cloned().collect();

        let all_found = missing.is_empty();

        WakeUpDetectionResult {
            missing,
            found,
            hints,
            all_found,
        }
    }

    /// Check if discovery is currently active.
    pub fn is_discovery_active(&self) -> bool {
        self.discovery_active
    }

    /// Get time since discovery started.
    pub fn discovery_elapsed(&self) -> Option<Duration> {
        self.discovery_started_at.map(|t| t.elapsed())
    }

    /// Check if extended discovery should be used.
    ///
    /// Returns true if:
    /// - Extended discovery is enabled
    /// - There are expected power meters that haven't been found
    /// - Enough time has elapsed (past the extension threshold)
    ///
    /// This should be called during the progressive timeout logic to determine
    /// whether to extend the discovery period beyond the standard timeout.
    pub fn should_use_extended_discovery(&self) -> bool {
        // Extended discovery must be enabled
        if !self.extended_discovery_config.enabled {
            return false;
        }

        // Must have expected power meters that aren't found
        if self.expected.is_empty() || self.all_found() {
            return false;
        }

        // Must be past the extension threshold
        if let Some(elapsed) = self.discovery_elapsed() {
            self.extended_discovery_config.should_extend(elapsed)
        } else {
            false
        }
    }

    /// Get the recommended discovery timeout based on current state.
    ///
    /// Returns the extended timeout if power meters are expected and not found,
    /// otherwise returns the standard timeout.
    pub fn get_recommended_timeout_secs(&self) -> u64 {
        if self.extended_discovery_config.enabled && self.has_expected() && !self.all_found() {
            self.extended_discovery_config.extended_timeout_secs
        } else {
            self.extended_discovery_config.standard_timeout_secs
        }
    }

    /// Get the extended discovery decision for the current state.
    ///
    /// This provides detailed information about whether extended discovery
    /// should be used and which power meters we're waiting for.
    pub fn get_extended_discovery_decision(&self) -> ExtendedDiscoveryDecision {
        if !self.extended_discovery_config.enabled {
            return ExtendedDiscoveryDecision::Disabled;
        }

        if self.expected.is_empty() || self.all_found() {
            return ExtendedDiscoveryDecision::UseStandardTimeout;
        }

        // Collect names of missing power meters
        let waiting_for: Vec<String> = self
            .expected
            .iter()
            .filter(|(id, _)| !self.found.contains(*id))
            .map(|(_, pm)| pm.name.clone())
            .collect();

        if waiting_for.is_empty() {
            return ExtendedDiscoveryDecision::UseStandardTimeout;
        }

        ExtendedDiscoveryDecision::ExtendForPowerMeters {
            waiting_for,
            extended_timeout_secs: self.extended_discovery_config.extended_timeout_secs,
        }
    }

    /// Mark that extended discovery has been triggered.
    ///
    /// This is called when the discovery process extends beyond the standard
    /// timeout specifically to find power meters.
    pub fn mark_extended_discovery_triggered(&mut self) {
        if !self.extended_discovery_triggered {
            self.extended_discovery_triggered = true;
            tracing::info!(
                "Extended discovery triggered for {} missing power meter(s)",
                self.missing_count()
            );
        }
    }

    /// Check if extended discovery has been triggered this session.
    pub fn is_extended_discovery_triggered(&self) -> bool {
        self.extended_discovery_triggered
    }

    /// Get names of power meters we're waiting for (missing).
    pub fn get_missing_power_meter_names(&self) -> Vec<String> {
        self.expected
            .iter()
            .filter(|(id, _)| !self.found.contains(*id))
            .map(|(_, pm)| pm.name.clone())
            .collect()
    }

    /// Clear all state and expectations.
    pub fn clear(&mut self) {
        self.expected.clear();
        self.found.clear();
        self.hints.clear();
        self.session_hint_count = 0;
        self.discovery_started_at = None;
        self.discovery_active = false;
        self.extended_discovery_triggered = false;
    }

    /// Clear found sensors (for retrying discovery).
    pub fn clear_found(&mut self) {
        self.found.clear();
        self.hints.clear();
        self.session_hint_count = 0;
    }
}

impl Default for PowerMeterWakeUpDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to check if a sensor type is a power meter or has power data.
pub fn provides_power_data(sensor_type: SensorType) -> bool {
    matches!(
        sensor_type,
        SensorType::PowerMeter | SensorType::Trainer | SensorType::SmartTrainer
    )
}

/// Helper function to check if a protocol is for power measurement.
pub fn is_power_protocol(protocol: Protocol) -> bool {
    matches!(
        protocol,
        Protocol::BleCyclingPower | Protocol::AntPower | Protocol::BleFtms | Protocol::AntFec
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_up_hint_new() {
        let hint = WakeUpHint::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
            WakeUpHintType::PedalToWake,
        );

        assert_eq!(hint.device_id, "device1");
        assert_eq!(hint.name, "Stages Power");
        assert_eq!(hint.protocol, Protocol::BleCyclingPower);
        assert_eq!(hint.hint_type, WakeUpHintType::PedalToWake);
        assert!(!hint.shown);
    }

    #[test]
    fn test_wake_up_hint_messages() {
        let hint = WakeUpHint::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
            WakeUpHintType::PedalToWake,
        );
        assert!(hint.message().contains("Stages Power"));
        assert!(hint.message().contains("pedaling"));

        let hint2 = WakeUpHint::new(
            "device2".to_string(),
            "Quarq".to_string(),
            Protocol::AntPower,
            WakeUpHintType::CheckBattery,
        );
        assert!(hint2.message().contains("battery"));
    }

    #[test]
    fn test_wake_up_hint_mark_shown() {
        let mut hint = WakeUpHint::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
            WakeUpHintType::PedalToWake,
        );

        assert!(!hint.shown);
        hint.mark_shown();
        assert!(hint.shown);
    }

    #[test]
    fn test_config_default() {
        let config = PowerMeterWakeUpConfig::default();
        assert_eq!(config.hint_delay, Duration::from_secs(10));
        assert_eq!(config.grace_period, Duration::from_secs(5));
        assert!(config.enabled);
    }

    #[test]
    fn test_config_presets() {
        let aggressive = PowerMeterWakeUpConfig::aggressive();
        assert!(aggressive.hint_delay < Duration::from_secs(10));

        let relaxed = PowerMeterWakeUpConfig::relaxed();
        assert!(relaxed.hint_delay > Duration::from_secs(10));

        let disabled = PowerMeterWakeUpConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_expected_power_meter() {
        let expected = ExpectedPowerMeter::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert_eq!(expected.device_id, "device1");
        assert_eq!(expected.name, "Stages Power");
        assert_eq!(expected.hint_count, 0);
        assert!(expected.last_hint_at.is_none());
    }

    #[test]
    fn test_detector_new() {
        let detector = PowerMeterWakeUpDetector::new();
        assert_eq!(detector.expected_count(), 0);
        assert_eq!(detector.found_count(), 0);
        assert!(!detector.is_discovery_active());
    }

    #[test]
    fn test_detector_register_expected() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.register_expected(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert_eq!(detector.expected_count(), 1);
        assert!(detector.has_expected());
    }

    #[test]
    fn test_detector_start_stop_discovery() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.start_discovery();
        assert!(detector.is_discovery_active());
        assert!(detector.discovery_elapsed().is_some());

        detector.stop_discovery();
        assert!(!detector.is_discovery_active());
    }

    #[test]
    fn test_detector_record_discovered() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.register_expected(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        detector.start_discovery();

        let discovered = DiscoveredSensor {
            device_id: "device1".to_string(),
            name: "Stages Power".to_string(),
            sensor_type: SensorType::PowerMeter,
            protocol: Protocol::BleCyclingPower,
            signal_strength: Some(-70),
            last_seen: Instant::now(),
        };

        detector.record_discovered(&discovered);

        assert_eq!(detector.found_count(), 1);
        assert!(detector.all_found());
        assert!(!detector.is_missing("device1"));
    }

    #[test]
    fn test_detector_missing_detection() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.register_expected(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );
        detector.register_expected(
            "device2".to_string(),
            "Quarq".to_string(),
            Protocol::AntPower,
        );

        detector.start_discovery();

        // Only find one of the two
        let discovered = DiscoveredSensor {
            device_id: "device1".to_string(),
            name: "Stages Power".to_string(),
            sensor_type: SensorType::PowerMeter,
            protocol: Protocol::BleCyclingPower,
            signal_strength: Some(-70),
            last_seen: Instant::now(),
        };
        detector.record_discovered(&discovered);

        assert!(!detector.all_found());
        assert_eq!(detector.missing_count(), 1);
        assert!(detector.is_missing("device2"));
        assert!(!detector.is_missing("device1"));

        let missing = detector.get_missing();
        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&"device2"));
    }

    #[test]
    fn test_detector_detection_result() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.register_expected(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        detector.start_discovery();

        let result = detector.get_detection_result();
        assert_eq!(result.missing.len(), 1);
        assert!(result.found.is_empty());
        assert!(!result.all_found);
        assert!(result.has_missing());
    }

    #[test]
    fn test_detector_clear() {
        let mut detector = PowerMeterWakeUpDetector::new();

        detector.register_expected(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );
        detector.start_discovery();

        detector.clear();

        assert_eq!(detector.expected_count(), 0);
        assert!(!detector.is_discovery_active());
    }

    #[test]
    fn test_provides_power_data() {
        assert!(provides_power_data(SensorType::PowerMeter));
        assert!(provides_power_data(SensorType::Trainer));
        assert!(provides_power_data(SensorType::SmartTrainer));
        assert!(!provides_power_data(SensorType::HeartRate));
        assert!(!provides_power_data(SensorType::Cadence));
    }

    #[test]
    fn test_is_power_protocol() {
        assert!(is_power_protocol(Protocol::BleCyclingPower));
        assert!(is_power_protocol(Protocol::AntPower));
        assert!(is_power_protocol(Protocol::BleFtms));
        assert!(is_power_protocol(Protocol::AntFec));
        assert!(!is_power_protocol(Protocol::BleHeartRate));
        assert!(!is_power_protocol(Protocol::AntHeartRate));
    }
}
