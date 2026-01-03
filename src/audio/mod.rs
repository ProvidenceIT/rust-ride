//! # Audio & Voice Alerts Module
//!
//! Provides comprehensive audio feedback for RustRide workouts, including sound effects,
//! voice announcements, countdown tones, achievement chimes, and milestone celebrations.
//!
//! ## Features
//!
//! - **Countdown Sounds**: Escalating tones for interval transitions (10, 5, 3, 2, 1 seconds)
//! - **Voice Announcements**: TTS for workout instructions and zone changes
//! - **Achievement Chimes**: Tiered celebration sounds (Bronze, Silver, Gold, Platinum)
//! - **Milestone Audio**: Subtle notifications for distance, time, and calorie milestones
//! - **Zone Change Alerts**: Ascending/descending tones for power zone transitions
//! - **Platform Support**: Windows (WASAPI), macOS (CoreAudio), Linux (ALSA/PulseAudio)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rustride::audio::{AudioConfig, AudioEngine, DefaultAudioEngine};
//!
//! // Create and configure the audio engine
//! let config = AudioConfig::default();
//! let engine = DefaultAudioEngine::new(config);
//!
//! // Initialize (connects to audio devices)
//! engine.initialize()?;
//!
//! // Play audio
//! engine.play_tone(440, 200).await?;     // Play a 440Hz tone for 200ms
//! engine.speak("Interval starting").await?;  // TTS announcement
//! engine.play_sound("countdown_tick").await?; // Play sound effect
//! # Ok::<(), rustride::audio::AudioError>(())
//! ```
//!
//! ## Architecture
//!
//! The audio system is organized into layers:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │               Application Bridges                        │
//! │  WorkoutAudioBridge | AchievementAudioBridge | etc.     │
//! ├─────────────────────────────────────────────────────────┤
//! │               Audio Engine (DefaultAudioEngine)          │
//! │  Priority queue, volume mixing, mute control, timing    │
//! ├─────────────────────────────────────────────────────────┤
//! │               Audio Backends                             │
//! │  RodioAudioBackend (tones/sounds) | TtsProvider (voice) │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Configuration
//!
//! Use [`AudioConfig`] to configure the audio system:
//!
//! ```rust
//! use rustride::audio::AudioConfig;
//!
//! let mut config = AudioConfig::default();
//! config.volume = 80;              // Master volume (0-100)
//! config.voice_enabled = true;     // Enable TTS
//! config.countdown_enabled = true; // Enable countdown sounds
//! config.countdown_volume = 100;   // Countdown volume (0-100)
//! config.achievements_enabled = true;
//! config.milestones_enabled = true;
//! config.milestone_volume = 70;    // Milestones are subtle (70%)
//! ```
//!
//! ## Audio Categories
//!
//! Audio is organized into categories for independent volume control:
//!
//! | Category | Description | Default Volume |
//! |----------|-------------|----------------|
//! | [`AudioCategory::Voice`] | TTS announcements | 100% |
//! | [`AudioCategory::SoundEffect`] | General sound effects | 80% |
//! | [`AudioCategory::Countdown`] | Interval countdown tones | 100% |
//! | [`AudioCategory::Achievement`] | Achievement chimes | 100% |
//! | [`AudioCategory::Milestone`] | Milestone notifications | 70% |
//!
//! ## Priority System
//!
//! Audio items have priority levels for queue ordering and interruption:
//!
//! - [`AudioPriority::Critical`] - Interrupts everything (emergency alerts)
//! - [`AudioPriority::High`] - Interrupts normal/low priority (interval changes)
//! - [`AudioPriority::Normal`] - Standard priority (most sounds)
//! - [`AudioPriority::Low`] - Can be dropped if queue is full (milestones)
//!
//! ## Mute Control
//!
//! Both global and per-category muting is supported:
//!
//! ```rust,no_run
//! use rustride::audio::{AudioEngine, AudioCategory, DefaultAudioEngine, AudioConfig};
//!
//! let engine = DefaultAudioEngine::new(AudioConfig::default());
//!
//! // Global mute
//! engine.mute();
//! engine.toggle_mute();
//!
//! // Category-specific mute
//! engine.mute_category(AudioCategory::Milestone);
//! engine.unmute_category(AudioCategory::Milestone);
//!
//! // Check mute state for UI
//! let state = engine.get_mute_state();
//! println!("{}", state.display_string());
//! ```
//!
//! ## Using Audio Bridges
//!
//! Bridges connect application events to audio feedback:
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rustride::audio::{
//!     DefaultAudioEngine, AudioConfig,
//!     WorkoutAudioBridge, WorkoutAudioBridgeConfig,
//! };
//!
//! let engine = Arc::new(DefaultAudioEngine::new(AudioConfig::default()));
//! engine.initialize().ok();
//!
//! let bridge_config = WorkoutAudioBridgeConfig::default();
//! let bridge = WorkoutAudioBridge::new(bridge_config, engine);
//!
//! // Bridge handles workout events and produces appropriate audio
//! ```
//!
//! ## Timing and Synchronization
//!
//! The audio queue includes safeguards for timing-sensitive audio:
//!
//! - Countdown sounds expire after 500ms (must play at correct time)
//! - Regular sounds expire after 3 seconds
//! - Voice/speech expires after 10 seconds
//! - Queue size limits with low-priority dropping
//! - Automatic cleanup of expired items
//!
//! Use [`AudioTimingConfig`] to customize timing behavior.
//!
//! ## Platform-Specific Notes
//!
//! - **Windows**: Uses WASAPI. Ensure audio drivers are current.
//! - **macOS**: Uses CoreAudio. Grant audio permissions if needed.
//! - **Linux**: Uses ALSA/PulseAudio. Install `pulseaudio` and add user to `audio` group.
//!
//! See [`Platform::troubleshooting_hints()`] for platform-specific troubleshooting.
//!
//! ## Module Organization
//!
//! - [`engine`] - AudioEngine trait and DefaultAudioEngine implementation
//! - [`backend`] - RodioAudioBackend for low-level audio playback
//! - [`tones`] - ToneGenerator, CuePattern, and frequency definitions
//! - [`sounds`] - SoundAsset catalog with fallback tones
//! - [`tts`] - Text-to-speech provider
//! - [`alerts`] - Alert types and manager
//! - [`cues`] - Cue templates and builder
//! - [`workout_bridge`] - Workout event to audio mapping
//! - [`achievement_bridge`] - Achievement audio handling
//! - [`milestone_bridge`] - Milestone celebration audio
//!
//! ## Documentation
//!
//! For detailed documentation with examples, see `docs/audio.md`.
//!
//! ### Legacy Task References
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
pub use backend::{
    AudioDeviceError, AudioDeviceStatus, BackendError, BackendState, CachedSound, HotPlugConfig,
    Platform, RodioAudioBackend,
};
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

/// Audio timing configuration for synchronization and queue management
///
/// These settings control how the audio engine handles timing-sensitive audio
/// items like countdown sounds, and how it prevents audio pileup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioTimingConfig {
    /// Maximum number of items allowed in the audio queue
    /// When exceeded, low-priority items will be dropped
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,
    /// Maximum age for countdown sounds before they are dropped (milliseconds)
    /// Countdown sounds need to play at the right time or not at all
    #[serde(default = "default_countdown_max_age_ms")]
    pub countdown_max_age_ms: u64,
    /// Maximum age for regular sounds before they are dropped (milliseconds)
    #[serde(default = "default_sound_max_age_ms")]
    pub sound_max_age_ms: u64,
    /// Maximum age for voice/speech before it is dropped (milliseconds)
    #[serde(default = "default_speech_max_age_ms")]
    pub speech_max_age_ms: u64,
    /// Minimum gap between consecutive audio items (milliseconds)
    /// Helps prevent audio overlap
    #[serde(default = "default_min_audio_gap_ms")]
    pub min_audio_gap_ms: u64,
    /// Enable aggressive cleanup of stale items during queue operations
    #[serde(default = "default_aggressive_cleanup")]
    pub aggressive_cleanup: bool,
    /// Drop low-priority items when queue is over this percentage full (0-100)
    #[serde(default = "default_queue_pressure_threshold")]
    pub queue_pressure_threshold: u8,
}

