//! Thread-safe Vosk speech recognizer wrapper.
//!
//! This module provides a thread-safe wrapper around the Vosk speech recognizer,
//! which is not `Send+Sync`. Similar to the TTS provider pattern, we run the
//! recognizer on a dedicated worker thread and communicate via channels.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │              ThreadSafeRecognizer                    │
//! │  (Send + Sync wrapper for external use)             │
//! ├─────────────────────────────────────────────────────┤
//! │  command_tx ──────────────────────┐                 │
//! │                                   ▼                 │
//! │                          ┌────────────────┐         │
//! │                          │ Worker Thread  │         │
//! │                          │                │         │
//! │                          │ vosk::Model    │         │
//! │                          │ vosk::Recognizer│        │
//! │                          └────────────────┘         │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::recognizer::ThreadSafeRecognizer;
//!
//! // Create recognizer with model path
//! let recognizer = ThreadSafeRecognizer::new("/path/to/vosk-model")?;
//!
//! // Initialize (loads the model)
//! recognizer.initialize()?;
//!
//! // Feed audio samples
//! let samples = vec![0i16; 1600]; // 100ms at 16kHz
//! recognizer.accept_waveform(&samples)?;
//!
//! // Get results
//! let partial = recognizer.partial_result()?;
//! let final_result = recognizer.final_result()?;
//! ```

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;

/// Default sample rate for Vosk (16kHz)
pub const RECOGNIZER_SAMPLE_RATE: f32 = 16000.0;

/// Default recognition timeout in milliseconds
pub const DEFAULT_RECOGNITION_TIMEOUT_MS: u64 = 5000;

/// Errors that can occur during recognition operations.
#[derive(Debug, Error)]
pub enum RecognizerError {
    /// Failed to load the Vosk model.
    #[error("Failed to load Vosk model from {path}: {message}")]
    ModelLoadFailed { path: PathBuf, message: String },

    /// Failed to create the recognizer.
    #[error("Failed to create recognizer: {0}")]
    RecognizerCreationFailed(String),

    /// Recognizer is not initialized.
    #[error("Recognizer not initialized")]
    NotInitialized,

    /// Worker thread is not responding.
    #[error("Recognizer worker thread not responding")]
    WorkerNotResponding,

    /// Recognition operation timed out.
    #[error("Recognition timed out after {0}ms")]
    Timeout(u64),

    /// Invalid audio data.
    #[error("Invalid audio data: {0}")]
    InvalidAudioData(String),

    /// Model path does not exist.
    #[error("Model path does not exist: {0}")]
    ModelNotFound(PathBuf),

    /// Failed to spawn worker thread.
    #[error("Failed to spawn worker thread: {0}")]
    ThreadSpawnFailed(String),

    /// Internal error during recognition.
    #[error("Recognition error: {0}")]
    RecognitionError(String),
}

/// Result of speech recognition.
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// The recognized text.
    pub text: String,
    /// Whether this is a final result (vs partial).
    pub is_final: bool,
    /// Confidence score (0.0 - 1.0) if available.
    pub confidence: Option<f32>,
}

impl RecognitionResult {
    /// Create a new recognition result.
    pub fn new(text: impl Into<String>, is_final: bool) -> Self {
        Self {
            text: text.into(),
            is_final,
            confidence: None,
        }
    }

