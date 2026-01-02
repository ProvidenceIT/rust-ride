//! Strava API Integration
//!
//! T106: Implement Strava API upload.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Strava upload API response
#[derive(Debug, Deserialize)]
struct StravaUploadResponse {
    /// Upload ID for status checking
    id: u64,
    /// External ID (optional, if provided in upload)
    #[allow(dead_code)]
    external_id: Option<String>,
    /// Activity ID (only present when processing complete)
    #[allow(dead_code)]
    activity_id: Option<u64>,
    /// Processing status: "Your activity is still being processed."
    #[allow(dead_code)]
    status: Option<String>,
    /// Error message if processing failed
    #[allow(dead_code)]
    error: Option<String>,
}

/// Strava API error response
#[derive(Debug, Deserialize)]
struct StravaApiError {
    /// Error message
    message: String,
    /// Error details
    #[serde(default)]
    errors: Vec<StravaFieldError>,
}

/// Strava field-level error detail
#[derive(Debug, Deserialize)]
struct StravaFieldError {
    /// Resource type
    #[allow(dead_code)]
    resource: String,
    /// Field name
    #[allow(dead_code)]
    field: String,
    /// Error code
    code: String,
}

impl std::fmt::Display for StravaApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.is_empty() {
            write!(f, "{}", self.message)
        } else {
            let details: Vec<String> = self
                .errors
                .iter()
                .map(|e| format!("{}", e.code))
                .collect();
            write!(f, "{} ({})", self.message, details.join(", "))
        }
    }
}

/// Strava API client
#[allow(dead_code)]
pub struct StravaClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL
    base_url: String,
    /// HTTP client for API requests
    http_client: Client,
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
            http_client: Client::new(),
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
    /// Returns the sync record with upload status. Strava processes uploads
    /// asynchronously, so the record will have status Uploading with an
    /// external_id containing the upload_id for status checking.
    ///
    /// # Arguments
    /// * `ride_id` - The local ride ID
    /// * `fit_data` - The FIT file data as bytes
    /// * `activity_name` - Optional activity name
    /// * `description` - Optional activity description
    ///
    /// # Returns
    /// A SyncRecord with the upload_id in external_id field for status polling
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

        // Build multipart form
        // Use ride_id as external_id for correlation
        let external_id = ride_id.to_string();
        let filename = format!("{}.fit", ride_id);

        // Create the file part with proper MIME type
        let file_part = Part::bytes(fit_data.to_vec())
            .file_name(filename)
            .mime_str("application/octet-stream")
            .map_err(|e| SyncError::UploadFailed(format!("Failed to create file part: {}", e)))?;

        // Build the multipart form
        let mut form = Form::new()
            .part("file", file_part)
            .text("data_type", "fit")
            .text("external_id", external_id);

        // Add optional fields
        if let Some(name) = activity_name {
            form = form.text("name", name.to_string());
        }
        if let Some(desc) = description {
            form = form.text("description", desc.to_string());
        }

        // Send the upload request
        let url = format!("{}/uploads", self.base_url);
        tracing::debug!("Sending upload request to {}", url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to send upload request: {}", e)))?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("Strava API rate limit exceeded");
            return Err(SyncError::ApiError(
                "Rate limit exceeded. Please try again later.".to_string(),
            ));
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!("Strava API returned 401 Unauthorized - token may be expired");
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            // Try to parse as Strava error response
            if let Ok(error_response) = serde_json::from_str::<StravaApiError>(&body) {
                tracing::error!("Strava upload failed: {}", error_response);
                return Err(SyncError::UploadFailed(format!(
                    "Strava error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!("Strava upload failed with status {}: {}", status_code, body);
            return Err(SyncError::UploadFailed(format!(
                "Upload failed with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let upload_response: StravaUploadResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::UploadFailed(format!("Failed to parse upload response: {}", e))
        })?;

        tracing::info!(
            "Strava upload initiated successfully, upload_id: {}",
            upload_response.id
        );

        // Create record with Uploading status (async processing)
        // Store upload_id in external_id for status checking
        let record = SyncRecord {
            id: record_id,
            ride_id: *ride_id,
            platform: SyncPlatform::Strava,
            status: SyncRecordStatus::Uploading,
            external_id: Some(upload_response.id.to_string()),
            external_url: None, // Will be set when processing completes
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
        let _token = self
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

    #[tokio::test]
    async fn test_upload_activity_without_token_returns_not_configured() {
        let client = StravaClient::new();
        let ride_id = Uuid::new_v4();
        let fit_data = vec![0u8; 100]; // Dummy FIT data

        let result = client
            .upload_activity(&ride_id, &fit_data, Some("Test Ride"), None)
            .await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_strava_api_error_display() {
        let error = StravaApiError {
            message: "Bad Request".to_string(),
            errors: vec![],
        };
        assert_eq!(format!("{}", error), "Bad Request");

        let error_with_details = StravaApiError {
            message: "Bad Request".to_string(),
            errors: vec![StravaFieldError {
                resource: "Upload".to_string(),
                field: "file".to_string(),
                code: "invalid".to_string(),
            }],
        };
        assert_eq!(format!("{}", error_with_details), "Bad Request (invalid)");
    }

    #[test]
    fn test_strava_upload_response_deserialization() {
        let json = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": "Your activity is still being processed.",
            "error": null
        }"#;

        let response: StravaUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345);
        assert_eq!(response.external_id, Some("test-uuid".to_string()));
        assert!(response.activity_id.is_none());
    }
}
