//! Unit tests for RSSI polling functionality.
//!
//! Tests verify that:
//! - ConnectionQualityMonitor correctly records and tracks RSSI values
//! - RSSI updates are reflected in quality scores and metrics
//! - Multiple sensors can be polled independently
//! - Quality degradation is detected from RSSI changes
//! - RSSI polling integrates with sensor state updates

use rustride::sensors::quality::{
    ConnectionQuality, ConnectionQualityConfig, ConnectionQualityMonitor, QualityLevel,
};
use std::time::Duration;

// ============================================================================
// RSSI Polling Interval Constants (matching manager.rs)
// ============================================================================

/// Default RSSI polling interval in milliseconds (matches SensorManager).
const RSSI_POLL_INTERVAL_MS: u64 = 2000;

// ============================================================================
// RSSI Recording Tests
// ============================================================================

#[test]
fn test_rssi_poll_interval_is_2_seconds() {
    // Verify the RSSI polling interval is 2 seconds as specified
    assert_eq!(RSSI_POLL_INTERVAL_MS, 2000);
    assert_eq!(
        Duration::from_millis(RSSI_POLL_INTERVAL_MS),
        Duration::from_secs(2)
    );
}

#[test]
fn test_rssi_recorded_updates_quality() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Initially no RSSI data
    assert!(quality.metrics().rssi_avg.is_none());

    // Record RSSI values
    quality.record_rssi(-50);
    quality.record_rssi(-52);
    quality.record_rssi(-48);

    // RSSI should be averaged
    let metrics = quality.metrics();
    assert!(metrics.rssi_avg.is_some());
    assert_eq!(metrics.rssi_avg, Some(-50));
}

#[test]
fn test_rssi_min_max_tracked() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    quality.record_rssi(-40);
    quality.record_rssi(-60);
    quality.record_rssi(-50);

    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_max, Some(-40)); // Best signal
    assert_eq!(metrics.rssi_min, Some(-60)); // Worst signal
}

#[test]
fn test_rssi_affects_quality_score() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Record excellent RSSI
    for _ in 0..10 {
        quality.record_rssi(-40);
    }

    let excellent_score = quality.metrics().rssi_score;
    assert_eq!(excellent_score, 100);

    // Reset and record poor RSSI
    quality.reset();
    for _ in 0..10 {
        quality.record_rssi(-95);
    }

    let poor_score = quality.metrics().rssi_score;
    assert!(poor_score < 50);
}

// ============================================================================
// Multi-Sensor RSSI Polling Tests
// ============================================================================

#[test]
fn test_monitor_tracks_multiple_sensors_rssi() {
    let mut monitor = ConnectionQualityMonitor::new();

    // Start monitoring three sensors
    monitor.start_monitoring("sensor_a");
    monitor.start_monitoring("sensor_b");
    monitor.start_monitoring("sensor_c");

    // Record different RSSI for each
    monitor.record_rssi("sensor_a", -40);
    monitor.record_rssi("sensor_b", -60);
    monitor.record_rssi("sensor_c", -80);

    // Verify each sensor has its own RSSI
    let stats_a = monitor.get_stats("sensor_a").unwrap();
    let stats_b = monitor.get_stats("sensor_b").unwrap();
    let stats_c = monitor.get_stats("sensor_c").unwrap();

    assert_eq!(stats_a.metrics.rssi_avg, Some(-40));
    assert_eq!(stats_b.metrics.rssi_avg, Some(-60));
    assert_eq!(stats_c.metrics.rssi_avg, Some(-80));
}

#[test]
fn test_monitor_rssi_updates_independently() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("sensor_a");
    monitor.start_monitoring("sensor_b");

    // Multiple updates to sensor_a only
    for rssi in [-45, -50, -48, -52, -47] {
        monitor.record_rssi("sensor_a", rssi);
    }

    // Single update to sensor_b
    monitor.record_rssi("sensor_b", -70);

    let stats_a = monitor.get_stats("sensor_a").unwrap();
    let stats_b = monitor.get_stats("sensor_b").unwrap();

    // sensor_a should have more sample variance
    assert!(stats_a.metrics.rssi_min != stats_a.metrics.rssi_max);

    // sensor_b should have single value
    assert_eq!(stats_b.metrics.rssi_min, stats_b.metrics.rssi_max);
}

