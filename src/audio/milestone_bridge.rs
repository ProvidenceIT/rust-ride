//! Milestone Audio Bridge
//!
//! Bridges milestone events to audio feedback during rides and workouts.
//! Provides subtle but noticeable audio cues for distance, time, and calorie
//! milestones without being intrusive during intense workout efforts.
//!
//! # Audio Behavior
//!
//! - **Distance milestones** (5km, 10km, etc.): Gentle two-note ascending chime
//! - **Time milestones** (15min, 30min, 1hr): Clock-like subtle tick
//! - **Calorie milestones** (100, 250, 500, etc.): Energetic but subtle chime
//! - **Personal records**: More elaborate triumphant fanfare
//!
//! # Design Principles
//!
//! - Sounds are **subtle** - shorter and quieter than achievement sounds
//! - Each milestone type has a **distinct** sound character
//! - Configurable per-type enable/disable
//! - Voice announcements optional and non-intrusive

use crate::audio::alerts::{AlertContext, AlertManager, AlertType};
use crate::audio::engine::AudioEngine;
use crate::audio::tones::CuePattern;
use crate::audio::AudioPriority;
use std::sync::Arc;

/// Type of milestone that can trigger audio feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MilestoneType {
    /// Distance milestone (e.g., 5km, 10km, 25km)
    Distance,
    /// Time milestone (e.g., 15min, 30min, 1hr)
    Time,
    /// Calorie milestone (e.g., 100, 250, 500 kcal)
    Calories,
    /// Personal record achieved
    PersonalRecord,
}

impl MilestoneType {
    /// Get the corresponding CuePattern for this milestone type.
    pub fn cue_pattern(&self) -> CuePattern {
        match self {
            MilestoneType::Distance => CuePattern::MilestoneDistance,
            MilestoneType::Time => CuePattern::MilestoneTime,
            MilestoneType::Calories => CuePattern::MilestoneCalories,
            MilestoneType::PersonalRecord => CuePattern::PersonalRecord,
        }
    }

    /// Get the corresponding AlertType for this milestone.
    pub fn alert_type(&self) -> AlertType {
        match self {
            MilestoneType::Distance => AlertType::DistanceMilestone,
            MilestoneType::Time => AlertType::TimeMilestone,
            MilestoneType::Calories => AlertType::CalorieMilestone,
            MilestoneType::PersonalRecord => AlertType::PersonalRecord,
        }
    }

    /// Get the display name for this milestone type.
    pub fn display_name(&self) -> &'static str {
        match self {
            MilestoneType::Distance => "Distance",
            MilestoneType::Time => "Time",
            MilestoneType::Calories => "Calories",
            MilestoneType::PersonalRecord => "Personal Record",
        }
    }
}

/// Data about a specific milestone that was reached.
#[derive(Debug, Clone)]
pub struct MilestoneData {
    /// Type of milestone
    pub milestone_type: MilestoneType,
    /// The value reached (e.g., 10.0 for 10km)
    pub value: f64,
    /// Unit of the value (e.g., "km", "min", "kcal")
    pub unit: String,
    /// Optional: Previous record value (for personal records)
    pub previous_record: Option<f64>,
}

impl MilestoneData {
    /// Create a distance milestone.
    pub fn distance(value: f64, unit: impl Into<String>) -> Self {
        Self {
            milestone_type: MilestoneType::Distance,
            value,
            unit: unit.into(),
            previous_record: None,
        }
    }

    /// Create a time milestone.
    pub fn time(value: f64, unit: impl Into<String>) -> Self {
        Self {
            milestone_type: MilestoneType::Time,
            value,
            unit: unit.into(),
            previous_record: None,
        }
    }

    /// Create a calorie milestone.
    pub fn calories(value: f64) -> Self {
        Self {
            milestone_type: MilestoneType::Calories,
            value,
            unit: "kcal".into(),
            previous_record: None,
        }
    }

    /// Create a personal record milestone.
    pub fn personal_record(
        value: f64,
        unit: impl Into<String>,
        previous_record: Option<f64>,
    ) -> Self {
        Self {
            milestone_type: MilestoneType::PersonalRecord,
            value,
            unit: unit.into(),
            previous_record,
        }
    }

