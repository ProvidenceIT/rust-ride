//! T054: Unit tests for cycling dynamics parsing.
//!
//! Tests the parsing of Cycling Power Service BLE data containing
//! left/right power balance and pedaling dynamics.

use rustride::sensors::{
    CyclingDynamicsData, DynamicsAverages, LeftRightBalance, PedalSmoothness, PowerFeatures,
    PowerMeasurementParser, PowerPhase, TorqueEffectiveness,
};

/// Test basic power measurement parsing without dynamics.
#[test]
fn test_power_measurement_basic() {
    // Basic power measurement: flags=0x0000, power=200W
    let data: Vec<u8> = vec![0x00, 0x00, 0xC8, 0x00];
    let result = PowerMeasurementParser::parse(&data);

    assert!(result.is_ok());
    let measurement = result.unwrap();
    assert_eq!(measurement.instant_power, 200);
    assert!(measurement.balance.is_none());
}

/// Test power measurement with L/R balance.
#[test]
fn test_power_measurement_with_balance() {
    // Flags bit 0 set = pedal power balance present
    // Power = 250W, Balance raw = 104 (52.0% at 0.5% resolution)
    let data: Vec<u8> = vec![0x01, 0x00, 0xFA, 0x00, 0x68];
    let result = PowerMeasurementParser::parse(&data);

    assert!(result.is_ok());
    let measurement = result.unwrap();
    assert_eq!(measurement.instant_power, 250);
    assert!(measurement.balance.is_some());

    let balance = measurement.balance.unwrap();
    assert_eq!(balance.left_percent, 52.0);
    assert_eq!(balance.right_percent, 48.0);
}

/// Test power measurement with balance reference indicator.
#[test]
fn test_power_measurement_balance_reference() {
    // Flags bit 0 = balance present, bit 1 clear = left is reference
    // Power = 200W, Balance raw = 96 (48.0% at 0.5% resolution)
    let data: Vec<u8> = vec![0x01, 0x00, 0xC8, 0x00, 0x60];
    let result = PowerMeasurementParser::parse(&data);

    assert!(result.is_ok());
    let measurement = result.unwrap();
    assert!(measurement.balance.is_some());
    let balance = measurement.balance.unwrap();
    // Bit 1 clear means left is reference
    assert!(balance.reference_is_left);
}

/// Test left/right balance creation and percentage calculation.
#[test]
fn test_left_right_balance_creation() {
    let balance = LeftRightBalance::from_reference(55.0, true);

    assert_eq!(balance.left_percent, 55.0);
    assert_eq!(balance.right_percent, 45.0);
    assert!(balance.reference_is_left);
}

/// Test balance from right reference.
#[test]
fn test_left_right_balance_right_reference() {
    let balance = LeftRightBalance::from_reference(52.0, false);

    // When right is reference, the percentage is for right
    assert_eq!(balance.right_percent, 52.0);
    assert_eq!(balance.left_percent, 48.0);
    assert!(!balance.reference_is_left);
}

/// Test balance detection for balanced rider.
#[test]
fn test_balance_is_balanced() {
    let balanced = LeftRightBalance::from_reference(50.0, true);
    assert!(balanced.is_balanced(2.0)); // Within 2% is balanced

    let imbalanced = LeftRightBalance::from_reference(55.0, true);
    assert!(!imbalanced.is_balanced(2.0)); // 55/45 is not balanced
}

/// Test balance imbalance detection.
#[test]
fn test_balance_imbalance() {
    let balance = LeftRightBalance::from_reference(53.0, true);

    assert!(!balance.is_balanced(2.0));
    assert_eq!(balance.imbalance(), 6.0); // |53-47| = 6
}

/// Test pedal smoothness creation.
#[test]
fn test_pedal_smoothness() {
    let smoothness = PedalSmoothness::new(22.5, 24.0);

    assert_eq!(smoothness.left_percent, 22.5);
    assert_eq!(smoothness.right_percent, 24.0);
    assert_eq!(smoothness.combined_percent, 23.25); // Average
}

/// Test torque effectiveness creation.
#[test]
fn test_torque_effectiveness() {
    let te = TorqueEffectiveness::new(75.0, 72.0);

    assert_eq!(te.left_percent, 75.0);
    assert_eq!(te.right_percent, 72.0);
    assert_eq!(te.combined_percent, 73.5); // Average
}

/// Test power phase arc length calculation.
#[test]
fn test_power_phase_arc() {
    let phase = PowerPhase::new(0.0, 180.0, Some(90.0));

    assert_eq!(phase.arc_length(), 180.0);
    assert_eq!(phase.peak_angle, Some(90.0));
}

/// Test dynamics averages update.
#[test]
fn test_dynamics_averages_update() {
    let mut averages = DynamicsAverages::default();

    let data1 = CyclingDynamicsData {
        balance: LeftRightBalance::from_reference(52.0, true),
        smoothness: PedalSmoothness::new(22.0, 24.0),
        torque_effectiveness: TorqueEffectiveness::new(75.0, 72.0),
        left_power_phase: None,
        right_power_phase: None,
        timestamp: None,
    };

    averages.update(&data1);
    assert_eq!(averages.sample_count, 1);
    assert_eq!(averages.avg_left_balance, 52.0);
    assert_eq!(averages.avg_right_balance, 48.0);

    let data2 = CyclingDynamicsData {
        balance: LeftRightBalance::from_reference(50.0, true),
        smoothness: PedalSmoothness::new(24.0, 26.0),
        torque_effectiveness: TorqueEffectiveness::new(78.0, 76.0),
        left_power_phase: None,
        right_power_phase: None,
        timestamp: None,
    };

    averages.update(&data2);
    assert_eq!(averages.sample_count, 2);
    assert_eq!(averages.avg_left_balance, 51.0); // (52+50)/2
    assert_eq!(averages.avg_right_balance, 49.0); // (48+50)/2
}

/// Test power features bit flag parsing.
#[test]
fn test_power_features_parsing() {
    // Features: supports L/R balance (bit 0), accumulated torque (bit 1)
    let features = PowerFeatures::from_bytes(&[0x03, 0x00, 0x00, 0x00]);

    assert!(features.pedal_power_balance);
    assert!(features.accumulated_torque);
}

/// Test power features without dynamics.
#[test]
fn test_power_features_no_dynamics() {
    // Basic power meter without dynamics
    let features = PowerFeatures::from_bytes(&[0x00, 0x00, 0x00, 0x00]);

    assert!(!features.pedal_power_balance);
    assert!(!features.crank_length_adjustment);
}

/// Test cycling dynamics data default values.
#[test]
fn test_dynamics_data_default() {
    let data = CyclingDynamicsData::default();

    assert_eq!(data.balance.left_percent, 50.0);
    assert_eq!(data.balance.right_percent, 50.0);
    assert_eq!(data.smoothness.combined_percent, 0.0);
    assert_eq!(data.torque_effectiveness.combined_percent, 0.0);
}

/// Test dynamics averages default state.
#[test]
fn test_dynamics_averages_default() {
    let averages = DynamicsAverages::default();

    assert_eq!(averages.sample_count, 0);
    // Default values are 0.0 (will be populated on first update)
    assert_eq!(averages.avg_left_balance, 0.0);
    assert_eq!(averages.avg_right_balance, 0.0);
}

/// Test parsing invalid data.
#[test]
fn test_power_measurement_invalid_data() {
    // Too short data
    let data: Vec<u8> = vec![0x00];
    let result = PowerMeasurementParser::parse(&data);

    assert!(result.is_err());
}
