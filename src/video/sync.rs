//! Video Sync Controller
//!
//! Synchronizes video playback with ride progress.

use super::{VideoError, PAUSE_THRESHOLD_KMH};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Video sync configuration
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSync {
    /// Unique ID
    pub id: Uuid,
    /// Route ID this sync is for
    pub route_id: Uuid,
    /// Video file path
    pub video_path: String,
    /// Total route distance in meters
    pub total_route_distance: f32,
    /// Video duration in seconds
    pub duration_seconds: f32,
    /// Average speed the video was recorded at (km/h)
    pub recording_speed_kmh: f32,
    /// Minimum playback speed
    pub min_playback_speed: f32,
    /// Maximum playback speed
    pub max_playback_speed: f32,
    /// Sync points for calibration
    pub sync_points: Vec<SyncPoint>,
    /// Configuration for sync behavior
    pub config: VideoSyncConfig,
}

/// Configuration for video sync behavior
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSyncConfig {
    /// Whether to pause video when rider stops
    pub pause_on_stop: bool,
    /// Speed threshold for pausing (km/h)
    pub pause_threshold: f32,
}

impl Default for VideoSyncConfig {
    fn default() -> Self {
        Self {
            pause_on_stop: true,
            pause_threshold: 5.0,
        }
    }
}

impl Default for VideoSync {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            route_id: Uuid::nil(),
            video_path: String::new(),
            total_route_distance: 0.0,
            duration_seconds: 0.0,
            recording_speed_kmh: 25.0,
            min_playback_speed: 0.5,
            max_playback_speed: 2.0,
            sync_points: Vec::new(),
            config: VideoSyncConfig::default(),
        }
    }
}

impl VideoSync {
    /// Playback speed range as tuple
    pub fn playback_speed_range(&self) -> (f32, f32) {
        (self.min_playback_speed, self.max_playback_speed)
    }
}

/// A calibration point linking distance to video time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPoint {
    /// Distance from start in meters
    pub distance_meters: f32,
    /// Video time at this distance
    pub video_time: Duration,
    /// Optional label
    pub label: Option<String>,
}

impl SyncPoint {
    /// Create a new sync point
    pub fn new(distance: f32, video_time: Duration) -> Self {
        Self {
            distance_meters: distance,
            video_time,
            label: None,
        }
    }

    /// Create a sync point with label
    pub fn with_label(distance: f32, video_time: Duration, label: impl Into<String>) -> Self {
        Self {
            distance_meters: distance,
            video_time,
            label: Some(label.into()),
        }
    }
}

/// Current rider state for sync calculations
#[derive(Debug, Clone)]
pub struct RiderState {
    /// Virtual speed in km/h
    pub virtual_speed_kmh: f32,
    /// Distance traveled in meters
    pub distance_meters: f32,
    /// Elapsed time
    pub elapsed_time: Duration,
    /// Whether rider is pedaling
    pub is_pedaling: bool,
}

/// Current sync state
#[derive(Debug, Clone)]
pub struct SyncState {
    /// Current video position
    pub video_position: Duration,
    /// Current playback speed
    pub video_speed: f32,
    /// Current route distance
    pub route_distance: f32,
    /// Expected video position based on distance
    pub expected_video_position: Duration,
    /// Offset between actual and expected
    pub sync_offset: Duration,
    /// Current status
    pub status: SyncStatus,
}

/// Sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Synchronized
    Synced,
    /// Catching up (video behind)
    CatchingUp,
    /// Slowing down (video ahead)
    SlowingDown,
    /// Paused (rider stopped)
    Paused,
    /// Not started
    NotStarted,
    /// Ended
    Ended,
}

/// Sync diagnostics
#[derive(Debug, Clone)]
pub struct SyncDiagnostics {
    /// Target playback speed
    pub target_speed: f32,
    /// Actual playback speed
    pub actual_speed: f32,
    /// Drift in seconds
    pub drift_seconds: f32,
    /// Number of corrections made
    pub corrections_count: u32,
    /// Last correction time
    pub last_correction: Option<Instant>,
}

/// Trait for video sync controllers
pub trait VideoSyncController: Send + Sync {
    /// Configure sync for a route
    fn configure(&self, sync_config: VideoSync);

