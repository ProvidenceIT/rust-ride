//! Unit tests for exponential backoff reconnection.
//!
//! Tests verify that:
//! - Exponential backoff follows the pattern: 1s, 2s, 4s, 8s, 16s, 30s (capped)
//! - Backoff resets on successful connection
//! - Max attempts is enforced correctly
//! - ReconnectionManager tracks multiple devices independently
//! - Configuration presets work correctly

use rustride::sensors::reconnection::{
    ExponentialBackoff, ExponentialBackoffConfig, ReconnectionManager, ReconnectionStats,
};
use std::time::Duration;

// ============================================================================
// ExponentialBackoffConfig Tests
// ============================================================================

#[test]
fn test_default_config() {
    let config = ExponentialBackoffConfig::default();

    assert_eq!(config.initial_delay, Duration::from_secs(1));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.multiplier, 2.0);
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.jitter_factor, 0.0);
}

#[test]
fn test_aggressive_config() {
    let config = ExponentialBackoffConfig::aggressive();

    assert_eq!(config.initial_delay, Duration::from_millis(500));
    assert_eq!(config.max_delay, Duration::from_secs(15));
    assert_eq!(config.max_attempts, 8);
    assert!(config.jitter_factor > 0.0);
}

#[test]
fn test_conservative_config() {
    let config = ExponentialBackoffConfig::conservative();

    assert_eq!(config.initial_delay, Duration::from_secs(2));
    assert_eq!(config.max_delay, Duration::from_secs(60));
    assert_eq!(config.max_attempts, 3);
}

#[test]
fn test_with_jitter_config() {
    let config = ExponentialBackoffConfig::with_jitter();

    assert!(config.jitter_factor > 0.0);
    assert_eq!(config.jitter_factor, 0.25);
}

// ============================================================================
// ExponentialBackoff Basic Tests
// ============================================================================

#[test]
fn test_new_backoff_initial_state() {
    let backoff = ExponentialBackoff::new();

    assert_eq!(backoff.current_attempt(), 0);
    assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    assert!(!backoff.is_exhausted());
}

#[test]
fn test_backoff_first_attempt() {
    let mut backoff = ExponentialBackoff::new();

    let delay = backoff.record_attempt();

    assert_eq!(delay, Duration::from_secs(1));
    assert_eq!(backoff.current_attempt(), 1);
}

// ============================================================================
// Exponential Backoff Sequence Tests
// ============================================================================

#[test]
fn test_exponential_backoff_sequence_1s_2s_4s_8s_16s_30s() {
    let mut backoff = ExponentialBackoff::new();

    // Attempt 1: 1s
    let delay1 = backoff.record_attempt();
    assert_eq!(delay1, Duration::from_secs(1));

    // Attempt 2: 2s
    let delay2 = backoff.record_attempt();
    assert_eq!(delay2, Duration::from_secs(2));

    // Attempt 3: 4s
    let delay3 = backoff.record_attempt();
    assert_eq!(delay3, Duration::from_secs(4));

    // Attempt 4: 8s
    let delay4 = backoff.record_attempt();
    assert_eq!(delay4, Duration::from_secs(8));

    // Attempt 5: 16s
    let delay5 = backoff.record_attempt();
    assert_eq!(delay5, Duration::from_secs(16));
}

#[test]
fn test_backoff_caps_at_max_delay() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        max_attempts: 10,
        jitter_factor: 0.0,
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    // 1, 2, 4, 8, 16, 32 (capped to 30), 60 (capped to 30)...
    backoff.record_attempt(); // 1s
    backoff.record_attempt(); // 2s
    backoff.record_attempt(); // 4s
    backoff.record_attempt(); // 8s
    backoff.record_attempt(); // 16s

    // 6th attempt would be 32s, but capped at 30s
    let delay6 = backoff.record_attempt();
    assert_eq!(delay6, Duration::from_secs(30));

    // 7th attempt stays at 30s (already capped)
    let delay7 = backoff.record_attempt();
    assert_eq!(delay7, Duration::from_secs(30));
}

