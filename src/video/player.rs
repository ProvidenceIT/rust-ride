//! Video Player
//!
//! Handles video decoding and playback.

use super::{VideoConfig, VideoError, VideoPlayerEvent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

/// Information about a loaded video
#[derive(Debug, Clone)]
pub struct VideoInfo {
    /// File path
    pub path: PathBuf,
    /// Total duration
    pub duration: Duration,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Frame rate (fps)
    pub frame_rate: f32,
    /// Video codec
    pub codec: String,
    /// Whether video has audio track
    pub has_audio: bool,
    /// Total number of frames
    pub total_frames: u64,
}

/// A decoded video frame
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// RGBA pixel data
    pub data: Vec<u8>,
    /// Timestamp of this frame
    pub timestamp: Duration,
    /// Frame number
    pub frame_number: u64,
}

impl VideoFrame {
    /// Get frame data as a slice of RGBA pixels
    pub fn as_rgba(&self) -> &[u8] {
        &self.data
    }

    /// Expected data size for this frame
    pub fn expected_size(&self) -> usize {
        (self.width * self.height * 4) as usize
    }

    /// Convert video frame to egui ColorImage for texture upload
    pub fn to_color_image(&self) -> egui::ColorImage {
        egui::ColorImage::from_rgba_unmultiplied(
            [self.width as usize, self.height as usize],
            &self.data,
        )
    }
}

/// Handle for an egui texture created from video frames
#[derive(Clone)]
pub struct VideoTextureHandle {
    texture_id: egui::TextureId,
    size: egui::Vec2,
}

impl VideoTextureHandle {
    /// Create from an egui texture handle
    pub fn from_texture(handle: egui::TextureHandle) -> Self {
        let size = handle.size_vec2();
        Self {
            texture_id: handle.id(),
            size,
        }
    }

    /// Get the texture ID for rendering
    pub fn texture_id(&self) -> egui::TextureId {
        self.texture_id
    }

    /// Get the texture size
    pub fn size(&self) -> egui::Vec2 {
        self.size
    }
}

/// Helper to manage video texture updates in egui
pub struct VideoTextureManager {
    texture_handle: Option<egui::TextureHandle>,
    last_frame_number: u64,
}

impl VideoTextureManager {
    /// Create a new video texture manager
    pub fn new() -> Self {
        Self {
            texture_handle: None,
            last_frame_number: 0,
        }
    }

    /// Update texture with new frame if needed
    /// Returns true if texture was updated
    pub fn update_frame(&mut self, ctx: &egui::Context, frame: &VideoFrame) -> bool {
        // Skip if same frame
        if self.last_frame_number == frame.frame_number && self.texture_handle.is_some() {
            return false;
        }

        let image = frame.to_color_image();

        if let Some(ref mut handle) = self.texture_handle {
            // Update existing texture
            handle.set(image, egui::TextureOptions::LINEAR);
        } else {
            // Create new texture
            self.texture_handle =
                Some(ctx.load_texture("video_frame", image, egui::TextureOptions::LINEAR));
        }

        self.last_frame_number = frame.frame_number;
        true
    }

    /// Get the current texture handle for rendering
    pub fn get_handle(&self) -> Option<VideoTextureHandle> {
        self.texture_handle
            .as_ref()
            .map(|h| VideoTextureHandle::from_texture(h.clone()))
    }

    /// Clear the texture
    pub fn clear(&mut self) {
        self.texture_handle = None;
        self.last_frame_number = 0;
    }

    /// Check if a texture is loaded
    pub fn is_loaded(&self) -> bool {
        self.texture_handle.is_some()
    }
}

impl Default for VideoTextureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for video player implementations
pub trait VideoPlayer: Send + Sync {
    /// Load a video file
    fn load(
        &self,
        path: &Path,
    ) -> impl std::future::Future<Output = Result<VideoInfo, VideoError>> + Send;

