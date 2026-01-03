//! Unit tests for extended power meter discovery.
//!
//! Tests the extended discovery configuration and behavior when power meters
//! are expected but not found, allowing discovery to extend up to 45 seconds.

use rust_ride::sensors::power_meter::{
    ExtendedDiscoveryDecision, ExtendedPowerMeterDiscoveryConfig,
    PowerMeterWakeUpConfig, PowerMeterWakeUpDetector,
    DEFAULT_EXTENDED_DISCOVERY_SECS, DEFAULT_STANDARD_DISCOVERY_SECS,
    EXTENDED_DISCOVERY_THRESHOLD_SECS,
};
use rust_ride::sensors::types::{DiscoveredSensor, Protocol, ProgressiveTimeoutConfig, SensorType};
use std::time::{Duration, Instant};

// ============================================================================
// Helper functions
// ============================================================================

/// Create a power meter sensor for testing.
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

/// Create a heart rate sensor for testing.
fn make_hr_sensor(name: &str, device_id: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::HeartRate,
        protocol: Protocol::BleHeartRate,
        signal_strength: Some(-65),
        last_seen: Instant::now(),
    }
}

// ============================================================================
// ExtendedPowerMeterDiscoveryConfig tests
// ============================================================================

#[test]
fn test_extended_config_default() {
    let config = ExtendedPowerMeterDiscoveryConfig::default();

    assert!(config.enabled);
    assert_eq!(config.standard_timeout_secs, DEFAULT_STANDARD_DISCOVERY_SECS);
    assert_eq!(config.extended_timeout_secs, DEFAULT_EXTENDED_DISCOVERY_SECS);
    assert_eq!(config.extension_threshold_secs, EXTENDED_DISCOVERY_THRESHOLD_SECS);

    // Verify default values
    assert_eq!(config.standard_timeout_secs, 30);
    assert_eq!(config.extended_timeout_secs, 45);
    assert_eq!(config.extension_threshold_secs, 15);
}

#[test]
fn test_extended_config_disabled() {
    let config = ExtendedPowerMeterDiscoveryConfig::disabled();
    assert!(!config.enabled);
}

#[test]
fn test_extended_config_aggressive() {
    let config = ExtendedPowerMeterDiscoveryConfig::aggressive();

    assert!(config.enabled);
    assert_eq!(config.standard_timeout_secs, 30);
    assert_eq!(config.extended_timeout_secs, 60);
    assert_eq!(config.extension_threshold_secs, 10);
}

#[test]
fn test_extended_config_minimal() {
    let config = ExtendedPowerMeterDiscoveryConfig::minimal();

    assert!(config.enabled);
    assert_eq!(config.standard_timeout_secs, 30);
    assert_eq!(config.extended_timeout_secs, 35);
    assert_eq!(config.extension_threshold_secs, 20);
}

#[test]
fn test_extended_config_extension_time() {
    let config = ExtendedPowerMeterDiscoveryConfig::default();

    let extension_time = config.extension_time();
    // 45 - 30 = 15 seconds
    assert_eq!(extension_time, Duration::from_secs(15));
}

#[test]
fn test_extended_config_should_extend() {
    let config = ExtendedPowerMeterDiscoveryConfig::default();

    // Before threshold
    assert!(!config.should_extend(Duration::from_secs(10)));
    assert!(!config.should_extend(Duration::from_secs(14)));

    // At threshold
    assert!(config.should_extend(Duration::from_secs(15)));

    // After threshold
    assert!(config.should_extend(Duration::from_secs(20)));
    assert!(config.should_extend(Duration::from_secs(30)));
}

#[test]
fn test_extended_config_should_extend_disabled() {
    let config = ExtendedPowerMeterDiscoveryConfig::disabled();

    // Should never extend when disabled
    assert!(!config.should_extend(Duration::from_secs(0)));
    assert!(!config.should_extend(Duration::from_secs(15)));
    assert!(!config.should_extend(Duration::from_secs(100)));
}

// ============================================================================
// ProgressiveTimeoutConfig power meter extension tests
// ============================================================================

#[test]
fn test_progressive_timeout_has_power_meter_max() {
    let config = ProgressiveTimeoutConfig::default();

    assert_eq!(config.max_total_secs, 30);
    assert_eq!(config.power_meter_max_secs, 45);
}

#[test]
fn test_progressive_timeout_fast_power_meter_max() {
    let config = ProgressiveTimeoutConfig::fast();

    assert_eq!(config.max_total_secs, 15);
    assert_eq!(config.power_meter_max_secs, 30);
}

#[test]
fn test_progressive_timeout_thorough_power_meter_max() {
    let config = ProgressiveTimeoutConfig::thorough();

    assert_eq!(config.max_total_secs, 45);
    assert_eq!(config.power_meter_max_secs, 60);
}

