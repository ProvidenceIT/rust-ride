//! Unit tests for sensor device cache.
//!
//! Tests verify that:
//! - CachedSensor stores device information correctly
//! - SensorCache provides fast lookup and persistence
//! - Cache prioritization works for reconnection
//! - Stale sensors are properly pruned

use rustride::sensors::cache::{CachedSensor, SensorCache};
use rustride::sensors::types::{Protocol, SensorType};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test cache with a temporary directory.
fn create_test_cache() -> (SensorCache, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("test_sensor_cache.json");
    let cache = SensorCache::with_path(cache_path);
    (cache, temp_dir)
}

// ============================================================================
// CachedSensor Tests
// ============================================================================

#[test]
fn test_cached_sensor_creation() {
    let sensor = CachedSensor::new(
        "00:11:22:33:44:55".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert_eq!(sensor.device_id, "00:11:22:33:44:55");
    assert_eq!(sensor.name, "KICKR CORE 1234");
    assert_eq!(sensor.sensor_type, SensorType::Trainer);
    assert_eq!(sensor.protocol, Protocol::BleFtms);
    assert_eq!(sensor.connection_count, 1);
    assert!(!sensor.is_preferred);
    assert!(sensor.nickname.is_none());
    assert!(!sensor.is_stale()); // Just created, not stale
}

#[test]
fn test_cached_sensor_mark_connected() {
    let mut sensor = CachedSensor::new(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert_eq!(sensor.connection_count, 1);

    sensor.mark_connected();
    assert_eq!(sensor.connection_count, 2);

    sensor.mark_connected();
    sensor.mark_connected();
    assert_eq!(sensor.connection_count, 4);
}

#[test]
fn test_cached_sensor_display_name_without_nickname() {
    let sensor = CachedSensor::new(
        "device1".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert_eq!(sensor.display_name(), "KICKR CORE 1234");
}

#[test]
fn test_cached_sensor_display_name_with_nickname() {
    let mut sensor = CachedSensor::new(
        "device1".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    sensor.nickname = Some("Pain Cave Trainer".to_string());
    assert_eq!(sensor.display_name(), "Pain Cave Trainer");
}

// ============================================================================
// SensorCache Basic Operations Tests
// ============================================================================

#[test]
fn test_sensor_cache_new_is_empty() {
    let cache = SensorCache::new();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_sensor_cache_add_sensor() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert!(!cache.is_empty());
    assert_eq!(cache.len(), 1);
    assert!(cache.contains("device1"));
}

#[test]
fn test_sensor_cache_get_sensor() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "My Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    let sensor = cache.get("device1").expect("Sensor should exist");
    assert_eq!(sensor.name, "My Trainer");
    assert_eq!(sensor.sensor_type, SensorType::Trainer);
}

#[test]
fn test_sensor_cache_get_nonexistent() {
    let cache = SensorCache::new();

    assert!(cache.get("nonexistent").is_none());
}

#[test]
fn test_sensor_cache_update_existing() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer v1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    let count_before = cache.get("device1").unwrap().connection_count;

    // Cache the same device again (simulating reconnection)
    cache.cache_sensor(
        "device1".to_string(),
        "Trainer v2".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Should still be one sensor, but updated
    assert_eq!(cache.len(), 1);

    let sensor = cache.get("device1").unwrap();
    assert_eq!(sensor.name, "Trainer v2"); // Name updated
    assert_eq!(sensor.connection_count, count_before + 1); // Count incremented
}

#[test]
fn test_sensor_cache_remove() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert!(cache.contains("device1"));

    let removed = cache.remove("device1");
    assert!(removed.is_some());
    assert!(!cache.contains("device1"));
    assert!(cache.is_empty());
}

#[test]
fn test_sensor_cache_remove_nonexistent() {
    let (mut cache, _temp_dir) = create_test_cache();

    let removed = cache.remove("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_sensor_cache_clear() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device2".to_string(),
        "HR Monitor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    assert_eq!(cache.len(), 2);

    cache.clear();
    assert!(cache.is_empty());
}

// ============================================================================
// SensorCache Filtering Tests
// ============================================================================

#[test]
fn test_sensor_cache_sensors_of_type() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "trainer1".to_string(),
        "Trainer 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "trainer2".to_string(),
        "Trainer 2".to_string(),
        SensorType::Trainer,
        Protocol::AntFec,
    );
    cache.cache_sensor(
        "hr1".to_string(),
        "HR Monitor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );
    cache.cache_sensor(
        "power1".to_string(),
        "Power Meter".to_string(),
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    );

    let trainers = cache.sensors_of_type(SensorType::Trainer);
    assert_eq!(trainers.len(), 2);

    let hr_sensors = cache.sensors_of_type(SensorType::HeartRate);
    assert_eq!(hr_sensors.len(), 1);
    assert_eq!(hr_sensors[0].device_id, "hr1");

    let power_meters = cache.sensors_of_type(SensorType::PowerMeter);
    assert_eq!(power_meters.len(), 1);

    let cadence_sensors = cache.sensors_of_type(SensorType::Cadence);
    assert!(cadence_sensors.is_empty());
}

#[test]
fn test_sensor_cache_all_sensors() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device2".to_string(),
        "HR".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    let all: Vec<_> = cache.all_sensors().collect();
    assert_eq!(all.len(), 2);
}

// ============================================================================
// SensorCache Preference Tests
// ============================================================================

#[test]
fn test_sensor_cache_set_preferred() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device2".to_string(),
        "Trainer 2".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Initially not preferred
    assert!(!cache.get("device1").unwrap().is_preferred);
    assert!(!cache.get("device2").unwrap().is_preferred);

    // Set device1 as preferred
    assert!(cache.set_preferred("device1", true));
    assert!(cache.get("device1").unwrap().is_preferred);
    assert!(!cache.get("device2").unwrap().is_preferred);

    // Set device2 as preferred too
    assert!(cache.set_preferred("device2", true));
    assert!(cache.get("device1").unwrap().is_preferred);
    assert!(cache.get("device2").unwrap().is_preferred);

    // Unset device1
    assert!(cache.set_preferred("device1", false));
    assert!(!cache.get("device1").unwrap().is_preferred);
}

