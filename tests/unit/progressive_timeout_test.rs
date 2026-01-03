//! Unit tests for progressive discovery timeout.
//!
//! Tests verify that:
//! - ProgressiveTimeoutConfig correctly configures timeout behavior
//! - ProgressiveTimeoutState correctly tracks discovery progress
//! - TimeoutDecision logic works as expected
//! - Discovery phases transition correctly

use rustride::sensors::{
    DiscoveryPhase, DiscoveryProgress, ProgressiveTimeoutConfig, ProgressiveTimeoutState,
    StopReason, TimeoutDecision,
};
use std::time::Duration;

// ============================================================================
// ProgressiveTimeoutConfig Tests
// ============================================================================

#[test]
fn test_progressive_timeout_config_default() {
    let config = ProgressiveTimeoutConfig::default();

    assert_eq!(config.initial_scan_secs, 10);
    assert_eq!(config.extension_period_secs, 5);
    assert_eq!(config.max_total_secs, 30);
    assert_eq!(config.activity_window_secs, 3);
    assert_eq!(config.idle_threshold_secs, 5);
    assert!(config.enabled);
}

#[test]
fn test_progressive_timeout_config_fast() {
    let config = ProgressiveTimeoutConfig::fast();

    assert_eq!(config.initial_scan_secs, 5);
    assert_eq!(config.extension_period_secs, 3);
    assert_eq!(config.max_total_secs, 15);
    assert!(config.enabled);
}

#[test]
fn test_progressive_timeout_config_thorough() {
    let config = ProgressiveTimeoutConfig::thorough();

    assert_eq!(config.initial_scan_secs, 15);
    assert_eq!(config.extension_period_secs, 10);
    assert_eq!(config.max_total_secs, 45);
    assert!(config.enabled);
}

#[test]
fn test_progressive_timeout_config_disabled() {
    let config = ProgressiveTimeoutConfig::disabled();

    assert!(!config.enabled);
    // Other values should still be default
    assert_eq!(config.initial_scan_secs, 10);
}

#[test]
fn test_progressive_timeout_config_clone() {
    let config = ProgressiveTimeoutConfig {
        initial_scan_secs: 15,
        extension_period_secs: 7,
        max_total_secs: 45,
        activity_window_secs: 4,
        idle_threshold_secs: 6,
        enabled: true,
    };

    let cloned = config.clone();
    assert_eq!(cloned.initial_scan_secs, config.initial_scan_secs);
    assert_eq!(cloned.max_total_secs, config.max_total_secs);
    assert_eq!(cloned.enabled, config.enabled);
}

#[test]
fn test_progressive_timeout_config_debug() {
    let config = ProgressiveTimeoutConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("initial_scan_secs: 10"));
    assert!(debug_str.contains("enabled: true"));
}

// ============================================================================
// ProgressiveTimeoutState Tests
// ============================================================================

#[test]
fn test_progressive_timeout_state_new() {
    let state = ProgressiveTimeoutState::new();

    assert_eq!(state.phase, DiscoveryPhase::Initial);
    assert_eq!(state.sensors_discovered, 0);
    assert_eq!(state.extensions_count, 0);
    assert!(state.last_discovery_at.is_none());
}

#[test]
fn test_progressive_timeout_state_default() {
    let state = ProgressiveTimeoutState::default();

    assert_eq!(state.phase, DiscoveryPhase::Initial);
    assert_eq!(state.sensors_discovered, 0);
}

#[test]
fn test_progressive_timeout_state_record_discovery() {
    let mut state = ProgressiveTimeoutState::new();

    assert_eq!(state.sensors_discovered, 0);
    assert!(state.last_discovery_at.is_none());

    state.record_discovery();

    assert_eq!(state.sensors_discovered, 1);
    assert!(state.last_discovery_at.is_some());

    state.record_discovery();
    state.record_discovery();

    assert_eq!(state.sensors_discovered, 3);
}

