//! Dual-protocol sensor detection and binding.
//!
//! Detects when a sensor is available on both BLE and ANT+ by matching
//! device names and serial numbers. Creates bindings between dual-protocol
//! instances to enable protocol preference selection and failover.

use crate::sensors::types::{DiscoveredSensor, Protocol, SensorProtocol, SensorType};
use std::collections::HashMap;
use std::time::Instant;

/// Identifier extracted from a sensor for matching purposes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SensorIdentifier {
    /// Normalized name for matching (lowercase, whitespace trimmed).
    pub normalized_name: String,
    /// Serial number if extracted from name or device ID.
    pub serial_number: Option<String>,
    /// Manufacturer if detected from name patterns.
    pub manufacturer: Option<SensorManufacturer>,
    /// Original device name for display.
    pub original_name: String,
}

impl SensorIdentifier {
    /// Create a new sensor identifier from a discovered sensor.
    pub fn from_discovered(sensor: &DiscoveredSensor) -> Self {
        let normalized_name = normalize_name(&sensor.name);
        let serial_number = extract_serial_number(&sensor.name, &sensor.device_id);
        let manufacturer = detect_manufacturer(&sensor.name);

        Self {
            normalized_name,
            serial_number,
            manufacturer,
            original_name: sensor.name.clone(),
        }
    }

    /// Check if this identifier matches another for dual-protocol binding.
    ///
    /// Matching criteria (in order of reliability):
    /// 1. Serial number match (if both have serial numbers)
    /// 2. Manufacturer + model pattern match
    /// 3. Normalized name match (fallback)
    pub fn matches(&self, other: &SensorIdentifier) -> bool {
        // Serial number match is most reliable
        if let (Some(ref self_serial), Some(ref other_serial)) = (&self.serial_number, &other.serial_number) {
            if self_serial == other_serial {
                return true;
            }
        }

        // Manufacturer + normalized name match
        if let (Some(ref self_mfr), Some(ref other_mfr)) = (&self.manufacturer, &other.manufacturer) {
            if self_mfr == other_mfr && self.normalized_name == other.normalized_name {
                return true;
            }
        }

        // Fallback: normalized name match for same manufacturer patterns
        // This requires high confidence (e.g., "KICKR CORE 1234" appears on both)
        if self.manufacturer.is_some() && other.manufacturer.is_some() {
            self.normalized_name == other.normalized_name
        } else {
            // Without manufacturer detection, require exact match to avoid false positives
            self.normalized_name == other.normalized_name && !self.normalized_name.is_empty()
        }
    }

    /// Get a display string for this identifier.
    pub fn display_name(&self) -> &str {
        &self.original_name
    }
}

/// Known sensor manufacturers for pattern matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorManufacturer {
    /// Wahoo (KICKR, TICKR, etc.)
    Wahoo,
    /// Garmin (Edge, Rally, HRM, etc.)
    Garmin,
    /// Stages (power meters)
    Stages,
    /// Tacx (trainers)
    Tacx,
    /// Elite (trainers)
    Elite,
    /// 4iiii (power meters)
    FourIiii,
    /// Favero (Assioma pedals)
    Favero,
    /// Quarq (power meters)
    Quarq,
    /// SRAM (power meters)
    Sram,
    /// Polar (heart rate)
    Polar,
    /// Zwift (Hub trainer)
    Zwift,
}

impl SensorManufacturer {
    /// Get name patterns that identify this manufacturer.
    pub fn name_patterns(&self) -> &[&str] {
        match self {
            SensorManufacturer::Wahoo => &["kickr", "tickr", "wahoo", "elemnt"],
            SensorManufacturer::Garmin => &["garmin", "rally", "hrm-", "edge"],
            SensorManufacturer::Stages => &["stages"],
            SensorManufacturer::Tacx => &["tacx", "neo", "flux"],
            SensorManufacturer::Elite => &["elite", "direto", "suito", "zumo"],
            SensorManufacturer::FourIiii => &["4iiii", "precision"],
            SensorManufacturer::Favero => &["favero", "assioma"],
            SensorManufacturer::Quarq => &["quarq"],
            SensorManufacturer::Sram => &["sram"],
            SensorManufacturer::Polar => &["polar", "h10", "h9", "verity"],
            SensorManufacturer::Zwift => &["zwift", "hub"],
        }
    }
}

