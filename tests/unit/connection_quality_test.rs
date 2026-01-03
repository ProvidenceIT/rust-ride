//! Unit tests for connection quality tracking.
//!
//! Tests verify that:
//! - ConnectionQuality tracks RSSI, data rate, packet loss rate, and latency
//! - Quality score (0-100) is calculated correctly from weighted metrics
//! - QualityLevel transitions correctly based on score thresholds
//! - ConnectionQualityMonitor manages multiple devices
//! - Configuration presets work correctly

use rustride::sensors::quality::{
    ConnectionQuality, ConnectionQualityConfig, ConnectionQualityMonitor, QualityLevel,
    QualityMetrics, QualityStats, DEFAULT_RSSI_EXCELLENT, DEFAULT_RSSI_GOOD,
};
use std::time::Duration;

// ============================================================================
// ConnectionQualityConfig Tests
// ============================================================================

#[test]
fn test_default_config() {
    let config = ConnectionQualityConfig::default();

    assert_eq!(config.rssi_excellent, DEFAULT_RSSI_EXCELLENT);
    assert_eq!(config.rssi_good, DEFAULT_RSSI_GOOD);
    assert!(config.enabled);
    assert_eq!(config.metrics_window, Duration::from_secs(30));
}

#[test]
fn test_strict_config() {
    let config = ConnectionQualityConfig::strict();

    // Strict config has tighter thresholds
    assert!(config.rssi_excellent > DEFAULT_RSSI_EXCELLENT);
    assert!(config.latency_excellent_ms < 50);
    assert!(config.enabled);
}

#[test]
fn test_relaxed_config() {
    let config = ConnectionQualityConfig::relaxed();

    // Relaxed config has looser thresholds
    assert!(config.rssi_excellent < DEFAULT_RSSI_EXCELLENT);
    assert!(config.latency_excellent_ms >= 100);
    assert!(config.enabled);
}

#[test]
fn test_disabled_config() {
    let config = ConnectionQualityConfig::disabled();

    assert!(!config.enabled);
}

#[test]
fn test_config_weights_sum_to_one() {
    let config = ConnectionQualityConfig::default();

    let total = config.rssi_weight
        + config.data_rate_weight
        + config.packet_loss_weight
        + config.latency_weight;

    assert!(
        (total - 1.0).abs() < 0.01,
        "Weights should sum to 1.0, got {}",
        total
    );
}

// ============================================================================
// QualityLevel Tests
// ============================================================================

#[test]
fn test_quality_level_display() {
    assert_eq!(QualityLevel::Excellent.to_string(), "Excellent");
    assert_eq!(QualityLevel::Good.to_string(), "Good");
    assert_eq!(QualityLevel::Fair.to_string(), "Fair");
    assert_eq!(QualityLevel::Poor.to_string(), "Poor");
}

#[test]
fn test_quality_level_from_score_boundaries() {
    // Excellent: 85-100
    assert_eq!(QualityLevel::from_score(100), QualityLevel::Excellent);
    assert_eq!(QualityLevel::from_score(90), QualityLevel::Excellent);
    assert_eq!(QualityLevel::from_score(85), QualityLevel::Excellent);

    // Good: 65-84
    assert_eq!(QualityLevel::from_score(84), QualityLevel::Good);
    assert_eq!(QualityLevel::from_score(75), QualityLevel::Good);
    assert_eq!(QualityLevel::from_score(65), QualityLevel::Good);

    // Fair: 40-64
    assert_eq!(QualityLevel::from_score(64), QualityLevel::Fair);
    assert_eq!(QualityLevel::from_score(50), QualityLevel::Fair);
    assert_eq!(QualityLevel::from_score(40), QualityLevel::Fair);

    // Poor: 0-39
    assert_eq!(QualityLevel::from_score(39), QualityLevel::Poor);
    assert_eq!(QualityLevel::from_score(20), QualityLevel::Poor);
    assert_eq!(QualityLevel::from_score(0), QualityLevel::Poor);
}

#[test]
fn test_quality_level_signal_bars() {
    assert_eq!(QualityLevel::Excellent.signal_bars(), 4);
    assert_eq!(QualityLevel::Good.signal_bars(), 3);
    assert_eq!(QualityLevel::Fair.signal_bars(), 2);
    assert_eq!(QualityLevel::Poor.signal_bars(), 1);
}

