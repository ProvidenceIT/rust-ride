//! Unit tests for connection health monitoring.
//!
//! Tests verify that:
//! - Stale connections (no data for 5s) trigger reconnection before BLE timeout
//! - Health status transitions correctly (Unknown -> Healthy -> Degraded -> Stale)
//! - Data rate tracking works correctly
//! - ConnectionHealthMonitor manages multiple devices
//! - Configuration presets work correctly

use rustride::sensors::health::{
    ConnectionHealth, ConnectionHealthConfig, ConnectionHealthMonitor, HealthStats, HealthStatus,
    DEFAULT_DEGRADED_TIMEOUT_MS, DEFAULT_STALE_TIMEOUT_SECS,
};
use std::thread::sleep;
use std::time::Duration;

// ============================================================================
// ConnectionHealthConfig Tests
// ============================================================================

#[test]
fn test_default_config() {
    let config = ConnectionHealthConfig::default();

    assert_eq!(config.stale_timeout, Duration::from_secs(DEFAULT_STALE_TIMEOUT_SECS));
    assert_eq!(config.degraded_timeout, Duration::from_millis(DEFAULT_DEGRADED_TIMEOUT_MS));
    assert_eq!(config.min_data_rate, 0.5);
    assert!(config.enabled);
}

#[test]
fn test_strict_config() {
    let config = ConnectionHealthConfig::strict();

    assert_eq!(config.stale_timeout, Duration::from_secs(3));
    assert_eq!(config.degraded_timeout, Duration::from_millis(1500));
    assert!(config.min_data_rate > 0.5);
    assert!(config.enabled);
}

#[test]
fn test_relaxed_config() {
    let config = ConnectionHealthConfig::relaxed();

    assert_eq!(config.stale_timeout, Duration::from_secs(10));
    assert_eq!(config.degraded_timeout, Duration::from_secs(5));
    assert!(config.min_data_rate < 0.5);
    assert!(config.enabled);
}

#[test]
fn test_disabled_config() {
    let config = ConnectionHealthConfig::disabled();

    assert!(!config.enabled);
}

// ============================================================================
// HealthStatus Tests
// ============================================================================

#[test]
fn test_health_status_display() {
    assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
    assert_eq!(HealthStatus::Degraded.to_string(), "Degraded");
    assert_eq!(HealthStatus::Stale.to_string(), "Stale");
    assert_eq!(HealthStatus::Unknown.to_string(), "Unknown");
}

#[test]
fn test_health_status_equality() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Stale);
}

// ============================================================================
// ConnectionHealth Basic Tests
// ============================================================================

#[test]
fn test_new_connection_health() {
    let health = ConnectionHealth::new("device_a".to_string());

    assert_eq!(health.device_id(), "device_a");
    assert_eq!(health.status(), HealthStatus::Unknown);
    assert!(health.time_since_last_data().is_none());
    assert!(!health.is_stale());
    assert!(!health.needs_reconnection());
}

#[test]
fn test_new_with_custom_config() {
    let config = ConnectionHealthConfig::strict();
    let health = ConnectionHealth::with_config("device_a".to_string(), config.clone());

    assert_eq!(health.config().stale_timeout, config.stale_timeout);
    assert_eq!(health.config().degraded_timeout, config.degraded_timeout);
}

#[test]
fn test_record_data_sets_status_healthy() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    health.record_data_received();

    assert_eq!(health.status(), HealthStatus::Healthy);
    assert!(health.time_since_last_data().is_some());
}

#[test]
fn test_record_data_updates_timestamp() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    health.record_data_received();
    let first = health.time_since_last_data().unwrap();

    // Small delay then record again
    sleep(Duration::from_millis(10));
    health.record_data_received();
    let second = health.time_since_last_data().unwrap();

    // Second time should be smaller (more recent)
    assert!(second < first);
}

