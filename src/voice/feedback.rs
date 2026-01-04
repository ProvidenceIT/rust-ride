//! Voice Feedback System
//!
//! Audio feedback for voice control events. Provides:
//!
//! ## Tone Feedback
//! Distinct tones for:
//! - Wake word detected (ascending activation tone)
//! - Command recognized (positive confirmation tone)
//! - Command failed (error/descending tone)
//!
//! ## TTS Confirmation
//! Spoken confirmation of recognized commands using ThreadSafeTtsProvider.
//! For example: "Pausing", "Skipping interval", "Marking lap".
//!
//! ## Microphone Coordination
//! Coordinates with audio capture to prevent TTS feedback into the microphone
//! by pausing capture during TTS playback.
//!
//! Uses the existing RodioAudioBackend::play_tone() infrastructure and
//! ThreadSafeTtsProvider for spoken confirmations.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::accessibility::voice_control::{VoiceCommand, VoskVoiceControl};
use crate::audio::backend::{BackendError, RodioAudioBackend};
use crate::audio::tts::{ThreadSafeTtsProvider, TtsProvider};

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

    /// TTS provider error.
    #[error("TTS error: {0}")]
    TtsError(String),

    /// Microphone is currently capturing (TTS blocked to prevent feedback).
    #[error("TTS blocked while microphone is capturing")]
    MicrophoneCapturing,
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
    /// Whether to speak TTS confirmation of commands.
    /// When enabled, commands are confirmed with spoken text like "Pausing", "Skipping interval".
    pub tts_confirmation: bool,
    /// Whether to pause microphone during TTS to prevent feedback loops.
    /// This should be enabled when using voice control with speakers (not headphones).
    pub pause_mic_during_tts: bool,
}

impl Default for VoiceFeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.5,
            wake_word_feedback: true,
            command_feedback: true,
            listening_feedback: true,
            tts_confirmation: true,
            pause_mic_during_tts: true,
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

    /// Enable or disable TTS command confirmation.
    ///
    /// When enabled, recognized commands are spoken aloud using TTS
    /// (e.g., "Pausing", "Skipping interval").
    pub fn with_tts_confirmation(mut self, enabled: bool) -> Self {
        self.tts_confirmation = enabled;
        self
    }

    /// Enable or disable microphone pausing during TTS.
    ///
    /// When enabled, the microphone is paused during TTS playback to prevent
    /// the TTS audio from being picked up and causing feedback loops.
    /// This should be enabled when using speakers instead of headphones.
    pub fn with_pause_mic_during_tts(mut self, enabled: bool) -> Self {
        self.pause_mic_during_tts = enabled;
        self
    }

    /// Create config with TTS disabled (tones only).
    pub fn tones_only() -> Self {
        Self::default().with_tts_confirmation(false)
    }
}

/// Voice feedback provider using RodioAudioBackend and ThreadSafeTtsProvider.
///
/// Provides both audio tones and TTS spoken confirmations for voice control events:
///
/// ## Audio Tones
/// - Wake word detected: Ascending two-note tone (A4 -> E5)
/// - Command recognized: Single positive tone (C5)
/// - Command failed: Descending two-note tone (600Hz -> 400Hz)
///
/// ## TTS Confirmations
/// - "Pausing", "Resuming", "Skipping interval", etc.
/// - Uses VoskVoiceControl::command_confirmation() for text
///
/// ## Microphone Coordination
/// - Tracks microphone state via `is_mic_capturing` atomic bool
/// - Can pause TTS when microphone is active to prevent feedback
pub struct VoiceFeedback {
    /// Reference to the audio backend for tones.
    backend: Arc<RodioAudioBackend>,
    /// TTS provider for spoken confirmations.
    tts_provider: Option<Arc<ThreadSafeTtsProvider>>,
    /// Configuration for feedback behavior.
    config: VoiceFeedbackConfig,
    /// Whether the microphone is currently capturing audio.
    /// Used to prevent TTS during capture to avoid feedback loops.
    is_mic_capturing: Arc<AtomicBool>,
}

