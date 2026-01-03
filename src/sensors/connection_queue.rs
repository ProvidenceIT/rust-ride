//! Priority-based connection queue for sensors.
//!
//! Implements a connection queue that prioritizes primary sensors (trainers,
//! power meters) over secondary sensors (heart rate, cadence). This ensures
//! the most critical sensors for training are connected first.

use crate::sensors::types::{DiscoveredSensor, Protocol, SensorType};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Priority level for sensor connection.
///
/// Primary sensors are essential for training (trainers, power meters).
/// Secondary sensors provide supplementary data (HR, cadence, speed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorPriority {
    /// Critical sensors: Smart trainers and power meters.
    /// These should connect first as they are essential for structured training.
    Primary,
    /// Supplementary sensors: Heart rate, cadence, speed, etc.
    /// Important but training can proceed without them.
    Secondary,
}

impl SensorPriority {
    /// Get the numeric priority value (lower is higher priority).
    pub fn priority_value(&self) -> u8 {
        match self {
            SensorPriority::Primary => 0,
            SensorPriority::Secondary => 1,
        }
    }

    /// Determine the priority for a given sensor type.
    pub fn from_sensor_type(sensor_type: SensorType) -> Self {
        match sensor_type {
            // Primary: Essential for training control and power data
            SensorType::Trainer => SensorPriority::Primary,
            SensorType::SmartTrainer => SensorPriority::Primary,
            SensorType::PowerMeter => SensorPriority::Primary,

            // Secondary: Supplementary data sensors
            SensorType::HeartRate => SensorPriority::Secondary,
            SensorType::Cadence => SensorPriority::Secondary,
            SensorType::CadenceSensor => SensorPriority::Secondary,
            SensorType::Speed => SensorPriority::Secondary,
            SensorType::SpeedCadence => SensorPriority::Secondary,
            SensorType::SmO2 => SensorPriority::Secondary,
            SensorType::Imu => SensorPriority::Secondary,
        }
    }
}

impl std::fmt::Display for SensorPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorPriority::Primary => write!(f, "Primary"),
            SensorPriority::Secondary => write!(f, "Secondary"),
        }
    }
}

/// An entry in the connection queue with priority and timing information.
#[derive(Debug, Clone)]
pub struct ConnectionQueueEntry {
    /// The discovered sensor to connect to.
    pub sensor: DiscoveredSensor,
    /// Priority level for this sensor.
    pub priority: SensorPriority,
    /// When this entry was added to the queue (for FIFO ordering within priority).
    added_at: std::time::Instant,
    /// Whether this sensor is marked as preferred by the user.
    pub is_preferred: bool,
}

impl ConnectionQueueEntry {
    /// Create a new queue entry from a discovered sensor.
    pub fn new(sensor: DiscoveredSensor) -> Self {
        Self {
            priority: SensorPriority::from_sensor_type(sensor.sensor_type),
            sensor,
            added_at: std::time::Instant::now(),
            is_preferred: false,
        }
    }

    /// Create a new queue entry with a custom priority.
    pub fn with_priority(sensor: DiscoveredSensor, priority: SensorPriority) -> Self {
        Self {
            priority,
            sensor,
            added_at: std::time::Instant::now(),
            is_preferred: false,
        }
    }

    /// Create a preferred queue entry (user's preferred sensor).
    pub fn preferred(sensor: DiscoveredSensor) -> Self {
        Self {
            priority: SensorPriority::from_sensor_type(sensor.sensor_type),
            sensor,
            added_at: std::time::Instant::now(),
            is_preferred: true,
        }
    }

    /// Get the device ID of this sensor.
    pub fn device_id(&self) -> &str {
        &self.sensor.device_id
    }

    /// Get the name of this sensor.
    pub fn name(&self) -> &str {
        &self.sensor.name
    }

    /// Get the sensor type.
    pub fn sensor_type(&self) -> SensorType {
        self.sensor.sensor_type
    }

    /// Get the protocol.
    pub fn protocol(&self) -> Protocol {
        self.sensor.protocol
    }
}

// Implement Eq/PartialEq based on device_id for deduplication
impl PartialEq for ConnectionQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.sensor.device_id == other.sensor.device_id
    }
}

impl Eq for ConnectionQueueEntry {}

// Implement Ord for priority queue ordering
// Higher priority = should be popped first = "greater" in the heap
impl Ord for ConnectionQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // First, preferred sensors come before non-preferred
        match (self.is_preferred, other.is_preferred) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }

        // Then, lower priority value means higher actual priority
        match self.priority.priority_value().cmp(&other.priority.priority_value()) {
            Ordering::Less => Ordering::Greater,  // Lower value = higher priority
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => {
                // Within same priority, FIFO ordering (earlier added = higher priority)
                other.added_at.cmp(&self.added_at)
            }
        }
    }
}

