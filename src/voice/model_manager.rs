//! Vosk model management for speech recognition.
//!
//! This module handles the lifecycle of the Vosk speech recognition model,
//! including path resolution, state tracking, and preparation for download.
//!
//! ## Model Information
//!
//! The default model is `vosk-model-small-en-us-0.15`:
//! - Size: ~50MB compressed, ~100MB extracted
//! - Language: English (US)
//! - Accuracy: Good for command recognition in quiet environments
//! - URL: <https://alphacephei.com/vosk/models>
//!
//! ## Storage Location
//!
//! Models are stored in the application data directory under `vosk-model/`:
//! - Windows: `%APPDATA%\RustRide\vosk-model`
//! - macOS: `~/Library/Application Support/RustRide/vosk-model`
//! - Linux: `~/.local/share/RustRide/vosk-model`
//!
//! ## State Machine
//!
//! The model lifecycle follows this state machine:
//!
//! ```text
//!                    ┌─────────────┐
//!                    │ Uninitialized│
//!                    └──────┬──────┘
//!                           │ check_state()
//!                    ┌──────▼──────┐
//!              ┌─────│ NotInstalled │
//!              │     └──────┬──────┘
//!              │            │ start_download()
//!              │     ┌──────▼──────┐
//!              │     │ Downloading  │◄───────┐
//!              │     │  (progress)  │        │ resume
//!              │     └──────┬──────┘        │
//!              │            │ download complete
//!              │     ┌──────▼──────┐        │
//!              │     │  Extracting  │        │
//!              │     └──────┬──────┘        │
//!              │            │               │
//!     ┌────────▼────────────▼───────────────┴──┐
//!     │                                         │
//! ┌───▼───┐                                 ┌───▼───┐
//! │ Error │                                 │ Ready │
//! └───────┘                                 └───────┘
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

/// Default model name for English (US) recognition.
pub const DEFAULT_MODEL_NAME: &str = "vosk-model-small-en-us-0.15";

/// Default model download URL.
pub const DEFAULT_MODEL_URL: &str =
    "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip";

/// Expected SHA256 hash of the model zip file for verification.
pub const DEFAULT_MODEL_SHA256: &str =
    "30f26242c4eb449f948e42cb302dd7a686cb29a3423a8367f99ff41780942498";

/// Errors that can occur during model management.
#[derive(Debug, Error)]
pub enum VoskModelError {
    /// The model directory does not exist and needs to be created.
    #[error("Model directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),

    /// Failed to create the model directory.
    #[error("Failed to create model directory: {0}")]
    DirectoryCreationFailed(String),

    /// The model is not installed.
    #[error("Model not installed at: {0}")]
    ModelNotInstalled(PathBuf),

    /// The model files are corrupted or incomplete.
    #[error("Model is corrupted or incomplete: {0}")]
    ModelCorrupted(String),

    /// Failed to read model metadata.
    #[error("Failed to read model metadata: {0}")]
    MetadataReadError(String),

    /// IO error during model operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// State of the Vosk model installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    /// Model state has not been checked yet.
    Unknown,

    /// Model is not installed and needs to be downloaded.
    NotInstalled,

    /// Model download is in progress.
    Downloading {
        /// Download progress as percentage (0-100).
        progress_percent: u8,
    },

    /// Model is being extracted from the downloaded archive.
    Extracting,

    /// Model is installed and ready to use.
    Ready,

    /// Model installation failed with an error.
    Error,
}

impl Default for ModelState {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ModelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelState::Unknown => write!(f, "Unknown"),
            ModelState::NotInstalled => write!(f, "Not Installed"),
            ModelState::Downloading { progress_percent } => {
                write!(f, "Downloading ({}%)", progress_percent)
            }
            ModelState::Extracting => write!(f, "Extracting..."),
            ModelState::Ready => write!(f, "Ready"),
            ModelState::Error => write!(f, "Error"),
        }
    }
}

/// Events emitted during model lifecycle transitions.
///
/// These events are designed for UI progress updates and can be sent
/// through a channel for async handling.
#[derive(Debug, Clone)]
pub enum ModelLifecycleEvent {
    /// State has transitioned to a new value.
    StateChanged {
        /// The previous state.
        from: ModelState,
        /// The new current state.
        to: ModelState,
    },

    /// Download progress has been updated.
    DownloadProgress {
        /// Bytes downloaded so far.
        bytes_received: u64,
        /// Total bytes to download (if known).
        total_bytes: Option<u64>,
        /// Progress percentage (0-100).
        percent: u8,
    },

    /// Extraction progress (file count based).
    ExtractionProgress {
        /// Number of files extracted so far.
        files_extracted: u32,
        /// Total number of files (if known).
        total_files: Option<u32>,
    },

    /// Download is being resumed from a partial file.
    DownloadResuming {
        /// Bytes already downloaded in the partial file.
        bytes_already_downloaded: u64,
    },

    /// Model installation completed successfully.
    InstallationComplete {
        /// Path where the model was installed.
        model_path: PathBuf,
    },

    /// An error occurred during the lifecycle.
    Error {
        /// The error message.
        message: String,
        /// Whether the error is recoverable (can retry).
        recoverable: bool,
    },
}

impl ModelLifecycleEvent {
    /// Check if this is an error event.
    pub fn is_error(&self) -> bool {
        matches!(self, ModelLifecycleEvent::Error { .. })
    }

    /// Check if this is a completion event.
    pub fn is_complete(&self) -> bool {
        matches!(self, ModelLifecycleEvent::InstallationComplete { .. })
    }

    /// Get the progress percentage if this is a progress event.
    pub fn progress_percent(&self) -> Option<u8> {
        match self {
            ModelLifecycleEvent::DownloadProgress { percent, .. } => Some(*percent),
            ModelLifecycleEvent::StateChanged { to: ModelState::Downloading { progress_percent }, .. } => {
                Some(*progress_percent)
            }
            ModelLifecycleEvent::StateChanged { to: ModelState::Extracting, .. } => Some(95),
            ModelLifecycleEvent::StateChanged { to: ModelState::Ready, .. } => Some(100),
            ModelLifecycleEvent::InstallationComplete { .. } => Some(100),
            _ => None,
        }
    }