#[test]
fn test_sensor_cache_set_preferred_nonexistent() {
    let (mut cache, _temp_dir) = create_test_cache();

    assert!(!cache.set_preferred("nonexistent", true));
}

#[test]
fn test_sensor_cache_preferred_sensors() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Trainer 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device2".to_string(),
        "Trainer 2".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device3".to_string(),
        "HR Monitor".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    cache.set_preferred("device1", true);
    cache.set_preferred("device3", true);

    let preferred = cache.preferred_sensors();
    assert_eq!(preferred.len(), 2);

    let preferred_ids: Vec<_> = preferred.iter().map(|s| s.device_id.as_str()).collect();
    assert!(preferred_ids.contains(&"device1"));
    assert!(preferred_ids.contains(&"device3"));
}

// ============================================================================
// SensorCache Nickname Tests
// ============================================================================

#[test]
fn test_sensor_cache_set_nickname() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    assert!(cache.set_nickname("device1", Some("Pain Cave".to_string())));

    let sensor = cache.get("device1").unwrap();
    assert_eq!(sensor.nickname, Some("Pain Cave".to_string()));
    assert_eq!(sensor.display_name(), "Pain Cave");
}

#[test]
fn test_sensor_cache_clear_nickname() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "KICKR CORE 1234".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    cache.set_nickname("device1", Some("Pain Cave".to_string()));
    cache.set_nickname("device1", None);

    let sensor = cache.get("device1").unwrap();
    assert!(sensor.nickname.is_none());
    assert_eq!(sensor.display_name(), "KICKR CORE 1234");
}

// ============================================================================
// SensorCache Priority Tests
// ============================================================================

#[test]
fn test_sensor_cache_reconnection_priority_prefers_preferred() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "regular".to_string(),
        "Regular".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "preferred".to_string(),
        "Preferred".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Simulate more connections on regular device
    for _ in 0..5 {
        if let Some(s) = cache.get_mut("regular") {
            s.mark_connected();
        }
    }

    // Set preferred
    cache.set_preferred("preferred", true);

    let priority = cache.reconnection_priority();
    assert!(!priority.is_empty());
    // Preferred should come first despite fewer connections
    assert_eq!(priority[0].device_id, "preferred");
}

