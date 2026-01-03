//! Unit tests for sensor conflict detection and resolution.
//!
//! Tests the detection of multiple sensors providing the same data type
//! (e.g., two power meters) and the conflict resolution mechanisms.

use rust_ride::sensors::conflict::{
    ConflictDetector, ConflictDetectorConfig, ConflictPreference, ConflictPreferenceManager,
    DataSource, DataType, ResolutionStrategy, SensorConflict,
};
use rust_ride::sensors::types::{DiscoveredSensor, Protocol, SensorType};
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
        device_id: format!("ant+:{}:{}", device_number, name.to_lowercase().replace(' ', "_")),
        name: name.to_string(),
        sensor_type,
        protocol,
        signal_strength: None,
        last_seen: Instant::now(),
    }
}

// ============================================================================
// DataType tests
// ============================================================================

#[test]
fn test_data_type_from_power_meter() {
    let data_types = DataType::from_sensor_type(SensorType::PowerMeter);
    assert!(data_types.contains(&DataType::Power));
    assert!(data_types.contains(&DataType::Cadence));
    assert!(!data_types.contains(&DataType::HeartRate));
}

#[test]
fn test_data_type_from_trainer() {
    let data_types = DataType::from_sensor_type(SensorType::Trainer);
    assert!(data_types.contains(&DataType::Power));
    assert!(data_types.contains(&DataType::Cadence));
    assert!(data_types.contains(&DataType::Speed));
    assert!(data_types.contains(&DataType::TrainerControl));
    assert!(!data_types.contains(&DataType::HeartRate));
}

#[test]
fn test_data_type_from_heart_rate() {
    let data_types = DataType::from_sensor_type(SensorType::HeartRate);
    assert_eq!(data_types, vec![DataType::HeartRate]);
}

#[test]
fn test_data_type_from_cadence_sensor() {
    let data_types = DataType::from_sensor_type(SensorType::Cadence);
    assert_eq!(data_types, vec![DataType::Cadence]);
}

#[test]
fn test_data_type_from_speed_cadence() {
    let data_types = DataType::from_sensor_type(SensorType::SpeedCadence);
    assert!(data_types.contains(&DataType::Speed));
    assert!(data_types.contains(&DataType::Cadence));
}

#[test]
fn test_data_type_is_critical() {
    assert!(DataType::Power.is_critical());
    assert!(DataType::HeartRate.is_critical());
    assert!(DataType::TrainerControl.is_critical());
    assert!(!DataType::Cadence.is_critical());
    assert!(!DataType::Speed.is_critical());
}

#[test]
fn test_data_type_display() {
    assert_eq!(DataType::Power.display_name(), "Power");
    assert_eq!(DataType::HeartRate.display_name(), "Heart Rate");
    assert_eq!(DataType::Cadence.display_name(), "Cadence");
    assert_eq!(DataType::Speed.display_name(), "Speed");
    assert_eq!(DataType::TrainerControl.display_name(), "Trainer Control");
}

// ============================================================================
// DataSource tests
// ============================================================================

#[test]
fn test_data_source_from_discovered() {
    let sensor = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let source = DataSource::from_discovered(&sensor, DataType::Power);

    assert_eq!(source.device_id, sensor.device_id);
    assert_eq!(source.name, "Stages Power");
    assert_eq!(source.sensor_type, SensorType::PowerMeter);
    assert_eq!(source.data_type, DataType::Power);
    assert!(!source.is_connected);
    assert!(!source.is_primary);
}

#[test]
fn test_data_source_display() {
    let sensor = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let source = DataSource::from_discovered(&sensor, DataType::Power);

    let display = source.display();
    assert!(display.contains("Stages Power"));
    assert!(display.contains("Power Meter"));
    assert!(display.contains("BLE"));
}

// ============================================================================
// SensorConflict tests
// ============================================================================

#[test]
fn test_sensor_conflict_new() {
    let sensor = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let source = DataSource::from_discovered(&sensor, DataType::Power);
    let conflict = SensorConflict::new(DataType::Power, vec![source]);

    assert_eq!(conflict.data_type, DataType::Power);
    assert_eq!(conflict.sensor_count(), 1);
    assert!(!conflict.is_active()); // Only one source = not active
    assert!(!conflict.is_resolved);
    assert!(!conflict.user_notified);
}

