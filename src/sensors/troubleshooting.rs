//! Sensor troubleshooting documentation and in-app tips.
//!
//! This module provides:
//! - Documentation of common sensor connection issues and resolutions
//! - Contextual troubleshooting tips based on detected issues
//! - Guided troubleshooting flows for users
//!
//! # Common Connection Issues
//!
//! ## Signal Quality Issues
//! - **Weak signal**: Sensor too far from computer or obstructions in path
//! - **Intermittent drops**: Interference from other devices or physical obstacles
//! - **No signal at all**: Bluetooth/ANT+ not enabled or sensor not powered on
//!
//! ## Discovery Issues
//! - **Sensor not found**: Sensor in sleep mode, needs wake-up (pedal/move)
//! - **Slow discovery**: Too many BLE devices nearby causing congestion
//! - **Wrong sensor type detected**: Firmware issue or multi-protocol confusion
//!
//! ## Connection Issues
//! - **Connection refused**: Another app already connected to sensor
//! - **Connection drops during ride**: Battery issues or interference
//! - **Cannot reconnect**: Sensor needs power cycle or cache cleared
//!
//! ## Power Meter Specific Issues
//! - **Power meter not waking**: Needs pedaling to exit sleep mode
//! - **Inaccurate readings**: Needs calibration (zero-offset)
//! - **Crank detection issues**: Magnet alignment or battery low
//!
//! ## ANT+ Specific Issues
//! - **No ANT+ dongle detected**: USB dongle not plugged in or drivers missing
//! - **ANT+ interference**: Multiple ANT+ devices on same channel
//! - **Slow ANT+ connection**: Dongle antenna orientation

use crate::sensors::health::HealthStatus;
use crate::sensors::quality::QualityLevel;
use crate::sensors::types::{Protocol, SensorType};

/// Category of troubleshooting issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TroubleshootingCategory {
    /// Signal strength and quality issues.
    Signal,
    /// Connection establishment and maintenance issues.
    Connection,
    /// Device discovery issues.
    Discovery,
    /// Power meter specific issues.
    PowerMeter,
    /// Battery and power issues.
    Battery,
    /// Protocol-specific issues (BLE vs ANT+).
    Protocol,
    /// Interference and environmental issues.
    Interference,
    /// Calibration-related issues.
    Calibration,
    /// General/other issues.
    General,
}

impl std::fmt::Display for TroubleshootingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TroubleshootingCategory::Signal => write!(f, "Signal Quality"),
            TroubleshootingCategory::Connection => write!(f, "Connection"),
            TroubleshootingCategory::Discovery => write!(f, "Discovery"),
            TroubleshootingCategory::PowerMeter => write!(f, "Power Meter"),
            TroubleshootingCategory::Battery => write!(f, "Battery"),
            TroubleshootingCategory::Protocol => write!(f, "Protocol"),
            TroubleshootingCategory::Interference => write!(f, "Interference"),
            TroubleshootingCategory::Calibration => write!(f, "Calibration"),
            TroubleshootingCategory::General => write!(f, "General"),
        }
    }
}

/// Priority level for troubleshooting tips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TipPriority {
    /// Low priority - informational tips.
    Low,
    /// Medium priority - common issues.
    Medium,
    /// High priority - likely cause of current issue.
    High,
    /// Critical - action required immediately.
    Critical,
}

impl std::fmt::Display for TipPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TipPriority::Low => write!(f, "Tip"),
            TipPriority::Medium => write!(f, "Suggestion"),
            TipPriority::High => write!(f, "Recommended"),
            TipPriority::Critical => write!(f, "Important"),
        }
    }
}

