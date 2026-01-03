//! Unit tests for dual-protocol sensor detection.
//!
//! Tests the detection of sensors available on both BLE and ANT+
//! and the creation of bindings between dual-protocol instances.

use rust_ride::sensors::types::{DiscoveredSensor, Protocol, SensorType};
use rust_ride::sensors::dual_protocol::{
    DetectionResult, DualProtocolBinding, DualProtocolDetector, MatchConfidence,
    SensorIdentifier, SensorManufacturer,
};
use std::time::Instant;

// ============================================================================
// Helper functions
// ============================================================================

/// Create a BLE discovered sensor for testing.
fn make_ble_sensor(name: &str, sensor_type: SensorType) -> DiscoveredSensor {
    let protocol = match sensor_type {
        SensorType::Trainer | SensorType::SmartTrainer => Protocol::BleFtms,
        SensorType::PowerMeter => Protocol::BleCyclingPower,
        SensorType::HeartRate => Protocol::BleHeartRate,
        _ => Protocol::BleCsc,
    };

    DiscoveredSensor {
        device_id: format!("ble:{}", name.replace(' ', "_").to_lowercase()),
        name: name.to_string(),
        sensor_type,
        protocol,
        signal_strength: Some(-60),
        last_seen: Instant::now(),
    }
}

/// Create an ANT+ discovered sensor for testing.
fn make_ant_sensor(name: &str, device_number: u16, sensor_type: SensorType) -> DiscoveredSensor {
    let protocol = match sensor_type {
        SensorType::HeartRate => Protocol::AntHeartRate,
        SensorType::PowerMeter => Protocol::AntPower,
        SensorType::Trainer | SensorType::SmartTrainer => Protocol::AntFec,
        _ => Protocol::AntSpeedCadence,
    };

    DiscoveredSensor {
        device_id: format!("ant+:{}:{}", device_type_number(sensor_type), device_number),
        name: name.to_string(),
        sensor_type,
        protocol,
        signal_strength: None,
        last_seen: Instant::now(),
    }
}

/// Get ANT+ device type number for sensor type.
fn device_type_number(sensor_type: SensorType) -> u8 {
    match sensor_type {
        SensorType::HeartRate => 120,
        SensorType::PowerMeter => 11,
        SensorType::Trainer | SensorType::SmartTrainer => 17,
        _ => 121,
    }
}

// ============================================================================
// SensorIdentifier tests
// ============================================================================

#[test]
fn test_sensor_identifier_from_wahoo_trainer() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let identifier = SensorIdentifier::from_discovered(&sensor);

    assert_eq!(identifier.normalized_name, "kickr core 1234");
    assert_eq!(identifier.serial_number, Some("1234".to_string()));
    assert_eq!(identifier.manufacturer, Some(SensorManufacturer::Wahoo));
    assert_eq!(identifier.original_name, "KICKR CORE 1234");
}

#[test]
fn test_sensor_identifier_from_garmin_hrm() {
    let sensor = make_ble_sensor("Garmin HRM-Dual 56789", SensorType::HeartRate);
    let identifier = SensorIdentifier::from_discovered(&sensor);

    assert_eq!(identifier.serial_number, Some("56789".to_string()));
    assert_eq!(identifier.manufacturer, Some(SensorManufacturer::Garmin));
}

#[test]
fn test_sensor_identifier_from_tacx_neo() {
    let sensor = make_ble_sensor("Tacx NEO 2T 9999", SensorType::Trainer);
    let identifier = SensorIdentifier::from_discovered(&sensor);

    assert_eq!(identifier.manufacturer, Some(SensorManufacturer::Tacx));
    assert_eq!(identifier.serial_number, Some("9999".to_string()));
}

#[test]
fn test_sensor_identifier_from_unknown_sensor() {
    let sensor = make_ble_sensor("Unknown Sensor", SensorType::PowerMeter);
    let identifier = SensorIdentifier::from_discovered(&sensor);

    assert!(identifier.serial_number.is_none());
    assert!(identifier.manufacturer.is_none());
    assert_eq!(identifier.normalized_name, "unknown sensor");
}

#[test]
fn test_sensor_identifier_matching_with_serial() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let ble_id = SensorIdentifier::from_discovered(&ble_sensor);
    let ant_id = SensorIdentifier::from_discovered(&ant_sensor);

    assert!(ble_id.matches(&ant_id), "Sensors with same serial should match");
}