    /// Unload current video
    fn unload(&self);

    /// Check if video is loaded
    fn is_loaded(&self) -> bool;

    /// Get video info
    fn get_info(&self) -> Option<VideoInfo>;

    /// Start playback
    fn play(&self);

    /// Pause playback
    fn pause(&self);

    /// Check if playing
    fn is_playing(&self) -> bool;

    /// Set playback speed (0.5 - 2.0)
    fn set_speed(&self, speed: f32);

    /// Get current playback speed
    fn get_speed(&self) -> f32;

    /// Seek to position
    fn seek(&self, position: Duration);

    /// Get current position
    fn get_position(&self) -> Duration;

    /// Get total duration
    fn get_duration(&self) -> Duration;

    /// Get current frame for rendering
    fn get_frame(&self) -> Option<VideoFrame>;

    /// Set volume (0-100)
    fn set_volume(&self, volume: u8);

    /// Get current volume
    fn get_volume(&self) -> u8;

    /// Subscribe to player events
    fn subscribe_events(&self) -> broadcast::Receiver<VideoPlayerEvent>;
}

/// Playback state
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum PlaybackState {
    Idle,
    Loading,
    Playing,
    Paused,
    Stopped,
}

/// Default video player implementation
pub struct DefaultVideoPlayer {
    /// Video configuration (reserved for future use)
    #[allow(dead_code)]
    config: VideoConfig,
    state: Arc<RwLock<PlaybackState>>,
    info: Arc<RwLock<Option<VideoInfo>>>,
    position: Arc<RwLock<Duration>>,
    speed: Arc<RwLock<f32>>,
    volume: Arc<RwLock<u8>>,
    event_tx: broadcast::Sender<VideoPlayerEvent>,
    current_frame: Arc<RwLock<Option<VideoFrame>>>,
}

impl DefaultVideoPlayer {
    /// Create a new video player
    pub fn new(config: VideoConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(PlaybackState::Idle)),
            info: Arc::new(RwLock::new(None)),
            position: Arc::new(RwLock::new(Duration::ZERO)),
            speed: Arc::new(RwLock::new(1.0)),
            volume: Arc::new(RwLock::new(config.default_volume)),
            event_tx,
            current_frame: Arc::new(RwLock::new(None)),
        }
    }
}

impl VideoPlayer for DefaultVideoPlayer {
    async fn load(&self, path: &Path) -> Result<VideoInfo, VideoError> {
        if !path.exists() {
            return Err(VideoError::FileNotFound(path.to_path_buf()));
        }

        if !super::is_supported(path) {
            return Err(VideoError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ));
        }

        *self.state.write().await = PlaybackState::Loading;

        tracing::info!("Loading video: {:?}", path);

        // TODO: Use ffmpeg-next to actually load the video
        // For now, return mock info
        let info = VideoInfo {
            path: path.to_path_buf(),
            duration: Duration::from_secs(3600), // 1 hour placeholder
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
            codec: "h264".to_string(),
            has_audio: true,
            total_frames: 108000,
        };

        *self.info.write().await = Some(info.clone());
        *self.state.write().await = PlaybackState::Paused;
        *self.position.write().await = Duration::ZERO;

        let _ = self.event_tx.send(VideoPlayerEvent::Loaded(info.clone()));

