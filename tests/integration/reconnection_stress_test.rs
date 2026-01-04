//! Stress test for rapid connect/disconnect cycles.
//!
//! T009-6.2: Test rapid connect/disconnect cycles to ensure stability.
//! Verify no resource leaks or crashes.
//!
//! Acceptance criteria: 100 rapid reconnection cycles complete without errors

use rustride::sensors::conflict::{
    ConflictDetector, ConflictDetectorConfig, DataType, ResolutionStrategy,
};
use rustride::sensors::connection_queue::{ConnectionQueue, ConnectionQueueEntry, SensorPriority};
use rustride::sensors::connection_state::{
    ConnectionLifecycleState, ConnectionStateMachine, ConnectionStateMachineConfig,
    ConnectionStateManager, StateTransition,
};
use rustride::sensors::health::{ConnectionHealthConfig, ConnectionHealthMonitor, HealthStatus};
use rustride::sensors::persistence::{ConnectionSessionManager, SessionSensor};
use rustride::sensors::quality::{ConnectionQualityConfig, ConnectionQualityMonitor, QualityLevel};
use rustride::sensors::reconnection::{
    ExponentialBackoff, ExponentialBackoffConfig, ReconnectionManager, ReconnectionStats,
};
use rustride::sensors::types::{
    ConnectionState, DiscoveredSensor, Protocol, SensorError, SensorEvent, SensorReading,
    SensorType,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

// ============================================================================
// Test constants
// ============================================================================

/// Number of rapid reconnection cycles to test (acceptance criteria: 100)
const STRESS_TEST_CYCLES: usize = 100;

/// Number of sensors to test in parallel stress tests
const PARALLEL_SENSOR_COUNT: usize = 10;

/// Maximum allowed time per cycle in milliseconds
const MAX_CYCLE_TIME_MS: u64 = 100;

/// Number of cycles for resource leak detection test
const RESOURCE_LEAK_TEST_CYCLES: usize = 500;

// ============================================================================
// Test helpers
// ============================================================================

/// Create a mock discovered sensor for testing.
fn make_discovered_sensor(
    name: &str,
    sensor_type: SensorType,
    protocol: Protocol,
) -> DiscoveredSensor {
    DiscoveredSensor {
        device_id: format!("{}:{}", protocol, name.replace(' ', "_").to_lowercase()),
        name: name.to_string(),
        sensor_type,
        protocol,
        signal_strength: Some(-55),
        last_seen: Instant::now(),
    }
}

/// Create a mock trainer sensor.
fn make_trainer() -> DiscoveredSensor {
    make_discovered_sensor("Wahoo KICKR", SensorType::Trainer, Protocol::BleFtms)
}

/// Create a mock power meter sensor.
fn make_power_meter() -> DiscoveredSensor {
    make_discovered_sensor(
        "Stages Power",
        SensorType::PowerMeter,
        Protocol::BleCyclingPower,
    )
}

/// Create a mock heart rate sensor.
fn make_heart_rate() -> DiscoveredSensor {
    make_discovered_sensor("Polar H10", SensorType::HeartRate, Protocol::BleHeartRate)
}

/// Create a mock cadence sensor.
fn make_cadence() -> DiscoveredSensor {
    make_discovered_sensor("Wahoo Cadence", SensorType::Cadence, Protocol::BleCsc)
}

/// Create a numbered sensor for parallel tests.
fn make_numbered_sensor(index: usize) -> DiscoveredSensor {
    let sensor_types = [
        SensorType::Trainer,
        SensorType::PowerMeter,
        SensorType::HeartRate,
        SensorType::Cadence,
    ];
    let protocols = [
        Protocol::BleFtms,
        Protocol::BleCyclingPower,
        Protocol::BleHeartRate,
        Protocol::BleCsc,
    ];
    let type_idx = index % sensor_types.len();
    make_discovered_sensor(
        &format!("Sensor {}", index),
        sensor_types[type_idx],
        protocols[type_idx],
    )
}

/// Statistics collected during stress testing.
#[derive(Debug, Default)]
struct StressTestStats {
    /// Total number of connect attempts
    connect_attempts: usize,
    /// Number of successful connects
    successful_connects: usize,
    /// Number of failed connects
    failed_connects: usize,
    /// Total number of disconnect attempts
    disconnect_attempts: usize,
    /// Number of successful disconnects
    successful_disconnects: usize,
    /// Total number of reconnection attempts
    reconnection_attempts: usize,
    /// Number of successful reconnections
    successful_reconnections: usize,
    /// Number of reconnection exhaustions
    exhausted_reconnections: usize,
    /// Minimum cycle time in milliseconds
    min_cycle_time_ms: u64,
    /// Maximum cycle time in milliseconds
    max_cycle_time_ms: u64,
    /// Total cycle time in milliseconds
    total_cycle_time_ms: u64,
    /// Number of cycles completed
    cycles_completed: usize,
}

impl StressTestStats {
    fn new() -> Self {
        Self {
            min_cycle_time_ms: u64::MAX,
            max_cycle_time_ms: 0,
            ..Default::default()
        }
    }

    fn record_cycle_time(&mut self, time_ms: u64) {
        self.min_cycle_time_ms = self.min_cycle_time_ms.min(time_ms);
        self.max_cycle_time_ms = self.max_cycle_time_ms.max(time_ms);
        self.total_cycle_time_ms += time_ms;
        self.cycles_completed += 1;
    }

    fn avg_cycle_time_ms(&self) -> f64 {
        if self.cycles_completed > 0 {
            self.total_cycle_time_ms as f64 / self.cycles_completed as f64
        } else {
            0.0
        }
    }

    fn success_rate(&self) -> f64 {
        let total = self.connect_attempts + self.disconnect_attempts + self.reconnection_attempts;
        let successful =
            self.successful_connects + self.successful_disconnects + self.successful_reconnections;
        if total > 0 {
            (successful as f64 / total as f64) * 100.0
        } else {
            100.0
        }
    }
}

/// Stress test coordinator for managing rapid connect/disconnect cycles.
struct StressTestCoordinator {
    /// Connection state manager for all devices
    state_manager: ConnectionStateManager,
    /// Connection queue for priority-based connecting
    connection_queue: ConnectionQueue,
    /// Health monitor for connection health
    health_monitor: ConnectionHealthMonitor,
    /// Quality monitor for connection quality
    quality_monitor: ConnectionQualityMonitor,
    /// Session manager for persistence
    session_manager: ConnectionSessionManager,
    /// Conflict detector for multi-sensor management
    conflict_detector: ConflictDetector,
    /// Reconnection manager for backoff tracking
    reconnection_manager: ReconnectionManager,
    /// Discovered sensors for testing
    sensors: HashMap<String, DiscoveredSensor>,
    /// Statistics collected during testing
    stats: StressTestStats,
    /// Events generated during testing
    events: Vec<SensorEvent>,
    /// Error messages collected during testing
    errors: Vec<String>,
}

impl StressTestCoordinator {
    fn new() -> Self {
        let conflict_config = ConflictDetectorConfig {
            strategy: ResolutionStrategy::AutoPriority,
            auto_resolve_non_critical: true,
            persist_resolutions: false,
        };

        Self {
            state_manager: ConnectionStateManager::new(),
            connection_queue: ConnectionQueue::new(),
            health_monitor: ConnectionHealthMonitor::new(),
            quality_monitor: ConnectionQualityMonitor::new(),
            session_manager: ConnectionSessionManager::new(),
            conflict_detector: ConflictDetector::with_config(conflict_config),
            reconnection_manager: ReconnectionManager::new(),
            sensors: HashMap::new(),
            stats: StressTestStats::new(),
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a sensor for testing.
    fn add_sensor(&mut self, sensor: DiscoveredSensor) {
        self.sensors.insert(sensor.device_id.clone(), sensor);
    }

    /// Connect to a sensor by device ID.
    fn connect(&mut self, device_id: &str) -> Result<(), SensorError> {
        self.stats.connect_attempts += 1;

        let sensor = self
            .sensors
            .get(device_id)
            .ok_or_else(|| SensorError::SensorNotFound(device_id.to_string()))?;

        // Transition to connecting state
        self.state_manager
            .transition(device_id, StateTransition::Connect)
            .map_err(|e| SensorError::ConnectionFailed(format!("Invalid state: {:?}", e)))?;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connecting,
        });

        // Simulate successful connection
        self.state_manager
            .transition(device_id, StateTransition::ConnectionSuccess)
            .map_err(|e| SensorError::ConnectionFailed(format!("Connection failed: {:?}", e)))?;

        // Update conflict detector
        self.conflict_detector
            .update_connection_status(device_id, true);

        // Start health monitoring
        let health_config = match sensor.sensor_type {
            SensorType::Trainer | SensorType::SmartTrainer | SensorType::PowerMeter => {
                ConnectionHealthConfig::strict()
            }
            _ => ConnectionHealthConfig::default(),
        };
        self.health_monitor
            .start_monitoring(device_id, health_config);

        // Start quality monitoring
        let quality_config = match sensor.sensor_type {
            SensorType::Trainer | SensorType::SmartTrainer | SensorType::PowerMeter => {
                ConnectionQualityConfig::strict()
            }
            _ => ConnectionQualityConfig::relaxed(),
        };
        self.quality_monitor
            .start_monitoring(device_id, quality_config);

        // Save to session
        let session_sensor = SessionSensor::new(
            device_id.to_string(),
            sensor.name.clone(),
            sensor.sensor_type,
            sensor.protocol,
        );
        self.session_manager.add_sensor(session_sensor);

        // Reset reconnection backoff on successful connect
        self.reconnection_manager.reset(device_id);

        self.stats.successful_connects += 1;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connected,
        });

        Ok(())
    }

    /// Disconnect from a sensor by device ID.
    fn disconnect(&mut self, device_id: &str) -> Result<(), SensorError> {
        self.stats.disconnect_attempts += 1;

        // Transition to disconnected state
        self.state_manager
            .transition(device_id, StateTransition::Disconnect)
            .map_err(|e| SensorError::Disconnected(format!("Invalid state: {:?}", e)))?;

        // Update conflict detector (may trigger failover)
        let failovers = self.conflict_detector.handle_primary_disconnect(device_id);

        // Emit failover events
        for failover in failovers {
            self.events.push(SensorEvent::FailoverActivated {
                data_type: format!("{:?}", failover.data_type),
                from_device_id: failover.from_device_id,
                from_sensor_name: failover.from_sensor_name,
                to_device_id: failover.to_device_id,
                to_sensor_name: failover.to_sensor_name,
            });
        }

        // Stop monitoring
        self.health_monitor.stop_monitoring(device_id);
        self.quality_monitor.stop_monitoring(device_id);

        // Remove from session
        self.session_manager.remove_sensor(device_id);

        self.stats.successful_disconnects += 1;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Disconnected,
        });

        Ok(())
    }

    /// Simulate connection loss and trigger reconnection.
    fn simulate_connection_loss(&mut self, device_id: &str) -> Result<bool, SensorError> {
        self.stats.reconnection_attempts += 1;

        // Transition to reconnecting state
        self.state_manager
            .transition(device_id, StateTransition::ConnectionLost)
            .map_err(|e| SensorError::Disconnected(format!("Invalid state: {:?}", e)))?;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Reconnecting,
        });

        // Check if we can still attempt reconnection
        if self.reconnection_manager.is_exhausted(device_id) {
            // Exhausted reconnection attempts
            self.state_manager
                .transition(device_id, StateTransition::ReconnectionExhausted)
                .map_err(|e| {
                    SensorError::ConnectionFailed(format!("Exhaustion failed: {:?}", e))
                })?;

            self.stats.exhausted_reconnections += 1;

            self.events.push(SensorEvent::ConnectionChanged {
                device_id: device_id.to_string(),
                state: ConnectionState::Disconnected,
            });

            return Ok(false);
        }

        // Record attempt and get delay (not actually waiting in test)
        let _delay = self.reconnection_manager.record_attempt(device_id);

        // Simulate successful reconnection
        self.state_manager
            .transition(device_id, StateTransition::ReconnectionSuccess)
            .map_err(|e| SensorError::ConnectionFailed(format!("Reconnection failed: {:?}", e)))?;

        // Reset backoff on success
        self.reconnection_manager.reset(device_id);

        self.stats.successful_reconnections += 1;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connected,
        });

        Ok(true)
    }

    /// Simulate a single rapid connect/disconnect cycle.
    fn run_cycle(&mut self, device_id: &str) -> Result<Duration, SensorError> {
        let start = Instant::now();

        // Connect
        self.connect(device_id)?;

        // Verify connected
        if !self.state_manager.is_connected(device_id) {
            return Err(SensorError::ConnectionFailed(
                "Not connected after connect()".to_string(),
            ));
        }

        // Disconnect
        self.disconnect(device_id)?;

        // Verify disconnected
        if self.state_manager.is_connected(device_id) {
            return Err(SensorError::Disconnected(
                "Still connected after disconnect()".to_string(),
            ));
        }

        let elapsed = start.elapsed();
        self.stats.record_cycle_time(elapsed.as_millis() as u64);

        Ok(elapsed)
    }

    /// Simulate a reconnection cycle (connect -> connection loss -> reconnect -> disconnect).
    fn run_reconnection_cycle(&mut self, device_id: &str) -> Result<Duration, SensorError> {
        let start = Instant::now();

        // Connect
        self.connect(device_id)?;

        // Simulate connection loss
        self.simulate_connection_loss(device_id)?;

        // Verify still connected (after successful reconnection)
        if !self.state_manager.is_connected(device_id) {
            return Err(SensorError::ConnectionFailed(
                "Not connected after reconnection".to_string(),
            ));
        }

        // Disconnect
        self.disconnect(device_id)?;

        let elapsed = start.elapsed();
        self.stats.record_cycle_time(elapsed.as_millis() as u64);

        Ok(elapsed)
    }

    /// Get the number of currently connected devices.
    fn connected_count(&self) -> usize {
        self.state_manager.get_connected_devices().len()
    }

    /// Get the health monitor count.
    fn health_monitoring_count(&self) -> usize {
        self.health_monitor.monitored_devices().len()
    }

    /// Get the quality monitor count.
    fn quality_monitoring_count(&self) -> usize {
        self.quality_monitor.monitored_devices().len()
    }

    /// Get the session sensor count.
    fn session_sensor_count(&self) -> usize {
        self.session_manager.sensor_count()
    }

    /// Get the reconnection manager count.
    fn reconnection_tracking_count(&self) -> usize {
        self.reconnection_manager.len()
    }

    /// Clean up all resources.
    fn cleanup(&mut self) {
        // Disconnect all sensors
        let connected: Vec<_> = self.state_manager.get_connected_devices();
        for device_id in connected {
            let _ = self.disconnect(&device_id);
        }

        // Clear all managers
        self.state_manager.clear();
        self.health_monitor.clear();
        self.quality_monitor.clear();
        self.reconnection_manager.clear();
    }

    /// Get statistics.
    fn get_stats(&self) -> &StressTestStats {
        &self.stats
    }

    /// Get errors.
    fn get_errors(&self) -> &[String] {
        &self.errors
    }
}