fn default_max_queue_size() -> usize {
    20
}

fn default_countdown_max_age_ms() -> u64 {
    500 // Countdown sounds must play within 500ms or not at all
}

fn default_sound_max_age_ms() -> u64 {
    3000 // Sound effects can wait up to 3 seconds
}

fn default_speech_max_age_ms() -> u64 {
    10000 // Speech can wait up to 10 seconds
}

fn default_min_audio_gap_ms() -> u64 {
    50 // At least 50ms between audio items
}

fn default_aggressive_cleanup() -> bool {
    true
}

fn default_queue_pressure_threshold() -> u8 {
    70 // Start dropping low-priority items when 70% full
}

impl Default for AudioTimingConfig {
    fn default() -> Self {
        Self {
            max_queue_size: default_max_queue_size(),
            countdown_max_age_ms: default_countdown_max_age_ms(),
            sound_max_age_ms: default_sound_max_age_ms(),
            speech_max_age_ms: default_speech_max_age_ms(),
            min_audio_gap_ms: default_min_audio_gap_ms(),
            aggressive_cleanup: default_aggressive_cleanup(),
            queue_pressure_threshold: default_queue_pressure_threshold(),
        }
    }
}

impl AudioTimingConfig {
    /// Get the maximum queue age for a given audio category
    pub fn max_queue_time_for_category(&self, category: Option<AudioCategory>) -> Duration {
        match category {
            Some(AudioCategory::Countdown) => Duration::from_millis(self.countdown_max_age_ms),
            Some(AudioCategory::Voice) => Duration::from_millis(self.speech_max_age_ms),
            _ => Duration::from_millis(self.sound_max_age_ms),
        }
    }

