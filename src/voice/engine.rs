//! Voice Recognition Engine
//!
//! The VoiceEngine orchestrates the complete voice recognition pipeline:
//! - Audio capture from microphone (AudioInputCapture)
//! - Speech recognition (ThreadSafeRecognizer via Vosk)
//! - Command parsing (VoiceCommand)
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                           VoiceEngine                                 │
//! │              (Send + Sync wrapper for external use)                   │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │   command_tx ─────────────────────────────────┐                      │
//! │                                               ▼                      │
//! │   ┌──────────────────────────────────────────────────────────────┐   │
//! │   │                     Engine Worker Thread                      │   │
//! │   │                                                               │   │
//! │   │   ┌───────────────┐    ┌─────────────┐    ┌──────────────┐   │   │
//! │   │   │AudioInputCapture│──▶│ThreadSafe   │──▶│VoiceCommand   │   │   │
//! │   │   │(microphone)    │    │Recognizer   │    │::from_phrase()│   │   │
//! │   │   └───────────────┘    │(Vosk)       │    └──────────────┘   │   │
//! │   │                        └─────────────┘            │          │   │
//! │   │                                                   ▼          │   │
//! │   │                                          recognized_commands_tx  │   │
//! │   └──────────────────────────────────────────────────────────────┘   │
//! │                                                                       │
//! │   recognized_commands_rx ◀────────────────────────────────────────────┤
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::engine::{VoiceEngine, VoiceEngineConfig, VoiceEngineEvent};
//!
//! // Create engine
//! let config = VoiceEngineConfig::new("/path/to/vosk-model");
//! let engine = VoiceEngine::new(config)?;
//!
//! // Subscribe to events (commands, partial results, state changes)
//! let mut event_rx = engine.subscribe();
//!
//! // Start listening
//! engine.start()?;
//!
//! // Process events in your event loop
//! while let Ok(event) = event_rx.try_recv() {
//!     match event {
//!         VoiceEngineEvent::CommandRecognized { command, text, confidence } => {
//!             println!("Command: {:?} ({})", command, text);
//!         }
//!         VoiceEngineEvent::PartialResult { text } => {
//!             println!("Partial: {}", text);
//!         }
//!         _ => {}
//!     }
//! }
//!
//! // Stop listening
//! engine.stop()?;
//! ```

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;

use crate::accessibility::voice_control::VoiceCommand;
use crate::storage::config::VoiceActivation;

use super::audio_input::{AudioInputCapture, AudioInputConfig, AudioInputError};
use super::recognizer::{
    RecognizerConfig, RecognizerError, ThreadSafeRecognizer,
    RECOGNIZER_SAMPLE_RATE,
};
use super::wake_word::{
    WakeWordConfig, WakeWordDetector, WakeWordEvent, WakeWordState,
    DEFAULT_ACTIVE_LISTENING_DURATION_MS,
};

/// Default audio buffer read interval in milliseconds.
const DEFAULT_AUDIO_READ_INTERVAL_MS: u64 = 100;

/// Default number of samples to read per interval (100ms at 16kHz).
const DEFAULT_SAMPLES_PER_READ: usize = 1600;

/// Minimum confidence threshold for accepting a command (0.0 - 1.0).
const DEFAULT_MIN_CONFIDENCE: f32 = 0.5;

/// Timeout for silence before finalizing recognition (in milliseconds).
const DEFAULT_SILENCE_TIMEOUT_MS: u64 = 1500;

/// Default cooldown period between same commands (in milliseconds).
/// Prevents rapid repetition of the same command.
const DEFAULT_COMMAND_COOLDOWN_MS: u64 = 1000;

/// Default debounce timeout for brief silences during speech (in milliseconds).
/// Prevents false command triggers from brief pauses while speaking.
const DEFAULT_DEBOUNCE_MS: u64 = 300;

/// Voice engine activation mode.
///
/// Determines how the engine responds to audio input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivationMode {
    /// Always listening and immediately processes commands.
    /// No wake word required - all recognized speech is treated as commands.
    AlwaysListening,

    /// Requires wake word to activate.
    /// The engine listens for "Hey Rust Ride" or "OK Ride" before processing commands.
    /// After wake word detection, enters active listening mode for a configurable duration.
    #[default]
    WakeWord,

    /// Requires manual activation (push-to-talk).
    /// Commands only processed when manually triggered via `activate()`.
    PushToTalk,
}

impl std::fmt::Display for ActivationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivationMode::AlwaysListening => write!(f, "Always Listening"),
            ActivationMode::WakeWord => write!(f, "Wake Word"),
            ActivationMode::PushToTalk => write!(f, "Push to Talk"),
        }
    }
}

/// Convert from config VoiceActivation to engine ActivationMode.
///
/// Note: `VoiceActivation::Off` maps to `ActivationMode::WakeWord` because the engine
/// doesn't have an "off" state - disabling voice control should be done by not
/// starting the engine rather than setting a mode.
impl From<VoiceActivation> for ActivationMode {
    fn from(activation: VoiceActivation) -> Self {
        match activation {
            VoiceActivation::AlwaysOn => ActivationMode::AlwaysListening,
            VoiceActivation::WakeWord => ActivationMode::WakeWord,
            VoiceActivation::PushToTalk => ActivationMode::PushToTalk,
            VoiceActivation::Off => ActivationMode::WakeWord, // Default to WakeWord when disabled
        }
    }
}

/// Convert from engine ActivationMode to config VoiceActivation.
impl From<ActivationMode> for VoiceActivation {
    fn from(mode: ActivationMode) -> Self {
        match mode {
            ActivationMode::AlwaysListening => VoiceActivation::AlwaysOn,
            ActivationMode::WakeWord => VoiceActivation::WakeWord,
            ActivationMode::PushToTalk => VoiceActivation::PushToTalk,
        }
    }
}

/// Errors that can occur during voice engine operations.
#[derive(Debug, Error)]
pub enum VoiceEngineError {
    /// Model path does not exist.
    #[error("Model not found at path: {0}")]
    ModelNotFound(PathBuf),

    /// Audio capture error.
    #[error("Audio input error: {0}")]
    AudioInputError(#[from] AudioInputError),

    /// Recognizer error.
    #[error("Recognizer error: {0}")]
    RecognizerError(#[from] RecognizerError),

    /// Engine is not initialized.
    #[error("Engine not initialized")]
    NotInitialized,

    /// Engine is already running.
    #[error("Engine already running")]
    AlreadyRunning,

    /// Engine is not running.
    #[error("Engine not running")]
    NotRunning,

    /// Worker thread error.
    #[error("Worker thread error: {0}")]
    WorkerError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// State of the voice engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEngineState {
    /// Engine is not initialized.
    Uninitialized,
    /// Engine is initialized but not listening.
    Ready,
    /// Engine is actively listening and processing audio.
    Listening,
    /// Engine is paused.
    Paused,
    /// Engine encountered an error.
    Error,
    /// Engine is shutting down.
    ShuttingDown,
}

impl Default for VoiceEngineState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

impl std::fmt::Display for VoiceEngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceEngineState::Uninitialized => write!(f, "Uninitialized"),
            VoiceEngineState::Ready => write!(f, "Ready"),
            VoiceEngineState::Listening => write!(f, "Listening"),
            VoiceEngineState::Paused => write!(f, "Paused"),
            VoiceEngineState::Error => write!(f, "Error"),
            VoiceEngineState::ShuttingDown => write!(f, "Shutting Down"),
        }
    }
}

/// Events emitted by the voice engine.
#[derive(Debug, Clone)]
pub enum VoiceEngineEvent {
    /// Engine state has changed.
    StateChanged {
        /// Previous state.
        from: VoiceEngineState,
        /// New state.
        to: VoiceEngineState,
    },

    /// A partial recognition result is available.
    /// These are intermediate results while the user is still speaking.
    PartialResult {
        /// The partial recognized text.
        text: String,
    },

    /// A final recognition result is available but no command was matched.
    FinalResult {
        /// The final recognized text.
        text: String,
        /// Confidence score if available.
        confidence: Option<f32>,
    },

    /// A voice command was recognized.
    CommandRecognized {
        /// The recognized command.
        command: VoiceCommand,
        /// The original text that was recognized.
        text: String,
        /// Confidence score if available.
        confidence: Option<f32>,
    },

    /// Audio input level (for visualization).
    AudioLevel {
        /// RMS level (0.0 - 1.0).
        level: f32,
    },

    /// An error occurred.
    Error {
        /// Error message.
        message: String,
    },

    /// Engine was initialized.
    Initialized,

    /// Engine started listening.
    Started,

    /// Engine stopped listening.
    Stopped,

    /// Command was blocked by cooldown.
    CommandCooldown {
        /// The blocked command.
        command: VoiceCommand,
        /// Time remaining on cooldown in milliseconds.
        remaining_ms: u64,
    },