// ============================================================================
// PowerMeterWakeUpDetector extended discovery tests
// ============================================================================

#[test]
fn test_detector_with_extended_config() {
    let wake_config = PowerMeterWakeUpConfig::default();
    let extended_config = ExtendedPowerMeterDiscoveryConfig::aggressive();

    let detector = PowerMeterWakeUpDetector::with_extended_config(wake_config, extended_config);

    assert_eq!(detector.extended_discovery_config().extended_timeout_secs, 60);
}

#[test]
fn test_detector_set_extended_discovery_config() {
    let mut detector = PowerMeterWakeUpDetector::new();

    let config = ExtendedPowerMeterDiscoveryConfig::minimal();
    detector.set_extended_discovery_config(config);

    assert_eq!(detector.extended_discovery_config().extended_timeout_secs, 35);
}

#[test]
fn test_detector_should_use_extended_discovery_no_expected() {
    let detector = PowerMeterWakeUpDetector::new();

    // No expected power meters, should not extend
    assert!(!detector.should_use_extended_discovery());
}

#[test]
fn test_detector_should_use_extended_discovery_all_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages Power", "pm1"));

    // All found, should not extend
    assert!(!detector.should_use_extended_discovery());
}

#[test]
fn test_detector_get_recommended_timeout_no_expected() {
    let detector = PowerMeterWakeUpDetector::new();

    // No expected power meters, use standard timeout
    assert_eq!(detector.get_recommended_timeout_secs(), 30);
}

#[test]
fn test_detector_get_recommended_timeout_with_expected() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    // Has expected power meters, use extended timeout
    assert_eq!(detector.get_recommended_timeout_secs(), 45);
}

#[test]
fn test_detector_get_recommended_timeout_disabled() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.set_extended_discovery_config(ExtendedPowerMeterDiscoveryConfig::disabled());

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    // Extended discovery disabled, use standard timeout
    assert_eq!(detector.get_recommended_timeout_secs(), 30);
}

// ============================================================================
// ExtendedDiscoveryDecision tests
// ============================================================================

#[test]
fn test_decision_disabled() {
    let mut detector = PowerMeterWakeUpDetector::new();
    detector.set_extended_discovery_config(ExtendedPowerMeterDiscoveryConfig::disabled());

    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::Disabled);
}

#[test]
fn test_decision_no_expected() {
    let detector = PowerMeterWakeUpDetector::new();

    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::UseStandardTimeout);
}

#[test]
fn test_decision_all_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages Power", "pm1"));

    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::UseStandardTimeout);
}

#[test]
fn test_decision_extend_for_power_meters() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    let decision = detector.get_extended_discovery_decision();

    match decision {
        ExtendedDiscoveryDecision::ExtendForPowerMeters {
            waiting_for,
            extended_timeout_secs,
        } => {
            assert_eq!(waiting_for.len(), 1);
            assert!(waiting_for.contains(&"Stages Power L".to_string()));
            assert_eq!(extended_timeout_secs, 45);
        }
        _ => panic!("Expected ExtendForPowerMeters decision"),
    }
}

#[test]
fn test_decision_extend_multiple_power_meters() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "pm2".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    let decision = detector.get_extended_discovery_decision();

    match decision {
        ExtendedDiscoveryDecision::ExtendForPowerMeters {
            waiting_for,
            extended_timeout_secs,
        } => {
            assert_eq!(waiting_for.len(), 2);
            assert!(waiting_for.contains(&"Stages Power L".to_string()));
            assert!(waiting_for.contains(&"Quarq DZero".to_string()));
            assert_eq!(extended_timeout_secs, 45);
        }
        _ => panic!("Expected ExtendForPowerMeters decision"),
    }
}

#[test]
fn test_decision_partial_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "pm2".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages Power L", "pm1"));

    let decision = detector.get_extended_discovery_decision();

    match decision {
        ExtendedDiscoveryDecision::ExtendForPowerMeters {
            waiting_for,
            extended_timeout_secs,
        } => {
            assert_eq!(waiting_for.len(), 1);
            assert!(waiting_for.contains(&"Quarq DZero".to_string()));
            assert!(!waiting_for.contains(&"Stages Power L".to_string()));
            assert_eq!(extended_timeout_secs, 45);
        }
        _ => panic!("Expected ExtendForPowerMeters decision"),
    }
}

// ============================================================================
// Extended discovery triggered tests
// ============================================================================

#[test]
fn test_extended_discovery_triggered_initially_false() {
    let detector = PowerMeterWakeUpDetector::new();
    assert!(!detector.is_extended_discovery_triggered());
}

