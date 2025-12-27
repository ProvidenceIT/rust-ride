//! Voice control support via Vosk speech recognition.
//!
//! Provides hands-free control of the application using voice commands.
//! This module is only compiled when the `voice-control` feature is enabled.


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

impl VoiceControl for VoskVoiceControl {
    fn initialize(&mut self) -> Result<(), VoiceControlError> {
        self.state = VoiceControlState::Initializing;

        // TODO: Implement actual Vosk initialization
        // For now, mark as unavailable since Vosk integration is not complete
        self.state = VoiceControlState::Unavailable;
        self.unavailable_reason = Some("Voice control model not yet configured".to_string());

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