    /// Wake word was detected, entering active listening mode.
    WakeWordDetected {
        /// The wake phrase that was recognized.
        phrase: String,
        /// How long active listening mode will last (milliseconds).
        duration_ms: u64,
    },

    /// Active listening mode timed out, returning to dormant.
    WakeWordTimeout,

    /// Active listening mode was extended.
    WakeWordExtended {
        /// Time remaining in active mode (milliseconds).
        remaining_ms: u64,
    },

    /// Activation mode changed.
    ActivationModeChanged {
        /// New activation mode.
        mode: ActivationMode,
    },
}

/// Tracks cooldown state for voice commands to prevent rapid repetition.
///
/// This struct maintains the last executed command and its timestamp,
/// allowing the engine to reject duplicate commands within the cooldown period.
#[derive(Debug, Clone)]
pub struct CommandCooldown {
    /// The last command that was executed.
    last_command: Option<VoiceCommand>,
    /// Timestamp when the last command was executed.
    last_command_time: Option<Instant>,
    /// Cooldown duration in milliseconds.
    cooldown_ms: u64,
}

impl CommandCooldown {
    /// Create a new cooldown tracker with the specified duration.
    pub fn new(cooldown_ms: u64) -> Self {
        Self {
            last_command: None,
            last_command_time: None,
            cooldown_ms,
        }
    }

    /// Check if a command is allowed (not in cooldown).
    ///
    /// Returns `true` if the command can be executed, `false` if it's blocked
    /// by cooldown. A command is blocked if it's the same as the last command
    /// and the cooldown period hasn't elapsed.
    pub fn is_allowed(&self, command: &VoiceCommand) -> bool {
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

    /// Record that a command was executed.
    pub fn record_command(&mut self, command: &VoiceCommand) {
        // Don't record unknown commands
        if !matches!(command, VoiceCommand::Unknown(_)) {
            self.last_command = Some(command.clone());
            self.last_command_time = Some(Instant::now());
        }
    }

    /// Reset the cooldown state.
    pub fn reset(&mut self) {
        self.last_command = None;
        self.last_command_time = None;
    }

    /// Get the cooldown duration in milliseconds.
    pub fn cooldown_ms(&self) -> u64 {
        self.cooldown_ms
    }

    /// Set the cooldown duration in milliseconds.
    pub fn set_cooldown_ms(&mut self, cooldown_ms: u64) {
        self.cooldown_ms = cooldown_ms;
    }

    /// Get the last executed command, if any.
    pub fn last_command(&self) -> Option<&VoiceCommand> {
        self.last_command.as_ref()
    }

    /// Get the time since the last command was executed, if any.
    pub fn time_since_last_command(&self) -> Option<Duration> {
        self.last_command_time.map(|t| t.elapsed())
    }
}

impl Default for CommandCooldown {
    fn default() -> Self {
        Self::new(DEFAULT_COMMAND_COOLDOWN_MS)
    }
}

/// Configuration for the voice engine.
#[derive(Debug, Clone)]
pub struct VoiceEngineConfig {
    /// Path to the Vosk model directory.
    pub model_path: PathBuf,
    /// Sample rate for audio capture (default: 16000 Hz for Vosk).
    pub sample_rate: u32,
    /// Audio buffer read interval in milliseconds.
    pub audio_read_interval_ms: u64,
    /// Number of samples to read per interval.
    pub samples_per_read: usize,
    /// Minimum confidence threshold for accepting a command.
    pub min_confidence: f32,
    /// Timeout for silence before finalizing recognition.
    pub silence_timeout_ms: u64,
    /// Whether to emit partial results.
    pub emit_partial_results: bool,
    /// Whether to emit audio levels.
    pub emit_audio_levels: bool,
    /// Custom grammar for constrained recognition.
    pub grammar: Option<Vec<String>>,
    /// Cooldown period between same commands in milliseconds.
    /// Prevents rapid repetition of the same command.
    pub command_cooldown_ms: u64,
    /// Debounce timeout for brief silences during speech in milliseconds.
    /// Prevents false command triggers from brief pauses while speaking.
    pub debounce_ms: u64,
    /// Whether to enable command cooldown.
    pub enable_cooldown: bool,
    /// Activation mode for the voice engine.
    pub activation_mode: ActivationMode,
    /// Duration to stay in active listening mode after wake word (milliseconds).
    pub wake_word_active_duration_ms: u64,
    /// Whether wake word detection is enabled (for WakeWord mode).
    pub wake_word_enabled: bool,
}

impl VoiceEngineConfig {
    /// Create a new configuration with the specified model path.
    pub fn new(model_path: impl AsRef<Path>) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
            sample_rate: RECOGNIZER_SAMPLE_RATE as u32,
            audio_read_interval_ms: DEFAULT_AUDIO_READ_INTERVAL_MS,
            samples_per_read: DEFAULT_SAMPLES_PER_READ,
            min_confidence: DEFAULT_MIN_CONFIDENCE,
            silence_timeout_ms: DEFAULT_SILENCE_TIMEOUT_MS,
            emit_partial_results: true,
            emit_audio_levels: false,
            grammar: None,
            command_cooldown_ms: DEFAULT_COMMAND_COOLDOWN_MS,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            enable_cooldown: true,
            activation_mode: ActivationMode::default(),
            wake_word_active_duration_ms: DEFAULT_ACTIVE_LISTENING_DURATION_MS,
            wake_word_enabled: true,
        }
    }

    /// Create a configuration optimized for command recognition.
    pub fn for_commands(model_path: impl AsRef<Path>) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
            sample_rate: RECOGNIZER_SAMPLE_RATE as u32,
            audio_read_interval_ms: DEFAULT_AUDIO_READ_INTERVAL_MS,
            samples_per_read: DEFAULT_SAMPLES_PER_READ,
            min_confidence: DEFAULT_MIN_CONFIDENCE,
            silence_timeout_ms: DEFAULT_SILENCE_TIMEOUT_MS,
            emit_partial_results: true,
            emit_audio_levels: false,
            grammar: Some(RecognizerConfig::for_commands(&model_path).grammar.unwrap()),
            command_cooldown_ms: DEFAULT_COMMAND_COOLDOWN_MS,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            enable_cooldown: true,
            activation_mode: ActivationMode::WakeWord,
            wake_word_active_duration_ms: DEFAULT_ACTIVE_LISTENING_DURATION_MS,
            wake_word_enabled: true,
        }
    }

    /// Set the minimum confidence threshold.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set whether to emit partial results.
    pub fn with_partial_results(mut self, emit: bool) -> Self {
        self.emit_partial_results = emit;
        self
    }

    /// Set whether to emit audio levels.
    pub fn with_audio_levels(mut self, emit: bool) -> Self {
        self.emit_audio_levels = emit;
        self
    }

    /// Set the silence timeout.
    pub fn with_silence_timeout(mut self, timeout_ms: u64) -> Self {
        self.silence_timeout_ms = timeout_ms;
        self
    }

    /// Set a custom grammar for constrained recognition.
    pub fn with_grammar(mut self, grammar: Vec<String>) -> Self {
        self.grammar = Some(grammar);
        self
    }

    /// Set the command cooldown duration in milliseconds.
    ///
    /// This prevents rapid repetition of the same command. Set to 0 to disable.
    pub fn with_command_cooldown(mut self, cooldown_ms: u64) -> Self {
        self.command_cooldown_ms = cooldown_ms;
        self
    }

    /// Set the debounce timeout for brief silences in milliseconds.
    ///
    /// This helps prevent false triggers from brief pauses while speaking.
    pub fn with_debounce(mut self, debounce_ms: u64) -> Self {
        self.debounce_ms = debounce_ms;
        self
    }

    /// Enable or disable command cooldown.
    pub fn with_cooldown_enabled(mut self, enabled: bool) -> Self {
        self.enable_cooldown = enabled;
        self
    }

    /// Set the activation mode.
    ///
    /// - `AlwaysListening`: Immediately processes all recognized speech as commands.
    /// - `WakeWord`: Requires "Hey Rust Ride" or "OK Ride" before processing commands.
    /// - `PushToTalk`: Only processes commands when manually activated.
    pub fn with_activation_mode(mut self, mode: ActivationMode) -> Self {
        self.activation_mode = mode;
        self
    }

    /// Set the wake word active listening duration in milliseconds.
    ///
    /// After a wake word is detected, the engine will actively listen for
    /// commands for this duration before returning to dormant mode.
    pub fn with_wake_word_duration(mut self, duration_ms: u64) -> Self {
        self.wake_word_active_duration_ms = duration_ms;
        self
    }

    /// Enable or disable wake word detection.
    pub fn with_wake_word_enabled(mut self, enabled: bool) -> Self {
        self.wake_word_enabled = enabled;
        self
    }

    /// Create a configuration for always-on listening (no wake word).
    pub fn always_listening(model_path: impl AsRef<Path>) -> Self {
        Self::for_commands(&model_path).with_activation_mode(ActivationMode::AlwaysListening)
    }

    /// Create a configuration for push-to-talk mode.
    pub fn push_to_talk(model_path: impl AsRef<Path>) -> Self {
        Self::for_commands(&model_path).with_activation_mode(ActivationMode::PushToTalk)
    }
}

