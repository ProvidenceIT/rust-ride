//! Dual Protocol Detection and Binding
//!
//! Handles detection of sensors that support both BLE and ANT+ protocols,
//! allowing users to choose their preferred connection method.

use super::AntDeviceType;
use crate::sensors::types::SensorType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a binding between the same physical sensor's different protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualProtocolBinding {
    /// Unique ID for this binding
    pub id: Uuid,
    /// The BLE sensor UUID (if discovered via BLE)
    pub ble_device_id: Option<String>,
    /// The ANT+ device ID (if discovered via ANT+)
    pub ant_device_id: Option<u16>,
    /// Sensor type
    pub sensor_type: SensorType,
    /// Manufacturer name if known
    pub manufacturer: Option<String>,
    /// Device serial number (used for matching)
    pub serial_number: Option<String>,
    /// User's preferred protocol
    pub preferred_protocol: Protocol,
    /// Display name for the sensor
    pub display_name: String,
    /// Whether this binding has been confirmed by the user
    pub confirmed: bool,
}

/// Protocol preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    /// Prefer BLE (Bluetooth Low Energy)
    Ble,
    /// Prefer ANT+
    AntPlus,
    /// No preference - use whichever connects first
    Auto,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Auto
    }
}

impl DualProtocolBinding {
    /// Create a new binding from a BLE discovery
    pub fn from_ble(ble_device_id: String, sensor_type: SensorType, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            ble_device_id: Some(ble_device_id),
            ant_device_id: None,
            sensor_type,
            manufacturer: None,
            serial_number: None,
            preferred_protocol: Protocol::Auto,
            display_name: name,
            confirmed: false,
        }
    }

    /// Create a new binding from an ANT+ discovery
    pub fn from_ant(ant_device_id: u16, device_type: AntDeviceType) -> Self {
        let sensor_type = match device_type {
            AntDeviceType::HeartRate => SensorType::HeartRate,
            AntDeviceType::Power => SensorType::PowerMeter,
            AntDeviceType::SpeedCadence => SensorType::CadenceSensor,
            AntDeviceType::FitnessEquipment => SensorType::SmartTrainer,
            AntDeviceType::Unknown(_) => SensorType::PowerMeter, // Default
        };

        Self {
            id: Uuid::new_v4(),
            ble_device_id: None,
            ant_device_id: Some(ant_device_id),
            sensor_type,
            manufacturer: None,
            serial_number: None,
            preferred_protocol: Protocol::Auto,
            display_name: format!("ANT+ Device {}", ant_device_id),
            confirmed: false,
        }
    }

    /// Check if this binding has both protocols available
    pub fn is_dual_protocol(&self) -> bool {
        self.ble_device_id.is_some() && self.ant_device_id.is_some()
    }

    /// Link an ANT+ device to an existing BLE binding
    pub fn link_ant(&mut self, ant_device_id: u16) {
        self.ant_device_id = Some(ant_device_id);
    }

    /// Link a BLE device to an existing ANT+ binding
    pub fn link_ble(&mut self, ble_device_id: String) {
        self.ble_device_id = Some(ble_device_id);
    }
}

/// Trait for detecting and managing dual-protocol sensors
pub trait DualProtocolDetector: Send + Sync {
    /// Check if a newly discovered sensor matches an existing sensor on another protocol
    fn detect_duplicate(
        &self,
        sensor_type: SensorType,
        serial_number: Option<&str>,
        manufacturer_id: Option<u16>,
    ) -> Option<DualProtocolBinding>;

    /// Get all known dual-protocol bindings
    fn get_bindings(&self) -> Vec<DualProtocolBinding>;

    /// Add or update a binding
    fn save_binding(&self, binding: DualProtocolBinding);

    /// Remove a binding
    fn remove_binding(&self, id: &Uuid);

    /// Set preferred protocol for a binding
    fn set_preferred_protocol(&self, id: &Uuid, protocol: Protocol);

    /// Confirm a binding (user verified it's the same sensor)
    fn confirm_binding(&self, id: &Uuid);
}

/// Potential match between sensors on different protocols
#[derive(Debug, Clone)]
pub struct PotentialMatch {
    /// The existing binding that might match
    pub existing: DualProtocolBinding,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reason for the match
    pub reason: MatchReason,
}

/// Reason why two sensors might be the same device
#[derive(Debug, Clone)]
pub enum MatchReason {
    /// Serial numbers match exactly
    SerialNumberMatch,
    /// Manufacturer ID matches
    ManufacturerMatch,
    /// Similar name pattern
    NameSimilarity(f32),
    /// Discovered at similar time with same type
    TimingAndType,
}

/// Utility for matching sensors across protocols
pub struct SensorMatcher;

impl SensorMatcher {
    /// Calculate match confidence between two sensor descriptions
    pub fn calculate_match(
        sensor_type: SensorType,
        serial1: Option<&str>,
        serial2: Option<&str>,
        manufacturer1: Option<&str>,
        manufacturer2: Option<&str>,
    ) -> f32 {
        let mut score: f32 = 0.0;

        // Serial number match is very strong indicator
        if let (Some(s1), Some(s2)) = (serial1, serial2) {
            if s1 == s2 {
                return 1.0; // Definite match
            }
        }

        // Manufacturer match adds confidence
        if let (Some(m1), Some(m2)) = (manufacturer1, manufacturer2) {
            if m1.to_lowercase() == m2.to_lowercase() {
                score += 0.4;
            }
        }

        // Same sensor type adds some confidence
        score += 0.2;

        score.min(0.9) // Cap at 0.9 without serial match
    }

    /// Parse manufacturer ID from ANT+ manufacturer field
    pub fn parse_ant_manufacturer(manufacturer_id: u16) -> Option<&'static str> {
        // Common ANT+ manufacturer IDs
        match manufacturer_id {
            1 => Some("Garmin"),
            2 => Some("Garmin"),
            7 => Some("Quarq"),
            32 => Some("Wahoo"),
            48 => Some("SRM"),
            51 => Some("4iiii"),
            52 => Some("Stages"),
            76 => Some("Tacx"),
            89 => Some("Favero"),
            95 => Some("Elite"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_creation() {
        let binding = DualProtocolBinding::from_ble(
            "device-123".to_string(),
            SensorType::PowerMeter,
            "Power Meter".to_string(),
        );
        assert!(binding.ble_device_id.is_some());
        assert!(binding.ant_device_id.is_none());
        assert!(!binding.is_dual_protocol());
    }

    #[test]
    fn test_dual_protocol_linking() {
        let mut binding = DualProtocolBinding::from_ble(
            "device-123".to_string(),
            SensorType::PowerMeter,
            "Power Meter".to_string(),
        );
        binding.link_ant(12345);
        assert!(binding.is_dual_protocol());
    }

    #[test]
    fn test_manufacturer_lookup() {
        assert_eq!(SensorMatcher::parse_ant_manufacturer(1), Some("Garmin"));
        assert_eq!(SensorMatcher::parse_ant_manufacturer(32), Some("Wahoo"));
        assert_eq!(SensorMatcher::parse_ant_manufacturer(9999), None);
    }

    #[test]
    fn test_match_calculation() {
        // Perfect serial match
        let score = SensorMatcher::calculate_match(
            SensorType::PowerMeter,
            Some("12345"),
            Some("12345"),
            None,
            None,
        );
        assert_eq!(score, 1.0);

        // Manufacturer match only
        let score = SensorMatcher::calculate_match(
            SensorType::PowerMeter,
            None,
            None,
            Some("Garmin"),
            Some("garmin"),
        );
        assert!(score > 0.5 && score < 1.0);
    }
}