    /// Check if the queue is under pressure (high fill percentage)
    pub fn is_queue_under_pressure(&self, current_size: usize) -> bool {
        if self.max_queue_size == 0 {
            return false;
        }
        let fill_percentage = (current_size * 100) / self.max_queue_size;
        fill_percentage >= self.queue_pressure_threshold as usize
    }
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
    /// Global mute state (all audio silenced, but volume preserved for unmute)
    #[serde(default)]
    pub muted: bool,
    /// Voice/TTS mute state (separate from enabled - mute preserves volume)
    #[serde(default)]
    pub voice_muted: bool,
    /// Sound effects mute state
    #[serde(default)]
    pub sound_effects_muted: bool,
    /// Countdown mute state
    #[serde(default)]
    pub countdown_muted: bool,
    /// Achievement mute state
    #[serde(default)]
    pub achievement_muted: bool,
    /// Milestone mute state
    #[serde(default)]
    pub milestone_muted: bool,
    /// Timing configuration for audio synchronization
    #[serde(default)]
    pub timing: AudioTimingConfig,
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
            muted: false,
            voice_muted: false,
            sound_effects_muted: false,
            countdown_muted: false,
            achievement_muted: false,
            milestone_muted: false,
            timing: AudioTimingConfig::default(),
        }
    }
}

/// Queue statistics for monitoring and debugging
///
/// Provides insight into the audio queue state for debugging
/// audio timing and pileup issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueStats {
    /// Current number of items in the queue
    pub item_count: usize,
    /// Number of items expired and dropped since last reset
    pub expired_count: usize,
    /// Number of items dropped due to queue pressure
    pub dropped_count: usize,
    /// Number of low-priority items in queue
    pub low_priority_count: usize,
    /// Number of high-priority items in queue
    pub high_priority_count: usize,
    /// Whether queue is currently under pressure
    pub under_pressure: bool,
}

impl Default for QueueStats {
    fn default() -> Self {
        Self {
            item_count: 0,
            expired_count: 0,
            dropped_count: 0,
            low_priority_count: 0,
            high_priority_count: 0,
            under_pressure: false,
        }
    }
}

