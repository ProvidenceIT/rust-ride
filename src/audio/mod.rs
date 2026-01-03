//! Audio & Voice Alerts Module
//!
//! Provides audio cues and voice alerts for workouts and training zones.
//!
//! T077: ToneGenerator for audio cues
//! T078: Tone frequencies and patterns
//! T081: Zone change cues
//! T082: ZoneChangeDetector with debouncing

pub mod alerts;
pub mod backend;
pub mod cues;
pub mod engine;
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
}

fn default_countdown_enabled() -> bool {
    true
}

fn default_countdown_volume() -> u8 {
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
        }
    }

    /// Create a high-priority speech item
    pub fn urgent_speech(text: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::Speech { text: text.into() },
            priority: AudioPriority::High,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
        }
    }

    /// Create a sound effect item
    pub fn sound(name: impl Into<String>) -> Self {
        Self {
            audio_type: AudioType::SoundEffect { name: name.into() },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
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
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: AudioPriority) -> Self {
        self.priority = priority;
        self
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

        let deserialized: AudioConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.countdown_enabled);
        assert_eq!(deserialized.countdown_volume, 100);
    }

    #[test]
    fn test_audio_config_serde_with_defaults() {
        // Test deserializing without the new countdown fields (backward compatibility)
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
        // Should use defaults for missing countdown fields
        assert!(config.countdown_enabled);
        assert_eq!(config.countdown_volume, 100);
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
}
