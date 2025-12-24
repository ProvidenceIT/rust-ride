//! Unit tests for FTMS data parsing.
//!
//! T025: Unit test for FTMS data parsing

use rustride::sensors::ftms::{
    build_request_control, build_set_target_power, build_start_training, build_stop_training,
    parse_cycling_power_measurement, parse_heart_rate_measurement, parse_indoor_bike_data,
};

#[test]
fn test_parse_indoor_bike_data_minimal() {
    // Flags: 0x0000 (only instantaneous speed present)
    // Speed: 0 km/h
    let data = [0x00, 0x00, 0x00, 0x00];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert_eq!(result.speed_kmh, Some(0.0));
    assert!(result.power_watts.is_none());
    assert!(result.cadence_rpm.is_none());
}

#[test]
fn test_parse_indoor_bike_data_speed_only() {
    // Flags: 0x0000 (only instantaneous speed)
    // Speed: 2500 = 25.00 km/h
    let data = [0x00, 0x00, 0xC4, 0x09];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert!((result.speed_kmh.unwrap() - 25.0).abs() < 0.01);
}

#[test]
fn test_parse_indoor_bike_data_with_power() {
    // Flags: 0x0040 (instantaneous power present)
    // Speed: 3000 = 30.00 km/h
    // Power: 250W
    let data = [0x40, 0x00, 0xB8, 0x0B, 0xFA, 0x00];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert!((result.speed_kmh.unwrap() - 30.0).abs() < 0.01);
    assert_eq!(result.power_watts.unwrap(), 250);
}

#[test]
fn test_parse_indoor_bike_data_with_cadence() {
    // Flags: 0x0004 (instantaneous cadence present)
    // Speed: 2000 = 20.00 km/h
    // Cadence: 180 = 90 RPM (0.5 resolution)
    let data = [0x04, 0x00, 0xD0, 0x07, 0xB4, 0x00];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert!((result.speed_kmh.unwrap() - 20.0).abs() < 0.01);
    assert_eq!(result.cadence_rpm.unwrap(), 90);
}

#[test]
fn test_parse_indoor_bike_data_full() {
    // Flags: 0x0044 (cadence + power)
    // Speed: 3500 = 35.00 km/h
    // Cadence: 190 = 95 RPM
    // Power: 300W
    let data = [0x44, 0x00, 0xAC, 0x0D, 0xBE, 0x00, 0x2C, 0x01];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert!((result.speed_kmh.unwrap() - 35.0).abs() < 0.01);
    assert_eq!(result.cadence_rpm.unwrap(), 95);
    assert_eq!(result.power_watts.unwrap(), 300);
}

#[test]
fn test_parse_indoor_bike_data_more_data_flag() {
    // Flags: 0x0001 (more data = no instantaneous speed)
    let data = [0x01, 0x00];
    let result = parse_indoor_bike_data(&data).unwrap();

    assert!(result.speed_kmh.is_none());
}

#[test]
fn test_parse_indoor_bike_data_invalid_too_short() {
    let data = [0x00];
    let result = parse_indoor_bike_data(&data);
    assert!(result.is_none());
}

#[test]
fn test_parse_cycling_power_measurement_basic() {
    // Flags: 0x0000 (no optional fields)
    // Power: 200W
    let data = [0x00, 0x00, 0xC8, 0x00];
    let result = parse_cycling_power_measurement(&data).unwrap();

    assert_eq!(result.power_watts, 200);
    assert!(result.power_balance.is_none());
    assert!(result.crank_revolutions.is_none());
}

#[test]
fn test_parse_cycling_power_measurement_negative() {
    // Flags: 0x0000
    // Power: -50W (invalid but should parse)
    let data = [0x00, 0x00, 0xCE, 0xFF];
    let result = parse_cycling_power_measurement(&data).unwrap();

    assert_eq!(result.power_watts, -50);
}

#[test]
fn test_parse_cycling_power_measurement_high_power() {
    // Flags: 0x0000
    // Power: 1500W
    let data = [0x00, 0x00, 0xDC, 0x05];
    let result = parse_cycling_power_measurement(&data).unwrap();

    assert_eq!(result.power_watts, 1500);
}

#[test]
fn test_parse_cycling_power_measurement_invalid() {
    let data = [0x00, 0x00, 0xC8];
    let result = parse_cycling_power_measurement(&data);
    assert!(result.is_none());
}

#[test]
fn test_parse_heart_rate_u8_format() {
    // Flags: 0x00 (8-bit HR)
    // HR: 145 BPM
    let data = [0x00, 0x91];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate_bpm, 145);
    assert!(!result.sensor_contact);
}

#[test]
fn test_parse_heart_rate_u16_format() {
    // Flags: 0x01 (16-bit HR)
    // HR: 180 BPM
    let data = [0x01, 0xB4, 0x00];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate_bpm, 180);
}

#[test]
fn test_parse_heart_rate_with_sensor_contact() {
    // Flags: 0x06 (sensor contact supported + detected)
    // HR: 120 BPM
    let data = [0x06, 0x78];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate_bpm, 120);
    assert!(result.sensor_contact);
}

#[test]
fn test_parse_heart_rate_no_contact() {
    // Flags: 0x04 (sensor contact supported, not detected)
    // HR: 100 BPM
    let data = [0x04, 0x64];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate_bpm, 100);
    assert!(!result.sensor_contact);
}

#[test]
fn test_parse_heart_rate_invalid() {
    let data: [u8; 0] = [];
    let result = parse_heart_rate_measurement(&data);
    assert!(result.is_none());
}

#[test]
fn test_build_request_control() {
    let cmd = build_request_control();
    assert_eq!(cmd, vec![0x00]);
}

#[test]
fn test_build_start_training() {
    let cmd = build_start_training();
    assert_eq!(cmd, vec![0x07]);
}

#[test]
fn test_build_stop_training_pause() {
    let cmd = build_stop_training(true);
    assert_eq!(cmd, vec![0x08, 0x02]);
}

#[test]
fn test_build_stop_training_stop() {
    let cmd = build_stop_training(false);
    assert_eq!(cmd, vec![0x08, 0x01]);
}

#[test]
fn test_build_set_target_power_low() {
    let cmd = build_set_target_power(100);
    assert_eq!(cmd, vec![0x05, 0x64, 0x00]);
}

#[test]
fn test_build_set_target_power_high() {
    let cmd = build_set_target_power(500);
    assert_eq!(cmd, vec![0x05, 0xF4, 0x01]);
}

#[test]
fn test_build_set_target_power_erg_mode() {
    // Typical ERG mode: set to 250W
    let cmd = build_set_target_power(250);
    assert_eq!(cmd, vec![0x05, 0xFA, 0x00]);
}