impl QueueStats {
    /// Check if the queue is healthy (low dropped/expired counts)
    pub fn is_healthy(&self) -> bool {
        self.expired_count == 0 && self.dropped_count == 0 && !self.under_pressure
    }

    /// Get a status string suitable for display
    pub fn status_string(&self) -> String {
        if self.is_healthy() {
            format!("Queue OK ({} items)", self.item_count)
        } else if self.under_pressure {
            format!(
                "Queue PRESSURE ({} items, {} dropped)",
                self.item_count, self.dropped_count
            )
        } else {
            format!(
                "Queue ACTIVE ({} items, {} expired)",
                self.item_count, self.expired_count
            )
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
    /// Audio item expired before playback
    ItemExpired {
        audio_type: String,
        age_ms: u64,
    },
    /// Audio item dropped due to queue pressure
    ItemDropped {
        audio_type: String,
        priority: AudioPriority,
    },
    /// Queue pressure warning
    QueuePressure {
        current_size: usize,
        max_size: usize,
    },
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

    /// Check if this category is muted in the config
    ///
    /// Returns true if either the global mute is on or the category-specific mute is on.
    pub fn is_muted(&self, config: &AudioConfig) -> bool {
        if config.muted {
            return true;
        }
        match self {
            AudioCategory::Voice => config.voice_muted,
            AudioCategory::SoundEffect => config.sound_effects_muted,
            AudioCategory::Countdown => config.countdown_muted,
            AudioCategory::Achievement => config.achievement_muted,
            AudioCategory::Milestone => config.milestone_muted,
        }
    }

    /// Check if audio should play for this category (enabled and not muted)
    pub fn should_play(&self, config: &AudioConfig) -> bool {
        self.is_enabled(config) && !self.is_muted(config)
    }

    /// Calculate the effective volume (0.0 - 1.0) by combining master and category volumes
    ///
    /// The formula is: (master_volume / 100) * (category_volume / 100)
    /// This ensures both volumes are respected and combined multiplicatively.
    /// Returns 0.0 if muted.
    pub fn effective_volume(&self, config: &AudioConfig) -> f32 {
        if self.is_muted(config) {
            return 0.0;
        }
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

    /// Get all audio categories
    pub fn all() -> &'static [AudioCategory] {
        &[
            AudioCategory::Voice,
            AudioCategory::SoundEffect,
            AudioCategory::Countdown,
            AudioCategory::Achievement,
            AudioCategory::Milestone,
        ]
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

    /// Create a countdown tone item with time-critical expiration
    ///
    /// Countdown tones have a very short max_queue_time (500ms by default)
    /// because they must play at the right time or not at all.
    pub fn countdown_tone(frequency_hz: u32, duration_ms: u32) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_millis(500), // Time-critical!
            category: Some(AudioCategory::Countdown),
        }
    }

    /// Create a countdown tone with custom max queue time
    pub fn countdown_tone_with_timing(
        frequency_hz: u32,
        duration_ms: u32,
        max_queue_ms: u64,
    ) -> Self {
        Self {
            audio_type: AudioType::Tone {
                frequency_hz,
                duration_ms,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_millis(max_queue_ms),
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

    /// Check if this audio item should play (enabled and not muted)
    pub fn should_play(&self, config: &AudioConfig) -> bool {
        if !self.is_enabled(config) {
            return false;
        }
        // Check mute state
        if config.muted {
            return false;
        }
        match &self.category {
            Some(category) => !category.is_muted(config),
            None => true,
        }
    }

    // ========== Timing Methods ==========

    /// Check if this audio item has expired
    pub fn is_expired(&self) -> bool {
        self.queued_at.elapsed() >= self.max_queue_time
    }

    /// Get the age of this audio item in milliseconds
    pub fn age_ms(&self) -> u64 {
        self.queued_at.elapsed().as_millis() as u64
    }

    /// Get the remaining time before expiration (None if already expired)
    pub fn time_remaining(&self) -> Option<Duration> {
        let elapsed = self.queued_at.elapsed();
        if elapsed >= self.max_queue_time {
            None
        } else {
            Some(self.max_queue_time - elapsed)
        }
    }

    /// Check if this is a time-critical audio item (countdown)
    pub fn is_time_critical(&self) -> bool {
        matches!(self.category, Some(AudioCategory::Countdown))
    }

    /// Set the maximum queue time
    pub fn with_max_queue_time(mut self, duration: Duration) -> Self {
        self.max_queue_time = duration;
        self
    }

    /// Get a string description of the audio type for logging
    pub fn type_description(&self) -> String {
        match &self.audio_type {
            AudioType::Speech { text } => {
                let preview = if text.len() > 20 {
                    format!("{}...", &text[..20])
                } else {
                    text.clone()
                };
                format!("Speech(\"{}\")", preview)
            }
            AudioType::SoundEffect { name } => format!("Sound({})", name),
            AudioType::Tone {
                frequency_hz,
                duration_ms,
            } => format!("Tone({}Hz, {}ms)", frequency_hz, duration_ms),
        }
    }
}

/// Mute state snapshot for UI display
///
/// Provides a snapshot of the current mute state for all audio categories.
/// This is useful for displaying mute indicators in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MuteState {
    /// Global mute is active (all audio silenced)
    pub globally_muted: bool,
    /// Voice/TTS is muted
    pub voice_muted: bool,
    /// Sound effects are muted
    pub sound_effects_muted: bool,
    /// Countdown sounds are muted
    pub countdown_muted: bool,
    /// Achievement sounds are muted
    pub achievement_muted: bool,
    /// Milestone sounds are muted
    pub milestone_muted: bool,
}

impl MuteState {
    /// Create a mute state from an AudioConfig
    pub fn from_config(config: &AudioConfig) -> Self {
        Self {
            globally_muted: config.muted,
            voice_muted: config.voice_muted,
            sound_effects_muted: config.sound_effects_muted,
            countdown_muted: config.countdown_muted,
            achievement_muted: config.achievement_muted,
            milestone_muted: config.milestone_muted,
        }
    }

    /// Check if any audio is muted (global or any category)
    pub fn any_muted(&self) -> bool {
        self.globally_muted
            || self.voice_muted
            || self.sound_effects_muted
            || self.countdown_muted
            || self.achievement_muted
            || self.milestone_muted
    }

    /// Check if a specific category is effectively muted (global or category-specific)
    pub fn is_category_muted(&self, category: AudioCategory) -> bool {
        if self.globally_muted {
            return true;
        }
        match category {
            AudioCategory::Voice => self.voice_muted,
            AudioCategory::SoundEffect => self.sound_effects_muted,
            AudioCategory::Countdown => self.countdown_muted,
            AudioCategory::Achievement => self.achievement_muted,
            AudioCategory::Milestone => self.milestone_muted,
        }
    }

    /// Get a display string describing the current mute state
    pub fn display_string(&self) -> &'static str {
        if self.globally_muted {
            "All Audio Muted"
        } else if self.any_muted() {
            "Some Audio Muted"
        } else {
            "Audio Active"
        }
    }

    /// Get icon hint for UI (could be used with icon libraries)
    pub fn icon_hint(&self) -> &'static str {
        if self.globally_muted {
            "volume_off"
        } else if self.any_muted() {
            "volume_mute"
        } else {
            "volume_up"
        }
    }
}

impl Default for MuteState {
    fn default() -> Self {
        Self {
            globally_muted: false,
            voice_muted: false,
            sound_effects_muted: false,
            countdown_muted: false,
            achievement_muted: false,
            milestone_muted: false,
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

    // ========== Mute Functionality Tests ==========

    #[test]
    fn test_audio_config_mute_defaults() {
        let config = AudioConfig::default();
        assert!(!config.muted);
        assert!(!config.voice_muted);
        assert!(!config.sound_effects_muted);
        assert!(!config.countdown_muted);
        assert!(!config.achievement_muted);
        assert!(!config.milestone_muted);
    }

    #[test]
    fn test_audio_category_is_muted_global() {
        let mut config = AudioConfig::default();

        // When global mute is off, categories should not be muted
        assert!(!AudioCategory::Voice.is_muted(&config));
        assert!(!AudioCategory::SoundEffect.is_muted(&config));

        // When global mute is on, all categories should be muted
        config.muted = true;
        assert!(AudioCategory::Voice.is_muted(&config));
        assert!(AudioCategory::SoundEffect.is_muted(&config));
        assert!(AudioCategory::Countdown.is_muted(&config));
        assert!(AudioCategory::Achievement.is_muted(&config));
        assert!(AudioCategory::Milestone.is_muted(&config));
    }

    #[test]
    fn test_audio_category_is_muted_per_category() {
        let mut config = AudioConfig::default();

        // Mute only voice
        config.voice_muted = true;
        assert!(AudioCategory::Voice.is_muted(&config));
        assert!(!AudioCategory::SoundEffect.is_muted(&config));

        // Mute countdown
        config.countdown_muted = true;
        assert!(AudioCategory::Countdown.is_muted(&config));
        assert!(!AudioCategory::Achievement.is_muted(&config));
    }

    #[test]
    fn test_audio_category_should_play() {
        let mut config = AudioConfig::default();

        // Should play when enabled and not muted
        assert!(AudioCategory::Voice.should_play(&config));

        // Should not play when disabled
        config.voice_enabled = false;
        assert!(!AudioCategory::Voice.should_play(&config));

        // Should not play when muted (even if enabled)
        config.voice_enabled = true;
        config.voice_muted = true;
        assert!(!AudioCategory::Voice.should_play(&config));

        // Should not play when globally muted
        config.voice_muted = false;
        config.muted = true;
        assert!(!AudioCategory::Voice.should_play(&config));
    }

    #[test]
    fn test_audio_category_effective_volume_when_muted() {
        let mut config = AudioConfig::default();
        config.volume = 80;
        config.voice_volume = 100;

        // Normal effective volume
        let normal_vol = AudioCategory::Voice.effective_volume(&config);
        assert!((normal_vol - 0.8).abs() < 0.001);

        // When muted, effective volume should be 0
        config.voice_muted = true;
        assert_eq!(AudioCategory::Voice.effective_volume(&config), 0.0);

        // When globally muted, also 0
        config.voice_muted = false;
        config.muted = true;
        assert_eq!(AudioCategory::Voice.effective_volume(&config), 0.0);
    }

    #[test]
    fn test_audio_category_all() {
        let all = AudioCategory::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&AudioCategory::Voice));
        assert!(all.contains(&AudioCategory::SoundEffect));
        assert!(all.contains(&AudioCategory::Countdown));
        assert!(all.contains(&AudioCategory::Achievement));
        assert!(all.contains(&AudioCategory::Milestone));
    }

