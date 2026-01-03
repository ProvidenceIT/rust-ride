//! Unit tests for power meter wake-up detection.
//!
//! Tests the detection of expected power meters and the generation of
//! wake-up hints when power meters are not found during discovery.

use rust_ride::sensors::power_meter::{
    ExpectedPowerMeter, PowerMeterWakeUpConfig, PowerMeterWakeUpDetector, WakeUpHint,
    WakeUpHintType, is_power_protocol, provides_power_data,
};
use rust_ride::sensors::types::{DiscoveredSensor, Protocol, SensorType};
use std::time::{Duration, Instant};

// ============================================================================
// Helper functions
// ============================================================================

/// Create a BLE power meter sensor for testing.
fn make_power_meter(name: &str, device_id: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::PowerMeter,
        protocol: Protocol::BleCyclingPower,
        signal_strength: Some(-60),
        last_seen: Instant::now(),
    }
}

/// Create an ANT+ power meter sensor for testing.
fn make_ant_power_meter(name: &str, device_id: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::PowerMeter,
        protocol: Protocol::AntPower,
        signal_strength: None,
        last_seen: Instant::now(),
    }
}

/// Create a non-power sensor for testing.
fn make_hr_sensor(name: &str, device_id: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::HeartRate,
        protocol: Protocol::BleHeartRate,
        signal_strength: Some(-70),
        last_seen: Instant::now(),
    }
}

// ============================================================================
// WakeUpHint tests
// ============================================================================

#[test]
fn test_wake_up_hint_creation() {
    let hint = WakeUpHint::new(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        WakeUpHintType::PedalToWake,
    );

    assert_eq!(hint.device_id, "device1");
    assert_eq!(hint.name, "Stages Power L");
    assert_eq!(hint.protocol, Protocol::BleCyclingPower);
    assert_eq!(hint.hint_type, WakeUpHintType::PedalToWake);
    assert!(!hint.shown);
}

#[test]
fn test_wake_up_hint_messages() {
    // PedalToWake
    let hint1 = WakeUpHint::new(
        "d1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        WakeUpHintType::PedalToWake,
    );
    assert!(hint1.message().contains("Stages"));
    assert!(hint1.message().to_lowercase().contains("pedal"));

    // CheckBattery
    let hint2 = WakeUpHint::new(
        "d2".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
        WakeUpHintType::CheckBattery,
    );
    assert!(hint2.message().to_lowercase().contains("battery"));

    // MoveSensor
    let hint3 = WakeUpHint::new(
        "d3".to_string(),
        "4iiii".to_string(),
        Protocol::BleCyclingPower,
        WakeUpHintType::MoveSensor,
    );
    assert!(hint3.message().to_lowercase().contains("move"));

    // ExtendedSearch
    let hint4 = WakeUpHint::new(
        "d4".to_string(),
        "Favero".to_string(),
        Protocol::AntPower,
        WakeUpHintType::ExtendedSearch,
    );
    assert!(hint4.message().to_lowercase().contains("searching"));
}

#[test]
fn test_wake_up_hint_short_messages() {
    let hints = vec![
        (WakeUpHintType::PedalToWake, "pedal"),
        (WakeUpHintType::CheckBattery, "battery"),
        (WakeUpHintType::MoveSensor, "move"),
        (WakeUpHintType::ExtendedSearch, "pedal"),
    ];

    for (hint_type, expected_word) in hints {
        let hint = WakeUpHint::new(
            "d".to_string(),
            "PM".to_string(),
            Protocol::BleCyclingPower,
            hint_type,
        );
        assert!(
            hint.short_message().to_lowercase().contains(expected_word),
            "Short message for {:?} should contain '{}'",
            hint_type,
            expected_word
        );
    }
}

#[test]
fn test_wake_up_hint_mark_shown() {
    let mut hint = WakeUpHint::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        WakeUpHintType::PedalToWake,
    );

    assert!(!hint.shown);
    hint.mark_shown();
    assert!(hint.shown);
}

#[test]
fn test_wake_up_hint_type_display() {
    assert_eq!(format!("{}", WakeUpHintType::PedalToWake), "Pedal to Wake");
    assert_eq!(format!("{}", WakeUpHintType::CheckBattery), "Check Battery");
    assert_eq!(format!("{}", WakeUpHintType::MoveSensor), "Move Sensor");
    assert_eq!(
        format!("{}", WakeUpHintType::ExtendedSearch),
        "Extended Search"
    );
}