#[test]
fn test_sensor_identifier_matching_with_manufacturer() {
    let ble_sensor = make_ble_sensor("TICKR X", SensorType::HeartRate);
    let ant_sensor = make_ant_sensor("TICKR X", 5678, SensorType::HeartRate);

    let ble_id = SensorIdentifier::from_discovered(&ble_sensor);
    let ant_id = SensorIdentifier::from_discovered(&ant_sensor);

    assert!(ble_id.matches(&ant_id), "Wahoo sensors with same name should match");
}

#[test]
fn test_sensor_identifier_no_match_different_serial() {
    let sensor1 = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let sensor2 = make_ble_sensor("KICKR CORE 5678", SensorType::Trainer);

    let id1 = SensorIdentifier::from_discovered(&sensor1);
    let id2 = SensorIdentifier::from_discovered(&sensor2);

    assert!(!id1.matches(&id2), "Different serials should not match");
}

// ============================================================================
// DualProtocolBinding tests
// ============================================================================

#[test]
fn test_binding_new_from_ble_sensor() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let binding = DualProtocolBinding::new(&sensor);

    assert!(binding.ble_device_id.is_some());
    assert!(binding.ant_device_id.is_none());
    assert!(!binding.is_complete());
    assert!(binding.is_partial());
    assert_eq!(binding.sensor_type, SensorType::Trainer);
}

#[test]
fn test_binding_new_from_ant_sensor() {
    let sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let binding = DualProtocolBinding::new(&sensor);

    assert!(binding.ble_device_id.is_none());
    assert!(binding.ant_device_id.is_some());
    assert!(!binding.is_complete());
    assert!(binding.is_partial());
}

#[test]
fn test_binding_add_second_protocol() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    let added = binding.add_protocol_instance(&ant_sensor);

    assert!(added, "Should successfully add ANT+ instance");
    assert!(binding.is_complete(), "Binding should be complete");
    assert!(binding.ble_device_id.is_some());
    assert!(binding.ant_device_id.is_some());
}

#[test]
fn test_binding_add_duplicate_protocol_fails() {
    let ble_sensor1 = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ble_sensor2 = make_ble_sensor("KICKR CORE 1234 BLE", SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor1);
    let added = binding.add_protocol_instance(&ble_sensor2);

    assert!(!added, "Should not add duplicate BLE instance");
    assert!(!binding.is_complete());
}

#[test]
fn test_binding_device_ids() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    binding.add_protocol_instance(&ant_sensor);

    let ids = binding.device_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&ble_sensor.device_id.as_str()));
    assert!(ids.contains(&ant_sensor.device_id.as_str()));
}

#[test]
fn test_binding_available_protocols() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let binding = DualProtocolBinding::new(&ble_sensor);

    let protocols = binding.available_protocols();
    assert_eq!(protocols.len(), 1);
    assert!(protocols.contains(&rust_ride::sensors::types::SensorProtocol::Ble));
}

#[test]
fn test_binding_confidence_increases_with_completion() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    let initial_confidence = binding.confidence;

    binding.add_protocol_instance(&ant_sensor);

    // Confidence should increase or stay the same when completing
    assert!(binding.confidence >= initial_confidence);
}

// ============================================================================
// DualProtocolDetector tests
// ============================================================================

#[test]
fn test_detector_new() {
    let detector = DualProtocolDetector::new();
    assert!(detector.is_empty());
    assert_eq!(detector.len(), 0);
}

#[test]
fn test_detector_process_single_sensor() {
    let mut detector = DualProtocolDetector::new();
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);

    let binding_id = detector.process_sensor(&sensor);

    assert!(binding_id.is_some());
    assert_eq!(detector.len(), 1);
    assert_eq!(detector.complete_count(), 0);
}

#[test]
fn test_detector_process_dual_protocol_pair() {
    let mut detector = DualProtocolDetector::new();

    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let binding_id1 = detector.process_sensor(&ble_sensor);
    let binding_id2 = detector.process_sensor(&ant_sensor);

    assert_eq!(binding_id1, binding_id2, "Both should be in same binding");
    assert_eq!(detector.len(), 1);
    assert_eq!(detector.complete_count(), 1);
    assert!(detector.is_dual_protocol(&ble_sensor.device_id));
    assert!(detector.is_dual_protocol(&ant_sensor.device_id));
}

#[test]
fn test_detector_process_same_sensor_twice() {
    let mut detector = DualProtocolDetector::new();
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);

    let binding_id1 = detector.process_sensor(&sensor);
    let binding_id2 = detector.process_sensor(&sensor);

    assert_eq!(binding_id1, binding_id2);
    assert_eq!(detector.len(), 1);
}

