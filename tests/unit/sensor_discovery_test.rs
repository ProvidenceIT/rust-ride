//! Unit tests for sensor discovery filtering.
//!
//! T026: Unit test for sensor discovery filtering

use rustride::sensors::ftms::{
    CSC_SERVICE_UUID, CYCLING_POWER_SERVICE_UUID, FTMS_SERVICE_UUID, HEART_RATE_SERVICE_UUID,
};
use rustride::sensors::types::{Protocol, SensorType};
use uuid::Uuid;

/// Maps a service UUID to a SensorType.
fn classify_sensor(service_uuid: &Uuid) -> Option<(SensorType, Protocol)> {
    if *service_uuid == FTMS_SERVICE_UUID {
        Some((SensorType::Trainer, Protocol::BleFtms))
    } else if *service_uuid == CYCLING_POWER_SERVICE_UUID {
        Some((SensorType::PowerMeter, Protocol::BleCyclingPower))
    } else if *service_uuid == HEART_RATE_SERVICE_UUID {
        Some((SensorType::HeartRate, Protocol::BleHeartRate))
    } else if *service_uuid == CSC_SERVICE_UUID {
        Some((SensorType::SpeedCadence, Protocol::BleCsc))
    } else {
        None
    }
}

/// Checks if a service UUID is a supported fitness service.
fn is_supported_service(service_uuid: &Uuid) -> bool {
    *service_uuid == FTMS_SERVICE_UUID
        || *service_uuid == CYCLING_POWER_SERVICE_UUID
        || *service_uuid == HEART_RATE_SERVICE_UUID
        || *service_uuid == CSC_SERVICE_UUID
}

#[test]
fn test_ftms_service_uuid() {
    // FTMS Service UUID should be 0x1826
    assert_eq!(
        FTMS_SERVICE_UUID,
        Uuid::from_u128(0x00001826_0000_1000_8000_00805f9b34fb)
    );
}

#[test]
fn test_cycling_power_service_uuid() {
    // Cycling Power Service UUID should be 0x1818
    assert_eq!(
        CYCLING_POWER_SERVICE_UUID,
        Uuid::from_u128(0x00001818_0000_1000_8000_00805f9b34fb)
    );
}

#[test]
fn test_heart_rate_service_uuid() {
    // Heart Rate Service UUID should be 0x180D
    assert_eq!(
        HEART_RATE_SERVICE_UUID,
        Uuid::from_u128(0x0000180d_0000_1000_8000_00805f9b34fb)
    );
}

#[test]
fn test_csc_service_uuid() {
    // CSC Service UUID should be 0x1816
    assert_eq!(
        CSC_SERVICE_UUID,
        Uuid::from_u128(0x00001816_0000_1000_8000_00805f9b34fb)
    );
}

#[test]
fn test_classify_ftms_trainer() {
    let result = classify_sensor(&FTMS_SERVICE_UUID);
    assert!(result.is_some());
    let (sensor_type, protocol) = result.unwrap();
    assert_eq!(sensor_type, SensorType::Trainer);
    assert_eq!(protocol, Protocol::BleFtms);
}

#[test]
fn test_classify_power_meter() {
    let result = classify_sensor(&CYCLING_POWER_SERVICE_UUID);
    assert!(result.is_some());
    let (sensor_type, protocol) = result.unwrap();
    assert_eq!(sensor_type, SensorType::PowerMeter);
    assert_eq!(protocol, Protocol::BleCyclingPower);
}

#[test]
fn test_classify_heart_rate() {
    let result = classify_sensor(&HEART_RATE_SERVICE_UUID);
    assert!(result.is_some());
    let (sensor_type, protocol) = result.unwrap();
    assert_eq!(sensor_type, SensorType::HeartRate);
    assert_eq!(protocol, Protocol::BleHeartRate);
}

#[test]
fn test_classify_speed_cadence() {
    let result = classify_sensor(&CSC_SERVICE_UUID);
    assert!(result.is_some());
    let (sensor_type, protocol) = result.unwrap();
    assert_eq!(sensor_type, SensorType::SpeedCadence);
    assert_eq!(protocol, Protocol::BleCsc);
}

#[test]
fn test_classify_unknown_service() {
    let unknown_uuid = Uuid::from_u128(0x0000abcd_0000_1000_8000_00805f9b34fb);
    let result = classify_sensor(&unknown_uuid);
    assert!(result.is_none());
}

#[test]
fn test_filter_supported_services() {
    let services = [
        FTMS_SERVICE_UUID,
        CYCLING_POWER_SERVICE_UUID,
        HEART_RATE_SERVICE_UUID,
        Uuid::from_u128(0x0000180f_0000_1000_8000_00805f9b34fb), // Battery Service (not supported)
        Uuid::from_u128(0x0000180a_0000_1000_8000_00805f9b34fb), // Device Info (not supported)
    ];

    let supported: Vec<_> = services
        .iter()
        .filter(|uuid| is_supported_service(uuid))
        .collect();

    assert_eq!(supported.len(), 3);
    assert!(supported.contains(&&FTMS_SERVICE_UUID));
    assert!(supported.contains(&&CYCLING_POWER_SERVICE_UUID));
    assert!(supported.contains(&&HEART_RATE_SERVICE_UUID));
}

#[test]
fn test_sensor_type_display() {
    assert_eq!(format!("{}", SensorType::Trainer), "Smart Trainer");
    assert_eq!(format!("{}", SensorType::PowerMeter), "Power Meter");
    assert_eq!(format!("{}", SensorType::HeartRate), "Heart Rate");
    assert_eq!(format!("{}", SensorType::Cadence), "Cadence");
    assert_eq!(format!("{}", SensorType::Speed), "Speed");
    assert_eq!(format!("{}", SensorType::SpeedCadence), "Speed/Cadence");
}

#[test]
fn test_protocol_display() {
    assert_eq!(format!("{}", Protocol::BleFtms), "FTMS");
    assert_eq!(format!("{}", Protocol::BleCyclingPower), "Cycling Power");
    assert_eq!(format!("{}", Protocol::BleHeartRate), "Heart Rate");
    assert_eq!(format!("{}", Protocol::BleCsc), "Cycling Speed/Cadence");
}

#[test]
fn test_multiple_services_per_device() {
    // A smart trainer might advertise both FTMS and Heart Rate
    let services = [FTMS_SERVICE_UUID, HEART_RATE_SERVICE_UUID];

    let classifications: Vec<_> = services.iter().filter_map(classify_sensor).collect();

    assert_eq!(classifications.len(), 2);

    // Should find trainer
    assert!(classifications
        .iter()
        .any(|(t, _)| *t == SensorType::Trainer));

    // Should find heart rate
    assert!(classifications
        .iter()
        .any(|(t, _)| *t == SensorType::HeartRate));
}