    /// Start synchronized playback
    fn start(&self) -> impl std::future::Future<Output = Result<(), VideoError>> + Send;

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

/// Calculate video playback speed from rider speed
pub fn calculate_playback_speed(
    rider_speed_kmh: f32,
    route_avg_speed_kmh: f32,
    min_speed: f32,
    max_speed: f32,
) -> f32 {
    if rider_speed_kmh < PAUSE_THRESHOLD_KMH {
        return 0.0; // Will trigger pause
    }

    let raw_speed = rider_speed_kmh / route_avg_speed_kmh;
    raw_speed.clamp(min_speed, max_speed)
}

/// Map distance to video position using sync points
pub fn distance_to_video_time(distance_meters: f32, sync_config: &VideoSync) -> Duration {
    if sync_config.sync_points.is_empty() {
        // Linear mapping
        let progress = distance_meters / sync_config.total_route_distance;
        Duration::from_secs_f32(progress * sync_config.duration_seconds)
    } else {
        // Interpolate between sync points
        interpolate_sync_points(&sync_config.sync_points, distance_meters)
    }
}

/// Interpolate between sync points
fn interpolate_sync_points(points: &[SyncPoint], distance: f32) -> Duration {
    if points.is_empty() {
        return Duration::ZERO;
    }

    // Find surrounding points
    let mut prev = &points[0];
    let mut next = &points[0];

    for point in points {
        if point.distance_meters <= distance {
            prev = point;
        } else {
            next = point;
            break;
        }
    }

    // If we're past all points, use last point
    if prev.distance_meters >= distance {
        return prev.video_time;
    }

    // Linear interpolation
    let distance_range = next.distance_meters - prev.distance_meters;
    if distance_range <= 0.0 {
        return prev.video_time;
    }

    let progress = (distance - prev.distance_meters) / distance_range;
    let time_range = next.video_time.as_secs_f32() - prev.video_time.as_secs_f32();

    Duration::from_secs_f32(prev.video_time.as_secs_f32() + (progress * time_range))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_speed_calculation() {
        // Normal speed
        let speed = calculate_playback_speed(25.0, 25.0, 0.5, 2.0);
        assert!((speed - 1.0).abs() < 0.01);

        // Faster than recording
        let speed = calculate_playback_speed(50.0, 25.0, 0.5, 2.0);
        assert_eq!(speed, 2.0); // Clamped to max

        // Slower than recording
        let speed = calculate_playback_speed(10.0, 25.0, 0.5, 2.0);
        assert!((speed - 0.5).abs() < 0.01); // Clamped to min

        // Stopped
        let speed = calculate_playback_speed(3.0, 25.0, 0.5, 2.0);
        assert_eq!(speed, 0.0); // Below threshold
    }

    #[test]
    fn test_distance_to_video_time_linear() {
        let sync = VideoSync {
            total_route_distance: 10000.0, // 10km
            duration_seconds: 1800.0,      // 30 min
            ..Default::default()
        };

        // At start
        let time = distance_to_video_time(0.0, &sync);
        assert_eq!(time, Duration::ZERO);

        // At middle
        let time = distance_to_video_time(5000.0, &sync);
        assert!((time.as_secs_f32() - 900.0).abs() < 1.0); // 15 min

        // At end
        let time = distance_to_video_time(10000.0, &sync);
        assert!((time.as_secs_f32() - 1800.0).abs() < 1.0);
    }

    #[test]
    fn test_sync_point_creation() {
        let point = SyncPoint::new(1000.0, Duration::from_secs(60));
        assert_eq!(point.distance_meters, 1000.0);
        assert!(point.label.is_none());

        let labeled = SyncPoint::with_label(2000.0, Duration::from_secs(120), "Checkpoint");
        assert_eq!(labeled.label, Some("Checkpoint".to_string()));
    }

    #[test]
    fn test_interpolation() {
        let points = vec![
            SyncPoint::new(0.0, Duration::ZERO),
            SyncPoint::new(1000.0, Duration::from_secs(60)),
            SyncPoint::new(2000.0, Duration::from_secs(180)), // Non-linear
        ];

        let time = interpolate_sync_points(&points, 500.0);
        assert!((time.as_secs_f32() - 30.0).abs() < 1.0);

        let time = interpolate_sync_points(&points, 1500.0);
        assert!((time.as_secs_f32() - 120.0).abs() < 1.0);
    }
}
