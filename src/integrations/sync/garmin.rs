//! Garmin Connect API Integration
//!
//! T105: Implement Garmin Connect API upload.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Garmin Connect API client
pub struct GarminClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL
    base_url: String,
}

impl Default for GarminClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GarminClient {
    /// Create a new Garmin Connect client
    pub fn new() -> Self {
        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url: "https://connect.garmin.com".to_string(),
        }
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

    /// Upload a FIT file to Garmin Connect
    ///
    /// Returns the sync record with upload status
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        fit_data: &[u8],
    ) -> Result<SyncRecord, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

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
        let _token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

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
        limit: u32,
    ) -> Result<Vec<GarminActivity>, SyncError> {
        let _token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

        // TODO: GET https://connect.garmin.com/modern/proxy/activitylist-service/activities/search/activities

        Ok(Vec::new())
    }

    /// Delete an uploaded activity
    pub async fn delete_activity(&self, activity_id: &str) -> Result<(), SyncError> {
        let _token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GarminClient::new();
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_set_token() {
        let client = GarminClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());
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
}