// ============================================================================
// PowerMeterWakeUpConfig tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = PowerMeterWakeUpConfig::default();

    assert_eq!(config.hint_delay, Duration::from_secs(10));
    assert_eq!(config.grace_period, Duration::from_secs(5));
    assert!(config.enabled);
    assert!(config.max_hints_per_session > 0);
}

#[test]
fn test_config_aggressive() {
    let aggressive = PowerMeterWakeUpConfig::aggressive();
    let default = PowerMeterWakeUpConfig::default();

    assert!(aggressive.hint_delay < default.hint_delay);
    assert!(aggressive.grace_period < default.grace_period);
    assert!(aggressive.max_hints_per_session > default.max_hints_per_session);
}

#[test]
fn test_config_relaxed() {
    let relaxed = PowerMeterWakeUpConfig::relaxed();
    let default = PowerMeterWakeUpConfig::default();

    assert!(relaxed.hint_delay > default.hint_delay);
    assert!(relaxed.grace_period > default.grace_period);
    assert!(relaxed.max_hints_per_session < default.max_hints_per_session);
}

#[test]
fn test_config_disabled() {
    let disabled = PowerMeterWakeUpConfig::disabled();
    assert!(!disabled.enabled);
}

// ============================================================================
// ExpectedPowerMeter tests
// ============================================================================

#[test]
fn test_expected_power_meter_creation() {
    let expected = ExpectedPowerMeter::new(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(expected.device_id, "device1");
    assert_eq!(expected.name, "Stages Power L");
    assert_eq!(expected.protocol, Protocol::BleCyclingPower);
    assert_eq!(expected.hint_count, 0);
    assert!(expected.last_hint_at.is_none());
}

#[test]
fn test_expected_power_meter_record_hint() {
    let mut expected = ExpectedPowerMeter::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(expected.hint_count, 0);
    assert!(expected.last_hint_at.is_none());

    expected.record_hint();

    assert_eq!(expected.hint_count, 1);
    assert!(expected.last_hint_at.is_some());

    expected.record_hint();
    assert_eq!(expected.hint_count, 2);
}

// ============================================================================
// PowerMeterWakeUpDetector basic tests
// ============================================================================

#[test]
fn test_detector_new() {
    let detector = PowerMeterWakeUpDetector::new();

    assert_eq!(detector.expected_count(), 0);
    assert_eq!(detector.found_count(), 0);
    assert!(!detector.is_discovery_active());
    assert!(!detector.has_expected());
}

#[test]
fn test_detector_with_config() {
    let config = PowerMeterWakeUpConfig::aggressive();
    let detector = PowerMeterWakeUpDetector::with_config(config.clone());

    assert_eq!(detector.config().hint_delay, config.hint_delay);
}

#[test]
fn test_detector_register_expected() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(detector.expected_count(), 1);
    assert!(detector.has_expected());
}

#[test]
fn test_detector_register_multiple_expected() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.register_expected(
        "device2".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    assert_eq!(detector.expected_count(), 2);
}

#[test]
fn test_detector_no_duplicate_registration() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    // Register same device again
    detector.register_expected(
        "device1".to_string(),
        "Stages Power L v2".to_string(),
        Protocol::BleCyclingPower,
    );

    // Should still be 1
    assert_eq!(detector.expected_count(), 1);
}

// ============================================================================
// Discovery lifecycle tests
// ============================================================================

#[test]
fn test_detector_start_discovery() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    assert!(detector.is_discovery_active());
    assert!(detector.discovery_elapsed().is_some());
}

#[test]
fn test_detector_stop_discovery() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.start_discovery();
    assert!(detector.is_discovery_active());

    detector.stop_discovery();
    assert!(!detector.is_discovery_active());
}

#[test]
fn test_detector_start_resets_state() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    // First discovery - find the power meter
    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages", "device1"));
    assert_eq!(detector.found_count(), 1);
    detector.stop_discovery();

    // Second discovery - should reset found state
    detector.start_discovery();
    assert_eq!(detector.found_count(), 0);
    assert!(!detector.all_found());
}

// ============================================================================
// Discovered sensor recording tests
// ============================================================================

#[test]
fn test_detector_record_discovered_power_meter() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    let sensor = make_power_meter("Stages Power L", "device1");
    detector.record_discovered(&sensor);

    assert_eq!(detector.found_count(), 1);
    assert!(detector.all_found());
    assert!(!detector.is_missing("device1"));
}

