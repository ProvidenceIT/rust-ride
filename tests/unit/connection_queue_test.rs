//! Unit tests for priority-based connection queue.
//!
//! Tests verify that:
//! - Sensors are prioritized correctly (trainers/power meters before HR/cadence)
//! - Preferred sensors are connected first within their priority level
//! - Queue operations work correctly (enqueue, dequeue, remove, etc.)
//! - Deduplication prevents adding the same sensor twice

use rustride::sensors::connection_queue::{ConnectionQueue, ConnectionQueueEntry, SensorPriority};
use rustride::sensors::types::{DiscoveredSensor, Protocol, SensorType};
use std::time::Instant;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create a discovered sensor with minimal configuration.
fn make_sensor(device_id: &str, name: &str, sensor_type: SensorType) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type,
        protocol: Protocol::BleFtms,
        signal_strength: Some(-50),
        last_seen: Instant::now(),
    }
}

/// Helper to create a trainer sensor.
fn make_trainer(device_id: &str, name: &str) -> DiscoveredSensor {
    make_sensor(device_id, name, SensorType::Trainer)
}

/// Helper to create a power meter sensor.
fn make_power_meter(device_id: &str, name: &str) -> DiscoveredSensor {
    make_sensor(device_id, name, SensorType::PowerMeter)
}

/// Helper to create a heart rate sensor.
fn make_hr_sensor(device_id: &str, name: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::HeartRate,
        protocol: Protocol::BleHeartRate,
        signal_strength: Some(-60),
        last_seen: Instant::now(),
    }
}

/// Helper to create a cadence sensor.
fn make_cadence_sensor(device_id: &str, name: &str) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: device_id.to_string(),
        name: name.to_string(),
        sensor_type: SensorType::Cadence,
        protocol: Protocol::BleCsc,
        signal_strength: Some(-55),
        last_seen: Instant::now(),
    }
}

// ============================================================================
// SensorPriority Tests
// ============================================================================

#[test]
fn test_sensor_priority_from_trainer() {
    let priority = SensorPriority::from_sensor_type(SensorType::Trainer);
    assert_eq!(priority, SensorPriority::Primary);
}

#[test]
fn test_sensor_priority_from_smart_trainer() {
    let priority = SensorPriority::from_sensor_type(SensorType::SmartTrainer);
    assert_eq!(priority, SensorPriority::Primary);
}

#[test]
fn test_sensor_priority_from_power_meter() {
    let priority = SensorPriority::from_sensor_type(SensorType::PowerMeter);
    assert_eq!(priority, SensorPriority::Primary);
}

#[test]
fn test_sensor_priority_from_heart_rate() {
    let priority = SensorPriority::from_sensor_type(SensorType::HeartRate);
    assert_eq!(priority, SensorPriority::Secondary);
}

#[test]
fn test_sensor_priority_from_cadence() {
    let priority = SensorPriority::from_sensor_type(SensorType::Cadence);
    assert_eq!(priority, SensorPriority::Secondary);
}

#[test]
fn test_sensor_priority_from_speed() {
    let priority = SensorPriority::from_sensor_type(SensorType::Speed);
    assert_eq!(priority, SensorPriority::Secondary);
}

#[test]
fn test_sensor_priority_from_speed_cadence() {
    let priority = SensorPriority::from_sensor_type(SensorType::SpeedCadence);
    assert_eq!(priority, SensorPriority::Secondary);
}

#[test]
fn test_primary_has_lower_priority_value() {
    // Lower value = higher priority (processed first)
    assert!(SensorPriority::Primary.priority_value() < SensorPriority::Secondary.priority_value());
}

// ============================================================================
// ConnectionQueueEntry Tests
// ============================================================================

#[test]
fn test_queue_entry_new_sets_priority() {
    let trainer = make_trainer("trainer1", "KICKR CORE");
    let entry = ConnectionQueueEntry::new(trainer);

    assert_eq!(entry.priority, SensorPriority::Primary);
    assert!(!entry.is_preferred);
}

