//! Async model download with progress tracking, verification, and extraction.
//!
//! This module handles the complete lifecycle of downloading and installing
//! the Vosk speech recognition model:
//!
//! 1. **Download**: Async streaming download with progress callbacks
//! 2. **Verification**: SHA256 hash verification of downloaded file
//! 3. **Extraction**: Zip extraction to temporary location
//! 4. **Installation**: Atomic move to final model directory
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustride::voice::{ModelDownloader, DownloadProgress};
//!
//! let downloader = ModelDownloader::new();
//!
//! // Download with progress callback
//! downloader.download_model(|progress| {
//!     match progress {
//!         DownloadProgress::Downloading { bytes_received, total_bytes } => {
//!             let percent = bytes_received * 100 / total_bytes.unwrap_or(1);
//!             println!("Downloading: {}%", percent);
//!         }
//!         DownloadProgress::Verifying => println!("Verifying checksum..."),
//!         DownloadProgress::Extracting => println!("Extracting model..."),
//!         DownloadProgress::Complete => println!("Model ready!"),
//!         DownloadProgress::Error(e) => eprintln!("Failed: {}", e),
//!     }
//! }).await?;
//! ```

use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::mpsc;

use super::model_manager::{DEFAULT_MODEL_SHA256, DEFAULT_MODEL_URL, VoskModelError};

/// Size of the read buffer for downloads (64KB).
const DOWNLOAD_BUFFER_SIZE: usize = 64 * 1024;

/// Errors that can occur during model download and installation.
#[derive(Debug, Error)]
pub enum DownloadError {
    /// Network or HTTP error during download.
    #[error("Download failed: {0}")]
    NetworkError(String),

    /// The downloaded file failed SHA256 verification.
    #[error("Checksum verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Failed to create required directories.
    #[error("Failed to create directory: {0}")]
    DirectoryCreationFailed(String),

    /// Failed to write downloaded data to disk.
    #[error("Failed to write file: {0}")]
    WriteFailed(String),

    /// Failed to extract the zip archive.
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    /// Failed to move extracted files to final location.
    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    /// The download was cancelled.
    #[error("Download cancelled")]
    Cancelled,

    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Model manager error.
    #[error("Model manager error: {0}")]
    ModelError(#[from] VoskModelError),
}

/// Progress updates during model download and installation.
#[derive(Debug, Clone)]
pub enum DownloadProgress {
    /// Starting the download.
    Starting,

    /// Download is in progress.
    Downloading {
        /// Bytes received so far.
        bytes_received: u64,
        /// Total bytes expected (if known from Content-Length header).
        total_bytes: Option<u64>,
    },

    /// Verifying the downloaded file's checksum.
    Verifying,

    /// Extracting the zip archive.
    Extracting,

    /// Moving files to final location.
    Installing,

    /// Download and installation completed successfully.
    Complete,

    /// An error occurred.
    Error(String),
}

impl DownloadProgress {
    /// Get the progress percentage (0-100) if available.
    pub fn percent(&self) -> Option<u8> {
        match self {
            DownloadProgress::Downloading {
                bytes_received,
                total_bytes: Some(total),
            } if *total > 0 => Some(((bytes_received * 100) / total) as u8),
            DownloadProgress::Complete => Some(100),
            DownloadProgress::Starting => Some(0),
            _ => None,
        }
    }
}

/// A callback type for receiving download progress updates.
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Handles downloading, verifying, and installing Vosk models.
pub struct ModelDownloader {
    /// HTTP client for downloads.
    client: reqwest::Client,

    /// URL to download the model from.
    download_url: String,

    /// Expected SHA256 hash of the downloaded file.
    expected_sha256: String,
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelDownloader {
    /// Create a new model downloader with default settings.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600)) // 10 minute timeout for large downloads
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            download_url: DEFAULT_MODEL_URL.to_string(),
            expected_sha256: DEFAULT_MODEL_SHA256.to_string(),
        }
    }