/// A single troubleshooting tip.
#[derive(Debug, Clone)]
pub struct TroubleshootingTip {
    /// Category of the issue.
    pub category: TroubleshootingCategory,
    /// Priority of this tip.
    pub priority: TipPriority,
    /// Short title for the tip.
    pub title: String,
    /// Detailed description of the issue.
    pub issue: String,
    /// Step-by-step resolution instructions.
    pub resolution: Vec<String>,
    /// Icon to display with the tip.
    pub icon: &'static str,
    /// Whether this tip has been dismissed by the user.
    pub dismissed: bool,
    /// Optional link to more documentation.
    pub help_url: Option<String>,
}

impl TroubleshootingTip {
    /// Create a new troubleshooting tip.
    pub fn new(
        category: TroubleshootingCategory,
        priority: TipPriority,
        title: impl Into<String>,
        issue: impl Into<String>,
        resolution: Vec<impl Into<String>>,
    ) -> Self {
        Self {
            category,
            priority,
            title: title.into(),
            issue: issue.into(),
            resolution: resolution.into_iter().map(|s| s.into()).collect(),
            icon: Self::icon_for_category(category),
            dismissed: false,
            help_url: None,
        }
    }

    /// Get the icon for a category.
    fn icon_for_category(category: TroubleshootingCategory) -> &'static str {
        match category {
            TroubleshootingCategory::Signal => "📶",
            TroubleshootingCategory::Connection => "🔗",
            TroubleshootingCategory::Discovery => "🔍",
            TroubleshootingCategory::PowerMeter => "⚡",
            TroubleshootingCategory::Battery => "🔋",
            TroubleshootingCategory::Protocol => "📡",
            TroubleshootingCategory::Interference => "📻",
            TroubleshootingCategory::Calibration => "⚙️",
            TroubleshootingCategory::General => "ℹ️",
        }
    }

    /// Get a short summary of the resolution (first step).
    pub fn short_resolution(&self) -> &str {
        self.resolution
            .first()
            .map(|s| s.as_str())
            .unwrap_or("See details")
    }

    /// Get the full resolution as a formatted string.
    pub fn full_resolution(&self) -> String {
        self.resolution
            .iter()
            .enumerate()
            .map(|(i, step)| format!("{}. {}", i + 1, step))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Mark this tip as dismissed.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Check if this tip is relevant for the given sensor type.
    pub fn is_relevant_for(&self, sensor_type: SensorType) -> bool {
        match self.category {
            TroubleshootingCategory::PowerMeter => matches!(
                sensor_type,
                SensorType::PowerMeter | SensorType::Trainer | SensorType::SmartTrainer
            ),
            TroubleshootingCategory::Calibration => matches!(
                sensor_type,
                SensorType::PowerMeter | SensorType::Trainer | SensorType::SmartTrainer
            ),
            _ => true,
        }
    }
}

/// Detected issue requiring troubleshooting.
#[derive(Debug, Clone)]
pub struct DetectedIssue {
    /// Category of the issue.
    pub category: TroubleshootingCategory,
    /// Description of what was detected.
    pub description: String,
    /// Device ID if applicable.
    pub device_id: Option<String>,
    /// Sensor name if applicable.
    pub sensor_name: Option<String>,
    /// Relevant data (RSSI, quality score, etc.).
    pub data: Option<String>,
}

impl DetectedIssue {
    /// Create a new detected issue.
    pub fn new(category: TroubleshootingCategory, description: impl Into<String>) -> Self {
        Self {
            category,
            description: description.into(),
            device_id: None,
            sensor_name: None,
            data: None,
        }
    }

    /// Set the device ID.
    pub fn with_device(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// Set the sensor name.
    pub fn with_sensor_name(mut self, name: impl Into<String>) -> Self {
        self.sensor_name = Some(name.into());
        self
    }

    /// Set additional data.
    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }
}

/// Collection of troubleshooting tips for common issues.
#[derive(Debug, Clone, Default)]
pub struct TroubleshootingGuide {
    tips: Vec<TroubleshootingTip>,
}

impl TroubleshootingGuide {
    /// Create a new troubleshooting guide with all standard tips.
    pub fn new() -> Self {
        let mut guide = Self { tips: Vec::new() };
        guide.add_standard_tips();
        guide
    }

