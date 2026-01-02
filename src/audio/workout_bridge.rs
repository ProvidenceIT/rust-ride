//! Workout Audio Bridge
//!
//! Bridges workout events from the WorkoutEngine to the audio alert system.
//! Subscribes to workout events and triggers appropriate voice announcements.

use crate::audio::alerts::{AlertContext, AlertManager, AlertType};
use crate::workouts::types::WorkoutEvent;
use std::sync::Arc;

/// Configuration for the workout audio bridge.
#[derive(Debug, Clone)]
pub struct WorkoutAudioBridgeConfig {
    /// Enable interval change announcements
    pub announce_interval_changes: bool,
    /// Enable countdown announcements
    pub announce_countdowns: bool,
    /// Enable workout start/complete announcements
    pub announce_workout_lifecycle: bool,
    /// Enable trainer disconnect/reconnect announcements
    pub announce_trainer_status: bool,
    /// Enable recovery interval special announcements
    pub announce_recovery_intervals: bool,
}

impl Default for WorkoutAudioBridgeConfig {
    fn default() -> Self {
        Self {
            announce_interval_changes: true,
            announce_countdowns: true,
            announce_workout_lifecycle: true,
            announce_trainer_status: true,
            announce_recovery_intervals: true,
        }
    }
}

/// Bridges workout events to audio alerts.
///
/// This component processes events from the WorkoutEngine and triggers
/// appropriate audio alerts via the AlertManager.
///
/// # Usage
///
/// ```ignore
/// let bridge = WorkoutAudioBridge::new(alert_manager);
///
/// // In your main loop:
/// let events = workout_engine.take_events();
/// bridge.process_events(&events).await;
/// ```
pub struct WorkoutAudioBridge<A: AlertManager> {
    /// The alert manager for triggering audio alerts
    alert_manager: Arc<A>,
    /// Configuration for which events to announce
    config: WorkoutAudioBridgeConfig,
}

impl<A: AlertManager> WorkoutAudioBridge<A> {
    /// Create a new workout audio bridge with the given alert manager.
    pub fn new(alert_manager: Arc<A>) -> Self {
        Self {
            alert_manager,
            config: WorkoutAudioBridgeConfig::default(),
        }
    }

    /// Create a new workout audio bridge with custom configuration.
    pub fn with_config(alert_manager: Arc<A>, config: WorkoutAudioBridgeConfig) -> Self {
        Self {
            alert_manager,
            config,
        }
    }

