//! Alert Types and Configuration
//!
//! Defines the various alert types and their configuration.

use super::engine::AudioEngine;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Types of alerts that can be triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertType {
    // Workout alerts
    /// Workout starting
    WorkoutStart,
    /// Interval transition
    IntervalChange,
    /// Countdown before interval change (e.g., "10 seconds")
    IntervalCountdown,
    /// Workout complete
    WorkoutComplete,
    /// Recovery interval
    RecoveryStart,

    // Power zone alerts
    /// Entered new power zone
    PowerZoneChange,
    /// Power too high for target
    PowerTooHigh,
    /// Power too low for target
    PowerTooLow,
    /// Power target achieved
    PowerOnTarget,

    // Heart rate alerts
    /// Entered new HR zone
    HeartRateZoneChange,
    /// HR too high
    HeartRateTooHigh,
    /// HR below target
    HeartRateTooLow,

    // Cadence alerts
    /// Cadence too low
    CadenceTooLow,
    /// Cadence too high
    CadenceTooHigh,

    // Milestone alerts
    /// Distance milestone (every 5km, 10km, etc.)
    DistanceMilestone,
    /// Time milestone (every 15 min, 30 min, etc.)
    TimeMilestone,
    /// Calorie milestone
    CalorieMilestone,

    // Sensor alerts
    /// Sensor connected
    SensorConnected,
    /// Sensor disconnected
    SensorDisconnected,
    /// Low battery on sensor
    SensorLowBattery,

    // Achievement alerts
    /// Personal record achieved
    PersonalRecord,
    /// Achievement unlocked
    AchievementUnlocked,

    // General
    /// Lap marker added
    LapMarker,
    /// Ride paused
    RidePaused,
    /// Ride resumed
    RideResumed,
}

impl AlertType {
    /// Get default enabled status for this alert type
    pub fn default_enabled(&self) -> bool {
        match self {
            // Always important
            AlertType::WorkoutStart
            | AlertType::IntervalChange
            | AlertType::WorkoutComplete
            | AlertType::SensorDisconnected => true,

            // Generally useful
            AlertType::PowerZoneChange
            | AlertType::HeartRateZoneChange
            | AlertType::DistanceMilestone
            | AlertType::PersonalRecord => true,

            // Optional by default
            AlertType::IntervalCountdown
            | AlertType::RecoveryStart
            | AlertType::PowerTooHigh
            | AlertType::PowerTooLow
            | AlertType::PowerOnTarget
            | AlertType::HeartRateTooHigh
            | AlertType::HeartRateTooLow
            | AlertType::CadenceTooLow
            | AlertType::CadenceTooHigh
            | AlertType::TimeMilestone
            | AlertType::CalorieMilestone
            | AlertType::SensorConnected
            | AlertType::SensorLowBattery
            | AlertType::AchievementUnlocked
            | AlertType::LapMarker
            | AlertType::RidePaused
            | AlertType::RideResumed => false,
        }
    }