    /// Add all standard troubleshooting tips.
    fn add_standard_tips(&mut self) {
        // Signal Quality Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::High,
            "Weak Signal Detected",
            "The sensor signal is weak which may cause data dropouts or disconnections.",
            vec![
                "Move closer to the sensor (within 3 meters / 10 feet)",
                "Remove obstacles between you and the sensor",
                "Reduce interference from other electronic devices",
                "Check sensor battery level - low battery affects signal strength",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::Medium,
            "Intermittent Signal",
            "Signal quality is fluctuating, which may indicate environmental interference.",
            vec![
                "Move away from WiFi routers, microwaves, or other 2.4GHz devices",
                "Try switching from BLE to ANT+ (or vice versa) if available",
                "Ensure sensor is securely attached with good antenna orientation",
            ],
        ));

        // Discovery Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Discovery,
            TipPriority::Medium,
            "Sensor Not Found",
            "The expected sensor was not discovered during scanning.",
            vec![
                "Ensure the sensor is powered on and has fresh batteries",
                "For power meters and trainers, pedal briefly to wake from sleep mode",
                "Check that no other app or device is connected to the sensor",
                "Try restarting the sensor by removing and reinserting the battery",
                "Move closer and try scanning again",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Discovery,
            TipPriority::Low,
            "Slow Discovery",
            "Sensor discovery is taking longer than usual.",
            vec![
                "Many Bluetooth devices nearby can slow discovery",
                "Try closing other Bluetooth apps on your device",
                "ANT+ typically discovers faster than BLE in crowded environments",
            ],
        ));

        // Connection Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Connection,
            TipPriority::High,
            "Connection Failed",
            "Failed to establish connection with the sensor.",
            vec![
                "Check if another app is already connected to this sensor",
                "Try restarting the sensor (power cycle or remove battery)",
                "Restart Bluetooth on your computer",
                "Try using a different protocol (BLE/ANT+) if available",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Connection,
            TipPriority::Medium,
            "Frequent Disconnections",
            "Sensor keeps disconnecting during use.",
            vec![
                "Check battery level - replace if below 20%",
                "Reduce distance to the sensor",
                "Remove sources of interference",
                "Try switching protocols (BLE to ANT+ or vice versa)",
                "Check for firmware updates for your sensor",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Connection,
            TipPriority::Low,
            "Connection Taking Long",
            "Connection is slow to establish.",
            vec![
                "First connection to a new sensor may take longer",
                "Subsequent connections will be faster due to caching",
                "Keep the sensor nearby during initial pairing",
            ],
        ));

        // Power Meter Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::PowerMeter,
            TipPriority::High,
            "Power Meter Not Waking",
            "Power meter is in sleep mode and not broadcasting.",
            vec![
                "Pedal the cranks briefly (2-3 rotations) to wake up the power meter",
                "Some power meters need sustained rotation before waking",
                "If still not appearing, check the battery",
                "Wait up to 30 seconds while pedaling slowly",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::PowerMeter,
            TipPriority::Medium,
            "Inaccurate Power Readings",
            "Power values seem incorrect or unstable.",
            vec![
                "Perform a zero-offset calibration before your ride",
                "Ensure the power meter is at operating temperature",
                "Check that crank is tightened to correct torque spec",
                "Avoid calibrating if cranks were recently moved or adjusted",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::PowerMeter,
            TipPriority::Low,
            "Delayed Power Data",
            "Power readings appear with a slight delay.",
            vec![
                "Some delay (100-500ms) is normal for power meters",
                "BLE may have slightly higher latency than ANT+",
                "Ensure sensor firmware is up to date",
            ],
        ));

        // Battery Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Battery,
            TipPriority::Critical,
            "Low Battery Warning",
            "Sensor battery is critically low.",
            vec![
                "Replace the battery as soon as possible",
                "Use a high-quality brand-name CR2032 battery (or as specified)",
                "Low battery can cause connection issues and inaccurate readings",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Battery,
            TipPriority::Medium,
            "Battery Check Recommended",
            "Battery level is getting low but not critical.",
            vec![
                "Consider replacing the battery before your next long ride",
                "Keep a spare battery on hand",
                "Low battery affects signal strength first, then data accuracy",
            ],
        ));

        // Protocol Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Protocol,
            TipPriority::Medium,
            "ANT+ Dongle Not Detected",
            "No ANT+ USB dongle was found.",
            vec![
                "Plug in an ANT+ USB dongle to use ANT+ sensors",
                "Try a different USB port (prefer USB 2.0 ports)",
                "Use a USB extension to move dongle away from interference",
                "Check device manager for driver issues (Windows)",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Protocol,
            TipPriority::Low,
            "BLE vs ANT+ Selection",
            "Sensor available on both BLE and ANT+.",
            vec![
                "ANT+ typically has lower latency and better multi-device support",
                "BLE is more widely supported and doesn't require a dongle",
                "Try both and see which works better in your environment",
                "Your preference will be saved for future connections",
            ],
        ));

        // Interference Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Interference,
            TipPriority::Medium,
            "Potential Interference Detected",
            "Signal characteristics suggest radio interference.",
            vec![
                "Move WiFi routers away from your training area",
                "Turn off or move other 2.4GHz devices (baby monitors, wireless speakers)",
                "USB 3.0 ports can interfere with 2.4GHz - use USB 2.0 for ANT+",
                "Consider using a USB extension to position ANT+ dongle better",
            ],
        ));

        // Calibration Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Calibration,
            TipPriority::Medium,
            "Calibration Recommended",
            "It's been a while since this power meter was calibrated.",
            vec![
                "Perform a zero-offset calibration for accurate power readings",
                "Calibrate after the power meter has warmed up (5-10 min riding)",
                "Keep cranks stationary and vertical during calibration",
                "Calibrate more often in varying temperature conditions",
            ],
        ));

        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::Calibration,
            TipPriority::Low,
            "Calibration Best Practices",
            "Tips for getting the most accurate calibration.",
            vec![
                "Calibrate at the start of each ride for best accuracy",
                "Don't stand on pedals or apply pressure during calibration",
                "Temperature changes affect calibration - recalibrate if temp varies",
                "A good calibration takes about 5 seconds",
            ],
        ));

        // General Tips
        self.tips.push(TroubleshootingTip::new(
            TroubleshootingCategory::General,
            TipPriority::Low,
            "First Time Setup",
            "Tips for setting up sensors for the first time.",
            vec![
                "Allow extra time for first-time discovery (up to 45 seconds)",
                "Have sensors nearby during initial setup",
                "Connection times improve after first successful pairing",
                "Your sensor preferences will be remembered for next time",
            ],
        ));
    }

    /// Get all tips.
    pub fn all_tips(&self) -> &[TroubleshootingTip] {
        &self.tips
    }

    /// Get tips for a specific category.
    pub fn tips_for_category(&self, category: TroubleshootingCategory) -> Vec<&TroubleshootingTip> {
        self.tips.iter().filter(|t| t.category == category).collect()
    }

    /// Get tips at or above a priority level.
    pub fn tips_at_priority(&self, min_priority: TipPriority) -> Vec<&TroubleshootingTip> {
        self.tips
            .iter()
            .filter(|t| t.priority >= min_priority)
            .collect()
    }

    /// Get tips relevant for a sensor type.
    pub fn tips_for_sensor_type(&self, sensor_type: SensorType) -> Vec<&TroubleshootingTip> {
        self.tips
            .iter()
            .filter(|t| t.is_relevant_for(sensor_type))
            .collect()
    }

    /// Get a tip by title.
    pub fn get_tip(&self, title: &str) -> Option<&TroubleshootingTip> {
        self.tips.iter().find(|t| t.title == title)
    }
}