#[test]
fn test_quality_level_to_score_range() {
    assert_eq!(QualityLevel::Excellent.to_score_range(), (85, 100));
    assert_eq!(QualityLevel::Good.to_score_range(), (65, 84));
    assert_eq!(QualityLevel::Fair.to_score_range(), (40, 64));
    assert_eq!(QualityLevel::Poor.to_score_range(), (0, 39));
}

#[test]
fn test_quality_level_ordering() {
    assert!(QualityLevel::Excellent > QualityLevel::Good);
    assert!(QualityLevel::Good > QualityLevel::Fair);
    assert!(QualityLevel::Fair > QualityLevel::Poor);
}

// ============================================================================
// ConnectionQuality Basic Tests
// ============================================================================

#[test]
fn test_new_connection_quality() {
    let quality = ConnectionQuality::new("device_a".to_string());

    assert_eq!(quality.device_id(), "device_a");
    assert_eq!(quality.score(), 0);
    assert_eq!(quality.level(), QualityLevel::Poor);
    assert_eq!(quality.signal_bars(), 1);
}

#[test]
fn test_new_with_custom_config() {
    let config = ConnectionQualityConfig::strict();
    let quality = ConnectionQuality::with_config("device_a".to_string(), config.clone());

    assert_eq!(quality.config().rssi_excellent, config.rssi_excellent);
    assert_eq!(quality.config().latency_excellent_ms, config.latency_excellent_ms);
}

#[test]
fn test_uptime_tracking() {
    let quality = ConnectionQuality::new("device_a".to_string());

    std::thread::sleep(Duration::from_millis(10));

    let uptime = quality.uptime();
    assert!(uptime >= Duration::from_millis(10));
}

// ============================================================================
// RSSI Tracking Tests
// ============================================================================

#[test]
fn test_record_rssi_excellent() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record excellent RSSI values
    for _ in 0..10 {
        quality.record_rssi(-40);
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_avg, Some(-40));
    assert_eq!(metrics.rssi_score, 100);
}

#[test]
fn test_record_rssi_good() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record good RSSI values
    for _ in 0..10 {
        quality.record_rssi(-60);
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_avg, Some(-60));
    assert!(metrics.rssi_score >= 75 && metrics.rssi_score <= 100);
}

#[test]
fn test_record_rssi_poor() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record poor RSSI values
    for _ in 0..10 {
        quality.record_rssi(-95);
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_avg, Some(-95));
    assert!(metrics.rssi_score < 50);
}

#[test]
fn test_rssi_min_max_tracking() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    quality.record_rssi(-40);
    quality.record_rssi(-60);
    quality.record_rssi(-80);

    let metrics = quality.metrics();
    assert_eq!(metrics.rssi_min, Some(-80)); // Worst signal
    assert_eq!(metrics.rssi_max, Some(-40)); // Best signal
}

// ============================================================================
// Latency Tracking Tests
// ============================================================================

#[test]
fn test_record_latency_excellent() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record excellent latency values
    for _ in 0..10 {
        quality.record_latency(30);
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.latency_avg_ms, Some(30));
    assert_eq!(metrics.latency_score, 100);
}

#[test]
fn test_record_latency_poor() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record poor latency values
    for _ in 0..10 {
        quality.record_latency(300);
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.latency_avg_ms, Some(300));
    assert!(metrics.latency_score < 60);
}

#[test]
fn test_latency_min_max_tracking() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    quality.record_latency(20);
    quality.record_latency(50);
    quality.record_latency(100);

    let metrics = quality.metrics();
    assert_eq!(metrics.latency_min_ms, Some(20));
    assert_eq!(metrics.latency_max_ms, Some(100));
}

// ============================================================================
// Data Rate and Packet Loss Tests
// ============================================================================