    /// Format the milestone as a string for announcements.
    pub fn format_announcement(&self) -> String {
        match self.milestone_type {
            MilestoneType::Distance => format!("{:.1} {} reached", self.value, self.unit),
            MilestoneType::Time => {
                if self.unit == "min" || self.unit == "minutes" {
                    let hours = self.value as u32 / 60;
                    let mins = self.value as u32 % 60;
                    if hours > 0 && mins > 0 {
                        format!("{} hours {} minutes", hours, mins)
                    } else if hours > 0 {
                        format!("{} hour{}", hours, if hours > 1 { "s" } else { "" })
                    } else {
                        format!("{} minutes", mins)
                    }
                } else {
                    format!("{:.0} {}", self.value, self.unit)
                }
            }
            MilestoneType::Calories => format!("{:.0} calories burned", self.value),
            MilestoneType::PersonalRecord => {
                if let Some(prev) = self.previous_record {
                    format!(
                        "New personal record! {:.1} {}, beating {:.1}",
                        self.value, self.unit, prev
                    )
                } else {
                    format!("New personal record! {:.1} {}", self.value, self.unit)
                }
            }
        }
    }
}

/// Configuration for the milestone audio bridge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MilestoneAudioBridgeConfig {
    /// Enable distance milestone sounds
    #[serde(default = "default_true")]
    pub distance_sounds_enabled: bool,
    /// Enable distance milestone voice announcements
    #[serde(default = "default_false")]
    pub distance_voice_enabled: bool,

    /// Enable time milestone sounds
    #[serde(default = "default_true")]
    pub time_sounds_enabled: bool,
    /// Enable time milestone voice announcements
    #[serde(default = "default_false")]
    pub time_voice_enabled: bool,

    /// Enable calorie milestone sounds
    #[serde(default = "default_true")]
    pub calories_sounds_enabled: bool,
    /// Enable calorie milestone voice announcements
    #[serde(default = "default_false")]
    pub calories_voice_enabled: bool,

    /// Enable personal record sounds (more celebratory)
    #[serde(default = "default_true")]
    pub pr_sounds_enabled: bool,
    /// Enable personal record voice announcements
    #[serde(default = "default_true")]
    pub pr_voice_enabled: bool,

    /// Volume multiplier for milestone sounds (0.0 - 1.0)
    /// Applied on top of the global volume setting
    #[serde(default = "default_milestone_volume")]
    pub volume_multiplier: f32,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_milestone_volume() -> f32 {
    0.7 // Milestones are subtle, 70% of normal volume
}

impl Default for MilestoneAudioBridgeConfig {
    fn default() -> Self {
        Self {
            distance_sounds_enabled: true,
            distance_voice_enabled: false, // Subtle by default
            time_sounds_enabled: true,
            time_voice_enabled: false, // Subtle by default
            calories_sounds_enabled: true,
            calories_voice_enabled: false, // Subtle by default
            pr_sounds_enabled: true,
            pr_voice_enabled: true, // PRs are special, announce them!
            volume_multiplier: default_milestone_volume(),
        }
    }
}

impl MilestoneAudioBridgeConfig {
    /// Check if sound is enabled for a given milestone type.
    pub fn is_sound_enabled(&self, milestone_type: MilestoneType) -> bool {
        match milestone_type {
            MilestoneType::Distance => self.distance_sounds_enabled,
            MilestoneType::Time => self.time_sounds_enabled,
            MilestoneType::Calories => self.calories_sounds_enabled,
            MilestoneType::PersonalRecord => self.pr_sounds_enabled,
        }
    }

    /// Check if voice is enabled for a given milestone type.
    pub fn is_voice_enabled(&self, milestone_type: MilestoneType) -> bool {
        match milestone_type {
            MilestoneType::Distance => self.distance_voice_enabled,
            MilestoneType::Time => self.time_voice_enabled,
            MilestoneType::Calories => self.calories_voice_enabled,
            MilestoneType::PersonalRecord => self.pr_voice_enabled,
        }
    }

    /// Enable all sounds and voices (for full feedback mode).
    pub fn enable_all(&mut self) {
        self.distance_sounds_enabled = true;
        self.distance_voice_enabled = true;
        self.time_sounds_enabled = true;
        self.time_voice_enabled = true;
        self.calories_sounds_enabled = true;
        self.calories_voice_enabled = true;
        self.pr_sounds_enabled = true;
        self.pr_voice_enabled = true;
    }

    /// Disable all sounds and voices (silent mode).
    pub fn disable_all(&mut self) {
        self.distance_sounds_enabled = false;
        self.distance_voice_enabled = false;
        self.time_sounds_enabled = false;
        self.time_voice_enabled = false;
        self.calories_sounds_enabled = false;
        self.calories_voice_enabled = false;
        self.pr_sounds_enabled = false;
        self.pr_voice_enabled = false;
    }

