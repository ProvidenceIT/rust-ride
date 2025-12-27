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
    /// Unknown/unrecognized command
    Unknown(String),
}

impl VoiceCommand {
    /// Parse a recognized phrase into a command.
    pub fn from_phrase(phrase: &str) -> Self {
        let phrase = phrase.to_lowercase();

        // Match common phrases and variations
        if phrase.contains("start") || phrase.contains("begin") || phrase.contains("go") {
            VoiceCommand::Start
        } else if phrase.contains("pause") || phrase.contains("stop") || phrase.contains("hold") {
            VoiceCommand::Pause
        } else if phrase.contains("resume") || phrase.contains("continue") || phrase.contains("unpause") {
            VoiceCommand::Resume
        } else if phrase.contains("end") || phrase.contains("finish") || phrase.contains("done") {
            VoiceCommand::End
        } else if phrase.contains("skip") || phrase.contains("next") {
            VoiceCommand::Skip
        } else if phrase.contains("increase") || phrase.contains("up") || phrase.contains("more") {
            VoiceCommand::Increase
        } else if phrase.contains("decrease") || phrase.contains("down") || phrase.contains("less") {
            VoiceCommand::Decrease
        } else if phrase.contains("status") || phrase.contains("metrics") || phrase.contains("how am i doing") {
            VoiceCommand::Status
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
        matches!(self.state(), VoiceControlState::Ready | VoiceControlState::Listening)
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
            VoiceCommand::Unknown(_) => "Command not recognized",
        }
    }

    /// T130: Get audio cue pattern for a command.
    pub fn command_audio_cue(command: &VoiceCommand) -> CommandAudioCue {
        match command {
            VoiceCommand::Start | VoiceCommand::Resume => CommandAudioCue::Positive,
            VoiceCommand::End | VoiceCommand::Pause => CommandAudioCue::Neutral,
            VoiceCommand::Skip => CommandAudioCue::Action,
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
        }
    }

    /// Queue a command for processing.
    pub fn queue_command(&mut self, command: VoiceCommand) {
        self.pending_command = Some(command);
    }

    /// Get and clear the pending command.
    pub fn take_pending(&mut self) -> Option<VoiceCommand> {
        let cmd = self.pending_command.take();
        if let Some(ref c) = cmd {
            self.last_command = Some(c.clone());
            self.show_confirmation = true;
            self.confirmation_timer = Some(std::time::Instant::now());
        }
        cmd
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
            tracing::info!("Vosk model not found at {:?}, attempting download...", model_path);

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
                self.unavailable_reason.clone().unwrap_or_default()
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
