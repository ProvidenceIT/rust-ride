//! Strava API Integration
//!
//! T106: Implement Strava API upload.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Upload timeout in seconds (longer for file uploads)
const UPLOAD_TIMEOUT_SECS: u64 = 120;

/// FIT file header magic bytes
const FIT_HEADER_SIZE: u8 = 14;
const FIT_HEADER_SIGNATURE: &[u8] = b".FIT";

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

/// Strava athlete API response
#[derive(Debug, Deserialize)]
struct StravaAthleteResponse {
    /// Athlete ID
    id: u64,
    /// Username (optional, can be null)
    username: Option<String>,
    /// First name
    firstname: String,
    /// Last name
    lastname: String,
    /// Medium profile image URL (optional)
    #[serde(rename = "profile_medium")]
    profile_medium: Option<String>,
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
    /// API base URL (for /api/v3 endpoints)
    base_url: String,
    /// OAuth base URL (for /oauth endpoints like deauthorize)
    oauth_base_url: String,
    /// HTTP client for API requests
    http_client: Client,
}

impl Default for StravaClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Default Strava API base URL
const STRAVA_API_BASE_URL: &str = "https://www.strava.com/api/v3";

/// Default Strava OAuth base URL (for deauthorize)
const STRAVA_OAUTH_BASE_URL: &str = "https://www.strava.com/oauth";

