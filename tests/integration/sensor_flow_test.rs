//! Integration test for full sensor discovery flow.
//!
//! T009-6.1: End-to-end test covering discovery, connection, data reception, and graceful disconnection.
//!
//! This test validates the complete sensor lifecycle without requiring actual hardware.
//! It uses mock sensor infrastructure to simulate realistic sensor interactions.

use rustride::sensors::cache::{CachedSensor, SensorCache};
use rustride::sensors::conflict::{
    ConflictDetector, ConflictDetectorConfig, DataType, ResolutionStrategy,
};
use rustride::sensors::connection_queue::{ConnectionQueue, ConnectionQueueEntry, SensorPriority};
use rustride::sensors::connection_state::{
    ConnectionLifecycleState, ConnectionStateMachine, ConnectionStateMachineConfig,
    ConnectionStateManager, StateTransition,
};
use rustride::sensors::ftms::{
    parse_cycling_power_measurement, parse_heart_rate_measurement, parse_indoor_bike_data,
};
use rustride::sensors::health::{ConnectionHealthConfig, ConnectionHealthMonitor, HealthStatus};
use rustride::sensors::persistence::{ConnectionSessionManager, SessionSensor};
use rustride::sensors::quality::{ConnectionQualityConfig, ConnectionQualityMonitor, QualityLevel};
use rustride::sensors::reconnection::{ExponentialBackoff, ExponentialBackoffConfig};
use rustride::sensors::types::{
    ConnectionState, DiscoveredSensor, DiscoveryPhase, DiscoveryProgress, ParallelDiscoveryResult,
    ProgressiveTimeoutConfig, ProgressiveTimeoutState, Protocol, SensorConfig, SensorError,
    SensorEvent, SensorReading, SensorState, SensorType, StopReason, TimeoutDecision,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

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

/// Simulate sensor data packet generation.
fn generate_mock_power_data(power_watts: u16) -> Vec<u8> {
    // CPS format: flags (2 bytes) + power (2 bytes)
    let mut data = vec![0x00, 0x00]; // No optional fields
    data.extend_from_slice(&(power_watts as i16).to_le_bytes());
    data
}

fn generate_mock_hr_data(heart_rate: u8) -> Vec<u8> {
    // HR format: flags (1 byte) + HR (1 byte for 8-bit format)
    vec![0x00, heart_rate]
}

fn generate_mock_ftms_data(power: u16, cadence: u16, speed: u16) -> Vec<u8> {
    // FTMS Indoor Bike Data format
    let mut data = vec![0x44, 0x00]; // Flags for cadence + power
    data.extend_from_slice(&speed.to_le_bytes());
    data.extend_from_slice(&cadence.to_le_bytes());
    data.extend_from_slice(&(power as i16).to_le_bytes());
    data
}

/// Sensor flow coordinator for integration testing.
/// Manages the complete lifecycle of sensor discovery, connection, data, and disconnection.
struct SensorFlowCoordinator {
    /// Discovered sensors during scan
    discovered: HashMap<String, DiscoveredSensor>,
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
    /// Progressive timeout state
    timeout_state: Option<ProgressiveTimeoutState>,
    /// Received data readings
    readings: Vec<SensorReading>,
    /// Events generated during flow
    events: Vec<SensorEvent>,
    /// Reconnection backoff configs
    backoff: HashMap<String, ExponentialBackoff>,
}

impl SensorFlowCoordinator {
    fn new() -> Self {
        let conflict_config = ConflictDetectorConfig {
            strategy: ResolutionStrategy::AutoPriority,
            auto_resolve_non_critical: true,
            persist_resolutions: false,
        };

        Self {
            discovered: HashMap::new(),
            state_manager: ConnectionStateManager::new(),
            connection_queue: ConnectionQueue::new(),
            health_monitor: ConnectionHealthMonitor::new(),
            quality_monitor: ConnectionQualityMonitor::new(),
            session_manager: ConnectionSessionManager::new(),
            conflict_detector: ConflictDetector::with_config(conflict_config),
            timeout_state: None,
            readings: Vec::new(),
            events: Vec::new(),
            backoff: HashMap::new(),
        }
    }

    /// Start discovery phase.
    fn start_discovery(&mut self) {
        self.timeout_state = Some(ProgressiveTimeoutState::new());
        self.events.push(SensorEvent::ScanStarted);
    }

    /// Simulate discovering a sensor.
    fn discover_sensor(&mut self, sensor: DiscoveredSensor) {
        // Record discovery in timeout state
        if let Some(ref mut state) = self.timeout_state {
            state.record_discovery();
        }

        let device_id = sensor.device_id.clone();

        // Add to discovered list
        self.discovered.insert(device_id.clone(), sensor.clone());

        // Queue for connection
        let entry = ConnectionQueueEntry::new(sensor.clone())
            .with_priority(SensorPriority::from_sensor_type(sensor.sensor_type));
        self.connection_queue.push(entry);

        // Register with conflict detector
        self.conflict_detector.register_sensor(&sensor);

        // Emit discovery event
        self.events.push(SensorEvent::Discovered(sensor));
    }

    /// Stop discovery phase.
    fn stop_discovery(&mut self) -> DiscoveryProgress {
        let progress = if let Some(ref mut state) = self.timeout_state {
            state.mark_completed();
            DiscoveryProgress {
                phase: state.phase,
                elapsed: state.elapsed(),
                sensors_discovered: state.sensors_discovered,
                extensions_count: state.extensions_count,
                is_active: false,
            }
        } else {
            DiscoveryProgress {
                phase: DiscoveryPhase::Completed,
                elapsed: Duration::from_secs(0),
                sensors_discovered: 0,
                extensions_count: 0,
                is_active: false,
            }
        };

        self.events.push(SensorEvent::ScanStopped);
        progress
    }

    /// Connect to a sensor by device ID.
    fn connect(&mut self, device_id: &str) -> Result<(), SensorError> {
        let sensor = self
            .discovered
            .get(device_id)
            .ok_or_else(|| SensorError::SensorNotFound(device_id.to_string()))?;

        // Transition to connecting state
        self.state_manager
            .transition(device_id, StateTransition::Connect)
            .map_err(|_| SensorError::ConnectionFailed("Invalid state transition".to_string()))?;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connecting,
        });

        // Simulate successful connection
        self.state_manager
            .transition(device_id, StateTransition::ConnectionSuccess)
            .map_err(|_| SensorError::ConnectionFailed("Connection failed".to_string()))?;

        // Update conflict detector
        self.conflict_detector
            .update_connection_status(device_id, true);

        // Start health monitoring with appropriate config
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

        // Create backoff config for reconnection
        self.backoff.insert(
            device_id.to_string(),
            ExponentialBackoff::new(ExponentialBackoffConfig::default()),
        );

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Connected,
        });

        Ok(())
    }

    /// Connect all sensors in the queue by priority.
    fn connect_all_by_priority(&mut self) -> Vec<Result<String, SensorError>> {
        let mut results = Vec::new();

        while let Some(entry) = self.connection_queue.pop() {
            let device_id = entry.sensor.device_id.clone();
            match self.connect(&device_id) {
                Ok(()) => results.push(Ok(device_id)),
                Err(e) => results.push(Err(e)),
            }
        }

        results
    }

    /// Simulate receiving data from a sensor.
    fn receive_data(&mut self, device_id: &str, reading: SensorReading) -> Result<(), SensorError> {
        // Verify connected
        if !self.state_manager.is_connected(device_id) {
            return Err(SensorError::Disconnected(device_id.to_string()));
        }

        // Update health monitor
        self.health_monitor.record_data(device_id);

        // Update quality monitor with mock RSSI
        self.quality_monitor.update_rssi(device_id, Some(-55));
        self.quality_monitor.record_data(device_id, 8); // 8 bytes typical

        // Store reading
        self.readings.push(reading.clone());

        // Emit data event
        self.events.push(SensorEvent::Data(reading));

        Ok(())
    }

    /// Gracefully disconnect a sensor.
    fn disconnect(&mut self, device_id: &str) -> Result<(), SensorError> {
        // Transition to disconnected state
        self.state_manager
            .transition(device_id, StateTransition::Disconnect)
            .map_err(|_| SensorError::Disconnected(device_id.to_string()))?;

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

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Disconnected,
        });

        Ok(())
    }

    /// Simulate connection loss and automatic reconnection.
    fn simulate_connection_loss(&mut self, device_id: &str) -> Result<bool, SensorError> {
        // Transition to reconnecting state
        self.state_manager
            .transition(device_id, StateTransition::ConnectionLost)
            .map_err(|_| SensorError::Disconnected(device_id.to_string()))?;

        self.events.push(SensorEvent::ConnectionChanged {
            device_id: device_id.to_string(),
            state: ConnectionState::Reconnecting,
        });

        // Get backoff delay
        let should_retry = if let Some(backoff) = self.backoff.get_mut(device_id) {
            backoff.next_delay().is_some()
        } else {
            false
        };

        if should_retry {
            // Simulate successful reconnection
            self.state_manager
                .transition(device_id, StateTransition::ReconnectionSuccess)
                .map_err(|_| SensorError::ConnectionFailed("Reconnection failed".to_string()))?;

            // Reset backoff on success
            if let Some(backoff) = self.backoff.get_mut(device_id) {
                backoff.reset();
            }

            self.events.push(SensorEvent::ConnectionChanged {
                device_id: device_id.to_string(),
                state: ConnectionState::Connected,
            });

            Ok(true)
        } else {
            // Exhausted reconnection attempts
            self.state_manager
                .transition(device_id, StateTransition::ReconnectionExhausted)
                .map_err(|_| SensorError::ConnectionFailed("Reconnection exhausted".to_string()))?;

            self.events.push(SensorEvent::ConnectionChanged {
                device_id: device_id.to_string(),
                state: ConnectionState::Disconnected,
            });

            Ok(false)
        }
    }

    /// Get health status for a sensor.
    fn get_health_status(&self, device_id: &str) -> Option<HealthStatus> {
        self.health_monitor.get_status(device_id)
    }

    /// Get quality level for a sensor.
    fn get_quality_level(&self, device_id: &str) -> Option<QualityLevel> {
        self.quality_monitor.get_quality_level(device_id)
    }

    /// Check if all sensors are connected.
    fn all_connected(&self) -> bool {
        self.discovered
            .keys()
            .all(|id| self.state_manager.is_connected(id))
    }

    /// Get connected sensor count.
    fn connected_count(&self) -> usize {
        self.state_manager.get_connected_devices().len()
    }

    /// Get total readings received.
    fn readings_count(&self) -> usize {
        self.readings.len()
    }

    /// Get all events for verification.
    fn get_events(&self) -> &[SensorEvent] {
        &self.events
    }

    /// Clean up all connections.
    fn shutdown(&mut self) {
        let connected_devices: Vec<_> = self.state_manager.get_connected_devices();
        for device_id in connected_devices {
            let _ = self.disconnect(&device_id);
        }
        self.state_manager.clear();
        self.health_monitor.clear();
        self.quality_monitor.clear();
    }
}

