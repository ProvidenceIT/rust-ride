//! Voice control support via Vosk speech recognition.
//!
//! Provides hands-free control of the application using voice commands.
//! This module is only compiled when the `voice-control` feature is enabled.
//!
//! T126: Implement Vosk model initialization (download on first run)
//! T130: Add visual/audio confirmation of recognized commands
//! T132: Integrate voice commands with ride control

use std::path::PathBuf;

/// Voice command types that can be recognized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceCommand {
    /// Start a ride or workout
    Start,
    /// Pause the current activity
    Pause,
    /// Resume a paused activity
    Resume,
    /// End/stop the current activity
    End,
    /// Skip to the next interval
    Skip,
    /// Increase something (power target, volume, etc.)
    Increase,
    /// Decrease something (power target, volume, etc.)
    Decrease,
    /// Request current metrics to be announced
    Status,
    /// Take a lap marker
    TakeLap,
    /// Unknown/unrecognized command
    Unknown(String),
}

impl VoiceCommand {
    /// Parse a recognized phrase into a command.
    ///
    /// This method uses fuzzy matching with Levenshtein distance to handle
    /// common speech recognition errors and variations. It applies corrections
    /// for known misrecognitions (e.g., "paws" -> "pause") and uses a default
    /// minimum confidence threshold.
    ///
    /// For more control over matching, use `from_phrase_with_confidence()` or
    /// `from_phrase_with_threshold()` from the `voice::command_parser` module.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rustride::accessibility::voice_control::VoiceCommand;
    ///
    /// // Exact match
    /// assert_eq!(VoiceCommand::from_phrase("pause"), VoiceCommand::Pause);
    ///
    /// // Misrecognition handling
    /// assert_eq!(VoiceCommand::from_phrase("paws"), VoiceCommand::Pause);
    ///
    /// // Unknown command
    /// matches!(VoiceCommand::from_phrase("xyz123"), VoiceCommand::Unknown(_));
    /// ```
    pub fn from_phrase(phrase: &str) -> Self {
        #[cfg(feature = "voice-control")]
        {
            use crate::voice::command_parser::CommandParser;

            let parser = CommandParser::new();
            match parser.parse(phrase) {
                Some(result) => result.command,
                None => VoiceCommand::Unknown(phrase.to_lowercase()),
            }
        }

        #[cfg(not(feature = "voice-control"))]
        {
            // Fallback to simple matching when voice-control feature is not enabled
            Self::from_phrase_simple(phrase)
        }
    }

    /// Simple phrase matching without fuzzy matching support.
    ///
    /// This is used as a fallback when the voice-control feature is not enabled,
    /// or for basic testing without the full command parser.
    pub fn from_phrase_simple(phrase: &str) -> Self {
        let phrase = phrase.to_lowercase();

        // Match common phrases and variations
        if phrase.contains("start") || phrase.contains("begin") || phrase.contains("go") {
            VoiceCommand::Start
        } else if phrase.contains("pause") || phrase.contains("stop") || phrase.contains("hold") {
            VoiceCommand::Pause
        } else if phrase.contains("resume")
            || phrase.contains("continue")
            || phrase.contains("unpause")
        {
            VoiceCommand::Resume
        } else if phrase.contains("end") || phrase.contains("finish") || phrase.contains("done") {
            VoiceCommand::End
        } else if phrase.contains("skip") || phrase.contains("next") {
            VoiceCommand::Skip
        } else if phrase.contains("increase") || phrase.contains("up") || phrase.contains("more") {
            VoiceCommand::Increase
        } else if phrase.contains("decrease") || phrase.contains("down") || phrase.contains("less")
        {
            VoiceCommand::Decrease
        } else if phrase.contains("status")
            || phrase.contains("metrics")
            || phrase.contains("how am i doing")
        {
            VoiceCommand::Status
        } else if phrase.contains("lap") || phrase.contains("mark lap") || phrase.contains("take lap") {
            VoiceCommand::TakeLap
        } else {
            VoiceCommand::Unknown(phrase)
        }
    }
}