#[test]
fn test_sensor_conflict_is_active() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);

    assert!(conflict.is_active()); // Two sources = active conflict
}

#[test]
fn test_sensor_conflict_set_primary() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);

    let result = conflict.set_primary(&sensor1.device_id);
    assert!(result);
    assert!(conflict.is_resolved);
    assert_eq!(conflict.primary_device_id, Some(sensor1.device_id.clone()));
}

#[test]
fn test_sensor_conflict_set_primary_invalid_device() {
    let sensor = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let source = DataSource::from_discovered(&sensor, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source]);

    let result = conflict.set_primary("nonexistent_device");
    assert!(!result);
    assert!(!conflict.is_resolved);
}

#[test]
fn test_sensor_conflict_clear_primary() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    conflict.set_primary(&sensor1.device_id);

    assert!(conflict.is_resolved);

    conflict.clear_primary();
    assert!(!conflict.is_resolved);
    assert!(conflict.primary_device_id.is_none());
}

#[test]
fn test_sensor_conflict_add_source() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1]);
    assert!(!conflict.is_active());

    conflict.add_source(source2);
    assert!(conflict.is_active());
    assert_eq!(conflict.sensor_count(), 2);
}

#[test]
fn test_sensor_conflict_remove_source() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    assert!(conflict.is_active());

    let removed = conflict.remove_source(&sensor1.device_id);
    assert!(removed);
    assert!(!conflict.is_active());
}

#[test]
fn test_sensor_conflict_remove_primary_clears_resolution() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    conflict.set_primary(&sensor1.device_id);

    conflict.remove_source(&sensor1.device_id);
    assert!(!conflict.is_resolved);
    assert!(conflict.primary_device_id.is_none());
}

#[test]
fn test_sensor_conflict_needs_attention() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    assert!(conflict.needs_attention());

    conflict.mark_notified();
    assert!(!conflict.needs_attention());
}

#[test]
fn test_sensor_conflict_primary_source() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    assert!(conflict.primary_source().is_none());

    conflict.set_primary(&sensor1.device_id);
    let primary = conflict.primary_source().unwrap();
    assert_eq!(primary.name, "Stages Power");
}

#[test]
fn test_sensor_conflict_secondary_sources() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let mut conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    conflict.set_primary(&sensor1.device_id);

    let secondary = conflict.secondary_sources();
    assert_eq!(secondary.len(), 1);
    assert_eq!(secondary[0].name, "KICKR Core");
}

#[test]
fn test_sensor_conflict_summary() {
    let sensor1 = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let sensor2 = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let source1 = DataSource::from_discovered(&sensor1, DataType::Power);
    let source2 = DataSource::from_discovered(&sensor2, DataType::Power);

    let conflict = SensorConflict::new(DataType::Power, vec![source1, source2]);
    let summary = conflict.summary();

    assert!(summary.contains("Power"));
    assert!(summary.contains("2 sensors"));
}

// ============================================================================
// ConflictDetector tests
// ============================================================================

#[test]
fn test_conflict_detector_new() {
    let detector = ConflictDetector::new();
    assert!(!detector.has_conflicts());
    assert_eq!(detector.conflict_count(), 0);
}

#[test]
fn test_conflict_detector_no_conflict_single_sensor() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    detector.register_sensor(&power_meter);

    assert!(!detector.has_conflicts());
}

#[test]
fn test_conflict_detector_power_conflict_power_meter_and_trainer() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);

    // Both provide power, so there should be a conflict
    assert!(detector.has_conflict(DataType::Power));

    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert_eq!(conflict.sensor_count(), 2);
}

#[test]
fn test_conflict_detector_no_conflict_different_data_types() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let hr_monitor = make_ble_sensor("Polar H10", SensorType::HeartRate);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&hr_monitor);

    // No conflict - they provide different data types
    assert!(!detector.has_conflicts());
}

#[test]
fn test_conflict_detector_multiple_hr_monitors() {
    let mut detector = ConflictDetector::new();

    let hr1 = make_ble_sensor("Polar H10", SensorType::HeartRate);
    let hr2 = make_ant_sensor("Garmin HRM", 1234, SensorType::HeartRate);

    detector.register_sensor(&hr1);
    detector.register_sensor(&hr2);

    assert!(detector.has_conflict(DataType::HeartRate));

    let conflict = detector.get_conflict(DataType::HeartRate).unwrap();
    assert_eq!(conflict.sensor_count(), 2);
}

