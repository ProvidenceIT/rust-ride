//! Unit tests for sensor fusion algorithms.
//!
//! T136: Write unit tests for sensor fusion algorithms

use rustride::sensors::{
    CadenceFusion, FusionDiagnostics, FusionMode, SensorFusion, SensorFusionConfig,
};
use std::thread::sleep;
use std::time::Duration;

/// Test default configuration values.
#[test]
fn test_default_config_values() {
    let config = SensorFusionConfig::default();

    assert_eq!(config.primary_weight, 0.6);
    assert_eq!(config.secondary_weight, 0.4);
    assert_eq!(config.dropout_timeout_ms, 3000);
    assert_eq!(config.max_deviation_percent, 20.0);
    assert!(config.auto_fallback);
    assert_eq!(config.smoothing_factor, 0.3);
    assert_eq!(config.window_size, 5);
}

/// Test primary dominant configuration.
#[test]
fn test_primary_dominant_config() {
    let config = SensorFusionConfig::primary_dominant();

    assert_eq!(config.primary_weight, 0.8);
    assert_eq!(config.secondary_weight, 0.2);
}

/// Test equal weights configuration.
#[test]
fn test_equal_weights_config() {
    let config = SensorFusionConfig::equal_weights();

    assert_eq!(config.primary_weight, 0.5);
    assert_eq!(config.secondary_weight, 0.5);
}

/// Test weight normalization with valid weights.
#[test]
fn test_weight_normalization() {
    let mut config = SensorFusionConfig {
        primary_weight: 3.0,
        secondary_weight: 1.0,
        ..Default::default()
    };
    config.normalize_weights();

    assert!((config.primary_weight - 0.75).abs() < 0.001);
    assert!((config.secondary_weight - 0.25).abs() < 0.001);
}

/// Test weight normalization with zero weights.
#[test]
fn test_weight_normalization_zero() {
    let mut config = SensorFusionConfig {
        primary_weight: 0.0,
        secondary_weight: 0.0,
        ..Default::default()
    };
    config.normalize_weights();

    // Should default to equal weights
    assert_eq!(config.primary_weight, 0.5);
    assert_eq!(config.secondary_weight, 0.5);
}

/// Test cadence fusion initialization.
#[test]
fn test_cadence_fusion_initial_state() {
    let fusion = CadenceFusion::new();

    assert!(fusion.get_fused_value().is_none());

    let diag = fusion.get_diagnostics();
    assert_eq!(diag.mode, FusionMode::NoData);
    assert!(!diag.primary_active);
    assert!(!diag.secondary_active);
    assert!(diag.fused_value.is_none());
}

/// Test fusion with only primary sensor.
#[test]
fn test_fusion_primary_only() {
    let mut fusion = CadenceFusion::new();

    fusion.update(Some(85.0), None);

    let diag = fusion.get_diagnostics();
    assert!(diag.primary_active);
    assert!(!diag.secondary_active);
    assert_eq!(diag.mode, FusionMode::PrimaryOnly);
    assert!(fusion.get_fused_value().is_some());
}

/// Test fusion with only secondary sensor.
#[test]
fn test_fusion_secondary_only() {
    let mut fusion = CadenceFusion::new();

    fusion.update(None, Some(78.0));

    let diag = fusion.get_diagnostics();
    assert!(!diag.primary_active);
    assert!(diag.secondary_active);
    assert_eq!(diag.mode, FusionMode::SecondaryOnly);
    assert!(fusion.get_fused_value().is_some());
}

/// Test fusion with both sensors agreeing.
#[test]
fn test_fusion_dual_sensor_agreeing() {
    let mut fusion = CadenceFusion::new();

    fusion.update(Some(90.0), Some(88.0));

    let diag = fusion.get_diagnostics();
    assert!(diag.primary_active);
    assert!(diag.secondary_active);
    assert_eq!(diag.mode, FusionMode::DualSensor);
    assert!(diag.sensors_agree);

    let fused = fusion.get_fused_value().unwrap();
    // With default weights (0.6/0.4), fused should be 0.6*90 + 0.4*88 = 89.2
    assert!((fused - 89.2).abs() < 1.0);
}