    /// Get a human-readable status message for this event.
    pub fn status_message(&self) -> String {
        match self {
            ModelLifecycleEvent::StateChanged { to, .. } => to.to_string(),
            ModelLifecycleEvent::DownloadProgress { percent, bytes_received, total_bytes } => {
                if let Some(total) = total_bytes {
                    format!(
                        "Downloading: {}% ({} / {})",
                        percent,
                        super::download::format_bytes(*bytes_received),
                        super::download::format_bytes(*total)
                    )
                } else {
                    format!(
                        "Downloading: {} received",
                        super::download::format_bytes(*bytes_received)
                    )
                }
            }
            ModelLifecycleEvent::ExtractionProgress { files_extracted, total_files } => {
                if let Some(total) = total_files {
                    format!("Extracting: {}/{} files", files_extracted, total)
                } else {
                    format!("Extracting: {} files", files_extracted)
                }
            }
            ModelLifecycleEvent::DownloadResuming { bytes_already_downloaded } => {
                format!(
                    "Resuming download from {}",
                    super::download::format_bytes(*bytes_already_downloaded)
                )
            }
            ModelLifecycleEvent::InstallationComplete { .. } => "Model installed successfully".to_string(),
            ModelLifecycleEvent::Error { message, .. } => format!("Error: {}", message),
        }
    }
}

/// Callback type for receiving lifecycle events.
pub type LifecycleEventCallback = Arc<dyn Fn(ModelLifecycleEvent) + Send + Sync>;

/// Information about a partial/interrupted download.
#[derive(Debug, Clone)]
pub struct PartialDownloadInfo {
    /// Path to the partial download file.
    pub path: PathBuf,
    /// Size of the partial file in bytes.
    pub bytes_downloaded: u64,
    /// Whether the partial file appears valid for resumption.
    pub can_resume: bool,
}

/// Information about the installed model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Name of the model.
    pub name: String,

    /// Path to the model directory.
    pub path: PathBuf,

    /// Size of the model in bytes (if known).
    pub size_bytes: Option<u64>,

    /// Language supported by the model.
    pub language: String,
}

/// Manages the Vosk speech recognition model lifecycle.
///
/// `VoskModelManager` handles:
/// - Model path resolution using the application data directory
/// - Model state tracking (installed, downloading, etc.)
/// - Model validation (checking for required files)
/// - Preparation for model download and extraction
///
/// # Example
///
/// ```rust,ignore
/// use rustride::voice::VoskModelManager;
///
/// let manager = VoskModelManager::new();
///
/// // Check current state
/// let state = manager.state();
/// println!("Model state: {}", state);
///
/// // Get model path
/// let path = manager.model_path();
/// println!("Model path: {:?}", path);
///
/// // Refresh state from disk
/// manager.refresh_state();
/// ```
pub struct VoskModelManager {
    /// Base directory for storing models.
    base_dir: PathBuf,

    /// Current model state.
    state: ModelState,

    /// Name of the model to use.
    model_name: String,

    /// Last error message if state is Error.
    last_error: Option<String>,
}

impl VoskModelManager {
    /// Create a new model manager using the default data directory.
    ///
    /// The model will be stored in `{data_dir}/vosk-model/`.
    pub fn new() -> Self {
        let base_dir = crate::storage::config::get_data_dir().join("vosk-model");

        let mut manager = Self {
            base_dir,
            state: ModelState::Unknown,
            model_name: DEFAULT_MODEL_NAME.to_string(),
            last_error: None,
        };

        // Check initial state
        manager.refresh_state();

        manager
    }

    /// Create a new model manager with a custom base directory.
    ///
    /// This is primarily useful for testing.
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        let mut manager = Self {
            base_dir,
            state: ModelState::Unknown,
            model_name: DEFAULT_MODEL_NAME.to_string(),
            last_error: None,
        };

        manager.refresh_state();