// ============================================================================
// Integration tests: Full sensor flow
// ============================================================================

#[test]
fn test_full_sensor_discovery_flow() {
    let mut coordinator = SensorFlowCoordinator::new();

    // Phase 1: Start discovery
    coordinator.start_discovery();
    assert!(coordinator
        .get_events()
        .iter()
        .any(|e| matches!(e, SensorEvent::ScanStarted)));

    // Phase 2: Discover sensors
    let trainer = make_trainer();
    let power_meter = make_power_meter();
    let heart_rate = make_heart_rate();
    let cadence = make_cadence();

    coordinator.discover_sensor(trainer.clone());
    coordinator.discover_sensor(power_meter.clone());
    coordinator.discover_sensor(heart_rate.clone());
    coordinator.discover_sensor(cadence.clone());

    // Verify all sensors discovered
    assert_eq!(coordinator.discovered.len(), 4);

    // Phase 3: Stop discovery
    let progress = coordinator.stop_discovery();
    assert_eq!(progress.sensors_discovered, 4);
    assert_eq!(progress.phase, DiscoveryPhase::Completed);

    // Verify discovery events
    let discovery_events: Vec<_> = coordinator
        .get_events()
        .iter()
        .filter(|e| matches!(e, SensorEvent::Discovered(_)))
        .collect();
    assert_eq!(discovery_events.len(), 4);
}