    /// Enable sounds only (no voice announcements).
    pub fn sounds_only(&mut self) {
        self.distance_sounds_enabled = true;
        self.distance_voice_enabled = false;
        self.time_sounds_enabled = true;
        self.time_voice_enabled = false;
        self.calories_sounds_enabled = true;
        self.calories_voice_enabled = false;
        self.pr_sounds_enabled = true;
        self.pr_voice_enabled = false;
    }
}

/// Bridges milestone events to audio feedback.
///
/// This component handles milestone notifications and triggers appropriate
/// audio (chimes + optional voice announcements) based on configuration.
///
/// # Usage
///
/// ```ignore
/// let bridge = MilestoneAudioBridge::new(alert_manager, audio_engine);
///
/// // When a distance milestone is reached:
/// bridge.handle_milestone(&MilestoneData::distance(10.0, "km")).await;
///
/// // When a personal record is set:
/// bridge.handle_milestone(&MilestoneData::personal_record(42.5, "km", Some(41.2))).await;
/// ```
///
/// # Audio Priority
///
/// - Distance/Time/Calorie milestones: Low priority (can be skipped if busy)
/// - Personal records: High priority (should always play)
pub struct MilestoneAudioBridge<A: AlertManager, E: AudioEngine> {
    /// The alert manager for triggering TTS/voice alerts
    alert_manager: Arc<A>,
    /// The audio engine for playing chime sounds
    audio_engine: Arc<E>,
    /// Configuration for which audio to play
    config: MilestoneAudioBridgeConfig,
}