impl PartialOrd for ConnectionQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A priority-based queue for connecting to discovered sensors.
///
/// Ensures that primary sensors (trainers, power meters) are connected
/// before secondary sensors (heart rate, cadence). Within each priority
/// level, preferred sensors are connected first, then FIFO ordering.
#[derive(Debug, Default)]
pub struct ConnectionQueue {
    /// Priority heap for sensor connection ordering.
    heap: BinaryHeap<ConnectionQueueEntry>,
    /// Set of device IDs in the queue (for deduplication).
    device_ids: std::collections::HashSet<String>,
}

impl ConnectionQueue {
    /// Create a new empty connection queue.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            device_ids: std::collections::HashSet::new(),
        }
    }

    /// Add a discovered sensor to the queue.
    ///
    /// If the sensor is already in the queue, this is a no-op.
    pub fn enqueue(&mut self, sensor: DiscoveredSensor) {
        if !self.device_ids.contains(&sensor.device_id) {
            let device_id = sensor.device_id.clone();
            self.heap.push(ConnectionQueueEntry::new(sensor));
            self.device_ids.insert(device_id);
        }
    }

    /// Add a discovered sensor to the queue with a specific priority.
    ///
    /// If the sensor is already in the queue, this is a no-op.
    pub fn enqueue_with_priority(&mut self, sensor: DiscoveredSensor, priority: SensorPriority) {
        if !self.device_ids.contains(&sensor.device_id) {
            let device_id = sensor.device_id.clone();
            self.heap.push(ConnectionQueueEntry::with_priority(sensor, priority));
            self.device_ids.insert(device_id);
        }
    }

    /// Add a preferred sensor to the queue (highest priority within its level).
    ///
    /// If the sensor is already in the queue, this is a no-op.
    pub fn enqueue_preferred(&mut self, sensor: DiscoveredSensor) {
        if !self.device_ids.contains(&sensor.device_id) {
            let device_id = sensor.device_id.clone();
            self.heap.push(ConnectionQueueEntry::preferred(sensor));
            self.device_ids.insert(device_id);
        }
    }

    /// Add multiple sensors to the queue.
    ///
    /// Sensors are automatically prioritized based on their type.
    pub fn enqueue_all(&mut self, sensors: impl IntoIterator<Item = DiscoveredSensor>) {
        for sensor in sensors {
            self.enqueue(sensor);
        }
    }

    /// Get the next sensor to connect to (highest priority first).
    ///
    /// Returns None if the queue is empty.
    pub fn dequeue(&mut self) -> Option<ConnectionQueueEntry> {
        if let Some(entry) = self.heap.pop() {
            self.device_ids.remove(&entry.sensor.device_id);
            Some(entry)
        } else {
            None
        }
    }

    /// Peek at the next sensor without removing it.
    pub fn peek(&self) -> Option<&ConnectionQueueEntry> {
        self.heap.peek()
    }

    /// Check if a sensor is in the queue.
    pub fn contains(&self, device_id: &str) -> bool {
        self.device_ids.contains(device_id)
    }

    /// Remove a sensor from the queue by device ID.
    ///
    /// Returns true if the sensor was removed, false if not found.
    pub fn remove(&mut self, device_id: &str) -> bool {
        if self.device_ids.remove(device_id) {
            // Rebuild the heap without this device
            let entries: Vec<_> = self.heap.drain().collect();
            for entry in entries {
                if entry.sensor.device_id != device_id {
                    self.heap.push(entry);
                }
            }
            true
        } else {
            false
        }
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Get the number of sensors in the queue.
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Clear all sensors from the queue.
    pub fn clear(&mut self) {
        self.heap.clear();
        self.device_ids.clear();
    }

    /// Get all entries in priority order (drains the queue).
    ///
    /// Returns entries from highest to lowest priority.
    pub fn drain_in_order(&mut self) -> Vec<ConnectionQueueEntry> {
        let mut result = Vec::with_capacity(self.heap.len());
        while let Some(entry) = self.dequeue() {
            result.push(entry);
        }
        result
    }

    /// Get a view of all entries sorted by priority (non-destructive).
    ///
    /// Returns a vector of references in priority order.
    pub fn iter_by_priority(&self) -> Vec<&ConnectionQueueEntry> {
        let mut entries: Vec<_> = self.heap.iter().collect();
        entries.sort_by(|a, b| b.cmp(a)); // Reverse for highest priority first
        entries
    }

    /// Count sensors at each priority level.
    pub fn count_by_priority(&self) -> (usize, usize) {
        let primary = self.heap.iter()
            .filter(|e| e.priority == SensorPriority::Primary)
            .count();
        let secondary = self.heap.iter()
            .filter(|e| e.priority == SensorPriority::Secondary)
            .count();
        (primary, secondary)
    }

    /// Get all primary sensors in the queue.
    pub fn primary_sensors(&self) -> Vec<&ConnectionQueueEntry> {
        self.heap.iter()
            .filter(|e| e.priority == SensorPriority::Primary)
            .collect()
    }

    /// Get all secondary sensors in the queue.
    pub fn secondary_sensors(&self) -> Vec<&ConnectionQueueEntry> {
        self.heap.iter()
            .filter(|e| e.priority == SensorPriority::Secondary)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

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

    #[test]
    fn test_sensor_priority_from_type() {
        // Primary types
        assert_eq!(SensorPriority::from_sensor_type(SensorType::Trainer), SensorPriority::Primary);
        assert_eq!(SensorPriority::from_sensor_type(SensorType::SmartTrainer), SensorPriority::Primary);
        assert_eq!(SensorPriority::from_sensor_type(SensorType::PowerMeter), SensorPriority::Primary);

        // Secondary types
        assert_eq!(SensorPriority::from_sensor_type(SensorType::HeartRate), SensorPriority::Secondary);
        assert_eq!(SensorPriority::from_sensor_type(SensorType::Cadence), SensorPriority::Secondary);
        assert_eq!(SensorPriority::from_sensor_type(SensorType::Speed), SensorPriority::Secondary);
    }

    #[test]
    fn test_queue_entry_from_sensor() {
        let trainer = make_sensor("trainer1", "KICKR", SensorType::Trainer);
        let entry = ConnectionQueueEntry::new(trainer);

        assert_eq!(entry.priority, SensorPriority::Primary);
        assert_eq!(entry.device_id(), "trainer1");
        assert!(!entry.is_preferred);
    }

    #[test]
    fn test_queue_basic_operations() {
        let mut queue = ConnectionQueue::new();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));

        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);
        assert!(queue.contains("trainer1"));
    }

    #[test]
    fn test_queue_prioritizes_primary_sensors() {
        let mut queue = ConnectionQueue::new();

        // Add sensors in reverse priority order
        queue.enqueue(make_sensor("hr1", "HR Monitor", SensorType::HeartRate));
        queue.enqueue(make_sensor("cadence1", "Cadence", SensorType::Cadence));
        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("power1", "Power Meter", SensorType::PowerMeter));

        assert_eq!(queue.len(), 4);

        // Dequeue should return primary sensors first
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
    fn test_queue_deduplication() {
        let mut queue = ConnectionQueue::new();

        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_preferred_first() {
        let mut queue = ConnectionQueue::new();

        // Add regular primary sensor
        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));

        // Add preferred primary sensor
        queue.enqueue_preferred(make_sensor("trainer2", "Wahoo", SensorType::Trainer));

        // Preferred should come first
        let first = queue.dequeue().unwrap();
        assert_eq!(first.device_id(), "trainer2");
        assert!(first.is_preferred);

        let second = queue.dequeue().unwrap();
        assert_eq!(second.device_id(), "trainer1");
    }

    #[test]
    fn test_queue_remove() {
        let mut queue = ConnectionQueue::new();

        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("hr1", "HR", SensorType::HeartRate));

        assert_eq!(queue.len(), 2);
        assert!(queue.contains("trainer1"));

        assert!(queue.remove("trainer1"));

        assert_eq!(queue.len(), 1);
        assert!(!queue.contains("trainer1"));
        assert!(queue.contains("hr1"));
    }

    #[test]
    fn test_queue_count_by_priority() {
        let mut queue = ConnectionQueue::new();

        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("power1", "Power", SensorType::PowerMeter));
        queue.enqueue(make_sensor("hr1", "HR", SensorType::HeartRate));
        queue.enqueue(make_sensor("cadence1", "Cadence", SensorType::Cadence));
        queue.enqueue(make_sensor("speed1", "Speed", SensorType::Speed));

        let (primary, secondary) = queue.count_by_priority();
        assert_eq!(primary, 2);
        assert_eq!(secondary, 3);
    }

    #[test]
    fn test_queue_enqueue_all() {
        let mut queue = ConnectionQueue::new();

        let sensors = vec![
            make_sensor("trainer1", "KICKR", SensorType::Trainer),
            make_sensor("hr1", "HR", SensorType::HeartRate),
            make_sensor("power1", "Power", SensorType::PowerMeter),
        ];

        queue.enqueue_all(sensors);

        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_queue_clear() {
        let mut queue = ConnectionQueue::new();

        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("hr1", "HR", SensorType::HeartRate));

        assert_eq!(queue.len(), 2);

        queue.clear();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_queue_iter_by_priority() {
        let mut queue = ConnectionQueue::new();

        queue.enqueue(make_sensor("hr1", "HR", SensorType::HeartRate));
        queue.enqueue(make_sensor("trainer1", "KICKR", SensorType::Trainer));
        queue.enqueue(make_sensor("cadence1", "Cadence", SensorType::Cadence));

        let ordered = queue.iter_by_priority();

        assert_eq!(ordered.len(), 3);
        // First should be primary (trainer)
        assert_eq!(ordered[0].priority, SensorPriority::Primary);
        // Rest should be secondary
        assert_eq!(ordered[1].priority, SensorPriority::Secondary);
        assert_eq!(ordered[2].priority, SensorPriority::Secondary);
    }
}