#[test]
fn test_queue_entry_new_secondary() {
    let hr = make_hr_sensor("hr1", "Garmin HRM");
    let entry = ConnectionQueueEntry::new(hr);

    assert_eq!(entry.priority, SensorPriority::Secondary);
}

#[test]
fn test_queue_entry_preferred() {
    let trainer = make_trainer("trainer1", "KICKR CORE");
    let entry = ConnectionQueueEntry::preferred(trainer);

    assert!(entry.is_preferred);
    assert_eq!(entry.priority, SensorPriority::Primary);
}

#[test]
fn test_queue_entry_accessors() {
    let trainer = make_trainer("trainer1", "KICKR CORE");
    let entry = ConnectionQueueEntry::new(trainer);

    assert_eq!(entry.device_id(), "trainer1");
    assert_eq!(entry.name(), "KICKR CORE");
    assert_eq!(entry.sensor_type(), SensorType::Trainer);
    assert_eq!(entry.protocol(), Protocol::BleFtms);
}

// ============================================================================
// ConnectionQueue Basic Operations Tests
// ============================================================================

#[test]
fn test_queue_new_is_empty() {
    let queue = ConnectionQueue::new();

    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_queue_enqueue() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));

    assert!(!queue.is_empty());
    assert_eq!(queue.len(), 1);
    assert!(queue.contains("trainer1"));
}

#[test]
fn test_queue_dequeue() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));

    let entry = queue.dequeue();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().device_id(), "trainer1");
    assert!(queue.is_empty());
}

#[test]
fn test_queue_dequeue_empty() {
    let mut queue = ConnectionQueue::new();

    assert!(queue.dequeue().is_none());
}

#[test]
fn test_queue_peek() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));

    let peeked = queue.peek();
    assert!(peeked.is_some());
    assert_eq!(peeked.unwrap().device_id(), "trainer1");

    // Peek should not remove the entry
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_queue_remove() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_hr_sensor("hr1", "Garmin HRM"));

    assert_eq!(queue.len(), 2);
    assert!(queue.remove("trainer1"));
    assert_eq!(queue.len(), 1);
    assert!(!queue.contains("trainer1"));
    assert!(queue.contains("hr1"));
}

#[test]
fn test_queue_remove_nonexistent() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));

    assert!(!queue.remove("nonexistent"));
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_queue_clear() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_hr_sensor("hr1", "Garmin HRM"));
    queue.enqueue(make_power_meter("power1", "Assioma"));

    assert_eq!(queue.len(), 3);

    queue.clear();

    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

// ============================================================================
// ConnectionQueue Deduplication Tests
// ============================================================================

#[test]
fn test_queue_deduplication() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_trainer("trainer1", "KICKR"));

    // Should only have one entry
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_queue_deduplication_different_names() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR v1"));
    queue.enqueue(make_trainer("trainer1", "KICKR v2")); // Same ID, different name

    // Should deduplicate by device_id
    assert_eq!(queue.len(), 1);
}

// ============================================================================
// ConnectionQueue Priority Ordering Tests
// ============================================================================

#[test]
fn test_queue_primary_before_secondary() {
    let mut queue = ConnectionQueue::new();

    // Add secondary sensors first
    queue.enqueue(make_hr_sensor("hr1", "Garmin HRM"));
    queue.enqueue(make_cadence_sensor("cadence1", "Wahoo Cadence"));

    // Add primary sensors after
    queue.enqueue(make_trainer("trainer1", "KICKR CORE"));
    queue.enqueue(make_power_meter("power1", "Assioma"));

    assert_eq!(queue.len(), 4);

    // Should dequeue primary sensors first
    let first = queue.dequeue().unwrap();
    assert_eq!(first.priority, SensorPriority::Primary);

    let second = queue.dequeue().unwrap();
    assert_eq!(second.priority, SensorPriority::Primary);

    // Then secondary sensors
    let third = queue.dequeue().unwrap();
    assert_eq!(third.priority, SensorPriority::Secondary);

    let fourth = queue.dequeue().unwrap();
    assert_eq!(fourth.priority, SensorPriority::Secondary);

    assert!(queue.is_empty());
}

