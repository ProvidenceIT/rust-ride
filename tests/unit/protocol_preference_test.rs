//! Unit tests for protocol preference storage.
//!
//! Tests the storage and retrieval of user's preferred protocol (BLE vs ANT+)
//! for dual-protocol sensors, including persistence and integration with
//! the DualProtocolDetector.

use rust_ride::sensors::types::{DiscoveredSensor, Protocol, SensorProtocol, SensorType};
use rust_ride::sensors::dual_protocol::{
    DualProtocolBinding, DualProtocolDetector, ProtocolPreference,
    ProtocolPreferenceManager,
};
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

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

/// Create a preference manager with a temp file path.
fn make_temp_manager() -> (ProtocolPreferenceManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("protocol_preferences.json");
    let mut manager = ProtocolPreferenceManager::with_path(path);
    manager.set_auto_save(false);
    (manager, temp_dir)
}

// ============================================================================
// ProtocolPreference tests
// ============================================================================

#[test]
fn test_protocol_preference_new() {
    let pref = ProtocolPreference::new(
        "binding:1234".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    assert_eq!(pref.binding_id, "binding:1234");
    assert_eq!(pref.sensor_name, "KICKR CORE 1234");
    assert_eq!(pref.sensor_type, SensorType::Trainer);
    assert_eq!(pref.preferred_protocol, SensorProtocol::Ble);
    assert!(pref.user_set);
    assert_eq!(pref.usage_count, 0);
}

#[test]
fn test_protocol_preference_record_usage() {
    let mut pref = ProtocolPreference::new(
        "binding:1234".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    assert_eq!(pref.usage_count, 0);

    pref.record_usage();
    assert_eq!(pref.usage_count, 1);

    pref.record_usage();
    assert_eq!(pref.usage_count, 2);
}

#[test]
fn test_protocol_preference_update_protocol() {
    let mut pref = ProtocolPreference::new(
        "binding:1234".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        SensorProtocol::Ble,
        false,
    );

    assert_eq!(pref.preferred_protocol, SensorProtocol::Ble);
    assert!(!pref.user_set);

    pref.update_protocol(SensorProtocol::AntPlus, true);

    assert_eq!(pref.preferred_protocol, SensorProtocol::AntPlus);
    assert!(pref.user_set);
}

// ============================================================================
// DualProtocolBinding preference tests
// ============================================================================

#[test]
fn test_binding_preferred_protocol_default() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let binding = DualProtocolBinding::new(&sensor);

    assert!(binding.preferred_protocol.is_none());
    assert_eq!(binding.get_effective_preferred_protocol(), SensorProtocol::Ble);
}

#[test]
fn test_binding_set_preferred_protocol() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let mut binding = DualProtocolBinding::new(&sensor);

    binding.set_preferred_protocol(SensorProtocol::AntPlus);

    assert_eq!(binding.preferred_protocol, Some(SensorProtocol::AntPlus));
}

#[test]
fn test_binding_clear_preferred_protocol() {
    let sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let mut binding = DualProtocolBinding::new(&sensor);

    binding.set_preferred_protocol(SensorProtocol::AntPlus);
    assert!(binding.preferred_protocol.is_some());

    binding.clear_preferred_protocol();
    assert!(binding.preferred_protocol.is_none());
}

#[test]
fn test_binding_get_preferred_device_id_with_preference() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    binding.add_protocol_instance(&ant_sensor);

    // Without preference, should default to BLE
    let default_id = binding.get_preferred_device_id();
    assert_eq!(default_id, Some(ble_sensor.device_id.as_str()));

    // With ANT+ preference
    binding.set_preferred_protocol(SensorProtocol::AntPlus);
    let preferred_id = binding.get_preferred_device_id();
    assert_eq!(preferred_id, Some(ant_sensor.device_id.as_str()));

    // With BLE preference
    binding.set_preferred_protocol(SensorProtocol::Ble);
    let preferred_id = binding.get_preferred_device_id();
    assert_eq!(preferred_id, Some(ble_sensor.device_id.as_str()));
}