        manager
    }

    /// Get the current model state.
    pub fn state(&self) -> ModelState {
        self.state
    }

    /// Get the last error message if the state is Error.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Get the path to the model directory.
    ///
    /// This returns the expected path, whether or not the model is installed.
    pub fn model_path(&self) -> PathBuf {
        self.base_dir.clone()
    }

    /// Get the base directory for model storage.
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Get the model name.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Check if the model is ready to use.
    pub fn is_ready(&self) -> bool {
        matches!(self.state, ModelState::Ready)
    }

    /// Check if the model is currently being installed (downloading or extracting).
    pub fn is_installing(&self) -> bool {
        matches!(
            self.state,
            ModelState::Downloading { .. } | ModelState::Extracting
        )
    }

    /// Refresh the model state by checking the filesystem.
    ///
    /// This checks if the model directory exists and contains the required files.
    pub fn refresh_state(&mut self) {
        // Don't override downloading/extracting states
        if self.is_installing() {
            return;
        }

        self.state = match self.check_model_installation() {
            Ok(true) => ModelState::Ready,
            Ok(false) => ModelState::NotInstalled,
            Err(e) => {
                self.last_error = Some(e.to_string());
                ModelState::Error
            }
        };
    }

    /// Check if the model is properly installed.
    ///
    /// Returns `Ok(true)` if the model is installed and valid,
    /// `Ok(false)` if not installed, or `Err` if there's a problem.
    fn check_model_installation(&self) -> Result<bool, VoskModelError> {
        let model_path = self.model_path();

        // Check if directory exists
        if !model_path.exists() {
            tracing::debug!("Model directory does not exist: {:?}", model_path);
            return Ok(false);
        }

        // Check if it's actually a directory
        if !model_path.is_dir() {
            return Err(VoskModelError::ModelCorrupted(format!(
                "Expected directory at {:?}, found file",
                model_path
            )));
        }

        // Check for required Vosk model files
        // A valid Vosk model must have these files:
        // - am/final.mdl (acoustic model)
        // - conf/mfcc.conf or similar config
        // - graph/HCLG.fst or graph/Gr.fst (language model graph)
        // - ivector/final.ie (optional, for some models)

        let required_indicators = [
            // Either am/final.mdl or just files in the root
            model_path.join("am").join("final.mdl"),
            // Alternative: some models have different structure
            model_path.join("final.mdl"),
        ];

        let has_model_file = required_indicators.iter().any(|p| p.exists());

        if !has_model_file {
            // Check if directory is empty or partially extracted
            let entries: Vec<_> = std::fs::read_dir(&model_path)?
                .filter_map(|e| e.ok())
                .collect();

            if entries.is_empty() {
                tracing::debug!("Model directory is empty: {:?}", model_path);
                return Ok(false);
            }

            // Directory has content but missing expected files - might be corrupted
            tracing::warn!(
                "Model directory exists but missing required files: {:?}",
                model_path
            );
            return Err(VoskModelError::ModelCorrupted(
                "Missing required model files (am/final.mdl or final.mdl)".to_string(),
            ));
        }

        tracing::info!("Vosk model found and validated at {:?}", model_path);
        Ok(true)
    }

    /// Ensure the model directory exists, creating it if necessary.
    ///
    /// This should be called before attempting to download or extract a model.
    pub fn ensure_directory(&self) -> Result<(), VoskModelError> {
        let path = self.model_path();

        if !path.exists() {
            tracing::info!("Creating model directory: {:?}", path);
            std::fs::create_dir_all(&path).map_err(|e| {
                VoskModelError::DirectoryCreationFailed(format!(
                    "Failed to create {:?}: {}",
                    path, e
                ))
            })?;
        }

        Ok(())
    }

    /// Get the path where a downloaded model archive should be saved.
    ///
    /// Returns a path like `{base_dir}/vosk-model-small-en-us-0.15.zip`.
    pub fn download_path(&self) -> PathBuf {
        // Store in parent of model dir to avoid extracting into the archive
        let parent = self
            .base_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.base_dir.clone());
        parent.join(format!("{}.zip", self.model_name))
    }

    /// Get the download URL for the model.
    pub fn download_url(&self) -> &'static str {
        DEFAULT_MODEL_URL
    }

    /// Get the expected SHA256 hash of the model archive.
    pub fn expected_sha256(&self) -> &'static str {
        DEFAULT_MODEL_SHA256
    }

    /// Update the state to indicate downloading has started.
    pub fn set_downloading(&mut self, progress_percent: u8) {
        self.state = ModelState::Downloading {
            progress_percent: progress_percent.min(100),
        };
        self.last_error = None;
    }

    /// Update the state to indicate extraction has started.
    pub fn set_extracting(&mut self) {
        self.state = ModelState::Extracting;
        self.last_error = None;
    }

    /// Update the state to indicate the model is ready.
    pub fn set_ready(&mut self) {
        self.state = ModelState::Ready;
        self.last_error = None;
    }

    /// Update the state to indicate an error occurred.
    pub fn set_error(&mut self, error: impl Into<String>) {
        let error_msg = error.into();
        self.last_error = Some(error_msg);
        self.state = ModelState::Error;
    }

    /// Reset the state to NotInstalled.
    ///
    /// This is useful for cancelling a download or retrying after an error.
    pub fn reset_to_not_installed(&mut self) {
        self.state = ModelState::NotInstalled;
    }

    /// Set a custom error message without changing state to Error.
    ///
    /// This is useful for storing cancel reasons.
    pub fn set_last_error(&mut self, error: Option<String>) {
        self.last_error = error;
    }

    /// Get information about the installed model.
    ///
    /// Returns `None` if the model is not installed.
    pub fn model_info(&self) -> Option<ModelInfo> {
        if !self.is_ready() {
            return None;
        }

        let path = self.model_path();

        // Try to calculate directory size
        let size_bytes = calculate_dir_size(&path).ok();

        Some(ModelInfo {
            name: self.model_name.clone(),
            path,
            size_bytes,
            language: "en-US".to_string(),
        })
    }

    /// Delete the installed model.
    ///
    /// This removes the model directory and all its contents.
    /// After calling this, the state will be set to `NotInstalled`.
    pub fn delete_model(&mut self) -> Result<(), VoskModelError> {
        let path = self.model_path();

        if path.exists() {
            tracing::info!("Deleting model at {:?}", path);
            std::fs::remove_dir_all(&path)?;
        }

        // Also remove any leftover download archive
        let download_path = self.download_path();
        if download_path.exists() {
            let _ = std::fs::remove_file(&download_path);
        }

        // Clean up partial downloads too
        let temp_path = self.partial_download_path();
        if temp_path.exists() {
            let _ = std::fs::remove_file(&temp_path);
        }

        self.state = ModelState::NotInstalled;
        self.last_error = None;

        Ok(())
    }

    /// Get the path where partial downloads are stored.
    ///
    /// This is used for resume functionality.
    pub fn partial_download_path(&self) -> PathBuf {
        self.download_path().with_extension("zip.partial")
    }

    /// Check for a partial/interrupted download that can be resumed.
    ///
    /// Returns information about the partial download if one exists.
    pub fn check_partial_download(&self) -> Option<PartialDownloadInfo> {
        let partial_path = self.partial_download_path();

        if !partial_path.exists() {
            return None;
        }

        match std::fs::metadata(&partial_path) {
            Ok(metadata) => {
                let bytes_downloaded = metadata.len();
                // A partial download is valid for resumption if it has some content
                // but is less than the expected size (approximately 50MB for the model)
                let can_resume = bytes_downloaded > 0 && bytes_downloaded < 100_000_000;

                tracing::debug!(
                    "Found partial download at {:?}: {} bytes, can_resume={}",
                    partial_path,
                    bytes_downloaded,
                    can_resume
                );

                Some(PartialDownloadInfo {
                    path: partial_path,
                    bytes_downloaded,
                    can_resume,
                })
            }
            Err(e) => {
                tracing::warn!("Failed to read partial download metadata: {}", e);
                None
            }
        }
    }

    /// Clean up any partial download files.
    pub fn cleanup_partial_download(&self) -> Result<(), VoskModelError> {
        let partial_path = self.partial_download_path();
        if partial_path.exists() {
            tracing::info!("Cleaning up partial download at {:?}", partial_path);
            std::fs::remove_file(&partial_path)?;
        }

        // Also clean up temp extraction directories
        let extract_dir = self.download_path().with_extension("extract");
        if extract_dir.exists() {
            let _ = std::fs::remove_dir_all(&extract_dir);
        }

        Ok(())
    }

    /// Download and install the Vosk model.
    ///
    /// This method handles the complete download lifecycle:
    /// 1. Download the model archive with progress tracking
    /// 2. Verify the SHA256 checksum
    /// 3. Extract the zip archive
    /// 4. Move to final installation location
    ///
    /// Progress is reported via the callback function.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustride::voice::{VoskModelManager, DownloadProgress};
    ///
    /// let mut manager = VoskModelManager::new();
    ///
    /// manager.download_model(|progress| {
    ///     if let Some(percent) = progress.percent() {
    ///         println!("Progress: {}%", percent);
    ///     }
    /// }).await?;
    /// ```
    pub async fn download_model<F>(&mut self, progress_callback: F) -> Result<(), super::download::DownloadError>
    where
        F: Fn(super::download::DownloadProgress) + Send + Sync,
    {
        use super::download::{DownloadProgress, ModelDownloader};

        // Check if already ready
        if self.is_ready() {
            tracing::info!("Model already installed, skipping download");
            progress_callback(DownloadProgress::Complete);
            return Ok(());
        }

        // Check if currently installing
        if self.is_installing() {
            tracing::warn!("Model download already in progress");
            return Err(super::download::DownloadError::InstallationFailed(
                "Download already in progress".to_string(),
            ));
        }

        // Ensure base directory exists
        self.ensure_directory()?;

        let download_path = self.download_path();
        let model_path = self.model_path();

        // Create wrapper callback that updates our state
        let progress_wrapper = |progress: DownloadProgress| {
            match &progress {
                DownloadProgress::Downloading { bytes_received, total_bytes } => {
                    let percent = total_bytes
                        .map(|total| if total > 0 { (bytes_received * 100 / total) as u8 } else { 0 })
                        .unwrap_or(0);
                    // Note: Can't update self.state here since we're in a closure
                    tracing::trace!("Download progress: {}%", percent);
                }
                DownloadProgress::Extracting => {
                    tracing::info!("Extracting model...");
                }
                DownloadProgress::Complete => {
                    tracing::info!("Model installation complete");
                }
                DownloadProgress::Error(e) => {
                    tracing::error!("Download error: {}", e);
                }
                _ => {}
            }
            progress_callback(progress);
        };

        // Update state to downloading
        self.set_downloading(0);

        let downloader = ModelDownloader::new();

        // Perform download and installation
        match downloader.download_and_install(&download_path, &model_path, progress_wrapper).await {
            Ok(()) => {
                self.set_ready();
                Ok(())
            }
            Err(e) => {
                self.set_error(e.to_string());
                Err(e)
            }
        }
    }

    /// Download the model with a channel-based progress receiver.
    ///
    /// This is useful when you need to receive progress updates asynchronously,
    /// for example in a UI thread.
    ///
    /// Returns a receiver that will receive progress updates during the download.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustride::voice::VoskModelManager;
    ///
    /// let mut manager = VoskModelManager::new();
    /// let mut rx = manager.download_model_with_channel().await?;
    ///
    /// while let Some(progress) = rx.recv().await {
    ///     println!("Progress: {:?}", progress);
    /// }
    /// ```
    pub async fn download_model_with_channel(
        &mut self,
    ) -> Result<tokio::sync::mpsc::Receiver<super::download::DownloadProgress>, super::download::DownloadError>
    {
        use super::download::{DownloadProgress, ModelDownloader};

        // Check if already ready
        if self.is_ready() {
            tracing::info!("Model already installed, skipping download");
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx.send(DownloadProgress::Complete).await;
            return Ok(rx);
        }

        // Check if currently installing
        if self.is_installing() {
            return Err(super::download::DownloadError::InstallationFailed(
                "Download already in progress".to_string(),
            ));
        }

        // Ensure base directory exists
        self.ensure_directory()?;

        let download_path = self.download_path();
        let model_path = self.model_path();

        // Create progress channel
        let (callback, rx) = ModelDownloader::create_progress_channel();

        // Update state to downloading
        self.set_downloading(0);

        let downloader = ModelDownloader::new();

        // Perform download and installation
        match downloader.download_and_install(&download_path, &model_path, move |p| callback(p)).await {
            Ok(()) => {
                self.set_ready();
                Ok(rx)
            }
            Err(e) => {
                self.set_error(e.to_string());
                Err(e)
            }
        }
    }
}