#[test]
fn test_priority_based_connection() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    // Discover in random order
    coordinator.discover_sensor(make_cadence());
    coordinator.discover_sensor(make_trainer());
    coordinator.discover_sensor(make_heart_rate());
    coordinator.discover_sensor(make_power_meter());

    coordinator.stop_discovery();

    // Connect all by priority
    let results = coordinator.connect_all_by_priority();

    // All should succeed
    assert!(results.iter().all(|r| r.is_ok()));

    // Verify connection order (primary sensors first: trainer, power meter)
    let connected_order: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .cloned()
        .collect();

    // First two should be primary sensors (trainer or power meter)
    let is_primary = |id: &str| id.contains("wahoo_kickr") || id.contains("stages_power");
    assert!(is_primary(&connected_order[0]));
    assert!(is_primary(&connected_order[1]));

    // All connected
    assert_eq!(coordinator.connected_count(), 4);
}

#[test]
fn test_data_reception_flow() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    let trainer = make_trainer();
    let hr = make_heart_rate();

    coordinator.discover_sensor(trainer.clone());
    coordinator.discover_sensor(hr.clone());
    coordinator.stop_discovery();

    // Connect sensors
    coordinator.connect(&trainer.device_id).unwrap();
    coordinator.connect(&hr.device_id).unwrap();

    // Simulate receiving data
    let sensor_id = Uuid::new_v4();

    // Receive 10 power readings
    for i in 0..10 {
        let reading = SensorReading {
            sensor_id,
            timestamp: Instant::now(),
            power_watts: Some(200 + i * 5),
            cadence_rpm: Some(90),
            heart_rate_bpm: None,
            speed_kmh: Some(30.0),
            distance_delta_m: Some(8.3),
        };
        coordinator
            .receive_data(&trainer.device_id, reading)
            .unwrap();
    }

    // Receive 10 HR readings
    for _ in 0..10 {
        let reading = SensorReading {
            sensor_id,
            timestamp: Instant::now(),
            power_watts: None,
            cadence_rpm: None,
            heart_rate_bpm: Some(145),
            speed_kmh: None,
            distance_delta_m: None,
        };
        coordinator.receive_data(&hr.device_id, reading).unwrap();
    }

    // Verify readings
    assert_eq!(coordinator.readings_count(), 20);

    // Verify health is good after receiving data
    assert_eq!(
        coordinator.get_health_status(&trainer.device_id),
        Some(HealthStatus::Healthy)
    );
    assert_eq!(
        coordinator.get_health_status(&hr.device_id),
        Some(HealthStatus::Healthy)
    );
}