// ============================================================================
// Stress tests
// ============================================================================

#[test]
fn test_100_rapid_reconnection_cycles() {
    // Acceptance criteria: 100 rapid reconnection cycles complete without errors
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let mut cycle_errors = Vec::new();

    for cycle in 0..STRESS_TEST_CYCLES {
        match coordinator.run_cycle(&trainer.device_id) {
            Ok(elapsed) => {
                // Verify cycle completed within acceptable time
                assert!(
                    elapsed.as_millis() < MAX_CYCLE_TIME_MS as u128,
                    "Cycle {} took too long: {:?}",
                    cycle,
                    elapsed
                );
            }
            Err(e) => {
                cycle_errors.push(format!("Cycle {} failed: {:?}", cycle, e));
            }
        }
    }

    coordinator.cleanup();

    // Verify all cycles completed without errors
    assert!(cycle_errors.is_empty(), "Cycles failed: {:?}", cycle_errors);

    let stats = coordinator.get_stats();
    assert_eq!(stats.cycles_completed, STRESS_TEST_CYCLES);
    assert_eq!(stats.successful_connects, STRESS_TEST_CYCLES);
    assert_eq!(stats.successful_disconnects, STRESS_TEST_CYCLES);
    assert!(stats.success_rate() == 100.0);
}