    /// Create a result with confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Check if the result is empty.
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Configuration for the speech recognizer.
#[derive(Debug, Clone)]
pub struct RecognizerConfig {
    /// Path to the Vosk model directory.
    pub model_path: PathBuf,
    /// Sample rate for audio (default: 16000 Hz).
    pub sample_rate: f32,
    /// Grammar for constrained recognition (command mode).
    /// If set, only phrases matching the grammar will be recognized.
    pub grammar: Option<Vec<String>>,
    /// Timeout for recognition operations in milliseconds.
    pub timeout_ms: u64,
    /// Maximum number of alternatives to return.
    pub max_alternatives: u32,
    /// Whether to enable partial results.
    pub enable_partial_results: bool,
    /// Whether to enable word timestamps in results.
    pub enable_words: bool,
}

impl RecognizerConfig {
    /// Create a new configuration with the specified model path.
    pub fn new(model_path: impl AsRef<Path>) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
            sample_rate: RECOGNIZER_SAMPLE_RATE,
            grammar: None,
            timeout_ms: DEFAULT_RECOGNITION_TIMEOUT_MS,
            max_alternatives: 1,
            enable_partial_results: true,
            enable_words: false,
        }
    }

    /// Create a configuration optimized for voice commands.
    ///
    /// This sets up a grammar for common workout commands, enabling
    /// constrained recognition for better accuracy.
    pub fn for_commands(model_path: impl AsRef<Path>) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
            sample_rate: RECOGNIZER_SAMPLE_RATE,
            grammar: Some(Self::default_command_grammar()),
            timeout_ms: DEFAULT_RECOGNITION_TIMEOUT_MS,
            max_alternatives: 3,
            enable_partial_results: true,
            enable_words: false,
        }
    }

    /// Get the default grammar for workout voice commands.
    fn default_command_grammar() -> Vec<String> {
        vec![
            // Basic commands
            "pause".to_string(),
            "resume".to_string(),
            "start".to_string(),
            "stop".to_string(),
            "end".to_string(),
            "end workout".to_string(),
            // Interval control
            "skip".to_string(),
            "skip interval".to_string(),
            "next".to_string(),
            "next interval".to_string(),
            // Lap marking
            "lap".to_string(),
            "take lap".to_string(),
            "mark lap".to_string(),
            // Intensity adjustments
            "increase".to_string(),
            "decrease".to_string(),
            "harder".to_string(),
            "easier".to_string(),
            // Status
            "status".to_string(),
            "what's my status".to_string(),
            // Wake words
            "hey rust ride".to_string(),
            "ok ride".to_string(),
            // Silence / unknown
            "[unk]".to_string(),
        ]
    }

    /// Set a custom grammar for constrained recognition.
    pub fn with_grammar(mut self, grammar: Vec<String>) -> Self {
        self.grammar = Some(grammar);
        self
    }

    /// Clear the grammar to allow free-form recognition.
    pub fn without_grammar(mut self) -> Self {
        self.grammar = None;
        self
    }

    /// Set the sample rate.
    pub fn with_sample_rate(mut self, sample_rate: f32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// Set the recognition timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the maximum number of alternatives to return.
    pub fn with_max_alternatives(mut self, max_alternatives: u32) -> Self {
        self.max_alternatives = max_alternatives;
        self
    }

    /// Enable or disable partial results.
    pub fn with_partial_results(mut self, enable: bool) -> Self {
        self.enable_partial_results = enable;
        self
    }
}

/// Commands sent to the recognizer worker thread.
#[derive(Debug)]
enum RecognizerCommand {
    /// Initialize the recognizer with the model.
    Initialize {
        response_tx: Sender<Result<(), RecognizerError>>,
    },
    /// Accept audio waveform data.
    AcceptWaveform {
        samples: Vec<i16>,
        response_tx: Sender<Result<bool, RecognizerError>>,
    },
    /// Get the partial recognition result.
    PartialResult {
        response_tx: Sender<Result<RecognitionResult, RecognizerError>>,
    },
    /// Get the final recognition result and reset.
    FinalResult {
        response_tx: Sender<Result<RecognitionResult, RecognizerError>>,
    },
    /// Reset the recognizer state.
    Reset {
        response_tx: Sender<Result<(), RecognizerError>>,
    },
    /// Update the grammar.
    SetGrammar {
        grammar: Option<Vec<String>>,
        response_tx: Sender<Result<(), RecognizerError>>,
    },
    /// Shutdown the worker thread.
    Shutdown,
}

/// Thread-safe wrapper for the Vosk speech recognizer.
///
/// This struct provides a thread-safe interface to the Vosk recognizer,
/// which is not `Send+Sync`. All operations are delegated to a dedicated
/// worker thread via channels.
pub struct ThreadSafeRecognizer {
    /// Channel to send commands to the worker thread.
    command_tx: Mutex<Option<Sender<RecognizerCommand>>>,
    /// Handle to the worker thread.
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    /// Configuration.
    config: RecognizerConfig,
    /// Whether the recognizer has been initialized.
    initialized: AtomicBool,
    /// Whether the recognizer is currently processing.
    is_processing: Arc<AtomicBool>,
}

impl ThreadSafeRecognizer {
    /// Create a new thread-safe recognizer with the specified model path.
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self, RecognizerError> {
        let path = model_path.as_ref();
        if !path.exists() {
            return Err(RecognizerError::ModelNotFound(path.to_path_buf()));
        }

        let config = RecognizerConfig::new(path);
        Ok(Self::with_config(config))
    }