    /// Get display name for this alert type
    pub fn display_name(&self) -> &'static str {
        match self {
            AlertType::WorkoutStart => "Workout Start",
            AlertType::IntervalChange => "Interval Changes",
            AlertType::IntervalCountdown => "Interval Countdown",
            AlertType::WorkoutComplete => "Workout Complete",
            AlertType::RecoveryStart => "Recovery Intervals",
            AlertType::PowerZoneChange => "Power Zone Changes",
            AlertType::PowerTooHigh => "Power Too High",
            AlertType::PowerTooLow => "Power Too Low",
            AlertType::PowerOnTarget => "Power On Target",
            AlertType::HeartRateZoneChange => "Heart Rate Zone Changes",
            AlertType::HeartRateTooHigh => "Heart Rate Too High",
            AlertType::HeartRateTooLow => "Heart Rate Too Low",
            AlertType::CadenceTooLow => "Cadence Too Low",
            AlertType::CadenceTooHigh => "Cadence Too High",
            AlertType::DistanceMilestone => "Distance Milestones",
            AlertType::TimeMilestone => "Time Milestones",
            AlertType::CalorieMilestone => "Calorie Milestones",
            AlertType::SensorConnected => "Sensor Connected",
            AlertType::SensorDisconnected => "Sensor Disconnected",
            AlertType::SensorLowBattery => "Sensor Low Battery",
            AlertType::PersonalRecord => "Personal Records",
            AlertType::AchievementUnlocked => "Achievements",
            AlertType::LapMarker => "Lap Markers",
            AlertType::RidePaused => "Ride Paused",
            AlertType::RideResumed => "Ride Resumed",
        }
    }

    /// Get category for grouping in UI
    pub fn category(&self) -> AlertCategory {
        match self {
            AlertType::WorkoutStart
            | AlertType::IntervalChange
            | AlertType::IntervalCountdown
            | AlertType::WorkoutComplete
            | AlertType::RecoveryStart => AlertCategory::Workout,

            AlertType::PowerZoneChange
            | AlertType::PowerTooHigh
            | AlertType::PowerTooLow
            | AlertType::PowerOnTarget => AlertCategory::Power,

            AlertType::HeartRateZoneChange
            | AlertType::HeartRateTooHigh
            | AlertType::HeartRateTooLow => AlertCategory::HeartRate,

            AlertType::CadenceTooLow | AlertType::CadenceTooHigh => AlertCategory::Cadence,

            AlertType::DistanceMilestone
            | AlertType::TimeMilestone
            | AlertType::CalorieMilestone => AlertCategory::Milestones,

            AlertType::SensorConnected
            | AlertType::SensorDisconnected
            | AlertType::SensorLowBattery => AlertCategory::Sensors,

            AlertType::PersonalRecord | AlertType::AchievementUnlocked => {
                AlertCategory::Achievements
            }

            AlertType::LapMarker | AlertType::RidePaused | AlertType::RideResumed => {
                AlertCategory::General
            }
        }
    }
}

/// Categories for grouping alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertCategory {
    Workout,
    Power,
    HeartRate,
    Cadence,
    Milestones,
    Sensors,
    Achievements,
    General,
}

impl AlertCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            AlertCategory::Workout => "Workout",
            AlertCategory::Power => "Power",
            AlertCategory::HeartRate => "Heart Rate",
            AlertCategory::Cadence => "Cadence",
            AlertCategory::Milestones => "Milestones",
            AlertCategory::Sensors => "Sensors",
            AlertCategory::Achievements => "Achievements",
            AlertCategory::General => "General",
        }
    }
}

/// Configuration for a specific alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Whether this alert is enabled
    pub enabled: bool,
    /// Use voice for this alert
    pub use_voice: bool,
    /// Play sound effect for this alert
    pub play_sound: bool,
    /// Sound effect name (if play_sound is true)
    pub sound_name: Option<String>,
    /// Cooldown before this alert can trigger again
    pub cooldown_secs: u32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_voice: true,
            play_sound: true,
            sound_name: None,
            cooldown_secs: 5,
        }
    }
}

/// Trait for managing alerts
pub trait AlertManager: Send + Sync {
    /// Trigger an alert
    fn trigger(
        &self,
        alert_type: AlertType,
        context: AlertContext,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Configure an alert type
    fn configure(&self, alert_type: AlertType, config: AlertConfig);

    /// Get configuration for an alert type
    fn get_config(&self, alert_type: AlertType) -> AlertConfig;

    /// Enable/disable an alert type
    fn set_enabled(&self, alert_type: AlertType, enabled: bool);

    /// Check if an alert type is on cooldown
    fn is_on_cooldown(&self, alert_type: AlertType) -> bool;
}

/// Context data for an alert
#[derive(Debug, Clone)]
pub struct AlertContext {
    /// Alert-specific data
    pub data: AlertData,
    /// When the alert was triggered
    pub timestamp: Instant,
}

/// Alert-specific data
#[derive(Debug, Clone)]
pub enum AlertData {
    /// No additional data
    None,
    /// Interval change data
    IntervalChange {
        new_interval_name: String,
        target_power: Option<u16>,
        duration_secs: u32,
    },
    /// Countdown data
    Countdown { seconds_remaining: u32 },
    /// Zone change data
    ZoneChange { zone_name: String, zone_number: u8 },
    /// Milestone data
    Milestone {
        metric_name: String,
        value: f32,
        unit: String,
    },
    /// Sensor data
    Sensor {
        sensor_name: String,
        sensor_type: String,
    },
    /// Personal record data
    PersonalRecord {
        record_type: String,
        value: f32,
        unit: String,
        previous_value: Option<f32>,
    },
}

impl AlertContext {
    /// Create a simple alert context with no data
    pub fn simple() -> Self {
        Self {
            data: AlertData::None,
            timestamp: Instant::now(),
        }
    }