    /// Create a model downloader with custom URL and hash.
    ///
    /// This is primarily useful for testing or using alternative models.
    pub fn with_url_and_hash(url: impl Into<String>, sha256: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            download_url: url.into(),
            expected_sha256: sha256.into(),
        }
    }

    /// Download the model to the specified path.
    ///
    /// Returns the path to the downloaded zip file.
    pub async fn download_to_file(
        &self,
        download_path: &Path,
        progress_callback: impl Fn(DownloadProgress) + Send + Sync,
    ) -> Result<(), DownloadError> {
        // Ensure parent directory exists
        if let Some(parent) = download_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DownloadError::DirectoryCreationFailed(format!(
                    "Failed to create {:?}: {}",
                    parent, e
                ))
            })?;
        }

        progress_callback(DownloadProgress::Starting);

        // Start the download
        tracing::info!("Downloading model from: {}", self.download_url);

        let response = self
            .client
            .get(&self.download_url)
            .send()
            .await
            .map_err(|e| DownloadError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DownloadError::NetworkError(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Get content length if available
        let total_bytes = response.content_length();
        tracing::debug!("Content-Length: {:?}", total_bytes);

        // Create temporary file for atomic download
        let temp_path = download_path.with_extension("zip.tmp");
        let mut file = std::fs::File::create(&temp_path).map_err(|e| {
            DownloadError::WriteFailed(format!("Failed to create {:?}: {}", temp_path, e))
        })?;

        // Stream the response body to file
        let mut bytes_received: u64 = 0;
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| DownloadError::NetworkError(e.to_string()))?;

            file.write_all(&chunk).map_err(|e| {
                DownloadError::WriteFailed(format!("Failed to write chunk: {}", e))
            })?;

            bytes_received += chunk.len() as u64;

            progress_callback(DownloadProgress::Downloading {
                bytes_received,
                total_bytes,
            });
        }

        // Flush and sync to ensure data is on disk
        file.flush()?;
        file.sync_all()?;
        drop(file);

        tracing::info!("Download complete: {} bytes", bytes_received);

        // Atomically rename temp file to final location
        std::fs::rename(&temp_path, download_path).map_err(|e| {
            // Clean up temp file on failure
            let _ = std::fs::remove_file(&temp_path);
            DownloadError::WriteFailed(format!(
                "Failed to rename {:?} to {:?}: {}",
                temp_path, download_path, e
            ))
        })?;

        Ok(())
    }

    /// Verify the SHA256 checksum of a downloaded file.
    pub fn verify_checksum(&self, file_path: &Path) -> Result<(), DownloadError> {
        tracing::info!("Verifying checksum of {:?}", file_path);

        let file = std::fs::File::open(file_path)?;
        let mut reader = BufReader::with_capacity(DOWNLOAD_BUFFER_SIZE, file);
        let mut hasher = Sha256::new();

        let mut buffer = [0u8; DOWNLOAD_BUFFER_SIZE];
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        let actual_hash = hex::encode(result);

        if actual_hash.to_lowercase() != self.expected_sha256.to_lowercase() {
            tracing::error!(
                "Checksum mismatch: expected {}, got {}",
                self.expected_sha256,
                actual_hash
            );
            return Err(DownloadError::ChecksumMismatch {
                expected: self.expected_sha256.clone(),
                actual: actual_hash,
            });
        }

        tracing::info!("Checksum verified successfully");
        Ok(())
    }

    /// Extract the zip file to a temporary directory.
    ///
    /// Returns the path to the extracted model directory.
    pub fn extract_zip(&self, zip_path: &Path, extract_dir: &Path) -> Result<PathBuf, DownloadError> {
        tracing::info!("Extracting {:?} to {:?}", zip_path, extract_dir);

        // Create extraction directory
        std::fs::create_dir_all(extract_dir).map_err(|e| {
            DownloadError::DirectoryCreationFailed(format!(
                "Failed to create {:?}: {}",
                extract_dir, e
            ))
        })?;

        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| DownloadError::ExtractionFailed(format!("Failed to open zip: {}", e)))?;

        // Track the root directory name from the archive
        let mut root_dir_name: Option<String> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                DownloadError::ExtractionFailed(format!("Failed to read zip entry {}: {}", i, e))
            })?;

            // Get the file path, handling potential path traversal
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => {
                    tracing::warn!("Skipping unsafe path in zip: {:?}", file.name());
                    continue;
                }
            };

            // Track the root directory
            if root_dir_name.is_none() {
                if let Some(first_component) = file_path.components().next() {
                    if let std::path::Component::Normal(name) = first_component {
                        root_dir_name = Some(name.to_string_lossy().to_string());
                    }
                }
            }

            let outpath = extract_dir.join(&file_path);

            if file.name().ends_with('/') {
                // Directory entry
                std::fs::create_dir_all(&outpath)?;
            } else {
                // File entry
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }

                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }

        // Determine the extracted model directory
        let model_dir = if let Some(root_name) = root_dir_name {
            extract_dir.join(root_name)
        } else {
            extract_dir.to_path_buf()
        };

        tracing::info!("Extraction complete: {:?}", model_dir);
        Ok(model_dir)
    }

    /// Move extracted model to final location atomically.
    ///
    /// If the destination already exists, it will be removed first.
    pub fn install_model(
        &self,
        extracted_dir: &Path,
        final_dir: &Path,
    ) -> Result<(), DownloadError> {
        tracing::info!(
            "Installing model from {:?} to {:?}",
            extracted_dir,
            final_dir
        );

        // Remove existing model if present
        if final_dir.exists() {
            tracing::debug!("Removing existing model at {:?}", final_dir);
            std::fs::remove_dir_all(final_dir).map_err(|e| {
                DownloadError::InstallationFailed(format!(
                    "Failed to remove existing model at {:?}: {}",
                    final_dir, e
                ))
            })?;
        }

        // Ensure parent directory exists
        if let Some(parent) = final_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Try atomic rename first (works if on same filesystem)
        match std::fs::rename(extracted_dir, final_dir) {
            Ok(()) => {
                tracing::info!("Model installed successfully (atomic rename)");
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("Atomic rename failed, falling back to copy: {}", e);
            }
        }

        // Fallback: copy directory tree
        copy_dir_all(extracted_dir, final_dir).map_err(|e| {
            DownloadError::InstallationFailed(format!("Failed to copy model files: {}", e))
        })?;

        // Remove the source directory after successful copy
        let _ = std::fs::remove_dir_all(extracted_dir);

        tracing::info!("Model installed successfully (copy)");
        Ok(())
    }

    /// Complete download, verification, extraction, and installation flow.
    ///
    /// This is the main entry point for downloading and installing a model.
    pub async fn download_and_install(
        &self,
        download_path: &Path,
        model_dir: &Path,
        progress_callback: impl Fn(DownloadProgress) + Send + Sync,
    ) -> Result<(), DownloadError> {
        // Download
        self.download_to_file(download_path, &progress_callback)
            .await?;

        // Verify
        progress_callback(DownloadProgress::Verifying);
        self.verify_checksum(download_path)?;

        // Extract to temporary location
        progress_callback(DownloadProgress::Extracting);
        let temp_extract_dir = download_path.with_extension("extract");
        let extracted_model = self.extract_zip(download_path, &temp_extract_dir)?;

        // Install (atomic move)
        progress_callback(DownloadProgress::Installing);
        self.install_model(&extracted_model, model_dir)?;

        // Clean up
        let _ = std::fs::remove_file(download_path);
        let _ = std::fs::remove_dir_all(&temp_extract_dir);

        progress_callback(DownloadProgress::Complete);
        Ok(())
    }

    /// Create a channel-based progress receiver for async progress tracking.
    ///
    /// Returns a callback that sends progress to the channel and the receiver.
    pub fn create_progress_channel() -> (Arc<impl Fn(DownloadProgress) + Send + Sync>, mpsc::Receiver<DownloadProgress>) {
        let (tx, rx) = mpsc::channel::<DownloadProgress>(100);
        let callback = Arc::new(move |progress: DownloadProgress| {
            let _ = tx.try_send(progress);
        });
        (callback, rx)
    }
}

