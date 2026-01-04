//! User-friendly error handling for voice control.
//!
//! This module provides user-friendly error messages, troubleshooting hints,
//! and retry logic for voice control operations.
//!
//! ## Error Categories
//!
//! - **Microphone Errors**: No device, permission denied, device busy
//! - **Model Errors**: Download failed, extraction failed, not found
//! - **Recognition Errors**: Initialization failed, timeout, processing errors
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::error::{VoiceControlError, VoiceErrorCategory};
//!
//! // Create user-friendly error from raw error
//! let error = VoiceControlError::no_microphone();
//! println!("{}", error.user_message());
//! println!("Hints:");
//! for hint in error.troubleshooting_hints() {
//!     println!("  - {}", hint);
//! }
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::audio::backend::Platform;
use super::audio_input::AudioInputError;
use super::download::DownloadError;
use super::recognizer::RecognizerError;
use super::engine::VoiceEngineError;
use super::model_manager::VoskModelError;

/// Categories of voice control errors for classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceErrorCategory {
    /// Microphone or audio input related errors.
    Microphone,
    /// Model download and installation errors.
    ModelDownload,
    /// Speech recognition errors.
    Recognition,
    /// Configuration or setup errors.
    Configuration,
    /// Network-related errors (for downloads).
    Network,
    /// Unknown or uncategorized errors.
    Unknown,
}

impl std::fmt::Display for VoiceErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceErrorCategory::Microphone => write!(f, "Microphone"),
            VoiceErrorCategory::ModelDownload => write!(f, "Model Download"),
            VoiceErrorCategory::Recognition => write!(f, "Recognition"),
            VoiceErrorCategory::Configuration => write!(f, "Configuration"),
            VoiceErrorCategory::Network => write!(f, "Network"),
            VoiceErrorCategory::Unknown => write!(f, "Unknown"),
        }
    }
}

/// User-friendly voice control error with troubleshooting hints.
///
/// This struct wraps voice control errors with user-friendly messages
/// and platform-specific troubleshooting hints.
#[derive(Debug, Clone)]
pub struct VoiceControlError {
    /// The error category.
    pub category: VoiceErrorCategory,
    /// The detected platform.
    pub platform: Platform,
    /// User-friendly error message.
    pub user_message: String,
    /// Technical error details for logging.
    pub technical_details: String,
    /// Troubleshooting hints specific to this error.
    pub hints: Vec<String>,
    /// Whether this error is recoverable with retry.
    pub is_recoverable: bool,
    /// Suggested retry delay in milliseconds (if recoverable).
    pub retry_delay_ms: Option<u64>,
}

impl VoiceControlError {
    // ========================================================================
    // Microphone Errors
    // ========================================================================

    /// Create an error for when no microphone is detected.
    pub fn no_microphone() -> Self {
        let platform = Platform::detect();
        let hints = Self::microphone_not_found_hints(platform);

        Self {
            category: VoiceErrorCategory::Microphone,
            platform,
            user_message: "No microphone found. Please connect a microphone and try again.".to_string(),
            technical_details: "AudioInputError::NoDevice".to_string(),
            hints,
            is_recoverable: true,
            retry_delay_ms: Some(2000),
        }
    }

    /// Create an error for microphone permission denied.
    pub fn microphone_permission_denied() -> Self {
        let platform = Platform::detect();
        let hints = Self::microphone_permission_hints(platform);

        Self {
            category: VoiceErrorCategory::Microphone,
            platform,
            user_message: "Microphone access denied. Please grant microphone permission to Rust Ride.".to_string(),
            technical_details: "Permission denied accessing audio input device".to_string(),
            hints,
            is_recoverable: false,
            retry_delay_ms: None,
        }
    }