#[test]
fn test_detector_process_multiple_sensors() {
    let mut detector = DualProtocolDetector::new();

    let sensors = vec![
        make_ble_sensor("KICKR CORE 1234", SensorType::Trainer),
        make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer),
        make_ble_sensor("TICKR X", SensorType::HeartRate),
        make_ant_sensor("TICKR X", 5678, SensorType::HeartRate),
        make_ble_sensor("Stages 12345", SensorType::PowerMeter),
    ];

    let result = detector.process_sensors(&sensors);

    assert_eq!(result.complete_bindings.len(), 2); // KICKR + TICKR
    assert!(result.has_dual_protocol_sensors());
}

#[test]
fn test_detector_different_sensor_types_no_match() {
    let mut detector = DualProtocolDetector::new();

    // Same name but different sensor types should NOT match
    let trainer = make_ble_sensor("Sensor 1234", SensorType::Trainer);
    let hr = make_ant_sensor("Sensor 1234", 1234, SensorType::HeartRate);

    detector.process_sensor(&trainer);
    detector.process_sensor(&hr);

    assert_eq!(detector.len(), 2, "Different types should create separate bindings");
    assert_eq!(detector.complete_count(), 0);
}

#[test]
fn test_detector_get_alternate_device_id() {
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
fn test_detector_get_binding_for_device() {
    let mut detector = DualProtocolDetector::new();

    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    detector.process_sensor(&sensor);

    let binding = detector.get_binding_for_device(&sensor.device_id);
    assert!(binding.is_some());
    assert_eq!(binding.unwrap().sensor_type, SensorType::Trainer);
}

#[test]
fn test_detector_get_complete_bindings() {
    let mut detector = DualProtocolDetector::new();

    let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let ble_hr = make_ble_sensor("TICKR X", SensorType::HeartRate);

    detector.process_sensor(&ble_trainer);
    detector.process_sensor(&ant_trainer);
    detector.process_sensor(&ble_hr);

    let complete = detector.get_complete_bindings();
    assert_eq!(complete.len(), 1);
    assert_eq!(complete[0].sensor_type, SensorType::Trainer);
}

#[test]
fn test_detector_get_partial_bindings() {
    let mut detector = DualProtocolDetector::new();

    let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let ble_hr = make_ble_sensor("TICKR X", SensorType::HeartRate);

    detector.process_sensor(&ble_trainer);
    detector.process_sensor(&ant_trainer);
    detector.process_sensor(&ble_hr);

    let partial = detector.get_partial_bindings();
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].sensor_type, SensorType::HeartRate);
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
fn test_detector_clear() {
    let mut detector = DualProtocolDetector::new();

    detector.process_sensor(&make_ble_sensor("KICKR 1234", SensorType::Trainer));
    detector.process_sensor(&make_ble_sensor("TICKR X", SensorType::HeartRate));

    assert_eq!(detector.len(), 2);

    detector.clear();
    assert!(detector.is_empty());
}

// ============================================================================
// Manufacturer detection tests
// ============================================================================

#[test]
fn test_manufacturer_detection_wahoo() {
    let sensors = [
        "KICKR CORE 1234",
        "TICKR X",
        "Wahoo ELEMNT",
    ];

    for name in sensors {
        let sensor = make_ble_sensor(name, SensorType::Trainer);
        let id = SensorIdentifier::from_discovered(&sensor);
        assert_eq!(id.manufacturer, Some(SensorManufacturer::Wahoo), "Failed for: {}", name);
    }
}

#[test]
fn test_manufacturer_detection_garmin() {
    let sensors = [
        "Garmin HRM-Dual",
        "Rally XC200",
        "Edge 530",
    ];

    for name in sensors {
        let sensor = make_ble_sensor(name, SensorType::HeartRate);
        let id = SensorIdentifier::from_discovered(&sensor);
        assert_eq!(id.manufacturer, Some(SensorManufacturer::Garmin), "Failed for: {}", name);
    }
}

#[test]
fn test_manufacturer_detection_tacx() {
    let sensors = [
        "Tacx NEO 2T",
        "NEO Bike Plus",
        "Flux 2",
    ];

    for name in sensors {
        let sensor = make_ble_sensor(name, SensorType::Trainer);
        let id = SensorIdentifier::from_discovered(&sensor);
        assert_eq!(id.manufacturer, Some(SensorManufacturer::Tacx), "Failed for: {}", name);
    }
}