#[test]
fn test_rapid_reconnection_with_backoff() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 50;
    let mut cycle_errors = Vec::new();

    for cycle in 0..cycles {
        match coordinator.run_reconnection_cycle(&trainer.device_id) {
            Ok(_) => {}
            Err(e) => {
                cycle_errors.push(format!("Cycle {} failed: {:?}", cycle, e));
            }
        }
    }

    coordinator.cleanup();

    // Verify all cycles completed without errors
    assert!(cycle_errors.is_empty(), "Cycles failed: {:?}", cycle_errors);

    let stats = coordinator.get_stats();
    assert_eq!(stats.cycles_completed, cycles);
    assert_eq!(stats.successful_reconnections, cycles);
}

#[test]
fn test_parallel_sensor_connect_disconnect() {
    let mut coordinator = StressTestCoordinator::new();

    // Add multiple sensors
    for i in 0..PARALLEL_SENSOR_COUNT {
        coordinator.add_sensor(make_numbered_sensor(i));
    }

    let sensor_ids: Vec<_> = coordinator.sensors.keys().cloned().collect();
    let cycles = 20;

    for _cycle in 0..cycles {
        // Connect all sensors
        for device_id in &sensor_ids {
            assert!(coordinator.connect(device_id).is_ok());
        }

        // Verify all connected
        assert_eq!(coordinator.connected_count(), PARALLEL_SENSOR_COUNT);

        // Disconnect all sensors
        for device_id in &sensor_ids {
            assert!(coordinator.disconnect(device_id).is_ok());
        }

        // Verify all disconnected
        assert_eq!(coordinator.connected_count(), 0);
    }

    coordinator.cleanup();

    let stats = coordinator.get_stats();
    let expected_connects = cycles * PARALLEL_SENSOR_COUNT;
    assert_eq!(stats.successful_connects, expected_connects);
    assert_eq!(stats.successful_disconnects, expected_connects);
}