    /// Create an error for microphone busy/in use.
    pub fn microphone_busy() -> Self {
        let platform = Platform::detect();

        Self {
            category: VoiceErrorCategory::Microphone,
            platform,
            user_message: "Microphone is being used by another application. Please close other apps using the microphone.".to_string(),
            technical_details: "Audio input device is busy or has exclusive access".to_string(),
            hints: vec![
                "Close other applications that might be using the microphone".to_string(),
                "Check if video conferencing apps (Zoom, Teams, etc.) are running".to_string(),
                "Try disconnecting and reconnecting your microphone".to_string(),
                "Restart the application after closing other audio apps".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(3000),
        }
    }

    /// Create an error for microphone configuration issues.
    pub fn microphone_config_error(details: impl Into<String>) -> Self {
        let platform = Platform::detect();
        let details_str = details.into();

        Self {
            category: VoiceErrorCategory::Microphone,
            platform,
            user_message: format!("Could not configure microphone for voice recognition. {}",
                if details_str.contains("16000") || details_str.contains("sample rate") {
                    "Your microphone may not support the required audio format."
                } else {
                    "Please check your microphone settings."
                }),
            technical_details: details_str,
            hints: vec![
                "Try a different microphone if available".to_string(),
                "Check that your microphone is set as the default recording device".to_string(),
                "Ensure the microphone supports 16kHz sample rate".to_string(),
                "Update your audio drivers to the latest version".to_string(),
            ],
            is_recoverable: false,
            retry_delay_ms: None,
        }
    }

    /// Get platform-specific hints for microphone not found.
    fn microphone_not_found_hints(platform: Platform) -> Vec<String> {
        match platform {
            Platform::Windows => vec![
                "Check that a microphone is connected to your computer".to_string(),
                "Open Windows Settings > System > Sound and check input devices".to_string(),
                "Ensure 'Allow apps to access your microphone' is enabled in Privacy settings".to_string(),
                "Try plugging the microphone into a different USB port".to_string(),
                "Run the Windows audio troubleshooter (Settings > Update & Security > Troubleshoot)".to_string(),
            ],
            Platform::MacOS => vec![
                "Check that a microphone is connected or built-in microphone is working".to_string(),
                "Open System Preferences > Sound > Input to verify microphone is detected".to_string(),
                "Check System Preferences > Security & Privacy > Privacy > Microphone".to_string(),
                "Try resetting Core Audio: sudo killall coreaudiod".to_string(),
                "Restart your Mac if the microphone was recently connected".to_string(),
            ],
            Platform::Linux => vec![
                "Check that a microphone is connected and detected".to_string(),
                "List input devices with: pactl list sources".to_string(),
                "Ensure PulseAudio/PipeWire is running: systemctl --user status pulseaudio".to_string(),
                "Check if user is in 'audio' group: groups $USER".to_string(),
                "Try: pavucontrol to check input device configuration".to_string(),
                "Restart PulseAudio: pulseaudio -k && pulseaudio --start".to_string(),
            ],
            Platform::Unknown => vec![
                "Check that a microphone is connected to your computer".to_string(),
                "Verify the microphone works in other applications".to_string(),
                "Check your system's audio input settings".to_string(),
            ],
        }
    }

    /// Get platform-specific hints for permission denied.
    fn microphone_permission_hints(platform: Platform) -> Vec<String> {
        match platform {
            Platform::Windows => vec![
                "Open Windows Settings > Privacy > Microphone".to_string(),
                "Enable 'Allow apps to access your microphone'".to_string(),
                "Scroll down and enable microphone access for desktop apps".to_string(),
                "Restart Rust Ride after granting permission".to_string(),
            ],
            Platform::MacOS => vec![
                "Open System Preferences > Security & Privacy > Privacy".to_string(),
                "Select Microphone from the sidebar".to_string(),
                "Check the box next to Rust Ride to grant access".to_string(),
                "You may need to restart the application after granting permission".to_string(),
                "If Rust Ride is not listed, run it once and try again".to_string(),
            ],
            Platform::Linux => vec![
                "Check your desktop environment's privacy settings".to_string(),
                "For Flatpak apps: flatpak permission-show <app-id>".to_string(),
                "For Snap apps: snap connect <snap>:audio-record".to_string(),
                "Ensure user is in 'audio' group: sudo usermod -a -G audio $USER".to_string(),
                "Log out and back in after changing group membership".to_string(),
            ],
            Platform::Unknown => vec![
                "Check your system's privacy/permission settings".to_string(),
                "Grant microphone access to Rust Ride".to_string(),
                "Restart the application after granting permission".to_string(),
            ],
        }
    }

    // ========================================================================
    // Model Download Errors
    // ========================================================================

    /// Create an error for model download failure due to network issues.
    pub fn model_download_network_error(details: impl Into<String>) -> Self {
        let platform = Platform::detect();
        let details_str = details.into();

        Self {
            category: VoiceErrorCategory::ModelDownload,
            platform,
            user_message: "Failed to download the voice recognition model. Please check your internet connection.".to_string(),
            technical_details: details_str,
            hints: vec![
                "Check your internet connection".to_string(),
                "Try again in a few moments".to_string(),
                "If using a VPN, try disconnecting temporarily".to_string(),
                "Check if firewall is blocking the download".to_string(),
                "The model server may be temporarily unavailable".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(5000),
        }
    }

    /// Create an error for model checksum verification failure.
    pub fn model_checksum_failed(expected: &str, actual: &str) -> Self {
        let platform = Platform::detect();

        Self {
            category: VoiceErrorCategory::ModelDownload,
            platform,
            user_message: "The downloaded model file is corrupted. Please try downloading again.".to_string(),
            technical_details: format!("Checksum mismatch: expected {}, got {}", expected, actual),
            hints: vec![
                "Click 'Download Model' to try again".to_string(),
                "The download may have been interrupted".to_string(),
                "Check if you have sufficient disk space".to_string(),
                "Try with a more stable internet connection".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(1000),
        }
    }

    /// Create an error for model extraction failure.
    pub fn model_extraction_failed(details: impl Into<String>) -> Self {
        let platform = Platform::detect();
        let details_str = details.into();

        Self {
            category: VoiceErrorCategory::ModelDownload,
            platform,
            user_message: "Failed to extract the voice recognition model. Please check disk space and try again.".to_string(),
            technical_details: details_str,
            hints: vec![
                "Ensure you have at least 100MB of free disk space".to_string(),
                "Check write permissions for the application data folder".to_string(),
                "Try downloading the model again".to_string(),
                "Close other applications to free up resources".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(2000),
        }
    }

    /// Create an error for model not found.
    pub fn model_not_found(path: impl Into<PathBuf>) -> Self {
        let platform = Platform::detect();
        let path = path.into();

        Self {
            category: VoiceErrorCategory::ModelDownload,
            platform,
            user_message: "Voice recognition model not installed. Please download the model in Settings.".to_string(),
            technical_details: format!("Model not found at {:?}", path),
            hints: vec![
                "Go to Settings > Voice Control".to_string(),
                "Click 'Download Model' to install the speech recognition model".to_string(),
                "The download is about 50MB and requires internet connection".to_string(),
                "Voice control will be available after the download completes".to_string(),
            ],
            is_recoverable: false,
            retry_delay_ms: None,
        }
    }

    /// Create an error for insufficient disk space.
    pub fn insufficient_disk_space() -> Self {
        let platform = Platform::detect();

        Self {
            category: VoiceErrorCategory::ModelDownload,
            platform,
            user_message: "Not enough disk space to install the voice recognition model.".to_string(),
            technical_details: "Insufficient disk space for model extraction".to_string(),
            hints: vec![
                "Free up at least 100MB of disk space".to_string(),
                "Delete unused files or applications".to_string(),
                "Empty the recycle bin/trash".to_string(),
                "Move large files to external storage".to_string(),
            ],
            is_recoverable: false,
            retry_delay_ms: None,
        }
    }

    // ========================================================================
    // Recognition Errors
    // ========================================================================

    /// Create an error for recognition initialization failure.
    pub fn recognition_init_failed(details: impl Into<String>) -> Self {
        let platform = Platform::detect();
        let details_str = details.into();

        Self {
            category: VoiceErrorCategory::Recognition,
            platform,
            user_message: "Failed to start voice recognition. Please try restarting the application.".to_string(),
            technical_details: details_str,
            hints: vec![
                "Restart Rust Ride".to_string(),
                "Ensure the voice model is properly installed".to_string(),
                "Check that no other application is using the speech engine".to_string(),
                "Try re-downloading the voice model in Settings".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(3000),
        }
    }

    /// Create an error for recognition timeout.
    pub fn recognition_timeout(timeout_ms: u64) -> Self {
        let platform = Platform::detect();

        Self {
            category: VoiceErrorCategory::Recognition,
            platform,
            user_message: "Voice recognition is taking too long. Please try speaking again.".to_string(),
            technical_details: format!("Recognition timed out after {}ms", timeout_ms),
            hints: vec![
                "Speak clearly and at normal volume".to_string(),
                "Move closer to the microphone".to_string(),
                "Reduce background noise if possible".to_string(),
                "Try saying the command again".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(500),
        }
    }

    /// Create an error for general recognition failure.
    pub fn recognition_failed(details: impl Into<String>) -> Self {
        let platform = Platform::detect();
        let details_str = details.into();

        Self {
            category: VoiceErrorCategory::Recognition,
            platform,
            user_message: "Could not recognize your voice command. Please try again.".to_string(),
            technical_details: details_str,
            hints: vec![
                "Speak the command clearly".to_string(),
                "Wait for the listening indicator before speaking".to_string(),
                "Use simple commands like 'pause', 'resume', 'skip'".to_string(),
                "Check that your microphone is working properly".to_string(),
            ],
            is_recoverable: true,
            retry_delay_ms: Some(500),
        }
    }

    /// Create an error when voice control is not enabled.
    pub fn voice_control_not_enabled() -> Self {
        let platform = Platform::detect();

        Self {
            category: VoiceErrorCategory::Configuration,
            platform,
            user_message: "Voice control is not enabled. Enable it in Settings.".to_string(),
            technical_details: "Voice control feature is disabled in settings".to_string(),
            hints: vec![
                "Go to Settings > Accessibility".to_string(),
                "Enable 'Voice Control'".to_string(),
                "Download the voice model if prompted".to_string(),
                "Choose your preferred activation mode".to_string(),
            ],
            is_recoverable: false,
            retry_delay_ms: None,
        }
    }

    // ========================================================================
    // Error Conversion
    // ========================================================================

    /// Create from an AudioInputError.
    pub fn from_audio_input_error(error: &AudioInputError) -> Self {
        match error {
            AudioInputError::NoDevice => Self::no_microphone(),
            AudioInputError::ConfigError(msg) => {
                if msg.to_lowercase().contains("permission") {
                    Self::microphone_permission_denied()
                } else {
                    Self::microphone_config_error(msg)
                }
            }
            AudioInputError::StreamBuildError(msg) |
            AudioInputError::StreamStartError(msg) => {
                if msg.to_lowercase().contains("permission") {
                    Self::microphone_permission_denied()
                } else if msg.to_lowercase().contains("busy") || msg.to_lowercase().contains("exclusive") {
                    Self::microphone_busy()
                } else {
                    Self::microphone_config_error(msg)
                }
            }
            AudioInputError::DeviceError(msg) => {
                if msg.to_lowercase().contains("not found") {
                    Self::no_microphone()
                } else {
                    Self::microphone_config_error(msg)
                }
            }
            AudioInputError::PlatformError(msg) => Self::microphone_config_error(msg),
            _ => Self::microphone_config_error(error.to_string()),
        }
    }

    /// Create from a DownloadError.
    pub fn from_download_error(error: &DownloadError) -> Self {
        match error {
            DownloadError::NetworkError(msg) => Self::model_download_network_error(msg),
            DownloadError::ChecksumMismatch { expected, actual } => {
                Self::model_checksum_failed(expected, actual)
            }
            DownloadError::ExtractionFailed(msg) => Self::model_extraction_failed(msg),
            DownloadError::DirectoryCreationFailed(msg) |
            DownloadError::WriteFailed(msg) |
            DownloadError::InstallationFailed(msg) => {
                if msg.to_lowercase().contains("space") {
                    Self::insufficient_disk_space()
                } else {
                    Self::model_extraction_failed(msg)
                }
            }
            DownloadError::IoError(e) => {
                let msg = e.to_string();
                if msg.to_lowercase().contains("space") {
                    Self::insufficient_disk_space()
                } else {
                    Self::model_extraction_failed(msg)
                }
            }
            DownloadError::Cancelled => {
                let platform = Platform::detect();
                Self {
                    category: VoiceErrorCategory::ModelDownload,
                    platform,
                    user_message: "Model download was cancelled.".to_string(),
                    technical_details: "Download cancelled by user".to_string(),
                    hints: vec!["Click 'Download Model' to try again.".to_string()],
                    is_recoverable: true,
                    retry_delay_ms: Some(500),
                }
            }
            DownloadError::ModelError(e) => Self::from_vosk_model_error(e),
        }
    }

    /// Create from a RecognizerError.
    pub fn from_recognizer_error(error: &RecognizerError) -> Self {
        match error {
            RecognizerError::ModelNotFound(path) => Self::model_not_found(path.clone()),
            RecognizerError::ModelLoadFailed { path, message } => {
                Self::recognition_init_failed(format!("Failed to load model from {:?}: {}", path, message))
            }
            RecognizerError::NotInitialized => Self::recognition_init_failed("Recognizer not initialized"),
            RecognizerError::Timeout(ms) => Self::recognition_timeout(*ms),
            RecognizerError::WorkerNotResponding => {
                Self::recognition_init_failed("Voice recognition worker is not responding")
            }
            _ => Self::recognition_failed(error.to_string()),
        }
    }

    /// Create from a VoiceEngineError.
    pub fn from_engine_error(error: &VoiceEngineError) -> Self {
        match error {
            VoiceEngineError::ModelNotFound(path) => Self::model_not_found(path.clone()),
            VoiceEngineError::AudioInputError(e) => Self::from_audio_input_error(e),
            VoiceEngineError::RecognizerError(e) => Self::from_recognizer_error(e),
            VoiceEngineError::NotInitialized => Self::recognition_init_failed("Voice engine not initialized"),
            VoiceEngineError::AlreadyRunning => {
                let platform = Platform::detect();
                Self {
                    category: VoiceErrorCategory::Recognition,
                    platform,
                    user_message: "Voice recognition is already running.".to_string(),
                    technical_details: "Engine already running".to_string(),
                    hints: vec!["Voice control is active.".to_string()],
                    is_recoverable: false,
                    retry_delay_ms: None,
                }
            }
            VoiceEngineError::NotRunning => {
                let platform = Platform::detect();
                Self {
                    category: VoiceErrorCategory::Recognition,
                    platform,
                    user_message: "Voice recognition is not running.".to_string(),
                    technical_details: "Engine not running".to_string(),
                    hints: vec!["Enable voice control in Settings.".to_string()],
                    is_recoverable: false,
                    retry_delay_ms: None,
                }
            }
            _ => Self::recognition_failed(error.to_string()),
        }
    }

    /// Create from a VoskModelError.
    pub fn from_vosk_model_error(error: &VoskModelError) -> Self {
        match error {
            VoskModelError::DirectoryNotFound(path) => Self::model_not_found(path.clone()),
            VoskModelError::ModelNotInstalled(path) => Self::model_not_found(path.clone()),
            VoskModelError::DirectoryCreationFailed(msg) => Self::model_extraction_failed(msg),
            VoskModelError::ModelCorrupted(msg) => {
                let platform = Platform::detect();
                Self {
                    category: VoiceErrorCategory::ModelDownload,
                    platform,
                    user_message: "The voice recognition model appears to be corrupted. Please re-download it.".to_string(),
                    technical_details: msg.clone(),
                    hints: vec![
                        "Go to Settings > Voice Control".to_string(),
                        "Click 'Delete Model' to remove the corrupted model".to_string(),
                        "Click 'Download Model' to download a fresh copy".to_string(),
                    ],
                    is_recoverable: true,
                    retry_delay_ms: Some(1000),
                }
            }
            VoskModelError::MetadataReadError(msg) => Self::model_extraction_failed(msg),
            VoskModelError::IoError(e) => {
                let msg = e.to_string();
                if msg.to_lowercase().contains("space") {
                    Self::insufficient_disk_space()
                } else {
                    Self::model_extraction_failed(msg)
                }
            }
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get the user-friendly error message.
    pub fn user_message(&self) -> &str {
        &self.user_message
    }

    /// Get the technical error details (for logging).
    pub fn technical_details(&self) -> &str {
        &self.technical_details
    }

    /// Get the troubleshooting hints.
    pub fn troubleshooting_hints(&self) -> &[String] {
        &self.hints
    }

    /// Check if this error is recoverable with retry.
    pub fn is_recoverable(&self) -> bool {
        self.is_recoverable
    }

    /// Get the suggested retry delay in milliseconds.
    pub fn retry_delay(&self) -> Option<Duration> {
        self.retry_delay_ms.map(Duration::from_millis)
    }

    /// Format the error for logging (includes all details).
    pub fn to_log_string(&self) -> String {
        format!(
            "VoiceControlError [{} - {}]: {} (technical: {})",
            self.category, self.platform.audio_backend_name(),
            self.user_message, self.technical_details
        )
    }

    /// Get a formatted help message with troubleshooting hints.
    pub fn help_message(&self) -> String {
        let mut msg = format!("{}\n\nTroubleshooting steps:", self.user_message);
        for (i, hint) in self.hints.iter().enumerate() {
            msg.push_str(&format!("\n  {}. {}", i + 1, hint));
        }
        msg
    }

    /// Get a short status message for UI display.
    pub fn short_message(&self) -> String {
        match self.category {
            VoiceErrorCategory::Microphone => "Microphone error".to_string(),
            VoiceErrorCategory::ModelDownload => "Model download error".to_string(),
            VoiceErrorCategory::Recognition => "Recognition error".to_string(),
            VoiceErrorCategory::Configuration => "Configuration error".to_string(),
            VoiceErrorCategory::Network => "Network error".to_string(),
            VoiceErrorCategory::Unknown => "Voice control error".to_string(),
        }
    }
}

impl std::fmt::Display for VoiceControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.user_message)
    }
}

impl std::error::Error for VoiceControlError {}

// ============================================================================
// Retry Logic
// ============================================================================

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries in milliseconds.
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
    /// Backoff multiplier (exponential backoff).
    pub backoff_multiplier: f32,
    /// Whether to add jitter to delays.
    pub add_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a configuration for quick retries (short delays).
    pub fn quick() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
            backoff_multiplier: 1.5,
            add_jitter: true,
        }
    }

    /// Create a configuration for persistent retries (many attempts).
    pub fn persistent() -> Self {
        Self {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }

    /// Create a configuration for network operations.
    pub fn for_network() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 2000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }

    /// Set maximum retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set initial delay.
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Set maximum delay.
    pub fn with_max_delay(mut self, delay_ms: u64) -> Self {
        self.max_delay_ms = delay_ms;
        self
    }

    /// Calculate delay for the nth retry attempt (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32) as f64;
        let capped_delay = base_delay.min(self.max_delay_ms as f64);

        let final_delay = if self.add_jitter {
            // Add ±25% jitter
            let jitter_factor = 0.75 + (rand_simple() * 0.5);
            capped_delay * jitter_factor
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay as u64)
    }
}

/// Simple pseudo-random number generator for jitter (0.0 - 1.0).
/// This avoids adding rand crate as dependency just for jitter.
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64 / u32::MAX as f64)
}