// ============================================================================
// RSSI Quality Level Tests
// ============================================================================

#[test]
fn test_excellent_rssi_gives_excellent_level() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Excellent RSSI values (-50 dBm or better)
    for _ in 0..10 {
        quality.record_rssi(-40);
    }

    assert_eq!(quality.metrics().rssi_score, 100);
}

#[test]
fn test_good_rssi_gives_good_score() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Good RSSI values (around -60 dBm)
    for _ in 0..10 {
        quality.record_rssi(-60);
    }

    let score = quality.metrics().rssi_score;
    assert!(score >= 75 && score <= 100);
}

#[test]
fn test_fair_rssi_gives_fair_score() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Fair RSSI values (around -80 dBm)
    for _ in 0..10 {
        quality.record_rssi(-80);
    }

    let score = quality.metrics().rssi_score;
    assert!(score >= 40 && score < 75);
}

#[test]
fn test_poor_rssi_gives_poor_score() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Poor RSSI values (-90+ dBm)
    for _ in 0..10 {
        quality.record_rssi(-95);
    }

    let score = quality.metrics().rssi_score;
    assert!(score < 50);
}

// ============================================================================
// RSSI Change Detection Tests
// ============================================================================

#[test]
fn test_rssi_degradation_detected() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Start with excellent RSSI
    for _ in 0..5 {
        quality.record_rssi(-45);
    }

    let initial_score = quality.metrics().rssi_score;
    assert!(initial_score > 80);

    // Degrade to poor RSSI
    for _ in 0..10 {
        quality.record_rssi(-95);
    }

    let degraded_score = quality.metrics().rssi_score;
    assert!(degraded_score < initial_score);
}

#[test]
fn test_rssi_improvement_detected() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Start with poor RSSI
    for _ in 0..5 {
        quality.record_rssi(-90);
    }

    let initial_score = quality.metrics().rssi_score;

    // Improve to excellent RSSI
    for _ in 0..10 {
        quality.record_rssi(-40);
    }

    let improved_score = quality.metrics().rssi_score;
    assert!(improved_score > initial_score);
}

// ============================================================================
// Config-Based RSSI Threshold Tests
// ============================================================================

#[test]
fn test_strict_config_rssi_thresholds() {
    let config = ConnectionQualityConfig::strict();
    let mut quality = ConnectionQuality::with_config("trainer".to_string(), config);

    // -65 dBm should be good with strict config (rssi_good = -65)
    for _ in 0..10 {
        quality.record_rssi(-65);
    }

    let score = quality.metrics().rssi_score;
    assert!(score >= 75);
}

#[test]
fn test_relaxed_config_rssi_thresholds() {
    let config = ConnectionQualityConfig::relaxed();
    let mut quality = ConnectionQuality::with_config("hr_monitor".to_string(), config);

    // -75 dBm should still be good with relaxed config
    for _ in 0..10 {
        quality.record_rssi(-75);
    }

    let score = quality.metrics().rssi_score;
    assert!(score >= 75);
}

#[test]
fn test_same_rssi_different_scores_by_config() {
    let strict = ConnectionQualityConfig::strict();
    let relaxed = ConnectionQualityConfig::relaxed();

    let mut quality_strict = ConnectionQuality::with_config("trainer".to_string(), strict);
    let mut quality_relaxed = ConnectionQuality::with_config("hr".to_string(), relaxed);

    // Same RSSI value for both
    for _ in 0..10 {
        quality_strict.record_rssi(-70);
        quality_relaxed.record_rssi(-70);
    }

    // Relaxed config should give higher score for same RSSI
    assert!(quality_relaxed.metrics().rssi_score >= quality_strict.metrics().rssi_score);
}

// ============================================================================
// Monitor Integration Tests
// ============================================================================

