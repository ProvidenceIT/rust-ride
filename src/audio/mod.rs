//! Audio & Voice Alerts Module
//!
//! Provides audio cues and voice alerts for workouts and training zones.
//!
//! T077: ToneGenerator for audio cues
//! T078: Tone frequencies and patterns
//! T081: Zone change cues
//! T082: ZoneChangeDetector with debouncing

pub mod achievement_bridge;
pub mod alerts;
pub mod backend;
pub mod cues;
pub mod engine;
pub mod milestone_bridge;
pub mod sounds;
pub mod tones;
pub mod tts;
pub mod workout_bridge;

use std::time::Duration;
use thiserror::Error;

// Re-export main types
pub use alerts::{
    AlertCategory, AlertConfig, AlertContext, AlertData, AlertManager, AlertType,
    DefaultAlertManager,
};
pub use backend::{BackendError, BackendState, CachedSound, RodioAudioBackend};
pub use cues::{CueBuilder, CueTemplate};
pub use engine::{AudioEngine, DefaultAudioEngine};
pub use tones::{
    CuePattern, Tone, ToneError, ToneGenerator, ZoneChange, ZoneChangeDetector, ZoneDirection,
};
pub use tts::{DefaultTtsProvider, ThreadSafeTtsProvider, TtsProvider, VoiceInfo};
pub use sounds::{SoundAsset, SoundCatalog, SoundCategory, SoundDefinition};
pub use workout_bridge::{WorkoutAudioBridge, WorkoutAudioBridgeConfig};
pub use achievement_bridge::{AchievementAudioBridge, AchievementAudioBridgeConfig};
pub use milestone_bridge::{MilestoneAudioBridge, MilestoneAudioBridgeConfig, MilestoneData, MilestoneType};

// Volume control types
// AudioCategory is re-exported through the enum defined in this module

/// Errors that can occur during audio operations
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Audio device not available")]
    DeviceNotAvailable,

    #[error("TTS initialization failed: {0}")]
    TtsInitFailed(String),

    #[error("Sound file not found: {0}")]
    SoundNotFound(String),

    #[error("Playback failed: {0}")]
    PlaybackFailed(String),

    #[error("Voice not available: {0}")]
    VoiceNotAvailable(String),
}

/// Audio configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    /// Master enable for all audio
    pub enabled: bool,
    /// Master volume (0-100)
    pub volume: u8,
    /// Enable voice/TTS
    pub voice_enabled: bool,
    /// Voice volume (0-100)
    pub voice_volume: u8,
    /// Preferred voice name (OS-dependent)
    pub preferred_voice: Option<String>,
    /// Speech rate multiplier (0.5 - 2.0)
    pub speech_rate: f32,
    /// Enable sound effects
    pub sound_effects_enabled: bool,
    /// Sound effects volume (0-100)
    pub sound_effects_volume: u8,
    /// Minimum interval between alerts (prevents spam)
    pub min_alert_interval_ms: u32,
    /// Enable countdown sounds and voice announcements
    #[serde(default = "default_countdown_enabled")]
    pub countdown_enabled: bool,
    /// Volume for countdown sounds (0-100), separate from master and speech volume
    #[serde(default = "default_countdown_volume")]
    pub countdown_volume: u8,
    /// Enable milestone sounds (distance, time, calorie)
    #[serde(default = "default_milestones_enabled")]
    pub milestones_enabled: bool,
    /// Volume for milestone sounds (0-100), separate from master volume
    #[serde(default = "default_milestone_volume")]
    pub milestone_volume: u8,
    /// Enable personal record sounds
    #[serde(default = "default_pr_enabled")]
    pub personal_record_sounds_enabled: bool,
    /// Enable achievement sounds
    #[serde(default = "default_achievement_enabled")]
    pub achievements_enabled: bool,
    /// Volume for achievement sounds (0-100), separate from master volume
    #[serde(default = "default_achievement_volume")]
    pub achievement_volume: u8,
}

fn default_countdown_enabled() -> bool {
    true
}

fn default_countdown_volume() -> u8 {
    100
}

fn default_milestones_enabled() -> bool {
    true
}

fn default_milestone_volume() -> u8 {
    70 // Milestones are subtle, 70% of normal volume
}

fn default_pr_enabled() -> bool {
    true
}

fn default_achievement_enabled() -> bool {
    true
}