#[test]
fn test_manufacturer_detection_polar() {
    let sensors = [
        "Polar H10",
        "H9 Heart Rate",
        "Verity Sense",
    ];

    for name in sensors {
        let sensor = make_ble_sensor(name, SensorType::HeartRate);
        let id = SensorIdentifier::from_discovered(&sensor);
        assert_eq!(id.manufacturer, Some(SensorManufacturer::Polar), "Failed for: {}", name);
    }
}

// ============================================================================
// Match confidence tests
// ============================================================================

#[test]
fn test_match_confidence_ordering() {
    assert!(MatchConfidence::Low < MatchConfidence::Medium);
    assert!(MatchConfidence::Medium < MatchConfidence::High);
    assert!(MatchConfidence::Low < MatchConfidence::High);
}

#[test]
fn test_high_confidence_with_serial() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let binding = DualProtocolBinding::new(&sensor);

    // Has serial number, should be high confidence
    assert_eq!(binding.confidence, MatchConfidence::High);
}

#[test]
fn test_medium_confidence_with_manufacturer() {
    let sensor = make_ble_sensor("KICKR", SensorType::Trainer);
    let binding = DualProtocolBinding::new(&sensor);

    // Has manufacturer but no serial
    assert_eq!(binding.confidence, MatchConfidence::Medium);
}

#[test]
fn test_low_confidence_unknown_sensor() {
    let sensor = make_ble_sensor("Unknown Device", SensorType::PowerMeter);
    let binding = DualProtocolBinding::new(&sensor);

    // No manufacturer, no serial
    assert_eq!(binding.confidence, MatchConfidence::Low);
}

#[test]
fn test_min_confidence_threshold() {
    let mut detector = DualProtocolDetector::with_min_confidence(MatchConfidence::Medium);

    // Unknown sensor should be rejected
    let unknown = make_ble_sensor("Random Sensor", SensorType::PowerMeter);
    let binding_id = detector.process_sensor(&unknown);

    // Should be rejected (no manufacturer = Low confidence)
    assert!(binding_id.is_none() || detector.get_binding(&binding_id.unwrap()).unwrap().confidence >= MatchConfidence::Medium);

    // Known manufacturer should be accepted
    let wahoo = make_ble_sensor("KICKR", SensorType::Trainer);
    let binding_id2 = detector.process_sensor(&wahoo);

    assert!(binding_id2.is_some());
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_real_world_wahoo_setup() {
    let mut detector = DualProtocolDetector::new();

    // Simulating a typical Wahoo setup with dual-protocol devices
    let sensors = vec![
        make_ble_sensor("KICKR CORE 7890", SensorType::Trainer),
        make_ant_sensor("KICKR CORE 7890", 7890, SensorType::Trainer),
        make_ble_sensor("TICKR X 1234", SensorType::HeartRate),
        make_ant_sensor("TICKR X 1234", 1234, SensorType::HeartRate),
    ];

    let result = detector.process_sensors(&sensors);

    assert_eq!(result.complete_bindings.len(), 2);
    assert!(result.has_dual_protocol_sensors());

    // Both sensors should be detected as dual-protocol
    for binding in &result.complete_bindings {
        assert!(binding.is_complete());
        assert!(binding.ble_device_id.is_some());
        assert!(binding.ant_device_id.is_some());
    }
}

#[test]
fn test_real_world_mixed_setup() {
    let mut detector = DualProtocolDetector::new();

    // Mix of dual-protocol and single-protocol devices
    let sensors = vec![
        // Dual-protocol trainer
        make_ble_sensor("Tacx NEO 2T 5555", SensorType::Trainer),
        make_ant_sensor("Tacx NEO 2T 5555", 5555, SensorType::Trainer),
        // BLE-only power meter
        make_ble_sensor("Stages 99999", SensorType::PowerMeter),
        // ANT+-only HR strap
        make_ant_sensor("Generic HR 1111", 1111, SensorType::HeartRate),
    ];

    let result = detector.process_sensors(&sensors);

    assert_eq!(result.complete_bindings.len(), 1); // Only Tacx
    assert_eq!(result.partial_bindings.len(), 2); // Stages + Generic HR
}

#[test]
fn test_binding_summary_format() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    binding.add_protocol_instance(&ant_sensor);

    let summary = binding.summary();
    assert!(summary.contains("KICKR CORE 1234"));
    assert!(summary.contains("BLE"));
    assert!(summary.contains("ANT+"));
}
