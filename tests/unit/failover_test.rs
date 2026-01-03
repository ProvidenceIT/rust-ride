//! Unit tests for automatic primary sensor failover.
//!
//! Tests the automatic promotion of secondary sensors when the primary
//! sensor disconnects, including user notification via FailoverActivated events.

use rust_ride::sensors::conflict::{
    ConflictDetector, ConflictDetectorConfig, DataType, FailoverResult, ResolutionStrategy,
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
// FailoverResult tests
// ============================================================================

#[test]
fn test_failover_result_message() {
    let result = FailoverResult {
        data_type: DataType::Power,
        from_device_id: "ble:stages_power".to_string(),
        from_sensor_name: "Stages Power".to_string(),
        to_device_id: "ble:kickr_core".to_string(),
        to_sensor_name: "KICKR Core".to_string(),
    };

    let message = result.message();
    assert!(message.contains("Power"));
    assert!(message.contains("Stages Power"));
    assert!(message.contains("KICKR Core"));
}

#[test]
fn test_failover_result_message_heart_rate() {
    let result = FailoverResult {
        data_type: DataType::HeartRate,
        from_device_id: "ble:polar_h10".to_string(),
        from_sensor_name: "Polar H10".to_string(),
        to_device_id: "ant+:garmin_hrm".to_string(),
        to_sensor_name: "Garmin HRM".to_string(),
    };

    let message = result.message();
    assert!(message.contains("Heart Rate"));
    assert!(message.contains("Polar H10"));
    assert!(message.contains("Garmin HRM"));
}

// ============================================================================
// Connection status update tests
// ============================================================================

#[test]
fn test_update_connection_status() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Mark sensors as connected
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);

    // Verify conflict sources have correct connection status
    let conflict = detector.get_conflict(DataType::Power).unwrap();
    let pm_source = conflict.sources.iter().find(|s| s.device_id == pm.device_id).unwrap();
    assert!(pm_source.is_connected);
}

#[test]
fn test_update_connection_status_disconnect() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Both connected initially
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);

    // Disconnect power meter
    detector.update_connection_status(&pm.device_id, false);

    let conflict = detector.get_conflict(DataType::Power).unwrap();
    let pm_source = conflict.sources.iter().find(|s| s.device_id == pm.device_id).unwrap();
    assert!(!pm_source.is_connected);

    // Trainer still connected
    let trainer_source = conflict.sources.iter().find(|s| s.device_id == trainer.device_id).unwrap();
    assert!(trainer_source.is_connected);
}

// ============================================================================
// Primary disconnect failover tests
// ============================================================================

#[test]
fn test_handle_primary_disconnect_no_secondary() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);

    detector.register_sensor(&pm);
    detector.update_connection_status(&pm.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Disconnect primary - no secondary available
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should have no failover (no connected secondary)
    assert!(failovers.is_empty());

    // Primary should be cleared
    assert!(detector.get_primary(DataType::Power).is_none());
}

#[test]
fn test_handle_primary_disconnect_with_secondary() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Both connected, power meter is primary
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Disconnect primary
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should have a failover for Power
    assert!(!failovers.is_empty());
    let power_failover = failovers.iter().find(|f| f.data_type == DataType::Power);
    assert!(power_failover.is_some());

    let failover = power_failover.unwrap();
    assert_eq!(failover.from_device_id, pm.device_id);
    assert_eq!(failover.from_sensor_name, "Stages Power");
    assert_eq!(failover.to_device_id, trainer.device_id);
    assert_eq!(failover.to_sensor_name, "KICKR Core");

    // New primary should be the trainer
    assert_eq!(detector.get_primary(DataType::Power), Some(trainer.device_id.as_str()));
}

#[test]
fn test_handle_primary_disconnect_multiple_data_types() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // Trainer provides Power, Cadence, Speed
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let cadence = make_ble_sensor("Wahoo Cadence", SensorType::Cadence);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);
    detector.register_sensor(&cadence);

    // All connected
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.update_connection_status(&cadence.device_id, true);

    // PM is primary for power and cadence
    detector.set_primary(DataType::Power, &pm.device_id);
    detector.set_primary(DataType::Cadence, &pm.device_id);

    // Disconnect power meter
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should have failovers for both Power and Cadence
    assert!(failovers.len() >= 2);

    // Power should failover to trainer
    let power_failover = failovers.iter().find(|f| f.data_type == DataType::Power);
    assert!(power_failover.is_some());
    assert_eq!(power_failover.unwrap().to_device_id, trainer.device_id);

    // Cadence could failover to either trainer or cadence sensor
    let cadence_failover = failovers.iter().find(|f| f.data_type == DataType::Cadence);
    assert!(cadence_failover.is_some());
}

#[test]
fn test_handle_primary_disconnect_secondary_not_connected() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Only power meter is connected
    detector.update_connection_status(&pm.device_id, true);
    // Trainer is NOT connected
    detector.update_connection_status(&trainer.device_id, false);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Disconnect primary
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should have no failover (secondary not connected)
    assert!(failovers.is_empty());

    // Primary should be cleared
    assert!(detector.get_primary(DataType::Power).is_none());
}