/// Test fusion with sensors disagreeing.
#[test]
fn test_fusion_inconsistent_sensors() {
    let config = SensorFusionConfig {
        max_deviation_percent: 10.0,
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    // 100 vs 70 = 30% deviation > 10% threshold
    fusion.update(Some(100.0), Some(70.0));

    let diag = fusion.get_diagnostics();
    assert_eq!(diag.mode, FusionMode::Inconsistent);
    assert!(!diag.sensors_agree);
}

/// Test weighted fusion calculation.
#[test]
fn test_weighted_fusion_calculation() {
    let config = SensorFusionConfig {
        primary_weight: 0.8,
        secondary_weight: 0.2,
        smoothing_factor: 0.0, // Disable smoothing for predictable result
        window_size: 1,
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    // Primary=100, Secondary=80
    // Normalized weights: 0.8/0.2
    // Fused = 0.8*100 + 0.2*80 = 80 + 16 = 96
    fusion.update(Some(100.0), Some(80.0));

    let fused = fusion.get_fused_value().unwrap();
    assert!((fused - 96.0).abs() < 0.5);
}

/// Test sensor dropout detection.
#[test]
fn test_dropout_detection() {
    let config = SensorFusionConfig {
        dropout_timeout_ms: 50, // 50ms timeout
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    // Both active
    fusion.update(Some(90.0), Some(88.0));
    assert_eq!(fusion.get_diagnostics().mode, FusionMode::DualSensor);

    // Wait for dropout
    sleep(Duration::from_millis(60));

    // Only update primary
    fusion.update(Some(92.0), None);

    let diag = fusion.get_diagnostics();
    assert_eq!(diag.mode, FusionMode::PrimaryOnly);
}

/// Test auto fallback disabled.
#[test]
fn test_auto_fallback_disabled() {
    let config = SensorFusionConfig {
        auto_fallback: false,
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    // Only primary active, but auto_fallback is off
    fusion.update(Some(85.0), None);

    // Without auto_fallback, PrimaryOnly mode won't produce a fused value
    // (The implementation may vary - check actual behavior)
    let diag = fusion.get_diagnostics();
    assert_eq!(diag.mode, FusionMode::PrimaryOnly);
}

/// Test smoothing effect over multiple updates.
#[test]
fn test_smoothing_effect() {
    let config = SensorFusionConfig {
        smoothing_factor: 0.5,
        window_size: 3,
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    // Feed consistent values
    for _ in 0..5 {
        fusion.update(Some(90.0), Some(90.0));
    }

    let fused = fusion.get_fused_value().unwrap();
    // After smoothing, should converge to 90
    assert!((fused - 90.0).abs() < 2.0);
}

/// Test reset clears all state.
#[test]
fn test_reset() {
    let mut fusion = CadenceFusion::new();

    fusion.update(Some(90.0), Some(88.0));
    assert!(fusion.get_fused_value().is_some());

    fusion.reset();

    assert!(fusion.get_fused_value().is_none());
    let diag = fusion.get_diagnostics();
    assert_eq!(diag.mode, FusionMode::NoData);
    assert!(!diag.primary_active);
    assert!(!diag.secondary_active);
}

/// Test diagnostics age calculation.
#[test]
fn test_diagnostics_age() {
    let mut fusion = CadenceFusion::new();

    fusion.update(Some(90.0), Some(88.0));

    // Immediately after update, ages should be very small
    let diag = fusion.get_diagnostics();
    assert!(diag.primary_age_ms < 100);
    assert!(diag.secondary_age_ms < 100);

    // Wait a bit
    sleep(Duration::from_millis(50));

    let diag = fusion.get_diagnostics();
    assert!(diag.primary_age_ms >= 40);
    assert!(diag.secondary_age_ms >= 40);
}

/// Test deviation calculation.
#[test]
fn test_deviation_calculation() {
    let mut fusion = CadenceFusion::new();

    // 100 vs 80 = 20% deviation
    fusion.update(Some(100.0), Some(80.0));

    let diag = fusion.get_diagnostics();
    let deviation = diag.deviation_percent.unwrap();
    assert!((deviation - 20.0).abs() < 0.5);
}

/// Test fusion mode descriptions.
#[test]
fn test_fusion_mode_descriptions() {
    assert_eq!(FusionMode::DualSensor.description(), "Both sensors active");
    assert_eq!(FusionMode::PrimaryOnly.description(), "Primary sensor only");
    assert_eq!(
        FusionMode::SecondaryOnly.description(),
        "Secondary sensor only"
    );
    assert_eq!(FusionMode::NoData.description(), "No sensor data");
    assert_eq!(FusionMode::Inconsistent.description(), "Sensors disagree");
}

/// Test configure_fusion updates settings.
#[test]
fn test_configure_fusion() {
    let mut fusion = CadenceFusion::new();

    let new_config = SensorFusionConfig {
        primary_weight: 0.9,
        secondary_weight: 0.1,
        dropout_timeout_ms: 5000,
        ..Default::default()
    };

    fusion.configure_fusion(new_config);

    // Feed data and verify new weights are applied
    fusion.update(Some(100.0), Some(50.0));

    let fused = fusion.get_fused_value().unwrap();
    // With 0.9/0.1 weights: 0.9*100 + 0.1*50 = 95
    // (May be slightly different due to smoothing)
    assert!(fused > 90.0);
}

/// Test sample count in diagnostics.
#[test]
fn test_sample_count() {
    let config = SensorFusionConfig {
        window_size: 5,
        ..Default::default()
    };
    let mut fusion = CadenceFusion::with_config(config);

    for i in 0..3 {
        fusion.update(Some(90.0 + i as f32), Some(88.0 + i as f32));
    }

    let diag = fusion.get_diagnostics();
    assert_eq!(diag.sample_count, 3);

    // Add more samples beyond window size
    for i in 3..10 {
        fusion.update(Some(90.0 + i as f32), Some(88.0 + i as f32));
    }

    let diag = fusion.get_diagnostics();
    assert_eq!(diag.sample_count, 5); // Capped at window_size
}