#[test]
fn test_binding_preferred_protocol_fallback() {
    // If preferred protocol is not available, fall back to what's available
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let mut binding = DualProtocolBinding::new(&ble_sensor);

    // Set preference to ANT+ but only BLE is available
    binding.set_preferred_protocol(SensorProtocol::AntPlus);

    // Should still return BLE as it's the only available option
    let device_id = binding.get_preferred_device_id();
    assert_eq!(device_id, Some(ble_sensor.device_id.as_str()));
}

#[test]
fn test_binding_is_preferred_protocol_available() {
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let mut binding = DualProtocolBinding::new(&ble_sensor);

    // No preference - always available
    assert!(binding.is_preferred_protocol_available());

    // BLE preference - available
    binding.set_preferred_protocol(SensorProtocol::Ble);
    assert!(binding.is_preferred_protocol_available());

    // ANT+ preference - not available (only BLE exists)
    binding.set_preferred_protocol(SensorProtocol::AntPlus);
    assert!(!binding.is_preferred_protocol_available());
}

// ============================================================================
// DualProtocolDetector preference tests
// ============================================================================

#[test]
fn test_detector_set_preferred_protocol() {
    let mut detector = DualProtocolDetector::new();

    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    detector.process_sensor(&ble_sensor);
    detector.process_sensor(&ant_sensor);

    // Set preference via device ID
    let result = detector.set_preferred_protocol(&ble_sensor.device_id, SensorProtocol::AntPlus);
    assert!(result);

    // Verify preference was set
    let pref = detector.get_preferred_protocol(&ble_sensor.device_id);
    assert_eq!(pref, Some(SensorProtocol::AntPlus));

    // Also accessible via ANT+ device ID
    let pref2 = detector.get_preferred_protocol(&ant_sensor.device_id);
    assert_eq!(pref2, Some(SensorProtocol::AntPlus));
}

#[test]
fn test_detector_set_preferred_protocol_nonexistent() {
    let mut detector = DualProtocolDetector::new();

    let result = detector.set_preferred_protocol("nonexistent", SensorProtocol::Ble);
    assert!(!result);
}

#[test]
fn test_detector_get_preferred_device_id() {
    let mut detector = DualProtocolDetector::new();

    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let binding_id = detector.process_sensor(&ble_sensor).unwrap();
    detector.process_sensor(&ant_sensor);

    // Default (no preference) should return BLE
    let device_id = detector.get_preferred_device_id(&binding_id);
    assert_eq!(device_id, Some(ble_sensor.device_id.as_str()));

    // Set ANT+ preference
    detector.set_preferred_protocol(&ble_sensor.device_id, SensorProtocol::AntPlus);

    let device_id = detector.get_preferred_device_id(&binding_id);
    assert_eq!(device_id, Some(ant_sensor.device_id.as_str()));
}

#[test]
fn test_detector_get_preferred_device_id_for_device() {
    let mut detector = DualProtocolDetector::new();

    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    detector.process_sensor(&ble_sensor);
    detector.process_sensor(&ant_sensor);

    // Set BLE preference
    detector.set_preferred_protocol(&ble_sensor.device_id, SensorProtocol::Ble);

    // Query via either device ID should return BLE device ID
    let preferred = detector.get_preferred_device_id_for_device(&ant_sensor.device_id);
    assert_eq!(preferred, Some(ble_sensor.device_id.as_str()));
}

#[test]
fn test_detector_bindings_with_preference() {
    let mut detector = DualProtocolDetector::new();

    let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let ble_hr = make_ble_sensor("TICKR X", SensorType::HeartRate);

    detector.process_sensor(&ble_trainer);
    detector.process_sensor(&ant_trainer);
    detector.process_sensor(&ble_hr);

    // Initially no preferences
    assert_eq!(detector.bindings_with_preference().len(), 0);

    // Set preference for trainer
    detector.set_preferred_protocol(&ble_trainer.device_id, SensorProtocol::AntPlus);

    let with_pref = detector.bindings_with_preference();
    assert_eq!(with_pref.len(), 1);
    assert_eq!(with_pref[0].sensor_type, SensorType::Trainer);
}