impl StravaClient {
    /// Create a new Strava client
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url: STRAVA_API_BASE_URL.to_string(),
            oauth_base_url: STRAVA_OAUTH_BASE_URL.to_string(),
            http_client,
        }
    }

    /// Create a new Strava client with custom base URLs (for testing)
    #[cfg(test)]
    pub fn with_base_url(base_url: String, oauth_base_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url,
            oauth_base_url,
            http_client,
        }
    }

    /// Validate FIT file data before upload.
    ///
    /// Checks:
    /// - Minimum file size (at least header size)
    /// - FIT file signature (".FIT" at offset 8-12)
    /// - Header size byte is valid
    ///
    /// # Arguments
    /// * `fit_data` - The FIT file bytes
    ///
    /// # Returns
    /// Ok(()) if valid, or InvalidFitFile error with description
    pub fn validate_fit_file(fit_data: &[u8]) -> Result<(), SyncError> {
        // Check minimum size (header must be at least 12 bytes for basic FIT)
        if fit_data.len() < 12 {
            return Err(SyncError::InvalidFitFile(format!(
                "File too small: {} bytes (minimum 12 bytes required)",
                fit_data.len()
            )));
        }

        // Check header size byte (first byte)
        let header_size = fit_data[0];
        if header_size != 12 && header_size != FIT_HEADER_SIZE {
            return Err(SyncError::InvalidFitFile(format!(
                "Invalid header size: {} (expected 12 or 14)",
                header_size
            )));
        }

        // Check FIT signature at bytes 8-11
        if fit_data.len() >= 12 {
            let signature = &fit_data[8..12];
            if signature != FIT_HEADER_SIGNATURE {
                return Err(SyncError::InvalidFitFile(
                    "Missing '.FIT' signature in header".to_string(),
                ));
            }
        }

        // Check total file size is reasonable (at least header + some data)
        let min_expected_size = header_size as usize + 2; // header + at least CRC
        if fit_data.len() < min_expected_size {
            return Err(SyncError::InvalidFitFile(format!(
                "File truncated: {} bytes (expected at least {})",
                fit_data.len(),
                min_expected_size
            )));
        }

        Ok(())
    }

    /// Check if an error message indicates a duplicate activity.
    fn is_duplicate_error(error_msg: &str) -> bool {
        let lower = error_msg.to_lowercase();
        lower.contains("duplicate")
            || lower.contains("already exists")
            || lower.contains("already uploaded")
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
    ///
    /// # Errors
    /// * `InvalidFitFile` - If the FIT file is malformed or too small
    /// * `DuplicateActivity` - If the activity was already uploaded to Strava
    /// * `RateLimited` - If Strava's rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        fit_data: &[u8],
        activity_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<SyncRecord, SyncError> {
        // Validate FIT file before attempting upload
        Self::validate_fit_file(fit_data)?;

        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        let record_id = Uuid::new_v4();

        tracing::info!(
            "Uploading activity {} to Strava (record: {}, size: {} bytes)",
            ride_id,
            record_id,
            fit_data.len()
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

        // Send the upload request with extended timeout for file uploads
        let url = format!("{}/uploads", self.base_url);
        tracing::debug!("Sending upload request to {}", url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .timeout(Duration::from_secs(UPLOAD_TIMEOUT_SECS))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    tracing::warn!("Strava upload request timed out after {} seconds", UPLOAD_TIMEOUT_SECS);
                    SyncError::Timeout(UPLOAD_TIMEOUT_SECS)
                } else if e.is_connect() {
                    tracing::warn!("Failed to connect to Strava: {}", e);
                    SyncError::NetworkError(format!("Connection failed: {}", e))
                } else {
                    tracing::warn!("Failed to send upload request: {}", e);
                    SyncError::NetworkError(format!("Request failed: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("Strava API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!("Strava API returned 401 Unauthorized - token may be expired or revoked");
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            // Try to parse as Strava error response
            if let Ok(error_response) = serde_json::from_str::<StravaApiError>(&body) {
                let error_msg = error_response.to_string();

                // Check for duplicate activity error
                if Self::is_duplicate_error(&error_msg) {
                    tracing::info!("Activity {} already exists on Strava", ride_id);
                    return Err(SyncError::DuplicateActivity(SyncPlatform::Strava));
                }

                tracing::error!("Strava upload failed: {}", error_msg);
                return Err(SyncError::UploadFailed(format!(
                    "Strava error: {}",
                    error_msg
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
    /// Strava processes uploads asynchronously, so we need to poll for status.
    /// Returns `UploadStatus::Ready` with `activity_id` when processing is complete,
    /// `UploadStatus::Error` with a message if processing failed, or
    /// `UploadStatus::Processing` if still being processed.
    ///
    /// # Arguments
    /// * `upload_id` - The upload ID returned from `upload_activity`
    ///
    /// # Returns
    /// The current upload status
    ///
    /// # Errors
    /// * `RateLimited` - If Strava's rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn check_upload_status(&self, upload_id: &str) -> Result<UploadStatus, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        tracing::debug!("Checking Strava upload status: {}", upload_id);

        let url = format!("{}/uploads/{}", self.base_url, upload_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    tracing::warn!("Strava status check timed out after {} seconds", DEFAULT_TIMEOUT_SECS);
                    SyncError::Timeout(DEFAULT_TIMEOUT_SECS)
                } else if e.is_connect() {
                    SyncError::NetworkError(format!("Connection failed: {}", e))
                } else {
                    SyncError::NetworkError(format!("Failed to check upload status: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("Strava API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!("Strava API returned 401 Unauthorized - token may be expired or revoked");
            return Err(SyncError::TokenExpired);
        }

        // Handle not found (404) - upload may have been deleted or invalid ID
        if status_code == reqwest::StatusCode::NOT_FOUND {
            tracing::warn!("Strava upload {} not found", upload_id);
            return Err(SyncError::ApiError(format!(
                "Upload {} not found",
                upload_id
            )));
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<StravaApiError>(&body) {
                tracing::error!("Strava status check failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "Strava error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "Strava status check failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Status check failed with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let upload_response: StravaUploadResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::ApiError(format!("Failed to parse upload status response: {}", e))
        })?;

        // Determine status based on response fields
        // Priority: error > activity_id > processing
        if let Some(error) = upload_response.error {
            if !error.is_empty() {
                // Check for duplicate activity in processing error
                if Self::is_duplicate_error(&error) {
                    tracing::info!("Strava upload {} detected as duplicate: {}", upload_id, error);
                    return Ok(UploadStatus::Duplicate { error });
                }
                tracing::warn!("Strava upload {} failed: {}", upload_id, error);
                return Ok(UploadStatus::Error { error });
            }
        }

        if let Some(activity_id) = upload_response.activity_id {
            tracing::info!(
                "Strava upload {} complete, activity_id: {}",
                upload_id,
                activity_id
            );
            return Ok(UploadStatus::Ready { activity_id });
        }

        tracing::debug!(
            "Strava upload {} still processing: {:?}",
            upload_id,
            upload_response.status
        );
        Ok(UploadStatus::Processing)
    }

    /// Get athlete profile
    ///
    /// Fetches the authenticated athlete's profile from Strava.
    ///
    /// # Returns
    /// The athlete profile including id, name, username, and profile image URL
    ///
    /// # Errors
    /// * `RateLimited` - If Strava's rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn get_athlete(&self) -> Result<AthleteProfile, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        tracing::debug!("Fetching Strava athlete profile");

        let url = format!("{}/athlete", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    SyncError::Timeout(DEFAULT_TIMEOUT_SECS)
                } else if e.is_connect() {
                    SyncError::NetworkError(format!("Connection failed: {}", e))
                } else {
                    SyncError::NetworkError(format!("Failed to fetch athlete profile: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("Strava API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!("Strava API returned 401 Unauthorized - token may be expired or revoked");
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<StravaApiError>(&body) {
                tracing::error!("Strava athlete fetch failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "Strava error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "Strava athlete fetch failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Failed to fetch athlete with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let athlete_response: StravaAthleteResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::ApiError(format!("Failed to parse athlete response: {}", e))
        })?;

        tracing::info!(
            "Fetched Strava athlete profile: {} (id: {})",
            athlete_response
                .username
                .as_deref()
                .unwrap_or(&format!("{} {}", athlete_response.firstname, athlete_response.lastname)),
            athlete_response.id
        );

        // Convert to AthleteProfile
        Ok(AthleteProfile {
            id: athlete_response.id,
            username: athlete_response.username,
            firstname: athlete_response.firstname,
            lastname: athlete_response.lastname,
            profile_medium: athlete_response.profile_medium,
        })
    }

    /// Deauthorize application
    ///
    /// Revokes the application's access to the user's Strava account by POSTing
    /// to the deauthorize endpoint. This invalidates the access token on Strava's
    /// side and clears the local token.
    ///
    /// # Returns
    /// Ok(()) if deauthorization was successful or the token was already invalid.
    /// The local token is cleared regardless of the API response.
    pub async fn deauthorize(&self) -> Result<(), SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::Strava))?;

        tracing::info!("Deauthorizing Strava");

        // POST to Strava's deauthorize endpoint (note: this is at oauth path, not api/v3)
        let url = format!("{}/deauthorize", self.oauth_base_url);

        let response = self
            .http_client
            .post(url)
            .bearer_auth(&token)
            .send()
            .await;

        // Always clear local token, even if the API call fails
        // This ensures the user can disconnect even with network issues
        self.clear_token().await;

        // Now handle the response
        let response = response.map_err(|e| {
            tracing::warn!(
                "Failed to call Strava deauthorize endpoint: {}. Local token cleared.",
                e
            );
            SyncError::NetworkError(format!("Failed to deauthorize: {}", e))
        })?;

        let status_code = response.status();

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!(
                "Strava API rate limit exceeded during deauthorization. Local token cleared."
            );
            // Token is already cleared locally, so this is still a success from user perspective
            return Ok(());
        }

        // Handle unauthorized (401) - token was already invalid/revoked
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::info!("Strava token was already invalid or revoked. Local token cleared.");
            return Ok(());
        }

        // Handle other errors
        if !status_code.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Ok(error_response) = serde_json::from_str::<StravaApiError>(&body) {
                tracing::warn!(
                    "Strava deauthorize returned error: {}. Local token cleared.",
                    error_response
                );
            } else {
                tracing::warn!(
                    "Strava deauthorize returned status {}: {}. Local token cleared.",
                    status_code,
                    body
                );
            }
            // Still consider this a success since local token is cleared
            return Ok(());
        }

        tracing::info!("Successfully deauthorized from Strava");

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
    /// Activity is a duplicate (already uploaded)
    Duplicate { error: String },
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

    #[test]
    fn test_strava_upload_response_ready_state() {
        // Response when upload processing is complete
        let json = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": 987654321,
            "status": "Your activity is ready.",
            "error": null
        }"#;

        let response: StravaUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345);
        assert_eq!(response.activity_id, Some(987654321));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_strava_upload_response_error_state() {
        // Response when upload processing failed
        let json = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": null,
            "error": "The activity appears to be a duplicate."
        }"#;

        let response: StravaUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345);
        assert!(response.activity_id.is_none());
        assert_eq!(
            response.error,
            Some("The activity appears to be a duplicate.".to_string())
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_without_token_returns_not_configured() {
        let client = StravaClient::new();

        let result = client.check_upload_status("12345").await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_upload_status_enum_variants() {
        // Test that UploadStatus variants can be created and compared
        let processing = UploadStatus::Processing;
        let ready = UploadStatus::Ready { activity_id: 12345 };
        let error = UploadStatus::Error {
            error: "Duplicate activity".to_string(),
        };

        assert_eq!(processing, UploadStatus::Processing);
        assert_eq!(
            ready,
            UploadStatus::Ready { activity_id: 12345 }
        );
        assert_eq!(
            error,
            UploadStatus::Error {
                error: "Duplicate activity".to_string()
            }
        );

        // Test they are not equal to each other
        assert_ne!(processing, ready);
        assert_ne!(ready, error);
    }

    #[tokio::test]
    async fn test_get_athlete_without_token_returns_not_configured() {
        let client = StravaClient::new();

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_strava_athlete_response_deserialization() {
        // Full response with all fields
        let json = r#"{
            "id": 12345678,
            "username": "cyclist123",
            "firstname": "John",
            "lastname": "Doe",
            "profile_medium": "https://dgalywyr863hv.cloudfront.net/pictures/athletes/12345678/medium.jpg"
        }"#;

        let response: StravaAthleteResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345678);
        assert_eq!(response.username, Some("cyclist123".to_string()));
        assert_eq!(response.firstname, "John");
        assert_eq!(response.lastname, "Doe");
        assert_eq!(
            response.profile_medium,
            Some("https://dgalywyr863hv.cloudfront.net/pictures/athletes/12345678/medium.jpg".to_string())
        );
    }

    #[test]
    fn test_strava_athlete_response_minimal() {
        // Response with optional fields as null
        let json = r#"{
            "id": 12345678,
            "username": null,
            "firstname": "Jane",
            "lastname": "Smith",
            "profile_medium": null
        }"#;

        let response: StravaAthleteResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345678);
        assert!(response.username.is_none());
        assert_eq!(response.firstname, "Jane");
        assert_eq!(response.lastname, "Smith");
        assert!(response.profile_medium.is_none());
    }

    #[test]
    fn test_strava_athlete_response_ignores_extra_fields() {
        // Strava API returns many more fields - ensure we ignore them gracefully
        let json = r#"{
            "id": 12345678,
            "username": "cyclist123",
            "firstname": "John",
            "lastname": "Doe",
            "profile_medium": "https://example.com/image.jpg",
            "profile": "https://example.com/large.jpg",
            "city": "San Francisco",
            "state": "California",
            "country": "United States",
            "sex": "M",
            "premium": true,
            "summit": true,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let response: StravaAthleteResponse =
            serde_json::from_str(json).expect("Deserialization should succeed with extra fields");

        assert_eq!(response.id, 12345678);
        assert_eq!(response.username, Some("cyclist123".to_string()));
    }

    #[tokio::test]
    async fn test_deauthorize_without_token_returns_not_configured() {
        let client = StravaClient::new();

        let result = client.deauthorize().await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[tokio::test]
    async fn test_deauthorize_clears_local_token() {
        let client = StravaClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());

        // Note: This test will make a real network call that will fail,
        // but the important thing is that the token gets cleared.
        // In a real scenario, you'd use a mock HTTP client.
        let _ = client.deauthorize().await;

        // Token should be cleared regardless of network outcome
        assert!(!client.is_configured());
    }

    // ========================================================================
    // FIT File Validation Tests
    // ========================================================================

    /// Create a valid FIT file header for testing
    fn create_valid_fit_header() -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[0] = 14; // Header size (14 bytes)
        data[1] = 0x10; // Protocol version
        data[2] = 0x00; // Profile version LSB
        data[3] = 0x00; // Profile version MSB
        data[4] = 0x00; // Data size LSB
        data[5] = 0x00;
        data[6] = 0x00;
        data[7] = 0x00; // Data size MSB
        // ".FIT" signature at bytes 8-11
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        data[12] = 0x00; // CRC LSB
        data[13] = 0x00; // CRC MSB
        data
    }

    #[test]
    fn test_validate_fit_file_valid() {
        let valid_fit = create_valid_fit_header();
        let result = StravaClient::validate_fit_file(&valid_fit);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fit_file_valid_12_byte_header() {
        // 12-byte header version (older FIT format)
        let mut data = vec![0u8; 14];
        data[0] = 12; // Header size (12 bytes)
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        let result = StravaClient::validate_fit_file(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fit_file_too_small() {
        let tiny_data = vec![0u8; 5];
        let result = StravaClient::validate_fit_file(&tiny_data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("too small")));
    }

    #[test]
    fn test_validate_fit_file_invalid_header_size() {
        let mut data = vec![0u8; 20];
        data[0] = 50; // Invalid header size
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        let result = StravaClient::validate_fit_file(&data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("Invalid header size")));
    }

    #[test]
    fn test_validate_fit_file_missing_signature() {
        let mut data = vec![0u8; 16];
        data[0] = 14; // Header size
        // Missing ".FIT" signature - just zeros
        let result = StravaClient::validate_fit_file(&data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("signature")));
    }

    #[test]
    fn test_validate_fit_file_truncated() {
        let mut data = vec![0u8; 12]; // Too small for 14-byte header
        data[0] = 14; // Claims 14-byte header but only 12 bytes
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        let result = StravaClient::validate_fit_file(&data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("truncated")));
    }

    // ========================================================================
    // Duplicate Detection Tests
    // ========================================================================

    #[test]
    fn test_is_duplicate_error_detection() {
        assert!(StravaClient::is_duplicate_error("The activity appears to be a duplicate."));
        assert!(StravaClient::is_duplicate_error("Activity already exists"));
        assert!(StravaClient::is_duplicate_error("This file has already uploaded"));
        assert!(StravaClient::is_duplicate_error("DUPLICATE activity detected"));

        // Should not match non-duplicate errors
        assert!(!StravaClient::is_duplicate_error("Invalid file format"));
        assert!(!StravaClient::is_duplicate_error("Rate limit exceeded"));
        assert!(!StravaClient::is_duplicate_error("Server error"));
    }

    #[test]
    fn test_upload_status_duplicate_variant() {
        let duplicate = UploadStatus::Duplicate {
            error: "Activity is a duplicate".to_string(),
        };
        assert_eq!(
            duplicate,
            UploadStatus::Duplicate {
                error: "Activity is a duplicate".to_string()
            }
        );
        // Duplicate should not equal other variants
        assert_ne!(duplicate, UploadStatus::Processing);
        assert_ne!(
            duplicate,
            UploadStatus::Error {
                error: "Activity is a duplicate".to_string()
            }
        );
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_timeout_constant() {
        assert!(DEFAULT_TIMEOUT_SECS > 0);
        assert!(UPLOAD_TIMEOUT_SECS > DEFAULT_TIMEOUT_SECS);
    }

    #[tokio::test]
    async fn test_upload_activity_with_invalid_fit_file() {
        let client = StravaClient::new();
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let invalid_fit_data = vec![0u8; 5]; // Too small

        let result = client
            .upload_activity(&ride_id, &invalid_fit_data, Some("Test Ride"), None)
            .await;

        assert!(matches!(result, Err(SyncError::InvalidFitFile(_))));
    }
}

/// HTTP mocked tests using wiremock
#[cfg(test)]
mod http_mocked_tests {
    use super::*;
    use wiremock::matchers::{bearer_token, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a valid FIT file header for testing
    fn create_valid_fit_data() -> Vec<u8> {
        let mut data = vec![0u8; 16];
        data[0] = 14; // Header size (14 bytes)
        data[1] = 0x10; // Protocol version
        data[2] = 0x00; // Profile version LSB
        data[3] = 0x00; // Profile version MSB
        data[4] = 0x00; // Data size LSB
        data[5] = 0x00;
        data[6] = 0x00;
        data[7] = 0x00; // Data size MSB
        // ".FIT" signature at bytes 8-11
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        data[12] = 0x00; // CRC LSB
        data[13] = 0x00; // CRC MSB
        data
    }

    // ============================================================================
    // Upload Activity Tests
    // ============================================================================

    #[tokio::test]
    async fn test_upload_activity_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345678,
            "external_id": "test-ride-uuid",
            "activity_id": null,
            "status": "Your activity is still being processed.",
            "error": null
        }"#;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(201).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client
            .upload_activity(&ride_id, &fit_data, Some("Test Ride"), Some("A test ride"))
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.ride_id, ride_id);
        assert_eq!(record.platform, SyncPlatform::Strava);
        assert_eq!(record.status, SyncRecordStatus::Uploading);
        assert_eq!(record.external_id, Some("12345678".to_string()));
    }

    #[tokio::test]
    async fn test_upload_activity_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_upload_activity_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_upload_activity_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Bad Request",
            "errors": [
                {"resource": "Upload", "field": "file", "code": "invalid"}
            ]
        }"#;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("Bad Request")));
    }

    #[tokio::test]
    async fn test_upload_activity_generic_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("500")));
    }

    #[tokio::test]
    async fn test_upload_activity_without_optional_fields() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 99999,
            "external_id": "ride-uuid",
            "activity_id": null,
            "status": "Your activity is still being processed."
        }"#;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(201).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        // Test without name and description
        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.external_id, Some("99999".to_string()));
    }

    #[tokio::test]
    async fn test_upload_activity_duplicate_detection() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "The activity appears to be a duplicate.",
            "errors": []
        }"#;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(result, Err(SyncError::DuplicateActivity(SyncPlatform::Strava))));
    }

    // ============================================================================
    // Check Upload Status Tests
    // ============================================================================

    #[tokio::test]
    async fn test_check_upload_status_processing() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": "Your activity is still being processed.",
            "error": null
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UploadStatus::Processing);
    }

    #[tokio::test]
    async fn test_check_upload_status_ready() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": 987654321,
            "status": "Your activity is ready.",
            "error": null
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            UploadStatus::Ready {
                activity_id: 987654321
            }
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_error() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": null,
            "error": "The activity appears to be a duplicate."
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            UploadStatus::Error {
                error: "The activity appears to be a duplicate.".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_check_upload_status_duplicate() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": null,
            "error": "The activity appears to be a duplicate."
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), UploadStatus::Duplicate { error } if error.contains("duplicate")));
    }

    #[tokio::test]
    async fn test_check_upload_status_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_check_upload_status_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/uploads/99999"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("99999").await;

        assert!(matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("not found")));
    }

    #[tokio::test]
    async fn test_check_upload_status_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Server Error",
            "errors": []
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(500).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("Server Error")));
    }

    #[tokio::test]
    async fn test_check_upload_status_empty_error_treated_as_processing() {
        let mock_server = MockServer::start().await;

        // Some edge case where error is empty string
        let response_body = r#"{
            "id": 12345,
            "external_id": "test-uuid",
            "activity_id": null,
            "status": "Processing...",
            "error": ""
        }"#;

        Mock::given(method("GET"))
            .and(path("/uploads/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        // Empty error string should be treated as still processing
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UploadStatus::Processing);
    }

    // ============================================================================
    // Get Athlete Tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_athlete_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345678,
            "username": "cyclist123",
            "firstname": "John",
            "lastname": "Doe",
            "profile_medium": "https://example.com/medium.jpg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/athlete"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(result.is_ok());
        let athlete = result.unwrap();
        assert_eq!(athlete.id, 12345678);
        assert_eq!(athlete.username, Some("cyclist123".to_string()));
        assert_eq!(athlete.firstname, "John");
        assert_eq!(athlete.lastname, "Doe");
        assert_eq!(
            athlete.profile_medium,
            Some("https://example.com/medium.jpg".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_athlete_minimal_response() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 99999,
            "username": null,
            "firstname": "Jane",
            "lastname": "Smith",
            "profile_medium": null
        }"#;

        Mock::given(method("GET"))
            .and(path("/athlete"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(result.is_ok());
        let athlete = result.unwrap();
        assert_eq!(athlete.id, 99999);
        assert!(athlete.username.is_none());
        assert_eq!(athlete.firstname, "Jane");
        assert_eq!(athlete.lastname, "Smith");
        assert!(athlete.profile_medium.is_none());
        // Test display name when username is None
        assert_eq!(athlete.display_name(), "Jane Smith");
    }

    #[tokio::test]
    async fn test_get_athlete_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/athlete"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_get_athlete_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/athlete"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_get_athlete_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Resource Not Found"
        }"#;

        Mock::given(method("GET"))
            .and(path("/athlete"))
            .respond_with(ResponseTemplate::new(404).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("Resource Not Found")));
    }

    // ============================================================================
    // Deauthorize Tests
    // ============================================================================

    #[tokio::test]
    async fn test_deauthorize_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/deauthorize"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"access_token": "revoked"}"#))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        assert!(client.is_configured());

        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured()); // Token should be cleared
    }

    #[tokio::test]
    async fn test_deauthorize_rate_limit_still_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/deauthorize"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with rate limit since token is cleared locally
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_deauthorize_unauthorized_still_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/deauthorize"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with 401 since token was already invalid
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_deauthorize_server_error_still_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/deauthorize"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with server error since local token is cleared
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_deauthorize_api_error_with_body_still_succeeds() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Something went wrong",
            "errors": []
        }"#;

        Mock::given(method("POST"))
            .and(path("/deauthorize"))
            .respond_with(ResponseTemplate::new(403).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = StravaClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    // ============================================================================
    // Token State Tests
    // ============================================================================

    #[tokio::test]
    async fn test_clear_token() {
        let client = StravaClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());

        client.clear_token().await;
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_is_configured_with_token() {
        let client = StravaClient::new();
        assert!(!client.is_configured());

        client.set_access_token("abc123".to_string()).await;
        assert!(client.is_configured());
    }
}