/// Commands sent to the engine worker thread.
#[derive(Debug)]
enum EngineCommand {
    /// Initialize the engine (load model, setup audio).
    Initialize {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Start listening for voice commands.
    Start {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Stop listening.
    Stop {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Pause listening (keeps resources allocated).
    Pause {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Resume from paused state.
    Resume {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Reset the recognizer state.
    Reset {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Update configuration.
    UpdateConfig {
        config: VoiceEngineConfig,
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Set the activation mode.
    SetActivationMode {
        mode: ActivationMode,
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Manually activate (enter active listening mode).
    /// Used for push-to-talk or manual activation.
    Activate {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Manually deactivate (return to dormant mode).
    Deactivate {
        response_tx: Sender<Result<(), VoiceEngineError>>,
    },
    /// Shutdown the worker thread.
    Shutdown,
}

/// Thread-safe voice recognition engine.
///
/// This engine orchestrates the complete voice recognition pipeline:
/// - Audio capture from microphone
/// - Speech recognition using Vosk
/// - Command parsing
///
/// It runs on a dedicated worker thread and communicates via channels.
pub struct VoiceEngine {
    /// Channel to send commands to the worker thread.
    command_tx: Mutex<Option<Sender<EngineCommand>>>,
    /// Handle to the worker thread.
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    /// Configuration.
    config: RwLock<VoiceEngineConfig>,
    /// Current state.
    state: Arc<RwLock<VoiceEngineState>>,
    /// Whether the engine has been initialized.
    initialized: AtomicBool,
    /// Event broadcaster.
    event_tx: broadcast::Sender<VoiceEngineEvent>,
    /// Last error message.
    last_error: RwLock<Option<String>>,
}

impl VoiceEngine {
    /// Create a new voice engine with the specified configuration.
    pub fn new(config: VoiceEngineConfig) -> Result<Self, VoiceEngineError> {
        // Validate model path exists
        if !config.model_path.exists() {
            return Err(VoiceEngineError::ModelNotFound(config.model_path.clone()));
        }

        let (event_tx, _) = broadcast::channel(100);

        Ok(Self {
            command_tx: Mutex::new(None),
            worker_handle: Mutex::new(None),
            config: RwLock::new(config),
            state: Arc::new(RwLock::new(VoiceEngineState::Uninitialized)),
            initialized: AtomicBool::new(false),
            event_tx,
            last_error: RwLock::new(None),
        })
    }

    /// Create a new voice engine with a model path (uses default configuration).
    pub fn with_model_path(model_path: impl AsRef<Path>) -> Result<Self, VoiceEngineError> {
        let config = VoiceEngineConfig::for_commands(&model_path);
        Self::new(config)
    }

    /// Get the current configuration.
    pub fn config(&self) -> VoiceEngineConfig {
        self.config.read().unwrap().clone()
    }

    /// Get the current state.
    pub fn state(&self) -> VoiceEngineState {
        *self.state.read().unwrap()
    }

    /// Check if the engine is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Check if the engine is currently listening.
    pub fn is_listening(&self) -> bool {
        matches!(self.state(), VoiceEngineState::Listening)
    }

    /// Get the last error message.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().unwrap().clone()
    }

    /// Subscribe to engine events.
    ///
    /// Returns a receiver that will receive all engine events.
    pub fn subscribe(&self) -> broadcast::Receiver<VoiceEngineEvent> {
        self.event_tx.subscribe()
    }

    /// Emit an event to all subscribers.
    fn emit_event(&self, event: VoiceEngineEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Update the state and emit state change event.
    fn set_state(&self, new_state: VoiceEngineState) {
        let old_state = {
            let mut state = self.state.write().unwrap();
            let old = *state;
            *state = new_state;
            old
        };

        if old_state != new_state {
            self.emit_event(VoiceEngineEvent::StateChanged {
                from: old_state,
                to: new_state,
            });
        }
    }

    /// Set an error state with message.
    fn set_error(&self, message: impl Into<String>) {
        let msg = message.into();
        *self.last_error.write().unwrap() = Some(msg.clone());
        self.set_state(VoiceEngineState::Error);
        self.emit_event(VoiceEngineEvent::Error { message: msg });
    }

    /// Start the worker thread.
    fn start_worker(&self) -> Result<(), VoiceEngineError> {
        let (command_tx, command_rx) = mpsc::channel::<EngineCommand>();
        let config = self.config.read().unwrap().clone();
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        let handle = thread::Builder::new()
            .name("voice-engine".to_string())
            .spawn(move || {
                Self::worker_thread(command_rx, config, state, event_tx);
            })
            .map_err(|e| VoiceEngineError::WorkerError(format!("Failed to spawn thread: {}", e)))?;

        *self.command_tx.lock().unwrap() = Some(command_tx);
        *self.worker_handle.lock().unwrap() = Some(handle);

        Ok(())
    }

    /// The worker thread function.
    fn worker_thread(
        command_rx: Receiver<EngineCommand>,
        config: VoiceEngineConfig,
        state: Arc<RwLock<VoiceEngineState>>,
        event_tx: broadcast::Sender<VoiceEngineEvent>,
    ) {
        tracing::debug!("Voice engine worker thread started");

        let mut current_config = config;
        let mut audio_capture: Option<AudioInputCapture> = None;
        let mut recognizer: Option<ThreadSafeRecognizer> = None;
        let mut is_listening = false;
        let mut last_speech_time: Option<Instant> = None;
        let mut last_partial_text = String::new();

        // Command cooldown tracker to prevent rapid repetition
        let mut cooldown = CommandCooldown::new(current_config.command_cooldown_ms);

        // Debounce timer - tracks when we last had speech activity
        // to prevent false triggers from brief pauses
        let mut last_audio_activity: Option<Instant> = None;

        // Wake word detector for activation mode
        let wake_word_config = WakeWordConfig::new(current_config.wake_word_active_duration_ms)
            .with_enabled(current_config.wake_word_enabled);
        let mut wake_word_detector = WakeWordDetector::new(wake_word_config);

        // Current activation mode
        let mut activation_mode = current_config.activation_mode;

        // Helper to update state
        let update_state = |new_state: VoiceEngineState| {
            let old_state = {
                let mut s = state.write().unwrap();
                let old = *s;
                *s = new_state;
                old
            };
            if old_state != new_state {
                let _ = event_tx.send(VoiceEngineEvent::StateChanged {
                    from: old_state,
                    to: new_state,
                });
            }
        };

        // Helper to emit event
        let emit = |event: VoiceEngineEvent| {
            let _ = event_tx.send(event);
        };

        loop {
            // Check for commands (non-blocking when listening)
            let command = if is_listening {
                // Use timeout to allow audio processing
                command_rx
                    .recv_timeout(Duration::from_millis(current_config.audio_read_interval_ms))
                    .ok()
            } else {
                // Block waiting for command when not listening
                command_rx.recv().ok()
            };

            // Process command if received
            if let Some(cmd) = command {
                match cmd {
                    EngineCommand::Initialize { response_tx } => {
                        let result = Self::handle_initialize(
                            &current_config,
                            &mut audio_capture,
                            &mut recognizer,
                        );
                        if result.is_ok() {
                            update_state(VoiceEngineState::Ready);
                            emit(VoiceEngineEvent::Initialized);
                        } else {
                            update_state(VoiceEngineState::Error);
                        }
                        let _ = response_tx.send(result);
                    }
                    EngineCommand::Start { response_tx } => {
                        let result = Self::handle_start(&audio_capture, &mut is_listening);
                        if result.is_ok() {
                            update_state(VoiceEngineState::Listening);
                            last_speech_time = Some(Instant::now());
                            last_partial_text.clear();
                            emit(VoiceEngineEvent::Started);
                        }
                        let _ = response_tx.send(result);
                    }
                    EngineCommand::Stop { response_tx } => {
                        let result =
                            Self::handle_stop(&audio_capture, &recognizer, &mut is_listening);
                        if result.is_ok() {
                            update_state(VoiceEngineState::Ready);
                            emit(VoiceEngineEvent::Stopped);
                        }
                        let _ = response_tx.send(result);
                    }
                    EngineCommand::Pause { response_tx } => {
                        if let Some(ref capture) = audio_capture {
                            let _ = capture.pause();
                        }
                        is_listening = false;
                        update_state(VoiceEngineState::Paused);
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::Resume { response_tx } => {
                        if let Some(ref capture) = audio_capture {
                            let _ = capture.resume();
                        }
                        is_listening = true;
                        last_speech_time = Some(Instant::now());
                        update_state(VoiceEngineState::Listening);
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::Reset { response_tx } => {
                        if let Some(ref rec) = recognizer {
                            let _ = rec.reset();
                        }
                        last_partial_text.clear();
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::UpdateConfig {
                        config: new_config,
                        response_tx,
                    } => {
                        // Update wake word detector if settings changed
                        if new_config.wake_word_active_duration_ms
                            != current_config.wake_word_active_duration_ms
                            || new_config.wake_word_enabled != current_config.wake_word_enabled
                        {
                            let new_wake_config =
                                WakeWordConfig::new(new_config.wake_word_active_duration_ms)
                                    .with_enabled(new_config.wake_word_enabled);
                            wake_word_detector.update_config(new_wake_config);
                        }

                        // Update activation mode if changed
                        if new_config.activation_mode != current_config.activation_mode {
                            activation_mode = new_config.activation_mode;
                            emit(VoiceEngineEvent::ActivationModeChanged {
                                mode: activation_mode,
                            });

                            // Reset wake word detector when mode changes
                            wake_word_detector.reset();
                        }

                        current_config = new_config;
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::SetActivationMode { mode, response_tx } => {
                        if mode != activation_mode {
                            activation_mode = mode;
                            wake_word_detector.reset();
                            emit(VoiceEngineEvent::ActivationModeChanged { mode });
                            tracing::info!("Activation mode changed to: {}", mode);
                        }
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::Activate { response_tx } => {
                        // Manually enter active mode (for push-to-talk)
                        if let Some(event) = wake_word_detector.activate() {
                            if let WakeWordEvent::StateChanged { to: WakeWordState::Active, .. } = event {
                                emit(VoiceEngineEvent::WakeWordDetected {
                                    phrase: "manual activation".to_string(),
                                    duration_ms: current_config.wake_word_active_duration_ms,
                                });
                            }
                        }
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::Deactivate { response_tx } => {
                        // Manually exit active mode
                        if let Some(_event) = wake_word_detector.deactivate() {
                            emit(VoiceEngineEvent::WakeWordTimeout);
                        }
                        let _ = response_tx.send(Ok(()));
                    }
                    EngineCommand::Shutdown => {
                        tracing::debug!("Voice engine worker: shutting down");
                        update_state(VoiceEngineState::ShuttingDown);

                        // Clean up
                        if let Some(ref capture) = audio_capture {
                            let _ = capture.stop();
                        }
                        break;
                    }
                }
            }

            // Process audio when listening
            if is_listening {
                // Check for wake word timeout (only in WakeWord mode)
                if activation_mode == ActivationMode::WakeWord {
                    if let Some(event) = wake_word_detector.check_timeout() {
                        if let WakeWordEvent::StateChanged { to: WakeWordState::Dormant, .. } = event
                        {
                            emit(VoiceEngineEvent::WakeWordTimeout);
                            tracing::debug!("Wake word active period expired");
                        }
                    }
                }

                if let (Some(ref capture), Some(ref rec)) = (&audio_capture, &recognizer) {
                    // Read audio samples from buffer
                    let samples = capture.read_samples(current_config.samples_per_read);

                    if !samples.is_empty() {
                        // Calculate audio level
                        let level = Self::calculate_audio_level(&samples);

                        // Emit audio level if enabled
                        if current_config.emit_audio_levels {
                            emit(VoiceEngineEvent::AudioLevel { level });
                        }

                        // Update audio activity tracker for debouncing
                        if level > 0.01 {
                            last_audio_activity = Some(Instant::now());
                        }

                        // Feed samples to recognizer
                        match rec.accept_waveform(&samples) {
                            Ok(has_final) => {
                                if has_final {
                                    // Apply debounce: only process if we've had
                                    // continuous activity or sufficient silence
                                    let debounce_elapsed = last_audio_activity
                                        .map(|t| t.elapsed().as_millis() as u64)
                                        .unwrap_or(current_config.debounce_ms);

                                    // If there's been recent activity, wait for debounce
                                    // This prevents partial speech from triggering commands
                                    if debounce_elapsed < current_config.debounce_ms
                                        && !last_partial_text.is_empty()
                                    {
                                        // Still have activity, wait a bit more
                                        tracing::trace!(
                                            "Debouncing: {}ms since last activity",
                                            debounce_elapsed
                                        );
                                    } else {
                                        // Process based on activation mode
                                        Self::process_final_result_with_activation(
                                            rec,
                                            &current_config,
                                            &event_tx,
                                            &mut last_partial_text,
                                            &mut cooldown,
                                            &mut wake_word_detector,
                                            activation_mode,
                                        );
                                    }
                                    last_speech_time = Some(Instant::now());
                                } else if current_config.emit_partial_results {
                                    // Get partial result and check for wake word
                                    Self::process_partial_result_with_activation(
                                        rec,
                                        &event_tx,
                                        &mut last_partial_text,
                                        &mut wake_word_detector,
                                        activation_mode,
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Recognizer error: {}", e);
                                emit(VoiceEngineEvent::Error {
                                    message: e.to_string(),
                                });
                            }
                        }

                        // Update last speech time if we have audio activity
                        if level > 0.01 {
                            // Threshold for "activity"
                            last_speech_time = Some(Instant::now());
                        }
                    }

                    // Check for silence timeout
                    if let Some(last_time) = last_speech_time {
                        if last_time.elapsed().as_millis()
                            > current_config.silence_timeout_ms as u128
                        {
                            // Force final result after silence (with activation check)
                            if !last_partial_text.is_empty() {
                                Self::process_final_result_with_activation(
                                    rec,
                                    &current_config,
                                    &event_tx,
                                    &mut last_partial_text,
                                    &mut cooldown,
                                    &mut wake_word_detector,
                                    activation_mode,
                                );
                            }
                            last_speech_time = Some(Instant::now());
                        }
                    }
                }
            }
        }

        tracing::debug!("Voice engine worker thread exiting");
    }

    /// Handle the Initialize command.
    fn handle_initialize(
        config: &VoiceEngineConfig,
        audio_capture: &mut Option<AudioInputCapture>,
        recognizer: &mut Option<ThreadSafeRecognizer>,
    ) -> Result<(), VoiceEngineError> {
        tracing::info!(
            "Initializing voice engine with model at {:?}",
            config.model_path
        );

        // Create audio capture
        let audio_config = AudioInputConfig::for_vosk()
            .with_sample_rate(config.sample_rate)
            .with_buffer_size(config.samples_per_read * 4); // 4x buffer for safety

        let capture = AudioInputCapture::new(audio_config)?;
        *audio_capture = Some(capture);

        // Create recognizer with appropriate config
        let rec_config = if let Some(ref grammar) = config.grammar {
            RecognizerConfig::new(&config.model_path)
                .with_grammar(grammar.clone())
                .with_partial_results(config.emit_partial_results)
        } else {
            RecognizerConfig::for_commands(&config.model_path)
                .with_partial_results(config.emit_partial_results)
        };

        let rec = ThreadSafeRecognizer::with_config(rec_config);
        rec.initialize()?;
        *recognizer = Some(rec);

        tracing::info!("Voice engine initialized successfully");
        Ok(())
    }

    /// Handle the Start command.
    fn handle_start(
        audio_capture: &Option<AudioInputCapture>,
        is_listening: &mut bool,
    ) -> Result<(), VoiceEngineError> {
        let capture = audio_capture
            .as_ref()
            .ok_or(VoiceEngineError::NotInitialized)?;

        capture.start()?;
        *is_listening = true;

        tracing::info!("Voice engine started listening");
        Ok(())
    }

    /// Handle the Stop command.
    fn handle_stop(
        audio_capture: &Option<AudioInputCapture>,
        recognizer: &Option<ThreadSafeRecognizer>,
        is_listening: &mut bool,
    ) -> Result<(), VoiceEngineError> {
        if let Some(ref capture) = audio_capture {
            let _ = capture.stop();
        }

        if let Some(ref rec) = recognizer {
            let _ = rec.reset();
        }

        *is_listening = false;

        tracing::info!("Voice engine stopped listening");
        Ok(())
    }

    /// Process and emit a partial recognition result.
    fn process_partial_result(
        recognizer: &ThreadSafeRecognizer,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        last_partial_text: &mut String,
    ) {
        if let Ok(result) = recognizer.partial_result() {
            if !result.is_empty() && result.text != *last_partial_text {
                *last_partial_text = result.text.clone();
                let _ = event_tx.send(VoiceEngineEvent::PartialResult { text: result.text });
            }
        }
    }

    /// Process and emit a final recognition result.
    ///
    /// This method applies cooldown checking to prevent rapid command repetition.
    /// If a command is blocked by cooldown, a `CommandCooldown` event is emitted.
    fn process_final_result(
        recognizer: &ThreadSafeRecognizer,
        config: &VoiceEngineConfig,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        last_partial_text: &mut String,
        cooldown: &mut CommandCooldown,
    ) {
        if let Ok(result) = recognizer.final_result() {
            last_partial_text.clear();

            if !result.is_empty() {
                let text = result.text.trim().to_string();
                let confidence = result.confidence;

                // Skip low confidence results
                if let Some(conf) = confidence {
                    if conf < config.min_confidence {
                        tracing::debug!(
                            "Skipping low confidence result: '{}' ({:.2})",
                            text,
                            conf
                        );
                        return;
                    }
                }

                // Try to parse as a command
                let command = VoiceCommand::from_phrase(&text);

                if matches!(command, VoiceCommand::Unknown(_)) {
                    // Not a recognized command, emit as final result
                    let _ = event_tx.send(VoiceEngineEvent::FinalResult { text, confidence });
                } else {
                    // Check cooldown if enabled
                    if config.enable_cooldown {
                        if let Some(remaining_ms) = cooldown.remaining_cooldown_ms(&command) {
                            tracing::debug!(
                                "Command {:?} blocked by cooldown ({}ms remaining)",
                                command,
                                remaining_ms
                            );
                            let _ = event_tx.send(VoiceEngineEvent::CommandCooldown {
                                command,
                                remaining_ms,
                            });
                            return;
                        }
                    }

                    // Record the command for cooldown tracking
                    cooldown.record_command(&command);

                    // Recognized command
                    let _ = event_tx.send(VoiceEngineEvent::CommandRecognized {
                        command,
                        text,
                        confidence,
                    });
                }
            }
        }
    }

    /// Process a partial result with activation mode awareness.
    ///
    /// In WakeWord mode, this checks partial results for wake phrases.
    /// In other modes, it simply forwards the partial result.
    fn process_partial_result_with_activation(
        recognizer: &ThreadSafeRecognizer,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        last_partial_text: &mut String,
        wake_word_detector: &mut WakeWordDetector,
        activation_mode: ActivationMode,
    ) {
        if let Ok(result) = recognizer.partial_result() {
            if result.is_empty() {
                return;
            }

            let text = result.text.clone();

            // In WakeWord mode, check for wake phrases in partial results
            if activation_mode == ActivationMode::WakeWord && !wake_word_detector.is_active() {
                if let Some(wake_event) = wake_word_detector.process_text(&text) {
                    match wake_event {
                        WakeWordEvent::Detected { phrase, duration_ms } => {
                            let _ = event_tx.send(VoiceEngineEvent::WakeWordDetected {
                                phrase,
                                duration_ms,
                            });
                        }
                        WakeWordEvent::Extended { remaining_ms } => {
                            let _ = event_tx.send(VoiceEngineEvent::WakeWordExtended {
                                remaining_ms,
                            });
                        }
                        _ => {}
                    }
                }
            }

            // Emit partial result if changed
            if text != *last_partial_text {
                *last_partial_text = text.clone();
                let _ = event_tx.send(VoiceEngineEvent::PartialResult { text });
            }
        }
    }

    /// Process a final result with activation mode awareness.
    ///
    /// - In `AlwaysListening` mode: Process all recognized speech as commands.
    /// - In `WakeWord` mode: Check for wake word first, then process commands only when active.
    /// - In `PushToTalk` mode: Only process commands when manually activated.
    fn process_final_result_with_activation(
        recognizer: &ThreadSafeRecognizer,
        config: &VoiceEngineConfig,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        last_partial_text: &mut String,
        cooldown: &mut CommandCooldown,
        wake_word_detector: &mut WakeWordDetector,
        activation_mode: ActivationMode,
    ) {
        if let Ok(result) = recognizer.final_result() {
            last_partial_text.clear();

            if result.is_empty() {
                return;
            }

            let text = result.text.trim().to_string();
            let confidence = result.confidence;

            // Skip low confidence results
            if let Some(conf) = confidence {
                if conf < config.min_confidence {
                    tracing::debug!(
                        "Skipping low confidence result: '{}' ({:.2})",
                        text,
                        conf
                    );
                    return;
                }
            }

            // Handle based on activation mode
            match activation_mode {
                ActivationMode::AlwaysListening => {
                    // Process all speech as commands
                    Self::process_command_with_cooldown(
                        &text,
                        confidence,
                        config,
                        event_tx,
                        cooldown,
                    );
                }
                ActivationMode::WakeWord => {
                    // Check for wake word first
                    if let Some(wake_event) = wake_word_detector.process_text(&text) {
                        match wake_event {
                            WakeWordEvent::Detected { phrase, duration_ms } => {
                                let _ = event_tx.send(VoiceEngineEvent::WakeWordDetected {
                                    phrase,
                                    duration_ms,
                                });
                                // Don't process the wake word as a command
                                return;
                            }
                            WakeWordEvent::Extended { remaining_ms } => {
                                let _ = event_tx.send(VoiceEngineEvent::WakeWordExtended {
                                    remaining_ms,
                                });
                            }
                            _ => {}
                        }
                    }

                    // Only process commands when in active mode
                    if wake_word_detector.is_active() {
                        // Extend the active period when receiving commands
                        if let Some(WakeWordEvent::Extended { remaining_ms }) =
                            wake_word_detector.extend_active()
                        {
                            let _ = event_tx.send(VoiceEngineEvent::WakeWordExtended {
                                remaining_ms,
                            });
                        }

                        Self::process_command_with_cooldown(
                            &text,
                            confidence,
                            config,
                            event_tx,
                            cooldown,
                        );
                    } else {
                        // Not active - emit as final result but don't process as command
                        tracing::trace!(
                            "Ignoring '{}' - not in active mode (say 'Hey Rust Ride' first)",
                            text
                        );
                        let _ = event_tx.send(VoiceEngineEvent::FinalResult { text, confidence });
                    }
                }
                ActivationMode::PushToTalk => {
                    // Only process commands when manually activated
                    if wake_word_detector.is_active() {
                        Self::process_command_with_cooldown(
                            &text,
                            confidence,
                            config,
                            event_tx,
                            cooldown,
                        );
                    } else {
                        // Not active - ignore
                        tracing::trace!(
                            "Ignoring '{}' - push-to-talk not active",
                            text
                        );
                    }
                }
            }
        }
    }

    /// Process recognized text as a command with cooldown checking.
    fn process_command_with_cooldown(
        text: &str,
        confidence: Option<f32>,
        config: &VoiceEngineConfig,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        cooldown: &mut CommandCooldown,
    ) {
        let command = VoiceCommand::from_phrase(text);

        if matches!(command, VoiceCommand::Unknown(_)) {
            // Not a recognized command, emit as final result
            let _ = event_tx.send(VoiceEngineEvent::FinalResult {
                text: text.to_string(),
                confidence,
            });
        } else {
            // Check cooldown if enabled
            if config.enable_cooldown {
                if let Some(remaining_ms) = cooldown.remaining_cooldown_ms(&command) {
                    tracing::debug!(
                        "Command {:?} blocked by cooldown ({}ms remaining)",
                        command,
                        remaining_ms
                    );
                    let _ = event_tx.send(VoiceEngineEvent::CommandCooldown {
                        command,
                        remaining_ms,
                    });
                    return;
                }
            }

            // Record the command for cooldown tracking
            cooldown.record_command(&command);

            // Recognized command
            let _ = event_tx.send(VoiceEngineEvent::CommandRecognized {
                command,
                text: text.to_string(),
                confidence,
            });
        }
    }

    /// Calculate the RMS audio level from samples.
    fn calculate_audio_level(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f64 = samples
            .iter()
            .map(|&s| {
                let normalized = s as f64 / i16::MAX as f64;
                normalized * normalized
            })
            .sum();

        let rms = (sum_squares / samples.len() as f64).sqrt();
        rms.min(1.0) as f32
    }

    /// Send a command to the worker thread and wait for response.
    fn send_command<F, T>(&self, make_command: F) -> Result<T, VoiceEngineError>
    where
        F: FnOnce(Sender<Result<T, VoiceEngineError>>) -> EngineCommand,
        T: Send + 'static,
    {
        let guard = self.command_tx.lock().unwrap();
        let tx = guard
            .as_ref()
            .ok_or(VoiceEngineError::WorkerError("Worker not started".to_string()))?;

        let (response_tx, response_rx) = mpsc::channel();
        let command = make_command(response_tx);

        tx.send(command)
            .map_err(|_| VoiceEngineError::WorkerError("Failed to send command".to_string()))?;

        response_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| VoiceEngineError::WorkerError("Command timeout".to_string()))?
    }

    /// Initialize the voice engine.
    ///
    /// This loads the Vosk model and sets up audio capture.
    /// Must be called before `start()`.
    pub fn initialize(&self) -> Result<(), VoiceEngineError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        // Start worker thread if not running
        {
            let guard = self.command_tx.lock().unwrap();
            if guard.is_none() {
                drop(guard);
                self.start_worker()?;
            }
        }

        // Send initialize command
        self.send_command(|response_tx| EngineCommand::Initialize { response_tx })?;

        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// Start listening for voice commands.
    ///
    /// The engine must be initialized before calling this.
    pub fn start(&self) -> Result<(), VoiceEngineError> {
        if !self.initialized.load(Ordering::Acquire) {
            self.initialize()?;
        }

        if self.is_listening() {
            return Err(VoiceEngineError::AlreadyRunning);
        }

        self.send_command(|response_tx| EngineCommand::Start { response_tx })
    }

    /// Stop listening for voice commands.
    pub fn stop(&self) -> Result<(), VoiceEngineError> {
        if !self.is_listening() && self.state() != VoiceEngineState::Paused {
            return Err(VoiceEngineError::NotRunning);
        }

        self.send_command(|response_tx| EngineCommand::Stop { response_tx })
    }

    /// Pause listening (keeps resources allocated).
    pub fn pause(&self) -> Result<(), VoiceEngineError> {
        if !self.is_listening() {
            return Err(VoiceEngineError::NotRunning);
        }

        self.send_command(|response_tx| EngineCommand::Pause { response_tx })
    }

    /// Resume from paused state.
    pub fn resume(&self) -> Result<(), VoiceEngineError> {
        if self.state() != VoiceEngineState::Paused {
            return Err(VoiceEngineError::NotRunning);
        }

        self.send_command(|response_tx| EngineCommand::Resume { response_tx })
    }

    /// Reset the recognizer state.
    ///
    /// This clears any pending recognition and starts fresh.
    pub fn reset(&self) -> Result<(), VoiceEngineError> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(VoiceEngineError::NotInitialized);
        }

        self.send_command(|response_tx| EngineCommand::Reset { response_tx })
    }

    /// Update the engine configuration.
    pub fn update_config(&self, config: VoiceEngineConfig) -> Result<(), VoiceEngineError> {
        // Update local config
        *self.config.write().unwrap() = config.clone();

        // Update worker config if running
        if self.command_tx.lock().unwrap().is_some() {
            self.send_command(|response_tx| EngineCommand::UpdateConfig { config, response_tx })?;
        }

        Ok(())
    }

    /// Set the activation mode.
    ///
    /// - `AlwaysListening`: Immediately processes all recognized speech as commands.
    /// - `WakeWord`: Requires "Hey Rust Ride" or "OK Ride" before processing commands.
    /// - `PushToTalk`: Only processes commands when manually activated via `activate()`.
    pub fn set_activation_mode(&self, mode: ActivationMode) -> Result<(), VoiceEngineError> {
        if !self.initialized.load(Ordering::Acquire) {
            // Update local config, will take effect when initialized
            self.config.write().unwrap().activation_mode = mode;
            return Ok(());
        }

        // Update local config
        self.config.write().unwrap().activation_mode = mode;

        // Update worker
        self.send_command(|response_tx| EngineCommand::SetActivationMode { mode, response_tx })
    }

    /// Get the current activation mode.
    pub fn activation_mode(&self) -> ActivationMode {
        self.config.read().unwrap().activation_mode
    }

    /// Manually enter active listening mode.
    ///
    /// This is used for push-to-talk or manual activation. In WakeWord mode,
    /// this is equivalent to detecting the wake word.
    pub fn activate(&self) -> Result<(), VoiceEngineError> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(VoiceEngineError::NotInitialized);
        }

        self.send_command(|response_tx| EngineCommand::Activate { response_tx })
    }

    /// Manually exit active listening mode.
    ///
    /// Returns to dormant mode (in WakeWord mode) or stops processing commands
    /// (in PushToTalk mode).
    pub fn deactivate(&self) -> Result<(), VoiceEngineError> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(VoiceEngineError::NotInitialized);
        }

        self.send_command(|response_tx| EngineCommand::Deactivate { response_tx })
    }

    /// Apply voice control settings from config.
    ///
    /// This method allows runtime mode switching without restart by converting
    /// the config-level `VoiceActivation` setting to the engine's `ActivationMode`.
    ///
    /// # Arguments
    ///
    /// * `activation` - The voice activation mode from `AccessibilitySettings`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustride::storage::config::VoiceActivation;
    /// use rustride::voice::VoiceEngine;
    ///
    /// let engine = VoiceEngine::new(config)?;
    /// engine.initialize()?;
    /// engine.start()?;
    ///
    /// // User changes setting in UI
    /// let new_activation = VoiceActivation::PushToTalk;
    /// engine.apply_voice_activation(new_activation)?;
    /// ```
    pub fn apply_voice_activation(&self, activation: VoiceActivation) -> Result<(), VoiceEngineError> {
        let mode: ActivationMode = activation.into();
        self.set_activation_mode(mode)
    }

    /// Get the current activation mode as a config VoiceActivation.
    ///
    /// This converts the engine's internal `ActivationMode` to the config-level
    /// `VoiceActivation` enum for saving to settings.
    pub fn voice_activation(&self) -> VoiceActivation {
        self.activation_mode().into()
    }

    /// Create a VoiceEngineConfig from VoiceActivation settings.
    ///
    /// This is a helper to create engine configuration that respects the
    /// user's accessibility settings.
    pub fn config_from_settings(
        model_path: impl AsRef<Path>,
        activation: VoiceActivation,
    ) -> VoiceEngineConfig {
        let mode: ActivationMode = activation.into();
        VoiceEngineConfig::for_commands(&model_path).with_activation_mode(mode)
    }
}

impl Drop for VoiceEngine {
    fn drop(&mut self) {
        // Send shutdown command to worker thread
        if let Some(tx) = self.command_tx.lock().unwrap().take() {
            let _ = tx.send(EngineCommand::Shutdown);
        }
        // Wait for worker thread to finish
        if let Some(handle) = self.worker_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

// Verify VoiceEngine is Send + Sync
fn _assert_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<VoiceEngine>();
    assert_sync::<VoiceEngine>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_engine_state_display() {
        assert_eq!(VoiceEngineState::Uninitialized.to_string(), "Uninitialized");
        assert_eq!(VoiceEngineState::Ready.to_string(), "Ready");
        assert_eq!(VoiceEngineState::Listening.to_string(), "Listening");
        assert_eq!(VoiceEngineState::Paused.to_string(), "Paused");
        assert_eq!(VoiceEngineState::Error.to_string(), "Error");
        assert_eq!(
            VoiceEngineState::ShuttingDown.to_string(),
            "Shutting Down"
        );
    }

    #[test]
    fn test_engine_state_default() {
        assert_eq!(VoiceEngineState::default(), VoiceEngineState::Uninitialized);
    }

    #[test]
    fn test_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        assert_eq!(config.model_path, model_path);
        assert_eq!(config.sample_rate, RECOGNIZER_SAMPLE_RATE as u32);
        assert!(config.emit_partial_results);
        assert!(!config.emit_audio_levels);
    }

    #[test]
    fn test_config_for_commands() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::for_commands(&model_path);
        assert!(config.grammar.is_some());
        let grammar = config.grammar.unwrap();
        assert!(grammar.contains(&"pause".to_string()));
        assert!(grammar.contains(&"resume".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path)
            .with_min_confidence(0.8)
            .with_partial_results(false)
            .with_audio_levels(true)
            .with_silence_timeout(2000)
            .with_grammar(vec!["test".to_string()]);

        assert_eq!(config.min_confidence, 0.8);
        assert!(!config.emit_partial_results);
        assert!(config.emit_audio_levels);
        assert_eq!(config.silence_timeout_ms, 2000);
        assert_eq!(config.grammar, Some(vec!["test".to_string()]));
    }

    #[test]
    fn test_config_min_confidence_clamping() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path).with_min_confidence(1.5);
        assert_eq!(config.min_confidence, 1.0);

        let config = VoiceEngineConfig::new(&model_path).with_min_confidence(-0.5);
        assert_eq!(config.min_confidence, 0.0);
    }

    #[test]
    fn test_engine_model_not_found() {
        let config = VoiceEngineConfig::new("/nonexistent/path");
        let result = VoiceEngine::new(config);
        assert!(matches!(result, Err(VoiceEngineError::ModelNotFound(_))));
    }

    #[test]
    fn test_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        let engine = VoiceEngine::new(config);
        assert!(engine.is_ok());

        let engine = engine.unwrap();
        assert_eq!(engine.state(), VoiceEngineState::Uninitialized);
        assert!(!engine.is_initialized());
        assert!(!engine.is_listening());
    }

    #[test]
    fn test_engine_with_model_path() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let engine = VoiceEngine::with_model_path(&model_path);
        assert!(engine.is_ok());

        let engine = engine.unwrap();
        assert_eq!(engine.config().model_path, model_path);
    }

    #[test]
    fn test_engine_subscribe() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        let engine = VoiceEngine::new(config).unwrap();

        // Should be able to subscribe multiple times
        let _rx1 = engine.subscribe();
        let _rx2 = engine.subscribe();
    }

    #[test]
    fn test_engine_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<VoiceEngine>();
        assert_sync::<VoiceEngine>();
    }

    #[test]
    fn test_calculate_audio_level() {
        // Empty samples
        assert_eq!(VoiceEngine::calculate_audio_level(&[]), 0.0);

        // Silence
        assert_eq!(VoiceEngine::calculate_audio_level(&[0, 0, 0, 0]), 0.0);

        // Max amplitude
        let max_samples = vec![i16::MAX; 100];
        let level = VoiceEngine::calculate_audio_level(&max_samples);
        assert!(level > 0.9 && level <= 1.0);

        // Some signal
        let samples: Vec<i16> = vec![1000, -1000, 1000, -1000];
        let level = VoiceEngine::calculate_audio_level(&samples);
        assert!(level > 0.0 && level < 1.0);
    }

    #[test]
    fn test_voice_engine_event_variants() {
        // Test that all event variants can be constructed
        let _e1 = VoiceEngineEvent::StateChanged {
            from: VoiceEngineState::Uninitialized,
            to: VoiceEngineState::Ready,
        };
        let _e2 = VoiceEngineEvent::PartialResult {
            text: "hello".to_string(),
        };
        let _e3 = VoiceEngineEvent::FinalResult {
            text: "hello world".to_string(),
            confidence: Some(0.95),
        };
        let _e4 = VoiceEngineEvent::CommandRecognized {
            command: VoiceCommand::Pause,
            text: "pause".to_string(),
            confidence: Some(0.9),
        };
        let _e5 = VoiceEngineEvent::AudioLevel { level: 0.5 };
        let _e6 = VoiceEngineEvent::Error {
            message: "test error".to_string(),
        };
        let _e7 = VoiceEngineEvent::Initialized;
        let _e8 = VoiceEngineEvent::Started;
        let _e9 = VoiceEngineEvent::Stopped;
    }

    #[test]
    fn test_voice_engine_error_variants() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Test error variant construction
        let _e1 = VoiceEngineError::ModelNotFound(path);
        let _e2 = VoiceEngineError::NotInitialized;
        let _e3 = VoiceEngineError::AlreadyRunning;
        let _e4 = VoiceEngineError::NotRunning;
        let _e5 = VoiceEngineError::WorkerError("test".to_string());
        let _e6 = VoiceEngineError::ConfigError("test".to_string());
    }

    #[test]
    fn test_engine_not_initialized_errors() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        let engine = VoiceEngine::new(config).unwrap();

        // Reset without initialization should fail
        let result = engine.reset();
        assert!(matches!(result, Err(VoiceEngineError::NotInitialized)));
    }

    // ========================================
    // Command Cooldown Tests
    // ========================================

    #[test]
    fn test_cooldown_creation() {
        let cooldown = CommandCooldown::new(1000);
        assert_eq!(cooldown.cooldown_ms(), 1000);
        assert!(cooldown.last_command().is_none());
        assert!(cooldown.time_since_last_command().is_none());
    }

    #[test]
    fn test_cooldown_default() {
        let cooldown = CommandCooldown::default();
        assert_eq!(cooldown.cooldown_ms(), DEFAULT_COMMAND_COOLDOWN_MS);
    }

    #[test]
    fn test_cooldown_allows_first_command() {
        let cooldown = CommandCooldown::new(1000);
        assert!(cooldown.is_allowed(&VoiceCommand::Pause));
        assert!(cooldown.is_allowed(&VoiceCommand::Resume));
        assert!(cooldown.is_allowed(&VoiceCommand::Start));
    }

    #[test]
    fn test_cooldown_blocks_same_command() {
        let mut cooldown = CommandCooldown::new(1000);

        // First command should be allowed
        assert!(cooldown.is_allowed(&VoiceCommand::Pause));
        cooldown.record_command(&VoiceCommand::Pause);

        // Same command immediately after should be blocked
        assert!(!cooldown.is_allowed(&VoiceCommand::Pause));

        // Different command should be allowed
        assert!(cooldown.is_allowed(&VoiceCommand::Resume));
    }

    #[test]
    fn test_cooldown_remaining_time() {
        let mut cooldown = CommandCooldown::new(1000);

        // No remaining time before first command
        assert!(cooldown.remaining_cooldown_ms(&VoiceCommand::Pause).is_none());

        cooldown.record_command(&VoiceCommand::Pause);

        // Should have remaining time for same command
        let remaining = cooldown.remaining_cooldown_ms(&VoiceCommand::Pause);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 1000);
        assert!(remaining.unwrap() > 0);

        // No remaining time for different command
        assert!(cooldown.remaining_cooldown_ms(&VoiceCommand::Resume).is_none());
    }

    #[test]
    fn test_cooldown_unknown_command_always_allowed() {
        let mut cooldown = CommandCooldown::new(1000);

        // Unknown commands should always be allowed
        let unknown = VoiceCommand::Unknown("test".to_string());
        assert!(cooldown.is_allowed(&unknown));

        cooldown.record_command(&unknown);

        // Should still be allowed (unknown commands don't get recorded)
        assert!(cooldown.is_allowed(&unknown));
        assert!(cooldown.remaining_cooldown_ms(&unknown).is_none());
    }

    #[test]
    fn test_cooldown_reset() {
        let mut cooldown = CommandCooldown::new(1000);

        cooldown.record_command(&VoiceCommand::Pause);
        assert!(!cooldown.is_allowed(&VoiceCommand::Pause));

        cooldown.reset();

        // Should be allowed again after reset
        assert!(cooldown.is_allowed(&VoiceCommand::Pause));
        assert!(cooldown.last_command().is_none());
    }

    #[test]
    fn test_cooldown_set_cooldown_ms() {
        let mut cooldown = CommandCooldown::new(1000);
        assert_eq!(cooldown.cooldown_ms(), 1000);

        cooldown.set_cooldown_ms(2000);
        assert_eq!(cooldown.cooldown_ms(), 2000);
    }

    #[test]
    fn test_cooldown_last_command() {
        let mut cooldown = CommandCooldown::new(1000);
        assert!(cooldown.last_command().is_none());

        cooldown.record_command(&VoiceCommand::Pause);
        assert_eq!(cooldown.last_command(), Some(&VoiceCommand::Pause));

        cooldown.record_command(&VoiceCommand::Resume);
        assert_eq!(cooldown.last_command(), Some(&VoiceCommand::Resume));
    }

    #[test]
    fn test_cooldown_time_since_last_command() {
        let mut cooldown = CommandCooldown::new(1000);
        assert!(cooldown.time_since_last_command().is_none());

        cooldown.record_command(&VoiceCommand::Pause);

        let elapsed = cooldown.time_since_last_command();
        assert!(elapsed.is_some());
        // Should be very small since we just recorded it
        assert!(elapsed.unwrap().as_millis() < 100);
    }

    #[test]
    fn test_cooldown_expires_over_time() {
        let mut cooldown = CommandCooldown::new(10); // Very short cooldown for testing

        cooldown.record_command(&VoiceCommand::Pause);
        assert!(!cooldown.is_allowed(&VoiceCommand::Pause));

        // Wait for cooldown to expire
        std::thread::sleep(std::time::Duration::from_millis(15));

        // Should be allowed now
        assert!(cooldown.is_allowed(&VoiceCommand::Pause));
        assert!(cooldown.remaining_cooldown_ms(&VoiceCommand::Pause).is_none());
    }

    // ========================================
    // Config Cooldown/Debounce Tests
    // ========================================

    #[test]
    fn test_config_cooldown_settings() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        assert_eq!(config.command_cooldown_ms, DEFAULT_COMMAND_COOLDOWN_MS);
        assert_eq!(config.debounce_ms, DEFAULT_DEBOUNCE_MS);
        assert!(config.enable_cooldown);
    }