impl Default for VoskModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// State machine for managing the Vosk model lifecycle with event emission.
///
/// This wraps `VoskModelManager` and provides event-driven state transitions
/// suitable for UI integration. Events are emitted through a callback or channel
/// for progress updates.
///
/// # Example
///
/// ```rust,ignore
/// use rustride::voice::{ModelLifecycleStateMachine, ModelLifecycleEvent};
/// use std::sync::Arc;
///
/// // Create with callback
/// let state_machine = ModelLifecycleStateMachine::with_callback(Arc::new(|event| {
///     println!("Event: {:?}", event);
/// }));
///
/// // Or create with channel
/// let (state_machine, mut rx) = ModelLifecycleStateMachine::with_channel();
///
/// // Start the download
/// state_machine.start_download().await?;
///
/// // Receive events
/// while let Some(event) = rx.recv().await {
///     match event {
///         ModelLifecycleEvent::DownloadProgress { percent, .. } => {
///             println!("Progress: {}%", percent);
///         }
///         ModelLifecycleEvent::InstallationComplete { .. } => {
///             println!("Done!");
///             break;
///         }
///         _ => {}
///     }
/// }
/// ```
pub struct ModelLifecycleStateMachine {
    /// The underlying model manager.
    manager: VoskModelManager,

    /// Optional callback for receiving lifecycle events.
    event_callback: Option<LifecycleEventCallback>,

    /// Optional channel sender for lifecycle events.
    event_sender: Option<mpsc::Sender<ModelLifecycleEvent>>,
}

impl ModelLifecycleStateMachine {
    /// Create a new state machine with a callback for events.
    pub fn with_callback(callback: LifecycleEventCallback) -> Self {
        Self {
            manager: VoskModelManager::new(),
            event_callback: Some(callback),
            event_sender: None,
        }
    }