impl std::fmt::Display for SensorManufacturer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorManufacturer::Wahoo => write!(f, "Wahoo"),
            SensorManufacturer::Garmin => write!(f, "Garmin"),
            SensorManufacturer::Stages => write!(f, "Stages"),
            SensorManufacturer::Tacx => write!(f, "Tacx"),
            SensorManufacturer::Elite => write!(f, "Elite"),
            SensorManufacturer::FourIiii => write!(f, "4iiii"),
            SensorManufacturer::Favero => write!(f, "Favero"),
            SensorManufacturer::Quarq => write!(f, "Quarq"),
            SensorManufacturer::Sram => write!(f, "SRAM"),
            SensorManufacturer::Polar => write!(f, "Polar"),
            SensorManufacturer::Zwift => write!(f, "Zwift"),
        }
    }
}

/// Binding between BLE and ANT+ instances of the same physical sensor.
#[derive(Debug, Clone)]
pub struct DualProtocolBinding {
    /// Unique identifier for this binding.
    pub binding_id: String,
    /// BLE device ID.
    pub ble_device_id: Option<String>,
    /// ANT+ device ID.
    pub ant_device_id: Option<String>,
    /// Sensor identifier used for matching.
    pub identifier: SensorIdentifier,
    /// Sensor type (should be same for both protocols).
    pub sensor_type: SensorType,
    /// When the binding was created.
    pub created_at: Instant,
    /// When the binding was last updated (new instance added).
    pub updated_at: Instant,
    /// Confidence level of the match.
    pub confidence: MatchConfidence,
}

impl DualProtocolBinding {
    /// Create a new binding from a single sensor.
    pub fn new(sensor: &DiscoveredSensor) -> Self {
        let identifier = SensorIdentifier::from_discovered(sensor);
        let binding_id = generate_binding_id(&identifier);
        let now = Instant::now();
        let protocol = sensor.protocol.sensor_protocol();

        let (ble_device_id, ant_device_id) = match protocol {
            SensorProtocol::Ble => (Some(sensor.device_id.clone()), None),
            SensorProtocol::AntPlus => (None, Some(sensor.device_id.clone())),
        };

        // Initial confidence based on identifier quality
        let confidence = if identifier.serial_number.is_some() {
            MatchConfidence::High
        } else if identifier.manufacturer.is_some() {
            MatchConfidence::Medium
        } else {
            MatchConfidence::Low
        };

        Self {
            binding_id,
            ble_device_id,
            ant_device_id,
            identifier,
            sensor_type: sensor.sensor_type,
            created_at: now,
            updated_at: now,
            confidence,
        }
    }

