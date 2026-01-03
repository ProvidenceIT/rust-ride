//! Unit tests for connection session persistence.
//!
//! Tests verify that:
//! - Session sensors are correctly created and updated
//! - Connection sessions track sensors properly
//! - Sessions are saved and loaded correctly
//! - Reconnection priority ordering works as expected
//! - Session staleness is detected correctly

use rustride::sensors::persistence::{
    ConnectionSession, ConnectionSessionManager, SessionSensor,
};
use rustride::sensors::types::{Protocol, SensorType};
use std::path::PathBuf;
use tempfile::tempdir;

// ============================================================================
// SessionSensor Tests
// ============================================================================

#[test]
fn test_session_sensor_new() {
    let sensor = SessionSensor::new(
        "00:11:22:33:44:55".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert_eq!(sensor.device_id, "00:11:22:33:44:55");
    assert_eq!(sensor.name, "KICKR CORE 1234");
    assert_eq!(sensor.sensor_type, SensorType::Trainer);
    assert_eq!(sensor.protocol, Protocol::BleFtms);
    assert!(!sensor.is_primary);
    assert!(sensor.was_healthy);
    assert!(sensor.last_data_at.is_none());
}

#[test]
fn test_session_sensor_record_data() {
    let mut sensor = SessionSensor::new(
        "device1".to_string(),
        "Sensor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    assert!(sensor.last_data_at.is_none());

    sensor.record_data();

    assert!(sensor.last_data_at.is_some());
    assert!(sensor.was_healthy);
}

#[test]
fn test_session_sensor_mark_unhealthy() {
    let mut sensor = SessionSensor::new(
        "device1".to_string(),
        "Sensor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    assert!(sensor.was_healthy);

    sensor.mark_unhealthy();

    assert!(!sensor.was_healthy);
}

#[test]
fn test_session_sensor_display_name() {
    let sensor = SessionSensor::new(
        "device1".to_string(),
        "Wahoo KICKR".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert_eq!(sensor.display_name(), "Wahoo KICKR");
}

// ============================================================================
// ConnectionSession Tests
// ============================================================================

#[test]
fn test_connection_session_new() {
    let session = ConnectionSession::new();

    assert!(session.is_empty());
    assert_eq!(session.len(), 0);
    assert!(!session.clean_shutdown);
    assert!(session.app_version.is_none());
}

#[test]
fn test_connection_session_with_version() {
    let session = ConnectionSession::with_version("1.0.0".to_string());

    assert_eq!(session.app_version, Some("1.0.0".to_string()));
}

#[test]
fn test_connection_session_add_sensor() {
    let mut session = ConnectionSession::new();

    let sensor = SessionSensor::new(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    session.add_sensor(sensor);

    assert_eq!(session.len(), 1);
    assert!(session.contains("device1"));
    assert!(session.get_sensor("device1").is_some());
}

#[test]
fn test_connection_session_add_multiple_sensors() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "trainer1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));
    session.add_sensor(SessionSensor::new(
        "hr1".to_string(),
        "Heart Rate".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));
    session.add_sensor(SessionSensor::new(
        "power1".to_string(),
        "Power Meter".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    ));

    assert_eq!(session.len(), 3);
    assert!(session.contains("trainer1"));
    assert!(session.contains("hr1"));
    assert!(session.contains("power1"));
}

#[test]
fn test_connection_session_remove_sensor() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));

    assert!(session.contains("device1"));

    let removed = session.remove_sensor("device1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().device_id, "device1");
    assert!(!session.contains("device1"));
    assert!(session.is_empty());
}

#[test]
fn test_connection_session_remove_nonexistent() {
    let mut session = ConnectionSession::new();

    let removed = session.remove_sensor("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_connection_session_get_sensor_mut() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));

    if let Some(sensor) = session.get_sensor_mut("device1") {
        sensor.is_primary = true;
    }

    assert!(session.get_sensor("device1").unwrap().is_primary);
}

#[test]
fn test_connection_session_sensors_of_type() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "trainer1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));
    session.add_sensor(SessionSensor::new(
        "hr1".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));
    session.add_sensor(SessionSensor::new(
        "power1".to_string(),
        "Power".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    ));

    let trainers = session.sensors_of_type(SensorType::Trainer);
    assert_eq!(trainers.len(), 1);
    assert_eq!(trainers[0].device_id, "trainer1");

    let hr_sensors = session.sensors_of_type(SensorType::HeartRate);
    assert_eq!(hr_sensors.len(), 1);
    assert_eq!(hr_sensors[0].device_id, "hr1");
}

#[test]
fn test_connection_session_healthy_sensors() {
    let mut session = ConnectionSession::new();

    let mut healthy = SessionSensor::new(
        "healthy1".to_string(),
        "Healthy".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );
    healthy.was_healthy = true;

    let mut unhealthy = SessionSensor::new(
        "unhealthy1".to_string(),
        "Unhealthy".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    );
    unhealthy.was_healthy = false;

    session.add_sensor(healthy);
    session.add_sensor(unhealthy);

    let healthy_sensors = session.healthy_sensors();
    assert_eq!(healthy_sensors.len(), 1);
    assert_eq!(healthy_sensors[0].device_id, "healthy1");
}

#[test]
fn test_connection_session_primary_sensors() {
    let mut session = ConnectionSession::new();

    let mut primary = SessionSensor::new(
        "primary1".to_string(),
        "Primary".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    primary.is_primary = true;

    let secondary = SessionSensor::new(
        "secondary1".to_string(),
        "Secondary".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    session.add_sensor(primary);
    session.add_sensor(secondary);

    let primaries = session.primary_sensors();
    assert_eq!(primaries.len(), 1);
    assert_eq!(primaries[0].device_id, "primary1");
}

#[test]
fn test_connection_session_reconnection_priority_primary_first() {
    let mut session = ConnectionSession::new();

    // Add sensors in random order
    session.add_sensor(SessionSensor::new(
        "hr1".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));

    let mut primary_trainer = SessionSensor::new(
        "trainer1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    primary_trainer.is_primary = true;
    session.add_sensor(primary_trainer);

    session.add_sensor(SessionSensor::new(
        "power1".to_string(),
        "Power".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    ));

    let priority = session.reconnection_priority();

    // Primary trainer should be first
    assert_eq!(priority[0].device_id, "trainer1");
    assert!(priority[0].is_primary);
}

#[test]
fn test_connection_session_reconnection_priority_healthy_before_unhealthy() {
    let mut session = ConnectionSession::new();

    let mut unhealthy = SessionSensor::new(
        "unhealthy".to_string(),
        "Unhealthy".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    );
    unhealthy.was_healthy = false;
    session.add_sensor(unhealthy);

    let healthy = SessionSensor::new(
        "healthy".to_string(),
        "Healthy".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    );
    session.add_sensor(healthy);

    let priority = session.reconnection_priority();

    // Healthy sensor should be first
    assert_eq!(priority[0].device_id, "healthy");
    assert!(priority[0].was_healthy);
    assert_eq!(priority[1].device_id, "unhealthy");
    assert!(!priority[1].was_healthy);
}

#[test]
fn test_connection_session_reconnection_priority_sensor_type() {
    let mut session = ConnectionSession::new();

    // Add sensors with different types (not primary, all healthy)
    session.add_sensor(SessionSensor::new(
        "hr1".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));
    session.add_sensor(SessionSensor::new(
        "trainer1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));
    session.add_sensor(SessionSensor::new(
        "power1".to_string(),
        "Power".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    ));
    session.add_sensor(SessionSensor::new(
        "cadence1".to_string(),
        "Cadence".to_string(),
        SensorType::Cadence,
        Protocol::BleCsc,
    ));

    let priority = session.reconnection_priority();

    // Order should be: trainer, power, hr, cadence
    assert_eq!(priority[0].sensor_type, SensorType::Trainer);
    assert_eq!(priority[1].sensor_type, SensorType::PowerMeter);
    assert_eq!(priority[2].sensor_type, SensorType::HeartRate);
    assert_eq!(priority[3].sensor_type, SensorType::Cadence);
}

#[test]
fn test_connection_session_clean_shutdown() {
    let mut session = ConnectionSession::new();

    assert!(!session.clean_shutdown);

    session.mark_clean_shutdown();

    assert!(session.clean_shutdown);
}

#[test]
fn test_connection_session_clear() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "device1".to_string(),
        "Sensor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));
    session.add_sensor(SessionSensor::new(
        "device2".to_string(),
        "Sensor2".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));

    assert_eq!(session.len(), 2);

    session.clear();

    assert!(session.is_empty());
    assert_eq!(session.len(), 0);
}

#[test]
fn test_connection_session_all_sensors() {
    let mut session = ConnectionSession::new();

    session.add_sensor(SessionSensor::new(
        "device1".to_string(),
        "Sensor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    ));
    session.add_sensor(SessionSensor::new(
        "device2".to_string(),
        "Sensor2".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    ));

    let all: Vec<_> = session.all_sensors().collect();
    assert_eq!(all.len(), 2);
}

// ============================================================================
// ConnectionSessionManager Tests
// ============================================================================

#[test]
fn test_session_manager_new() {
    let manager = ConnectionSessionManager::new();

    assert!(manager.session().is_empty());
    assert_eq!(manager.sensor_count(), 0);
}

#[test]
fn test_session_manager_with_path() {
    let path = PathBuf::from("/tmp/test_session_custom.json");
    let manager = ConnectionSessionManager::with_path(path);

    assert!(manager.session().is_empty());
}

#[test]
fn test_session_manager_sensor_connected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );

    assert_eq!(manager.sensor_count(), 1);
    assert!(manager.session().contains("device1"));

    let sensor = manager.session().get_sensor("device1").unwrap();
    assert!(sensor.is_primary);
}

#[test]
fn test_session_manager_sensor_disconnected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        false,
    );

    assert_eq!(manager.sensor_count(), 1);

    manager.sensor_disconnected("device1");

    assert_eq!(manager.sensor_count(), 0);
}

#[test]
fn test_session_manager_sensor_data_received() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        false,
    );

    let before = manager.session().get_sensor("device1").unwrap().last_data_at;
    assert!(before.is_none());

    manager.sensor_data_received("device1");

    let after = manager.session().get_sensor("device1").unwrap().last_data_at;
    assert!(after.is_some());
}

#[test]
fn test_session_manager_sensor_unhealthy() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        false,
    );

    assert!(manager.session().get_sensor("device1").unwrap().was_healthy);

    manager.sensor_unhealthy("device1");

    assert!(!manager.session().get_sensor("device1").unwrap().was_healthy);
}

