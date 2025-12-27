//! Strava API Integration
//!
//! T106: Implement Strava API upload.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Strava API client
pub struct StravaClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL
    base_url: String,
}

impl Default for StravaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl StravaClient {
    /// Create a new Strava client
    pub fn new() -> Self {
        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url: "https://www.strava.com/api/v3".to_string(),
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

    /// Upload a FIT file to Strava
    ///
    /// Returns the sync record with upload status
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        fit_data: &[u8],
        activity_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<SyncRecord, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        let record_id = Uuid::new_v4();

        tracing::info!(
            "Uploading activity {} to Strava (record: {})",
            ride_id,
            record_id
        );

        // TODO: Make actual HTTP request to Strava API
        // POST https://www.strava.com/api/v3/uploads
        //
        // Form data:
        // - file: FIT file data
        // - data_type: "fit"
        // - name: activity_name
        // - description: description
        //
        // Headers:
        // - Authorization: Bearer {token}

        // For now, create a pending record
        let record = SyncRecord {
            id: record_id,
            ride_id: *ride_id,
            platform: SyncPlatform::Strava,
            status: SyncRecordStatus::Pending,
            external_id: None,
            external_url: None,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            retry_count: 0,
        };

        tracing::debug!("Strava upload initiated: {:?}", record);

        Ok(record)
    }

    /// Check upload status
    ///
    /// Strava processes uploads asynchronously, so we need to poll for status
    pub async fn check_upload_status(&self, upload_id: &str) -> Result<UploadStatus, SyncError> {
        let _token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        tracing::debug!("Checking Strava upload status: {}", upload_id);

        // TODO: GET https://www.strava.com/api/v3/uploads/{upload_id}
        // Returns status, activity_id when complete

        Ok(UploadStatus::Processing)
    }

    /// Get athlete profile
    pub async fn get_athlete(&self) -> Result<AthleteProfile, SyncError> {
        let _token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        // TODO: GET https://www.strava.com/api/v3/athlete

        Ok(AthleteProfile {
            id: 0,
            username: None,
            firstname: "Test".to_string(),
            lastname: "User".to_string(),
            profile_medium: None,
        })
    }

    /// Deauthorize application
    pub async fn deauthorize(&self) -> Result<(), SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        tracing::info!("Deauthorizing Strava");

        // TODO: POST https://www.strava.com/oauth/deauthorize
        // Body: access_token={token}

        self.clear_token().await;

        Ok(())
    }
}

/// Strava upload status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStatus {
    /// Still being processed
    Processing,
    /// Successfully processed
    Ready { activity_id: u64 },
    /// Processing failed
    Error { error: String },
}

/// Strava athlete profile
#[derive(Debug, Clone)]
pub struct AthleteProfile {
    pub id: u64,
    pub username: Option<String>,
    pub firstname: String,
    pub lastname: String,
    pub profile_medium: Option<String>,
}

impl AthleteProfile {
    /// Get display name
    pub fn display_name(&self) -> String {
        if let Some(ref username) = self.username {
            username.clone()
        } else {
            format!("{} {}", self.firstname, self.lastname)
        }
    }
}

/// Strava OAuth scopes
pub mod scopes {
    /// Read public profile
    pub const READ: &str = "read";
    /// Read private activities
    pub const ACTIVITY_READ: &str = "activity:read";
    /// Read all activities
    pub const ACTIVITY_READ_ALL: &str = "activity:read_all";
    /// Write activities
    pub const ACTIVITY_WRITE: &str = "activity:write";
}

/// Get default OAuth scopes for Strava
pub fn default_scopes() -> Vec<String> {
    vec![
        scopes::READ.to_string(),
        scopes::ACTIVITY_READ_ALL.to_string(),
        scopes::ACTIVITY_WRITE.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = StravaClient::new();
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_set_token() {
        let client = StravaClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());
    }

    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert!(scopes.contains(&scopes::ACTIVITY_WRITE.to_string()));
    }

    #[test]
    fn test_athlete_display_name() {
        let athlete = AthleteProfile {
            id: 123,
            username: Some("cyclist123".to_string()),
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_medium: None,
        };
        assert_eq!(athlete.display_name(), "cyclist123");

        let athlete_no_username = AthleteProfile {
            id: 123,
            username: None,
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_medium: None,
        };
        assert_eq!(athlete_no_username.display_name(), "John Doe");
    }
}