#[test]
fn test_graceful_disconnection() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    let trainer = make_trainer();
    let power_meter = make_power_meter();

    coordinator.discover_sensor(trainer.clone());
    coordinator.discover_sensor(power_meter.clone());
    coordinator.stop_discovery();

    // Connect both
    coordinator.connect(&trainer.device_id).unwrap();
    coordinator.connect(&power_meter.device_id).unwrap();
    assert_eq!(coordinator.connected_count(), 2);

    // Disconnect trainer gracefully
    coordinator.disconnect(&trainer.device_id).unwrap();
    assert_eq!(coordinator.connected_count(), 1);
    assert!(!coordinator.state_manager.is_connected(&trainer.device_id));
    assert!(coordinator
        .state_manager
        .is_connected(&power_meter.device_id));

    // Verify disconnect event
    let disconnect_events: Vec<_> = coordinator
        .get_events()
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
        .collect();
    assert!(!disconnect_events.is_empty());

    // Disconnect remaining sensor
    coordinator.disconnect(&power_meter.device_id).unwrap();
    assert_eq!(coordinator.connected_count(), 0);
}

#[test]
fn test_connection_loss_and_reconnection() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    let trainer = make_trainer();
    coordinator.discover_sensor(trainer.clone());
    coordinator.stop_discovery();

    // Connect
    coordinator.connect(&trainer.device_id).unwrap();
    assert!(coordinator.state_manager.is_connected(&trainer.device_id));

    // Simulate connection loss
    let reconnected = coordinator
        .simulate_connection_loss(&trainer.device_id)
        .unwrap();

    // Should have reconnected
    assert!(reconnected);
    assert!(coordinator.state_manager.is_connected(&trainer.device_id));

    // Verify reconnection event sequence
    let events = coordinator.get_events();
    let has_reconnecting = events.iter().any(|e| {
        matches!(
            e,
            SensorEvent::ConnectionChanged {
                state: ConnectionState::Reconnecting,
                ..
            }
        )
    });
    assert!(has_reconnecting);
}