    #[test]
    fn test_audio_item_should_play() {
        let mut config = AudioConfig::default();
        let item = AudioItem::countdown_tone(440, 100);

        // Should play by default
        assert!(item.should_play(&config));

        // Should not play when category is muted
        config.countdown_muted = true;
        assert!(!item.should_play(&config));

        // Should not play when globally muted
        config.countdown_muted = false;
        config.muted = true;
        assert!(!item.should_play(&config));

        // Item without category respects global mute only
        let generic_tone = AudioItem::tone(440, 100);
        assert!(!generic_tone.should_play(&config));

        config.muted = false;
        assert!(generic_tone.should_play(&config));
    }

    #[test]
    fn test_mute_state_from_config() {
        let mut config = AudioConfig::default();
        config.muted = true;
        config.voice_muted = true;
        config.sound_effects_muted = false;

        let mute_state = MuteState::from_config(&config);
        assert!(mute_state.globally_muted);
        assert!(mute_state.voice_muted);
        assert!(!mute_state.sound_effects_muted);
    }

    #[test]
    fn test_mute_state_any_muted() {
        let mut state = MuteState::default();
        assert!(!state.any_muted());

        state.voice_muted = true;
        assert!(state.any_muted());

        state = MuteState::default();
        state.globally_muted = true;
        assert!(state.any_muted());
    }