/// Context-aware issue detector that generates relevant tips.
#[derive(Debug, Default)]
pub struct IssueDetector {
    /// The troubleshooting guide.
    guide: TroubleshootingGuide,
    /// Currently detected issues.
    detected_issues: Vec<DetectedIssue>,
}

impl IssueDetector {
    /// Create a new issue detector.
    pub fn new() -> Self {
        Self {
            guide: TroubleshootingGuide::new(),
            detected_issues: Vec::new(),
        }
    }

    /// Get the troubleshooting guide.
    pub fn guide(&self) -> &TroubleshootingGuide {
        &self.guide
    }

    /// Clear all detected issues.
    pub fn clear(&mut self) {
        self.detected_issues.clear();
    }

    /// Record an issue for signal quality.
    pub fn record_quality_issue(
        &mut self,
        device_id: &str,
        sensor_name: &str,
        quality: QualityLevel,
        rssi: Option<i16>,
    ) {
        let issue = match quality {
            QualityLevel::Poor => Some(DetectedIssue::new(
                TroubleshootingCategory::Signal,
                format!("{} has poor signal quality", sensor_name),
            )),
            QualityLevel::Fair => Some(DetectedIssue::new(
                TroubleshootingCategory::Signal,
                format!("{} has weak signal", sensor_name),
            )),
            _ => None,
        };

        if let Some(mut issue) = issue {
            issue = issue.with_device(device_id).with_sensor_name(sensor_name);
            if let Some(rssi) = rssi {
                issue = issue.with_data(format!("RSSI: {} dBm", rssi));
            }
            self.detected_issues.push(issue);
        }
    }