/// State of the voice control system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceControlState {
    /// Voice control is not initialized
    #[default]
    Uninitialized,
    /// Voice control is initializing (downloading model, etc.)
    Initializing,
    /// Voice control is ready and listening
    Ready,
    /// Voice control is actively processing speech
    Listening,
    /// Voice control is unavailable (missing microphone, model, etc.)
    Unavailable,
    /// Voice control encountered an error
    Error,
}

impl std::fmt::Display for VoiceControlState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceControlState::Uninitialized => write!(f, "Not Initialized"),
            VoiceControlState::Initializing => write!(f, "Initializing..."),
            VoiceControlState::Ready => write!(f, "Ready"),
            VoiceControlState::Listening => write!(f, "Listening..."),
            VoiceControlState::Unavailable => write!(f, "Unavailable"),
            VoiceControlState::Error => write!(f, "Error"),
        }
    }
}

/// Trait for voice control implementations.
pub trait VoiceControl {
    /// Initialize the voice control system.
    fn initialize(&mut self) -> Result<(), VoiceControlError>;

    /// Get the current state.
    fn state(&self) -> VoiceControlState;

    /// Start listening for commands.
    fn start_listening(&mut self) -> Result<(), VoiceControlError>;

    /// Stop listening for commands.
    fn stop_listening(&mut self);

    /// Get the next recognized command, if any.
    fn poll_command(&mut self) -> Option<VoiceCommand>;

    /// Check if voice control is available on this system.
    fn is_available(&self) -> bool {
        matches!(
            self.state(),
            VoiceControlState::Ready | VoiceControlState::Listening
        )
    }

    /// Get the reason voice control is unavailable, if applicable.
    fn unavailable_reason(&self) -> Option<&str>;
}

/// Voice control errors.
#[derive(Debug, thiserror::Error)]
pub enum VoiceControlError {
    #[error("Microphone not available: {0}")]
    MicrophoneUnavailable(String),

    #[error("Voice model not found: {0}")]
    ModelNotFound(String),

    #[error("Voice model download failed: {0}")]
    ModelDownloadFailed(String),

    #[error("Voice recognition initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Voice recognition error: {0}")]
    RecognitionError(String),
}

/// Placeholder voice control implementation for when voice-control feature is enabled.
/// The actual implementation using Vosk will be added later.
pub struct VoskVoiceControl {
    state: VoiceControlState,
    unavailable_reason: Option<String>,
}

impl VoskVoiceControl {
    /// Create a new voice control instance.
    pub fn new() -> Self {
        Self {
            state: VoiceControlState::Uninitialized,
            unavailable_reason: None,
        }
    }
}

impl Default for VoskVoiceControl {
    fn default() -> Self {
        Self::new()
    }
}

impl VoskVoiceControl {
    /// T126: Get the path where the Vosk model should be stored.
    pub fn get_model_path() -> PathBuf {
        // Use the project's standard data directory
        crate::storage::config::get_data_dir().join("vosk-model")
    }

    /// T126: Check if the Vosk model is available.
    pub fn is_model_available() -> bool {
        Self::get_model_path().exists()
    }