    #[test]
    fn test_mute_state_is_category_muted() {
        let mut state = MuteState::default();

        // Nothing muted
        assert!(!state.is_category_muted(AudioCategory::Voice));

        // Category-specific mute
        state.voice_muted = true;
        assert!(state.is_category_muted(AudioCategory::Voice));
        assert!(!state.is_category_muted(AudioCategory::SoundEffect));

        // Global mute affects all categories
        state.voice_muted = false;
        state.globally_muted = true;
        assert!(state.is_category_muted(AudioCategory::Voice));
        assert!(state.is_category_muted(AudioCategory::SoundEffect));
    }

    #[test]
    fn test_mute_state_display_string() {
        let mut state = MuteState::default();
        assert_eq!(state.display_string(), "Audio Active");

        state.voice_muted = true;
        assert_eq!(state.display_string(), "Some Audio Muted");

        state.globally_muted = true;
        assert_eq!(state.display_string(), "All Audio Muted");
    }

    #[test]
    fn test_mute_state_icon_hint() {
        let mut state = MuteState::default();
        assert_eq!(state.icon_hint(), "volume_up");

        state.sound_effects_muted = true;
        assert_eq!(state.icon_hint(), "volume_mute");

        state.globally_muted = true;
        assert_eq!(state.icon_hint(), "volume_off");
    }

