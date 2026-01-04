//! Voice Feedback System
//!
//! Audio feedback for voice control events. Provides distinct tones for:
//! - Wake word detected (ascending activation tone)
//! - Command recognized (positive confirmation tone)
//! - Command failed (error/descending tone)
//!
//! Uses the existing RodioAudioBackend::play_tone() infrastructure.

use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::audio::backend::{BackendError, RodioAudioBackend};

/// Voice feedback tone frequencies (in Hz).
pub mod frequencies {
    /// Wake word detected - starting frequency for ascending tone
    pub const WAKE_WORD_START: f32 = 440.0; // A4
    /// Wake word detected - ending frequency for ascending tone
    pub const WAKE_WORD_END: f32 = 660.0; // E5

    /// Command recognized - positive confirmation frequency
    pub const COMMAND_RECOGNIZED: f32 = 523.25; // C5

    /// Command failed - starting frequency for descending tone
    pub const COMMAND_FAILED_START: f32 = 600.0;
    /// Command failed - ending frequency for descending tone
    pub const COMMAND_FAILED_END: f32 = 400.0;

    /// Listening activated (push-to-talk or manual activation)
    pub const LISTENING_START: f32 = 400.0;
    /// Listening deactivated
    pub const LISTENING_END: f32 = 350.0;

    /// Command cooldown blocked
    pub const COOLDOWN_BLOCKED: f32 = 300.0; // Lower pitch for "blocked" feedback
}

/// Voice feedback tone durations (in milliseconds).
pub mod durations {
    /// Duration for each note in wake word ascending tone
    pub const WAKE_WORD_NOTE: u64 = 80;
    /// Pause between wake word notes
    pub const WAKE_WORD_PAUSE: u64 = 30;

    /// Duration for command recognized tone
    pub const COMMAND_RECOGNIZED: u64 = 150;

    /// Duration for each note in command failed descending tone
    pub const COMMAND_FAILED_NOTE: u64 = 100;
    /// Pause between command failed notes
    pub const COMMAND_FAILED_PAUSE: u64 = 50;

    /// Duration for listening start/end tones
    pub const LISTENING_TONE: u64 = 100;

    /// Duration for cooldown blocked tone
    pub const COOLDOWN_BLOCKED: u64 = 60;
}

/// Voice feedback event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceFeedbackEvent {
    /// Wake word was detected, entering active listening mode.
    WakeWordDetected,

    /// A voice command was successfully recognized.
    CommandRecognized,

    /// A voice command failed to be recognized or was invalid.
    CommandFailed,

    /// Listening mode started (push-to-talk or manual activation).
    ListeningStarted,

    /// Listening mode ended (push-to-talk released or timeout).
    ListeningEnded,

    /// Command was blocked by cooldown.
    CooldownBlocked,
}

impl VoiceFeedbackEvent {
    /// Get a description of the feedback event.
    pub fn description(&self) -> &'static str {
        match self {
            VoiceFeedbackEvent::WakeWordDetected => "Wake word detected",
            VoiceFeedbackEvent::CommandRecognized => "Command recognized",
            VoiceFeedbackEvent::CommandFailed => "Command failed",
            VoiceFeedbackEvent::ListeningStarted => "Listening started",
            VoiceFeedbackEvent::ListeningEnded => "Listening ended",
            VoiceFeedbackEvent::CooldownBlocked => "Command blocked by cooldown",
        }
    }
}

/// Errors that can occur in voice feedback.
#[derive(Debug, Error)]
pub enum VoiceFeedbackError {
    /// Audio backend error.
    #[error("Audio backend error: {0}")]
    BackendError(#[from] BackendError),

    /// Backend not initialized.
    #[error("Audio backend not initialized")]
    NotInitialized,
}

/// Configuration for voice feedback.
#[derive(Debug, Clone)]
pub struct VoiceFeedbackConfig {
    /// Whether audio feedback is enabled.
    pub enabled: bool,
    /// Volume for feedback tones (0.0 - 1.0).
    pub volume: f32,
    /// Whether to play wake word feedback.
    pub wake_word_feedback: bool,
    /// Whether to play command feedback.
    pub command_feedback: bool,
    /// Whether to play listening state feedback.
    pub listening_feedback: bool,
}

impl Default for VoiceFeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.5,
            wake_word_feedback: true,
            command_feedback: true,
            listening_feedback: true,
        }
    }
}

impl VoiceFeedbackConfig {
    /// Create a new feedback config with feedback enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable all audio feedback.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set the volume level (0.0 - 1.0).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable wake word feedback.
    pub fn with_wake_word_feedback(mut self, enabled: bool) -> Self {
        self.wake_word_feedback = enabled;
        self
    }

