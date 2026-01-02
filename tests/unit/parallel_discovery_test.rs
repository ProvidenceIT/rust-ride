//! Unit tests for parallel BLE/ANT+ sensor discovery.
//!
//! Tests verify that:
//! - ParallelDiscoveryResult correctly tracks discovery status
//! - Concurrent discovery configuration works correctly
//! - Both protocols can be started/tracked independently

use rustride::sensors::types::ParallelDiscoveryResult;

#[test]
fn test_parallel_discovery_result_default() {
    let result = ParallelDiscoveryResult::default();

    assert!(!result.ble_started);
    assert!(!result.ant_started);
    assert!(result.ble_error.is_none());
    assert!(result.ant_error.is_none());
}

#[test]
fn test_parallel_discovery_result_any_started_none() {
    let result = ParallelDiscoveryResult {
        ble_started: false,
        ant_started: false,
        ble_error: None,
        ant_error: None,
    };

    assert!(!result.any_started());
}

#[test]
fn test_parallel_discovery_result_any_started_ble_only() {
    let result = ParallelDiscoveryResult {
        ble_started: true,
        ant_started: false,
        ble_error: None,
        ant_error: Some("No dongle found".to_string()),
    };

    assert!(result.any_started());
    assert!(!result.all_started());
}

#[test]
fn test_parallel_discovery_result_any_started_ant_only() {
    let result = ParallelDiscoveryResult {
        ble_started: false,
        ant_started: true,
        ble_error: Some("Adapter not found".to_string()),
        ant_error: None,
    };

    assert!(result.any_started());
    assert!(!result.all_started());
}

#[test]
fn test_parallel_discovery_result_all_started() {
    let result = ParallelDiscoveryResult {
        ble_started: true,
        ant_started: true,
        ble_error: None,
        ant_error: None,
    };

    assert!(result.any_started());
    assert!(result.all_started());
    assert!(!result.has_errors());
}

#[test]
fn test_parallel_discovery_result_has_errors_ble() {
    let result = ParallelDiscoveryResult {
        ble_started: false,
        ant_started: true,
        ble_error: Some("Bluetooth disabled".to_string()),
        ant_error: None,
    };

    assert!(result.has_errors());
    assert_eq!(result.ble_error, Some("Bluetooth disabled".to_string()));
}

#[test]
fn test_parallel_discovery_result_has_errors_ant() {
    let result = ParallelDiscoveryResult {
        ble_started: true,
        ant_started: false,
        ble_error: None,
        ant_error: Some("ANT+ dongle not found".to_string()),
    };

    assert!(result.has_errors());
    assert_eq!(result.ant_error, Some("ANT+ dongle not found".to_string()));
}

#[test]
fn test_parallel_discovery_result_has_errors_both() {
    let result = ParallelDiscoveryResult {
        ble_started: false,
        ant_started: false,
        ble_error: Some("Bluetooth disabled".to_string()),
        ant_error: Some("ANT+ dongle not found".to_string()),
    };

    assert!(result.has_errors());
    assert!(!result.any_started());
    assert!(!result.all_started());
}

#[test]
fn test_parallel_discovery_result_clone() {
    let result = ParallelDiscoveryResult {
        ble_started: true,
        ant_started: true,
        ble_error: None,
        ant_error: None,
    };

    let cloned = result.clone();
    assert_eq!(cloned.ble_started, result.ble_started);
    assert_eq!(cloned.ant_started, result.ant_started);
}

#[test]
fn test_parallel_discovery_result_debug() {
    let result = ParallelDiscoveryResult {
        ble_started: true,
        ant_started: false,
        ble_error: None,
        ant_error: Some("Test error".to_string()),
    };

    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("ble_started: true"));
    assert!(debug_str.contains("ant_started: false"));
    assert!(debug_str.contains("Test error"));
}

/// Tests for concurrent scanning timing behavior.
/// These are simulated tests since we can't easily create real BLE/ANT+ hardware.
mod timing_tests {
    use std::time::{Duration, Instant};

    /// Simulate concurrent task execution to verify join behavior.
    #[tokio::test]
    async fn test_concurrent_execution_timing() {
        let start = Instant::now();

        // Simulate two 100ms tasks running concurrently
        let task1 = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            "ble"
        };

        let task2 = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            "ant"
        };

        // Run both concurrently using tokio::join!
        let (result1, result2) = tokio::join!(task1, task2);

        let elapsed = start.elapsed();

        // If run concurrently, should complete in ~100ms, not 200ms
        // Allow some margin for test overhead
        assert!(
            elapsed < Duration::from_millis(150),
            "Concurrent tasks took {:?}, expected ~100ms",
            elapsed
        );
        assert_eq!(result1, "ble");
        assert_eq!(result2, "ant");
    }

    /// Simulate sequential execution to compare with concurrent.
    #[tokio::test]
    async fn test_sequential_execution_timing() {
        let start = Instant::now();

        // Run two 50ms tasks sequentially
        tokio::time::sleep(Duration::from_millis(50)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let elapsed = start.elapsed();

        // Sequential execution should take at least 100ms
        assert!(
            elapsed >= Duration::from_millis(100),
            "Sequential tasks took {:?}, expected >= 100ms",
            elapsed
        );
    }

    /// Verify that failure in one concurrent task doesn't block the other.
    #[tokio::test]
    async fn test_concurrent_partial_failure() {
        let start = Instant::now();

        // One task succeeds, one fails
        let success_task = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<&str, &str>("ble")
        };

        let fail_task = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Err::<&str, &str>("ant failed")
        };

        let (success_result, fail_result) = tokio::join!(success_task, fail_task);

        let elapsed = start.elapsed();

        // Both should complete concurrently
        assert!(elapsed < Duration::from_millis(100));
        assert!(success_result.is_ok());
        assert!(fail_result.is_err());
    }
}

/// Tests for SensorConfig discovery settings.
mod config_tests {
    use rustride::sensors::SensorConfig;

    #[test]
    fn test_default_discovery_timeout() {
        let config = SensorConfig::default();
        assert_eq!(config.discovery_timeout_secs, 30);
    }

    #[test]
    fn test_custom_discovery_timeout() {
        let config = SensorConfig {
            discovery_timeout_secs: 15,
            ..SensorConfig::default()
        };
        assert_eq!(config.discovery_timeout_secs, 15);
    }

    #[test]
    fn test_auto_reconnect_default() {
        let config = SensorConfig::default();
        assert!(config.auto_reconnect);
    }
}