#[test]
fn test_delay_for_attempt_without_state_change() {
    let backoff = ExponentialBackoff::new();

    // delay_for_attempt should not change state
    assert_eq!(backoff.delay_for_attempt(1), Duration::from_secs(1));
    assert_eq!(backoff.delay_for_attempt(2), Duration::from_secs(2));
    assert_eq!(backoff.delay_for_attempt(3), Duration::from_secs(4));
    assert_eq!(backoff.delay_for_attempt(4), Duration::from_secs(8));
    assert_eq!(backoff.delay_for_attempt(5), Duration::from_secs(16));
    assert_eq!(backoff.delay_for_attempt(6), Duration::from_secs(30)); // capped
    assert_eq!(backoff.delay_for_attempt(7), Duration::from_secs(30)); // stays capped

    // State should be unchanged
    assert_eq!(backoff.current_attempt(), 0);
}

#[test]
fn test_delay_for_attempt_zero_returns_zero() {
    let backoff = ExponentialBackoff::new();

    assert_eq!(backoff.delay_for_attempt(0), Duration::ZERO);
}

// ============================================================================
// Reset Tests
// ============================================================================

#[test]
fn test_reset_clears_attempt_count() {
    let mut backoff = ExponentialBackoff::new();

    // Make some attempts
    backoff.record_attempt();
    backoff.record_attempt();
    backoff.record_attempt();

    assert_eq!(backoff.current_attempt(), 3);
    assert_eq!(backoff.next_delay(), Duration::from_secs(8));

    // Reset
    backoff.reset();

    assert_eq!(backoff.current_attempt(), 0);
    assert_eq!(backoff.next_delay(), Duration::from_secs(1));
}

#[test]
fn test_reset_allows_new_attempt_sequence() {
    let mut backoff = ExponentialBackoff::new();

    // First sequence
    backoff.record_attempt();
    backoff.record_attempt();
    backoff.record_attempt();

    // Reset on successful connection
    backoff.reset();

    // New sequence starts from 1s
    let delay = backoff.record_attempt();
    assert_eq!(delay, Duration::from_secs(1));
}

// ============================================================================
// Exhaustion Tests
// ============================================================================

#[test]
fn test_exhaustion_after_max_attempts() {
    let config = ExponentialBackoffConfig {
        max_attempts: 3,
        ..ExponentialBackoffConfig::default()
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), Some(3));

    backoff.record_attempt();
    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), Some(2));

    backoff.record_attempt();
    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), Some(1));

    backoff.record_attempt();
    assert!(backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), Some(0));
}

#[test]
fn test_unlimited_attempts() {
    let config = ExponentialBackoffConfig {
        max_attempts: 0, // 0 = unlimited
        ..ExponentialBackoffConfig::default()
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), None);

    // Even after many attempts, should not be exhausted
    for _ in 0..100 {
        backoff.record_attempt();
    }

    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), None);
}

#[test]
fn test_reset_clears_exhaustion() {
    let config = ExponentialBackoffConfig {
        max_attempts: 2,
        ..ExponentialBackoffConfig::default()
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    backoff.record_attempt();
    backoff.record_attempt();
    assert!(backoff.is_exhausted());

    // Reset should clear exhaustion
    backoff.reset();
    assert!(!backoff.is_exhausted());
    assert_eq!(backoff.remaining_attempts(), Some(2));
}

// ============================================================================
// All Delays Tests
// ============================================================================

#[test]
fn test_all_delays_returns_correct_sequence() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        max_attempts: 6,
        jitter_factor: 0.0,
    };
    let backoff = ExponentialBackoff::with_config(config);

    let delays = backoff.all_delays();

    assert_eq!(delays.len(), 6);
    assert_eq!(delays[0], Duration::from_secs(1));
    assert_eq!(delays[1], Duration::from_secs(2));
    assert_eq!(delays[2], Duration::from_secs(4));
    assert_eq!(delays[3], Duration::from_secs(8));
    assert_eq!(delays[4], Duration::from_secs(16));
    assert_eq!(delays[5], Duration::from_secs(30)); // capped
}

#[test]
fn test_all_delays_unlimited_returns_ten() {
    let config = ExponentialBackoffConfig {
        max_attempts: 0, // unlimited
        ..ExponentialBackoffConfig::default()
    };
    let backoff = ExponentialBackoff::with_config(config);

    let delays = backoff.all_delays();

    assert_eq!(delays.len(), 10); // Returns first 10 for unlimited
}