#[test]
fn test_failover_on_primary_disconnect() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    let trainer = make_trainer();
    let power_meter = make_power_meter();

    coordinator.discover_sensor(power_meter.clone());
    coordinator.discover_sensor(trainer.clone());
    coordinator.stop_discovery();

    // Connect both
    coordinator.connect(&power_meter.device_id).unwrap();
    coordinator.connect(&trainer.device_id).unwrap();

    // Set power meter as primary for power
    coordinator
        .conflict_detector
        .set_primary(DataType::Power, &power_meter.device_id);

    // Disconnect primary (power meter)
    coordinator.disconnect(&power_meter.device_id).unwrap();

    // Verify failover event was generated
    let failover_events: Vec<_> = coordinator
        .get_events()
        .iter()
        .filter(|e| matches!(e, SensorEvent::FailoverActivated { .. }))
        .collect();

    // Should have failover to trainer for power
    assert!(!failover_events.is_empty());
}

#[test]
fn test_complete_workout_session() {
    let mut coordinator = SensorFlowCoordinator::new();

    // 1. Discovery phase
    coordinator.start_discovery();
    coordinator.discover_sensor(make_trainer());
    coordinator.discover_sensor(make_heart_rate());
    let progress = coordinator.stop_discovery();
    assert_eq!(progress.sensors_discovered, 2);

    // 2. Connection phase
    let results = coordinator.connect_all_by_priority();
    assert!(results.iter().all(|r| r.is_ok()));
    assert!(coordinator.all_connected());

    // 3. Workout phase - simulate 60 seconds of data
    let sensor_id = Uuid::new_v4();
    let trainer_id = "BleFtms:wahoo_kickr".to_string();
    let hr_id = "BleHeartRate:polar_h10".to_string();

    for second in 0..60 {
        // Power data every second
        let power_reading = SensorReading {
            sensor_id,
            timestamp: Instant::now(),
            power_watts: Some(200 + (second % 50) as u16),
            cadence_rpm: Some(90),
            heart_rate_bpm: None,
            speed_kmh: Some(30.0),
            distance_delta_m: Some(8.33),
        };
        coordinator
            .receive_data(&trainer_id, power_reading)
            .unwrap();

        // HR data every second
        let hr_reading = SensorReading {
            sensor_id,
            timestamp: Instant::now(),
            power_watts: None,
            cadence_rpm: None,
            heart_rate_bpm: Some(140 + (second % 20) as u8),
            speed_kmh: None,
            distance_delta_m: None,
        };
        coordinator.receive_data(&hr_id, hr_reading).unwrap();
    }

    // Verify data was received
    assert_eq!(coordinator.readings_count(), 120); // 60 power + 60 HR

    // 4. Verify health is good
    assert_eq!(
        coordinator.get_health_status(&trainer_id),
        Some(HealthStatus::Healthy)
    );
    assert_eq!(
        coordinator.get_health_status(&hr_id),
        Some(HealthStatus::Healthy)
    );

    // 5. End workout - graceful shutdown
    coordinator.shutdown();
    assert_eq!(coordinator.connected_count(), 0);
}