    /// Create an interval change context
    pub fn interval_change(
        name: impl Into<String>,
        target_power: Option<u16>,
        duration_secs: u32,
    ) -> Self {
        Self {
            data: AlertData::IntervalChange {
                new_interval_name: name.into(),
                target_power,
                duration_secs,
            },
            timestamp: Instant::now(),
        }
    }

    /// Create a zone change context
    pub fn zone_change(name: impl Into<String>, zone_number: u8) -> Self {
        Self {
            data: AlertData::ZoneChange {
                zone_name: name.into(),
                zone_number,
            },
            timestamp: Instant::now(),
        }
    }

    /// Create a milestone context
    pub fn milestone(metric: impl Into<String>, value: f32, unit: impl Into<String>) -> Self {
        Self {
            data: AlertData::Milestone {
                metric_name: metric.into(),
                value,
                unit: unit.into(),
            },
            timestamp: Instant::now(),
        }
    }

    /// Create a countdown context
    pub fn countdown(seconds_remaining: u32) -> Self {
        Self {
            data: AlertData::Countdown { seconds_remaining },
            timestamp: Instant::now(),
        }
    }

    /// Create a sensor context
    pub fn sensor(name: impl Into<String>, sensor_type: impl Into<String>) -> Self {
        Self {
            data: AlertData::Sensor {
                sensor_name: name.into(),
                sensor_type: sensor_type.into(),
            },
            timestamp: Instant::now(),
        }
    }

    /// Create a personal record context
    pub fn personal_record(
        record_type: impl Into<String>,
        value: f32,
        unit: impl Into<String>,
        previous_value: Option<f32>,
    ) -> Self {
        Self {
            data: AlertData::PersonalRecord {
                record_type: record_type.into(),
                value,
                unit: unit.into(),
                previous_value,
            },
            timestamp: Instant::now(),
        }
    }
}

/// Default implementation of AlertManager
pub struct DefaultAlertManager {
    /// Per-alert configurations
    configs: std::sync::RwLock<std::collections::HashMap<AlertType, AlertConfig>>,
    /// Cooldown tracking - when each alert type was last triggered
    last_triggered: std::sync::RwLock<std::collections::HashMap<AlertType, Instant>>,
    /// Audio engine for playback
    audio_engine: std::sync::Arc<super::engine::DefaultAudioEngine>,
    /// Cue builder for message generation
    cue_builder: std::sync::RwLock<super::cues::CueBuilder>,
}

impl DefaultAlertManager {
    /// Create a new alert manager with the given audio engine
    pub fn new(audio_engine: std::sync::Arc<super::engine::DefaultAudioEngine>) -> Self {
        // Initialize with default configs for each alert type
        let mut configs = std::collections::HashMap::new();
        for alert_type in Self::all_alert_types() {
            let mut config = AlertConfig::default();
            config.enabled = alert_type.default_enabled();
            configs.insert(alert_type, config);
        }

        Self {
            configs: std::sync::RwLock::new(configs),
            last_triggered: std::sync::RwLock::new(std::collections::HashMap::new()),
            audio_engine,
            cue_builder: std::sync::RwLock::new(super::cues::CueBuilder::new()),
        }
    }

    /// Get all alert types
    fn all_alert_types() -> Vec<AlertType> {
        vec![
            AlertType::WorkoutStart,
            AlertType::IntervalChange,
            AlertType::IntervalCountdown,
            AlertType::WorkoutComplete,
            AlertType::RecoveryStart,
            AlertType::PowerZoneChange,
            AlertType::PowerTooHigh,
            AlertType::PowerTooLow,
            AlertType::PowerOnTarget,
            AlertType::HeartRateZoneChange,
            AlertType::HeartRateTooHigh,
            AlertType::HeartRateTooLow,
            AlertType::CadenceTooLow,
            AlertType::CadenceTooHigh,
            AlertType::DistanceMilestone,
            AlertType::TimeMilestone,
            AlertType::CalorieMilestone,
            AlertType::SensorConnected,
            AlertType::SensorDisconnected,
            AlertType::SensorLowBattery,
            AlertType::PersonalRecord,
            AlertType::AchievementUnlocked,
            AlertType::LapMarker,
            AlertType::RidePaused,
            AlertType::RideResumed,
        ]
    }

