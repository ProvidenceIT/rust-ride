//! Fitness Platform Sync
//!
//! Provides OAuth authentication and activity upload to fitness platforms.

pub mod garmin;
pub mod oauth;
pub mod service;
pub mod strava;
pub mod trainingpeaks;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use garmin::{ErrorCategory, GarminClient, GarminUserProfile, SyncErrorExt};
pub use oauth::{
    CallbackResult, CredentialStore, KeyringCredentialStore, OAuthCallbackServer, OAuthHandler,
};
pub use service::{
    create_sync_service, create_sync_service_with_db, PlatformStatus, SyncEvent, SyncMessage,
    SyncService, SyncServiceHandle, UploadQueueEntry,
};
pub use trainingpeaks::{
    ImportedWorkout, TrainingPeaksClient, TrainingPeaksSyncManager, WorkoutSyncConfig,
    WorkoutSyncResult,
};

/// Sync-related errors
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Platform not configured: {0:?}")]
    NotConfigured(SyncPlatform),

    #[error("Authorization required")]
    AuthorizationRequired,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Credential storage error: {0}")]
    CredentialError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Activity already exists on {0:?}")]
    DuplicateActivity(SyncPlatform),

    #[error("Invalid FIT file: {0}")]
    InvalidFitFile(String),

    #[error("Request timed out after {0} seconds")]
    Timeout(u64),

    #[error("Rate limit exceeded. Please try again later.")]
    RateLimited,
}

/// Supported sync platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncPlatform {
    /// Garmin Connect
    GarminConnect,
    /// Strava
    Strava,
    /// Apple Health (macOS only)
    #[cfg(target_os = "macos")]
    HealthKit,
    /// TrainingPeaks
    TrainingPeaks,
    /// Intervals.icu
    IntervalsIcu,
}

impl SyncPlatform {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            SyncPlatform::GarminConnect => "Garmin Connect",
            SyncPlatform::Strava => "Strava",
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => "Apple Health",
            SyncPlatform::TrainingPeaks => "TrainingPeaks",
            SyncPlatform::IntervalsIcu => "Intervals.icu",
        }
    }

    /// Get authorization URL base
    pub fn auth_url_base(&self) -> &'static str {
        match self {
            SyncPlatform::GarminConnect => "https://connect.garmin.com/oauthConfirm",
            SyncPlatform::Strava => "https://www.strava.com/oauth/authorize",
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => "", // No OAuth for HealthKit
            SyncPlatform::TrainingPeaks => "https://oauth.trainingpeaks.com/OAuth/Authorize",
            SyncPlatform::IntervalsIcu => "https://intervals.icu/oauth/authorize",
        }
    }

    /// Check if platform uses OAuth
    pub fn uses_oauth(&self) -> bool {
        match self {
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => false,
            _ => true,
        }
    }
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Platform-specific configurations
    pub platforms: HashMap<SyncPlatform, PlatformConfig>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        let mut platforms = HashMap::new();

        platforms.insert(SyncPlatform::GarminConnect, PlatformConfig::default());
        platforms.insert(SyncPlatform::Strava, PlatformConfig::default());
        platforms.insert(SyncPlatform::TrainingPeaks, PlatformConfig::default());
        platforms.insert(SyncPlatform::IntervalsIcu, PlatformConfig::default());

        Self { platforms }
    }
}

/// Platform-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformConfig {
    /// Whether this platform is enabled
    pub enabled: bool,
    /// Auto-sync after ride completion
    pub auto_sync: bool,
}

/// TrainingPeaks-specific platform configuration
///
/// Extends the base PlatformConfig with TrainingPeaks-specific options
/// for workout plan syncing and sync frequency control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPeaksPlatformConfig {
    /// Base platform configuration
    pub enabled: bool,
    /// Auto-sync rides after completion
    pub auto_sync_rides: bool,
    /// Whether to sync workout plans from TrainingPeaks
    pub sync_workout_plans: bool,
    /// Sync frequency in hours (how often to check for new workouts)
    pub sync_frequency_hours: u32,
    /// Number of days to look ahead for scheduled workouts
    pub lookahead_days: i32,
    /// Number of days to look back for scheduled workouts
    pub lookback_days: i32,
    /// Only sync cycling workouts (filter out running, swimming, etc.)
    pub cycling_only: bool,
}

impl Default for TrainingPeaksPlatformConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_sync_rides: true,
            sync_workout_plans: true,
            sync_frequency_hours: 6,
            lookahead_days: 14,
            lookback_days: 7,
            cycling_only: true,
        }
    }
}

impl TrainingPeaksPlatformConfig {
    /// Create a new TrainingPeaks configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config from base PlatformConfig
    pub fn from_platform_config(config: &PlatformConfig) -> Self {
        Self {
            enabled: config.enabled,
            auto_sync_rides: config.auto_sync,
            ..Default::default()
        }
    }

    /// Convert to base PlatformConfig
    pub fn to_platform_config(&self) -> PlatformConfig {
        PlatformConfig {
            enabled: self.enabled,
            auto_sync: self.auto_sync_rides,
        }
    }

    /// Check if workout sync is due based on last sync time
    pub fn is_workout_sync_due(&self, last_sync_hours_ago: u64) -> bool {
        self.sync_workout_plans && last_sync_hours_ago >= self.sync_frequency_hours as u64
    }