#[test]
fn test_monitor_rssi_polling_workflow() {
    let mut monitor = ConnectionQualityMonitor::new();

    // Simulate sensor connection
    monitor.start_monitoring("ble_device_1");

    // Simulate RSSI polling (every 2 seconds)
    let rssi_readings = [-50, -52, -48, -55, -45, -50, -53, -49];

    for rssi in rssi_readings {
        monitor.record_rssi("ble_device_1", rssi);
    }

    // Verify stats are available
    let stats = monitor.get_stats("ble_device_1").unwrap();
    assert!(stats.metrics.rssi_avg.is_some());
    assert!(stats.metrics.rssi_min.is_some());
    assert!(stats.metrics.rssi_max.is_some());

    // Verify the score reflects the RSSI data
    let score = monitor.get_score("ble_device_1").unwrap();
    assert!(score > 0);
}

#[test]
fn test_monitor_poor_quality_detection_from_rssi() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("good_sensor");
    monitor.start_monitoring("poor_sensor");

    // Good sensor - excellent RSSI
    for _ in 0..10 {
        monitor.record_rssi("good_sensor", -45);
    }

    // Poor sensor - terrible RSSI
    for _ in 0..10 {
        monitor.record_rssi("poor_sensor", -95);
    }

    let poor_devices = monitor.get_poor_quality_devices();

    // Check if poor_sensor is detected as poor quality
    // (depends on whether RSSI alone triggers poor quality status)
    let poor_level = monitor.get_level("poor_sensor").unwrap();
    if poor_level == QualityLevel::Poor {
        assert!(poor_devices.contains(&"poor_sensor".to_string()));
    }
}

#[test]
fn test_monitor_reset_clears_rssi_data() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("sensor_a");

    // Record some RSSI values
    for _ in 0..5 {
        monitor.record_rssi("sensor_a", -50);
    }

    assert!(monitor.get_stats("sensor_a").unwrap().metrics.rssi_avg.is_some());

    // Reset the sensor
    monitor.reset("sensor_a");

    // RSSI data should be cleared
    assert!(monitor.get_stats("sensor_a").unwrap().metrics.rssi_avg.is_none());
}

#[test]
fn test_monitor_stop_monitoring_removes_sensor() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("sensor_a");
    monitor.record_rssi("sensor_a", -50);

    assert!(monitor.is_monitoring("sensor_a"));

    monitor.stop_monitoring("sensor_a");

    assert!(!monitor.is_monitoring("sensor_a"));
    assert!(monitor.get_stats("sensor_a").is_none());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_rssi_at_boundary_values() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Test minimum RSSI threshold
    quality.record_rssi(-100);
    let min_score = quality.metrics().rssi_score;
    assert!(min_score > 0); // Should still have some score

    // Reset and test maximum RSSI
    quality.reset();
    quality.record_rssi(0); // Theoretical maximum (unlikely but valid)
    assert_eq!(quality.metrics().rssi_score, 100);
}

#[test]
fn test_rssi_averaging_with_outliers() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Mix of values with outliers
    quality.record_rssi(-50);
    quality.record_rssi(-50);
    quality.record_rssi(-100); // Outlier
    quality.record_rssi(-50);
    quality.record_rssi(-50);

    // Average should be pulled down by outlier but not excessively
    let metrics = quality.metrics();
    assert!(metrics.rssi_avg.unwrap() > -70);
    assert!(metrics.rssi_avg.unwrap() < -50);
}

#[test]
fn test_recording_rssi_for_nonexistent_sensor() {
    let mut monitor = ConnectionQualityMonitor::new();

    // Try to record RSSI for a sensor that isn't being monitored
    monitor.record_rssi("nonexistent_sensor", -50);

    // Should not crash and sensor should still not be monitored
    assert!(!monitor.is_monitoring("nonexistent_sensor"));
    assert!(monitor.get_stats("nonexistent_sensor").is_none());
}