    /// Record an issue for connection health.
    pub fn record_health_issue(&mut self, device_id: &str, sensor_name: &str, status: HealthStatus) {
        let issue = match status {
            HealthStatus::Stale => Some(DetectedIssue::new(
                TroubleshootingCategory::Connection,
                format!("{} has stopped responding", sensor_name),
            )),
            HealthStatus::Degraded => Some(DetectedIssue::new(
                TroubleshootingCategory::Connection,
                format!("{} data rate is degraded", sensor_name),
            )),
            _ => None,
        };

        if let Some(issue) = issue {
            self.detected_issues.push(
                issue.with_device(device_id).with_sensor_name(sensor_name),
            );
        }
    }

    /// Record a discovery issue for missing sensors.
    pub fn record_discovery_issue(&mut self, sensor_name: &str, sensor_type: SensorType) {
        let category = if matches!(
            sensor_type,
            SensorType::PowerMeter | SensorType::Trainer | SensorType::SmartTrainer
        ) {
            TroubleshootingCategory::PowerMeter
        } else {
            TroubleshootingCategory::Discovery
        };

        let issue = DetectedIssue::new(category, format!("{} not found during discovery", sensor_name))
            .with_sensor_name(sensor_name);
        self.detected_issues.push(issue);
    }

    /// Record a battery issue.
    pub fn record_battery_issue(&mut self, device_id: &str, sensor_name: &str, level: u8) {
        let (category, description) = if level <= 10 {
            (
                TroubleshootingCategory::Battery,
                format!("{} battery critically low ({}%)", sensor_name, level),
            )
        } else if level <= 20 {
            (
                TroubleshootingCategory::Battery,
                format!("{} battery low ({}%)", sensor_name, level),
            )
        } else {
            return;
        };

        let issue = DetectedIssue::new(category, description)
            .with_device(device_id)
            .with_sensor_name(sensor_name)
            .with_data(format!("Battery: {}%", level));
        self.detected_issues.push(issue);
    }

    /// Record an ANT+ dongle issue.
    pub fn record_ant_dongle_missing(&mut self) {
        let issue = DetectedIssue::new(
            TroubleshootingCategory::Protocol,
            "ANT+ USB dongle not detected",
        );
        self.detected_issues.push(issue);
    }