#[test]
fn test_session_manager_get_reconnection_device_ids() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );
    manager.sensor_connected(
        "device2".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
        false,
    );

    let ids = manager.get_reconnection_device_ids();

    assert_eq!(ids.len(), 2);
    // Primary trainer should be first due to priority
    assert_eq!(ids[0], "device1");
}

#[test]
fn test_session_manager_has_reconnectable_session() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    // Empty session is not reconnectable
    assert!(!manager.has_reconnectable_session());

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );

    // Session with sensors is reconnectable
    assert!(manager.has_reconnectable_session());
}

#[test]
fn test_session_manager_prepare_shutdown() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    assert!(!manager.was_clean_shutdown());

    manager.prepare_shutdown();

    assert!(manager.was_clean_shutdown());
}

#[test]
fn test_session_manager_start_new_session() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );

    assert_eq!(manager.sensor_count(), 1);

    manager.start_new_session(false);

    assert_eq!(manager.sensor_count(), 0);
}

#[test]
fn test_session_manager_start_new_session_preserve_sensors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );

    assert_eq!(manager.sensor_count(), 1);

    manager.start_new_session(true);

    // Sensors preserved
    assert_eq!(manager.sensor_count(), 1);
}

#[test]
fn test_session_manager_clear() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path);
    manager.set_auto_save(false);

    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );
    manager.sensor_connected(
        "device2".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
        false,
    );

    assert_eq!(manager.sensor_count(), 2);

    manager.clear();

    assert_eq!(manager.sensor_count(), 0);
}