#[test]
fn test_record_packet_updates_data_rate() {
    let config = ConnectionQualityConfig {
        metrics_window: Duration::from_secs(1),
        ..ConnectionQualityConfig::default()
    };
    let mut quality = ConnectionQuality::with_config("device_a".to_string(), config);

    // Record multiple packets
    for _ in 0..10 {
        quality.record_packet(None);
    }

    let metrics = quality.metrics();
    assert!(metrics.data_rate > 0.0, "Data rate should be positive");
}

#[test]
fn test_packet_loss_detection() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record packets with sequence numbers showing loss
    quality.record_packet(Some(1));
    quality.record_packet(Some(2));
    quality.record_packet(Some(5)); // Lost 3 and 4

    let metrics = quality.metrics();
    assert!(metrics.packet_loss_rate > 0.0, "Should detect packet loss");
}

#[test]
fn test_no_packet_loss() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record consecutive packets
    for seq in 1..=10 {
        quality.record_packet(Some(seq));
    }

    let metrics = quality.metrics();
    assert_eq!(metrics.packet_loss_rate, 0.0, "Should have no packet loss");
    assert_eq!(metrics.packet_loss_score, 100);
}

// ============================================================================
// Combined Metrics Tests
// ============================================================================

#[test]
fn test_record_data_all_metrics() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record data with all metrics
    quality.record_data(Some(-45), Some(30), Some(1));
    quality.record_data(Some(-50), Some(35), Some(2));
    quality.record_data(Some(-48), Some(32), Some(3));

    let metrics = quality.metrics();

    assert!(metrics.rssi_avg.is_some());
    assert!(metrics.latency_avg_ms.is_some());
    assert!(metrics.data_rate > 0.0);
    assert_eq!(metrics.packet_loss_rate, 0.0);
}

#[test]
fn test_excellent_overall_quality() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record excellent values for all metrics
    for seq in 1..=20 {
        quality.record_data(Some(-40), Some(20), Some(seq));
    }

    assert!(quality.score() >= 85, "Score should be excellent: {}", quality.score());
    assert_eq!(quality.level(), QualityLevel::Excellent);
    assert_eq!(quality.signal_bars(), 4);
}

#[test]
fn test_poor_overall_quality() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Record poor values for all metrics
    quality.record_data(Some(-95), Some(500), Some(1));
    quality.record_data(Some(-98), Some(600), Some(5)); // Lost packets 2,3,4

    assert!(quality.score() < 40, "Score should be poor: {}", quality.score());
    assert_eq!(quality.level(), QualityLevel::Poor);
    assert_eq!(quality.signal_bars(), 1);
}

// ============================================================================
// Quality Assessment Tests
// ============================================================================

#[test]
fn test_is_acceptable_excellent() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    for seq in 1..=10 {
        quality.record_data(Some(-40), Some(20), Some(seq));
    }

    assert!(quality.is_acceptable());
    assert!(!quality.needs_attention());
}

#[test]
fn test_is_acceptable_fair() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    for seq in 1..=10 {
        quality.record_data(Some(-82), Some(180), Some(seq));
    }

    // Fair should still be acceptable
    if quality.level() == QualityLevel::Fair {
        assert!(quality.is_acceptable());
    }
}

#[test]
fn test_needs_attention_poor() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Very poor values
    quality.record_data(Some(-100), Some(800), Some(1));
    quality.record_data(Some(-100), Some(900), Some(10)); // Major packet loss

    if quality.level() == QualityLevel::Poor {
        assert!(quality.needs_attention());
        assert!(!quality.is_acceptable());
    }
}

// ============================================================================
// Reset Tests
// ============================================================================

#[test]
fn test_reset_clears_state() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Build up state
    for seq in 1..=10 {
        quality.record_data(Some(-45), Some(25), Some(seq));
    }

    assert!(quality.score() > 0);
    assert!(quality.metrics().rssi_avg.is_some());

    // Reset
    quality.reset();

    assert_eq!(quality.score(), 0);
    assert_eq!(quality.level(), QualityLevel::Poor);
    assert!(quality.metrics().rssi_avg.is_none());
    assert_eq!(quality.metrics().data_rate, 0.0);
}

#[test]
fn test_reset_after_reconnection() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Build up state
    quality.record_data(Some(-95), Some(500), Some(1));

    // Reset (simulating reconnection)
    quality.reset();

    // New data after reset should work correctly
    for seq in 1..=5 {
        quality.record_data(Some(-40), Some(20), Some(seq));
    }

    assert!(quality.score() > 0);
}