    /// Update the bridge configuration.
    pub fn set_config(&mut self, config: WorkoutAudioBridgeConfig) {
        self.config = config;
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &WorkoutAudioBridgeConfig {
        &self.config
    }

    /// Process a batch of workout events from the engine.
    ///
    /// Events are processed in order. Each event is mapped to an appropriate
    /// AlertType and AlertContext, then triggered via the AlertManager.
    pub async fn process_events(&self, events: &[WorkoutEvent]) {
        for event in events {
            self.process_event(event).await;
        }
    }

    /// Process a single workout event.
    pub async fn process_event(&self, event: &WorkoutEvent) {
        match event {
            WorkoutEvent::Started { workout_name } => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout start: {}", workout_name);
                    self.alert_manager
                        .trigger(AlertType::WorkoutStart, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::IntervalChange {
                interval_name,
                target_power,
                duration_secs,
                is_recovery,
            } => {
                // Handle recovery intervals specially if enabled
                if *is_recovery && self.config.announce_recovery_intervals {
                    tracing::debug!("Announcing recovery interval: {}", interval_name);
                    self.alert_manager
                        .trigger(AlertType::RecoveryStart, AlertContext::simple())
                        .await;
                } else if self.config.announce_interval_changes {
                    tracing::debug!(
                        "Announcing interval change: {} ({} watts, {} secs)",
                        interval_name,
                        target_power.unwrap_or(0),
                        duration_secs
                    );
                    self.alert_manager
                        .trigger(
                            AlertType::IntervalChange,
                            AlertContext::interval_change(
                                interval_name.clone(),
                                *target_power,
                                *duration_secs,
                            ),
                        )
                        .await;
                }
            }

            WorkoutEvent::IntervalCountdown { seconds_remaining } => {
                if self.config.announce_countdowns {
                    tracing::debug!("Announcing countdown: {} seconds", seconds_remaining);
                    self.alert_manager
                        .trigger(
                            AlertType::IntervalCountdown,
                            AlertContext::countdown(*seconds_remaining),
                        )
                        .await;
                }
            }

            WorkoutEvent::Paused => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout paused");
                    self.alert_manager
                        .trigger(AlertType::RidePaused, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Resumed => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout resumed");
                    self.alert_manager
                        .trigger(AlertType::RideResumed, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Completed {
                total_duration_secs,
            } => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!(
                        "Announcing workout complete (duration: {} secs)",
                        total_duration_secs
                    );
                    self.alert_manager
                        .trigger(AlertType::WorkoutComplete, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Stopped => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout stopped");
                    // Use RidePaused as a reasonable fallback since there's no explicit "stopped" alert type
                    self.alert_manager
                        .trigger(AlertType::RidePaused, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::TrainerDisconnected => {
                if self.config.announce_trainer_status {
                    tracing::debug!("Announcing trainer disconnected");
                    self.alert_manager
                        .trigger(
                            AlertType::SensorDisconnected,
                            AlertContext::sensor("Trainer", "smart_trainer"),
                        )
                        .await;
                }
            }

            WorkoutEvent::TrainerReconnected => {
                if self.config.announce_trainer_status {
                    tracing::debug!("Announcing trainer reconnected");
                    self.alert_manager
                        .trigger(
                            AlertType::SensorConnected,
                            AlertContext::sensor("Trainer", "smart_trainer"),
                        )
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::alerts::{AlertConfig, AlertData};
    use std::sync::Mutex;

    /// Mock alert manager for testing
    struct MockAlertManager {
        triggered_alerts: Mutex<Vec<(AlertType, AlertContext)>>,
        configs: Mutex<std::collections::HashMap<AlertType, AlertConfig>>,
    }

    impl MockAlertManager {
        fn new() -> Self {
            Self {
                triggered_alerts: Mutex::new(Vec::new()),
                configs: Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn get_triggered_alerts(&self) -> Vec<(AlertType, AlertContext)> {
            self.triggered_alerts.lock().unwrap().clone()
        }
    }

    impl AlertManager for MockAlertManager {
        async fn trigger(&self, alert_type: AlertType, context: AlertContext) {
            self.triggered_alerts
                .lock()
                .unwrap()
                .push((alert_type, context));
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

    #[tokio::test]
    async fn test_workout_start_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![WorkoutEvent::Started {
            workout_name: "Test Workout".to_string(),
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::WorkoutStart);
    }

    #[tokio::test]
    async fn test_interval_change_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Sweet Spot".to_string(),
            target_power: Some(260),
            duration_secs: 300,
            is_recovery: false,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::IntervalChange);

        // Verify context data
        match &triggered[0].1.data {
            AlertData::IntervalChange {
                new_interval_name,
                target_power,
                duration_secs,
            } => {
                assert_eq!(new_interval_name, "Sweet Spot");
                assert_eq!(*target_power, Some(260));
                assert_eq!(*duration_secs, 300);
            }
            _ => panic!("Expected IntervalChange data"),
        }
    }

    #[tokio::test]
    async fn test_recovery_interval_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 120,
            is_recovery: true,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::RecoveryStart);
    }

    #[tokio::test]
    async fn test_countdown_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 10,
            },
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 5,
            },
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 3,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 3);

        for (i, (alert_type, context)) in triggered.iter().enumerate() {
            assert_eq!(*alert_type, AlertType::IntervalCountdown);
            match &context.data {
                AlertData::Countdown { seconds_remaining } => {
                    let expected = [10, 5, 3][i];
                    assert_eq!(*seconds_remaining, expected);
                }
                _ => panic!("Expected Countdown data"),
            }
        }
    }

    #[tokio::test]
    async fn test_workout_lifecycle_events() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![
            WorkoutEvent::Paused,
            WorkoutEvent::Resumed,
            WorkoutEvent::Completed {
                total_duration_secs: 3600,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 3);
        assert_eq!(triggered[0].0, AlertType::RidePaused);
        assert_eq!(triggered[1].0, AlertType::RideResumed);
        assert_eq!(triggered[2].0, AlertType::WorkoutComplete);
    }

    #[tokio::test]
    async fn test_trainer_disconnect_events() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone());

        let events = vec![
            WorkoutEvent::TrainerDisconnected,
            WorkoutEvent::TrainerReconnected,
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2);
        assert_eq!(triggered[0].0, AlertType::SensorDisconnected);
        assert_eq!(triggered[1].0, AlertType::SensorConnected);

        // Verify sensor context
        match &triggered[0].1.data {
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
    async fn test_config_disables_announcements() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let config = WorkoutAudioBridgeConfig {
            announce_interval_changes: false,
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_recovery_intervals: false,
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), config);

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

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 0, "No alerts should be triggered when all announcements are disabled");
    }

    #[tokio::test]
    async fn test_selective_config() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let config = WorkoutAudioBridgeConfig {
            announce_interval_changes: true,
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_recovery_intervals: false,
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), config);

        let events = vec![
            WorkoutEvent::Started {
                workout_name: "Test".to_string(),
            },
            WorkoutEvent::IntervalChange {
                interval_name: "Interval 1".to_string(),
                target_power: Some(200),
                duration_secs: 60,
                is_recovery: false,
            },
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 5,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1, "Only interval change should be triggered");
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
    }

    #[tokio::test]
    async fn test_recovery_interval_with_config_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let config = WorkoutAudioBridgeConfig {
            announce_interval_changes: true,
            announce_countdowns: true,
            announce_workout_lifecycle: true,
            announce_trainer_status: true,
            announce_recovery_intervals: false, // Disable recovery-specific announcements
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), config);

        // Recovery interval should fall back to regular interval change
        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 120,
            is_recovery: true,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        // Should be a regular IntervalChange, not RecoveryStart
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
    }

    #[test]
    fn test_default_config() {
        let config = WorkoutAudioBridgeConfig::default();
        assert!(config.announce_interval_changes);
        assert!(config.announce_countdowns);
        assert!(config.announce_workout_lifecycle);
        assert!(config.announce_trainer_status);
        assert!(config.announce_recovery_intervals);
    }
}