// ============================================================================
// Persistence Save/Load Tests
// ============================================================================

#[test]
fn test_session_manager_save_and_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");

    // Create and save a session
    {
        let mut manager = ConnectionSessionManager::with_path(path.clone());
        manager.set_auto_save(false);

        manager.sensor_connected(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );
        manager.sensor_connected(
            "device2".to_string(),
            "HR".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
            false,
        );

        manager.save().unwrap();
    }

    // Load the session
    {
        let manager = ConnectionSessionManager::load_from_path(path);

        assert_eq!(manager.sensor_count(), 2);
        assert!(manager.session().contains("device1"));
        assert!(manager.session().contains("device2"));

        let sensor = manager.session().get_sensor("device1").unwrap();
        assert!(sensor.is_primary);
        assert_eq!(sensor.sensor_type, SensorType::Trainer);
    }
}

#[test]
fn test_session_manager_load_nonexistent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");

    let manager = ConnectionSessionManager::load_from_path(path);

    // Should return empty session
    assert!(manager.session().is_empty());
}

#[test]
fn test_session_manager_delete_session_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");

    // Create and save a session
    let mut manager = ConnectionSessionManager::with_path(path.clone());
    manager.sensor_connected(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );
    manager.save().unwrap();

    assert!(path.exists());

    manager.delete_session_file().unwrap();

    assert!(!path.exists());
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_workout_session_scenario() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");
    let mut manager = ConnectionSessionManager::with_path(path.clone());
    manager.set_auto_save(false);

    // User starts workout, connects sensors
    manager.sensor_connected(
        "trainer1".to_string(),
        "KICKR CORE".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
        true,
    );
    manager.sensor_connected(
        "hr1".to_string(),
        "Polar H10".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
        false,
    );
    manager.sensor_connected(
        "power1".to_string(),
        "Assioma Duo".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
        false,
    );

    // Sensors receive data during workout
    manager.sensor_data_received("trainer1");
    manager.sensor_data_received("hr1");
    manager.sensor_data_received("power1");

    // HR drops connection and goes stale
    manager.sensor_unhealthy("hr1");

    // User ends workout
    manager.prepare_shutdown();
    manager.save().unwrap();

    // Simulate app restart
    let loaded = ConnectionSessionManager::load_from_path(path);

    assert!(loaded.was_clean_shutdown());
    assert_eq!(loaded.sensor_count(), 3);

    // Get reconnection targets
    let targets = loaded.get_reconnection_device_ids();

    // Trainer should be first (primary)
    assert_eq!(targets[0], "trainer1");

    // Power meter should come before HR (HR is unhealthy)
    assert_eq!(targets[1], "power1");
    assert_eq!(targets[2], "hr1");
}