#[test]
fn test_handle_primary_disconnect_non_primary_sensor() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);

    // Trainer is primary
    detector.set_primary(DataType::Power, &trainer.device_id);

    // Disconnect non-primary (power meter)
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should have no failover (wasn't primary)
    assert!(failovers.is_empty());

    // Trainer still primary
    assert_eq!(detector.get_primary(DataType::Power), Some(trainer.device_id.as_str()));
}

// ============================================================================
// Failover target tests
// ============================================================================

#[test]
fn test_get_failover_targets_empty() {
    let detector = ConflictDetector::new();
    let targets = detector.get_failover_targets(DataType::Power);
    assert!(targets.is_empty());
}

#[test]
fn test_get_failover_targets_with_connected_secondary() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    let targets = detector.get_failover_targets(DataType::Power);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].device_id, trainer.device_id);
}

#[test]
fn test_get_failover_targets_excludes_disconnected() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    // Trainer not connected
    detector.update_connection_status(&trainer.device_id, false);
    detector.set_primary(DataType::Power, &pm.device_id);

    let targets = detector.get_failover_targets(DataType::Power);
    assert!(targets.is_empty());
}

#[test]
fn test_get_failover_targets_sorted_by_priority() {
    let mut detector = ConflictDetector::new();

    // Trainer has lower priority (2) than power meter (1) for power
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let pm2 = make_ble_sensor("Quarq DZero", SensorType::PowerMeter);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);
    detector.register_sensor(&pm2);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.update_connection_status(&pm2.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    let targets = detector.get_failover_targets(DataType::Power);
    assert_eq!(targets.len(), 2);

    // Power meter should be first (higher priority)
    assert_eq!(targets[0].sensor_type, SensorType::PowerMeter);
}

// ============================================================================
// Failover availability tests
// ============================================================================

#[test]
fn test_has_failover_available() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    assert!(detector.has_failover_available(DataType::Power));
}

#[test]
fn test_has_failover_not_available() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);

    detector.register_sensor(&pm);
    detector.update_connection_status(&pm.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    assert!(!detector.has_failover_available(DataType::Power));
}

// ============================================================================
// Protected data types tests
// ============================================================================

#[test]
fn test_get_protected_data_types() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);
    detector.set_primary(DataType::Cadence, &pm.device_id);

    let protected = detector.get_protected_data_types();

    // Both Power and Cadence should have failover protection
    assert!(protected.contains(&DataType::Power));
    assert!(protected.contains(&DataType::Cadence));
}

#[test]
fn test_get_at_risk_data_types() {
    let mut detector = ConflictDetector::new();

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let hr = make_ble_sensor("Polar H10", SensorType::HeartRate);

    detector.register_sensor(&pm);
    detector.register_sensor(&hr);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&hr.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);
    detector.set_primary(DataType::HeartRate, &hr.device_id);

    let at_risk = detector.get_at_risk_data_types();

    // Both are at risk (only one sensor for each type)
    assert!(at_risk.contains(&DataType::Power) || at_risk.contains(&DataType::Cadence));
    assert!(at_risk.contains(&DataType::HeartRate));
}

// ============================================================================
// Real-world failover scenarios
// ============================================================================

#[test]
fn test_power_meter_disconnect_failover_to_trainer() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // Typical indoor setup: power meter + trainer
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Both connected, user prefers power meter for accuracy
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Power meter battery dies mid-ride
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should automatically switch to trainer power
    assert!(!failovers.is_empty());
    let power_failover = failovers.iter().find(|f| f.data_type == DataType::Power).unwrap();
    assert_eq!(power_failover.to_sensor_name, "KICKR Core");

    // User can continue their workout with trainer power
    assert_eq!(detector.get_primary(DataType::Power), Some(trainer.device_id.as_str()));
}

#[test]
fn test_hr_monitor_disconnect_failover_to_ant() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // User has both BLE and ANT+ HR monitors
    let ble_hr = make_ble_sensor("Polar H10", SensorType::HeartRate);
    let ant_hr = make_ant_sensor("Garmin HRM", 5678, SensorType::HeartRate);

    detector.register_sensor(&ble_hr);
    detector.register_sensor(&ant_hr);

    // Both connected, BLE is primary (usually more reliable)
    detector.update_connection_status(&ble_hr.device_id, true);
    detector.update_connection_status(&ant_hr.device_id, true);
    detector.set_primary(DataType::HeartRate, &ble_hr.device_id);

    // BLE connection drops (maybe interference)
    let failovers = detector.handle_primary_disconnect(&ble_hr.device_id);

    // Should automatically switch to ANT+ HR
    assert!(!failovers.is_empty());
    let hr_failover = failovers.iter().find(|f| f.data_type == DataType::HeartRate).unwrap();
    assert_eq!(hr_failover.to_sensor_name, "Garmin HRM");
}