        Ok(info)
    }

    fn unload(&self) {
        if let Ok(mut state) = self.state.try_write() {
            *state = PlaybackState::Idle;
        }
        if let Ok(mut info) = self.info.try_write() {
            *info = None;
        }
        if let Ok(mut position) = self.position.try_write() {
            *position = Duration::ZERO;
        }
        if let Ok(mut frame) = self.current_frame.try_write() {
            *frame = None;
        }

        let _ = self.event_tx.send(VideoPlayerEvent::PlaybackStopped);

        tracing::info!("Video unloaded");
    }

    fn is_loaded(&self) -> bool {
        self.info.try_read().map(|i| i.is_some()).unwrap_or(false)
    }

    fn get_info(&self) -> Option<VideoInfo> {
        self.info.try_read().ok()?.clone()
    }

    fn play(&self) {
        if let Ok(mut state) = self.state.try_write() {
            if matches!(*state, PlaybackState::Paused) {
                *state = PlaybackState::Playing;
                let _ = self.event_tx.send(VideoPlayerEvent::PlaybackStarted);
                tracing::debug!("Video playback started");
            }
        }
    }

    fn pause(&self) {
        if let Ok(mut state) = self.state.try_write() {
            if matches!(*state, PlaybackState::Playing) {
                *state = PlaybackState::Paused;
                let _ = self.event_tx.send(VideoPlayerEvent::PlaybackPaused);
                tracing::debug!("Video playback paused");
            }
        }
    }

    fn is_playing(&self) -> bool {
        self.state
            .try_read()
            .map(|s| matches!(*s, PlaybackState::Playing))
            .unwrap_or(false)
    }

    fn set_speed(&self, speed: f32) {
        let clamped = speed.clamp(super::MIN_PLAYBACK_SPEED, super::MAX_PLAYBACK_SPEED);
        if let Ok(mut s) = self.speed.try_write() {
            *s = clamped;
            let _ = self.event_tx.send(VideoPlayerEvent::SpeedChanged(clamped));
        }
    }

    fn get_speed(&self) -> f32 {
        self.speed.try_read().map(|s| *s).unwrap_or(1.0)
    }

    fn seek(&self, position: Duration) {
        if let Some(info) = self.get_info() {
            let clamped = if position > info.duration {
                info.duration
            } else {
                position
            };

            if let Ok(mut pos) = self.position.try_write() {
                *pos = clamped;
                let _ = self
                    .event_tx
                    .send(VideoPlayerEvent::PositionChanged(clamped));
                tracing::debug!("Seek to {:?}", clamped);
            }
        }
    }

    fn get_position(&self) -> Duration {
        self.position
            .try_read()
            .map(|p| *p)
            .unwrap_or(Duration::ZERO)
    }

    fn get_duration(&self) -> Duration {
        self.get_info()
            .map(|i| i.duration)
            .unwrap_or(Duration::ZERO)
    }

    fn get_frame(&self) -> Option<VideoFrame> {
        self.current_frame.try_read().ok()?.clone()
    }

    fn set_volume(&self, volume: u8) {
        if let Ok(mut v) = self.volume.try_write() {
            *v = volume.min(100);
        }
    }

    fn get_volume(&self) -> u8 {
        self.volume.try_read().map(|v| *v).unwrap_or(50)
    }

    fn subscribe_events(&self) -> broadcast::Receiver<VideoPlayerEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let config = VideoConfig::default();
        let player = DefaultVideoPlayer::new(config);

        assert!(!player.is_loaded());
        assert!(!player.is_playing());
        assert_eq!(player.get_speed(), 1.0);
        assert_eq!(player.get_volume(), 50);
    }

    #[test]
    fn test_speed_clamping() {
        let config = VideoConfig::default();
        let player = DefaultVideoPlayer::new(config);

        player.set_speed(3.0);
        assert_eq!(player.get_speed(), super::super::MAX_PLAYBACK_SPEED);

        player.set_speed(0.1);
        assert_eq!(player.get_speed(), super::super::MIN_PLAYBACK_SPEED);
    }

    #[test]
    fn test_video_frame() {
        let frame = VideoFrame {
            width: 1920,
            height: 1080,
            data: vec![0u8; 1920 * 1080 * 4],
            timestamp: Duration::from_secs(1),
            frame_number: 30,
        };

        assert_eq!(frame.expected_size(), 1920 * 1080 * 4);
        assert_eq!(frame.as_rgba().len(), frame.expected_size());
    }
}