    /// Create a new thread-safe recognizer configured for command recognition.
    pub fn for_commands(model_path: impl AsRef<Path>) -> Result<Self, RecognizerError> {
        let path = model_path.as_ref();
        if !path.exists() {
            return Err(RecognizerError::ModelNotFound(path.to_path_buf()));
        }

        let config = RecognizerConfig::for_commands(path);
        Ok(Self::with_config(config))
    }

    /// Create a new thread-safe recognizer with custom configuration.
    pub fn with_config(config: RecognizerConfig) -> Self {
        Self {
            command_tx: Mutex::new(None),
            worker_handle: Mutex::new(None),
            config,
            initialized: AtomicBool::new(false),
            is_processing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &RecognizerConfig {
        &self.config
    }

    /// Check if the recognizer is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Check if the recognizer is currently processing audio.
    pub fn is_processing(&self) -> bool {
        self.is_processing.load(Ordering::Acquire)
    }

    /// Start the worker thread.
    fn start_worker(&self) -> Result<(), RecognizerError> {
        let (command_tx, command_rx) = mpsc::channel::<RecognizerCommand>();
        let config = self.config.clone();
        let is_processing = Arc::clone(&self.is_processing);

        let handle = thread::Builder::new()
            .name("vosk-recognizer".to_string())
            .spawn(move || {
                Self::worker_thread(command_rx, config, is_processing);
            })
            .map_err(|e| RecognizerError::ThreadSpawnFailed(e.to_string()))?;

        *self.command_tx.lock().unwrap() = Some(command_tx);
        *self.worker_handle.lock().unwrap() = Some(handle);

        Ok(())
    }

    /// The worker thread function.
    fn worker_thread(
        command_rx: Receiver<RecognizerCommand>,
        config: RecognizerConfig,
        is_processing: Arc<AtomicBool>,
    ) {
        tracing::debug!("Vosk recognizer worker thread started");

        let mut model: Option<vosk::Model> = None;
        let mut recognizer: Option<vosk::Recognizer> = None;

        while let Ok(command) = command_rx.recv() {
            match command {
                RecognizerCommand::Initialize { response_tx } => {
                    let result = Self::handle_initialize(&config, &mut model, &mut recognizer);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::AcceptWaveform {
                    samples,
                    response_tx,
                } => {
                    is_processing.store(true, Ordering::Release);
                    let result = Self::handle_accept_waveform(&mut recognizer, &samples);
                    is_processing.store(false, Ordering::Release);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::PartialResult { response_tx } => {
                    let result = Self::handle_partial_result(&mut recognizer);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::FinalResult { response_tx } => {
                    let result = Self::handle_final_result(&mut recognizer);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::Reset { response_tx } => {
                    let result = Self::handle_reset(&mut recognizer);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::SetGrammar {
                    grammar,
                    response_tx,
                } => {
                    // Recreating recognizer with new grammar requires model reference
                    let result = Self::handle_set_grammar(&config, &model, &mut recognizer, grammar);
                    let _ = response_tx.send(result);
                }
                RecognizerCommand::Shutdown => {
                    tracing::debug!("Vosk recognizer worker: Shutting down");
                    break;
                }
            }
        }

        tracing::debug!("Vosk recognizer worker thread exiting");
    }

    /// Handle the Initialize command.
    fn handle_initialize(
        config: &RecognizerConfig,
        model: &mut Option<vosk::Model>,
        recognizer: &mut Option<vosk::Recognizer>,
    ) -> Result<(), RecognizerError> {
        if model.is_some() {
            tracing::info!("Vosk model already loaded");
            return Ok(());
        }

        tracing::info!("Loading Vosk model from: {:?}", config.model_path);

        // Load the model
        let model_path_str = config
            .model_path
            .to_str()
            .ok_or_else(|| RecognizerError::ModelLoadFailed {
                path: config.model_path.clone(),
                message: "Invalid UTF-8 in path".to_string(),
            })?;

        let loaded_model = vosk::Model::new(model_path_str).ok_or_else(|| {
            RecognizerError::ModelLoadFailed {
                path: config.model_path.clone(),
                message: "Failed to load Vosk model".to_string(),
            }
        })?;

        // Create the recognizer with or without grammar
        let rec = if let Some(grammar) = &config.grammar {
            let grammar_json = serde_json::to_string(grammar).map_err(|e| {
                RecognizerError::RecognizerCreationFailed(format!(
                    "Failed to serialize grammar: {}",
                    e
                ))
            })?;

            vosk::Recognizer::new_with_grammar(&loaded_model, config.sample_rate, &grammar_json)
                .ok_or_else(|| {
                    RecognizerError::RecognizerCreationFailed(
                        "Failed to create recognizer with grammar".to_string(),
                    )
                })?
        } else {
            vosk::Recognizer::new(&loaded_model, config.sample_rate).ok_or_else(|| {
                RecognizerError::RecognizerCreationFailed(
                    "Failed to create recognizer".to_string(),
                )
            })?
        };

        *model = Some(loaded_model);
        *recognizer = Some(rec);

        tracing::info!("Vosk recognizer initialized successfully");
        Ok(())
    }

    /// Handle the AcceptWaveform command.
    fn handle_accept_waveform(
        recognizer: &mut Option<vosk::Recognizer>,
        samples: &[i16],
    ) -> Result<bool, RecognizerError> {
        let rec = recognizer
            .as_mut()
            .ok_or(RecognizerError::NotInitialized)?;

        // Vosk's accept_waveform returns true when it has a complete result ready
        let completed = rec.accept_waveform(samples);

        Ok(completed == vosk::DecodingState::Finalized)
    }

    /// Handle the PartialResult command.
    fn handle_partial_result(
        recognizer: &mut Option<vosk::Recognizer>,
    ) -> Result<RecognitionResult, RecognizerError> {
        let rec = recognizer
            .as_mut()
            .ok_or(RecognizerError::NotInitialized)?;

        let result_json = rec.partial_result();
        Self::parse_result_json(result_json.partial, false)
    }

    /// Handle the FinalResult command.
    fn handle_final_result(
        recognizer: &mut Option<vosk::Recognizer>,
    ) -> Result<RecognitionResult, RecognizerError> {
        let rec = recognizer
            .as_mut()
            .ok_or(RecognizerError::NotInitialized)?;

        let result = rec.final_result();

        // Handle either single or multiple results
        match result {
            vosk::CompleteResult::Single(single) => {
                Self::parse_result_json(single.text, true)
            }
            vosk::CompleteResult::Multiple(multi) => {
                // Get the best result (first alternative)
                if let Some(alt) = multi.alternatives.first() {
                    let mut result = RecognitionResult::new(&alt.text, true);
                    result.confidence = Some(alt.confidence);
                    Ok(result)
                } else {
                    Ok(RecognitionResult::new("", true))
                }
            }
        }
    }

    /// Handle the Reset command.
    fn handle_reset(recognizer: &mut Option<vosk::Recognizer>) -> Result<(), RecognizerError> {
        if let Some(rec) = recognizer.as_mut() {
            rec.reset();
            Ok(())
        } else {
            Err(RecognizerError::NotInitialized)
        }
    }

    /// Handle the SetGrammar command.
    fn handle_set_grammar(
        config: &RecognizerConfig,
        model: &Option<vosk::Model>,
        recognizer: &mut Option<vosk::Recognizer>,
        grammar: Option<Vec<String>>,
    ) -> Result<(), RecognizerError> {
        let mdl = model.as_ref().ok_or(RecognizerError::NotInitialized)?;

        let new_rec = if let Some(ref grammar) = grammar {
            let grammar_json = serde_json::to_string(grammar).map_err(|e| {
                RecognizerError::RecognizerCreationFailed(format!(
                    "Failed to serialize grammar: {}",
                    e
                ))
            })?;

            vosk::Recognizer::new_with_grammar(mdl, config.sample_rate, &grammar_json)
                .ok_or_else(|| {
                    RecognizerError::RecognizerCreationFailed(
                        "Failed to create recognizer with grammar".to_string(),
                    )
                })?
        } else {
            vosk::Recognizer::new(mdl, config.sample_rate).ok_or_else(|| {
                RecognizerError::RecognizerCreationFailed(
                    "Failed to create recognizer".to_string(),
                )
            })?
        };

        *recognizer = Some(new_rec);
        tracing::info!("Recognizer grammar updated");
        Ok(())
    }

    /// Parse Vosk result JSON into RecognitionResult.
    fn parse_result_json(text: &str, is_final: bool) -> Result<RecognitionResult, RecognizerError> {
        Ok(RecognitionResult::new(text.trim(), is_final))
    }

    /// Send a command to the worker thread.
    fn send_command(&self, command: RecognizerCommand) -> Result<(), RecognizerError> {
        let guard = self.command_tx.lock().unwrap();
        if let Some(ref tx) = *guard {
            tx.send(command)
                .map_err(|_| RecognizerError::WorkerNotResponding)
        } else {
            Err(RecognizerError::WorkerNotResponding)
        }
    }

    /// Ensure the worker thread is started and initialized.
    fn ensure_initialized(&self) -> Result<(), RecognizerError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        // Start worker thread if not already running
        {
            let guard = self.command_tx.lock().unwrap();
            if guard.is_none() {
                drop(guard);
                self.start_worker()?;
            }
        }

        // Send initialize command
        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::Initialize { response_tx })?;

        // Wait for initialization to complete
        let result = response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms * 2))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms * 2))??;

        self.initialized.store(true, Ordering::Release);
        Ok(result)
    }

    /// Initialize the recognizer (load model and create recognizer).
    ///
    /// This must be called before any recognition operations.
    pub fn initialize(&self) -> Result<(), RecognizerError> {
        self.ensure_initialized()
    }

    /// Accept audio waveform data for recognition.
    ///
    /// Returns `true` if Vosk has a complete result ready, `false` if more
    /// audio is needed.
    ///
    /// # Arguments
    ///
    /// * `samples` - 16-bit PCM audio samples at the configured sample rate
    pub fn accept_waveform(&self, samples: &[i16]) -> Result<bool, RecognizerError> {
        self.ensure_initialized()?;

        if samples.is_empty() {
            return Ok(false);
        }

        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::AcceptWaveform {
            samples: samples.to_vec(),
            response_tx,
        })?;

        response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms))?
    }

    /// Get the partial recognition result.
    ///
    /// Call this during active recognition to get intermediate results.
    pub fn partial_result(&self) -> Result<RecognitionResult, RecognizerError> {
        self.ensure_initialized()?;

        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::PartialResult { response_tx })?;

        response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms))?
    }

    /// Get the final recognition result and reset for next utterance.
    ///
    /// Call this when you want to finalize the current recognition and
    /// start fresh for the next utterance.
    pub fn final_result(&self) -> Result<RecognitionResult, RecognizerError> {
        self.ensure_initialized()?;

        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::FinalResult { response_tx })?;

        response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms))?
    }

    /// Reset the recognizer state without getting results.
    ///
    /// Use this to discard any pending audio and start fresh.
    pub fn reset(&self) -> Result<(), RecognizerError> {
        self.ensure_initialized()?;

        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::Reset { response_tx })?;

        response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms))?
    }

    /// Update the grammar for constrained recognition.
    ///
    /// This recreates the recognizer with the new grammar. Pass `None` to
    /// switch to free-form recognition.
    pub fn set_grammar(&self, grammar: Option<Vec<String>>) -> Result<(), RecognizerError> {
        self.ensure_initialized()?;

        let (response_tx, response_rx) = mpsc::channel();
        self.send_command(RecognizerCommand::SetGrammar {
            grammar,
            response_tx,
        })?;

        response_rx
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| RecognizerError::Timeout(self.config.timeout_ms))?
    }
}