    /// Add the other protocol instance to this binding.
    ///
    /// Returns true if the sensor was added, false if it was already present.
    pub fn add_protocol_instance(&mut self, sensor: &DiscoveredSensor) -> bool {
        let protocol = sensor.protocol.sensor_protocol();

        match protocol {
            SensorProtocol::Ble => {
                if self.ble_device_id.is_none() {
                    self.ble_device_id = Some(sensor.device_id.clone());
                    self.updated_at = Instant::now();
                    self.update_confidence();
                    true
                } else {
                    false
                }
            }
            SensorProtocol::AntPlus => {
                if self.ant_device_id.is_none() {
                    self.ant_device_id = Some(sensor.device_id.clone());
                    self.updated_at = Instant::now();
                    self.update_confidence();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Update confidence level based on binding completeness.
    fn update_confidence(&mut self) {
        // Having both protocols increases confidence
        if self.is_complete() {
            self.confidence = match self.confidence {
                MatchConfidence::Low => MatchConfidence::Medium,
                MatchConfidence::Medium => MatchConfidence::High,
                MatchConfidence::High => MatchConfidence::High,
            };
        }
    }

    /// Check if this binding has both BLE and ANT+ instances.
    pub fn is_complete(&self) -> bool {
        self.ble_device_id.is_some() && self.ant_device_id.is_some()
    }

    /// Check if this binding has only one protocol.
    pub fn is_partial(&self) -> bool {
        !self.is_complete() && (self.ble_device_id.is_some() || self.ant_device_id.is_some())
    }

    /// Get the device IDs for this binding.
    pub fn device_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        if let Some(ref ble_id) = self.ble_device_id {
            ids.push(ble_id.as_str());
        }
        if let Some(ref ant_id) = self.ant_device_id {
            ids.push(ant_id.as_str());
        }
        ids
    }

    /// Get the device ID for a specific protocol.
    pub fn device_id_for_protocol(&self, protocol: SensorProtocol) -> Option<&str> {
        match protocol {
            SensorProtocol::Ble => self.ble_device_id.as_deref(),
            SensorProtocol::AntPlus => self.ant_device_id.as_deref(),
        }
    }

    /// Get available protocols for this binding.
    pub fn available_protocols(&self) -> Vec<SensorProtocol> {
        let mut protocols = Vec::new();
        if self.ble_device_id.is_some() {
            protocols.push(SensorProtocol::Ble);
        }
        if self.ant_device_id.is_some() {
            protocols.push(SensorProtocol::AntPlus);
        }
        protocols
    }

    /// Get the display name for this binding.
    pub fn display_name(&self) -> &str {
        self.identifier.display_name()
    }

    /// Get a summary string for this binding.
    pub fn summary(&self) -> String {
        let protocols: Vec<&str> = self.available_protocols()
            .iter()
            .map(|p| match p {
                SensorProtocol::Ble => "BLE",
                SensorProtocol::AntPlus => "ANT+",
            })
            .collect();

        format!(
            "{} ({}) - {}",
            self.display_name(),
            protocols.join("/"),
            self.confidence
        )
    }
}

/// Confidence level for a dual-protocol match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    /// Low confidence - name match only, may be false positive.
    Low,
    /// Medium confidence - manufacturer + name pattern match.
    Medium,
    /// High confidence - serial number match or both protocols confirmed.
    High,
}

impl std::fmt::Display for MatchConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchConfidence::Low => write!(f, "Low"),
            MatchConfidence::Medium => write!(f, "Medium"),
            MatchConfidence::High => write!(f, "High"),
        }
    }
}

/// Result of detecting dual-protocol sensors.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Complete bindings (have both BLE and ANT+ instances).
    pub complete_bindings: Vec<DualProtocolBinding>,
    /// Partial bindings (only one protocol found so far).
    pub partial_bindings: Vec<DualProtocolBinding>,
    /// Sensors that couldn't be identified for binding.
    pub unmatched_sensors: Vec<String>,
}

impl DetectionResult {
    /// Get total number of bindings (complete + partial).
    pub fn total_bindings(&self) -> usize {
        self.complete_bindings.len() + self.partial_bindings.len()
    }

    /// Get the number of dual-protocol sensors detected.
    pub fn dual_protocol_count(&self) -> usize {
        self.complete_bindings.len()
    }

    /// Check if any dual-protocol sensors were detected.
    pub fn has_dual_protocol_sensors(&self) -> bool {
        !self.complete_bindings.is_empty()
    }
}

/// Detector for dual-protocol sensors.
///
/// Manages detection of sensors available on both BLE and ANT+ and creates
/// bindings between their protocol instances.
#[derive(Debug)]
pub struct DualProtocolDetector {
    /// Current bindings indexed by binding ID.
    bindings: HashMap<String, DualProtocolBinding>,
    /// Device ID to binding ID lookup.
    device_to_binding: HashMap<String, String>,
    /// Minimum confidence level for auto-binding.
    min_confidence: MatchConfidence,
}