impl<A: AlertManager, E: AudioEngine> MilestoneAudioBridge<A, E> {
    /// Create a new milestone audio bridge with default configuration.
    pub fn new(alert_manager: Arc<A>, audio_engine: Arc<E>) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config: MilestoneAudioBridgeConfig::default(),
        }
    }

    /// Create a new milestone audio bridge with custom configuration.
    pub fn with_config(
        alert_manager: Arc<A>,
        audio_engine: Arc<E>,
        config: MilestoneAudioBridgeConfig,
    ) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config,
        }
    }

    /// Update the bridge configuration.
    pub fn set_config(&mut self, config: MilestoneAudioBridgeConfig) {
        self.config = config;
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &MilestoneAudioBridgeConfig {
        &self.config
    }

    /// Get the audio priority for a milestone type.
    fn priority_for(&self, milestone_type: MilestoneType) -> AudioPriority {
        match milestone_type {
            // Regular milestones are low priority to avoid interrupting workout
            MilestoneType::Distance | MilestoneType::Time | MilestoneType::Calories => {
                AudioPriority::Low
            }
            // Personal records are special - they should always play
            MilestoneType::PersonalRecord => AudioPriority::High,
        }
    }

    /// Handle a milestone event, triggering appropriate audio.
    ///
    /// This is the main entry point for milestone audio feedback.
    pub async fn handle_milestone(&self, milestone: &MilestoneData) {
        let milestone_type = milestone.milestone_type;

        tracing::debug!(
            "Handling milestone: {:?} = {} {}",
            milestone_type,
            milestone.value,
            milestone.unit
        );

        // Play chime if enabled for this type
        if self.config.is_sound_enabled(milestone_type) {
            self.play_milestone_chime(milestone_type).await;
        }

        // Voice announcement if enabled for this type
        if self.config.is_voice_enabled(milestone_type) {
            self.announce_milestone(milestone).await;
        }
    }

    /// Play the appropriate chime for a milestone type.
    async fn play_milestone_chime(&self, milestone_type: MilestoneType) {
        let pattern = milestone_type.cue_pattern();
        let tones = pattern.tones();
        let priority = self.priority_for(milestone_type);

        tracing::debug!(
            "Playing milestone chime for {:?} ({} tones, priority: {:?})",
            milestone_type,
            tones.len(),
            priority
        );

        for tone in tones {
            if tone.is_pause() {
                tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
            } else {
                if let Err(e) = self
                    .audio_engine
                    .play_tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                    .await
                {
                    tracing::warn!("Failed to play milestone chime tone: {}", e);
                }
            }
        }
    }

    /// Announce a milestone via the alert manager (TTS).
    async fn announce_milestone(&self, milestone: &MilestoneData) {
        let message = milestone.format_announcement();
        tracing::debug!("Announcing milestone: {}", message);

        let alert_type = milestone.milestone_type.alert_type();
        let context = if milestone.milestone_type == MilestoneType::PersonalRecord {
            AlertContext::personal_record(
                "record",
                milestone.value as f32,
                &milestone.unit,
                milestone.previous_record.map(|v| v as f32),
            )
        } else {
            AlertContext::milestone(&milestone.unit, milestone.value as f32, &milestone.unit)
        };

        self.alert_manager.trigger(alert_type, context).await;
    }

    /// Handle a distance milestone (convenience method).
    pub async fn handle_distance_milestone(&self, distance: f64, unit: &str) {
        self.handle_milestone(&MilestoneData::distance(distance, unit))
            .await;
    }

    /// Handle a time milestone (convenience method).
    pub async fn handle_time_milestone(&self, minutes: f64) {
        self.handle_milestone(&MilestoneData::time(minutes, "min"))
            .await;
    }

    /// Handle a calorie milestone (convenience method).
    pub async fn handle_calorie_milestone(&self, calories: f64) {
        self.handle_milestone(&MilestoneData::calories(calories))
            .await;
    }

    /// Handle a personal record (convenience method).
    pub async fn handle_personal_record(&self, value: f64, unit: &str, previous: Option<f64>) {
        self.handle_milestone(&MilestoneData::personal_record(value, unit, previous))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::alerts::{AlertConfig, AlertData};
    use crate::audio::{AudioError, AudioEvent, AudioItem};
    use std::sync::Mutex;
    use tokio::sync::broadcast;

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

    /// Mock audio engine for testing
    struct MockAudioEngine {
        played_tones: Mutex<Vec<(u32, u32)>>,
        played_sounds: Mutex<Vec<String>>,
        event_tx: broadcast::Sender<AudioEvent>,
    }

    impl MockAudioEngine {
        fn new() -> Self {
            let (event_tx, _) = broadcast::channel(100);
            Self {
                played_tones: Mutex::new(Vec::new()),
                played_sounds: Mutex::new(Vec::new()),
                event_tx,
            }
        }

        fn get_played_tones(&self) -> Vec<(u32, u32)> {
            self.played_tones.lock().unwrap().clone()
        }
    }

    impl AudioEngine for MockAudioEngine {
        fn initialize(&self) -> Result<(), AudioError> {
            Ok(())
        }

        async fn play_sound(&self, name: &str) -> Result<(), AudioError> {
            self.played_sounds.lock().unwrap().push(name.to_string());
            Ok(())
        }

        async fn speak(&self, _text: &str) -> Result<(), AudioError> {
            Ok(())
        }

        async fn play_tone(&self, frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
            self.played_tones
                .lock()
                .unwrap()
                .push((frequency_hz, duration_ms));
            Ok(())
        }

        fn set_volume(&self, _volume: u8) {}

        fn get_volume(&self) -> u8 {
            80
        }

        fn queue(&self, _item: AudioItem) {}

        fn is_playing(&self) -> bool {
            false
        }

        fn stop(&self) {}

        fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
            self.event_tx.subscribe()
        }
    }

    #[test]
    fn test_default_config() {
        let config = MilestoneAudioBridgeConfig::default();

        // Sounds should be enabled by default
        assert!(config.distance_sounds_enabled);
        assert!(config.time_sounds_enabled);
        assert!(config.calories_sounds_enabled);
        assert!(config.pr_sounds_enabled);

        // Regular milestone voices should be disabled (subtle)
        assert!(!config.distance_voice_enabled);
        assert!(!config.time_voice_enabled);
        assert!(!config.calories_voice_enabled);

        // But PR voice should be enabled
        assert!(config.pr_voice_enabled);

        // Volume should be 0.7
        assert!((config.volume_multiplier - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_config_serialization() {
        let config = MilestoneAudioBridgeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("distance_sounds_enabled"));
        assert!(json.contains("pr_voice_enabled"));
        assert!(json.contains("volume_multiplier"));

        let deserialized: MilestoneAudioBridgeConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.distance_sounds_enabled);
        assert!(deserialized.pr_voice_enabled);
    }

    #[test]
    fn test_config_deserialization_with_defaults() {
        // Test backward compatibility - deserializing without all fields
        let json = r#"{}"#;
        let config: MilestoneAudioBridgeConfig = serde_json::from_str(json).unwrap();

        assert!(config.distance_sounds_enabled);
        assert!(!config.distance_voice_enabled);
        assert!(config.pr_voice_enabled);
        assert!((config.volume_multiplier - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_config_is_sound_enabled() {
        let config = MilestoneAudioBridgeConfig::default();

        assert!(config.is_sound_enabled(MilestoneType::Distance));
        assert!(config.is_sound_enabled(MilestoneType::Time));
        assert!(config.is_sound_enabled(MilestoneType::Calories));
        assert!(config.is_sound_enabled(MilestoneType::PersonalRecord));
    }

    #[test]
    fn test_config_is_voice_enabled() {
        let config = MilestoneAudioBridgeConfig::default();

        assert!(!config.is_voice_enabled(MilestoneType::Distance));
        assert!(!config.is_voice_enabled(MilestoneType::Time));
        assert!(!config.is_voice_enabled(MilestoneType::Calories));
        assert!(config.is_voice_enabled(MilestoneType::PersonalRecord));
    }

    #[test]
    fn test_config_enable_all() {
        let mut config = MilestoneAudioBridgeConfig::default();
        config.enable_all();

        assert!(config.distance_sounds_enabled);
        assert!(config.distance_voice_enabled);
        assert!(config.time_sounds_enabled);
        assert!(config.time_voice_enabled);
        assert!(config.calories_sounds_enabled);
        assert!(config.calories_voice_enabled);
        assert!(config.pr_sounds_enabled);
        assert!(config.pr_voice_enabled);
    }

    #[test]
    fn test_config_disable_all() {
        let mut config = MilestoneAudioBridgeConfig::default();
        config.disable_all();

        assert!(!config.distance_sounds_enabled);
        assert!(!config.distance_voice_enabled);
        assert!(!config.time_sounds_enabled);
        assert!(!config.time_voice_enabled);
        assert!(!config.calories_sounds_enabled);
        assert!(!config.calories_voice_enabled);
        assert!(!config.pr_sounds_enabled);
        assert!(!config.pr_voice_enabled);
    }

    #[test]
    fn test_config_sounds_only() {
        let mut config = MilestoneAudioBridgeConfig::default();
        config.sounds_only();

        assert!(config.distance_sounds_enabled);
        assert!(!config.distance_voice_enabled);
        assert!(config.time_sounds_enabled);
        assert!(!config.time_voice_enabled);
        assert!(config.calories_sounds_enabled);
        assert!(!config.calories_voice_enabled);
        assert!(config.pr_sounds_enabled);
        assert!(!config.pr_voice_enabled);
    }

    #[test]
    fn test_milestone_type_cue_pattern() {
        assert_eq!(
            MilestoneType::Distance.cue_pattern(),
            CuePattern::MilestoneDistance
        );
        assert_eq!(MilestoneType::Time.cue_pattern(), CuePattern::MilestoneTime);
        assert_eq!(
            MilestoneType::Calories.cue_pattern(),
            CuePattern::MilestoneCalories
        );
        assert_eq!(
            MilestoneType::PersonalRecord.cue_pattern(),
            CuePattern::PersonalRecord
        );
    }

    #[test]
    fn test_milestone_type_alert_type() {
        assert_eq!(
            MilestoneType::Distance.alert_type(),
            AlertType::DistanceMilestone
        );
        assert_eq!(MilestoneType::Time.alert_type(), AlertType::TimeMilestone);
        assert_eq!(
            MilestoneType::Calories.alert_type(),
            AlertType::CalorieMilestone
        );
        assert_eq!(
            MilestoneType::PersonalRecord.alert_type(),
            AlertType::PersonalRecord
        );
    }

    #[test]
    fn test_milestone_data_creation() {
        let distance = MilestoneData::distance(10.0, "km");
        assert_eq!(distance.milestone_type, MilestoneType::Distance);
        assert_eq!(distance.value, 10.0);
        assert_eq!(distance.unit, "km");

        let time = MilestoneData::time(30.0, "min");
        assert_eq!(time.milestone_type, MilestoneType::Time);
        assert_eq!(time.value, 30.0);

        let calories = MilestoneData::calories(500.0);
        assert_eq!(calories.milestone_type, MilestoneType::Calories);
        assert_eq!(calories.unit, "kcal");

        let pr = MilestoneData::personal_record(42.5, "km", Some(41.2));
        assert_eq!(pr.milestone_type, MilestoneType::PersonalRecord);
        assert_eq!(pr.previous_record, Some(41.2));
    }

    #[test]
    fn test_milestone_format_announcement() {
        let distance = MilestoneData::distance(10.0, "km");
        assert!(distance.format_announcement().contains("10.0 km"));

        let time = MilestoneData::time(90.0, "min");
        let time_msg = time.format_announcement();
        assert!(time_msg.contains("1 hour") && time_msg.contains("30 minutes"));

        let calories = MilestoneData::calories(500.0);
        assert!(calories.format_announcement().contains("500 calories"));

        let pr = MilestoneData::personal_record(42.5, "km", Some(41.2));
        let pr_msg = pr.format_announcement();
        assert!(pr_msg.contains("personal record"));
        assert!(pr_msg.contains("42.5"));
        assert!(pr_msg.contains("41.2"));
    }

    #[tokio::test]
    async fn test_handle_distance_milestone() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        bridge.handle_distance_milestone(10.0, "km").await;

        // Should have played tones
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played milestone tones");
    }

    #[tokio::test]
    async fn test_handle_time_milestone() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        bridge.handle_time_milestone(30.0).await;

        // Should have played tones
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played time milestone tones");
    }

    #[tokio::test]
    async fn test_handle_calorie_milestone() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        bridge.handle_calorie_milestone(500.0).await;

        // Should have played tones
        let tones = audio_engine.get_played_tones();
        assert!(
            !tones.is_empty(),
            "Should have played calorie milestone tones"
        );
    }

    #[tokio::test]
    async fn test_handle_personal_record() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        bridge.handle_personal_record(42.5, "km", Some(41.2)).await;

        // Should have played tones
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played PR tones");

        // PR should also trigger voice alert (enabled by default)
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].0, AlertType::PersonalRecord);
    }

    #[tokio::test]
    async fn test_sounds_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let mut config = MilestoneAudioBridgeConfig::default();
        config.distance_sounds_enabled = false;
        config.distance_voice_enabled = false;

        let bridge =
            MilestoneAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        bridge.handle_distance_milestone(10.0, "km").await;

        // No tones should be played
        let tones = audio_engine.get_played_tones();
        assert!(tones.is_empty(), "No tones when sounds disabled");

        // No alerts either
        let alerts = alert_manager.get_triggered_alerts();
        assert!(alerts.is_empty(), "No alerts when voice disabled");
    }

    #[tokio::test]
    async fn test_voice_enabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let mut config = MilestoneAudioBridgeConfig::default();
        config.distance_voice_enabled = true; // Enable voice for distance

        let bridge =
            MilestoneAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        bridge.handle_distance_milestone(10.0, "km").await;

        // Should have triggered voice alert
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].0, AlertType::DistanceMilestone);
    }

    #[test]
    fn test_bridge_config_update() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let mut bridge = MilestoneAudioBridge::new(alert_manager, audio_engine);

        assert!(bridge.config().distance_sounds_enabled);

        let mut new_config = MilestoneAudioBridgeConfig::default();
        new_config.distance_sounds_enabled = false;
        bridge.set_config(new_config);

        assert!(!bridge.config().distance_sounds_enabled);
    }

    #[test]
    fn test_priority_for_milestone_type() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = MilestoneAudioBridge::new(alert_manager, audio_engine);

        // Regular milestones are low priority
        assert_eq!(
            bridge.priority_for(MilestoneType::Distance),
            AudioPriority::Low
        );
        assert_eq!(bridge.priority_for(MilestoneType::Time), AudioPriority::Low);
        assert_eq!(
            bridge.priority_for(MilestoneType::Calories),
            AudioPriority::Low
        );

        // PRs are high priority
        assert_eq!(
            bridge.priority_for(MilestoneType::PersonalRecord),
            AudioPriority::High
        );
    }

    #[test]
    fn test_time_format_announcement_various_durations() {
        // Less than 60 minutes
        let short = MilestoneData::time(15.0, "min");
        assert!(short.format_announcement().contains("15 minutes"));

        // Exactly 60 minutes
        let hour = MilestoneData::time(60.0, "min");
        assert!(hour.format_announcement().contains("1 hour"));

        // More than 60 minutes
        let long = MilestoneData::time(75.0, "min");
        let msg = long.format_announcement();
        assert!(msg.contains("1 hour"));
        assert!(msg.contains("15 minutes"));

        // Multiple hours
        let very_long = MilestoneData::time(120.0, "min");
        assert!(very_long.format_announcement().contains("2 hours"));
    }
}