    /// Enable or disable command feedback.
    pub fn with_command_feedback(mut self, enabled: bool) -> Self {
        self.command_feedback = enabled;
        self
    }

    /// Enable or disable listening state feedback.
    pub fn with_listening_feedback(mut self, enabled: bool) -> Self {
        self.listening_feedback = enabled;
        self
    }
}

/// Voice feedback provider using RodioAudioBackend.
///
/// Plays distinct audio tones for voice control events:
/// - Wake word detected: Ascending two-note tone (A4 -> E5)
/// - Command recognized: Single positive tone (C5)
/// - Command failed: Descending two-note tone (600Hz -> 400Hz)
pub struct VoiceFeedback {
    /// Reference to the audio backend.
    backend: Arc<RodioAudioBackend>,
    /// Configuration for feedback behavior.
    config: VoiceFeedbackConfig,
}

impl VoiceFeedback {
    /// Create a new voice feedback provider.
    pub fn new(backend: Arc<RodioAudioBackend>) -> Self {
        Self {
            backend,
            config: VoiceFeedbackConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(backend: Arc<RodioAudioBackend>, config: VoiceFeedbackConfig) -> Self {
        Self { backend, config }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &VoiceFeedbackConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: VoiceFeedbackConfig) {
        self.config = config;
    }

    /// Set whether feedback is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Check if feedback is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Set the volume for feedback tones.
    pub fn set_volume(&mut self, volume: f32) {
        self.config.volume = volume.clamp(0.0, 1.0);
    }

    /// Get the current volume.
    pub fn volume(&self) -> f32 {
        self.config.volume
    }

    /// Play feedback for an event.
    ///
    /// Returns Ok(true) if feedback was played, Ok(false) if skipped
    /// (due to configuration), or Err if playback failed.
    pub fn play(&self, event: VoiceFeedbackEvent) -> Result<bool, VoiceFeedbackError> {
        if !self.config.enabled {
            return Ok(false);
        }

        // Check event-specific configuration
        match event {
            VoiceFeedbackEvent::WakeWordDetected => {
                if !self.config.wake_word_feedback {
                    return Ok(false);
                }
            }
            VoiceFeedbackEvent::CommandRecognized | VoiceFeedbackEvent::CommandFailed => {
                if !self.config.command_feedback {
                    return Ok(false);
                }
            }
            VoiceFeedbackEvent::ListeningStarted
            | VoiceFeedbackEvent::ListeningEnded
            | VoiceFeedbackEvent::CooldownBlocked => {
                if !self.config.listening_feedback {
                    return Ok(false);
                }
            }
        }

        // Play the appropriate feedback
        match event {
            VoiceFeedbackEvent::WakeWordDetected => self.play_wake_word_detected()?,
            VoiceFeedbackEvent::CommandRecognized => self.play_command_recognized()?,
            VoiceFeedbackEvent::CommandFailed => self.play_command_failed()?,
            VoiceFeedbackEvent::ListeningStarted => self.play_listening_started()?,
            VoiceFeedbackEvent::ListeningEnded => self.play_listening_ended()?,
            VoiceFeedbackEvent::CooldownBlocked => self.play_cooldown_blocked()?,
        }

        Ok(true)
    }

    /// Play wake word detected feedback (ascending two-note tone).
    fn play_wake_word_detected(&self) -> Result<(), VoiceFeedbackError> {
        // Ascending two-note tone: A4 (440Hz) -> E5 (660Hz)
        // Creates a pleasant "activation" feel
        let volume = Some(self.config.volume);

        // First note
        self.backend.play_tone_with_volume(
            frequencies::WAKE_WORD_START,
            Duration::from_millis(durations::WAKE_WORD_NOTE),
            volume,
        )?;

        // Brief pause
        std::thread::sleep(Duration::from_millis(durations::WAKE_WORD_PAUSE));

        // Second note (higher)
        self.backend.play_tone_with_volume(
            frequencies::WAKE_WORD_END,
            Duration::from_millis(durations::WAKE_WORD_NOTE),
            volume,
        )?;

        Ok(())
    }

    /// Play command recognized feedback (positive single tone).
    fn play_command_recognized(&self) -> Result<(), VoiceFeedbackError> {
        // Single positive tone at C5 (523Hz)
        // Clear, pleasant confirmation sound
        self.backend.play_tone_with_volume(
            frequencies::COMMAND_RECOGNIZED,
            Duration::from_millis(durations::COMMAND_RECOGNIZED),
            Some(self.config.volume),
        )?;

        Ok(())
    }

    /// Play command failed feedback (descending two-note tone).
    fn play_command_failed(&self) -> Result<(), VoiceFeedbackError> {
        // Descending two-note tone: 600Hz -> 400Hz
        // Creates a gentle "error" or "not understood" feel
        let volume = Some(self.config.volume);

        // First note (higher)
        self.backend.play_tone_with_volume(
            frequencies::COMMAND_FAILED_START,
            Duration::from_millis(durations::COMMAND_FAILED_NOTE),
            volume,
        )?;

        // Brief pause
        std::thread::sleep(Duration::from_millis(durations::COMMAND_FAILED_PAUSE));

        // Second note (lower)
        self.backend.play_tone_with_volume(
            frequencies::COMMAND_FAILED_END,
            Duration::from_millis(durations::COMMAND_FAILED_NOTE),
            volume,
        )?;

        Ok(())
    }

    /// Play listening started feedback (gentle activation tone).
    fn play_listening_started(&self) -> Result<(), VoiceFeedbackError> {
        self.backend.play_tone_with_volume(
            frequencies::LISTENING_START,
            Duration::from_millis(durations::LISTENING_TONE),
            Some(self.config.volume),
        )?;

        Ok(())
    }

    /// Play listening ended feedback (subtle deactivation tone).
    fn play_listening_ended(&self) -> Result<(), VoiceFeedbackError> {
        self.backend.play_tone_with_volume(
            frequencies::LISTENING_END,
            Duration::from_millis(durations::LISTENING_TONE),
            Some(self.config.volume),
        )?;

        Ok(())
    }

    /// Play cooldown blocked feedback (short low tone).
    fn play_cooldown_blocked(&self) -> Result<(), VoiceFeedbackError> {
        self.backend.play_tone_with_volume(
            frequencies::COOLDOWN_BLOCKED,
            Duration::from_millis(durations::COOLDOWN_BLOCKED),
            Some(self.config.volume * 0.7), // Slightly quieter
        )?;

        Ok(())
    }

    /// Play wake word feedback asynchronously (non-blocking).
    ///
    /// Spawns a background thread to play the tones.
    pub fn play_async(&self, event: VoiceFeedbackEvent) {
        if !self.config.enabled {
            return;
        }

        // Check event-specific configuration
        let should_play = match event {
            VoiceFeedbackEvent::WakeWordDetected => self.config.wake_word_feedback,
            VoiceFeedbackEvent::CommandRecognized | VoiceFeedbackEvent::CommandFailed => {
                self.config.command_feedback
            }
            VoiceFeedbackEvent::ListeningStarted
            | VoiceFeedbackEvent::ListeningEnded
            | VoiceFeedbackEvent::CooldownBlocked => self.config.listening_feedback,
        };

        if !should_play {
            return;
        }

        let backend = Arc::clone(&self.backend);
        let volume = self.config.volume;

        std::thread::spawn(move || {
            let result = match event {
                VoiceFeedbackEvent::WakeWordDetected => {
                    play_wake_word_tone(&backend, volume)
                }
                VoiceFeedbackEvent::CommandRecognized => {
                    play_command_recognized_tone(&backend, volume)
                }
                VoiceFeedbackEvent::CommandFailed => {
                    play_command_failed_tone(&backend, volume)
                }
                VoiceFeedbackEvent::ListeningStarted => {
                    play_listening_started_tone(&backend, volume)
                }
                VoiceFeedbackEvent::ListeningEnded => {
                    play_listening_ended_tone(&backend, volume)
                }
                VoiceFeedbackEvent::CooldownBlocked => {
                    play_cooldown_blocked_tone(&backend, volume)
                }
            };

            if let Err(e) = result {
                tracing::warn!("Failed to play voice feedback: {}", e);
            }
        });
    }
}

// Standalone functions for async playback in background threads

fn play_wake_word_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::WAKE_WORD_START,
        Duration::from_millis(durations::WAKE_WORD_NOTE),
        Some(volume),
    )?;
    std::thread::sleep(Duration::from_millis(durations::WAKE_WORD_PAUSE));
    backend.play_tone_with_volume(
        frequencies::WAKE_WORD_END,
        Duration::from_millis(durations::WAKE_WORD_NOTE),
        Some(volume),
    )?;
    Ok(())
}