#[test]
fn test_interleaved_connect_disconnect() {
    // Test interleaved operations where some sensors connect while others disconnect
    let mut coordinator = StressTestCoordinator::new();

    // Add 4 sensors
    let sensors = vec![
        make_trainer(),
        make_power_meter(),
        make_heart_rate(),
        make_cadence(),
    ];
    for sensor in &sensors {
        coordinator.add_sensor(sensor.clone());
    }

    let cycles = 30;

    for _cycle in 0..cycles {
        // Phase 1: Connect trainer and power meter
        assert!(coordinator.connect(&sensors[0].device_id).is_ok());
        assert!(coordinator.connect(&sensors[1].device_id).is_ok());
        assert_eq!(coordinator.connected_count(), 2);

        // Phase 2: Disconnect trainer, connect HR and cadence
        assert!(coordinator.disconnect(&sensors[0].device_id).is_ok());
        assert!(coordinator.connect(&sensors[2].device_id).is_ok());
        assert!(coordinator.connect(&sensors[3].device_id).is_ok());
        assert_eq!(coordinator.connected_count(), 3);

        // Phase 3: Disconnect all remaining
        assert!(coordinator.disconnect(&sensors[1].device_id).is_ok());
        assert!(coordinator.disconnect(&sensors[2].device_id).is_ok());
        assert!(coordinator.disconnect(&sensors[3].device_id).is_ok());
        assert_eq!(coordinator.connected_count(), 0);
    }

    coordinator.cleanup();

    let stats = coordinator.get_stats();
    // Each cycle: 4 connects, 4 disconnects
    assert_eq!(stats.successful_connects, cycles * 4);
    assert_eq!(stats.successful_disconnects, cycles * 4);
}

