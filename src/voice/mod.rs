//! Voice control module with Vosk speech recognition.
//!
//! This module provides offline voice control functionality using Vosk speech
//! recognition for hands-free workout control.
//!
//! ## Features
//!
//! - **Model Management**: Automatic download and installation of Vosk models
//! - **Speech Recognition**: Local speech-to-text using Vosk
//! - **Command Parsing**: Convert recognized speech to workout commands
//! - **Audio Capture**: Cross-platform microphone input using cpal
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    VoiceEngine                               │
//! │  Orchestrates audio capture -> recognition -> commands      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  AudioInputCapture  |  Recognizer  |  CommandParser         │
//! │  (cpal microphone)  |  (Vosk)      |  (phrase->command)     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    VoskModelManager                          │
//! │  Model download, extraction, and lifecycle management       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::{VoskModelManager, ModelState};
//!
//! // Check if model is available
//! let manager = VoskModelManager::new();
//! match manager.state() {
//!     ModelState::Ready => {
//!         // Model is ready for use
//!     }
//!     ModelState::NotInstalled => {
//!         // Need to download model
//!         manager.download_model().await?;
//!     }
//!     _ => {}
//! }
//! ```
//!
//! ## Module Organization
//!
//! - [`model_manager`] - Vosk model download, installation, and state management
//!
//! ## Configuration
//!
//! Voice control settings are stored in `AccessibilitySettings` in `storage/config.rs`:
//! - `voice_control_enabled` - Master enable/disable
//! - `voice_activation` - Activation mode (AlwaysOn, PushToTalk, Off)
//!
//! ## Model Storage
//!
//! The Vosk model is stored in the application data directory:
//! - Windows: `%APPDATA%\RustRide\vosk-model`
//! - macOS: `~/Library/Application Support/RustRide/vosk-model`
//! - Linux: `~/.local/share/RustRide/vosk-model`

pub mod model_manager;
pub mod download;
pub mod audio_input;
pub mod recognizer;
pub mod engine;
pub mod command_parser;
pub mod wake_word;
pub mod push_to_talk;
pub mod feedback;
pub mod executor;

// Re-export main types
pub use model_manager::{
    ModelState, VoskModelManager, VoskModelError,
    ModelLifecycleEvent, ModelLifecycleStateMachine, PartialDownloadInfo,
    LifecycleEventCallback,
};
pub use download::{DownloadError, DownloadProgress, ModelDownloader, format_bytes};
pub use audio_input::{
    AudioInputCapture, AudioInputConfig, AudioInputError,
    AudioInputDeviceInfo, AudioRingBuffer, CaptureState,
    VOSK_SAMPLE_RATE, VOSK_CHANNELS, DEFAULT_BUFFER_SIZE,
};
pub use recognizer::{
    ThreadSafeRecognizer, RecognizerConfig, RecognizerError, RecognitionResult,
    RECOGNIZER_SAMPLE_RATE, DEFAULT_RECOGNITION_TIMEOUT_MS,
};
pub use engine::{
    VoiceEngine, VoiceEngineConfig, VoiceEngineError, VoiceEngineEvent, VoiceEngineState,
    CommandCooldown, ActivationMode,
};
// Re-export VoiceActivation from config for convenience
pub use crate::storage::config::VoiceActivation;
pub use command_parser::{
    CommandParser, ParseResult, levenshtein_distance, string_similarity,
    DEFAULT_MIN_CONFIDENCE,
};
pub use wake_word::{
    WakeWordDetector, WakeWordConfig, WakeWordEvent, WakeWordState,
    DEFAULT_ACTIVE_LISTENING_DURATION_MS, WAKE_PHRASES, wake_word_grammar,
};
pub use push_to_talk::{
    PushToTalkHandler, PushToTalkConfig, PushToTalkEvent, PushToTalkKey, PushToTalkState,
    DEFAULT_PUSH_TO_TALK_KEY, DEFAULT_MIN_HOLD_DURATION_MS, DEFAULT_MAX_HOLD_DURATION_MS,
};
pub use feedback::{
    VoiceFeedback, VoiceFeedbackConfig, VoiceFeedbackEvent, VoiceFeedbackError,
};
pub use executor::{
    VoiceCommandExecutor, VoiceExecutorError, ExecutorContext, MappingResult,
};