#[test]
fn test_queue_trainer_and_power_meter_first() {
    let mut queue = ConnectionQueue::new();

    // Mix of sensors added in various order
    queue.enqueue(make_hr_sensor("hr1", "HR Monitor"));
    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_cadence_sensor("cadence1", "Cadence"));
    queue.enqueue(make_power_meter("power1", "Power Meter"));

    // Get first two - should be trainer and power meter (primary)
    let first = queue.dequeue().unwrap();
    let second = queue.dequeue().unwrap();

    assert_eq!(first.priority, SensorPriority::Primary);
    assert_eq!(second.priority, SensorPriority::Primary);

    // Verify they are trainer and power meter
    let primary_ids: Vec<_> = vec![first.device_id(), second.device_id()];
    assert!(primary_ids.contains(&"trainer1"));
    assert!(primary_ids.contains(&"power1"));
}

#[test]
fn test_queue_preferred_first_within_priority() {
    let mut queue = ConnectionQueue::new();

    // Add non-preferred trainer first
    queue.enqueue(make_trainer("trainer1", "KICKR"));

    // Add preferred trainer second
    queue.enqueue_preferred(make_trainer("trainer2", "Wahoo KICKR Preferred"));

    // Preferred should come first despite being added later
    let first = queue.dequeue().unwrap();
    assert_eq!(first.device_id(), "trainer2");
    assert!(first.is_preferred);

    let second = queue.dequeue().unwrap();
    assert_eq!(second.device_id(), "trainer1");
    assert!(!second.is_preferred);
}

#[test]
fn test_queue_preferred_secondary_after_regular_primary() {
    let mut queue = ConnectionQueue::new();

    // Add preferred HR sensor
    queue.enqueue_preferred(make_hr_sensor("hr1", "Preferred HR"));

    // Add regular trainer
    queue.enqueue(make_trainer("trainer1", "Regular Trainer"));

    // Regular primary should still come before preferred secondary
    let first = queue.dequeue().unwrap();
    assert_eq!(first.device_id(), "trainer1");
    assert_eq!(first.priority, SensorPriority::Primary);

    let second = queue.dequeue().unwrap();
    assert_eq!(second.device_id(), "hr1");
    assert_eq!(second.priority, SensorPriority::Secondary);
}

// ============================================================================
// ConnectionQueue Batch Operations Tests
// ============================================================================

#[test]
fn test_queue_enqueue_all() {
    let mut queue = ConnectionQueue::new();

    let sensors = vec![
        make_trainer("trainer1", "KICKR"),
        make_hr_sensor("hr1", "Garmin HRM"),
        make_power_meter("power1", "Assioma"),
        make_cadence_sensor("cadence1", "Wahoo"),
    ];

    queue.enqueue_all(sensors);

    assert_eq!(queue.len(), 4);
}

#[test]
fn test_queue_drain_in_order() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_hr_sensor("hr1", "HR Monitor"));
    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_cadence_sensor("cadence1", "Cadence"));
    queue.enqueue(make_power_meter("power1", "Power Meter"));

    let drained = queue.drain_in_order();

    assert_eq!(drained.len(), 4);
    assert!(queue.is_empty());

    // First two should be primary
    assert_eq!(drained[0].priority, SensorPriority::Primary);
    assert_eq!(drained[1].priority, SensorPriority::Primary);

    // Last two should be secondary
    assert_eq!(drained[2].priority, SensorPriority::Secondary);
    assert_eq!(drained[3].priority, SensorPriority::Secondary);
}

#[test]
fn test_queue_iter_by_priority() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_hr_sensor("hr1", "HR Monitor"));
    queue.enqueue(make_trainer("trainer1", "KICKR"));

    let ordered = queue.iter_by_priority();

    assert_eq!(ordered.len(), 2);
    // Primary (trainer) should be first
    assert_eq!(ordered[0].priority, SensorPriority::Primary);
    // Secondary (HR) should be second
    assert_eq!(ordered[1].priority, SensorPriority::Secondary);

    // Non-destructive - queue should still have items
    assert_eq!(queue.len(), 2);
}