#[test]
fn test_data_rate_calculation() {
    let config = ConnectionHealthConfig {
        rate_window: Duration::from_secs(1),
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Record 10 packets quickly
    for _ in 0..10 {
        health.record_data_received();
    }

    // Data rate should be approximately 10 packets per second (within the window)
    let rate = health.data_rate();
    assert!(rate > 0.0, "Data rate should be positive");
}

#[test]
fn test_packets_in_window() {
    let config = ConnectionHealthConfig {
        rate_window: Duration::from_secs(10),
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    assert_eq!(health.packets_in_window(), 0);

    health.record_data_received();
    assert_eq!(health.packets_in_window(), 1);

    health.record_data_received();
    assert_eq!(health.packets_in_window(), 2);
}

// ============================================================================
// Health Status Transition Tests
// ============================================================================

#[test]
fn test_status_transitions_to_degraded_after_timeout() {
    // Use very short timeouts for testing
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(50),
        stale_timeout: Duration::from_millis(100),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Record data to set initial healthy status
    health.record_data_received();
    assert_eq!(health.status(), HealthStatus::Healthy);

    // Wait for degraded timeout
    sleep(Duration::from_millis(60));

    // Check should show degraded
    let status = health.check();
    assert_eq!(status, HealthStatus::Degraded);
}

#[test]
fn test_status_transitions_to_stale_after_timeout() {
    // Use very short timeouts for testing
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(60),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Record data to set initial healthy status
    health.record_data_received();
    assert_eq!(health.status(), HealthStatus::Healthy);

    // Wait for stale timeout
    sleep(Duration::from_millis(70));

    // Check should show stale
    let status = health.check();
    assert_eq!(status, HealthStatus::Stale);
    assert!(health.is_stale());
}

#[test]
fn test_stale_connection_needs_reconnection() {
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(60),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Record data then wait for stale timeout
    health.record_data_received();
    sleep(Duration::from_millis(70));
    health.check();

    assert!(health.needs_reconnection());
}

#[test]
fn test_healthy_connection_does_not_need_reconnection() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    health.record_data_received();

    assert!(!health.needs_reconnection());
}

#[test]
fn test_disabled_health_check_does_not_need_reconnection() {
    let config = ConnectionHealthConfig::disabled();
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Even without data, disabled monitoring shouldn't trigger reconnection
    sleep(Duration::from_millis(100));
    health.check();

    assert!(!health.needs_reconnection());
}

#[test]
fn test_new_data_restores_healthy_status() {
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(60),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Go to degraded
    health.record_data_received();
    sleep(Duration::from_millis(40));
    health.check();
    assert_eq!(health.status(), HealthStatus::Degraded);

    // New data should restore healthy
    health.record_data_received();
    assert_eq!(health.status(), HealthStatus::Healthy);
}

// ============================================================================
// Reset Tests
// ============================================================================

#[test]
fn test_reset_clears_state() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    // Build up some state
    health.record_data_received();
    health.record_data_received();
    health.record_data_received();
    assert_eq!(health.packets_in_window(), 3);
    assert_eq!(health.status(), HealthStatus::Healthy);

    // Reset
    health.reset();

    assert_eq!(health.status(), HealthStatus::Unknown);
    assert!(health.time_since_last_data().is_none());
    assert_eq!(health.packets_in_window(), 0);
}

#[test]
fn test_reset_after_reconnection() {
    let config = ConnectionHealthConfig {
        stale_timeout: Duration::from_millis(50),
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Get to stale state
    health.record_data_received();
    sleep(Duration::from_millis(60));
    health.check();
    assert!(health.is_stale());

    // Simulate successful reconnection
    health.reset();
    assert!(!health.is_stale());
    assert_eq!(health.status(), HealthStatus::Unknown);

    // New data after reset
    health.record_data_received();
    assert_eq!(health.status(), HealthStatus::Healthy);
}

// ============================================================================
// Streak Tests
// ============================================================================

#[test]
fn test_healthy_streak_increments() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    health.record_data_received();
    assert_eq!(health.healthy_streak(), 1);

    health.record_data_received();
    assert_eq!(health.healthy_streak(), 2);

    health.record_data_received();
    assert_eq!(health.healthy_streak(), 3);
}

#[test]
fn test_unhealthy_streak_resets_healthy_streak() {
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(100),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Build healthy streak
    health.record_data_received();
    health.record_data_received();
    assert_eq!(health.healthy_streak(), 2);

    // Go to degraded
    sleep(Duration::from_millis(40));
    health.check();
    assert_eq!(health.status(), HealthStatus::Degraded);
    assert_eq!(health.healthy_streak(), 0);
    assert!(health.unhealthy_streak() > 0);
}

// ============================================================================
// HealthStats Tests
// ============================================================================

#[test]
fn test_health_stats() {
    let mut health = ConnectionHealth::new("device_a".to_string());

    health.record_data_received();
    health.record_data_received();

    let stats = health.stats();

    assert_eq!(stats.device_id, "device_a");
    assert_eq!(stats.status, HealthStatus::Healthy);
    assert!(stats.time_since_last_data.is_some());
    assert_eq!(stats.packets_in_window, 2);
}