    #[test]
    fn test_mute_state_serde() {
        let state = MuteState {
            globally_muted: true,
            voice_muted: false,
            sound_effects_muted: true,
            countdown_muted: false,
            achievement_muted: true,
            milestone_muted: false,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: MuteState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_audio_config_mute_serde_backward_compat() {
        // Test deserializing without mute fields (backward compatibility)
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
        // Should use defaults for mute fields
        assert!(!config.muted);
        assert!(!config.voice_muted);
        assert!(!config.sound_effects_muted);
    }

    #[test]
    fn test_audio_config_mute_serde_with_values() {
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
            "muted": true,
            "voice_muted": true,
            "countdown_muted": true
        }"#;

        let config: AudioConfig = serde_json::from_str(json).unwrap();
        assert!(config.muted);
        assert!(config.voice_muted);
        assert!(config.countdown_muted);
        assert!(!config.sound_effects_muted);
    }

    // ========== AudioTimingConfig Tests ==========

    #[test]
    fn test_audio_timing_config_defaults() {
        let timing = AudioTimingConfig::default();
        assert_eq!(timing.max_queue_size, 20);
        assert_eq!(timing.countdown_max_age_ms, 500);
        assert_eq!(timing.sound_max_age_ms, 3000);
        assert_eq!(timing.speech_max_age_ms, 10000);
        assert_eq!(timing.min_audio_gap_ms, 50);
        assert!(timing.aggressive_cleanup);
        assert_eq!(timing.queue_pressure_threshold, 70);
    }

    #[test]
    fn test_audio_timing_config_serde() {
        let timing = AudioTimingConfig::default();
        let json = serde_json::to_string(&timing).unwrap();

        let deserialized: AudioTimingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_queue_size, timing.max_queue_size);
        assert_eq!(deserialized.countdown_max_age_ms, timing.countdown_max_age_ms);
    }

    #[test]
    fn test_audio_timing_config_max_queue_time_for_category() {
        let timing = AudioTimingConfig::default();

        let countdown = timing.max_queue_time_for_category(Some(AudioCategory::Countdown));
        assert_eq!(countdown, Duration::from_millis(500));

        let voice = timing.max_queue_time_for_category(Some(AudioCategory::Voice));
        assert_eq!(voice, Duration::from_millis(10000));

        let sound = timing.max_queue_time_for_category(Some(AudioCategory::SoundEffect));
        assert_eq!(sound, Duration::from_millis(3000));
    }

    #[test]
    fn test_audio_timing_config_queue_pressure() {
        let timing = AudioTimingConfig::default();

        // 70% of 20 = 14
        assert!(!timing.is_queue_under_pressure(13));
        assert!(timing.is_queue_under_pressure(14));
        assert!(timing.is_queue_under_pressure(20));
    }