    /// Create a new state machine with a custom base directory and callback.
    pub fn with_base_dir_and_callback(base_dir: PathBuf, callback: LifecycleEventCallback) -> Self {
        Self {
            manager: VoskModelManager::with_base_dir(base_dir),
            event_callback: Some(callback),
            event_sender: None,
        }
    }

    /// Create a new state machine with a channel for receiving events.
    ///
    /// Returns the state machine and a receiver for events.
    pub fn with_channel() -> (Self, mpsc::Receiver<ModelLifecycleEvent>) {
        let (tx, rx) = mpsc::channel(100);
        let state_machine = Self {
            manager: VoskModelManager::new(),
            event_callback: None,
            event_sender: Some(tx),
        };
        (state_machine, rx)
    }

    /// Create a new state machine with a custom base directory and channel.
    pub fn with_base_dir_and_channel(
        base_dir: PathBuf,
    ) -> (Self, mpsc::Receiver<ModelLifecycleEvent>) {
        let (tx, rx) = mpsc::channel(100);
        let state_machine = Self {
            manager: VoskModelManager::with_base_dir(base_dir),
            event_callback: None,
            event_sender: Some(tx),
        };
        (state_machine, rx)
    }

    /// Get the current model state.
    pub fn state(&self) -> ModelState {
        self.manager.state()
    }

    /// Check if the model is ready to use.
    pub fn is_ready(&self) -> bool {
        self.manager.is_ready()
    }

    /// Check if the model is currently being installed.
    pub fn is_installing(&self) -> bool {
        self.manager.is_installing()
    }

    /// Get the model path.
    pub fn model_path(&self) -> PathBuf {
        self.manager.model_path()
    }

    /// Get the last error message if any.
    pub fn last_error(&self) -> Option<&str> {
        self.manager.last_error()
    }

    /// Check for a partial download that can be resumed.
    pub fn check_partial_download(&self) -> Option<PartialDownloadInfo> {
        self.manager.check_partial_download()
    }

    /// Get access to the underlying manager.
    pub fn manager(&self) -> &VoskModelManager {
        &self.manager
    }

    /// Get mutable access to the underlying manager.
    pub fn manager_mut(&mut self) -> &mut VoskModelManager {
        &mut self.manager
    }

    /// Emit an event to callback and/or channel.
    fn emit_event(&self, event: ModelLifecycleEvent) {
        // Send to callback if present
        if let Some(callback) = &self.event_callback {
            callback(event.clone());
        }

        // Send to channel if present
        if let Some(sender) = &self.event_sender {
            // Use try_send to avoid blocking
            let _ = sender.try_send(event);
        }
    }

    /// Transition to a new state and emit the appropriate event.
    fn transition_to(&mut self, new_state: ModelState) {
        let old_state = self.manager.state();

        // Apply the state change
        match new_state {
            ModelState::Unknown => {
                // Can't transition to Unknown
            }
            ModelState::NotInstalled => {
                // Reset state - happens after errors or cleanup
            }
            ModelState::Downloading { progress_percent } => {
                self.manager.set_downloading(progress_percent);
            }
            ModelState::Extracting => {
                self.manager.set_extracting();
            }
            ModelState::Ready => {
                self.manager.set_ready();
            }
            ModelState::Error => {
                // Error state should be set with a message via set_error
            }
        }

        // Emit state change event
        if old_state != new_state {
            self.emit_event(ModelLifecycleEvent::StateChanged {
                from: old_state,
                to: new_state,
            });
        }
    }

    /// Start the model download process.
    ///
    /// This method handles the complete lifecycle:
    /// 1. Check for partial downloads and optionally resume
    /// 2. Download the model with progress events
    /// 3. Verify the checksum
    /// 4. Extract the model
    /// 5. Install to the final location
    ///
    /// Events are emitted throughout the process for UI updates.
    pub async fn start_download(&mut self) -> Result<(), super::download::DownloadError> {
        self.start_download_with_resume(true).await
    }

