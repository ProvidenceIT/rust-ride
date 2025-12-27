//! Audio & Voice Alerts Module
//!
//! Provides audio cues and voice alerts for workouts and training zones.
//!
//! T077: ToneGenerator for audio cues
//! T078: Tone frequencies and patterns
//! T081: Zone change cues
//! T082: ZoneChangeDetector with debouncing

pub mod alerts;
pub mod cues;
pub mod engine;
pub mod tones;
pub mod tts;

use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;

// Re-export main types
pub use alerts::{
    AlertCategory, AlertConfig, AlertContext, AlertData, AlertManager, AlertType,
    DefaultAlertManager,
};
pub use cues::{CueBuilder, CueTemplate};
pub use engine::{AudioEngine, DefaultAudioEngine};
pub use tones::{
    CuePattern, Tone, ToneError, ToneGenerator, ZoneChange, ZoneChangeDetector, ZoneDirection,
};
pub use tts::{DefaultTtsProvider, TtsProvider, VoiceInfo};

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
}
