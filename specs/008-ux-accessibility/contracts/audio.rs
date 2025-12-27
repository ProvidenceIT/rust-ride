//! Audio Feedback Module Contract
//!
//! Public API for audio cues and feedback during training.

use std::time::Duration;

// ============================================================================
// Audio Cue System
// ============================================================================

/// Manages audio cue playback.
pub trait AudioCueSystem {
    /// Initialize the audio system.
    fn initialize(&mut self) -> Result<(), AudioError>;

    /// Check if audio is available and initialized.
    fn is_available(&self) -> bool;

    /// Play an interval transition cue.
    fn play_interval_cue(&self, transition: IntervalTransition);

    /// Play a zone change cue.
    fn play_zone_change_cue(&self, direction: ZoneChangeDirection);

    /// Play a workout start cue.
    fn play_workout_start(&self);

    /// Play a workout end cue.
    fn play_workout_end(&self);

    /// Play a countdown beep (for interval countdown).
    fn play_countdown_beep(&self, seconds_remaining: u8);

    /// Play a custom tone.
    fn play_tone(&self, frequency_hz: f32, duration: Duration);

    /// Stop all currently playing audio.
    fn stop_all(&self);

    /// Get current settings.
    fn settings(&self) -> &AudioSettings;

    /// Update settings.
    fn update_settings(&mut self, settings: AudioSettings);
}

/// Interval transition types.
#[derive(Clone, Copy, Debug)]
pub enum IntervalTransition {
    /// Moving to a higher intensity interval
    ToHigher,
    /// Moving to a lower intensity interval
    ToLower,
    /// Moving to rest/recovery
    ToRest,
    /// Starting from rest
    FromRest,
    /// Generic interval change
    Generic,
}

/// Zone change direction.
#[derive(Clone, Copy, Debug)]
pub enum ZoneChangeDirection {
    /// Moving to a higher zone
    Up,
    /// Moving to a lower zone
    Down,
}

/// Audio settings.
#[derive(Clone, Debug)]
pub struct AudioSettings {
    /// Master audio enabled
    pub enabled: bool,

    /// Volume level (0.0 - 1.0)
    pub volume: f32,

    /// Play cues on interval transitions
    pub interval_cues_enabled: bool,

    /// Play cues on zone changes
    pub zone_cues_enabled: bool,

    /// Play cues on workout start/end
    pub workout_cues_enabled: bool,

    /// Play countdown beeps before intervals
    pub countdown_enabled: bool,

    /// Number of countdown beeps (3, 5, or 10 seconds)
    pub countdown_seconds: u8,

    /// Selected audio profile
    pub profile: AudioProfile,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.7,
            interval_cues_enabled: true,
            zone_cues_enabled: true,
            workout_cues_enabled: true,
            countdown_enabled: true,
            countdown_seconds: 3,
            profile: AudioProfile::Simple,
        }
    }
}

/// Audio profile presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioProfile {
    /// Simple beep tones
    #[default]
    Simple,
    /// Melodic two-tone cues
    Melodic,
    /// Minimal (only critical alerts)
    Minimal,
    /// Custom (user-defined frequencies)
    Custom,
}

// ============================================================================
// Tone Generation
// ============================================================================

/// Low-level tone generation.
pub trait ToneGenerator {
    /// Generate a sine wave tone.
    fn sine_wave(&self, frequency_hz: f32, duration: Duration, volume: f32) -> AudioSource;

    /// Generate a two-tone sequence (for zone changes).
    fn two_tone(&self, freq1: f32, freq2: f32, duration_each: Duration, volume: f32) -> AudioSource;

    /// Generate a beep pattern (for countdown).
    fn beep_pattern(&self, frequency: f32, beep_duration: Duration, gap: Duration, count: u8, volume: f32) -> AudioSource;
}

/// An audio source that can be played.
pub trait AudioSource: Send + Sync {
    /// Get the duration of this audio source.
    fn duration(&self) -> Duration;

    /// Check if the source is currently playing.
    fn is_playing(&self) -> bool;
}

// ============================================================================
// Predefined Tones
// ============================================================================

/// Standard tone frequencies.
pub mod tones {
    /// A4 (440 Hz) - Standard interval cue
    pub const A4: f32 = 440.0;

    /// C5 (523 Hz) - Higher zone
    pub const C5: f32 = 523.25;

    /// E4 (330 Hz) - Lower zone
    pub const E4: f32 = 329.63;

    /// G4 (392 Hz) - Rest/recovery
    pub const G4: f32 = 392.0;

    /// High beep for countdown
    pub const COUNTDOWN: f32 = 880.0;

    /// Success tone (major third)
    pub const SUCCESS_LOW: f32 = 440.0;
    pub const SUCCESS_HIGH: f32 = 554.37;

    /// Alert tone
    pub const ALERT: f32 = 660.0;
}

/// Standard tone durations.
pub mod durations {
    use std::time::Duration;

    /// Short beep (100ms)
    pub const SHORT: Duration = Duration::from_millis(100);

    /// Standard beep (200ms)
    pub const STANDARD: Duration = Duration::from_millis(200);

    /// Long beep (400ms)
    pub const LONG: Duration = Duration::from_millis(400);

    /// Gap between tones (50ms)
    pub const GAP: Duration = Duration::from_millis(50);
}

// ============================================================================
// Zone Change Detection
// ============================================================================

/// Detects zone changes and triggers audio cues.
pub trait ZoneChangeDetector {
    /// Update with current power and check for zone change.
    fn update_power(&mut self, power: u16, ftp: u16) -> Option<ZoneChange>;

    /// Update with current heart rate and check for zone change.
    fn update_heart_rate(&mut self, hr: u8, max_hr: u8, rest_hr: u8) -> Option<ZoneChange>;

    /// Get the current power zone.
    fn current_power_zone(&self) -> u8;

    /// Get the current HR zone.
    fn current_hr_zone(&self) -> Option<u8>;

    /// Set the debounce time to prevent rapid zone change cues.
    fn set_debounce(&mut self, duration: Duration);
}

/// A detected zone change.
#[derive(Clone, Copy, Debug)]
pub struct ZoneChange {
    /// Previous zone
    pub from_zone: u8,

    /// New zone
    pub to_zone: u8,

    /// Direction of change
    pub direction: ZoneChangeDirection,

    /// Type of zone (power or HR)
    pub zone_type: ZoneType,
}

#[derive(Clone, Copy, Debug)]
pub enum ZoneType {
    Power,
    HeartRate,
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio device not available")]
    DeviceNotAvailable,

    #[error("Audio initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Audio playback failed: {0}")]
    PlaybackFailed(String),
}