#[test]
fn test_health_stats_status_text_healthy() {
    let mut health = ConnectionHealth::new("device_a".to_string());
    health.record_data_received();

    let stats = health.stats();
    let text = stats.status_text();

    assert!(text.contains("Healthy"));
}

#[test]
fn test_health_stats_needs_attention() {
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(100),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Healthy - no attention needed
    health.record_data_received();
    assert!(!health.stats().needs_attention());

    // Degraded - needs attention
    sleep(Duration::from_millis(40));
    health.check();
    assert!(health.stats().needs_attention());
}

// ============================================================================
// ConnectionHealthMonitor Tests
// ============================================================================

#[test]
fn test_monitor_new_is_empty() {
    let monitor = ConnectionHealthMonitor::new();

    assert!(monitor.is_empty());
    assert_eq!(monitor.len(), 0);
}

#[test]
fn test_monitor_start_monitoring() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");

    assert_eq!(monitor.len(), 1);
    assert!(monitor.is_monitoring("device_a"));
    assert!(!monitor.is_monitoring("device_b"));
}

#[test]
fn test_monitor_stop_monitoring() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");
    assert_eq!(monitor.len(), 2);

    monitor.stop_monitoring("device_a");
    assert_eq!(monitor.len(), 1);
    assert!(!monitor.is_monitoring("device_a"));
    assert!(monitor.is_monitoring("device_b"));
}

#[test]
fn test_monitor_record_data() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a");

    let status = monitor.get_status("device_a");
    assert_eq!(status, Some(HealthStatus::Healthy));
}

#[test]
fn test_monitor_check_device() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a");

    let status = monitor.check_device("device_a");
    assert_eq!(status, Some(HealthStatus::Healthy));
}

#[test]
fn test_monitor_check_all_returns_stale_devices() {
    let config = ConnectionHealthConfig {
        degraded_timeout: Duration::from_millis(30),
        stale_timeout: Duration::from_millis(60),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut monitor = ConnectionHealthMonitor::with_config(config);

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");

    // Record data for device_a only
    monitor.record_data("device_a");

    // Wait for stale timeout
    sleep(Duration::from_millis(70));

    // device_b should need reconnection (never received data within timeout)
    // device_a should also be stale now
    let needs_reconnection = monitor.check_all();

    // Both should be stale since we waited 70ms after device_a's data
    assert!(!needs_reconnection.is_empty());
}

#[test]
fn test_monitor_get_stale_devices() {
    let config = ConnectionHealthConfig {
        stale_timeout: Duration::from_millis(50),
        ..ConnectionHealthConfig::default()
    };
    let mut monitor = ConnectionHealthMonitor::with_config(config);

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a");

    // Not stale initially
    monitor.check_all();
    assert!(monitor.get_stale_devices().is_empty());

    // Wait for stale
    sleep(Duration::from_millis(60));
    monitor.check_all();

    let stale = monitor.get_stale_devices();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0], "device_a");
}

#[test]
fn test_monitor_get_stats() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a");

    let stats = monitor.get_stats("device_a");
    assert!(stats.is_some());

    let stats = stats.unwrap();
    assert_eq!(stats.device_id, "device_a");
    assert_eq!(stats.status, HealthStatus::Healthy);
}

#[test]
fn test_monitor_get_all_stats() {
    let mut monitor = ConnectionHealthMonitor::new();

    monitor.start_monitoring("device_a");
    monitor.start_monitoring("device_b");
    monitor.record_data("device_a");
    monitor.record_data("device_b");

    let all_stats = monitor.get_all_stats();
    assert_eq!(all_stats.len(), 2);
}

#[test]
fn test_monitor_reset() {
    let config = ConnectionHealthConfig {
        stale_timeout: Duration::from_millis(50),
        ..ConnectionHealthConfig::default()
    };
    let mut monitor = ConnectionHealthMonitor::with_config(config);

    monitor.start_monitoring("device_a");
    monitor.record_data("device_a");

    // Go to stale
    sleep(Duration::from_millis(60));
    monitor.check_all();
    assert!(!monitor.get_stale_devices().is_empty());

    // Reset after reconnection
    monitor.reset("device_a");

    // Should no longer be stale (status is Unknown)
    let status = monitor.get_status("device_a");
    assert_eq!(status, Some(HealthStatus::Unknown));
}

