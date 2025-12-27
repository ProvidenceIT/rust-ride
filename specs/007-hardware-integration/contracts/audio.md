# Contract: Audio Module

**Module**: `src/audio/`
**Feature**: Hardware Integration
**Date**: 2025-12-26

This contract defines the audio cues and text-to-speech API.

---

## Audio Engine (`src/audio/engine.rs`)

```rust
/// Core audio playback engine
pub trait AudioEngine: Send + Sync {
    /// Initialize audio system
    async fn initialize(&self) -> Result<(), AudioError>;

    /// Play a pre-recorded sound
    async fn play_sound(&self, sound: SoundType) -> Result<(), AudioError>;

    /// Speak text using TTS
    async fn speak(&self, text: &str) -> Result<(), AudioError>;

    /// Speak text with specific voice settings
    async fn speak_with_options(&self, text: &str, options: SpeakOptions) -> Result<(), AudioError>;

    /// Set master volume (0-100)
    fn set_volume(&self, volume: u8);

    /// Get current volume
    fn get_volume(&self) -> u8;

    /// Mute/unmute
    fn set_muted(&self, muted: bool);

    /// Check if audio system is available
    fn is_available(&self) -> bool;

    /// List available TTS voices
    fn list_voices(&self) -> Vec<VoiceInfo>;

    /// Set preferred voice
    fn set_voice(&self, voice_id: &str) -> Result<(), AudioError>;

    /// Stop all audio (cancel queue)
    fn stop_all(&self);
}

pub struct SpeakOptions {
    pub rate: f32,      // Speech rate multiplier (0.5 - 2.0)
    pub pitch: f32,     // Pitch multiplier (0.5 - 2.0)
    pub volume: u8,     // Override volume (0-100)
    pub priority: AlertPriority,
}

pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: Option<VoiceGender>,
}

pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

pub enum SoundType {
    Beep,
    Chime,
    Alert,
    Success,
    Error,
    Countdown3,
    Countdown2,
    Countdown1,
    Go,
}

pub enum AlertPriority {
    Low,
    Normal,
    High,
    Critical,
}
```

---

## Alert Manager (`src/audio/alerts.rs`)

```rust
/// Manages audio alert triggers and configuration
pub trait AlertManager: Send + Sync {
    /// Configure alerts
    fn configure(&self, config: AudioConfig);

    /// Get current configuration
    fn get_config(&self) -> AudioConfig;

    /// Check if a specific alert type is enabled
    fn is_enabled(&self, alert_type: AlertType) -> bool;

    /// Enable/disable specific alert
    fn set_alert_enabled(&self, alert_type: AlertType, enabled: bool);

    /// Trigger an alert (checks if enabled, builds message, queues audio)
    async fn trigger_alert(&self, alert: Alert) -> Result<(), AudioError>;

    /// Register custom cue template
    fn register_template(&self, template: CueTemplate);

    /// Get all registered templates
    fn get_templates(&self) -> Vec<CueTemplate>;
}

pub struct Alert {
    pub alert_type: AlertType,
    pub context: AlertContext,
    pub priority: AlertPriority,
}

pub enum AlertContext {
    IntervalStart {
        interval_name: String,
        target_power: Option<u16>,
        target_cadence: Option<u8>,
        duration_seconds: u32,
    },
    IntervalEnd {
        interval_name: String,
        next_interval: Option<String>,
    },
    ZoneChange {
        old_zone: u8,
        new_zone: u8,
        zone_name: String,
    },
    Milestone {
        milestone_type: MilestoneType,
        value: String,
    },
    PowerTarget {
        target: u16,
        current: u16,
        variance_percent: i8,
    },
    LapMarker {
        lap_number: u32,
        lap_time: Duration,
    },
    Custom {
        message: String,
    },
}

pub enum MilestoneType {
    Halfway,
    FinalMinute,
    TenSecondsLeft,
    WorkoutComplete,
    DistanceMilestone,
    TimeMilestone,
}
```

---

## Cue Builder (`src/audio/cues.rs`)