    /// T130: Get confirmation message for a command.
    pub fn command_confirmation(command: &VoiceCommand) -> &'static str {
        match command {
            VoiceCommand::Start => "Starting ride",
            VoiceCommand::Pause => "Pausing",
            VoiceCommand::Resume => "Resuming",
            VoiceCommand::End => "Ending ride",
            VoiceCommand::Skip => "Skipping interval",
            VoiceCommand::Increase => "Increasing",
            VoiceCommand::Decrease => "Decreasing",
            VoiceCommand::Status => "Reading metrics",
            VoiceCommand::TakeLap => "Marking lap",
            VoiceCommand::Unknown(_) => "Command not recognized",
        }
    }

    /// T130: Get audio cue pattern for a command.
    pub fn command_audio_cue(command: &VoiceCommand) -> CommandAudioCue {
        match command {
            VoiceCommand::Start | VoiceCommand::Resume => CommandAudioCue::Positive,
            VoiceCommand::End | VoiceCommand::Pause => CommandAudioCue::Neutral,
            VoiceCommand::Skip | VoiceCommand::TakeLap => CommandAudioCue::Action,
            VoiceCommand::Increase | VoiceCommand::Decrease => CommandAudioCue::Adjustment,
            VoiceCommand::Status => CommandAudioCue::Info,
            VoiceCommand::Unknown(_) => CommandAudioCue::Error,
        }
    }
}

/// T130: Audio cue types for voice command confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAudioCue {
    /// Positive confirmation (start, resume)
    Positive,
    /// Neutral confirmation (pause, end)
    Neutral,
    /// Action taken (skip)
    Action,
    /// Adjustment made (increase, decrease)
    Adjustment,
    /// Information provided (status)
    Info,
    /// Error/unrecognized
    Error,
}

/// Default cooldown period between same commands in milliseconds.
const DEFAULT_COMMAND_COOLDOWN_MS: u64 = 1000;

/// T132: Voice command handler for ride control integration.
pub struct VoiceCommandHandler {
    /// Pending command to be processed
    pending_command: Option<VoiceCommand>,
    /// Last command executed
    last_command: Option<VoiceCommand>,
    /// Whether to show visual confirmation
    show_confirmation: bool,
    /// Confirmation display timer
    confirmation_timer: Option<std::time::Instant>,
    /// Last command timestamp for cooldown tracking
    last_command_time: Option<std::time::Instant>,
    /// Cooldown duration in milliseconds
    cooldown_ms: u64,
}

impl Default for VoiceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceCommandHandler {
    /// Create a new voice command handler.
    pub fn new() -> Self {
        Self {
            pending_command: None,
            last_command: None,
            show_confirmation: false,
            confirmation_timer: None,
            last_command_time: None,
            cooldown_ms: DEFAULT_COMMAND_COOLDOWN_MS,
        }
    }

    /// Create a new handler with a custom cooldown duration.
    pub fn with_cooldown(cooldown_ms: u64) -> Self {
        Self {
            pending_command: None,
            last_command: None,
            show_confirmation: false,
            confirmation_timer: None,
            last_command_time: None,
            cooldown_ms,
        }
    }

    /// Set the cooldown duration in milliseconds.
    pub fn set_cooldown_ms(&mut self, cooldown_ms: u64) {
        self.cooldown_ms = cooldown_ms;
    }

    /// Get the cooldown duration in milliseconds.
    pub fn cooldown_ms(&self) -> u64 {
        self.cooldown_ms
    }

    /// Check if a command is allowed (not in cooldown).
    ///
    /// Returns `true` if the command can be executed, `false` if it's blocked
    /// by cooldown. A command is blocked if it's the same as the last command
    /// and the cooldown period hasn't elapsed.
    pub fn is_command_allowed(&self, command: &VoiceCommand) -> bool {
        // Unknown commands are always allowed (no cooldown)
        if matches!(command, VoiceCommand::Unknown(_)) {
            return true;
        }

        match (&self.last_command, self.last_command_time) {
            (Some(last_cmd), Some(last_time)) => {
                // Check if it's the same command
                if std::mem::discriminant(last_cmd) == std::mem::discriminant(command) {
                    // Check if cooldown has elapsed
                    last_time.elapsed().as_millis() >= self.cooldown_ms as u128
                } else {
                    // Different command, always allowed
                    true
                }
            }
            _ => true, // No previous command, allowed
        }
    }

