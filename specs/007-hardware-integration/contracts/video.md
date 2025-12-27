# Contract: Video Module

**Module**: `src/video/`
**Feature**: Hardware Integration
**Date**: 2025-12-26

This contract defines the video course sync API for synchronized scenic ride video playback.

---

## Video Player (`src/video/player.rs`)

```rust
/// Video playback engine
pub trait VideoPlayer: Send + Sync {
    /// Load a video file
    async fn load(&self, path: &Path) -> Result<VideoInfo, VideoError>;

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

    /// Subscribe to player events
    fn subscribe_events(&self) -> broadcast::Receiver<VideoPlayerEvent>;
}

pub struct VideoInfo {
    pub path: PathBuf,
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub codec: String,
    pub has_audio: bool,
}

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,     // RGBA pixel data
    pub timestamp: Duration,
}

pub enum VideoPlayerEvent {
    Loaded(VideoInfo),
    PlaybackStarted,
    PlaybackPaused,
    PlaybackStopped,
    PositionChanged(Duration),
    SpeedChanged(f32),
    EndReached,
    Error(String),
}
```

---

## Video Sync Controller (`src/video/sync.rs`)

```rust
/// Synchronize video playback with ride progress
pub trait VideoSyncController: Send + Sync {
    /// Configure sync for a route
    fn configure(&self, sync_config: VideoSync);

    /// Start synchronized playback
    async fn start(&self) -> Result<(), VideoError>;

    /// Stop synchronized playback
    fn stop(&self);

    /// Update rider position/speed (called each frame)
    fn update(&self, rider_state: &RiderState);

    /// Get current sync state
    fn get_sync_state(&self) -> SyncState;

    /// Add manual sync point
    fn add_sync_point(&self, point: SyncPoint);

    /// Remove sync point
    fn remove_sync_point(&self, index: usize);

    /// Get sync points
    fn get_sync_points(&self) -> Vec<SyncPoint>;

    /// Check if currently synced
    fn is_synced(&self) -> bool;

    /// Get sync diagnostics
    fn get_diagnostics(&self) -> SyncDiagnostics;
}

pub struct RiderState {
    pub virtual_speed_kmh: f32,
    pub distance_meters: f32,
    pub elapsed_time: Duration,
    pub is_pedaling: bool,
}

pub struct SyncState {
    pub video_position: Duration,
    pub video_speed: f32,
    pub route_distance: f32,
    pub expected_video_position: Duration,
    pub sync_offset: Duration,
    pub status: SyncStatus,
}

pub enum SyncStatus {
    Synced,
    CatchingUp,
    SlowingDown,
    Paused,
    NotStarted,
    Ended,
}

pub struct SyncDiagnostics {
    pub target_speed: f32,
    pub actual_speed: f32,
    pub drift_seconds: f32,
    pub corrections_count: u32,
    pub last_correction: Option<Instant>,
}
```

---

## Speed Mapping Algorithm

```rust
/// Calculate video playback speed from rider speed
impl VideoSyncController {
    /// Map rider virtual speed to video playback speed
    ///
    /// Formula:
    ///   video_speed = clamp(rider_speed / route_avg_speed, min_speed, max_speed)
    ///
    /// Where:
    ///   - route_avg_speed: Average speed the video was recorded at
    ///   - min_speed: Minimum playback speed (default 0.5)
    ///   - max_speed: Maximum playback speed (default 2.0)
    ///
    /// Special cases:
    ///   - If rider_speed < 5 km/h: pause video
    ///   - If rider_speed > 2x route_avg_speed: cap at max_speed
    pub fn calculate_playback_speed(
        &self,
        rider_speed_kmh: f32,
        route_avg_speed_kmh: f32,
        config: &VideoSync,
    ) -> f32 {
        const PAUSE_THRESHOLD_KMH: f32 = 5.0;

        if rider_speed_kmh < PAUSE_THRESHOLD_KMH {
            return 0.0; // Will trigger pause
        }

        let raw_speed = rider_speed_kmh / route_avg_speed_kmh;
        raw_speed.clamp(config.playback_speed_range.0, config.playback_speed_range.1)
    }

    /// Map distance to video position using sync points
    ///
    /// Linear interpolation between sync points:
    ///   video_time = interpolate(sync_points, distance)
    ///
    /// If no sync points: assume linear mapping:
    ///   video_time = (distance / total_distance) * video_duration
    pub fn distance_to_video_time(
        &self,
        distance_meters: f32,
        sync: &VideoSync,
    ) -> Duration {
        if sync.sync_points.is_empty() {
            // Linear mapping
            let progress = distance_meters / sync.total_route_distance;
            Duration::from_secs_f32(progress * sync.duration_seconds)
        } else {
            // Interpolate between sync points
            self.interpolate_sync_points(&sync.sync_points, distance_meters)
        }
    }
}
```

---

## Video Renderer Integration

```rust
/// Integration with egui for video display
pub trait VideoRenderer {
    /// Create egui texture from video frame
    fn frame_to_texture(&self, ctx: &egui::Context, frame: &VideoFrame) -> egui::TextureHandle;

    /// Render video in UI
    fn render(&self, ui: &mut egui::Ui, texture: &egui::TextureHandle, max_size: egui::Vec2);

    /// Get optimal display size maintaining aspect ratio
    fn calculate_display_size(&self, video: &VideoInfo, available: egui::Vec2) -> egui::Vec2;
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
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
```

---

## Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub enabled: bool,
    pub default_volume: u8,
    pub auto_pause_on_stop: bool,
    pub show_debug_overlay: bool,
    pub preferred_resolution: VideoResolutionPreference,
    pub hardware_acceleration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoResolutionPreference {
    Native,
    Limit720p,
    Limit1080p,
    Limit4k,
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
        }
    }
}
```

---

## Supported Formats

```rust
pub const SUPPORTED_CONTAINERS: &[&str] = &["mp4", "mkv", "webm", "avi", "mov"];

pub const SUPPORTED_CODECS: &[&str] = &[
    "h264",   // AVC
    "hevc",   // H.265
    "vp9",    // WebM
    "av1",    // AV1
];

/// Check if a file is supported
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_CONTAINERS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
```

---

## Performance Considerations

1. **Frame buffering**: Pre-decode 2-5 frames ahead to handle speed changes smoothly
2. **Resolution scaling**: Downscale high-res videos to match display size
3. **Hardware acceleration**: Use GPU decoding when available (NVDEC, VideoToolbox, VAAPI)
4. **Memory management**: Limit decoded frame buffer to ~500MB
5. **Seeking**: Use keyframe-based seeking for speed changes, precise seeking only when paused