impl VoiceFeedback {
    /// Create a new voice feedback provider (tones only, no TTS).
    pub fn new(backend: Arc<RodioAudioBackend>) -> Self {
        Self {
            backend,
            tts_provider: None,
            config: VoiceFeedbackConfig::default().with_tts_confirmation(false),
            is_mic_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with TTS support for spoken command confirmations.
    pub fn with_tts(
        backend: Arc<RodioAudioBackend>,
        tts_provider: Arc<ThreadSafeTtsProvider>,
    ) -> Self {
        Self {
            backend,
            tts_provider: Some(tts_provider),
            config: VoiceFeedbackConfig::default(),
            is_mic_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(backend: Arc<RodioAudioBackend>, config: VoiceFeedbackConfig) -> Self {
        Self {
            backend,
            tts_provider: None,
            config,
            is_mic_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with TTS and custom configuration.
    pub fn with_tts_and_config(
        backend: Arc<RodioAudioBackend>,
        tts_provider: Arc<ThreadSafeTtsProvider>,
        config: VoiceFeedbackConfig,
    ) -> Self {
        Self {
            backend,
            tts_provider: Some(tts_provider),
            config,
            is_mic_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set the TTS provider.
    pub fn set_tts_provider(&mut self, provider: Option<Arc<ThreadSafeTtsProvider>>) {
        self.tts_provider = provider;
    }

    /// Check if TTS is available.
    pub fn has_tts(&self) -> bool {
        self.tts_provider.is_some()
    }

    /// Get a shared reference to the microphone capturing state.
    ///
    /// This should be shared with the VoiceEngine/AudioInputCapture to
    /// coordinate microphone pausing during TTS playback.
    pub fn mic_capturing_state(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_mic_capturing)
    }

    /// Set the microphone capturing state.
    ///
    /// Called by VoiceEngine when starting/stopping audio capture.
    pub fn set_mic_capturing(&self, is_capturing: bool) {
        self.is_mic_capturing.store(is_capturing, Ordering::Release);
    }

    /// Check if the microphone is currently capturing.
    pub fn is_mic_capturing(&self) -> bool {
        self.is_mic_capturing.load(Ordering::Acquire)
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

    // =========================================================================
    // TTS Confirmation Methods
    // =========================================================================

    /// Speak TTS confirmation of a recognized command.
    ///
    /// Uses `VoskVoiceControl::command_confirmation()` to get the confirmation text
    /// (e.g., "Pausing", "Skipping interval", "Marking lap").
    ///
    /// # Microphone Coordination
    ///
    /// If `pause_mic_during_tts` is enabled in config and the microphone is currently
    /// capturing, this method will:
    /// 1. Return `Err(MicrophoneCapturing)` if called synchronously
    /// 2. For async version, the caller should pause the mic before calling
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustride::voice::feedback::{VoiceFeedback, VoiceFeedbackConfig};
    /// use rustride::accessibility::voice_control::VoiceCommand;
    ///
    /// let feedback = VoiceFeedback::with_tts(backend, tts_provider);
    ///
    /// // When a command is recognized:
    /// feedback.speak_confirmation(&VoiceCommand::Pause)?;  // Speaks "Pausing"
    /// ```
    pub fn speak_confirmation(&self, command: &VoiceCommand) -> Result<bool, VoiceFeedbackError> {
        // Check if TTS confirmation is enabled
        if !self.config.enabled || !self.config.tts_confirmation {
            return Ok(false);
        }

        // Check if TTS provider is available
        let tts = match &self.tts_provider {
            Some(provider) => provider,
            None => {
                tracing::debug!("TTS confirmation skipped: no TTS provider");
                return Ok(false);
            }
        };

        // Check if microphone is capturing and we should avoid TTS
        if self.config.pause_mic_during_tts && self.is_mic_capturing() {
            tracing::debug!("TTS confirmation blocked: microphone is capturing");
            return Err(VoiceFeedbackError::MicrophoneCapturing);
        }

        // Get the confirmation text for this command
        let confirmation_text = VoskVoiceControl::command_confirmation(command);

        // Don't speak for unknown commands
        if confirmation_text == "Command not recognized" {
            return Ok(false);
        }

        tracing::debug!("TTS confirmation: {}", confirmation_text);

        // Speak the confirmation
        tts.speak(confirmation_text)
            .map_err(|e| VoiceFeedbackError::TtsError(e.to_string()))?;

        Ok(true)
    }

    /// Speak TTS confirmation asynchronously (non-blocking).
    ///
    /// Spawns a background thread to speak the confirmation. This is preferred
    /// for use in the voice recognition pipeline to avoid blocking audio processing.
    ///
    /// # Microphone Handling
    ///
    /// If `pause_mic_during_tts` is enabled, the caller should:
    /// 1. Pause the microphone before calling
    /// 2. Resume the microphone after TTS completes
    ///
    /// Consider using `speak_confirmation_with_mic_pause()` which handles this
    /// automatically when a mic pause callback is provided.
    pub fn speak_confirmation_async(&self, command: VoiceCommand) {
        // Check if TTS confirmation is enabled
        if !self.config.enabled || !self.config.tts_confirmation {
            return;
        }

        // Check if TTS provider is available
        let tts = match &self.tts_provider {
            Some(provider) => Arc::clone(provider),
            None => return,
        };

        // Get the confirmation text for this command
        let confirmation_text = VoskVoiceControl::command_confirmation(&command);

        // Don't speak for unknown commands
        if confirmation_text == "Command not recognized" {
            return;
        }

        // Spawn thread to speak confirmation
        std::thread::spawn(move || {
            tracing::debug!("TTS confirmation (async): {}", confirmation_text);
            if let Err(e) = tts.speak(confirmation_text) {
                tracing::warn!("TTS confirmation failed: {}", e);
            }
        });
    }

    /// Speak TTS confirmation with automatic microphone pause handling.
    ///
    /// This method coordinates with the voice engine to pause the microphone
    /// during TTS playback, preventing the spoken confirmation from being
    /// picked up by the microphone and causing a feedback loop.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to confirm
    /// * `pause_mic` - Callback to pause microphone (called before TTS)
    /// * `resume_mic` - Callback to resume microphone (called after TTS)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pause_mic = || engine.pause();
    /// let resume_mic = || engine.resume();
    ///
    /// feedback.speak_confirmation_with_mic_pause(
    ///     &VoiceCommand::Pause,
    ///     pause_mic,
    ///     resume_mic,
    /// )?;
    /// ```
    pub fn speak_confirmation_with_mic_pause<F1, F2>(
        &self,
        command: &VoiceCommand,
        pause_mic: F1,
        resume_mic: F2,
    ) -> Result<bool, VoiceFeedbackError>
    where
        F1: FnOnce(),
        F2: FnOnce(),
    {
        // Check if TTS confirmation is enabled
        if !self.config.enabled || !self.config.tts_confirmation {
            return Ok(false);
        }

        // Check if TTS provider is available
        let tts = match &self.tts_provider {
            Some(provider) => provider,
            None => return Ok(false),
        };

        // Get the confirmation text for this command
        let confirmation_text = VoskVoiceControl::command_confirmation(command);

        // Don't speak for unknown commands
        if confirmation_text == "Command not recognized" {
            return Ok(false);
        }

        // Pause microphone if configured
        if self.config.pause_mic_during_tts {
            pause_mic();
            self.set_mic_capturing(false);
        }

        tracing::debug!("TTS confirmation (with mic pause): {}", confirmation_text);

        // Speak the confirmation
        let result = tts.speak(confirmation_text);

        // Resume microphone
        if self.config.pause_mic_during_tts {
            resume_mic();
            self.set_mic_capturing(true);
        }

        result.map_err(|e| VoiceFeedbackError::TtsError(e.to_string()))?;
        Ok(true)
    }

    /// Play both tone feedback and TTS confirmation for a recognized command.
    ///
    /// This is a convenience method that:
    /// 1. Plays the command recognized tone
    /// 2. Speaks the TTS confirmation (if TTS is enabled and available)
    ///
    /// This is the recommended way to provide feedback for recognized commands
    /// as it gives users both immediate audio feedback (tone) and verbal
    /// confirmation of what action is being taken.
    pub fn play_command_with_confirmation(
        &self,
        command: &VoiceCommand,
    ) -> Result<bool, VoiceFeedbackError> {
        // Play the recognition tone
        let tone_played = self.play(VoiceFeedbackEvent::CommandRecognized)?;

        // Speak TTS confirmation
        let tts_spoken = match self.speak_confirmation(command) {
            Ok(spoken) => spoken,
            Err(VoiceFeedbackError::MicrophoneCapturing) => {
                // Microphone is capturing, skip TTS but don't error
                tracing::debug!("Skipping TTS confirmation while mic is capturing");
                false
            }
            Err(e) => {
                // Log other TTS errors but don't fail
                tracing::warn!("TTS confirmation error: {}", e);
                false
            }
        };

        Ok(tone_played || tts_spoken)
    }

    /// Check if TTS confirmation is enabled in the configuration.
    pub fn is_tts_confirmation_enabled(&self) -> bool {
        self.config.tts_confirmation && self.tts_provider.is_some()
    }

    // =========================================================================
    // Async Playback Methods
    // =========================================================================

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
        assert!(config.tts_confirmation);
        assert!(config.pause_mic_during_tts);
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

    // ========================================================================
    // TTS Confirmation Config Tests
    // ========================================================================

    #[test]
    fn test_feedback_config_tts_confirmation() {
        let config = VoiceFeedbackConfig::new()
            .with_tts_confirmation(true)
            .with_pause_mic_during_tts(true);

        assert!(config.tts_confirmation);
        assert!(config.pause_mic_during_tts);

        let config = VoiceFeedbackConfig::new()
            .with_tts_confirmation(false)
            .with_pause_mic_during_tts(false);

        assert!(!config.tts_confirmation);
        assert!(!config.pause_mic_during_tts);
    }

    #[test]
    fn test_feedback_config_tones_only() {
        let config = VoiceFeedbackConfig::tones_only();
        assert!(config.enabled);
        assert!(!config.tts_confirmation);
        // Other defaults should still apply
        assert!(config.wake_word_feedback);
        assert!(config.command_feedback);
    }

    #[test]
    fn test_mic_capturing_state() {
        let is_capturing = Arc::new(AtomicBool::new(false));

        // Test initial state
        assert!(!is_capturing.load(Ordering::Acquire));

        // Test state change
        is_capturing.store(true, Ordering::Release);
        assert!(is_capturing.load(Ordering::Acquire));

        is_capturing.store(false, Ordering::Release);
        assert!(!is_capturing.load(Ordering::Acquire));
    }

    #[test]
    fn test_command_confirmation_text() {
        // Test that command_confirmation returns expected text
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::Pause),
            "Pausing"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::Resume),
            "Resuming"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::Skip),
            "Skipping interval"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::TakeLap),
            "Marking lap"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::Start),
            "Starting ride"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::End),
            "Ending ride"
        );
        assert_eq!(
            VoskVoiceControl::command_confirmation(&VoiceCommand::Unknown("foo".to_string())),
            "Command not recognized"
        );
    }

    #[test]
    fn test_tts_confirmation_enabled_check() {
        // Without TTS provider, TTS confirmation is not enabled
        let config = VoiceFeedbackConfig::default();
        assert!(config.tts_confirmation);
        // But we can't test is_tts_confirmation_enabled() without VoiceFeedback
        // because it requires a backend, which requires audio hardware
    }

    #[test]
    fn test_voice_feedback_error_variants() {
        // Test that all error variants can be constructed
        let _e1 = VoiceFeedbackError::NotInitialized;
        let _e2 = VoiceFeedbackError::TtsError("test error".to_string());
        let _e3 = VoiceFeedbackError::MicrophoneCapturing;

        // Test error messages
        assert_eq!(
            format!("{}", VoiceFeedbackError::NotInitialized),
            "Audio backend not initialized"
        );
        assert_eq!(
            format!("{}", VoiceFeedbackError::TtsError("test".to_string())),
            "TTS error: test"
        );
        assert_eq!(
            format!("{}", VoiceFeedbackError::MicrophoneCapturing),
            "TTS blocked while microphone is capturing"
        );
    }

    // Note: Tests that require actual audio hardware are in tests/voice_integration.rs
    // They should be marked as #[ignore] since they require audio device
}