/// Recursively copy a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Convert bytes to a human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_download_progress_percent() {
        let progress = DownloadProgress::Downloading {
            bytes_received: 50,
            total_bytes: Some(100),
        };
        assert_eq!(progress.percent(), Some(50));

        let progress = DownloadProgress::Downloading {
            bytes_received: 75,
            total_bytes: Some(100),
        };
        assert_eq!(progress.percent(), Some(75));

        let progress = DownloadProgress::Downloading {
            bytes_received: 100,
            total_bytes: None,
        };
        assert_eq!(progress.percent(), None);

        let progress = DownloadProgress::Complete;
        assert_eq!(progress.percent(), Some(100));

        let progress = DownloadProgress::Starting;
        assert_eq!(progress.percent(), Some(0));

        let progress = DownloadProgress::Verifying;
        assert_eq!(progress.percent(), None);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(52428800), "50.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_downloader_creation() {
        let downloader = ModelDownloader::new();
        assert!(downloader.download_url.contains("vosk"));
        assert!(!downloader.expected_sha256.is_empty());
    }

    #[test]
    fn test_downloader_custom_url() {
        let downloader = ModelDownloader::with_url_and_hash(
            "https://example.com/model.zip",
            "abc123",
        );
        assert_eq!(downloader.download_url, "https://example.com/model.zip");
        assert_eq!(downloader.expected_sha256, "abc123");
    }

    #[test]
    fn test_copy_dir_all() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        // Create source structure
        std::fs::create_dir_all(src.join("subdir")).unwrap();
        std::fs::write(src.join("file1.txt"), b"hello").unwrap();
        std::fs::write(src.join("subdir").join("file2.txt"), b"world").unwrap();

        // Copy
        copy_dir_all(&src, &dst).unwrap();

        // Verify
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("subdir").join("file2.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dst.join("file1.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("subdir").join("file2.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn test_verify_checksum_failure() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");

        // Create a file with known content
        std::fs::write(&file_path, b"test content").unwrap();

        // Create downloader with wrong hash
        let downloader = ModelDownloader::with_url_and_hash(
            "https://example.com/model.zip",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );

        // Verify should fail
        let result = downloader.verify_checksum(&file_path);
        assert!(matches!(result, Err(DownloadError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_verify_checksum_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");

        // Create a file with known content
        let content = b"test content for sha256";
        std::fs::write(&file_path, content).unwrap();

        // Calculate expected hash
        let mut hasher = Sha256::new();
        hasher.update(content);
        let expected_hash = hex::encode(hasher.finalize());

        // Create downloader with correct hash
        let downloader = ModelDownloader::with_url_and_hash(
            "https://example.com/model.zip",
            &expected_hash,
        );

        // Verify should succeed
        let result = downloader.verify_checksum(&file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_install_model_atomic_rename() {
        let temp_dir = TempDir::new().unwrap();
        let extracted = temp_dir.path().join("extracted");
        let final_dir = temp_dir.path().join("final");

        // Create extracted model structure
        std::fs::create_dir_all(extracted.join("am")).unwrap();
        std::fs::write(extracted.join("am").join("final.mdl"), b"model").unwrap();

        let downloader = ModelDownloader::new();

        // Install
        let result = downloader.install_model(&extracted, &final_dir);
        assert!(result.is_ok());

        // Verify
        assert!(final_dir.join("am").join("final.mdl").exists());
        assert!(!extracted.exists()); // Source should be gone (renamed)
    }

    #[test]
    fn test_install_model_replaces_existing() {
        let temp_dir = TempDir::new().unwrap();
        let extracted = temp_dir.path().join("extracted");
        let final_dir = temp_dir.path().join("final");

        // Create existing model
        std::fs::create_dir_all(final_dir.join("old")).unwrap();
        std::fs::write(final_dir.join("old").join("old_file.txt"), b"old").unwrap();

        // Create new model
        std::fs::create_dir_all(extracted.join("am")).unwrap();
        std::fs::write(extracted.join("am").join("final.mdl"), b"new").unwrap();

        let downloader = ModelDownloader::new();

        // Install
        let result = downloader.install_model(&extracted, &final_dir);
        assert!(result.is_ok());

        // Verify old content is gone, new content is there
        assert!(!final_dir.join("old").exists());
        assert!(final_dir.join("am").join("final.mdl").exists());
    }

    #[test]
    fn test_progress_channel() {
        let (callback, mut rx) = ModelDownloader::create_progress_channel();

        // Send some progress
        callback(DownloadProgress::Starting);
        callback(DownloadProgress::Downloading {
            bytes_received: 100,
            total_bytes: Some(1000),
        });

        // Receive and verify
        let progress1 = rx.try_recv().unwrap();
        assert!(matches!(progress1, DownloadProgress::Starting));

        let progress2 = rx.try_recv().unwrap();
        assert!(matches!(
            progress2,
            DownloadProgress::Downloading { bytes_received: 100, .. }
        ));
    }
}