fn play_command_recognized_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::COMMAND_RECOGNIZED,
        Duration::from_millis(durations::COMMAND_RECOGNIZED),
        Some(volume),
    )?;
    Ok(())
}

fn play_command_failed_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::COMMAND_FAILED_START,
        Duration::from_millis(durations::COMMAND_FAILED_NOTE),
        Some(volume),
    )?;
    std::thread::sleep(Duration::from_millis(durations::COMMAND_FAILED_PAUSE));
    backend.play_tone_with_volume(
        frequencies::COMMAND_FAILED_END,
        Duration::from_millis(durations::COMMAND_FAILED_NOTE),
        Some(volume),
    )?;
    Ok(())
}

fn play_listening_started_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::LISTENING_START,
        Duration::from_millis(durations::LISTENING_TONE),
        Some(volume),
    )?;
    Ok(())
}

fn play_listening_ended_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::LISTENING_END,
        Duration::from_millis(durations::LISTENING_TONE),
        Some(volume),
    )?;
    Ok(())
}

fn play_cooldown_blocked_tone(backend: &RodioAudioBackend, volume: f32) -> Result<(), BackendError> {
    backend.play_tone_with_volume(
        frequencies::COOLDOWN_BLOCKED,
        Duration::from_millis(durations::COOLDOWN_BLOCKED),
        Some(volume * 0.7),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_config_default() {
        let config = VoiceFeedbackConfig::default();
        assert!(config.enabled);
        assert_eq!(config.volume, 0.5);
        assert!(config.wake_word_feedback);
        assert!(config.command_feedback);
        assert!(config.listening_feedback);
    }

    #[test]
    fn test_feedback_config_disabled() {
        let config = VoiceFeedbackConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_feedback_config_builder() {
        let config = VoiceFeedbackConfig::new()
            .with_volume(0.8)
            .with_wake_word_feedback(false)
            .with_command_feedback(true)
            .with_listening_feedback(false);

        assert!(config.enabled);
        assert_eq!(config.volume, 0.8);
        assert!(!config.wake_word_feedback);
        assert!(config.command_feedback);
        assert!(!config.listening_feedback);
    }

    #[test]
    fn test_feedback_config_volume_clamping() {
        let config = VoiceFeedbackConfig::new().with_volume(1.5);
        assert_eq!(config.volume, 1.0);

        let config = VoiceFeedbackConfig::new().with_volume(-0.5);
        assert_eq!(config.volume, 0.0);
    }

    #[test]
    fn test_feedback_event_description() {
        assert_eq!(
            VoiceFeedbackEvent::WakeWordDetected.description(),
            "Wake word detected"
        );
        assert_eq!(
            VoiceFeedbackEvent::CommandRecognized.description(),
            "Command recognized"
        );
        assert_eq!(
            VoiceFeedbackEvent::CommandFailed.description(),
            "Command failed"
        );
        assert_eq!(
            VoiceFeedbackEvent::ListeningStarted.description(),
            "Listening started"
        );
        assert_eq!(
            VoiceFeedbackEvent::ListeningEnded.description(),
            "Listening ended"
        );
        assert_eq!(
            VoiceFeedbackEvent::CooldownBlocked.description(),
            "Command blocked by cooldown"
        );
    }

    #[test]
    fn test_feedback_frequencies() {
        // Wake word should be ascending
        assert!(frequencies::WAKE_WORD_END > frequencies::WAKE_WORD_START);

        // Command failed should be descending
        assert!(frequencies::COMMAND_FAILED_START > frequencies::COMMAND_FAILED_END);

        // Listening end should be lower than listening start
        assert!(frequencies::LISTENING_END < frequencies::LISTENING_START);
    }

    #[test]
    fn test_feedback_durations_are_reasonable() {
        // All durations should be under 500ms for responsive feedback
        assert!(durations::WAKE_WORD_NOTE < 500);
        assert!(durations::COMMAND_RECOGNIZED < 500);
        assert!(durations::COMMAND_FAILED_NOTE < 500);
        assert!(durations::LISTENING_TONE < 500);
        assert!(durations::COOLDOWN_BLOCKED < 500);

        // But long enough to be audible (>30ms)
        assert!(durations::WAKE_WORD_NOTE >= 30);
        assert!(durations::COMMAND_RECOGNIZED >= 30);
        assert!(durations::COMMAND_FAILED_NOTE >= 30);
        assert!(durations::LISTENING_TONE >= 30);
        assert!(durations::COOLDOWN_BLOCKED >= 30);
    }

    #[test]
    fn test_wake_word_total_duration() {
        // Wake word feedback: 2 notes + 1 pause
        let total = durations::WAKE_WORD_NOTE * 2 + durations::WAKE_WORD_PAUSE;
        // Should be under 300ms for quick feedback
        assert!(total < 300, "Wake word feedback is {}ms, should be < 300ms", total);
    }

    #[test]
    fn test_command_failed_total_duration() {
        // Command failed feedback: 2 notes + 1 pause
        let total = durations::COMMAND_FAILED_NOTE * 2 + durations::COMMAND_FAILED_PAUSE;
        // Should be under 400ms
        assert!(total < 400, "Command failed feedback is {}ms, should be < 400ms", total);
    }

    // Note: Tests that require actual audio hardware are in tests/voice_integration.rs
    // They should be marked as #[ignore] since they require audio device
}