    /// Get available sync frequency options in hours
    pub fn sync_frequency_options() -> &'static [(u32, &'static str)] {
        &[
            (1, "Every hour"),
            (3, "Every 3 hours"),
            (6, "Every 6 hours"),
            (12, "Every 12 hours"),
            (24, "Daily"),
        ]
    }

    /// Get display name for sync frequency
    pub fn sync_frequency_display(&self) -> &'static str {
        match self.sync_frequency_hours {
            1 => "Every hour",
            3 => "Every 3 hours",
            6 => "Every 6 hours",
            12 => "Every 12 hours",
            24 => "Daily",
            _ => "Custom",
        }
    }
}

/// Sync record for tracking upload status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// Unique ID
    pub id: Uuid,
    /// Ride ID
    pub ride_id: Uuid,
    /// Target platform
    pub platform: SyncPlatform,
    /// Current status
    pub status: SyncRecordStatus,
    /// External activity ID (from platform)
    pub external_id: Option<String>,
    /// External activity URL
    pub external_url: Option<String>,
    /// When sync was initiated
    pub created_at: DateTime<Utc>,
    /// When sync completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
}

/// Sync record status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncRecordStatus {
    /// Pending upload
    Pending,
    /// Currently uploading
    Uploading,
    /// Upload completed successfully
    Completed,
    /// Upload failed
    Failed,
    /// Cancelled by user
    Cancelled,
}

/// Trait for uploading to platforms
pub trait PlatformUploader: Send + Sync {
    /// Upload a ride to platform
    fn upload(
        &self,
        platform: SyncPlatform,
        ride_id: &Uuid,
        fit_data: &[u8],
    ) -> impl std::future::Future<Output = Result<SyncRecord, SyncError>> + Send;

    /// Get upload status
    fn get_status(&self, record_id: &Uuid) -> Option<SyncRecordStatus>;

    /// Retry failed upload
    fn retry(
        &self,
        record_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<SyncRecord, SyncError>> + Send;

    /// Get sync history for a ride
    fn get_sync_history(&self, ride_id: &Uuid) -> Vec<SyncRecord>;

    /// Cancel pending upload
    fn cancel(&self, record_id: &Uuid) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_display_names() {
        assert_eq!(SyncPlatform::Strava.display_name(), "Strava");
        assert_eq!(SyncPlatform::GarminConnect.display_name(), "Garmin Connect");
    }

    #[test]
    fn test_config_default() {
        let config = SyncConfig::default();
        assert!(config.platforms.contains_key(&SyncPlatform::Strava));
        assert!(!config.platforms.get(&SyncPlatform::Strava).unwrap().enabled);
    }

    #[test]
    fn test_trainingpeaks_platform_config_default() {
        let config = TrainingPeaksPlatformConfig::default();
        assert!(!config.enabled);
        assert!(config.auto_sync_rides);
        assert!(config.sync_workout_plans);
        assert_eq!(config.sync_frequency_hours, 6);
        assert_eq!(config.lookahead_days, 14);
        assert_eq!(config.lookback_days, 7);
        assert!(config.cycling_only);
    }

    #[test]
    fn test_trainingpeaks_platform_config_new() {
        let config = TrainingPeaksPlatformConfig::new();
        assert!(!config.enabled);
        assert!(config.auto_sync_rides);
    }

    #[test]
    fn test_trainingpeaks_from_platform_config() {
        let platform_config = PlatformConfig {
            enabled: true,
            auto_sync: false,
        };
        let tp_config = TrainingPeaksPlatformConfig::from_platform_config(&platform_config);

        assert!(tp_config.enabled);
        assert!(!tp_config.auto_sync_rides);
        // Defaults should be preserved
        assert!(tp_config.sync_workout_plans);
        assert_eq!(tp_config.sync_frequency_hours, 6);
    }

    #[test]
    fn test_trainingpeaks_to_platform_config() {
        let tp_config = TrainingPeaksPlatformConfig {
            enabled: true,
            auto_sync_rides: false,
            sync_workout_plans: true,
            sync_frequency_hours: 12,
            lookahead_days: 14,
            lookback_days: 7,
            cycling_only: true,
        };
        let platform_config = tp_config.to_platform_config();

        assert!(platform_config.enabled);
        assert!(!platform_config.auto_sync);
    }

    #[test]
    fn test_trainingpeaks_is_workout_sync_due() {
        let config = TrainingPeaksPlatformConfig {
            sync_workout_plans: true,
            sync_frequency_hours: 6,
            ..Default::default()
        };

        // Not due yet
        assert!(!config.is_workout_sync_due(5));
        // Due now
        assert!(config.is_workout_sync_due(6));
        // Overdue
        assert!(config.is_workout_sync_due(10));
    }

    #[test]
    fn test_trainingpeaks_sync_due_disabled() {
        let config = TrainingPeaksPlatformConfig {
            sync_workout_plans: false,
            sync_frequency_hours: 6,
            ..Default::default()
        };

        // Never due when disabled
        assert!(!config.is_workout_sync_due(100));
    }

    #[test]
    fn test_trainingpeaks_sync_frequency_options() {
        let options = TrainingPeaksPlatformConfig::sync_frequency_options();
        assert_eq!(options.len(), 5);
        assert_eq!(options[0], (1, "Every hour"));
        assert_eq!(options[4], (24, "Daily"));
    }

    #[test]
    fn test_trainingpeaks_sync_frequency_display() {
        let mut config = TrainingPeaksPlatformConfig::default();

        config.sync_frequency_hours = 1;
        assert_eq!(config.sync_frequency_display(), "Every hour");

        config.sync_frequency_hours = 6;
        assert_eq!(config.sync_frequency_display(), "Every 6 hours");

        config.sync_frequency_hours = 24;
        assert_eq!(config.sync_frequency_display(), "Daily");

        config.sync_frequency_hours = 48;
        assert_eq!(config.sync_frequency_display(), "Custom");
    }
}