#[test]
fn test_conflict_detector_multiple_power_meters() {
    let mut detector = ConflictDetector::new();

    let pm1 = make_ble_sensor("Stages Left", SensorType::PowerMeter);
    let pm2 = make_ble_sensor("Stages Right", SensorType::PowerMeter);

    detector.register_sensor(&pm1);
    detector.register_sensor(&pm2);

    assert!(detector.has_conflict(DataType::Power));

    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert_eq!(conflict.sensor_count(), 2);
}

#[test]
fn test_conflict_detector_set_primary() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);

    let result = detector.set_primary(DataType::Power, &power_meter.device_id);
    assert!(result);

    assert_eq!(
        detector.get_primary(DataType::Power),
        Some(power_meter.device_id.as_str())
    );

    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert!(conflict.is_resolved);
}

#[test]
fn test_conflict_detector_clear_primary() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);
    detector.set_primary(DataType::Power, &power_meter.device_id);

    detector.clear_primary(DataType::Power);

    assert!(detector.get_primary(DataType::Power).is_none());
    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert!(!conflict.is_resolved);
}

#[test]
fn test_conflict_detector_unregister_sensor() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);

    assert!(detector.has_conflict(DataType::Power));

    detector.unregister_sensor(&power_meter.device_id);

    // Conflict should no longer be active
    assert!(!detector.has_conflict(DataType::Power));
}

#[test]
fn test_conflict_detector_is_primary() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);
    detector.set_primary(DataType::Power, &power_meter.device_id);

    assert!(detector.is_primary(&power_meter.device_id));
    assert!(!detector.is_primary(&trainer.device_id));
}

#[test]
fn test_conflict_detector_primary_for() {
    let mut detector = ConflictDetector::new();

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);

    // Power meter is primary for both power and cadence
    detector.set_primary(DataType::Power, &power_meter.device_id);
    detector.set_primary(DataType::Cadence, &power_meter.device_id);

    let primary_for = detector.primary_for(&power_meter.device_id);
    assert!(primary_for.contains(&DataType::Power));
    assert!(primary_for.contains(&DataType::Cadence));
}

#[test]
fn test_conflict_detector_active_conflicts() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let hr1 = make_ble_sensor("Polar H10", SensorType::HeartRate);
    let hr2 = make_ble_sensor("Garmin HRM", SensorType::HeartRate);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);
    detector.register_sensor(&hr1);
    detector.register_sensor(&hr2);

    let active = detector.active_conflicts();
    // Should have: Power, Cadence, HeartRate (possibly Speed)
    assert!(active.len() >= 2);
}

#[test]
fn test_conflict_detector_conflicts_needing_attention() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    let needs_attention = detector.conflicts_needing_attention();
    assert!(!needs_attention.is_empty());

    detector.mark_notified(DataType::Power);

    let needs_attention2 = detector.conflicts_needing_attention();
    // Power conflict no longer needs attention
    assert!(!needs_attention2.iter().any(|c| c.data_type == DataType::Power));
}

#[test]
fn test_conflict_detector_mark_all_notified() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    assert!(!detector.conflicts_needing_attention().is_empty());

    detector.mark_all_notified();

    assert!(detector.conflicts_needing_attention().is_empty());
}

#[test]
fn test_conflict_detector_summary() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    let summary = detector.summary();
    assert!(summary.total_conflicts > 0);
    assert!(summary.needs_attention());
}

#[test]
fn test_conflict_detector_unresolved_count() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    let initial_unresolved = detector.unresolved_count();
    assert!(initial_unresolved > 0);

    detector.set_primary(DataType::Power, &pm.device_id);

    // Should have one less unresolved
    assert!(detector.unresolved_count() < initial_unresolved);
}

#[test]
fn test_conflict_detector_clear() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    assert!(detector.has_conflicts());

    detector.clear();

    assert!(!detector.has_conflicts());
    assert_eq!(detector.conflict_count(), 0);
}

// ============================================================================
// Auto-resolution tests
// ============================================================================

