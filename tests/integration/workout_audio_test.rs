//! Workout Audio Integration Tests
//!
//! Tests that verify workout events trigger appropriate voice announcements.
//! Uses mock audio engine to verify the correct messages are generated.

use rustride::audio::{
    AlertConfig, AlertContext, AlertData, AlertManager, AlertType, CueBuilder, CueTemplate,
    WorkoutAudioBridge, WorkoutAudioBridgeConfig,
};
use rustride::workouts::types::WorkoutEvent;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Mock Alert Manager
// ============================================================================

/// Mock alert manager that captures triggered alerts for verification.
#[derive(Debug)]
struct MockAlertManager {
    /// Captured alerts: (AlertType, AlertContext, generated message)
    triggered_alerts: Mutex<Vec<(AlertType, AlertContext, String)>>,
    /// Configuration for each alert type
    configs: Mutex<HashMap<AlertType, AlertConfig>>,
    /// Cue builder for message generation
    cue_builder: CueBuilder,
}

impl MockAlertManager {
    fn new() -> Self {
        Self {
            triggered_alerts: Mutex::new(Vec::new()),
            configs: Mutex::new(HashMap::new()),
            cue_builder: CueBuilder::new(),
        }
    }

    /// Get all triggered alerts with their generated messages.
    fn get_triggered_alerts(&self) -> Vec<(AlertType, AlertContext, String)> {
        self.triggered_alerts.lock().unwrap().clone()
    }

    /// Get just the alert types that were triggered.
    fn get_alert_types(&self) -> Vec<AlertType> {
        self.triggered_alerts
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _, _)| *t)
            .collect()
    }

    /// Get just the messages that were generated.
    fn get_messages(&self) -> Vec<String> {
        self.triggered_alerts
            .lock()
            .unwrap()
            .iter()
            .map(|(_, _, m)| m.clone())
            .collect()
    }

    /// Clear all captured alerts.
    fn clear(&self) {
        self.triggered_alerts.lock().unwrap().clear();
    }
}

impl AlertManager for MockAlertManager {
    async fn trigger(&self, alert_type: AlertType, context: AlertContext) {
        // Generate the message using the cue builder (simulates what DefaultAlertManager does)
        let message = self.cue_builder.build(alert_type, &context);

        self.triggered_alerts
            .lock()
            .unwrap()
            .push((alert_type, context, message));
    }

    fn configure(&self, alert_type: AlertType, config: AlertConfig) {
        self.configs.lock().unwrap().insert(alert_type, config);
    }

    fn get_config(&self, alert_type: AlertType) -> AlertConfig {
        self.configs
            .lock()
            .unwrap()
            .get(&alert_type)
            .cloned()
            .unwrap_or_default()
    }

    fn set_enabled(&self, alert_type: AlertType, enabled: bool) {
        let mut configs = self.configs.lock().unwrap();
        if let Some(config) = configs.get_mut(&alert_type) {
            config.enabled = enabled;
        } else {
            let mut config = AlertConfig::default();
            config.enabled = enabled;
            configs.insert(alert_type, config);
        }
    }

    fn is_on_cooldown(&self, _alert_type: AlertType) -> bool {
        false
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a default workout audio bridge with the mock alert manager.
fn create_bridge() -> (WorkoutAudioBridge<MockAlertManager>, Arc<MockAlertManager>) {
    let alert_manager = Arc::new(MockAlertManager::new());
    let bridge = WorkoutAudioBridge::new(alert_manager.clone());
    (bridge, alert_manager)
}

/// Create a bridge with custom configuration.
fn create_bridge_with_config(
    config: WorkoutAudioBridgeConfig,
) -> (WorkoutAudioBridge<MockAlertManager>, Arc<MockAlertManager>) {
    let alert_manager = Arc::new(MockAlertManager::new());
    let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), config);
    (bridge, alert_manager)
}

// ============================================================================
// Workout Start Tests
// ============================================================================