    /// Get all currently detected issues.
    pub fn get_issues(&self) -> &[DetectedIssue] {
        &self.detected_issues
    }

    /// Check if there are any issues detected.
    pub fn has_issues(&self) -> bool {
        !self.detected_issues.is_empty()
    }

    /// Get the count of detected issues.
    pub fn issue_count(&self) -> usize {
        self.detected_issues.len()
    }

    /// Get relevant tips for the currently detected issues.
    pub fn get_relevant_tips(&self) -> Vec<&TroubleshootingTip> {
        let categories: std::collections::HashSet<_> = self
            .detected_issues
            .iter()
            .map(|i| i.category)
            .collect();

        self.guide
            .all_tips()
            .iter()
            .filter(|t| categories.contains(&t.category))
            .collect()
    }

    /// Get the most important tip based on current issues.
    pub fn get_primary_tip(&self) -> Option<&TroubleshootingTip> {
        let relevant = self.get_relevant_tips();

        // Return highest priority tip
        relevant
            .into_iter()
            .max_by_key(|t| t.priority)
    }

    /// Generate contextual tips based on detected issues.
    pub fn generate_contextual_tips(&self) -> Vec<ContextualTip> {
        let mut tips = Vec::new();

        for issue in &self.detected_issues {
            let guide_tips = self.guide.tips_for_category(issue.category);
            for tip in guide_tips {
                tips.push(ContextualTip {
                    tip: tip.clone(),
                    issue: issue.clone(),
                    context: self.format_context(&issue),
                });
            }
        }

        // Sort by priority (highest first) and deduplicate by title
        tips.sort_by(|a, b| b.tip.priority.cmp(&a.tip.priority));

        // Deduplicate by tip title
        let mut seen_titles = std::collections::HashSet::new();
        tips.retain(|t| seen_titles.insert(t.tip.title.clone()));

        tips
    }

    /// Format context string for an issue.
    fn format_context(&self, issue: &DetectedIssue) -> String {
        let mut parts = Vec::new();

        if let Some(name) = &issue.sensor_name {
            parts.push(format!("Sensor: {}", name));
        }
        if let Some(data) = &issue.data {
            parts.push(data.clone());
        }

        if parts.is_empty() {
            issue.description.clone()
        } else {
            format!("{} ({})", issue.description, parts.join(", "))
        }
    }
}

/// A tip with context about why it's being shown.
#[derive(Debug, Clone)]
pub struct ContextualTip {
    /// The troubleshooting tip.
    pub tip: TroubleshootingTip,
    /// The issue that triggered this tip.
    pub issue: DetectedIssue,
    /// Formatted context string.
    pub context: String,
}

/// Quick troubleshooting tips for display when no sensors are found.
pub fn get_no_sensors_tips() -> Vec<&'static str> {
    vec![
        "Make sure Bluetooth is enabled on your device",
        "Ensure your trainer/sensors are powered on",
        "Keep sensors within 10 meters of your computer",
        "Wake up sensors by moving/pedaling",
        "Check that no other app is connected to the sensor",
        "Try restarting the sensor if it won't appear",
    ]
}

/// Quick troubleshooting tips for poor signal quality.
pub fn get_poor_signal_tips() -> Vec<&'static str> {
    vec![
        "Move closer to the sensor or reduce obstacles",
        "Check sensor battery - low battery affects signal",
        "Reduce interference from WiFi routers and other 2.4GHz devices",
        "Try switching between BLE and ANT+ if available",
    ]
}

/// Quick troubleshooting tips for connection drops.
pub fn get_connection_drop_tips() -> Vec<&'static str> {
    vec![
        "Check sensor battery level",
        "Move away from interference sources",
        "Try a different USB port for ANT+ dongle",
        "Ensure sensor is securely attached",
    ]
}