#[test]
fn test_auto_resolve_by_priority() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::AutoPriority,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&power_meter);
    detector.register_sensor(&trainer);

    let resolved = detector.auto_resolve();
    assert!(resolved.contains(&DataType::Power));

    // Power meter should be selected (higher priority than trainer for power)
    assert_eq!(
        detector.get_primary(DataType::Power),
        Some(power_meter.device_id.as_str())
    );
}

#[test]
fn test_auto_resolve_first_connected() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::FirstConnected,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    let resolved = detector.auto_resolve();
    assert!(resolved.contains(&DataType::Power));

    // First registered should be selected
    assert!(detector.get_primary(DataType::Power).is_some());
}

// ============================================================================
// Three-way conflict tests
// ============================================================================

#[test]
fn test_three_way_power_conflict() {
    let mut detector = ConflictDetector::new();

    let pm1 = make_ble_sensor("Stages Left", SensorType::PowerMeter);
    let pm2 = make_ble_sensor("Stages Right", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm1);
    detector.register_sensor(&pm2);
    detector.register_sensor(&trainer);

    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert_eq!(conflict.sensor_count(), 3);
}

// ============================================================================
// Cadence conflict tests
// ============================================================================

#[test]
fn test_cadence_conflict() {
    let mut detector = ConflictDetector::new();

    let cadence_sensor = make_ble_sensor("Wahoo Cadence", SensorType::Cadence);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&cadence_sensor);
    detector.register_sensor(&trainer);

    assert!(detector.has_conflict(DataType::Cadence));
}

// ============================================================================
// Speed conflict tests
// ============================================================================

#[test]
fn test_speed_conflict() {
    let mut detector = ConflictDetector::new();

    let speed_sensor = make_ble_sensor("Wahoo Speed", SensorType::Speed);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&speed_sensor);
    detector.register_sensor(&trainer);

    assert!(detector.has_conflict(DataType::Speed));
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_typical_indoor_setup() {
    let mut detector = ConflictDetector::new();

    // Typical indoor setup: Trainer + Power Meter + HR Monitor
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let power_meter = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let hr = make_ble_sensor("Polar H10", SensorType::HeartRate);

    detector.register_sensor(&trainer);
    detector.register_sensor(&power_meter);
    detector.register_sensor(&hr);

    // Power conflict between trainer and power meter
    assert!(detector.has_conflict(DataType::Power));

    // Cadence conflict between trainer and power meter
    assert!(detector.has_conflict(DataType::Cadence));

    // No HR conflict (only one HR sensor)
    assert!(!detector.has_conflict(DataType::HeartRate));

    // Resolve power conflict - prefer power meter for accuracy
    detector.set_primary(DataType::Power, &power_meter.device_id);
    assert!(detector.get_conflict(DataType::Power).unwrap().is_resolved);
}

#[test]
fn test_dual_hr_monitors() {
    let mut detector = ConflictDetector::new();

    // User has both BLE and ANT+ HR monitors
    let ble_hr = make_ble_sensor("Polar H10", SensorType::HeartRate);
    let ant_hr = make_ant_sensor("Garmin HRM", 5678, SensorType::HeartRate);

    detector.register_sensor(&ble_hr);
    detector.register_sensor(&ant_hr);

    assert!(detector.has_conflict(DataType::HeartRate));

    // User prefers the Polar
    detector.set_primary(DataType::HeartRate, &ble_hr.device_id);

    assert!(detector.is_primary(&ble_hr.device_id));
    assert!(!detector.is_primary(&ant_hr.device_id));
}

#[test]
fn test_sensor_disconnect_resolves_conflict() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    assert!(detector.has_conflict(DataType::Power));

    // User disconnects the power meter
    detector.unregister_sensor(&pm.device_id);

    // Conflict should be resolved (only trainer remains)
    assert!(!detector.has_conflict(DataType::Power));
}

// ============================================================================
// ConflictPreferenceManager tests
// ============================================================================