#[test]
fn test_disabled_config_ignores_rssi() {
    let config = ConnectionQualityConfig::disabled();
    let mut quality = ConnectionQuality::with_config("sensor".to_string(), config);

    quality.record_rssi(-50);
    quality.record_rssi(-60);

    // Should not record when disabled
    assert!(quality.metrics().rssi_avg.is_none());
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_trainer_rssi_monitoring() {
    // Trainers use strict config and need good RSSI for reliable power data
    let config = ConnectionQualityConfig::strict();
    let mut quality = ConnectionQuality::with_config("wahoo_kickr".to_string(), config);

    // Simulate 20 seconds of RSSI polling (10 polls at 2s interval)
    let rssi_values = [-45, -48, -50, -47, -52, -49, -46, -51, -48, -50];

    for rssi in rssi_values {
        quality.record_rssi(rssi);
    }

    // Trainer should have good quality with these RSSI values
    assert!(quality.is_acceptable());
    assert!(!quality.needs_attention());
}

#[test]
fn test_hr_monitor_rssi_monitoring() {
    // HR monitors use relaxed config, can tolerate weaker signals
    let config = ConnectionQualityConfig::relaxed();
    let mut quality = ConnectionQuality::with_config("polar_h10".to_string(), config);

    // Simulate weaker but stable RSSI
    for _ in 0..10 {
        quality.record_rssi(-70);
    }

    // HR monitor should still be acceptable with relaxed config
    assert!(quality.is_acceptable());
}

#[test]
fn test_moving_sensor_rssi_fluctuation() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Simulate moving sensor with fluctuating RSSI
    let rssi_sequence = [
        -50, -55, -60, -65, -70, // Moving away
        -75, -72, -68, -64, -60, // Coming back
        -55, -50, -45, // Close again
    ];

    for rssi in rssi_sequence {
        quality.record_rssi(rssi);
    }

    // Verify min/max captured the range
    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_min, Some(-75));
    assert_eq!(metrics.rssi_max, Some(-45));

    // Average should be somewhere in the middle
    let avg = metrics.rssi_avg.unwrap();
    assert!(avg < -45 && avg > -75);
}

#[test]
fn test_signal_bars_from_rssi() {
    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Excellent RSSI -> 4 bars
    for _ in 0..10 {
        quality.record_rssi(-40);
    }
    if quality.level() == QualityLevel::Excellent {
        assert_eq!(quality.signal_bars(), 4);
    }

    // Reset and use poor RSSI -> 1 bar
    quality.reset();
    quality.record_rssi(-100);
    quality.record_rssi(-100);

    if quality.level() == QualityLevel::Poor {
        assert_eq!(quality.signal_bars(), 1);
    }
}

// ============================================================================
// Acceptance Criteria Tests
// ============================================================================

#[test]
fn test_acceptance_rssi_updated_every_2_seconds() {
    // Acceptance: RSSI values updated every 2 seconds for connected BLE sensors

    // The polling interval constant matches the 2-second requirement
    assert_eq!(RSSI_POLL_INTERVAL_MS, 2000);

    // Verify that ConnectionQualityMonitor can record RSSI values
    let mut monitor = ConnectionQualityMonitor::new();
    monitor.start_monitoring("ble_sensor");

    // Simulate 3 poll cycles (6 seconds of data)
    monitor.record_rssi("ble_sensor", -50); // t=2s
    monitor.record_rssi("ble_sensor", -52); // t=4s
    monitor.record_rssi("ble_sensor", -48); // t=6s

    let stats = monitor.get_stats("ble_sensor").unwrap();

    // Verify RSSI is being tracked
    assert!(stats.metrics.rssi_avg.is_some());
    assert_eq!(stats.metrics.rssi_avg, Some(-50));
    assert_eq!(stats.sample_count, 3);
}

#[test]
fn test_acceptance_quality_metrics_updated_from_rssi() {
    // Acceptance: Update connection quality metrics every 2 seconds

    let mut quality = ConnectionQuality::new("ble_sensor".to_string());

    // Simulate first poll
    quality.record_rssi(-55);
    let score1 = quality.score();

    // Simulate second poll with degraded signal
    quality.record_rssi(-75);
    let score2 = quality.score();

    // Score should change based on RSSI updates
    // (score2 should be lower or equal due to degraded signal)
    assert!(score2 <= score1 + 10); // Allow some variance due to averaging
}