/// Quick troubleshooting tips for power meters.
pub fn get_power_meter_tips() -> Vec<&'static str> {
    vec![
        "Pedal briefly (2-3 rotations) to wake from sleep mode",
        "Power meters may take up to 30 seconds to appear",
        "Check battery if power meter doesn't wake up",
        "Perform zero-offset calibration for accurate readings",
    ]
}

/// Quick troubleshooting tips for ANT+ issues.
pub fn get_ant_plus_tips() -> Vec<&'static str> {
    vec![
        "Ensure ANT+ USB dongle is plugged in",
        "Try a different USB port (prefer USB 2.0)",
        "Use a USB extension to move dongle away from interference",
        "Check device manager for driver issues (Windows)",
    ]
}

/// Quick tips for calibration reminders.
pub fn get_calibration_tips() -> Vec<&'static str> {
    vec![
        "Warm up the power meter first (5-10 min riding)",
        "Keep cranks stationary and vertical during calibration",
        "Don't apply any pressure to pedals during calibration",
        "Recalibrate if temperature changes significantly",
    ]
}

/// Get tips for a specific protocol.
pub fn get_protocol_specific_tips(protocol: Protocol) -> Vec<&'static str> {
    match protocol {
        Protocol::BleFtms | Protocol::BleCyclingPower | Protocol::BleHeartRate | Protocol::BleCsc => {
            vec![
                "Ensure Bluetooth is enabled",
                "Close other Bluetooth apps that might be connected",
                "Stay within 10 meters of the sensor",
                "BLE may have slightly higher latency than ANT+",
            ]
        }
        Protocol::AntFec | Protocol::AntPower | Protocol::AntHeartRate | Protocol::AntSpeedCadence => {
            vec![
                "Ensure ANT+ USB dongle is connected",
                "Use USB 2.0 port for better stability",
                "Position dongle away from USB 3.0 devices",
                "ANT+ typically has lower latency than BLE",
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_display() {
        assert_eq!(TroubleshootingCategory::Signal.to_string(), "Signal Quality");
        assert_eq!(TroubleshootingCategory::Connection.to_string(), "Connection");
        assert_eq!(TroubleshootingCategory::Discovery.to_string(), "Discovery");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(TipPriority::Critical > TipPriority::High);
        assert!(TipPriority::High > TipPriority::Medium);
        assert!(TipPriority::Medium > TipPriority::Low);
    }

    #[test]
    fn test_tip_creation() {
        let tip = TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::High,
            "Test Tip",
            "Test issue description",
            vec!["Step 1", "Step 2", "Step 3"],
        );

        assert_eq!(tip.title, "Test Tip");
        assert_eq!(tip.issue, "Test issue description");
        assert_eq!(tip.resolution.len(), 3);
        assert!(!tip.dismissed);
        assert_eq!(tip.icon, "📶");
    }

    #[test]
    fn test_tip_short_resolution() {
        let tip = TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::High,
            "Test",
            "Issue",
            vec!["First step", "Second step"],
        );

        assert_eq!(tip.short_resolution(), "First step");
    }

    #[test]
    fn test_tip_full_resolution() {
        let tip = TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::High,
            "Test",
            "Issue",
            vec!["First", "Second"],
        );

        let full = tip.full_resolution();
        assert!(full.contains("1. First"));
        assert!(full.contains("2. Second"));
    }

    #[test]
    fn test_tip_relevance() {
        let power_tip = TroubleshootingTip::new(
            TroubleshootingCategory::PowerMeter,
            TipPriority::High,
            "Power Tip",
            "Issue",
            vec!["Step"],
        );

        assert!(power_tip.is_relevant_for(SensorType::PowerMeter));
        assert!(power_tip.is_relevant_for(SensorType::Trainer));
        assert!(!power_tip.is_relevant_for(SensorType::HeartRate));

        let signal_tip = TroubleshootingTip::new(
            TroubleshootingCategory::Signal,
            TipPriority::Medium,
            "Signal Tip",
            "Issue",
            vec!["Step"],
        );

        assert!(signal_tip.is_relevant_for(SensorType::HeartRate));
        assert!(signal_tip.is_relevant_for(SensorType::PowerMeter));
    }

    #[test]
    fn test_guide_creation() {
        let guide = TroubleshootingGuide::new();
        assert!(!guide.all_tips().is_empty());
    }

    #[test]
    fn test_guide_category_filter() {
        let guide = TroubleshootingGuide::new();
        let signal_tips = guide.tips_for_category(TroubleshootingCategory::Signal);

        assert!(!signal_tips.is_empty());
        for tip in signal_tips {
            assert_eq!(tip.category, TroubleshootingCategory::Signal);
        }
    }

    #[test]
    fn test_guide_priority_filter() {
        let guide = TroubleshootingGuide::new();
        let high_tips = guide.tips_at_priority(TipPriority::High);

        for tip in high_tips {
            assert!(tip.priority >= TipPriority::High);
        }
    }

    #[test]
    fn test_detected_issue() {
        let issue = DetectedIssue::new(
            TroubleshootingCategory::Signal,
            "Test issue",
        )
        .with_device("device1")
        .with_sensor_name("HR Sensor")
        .with_data("RSSI: -80 dBm");

        assert_eq!(issue.category, TroubleshootingCategory::Signal);
        assert_eq!(issue.device_id, Some("device1".to_string()));
        assert_eq!(issue.sensor_name, Some("HR Sensor".to_string()));
        assert_eq!(issue.data, Some("RSSI: -80 dBm".to_string()));
    }

    #[test]
    fn test_issue_detector_new() {
        let detector = IssueDetector::new();
        assert!(!detector.has_issues());
        assert_eq!(detector.issue_count(), 0);
    }

    #[test]
    fn test_issue_detector_quality_issue() {
        let mut detector = IssueDetector::new();
        detector.record_quality_issue("device1", "HR Sensor", QualityLevel::Poor, Some(-85));

        assert!(detector.has_issues());
        assert_eq!(detector.issue_count(), 1);

        let issues = detector.get_issues();
        assert_eq!(issues[0].category, TroubleshootingCategory::Signal);
    }

    #[test]
    fn test_issue_detector_health_issue() {
        let mut detector = IssueDetector::new();
        detector.record_health_issue("device1", "Power Meter", HealthStatus::Stale);

        assert!(detector.has_issues());
        let issues = detector.get_issues();
        assert_eq!(issues[0].category, TroubleshootingCategory::Connection);
    }

    #[test]
    fn test_issue_detector_relevant_tips() {
        let mut detector = IssueDetector::new();
        detector.record_quality_issue("device1", "HR", QualityLevel::Poor, None);

        let tips = detector.get_relevant_tips();
        assert!(!tips.is_empty());

        for tip in tips {
            assert_eq!(tip.category, TroubleshootingCategory::Signal);
        }
    }

    #[test]
    fn test_issue_detector_clear() {
        let mut detector = IssueDetector::new();
        detector.record_battery_issue("device1", "Power Meter", 5);
        assert!(detector.has_issues());

        detector.clear();
        assert!(!detector.has_issues());
    }

    #[test]
    fn test_quick_tips() {
        let tips = get_no_sensors_tips();
        assert!(!tips.is_empty());

        let signal_tips = get_poor_signal_tips();
        assert!(!signal_tips.is_empty());

        let power_tips = get_power_meter_tips();
        assert!(!power_tips.is_empty());
    }

    #[test]
    fn test_protocol_tips() {
        let ble_tips = get_protocol_specific_tips(Protocol::BleFtms);
        assert!(!ble_tips.is_empty());

        let ant_tips = get_protocol_specific_tips(Protocol::AntPower);
        assert!(!ant_tips.is_empty());
    }
}
