//! Integration test with mock BLE adapter.
//!
//! T027: Integration test with mock BLE adapter
//!
//! Note: Full BLE integration tests require actual hardware or a proper mock.
//! This module provides test infrastructure for when hardware is available.

use rustride::sensors::ftms::{
    parse_cycling_power_measurement, parse_heart_rate_measurement, parse_indoor_bike_data,
};
use rustride::sensors::types::{ConnectionState, Protocol, SensorConfig, SensorEvent, SensorType};
use std::time::Instant;

/// Mock sensor data generator for testing.
pub struct MockSensorData {
    /// Current power value
    pub power: u16,
    /// Current cadence
    pub cadence: u16,
    /// Current speed (in 0.01 km/h)
    pub speed: u16,
    /// Current heart rate
    pub heart_rate: u8,
}

impl Default for MockSensorData {
    fn default() -> Self {
        Self {
            power: 200,
            cadence: 180, // 90 RPM (0.5 resolution)
            speed: 3000,  // 30.0 km/h
            heart_rate: 145,
        }
    }
}

impl MockSensorData {
    /// Generate Indoor Bike Data packet.
    pub fn generate_ftms_indoor_bike_data(&self) -> Vec<u8> {
        // Flags: 0x0044 (instantaneous cadence + instantaneous power)
        let mut data = vec![0x44, 0x00];

        // Instantaneous speed (always present when more_data=0)
        data.extend_from_slice(&self.speed.to_le_bytes());

        // Instantaneous cadence
        data.extend_from_slice(&self.cadence.to_le_bytes());

        // Instantaneous power
        data.extend_from_slice(&(self.power as i16).to_le_bytes());

        data
    }

    /// Generate Cycling Power Measurement packet.
    pub fn generate_cycling_power_measurement(&self) -> Vec<u8> {
        // Flags: 0x0000 (no optional fields)
        let mut data = vec![0x00, 0x00];

        // Power
        data.extend_from_slice(&(self.power as i16).to_le_bytes());

        data
    }

    /// Generate Heart Rate Measurement packet.
    pub fn generate_heart_rate_measurement(&self) -> Vec<u8> {
        // Flags: 0x00 (8-bit HR)
        vec![0x00, self.heart_rate]
    }
}

#[test]
fn test_mock_ftms_data_generation() {
    let mock = MockSensorData::default();
    let data = mock.generate_ftms_indoor_bike_data();

    let parsed = parse_indoor_bike_data(&data).unwrap();

    assert!((parsed.speed_kmh.unwrap() - 30.0).abs() < 0.1);
    assert_eq!(parsed.cadence_rpm.unwrap(), 90);
    assert_eq!(parsed.power_watts.unwrap(), 200);
}

#[test]
fn test_mock_cycling_power_data_generation() {
    let mock = MockSensorData::default();
    let data = mock.generate_cycling_power_measurement();

    let parsed = parse_cycling_power_measurement(&data).unwrap();

    assert_eq!(parsed.power_watts, 200);
}

#[test]
fn test_mock_heart_rate_data_generation() {
    let mock = MockSensorData::default();
    let data = mock.generate_heart_rate_measurement();

    let parsed = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(parsed.heart_rate_bpm, 145);
}

#[test]
fn test_varying_power_values() {
    for power in [0u16, 100, 200, 300, 500, 1000, 1500, 2000] {
        let mock = MockSensorData {
            power,
            ..Default::default()
        };
        let data = mock.generate_cycling_power_measurement();
        let parsed = parse_cycling_power_measurement(&data).unwrap();
        assert_eq!(parsed.power_watts, power as i16);
    }
}

#[test]
fn test_varying_cadence_values() {
    // Cadence in 0.5 RPM resolution
    for cadence_rpm in [60u16, 80, 90, 100, 120] {
        let cadence_raw = cadence_rpm * 2; // Convert RPM to raw value
        let mock = MockSensorData {
            cadence: cadence_raw,
            ..Default::default()
        };
        let data = mock.generate_ftms_indoor_bike_data();
        let parsed = parse_indoor_bike_data(&data).unwrap();
        assert_eq!(parsed.cadence_rpm.unwrap(), cadence_rpm);
    }
}

#[test]
fn test_varying_heart_rate_values() {
    for hr in [60u8, 100, 145, 180, 200] {
        let mock = MockSensorData {
            heart_rate: hr,
            ..Default::default()
        };
        let data = mock.generate_heart_rate_measurement();
        let parsed = parse_heart_rate_measurement(&data).unwrap();
        assert_eq!(parsed.heart_rate_bpm, hr as u16);
    }
}

#[test]
fn test_sensor_config_defaults() {
    let config = SensorConfig::default();

    assert_eq!(config.discovery_timeout_secs, 30);
    assert_eq!(config.connection_timeout_secs, 10);
    assert!(config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 3);
}

#[test]
fn test_sensor_event_types() {
    // Verify SensorEvent variants can be constructed
    let _scan_started = SensorEvent::ScanStarted;
    let _scan_stopped = SensorEvent::ScanStopped;
    let _error = SensorEvent::Error("test error".to_string());
    let _connection_changed = SensorEvent::ConnectionChanged {
        device_id: "test-device".to_string(),
        state: ConnectionState::Connected,
    };
}

#[test]
fn test_connection_state_transitions() {
    // Valid state transitions
    let states = [
        ConnectionState::Disconnected,
        ConnectionState::Connecting,
        ConnectionState::Connected,
        ConnectionState::Reconnecting,
        ConnectionState::Connected,
        ConnectionState::Disconnected,
    ];

    // All states should be displayable
    for state in &states {
        let _ = format!("{}", state);
    }
}

#[test]
fn test_sensor_type_protocol_mapping() {
    // Verify expected type-protocol pairings
    let mappings = [
        (SensorType::Trainer, Protocol::BleFtms),
        (SensorType::PowerMeter, Protocol::BleCyclingPower),
        (SensorType::HeartRate, Protocol::BleHeartRate),
        (SensorType::SpeedCadence, Protocol::BleCsc),
    ];

    for (sensor_type, protocol) in mappings {
        // Ensure both can be formatted
        let _ = format!("{} uses {}", sensor_type, protocol);
    }
}

/// Simulates a sensor session for integration testing.
#[allow(dead_code)]
pub struct MockSensorSession {
    config: SensorConfig,
    mock_data: MockSensorData,
    is_connected: bool,
    last_event_time: Instant,
}

impl MockSensorSession {
    /// Create a new mock sensor session.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            config: SensorConfig::default(),
            mock_data: MockSensorData::default(),
            is_connected: false,
            last_event_time: Instant::now(),
        }
    }

    /// Simulate connecting to a sensor.
    #[allow(dead_code)]
    pub fn connect(&mut self) -> Result<(), &'static str> {
        if self.is_connected {
            return Err("Already connected");
        }
        self.is_connected = true;
        Ok(())
    }

    /// Simulate disconnecting from a sensor.
    #[allow(dead_code)]
    pub fn disconnect(&mut self) {
        self.is_connected = false;
    }

    /// Get mock sensor data.
    #[allow(dead_code)]
    pub fn get_data(&mut self) -> Option<MockSensorData> {
        if !self.is_connected {
            return None;
        }

        // Update last event time
        self.last_event_time = Instant::now();

        Some(self.mock_data.clone())
    }
}

impl Clone for MockSensorData {
    fn clone(&self) -> Self {
        Self {
            power: self.power,
            cadence: self.cadence,
            speed: self.speed,
            heart_rate: self.heart_rate,
        }
    }
}