fn default_achievement_volume() -> u8 {
    100
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 80,
            voice_enabled: true,
            voice_volume: 100,
            preferred_voice: None,
            speech_rate: 1.0,
            sound_effects_enabled: true,
            sound_effects_volume: 80,
            min_alert_interval_ms: 3000,
            countdown_enabled: default_countdown_enabled(),
            countdown_volume: default_countdown_volume(),
            milestones_enabled: default_milestones_enabled(),
            milestone_volume: default_milestone_volume(),
            personal_record_sounds_enabled: default_pr_enabled(),
            achievements_enabled: default_achievement_enabled(),
            achievement_volume: default_achievement_volume(),
        }
    }
}

/// Audio events for monitoring
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Speech started
    SpeechStarted { text: String },
    /// Speech completed
    SpeechCompleted,
    /// Sound effect played
    SoundPlayed { name: String },
    /// Alert triggered
    AlertTriggered { alert_type: AlertType },
    /// Audio error occurred
    Error { message: String },
}

/// Priority levels for audio queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioPriority {
    /// Low priority - can be skipped if queue is full
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority - interrupts lower priority
    High = 2,
    /// Critical - interrupts everything
    Critical = 3,
}

/// Audio categories for per-category volume control
///
/// Each category has its own volume setting that is multiplied with
/// the master volume to calculate the final playback volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AudioCategory {
    /// Voice/TTS announcements
    Voice,
    /// General sound effects
    SoundEffect,
    /// Countdown tones before interval changes
    Countdown,
    /// Achievement unlock chimes
    Achievement,
    /// Milestone progress sounds (distance, time, calories)
    Milestone,
}

impl AudioCategory {
    /// Get the category-specific volume from config (0-100)
    pub fn volume_from_config(&self, config: &AudioConfig) -> u8 {
        match self {
            AudioCategory::Voice => config.voice_volume,
            AudioCategory::SoundEffect => config.sound_effects_volume,
            AudioCategory::Countdown => config.countdown_volume,
            AudioCategory::Achievement => config.achievement_volume,
            AudioCategory::Milestone => config.milestone_volume,
        }
    }

    /// Check if this category is enabled in the config
    pub fn is_enabled(&self, config: &AudioConfig) -> bool {
        if !config.enabled {
            return false;
        }
        match self {
            AudioCategory::Voice => config.voice_enabled,
            AudioCategory::SoundEffect => config.sound_effects_enabled,
            AudioCategory::Countdown => config.countdown_enabled,
            AudioCategory::Achievement => config.achievements_enabled,
            AudioCategory::Milestone => config.milestones_enabled,
        }
    }

    /// Calculate the effective volume (0.0 - 1.0) by combining master and category volumes
    ///
    /// The formula is: (master_volume / 100) * (category_volume / 100)
    /// This ensures both volumes are respected and combined multiplicatively.
    pub fn effective_volume(&self, config: &AudioConfig) -> f32 {
        let master = config.volume as f32 / 100.0;
        let category = self.volume_from_config(config) as f32 / 100.0;
        master * category
    }

    /// Get a human-readable name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            AudioCategory::Voice => "Voice",
            AudioCategory::SoundEffect => "Sound Effects",
            AudioCategory::Countdown => "Countdown",
            AudioCategory::Achievement => "Achievement",
            AudioCategory::Milestone => "Milestone",
        }
    }
}

/// Audio item in the queue
#[derive(Debug, Clone)]
pub struct AudioItem {
    /// Type of audio to play
    pub audio_type: AudioType,
    /// Priority level
    pub priority: AudioPriority,
    /// When this item was queued
    pub queued_at: std::time::Instant,
    /// Maximum time to wait in queue before discarding
    pub max_queue_time: Duration,
    /// Audio category for volume mixing (affects which volume setting is used)
    pub category: Option<AudioCategory>,
}

/// Type of audio to play
#[derive(Debug, Clone)]
pub enum AudioType {
    /// Text to speak
    Speech { text: String },
    /// Sound effect by name
    SoundEffect { name: String },
    /// Tone (frequency, duration)
    Tone { frequency_hz: u32, duration_ms: u32 },
}