```rust
/// Build audio cue messages from templates
pub trait CueBuilder {
    /// Build message from template and context
    fn build_message(&self, template: &CueTemplate, context: &AlertContext) -> String;

    /// Get default templates for alert types
    fn get_default_templates() -> HashMap<AlertType, CueTemplate>;
}

/// Default template patterns
impl Default for CueBuilder {
    // IntervalStart: "Starting {interval_name}. Target {target_power} watts for {duration}."
    // IntervalEnd: "Interval complete. {next_interval} is next."
    // ZoneChange: "Entering zone {new_zone}. {zone_name}."
    // Halfway: "Halfway there. Keep pushing."
    // FinalMinute: "Final minute. Finish strong."
    // WorkoutComplete: "Workout complete. Great job!"
    // LapMarker: "Lap {lap_number}. Time {lap_time}."
}
```

---

## TTS Provider (`src/audio/tts.rs`)

```rust
/// Text-to-speech abstraction
pub trait TtsProvider: Send + Sync {
    /// Initialize TTS engine
    async fn initialize(&self) -> Result<(), AudioError>;

    /// Synthesize text to audio
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<AudioData, AudioError>;

    /// Speak directly (synthesize and play)
    async fn speak(&self, text: &str, options: &TtsOptions) -> Result<(), AudioError>;

    /// Stop current speech
    fn stop(&self);

    /// Check if currently speaking
    fn is_speaking(&self) -> bool;

    /// Get available voices
    fn get_voices(&self) -> Vec<VoiceInfo>;

    /// Set active voice
    fn set_voice(&self, voice_id: &str) -> Result<(), AudioError>;
}

pub struct TtsOptions {
    pub voice_id: Option<String>,
    pub rate: f32,
    pub pitch: f32,
    pub volume: u8,
}

pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u8,
}
```

---

## Audio Queue

```rust
/// Queued audio playback to prevent overlap
pub struct AudioQueue {
    pending: VecDeque<QueuedItem>,
    current: Option<PlayingItem>,
}

pub struct QueuedItem {
    pub id: Uuid,
    pub audio_type: QueuedAudioType,
    pub priority: AlertPriority,
    pub queued_at: Instant,
}

pub enum QueuedAudioType {
    Sound(SoundType),
    Speech { text: String, options: TtsOptions },
}

impl AudioQueue {
    /// Add item to queue (respects priority)
    pub fn enqueue(&mut self, item: QueuedItem);

    /// Get next item to play
    pub fn dequeue(&mut self) -> Option<QueuedItem>;

    /// Clear queue (except critical priority)
    pub fn clear(&mut self);

    /// Cancel specific item by ID
    pub fn cancel(&mut self, id: &Uuid) -> bool;

    /// Get queue length
    pub fn len(&self) -> usize;
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio system not available")]
    NotAvailable,

    #[error("TTS engine not initialized")]
    TtsNotInitialized,

    #[error("Voice not found: {0}")]
    VoiceNotFound(String),

    #[error("Audio playback failed: {0}")]
    PlaybackFailed(String),

    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    #[error("Audio device error: {0}")]
    DeviceError(String),
}
```

---

## Events

```rust
pub enum AudioEvent {
    /// Audio system initialized
    Initialized,

    /// Alert triggered
    AlertTriggered { alert_type: AlertType, message: String },

    /// Speech started
    SpeechStarted { text: String },

    /// Speech completed
    SpeechCompleted,

    /// Sound played
    SoundPlayed { sound: SoundType },

    /// Volume changed
    VolumeChanged { volume: u8, muted: bool },

    /// Error occurred
    Error { error: String },
}
```

---

## Configuration

```rust
/// Audio configuration stored in user settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub enabled: bool,
    pub volume: u8,
    pub voice_id: Option<String>,
    pub speech_rate: f32,
    pub alerts: HashMap<AlertType, AlertSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSettings {
    pub enabled: bool,
    pub custom_message: Option<String>,
    pub sound_only: bool, // Play sound instead of speech
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 80,
            voice_id: None, // Use system default
            speech_rate: 1.0,
            alerts: AlertType::all()
                .into_iter()
                .map(|t| (t, AlertSettings::default()))
                .collect(),
        }
    }
}
```