#[tokio::test]
async fn test_workout_start_triggers_announcement() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::Started {
        workout_name: "Sweet Spot Training".to_string(),
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1, "Should trigger one alert");
    assert_eq!(alerts[0].0, AlertType::WorkoutStart);

    // Verify message is generated (should be one of the alternatives)
    let message = &alerts[0].2;
    let valid_messages = ["Starting workout", "Let's begin", "Workout starting now"];
    assert!(
        valid_messages.contains(&message.as_str()),
        "Message '{}' should be one of the workout start messages",
        message
    );
}

#[tokio::test]
async fn test_workout_start_disabled() {
    let config = WorkoutAudioBridgeConfig {
        announce_workout_lifecycle: false,
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    let event = WorkoutEvent::Started {
        workout_name: "Test".to_string(),
    };
    bridge.process_event(&event).await;

    assert!(
        alert_manager.get_triggered_alerts().is_empty(),
        "Should not trigger when lifecycle announcements are disabled"
    );
}

// ============================================================================
// Interval Change Tests
// ============================================================================

#[tokio::test]
async fn test_interval_change_with_power_and_duration() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Sweet Spot".to_string(),
        target_power: Some(260),
        duration_secs: 300,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::IntervalChange);

    // Verify message contains interval name, power, and duration
    let message = &alerts[0].2;
    assert!(
        message.contains("Sweet Spot interval"),
        "Message '{}' should contain interval name",
        message
    );
    assert!(
        message.contains("260 watts"),
        "Message '{}' should contain power",
        message
    );
    assert!(
        message.contains("5 minutes"),
        "Message '{}' should contain duration",
        message
    );

    // Verify exact message format
    assert_eq!(
        message, "Sweet Spot interval, 260 watts, 5 minutes",
        "Should produce natural-sounding message"
    );
}

#[tokio::test]
async fn test_interval_change_without_power() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Free Ride".to_string(),
        target_power: None,
        duration_secs: 120,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);

    let message = &alerts[0].2;
    assert!(
        message.contains("Free Ride interval"),
        "Message '{}' should contain interval name",
        message
    );
    assert!(
        !message.contains("watts"),
        "Message '{}' should not contain watts when power is None",
        message
    );
    assert!(
        message.contains("2 minutes"),
        "Message '{}' should contain duration",
        message
    );
}

#[tokio::test]
async fn test_interval_change_various_durations() {
    let (bridge, alert_manager) = create_bridge();

    // Test short duration (seconds)
    let event = WorkoutEvent::IntervalChange {
        interval_name: "Sprint".to_string(),
        target_power: Some(400),
        duration_secs: 30,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let message = &alert_manager.get_messages()[0];
    assert!(
        message.contains("30 seconds"),
        "Message '{}' should show seconds for short durations",
        message
    );

    alert_manager.clear();

    // Test long duration (hours)
    let event = WorkoutEvent::IntervalChange {
        interval_name: "Endurance".to_string(),
        target_power: Some(150),
        duration_secs: 3660,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let message = &alert_manager.get_messages()[0];
    assert!(
        message.contains("1 hours 1 minutes"),
        "Message '{}' should show hours for long durations",
        message
    );
}

// ============================================================================
// Recovery Interval Tests
// ============================================================================

#[tokio::test]
async fn test_recovery_interval_triggers_recovery_start() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Recovery".to_string(),
        target_power: Some(100),
        duration_secs: 120,
        is_recovery: true,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 1);
    assert_eq!(
        alert_types[0],
        AlertType::RecoveryStart,
        "Recovery interval should trigger RecoveryStart, not IntervalChange"
    );

    // Verify recovery message
    let message = &alert_manager.get_messages()[0];
    assert_eq!(
        message, "Recovery. Take it easy.",
        "Should produce recovery message"
    );
}