impl Drop for ThreadSafeRecognizer {
    fn drop(&mut self) {
        // Send shutdown command to worker thread
        if let Some(tx) = self.command_tx.lock().unwrap().take() {
            let _ = tx.send(RecognizerCommand::Shutdown);
        }
        // Wait for worker thread to finish
        if let Some(handle) = self.worker_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

// Verify ThreadSafeRecognizer is Send + Sync
fn _assert_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<ThreadSafeRecognizer>();
    assert_sync::<ThreadSafeRecognizer>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_recognition_result_creation() {
        let result = RecognitionResult::new("hello world", false);
        assert_eq!(result.text, "hello world");
        assert!(!result.is_final);
        assert!(result.confidence.is_none());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_recognition_result_with_confidence() {
        let result = RecognitionResult::new("test", true).with_confidence(0.95);
        assert!(result.is_final);
        assert_eq!(result.confidence, Some(0.95));
    }

    #[test]
    fn test_recognition_result_confidence_clamping() {
        let result = RecognitionResult::new("test", true).with_confidence(1.5);
        assert_eq!(result.confidence, Some(1.0));

        let result = RecognitionResult::new("test", true).with_confidence(-0.5);
        assert_eq!(result.confidence, Some(0.0));
    }

    #[test]
    fn test_recognition_result_is_empty() {
        let result = RecognitionResult::new("", false);
        assert!(result.is_empty());

        let result = RecognitionResult::new("   ", false);
        assert!(result.is_empty());

        let result = RecognitionResult::new("hello", false);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_config_creation() {
        let config = RecognizerConfig::new("/path/to/model");
        assert_eq!(config.model_path, PathBuf::from("/path/to/model"));
        assert_eq!(config.sample_rate, RECOGNIZER_SAMPLE_RATE);
        assert!(config.grammar.is_none());
        assert_eq!(config.timeout_ms, DEFAULT_RECOGNITION_TIMEOUT_MS);
    }

    #[test]
    fn test_config_for_commands() {
        let config = RecognizerConfig::for_commands("/path/to/model");
        assert!(config.grammar.is_some());
        let grammar = config.grammar.unwrap();
        assert!(grammar.contains(&"pause".to_string()));
        assert!(grammar.contains(&"resume".to_string()));
        assert!(grammar.contains(&"skip".to_string()));
        assert!(grammar.contains(&"take lap".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let config = RecognizerConfig::new("/path/to/model")
            .with_sample_rate(8000.0)
            .with_timeout(10000)
            .with_max_alternatives(5)
            .with_partial_results(false)
            .with_grammar(vec!["hello".to_string(), "world".to_string()]);

        assert_eq!(config.sample_rate, 8000.0);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.max_alternatives, 5);
        assert!(!config.enable_partial_results);
        assert_eq!(
            config.grammar,
            Some(vec!["hello".to_string(), "world".to_string()])
        );
    }

    #[test]
    fn test_config_without_grammar() {
        let config = RecognizerConfig::for_commands("/path/to/model").without_grammar();
        assert!(config.grammar.is_none());
    }

    #[test]
    fn test_recognizer_model_not_found() {
        let result = ThreadSafeRecognizer::new("/nonexistent/path");
        assert!(matches!(result, Err(RecognizerError::ModelNotFound(_))));
    }

    #[test]
    fn test_recognizer_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model");
        std::fs::create_dir_all(&model_path).unwrap();

        // Create a fake model file to pass the path check
        // Note: actual initialization will fail without a real Vosk model
        let config = RecognizerConfig::new(&model_path);
        let recognizer = ThreadSafeRecognizer::with_config(config);

        assert!(!recognizer.is_initialized());
        assert!(!recognizer.is_processing());
        assert_eq!(recognizer.config().model_path, model_path);
    }

    #[test]
    fn test_recognizer_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ThreadSafeRecognizer>();
        assert_sync::<ThreadSafeRecognizer>();
    }

    #[test]
    fn test_parse_result_json() {
        let result = ThreadSafeRecognizer::parse_result_json("hello world", false).unwrap();
        assert_eq!(result.text, "hello world");
        assert!(!result.is_final);

        let result = ThreadSafeRecognizer::parse_result_json("  trimmed  ", true).unwrap();
        assert_eq!(result.text, "trimmed");
        assert!(result.is_final);
    }

    #[test]
    fn test_default_command_grammar_completeness() {
        let grammar = RecognizerConfig::default_command_grammar();

        // Check essential commands are present
        assert!(grammar.iter().any(|s| s == "pause"));
        assert!(grammar.iter().any(|s| s == "resume"));
        assert!(grammar.iter().any(|s| s == "start"));
        assert!(grammar.iter().any(|s| s == "stop"));
        assert!(grammar.iter().any(|s| s == "end workout"));
        assert!(grammar.iter().any(|s| s == "skip"));
        assert!(grammar.iter().any(|s| s == "next"));
        assert!(grammar.iter().any(|s| s == "lap"));
        assert!(grammar.iter().any(|s| s == "take lap"));
        assert!(grammar.iter().any(|s| s == "increase"));
        assert!(grammar.iter().any(|s| s == "decrease"));
        assert!(grammar.iter().any(|s| s == "status"));

        // Check wake words
        assert!(grammar.iter().any(|s| s == "hey rust ride"));
        assert!(grammar.iter().any(|s| s == "ok ride"));

        // Check unknown token
        assert!(grammar.iter().any(|s| s == "[unk]"));
    }

    // Note: Integration tests with actual Vosk model would be in tests/voice_integration.rs
    // These would be marked as #[ignore] since they require the model to be downloaded
}