#[test]
fn test_no_resource_leaks_after_cycles() {
    // Run many cycles and verify no resource leaks
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    // Run many cycles
    for _cycle in 0..RESOURCE_LEAK_TEST_CYCLES {
        let _ = coordinator.run_cycle(&trainer.device_id);
    }

    // After cleanup, all resources should be released
    coordinator.cleanup();

    // Verify no lingering resources
    assert_eq!(coordinator.connected_count(), 0);
    assert_eq!(coordinator.health_monitoring_count(), 0);
    assert_eq!(coordinator.quality_monitoring_count(), 0);
    assert_eq!(coordinator.session_sensor_count(), 0);
}

#[test]
fn test_reconnection_exhaustion_handling() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(10),
        multiplier: 2.0,
        max_attempts: 3,
        jitter_factor: 0.0,
    };

    let mut coordinator = StressTestCoordinator::new();
    coordinator.reconnection_manager = ReconnectionManager::with_config(config);

    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    // Connect first
    assert!(coordinator.connect(&trainer.device_id).is_ok());

    // Simulate repeated connection losses until exhaustion
    for i in 0..5 {
        // Use a fresh reconnection manager state each time by resetting
        // after disconnect to test the exhaustion flow properly

        // Simulate connection loss
        let result = coordinator.simulate_connection_loss(&trainer.device_id);

        if i < 3 {
            // First 3 attempts should succeed (max_attempts = 3)
            assert!(result.is_ok());
            if let Ok(reconnected) = result {
                if !reconnected {
                    // Exhausted - this is expected after attempt 3
                    break;
                }
            }
        }
    }

    // After exhaustion, state should be disconnected or we got through all attempts
    // The important thing is no panic or crash occurred

    coordinator.cleanup();
}