#[tokio::test]
async fn test_recovery_interval_falls_back_when_disabled() {
    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: true,
        announce_recovery_intervals: false, // Disable recovery-specific announcements
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Recovery".to_string(),
        target_power: Some(100),
        duration_secs: 120,
        is_recovery: true,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 1);
    assert_eq!(
        alert_types[0],
        AlertType::IntervalChange,
        "Should fall back to regular IntervalChange when recovery is disabled"
    );
}

// ============================================================================
// Countdown Tests
// ============================================================================

#[tokio::test]
async fn test_countdown_announcements() {
    let (bridge, alert_manager) = create_bridge();

    // Test all standard countdown thresholds: 10, 5, 3, 2, 1
    let thresholds = [10, 5, 3, 2, 1];

    for &seconds in &thresholds {
        alert_manager.clear();

        let event = WorkoutEvent::IntervalCountdown {
            seconds_remaining: seconds,
        };
        bridge.process_event(&event).await;

        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].0, AlertType::IntervalCountdown);

        // Verify countdown message format
        let message = &alerts[0].2;
        match seconds {
            1 | 2 | 3 => {
                // Short form for urgency
                assert_eq!(
                    message,
                    &seconds.to_string(),
                    "Countdown {} should use short form",
                    seconds
                );
            }
            _ => {
                // Full form for longer countdowns
                assert_eq!(
                    message,
                    &format!("{} seconds", seconds),
                    "Countdown {} should use full form",
                    seconds
                );
            }
        }
    }
}

#[tokio::test]
async fn test_countdown_disabled() {
    let config = WorkoutAudioBridgeConfig {
        announce_countdowns: false,
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    let event = WorkoutEvent::IntervalCountdown {
        seconds_remaining: 5,
    };
    bridge.process_event(&event).await;

    assert!(
        alert_manager.get_triggered_alerts().is_empty(),
        "Should not trigger when countdowns are disabled"
    );
}

// ============================================================================
// Workout Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_workout_pause_resume() {
    let (bridge, alert_manager) = create_bridge();

    let events = vec![WorkoutEvent::Paused, WorkoutEvent::Resumed];
    bridge.process_events(&events).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 2);
    assert_eq!(alert_types[0], AlertType::RidePaused);
    assert_eq!(alert_types[1], AlertType::RideResumed);

    let messages = alert_manager.get_messages();
    assert_eq!(messages[0], "Paused");
    assert_eq!(messages[1], "Resumed");
}

#[tokio::test]
async fn test_workout_completed() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::Completed {
        total_duration_secs: 3600,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::WorkoutComplete);

    // Verify completion message (one of the alternatives)
    let message = &alerts[0].2;
    let valid_messages = [
        "Workout complete. Great job!",
        "Workout finished. Well done!",
        "You did it! Workout complete.",
    ];
    assert!(
        valid_messages.contains(&message.as_str()),
        "Message '{}' should be one of the completion messages",
        message
    );
}

#[tokio::test]
async fn test_workout_stopped() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::Stopped;
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::RidePaused); // Stopped uses RidePaused as fallback
}

// ============================================================================
// Trainer Status Tests
// ============================================================================

#[tokio::test]
async fn test_trainer_disconnect_reconnect() {
    let (bridge, alert_manager) = create_bridge();

    let events = vec![
        WorkoutEvent::TrainerDisconnected,
        WorkoutEvent::TrainerReconnected,
    ];
    bridge.process_events(&events).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 2);

    // Verify disconnect
    assert_eq!(alerts[0].0, AlertType::SensorDisconnected);
    assert_eq!(alerts[0].2, "Trainer disconnected");

    // Verify reconnect
    assert_eq!(alerts[1].0, AlertType::SensorConnected);
    assert_eq!(alerts[1].2, "Trainer connected");

    // Verify sensor context
    match &alerts[0].1.data {
        AlertData::Sensor {
            sensor_name,
            sensor_type,
        } => {
            assert_eq!(sensor_name, "Trainer");
            assert_eq!(sensor_type, "smart_trainer");
        }
        _ => panic!("Expected Sensor data"),
    }
}