#[test]
fn test_progressive_timeout_state_elapsed() {
    let state = ProgressiveTimeoutState::new();

    // Small delay to ensure elapsed > 0
    std::thread::sleep(Duration::from_millis(10));

    let elapsed = state.elapsed();
    assert!(elapsed >= Duration::from_millis(10));
}

#[test]
fn test_progressive_timeout_state_time_since_last_discovery() {
    let mut state = ProgressiveTimeoutState::new();

    assert!(state.time_since_last_discovery().is_none());

    state.record_discovery();

    std::thread::sleep(Duration::from_millis(10));

    let time_since = state.time_since_last_discovery();
    assert!(time_since.is_some());
    assert!(time_since.unwrap() >= Duration::from_millis(10));
}

#[test]
fn test_progressive_timeout_state_has_recent_activity_none() {
    let state = ProgressiveTimeoutState::new();

    // No discoveries yet
    assert!(!state.has_recent_activity(5));
}

#[test]
fn test_progressive_timeout_state_has_recent_activity_recent() {
    let mut state = ProgressiveTimeoutState::new();

    state.record_discovery();

    // Just discovered, should be recent
    assert!(state.has_recent_activity(5));
}

#[test]
fn test_progressive_timeout_state_is_idle_no_discovery() {
    let state = ProgressiveTimeoutState::new();

    // Very short threshold - should not be idle immediately
    assert!(!state.is_idle(1));

    // Wait and check again with very short threshold
    std::thread::sleep(Duration::from_millis(50));
    // With 0 second threshold, should now be idle
    assert!(state.is_idle(0));
}

#[test]
fn test_progressive_timeout_state_apply_extension() {
    let mut state = ProgressiveTimeoutState::new();

    assert_eq!(state.phase, DiscoveryPhase::Initial);
    assert_eq!(state.extensions_count, 0);

    state.apply_extension();

    assert_eq!(state.phase, DiscoveryPhase::Extended);
    assert_eq!(state.extensions_count, 1);

    state.apply_extension();

    assert_eq!(state.phase, DiscoveryPhase::Extended);
    assert_eq!(state.extensions_count, 2);
}

#[test]
fn test_progressive_timeout_state_mark_completed() {
    let mut state = ProgressiveTimeoutState::new();

    assert_eq!(state.phase, DiscoveryPhase::Initial);

    state.mark_completed();

    assert_eq!(state.phase, DiscoveryPhase::Completed);
}

// ============================================================================
// TimeoutDecision Tests
// ============================================================================

#[test]
fn test_timeout_decision_continue_during_initial_phase() {
    let state = ProgressiveTimeoutState::new();
    let config = ProgressiveTimeoutConfig::default();

    // Just started, should continue
    let decision = state.calculate_decision(&config);

    assert_eq!(decision, TimeoutDecision::Continue);
}

#[test]
fn test_timeout_decision_disabled_uses_fixed_timeout() {
    let state = ProgressiveTimeoutState::new();
    let config = ProgressiveTimeoutConfig::disabled();

    // Should continue when disabled and not at max time
    let decision = state.calculate_decision(&config);

    assert_eq!(decision, TimeoutDecision::Continue);
}

#[test]
fn test_timeout_decision_completed_phase_stops() {
    let mut state = ProgressiveTimeoutState::new();
    let config = ProgressiveTimeoutConfig::default();

    state.mark_completed();

    let decision = state.calculate_decision(&config);

    assert_eq!(
        decision,
        TimeoutDecision::Stop {
            reason: StopReason::Completed
        }
    );
}

// ============================================================================
// DiscoveryPhase Tests
// ============================================================================

#[test]
fn test_discovery_phase_display_initial() {
    assert_eq!(format!("{}", DiscoveryPhase::Initial), "Initial Scan");
}

#[test]
fn test_discovery_phase_display_extended() {
    assert_eq!(format!("{}", DiscoveryPhase::Extended), "Extended Scan");
}

#[test]
fn test_discovery_phase_display_completed() {
    assert_eq!(format!("{}", DiscoveryPhase::Completed), "Completed");
}

