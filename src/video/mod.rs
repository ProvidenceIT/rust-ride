//! Video Playback & Sync Module
//!
//! Provides synchronized video playback for scenic rides.

pub mod player;
pub mod sync;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;

// Re-export main types
pub use player::{VideoFrame, VideoInfo, VideoPlayer, VideoTextureHandle, VideoTextureManager};
pub use sync::{SyncPoint, VideoSync, VideoSyncController};

/// Video-related errors
#[derive(Debug, Error)]
pub enum VideoError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Decode error: {0}")]
    DecodeError(String),

    #[error("No video loaded")]
    NoVideoLoaded,

    #[error("Seek failed: {0}")]
    SeekFailed(String),

    #[error("FFmpeg error: {0}")]
    FfmpegError(String),

    #[error("Route not compatible with video")]
    RouteVideoMismatch,
}

/// Video configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    /// Whether video is enabled
    pub enabled: bool,
    /// Default volume (0-100)
    pub default_volume: u8,
    /// Auto-pause video when rider stops
    pub auto_pause_on_stop: bool,
    /// Show debug overlay with sync info
    pub show_debug_overlay: bool,
    /// Preferred resolution
    pub preferred_resolution: VideoResolutionPreference,
    /// Use hardware acceleration if available
    pub hardware_acceleration: bool,
    /// Buffer size in frames
    pub buffer_frames: u8,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_volume: 50,
            auto_pause_on_stop: true,
            show_debug_overlay: false,
            preferred_resolution: VideoResolutionPreference::Native,
            hardware_acceleration: true,
            buffer_frames: 5,
        }
    }
}

/// Resolution preference for video playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoResolutionPreference {
    /// Use native video resolution
    Native,
    /// Limit to 720p
    Limit720p,
    /// Limit to 1080p
    Limit1080p,
    /// Limit to 4K
    Limit4k,
}

impl VideoResolutionPreference {
    /// Get maximum height for this preference
    pub fn max_height(&self) -> Option<u32> {
        match self {
            VideoResolutionPreference::Native => None,
            VideoResolutionPreference::Limit720p => Some(720),
            VideoResolutionPreference::Limit1080p => Some(1080),
            VideoResolutionPreference::Limit4k => Some(2160),
        }
    }
}

/// Video player events
#[derive(Debug, Clone)]
pub enum VideoPlayerEvent {
    /// Video loaded successfully
    Loaded(VideoInfo),
    /// Playback started
    PlaybackStarted,
    /// Playback paused
    PlaybackPaused,
    /// Playback stopped
    PlaybackStopped,
    /// Position changed
    PositionChanged(Duration),
    /// Speed changed
    SpeedChanged(f32),
    /// End of video reached
    EndReached,
    /// Error occurred
    Error(String),
}

/// Supported video containers
pub const SUPPORTED_CONTAINERS: &[&str] = &["mp4", "mkv", "webm", "avi", "mov"];

/// Supported video codecs
pub const SUPPORTED_CODECS: &[&str] = &["h264", "hevc", "vp9", "av1"];

/// Check if a file is supported
pub fn is_supported(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_CONTAINERS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Speed range for playback
pub const MIN_PLAYBACK_SPEED: f32 = 0.5;
pub const MAX_PLAYBACK_SPEED: f32 = 2.0;
pub const PAUSE_THRESHOLD_KMH: f32 = 5.0;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_config_default() {
        let config = VideoConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_volume, 50);
        assert!(config.auto_pause_on_stop);
    }

    #[test]
    fn test_resolution_preference() {
        assert!(VideoResolutionPreference::Native.max_height().is_none());
        assert_eq!(VideoResolutionPreference::Limit720p.max_height(), Some(720));
        assert_eq!(
            VideoResolutionPreference::Limit1080p.max_height(),
            Some(1080)
        );
    }

    #[test]
    fn test_is_supported() {
        assert!(is_supported(Path::new("video.mp4")));
        assert!(is_supported(Path::new("video.MKV")));
        assert!(!is_supported(Path::new("video.txt")));
        assert!(!is_supported(Path::new("video")));
    }
}
