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

use std::path::PathBuf;
use thiserror::Error;

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

        self.state = ModelState::NotInstalled;
        self.last_error = None;

        Ok(())
    }
}

impl Default for VoskModelManager {
    fn default() -> Self {
        Self::new()
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
}