#[test]
fn test_mark_extended_discovery_triggered() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    assert!(!detector.is_extended_discovery_triggered());

    detector.mark_extended_discovery_triggered();
    assert!(detector.is_extended_discovery_triggered());
}

#[test]
fn test_extended_discovery_reset_on_new_discovery() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.mark_extended_discovery_triggered();
    assert!(detector.is_extended_discovery_triggered());

    // Start new discovery session
    detector.start_discovery();
    assert!(!detector.is_extended_discovery_triggered());
}

#[test]
fn test_extended_discovery_reset_on_clear() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.mark_extended_discovery_triggered();
    assert!(detector.is_extended_discovery_triggered());

    detector.clear();
    assert!(!detector.is_extended_discovery_triggered());
}

// ============================================================================
// Get missing power meter names tests
// ============================================================================

#[test]
fn test_get_missing_power_meter_names_empty() {
    let detector = PowerMeterWakeUpDetector::new();

    let names = detector.get_missing_power_meter_names();
    assert!(names.is_empty());
}

#[test]
fn test_get_missing_power_meter_names_all_missing() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "pm2".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();

    let names = detector.get_missing_power_meter_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Stages Power L".to_string()));
    assert!(names.contains(&"Quarq DZero".to_string()));
}

#[test]
fn test_get_missing_power_meter_names_partial() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );
    detector.register_expected(
        "pm2".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages Power L", "pm1"));

    let names = detector.get_missing_power_meter_names();
    assert_eq!(names.len(), 1);
    assert!(names.contains(&"Quarq DZero".to_string()));
    assert!(!names.contains(&"Stages Power L".to_string()));
}

#[test]
fn test_get_missing_power_meter_names_all_found() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();
    detector.record_discovered(&make_power_meter("Stages Power L", "pm1"));

    let names = detector.get_missing_power_meter_names();
    assert!(names.is_empty());
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_scenario_single_power_meter_extended_discovery() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // User has a Stages power meter saved
    detector.register_expected(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    // Start discovery
    detector.start_discovery();

    // Initially, extended discovery should be recommended
    let decision = detector.get_extended_discovery_decision();
    match decision {
        ExtendedDiscoveryDecision::ExtendForPowerMeters { waiting_for, .. } => {
            assert!(waiting_for.contains(&"Stages Power L".to_string()));
        }
        _ => panic!("Expected ExtendForPowerMeters"),
    }

    // Simulate time passing and extended discovery being triggered
    detector.mark_extended_discovery_triggered();
    assert!(detector.is_extended_discovery_triggered());

    // Power meter eventually wakes up and is found
    detector.record_discovered(&make_power_meter("Stages Power L", "stages_001"));

    // Now decision should be standard timeout
    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::UseStandardTimeout);

    // Stop discovery
    detector.stop_discovery();
    assert!(!detector.is_discovery_active());
}

#[test]
fn test_scenario_extended_discovery_with_hr_sensor() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // User expects a power meter
    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    detector.start_discovery();

    // HR sensor is found first (not a power meter)
    detector.record_discovered(&make_hr_sensor("Polar H10", "hr1"));

    // Should still want extended discovery for the power meter
    let decision = detector.get_extended_discovery_decision();
    match decision {
        ExtendedDiscoveryDecision::ExtendForPowerMeters { waiting_for, .. } => {
            assert!(waiting_for.contains(&"Stages Power".to_string()));
        }
        _ => panic!("Expected ExtendForPowerMeters"),
    }

    // Power meter found later
    detector.record_discovered(&make_power_meter("Stages Power", "pm1"));

    // Now standard timeout is fine
    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::UseStandardTimeout);
}

#[test]
fn test_scenario_no_power_meter_expected() {
    let mut detector = PowerMeterWakeUpDetector::new();

    // No power meter expected
    detector.start_discovery();

    // Should not extend
    let decision = detector.get_extended_discovery_decision();
    assert_eq!(decision, ExtendedDiscoveryDecision::UseStandardTimeout);
    assert_eq!(detector.get_recommended_timeout_secs(), 30);
}

#[test]
fn test_scenario_discovery_restart() {
    let mut detector = PowerMeterWakeUpDetector::new();

    detector.register_expected(
        "pm1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    // First discovery session
    detector.start_discovery();
    detector.mark_extended_discovery_triggered();
    detector.record_discovered(&make_power_meter("Stages Power", "pm1"));
    detector.stop_discovery();

    // Second discovery session
    detector.start_discovery();

    // Extended discovery should not be triggered yet
    assert!(!detector.is_extended_discovery_triggered());

    // Power meter needs to be found again
    assert!(!detector.all_found());
}