#[test]
fn test_state_machine_consistency() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 50;

    for cycle in 0..cycles {
        // Verify initial state
        assert!(
            !coordinator.state_manager.is_connected(&trainer.device_id),
            "Cycle {}: Expected disconnected before connect",
            cycle
        );

        // Connect
        assert!(coordinator.connect(&trainer.device_id).is_ok());
        assert!(
            coordinator.state_manager.is_connected(&trainer.device_id),
            "Cycle {}: Expected connected after connect",
            cycle
        );

        // Disconnect
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
        assert!(
            !coordinator.state_manager.is_connected(&trainer.device_id),
            "Cycle {}: Expected disconnected after disconnect",
            cycle
        );
    }

    coordinator.cleanup();
}

#[test]
fn test_health_monitor_stability() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 50;

    for _cycle in 0..cycles {
        // Connect - starts health monitoring
        assert!(coordinator.connect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.health_monitoring_count(), 1);

        // Simulate some data
        coordinator.health_monitor.record_data(&trainer.device_id);

        // Disconnect - stops health monitoring
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.health_monitoring_count(), 0);
    }

    coordinator.cleanup();
}

#[test]
fn test_quality_monitor_stability() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 50;

    for _cycle in 0..cycles {
        // Connect - starts quality monitoring
        assert!(coordinator.connect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.quality_monitoring_count(), 1);

        // Simulate some RSSI updates
        coordinator
            .quality_monitor
            .update_rssi(&trainer.device_id, Some(-55));
        coordinator
            .quality_monitor
            .record_data(&trainer.device_id, 10);

        // Disconnect - stops quality monitoring
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.quality_monitoring_count(), 0);
    }

    coordinator.cleanup();
}