impl Default for DualProtocolDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DualProtocolDetector {
    /// Create a new dual-protocol detector.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            device_to_binding: HashMap::new(),
            min_confidence: MatchConfidence::Low,
        }
    }

    /// Create a detector with a minimum confidence threshold.
    pub fn with_min_confidence(min_confidence: MatchConfidence) -> Self {
        Self {
            bindings: HashMap::new(),
            device_to_binding: HashMap::new(),
            min_confidence,
        }
    }

    /// Process a discovered sensor and detect/create bindings.
    ///
    /// Returns the binding ID if the sensor was bound to an existing or new binding.
    pub fn process_sensor(&mut self, sensor: &DiscoveredSensor) -> Option<String> {
        // Check if this device is already bound
        if let Some(binding_id) = self.device_to_binding.get(&sensor.device_id) {
            return Some(binding_id.clone());
        }

        let identifier = SensorIdentifier::from_discovered(sensor);

        // Check for matching existing binding
        let matching_binding = self.find_matching_binding(&identifier, sensor.sensor_type);

        if let Some(binding_id) = matching_binding {
            // Add to existing binding
            if let Some(binding) = self.bindings.get_mut(&binding_id) {
                if binding.add_protocol_instance(sensor) {
                    self.device_to_binding.insert(sensor.device_id.clone(), binding_id.clone());
                    tracing::info!(
                        "Added {} instance to dual-protocol binding: {} ({})",
                        sensor.protocol.sensor_protocol(),
                        binding.display_name(),
                        binding_id
                    );
                }
            }
            Some(binding_id)
        } else {
            // Create new binding
            let binding = DualProtocolBinding::new(sensor);

            // Check if it meets minimum confidence
            if binding.confidence >= self.min_confidence {
                let binding_id = binding.binding_id.clone();
                self.device_to_binding.insert(sensor.device_id.clone(), binding_id.clone());
                self.bindings.insert(binding_id.clone(), binding);
                tracing::debug!(
                    "Created new dual-protocol binding for {}: {}",
                    sensor.name,
                    binding_id
                );
                Some(binding_id)
            } else {
                tracing::trace!(
                    "Sensor {} below confidence threshold for binding",
                    sensor.name
                );
                None
            }
        }
    }

    /// Find a matching binding for the given identifier.
    fn find_matching_binding(&self, identifier: &SensorIdentifier, sensor_type: SensorType) -> Option<String> {
        for (binding_id, binding) in &self.bindings {
            // Must be same sensor type
            if binding.sensor_type != sensor_type {
                continue;
            }

            // Check if identifiers match
            if binding.identifier.matches(identifier) {
                return Some(binding_id.clone());
            }
        }
        None
    }

    /// Process multiple discovered sensors at once.
    pub fn process_sensors(&mut self, sensors: &[DiscoveredSensor]) -> DetectionResult {
        let mut unmatched = Vec::new();

        for sensor in sensors {
            if self.process_sensor(sensor).is_none() {
                unmatched.push(sensor.device_id.clone());
            }
        }

        // Separate complete and partial bindings
        let mut complete = Vec::new();
        let mut partial = Vec::new();

        for binding in self.bindings.values() {
            if binding.is_complete() {
                complete.push(binding.clone());
            } else if binding.is_partial() {
                partial.push(binding.clone());
            }
        }

        DetectionResult {
            complete_bindings: complete,
            partial_bindings: partial,
            unmatched_sensors: unmatched,
        }
    }

    /// Get a binding by its ID.
    pub fn get_binding(&self, binding_id: &str) -> Option<&DualProtocolBinding> {
        self.bindings.get(binding_id)
    }

    /// Get a binding by device ID.
    pub fn get_binding_for_device(&self, device_id: &str) -> Option<&DualProtocolBinding> {
        self.device_to_binding
            .get(device_id)
            .and_then(|binding_id| self.bindings.get(binding_id))
    }

    /// Get the binding ID for a device ID.
    pub fn get_binding_id(&self, device_id: &str) -> Option<&str> {
        self.device_to_binding.get(device_id).map(|s| s.as_str())
    }

    /// Check if a device is part of a dual-protocol binding.
    pub fn is_dual_protocol(&self, device_id: &str) -> bool {
        self.device_to_binding
            .get(device_id)
            .and_then(|binding_id| self.bindings.get(binding_id))
            .map_or(false, |b| b.is_complete())
    }

    /// Get the other protocol's device ID for a given device.
    ///
    /// If device_id is the BLE instance, returns the ANT+ instance and vice versa.
    pub fn get_alternate_device_id(&self, device_id: &str) -> Option<&str> {
        let binding = self.get_binding_for_device(device_id)?;

        if binding.ble_device_id.as_deref() == Some(device_id) {
            binding.ant_device_id.as_deref()
        } else if binding.ant_device_id.as_deref() == Some(device_id) {
            binding.ble_device_id.as_deref()
        } else {
            None
        }
    }

    /// Get all complete (dual-protocol) bindings.
    pub fn get_complete_bindings(&self) -> Vec<&DualProtocolBinding> {
        self.bindings.values().filter(|b| b.is_complete()).collect()
    }

    /// Get all partial bindings.
    pub fn get_partial_bindings(&self) -> Vec<&DualProtocolBinding> {
        self.bindings.values().filter(|b| b.is_partial()).collect()
    }

    /// Get all bindings.
    pub fn get_all_bindings(&self) -> Vec<&DualProtocolBinding> {
        self.bindings.values().collect()
    }

    /// Get the number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Get the number of complete (dual-protocol) bindings.
    pub fn complete_count(&self) -> usize {
        self.bindings.values().filter(|b| b.is_complete()).count()
    }

    /// Remove a binding.
    pub fn remove_binding(&mut self, binding_id: &str) -> Option<DualProtocolBinding> {
        if let Some(binding) = self.bindings.remove(binding_id) {
            // Remove device ID mappings
            if let Some(ref ble_id) = binding.ble_device_id {
                self.device_to_binding.remove(ble_id);
            }
            if let Some(ref ant_id) = binding.ant_device_id {
                self.device_to_binding.remove(ant_id);
            }
            Some(binding)
        } else {
            None
        }
    }

    /// Clear all bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.device_to_binding.clear();
    }
}

