//! Garmin Connect API Integration
//!
//! T105: Implement Garmin Connect API upload.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Upload timeout in seconds (longer for file uploads)
const UPLOAD_TIMEOUT_SECS: u64 = 120;

/// Default Garmin Connect API base URL
const GARMIN_API_BASE_URL: &str = "https://connect.garmin.com/modern/proxy";

/// Garmin Connect API client
#[allow(dead_code)]
pub struct GarminClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL (for /upload-service, /userprofile-service, etc.)
    base_url: String,
    /// HTTP client for API requests
    http_client: Client,
}

impl Default for GarminClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GarminClient {
    /// Create a new Garmin Connect client
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url: GARMIN_API_BASE_URL.to_string(),
            http_client,
        }
    }

    /// Create a new Garmin Connect client with custom base URL (for testing)
    #[cfg(test)]
    pub fn with_base_url(base_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url,
            http_client,
        }
    }

    /// Get the upload timeout duration
    pub fn upload_timeout() -> Duration {
        Duration::from_secs(UPLOAD_TIMEOUT_SECS)
    }

    /// Get the default request timeout duration
    pub fn default_timeout() -> Duration {
        Duration::from_secs(DEFAULT_TIMEOUT_SECS)
    }

    /// Get the base URL for API requests
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get a reference to the HTTP client
    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    /// Set the access token for API calls
    pub async fn set_access_token(&self, token: String) {
        *self.access_token.write().await = Some(token);
    }

    /// Clear the access token
    pub async fn clear_token(&self) {
        *self.access_token.write().await = None;
    }

    /// Check if client has a token configured
    pub fn is_configured(&self) -> bool {
        self.access_token
            .try_read()
            .map(|t| t.is_some())
            .unwrap_or(false)
    }

    /// Get the current access token (for internal use by API methods)
    async fn get_access_token(&self) -> Result<String, SyncError> {
        self.access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))
    }

    /// Upload a FIT file to Garmin Connect
    ///
    /// Returns the sync record with upload status
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        _fit_data: &[u8],
    ) -> Result<SyncRecord, SyncError> {
        let _token = self.get_access_token().await?;

        let record_id = Uuid::new_v4();

        tracing::info!(
            "Uploading activity {} to Garmin Connect (record: {})",
            ride_id,
            record_id
        );

        // Garmin Connect uses a different upload flow than Strava
        // It typically uses the Garmin Connect API or the GarminConnect-Upload endpoint

        // TODO: Make actual HTTP request to Garmin Connect
        // POST https://connect.garmin.com/modern/proxy/upload-service/upload/.fit
        //
        // Multipart form data:
        // - file: FIT file data
        //
        // Headers:
        // - Authorization: Bearer {token}
        // - NK: various required Garmin headers

        // For now, create a pending record
        let record = SyncRecord {
            id: record_id,
            ride_id: *ride_id,
            platform: SyncPlatform::GarminConnect,
            status: SyncRecordStatus::Pending,
            external_id: None,
            external_url: None,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            retry_count: 0,
        };

        tracing::debug!("Garmin Connect upload initiated: {:?}", record);

        Ok(record)
    }

    /// Get user profile
    pub async fn get_user_profile(&self) -> Result<GarminUserProfile, SyncError> {
        let _token = self.get_access_token().await?;

        // TODO: GET https://connect.garmin.com/modern/proxy/userprofile-service/socialProfile

        Ok(GarminUserProfile {
            display_name: "Test User".to_string(),
            profile_image_url: None,
            user_id: 0,
        })
    }

    /// Get recent activities
    pub async fn get_recent_activities(
        &self,
        _limit: u32,
    ) -> Result<Vec<GarminActivity>, SyncError> {
        let _token = self.get_access_token().await?;

        // TODO: GET https://connect.garmin.com/modern/proxy/activitylist-service/activities/search/activities

        Ok(Vec::new())
    }

    /// Delete an uploaded activity
    pub async fn delete_activity(&self, activity_id: &str) -> Result<(), SyncError> {
        let _token = self.get_access_token().await?;

        tracing::info!("Deleting Garmin activity: {}", activity_id);

        // TODO: DELETE https://connect.garmin.com/modern/proxy/activity-service/activity/{activity_id}

        Ok(())
    }

    /// Log out and revoke access
    pub async fn logout(&self) -> Result<(), SyncError> {
        tracing::info!("Logging out from Garmin Connect");

        // Garmin doesn't have a standard OAuth revoke endpoint
        // Just clear local token
        self.clear_token().await;

        Ok(())
    }
}

/// Garmin user profile
#[derive(Debug, Clone)]
pub struct GarminUserProfile {
    pub display_name: String,
    pub profile_image_url: Option<String>,
    pub user_id: u64,
}

/// Garmin activity summary
#[derive(Debug, Clone)]
pub struct GarminActivity {
    pub activity_id: u64,
    pub activity_name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub duration_seconds: u32,
    pub distance_meters: Option<f64>,
    pub activity_type: GarminActivityType,
}

/// Garmin activity types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarminActivityType {
    Cycling,
    VirtualRide,
    IndoorCycling,
    Running,
    Walking,
    Other,
}