#[test]
fn test_discovery_phase_equality() {
    assert_eq!(DiscoveryPhase::Initial, DiscoveryPhase::Initial);
    assert_ne!(DiscoveryPhase::Initial, DiscoveryPhase::Extended);
    assert_ne!(DiscoveryPhase::Extended, DiscoveryPhase::Completed);
}

#[test]
fn test_discovery_phase_clone() {
    let phase = DiscoveryPhase::Extended;
    let cloned = phase.clone();
    assert_eq!(phase, cloned);
}

// ============================================================================
// StopReason Tests
// ============================================================================

#[test]
fn test_stop_reason_equality() {
    assert_eq!(StopReason::MaxTimeReached, StopReason::MaxTimeReached);
    assert_eq!(StopReason::IdleTimeout, StopReason::IdleTimeout);
    assert_ne!(StopReason::MaxTimeReached, StopReason::IdleTimeout);
}

#[test]
fn test_stop_reason_debug() {
    let reason = StopReason::MaxTimeReached;
    let debug_str = format!("{:?}", reason);
    assert!(debug_str.contains("MaxTimeReached"));
}

// ============================================================================
// TimeoutDecision Tests
// ============================================================================

#[test]
fn test_timeout_decision_equality() {
    assert_eq!(TimeoutDecision::Continue, TimeoutDecision::Continue);
    assert_eq!(TimeoutDecision::Extend, TimeoutDecision::Extend);
    assert_ne!(TimeoutDecision::Continue, TimeoutDecision::Extend);

    assert_eq!(
        TimeoutDecision::Stop {
            reason: StopReason::IdleTimeout
        },
        TimeoutDecision::Stop {
            reason: StopReason::IdleTimeout
        }
    );

    assert_ne!(
        TimeoutDecision::Stop {
            reason: StopReason::IdleTimeout
        },
        TimeoutDecision::Stop {
            reason: StopReason::MaxTimeReached
        }
    );
}

#[test]
fn test_timeout_decision_debug() {
    let decision = TimeoutDecision::Stop {
        reason: StopReason::IdleTimeout,
    };
    let debug_str = format!("{:?}", decision);
    assert!(debug_str.contains("IdleTimeout"));
}

// ============================================================================
// DiscoveryProgress Tests
// ============================================================================

#[test]
fn test_discovery_progress_status_text_active() {
    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Initial,
        elapsed: Duration::from_secs(5),
        sensors_discovered: 2,
        extensions_count: 0,
        is_active: true,
    };

    let status = progress.status_text();
    assert!(status.contains("Scanning"));
    assert!(status.contains("5s"));
    assert!(status.contains("2 sensors"));
}

#[test]
fn test_discovery_progress_status_text_extended() {
    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Extended,
        elapsed: Duration::from_secs(15),
        sensors_discovered: 3,
        extensions_count: 1,
        is_active: true,
    };

    let status = progress.status_text();
    assert!(status.contains("Extended"));
}

#[test]
fn test_discovery_progress_status_text_completed() {
    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Completed,
        elapsed: Duration::from_secs(12),
        sensors_discovered: 4,
        extensions_count: 0,
        is_active: false,
    };

    let status = progress.status_text();
    assert!(status.contains("Completed"));
    assert!(status.contains("4 sensors"));
}

#[test]
fn test_discovery_progress_status_text_single_sensor() {
    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Completed,
        elapsed: Duration::from_secs(8),
        sensors_discovered: 1,
        extensions_count: 0,
        is_active: false,
    };

    let status = progress.status_text();
    assert!(status.contains("1 sensor"));
    assert!(!status.contains("sensors")); // Should not be plural
}

#[test]
fn test_discovery_progress_progress_percent() {
    let config = ProgressiveTimeoutConfig::default();

    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Initial,
        elapsed: Duration::from_secs(15),
        sensors_discovered: 2,
        extensions_count: 0,
        is_active: true,
    };

    // 15s out of 30s max = 50%
    let percent = progress.progress_percent(&config);
    assert!((percent - 50.0).abs() < 0.1);
}

