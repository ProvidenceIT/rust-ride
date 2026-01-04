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

use super::audio_input::{AudioInputCapture, AudioInputConfig, AudioInputError};
use super::recognizer::{
    RecognizerConfig, RecognizerError, ThreadSafeRecognizer,
    RECOGNIZER_SAMPLE_RATE,
};

/// Default audio buffer read interval in milliseconds.
const DEFAULT_AUDIO_READ_INTERVAL_MS: u64 = 100;

/// Default number of samples to read per interval (100ms at 16kHz).
const DEFAULT_SAMPLES_PER_READ: usize = 1600;

/// Minimum confidence threshold for accepting a command (0.0 - 1.0).
const DEFAULT_MIN_CONFIDENCE: f32 = 0.5;

/// Timeout for silence before finalizing recognition (in milliseconds).
const DEFAULT_SILENCE_TIMEOUT_MS: u64 = 1500;

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
                        current_config = new_config;
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
                if let (Some(ref capture), Some(ref rec)) = (&audio_capture, &recognizer) {
                    // Read audio samples from buffer
                    let samples = capture.read_samples(current_config.samples_per_read);

                    if !samples.is_empty() {
                        // Calculate audio level if enabled
                        if current_config.emit_audio_levels {
                            let level = Self::calculate_audio_level(&samples);
                            emit(VoiceEngineEvent::AudioLevel { level });
                        }

                        // Feed samples to recognizer
                        match rec.accept_waveform(&samples) {
                            Ok(has_final) => {
                                if has_final {
                                    // Get final result
                                    Self::process_final_result(
                                        rec,
                                        &current_config,
                                        &event_tx,
                                        &mut last_partial_text,
                                    );
                                    last_speech_time = Some(Instant::now());
                                } else if current_config.emit_partial_results {
                                    // Get partial result
                                    Self::process_partial_result(
                                        rec,
                                        &event_tx,
                                        &mut last_partial_text,
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
                        let level = Self::calculate_audio_level(&samples);
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
                            // Force final result after silence
                            if !last_partial_text.is_empty() {
                                Self::process_final_result(
                                    rec,
                                    &current_config,
                                    &event_tx,
                                    &mut last_partial_text,
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
    fn process_final_result(
        recognizer: &ThreadSafeRecognizer,
        config: &VoiceEngineConfig,
        event_tx: &broadcast::Sender<VoiceEngineEvent>,
        last_partial_text: &mut String,
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

    // Note: Tests that require actual audio hardware and Vosk model are in tests/voice_integration.rs
    // They should be marked as #[ignore] since they require the model to be downloaded
}