impl GarminActivityType {
    /// Convert from Garmin type key
    pub fn from_type_key(key: &str) -> Self {
        match key {
            "cycling" => Self::Cycling,
            "virtual_ride" => Self::VirtualRide,
            "indoor_cycling" => Self::IndoorCycling,
            "running" => Self::Running,
            "walking" => Self::Walking,
            _ => Self::Other,
        }
    }

    /// Get the type key for API calls
    pub fn type_key(&self) -> &'static str {
        match self {
            Self::Cycling => "cycling",
            Self::VirtualRide => "virtual_ride",
            Self::IndoorCycling => "indoor_cycling",
            Self::Running => "running",
            Self::Walking => "walking",
            Self::Other => "other",
        }
    }
}

/// Garmin Connect OAuth scopes
///
/// Garmin Connect uses OAuth 2.0 with specific scopes for different API capabilities.
/// These scopes control what data the application can access and modify.
pub mod scopes {
    /// Read user profile information
    pub const PROFILE_READ: &str = "profile:read";
    /// Read activity data (workouts, activities, etc.)
    pub const ACTIVITY_READ: &str = "activity:read";
    /// Write activity data (upload workouts, activities)
    pub const ACTIVITY_WRITE: &str = "activity:write";
    /// Read device information
    pub const DEVICE_READ: &str = "device:read";
}

/// Get default OAuth scopes for Garmin Connect
///
/// Returns the standard scopes needed for uploading activities and reading user profile.
pub fn default_scopes() -> Vec<String> {
    vec![
        scopes::PROFILE_READ.to_string(),
        scopes::ACTIVITY_READ.to_string(),
        scopes::ACTIVITY_WRITE.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GarminClient::new();
        assert!(!client.is_configured());
        // Verify default base URL
        assert_eq!(client.base_url(), GARMIN_API_BASE_URL);
    }

    #[test]
    fn test_client_with_custom_base_url() {
        let custom_url = "http://localhost:8080".to_string();
        let client = GarminClient::with_base_url(custom_url.clone());
        assert!(!client.is_configured());
        assert_eq!(client.base_url(), custom_url);
    }

    #[test]
    fn test_http_client_exists() {
        let client = GarminClient::new();
        // Just verify we can get a reference to the HTTP client
        let _http_client = client.http_client();
    }

    #[test]
    fn test_timeout_constants() {
        // Verify timeout durations are reasonable
        let default_timeout = GarminClient::default_timeout();
        let upload_timeout = GarminClient::upload_timeout();

        assert_eq!(default_timeout, Duration::from_secs(60));
        assert_eq!(upload_timeout, Duration::from_secs(120));
        assert!(upload_timeout > default_timeout);
    }

    #[tokio::test]
    async fn test_set_token() {
        let client = GarminClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());
    }

    #[tokio::test]
    async fn test_clear_token() {
        let client = GarminClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());

        client.clear_token().await;
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_get_access_token_returns_error_when_not_configured() {
        let client = GarminClient::new();
        let result = client.get_access_token().await;
        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_get_access_token_returns_token_when_configured() {
        let client = GarminClient::new();
        client.set_access_token("my_secret_token".to_string()).await;
        let result = client.get_access_token().await;
        assert_eq!(result.unwrap(), "my_secret_token");
    }

    #[tokio::test]
    async fn test_upload_activity_without_token_returns_not_configured() {
        let client = GarminClient::new();
        let ride_id = Uuid::new_v4();
        let fit_data = vec![0u8; 100];

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_get_user_profile_without_token_returns_not_configured() {
        let client = GarminClient::new();
        let result = client.get_user_profile().await;
        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_get_recent_activities_without_token_returns_not_configured() {
        let client = GarminClient::new();
        let result = client.get_recent_activities(10).await;
        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_delete_activity_without_token_returns_not_configured() {
        let client = GarminClient::new();
        let result = client.delete_activity("12345").await;
        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_logout_clears_token() {
        let client = GarminClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());

        let result = client.logout().await;
        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[test]
    fn test_activity_type_conversion() {
        assert_eq!(
            GarminActivityType::from_type_key("cycling"),
            GarminActivityType::Cycling
        );
        assert_eq!(
            GarminActivityType::from_type_key("indoor_cycling"),
            GarminActivityType::IndoorCycling
        );
        assert_eq!(
            GarminActivityType::from_type_key("unknown"),
            GarminActivityType::Other
        );
    }

    #[test]
    fn test_activity_type_key() {
        assert_eq!(GarminActivityType::VirtualRide.type_key(), "virtual_ride");
        assert_eq!(
            GarminActivityType::IndoorCycling.type_key(),
            "indoor_cycling"
        );
    }

    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert!(scopes.contains(&scopes::ACTIVITY_WRITE.to_string()));
        assert!(scopes.contains(&scopes::PROFILE_READ.to_string()));
        assert!(scopes.contains(&scopes::ACTIVITY_READ.to_string()));
        assert!(!scopes.contains(&scopes::DEVICE_READ.to_string()));
    }

    #[test]
    fn test_scope_values() {
        assert_eq!(scopes::PROFILE_READ, "profile:read");
        assert_eq!(scopes::ACTIVITY_READ, "activity:read");
        assert_eq!(scopes::ACTIVITY_WRITE, "activity:write");
        assert_eq!(scopes::DEVICE_READ, "device:read");
    }
}