#[test]
fn test_crash_recovery_scenario() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");

    // Simulate mid-workout (no clean shutdown)
    {
        let mut manager = ConnectionSessionManager::with_path(path.clone());
        manager.set_auto_save(false);

        manager.sensor_connected(
            "trainer1".to_string(),
            "KICKR".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );

        // Crash happens - no prepare_shutdown called
        manager.save().unwrap();
    }

    // App restarts
    let loaded = ConnectionSessionManager::load_from_path(path);

    // Session should indicate unclean shutdown
    assert!(!loaded.was_clean_shutdown());

    // But sensors should still be available for reconnection
    assert!(loaded.has_reconnectable_session());
    assert_eq!(loaded.sensor_count(), 1);
}

#[test]
fn test_multiple_workout_sessions() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("session.json");

    // First workout
    {
        let mut manager = ConnectionSessionManager::with_path(path.clone());
        manager.set_auto_save(false);

        manager.sensor_connected(
            "trainer1".to_string(),
            "KICKR".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );
        manager.prepare_shutdown();
        manager.save().unwrap();
    }

    // Second workout - start new session
    {
        let mut manager = ConnectionSessionManager::load_from_path(path.clone());

        // User explicitly starts new session
        manager.start_new_session(false);

        // Connect different sensors
        manager.sensor_connected(
            "trainer2".to_string(),
            "Neo 2T".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );
        manager.save().unwrap();
    }

    // Verify new session only has new sensors
    let loaded = ConnectionSessionManager::load_from_path(path);
    assert_eq!(loaded.sensor_count(), 1);
    assert!(loaded.session().contains("trainer2"));
    assert!(!loaded.session().contains("trainer1"));
}