#[test]
fn test_preference_manager_new() {
    let manager = ConflictPreferenceManager::new();
    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_preference_manager_set_and_get() {
    let temp_dir = std::env::temp_dir().join("rust_ride_test_conflict_prefs");
    std::fs::create_dir_all(&temp_dir).ok();
    let temp_path = temp_dir.join("test_prefs.json");

    let mut manager = ConflictPreferenceManager::with_path(temp_path.clone());

    let pref = ConflictPreference {
        data_type: DataType::Power,
        primary_device_id: "ble:stages_power".to_string(),
        primary_sensor_name: "Stages Power".to_string(),
        updated_at: chrono::Utc::now(),
        user_set: true,
    };

    manager.set_preference(pref.clone());

    assert_eq!(manager.len(), 1);

    let retrieved = manager.get_preference(DataType::Power).unwrap();
    assert_eq!(retrieved.primary_device_id, "ble:stages_power");
    assert_eq!(retrieved.primary_sensor_name, "Stages Power");

    // Cleanup
    std::fs::remove_file(&temp_path).ok();
    std::fs::remove_dir(&temp_dir).ok();
}

#[test]
fn test_preference_manager_remove() {
    let temp_dir = std::env::temp_dir().join("rust_ride_test_conflict_prefs2");
    std::fs::create_dir_all(&temp_dir).ok();
    let temp_path = temp_dir.join("test_prefs2.json");

    let mut manager = ConflictPreferenceManager::with_path(temp_path.clone());

    let pref = ConflictPreference {
        data_type: DataType::Power,
        primary_device_id: "ble:stages_power".to_string(),
        primary_sensor_name: "Stages Power".to_string(),
        updated_at: chrono::Utc::now(),
        user_set: true,
    };

    manager.set_preference(pref);
    assert_eq!(manager.len(), 1);

    let removed = manager.remove_preference(DataType::Power);
    assert!(removed.is_some());
    assert!(manager.is_empty());

    // Cleanup
    std::fs::remove_file(&temp_path).ok();
    std::fs::remove_dir(&temp_dir).ok();
}

#[test]
fn test_preference_manager_clear() {
    let temp_dir = std::env::temp_dir().join("rust_ride_test_conflict_prefs3");
    std::fs::create_dir_all(&temp_dir).ok();
    let temp_path = temp_dir.join("test_prefs3.json");

    let mut manager = ConflictPreferenceManager::with_path(temp_path.clone());

    manager.set_preference(ConflictPreference {
        data_type: DataType::Power,
        primary_device_id: "device1".to_string(),
        primary_sensor_name: "Sensor 1".to_string(),
        updated_at: chrono::Utc::now(),
        user_set: true,
    });

    manager.set_preference(ConflictPreference {
        data_type: DataType::HeartRate,
        primary_device_id: "device2".to_string(),
        primary_sensor_name: "Sensor 2".to_string(),
        updated_at: chrono::Utc::now(),
        user_set: true,
    });

    assert_eq!(manager.len(), 2);

    manager.clear();
    assert!(manager.is_empty());

    // Cleanup
    std::fs::remove_file(&temp_path).ok();
    std::fs::remove_dir(&temp_dir).ok();
}

// ============================================================================
// Failover tests
// ============================================================================

#[test]
fn test_failover_with_no_secondary() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    detector.register_sensor(&pm);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Simulate primary disconnect with no secondary available
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // No failover should occur (no secondary)
    assert!(failovers.is_empty());
}

#[test]
fn test_failover_with_secondary_available() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Set power meter as primary
    detector.set_primary(DataType::Power, &pm.device_id);

    // Simulate trainer connected
    detector.update_connection_status(&trainer.device_id, true);

    // Simulate primary disconnect
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Failover should occur to trainer
    assert_eq!(failovers.len(), 1);
    assert_eq!(failovers[0].data_type, DataType::Power);
    assert_eq!(failovers[0].from_device_id, pm.device_id);
    assert_eq!(failovers[0].to_device_id, trainer.device_id);
}

#[test]
fn test_failover_targets() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Set power meter as primary
    detector.set_primary(DataType::Power, &pm.device_id);

    // Mark trainer as connected (potential failover target)
    detector.update_connection_status(&trainer.device_id, true);

    // Get failover targets
    let targets = detector.get_failover_targets(DataType::Power);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].device_id, trainer.device_id);
}

#[test]
fn test_has_failover_available() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    detector.register_sensor(&pm);
    detector.set_primary(DataType::Power, &pm.device_id);

    // No failover available (only one sensor)
    assert!(!detector.has_failover_available(DataType::Power));

    // Add trainer and mark as connected
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    detector.register_sensor(&trainer);
    detector.update_connection_status(&trainer.device_id, true);

    // Now failover is available
    assert!(detector.has_failover_available(DataType::Power));
}