/// Normalize a sensor name for matching.
///
/// - Converts to lowercase
/// - Trims whitespace
/// - Removes common suffixes/prefixes that vary between protocols
fn normalize_name(name: &str) -> String {
    let normalized = name.to_lowercase().trim().to_string();

    // Remove common protocol-specific suffixes
    let suffixes = [" ble", " bt", " ant+", " ant"];
    let mut result = normalized;

    for suffix in suffixes {
        if result.ends_with(suffix) {
            result = result[..result.len() - suffix.len()].to_string();
        }
    }

    result.trim().to_string()
}

/// Extract a serial number from sensor name or device ID.
///
/// Common patterns:
/// - "KICKR CORE 1234" -> "1234"
/// - "Stages 12345" -> "12345"
/// - Device IDs may contain serial numbers
fn extract_serial_number(name: &str, device_id: &str) -> Option<String> {
    // Try to extract from name first (more reliable)

    // Pattern: 4+ digit number at end of name
    let name_parts: Vec<&str> = name.split_whitespace().collect();
    if let Some(last) = name_parts.last() {
        if last.len() >= 4 && last.chars().all(|c| c.is_ascii_digit()) {
            return Some(last.to_string());
        }
    }

    // Pattern: alphanumeric serial in name (e.g., "A1B2C3")
    for part in name_parts.iter().rev() {
        if part.len() >= 4 && part.chars().all(|c| c.is_ascii_alphanumeric()) {
            // Check if it looks like a serial (mix of letters and numbers)
            let has_digit = part.chars().any(|c| c.is_ascii_digit());
            let has_letter = part.chars().any(|c| c.is_ascii_alphabetic());
            if has_digit && has_letter {
                return Some(part.to_string());
            }
            // All digits is also valid
            if part.chars().all(|c| c.is_ascii_digit()) {
                return Some(part.to_string());
            }
        }
    }

    // Try device_id for ANT+ (often contains device number)
    if device_id.starts_with("ant+:") {
        let parts: Vec<&str> = device_id.split(':').collect();
        if parts.len() >= 3 {
            return Some(parts[2].to_string());
        }
    }

    None
}