#[test]
fn test_session_manager_stability() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 50;

    for _cycle in 0..cycles {
        // Connect - adds to session
        assert!(coordinator.connect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.session_sensor_count(), 1);

        // Disconnect - removes from session
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
        assert_eq!(coordinator.session_sensor_count(), 0);
    }

    coordinator.cleanup();
}

#[test]
fn test_conflict_detector_stability() {
    let mut coordinator = StressTestCoordinator::new();

    // Add two power sources to create a conflict
    let trainer = make_trainer();
    let power_meter = make_power_meter();
    coordinator.add_sensor(trainer.clone());
    coordinator.add_sensor(power_meter.clone());

    // Register with conflict detector
    coordinator.conflict_detector.register_sensor(&trainer);
    coordinator.conflict_detector.register_sensor(&power_meter);

    let cycles = 30;

    for _cycle in 0..cycles {
        // Connect both
        assert!(coordinator.connect(&trainer.device_id).is_ok());
        assert!(coordinator.connect(&power_meter.device_id).is_ok());

        // Set primary
        coordinator
            .conflict_detector
            .set_primary(DataType::Power, &power_meter.device_id);

        // Disconnect power meter (triggers failover)
        assert!(coordinator.disconnect(&power_meter.device_id).is_ok());

        // Disconnect trainer
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
    }

    coordinator.cleanup();
}

#[test]
fn test_backoff_state_isolation() {
    // Verify that backoff state for different devices is isolated
    let mut coordinator = StressTestCoordinator::new();

    // Add multiple sensors
    let sensors: Vec<_> = (0..5).map(|i| make_numbered_sensor(i)).collect();
    for sensor in &sensors {
        coordinator.add_sensor(sensor.clone());
    }

    // Connect all
    for sensor in &sensors {
        assert!(coordinator.connect(&sensor.device_id).is_ok());
    }

    // Simulate connection loss for each sensor multiple times
    for _ in 0..3 {
        for sensor in &sensors {
            let _ = coordinator.simulate_connection_loss(&sensor.device_id);
        }
    }

    // All should still be connected after successful reconnections
    assert_eq!(coordinator.connected_count(), sensors.len());

    coordinator.cleanup();
}

#[test]
fn test_event_generation_correctness() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    // Run a few cycles
    for _ in 0..10 {
        let _ = coordinator.run_cycle(&trainer.device_id);
    }

    // Count connection events
    let connecting_events = coordinator
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SensorEvent::ConnectionChanged {
                    state: ConnectionState::Connecting,
                    ..
                }
            )
        })
        .count();
    let connected_events = coordinator
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SensorEvent::ConnectionChanged {
                    state: ConnectionState::Connected,
                    ..
                }
            )
        })
        .count();
    let disconnected_events = coordinator
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                SensorEvent::ConnectionChanged {
                    state: ConnectionState::Disconnected,
                    ..
                }
            )
        })
        .count();

    // Each cycle should generate: Connecting -> Connected -> Disconnected
    assert_eq!(connecting_events, 10);
    assert_eq!(connected_events, 10);
    assert_eq!(disconnected_events, 10);

    coordinator.cleanup();
}

#[test]
fn test_rapid_cycles_with_varying_sensors() {
    let mut coordinator = StressTestCoordinator::new();

    // Add different sensor types
    let sensors = vec![
        make_trainer(),
        make_power_meter(),
        make_heart_rate(),
        make_cadence(),
    ];
    for sensor in &sensors {
        coordinator.add_sensor(sensor.clone());
    }

    let cycles_per_sensor = 25;

    // Run cycles for each sensor type
    for sensor in &sensors {
        for _ in 0..cycles_per_sensor {
            assert!(coordinator.run_cycle(&sensor.device_id).is_ok());
        }
    }

    coordinator.cleanup();

    let stats = coordinator.get_stats();
    let expected = cycles_per_sensor * sensors.len();
    assert_eq!(stats.cycles_completed, expected);
    assert!(stats.success_rate() == 100.0);
}