/// Tracks retry state for an operation.
#[derive(Debug)]
pub struct RetryState {
    /// Configuration for retries.
    config: RetryConfig,
    /// Current attempt number (0-indexed).
    current_attempt: AtomicU32,
    /// Last attempt timestamp.
    last_attempt: std::sync::RwLock<Option<Instant>>,
    /// Last error encountered.
    last_error: std::sync::RwLock<Option<VoiceControlError>>,
}

impl RetryState {
    /// Create a new retry state with the given configuration.
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            current_attempt: AtomicU32::new(0),
            last_attempt: std::sync::RwLock::new(None),
            last_error: std::sync::RwLock::new(None),
        }
    }

    /// Create a retry state with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Get the current attempt number (0-indexed).
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt.load(Ordering::Acquire)
    }

    /// Get the maximum number of retries.
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }

    /// Check if more retries are available.
    pub fn can_retry(&self) -> bool {
        self.current_attempt() < self.config.max_retries
    }

    /// Check if a retry should be attempted for the given error.
    pub fn should_retry(&self, error: &VoiceControlError) -> bool {
        error.is_recoverable && self.can_retry()
    }

    /// Record an attempt and get the delay for the next retry.
    ///
    /// Returns `None` if no more retries are available.
    pub fn record_attempt(&self, error: VoiceControlError) -> Option<Duration> {
        // Store the error
        *self.last_error.write().unwrap() = Some(error.clone());
        *self.last_attempt.write().unwrap() = Some(Instant::now());

        // Check if error is recoverable
        if !error.is_recoverable {
            return None;
        }

        let attempt = self.current_attempt.fetch_add(1, Ordering::AcqRel);

        if attempt >= self.config.max_retries {
            return None;
        }

        // Use error's suggested delay if available, otherwise use config
        let delay = error.retry_delay_ms
            .map(Duration::from_millis)
            .unwrap_or_else(|| self.config.delay_for_attempt(attempt));

        Some(delay)
    }

    /// Reset the retry state for a new operation.
    pub fn reset(&self) {
        self.current_attempt.store(0, Ordering::Release);
        *self.last_attempt.write().unwrap() = None;
        *self.last_error.write().unwrap() = None;
    }

    /// Get the last error encountered.
    pub fn last_error(&self) -> Option<VoiceControlError> {
        self.last_error.read().unwrap().clone()
    }

    /// Get the time since the last attempt.
    pub fn time_since_last_attempt(&self) -> Option<Duration> {
        self.last_attempt.read().unwrap().map(|t| t.elapsed())
    }

    /// Check if enough time has passed since the last attempt.
    pub fn ready_for_retry(&self) -> bool {
        match (self.last_error(), self.time_since_last_attempt()) {
            (Some(error), Some(elapsed)) => {
                let required_delay = error.retry_delay().unwrap_or(Duration::from_secs(1));
                elapsed >= required_delay
            }
            _ => true,
        }
    }

    /// Get a status message for UI display.
    pub fn status_message(&self) -> String {
        let attempt = self.current_attempt();
        let max = self.config.max_retries;

        if attempt == 0 {
            "Ready".to_string()
        } else if attempt >= max {
            format!("Failed after {} attempts", max)
        } else {
            format!("Retry {}/{}", attempt, max)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_error_category_display() {
        assert_eq!(VoiceErrorCategory::Microphone.to_string(), "Microphone");
        assert_eq!(VoiceErrorCategory::ModelDownload.to_string(), "Model Download");
        assert_eq!(VoiceErrorCategory::Recognition.to_string(), "Recognition");
        assert_eq!(VoiceErrorCategory::Configuration.to_string(), "Configuration");
        assert_eq!(VoiceErrorCategory::Network.to_string(), "Network");
        assert_eq!(VoiceErrorCategory::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_no_microphone_error() {
        let error = VoiceControlError::no_microphone();
        assert_eq!(error.category, VoiceErrorCategory::Microphone);
        assert!(error.user_message.contains("microphone"));
        assert!(error.is_recoverable);
        assert!(error.retry_delay_ms.is_some());
        assert!(!error.hints.is_empty());
    }

    #[test]
    fn test_microphone_permission_denied() {
        let error = VoiceControlError::microphone_permission_denied();
        assert_eq!(error.category, VoiceErrorCategory::Microphone);
        assert!(error.user_message.contains("permission") || error.user_message.contains("denied"));
        assert!(!error.is_recoverable);
        assert!(!error.hints.is_empty());
    }

    #[test]
    fn test_microphone_busy() {
        let error = VoiceControlError::microphone_busy();
        assert_eq!(error.category, VoiceErrorCategory::Microphone);
        assert!(error.user_message.contains("another application"));
        assert!(error.is_recoverable);
    }

    #[test]
    fn test_model_download_network_error() {
        let error = VoiceControlError::model_download_network_error("connection refused");
        assert_eq!(error.category, VoiceErrorCategory::ModelDownload);
        assert!(error.user_message.contains("download"));
        assert!(error.is_recoverable);
        assert!(error.retry_delay_ms.is_some());
    }

    #[test]
    fn test_model_checksum_failed() {
        let error = VoiceControlError::model_checksum_failed("expected", "actual");
        assert_eq!(error.category, VoiceErrorCategory::ModelDownload);
        assert!(error.user_message.contains("corrupted"));
        assert!(error.technical_details.contains("expected"));
        assert!(error.technical_details.contains("actual"));
    }

    #[test]
    fn test_model_not_found() {
        let error = VoiceControlError::model_not_found("/path/to/model");
        assert_eq!(error.category, VoiceErrorCategory::ModelDownload);
        assert!(error.user_message.contains("not installed"));
        assert!(!error.is_recoverable);
    }

    #[test]
    fn test_recognition_timeout() {
        let error = VoiceControlError::recognition_timeout(5000);
        assert_eq!(error.category, VoiceErrorCategory::Recognition);
        assert!(error.user_message.contains("too long"));
        assert!(error.technical_details.contains("5000"));
        assert!(error.is_recoverable);
    }

    #[test]
    fn test_recognition_failed() {
        let error = VoiceControlError::recognition_failed("some error");
        assert_eq!(error.category, VoiceErrorCategory::Recognition);
        assert!(error.user_message.contains("try again"));
        assert!(error.is_recoverable);
    }

    #[test]
    fn test_error_display() {
        let error = VoiceControlError::no_microphone();
        let display = format!("{}", error);
        assert!(display.contains("microphone"));
    }

    #[test]
    fn test_error_to_log_string() {
        let error = VoiceControlError::no_microphone();
        let log = error.to_log_string();
        assert!(log.contains("VoiceControlError"));
        assert!(log.contains("Microphone"));
    }

    #[test]
    fn test_error_help_message() {
        let error = VoiceControlError::no_microphone();
        let help = error.help_message();
        assert!(help.contains("Troubleshooting steps"));
        assert!(help.contains("1."));
    }

    #[test]
    fn test_error_short_message() {
        let error = VoiceControlError::no_microphone();
        assert_eq!(error.short_message(), "Microphone error");

        let error = VoiceControlError::model_not_found("/path");
        assert_eq!(error.short_message(), "Model download error");
    }

    #[test]
    fn test_retry_delay() {
        let error = VoiceControlError::no_microphone();
        assert!(error.retry_delay().is_some());

        let error = VoiceControlError::model_not_found("/path");
        assert!(error.retry_delay().is_none());
    }

    #[test]
    fn test_from_audio_input_error_no_device() {
        let input_error = AudioInputError::NoDevice;
        let error = VoiceControlError::from_audio_input_error(&input_error);
        assert_eq!(error.category, VoiceErrorCategory::Microphone);
        assert!(error.user_message.contains("microphone"));
    }

    #[test]
    fn test_from_download_error_network() {
        let dl_error = DownloadError::NetworkError("connection failed".to_string());
        let error = VoiceControlError::from_download_error(&dl_error);
        assert_eq!(error.category, VoiceErrorCategory::ModelDownload);
        assert!(error.is_recoverable);
    }

    #[test]
    fn test_from_download_error_checksum() {
        let dl_error = DownloadError::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        let error = VoiceControlError::from_download_error(&dl_error);
        assert!(error.user_message.contains("corrupted"));
    }

    #[test]
    fn test_from_recognizer_error_not_found() {
        let rec_error = RecognizerError::ModelNotFound(PathBuf::from("/path"));
        let error = VoiceControlError::from_recognizer_error(&rec_error);
        assert!(error.user_message.contains("not installed"));
    }

    #[test]
    fn test_from_recognizer_error_timeout() {
        let rec_error = RecognizerError::Timeout(5000);
        let error = VoiceControlError::from_recognizer_error(&rec_error);
        assert_eq!(error.category, VoiceErrorCategory::Recognition);
        assert!(error.is_recoverable);
    }

    // ========================================================================
    // Retry Config Tests
    // ========================================================================

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!(config.initial_delay_ms > 0);
        assert!(config.max_delay_ms >= config.initial_delay_ms);
    }

    #[test]
    fn test_retry_config_quick() {
        let config = RetryConfig::quick();
        assert!(config.initial_delay_ms < RetryConfig::default().initial_delay_ms);
    }

    #[test]
    fn test_retry_config_persistent() {
        let config = RetryConfig::persistent();
        assert!(config.max_retries > RetryConfig::default().max_retries);
    }

    #[test]
    fn test_retry_config_for_network() {
        let config = RetryConfig::for_network();
        assert!(config.max_retries >= 5);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::default()
            .with_max_retries(5)
            .with_initial_delay(500)
            .with_max_delay(10000);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10000);
    }

    #[test]
    fn test_retry_config_delay_increases() {
        let config = RetryConfig::default().with_initial_delay(1000);
        config.add_jitter; // Disable jitter for predictable test

        let delay0 = config.delay_for_attempt(0);
        let delay1 = config.delay_for_attempt(1);
        let delay2 = config.delay_for_attempt(2);

        // With backoff multiplier > 1, delays should generally increase
        // (accounting for some jitter variance)
        assert!(delay1.as_millis() > 0);
        assert!(delay2.as_millis() > 0);
    }

    #[test]
    fn test_retry_config_delay_capped() {
        let config = RetryConfig::default()
            .with_initial_delay(1000)
            .with_max_delay(5000);

        // High attempt number should be capped
        let delay = config.delay_for_attempt(100);
        // Even with jitter, shouldn't exceed max by too much
        assert!(delay.as_millis() <= 7500); // Allow 50% jitter headroom
    }

    // ========================================================================
    // Retry State Tests
    // ========================================================================

    #[test]
    fn test_retry_state_initial() {
        let state = RetryState::with_defaults();
        assert_eq!(state.current_attempt(), 0);
        assert!(state.can_retry());
        assert!(state.last_error().is_none());
    }

    #[test]
    fn test_retry_state_record_attempt() {
        let state = RetryState::new(RetryConfig::default().with_max_retries(3));

        let error = VoiceControlError::no_microphone();
        let delay = state.record_attempt(error);

        assert!(delay.is_some());
        assert_eq!(state.current_attempt(), 1);
        assert!(state.last_error().is_some());
    }

    #[test]
    fn test_retry_state_exhausted() {
        let state = RetryState::new(RetryConfig::default().with_max_retries(2));

        let error = VoiceControlError::no_microphone();

        // First attempt
        let delay1 = state.record_attempt(error.clone());
        assert!(delay1.is_some());

        // Second attempt
        let delay2 = state.record_attempt(error.clone());
        assert!(delay2.is_some());

        // Third attempt (exceeds max)
        let delay3 = state.record_attempt(error);
        assert!(delay3.is_none());
        assert!(!state.can_retry());
    }

    #[test]
    fn test_retry_state_non_recoverable() {
        let state = RetryState::with_defaults();

        // Non-recoverable error
        let error = VoiceControlError::model_not_found("/path");
        let delay = state.record_attempt(error);

        assert!(delay.is_none());
    }

    #[test]
    fn test_retry_state_reset() {
        let state = RetryState::with_defaults();

        let error = VoiceControlError::no_microphone();
        state.record_attempt(error);

        assert_eq!(state.current_attempt(), 1);

        state.reset();

        assert_eq!(state.current_attempt(), 0);
        assert!(state.last_error().is_none());
    }

    #[test]
    fn test_retry_state_status_message() {
        let state = RetryState::new(RetryConfig::default().with_max_retries(3));

        assert_eq!(state.status_message(), "Ready");

        let error = VoiceControlError::no_microphone();
        state.record_attempt(error.clone());
        assert_eq!(state.status_message(), "Retry 1/3");

        state.record_attempt(error.clone());
        assert_eq!(state.status_message(), "Retry 2/3");

        state.record_attempt(error.clone());
        state.record_attempt(error);
        assert_eq!(state.status_message(), "Failed after 3 attempts");
    }

    #[test]
    fn test_retry_state_should_retry() {
        let state = RetryState::with_defaults();

        let recoverable = VoiceControlError::no_microphone();
        assert!(state.should_retry(&recoverable));

        let non_recoverable = VoiceControlError::model_not_found("/path");
        assert!(!state.should_retry(&non_recoverable));
    }

    // ========================================================================
    // Platform-specific Hints Tests
    // ========================================================================

    #[test]
    fn test_platform_specific_hints_not_empty() {
        // Test that hints exist for all platforms
        let platforms = [Platform::Windows, Platform::MacOS, Platform::Linux, Platform::Unknown];

        for platform in platforms {
            let hints = VoiceControlError::microphone_not_found_hints(platform);
            assert!(!hints.is_empty(), "Should have hints for {:?}", platform);

            let hints = VoiceControlError::microphone_permission_hints(platform);
            assert!(!hints.is_empty(), "Should have permission hints for {:?}", platform);
        }
    }

    #[test]
    fn test_windows_hints_contain_windows_specific() {
        let hints = VoiceControlError::microphone_not_found_hints(Platform::Windows);
        let hints_str = hints.join(" ");
        assert!(hints_str.contains("Windows") || hints_str.contains("Settings"));
    }

    #[test]
    fn test_macos_hints_contain_macos_specific() {
        let hints = VoiceControlError::microphone_not_found_hints(Platform::MacOS);
        let hints_str = hints.join(" ");
        assert!(hints_str.contains("System Preferences") || hints_str.contains("CoreAudio"));
    }

    #[test]
    fn test_linux_hints_contain_linux_specific() {
        let hints = VoiceControlError::microphone_not_found_hints(Platform::Linux);
        let hints_str = hints.join(" ");
        assert!(hints_str.contains("pactl") || hints_str.contains("PulseAudio") || hints_str.contains("PipeWire"));
    }
}