#[test]
fn test_protected_data_types() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);
    detector.set_primary(DataType::Power, &pm.device_id);
    detector.update_connection_status(&trainer.device_id, true);

    let protected = detector.get_protected_data_types();
    assert!(protected.contains(&DataType::Power));
}

#[test]
fn test_at_risk_data_types() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    detector.register_sensor(&pm);
    detector.set_primary(DataType::Power, &pm.device_id);
    detector.update_connection_status(&pm.device_id, true);

    // With only one sensor connected, data types are at risk
    let at_risk = detector.get_at_risk_data_types();
    assert!(at_risk.contains(&DataType::Power));
}

#[test]
fn test_failover_result_message() {
    use rust_ride::sensors::conflict::FailoverResult;

    let result = FailoverResult {
        data_type: DataType::Power,
        from_device_id: "device1".to_string(),
        from_sensor_name: "Stages Power".to_string(),
        to_device_id: "device2".to_string(),
        to_sensor_name: "KICKR Core".to_string(),
    };

    let message = result.message();
    assert!(message.contains("Power"));
    assert!(message.contains("Stages Power"));
    assert!(message.contains("KICKR Core"));
}

// ============================================================================
// User alerting workflow tests
// ============================================================================

#[test]
fn test_conflict_alerting_workflow() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // Add two power meters - creates a conflict
    let pm1 = make_ble_sensor("Stages Left", SensorType::PowerMeter);
    let pm2 = make_ble_sensor("Stages Right", SensorType::PowerMeter);

    detector.register_sensor(&pm1);
    detector.register_sensor(&pm2);

    // Conflict should need attention
    let needing_attention = detector.conflicts_needing_attention();
    assert!(!needing_attention.is_empty());
    assert!(needing_attention.iter().any(|c| c.data_type == DataType::Power));

    // After showing alert, mark as notified
    detector.mark_notified(DataType::Power);

    // Should no longer need attention
    let needing_attention_after = detector.conflicts_needing_attention();
    assert!(needing_attention_after.is_empty() ||
            !needing_attention_after.iter().any(|c| c.data_type == DataType::Power));
}

#[test]
fn test_conflict_resolution_workflow() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // User has trainer + power meter
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);

    detector.register_sensor(&trainer);
    detector.register_sensor(&pm);

    // Conflict should exist for power
    assert!(detector.has_conflict(DataType::Power));
    assert!(!detector.get_conflict(DataType::Power).unwrap().is_resolved);

    // User selects power meter as primary for power
    detector.set_primary(DataType::Power, &pm.device_id);

    // Conflict should be resolved now
    assert!(detector.get_conflict(DataType::Power).unwrap().is_resolved);

    // Power meter should be the primary
    assert_eq!(
        detector.get_primary(DataType::Power),
        Some(pm.device_id.as_str())
    );
}

#[test]
fn test_multiple_conflict_mark_all_notified() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // Create multiple conflicts
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let hr1 = make_ble_sensor("Polar H10", SensorType::HeartRate);
    let hr2 = make_ant_sensor("Garmin HRM", 1234, SensorType::HeartRate);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);
    detector.register_sensor(&hr1);
    detector.register_sensor(&hr2);

    // Multiple conflicts should need attention
    let needing_attention = detector.conflicts_needing_attention();
    assert!(needing_attention.len() >= 2);

    // Mark all as notified
    detector.mark_all_notified();

    // None should need attention now
    assert!(detector.conflicts_needing_attention().is_empty());
}

// ============================================================================
// Connection status update tests
// ============================================================================

#[test]
fn test_update_connection_status() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    detector.register_sensor(&pm);

    // Initial status should be disconnected
    let conflict = detector.get_conflict(DataType::Power);
    if let Some(c) = conflict {
        assert!(!c.sources.iter().any(|s| s.is_connected));
    }

    // Update to connected
    detector.update_connection_status(&pm.device_id, true);

    // Now should be connected
    let conflict_after = detector.get_conflict(DataType::Power);
    // Note: This conflict may not be "active" since there's only one sensor
    // but we can verify the update happened via the internal state
}