#[test]
fn test_detector_get_reconnection_targets() {
    let mut detector = DualProtocolDetector::new();

    let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let ble_hr = make_ble_sensor("TICKR X 5678", SensorType::HeartRate);
    let ant_hr = make_ant_sensor("TICKR X 5678", 5678, SensorType::HeartRate);

    detector.process_sensor(&ble_trainer);
    detector.process_sensor(&ant_trainer);
    detector.process_sensor(&ble_hr);
    detector.process_sensor(&ant_hr);

    // Set different preferences for each
    detector.set_preferred_protocol(&ble_trainer.device_id, SensorProtocol::AntPlus);
    detector.set_preferred_protocol(&ble_hr.device_id, SensorProtocol::Ble);

    let targets = detector.get_reconnection_targets();
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&ant_trainer.device_id));
    assert!(targets.contains(&ble_hr.device_id));
}

// ============================================================================
// ProtocolPreferenceManager tests
// ============================================================================

#[test]
fn test_preference_manager_new() {
    let (manager, _temp) = make_temp_manager();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_preference_manager_set_preference() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::AntPlus,
        true,
    );

    assert_eq!(manager.len(), 1);

    let pref = manager.get_preference("binding:1234");
    assert!(pref.is_some());
    assert_eq!(pref.unwrap().preferred_protocol, SensorProtocol::AntPlus);
    assert!(pref.unwrap().user_set);
}

#[test]
fn test_preference_manager_get_preferred_protocol() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    let protocol = manager.get_preferred_protocol("binding:1234");
    assert_eq!(protocol, Some(SensorProtocol::Ble));

    let none = manager.get_preferred_protocol("nonexistent");
    assert!(none.is_none());
}

#[test]
fn test_preference_manager_update_existing() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::Ble,
        false,
    );

    // Update to different protocol
    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::AntPlus,
        true,
    );

    assert_eq!(manager.len(), 1);

    let pref = manager.get_preference("binding:1234").unwrap();
    assert_eq!(pref.preferred_protocol, SensorProtocol::AntPlus);
    assert!(pref.user_set);
}

#[test]
fn test_preference_manager_record_usage() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    manager.record_usage("binding:1234");
    manager.record_usage("binding:1234");

    let pref = manager.get_preference("binding:1234").unwrap();
    assert_eq!(pref.usage_count, 2);
}

#[test]
fn test_preference_manager_remove() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    assert_eq!(manager.len(), 1);

    let removed = manager.remove_preference("binding:1234");
    assert!(removed.is_some());
    assert!(manager.is_empty());
}

#[test]
fn test_preference_manager_clear() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference("binding:1", "Sensor 1", SensorType::Trainer, SensorProtocol::Ble, true);
    manager.set_preference("binding:2", "Sensor 2", SensorType::HeartRate, SensorProtocol::AntPlus, true);

    assert_eq!(manager.len(), 2);

    manager.clear();
    assert!(manager.is_empty());
}

#[test]
fn test_preference_manager_preferences_for_type() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference("binding:1", "Trainer", SensorType::Trainer, SensorProtocol::Ble, true);
    manager.set_preference("binding:2", "HR", SensorType::HeartRate, SensorProtocol::AntPlus, true);
    manager.set_preference("binding:3", "Power", SensorType::PowerMeter, SensorProtocol::Ble, true);

    let trainers = manager.preferences_for_type(SensorType::Trainer);
    assert_eq!(trainers.len(), 1);
    assert_eq!(trainers[0].binding_id, "binding:1");

    let hr = manager.preferences_for_type(SensorType::HeartRate);
    assert_eq!(hr.len(), 1);
    assert_eq!(hr[0].binding_id, "binding:2");
}