    #[test]
    fn test_config_cooldown_builder() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path)
            .with_command_cooldown(2000)
            .with_debounce(500)
            .with_cooldown_enabled(false);

        assert_eq!(config.command_cooldown_ms, 2000);
        assert_eq!(config.debounce_ms, 500);
        assert!(!config.enable_cooldown);
    }

    #[test]
    fn test_config_for_commands_has_cooldown() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::for_commands(&model_path);
        assert_eq!(config.command_cooldown_ms, DEFAULT_COMMAND_COOLDOWN_MS);
        assert!(config.enable_cooldown);
    }

    #[test]
    fn test_command_cooldown_event_variant() {
        let event = VoiceEngineEvent::CommandCooldown {
            command: VoiceCommand::Pause,
            remaining_ms: 500,
        };

        // Verify the event can be constructed and matches
        if let VoiceEngineEvent::CommandCooldown { command, remaining_ms } = event {
            assert_eq!(command, VoiceCommand::Pause);
            assert_eq!(remaining_ms, 500);
        } else {
            panic!("Expected CommandCooldown event");
        }
    }

    // ========================================
    // Activation Mode Tests
    // ========================================

    #[test]
    fn test_activation_mode_default() {
        assert_eq!(ActivationMode::default(), ActivationMode::WakeWord);
    }

    #[test]
    fn test_activation_mode_display() {
        assert_eq!(ActivationMode::AlwaysListening.to_string(), "Always Listening");
        assert_eq!(ActivationMode::WakeWord.to_string(), "Wake Word");
        assert_eq!(ActivationMode::PushToTalk.to_string(), "Push to Talk");
    }

    #[test]
    fn test_config_activation_mode_default() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        assert_eq!(config.activation_mode, ActivationMode::WakeWord);
        assert!(config.wake_word_enabled);
        assert_eq!(config.wake_word_active_duration_ms, DEFAULT_ACTIVE_LISTENING_DURATION_MS);
    }

    #[test]
    fn test_config_activation_mode_builder() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path)
            .with_activation_mode(ActivationMode::AlwaysListening)
            .with_wake_word_duration(10000)
            .with_wake_word_enabled(false);

        assert_eq!(config.activation_mode, ActivationMode::AlwaysListening);
        assert_eq!(config.wake_word_active_duration_ms, 10000);
        assert!(!config.wake_word_enabled);
    }

    #[test]
    fn test_config_always_listening() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::always_listening(&model_path);
        assert_eq!(config.activation_mode, ActivationMode::AlwaysListening);
        assert!(config.grammar.is_some()); // Should still have grammar
    }

    #[test]
    fn test_config_push_to_talk() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::push_to_talk(&model_path);
        assert_eq!(config.activation_mode, ActivationMode::PushToTalk);
        assert!(config.grammar.is_some());
    }

    #[test]
    fn test_engine_activation_mode_getter() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path)
            .with_activation_mode(ActivationMode::PushToTalk);
        let engine = VoiceEngine::new(config).unwrap();

        assert_eq!(engine.activation_mode(), ActivationMode::PushToTalk);
    }

    #[test]
    fn test_engine_set_activation_mode_before_init() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        let engine = VoiceEngine::new(config).unwrap();

        // Can set mode before initialization
        let result = engine.set_activation_mode(ActivationMode::AlwaysListening);
        assert!(result.is_ok());
        assert_eq!(engine.activation_mode(), ActivationMode::AlwaysListening);
    }

    #[test]
    fn test_wake_word_event_variants() {
        // Test WakeWordDetected
        let _e1 = VoiceEngineEvent::WakeWordDetected {
            phrase: "hey rust ride".to_string(),
            duration_ms: 5000,
        };

        // Test WakeWordTimeout
        let _e2 = VoiceEngineEvent::WakeWordTimeout;

        // Test WakeWordExtended
        let _e3 = VoiceEngineEvent::WakeWordExtended {
            remaining_ms: 4000,
        };

        // Test ActivationModeChanged
        let _e4 = VoiceEngineEvent::ActivationModeChanged {
            mode: ActivationMode::WakeWord,
        };
    }

    #[test]
    fn test_wake_word_detected_event() {
        let event = VoiceEngineEvent::WakeWordDetected {
            phrase: "ok ride".to_string(),
            duration_ms: 5000,
        };

        if let VoiceEngineEvent::WakeWordDetected { phrase, duration_ms } = event {
            assert_eq!(phrase, "ok ride");
            assert_eq!(duration_ms, 5000);
        } else {
            panic!("Expected WakeWordDetected event");
        }
    }

    #[test]
    fn test_activation_mode_changed_event() {
        let event = VoiceEngineEvent::ActivationModeChanged {
            mode: ActivationMode::PushToTalk,
        };

        if let VoiceEngineEvent::ActivationModeChanged { mode } = event {
            assert_eq!(mode, ActivationMode::PushToTalk);
        } else {
            panic!("Expected ActivationModeChanged event");
        }
    }

    // ========================================
    // VoiceActivation <-> ActivationMode Conversion Tests
    // ========================================

    #[test]
    fn test_voice_activation_to_activation_mode_always_on() {
        let activation = VoiceActivation::AlwaysOn;
        let mode: ActivationMode = activation.into();
        assert_eq!(mode, ActivationMode::AlwaysListening);
    }

    #[test]
    fn test_voice_activation_to_activation_mode_wake_word() {
        let activation = VoiceActivation::WakeWord;
        let mode: ActivationMode = activation.into();
        assert_eq!(mode, ActivationMode::WakeWord);
    }

    #[test]
    fn test_voice_activation_to_activation_mode_push_to_talk() {
        let activation = VoiceActivation::PushToTalk;
        let mode: ActivationMode = activation.into();
        assert_eq!(mode, ActivationMode::PushToTalk);
    }

    #[test]
    fn test_voice_activation_to_activation_mode_off() {
        // Off maps to WakeWord (default) since engine doesn't have an "off" mode
        let activation = VoiceActivation::Off;
        let mode: ActivationMode = activation.into();
        assert_eq!(mode, ActivationMode::WakeWord);
    }

    #[test]
    fn test_activation_mode_to_voice_activation_always_listening() {
        let mode = ActivationMode::AlwaysListening;
        let activation: VoiceActivation = mode.into();
        assert_eq!(activation, VoiceActivation::AlwaysOn);
    }

    #[test]
    fn test_activation_mode_to_voice_activation_wake_word() {
        let mode = ActivationMode::WakeWord;
        let activation: VoiceActivation = mode.into();
        assert_eq!(activation, VoiceActivation::WakeWord);
    }

    #[test]
    fn test_activation_mode_to_voice_activation_push_to_talk() {
        let mode = ActivationMode::PushToTalk;
        let activation: VoiceActivation = mode.into();
        assert_eq!(activation, VoiceActivation::PushToTalk);
    }

    #[test]
    fn test_roundtrip_voice_activation_to_mode_and_back() {
        // AlwaysOn
        let activation = VoiceActivation::AlwaysOn;
        let mode: ActivationMode = activation.into();
        let back: VoiceActivation = mode.into();
        assert_eq!(back, VoiceActivation::AlwaysOn);

        // WakeWord
        let activation = VoiceActivation::WakeWord;
        let mode: ActivationMode = activation.into();
        let back: VoiceActivation = mode.into();
        assert_eq!(back, VoiceActivation::WakeWord);

        // PushToTalk
        let activation = VoiceActivation::PushToTalk;
        let mode: ActivationMode = activation.into();
        let back: VoiceActivation = mode.into();
        assert_eq!(back, VoiceActivation::PushToTalk);
    }

    #[test]
    fn test_engine_apply_voice_activation() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path);
        let engine = VoiceEngine::new(config).unwrap();

        // Apply voice activation from config
        let result = engine.apply_voice_activation(VoiceActivation::PushToTalk);
        assert!(result.is_ok());
        assert_eq!(engine.activation_mode(), ActivationMode::PushToTalk);

        // Apply another mode
        let result = engine.apply_voice_activation(VoiceActivation::AlwaysOn);
        assert!(result.is_ok());
        assert_eq!(engine.activation_mode(), ActivationMode::AlwaysListening);
    }

    #[test]
    fn test_engine_voice_activation_getter() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngineConfig::new(&model_path)
            .with_activation_mode(ActivationMode::PushToTalk);
        let engine = VoiceEngine::new(config).unwrap();

        // Get voice activation (config-level enum)
        assert_eq!(engine.voice_activation(), VoiceActivation::PushToTalk);
    }

    #[test]
    fn test_engine_config_from_settings() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        let config = VoiceEngine::config_from_settings(&model_path, VoiceActivation::AlwaysOn);
        assert_eq!(config.activation_mode, ActivationMode::AlwaysListening);
        assert!(config.grammar.is_some()); // Should have command grammar

        let config = VoiceEngine::config_from_settings(&model_path, VoiceActivation::PushToTalk);
        assert_eq!(config.activation_mode, ActivationMode::PushToTalk);
    }

    // Note: Tests that require actual audio hardware and Vosk model are in tests/voice_integration.rs
    // They should be marked as #[ignore] since they require the model to be downloaded
}