    /// Start the model download process with optional resume support.
    ///
    /// If `try_resume` is true and a partial download exists, it will be
    /// cleaned up and a fresh download will be started. Full resume support
    /// requires HTTP Range header support from the server.
    pub async fn start_download_with_resume(
        &mut self,
        try_resume: bool,
    ) -> Result<(), super::download::DownloadError> {
        use super::download::{DownloadProgress, ModelDownloader};

        // Check if already ready
        if self.manager.is_ready() {
            tracing::info!("Model already installed, skipping download");
            self.emit_event(ModelLifecycleEvent::InstallationComplete {
                model_path: self.manager.model_path(),
            });
            return Ok(());
        }

        // Check if currently installing
        if self.manager.is_installing() {
            tracing::warn!("Model download already in progress");
            return Err(super::download::DownloadError::InstallationFailed(
                "Download already in progress".to_string(),
            ));
        }

        // Check for partial download
        if try_resume {
            if let Some(partial) = self.manager.check_partial_download() {
                if partial.can_resume {
                    self.emit_event(ModelLifecycleEvent::DownloadResuming {
                        bytes_already_downloaded: partial.bytes_downloaded,
                    });
                    // For now, we clean up and restart
                    // Full HTTP Range resume would require server support
                    tracing::info!(
                        "Found partial download ({} bytes), cleaning up for fresh download",
                        partial.bytes_downloaded
                    );
                }
                // Clean up the partial file
                let _ = self.manager.cleanup_partial_download();
            }
        } else {
            // Clean up any partial downloads
            let _ = self.manager.cleanup_partial_download();
        }

        // Ensure base directory exists
        self.manager.ensure_directory()?;

        // Transition to downloading state
        self.transition_to(ModelState::Downloading { progress_percent: 0 });

        let download_path = self.manager.download_path();
        let model_path = self.manager.model_path();

        // Create a clone of event sender/callback for the progress callback
        let event_callback = self.event_callback.clone();
        let event_sender = self.event_sender.clone();

        let progress_wrapper = move |progress: DownloadProgress| {
            let event = match &progress {
                DownloadProgress::Starting => Some(ModelLifecycleEvent::StateChanged {
                    from: ModelState::NotInstalled,
                    to: ModelState::Downloading { progress_percent: 0 },
                }),
                DownloadProgress::Downloading { bytes_received, total_bytes } => {
                    let percent = total_bytes
                        .map(|total| if total > 0 { (bytes_received * 100 / total) as u8 } else { 0 })
                        .unwrap_or(0);
                    Some(ModelLifecycleEvent::DownloadProgress {
                        bytes_received: *bytes_received,
                        total_bytes: *total_bytes,
                        percent,
                    })
                }
                DownloadProgress::Verifying => Some(ModelLifecycleEvent::StateChanged {
                    from: ModelState::Downloading { progress_percent: 100 },
                    to: ModelState::Extracting,
                }),
                DownloadProgress::Extracting => Some(ModelLifecycleEvent::StateChanged {
                    from: ModelState::Downloading { progress_percent: 100 },
                    to: ModelState::Extracting,
                }),
                DownloadProgress::Installing => None, // Covered by Extracting
                DownloadProgress::Complete => Some(ModelLifecycleEvent::StateChanged {
                    from: ModelState::Extracting,
                    to: ModelState::Ready,
                }),
                DownloadProgress::Error(e) => Some(ModelLifecycleEvent::Error {
                    message: e.clone(),
                    recoverable: true,
                }),
            };

            if let Some(evt) = event {
                if let Some(cb) = &event_callback {
                    cb(evt.clone());
                }
                if let Some(tx) = &event_sender {
                    let _ = tx.try_send(evt);
                }
            }
        };

        let downloader = ModelDownloader::new();

        // Perform download and installation
        match downloader
            .download_and_install(&download_path, &model_path, progress_wrapper)
            .await
        {
            Ok(()) => {
                self.manager.set_ready();
                self.emit_event(ModelLifecycleEvent::InstallationComplete {
                    model_path: model_path.clone(),
                });
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.manager.set_error(&error_msg);
                self.emit_event(ModelLifecycleEvent::Error {
                    message: error_msg,
                    recoverable: true,
                });
                Err(e)
            }
        }
    }

    /// Cancel an in-progress download.
    ///
    /// Note: This currently just resets the state. The actual download
    /// cancellation would require additional cancellation token support.
    pub fn cancel_download(&mut self) {
        if self.manager.is_installing() {
            let old_state = self.manager.state();
            self.manager.reset_to_not_installed();
            self.manager.set_last_error(Some("Download cancelled".to_string()));

            self.emit_event(ModelLifecycleEvent::StateChanged {
                from: old_state,
                to: ModelState::NotInstalled,
            });

            // Clean up partial files
            let _ = self.manager.cleanup_partial_download();
        }
    }

    /// Retry a failed download.
    pub async fn retry_download(&mut self) -> Result<(), super::download::DownloadError> {
        // Clean up any partial files from the failed attempt
        let _ = self.manager.cleanup_partial_download();

        // Reset state
        self.manager.reset_to_not_installed();
        self.manager.set_last_error(None);

        // Try again
        self.start_download().await
    }

    /// Refresh the model state from disk.
    pub fn refresh_state(&mut self) {
        let old_state = self.manager.state();
        self.manager.refresh_state();
        let new_state = self.manager.state();

        if old_state != new_state {
            self.emit_event(ModelLifecycleEvent::StateChanged {
                from: old_state,
                to: new_state,
            });
        }
    }
}