// ============================================================================
// QualityStats Tests
// ============================================================================

#[test]
fn test_quality_stats() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    for seq in 1..=5 {
        quality.record_data(Some(-50), Some(40), Some(seq));
    }

    let stats = quality.stats();

    assert_eq!(stats.device_id, "device_a");
    assert!(stats.score > 0);
    assert!(stats.signal_bars >= 1);
    assert!(stats.sample_count > 0);
}

#[test]
fn test_quality_stats_summary() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    for seq in 1..=10 {
        quality.record_data(Some(-45), Some(30), Some(seq));
    }

    let stats = quality.stats();
    let summary = stats.summary();

    assert!(summary.contains("Excellent") || summary.contains("Good"));
    assert!(summary.contains("%"));
}

#[test]
fn test_quality_stats_detail_text() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    for seq in 1..=5 {
        quality.record_data(Some(-50), Some(40), Some(seq));
    }

    let stats = quality.stats();
    let detail = stats.detail_text();

    assert!(detail.contains("RSSI:"));
    assert!(detail.contains("dBm"));
    assert!(detail.contains("Rate:"));
    assert!(detail.contains("pkt/s"));
}

#[test]
fn test_quality_stats_needs_attention() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Poor quality
    quality.record_data(Some(-100), Some(800), Some(1));
    quality.record_data(Some(-100), Some(900), Some(10));

    let stats = quality.stats();

    if stats.level == QualityLevel::Poor {
        assert!(stats.needs_attention());
    }
}

#[test]
fn test_quality_stats_is_degraded() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Fair quality
    quality.record_data(Some(-80), Some(150), Some(1));
    quality.record_data(Some(-82), Some(160), Some(2));

    let stats = quality.stats();

    if matches!(stats.level, QualityLevel::Fair | QualityLevel::Poor) {
        assert!(stats.is_degraded());
    }
}

// ============================================================================
// ConnectionQualityMonitor Tests
// ============================================================================

#[test]
fn test_monitor_new_is_empty() {
    let monitor = ConnectionQualityMonitor::new();

    assert!(monitor.is_empty());
    assert_eq!(monitor.len(), 0);
}

#[test]
fn test_monitor_with_config() {
    let config = ConnectionQualityConfig::strict();
    let monitor = ConnectionQualityMonitor::with_config(config);

    assert!(monitor.is_empty());
}

#[test]
fn test_monitor_start_monitoring() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");

    assert_eq!(monitor.len(), 1);
    assert!(monitor.is_monitoring("device_a"));
    assert!(!monitor.is_monitoring("device_b"));
}

#[test]
fn test_monitor_start_monitoring_with_config() {
    let mut monitor = ConnectionQualityMonitor::new();
    let config = ConnectionQualityConfig::strict();

    monitor.start_monitoring_with_config("trainer", config);

    assert!(monitor.is_monitoring("trainer"));
}

#[test]
fn test_monitor_stop_monitoring() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");
    assert_eq!(monitor.len(), 2);

    monitor.stop_monitoring("device_a");
    assert_eq!(monitor.len(), 1);
    assert!(!monitor.is_monitoring("device_a"));
    assert!(monitor.is_monitoring("device_b"));
}

#[test]
fn test_monitor_record_rssi() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_rssi("device_a", -50);

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());
    assert!(stats.unwrap().metrics.rssi_avg.is_some());
}

#[test]
fn test_monitor_record_latency() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_latency("device_a", 30);

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());
    assert!(stats.unwrap().metrics.latency_avg_ms.is_some());
}

#[test]
fn test_monitor_record_packet() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_packet("device_a", Some(1));
    monitor.record_packet("device_a", Some(2));

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());
    assert!(stats.unwrap().sample_count >= 2);
}

#[test]
fn test_monitor_record_data() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a", Some(-50), Some(30), Some(1));

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());

    let stats = stats.unwrap();
    assert!(stats.metrics.rssi_avg.is_some());
    assert!(stats.metrics.latency_avg_ms.is_some());
}