// ============================================================================
// Custom Configuration Tests
// ============================================================================

#[test]
fn test_custom_multiplier() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(100),
        multiplier: 3.0, // Triple each time
        max_attempts: 5,
        jitter_factor: 0.0,
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    assert_eq!(backoff.record_attempt(), Duration::from_secs(1));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(3));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(9));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(27));
}

#[test]
fn test_custom_initial_delay() {
    let config = ExponentialBackoffConfig {
        initial_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        max_attempts: 5,
        jitter_factor: 0.0,
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    assert_eq!(backoff.record_attempt(), Duration::from_millis(500));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(1));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(2));
}

// ============================================================================
// ReconnectionManager Tests
// ============================================================================

#[test]
fn test_manager_new_is_empty() {
    let manager = ReconnectionManager::new();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_manager_get_or_create() {
    let mut manager = ReconnectionManager::new();

    let backoff = manager.get_or_create("device_a");
    assert_eq!(backoff.current_attempt(), 0);

    assert_eq!(manager.len(), 1);
}

#[test]
fn test_manager_record_attempt() {
    let mut manager = ReconnectionManager::new();

    let delay = manager.record_attempt("device_a");
    assert_eq!(delay, Duration::from_secs(1));

    assert_eq!(manager.len(), 1);
}

#[test]
fn test_manager_tracks_multiple_devices_independently() {
    let mut manager = ReconnectionManager::new();

    // Record attempts for different devices
    manager.record_attempt("device_a");
    manager.record_attempt("device_a");
    manager.record_attempt("device_b");

    assert_eq!(manager.len(), 2);

    // Device A should be on attempt 2
    let stats_a = manager.get_stats("device_a").unwrap();
    assert_eq!(stats_a.current_attempt, 2);

    // Device B should be on attempt 1
    let stats_b = manager.get_stats("device_b").unwrap();
    assert_eq!(stats_b.current_attempt, 1);
}

#[test]
fn test_manager_reset_device() {
    let mut manager = ReconnectionManager::new();

    manager.record_attempt("device_a");
    manager.record_attempt("device_a");
    manager.record_attempt("device_a");

    // Reset device A
    manager.reset("device_a");

    let stats = manager.get_stats("device_a").unwrap();
    assert_eq!(stats.current_attempt, 0);
}

#[test]
fn test_manager_reset_does_not_affect_other_devices() {
    let mut manager = ReconnectionManager::new();

    manager.record_attempt("device_a");
    manager.record_attempt("device_a");
    manager.record_attempt("device_b");

    // Reset only device A
    manager.reset("device_a");

    let stats_a = manager.get_stats("device_a").unwrap();
    assert_eq!(stats_a.current_attempt, 0);

    let stats_b = manager.get_stats("device_b").unwrap();
    assert_eq!(stats_b.current_attempt, 1);
}

#[test]
fn test_manager_remove_device() {
    let mut manager = ReconnectionManager::new();

    manager.record_attempt("device_a");
    manager.record_attempt("device_b");

    assert_eq!(manager.len(), 2);

    manager.remove("device_a");

    assert_eq!(manager.len(), 1);
    assert!(manager.get("device_a").is_none());
    assert!(manager.get("device_b").is_some());
}

#[test]
fn test_manager_clear() {
    let mut manager = ReconnectionManager::new();

    manager.record_attempt("device_a");
    manager.record_attempt("device_b");
    manager.record_attempt("device_c");

    assert_eq!(manager.len(), 3);

    manager.clear();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_manager_is_exhausted() {
    let config = ExponentialBackoffConfig {
        max_attempts: 2,
        ..ExponentialBackoffConfig::default()
    };
    let mut manager = ReconnectionManager::with_config(config);

    assert!(!manager.is_exhausted("device_a"));

    manager.record_attempt("device_a");
    assert!(!manager.is_exhausted("device_a"));

    manager.record_attempt("device_a");
    assert!(manager.is_exhausted("device_a"));
}

#[test]
fn test_manager_get_nonexistent_device() {
    let manager = ReconnectionManager::new();

    assert!(manager.get("nonexistent").is_none());
    assert!(manager.get_stats("nonexistent").is_none());
    assert!(!manager.is_exhausted("nonexistent"));
}

// ============================================================================
// ReconnectionStats Tests
// ============================================================================

#[test]
fn test_stats_status_text_normal() {
    let stats = ReconnectionStats {
        device_id: "device_a".to_string(),
        current_attempt: 2,
        next_delay: Duration::from_secs(4),
        is_exhausted: false,
        remaining_attempts: Some(3),
    };

    let text = stats.status_text();
    assert!(text.contains("Attempt 2"));
    assert!(text.contains("4s"));
    assert!(text.contains("3 remaining"));
}

#[test]
fn test_stats_status_text_exhausted() {
    let stats = ReconnectionStats {
        device_id: "device_a".to_string(),
        current_attempt: 5,
        next_delay: Duration::from_secs(30),
        is_exhausted: true,
        remaining_attempts: Some(0),
    };

    let text = stats.status_text();
    assert!(text.contains("Gave up"));
    assert!(text.contains("5 attempts"));
}

#[test]
fn test_stats_status_text_unlimited() {
    let stats = ReconnectionStats {
        device_id: "device_a".to_string(),
        current_attempt: 10,
        next_delay: Duration::from_secs(30),
        is_exhausted: false,
        remaining_attempts: None,
    };

    let text = stats.status_text();
    assert!(text.contains("Attempt 10"));
    assert!(!text.contains("remaining"));
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_sensor_reconnection_success_resets_backoff() {
    let mut manager = ReconnectionManager::new();
    let device_id = "wahoo_kickr";

    // First disconnection - immediate reconnect after 1s
    let delay1 = manager.record_attempt(device_id);
    assert_eq!(delay1, Duration::from_secs(1));

    // Reconnection failed - wait 2s
    let delay2 = manager.record_attempt(device_id);
    assert_eq!(delay2, Duration::from_secs(2));

    // Reconnection successful - reset
    manager.reset(device_id);

    // Another disconnection - back to 1s
    let delay3 = manager.record_attempt(device_id);
    assert_eq!(delay3, Duration::from_secs(1));
}

#[test]
fn test_multiple_sensors_reconnecting() {
    let mut manager = ReconnectionManager::new();

    // Trainer disconnects first
    let trainer_delay = manager.record_attempt("trainer");
    assert_eq!(trainer_delay, Duration::from_secs(1));

    // HR monitor disconnects
    let hr_delay = manager.record_attempt("hr_monitor");
    assert_eq!(hr_delay, Duration::from_secs(1));

    // Trainer fails to reconnect
    let trainer_delay2 = manager.record_attempt("trainer");
    assert_eq!(trainer_delay2, Duration::from_secs(2));

    // HR successfully reconnects
    manager.reset("hr_monitor");

    // Trainer still on backoff
    let stats = manager.get_stats("trainer").unwrap();
    assert_eq!(stats.current_attempt, 2);
    assert_eq!(stats.next_delay, Duration::from_secs(4));

    // HR reset to initial
    let hr_stats = manager.get_stats("hr_monitor").unwrap();
    assert_eq!(hr_stats.current_attempt, 0);
}

#[test]
fn test_backoff_delay_sequence_matches_acceptance_criteria() {
    // Acceptance criteria: 1s, 2s, 4s, 8s, 16s, 30s
    let mut backoff = ExponentialBackoff::new();

    assert_eq!(backoff.record_attempt(), Duration::from_secs(1));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(2));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(4));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(8));
    assert_eq!(backoff.record_attempt(), Duration::from_secs(16));

    // Next would be 32s but capped at 30s
    // The 6th attempt uses the capped value from previous calculation
    // After 5th attempt (16s), next_delay becomes min(32, 30) = 30s
    assert_eq!(backoff.next_delay(), Duration::from_secs(30));
}

#[test]
fn test_default_max_attempts_matches_sensor_config() {
    // SensorConfig default is max_reconnect_attempts: 3
    // But ExponentialBackoffConfig default is 5
    // In production, manager.rs uses config.max_reconnect_attempts
    let config = ExponentialBackoffConfig {
        max_attempts: 3,
        ..ExponentialBackoffConfig::default()
    };
    let mut backoff = ExponentialBackoff::with_config(config);

    backoff.record_attempt();
    backoff.record_attempt();
    assert!(!backoff.is_exhausted());

    backoff.record_attempt();
    assert!(backoff.is_exhausted());
}