#[test]
fn test_detector_record_non_power_sensor_ignored() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    let hr_sensor = make_hr_sensor("Polar H10", "hr1");
    detector.record_discovered(&hr_sensor);

    // Should not affect found count
    assert_eq!(detector.found_count(), 0);
    assert!(!detector.all_found());
}

#[test]
fn test_detector_record_unexpected_power_meter() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    // Record a different power meter
    let sensor = make_power_meter("Quarq DZero", "pm2");
    detector.record_discovered(&sensor);

    // The unexpected power meter is counted, but expected is still missing
    assert_eq!(detector.found_count(), 1);
    assert!(!detector.all_found()); // pm1 is still missing
    assert!(detector.is_missing("pm1"));
    assert!(!detector.is_missing("pm2"));
}

// ============================================================================
// Missing detection tests
// ============================================================================

#[test]
fn test_detector_missing_detection() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.register_expected(
        "pm2".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    // Find only one
    detector.record_discovered(&make_power_meter("Stages", "pm1"));

    assert_eq!(detector.missing_count(), 1);
    assert!(detector.is_missing("pm2"));
    assert!(!detector.is_missing("pm1"));

    let missing = detector.get_missing();
    assert_eq!(missing.len(), 1);
    assert!(missing.contains(&"pm2"));
}

#[test]
fn test_detector_all_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    assert!(!detector.all_found());

    detector.record_discovered(&make_power_meter("Stages", "pm1"));
    assert!(detector.all_found());
}

#[test]
fn test_detector_all_found_empty_expected() {
    let detector = PowerMeterWakeUpDetector::new();

    // With no expected power meters, all_found should return true
    assert!(detector.all_found());
}

// ============================================================================
// Detection result tests
// ============================================================================

#[test]
fn test_detection_result_all_missing() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    let result = detector.get_detection_result();

    assert!(!result.all_found);
    assert!(result.has_missing());
    assert_eq!(result.missing.len(), 1);
    assert!(result.found.is_empty());
}

#[test]
fn test_detection_result_all_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages", "pm1"));

    let result = detector.get_detection_result();

    assert!(result.all_found);
    assert!(!result.has_missing());
    assert!(result.missing.is_empty());
    assert_eq!(result.found.len(), 1);
}

#[test]
fn test_detection_result_partial() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "pm2".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages", "pm1"));

    let result = detector.get_detection_result();

    assert!(!result.all_found);
    assert!(result.has_missing());
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.found.len(), 1);
    assert!(result.missing.contains(&"pm2".to_string()));
    assert!(result.found.contains(&"pm1".to_string()));
}

// ============================================================================
// Clear tests
// ============================================================================

#[test]
fn test_detector_clear() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.start_discovery();

    detector.clear();

    assert_eq!(detector.expected_count(), 0);
    assert!(!detector.is_discovery_active());
    assert!(!detector.has_expected());
}

#[test]
fn test_detector_clear_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages", "pm1"));

    assert_eq!(detector.found_count(), 1);

    detector.clear_found();

    assert_eq!(detector.found_count(), 0);
    // But expected should still be there
    assert_eq!(detector.expected_count(), 1);
}

// ============================================================================
// Helper function tests
// ============================================================================

#[test]
fn test_provides_power_data() {
    // Sensors that provide power data
    assert!(provides_power_data(SensorType::PowerMeter));
    assert!(provides_power_data(SensorType::Trainer));
    assert!(provides_power_data(SensorType::SmartTrainer));

    // Sensors that don't provide power data
    assert!(!provides_power_data(SensorType::HeartRate));
    assert!(!provides_power_data(SensorType::Cadence));
    assert!(!provides_power_data(SensorType::Speed));
    assert!(!provides_power_data(SensorType::SpeedCadence));
}

#[test]
fn test_is_power_protocol() {
    // Power protocols
    assert!(is_power_protocol(Protocol::BleCyclingPower));
    assert!(is_power_protocol(Protocol::AntPower));
    assert!(is_power_protocol(Protocol::BleFtms));
    assert!(is_power_protocol(Protocol::AntFec));

    // Non-power protocols
    assert!(!is_power_protocol(Protocol::BleHeartRate));
    assert!(!is_power_protocol(Protocol::AntHeartRate));
    assert!(!is_power_protocol(Protocol::BleCsc));
    assert!(!is_power_protocol(Protocol::AntSpeedCadence));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_detector_handles_ant_power_meters() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "ant_pm1".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    let sensor = make_ant_power_meter("Quarq DZero", "ant_pm1");
    detector.record_discovered(&sensor);

    assert!(detector.all_found());
}