// ============================================================================
// ConnectionQueue Count Tests
// ============================================================================

#[test]
fn test_queue_count_by_priority() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_power_meter("power1", "Assioma"));
    queue.enqueue(make_hr_sensor("hr1", "Garmin"));
    queue.enqueue(make_cadence_sensor("cadence1", "Wahoo"));
    queue.enqueue(make_sensor("speed1", "Speed", SensorType::Speed));

    let (primary, secondary) = queue.count_by_priority();

    assert_eq!(primary, 2);   // trainer + power meter
    assert_eq!(secondary, 3); // hr + cadence + speed
}

#[test]
fn test_queue_primary_sensors() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_hr_sensor("hr1", "Garmin"));
    queue.enqueue(make_power_meter("power1", "Assioma"));

    let primary = queue.primary_sensors();

    assert_eq!(primary.len(), 2);
    let ids: Vec<_> = primary.iter().map(|e| e.device_id()).collect();
    assert!(ids.contains(&"trainer1"));
    assert!(ids.contains(&"power1"));
}

#[test]
fn test_queue_secondary_sensors() {
    let mut queue = ConnectionQueue::new();

    queue.enqueue(make_trainer("trainer1", "KICKR"));
    queue.enqueue(make_hr_sensor("hr1", "Garmin"));
    queue.enqueue(make_cadence_sensor("cadence1", "Wahoo"));

    let secondary = queue.secondary_sensors();

    assert_eq!(secondary.len(), 2);
    let ids: Vec<_> = secondary.iter().map(|e| e.device_id()).collect();
    assert!(ids.contains(&"hr1"));
    assert!(ids.contains(&"cadence1"));
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_typical_sensor_setup_connection_order() {
    // Simulate a typical setup with:
    // - Smart trainer (primary)
    // - Power meter (primary, preferred as it's the user's preferred power source)
    // - Heart rate monitor (secondary)
    // - Cadence sensor (secondary)

    let mut queue = ConnectionQueue::new();

    // User has a preferred power meter they want to use for power data
    queue.enqueue_preferred(make_power_meter("assioma", "Favero Assioma Duo"));

    // Other sensors discovered
    queue.enqueue(make_hr_sensor("hrm-pro", "Garmin HRM-Pro"));
    queue.enqueue(make_trainer("kickr", "Wahoo KICKR v5"));
    queue.enqueue(make_cadence_sensor("cadence", "Wahoo Cadence"));

    // Connection order should be:
    // 1. Preferred power meter (primary + preferred)
    // 2. Trainer (primary)
    // 3. HR monitor (secondary)
    // 4. Cadence (secondary)

    let first = queue.dequeue().unwrap();
    assert_eq!(first.device_id(), "assioma");
    assert!(first.is_preferred);
    assert_eq!(first.priority, SensorPriority::Primary);

    let second = queue.dequeue().unwrap();
    assert_eq!(second.device_id(), "kickr");
    assert_eq!(second.priority, SensorPriority::Primary);

    let third = queue.dequeue().unwrap();
    assert_eq!(third.priority, SensorPriority::Secondary);

    let fourth = queue.dequeue().unwrap();
    assert_eq!(fourth.priority, SensorPriority::Secondary);
}

#[test]
fn test_multiple_trainers_fifo_within_priority() {
    let mut queue = ConnectionQueue::new();

    // Add multiple trainers - should maintain FIFO order within same priority
    queue.enqueue(make_trainer("trainer1", "First Trainer"));

    // Small sleep to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(1));

    queue.enqueue(make_trainer("trainer2", "Second Trainer"));

    std::thread::sleep(std::time::Duration::from_millis(1));

    queue.enqueue(make_trainer("trainer3", "Third Trainer"));

    // Should dequeue in FIFO order since all have same priority
    let first = queue.dequeue().unwrap();
    assert_eq!(first.device_id(), "trainer1");

    let second = queue.dequeue().unwrap();
    assert_eq!(second.device_id(), "trainer2");

    let third = queue.dequeue().unwrap();
    assert_eq!(third.device_id(), "trainer3");
}