#[test]
fn test_preference_manager_user_set_preferences() {
    let (mut manager, _temp) = make_temp_manager();

    manager.set_preference("binding:1", "User Set", SensorType::Trainer, SensorProtocol::Ble, true);
    manager.set_preference("binding:2", "Auto", SensorType::HeartRate, SensorProtocol::AntPlus, false);

    let user_set = manager.user_set_preferences();
    assert_eq!(user_set.len(), 1);
    assert_eq!(user_set[0].binding_id, "binding:1");
}

// ============================================================================
// Persistence tests
// ============================================================================

#[test]
fn test_preference_manager_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("protocol_preferences.json");

    // Create and save preferences
    {
        let mut manager = ProtocolPreferenceManager::with_path(path.clone());
        manager.set_auto_save(false);

        manager.set_preference(
            "binding:1234",
            "KICKR CORE 1234",
            SensorType::Trainer,
            SensorProtocol::AntPlus,
            true,
        );
        manager.set_preference(
            "binding:5678",
            "TICKR X",
            SensorType::HeartRate,
            SensorProtocol::Ble,
            false,
        );

        manager.save().expect("Save should succeed");
    }

    // Load and verify
    {
        let manager = ProtocolPreferenceManager::load_from_path(path);

        assert_eq!(manager.len(), 2);

        let trainer_pref = manager.get_preference("binding:1234").unwrap();
        assert_eq!(trainer_pref.preferred_protocol, SensorProtocol::AntPlus);
        assert!(trainer_pref.user_set);

        let hr_pref = manager.get_preference("binding:5678").unwrap();
        assert_eq!(hr_pref.preferred_protocol, SensorProtocol::Ble);
        assert!(!hr_pref.user_set);
    }
}

#[test]
fn test_preference_manager_load_missing_file() {
    let path = PathBuf::from("/tmp/nonexistent_prefs_12345.json");
    let manager = ProtocolPreferenceManager::load_from_path(path);

    assert!(manager.is_empty());
}

#[test]
fn test_preference_manager_auto_save() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("protocol_preferences.json");

    // Create with auto-save enabled
    let mut manager = ProtocolPreferenceManager::with_path(path.clone());
    assert!(manager.is_auto_save_enabled());

    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::Ble,
        true,
    );

    // File should exist due to auto-save
    assert!(path.exists());

    // Reload and verify
    let loaded = ProtocolPreferenceManager::load_from_path(path);
    assert_eq!(loaded.len(), 1);
}

// ============================================================================
// Integration tests: Detector + Manager
// ============================================================================

#[test]
fn test_apply_preferences_to_detector() {
    let (mut manager, _temp) = make_temp_manager();

    // Set up preferences
    manager.set_preference(
        "binding:1234",
        "KICKR CORE 1234",
        SensorType::Trainer,
        SensorProtocol::AntPlus,
        true,
    );

    // Create detector with matching binding
    let mut detector = DualProtocolDetector::new();
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    detector.process_sensor(&ble_sensor);
    detector.process_sensor(&ant_sensor);

    // Initially no preference on binding
    assert!(detector.get_preferred_protocol(&ble_sensor.device_id).is_none());

    // Apply saved preferences
    manager.apply_to_detector(&mut detector);

    // Now preference should be set
    let pref = detector.get_preferred_protocol(&ble_sensor.device_id);
    assert_eq!(pref, Some(SensorProtocol::AntPlus));
}

#[test]
fn test_sync_preferences_from_detector() {
    let (mut manager, _temp) = make_temp_manager();

    // Create detector with preferences set
    let mut detector = DualProtocolDetector::new();
    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    detector.process_sensor(&ble_sensor);
    detector.process_sensor(&ant_sensor);
    detector.set_preferred_protocol(&ble_sensor.device_id, SensorProtocol::AntPlus);

    // Initially manager is empty
    assert!(manager.is_empty());

    // Sync from detector
    manager.sync_from_detector(&detector);

    // Now manager should have the preference
    assert_eq!(manager.len(), 1);
    let pref = manager.get_preferred_protocol("binding:1234");
    assert_eq!(pref, Some(SensorProtocol::AntPlus));
}