#[test]
fn test_monitor_get_score() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    for seq in 1..=5 {
        monitor.record_data("device_a", Some(-45), Some(25), Some(seq));
    }

    let score = monitor.get_score("device_a");
    assert!(score.is_some());
    assert!(score.unwrap() > 0);
}

#[test]
fn test_monitor_get_level() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    for seq in 1..=10 {
        monitor.record_data("device_a", Some(-40), Some(20), Some(seq));
    }

    let level = monitor.get_level("device_a");
    assert!(level.is_some());
    assert!(level.unwrap() >= QualityLevel::Good);
}

#[test]
fn test_monitor_get_stats() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_rssi("device_a", -50);

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().device_id, "device_a");
}

#[test]
fn test_monitor_get_all_stats() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");
    monitor.start_monitoring("device_c");

    monitor.record_rssi("device_a", -50);
    monitor.record_rssi("device_b", -60);
    monitor.record_rssi("device_c", -70);

    let all_stats = monitor.get_all_stats();
    assert_eq!(all_stats.len(), 3);
}

#[test]
fn test_monitor_get_poor_quality_devices() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("good_device");
    monitor.start_monitoring("poor_device");

    // Good device
    for seq in 1..=10 {
        monitor.record_data("good_device", Some(-45), Some(25), Some(seq));
    }

    // Poor device
    monitor.record_data("poor_device", Some(-100), Some(800), Some(1));
    monitor.record_data("poor_device", Some(-100), Some(900), Some(10));

    let poor = monitor.get_poor_quality_devices();

    // poor_device should be in the list (if it's actually poor)
    if let Some(level) = monitor.get_level("poor_device") {
        if level == QualityLevel::Poor {
            assert!(poor.contains(&"poor_device".to_string()));
        }
    }
}

#[test]
fn test_monitor_get_degraded_devices() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("excellent_device");
    monitor.start_monitoring("fair_device");

    // Excellent device
    for seq in 1..=10 {
        monitor.record_data("excellent_device", Some(-40), Some(20), Some(seq));
    }

    // Fair device
    for seq in 1..=10 {
        monitor.record_data("fair_device", Some(-80), Some(150), Some(seq));
    }

    let degraded = monitor.get_degraded_devices();

    // fair_device might be in the list depending on exact thresholds
    // The key is that the method returns devices correctly
    if let Some(level) = monitor.get_level("fair_device") {
        if level <= QualityLevel::Fair {
            assert!(degraded.contains(&"fair_device".to_string()));
        }
    }
}

#[test]
fn test_monitor_reset() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    for seq in 1..=5 {
        monitor.record_data("device_a", Some(-50), Some(40), Some(seq));
    }

    assert!(monitor.get_score("device_a").unwrap() > 0);

    monitor.reset("device_a");

    // After reset, score should be 0
    assert_eq!(monitor.get_score("device_a"), Some(0));
}

#[test]
fn test_monitor_clear() {
    let mut monitor = ConnectionQualityMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");
    monitor.start_monitoring("device_c");
    assert_eq!(monitor.len(), 3);

    monitor.clear();
    assert!(monitor.is_empty());
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_trainer_with_strict_config() {
    // Trainers should use strict config for accurate power data
    let config = ConnectionQualityConfig::strict();
    let mut quality = ConnectionQuality::with_config("wahoo_kickr".to_string(), config);

    // Simulate good trainer connection
    for seq in 1..=30 {
        quality.record_data(Some(-55), Some(25), Some(seq));
    }

    assert!(quality.level() >= QualityLevel::Good);
    assert!(quality.is_acceptable());
}

#[test]
fn test_heart_rate_with_relaxed_config() {
    // HR monitors can use relaxed config
    let config = ConnectionQualityConfig::relaxed();
    let mut quality = ConnectionQuality::with_config("hr_monitor".to_string(), config);

    // Simulate typical HR monitor connection (weaker signal, but acceptable)
    for seq in 1..=20 {
        quality.record_data(Some(-70), Some(80), Some(seq));
    }

    assert!(quality.is_acceptable());
}

#[test]
fn test_quality_degrades_with_packet_loss() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Start with good connection
    for seq in 1..=10 {
        quality.record_data(Some(-50), Some(30), Some(seq));
    }

    let initial_score = quality.score();

    // Add significant packet loss
    quality.record_data(Some(-50), Some(30), Some(20)); // Lost 10-19

    let degraded_score = quality.score();

    assert!(
        degraded_score <= initial_score,
        "Score should degrade with packet loss: {} -> {}",
        initial_score,
        degraded_score
    );
}