#[test]
fn test_progressive_timeout_with_activity() {
    let config = ProgressiveTimeoutConfig::default();
    let mut state = ProgressiveTimeoutState::new();

    // Initial phase - should continue
    let decision = state.calculate_decision(&config);
    assert_eq!(decision, TimeoutDecision::Continue);

    // Simulate discovering sensors
    state.record_discovery();
    std::thread::sleep(Duration::from_millis(100));
    state.record_discovery();

    // Still in initial phase with activity
    assert!(state.has_recent_activity(config.activity_window_secs));
    assert!(state.sensors_discovered > 0);
}

#[test]
fn test_connection_quality_monitoring() {
    let mut coordinator = SensorFlowCoordinator::new();
    coordinator.start_discovery();

    let trainer = make_trainer();
    coordinator.discover_sensor(trainer.clone());
    coordinator.stop_discovery();

    coordinator.connect(&trainer.device_id).unwrap();

    // Simulate some data to update quality
    let sensor_id = Uuid::new_v4();
    for _ in 0..10 {
        let reading = SensorReading {
            sensor_id,
            timestamp: Instant::now(),
            power_watts: Some(200),
            cadence_rpm: Some(90),
            heart_rate_bpm: None,
            speed_kmh: Some(30.0),
            distance_delta_m: Some(8.33),
        };
        coordinator
            .receive_data(&trainer.device_id, reading)
            .unwrap();
    }

    // Quality should be good with strong signal and consistent data
    let quality = coordinator.get_quality_level(&trainer.device_id);
    assert!(quality.is_some());
    // With good RSSI (-55) and consistent data, should be at least Fair
    let level = quality.unwrap();
    assert!(
        level == QualityLevel::Excellent
            || level == QualityLevel::Good
            || level == QualityLevel::Fair
    );
}

#[test]
fn test_sensor_cache_integration() {
    // Create a cached sensor
    let cached = CachedSensor {
        device_id: "ble:wahoo_kickr_1234".to_string(),
        name: "KICKR Core 1234".to_string(),
        sensor_type: SensorType::Trainer,
        protocol: Protocol::BleFtms,
        last_connected: chrono::Utc::now(),
        connection_count: 5,
        nickname: Some("My Trainer".to_string()),
        is_preferred: true,
    };

    // Verify cached sensor properties
    assert!(cached.is_preferred);
    assert_eq!(cached.connection_count, 5);
    assert!(cached.nickname.is_some());
}

#[test]
fn test_session_persistence_integration() {
    let mut session = ConnectionSessionManager::new();

    // Add sensors to session using the constructor
    let mut sensor1 = SessionSensor::new(
        "ble:trainer_1".to_string(),
        "Trainer 1".to_string(),
        SensorType::Trainer,
        Protocol::BleFtms,
    );
    sensor1.is_primary = true;

    let mut sensor2 = SessionSensor::new(
        "ble:hr_1".to_string(),
        "HR 1".to_string(),
        SensorType::HeartRate,
        Protocol::BleHeartRate,
    );
    sensor2.is_primary = true;

    session.add_sensor(sensor1);
    session.add_sensor(sensor2);

    // Verify session state
    assert_eq!(session.sensor_count(), 2);
    assert!(session.has_sensor("ble:trainer_1"));
    assert!(session.has_sensor("ble:hr_1"));

    // Get reconnection targets
    let targets = session.get_reconnection_targets();
    assert_eq!(targets.len(), 2);

    // Remove sensor
    session.remove_sensor("ble:hr_1");
    assert_eq!(session.sensor_count(), 1);
    assert!(!session.has_sensor("ble:hr_1"));
}