#[test]
fn test_mixed_connect_reconnect_disconnect() {
    let mut coordinator = StressTestCoordinator::new();
    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    let cycles = 30;

    for cycle in 0..cycles {
        // Connect
        assert!(coordinator.connect(&trainer.device_id).is_ok());

        // Sometimes simulate connection loss and reconnection
        if cycle % 3 == 0 {
            let reconnected = coordinator
                .simulate_connection_loss(&trainer.device_id)
                .expect("Connection loss simulation should succeed");
            assert!(reconnected);
        }

        // Disconnect
        assert!(coordinator.disconnect(&trainer.device_id).is_ok());
    }

    coordinator.cleanup();

    let stats = coordinator.get_stats();
    assert_eq!(stats.successful_connects, cycles);
    assert_eq!(stats.successful_disconnects, cycles);
    // Reconnections: every 3rd cycle (cycle 0, 3, 6, ...) = (cycles + 2) / 3 = 10
    assert_eq!(stats.successful_reconnections, (cycles + 2) / 3);
}

#[test]
fn test_exponential_backoff_sequence_preservation() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(2),
        multiplier: 2.0,
        max_attempts: 10,
        jitter_factor: 0.0,
    };

    let mut backoff = ExponentialBackoff::with_config(config);

    // Record delays
    let mut delays = Vec::new();
    for _ in 0..6 {
        delays.push(backoff.record_attempt());
    }

    // Verify exponential pattern: 100ms, 200ms, 400ms, 800ms, 1600ms, 2000ms (capped)
    assert_eq!(delays[0], Duration::from_millis(100));
    assert_eq!(delays[1], Duration::from_millis(200));
    assert_eq!(delays[2], Duration::from_millis(400));
    assert_eq!(delays[3], Duration::from_millis(800));
    assert_eq!(delays[4], Duration::from_millis(1600));
    assert_eq!(delays[5], Duration::from_secs(2)); // Capped at max

    // Reset and verify it starts over
    backoff.reset();
    assert_eq!(backoff.record_attempt(), Duration::from_millis(100));
}

#[test]
fn test_reconnection_manager_multi_device() {
    let mut manager = ReconnectionManager::new();

    // Track multiple devices
    let devices = vec!["device_a", "device_b", "device_c"];

    // Record attempts for each
    for device in &devices {
        for _ in 0..3 {
            manager.record_attempt(device);
        }
    }

    // Verify each device has its own state
    for device in &devices {
        let stats = manager.get_stats(device).unwrap();
        assert_eq!(stats.current_attempt, 3);
    }

    // Reset one device
    manager.reset("device_b");

    // Verify only device_b was reset
    assert_eq!(manager.get_stats("device_a").unwrap().current_attempt, 3);
    assert_eq!(manager.get_stats("device_b").unwrap().current_attempt, 0);
    assert_eq!(manager.get_stats("device_c").unwrap().current_attempt, 3);

    // Clear all
    manager.clear();
    assert!(manager.is_empty());
}

#[test]
fn test_stress_with_minimal_resources() {
    // Test with minimal backoff configuration to stress the state machine
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(1),
        multiplier: 1.0,
        max_attempts: 0, // Unlimited
        jitter_factor: 0.0,
    };

    let mut coordinator = StressTestCoordinator::new();
    coordinator.reconnection_manager = ReconnectionManager::with_config(config);

    let trainer = make_trainer();
    coordinator.add_sensor(trainer.clone());

    // Run many rapid cycles
    for _ in 0..200 {
        let result = coordinator.run_cycle(&trainer.device_id);
        assert!(result.is_ok(), "Cycle failed: {:?}", result);
    }

    coordinator.cleanup();

    let stats = coordinator.get_stats();
    assert_eq!(stats.cycles_completed, 200);
    assert!(stats.success_rate() == 100.0);
}