#[test]
fn test_discovery_progress_progress_percent_completed() {
    let config = ProgressiveTimeoutConfig::default();

    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Completed,
        elapsed: Duration::from_secs(12),
        sensors_discovered: 3,
        extensions_count: 0,
        is_active: false,
    };

    // Completed = 100%
    let percent = progress.progress_percent(&config);
    assert!((percent - 100.0).abs() < 0.1);
}

#[test]
fn test_discovery_progress_progress_percent_max_99() {
    let config = ProgressiveTimeoutConfig::default();

    let progress = DiscoveryProgress {
        phase: DiscoveryPhase::Extended,
        elapsed: Duration::from_secs(35), // Over max
        sensors_discovered: 2,
        extensions_count: 1,
        is_active: true,
    };

    // Should cap at 99% while still active
    let percent = progress.progress_percent(&config);
    assert!(percent <= 99.0);
}

// ============================================================================
// Integration-like Tests (Simulating Progressive Behavior)
// ============================================================================

/// Test the typical flow: initial scan, extend on activity, complete.
mod integration_simulation {
    use super::*;

    #[test]
    fn test_typical_discovery_flow_no_sensors() {
        // Simulate discovery with no sensors found
        let mut state = ProgressiveTimeoutState::new();
        let config = ProgressiveTimeoutConfig {
            initial_scan_secs: 0, // Immediate for test
            extension_period_secs: 0,
            max_total_secs: 1,
            activity_window_secs: 1,
            idle_threshold_secs: 0,
            enabled: true,
        };

        // No sensors discovered, should extend to look for more
        std::thread::sleep(Duration::from_millis(10));
        let decision = state.calculate_decision(&config);

        // Should extend since we haven't found anything
        assert_eq!(decision, TimeoutDecision::Extend);
    }

    #[test]
    fn test_typical_discovery_flow_with_sensors_idle() {
        // Simulate discovery with sensors found but then idle
        let mut state = ProgressiveTimeoutState::new();
        let config = ProgressiveTimeoutConfig {
            initial_scan_secs: 0, // Immediate for test
            extension_period_secs: 0,
            max_total_secs: 5,
            activity_window_secs: 0, // No recent activity
            idle_threshold_secs: 0,  // Immediately idle
            enabled: true,
        };

        // Discover a sensor
        state.record_discovery();

        std::thread::sleep(Duration::from_millis(10));

        let decision = state.calculate_decision(&config);

        // Should stop due to idle timeout (found sensors but been idle)
        assert_eq!(
            decision,
            TimeoutDecision::Stop {
                reason: StopReason::IdleTimeout
            }
        );
    }

    #[test]
    fn test_extensions_are_counted() {
        let mut state = ProgressiveTimeoutState::new();

        assert_eq!(state.extensions_count, 0);

        for i in 1..=5 {
            state.apply_extension();
            assert_eq!(state.extensions_count, i);
        }

        assert_eq!(state.extensions_count, 5);
    }

    #[test]
    fn test_multiple_discoveries_recorded() {
        let mut state = ProgressiveTimeoutState::new();

        for i in 1..=10 {
            state.record_discovery();
            assert_eq!(state.sensors_discovered, i);
            assert!(state.last_discovery_at.is_some());
        }
    }
}

/// Tests for SensorConfig with progressive timeout.
mod config_integration_tests {
    use rustride::sensors::SensorConfig;

    #[test]
    fn test_sensor_config_includes_progressive_timeout() {
        let config = SensorConfig::default();

        // Progressive timeout should be included and enabled by default
        assert!(config.progressive_timeout.enabled);
        assert_eq!(config.progressive_timeout.initial_scan_secs, 10);
        assert_eq!(config.progressive_timeout.max_total_secs, 30);
    }

    #[test]
    fn test_sensor_config_custom_progressive_timeout() {
        use rustride::sensors::ProgressiveTimeoutConfig;

        let config = SensorConfig {
            progressive_timeout: ProgressiveTimeoutConfig::fast(),
            ..SensorConfig::default()
        };

        assert_eq!(config.progressive_timeout.initial_scan_secs, 5);
        assert_eq!(config.progressive_timeout.max_total_secs, 15);
    }
}