#[test]
fn test_exponential_backoff_integration() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        multiplier: 2.0,
        max_attempts: 5,
        jitter_factor: 0.0,
    };
    let mut backoff = ExponentialBackoff::new(config);

    // First few delays should follow exponential pattern
    let d1 = backoff.next_delay().unwrap();
    let d2 = backoff.next_delay().unwrap();
    let d3 = backoff.next_delay().unwrap();

    // Delays should increase
    assert!(d2 > d1);
    assert!(d3 > d2);

    // After max attempts, should return None
    let _ = backoff.next_delay(); // 4th
    let _ = backoff.next_delay(); // 5th
    assert!(backoff.next_delay().is_none()); // 6th - exhausted

    // Reset should allow retries
    backoff.reset();
    assert!(backoff.next_delay().is_some());
}

#[test]
fn test_conflict_detector_integration() {
    let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
        strategy: ResolutionStrategy::AutoPriority,
        auto_resolve_non_critical: true,
        persist_resolutions: false,
    });

    let trainer = make_trainer();
    let power_meter = make_power_meter();

    // Register both (creates power conflict)
    detector.register_sensor(&trainer);
    detector.register_sensor(&power_meter);

    // Should have a power conflict
    assert!(detector.has_conflict(DataType::Power));
    let conflict = detector.get_conflict(DataType::Power).unwrap();
    assert_eq!(conflict.sources.len(), 2);

    // Mark both connected
    detector.update_connection_status(&trainer.device_id, true);
    detector.update_connection_status(&power_meter.device_id, true);

    // Set power meter as primary
    detector.set_primary(DataType::Power, &power_meter.device_id);
    assert!(detector.is_primary(&power_meter.device_id, DataType::Power));

    // Verify failover is available
    assert!(detector.has_failover_available(DataType::Power));

    // Handle primary disconnect
    let failovers = detector.handle_primary_disconnect(&power_meter.device_id);
    assert!(!failovers.is_empty());

    // Trainer should now be primary
    assert_eq!(
        detector.get_primary(DataType::Power),
        Some(trainer.device_id.as_str())
    );
}

#[test]
fn test_mock_data_parsing() {
    // Test FTMS data parsing
    let ftms_data = generate_mock_ftms_data(250, 180, 3000);
    let parsed = parse_indoor_bike_data(&ftms_data).unwrap();
    assert_eq!(parsed.power_watts, Some(250));
    assert_eq!(parsed.cadence_rpm, Some(90)); // 180 / 2
    assert!((parsed.speed_kmh.unwrap() - 30.0).abs() < 0.1); // 3000 * 0.01

    // Test cycling power parsing
    let power_data = generate_mock_power_data(275);
    let parsed = parse_cycling_power_measurement(&power_data).unwrap();
    assert_eq!(parsed.power_watts, 275);

    // Test heart rate parsing
    let hr_data = generate_mock_hr_data(155);
    let parsed = parse_heart_rate_measurement(&hr_data).unwrap();
    assert_eq!(parsed.heart_rate_bpm, 155);
}

#[test]
fn test_event_sequence_verification() {
    let mut coordinator = SensorFlowCoordinator::new();

    // Execute full flow
    coordinator.start_discovery();
    coordinator.discover_sensor(make_trainer());
    coordinator.stop_discovery();
    coordinator.connect_all_by_priority();
    coordinator.shutdown();

    // Verify event sequence
    let events = coordinator.get_events();

    // First event should be ScanStarted
    assert!(matches!(events.first(), Some(SensorEvent::ScanStarted)));

    // Should have discovery event
    assert!(events
        .iter()
        .any(|e| matches!(e, SensorEvent::Discovered(_))));

    // Should have ScanStopped
    assert!(events.iter().any(|e| matches!(e, SensorEvent::ScanStopped)));

    // Should have connection events
    assert!(events
        .iter()
        .any(|e| matches!(e, SensorEvent::ConnectionChanged { .. })));
}
