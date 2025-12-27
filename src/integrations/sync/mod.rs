//! Fitness Platform Sync
//!
//! Provides OAuth authentication and activity upload to fitness platforms.

pub mod garmin;
pub mod oauth;
pub mod strava;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use oauth::{CallbackResult, CredentialStore, OAuthCallbackServer, OAuthHandler};

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
}