#[tokio::test]
async fn test_trainer_status_disabled() {
    let config = WorkoutAudioBridgeConfig {
        announce_trainer_status: false,
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    let events = vec![
        WorkoutEvent::TrainerDisconnected,
        WorkoutEvent::TrainerReconnected,
    ];
    bridge.process_events(&events).await;

    assert!(
        alert_manager.get_triggered_alerts().is_empty(),
        "Should not trigger when trainer status is disabled"
    );
}

// ============================================================================
// Motivational Message Tests
// ============================================================================

#[tokio::test]
async fn test_motivational_messages_high_intensity() {
    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: true,
        announce_motivational_messages: true, // Enable motivational messages
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    // Non-recovery interval should trigger IntervalChange + MotivationalHighIntensity
    let event = WorkoutEvent::IntervalChange {
        interval_name: "Threshold".to_string(),
        target_power: Some(280),
        duration_secs: 300,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 2);
    assert_eq!(alert_types[0], AlertType::IntervalChange);
    assert_eq!(alert_types[1], AlertType::MotivationalHighIntensity);

    // Verify motivational message is one of the valid options
    let messages = alert_manager.get_messages();
    let valid_motivational = [
        "You're doing great!",
        "Keep pushing!",
        "Stay strong!",
        "You've got this!",
        "Keep it up!",
        "Great effort!",
        "Push through!",
    ];
    assert!(
        valid_motivational.contains(&messages[1].as_str()),
        "Motivational message '{}' should be one of the valid messages",
        messages[1]
    );
}

#[tokio::test]
async fn test_motivational_messages_recovery() {
    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: true,
        announce_recovery_intervals: true,
        announce_motivational_messages: true,
        ..Default::default()
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    // Recovery interval should trigger RecoveryStart + MotivationalRecovery
    let event = WorkoutEvent::IntervalChange {
        interval_name: "Rest".to_string(),
        target_power: Some(100),
        duration_secs: 60,
        is_recovery: true,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 2);
    assert_eq!(alert_types[0], AlertType::RecoveryStart);
    assert_eq!(alert_types[1], AlertType::MotivationalRecovery);

    // Verify recovery motivational message
    let messages = alert_manager.get_messages();
    let valid_recovery = [
        "Nice work, catch your breath",
        "Great job, take it easy",
        "Well done, recover well",
        "Excellent effort, rest up",
        "Good work, relax and recover",
    ];
    assert!(
        valid_recovery.contains(&messages[1].as_str()),
        "Recovery message '{}' should be one of the valid messages",
        messages[1]
    );
}

#[tokio::test]
async fn test_motivational_messages_disabled_by_default() {
    let (bridge, alert_manager) = create_bridge();

    // Default config should have motivational messages disabled
    let event = WorkoutEvent::IntervalChange {
        interval_name: "Threshold".to_string(),
        target_power: Some(280),
        duration_secs: 300,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(
        alert_types.len(),
        1,
        "Should only trigger IntervalChange, not motivational"
    );
    assert_eq!(alert_types[0], AlertType::IntervalChange);
}

// ============================================================================
// Full Workout Simulation Tests
// ============================================================================

#[tokio::test]
async fn test_complete_workout_sequence() {
    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: true,
        announce_countdowns: true,
        announce_workout_lifecycle: true,
        announce_trainer_status: true,
        announce_recovery_intervals: true,
        announce_motivational_messages: false, // Keep simple for testing
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    // Simulate a complete mini-workout
    let events = vec![
        // Start
        WorkoutEvent::Started {
            workout_name: "Quick Intervals".to_string(),
        },
        // Warmup
        WorkoutEvent::IntervalChange {
            interval_name: "Warmup".to_string(),
            target_power: Some(150),
            duration_secs: 300,
            is_recovery: false,
        },
        // Countdown to first interval
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 3,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 2,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 1,
        },
        // High intensity interval
        WorkoutEvent::IntervalChange {
            interval_name: "VO2 Max".to_string(),
            target_power: Some(350),
            duration_secs: 60,
            is_recovery: false,
        },
        // Pause during workout
        WorkoutEvent::Paused,
        WorkoutEvent::Resumed,
        // Recovery
        WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 60,
            is_recovery: true,
        },
        // Complete
        WorkoutEvent::Completed {
            total_duration_secs: 420,
        },
    ];

    bridge.process_events(&events).await;

    let alert_types = alert_manager.get_alert_types();

    // Verify the sequence of alerts
    assert_eq!(alert_types.len(), 12);
    assert_eq!(alert_types[0], AlertType::WorkoutStart);
    assert_eq!(alert_types[1], AlertType::IntervalChange); // Warmup
    assert_eq!(alert_types[2], AlertType::IntervalCountdown); // 10s
    assert_eq!(alert_types[3], AlertType::IntervalCountdown); // 5s
    assert_eq!(alert_types[4], AlertType::IntervalCountdown); // 3s
    assert_eq!(alert_types[5], AlertType::IntervalCountdown); // 2s
    assert_eq!(alert_types[6], AlertType::IntervalCountdown); // 1s
    assert_eq!(alert_types[7], AlertType::IntervalChange); // VO2 Max
    assert_eq!(alert_types[8], AlertType::RidePaused);
    assert_eq!(alert_types[9], AlertType::RideResumed);
    assert_eq!(alert_types[10], AlertType::RecoveryStart);
    assert_eq!(alert_types[11], AlertType::WorkoutComplete);

    // Verify some key messages
    let messages = alert_manager.get_messages();
    assert!(messages[1].contains("Warmup interval"));
    assert!(messages[7].contains("VO2 Max interval"));
    assert!(messages[7].contains("350 watts"));
    assert!(messages[7].contains("1 minutes"));
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_all_announcements_disabled() {
    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: false,
        announce_countdowns: false,
        announce_workout_lifecycle: false,
        announce_trainer_status: false,
        announce_recovery_intervals: false,
        announce_motivational_messages: false,
    };
    let (bridge, alert_manager) = create_bridge_with_config(config);

    let events = vec![
        WorkoutEvent::Started {
            workout_name: "Test".to_string(),
        },
        WorkoutEvent::IntervalChange {
            interval_name: "Test".to_string(),
            target_power: Some(200),
            duration_secs: 60,
            is_recovery: false,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
        WorkoutEvent::Paused,
        WorkoutEvent::TrainerDisconnected,
    ];
    bridge.process_events(&events).await;

    assert!(
        alert_manager.get_triggered_alerts().is_empty(),
        "No alerts should be triggered when all are disabled"
    );
}

#[tokio::test]
async fn test_config_update() {
    let (mut bridge, alert_manager) = create_bridge();

    // Initially with default config (countdowns enabled)
    let event = WorkoutEvent::IntervalCountdown {
        seconds_remaining: 5,
    };
    bridge.process_event(&event).await;
    assert_eq!(alert_manager.get_alert_types().len(), 1);

    alert_manager.clear();

    // Update config to disable countdowns
    let new_config = WorkoutAudioBridgeConfig {
        announce_countdowns: false,
        ..Default::default()
    };
    bridge.set_config(new_config);

    // Now countdown should not trigger
    bridge.process_event(&event).await;
    assert!(
        alert_manager.get_triggered_alerts().is_empty(),
        "Countdown should not trigger after config update"
    );
}

// ============================================================================
// CueBuilder Integration Tests
// ============================================================================

#[test]
fn test_cue_builder_interval_change_message() {
    let builder = CueBuilder::new();
    let context = AlertContext::interval_change("Threshold", Some(280), 360);

    let message = builder.build(AlertType::IntervalChange, &context);
    assert_eq!(message, "Threshold interval, 280 watts, 6 minutes");
}

#[test]
fn test_cue_builder_countdown_short_form() {
    let builder = CueBuilder::new();

    // Test short form for final 3 seconds
    for seconds in [3, 2, 1] {
        let context = AlertContext::countdown(seconds);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(
            message,
            seconds.to_string(),
            "Countdown {} should use short form",
            seconds
        );
    }
}

#[test]
fn test_cue_builder_countdown_full_form() {
    let builder = CueBuilder::new();

    // Test full form for 5+ seconds
    for seconds in [10, 5] {
        let context = AlertContext::countdown(seconds);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(
            message,
            format!("{} seconds", seconds),
            "Countdown {} should use full form",
            seconds
        );
    }
}

#[test]
fn test_cue_builder_sensor_messages() {
    let builder = CueBuilder::new();

    let context = AlertContext::sensor("Heart Rate Monitor", "hrm");
    let connected_msg = builder.build(AlertType::SensorConnected, &context);
    assert_eq!(connected_msg, "Heart Rate Monitor connected");

    let disconnected_msg = builder.build(AlertType::SensorDisconnected, &context);
    assert_eq!(disconnected_msg, "Heart Rate Monitor disconnected");
}

#[test]
fn test_cue_builder_zone_change() {
    let builder = CueBuilder::new();
    let context = AlertContext::zone_change("Sweet Spot", 4);

    let message = builder.build(AlertType::PowerZoneChange, &context);
    assert!(message.contains("4"));
    assert!(message.contains("Sweet Spot"));
}

#[test]
fn test_cue_builder_custom_template() {
    let mut builder = CueBuilder::new();

    // Set a custom template
    builder.set_template(AlertType::WorkoutStart, CueTemplate::simple("Let's go!"));

    let context = AlertContext::simple();
    let message = builder.build(AlertType::WorkoutStart, &context);
    assert_eq!(message, "Let's go!");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_empty_interval_name() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: String::new(), // Empty name
        target_power: Some(200),
        duration_secs: 60,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let messages = alert_manager.get_messages();
    assert_eq!(messages.len(), 1);
    // Should still produce a message, just with empty interval name
    assert!(messages[0].contains("interval"));
}

#[tokio::test]
async fn test_zero_duration() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Quick".to_string(),
        target_power: Some(200),
        duration_secs: 0, // Zero duration
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let messages = alert_manager.get_messages();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("0 seconds"));
}