/// Detect the manufacturer from a sensor name.
fn detect_manufacturer(name: &str) -> Option<SensorManufacturer> {
    let name_lower = name.to_lowercase();

    let manufacturers = [
        SensorManufacturer::Wahoo,
        SensorManufacturer::Garmin,
        SensorManufacturer::Stages,
        SensorManufacturer::Tacx,
        SensorManufacturer::Elite,
        SensorManufacturer::FourIiii,
        SensorManufacturer::Favero,
        SensorManufacturer::Quarq,
        SensorManufacturer::Sram,
        SensorManufacturer::Polar,
        SensorManufacturer::Zwift,
    ];

    for manufacturer in manufacturers {
        for pattern in manufacturer.name_patterns() {
            if name_lower.contains(pattern) {
                return Some(manufacturer);
            }
        }
    }

    None
}

/// Generate a binding ID from an identifier.
fn generate_binding_id(identifier: &SensorIdentifier) -> String {
    // Use serial if available, otherwise use normalized name
    if let Some(ref serial) = identifier.serial_number {
        format!("binding:{}", serial)
    } else if let Some(ref mfr) = identifier.manufacturer {
        format!("binding:{}:{}", mfr, identifier.normalized_name.replace(' ', "_"))
    } else {
        format!("binding:{}", identifier.normalized_name.replace(' ', "_"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ble_sensor(name: &str, sensor_type: SensorType) -> DiscoveredSensor {
        DiscoveredSensor {
            device_id: format!("ble:{}", name.replace(' ', "_").to_lowercase()),
            name: name.to_string(),
            sensor_type,
            protocol: Protocol::BleFtms,
            signal_strength: Some(-60),
            last_seen: Instant::now(),
        }
    }

    fn make_ant_sensor(name: &str, device_number: u16, sensor_type: SensorType) -> DiscoveredSensor {
        let protocol = match sensor_type {
            SensorType::HeartRate => Protocol::AntHeartRate,
            SensorType::PowerMeter => Protocol::AntPower,
            SensorType::Trainer | SensorType::SmartTrainer => Protocol::AntFec,
            _ => Protocol::AntSpeedCadence,
        };

        DiscoveredSensor {
            device_id: format!("ant+:{}:{}", protocol_type_number(&protocol), device_number),
            name: name.to_string(),
            sensor_type,
            protocol,
            signal_strength: None,
            last_seen: Instant::now(),
        }
    }

    fn protocol_type_number(protocol: &Protocol) -> u8 {
        match protocol {
            Protocol::AntHeartRate => 120,
            Protocol::AntPower => 11,
            Protocol::AntFec => 17,
            Protocol::AntSpeedCadence => 121,
            _ => 0,
        }
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("KICKR CORE 1234"), "kickr core 1234");
        assert_eq!(normalize_name("KICKR CORE 1234 BLE"), "kickr core 1234");
        assert_eq!(normalize_name("KICKR CORE 1234 ANT+"), "kickr core 1234");
        assert_eq!(normalize_name("  Polar H10  "), "polar h10");
    }

    #[test]
    fn test_extract_serial_number() {
        assert_eq!(
            extract_serial_number("KICKR CORE 1234", "ble:xxx"),
            Some("1234".to_string())
        );
        assert_eq!(
            extract_serial_number("Stages 12345", "ble:xxx"),
            Some("12345".to_string())
        );
        assert_eq!(
            extract_serial_number("HR Monitor", "ant+:120:54321"),
            Some("54321".to_string())
        );
        assert_eq!(
            extract_serial_number("Unknown", "ble:xxx"),
            None
        );
    }

    #[test]
    fn test_detect_manufacturer() {
        assert_eq!(detect_manufacturer("KICKR CORE 1234"), Some(SensorManufacturer::Wahoo));
        assert_eq!(detect_manufacturer("TICKR X"), Some(SensorManufacturer::Wahoo));
        assert_eq!(detect_manufacturer("Garmin HRM-Dual"), Some(SensorManufacturer::Garmin));
        assert_eq!(detect_manufacturer("Tacx NEO 2T"), Some(SensorManufacturer::Tacx));
        assert_eq!(detect_manufacturer("Polar H10"), Some(SensorManufacturer::Polar));
        assert_eq!(detect_manufacturer("Unknown Sensor"), None);
    }

    #[test]
    fn test_sensor_identifier_matches() {
        // Same serial number should match
        let id1 = SensorIdentifier {
            normalized_name: "kickr core 1234".to_string(),
            serial_number: Some("1234".to_string()),
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "KICKR CORE 1234".to_string(),
        };
        let id2 = SensorIdentifier {
            normalized_name: "kickr core 1234".to_string(),
            serial_number: Some("1234".to_string()),
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "KICKR CORE 1234 ANT+".to_string(),
        };
        assert!(id1.matches(&id2));

        // Same manufacturer + name should match
        let id3 = SensorIdentifier {
            normalized_name: "tickr x".to_string(),
            serial_number: None,
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "TICKR X".to_string(),
        };
        let id4 = SensorIdentifier {
            normalized_name: "tickr x".to_string(),
            serial_number: None,
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "TICKR X".to_string(),
        };
        assert!(id3.matches(&id4));

        // Different serial should not match
        let id5 = SensorIdentifier {
            normalized_name: "kickr 1234".to_string(),
            serial_number: Some("1234".to_string()),
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "KICKR 1234".to_string(),
        };
        let id6 = SensorIdentifier {
            normalized_name: "kickr 5678".to_string(),
            serial_number: Some("5678".to_string()),
            manufacturer: Some(SensorManufacturer::Wahoo),
            original_name: "KICKR 5678".to_string(),
        };
        assert!(!id5.matches(&id6));
    }

    #[test]
    fn test_dual_protocol_binding_creation() {
        let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let binding = DualProtocolBinding::new(&sensor);

        assert!(binding.ble_device_id.is_some());
        assert!(binding.ant_device_id.is_none());
        assert!(!binding.is_complete());
        assert!(binding.is_partial());
        assert_eq!(binding.sensor_type, SensorType::Trainer);
    }

    #[test]
    fn test_dual_protocol_binding_add_instance() {
        let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let mut binding = DualProtocolBinding::new(&ble_sensor);

        let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
        let added = binding.add_protocol_instance(&ant_sensor);

        assert!(added);
        assert!(binding.is_complete());
        assert!(binding.ble_device_id.is_some());
        assert!(binding.ant_device_id.is_some());
    }

    #[test]
    fn test_dual_protocol_detector_basic() {
        let mut detector = DualProtocolDetector::new();

        let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let binding_id = detector.process_sensor(&ble_sensor);
        assert!(binding_id.is_some());

        assert_eq!(detector.len(), 1);
        assert!(!detector.is_dual_protocol(&ble_sensor.device_id));

        let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
        let binding_id2 = detector.process_sensor(&ant_sensor);
        assert_eq!(binding_id, binding_id2);

        // Now both are bound
        assert!(detector.is_dual_protocol(&ble_sensor.device_id));
        assert!(detector.is_dual_protocol(&ant_sensor.device_id));
        assert_eq!(detector.complete_count(), 1);
    }

    #[test]
    fn test_dual_protocol_detector_multiple_sensors() {
        let mut detector = DualProtocolDetector::new();

        let sensors = vec![
            make_ble_sensor("KICKR CORE 1234", SensorType::Trainer),
            make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer),
            make_ble_sensor("TICKR X", SensorType::HeartRate),
            make_ant_sensor("TICKR X", 5678, SensorType::HeartRate),
            make_ble_sensor("Unknown PM", SensorType::PowerMeter),
        ];

        let result = detector.process_sensors(&sensors);

        assert_eq!(result.complete_bindings.len(), 2); // KICKR + TICKR
        assert!(result.partial_bindings.len() >= 1); // Unknown PM
    }

    #[test]
    fn test_dual_protocol_detector_get_alternate() {
        let mut detector = DualProtocolDetector::new();

        let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

        detector.process_sensor(&ble_sensor);
        detector.process_sensor(&ant_sensor);

        let alternate = detector.get_alternate_device_id(&ble_sensor.device_id);
        assert_eq!(alternate, Some(ant_sensor.device_id.as_str()));

        let alternate2 = detector.get_alternate_device_id(&ant_sensor.device_id);
        assert_eq!(alternate2, Some(ble_sensor.device_id.as_str()));
    }

    #[test]
    fn test_binding_device_id_for_protocol() {
        let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

        let mut binding = DualProtocolBinding::new(&ble_sensor);
        binding.add_protocol_instance(&ant_sensor);

        assert_eq!(
            binding.device_id_for_protocol(SensorProtocol::Ble),
            Some(ble_sensor.device_id.as_str())
        );
        assert_eq!(
            binding.device_id_for_protocol(SensorProtocol::AntPlus),
            Some(ant_sensor.device_id.as_str())
        );
    }

    #[test]
    fn test_binding_available_protocols() {
        let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let binding = DualProtocolBinding::new(&ble_sensor);

        let protocols = binding.available_protocols();
        assert_eq!(protocols.len(), 1);
        assert_eq!(protocols[0], SensorProtocol::Ble);
    }

    #[test]
    fn test_detector_remove_binding() {
        let mut detector = DualProtocolDetector::new();

        let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let binding_id = detector.process_sensor(&sensor).unwrap();

        assert_eq!(detector.len(), 1);

        let removed = detector.remove_binding(&binding_id);
        assert!(removed.is_some());
        assert!(detector.is_empty());
        assert!(detector.get_binding_for_device(&sensor.device_id).is_none());
    }

    #[test]
    fn test_different_sensor_types_no_match() {
        let mut detector = DualProtocolDetector::new();

        // Same name but different types should not match
        let trainer = make_ble_sensor("Sensor 1234", SensorType::Trainer);
        let hr = make_ant_sensor("Sensor 1234", 1234, SensorType::HeartRate);

        detector.process_sensor(&trainer);
        detector.process_sensor(&hr);

        // Should create two separate bindings
        assert_eq!(detector.len(), 2);
        assert_eq!(detector.complete_count(), 0); // No complete dual-protocol
    }

    #[test]
    fn test_match_confidence_ordering() {
        assert!(MatchConfidence::Low < MatchConfidence::Medium);
        assert!(MatchConfidence::Medium < MatchConfidence::High);
    }

    #[test]
    fn test_min_confidence_threshold() {
        let mut detector = DualProtocolDetector::with_min_confidence(MatchConfidence::Medium);

        // Unknown sensor with no manufacturer should have Low confidence
        let sensor = make_ble_sensor("Generic Sensor", SensorType::PowerMeter);
        let binding_id = detector.process_sensor(&sensor);

        // Should be rejected due to low confidence
        // (Generic sensor with no manufacturer pattern = Low confidence)
        // Note: This depends on the name not matching any manufacturer patterns
        // If "Generic Sensor" matches no patterns, it should be None
        assert!(binding_id.is_none() || detector.get_binding(&binding_id.unwrap()).map_or(false, |b| b.confidence >= MatchConfidence::Medium));
    }
}