#[test]
fn test_sensor_cache_reconnection_priority_by_connection_count() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "rarely_used".to_string(),
        "Rarely Used".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "frequently_used".to_string(),
        "Frequently Used".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Simulate more connections on frequently_used
    for _ in 0..10 {
        if let Some(s) = cache.get_mut("frequently_used") {
            s.mark_connected();
        }
    }

    let priority = cache.reconnection_priority();
    assert_eq!(priority.len(), 2);
    // Frequently used should come first
    assert_eq!(priority[0].device_id, "frequently_used");
}

// ============================================================================
// SensorCache Persistence Tests
// ============================================================================

#[test]
fn test_sensor_cache_save_and_load() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("test_cache.json");

    // Create and populate cache
    {
        let mut cache = SensorCache::with_path(cache_path.clone());
        cache.cache_sensor(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        cache.cache_sensor(
            "device2".to_string(),
            "HR Monitor".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );
        cache.set_preferred("device1", true);
        cache.set_nickname("device1", Some("My Trainer".to_string()));
        cache.save().expect("Failed to save cache");
    }

    // Load cache from disk
    let loaded_cache = SensorCache::load_from_path(cache_path);

    assert_eq!(loaded_cache.len(), 2);
    assert!(loaded_cache.contains("device1"));
    assert!(loaded_cache.contains("device2"));

    let sensor1 = loaded_cache.get("device1").unwrap();
    assert_eq!(sensor1.name, "Trainer");
    assert!(sensor1.is_preferred);
    assert_eq!(sensor1.nickname, Some("My Trainer".to_string()));

    let sensor2 = loaded_cache.get("device2").unwrap();
    assert_eq!(sensor2.name, "HR Monitor");
    assert_eq!(sensor2.sensor_type, SensorType::HeartRate);
}

#[test]
fn test_sensor_cache_load_nonexistent_returns_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("nonexistent.json");

    let cache = SensorCache::load_from_path(cache_path);
    assert!(cache.is_empty());
}

#[test]
fn test_sensor_cache_load_invalid_json_returns_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("invalid.json");

    // Write invalid JSON
    std::fs::write(&cache_path, "{ invalid json }").expect("Failed to write file");

    let cache = SensorCache::load_from_path(cache_path);
    assert!(cache.is_empty());
}

// ============================================================================
// SensorCache Stale Pruning Tests
// ============================================================================

#[test]
fn test_sensor_cache_prune_stale_removes_old() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "fresh".to_string(),
        "Fresh".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Add a stale sensor by manipulating last_connected
    cache.cache_sensor(
        "stale".to_string(),
        "Stale".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    if let Some(sensor) = cache.get_mut("stale") {
        // Set last_connected to 30 days ago
        sensor.last_connected = chrono::Utc::now() - chrono::Duration::days(30);
    }

    assert_eq!(cache.len(), 2);

    let pruned = cache.prune_stale();
    assert_eq!(pruned, 1);
    assert_eq!(cache.len(), 1);
    assert!(cache.contains("fresh"));
    assert!(!cache.contains("stale"));
}

#[test]
fn test_sensor_cache_prune_stale_keeps_recent() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "device1".to_string(),
        "Device 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    cache.cache_sensor(
        "device2".to_string(),
        "Device 2".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    // Both are fresh, none should be pruned
    let pruned = cache.prune_stale();
    assert_eq!(pruned, 0);
    assert_eq!(cache.len(), 2);
}

// ============================================================================
// SensorCache Recent Sensors Tests
// ============================================================================

#[test]
fn test_sensor_cache_recent_sensors_ordered() {
    let (mut cache, _temp_dir) = create_test_cache();

    cache.cache_sensor(
        "oldest".to_string(),
        "Oldest".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );

    // Sleep briefly to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));

    cache.cache_sensor(
        "newest".to_string(),
        "Newest".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );

    let recent = cache.recent_sensors();
    assert_eq!(recent.len(), 2);
    // Newest should be first
    assert_eq!(recent[0].device_id, "newest");
    assert_eq!(recent[1].device_id, "oldest");
}