#[test]
fn test_set_preference_from_binding() {
    let (mut manager, _temp) = make_temp_manager();

    let ble_sensor = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_sensor = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

    let mut binding = DualProtocolBinding::new(&ble_sensor);
    binding.add_protocol_instance(&ant_sensor);

    manager.set_preference_from_binding(&binding, SensorProtocol::AntPlus, true);

    let pref = manager.get_preference(&binding.binding_id);
    assert!(pref.is_some());
    assert_eq!(pref.unwrap().preferred_protocol, SensorProtocol::AntPlus);
    assert_eq!(pref.unwrap().sensor_name, "KICKR CORE 1234");
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_reconnection_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("protocol_preferences.json");

    // Session 1: User connects and sets preference
    {
        let mut manager = ProtocolPreferenceManager::with_path(path.clone());
        let mut detector = DualProtocolDetector::new();

        // Discover sensors
        let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

        detector.process_sensor(&ble_trainer);
        detector.process_sensor(&ant_trainer);

        // User prefers ANT+ for this trainer
        detector.set_preferred_protocol(&ble_trainer.device_id, SensorProtocol::AntPlus);

        // Save preferences
        manager.sync_from_detector(&detector);
        manager.save().unwrap();
    }

    // Session 2: App restarts, preferences restored
    {
        let manager = ProtocolPreferenceManager::load_from_path(path);
        let mut detector = DualProtocolDetector::new();

        // Rediscover sensors
        let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
        let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);

        detector.process_sensor(&ble_trainer);
        detector.process_sensor(&ant_trainer);

        // Apply saved preferences
        manager.apply_to_detector(&mut detector);

        // Get reconnection target - should use saved ANT+ preference
        let targets = detector.get_reconnection_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], ant_trainer.device_id);
    }
}

#[test]
fn test_multi_sensor_preferences() {
    let (mut manager, _temp) = make_temp_manager();
    let mut detector = DualProtocolDetector::new();

    // Set up multiple dual-protocol sensors
    let ble_trainer = make_ble_sensor("KICKR CORE 1234", SensorType::Trainer);
    let ant_trainer = make_ant_sensor("KICKR CORE 1234", 1234, SensorType::Trainer);
    let ble_hr = make_ble_sensor("TICKR X 5678", SensorType::HeartRate);
    let ant_hr = make_ant_sensor("TICKR X 5678", 5678, SensorType::HeartRate);
    let ble_power = make_ble_sensor("Stages 9999", SensorType::PowerMeter);
    let ant_power = make_ant_sensor("Stages 9999", 9999, SensorType::PowerMeter);

    detector.process_sensor(&ble_trainer);
    detector.process_sensor(&ant_trainer);
    detector.process_sensor(&ble_hr);
    detector.process_sensor(&ant_hr);
    detector.process_sensor(&ble_power);
    detector.process_sensor(&ant_power);

    // Set different preferences for each
    detector.set_preferred_protocol(&ble_trainer.device_id, SensorProtocol::AntPlus);
    detector.set_preferred_protocol(&ble_hr.device_id, SensorProtocol::Ble);
    detector.set_preferred_protocol(&ble_power.device_id, SensorProtocol::AntPlus);

    // Sync all to manager
    manager.sync_from_detector(&detector);

    assert_eq!(manager.len(), 3);

    // Verify each preference
    let trainers = manager.preferences_for_type(SensorType::Trainer);
    assert_eq!(trainers.len(), 1);
    assert_eq!(trainers[0].preferred_protocol, SensorProtocol::AntPlus);

    let hrs = manager.preferences_for_type(SensorType::HeartRate);
    assert_eq!(hrs.len(), 1);
    assert_eq!(hrs[0].preferred_protocol, SensorProtocol::Ble);

    let powers = manager.preferences_for_type(SensorType::PowerMeter);
    assert_eq!(powers.len(), 1);
    assert_eq!(powers[0].preferred_protocol, SensorProtocol::AntPlus);
}