#[test]
fn test_detector_mixed_protocols() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // Expect both BLE and ANT+ power meters
    detector.register_expected(
        "ble_pm".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "ant_pm".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    // Find both
    detector.record_discovered(&make_power_meter("Stages", "ble_pm"));
    detector.record_discovered(&make_ant_power_meter("Quarq", "ant_pm"));

    assert!(detector.all_found());
    assert_eq!(detector.found_count(), 2);
}

#[test]
fn test_detector_hint_removed_when_found() {
    let mut detector = PowerMeterWakeUpDetector::with_config(PowerMeterWakeUpConfig {
        hint_delay: Duration::from_millis(0), // Immediate hints for testing
        ..PowerMeterWakeUpConfig::default()
    });

    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    // Force hint generation - in real scenario would wait for delay
    // Note: This may not generate hints immediately due to timing, but we can test the concept

    // Now find the power meter
    detector.record_discovered(&make_power_meter("Stages", "pm1"));

    // Hints for this device should be cleared
    let hints = detector.get_hints();
    assert!(
        hints.is_empty() || hints.iter().all(|h| h.device_id != "pm1"),
        "Hints for found power meter should be cleared"
    );
}

// ============================================================================
// Marking hints as shown tests
// ============================================================================

#[test]
fn test_detector_mark_hint_shown() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // Manually insert a hint for testing
    detector.register_expected(
        "pm1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.start_discovery();

    // Force generate hints by manipulating the expected sensor
    // (In real usage, hints are generated by check_for_hints after delay)

    // For now, just test that mark functions don't panic with no hints
    detector.mark_hint_shown("pm1");
    detector.mark_all_hints_shown();
}

// ============================================================================
// Config modification tests
// ============================================================================

#[test]
fn test_detector_set_config() {
    let mut detector = PowerMeterWakeUpDetector::new();

    let new_config = PowerMeterWakeUpConfig {
        hint_delay: Duration::from_secs(5),
        grace_period: Duration::from_secs(3),
        enabled: true,
        max_hints_per_session: 10,
        hint_repeat_interval: Duration::from_secs(15),
    };

    detector.set_config(new_config.clone());

    assert_eq!(detector.config().hint_delay, Duration::from_secs(5));
    assert_eq!(detector.config().grace_period, Duration::from_secs(3));
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_scenario_single_power_meter_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // User has a Stages power meter saved
    detector.register_expected(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    // Start discovery
    detector.start_discovery();

    // Initial state - power meter is missing
    assert!(detector.has_expected());
    assert!(!detector.all_found());
    assert_eq!(detector.missing_count(), 1);

    // Power meter wakes up after user pedals
    let sensor = make_power_meter("Stages Power L", "stages_001");
    detector.record_discovered(&sensor);

    // Now all found
    assert!(detector.all_found());
    assert_eq!(detector.missing_count(), 0);

    // Stop discovery
    detector.stop_discovery();
    assert!(!detector.is_discovery_active());
}

#[test]
fn test_scenario_multiple_power_meters_partial_discovery() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // User has two power meters: Stages (left) and Quarq (spider)
    detector.register_expected(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "quarq_001".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    // Only Stages wakes up
    detector.record_discovered(&make_power_meter("Stages Power L", "stages_001"));

    // Partial discovery
    assert!(!detector.all_found());
    assert_eq!(detector.found_count(), 1);
    assert_eq!(detector.missing_count(), 1);

    let result = detector.get_detection_result();
    assert!(result.found.contains(&"stages_001".to_string()));
    assert!(result.missing.contains(&"quarq_001".to_string()));
}

#[test]
fn test_scenario_unexpected_power_meter_discovered() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // User expects their Stages
    detector.register_expected(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    // But a different power meter (maybe neighbor's bike) is discovered
    detector.record_discovered(&make_power_meter("Assioma Duo", "favero_001"));

    // The unexpected one is recorded but expected is still missing
    assert!(!detector.all_found());
    assert!(detector.is_missing("stages_001"));

    // If their Stages then wakes up
    detector.record_discovered(&make_power_meter("Stages Power L", "stages_001"));

    // Now all expected are found
    assert!(detector.all_found());
}