#[tokio::test]
async fn test_very_high_power() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Sprint".to_string(),
        target_power: Some(1500), // Very high power
        duration_secs: 10,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let messages = alert_manager.get_messages();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("1500 watts"));
}

#[tokio::test]
async fn test_concurrent_events() {
    let (bridge, alert_manager) = create_bridge();

    // Process many events in sequence
    let mut events = Vec::new();
    for i in 0..100 {
        events.push(WorkoutEvent::IntervalCountdown {
            seconds_remaining: (i % 10) + 1,
        });
    }

    bridge.process_events(&events).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 100, "Should process all 100 events");
}

// ============================================================================
// Alert Context Data Tests
// ============================================================================

#[tokio::test]
async fn test_interval_change_context_data() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Test Interval".to_string(),
        target_power: Some(250),
        duration_secs: 180,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    match &alerts[0].1.data {
        AlertData::IntervalChange {
            new_interval_name,
            target_power,
            duration_secs,
        } => {
            assert_eq!(new_interval_name, "Test Interval");
            assert_eq!(*target_power, Some(250));
            assert_eq!(*duration_secs, 180);
        }
        _ => panic!("Expected IntervalChange data"),
    }
}

#[tokio::test]
async fn test_countdown_context_data() {
    let (bridge, alert_manager) = create_bridge();

    let event = WorkoutEvent::IntervalCountdown {
        seconds_remaining: 7,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    match &alerts[0].1.data {
        AlertData::Countdown { seconds_remaining } => {
            assert_eq!(*seconds_remaining, 7);
        }
        _ => panic!("Expected Countdown data"),
    }
}