    /// Get the remaining cooldown time for a command in milliseconds.
    ///
    /// Returns `None` if the command is allowed (not in cooldown).
    pub fn remaining_cooldown_ms(&self, command: &VoiceCommand) -> Option<u64> {
        if matches!(command, VoiceCommand::Unknown(_)) {
            return None;
        }

        match (&self.last_command, self.last_command_time) {
            (Some(last_cmd), Some(last_time)) => {
                if std::mem::discriminant(last_cmd) == std::mem::discriminant(command) {
                    let elapsed = last_time.elapsed().as_millis() as u64;
                    if elapsed < self.cooldown_ms {
                        Some(self.cooldown_ms - elapsed)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Queue a command for processing.
    ///
    /// The command will be queued regardless of cooldown status.
    /// Use `queue_command_with_cooldown` to respect cooldown.
    pub fn queue_command(&mut self, command: VoiceCommand) {
        self.pending_command = Some(command);
    }

    /// Queue a command for processing, respecting cooldown.
    ///
    /// Returns `true` if the command was queued, `false` if blocked by cooldown.
    pub fn queue_command_with_cooldown(&mut self, command: VoiceCommand) -> bool {
        if self.is_command_allowed(&command) {
            self.pending_command = Some(command);
            true
        } else {
            false
        }
    }

    /// Get and clear the pending command.
    ///
    /// This also records the command for cooldown tracking and sets up
    /// the confirmation display timer.
    pub fn take_pending(&mut self) -> Option<VoiceCommand> {
        let cmd = self.pending_command.take();
        if let Some(ref c) = cmd {
            // Record for cooldown tracking (don't record Unknown commands)
            if !matches!(c, VoiceCommand::Unknown(_)) {
                self.last_command_time = Some(std::time::Instant::now());
            }
            self.last_command = Some(c.clone());
            self.show_confirmation = true;
            self.confirmation_timer = Some(std::time::Instant::now());
        }
        cmd
    }

    /// Reset the cooldown state.
    ///
    /// This clears the last command and timestamp, allowing immediate
    /// execution of any command.
    pub fn reset_cooldown(&mut self) {
        self.last_command = None;
        self.last_command_time = None;
    }

    /// Get the last executed command, if any.
    pub fn last_executed_command(&self) -> Option<&VoiceCommand> {
        self.last_command.as_ref()
    }

    /// Get the time since the last command was executed.
    pub fn time_since_last_command(&self) -> Option<std::time::Duration> {
        self.last_command_time.map(|t| t.elapsed())
    }

    /// Check if confirmation should be shown.
    pub fn should_show_confirmation(&mut self) -> bool {
        if let Some(timer) = self.confirmation_timer {
            // Show confirmation for 2 seconds
            if timer.elapsed().as_secs() < 2 {
                return self.show_confirmation;
            } else {
                self.show_confirmation = false;
                self.confirmation_timer = None;
            }
        }
        false
    }

    /// Get the confirmation message if showing.
    pub fn confirmation_message(&self) -> Option<&'static str> {
        if self.show_confirmation {
            self.last_command
                .as_ref()
                .map(VoskVoiceControl::command_confirmation)
        } else {
            None
        }
    }

    /// Get the audio cue type if showing confirmation.
    pub fn confirmation_audio_cue(&self) -> Option<CommandAudioCue> {
        if self.show_confirmation {
            self.last_command
                .as_ref()
                .map(VoskVoiceControl::command_audio_cue)
        } else {
            None
        }
    }
}

impl VoiceControl for VoskVoiceControl {
    fn initialize(&mut self) -> Result<(), VoiceControlError> {
        self.state = VoiceControlState::Initializing;

        // T126: Check for Vosk model and download if needed
        let model_path = Self::get_model_path();
        if !model_path.exists() {
            tracing::info!(
                "Vosk model not found at {:?}, attempting download...",
                model_path
            );

            // In a real implementation, we would download the model here
            // For now, mark as unavailable with instructions
            self.state = VoiceControlState::Unavailable;
            self.unavailable_reason = Some(format!(
                "Voice model not found. Please download the Vosk model to {:?}",
                model_path
            ));
            return Ok(());
        }

        // Model exists, initialize would happen here
        self.state = VoiceControlState::Ready;
        tracing::info!("Voice control initialized with model at {:?}", model_path);

        Ok(())
    }

    fn state(&self) -> VoiceControlState {
        self.state
    }

    fn start_listening(&mut self) -> Result<(), VoiceControlError> {
        if self.state == VoiceControlState::Unavailable {
            return Err(VoiceControlError::InitializationFailed(
                self.unavailable_reason.clone().unwrap_or_default(),
            ));
        }

        self.state = VoiceControlState::Listening;
        Ok(())
    }

    fn stop_listening(&mut self) {
        if self.state == VoiceControlState::Listening {
            self.state = VoiceControlState::Ready;
        }
    }

    fn poll_command(&mut self) -> Option<VoiceCommand> {
        // TODO: Implement actual command polling when Vosk is integrated
        None
    }

    fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // VoiceCommandHandler Cooldown Tests
    // ========================================

    #[test]
    fn test_handler_creation() {
        let handler = VoiceCommandHandler::new();
        assert_eq!(handler.cooldown_ms(), DEFAULT_COMMAND_COOLDOWN_MS);
        assert!(handler.last_executed_command().is_none());
        assert!(handler.time_since_last_command().is_none());
    }

    #[test]
    fn test_handler_with_cooldown() {
        let handler = VoiceCommandHandler::with_cooldown(2000);
        assert_eq!(handler.cooldown_ms(), 2000);
    }

    #[test]
    fn test_handler_set_cooldown() {
        let mut handler = VoiceCommandHandler::new();
        handler.set_cooldown_ms(3000);
        assert_eq!(handler.cooldown_ms(), 3000);
    }

    #[test]
    fn test_handler_command_allowed_initially() {
        let handler = VoiceCommandHandler::new();
        assert!(handler.is_command_allowed(&VoiceCommand::Pause));
        assert!(handler.is_command_allowed(&VoiceCommand::Resume));
        assert!(handler.is_command_allowed(&VoiceCommand::Start));
    }

    #[test]
    fn test_handler_queue_and_take() {
        let mut handler = VoiceCommandHandler::new();

        handler.queue_command(VoiceCommand::Pause);
        let cmd = handler.take_pending();
        assert_eq!(cmd, Some(VoiceCommand::Pause));

        // After take, the last command should be tracked
        assert_eq!(handler.last_executed_command(), Some(&VoiceCommand::Pause));
        assert!(handler.time_since_last_command().is_some());
    }

    #[test]
    fn test_handler_cooldown_blocks_same_command() {
        let mut handler = VoiceCommandHandler::new();

        // Queue and execute a command
        handler.queue_command(VoiceCommand::Pause);
        let _ = handler.take_pending();

        // Same command should be blocked
        assert!(!handler.is_command_allowed(&VoiceCommand::Pause));

        // Different command should be allowed
        assert!(handler.is_command_allowed(&VoiceCommand::Resume));
    }

    #[test]
    fn test_handler_queue_with_cooldown() {
        let mut handler = VoiceCommandHandler::new();

        // First queue should succeed
        assert!(handler.queue_command_with_cooldown(VoiceCommand::Pause));
        let _ = handler.take_pending();

        // Second same command should fail
        assert!(!handler.queue_command_with_cooldown(VoiceCommand::Pause));

        // Different command should succeed
        assert!(handler.queue_command_with_cooldown(VoiceCommand::Resume));
    }

    #[test]
    fn test_handler_remaining_cooldown() {
        let mut handler = VoiceCommandHandler::new();

        // No cooldown initially
        assert!(handler.remaining_cooldown_ms(&VoiceCommand::Pause).is_none());

        handler.queue_command(VoiceCommand::Pause);
        let _ = handler.take_pending();

        // Should have remaining cooldown for same command
        let remaining = handler.remaining_cooldown_ms(&VoiceCommand::Pause);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 0);
        assert!(remaining.unwrap() <= DEFAULT_COMMAND_COOLDOWN_MS);

        // No cooldown for different command
        assert!(handler.remaining_cooldown_ms(&VoiceCommand::Resume).is_none());
    }

    #[test]
    fn test_handler_unknown_command_no_cooldown() {
        let mut handler = VoiceCommandHandler::new();
        let unknown = VoiceCommand::Unknown("test".to_string());

        // Unknown commands are always allowed
        assert!(handler.is_command_allowed(&unknown));

        handler.queue_command(unknown.clone());
        let _ = handler.take_pending();

        // Unknown command shouldn't trigger cooldown for itself
        assert!(handler.is_command_allowed(&VoiceCommand::Unknown("test".to_string())));
    }

    #[test]
    fn test_handler_reset_cooldown() {
        let mut handler = VoiceCommandHandler::new();

        handler.queue_command(VoiceCommand::Pause);
        let _ = handler.take_pending();

        // Command blocked
        assert!(!handler.is_command_allowed(&VoiceCommand::Pause));

        handler.reset_cooldown();

        // Command allowed after reset
        assert!(handler.is_command_allowed(&VoiceCommand::Pause));
        assert!(handler.last_executed_command().is_none());
    }

    #[test]
    fn test_handler_cooldown_expires() {
        let mut handler = VoiceCommandHandler::with_cooldown(10); // Short cooldown

        handler.queue_command(VoiceCommand::Pause);
        let _ = handler.take_pending();

        assert!(!handler.is_command_allowed(&VoiceCommand::Pause));

        // Wait for cooldown
        std::thread::sleep(std::time::Duration::from_millis(15));

        assert!(handler.is_command_allowed(&VoiceCommand::Pause));
    }

    // ========================================
    // VoiceCommand Tests
    // ========================================

    #[test]
    fn test_voice_command_from_phrase_simple() {
        assert_eq!(VoiceCommand::from_phrase_simple("pause"), VoiceCommand::Pause);
        assert_eq!(VoiceCommand::from_phrase_simple("resume"), VoiceCommand::Resume);
        assert_eq!(VoiceCommand::from_phrase_simple("start"), VoiceCommand::Start);
        assert_eq!(VoiceCommand::from_phrase_simple("end"), VoiceCommand::End);
        assert_eq!(VoiceCommand::from_phrase_simple("skip"), VoiceCommand::Skip);
        assert_eq!(VoiceCommand::from_phrase_simple("lap"), VoiceCommand::TakeLap);
    }

    #[test]
    fn test_voice_command_unknown() {
        let cmd = VoiceCommand::from_phrase_simple("gibberish xyz");
        assert!(matches!(cmd, VoiceCommand::Unknown(_)));
    }

    // ========================================
    // VoskVoiceControl Tests
    // ========================================

    #[test]
    fn test_command_confirmation_messages() {
        assert_eq!(VoskVoiceControl::command_confirmation(&VoiceCommand::Pause), "Pausing");
        assert_eq!(VoskVoiceControl::command_confirmation(&VoiceCommand::Resume), "Resuming");
        assert_eq!(VoskVoiceControl::command_confirmation(&VoiceCommand::TakeLap), "Marking lap");
    }

    #[test]
    fn test_command_audio_cues() {
        assert_eq!(VoskVoiceControl::command_audio_cue(&VoiceCommand::Start), CommandAudioCue::Positive);
        assert_eq!(VoskVoiceControl::command_audio_cue(&VoiceCommand::Pause), CommandAudioCue::Neutral);
        assert_eq!(VoskVoiceControl::command_audio_cue(&VoiceCommand::TakeLap), CommandAudioCue::Action);
    }
}