    #[test]
    fn test_audio_config_includes_timing() {
        let config = AudioConfig::default();
        assert_eq!(config.timing.max_queue_size, 20);
    }

    // ========== QueueStats Tests ==========

    #[test]
    fn test_queue_stats_default() {
        let stats = QueueStats::default();
        assert_eq!(stats.item_count, 0);
        assert_eq!(stats.expired_count, 0);
        assert_eq!(stats.dropped_count, 0);
        assert_eq!(stats.low_priority_count, 0);
        assert_eq!(stats.high_priority_count, 0);
        assert!(!stats.under_pressure);
    }

    #[test]
    fn test_queue_stats_is_healthy() {
        let healthy = QueueStats::default();
        assert!(healthy.is_healthy());

        let unhealthy_expired = QueueStats {
            expired_count: 1,
            ..Default::default()
        };
        assert!(!unhealthy_expired.is_healthy());

        let unhealthy_dropped = QueueStats {
            dropped_count: 1,
            ..Default::default()
        };
        assert!(!unhealthy_dropped.is_healthy());

        let unhealthy_pressure = QueueStats {
            under_pressure: true,
            ..Default::default()
        };
        assert!(!unhealthy_pressure.is_healthy());
    }

    #[test]
    fn test_queue_stats_status_string() {
        let ok = QueueStats {
            item_count: 5,
            ..Default::default()
        };
        assert!(ok.status_string().contains("OK"));
        assert!(ok.status_string().contains("5"));

        let pressure = QueueStats {
            item_count: 15,
            dropped_count: 3,
            under_pressure: true,
            ..Default::default()
        };
        assert!(pressure.status_string().contains("PRESSURE"));

        let active = QueueStats {
            item_count: 5,
            expired_count: 2,
            ..Default::default()
        };
        assert!(active.status_string().contains("ACTIVE"));
    }

    // ========== AudioItem Timing Tests ==========

    #[test]
    fn test_audio_item_with_max_queue_time() {
        let item = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_millis(100));
        assert_eq!(item.max_queue_time, Duration::from_millis(100));
    }

    #[test]
    fn test_audio_item_countdown_tone_timing() {
        let item = AudioItem::countdown_tone(440, 100);
        assert_eq!(item.max_queue_time, Duration::from_millis(500));
        assert!(item.is_time_critical());
    }

    #[test]
    fn test_audio_item_countdown_tone_with_timing() {
        let item = AudioItem::countdown_tone_with_timing(440, 100, 200);
        assert_eq!(item.max_queue_time, Duration::from_millis(200));
    }

    #[test]
    fn test_audio_item_type_description() {
        let speech = AudioItem::speech("Hello world, this is a long message");
        let desc = speech.type_description();
        assert!(desc.contains("Speech"));
        assert!(desc.contains("Hello world, this is..."));

        let sound = AudioItem::sound("achievement_chime");
        let desc = sound.type_description();
        assert!(desc.contains("Sound"));
        assert!(desc.contains("achievement_chime"));

        let tone = AudioItem::tone(880, 200);
        let desc = tone.type_description();
        assert!(desc.contains("Tone"));
        assert!(desc.contains("880Hz"));
        assert!(desc.contains("200ms"));
    }

    #[test]
    fn test_audio_item_is_time_critical() {
        let countdown = AudioItem::countdown_tone(440, 100);
        assert!(countdown.is_time_critical());

        let regular = AudioItem::tone(440, 100);
        assert!(!regular.is_time_critical());

        let speech = AudioItem::speech("Hello");
        assert!(!speech.is_time_critical());
    }

    #[test]
    fn test_audio_item_time_remaining() {
        let long_lived = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_secs(60));
        let remaining = long_lived.time_remaining();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(59));

        // Very short-lived item
        let short_lived = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(short_lived.time_remaining().is_none());
        assert!(short_lived.is_expired());
    }

    #[test]
    fn test_audio_item_age() {
        let item = AudioItem::tone(440, 100);
        std::thread::sleep(Duration::from_millis(10));
        let age = item.age_ms();
        assert!(age >= 10);
    }
}