impl AudioItem {
    /// Create a speech item
    pub fn speech(text: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::Speech { text: text.into() },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(10),
            category: Some(AudioCategory::Voice),
        }
    }

    /// Create a high-priority speech item
    pub fn urgent_speech(text: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::Speech { text: text.into() },
            priority: AudioPriority::High,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Voice),
        }
    }

    /// Create a sound effect item
    pub fn sound(name: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::SoundEffect { name: name.into() },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::SoundEffect),
        }
    }

    /// Create a tone item
    pub fn tone(frequency_hz: u32, duration_ms: u32) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: None, // Tones use master volume unless overridden
        }
    }

    /// Create a countdown tone item
    pub fn countdown_tone(frequency_hz: u32, duration_ms: u32) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Countdown),
        }
    }

    /// Create an achievement sound item
    pub fn achievement_sound(name: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::SoundEffect { name: name.into() },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Achievement),
        }
    }

    /// Create an achievement tone item
    pub fn achievement_tone(frequency_hz: u32, duration_ms: u32) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Achievement),
        }
    }

    /// Create a milestone sound item
    pub fn milestone_sound(name: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::SoundEffect { name: name.into() },
            priority: AudioPriority::Low, // Milestones are subtle
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Milestone),
        }
    }

    /// Create a milestone tone item
    pub fn milestone_tone(frequency_hz: u32, duration_ms: u32) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Low, // Milestones are subtle
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: Some(AudioCategory::Milestone),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: AudioPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the audio category for volume mixing
    pub fn with_category(mut self, category: AudioCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Get the effective volume for this item given the config
    ///
    /// If a category is set, uses the category-specific volume multiplied by master volume.
    /// Otherwise uses just the master volume.
    pub fn effective_volume(&self, config: &AudioConfig) -> f32 {
        match &self.category {
            Some(category) => category.effective_volume(config),
            None => config.volume as f32 / 100.0,
        }
    }

    /// Check if this audio item is enabled given the config
    pub fn is_enabled(&self, config: &AudioConfig) -> bool {
        if !config.enabled {
            return false;
        }
        match &self.category {
            Some(category) => category.is_enabled(config),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert!(config.enabled);
        assert_eq!(config.volume, 80);
        assert_eq!(config.speech_rate, 1.0);
        assert!(config.countdown_enabled);
        assert_eq!(config.countdown_volume, 100);
        assert!(config.milestones_enabled);
        assert_eq!(config.milestone_volume, 70);
        assert!(config.personal_record_sounds_enabled);
        assert!(config.achievements_enabled);
        assert_eq!(config.achievement_volume, 100);
    }

    #[test]
    fn test_audio_priority_ordering() {
        assert!(AudioPriority::Critical > AudioPriority::High);
        assert!(AudioPriority::High > AudioPriority::Normal);
        assert!(AudioPriority::Normal > AudioPriority::Low);
    }

    #[test]
    fn test_audio_item_creation() {
        let item = AudioItem::speech("Test message");
        assert!(matches!(item.audio_type, AudioType::Speech { .. }));
        assert_eq!(item.priority, AudioPriority::Normal);

        let urgent = AudioItem::urgent_speech("Urgent!");
        assert_eq!(urgent.priority, AudioPriority::High);
    }

    #[test]
    fn test_tone_audio_item_creation() {
        let item = AudioItem::tone(440, 200);
        assert_eq!(item.priority, AudioPriority::Normal);
        match item.audio_type {
            AudioType::Tone {
                frequency_hz,
                duration_ms,
            } => {
                assert_eq!(frequency_hz, 440);
                assert_eq!(duration_ms, 200);
            }
            _ => panic!("Expected Tone type"),
        }
    }

    #[test]
    fn test_tone_with_priority() {
        let item = AudioItem::tone(880, 100).with_priority(AudioPriority::High);
        assert_eq!(item.priority, AudioPriority::High);
        match item.audio_type {
            AudioType::Tone {
                frequency_hz,
                duration_ms,
            } => {
                assert_eq!(frequency_hz, 880);
                assert_eq!(duration_ms, 100);
            }
            _ => panic!("Expected Tone type"),
        }
    }

    #[test]
    fn test_audio_config_serde_serialization() {
        let config = AudioConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("countdown_enabled"));
        assert!(json.contains("countdown_volume"));
        assert!(json.contains("milestones_enabled"));
        assert!(json.contains("milestone_volume"));
        assert!(json.contains("personal_record_sounds_enabled"));

        let deserialized: AudioConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.countdown_enabled);
        assert_eq!(deserialized.countdown_volume, 100);
        assert!(deserialized.milestones_enabled);
        assert_eq!(deserialized.milestone_volume, 70);
        assert!(deserialized.personal_record_sounds_enabled);
    }

    #[test]
    fn test_audio_config_serde_with_defaults() {
        // Test deserializing without the new fields (backward compatibility)
        let json = r#"{
            "enabled": true,
            "volume": 80,
            "voice_enabled": true,
            "voice_volume": 100,
            "preferred_voice": null,
            "speech_rate": 1.0,
            "sound_effects_enabled": true,
            "sound_effects_volume": 80,
            "min_alert_interval_ms": 3000
        }"#;

        let config: AudioConfig = serde_json::from_str(json).unwrap();
        // Should use defaults for missing fields
        assert!(config.countdown_enabled);
        assert_eq!(config.countdown_volume, 100);
        assert!(config.milestones_enabled);
        assert_eq!(config.milestone_volume, 70);
        assert!(config.personal_record_sounds_enabled);
    }

    #[test]
    fn test_audio_config_with_custom_countdown_settings() {
        let json = r#"{
            "enabled": true,
            "volume": 80,
            "voice_enabled": true,
            "voice_volume": 100,
            "preferred_voice": null,
            "speech_rate": 1.0,
            "sound_effects_enabled": true,
            "sound_effects_volume": 80,
            "min_alert_interval_ms": 3000,
            "countdown_enabled": false,
            "countdown_volume": 50
        }"#;

        let config: AudioConfig = serde_json::from_str(json).unwrap();
        assert!(!config.countdown_enabled);
        assert_eq!(config.countdown_volume, 50);
    }

    #[test]
    fn test_audio_config_with_custom_milestone_settings() {
        let json = r#"{
            "enabled": true,
            "volume": 80,
            "voice_enabled": true,
            "voice_volume": 100,
            "preferred_voice": null,
            "speech_rate": 1.0,
            "sound_effects_enabled": true,
            "sound_effects_volume": 80,
            "min_alert_interval_ms": 3000,
            "milestones_enabled": false,
            "milestone_volume": 50,
            "personal_record_sounds_enabled": false
        }"#;

        let config: AudioConfig = serde_json::from_str(json).unwrap();
        assert!(!config.milestones_enabled);
        assert_eq!(config.milestone_volume, 50);
        assert!(!config.personal_record_sounds_enabled);
    }

    #[test]
    fn test_audio_config_with_achievement_settings() {
        let json = r#"{
            "enabled": true,
            "volume": 80,
            "voice_enabled": true,
            "voice_volume": 100,
            "preferred_voice": null,
            "speech_rate": 1.0,
            "sound_effects_enabled": true,
            "sound_effects_volume": 80,
            "min_alert_interval_ms": 3000,
            "achievements_enabled": false,
            "achievement_volume": 60
        }"#;

        let config: AudioConfig = serde_json::from_str(json).unwrap();
        assert!(!config.achievements_enabled);
        assert_eq!(config.achievement_volume, 60);
    }

    // ========== AudioCategory Tests ==========

    #[test]
    fn test_audio_category_volume_from_config() {
        let config = AudioConfig::default();

        assert_eq!(AudioCategory::Voice.volume_from_config(&config), 100);
        assert_eq!(AudioCategory::SoundEffect.volume_from_config(&config), 80);
        assert_eq!(AudioCategory::Countdown.volume_from_config(&config), 100);
        assert_eq!(AudioCategory::Achievement.volume_from_config(&config), 100);
        assert_eq!(AudioCategory::Milestone.volume_from_config(&config), 70);
    }

    #[test]
    fn test_audio_category_is_enabled() {
        let mut config = AudioConfig::default();

        // All should be enabled by default
        assert!(AudioCategory::Voice.is_enabled(&config));
        assert!(AudioCategory::SoundEffect.is_enabled(&config));
        assert!(AudioCategory::Countdown.is_enabled(&config));
        assert!(AudioCategory::Achievement.is_enabled(&config));
        assert!(AudioCategory::Milestone.is_enabled(&config));

        // Disable master - all should be disabled
        config.enabled = false;
        assert!(!AudioCategory::Voice.is_enabled(&config));
        assert!(!AudioCategory::SoundEffect.is_enabled(&config));

        // Re-enable master, disable specific categories
        config.enabled = true;
        config.voice_enabled = false;
        assert!(!AudioCategory::Voice.is_enabled(&config));
        assert!(AudioCategory::SoundEffect.is_enabled(&config));

        config.achievements_enabled = false;
        assert!(!AudioCategory::Achievement.is_enabled(&config));
    }

    #[test]
    fn test_audio_category_effective_volume() {
        let mut config = AudioConfig::default();
        config.volume = 80; // Master volume 80%
        config.voice_volume = 100; // Voice at 100%
        config.sound_effects_volume = 50; // Sound effects at 50%
        config.countdown_volume = 75; // Countdown at 75%

        // Effective volume = (master / 100) * (category / 100)
        // Voice: 0.8 * 1.0 = 0.8
        assert!((AudioCategory::Voice.effective_volume(&config) - 0.8).abs() < 0.001);
        // SoundEffect: 0.8 * 0.5 = 0.4
        assert!((AudioCategory::SoundEffect.effective_volume(&config) - 0.4).abs() < 0.001);
        // Countdown: 0.8 * 0.75 = 0.6
        assert!((AudioCategory::Countdown.effective_volume(&config) - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_audio_category_display_name() {
        assert_eq!(AudioCategory::Voice.display_name(), "Voice");
        assert_eq!(AudioCategory::SoundEffect.display_name(), "Sound Effects");
        assert_eq!(AudioCategory::Countdown.display_name(), "Countdown");
        assert_eq!(AudioCategory::Achievement.display_name(), "Achievement");
        assert_eq!(AudioCategory::Milestone.display_name(), "Milestone");
    }

    // ========== AudioItem Category Tests ==========

    #[test]
    fn test_audio_item_speech_has_voice_category() {
        let item = AudioItem::speech("Test");
        assert_eq!(item.category, Some(AudioCategory::Voice));
    }

    #[test]
    fn test_audio_item_sound_has_sound_effect_category() {
        let item = AudioItem::sound("beep");
        assert_eq!(item.category, Some(AudioCategory::SoundEffect));
    }

    #[test]
    fn test_audio_item_tone_has_no_category() {
        let item = AudioItem::tone(440, 200);
        assert_eq!(item.category, None);
    }

    #[test]
    fn test_audio_item_countdown_tone() {
        let item = AudioItem::countdown_tone(880, 100);
        assert_eq!(item.category, Some(AudioCategory::Countdown));
        match item.audio_type {
            AudioType::Tone { frequency_hz, duration_ms } => {
                assert_eq!(frequency_hz, 880);
                assert_eq!(duration_ms, 100);
            }
            _ => panic!("Expected Tone type"),
        }
    }

    #[test]
    fn test_audio_item_achievement_sound() {
        let item = AudioItem::achievement_sound("chime");
        assert_eq!(item.category, Some(AudioCategory::Achievement));
        match item.audio_type {
            AudioType::SoundEffect { name } => assert_eq!(name, "chime"),
            _ => panic!("Expected SoundEffect type"),
        }
    }

    #[test]
    fn test_audio_item_achievement_tone() {
        let item = AudioItem::achievement_tone(523, 150);
        assert_eq!(item.category, Some(AudioCategory::Achievement));
    }

    #[test]
    fn test_audio_item_milestone_sound() {
        let item = AudioItem::milestone_sound("ding");
        assert_eq!(item.category, Some(AudioCategory::Milestone));
        assert_eq!(item.priority, AudioPriority::Low); // Milestones are subtle
    }

    #[test]
    fn test_audio_item_milestone_tone() {
        let item = AudioItem::milestone_tone(440, 200);
        assert_eq!(item.category, Some(AudioCategory::Milestone));
        assert_eq!(item.priority, AudioPriority::Low);
    }

    #[test]
    fn test_audio_item_with_category() {
        let item = AudioItem::tone(440, 200).with_category(AudioCategory::Countdown);
        assert_eq!(item.category, Some(AudioCategory::Countdown));
    }

    #[test]
    fn test_audio_item_effective_volume() {
        let mut config = AudioConfig::default();
        config.volume = 80; // Master 80%
        config.countdown_volume = 50; // Countdown 50%

        // Item with countdown category should use effective volume
        let countdown_item = AudioItem::countdown_tone(440, 100);
        // 0.8 * 0.5 = 0.4
        assert!((countdown_item.effective_volume(&config) - 0.4).abs() < 0.001);

        // Item without category uses just master volume
        let tone_item = AudioItem::tone(440, 100);
        // 0.8
        assert!((tone_item.effective_volume(&config) - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_audio_item_is_enabled() {
        let mut config = AudioConfig::default();

        // All enabled by default
        let countdown = AudioItem::countdown_tone(440, 100);
        assert!(countdown.is_enabled(&config));

        // Disable countdown
        config.countdown_enabled = false;
        assert!(!countdown.is_enabled(&config));

        // Item without category is always enabled (if master is enabled)
        let tone = AudioItem::tone(440, 100);
        assert!(tone.is_enabled(&config));

        // Disable master
        config.enabled = false;
        assert!(!tone.is_enabled(&config));
    }

    #[test]
    fn test_audio_category_serde() {
        // Test that AudioCategory can be serialized/deserialized
        let category = AudioCategory::Achievement;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"Achievement\"");

        let deserialized: AudioCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AudioCategory::Achievement);
    }
}