/// Calculate the total size of a directory in bytes.
fn calculate_dir_size(path: &PathBuf) -> Result<u64, std::io::Error> {
    let mut total_size = 0u64;

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                total_size += calculate_dir_size(&path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_model_state_display() {
        assert_eq!(ModelState::Unknown.to_string(), "Unknown");
        assert_eq!(ModelState::NotInstalled.to_string(), "Not Installed");
        assert_eq!(
            ModelState::Downloading {
                progress_percent: 50
            }
            .to_string(),
            "Downloading (50%)"
        );
        assert_eq!(ModelState::Extracting.to_string(), "Extracting...");
        assert_eq!(ModelState::Ready.to_string(), "Ready");
        assert_eq!(ModelState::Error.to_string(), "Error");
    }

    #[test]
    fn test_model_state_default() {
        assert_eq!(ModelState::default(), ModelState::Unknown);
    }

    #[test]
    fn test_manager_with_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir.clone());

        // Directory doesn't exist, should be NotInstalled
        assert_eq!(manager.state(), ModelState::NotInstalled);
        assert!(!manager.is_ready());
        assert!(!manager.is_installing());
    }

    #[test]
    fn test_manager_with_empty_model_dir() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create empty directory
        fs::create_dir_all(&model_dir).unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);

        // Empty directory should be NotInstalled
        assert_eq!(manager.state(), ModelState::NotInstalled);
    }

    #[test]
    fn test_manager_with_valid_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create model structure with required file
        fs::create_dir_all(model_dir.join("am")).unwrap();
        fs::write(model_dir.join("am").join("final.mdl"), b"model data").unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);

        // Should be Ready
        assert_eq!(manager.state(), ModelState::Ready);
        assert!(manager.is_ready());
    }

    #[test]
    fn test_manager_with_alternative_model_structure() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create alternative model structure
        fs::create_dir_all(&model_dir).unwrap();
        fs::write(model_dir.join("final.mdl"), b"model data").unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);

        // Should be Ready with alternative structure
        assert_eq!(manager.state(), ModelState::Ready);
    }

    #[test]
    fn test_manager_with_corrupted_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create directory with some files but not the required ones
        fs::create_dir_all(&model_dir).unwrap();
        fs::write(model_dir.join("random_file.txt"), b"not a model").unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);

        // Should be Error (corrupted)
        assert_eq!(manager.state(), ModelState::Error);
        assert!(manager.last_error().is_some());
        assert!(manager
            .last_error()
            .unwrap()
            .contains("Missing required model files"));
    }

    #[test]
    fn test_manager_model_path() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir.clone());

        assert_eq!(manager.model_path(), model_dir);
        assert_eq!(manager.base_dir(), &model_dir);
    }

    #[test]
    fn test_manager_download_path() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir);

        let download_path = manager.download_path();
        assert!(download_path.to_string_lossy().ends_with(".zip"));
        assert!(download_path
            .to_string_lossy()
            .contains(DEFAULT_MODEL_NAME));
    }

    #[test]
    fn test_manager_ensure_directory() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir.clone());

        assert!(!model_dir.exists());

        manager.ensure_directory().unwrap();

        assert!(model_dir.exists());
        assert!(model_dir.is_dir());
    }

    #[test]
    fn test_manager_state_transitions() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let mut manager = VoskModelManager::with_base_dir(model_dir);

        // Initial state
        assert_eq!(manager.state(), ModelState::NotInstalled);

        // Transition to downloading
        manager.set_downloading(25);
        assert_eq!(
            manager.state(),
            ModelState::Downloading {
                progress_percent: 25
            }
        );
        assert!(manager.is_installing());

        // Update progress
        manager.set_downloading(75);
        assert_eq!(
            manager.state(),
            ModelState::Downloading {
                progress_percent: 75
            }
        );

        // Progress capped at 100
        manager.set_downloading(150);
        assert_eq!(
            manager.state(),
            ModelState::Downloading {
                progress_percent: 100
            }
        );

        // Transition to extracting
        manager.set_extracting();
        assert_eq!(manager.state(), ModelState::Extracting);
        assert!(manager.is_installing());

        // Transition to ready
        manager.set_ready();
        assert_eq!(manager.state(), ModelState::Ready);
        assert!(manager.is_ready());
        assert!(!manager.is_installing());

        // Transition to error
        manager.set_error("Test error");
        assert_eq!(manager.state(), ModelState::Error);
        assert_eq!(manager.last_error(), Some("Test error"));
    }

    #[test]
    fn test_manager_refresh_preserves_installing_state() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let mut manager = VoskModelManager::with_base_dir(model_dir);

        // Set to downloading
        manager.set_downloading(50);
        assert!(manager.is_installing());

        // Refresh should not override installing state
        manager.refresh_state();
        assert_eq!(
            manager.state(),
            ModelState::Downloading {
                progress_percent: 50
            }
        );
    }

    #[test]
    fn test_manager_model_info() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create valid model
        fs::create_dir_all(model_dir.join("am")).unwrap();
        fs::write(model_dir.join("am").join("final.mdl"), b"model data").unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir.clone());

        let info = manager.model_info();
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.name, DEFAULT_MODEL_NAME);
        assert_eq!(info.path, model_dir);
        assert_eq!(info.language, "en-US");
        assert!(info.size_bytes.is_some());
    }

    #[test]
    fn test_manager_model_info_not_ready() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir);

        // Model not installed, should return None
        assert!(manager.model_info().is_none());
    }

    #[test]
    fn test_manager_delete_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Create valid model
        fs::create_dir_all(model_dir.join("am")).unwrap();
        fs::write(model_dir.join("am").join("final.mdl"), b"model data").unwrap();

        let mut manager = VoskModelManager::with_base_dir(model_dir.clone());

        assert!(manager.is_ready());
        assert!(model_dir.exists());

        // Delete
        manager.delete_model().unwrap();

        assert_eq!(manager.state(), ModelState::NotInstalled);
        assert!(!model_dir.exists());
    }

    #[test]
    fn test_manager_constants() {
        assert!(!DEFAULT_MODEL_NAME.is_empty());
        assert!(DEFAULT_MODEL_URL.starts_with("https://"));
        assert!(DEFAULT_MODEL_URL.ends_with(".zip"));
        assert_eq!(DEFAULT_MODEL_SHA256.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create some files
        fs::write(path.join("file1.txt"), b"hello").unwrap();
        fs::write(path.join("file2.txt"), b"world!").unwrap();
        fs::create_dir(path.join("subdir")).unwrap();
        fs::write(path.join("subdir").join("file3.txt"), b"test").unwrap();

        let size = calculate_dir_size(&path).unwrap();
        // 5 + 6 + 4 = 15 bytes
        assert_eq!(size, 15);
    }

    // ==========================================================================
    // ModelLifecycleEvent tests
    // ==========================================================================

    #[test]
    fn test_lifecycle_event_is_error() {
        let event = ModelLifecycleEvent::Error {
            message: "test error".to_string(),
            recoverable: true,
        };
        assert!(event.is_error());

        let event = ModelLifecycleEvent::DownloadProgress {
            bytes_received: 100,
            total_bytes: Some(1000),
            percent: 10,
        };
        assert!(!event.is_error());
    }

    #[test]
    fn test_lifecycle_event_is_complete() {
        let temp_dir = TempDir::new().unwrap();
        let event = ModelLifecycleEvent::InstallationComplete {
            model_path: temp_dir.path().to_path_buf(),
        };
        assert!(event.is_complete());

        let event = ModelLifecycleEvent::DownloadProgress {
            bytes_received: 100,
            total_bytes: Some(1000),
            percent: 10,
        };
        assert!(!event.is_complete());
    }

    #[test]
    fn test_lifecycle_event_progress_percent() {
        // Download progress event
        let event = ModelLifecycleEvent::DownloadProgress {
            bytes_received: 500,
            total_bytes: Some(1000),
            percent: 50,
        };
        assert_eq!(event.progress_percent(), Some(50));

        // State change to downloading
        let event = ModelLifecycleEvent::StateChanged {
            from: ModelState::NotInstalled,
            to: ModelState::Downloading { progress_percent: 25 },
        };
        assert_eq!(event.progress_percent(), Some(25));

        // State change to extracting
        let event = ModelLifecycleEvent::StateChanged {
            from: ModelState::Downloading { progress_percent: 100 },
            to: ModelState::Extracting,
        };
        assert_eq!(event.progress_percent(), Some(95));

        // State change to ready
        let event = ModelLifecycleEvent::StateChanged {
            from: ModelState::Extracting,
            to: ModelState::Ready,
        };
        assert_eq!(event.progress_percent(), Some(100));

        // Installation complete
        let temp_dir = TempDir::new().unwrap();
        let event = ModelLifecycleEvent::InstallationComplete {
            model_path: temp_dir.path().to_path_buf(),
        };
        assert_eq!(event.progress_percent(), Some(100));

        // Error - no progress
        let event = ModelLifecycleEvent::Error {
            message: "test".to_string(),
            recoverable: false,
        };
        assert_eq!(event.progress_percent(), None);
    }

    #[test]
    fn test_lifecycle_event_status_message() {
        // Download progress with known total
        let event = ModelLifecycleEvent::DownloadProgress {
            bytes_received: 1048576, // 1 MB
            total_bytes: Some(52428800), // 50 MB
            percent: 2,
        };
        let msg = event.status_message();
        assert!(msg.contains("2%"));
        assert!(msg.contains("1.0 MB"));

        // Extraction progress
        let event = ModelLifecycleEvent::ExtractionProgress {
            files_extracted: 10,
            total_files: Some(100),
        };
        let msg = event.status_message();
        assert!(msg.contains("10/100"));

        // Resuming download
        let event = ModelLifecycleEvent::DownloadResuming {
            bytes_already_downloaded: 5242880, // 5 MB
        };
        let msg = event.status_message();
        assert!(msg.contains("5.0 MB"));

        // Error
        let event = ModelLifecycleEvent::Error {
            message: "Connection failed".to_string(),
            recoverable: true,
        };
        let msg = event.status_message();
        assert!(msg.contains("Connection failed"));
    }

    // ==========================================================================
    // Partial download tests
    // ==========================================================================

    #[test]
    fn test_partial_download_path() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir);
        let partial_path = manager.partial_download_path();

        assert!(partial_path.to_string_lossy().ends_with(".zip.partial"));
    }

    #[test]
    fn test_check_partial_download_none() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let manager = VoskModelManager::with_base_dir(model_dir);

        // No partial download file exists
        assert!(manager.check_partial_download().is_none());
    }

    #[test]
    fn test_check_partial_download_exists() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        // Ensure parent directory exists
        fs::create_dir_all(temp_dir.path()).unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);
        let partial_path = manager.partial_download_path();

        // Create a partial download file
        fs::write(&partial_path, vec![0u8; 1000]).unwrap();

        let info = manager.check_partial_download();
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.bytes_downloaded, 1000);
        assert!(info.can_resume);
        assert_eq!(info.path, partial_path);
    }

    #[test]
    fn test_cleanup_partial_download() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        fs::create_dir_all(temp_dir.path()).unwrap();

        let manager = VoskModelManager::with_base_dir(model_dir);
        let partial_path = manager.partial_download_path();

        // Create a partial download file
        fs::write(&partial_path, vec![0u8; 1000]).unwrap();
        assert!(partial_path.exists());

        // Clean up
        manager.cleanup_partial_download().unwrap();
        assert!(!partial_path.exists());
    }

    // ==========================================================================
    // State machine tests
    // ==========================================================================

    #[test]
    fn test_state_machine_with_channel() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let (state_machine, _rx) = ModelLifecycleStateMachine::with_base_dir_and_channel(model_dir);

        assert_eq!(state_machine.state(), ModelState::NotInstalled);
        assert!(!state_machine.is_ready());
        assert!(!state_machine.is_installing());
    }

    #[test]
    fn test_state_machine_with_callback() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let event_count = Arc::new(AtomicU32::new(0));
        let event_count_clone = event_count.clone();

        let callback = Arc::new(move |_event: ModelLifecycleEvent| {
            event_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let state_machine = ModelLifecycleStateMachine::with_base_dir_and_callback(
            model_dir,
            callback,
        );

        assert_eq!(state_machine.state(), ModelState::NotInstalled);
    }

    #[test]
    fn test_state_machine_model_path() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let (state_machine, _rx) = ModelLifecycleStateMachine::with_base_dir_and_channel(model_dir.clone());

        assert_eq!(state_machine.model_path(), model_dir);
    }

    #[test]
    fn test_state_machine_check_partial_download() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        fs::create_dir_all(temp_dir.path()).unwrap();

        let (state_machine, _rx) = ModelLifecycleStateMachine::with_base_dir_and_channel(model_dir);
        let partial_path = state_machine.manager().partial_download_path();

        // Initially no partial download
        assert!(state_machine.check_partial_download().is_none());

        // Create a partial download
        fs::write(&partial_path, vec![0u8; 5000]).unwrap();

        let info = state_machine.check_partial_download();
        assert!(info.is_some());
        assert_eq!(info.unwrap().bytes_downloaded, 5000);
    }

    #[test]
    fn test_state_machine_refresh_state() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let (mut state_machine, _rx) = ModelLifecycleStateMachine::with_base_dir_and_channel(model_dir.clone());

        // Initially not installed
        assert_eq!(state_machine.state(), ModelState::NotInstalled);

        // Create valid model structure
        fs::create_dir_all(model_dir.join("am")).unwrap();
        fs::write(model_dir.join("am").join("final.mdl"), b"model data").unwrap();

        // Refresh should detect the model
        state_machine.refresh_state();
        assert_eq!(state_machine.state(), ModelState::Ready);
    }

    #[test]
    fn test_state_machine_cancel_not_installing() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let (mut state_machine, _rx) = ModelLifecycleStateMachine::with_base_dir_and_channel(model_dir);

        // Not installing, cancel should do nothing
        state_machine.cancel_download();
        assert_eq!(state_machine.state(), ModelState::NotInstalled);
    }

    #[test]
    fn test_manager_reset_to_not_installed() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let mut manager = VoskModelManager::with_base_dir(model_dir);

        // Set to downloading
        manager.set_downloading(50);
        assert!(manager.is_installing());

        // Reset
        manager.reset_to_not_installed();
        assert_eq!(manager.state(), ModelState::NotInstalled);
        assert!(!manager.is_installing());
    }

    #[test]
    fn test_manager_set_last_error() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("vosk-model");

        let mut manager = VoskModelManager::with_base_dir(model_dir);

        // Set error message
        manager.set_last_error(Some("Test error".to_string()));
        assert_eq!(manager.last_error(), Some("Test error"));

        // Clear error message
        manager.set_last_error(None);
        assert!(manager.last_error().is_none());
    }
}
