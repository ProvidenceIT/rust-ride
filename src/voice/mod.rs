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

// Re-export main types
pub use model_manager::{
    ModelState, VoskModelManager, VoskModelError,
    ModelLifecycleEvent, ModelLifecycleStateMachine, PartialDownloadInfo,
    LifecycleEventCallback,
};
pub use download::{DownloadError, DownloadProgress, ModelDownloader, format_bytes};