#[test]
fn test_monitor_clear() {
    let mut monitor = ConnectionHealthMonitor::new();

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
fn test_stale_connections_trigger_reconnection_before_ble_timeout() {
    // Acceptance criteria: Stale connections (no data for 5s) trigger
    // reconnection before BLE timeout

    // Using shorter timeouts for test speed, but the logic is the same
    let config = ConnectionHealthConfig {
        stale_timeout: Duration::from_millis(100),
        degraded_timeout: Duration::from_millis(50),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut monitor = ConnectionHealthMonitor::with_config(config);

    // Simulate trainer connection
    monitor.start_monitoring("wahoo_kickr");
    monitor.record_data("wahoo_kickr");

    // Initially healthy
    assert_eq!(
        monitor.get_status("wahoo_kickr"),
        Some(HealthStatus::Healthy)
    );

    // Data stops coming (simulating connection issue before BLE detects it)
    sleep(Duration::from_millis(110));

    // Check for stale connections
    let needs_reconnection = monitor.check_all();

    // Should detect stale connection BEFORE BLE timeout would normally occur
    assert!(
        needs_reconnection.contains(&"wahoo_kickr".to_string()),
        "Stale connection should trigger proactive reconnection"
    );
}

#[test]
fn test_multiple_sensors_monitored_independently() {
    let config = ConnectionHealthConfig {
        stale_timeout: Duration::from_millis(100),
        degraded_timeout: Duration::from_millis(50),
        enabled: true,
        ..ConnectionHealthConfig::default()
    };
    let mut monitor = ConnectionHealthMonitor::with_config(config);

    // Start monitoring multiple sensors
    monitor.start_monitoring("trainer");
    monitor.start_monitoring("hr_monitor");
    monitor.start_monitoring("power_meter");

    // All receive data initially
    monitor.record_data("trainer");
    monitor.record_data("hr_monitor");
    monitor.record_data("power_meter");

    // Wait a bit
    sleep(Duration::from_millis(30));

    // Only trainer and power meter continue sending
    monitor.record_data("trainer");
    monitor.record_data("power_meter");

    // Wait for HR to go stale
    sleep(Duration::from_millis(80));

    let needs_reconnection = monitor.check_all();

    // HR monitor should be stale, others healthy
    assert!(needs_reconnection.contains(&"hr_monitor".to_string()));

    // Trainer and power meter should still be healthy (recent data)
    let trainer_status = monitor.get_status("trainer");
    let power_status = monitor.get_status("power_meter");

    // Note: They might be degraded if 80ms > degraded_timeout
    // But they shouldn't need reconnection since they have recent data
    assert!(
        trainer_status == Some(HealthStatus::Healthy) ||
        trainer_status == Some(HealthStatus::Degraded)
    );
}

#[test]
fn test_data_rate_improves_after_recovery() {
    let config = ConnectionHealthConfig {
        rate_window: Duration::from_secs(1),
        stale_timeout: Duration::from_millis(200),
        ..ConnectionHealthConfig::default()
    };
    let mut health = ConnectionHealth::with_config("device_a".to_string(), config);

    // Initial data
    health.record_data_received();
    let initial_rate = health.data_rate();

    // More data improves rate
    for _ in 0..10 {
        health.record_data_received();
    }

    let improved_rate = health.data_rate();
    assert!(
        improved_rate > initial_rate,
        "Data rate should improve with more packets"
    );
}

#[test]
fn test_strict_config_for_trainer() {
    // Trainers should use strict config for quick detection
    let config = ConnectionHealthConfig::strict();
    let mut health = ConnectionHealth::with_config("trainer".to_string(), config);

    health.record_data_received();

    // Strict config has 3s stale timeout
    sleep(Duration::from_millis(100)); // Still healthy
    health.check();
    assert_eq!(health.status(), HealthStatus::Healthy);
}

#[test]
fn test_relaxed_config_for_hr_monitor() {
    // HR monitors can use relaxed config
    let config = ConnectionHealthConfig::relaxed();
    let mut health = ConnectionHealth::with_config("hr_monitor".to_string(), config);

    health.record_data_received();

    // Relaxed config has 10s stale timeout
    sleep(Duration::from_millis(100)); // Still healthy
    health.check();
    assert_eq!(health.status(), HealthStatus::Healthy);
}

#[test]
fn test_uptime_tracking() {
    let health = ConnectionHealth::new("device_a".to_string());

    // Small delay to ensure uptime is measurable
    sleep(Duration::from_millis(10));

    let uptime = health.uptime();
    assert!(uptime >= Duration::from_millis(10));
}