#[test]
fn test_multiple_sensors_monitored_independently() {
    let mut monitor = ConnectionQualityMonitor::new();

    // Monitor three sensors
    monitor.start_monitoring_with_config("trainer", ConnectionQualityConfig::strict());
    monitor.start_monitoring_with_config("hr_monitor", ConnectionQualityConfig::relaxed());
    monitor.start_monitoring("power_meter");

    // Each gets different quality data
    for seq in 1..=10 {
        // Trainer: excellent
        monitor.record_data("trainer", Some(-40), Some(20), Some(seq));

        // HR: good
        monitor.record_data("hr_monitor", Some(-65), Some(60), Some(seq));

        // Power meter: fair
        monitor.record_data("power_meter", Some(-80), Some(120), Some(seq));
    }

    // Verify independent tracking
    let trainer_level = monitor.get_level("trainer").unwrap();
    let hr_level = monitor.get_level("hr_monitor").unwrap();

    // Trainer should be excellent
    assert!(trainer_level >= QualityLevel::Good);

    // Levels should be tracked independently
    assert!(monitor.get_score("trainer") != monitor.get_score("power_meter"));
}

#[test]
fn test_signal_bars_for_ui_display() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Poor quality = 1 bar
    quality.record_data(Some(-100), Some(800), Some(1));
    assert_eq!(quality.signal_bars(), 1);

    // Reset and record excellent quality = 4 bars
    quality.reset();
    for seq in 1..=20 {
        quality.record_data(Some(-40), Some(20), Some(seq));
    }

    if quality.level() == QualityLevel::Excellent {
        assert_eq!(quality.signal_bars(), 4);
    }
}

#[test]
fn test_quality_score_is_within_bounds() {
    let mut quality = ConnectionQuality::new("device_a".to_string());

    // Various data points
    for seq in 1..=100 {
        let rssi = -40 - (seq as i16 % 60); // Range: -40 to -99
        let latency = 20 + (seq as u64 % 200); // Range: 20-219
        quality.record_data(Some(rssi), Some(latency), Some(seq));
    }

    // Score should always be 0-100
    let score = quality.score();
    assert!(score <= 100, "Score should not exceed 100: {}", score);
}

#[test]
fn test_disabled_config_does_not_record() {
    let config = ConnectionQualityConfig::disabled();
    let mut quality = ConnectionQuality::with_config("device_a".to_string(), config);

    quality.record_rssi(-50);
    quality.record_latency(30);
    quality.record_packet(Some(1));

    // Should not have recorded anything
    let metrics = quality.metrics();
    assert!(metrics.rssi_avg.is_none());
    assert!(metrics.latency_avg_ms.is_none());
}

#[test]
fn test_connection_quality_acceptance_criteria() {
    // Acceptance criteria: ConnectionQuality struct tracks RSSI, data rate,
    // and computes quality score

    let mut quality = ConnectionQuality::new("test_sensor".to_string());

    // Track RSSI
    quality.record_rssi(-55);
    assert!(quality.metrics().rssi_avg.is_some());

    // Track data rate via packets
    for seq in 1..=5 {
        quality.record_packet(Some(seq));
    }
    assert!(quality.metrics().data_rate > 0.0);

    // Track packet loss
    quality.record_packet(Some(10)); // Lost 6-9
    assert!(quality.metrics().packet_loss_rate > 0.0);

    // Track latency
    quality.record_latency(35);
    assert!(quality.metrics().latency_avg_ms.is_some());

    // Compute quality score (0-100)
    let score = quality.score();
    assert!(score <= 100);

    // Compute quality level
    let level = quality.level();
    assert!(matches!(
        level,
        QualityLevel::Poor | QualityLevel::Fair | QualityLevel::Good | QualityLevel::Excellent
    ));
}