    /// Check if we should play this alert (respects cooldown)
    fn should_trigger(&self, alert_type: AlertType) -> bool {
        let configs = self.configs.read().unwrap();
        let config = configs.get(&alert_type).cloned().unwrap_or_default();

        if !config.enabled {
            return false;
        }

        // Check cooldown
        let last_triggered = self.last_triggered.read().unwrap();
        if let Some(last_time) = last_triggered.get(&alert_type) {
            let elapsed = last_time.elapsed();
            if elapsed.as_secs() < config.cooldown_secs as u64 {
                tracing::debug!(
                    "Alert {:?} on cooldown ({:.1}s remaining)",
                    alert_type,
                    config.cooldown_secs as f32 - elapsed.as_secs_f32()
                );
                return false;
            }
        }

        true
    }

    /// Record that an alert was triggered
    fn record_trigger(&self, alert_type: AlertType) {
        let mut last_triggered = self.last_triggered.write().unwrap();
        last_triggered.insert(alert_type, Instant::now());
    }

    /// Set a custom cue template for an alert type
    pub fn set_template(&self, alert_type: AlertType, template: super::cues::CueTemplate) {
        let mut builder = self.cue_builder.write().unwrap();
        builder.set_template(alert_type, template);
    }

    /// Get all configurations for serialization
    pub fn get_all_configs(&self) -> std::collections::HashMap<AlertType, AlertConfig> {
        self.configs.read().unwrap().clone()
    }

    /// Load configurations from storage
    pub fn load_configs(&self, configs: std::collections::HashMap<AlertType, AlertConfig>) {
        let mut current = self.configs.write().unwrap();
        for (alert_type, config) in configs {
            current.insert(alert_type, config);
        }
    }
}

impl AlertManager for DefaultAlertManager {
    async fn trigger(&self, alert_type: AlertType, context: AlertContext) {
        if !self.should_trigger(alert_type) {
            return;
        }

        let config = self.get_config(alert_type);

        // Build the message
        let message = {
            let builder = self.cue_builder.read().unwrap();
            builder.build(alert_type, &context)
        };

        tracing::debug!("Triggering alert {:?}: {}", alert_type, message);

        // Record trigger time
        self.record_trigger(alert_type);

        // Play sound effect if configured
        if config.play_sound {
            if let Some(sound_name) = &config.sound_name {
                if let Err(e) = self.audio_engine.play_sound(sound_name).await {
                    tracing::warn!("Failed to play sound for alert: {}", e);
                }
            }
        }

        // Speak the message if configured
        if config.use_voice {
            if let Err(e) = self.audio_engine.speak(&message).await {
                tracing::warn!("Failed to speak alert: {}", e);
            }
        }
    }

    fn configure(&self, alert_type: AlertType, config: AlertConfig) {
        let mut configs = self.configs.write().unwrap();
        configs.insert(alert_type, config);
    }

    fn get_config(&self, alert_type: AlertType) -> AlertConfig {
        let configs = self.configs.read().unwrap();
        configs.get(&alert_type).cloned().unwrap_or_default()
    }

    fn set_enabled(&self, alert_type: AlertType, enabled: bool) {
        let mut configs = self.configs.write().unwrap();
        if let Some(config) = configs.get_mut(&alert_type) {
            config.enabled = enabled;
        } else {
            let mut config = AlertConfig::default();
            config.enabled = enabled;
            configs.insert(alert_type, config);
        }
    }

    fn is_on_cooldown(&self, alert_type: AlertType) -> bool {
        let configs = self.configs.read().unwrap();
        let config = configs.get(&alert_type).cloned().unwrap_or_default();

        let last_triggered = self.last_triggered.read().unwrap();
        if let Some(last_time) = last_triggered.get(&alert_type) {
            let elapsed = last_time.elapsed();
            return elapsed.as_secs() < config.cooldown_secs as u64;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_type_categories() {
        assert_eq!(AlertType::IntervalChange.category(), AlertCategory::Workout);
        assert_eq!(AlertType::PowerZoneChange.category(), AlertCategory::Power);
        assert_eq!(
            AlertType::SensorDisconnected.category(),
            AlertCategory::Sensors
        );
    }

    #[test]
    fn test_alert_context_creation() {
        let ctx = AlertContext::interval_change("Warmup", Some(150), 300);
        match ctx.data {
            AlertData::IntervalChange {
                new_interval_name,
                target_power,
                duration_secs,
            } => {
                assert_eq!(new_interval_name, "Warmup");
                assert_eq!(target_power, Some(150));
                assert_eq!(duration_secs, 300);
            }
            _ => panic!("Wrong alert data type"),
        }
    }
}