#[test]
fn test_trainer_disconnect_no_power_failover() {
    let mut detector = ConflictDetector::new();

    // Only have a trainer (no separate power meter)
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&trainer);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &trainer.device_id);
    detector.set_primary(DataType::TrainerControl, &trainer.device_id);

    // Trainer disconnects
    let failovers = detector.handle_primary_disconnect(&trainer.device_id);

    // No failover possible - no secondary sensors
    assert!(failovers.is_empty());

    // Primary should be cleared
    assert!(detector.get_primary(DataType::Power).is_none());
    assert!(detector.get_primary(DataType::TrainerControl).is_none());
}

#[test]
fn test_cadence_failover_prefers_dedicated_sensor() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    // User has trainer, power meter with cadence, and dedicated cadence sensor
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);
    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let cadence = make_ble_sensor("Wahoo Cadence", SensorType::Cadence);

    detector.register_sensor(&trainer);
    detector.register_sensor(&pm);
    detector.register_sensor(&cadence);

    // All connected, power meter is primary for cadence
    detector.update_connection_status(&trainer.device_id, true);
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&cadence.device_id, true);
    detector.set_primary(DataType::Cadence, &pm.device_id);

    // Power meter disconnects
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Should failover cadence, preferring dedicated sensor over trainer
    let cadence_failover = failovers.iter().find(|f| f.data_type == DataType::Cadence);
    assert!(cadence_failover.is_some());

    // Dedicated cadence sensor should be preferred (priority 1) over trainer (priority 2)
    assert_eq!(cadence_failover.unwrap().to_device_id, cadence.device_id);
}

// ============================================================================
// Automatic failover integration tests
// These test the behavior used by SensorManager's handle_notifications when
// a sensor unexpectedly disconnects and reconnection attempts are exhausted.
// ============================================================================

#[test]
fn test_auto_failover_on_unexpected_disconnect() {
    // Simulates what happens when a sensor's notification stream ends unexpectedly
    // and the SensorManager triggers failover via handle_primary_disconnect
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    // Both connected, power meter is primary for power
    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Verify failover is available before disconnect
    assert!(detector.has_failover_available(DataType::Power));

    // Simulate unexpected disconnect (like BLE connection dropping)
    // This is what handle_notifications calls when reconnection fails
    let failovers = detector.handle_primary_disconnect(&pm.device_id);

    // Failover should automatically promote trainer
    assert!(!failovers.is_empty());
    let power_failover = failovers.iter().find(|f| f.data_type == DataType::Power).unwrap();

    // Verify failover message is user-friendly for notification
    let message = power_failover.message();
    assert!(message.contains("Power"));
    assert!(message.contains("Stages Power"));
    assert!(message.contains("KICKR Core"));

    // New primary should be the trainer
    assert_eq!(detector.get_primary(DataType::Power), Some(trainer.device_id.as_str()));
}

#[test]
fn test_auto_failover_preserves_user_experience() {
    // Verifies that automatic failover provides a seamless experience:
    // - Data continues flowing from the new primary
    // - User is notified via FailoverActivated event
    // - Original primary can be restored if it reconnects
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm = make_ble_sensor("Stages Power", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm);
    detector.register_sensor(&trainer);

    detector.update_connection_status(&pm.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm.device_id);

    // Power meter disconnects unexpectedly
    let failovers = detector.handle_primary_disconnect(&pm.device_id);
    assert!(!failovers.is_empty());

    // Trainer is now primary - workout can continue
    assert_eq!(detector.get_primary(DataType::Power), Some(trainer.device_id.as_str()));

    // Later, power meter reconnects
    detector.update_connection_status(&pm.device_id, true);

    // User can manually switch back if they prefer
    let result = detector.set_primary(DataType::Power, &pm.device_id);
    assert!(result);
    assert_eq!(detector.get_primary(DataType::Power), Some(pm.device_id.as_str()));
}

#[test]
fn test_auto_failover_with_multiple_secondaries() {
    // When multiple secondary sensors are available, failover should
    // prefer sensors by priority (dedicated sensors over multi-function ones)
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::UserSelection,
        auto_resolve_non_critical: false,
        persist_resolutions: false,
    });

    let pm1 = make_ble_sensor("Stages Left", SensorType::PowerMeter);
    let pm2 = make_ble_sensor("Quarq DZero", SensorType::PowerMeter);
    let trainer = make_ble_sensor("KICKR Core", SensorType::Trainer);

    detector.register_sensor(&pm1);
    detector.register_sensor(&pm2);
    detector.register_sensor(&trainer);

    // All connected, Stages is primary
    detector.update_connection_status(&pm1.device_id, true);
    detector.update_connection_status(&pm2.device_id, true);
    detector.update_connection_status(&trainer.device_id, true);
    detector.set_primary(DataType::Power, &pm1.device_id);

    // Stages disconnects
    let failovers = detector.handle_primary_disconnect(&pm1.device_id);

    // Should failover to Quarq (power meter) not trainer (lower priority)
    let power_failover = failovers.iter().find(|f| f.data_type == DataType::Power).unwrap();
    assert_eq!(power_failover.to_device_id, pm2.device_id);
    assert_eq!(power_failover.to_sensor_name, "Quarq DZero");
}
