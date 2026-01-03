//! TrainingPeaks API Integration
//!
//! T001: Create TrainingPeaks API client module with OAuth and activity upload support.
//! T010: Create types for TrainingPeaks workout API responses and mapping to internal types.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use crate::workouts::types::{
    CadenceTarget, PowerTarget, SegmentType, Workout, WorkoutFormat, WorkoutSegment,
};
use chrono::{NaiveDate, Utc};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Upload timeout in seconds (longer for file uploads)
const UPLOAD_TIMEOUT_SECS: u64 = 120;

/// TrainingPeaks API base URL
const TRAININGPEAKS_API_BASE_URL: &str = "https://api.trainingpeaks.com/v1";

/// TrainingPeaks OAuth base URL
const TRAININGPEAKS_OAUTH_BASE_URL: &str = "https://oauth.trainingpeaks.com";

/// FIT file header magic bytes
const FIT_HEADER_SIZE: u8 = 14;
const FIT_HEADER_SIGNATURE: &[u8] = b".FIT";

/// TrainingPeaks upload API response
#[derive(Debug, Deserialize)]
struct TrainingPeaksUploadResponse {
    /// Upload ID for status checking
    #[serde(rename = "Id")]
    id: String,
    /// File ID (only present when processing complete)
    #[serde(rename = "FileId")]
    #[allow(dead_code)]
    file_id: Option<i64>,
    /// Workout ID if created
    #[serde(rename = "WorkoutId")]
    #[allow(dead_code)]
    workout_id: Option<i64>,
    /// Processing status
    #[serde(rename = "Status")]
    #[allow(dead_code)]
    status: Option<String>,
    /// Error message if processing failed
    #[serde(rename = "Error")]
    #[allow(dead_code)]
    error: Option<String>,
}

/// TrainingPeaks athlete API response
#[derive(Debug, Deserialize)]
struct TrainingPeaksAthleteResponse {
    /// Athlete ID
    #[serde(rename = "Id")]
    id: i64,
    /// First name
    #[serde(rename = "FirstName")]
    firstname: String,
    /// Last name
    #[serde(rename = "LastName")]
    lastname: String,
    /// Email address
    #[serde(rename = "Email")]
    #[allow(dead_code)]
    email: Option<String>,
    /// Profile image URL (optional)
    #[serde(rename = "ProfilePhotoUrl")]
    profile_photo_url: Option<String>,
}

/// TrainingPeaks API error response
#[derive(Debug, Deserialize)]
struct TrainingPeaksApiError {
    /// Error message
    #[serde(rename = "Message")]
    message: String,
    /// Error code (optional)
    #[serde(rename = "ErrorCode")]
    #[allow(dead_code)]
    error_code: Option<String>,
    /// Error details (optional)
    #[serde(rename = "Errors")]
    #[serde(default)]
    errors: Vec<TrainingPeaksFieldError>,
}

/// TrainingPeaks field-level error detail
#[derive(Debug, Deserialize)]
struct TrainingPeaksFieldError {
    /// Field name
    #[serde(rename = "Field")]
    #[allow(dead_code)]
    field: String,
    /// Error code
    #[serde(rename = "Code")]
    code: String,
}

impl std::fmt::Display for TrainingPeaksApiError {
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

/// TrainingPeaks workout from API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TPWorkout {
    /// Workout ID
    #[serde(rename = "Id")]
    pub id: i64,
    /// Workout title
    #[serde(rename = "Title")]
    pub title: String,
    /// Workout description
    #[serde(rename = "Description")]
    pub description: Option<String>,
    /// Workout type (Bike, Run, Swim, etc.)
    #[serde(rename = "WorkoutType")]
    pub workout_type: String,
    /// Scheduled date (ISO 8601)
    #[serde(rename = "WorkoutDay")]
    pub workout_day: String,
    /// Total duration in seconds
    #[serde(rename = "TotalTime")]
    pub total_time: Option<f64>,
    /// Target TSS (Training Stress Score)
    #[serde(rename = "TSSPlanned")]
    pub tss_planned: Option<f64>,
    /// Target IF (Intensity Factor)
    #[serde(rename = "IFPlanned")]
    pub if_planned: Option<f64>,
    /// Structured workout data (if available)
    #[serde(rename = "Structure")]
    pub structure: Option<TPWorkoutStructure>,
}

/// TrainingPeaks workout structure containing steps/intervals
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TPWorkoutStructure {
    /// Primary length type (Duration, Distance)
    #[serde(rename = "PrimaryLengthMetric")]
    pub primary_length_metric: Option<String>,
    /// Primary intensity type (Power, HeartRate, Pace)
    #[serde(rename = "PrimaryIntensityMetric")]
    pub primary_intensity_metric: Option<String>,
    /// Workout steps
    #[serde(rename = "Steps")]
    pub steps: Vec<TPWorkoutStep>,
}

/// TrainingPeaks workout step (interval or rest)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TPWorkoutStep {
    /// Step type (Interval, Rest, Warmup, Cooldown, etc.)
    #[serde(rename = "Type")]
    pub step_type: String,
    /// Step name/description
    #[serde(rename = "Name")]
    pub name: Option<String>,
    /// Duration in seconds
    #[serde(rename = "Length")]
    pub length: Option<f64>,
    /// Length type (Duration, Distance)
    #[serde(rename = "LengthMetric")]
    pub length_metric: Option<String>,
    /// Target range for primary intensity metric
    #[serde(rename = "Targets")]
    pub targets: Option<Vec<TPWorkoutTarget>>,
    /// Nested steps for repeat intervals
    #[serde(rename = "Steps")]
    pub steps: Option<Vec<TPWorkoutStep>>,
    /// Number of repetitions for repeat steps
    #[serde(rename = "Reps")]
    pub reps: Option<u32>,
}

/// TrainingPeaks workout target (power zone, heart rate, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TPWorkoutTarget {
    /// Target type (Power, HeartRate, Pace, Cadence)
    #[serde(rename = "Type")]
    pub target_type: String,
    /// Minimum value
    #[serde(rename = "MinValue")]
    pub min_value: Option<f64>,
    /// Maximum value
    #[serde(rename = "MaxValue")]
    pub max_value: Option<f64>,
    /// Unit (Watts, BPM, etc.)
    #[serde(rename = "Unit")]
    pub unit: Option<String>,
}

/// TrainingPeaks API client
#[allow(dead_code)]
pub struct TrainingPeaksClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL (for /v1 endpoints)
    base_url: String,
    /// OAuth base URL (for token endpoints)
    oauth_base_url: String,
    /// HTTP client for API requests
    http_client: Client,
}

impl Default for TrainingPeaksClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingPeaksClient {
    /// Create a new TrainingPeaks client
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            access_token: Arc::new(RwLock::new(None)),
            base_url: TRAININGPEAKS_API_BASE_URL.to_string(),
            oauth_base_url: TRAININGPEAKS_OAUTH_BASE_URL.to_string(),
            http_client,
        }
    }

    /// Create a new TrainingPeaks client with custom base URLs (for testing)
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

    /// Upload a FIT file to TrainingPeaks
    ///
    /// Returns the sync record with upload status. TrainingPeaks processes uploads
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
    /// * `DuplicateActivity` - If the activity was already uploaded to TrainingPeaks
    /// * `RateLimited` - If TrainingPeaks' rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        fit_data: &[u8],
        _activity_name: Option<&str>,
        _description: Option<&str>,
    ) -> Result<SyncRecord, SyncError> {
        // Validate FIT file before attempting upload
        Self::validate_fit_file(fit_data)?;

        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        let record_id = Uuid::new_v4();

        tracing::info!(
            "Uploading activity {} to TrainingPeaks (record: {}, size: {} bytes)",
            ride_id,
            record_id,
            fit_data.len()
        );

        // Build multipart form
        let filename = format!("{}.fit", ride_id);

        // Create the file part with proper MIME type
        let file_part = Part::bytes(fit_data.to_vec())
            .file_name(filename)
            .mime_str("application/octet-stream")
            .map_err(|e| SyncError::UploadFailed(format!("Failed to create file part: {}", e)))?;

        // TrainingPeaks uses a simple file upload endpoint
        let form = Form::new().part("file", file_part);

        // Send the upload request with extended timeout for file uploads
        let url = format!("{}/file", self.base_url);
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
                    tracing::warn!(
                        "TrainingPeaks upload request timed out after {} seconds",
                        UPLOAD_TIMEOUT_SECS
                    );
                    SyncError::Timeout(UPLOAD_TIMEOUT_SECS)
                } else if e.is_connect() {
                    tracing::warn!("Failed to connect to TrainingPeaks: {}", e);
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
            tracing::warn!("TrainingPeaks API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "TrainingPeaks API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            // Try to parse as TrainingPeaks error response
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                let error_msg = error_response.to_string();

                // Check for duplicate activity error
                if Self::is_duplicate_error(&error_msg) {
                    tracing::info!("Activity {} already exists on TrainingPeaks", ride_id);
                    return Err(SyncError::DuplicateActivity(SyncPlatform::TrainingPeaks));
                }

                tracing::error!("TrainingPeaks upload failed: {}", error_msg);
                return Err(SyncError::UploadFailed(format!(
                    "TrainingPeaks error: {}",
                    error_msg
                )));
            }
            // Fall back to generic error
            tracing::error!(
                "TrainingPeaks upload failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::UploadFailed(format!(
                "Upload failed with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let upload_response: TrainingPeaksUploadResponse =
            serde_json::from_str(&body).map_err(|e| {
                SyncError::UploadFailed(format!("Failed to parse upload response: {}", e))
            })?;

        tracing::info!(
            "TrainingPeaks upload initiated successfully, upload_id: {}",
            upload_response.id
        );

        // Create record with Uploading status (async processing)
        // Store upload_id in external_id for status checking
        let record = SyncRecord {
            id: record_id,
            ride_id: *ride_id,
            platform: SyncPlatform::TrainingPeaks,
            status: SyncRecordStatus::Uploading,
            external_id: Some(upload_response.id),
            external_url: None, // Will be set when processing completes
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            retry_count: 0,
        };

        tracing::debug!("TrainingPeaks upload initiated: {:?}", record);

        Ok(record)
    }

    /// Check upload status
    ///
    /// TrainingPeaks processes uploads asynchronously, so we need to poll for status.
    /// Returns `UploadStatus::Ready` with `file_id` when processing is complete,
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
    /// * `RateLimited` - If TrainingPeaks' rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn check_upload_status(&self, upload_id: &str) -> Result<UploadStatus, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        tracing::debug!("Checking TrainingPeaks upload status: {}", upload_id);

        let url = format!("{}/file/{}", self.base_url, upload_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    tracing::warn!(
                        "TrainingPeaks status check timed out after {} seconds",
                        DEFAULT_TIMEOUT_SECS
                    );
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
            tracing::warn!("TrainingPeaks API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "TrainingPeaks API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle not found (404) - upload may have been deleted or invalid ID
        if status_code == reqwest::StatusCode::NOT_FOUND {
            tracing::warn!("TrainingPeaks upload {} not found", upload_id);
            return Err(SyncError::ApiError(format!(
                "Upload {} not found",
                upload_id
            )));
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                tracing::error!("TrainingPeaks status check failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "TrainingPeaks error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "TrainingPeaks status check failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Status check failed with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let upload_response: TrainingPeaksUploadResponse =
            serde_json::from_str(&body).map_err(|e| {
                SyncError::ApiError(format!("Failed to parse upload status response: {}", e))
            })?;

        // Determine status based on response fields
        // Priority: error > file_id > processing
        if let Some(error) = upload_response.error {
            if !error.is_empty() {
                // Check for duplicate activity in processing error
                if Self::is_duplicate_error(&error) {
                    tracing::info!(
                        "TrainingPeaks upload {} detected as duplicate: {}",
                        upload_id,
                        error
                    );
                    return Ok(UploadStatus::Duplicate { error });
                }
                tracing::warn!("TrainingPeaks upload {} failed: {}", upload_id, error);
                return Ok(UploadStatus::Error { error });
            }
        }

        if let Some(file_id) = upload_response.file_id {
            tracing::info!(
                "TrainingPeaks upload {} complete, file_id: {}",
                upload_id,
                file_id
            );
            return Ok(UploadStatus::Ready { file_id });
        }

        tracing::debug!(
            "TrainingPeaks upload {} still processing: {:?}",
            upload_id,
            upload_response.status
        );
        Ok(UploadStatus::Processing)
    }

    /// Get athlete profile
    ///
    /// Fetches the authenticated athlete's profile from TrainingPeaks.
    ///
    /// # Returns
    /// The athlete profile including id, name, and profile image URL
    ///
    /// # Errors
    /// * `RateLimited` - If TrainingPeaks' rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn get_athlete(&self) -> Result<AthleteProfile, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        tracing::debug!("Fetching TrainingPeaks athlete profile");

        let url = format!("{}/athlete/profile", self.base_url);

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
            tracing::warn!("TrainingPeaks API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "TrainingPeaks API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                tracing::error!("TrainingPeaks athlete fetch failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "TrainingPeaks error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "TrainingPeaks athlete fetch failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Failed to fetch athlete with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let athlete_response: TrainingPeaksAthleteResponse =
            serde_json::from_str(&body).map_err(|e| {
                SyncError::ApiError(format!("Failed to parse athlete response: {}", e))
            })?;

        tracing::info!(
            "Fetched TrainingPeaks athlete profile: {} {} (id: {})",
            athlete_response.firstname,
            athlete_response.lastname,
            athlete_response.id
        );

        // Convert to AthleteProfile
        Ok(AthleteProfile {
            id: athlete_response.id,
            firstname: athlete_response.firstname,
            lastname: athlete_response.lastname,
            profile_photo_url: athlete_response.profile_photo_url,
        })
    }

    /// Deauthorize application
    ///
    /// Revokes the application's access to the user's TrainingPeaks account.
    /// This invalidates the access token and clears the local token.
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
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        tracing::info!("Deauthorizing TrainingPeaks");

        // POST to TrainingPeaks' token revocation endpoint
        let url = format!("{}/oauth/deauthorize", self.oauth_base_url);

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
                "Failed to call TrainingPeaks deauthorize endpoint: {}. Local token cleared.",
                e
            );
            SyncError::NetworkError(format!("Failed to deauthorize: {}", e))
        })?;

        let status_code = response.status();

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!(
                "TrainingPeaks API rate limit exceeded during deauthorization. Local token cleared."
            );
            // Token is already cleared locally, so this is still a success from user perspective
            return Ok(());
        }

        // Handle unauthorized (401) - token was already invalid/revoked
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::info!(
                "TrainingPeaks token was already invalid or revoked. Local token cleared."
            );
            return Ok(());
        }

        // Handle other errors
        if !status_code.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                tracing::warn!(
                    "TrainingPeaks deauthorize returned error: {}. Local token cleared.",
                    error_response
                );
            } else {
                tracing::warn!(
                    "TrainingPeaks deauthorize returned status {}: {}. Local token cleared.",
                    status_code,
                    body
                );
            }
            // Still consider this a success since local token is cleared
            return Ok(());
        }

        tracing::info!("Successfully deauthorized from TrainingPeaks");

        Ok(())
    }

    /// Fetch scheduled workouts for a date range
    ///
    /// Retrieves all workouts scheduled within the specified date range from
    /// the TrainingPeaks API. The date range is inclusive of both start and end dates.
    ///
    /// # Arguments
    /// * `start_date` - The start date of the range (inclusive)
    /// * `end_date` - The end date of the range (inclusive)
    ///
    /// # Returns
    /// A vector of `TPWorkout` structs representing the scheduled workouts.
    ///
    /// # Errors
    /// * `NotConfigured` - If no access token is configured
    /// * `RateLimited` - If TrainingPeaks' rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    /// * `ApiError` - If the API returned an error response
    pub async fn get_scheduled_workouts(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<TPWorkout>, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        tracing::debug!(
            "Fetching TrainingPeaks workouts from {} to {}",
            start_date,
            end_date
        );

        // Format dates as YYYY-MM-DD for the API query parameters
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        let url = format!(
            "{}/workouts?startDate={}&endDate={}",
            self.base_url, start_date_str, end_date_str
        );

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
                    SyncError::NetworkError(format!("Failed to fetch workouts: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("TrainingPeaks API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "TrainingPeaks API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                tracing::error!("TrainingPeaks workout fetch failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "TrainingPeaks error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "TrainingPeaks workout fetch failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Failed to fetch workouts with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response - API returns an array of workouts
        let workouts: Vec<TPWorkout> = serde_json::from_str(&body).map_err(|e| {
            SyncError::ApiError(format!("Failed to parse workouts response: {}", e))
        })?;

        tracing::info!(
            "Fetched {} workouts from TrainingPeaks for {} to {}",
            workouts.len(),
            start_date,
            end_date
        );

        Ok(workouts)
    }

    /// Get workout details by ID
    ///
    /// Fetches the full workout structure with steps/intervals from TrainingPeaks.
    /// This provides more detailed workout structure than the scheduled workouts list.
    ///
    /// # Arguments
    /// * `workout_id` - The TrainingPeaks workout ID to fetch
    ///
    /// # Returns
    /// The full workout structure including all steps and intervals
    ///
    /// # Errors
    /// * `NotConfigured` - If no access token is set
    /// * `RateLimited` - If TrainingPeaks' rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    /// * `ApiError` - If the API returned an error response (including not found)
    pub async fn get_workout_details(&self, workout_id: i64) -> Result<TPWorkout, SyncError> {
        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        tracing::debug!("Fetching TrainingPeaks workout details for ID {}", workout_id);

        let url = format!("{}/workouts/{}", self.base_url, workout_id);

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
                    SyncError::NetworkError(format!("Failed to fetch workout details: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("TrainingPeaks API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "TrainingPeaks API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle not found (404)
        if status_code == reqwest::StatusCode::NOT_FOUND {
            tracing::warn!("TrainingPeaks workout {} not found", workout_id);
            return Err(SyncError::ApiError(format!(
                "Workout {} not found",
                workout_id
            )));
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksApiError>(&body) {
                tracing::error!("TrainingPeaks workout fetch failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "TrainingPeaks error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "TrainingPeaks workout fetch failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Failed to fetch workout with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response - API returns a single workout object
        let workout: TPWorkout = serde_json::from_str(&body).map_err(|e| {
            SyncError::ApiError(format!("Failed to parse workout response: {}", e))
        })?;

        tracing::info!(
            "Fetched workout details for TrainingPeaks workout {} ({})",
            workout_id,
            workout.title
        );

        Ok(workout)
    }
}

/// TrainingPeaks upload status
#[derive(Debug, Clone, PartialEq)]
pub enum UploadStatus {
    /// Still being processed
    Processing,
    /// Successfully processed
    Ready { file_id: i64 },
    /// Processing failed
    Error { error: String },
    /// Activity is a duplicate (already uploaded)
    Duplicate { error: String },
}

/// TrainingPeaks athlete profile
#[derive(Debug, Clone)]
pub struct AthleteProfile {
    pub id: i64,
    pub firstname: String,
    pub lastname: String,
    pub profile_photo_url: Option<String>,
}

impl AthleteProfile {
    /// Get display name
    pub fn display_name(&self) -> String {
        format!("{} {}", self.firstname, self.lastname)
    }
}

/// TrainingPeaks OAuth scopes
pub mod scopes {
    /// Read athlete profile
    pub const ATHLETE_PROFILE: &str = "athlete:profile";
    /// Read workouts
    pub const WORKOUTS_READ: &str = "workouts:read";
    /// Write/upload files (activities)
    pub const FILE_WRITE: &str = "file:write";
}

/// Get default OAuth scopes for TrainingPeaks
pub fn default_scopes() -> Vec<String> {
    vec![
        scopes::ATHLETE_PROFILE.to_string(),
        scopes::WORKOUTS_READ.to_string(),
        scopes::FILE_WRITE.to_string(),
    ]
}

// ============================================================================
// TrainingPeaks Workout Conversion (T010)
// ============================================================================

/// Error type for workout conversion failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkoutConversionError {
    pub message: String,
}

impl std::fmt::Display for WorkoutConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Workout conversion error: {}", self.message)
    }
}

impl std::error::Error for WorkoutConversionError {}

impl WorkoutConversionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl TPWorkout {
    /// Convert a TrainingPeaks workout to the internal Workout format.
    ///
    /// This handles converting the TrainingPeaks structured workout format into
    /// our internal representation with segments, power targets, and cadence targets.
    ///
    /// # Arguments
    /// * `ftp` - Optional FTP value for calculating absolute power values. If not provided,
    ///           percentage-based targets will be used where possible.
    ///
    /// # Returns
    /// A `Workout` struct or an error if conversion fails.
    pub fn to_workout(&self, ftp: Option<u16>) -> Result<Workout, WorkoutConversionError> {
        let segments = self.convert_structure_to_segments(ftp)?;

        if segments.is_empty() {
            return Err(WorkoutConversionError::new(
                "Workout has no valid segments to convert",
            ));
        }

        let total_duration_seconds: u32 = segments.iter().map(|s| s.duration_seconds).sum();

        let mut workout = Workout {
            id: Uuid::new_v4(),
            name: self.title.clone(),
            description: self.description.clone(),
            author: Some("TrainingPeaks".to_string()),
            source_file: None,
            source_format: Some(WorkoutFormat::TrainingPeaks),
            segments,
            total_duration_seconds,
            estimated_tss: self.tss_planned.map(|v| v as f32),
            estimated_if: self.if_planned.map(|v| v as f32),
            tags: vec!["TrainingPeaks".to_string()],
            created_at: Utc::now(),
        };

        // If FTP is provided, recalculate estimates
        if let Some(ftp_value) = ftp {
            workout.calculate_estimates(ftp_value);
        }

        Ok(workout)
    }

    /// Convert workout structure to internal segments
    fn convert_structure_to_segments(
        &self,
        ftp: Option<u16>,
    ) -> Result<Vec<WorkoutSegment>, WorkoutConversionError> {
        let structure = match &self.structure {
            Some(s) => s,
            None => {
                // If no structure, create a single segment from total time
                if let Some(total_time) = self.total_time {
                    let duration = total_time as u32;
                    if duration > 0 {
                        // Use IF to estimate power target if available
                        let power_target = if let Some(if_planned) = self.if_planned {
                            let percent = (if_planned * 100.0) as u8;
                            PowerTarget::PercentFtp { percent }
                        } else {
                            // Default to 75% FTP (endurance)
                            PowerTarget::PercentFtp { percent: 75 }
                        };

                        return Ok(vec![WorkoutSegment {
                            segment_type: SegmentType::SteadyState,
                            duration_seconds: duration,
                            power_target,
                            cadence_target: None,
                            text_event: self.description.clone(),
                        }]);
                    }
                }
                return Err(WorkoutConversionError::new(
                    "Workout has no structure and no total time",
                ));
            }
        };

        let mut segments = Vec::new();
        for step in &structure.steps {
            self.convert_step_to_segments(step, &mut segments, ftp, None)?;
        }

        Ok(segments)
    }

    /// Recursively convert a TrainingPeaks step (and any nested steps) to segments
    fn convert_step_to_segments(
        &self,
        step: &TPWorkoutStep,
        segments: &mut Vec<WorkoutSegment>,
        ftp: Option<u16>,
        repeat_count: Option<u32>,
    ) -> Result<(), WorkoutConversionError> {
        // Handle repeat steps
        if let Some(nested_steps) = &step.steps {
            let reps = step.reps.unwrap_or(1);
            for _rep in 0..reps {
                for nested_step in nested_steps {
                    self.convert_step_to_segments(nested_step, segments, ftp, Some(reps))?;
                }
            }
            return Ok(());
        }

        // Convert single step to segment
        let segment = self.step_to_segment(step, ftp)?;
        segments.push(segment);

        Ok(())
    }

    /// Convert a single TrainingPeaks step to a WorkoutSegment
    fn step_to_segment(
        &self,
        step: &TPWorkoutStep,
        ftp: Option<u16>,
    ) -> Result<WorkoutSegment, WorkoutConversionError> {
        // Determine segment type
        let segment_type = map_step_type_to_segment_type(&step.step_type);

        // Get duration
        let duration_seconds = step.length.map(|l| l as u32).unwrap_or(0);
        if duration_seconds == 0 {
            return Err(WorkoutConversionError::new(format!(
                "Step '{}' has no duration",
                step.name.as_deref().unwrap_or("unknown")
            )));
        }

        // Convert power target
        let power_target = self.extract_power_target(step, ftp, segment_type)?;

        // Convert cadence target
        let cadence_target = self.extract_cadence_target(step);

        Ok(WorkoutSegment {
            segment_type,
            duration_seconds,
            power_target,
            cadence_target,
            text_event: step.name.clone(),
        })
    }

    /// Extract power target from a TrainingPeaks step (T013 enhanced)
    ///
    /// Handles:
    /// - Power targets with absolute watts or percentage of FTP
    /// - Power zone targets (Zone 1-7) mapped to FTP percentages
    /// - Range power targets for warmup/cooldown/ramp segments
    fn extract_power_target(
        &self,
        step: &TPWorkoutStep,
        ftp: Option<u16>,
        segment_type: SegmentType,
    ) -> Result<PowerTarget, WorkoutConversionError> {
        let targets = match &step.targets {
            Some(t) if !t.is_empty() => t,
            _ => {
                // No explicit target, use defaults based on segment type
                // For warmup/cooldown, provide Range targets for gradual power change
                return Ok(self.default_power_for_segment(segment_type));
            }
        };

        // First check for PowerZone target (T013: power zone handling)
        if let Some(zone_target) = targets.iter().find(|t| t.target_type == "PowerZone") {
            return self.extract_power_zone_target(zone_target, segment_type);
        }

        // Find power target
        let power_target = targets.iter().find(|t| t.target_type == "Power");

        match power_target {
            Some(target) => {
                self.extract_power_value_target(target, ftp, segment_type)
            }
            None => {
                // No power target found, use defaults
                Ok(self.default_power_for_segment(segment_type))
            }
        }
    }

    /// Get default power target for a segment type (T013)
    fn default_power_for_segment(&self, segment_type: SegmentType) -> PowerTarget {
        match segment_type {
            SegmentType::Warmup => {
                // Warmup: gradual increase from 40% to 75% FTP
                PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(75),
                )
            }
            SegmentType::Cooldown => {
                // Cooldown: gradual decrease from 65% to 40% FTP
                PowerTarget::range(
                    PowerTarget::percent_ftp(65),
                    PowerTarget::percent_ftp(40),
                )
            }
            SegmentType::Ramp => {
                // Ramp: default gradual increase
                PowerTarget::range(
                    PowerTarget::percent_ftp(50),
                    PowerTarget::percent_ftp(100),
                )
            }
            SegmentType::FreeRide => PowerTarget::PercentFtp { percent: 0 },
            _ => PowerTarget::PercentFtp { percent: 75 },
        }
    }

    /// Extract power target from a PowerZone target type (T013)
    fn extract_power_zone_target(
        &self,
        target: &TPWorkoutTarget,
        segment_type: SegmentType,
    ) -> Result<PowerTarget, WorkoutConversionError> {
        // Try to parse zone from min_value (zone number)
        let zone = if let Some(zone_num) = target.min_value {
            zone_num as u8
        } else if let Some(ref unit) = target.unit {
            // Try to parse zone from unit string like "Zone 3"
            parse_power_zone(unit).unwrap_or(3)
        } else {
            3 // Default to Zone 3 (Tempo)
        };

        let (min_pct, max_pct) = power_zone_to_percent_range(zone);

        // For ramp segments, use the full zone range; otherwise use midpoint
        if segment_type_uses_range_power(segment_type) {
            Ok(PowerTarget::range(
                PowerTarget::percent_ftp(min_pct),
                PowerTarget::percent_ftp(max_pct),
            ))
        } else {
            // Use midpoint of zone range
            let avg_pct = (min_pct + max_pct) / 2;
            Ok(PowerTarget::PercentFtp { percent: avg_pct })
        }
    }

    /// Extract power target from a Power value target (T013 enhanced)
    fn extract_power_value_target(
        &self,
        target: &TPWorkoutTarget,
        ftp: Option<u16>,
        segment_type: SegmentType,
    ) -> Result<PowerTarget, WorkoutConversionError> {
        let min_value = target.min_value.unwrap_or(0.0);
        let max_value = target.max_value.unwrap_or(min_value);

        // Determine if values are percentages or absolute watts
        // TrainingPeaks typically uses "PercentOfFtp" unit for percentages
        let is_percent = target
            .unit
            .as_ref()
            .map(|u| u.to_lowercase().contains("percent"))
            .unwrap_or(false);

        // Check if this is a ramp/warmup/cooldown segment with a range
        let has_range = (max_value - min_value).abs() > 1.0;
        let use_range = has_range && segment_type_uses_range_power(segment_type);

        if is_percent {
            if use_range {
                // Use Range power target for warmup/cooldown/ramp with min→max
                let start_pct = min_value as u8;
                let end_pct = max_value as u8;
                Ok(PowerTarget::range(
                    PowerTarget::percent_ftp(start_pct),
                    PowerTarget::percent_ftp(end_pct),
                ))
            } else {
                // Average the min/max for a single percent target
                let avg_percent = ((min_value + max_value) / 2.0) as u8;
                Ok(PowerTarget::PercentFtp {
                    percent: avg_percent,
                })
            }
        } else {
            // Absolute watts
            if use_range && ftp.is_some() {
                let ftp_value = ftp.unwrap();
                if ftp_value > 0 {
                    let start_pct = ((min_value / ftp_value as f64) * 100.0) as u8;
                    let end_pct = ((max_value / ftp_value as f64) * 100.0) as u8;
                    return Ok(PowerTarget::range(
                        PowerTarget::percent_ftp(start_pct),
                        PowerTarget::percent_ftp(end_pct),
                    ));
                }
            }

            // Use midpoint of range
            let avg_watts = ((min_value + max_value) / 2.0) as u16;

            // If we have FTP, convert to percentage for flexibility
            if let Some(ftp_value) = ftp {
                if ftp_value > 0 {
                    let percent = ((avg_watts as f32 / ftp_value as f32) * 100.0) as u8;
                    return Ok(PowerTarget::PercentFtp { percent });
                }
            }

            Ok(PowerTarget::Absolute { watts: avg_watts })
        }
    }

    /// Extract cadence target from a TrainingPeaks step
    fn extract_cadence_target(&self, step: &TPWorkoutStep) -> Option<CadenceTarget> {
        let targets = step.targets.as_ref()?;

        // Find cadence target
        let cadence_target = targets.iter().find(|t| t.target_type == "Cadence")?;

        let min_rpm = cadence_target.min_value.unwrap_or(80.0) as u8;
        let max_rpm = cadence_target.max_value.unwrap_or(min_rpm as f64 + 10.0) as u8;

        Some(CadenceTarget { min_rpm, max_rpm })
    }
}

/// Map TrainingPeaks step type string to internal SegmentType
fn map_step_type_to_segment_type(step_type: &str) -> SegmentType {
    match step_type.to_lowercase().as_str() {
        "warmup" | "warm up" | "warm-up" => SegmentType::Warmup,
        "cooldown" | "cool down" | "cool-down" => SegmentType::Cooldown,
        "interval" | "work" | "on" => SegmentType::SteadyState,
        "rest" | "recovery" | "off" => SegmentType::SteadyState, // Recovery is still steady state at lower power
        "ramp" | "rampup" | "rampdown" => SegmentType::Ramp,
        "freeride" | "free ride" | "free" => SegmentType::FreeRide,
        "repeat" | "repetition" => SegmentType::Intervals,
        _ => SegmentType::SteadyState, // Default to steady state
    }
}

// ============================================================================
// Power Zone Handling (T013)
// ============================================================================

/// Standard power zones with FTP percentage ranges (based on Coggan zones)
/// Returns (min_percent, max_percent) for the zone
fn power_zone_to_percent_range(zone: u8) -> (u8, u8) {
    match zone {
        1 => (0, 55),     // Active Recovery: <55%
        2 => (55, 75),    // Endurance: 55-75%
        3 => (75, 90),    // Tempo: 75-90%
        4 => (90, 105),   // Threshold: 90-105%
        5 => (105, 120),  // VO2max: 105-120%
        6 => (120, 150),  // Anaerobic: 120-150%
        7 => (150, 200),  // Neuromuscular: >150%
        _ => (75, 90),    // Default to Tempo (Zone 3)
    }
}

/// Parse a power zone from a string like "Zone 3", "Z3", "3", etc.
fn parse_power_zone(s: &str) -> Option<u8> {
    let s = s.trim().to_lowercase();

    // Try "zone N" or "zone N" patterns
    if let Some(rest) = s.strip_prefix("zone") {
        if let Ok(zone) = rest.trim().parse::<u8>() {
            if (1..=7).contains(&zone) {
                return Some(zone);
            }
        }
    }

    // Try "zN" pattern
    if let Some(rest) = s.strip_prefix('z') {
        if let Ok(zone) = rest.trim().parse::<u8>() {
            if (1..=7).contains(&zone) {
                return Some(zone);
            }
        }
    }

    // Try just a number
    if let Ok(zone) = s.parse::<u8>() {
        if (1..=7).contains(&zone) {
            return Some(zone);
        }
    }

    None
}

/// Check if a segment type should use Range power targets for gradual power changes
fn segment_type_uses_range_power(segment_type: SegmentType) -> bool {
    matches!(
        segment_type,
        SegmentType::Warmup | SegmentType::Cooldown | SegmentType::Ramp
    )
}

/// Convert a list of TrainingPeaks workouts to internal format
pub fn convert_tp_workouts(
    workouts: Vec<TPWorkout>,
    ftp: Option<u16>,
) -> Vec<Result<Workout, WorkoutConversionError>> {
    workouts
        .into_iter()
        .map(|tp_workout| tp_workout.to_workout(ftp))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TrainingPeaksClient::new();
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_set_token() {
        let client = TrainingPeaksClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());
    }

    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert!(scopes.contains(&scopes::ATHLETE_PROFILE.to_string()));
        assert!(scopes.contains(&scopes::WORKOUTS_READ.to_string()));
        assert!(scopes.contains(&scopes::FILE_WRITE.to_string()));
    }

    #[test]
    fn test_athlete_display_name() {
        let athlete = AthleteProfile {
            id: 123,
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_photo_url: None,
        };
        assert_eq!(athlete.display_name(), "John Doe");
    }

    #[tokio::test]
    async fn test_upload_activity_without_token_returns_not_configured() {
        let client = TrainingPeaksClient::new();
        let ride_id = Uuid::new_v4();
        let fit_data = vec![0u8; 100]; // Dummy FIT data

        let result = client
            .upload_activity(&ride_id, &fit_data, Some("Test Ride"), None)
            .await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_trainingpeaks_api_error_display() {
        let error = TrainingPeaksApiError {
            message: "Bad Request".to_string(),
            error_code: None,
            errors: vec![],
        };
        assert_eq!(format!("{}", error), "Bad Request");

        let error_with_details = TrainingPeaksApiError {
            message: "Bad Request".to_string(),
            error_code: Some("VALIDATION_ERROR".to_string()),
            errors: vec![TrainingPeaksFieldError {
                field: "file".to_string(),
                code: "invalid".to_string(),
            }],
        };
        assert_eq!(format!("{}", error_with_details), "Bad Request (invalid)");
    }

    #[test]
    fn test_trainingpeaks_upload_response_deserialization() {
        let json = r#"{
            "Id": "test-uuid-12345",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Processing",
            "Error": null
        }"#;

        let response: TrainingPeaksUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, "test-uuid-12345");
        assert!(response.file_id.is_none());
        assert!(response.workout_id.is_none());
    }

    #[test]
    fn test_trainingpeaks_upload_response_ready_state() {
        // Response when upload processing is complete
        let json = r#"{
            "Id": "test-uuid-12345",
            "FileId": 987654321,
            "WorkoutId": 123456,
            "Status": "Completed",
            "Error": null
        }"#;

        let response: TrainingPeaksUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, "test-uuid-12345");
        assert_eq!(response.file_id, Some(987654321));
        assert_eq!(response.workout_id, Some(123456));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_trainingpeaks_upload_response_error_state() {
        // Response when upload processing failed
        let json = r#"{
            "Id": "test-uuid-12345",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Failed",
            "Error": "The activity appears to be a duplicate."
        }"#;

        let response: TrainingPeaksUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, "test-uuid-12345");
        assert!(response.file_id.is_none());
        assert_eq!(
            response.error,
            Some("The activity appears to be a duplicate.".to_string())
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_without_token_returns_not_configured() {
        let client = TrainingPeaksClient::new();

        let result = client.check_upload_status("12345").await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_upload_status_enum_variants() {
        // Test that UploadStatus variants can be created and compared
        let processing = UploadStatus::Processing;
        let ready = UploadStatus::Ready { file_id: 12345 };
        let error = UploadStatus::Error {
            error: "Duplicate activity".to_string(),
        };

        assert_eq!(processing, UploadStatus::Processing);
        assert_eq!(ready, UploadStatus::Ready { file_id: 12345 });
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
        let client = TrainingPeaksClient::new();

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_without_token_returns_not_configured() {
        let client = TrainingPeaksClient::new();

        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 7).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[test]
    fn test_trainingpeaks_athlete_response_deserialization() {
        // Full response with all fields
        let json = r#"{
            "Id": 12345678,
            "FirstName": "John",
            "LastName": "Doe",
            "Email": "john@example.com",
            "ProfilePhotoUrl": "https://example.com/photo.jpg"
        }"#;

        let response: TrainingPeaksAthleteResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345678);
        assert_eq!(response.firstname, "John");
        assert_eq!(response.lastname, "Doe");
        assert_eq!(response.email, Some("john@example.com".to_string()));
        assert_eq!(
            response.profile_photo_url,
            Some("https://example.com/photo.jpg".to_string())
        );
    }

    #[test]
    fn test_trainingpeaks_athlete_response_minimal() {
        // Response with optional fields as null
        let json = r#"{
            "Id": 12345678,
            "FirstName": "Jane",
            "LastName": "Smith",
            "Email": null,
            "ProfilePhotoUrl": null
        }"#;

        let response: TrainingPeaksAthleteResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.id, 12345678);
        assert_eq!(response.firstname, "Jane");
        assert_eq!(response.lastname, "Smith");
        assert!(response.email.is_none());
        assert!(response.profile_photo_url.is_none());
    }

    #[tokio::test]
    async fn test_deauthorize_without_token_returns_not_configured() {
        let client = TrainingPeaksClient::new();

        let result = client.deauthorize().await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[tokio::test]
    async fn test_deauthorize_clears_local_token() {
        let client = TrainingPeaksClient::new();
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
        let result = TrainingPeaksClient::validate_fit_file(&valid_fit);
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
        let result = TrainingPeaksClient::validate_fit_file(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fit_file_too_small() {
        let tiny_data = vec![0u8; 5];
        let result = TrainingPeaksClient::validate_fit_file(&tiny_data);
        assert!(
            matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("too small"))
        );
    }

    #[test]
    fn test_validate_fit_file_invalid_header_size() {
        let mut data = vec![0u8; 20];
        data[0] = 50; // Invalid header size
        data[8] = b'.';
        data[9] = b'F';
        data[10] = b'I';
        data[11] = b'T';
        let result = TrainingPeaksClient::validate_fit_file(&data);
        assert!(
            matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("Invalid header size"))
        );
    }

    #[test]
    fn test_validate_fit_file_missing_signature() {
        let mut data = vec![0u8; 16];
        data[0] = 14; // Header size
        // Missing ".FIT" signature - just zeros
        let result = TrainingPeaksClient::validate_fit_file(&data);
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
        let result = TrainingPeaksClient::validate_fit_file(&data);
        assert!(
            matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("truncated"))
        );
    }

    // ========================================================================
    // Duplicate Detection Tests
    // ========================================================================

    #[test]
    fn test_is_duplicate_error_detection() {
        assert!(TrainingPeaksClient::is_duplicate_error(
            "The activity appears to be a duplicate."
        ));
        assert!(TrainingPeaksClient::is_duplicate_error("Activity already exists"));
        assert!(TrainingPeaksClient::is_duplicate_error(
            "This file has already uploaded"
        ));
        assert!(TrainingPeaksClient::is_duplicate_error("DUPLICATE activity detected"));

        // Should not match non-duplicate errors
        assert!(!TrainingPeaksClient::is_duplicate_error("Invalid file format"));
        assert!(!TrainingPeaksClient::is_duplicate_error("Rate limit exceeded"));
        assert!(!TrainingPeaksClient::is_duplicate_error("Server error"));
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
    // Workout Types Tests
    // ========================================================================

    #[test]
    fn test_tp_workout_deserialization() {
        let json = r#"{
            "Id": 12345,
            "Title": "Sweet Spot Base",
            "Description": "Build aerobic base with sweet spot intervals",
            "WorkoutType": "Bike",
            "WorkoutDay": "2024-01-15T00:00:00",
            "TotalTime": 3600.0,
            "TSSPlanned": 75.0,
            "IFPlanned": 0.85,
            "Structure": null
        }"#;

        let workout: TPWorkout =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(workout.id, 12345);
        assert_eq!(workout.title, "Sweet Spot Base");
        assert_eq!(workout.workout_type, "Bike");
        assert_eq!(workout.total_time, Some(3600.0));
        assert_eq!(workout.tss_planned, Some(75.0));
        assert_eq!(workout.if_planned, Some(0.85));
        assert!(workout.structure.is_none());
    }

    #[test]
    fn test_tp_workout_with_structure() {
        let json = r#"{
            "Id": 12345,
            "Title": "VO2max Intervals",
            "Description": null,
            "WorkoutType": "Bike",
            "WorkoutDay": "2024-01-15",
            "TotalTime": 3600.0,
            "TSSPlanned": null,
            "IFPlanned": null,
            "Structure": {
                "PrimaryLengthMetric": "Duration",
                "PrimaryIntensityMetric": "Power",
                "Steps": [
                    {
                        "Type": "Warmup",
                        "Name": "Easy spin",
                        "Length": 600.0,
                        "LengthMetric": "Duration",
                        "Targets": null,
                        "Steps": null,
                        "Reps": null
                    },
                    {
                        "Type": "Interval",
                        "Name": "VO2max",
                        "Length": 180.0,
                        "LengthMetric": "Duration",
                        "Targets": [
                            {
                                "Type": "Power",
                                "MinValue": 300.0,
                                "MaxValue": 320.0,
                                "Unit": "Watts"
                            }
                        ],
                        "Steps": null,
                        "Reps": null
                    }
                ]
            }
        }"#;

        let workout: TPWorkout =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert!(workout.structure.is_some());
        let structure = workout.structure.unwrap();
        assert_eq!(structure.primary_intensity_metric, Some("Power".to_string()));
        assert_eq!(structure.steps.len(), 2);
        assert_eq!(structure.steps[0].step_type, "Warmup");
        assert_eq!(structure.steps[1].step_type, "Interval");

        let targets = structure.steps[1].targets.as_ref().unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_type, "Power");
        assert_eq!(targets[0].min_value, Some(300.0));
        assert_eq!(targets[0].max_value, Some(320.0));
    }

    // ========================================================================
    // Workout Conversion Tests (T010)
    // ========================================================================

    #[test]
    fn test_workout_conversion_simple() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Sweet Spot".to_string(),
            description: Some("Endurance ride".to_string()),
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: Some(3600.0),
            tss_planned: Some(75.0),
            if_planned: Some(0.85),
            structure: None,
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();

        assert_eq!(workout.name, "Sweet Spot");
        assert_eq!(workout.description, Some("Endurance ride".to_string()));
        assert_eq!(workout.author, Some("TrainingPeaks".to_string()));
        assert_eq!(
            workout.source_format,
            Some(crate::workouts::types::WorkoutFormat::TrainingPeaks)
        );
        assert!(workout.tags.contains(&"TrainingPeaks".to_string()));
        assert_eq!(workout.segments.len(), 1);
        assert_eq!(workout.total_duration_seconds, 3600);
    }

    #[test]
    fn test_workout_conversion_with_structure() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "VO2max Intervals".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: Some(3600.0),
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![
                    TPWorkoutStep {
                        step_type: "Warmup".to_string(),
                        name: Some("Easy spin".to_string()),
                        length: Some(600.0),
                        length_metric: Some("Duration".to_string()),
                        targets: None,
                        steps: None,
                        reps: None,
                    },
                    TPWorkoutStep {
                        step_type: "Interval".to_string(),
                        name: Some("VO2max effort".to_string()),
                        length: Some(180.0),
                        length_metric: Some("Duration".to_string()),
                        targets: Some(vec![TPWorkoutTarget {
                            target_type: "Power".to_string(),
                            min_value: Some(300.0),
                            max_value: Some(320.0),
                            unit: Some("Watts".to_string()),
                        }]),
                        steps: None,
                        reps: None,
                    },
                    TPWorkoutStep {
                        step_type: "Cooldown".to_string(),
                        name: Some("Easy spin down".to_string()),
                        length: Some(300.0),
                        length_metric: Some("Duration".to_string()),
                        targets: None,
                        steps: None,
                        reps: None,
                    },
                ],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();

        assert_eq!(workout.name, "VO2max Intervals");
        assert_eq!(workout.segments.len(), 3);

        // Check warmup segment
        assert_eq!(
            workout.segments[0].segment_type,
            crate::workouts::types::SegmentType::Warmup
        );
        assert_eq!(workout.segments[0].duration_seconds, 600);
        assert_eq!(workout.segments[0].text_event, Some("Easy spin".to_string()));

        // Check interval segment
        assert_eq!(
            workout.segments[1].segment_type,
            crate::workouts::types::SegmentType::SteadyState
        );
        assert_eq!(workout.segments[1].duration_seconds, 180);

        // Check cooldown segment
        assert_eq!(
            workout.segments[2].segment_type,
            crate::workouts::types::SegmentType::Cooldown
        );
        assert_eq!(workout.segments[2].duration_seconds, 300);
    }

    #[test]
    fn test_workout_conversion_with_repeats() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "5x3min VO2max".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![
                    TPWorkoutStep {
                        step_type: "Warmup".to_string(),
                        name: Some("Warmup".to_string()),
                        length: Some(600.0),
                        length_metric: Some("Duration".to_string()),
                        targets: None,
                        steps: None,
                        reps: None,
                    },
                    TPWorkoutStep {
                        step_type: "Repeat".to_string(),
                        name: Some("Main set".to_string()),
                        length: None,
                        length_metric: None,
                        targets: None,
                        steps: Some(vec![
                            TPWorkoutStep {
                                step_type: "Interval".to_string(),
                                name: Some("VO2max".to_string()),
                                length: Some(180.0),
                                length_metric: Some("Duration".to_string()),
                                targets: Some(vec![TPWorkoutTarget {
                                    target_type: "Power".to_string(),
                                    min_value: Some(110.0),
                                    max_value: Some(120.0),
                                    unit: Some("PercentOfFtp".to_string()),
                                }]),
                                steps: None,
                                reps: None,
                            },
                            TPWorkoutStep {
                                step_type: "Rest".to_string(),
                                name: Some("Recovery".to_string()),
                                length: Some(180.0),
                                length_metric: Some("Duration".to_string()),
                                targets: Some(vec![TPWorkoutTarget {
                                    target_type: "Power".to_string(),
                                    min_value: Some(40.0),
                                    max_value: Some(50.0),
                                    unit: Some("PercentOfFtp".to_string()),
                                }]),
                                steps: None,
                                reps: None,
                            },
                        ]),
                        reps: Some(5),
                    },
                ],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();

        // Should have 1 warmup + 5x(interval + rest) = 11 segments
        assert_eq!(workout.segments.len(), 11);
        assert_eq!(workout.name, "5x3min VO2max");

        // Verify first segment is warmup
        assert_eq!(
            workout.segments[0].segment_type,
            crate::workouts::types::SegmentType::Warmup
        );

        // Verify repeat structure (segments 1, 3, 5, 7, 9 should be intervals)
        for i in (1..11).step_by(2) {
            assert_eq!(workout.segments[i].duration_seconds, 180);
            assert_eq!(
                workout.segments[i].text_event,
                Some("VO2max".to_string())
            );
        }

        // Segments 2, 4, 6, 8, 10 should be recovery
        for i in (2..11).step_by(2) {
            assert_eq!(workout.segments[i].duration_seconds, 180);
            assert_eq!(
                workout.segments[i].text_event,
                Some("Recovery".to_string())
            );
        }
    }

    #[test]
    fn test_workout_conversion_with_cadence_target() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "High Cadence".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Interval".to_string(),
                    name: Some("High cadence drills".to_string()),
                    length: Some(300.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![
                        TPWorkoutTarget {
                            target_type: "Power".to_string(),
                            min_value: Some(200.0),
                            max_value: Some(220.0),
                            unit: Some("Watts".to_string()),
                        },
                        TPWorkoutTarget {
                            target_type: "Cadence".to_string(),
                            min_value: Some(100.0),
                            max_value: Some(110.0),
                            unit: Some("RPM".to_string()),
                        },
                    ]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();

        assert_eq!(workout.segments.len(), 1);
        let segment = &workout.segments[0];

        // Check cadence target is converted
        assert!(segment.cadence_target.is_some());
        let cadence = segment.cadence_target.as_ref().unwrap();
        assert_eq!(cadence.min_rpm, 100);
        assert_eq!(cadence.max_rpm, 110);
    }

    #[test]
    fn test_workout_conversion_error_no_structure_no_time() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Empty Workout".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: None,
        };

        let result = tp_workout.to_workout(Some(250));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("no structure and no total time"));
    }

    #[test]
    fn test_workout_conversion_error_empty_structure() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Empty Structure".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: None,
                primary_intensity_metric: None,
                steps: vec![],
            }),
        };

        let result = tp_workout.to_workout(Some(250));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("no valid segments"));
    }

    #[test]
    fn test_map_step_type_to_segment_type() {
        assert_eq!(
            map_step_type_to_segment_type("Warmup"),
            crate::workouts::types::SegmentType::Warmup
        );
        assert_eq!(
            map_step_type_to_segment_type("warm up"),
            crate::workouts::types::SegmentType::Warmup
        );
        assert_eq!(
            map_step_type_to_segment_type("Cooldown"),
            crate::workouts::types::SegmentType::Cooldown
        );
        assert_eq!(
            map_step_type_to_segment_type("Interval"),
            crate::workouts::types::SegmentType::SteadyState
        );
        assert_eq!(
            map_step_type_to_segment_type("FreeRide"),
            crate::workouts::types::SegmentType::FreeRide
        );
        assert_eq!(
            map_step_type_to_segment_type("Ramp"),
            crate::workouts::types::SegmentType::Ramp
        );
        // Default case
        assert_eq!(
            map_step_type_to_segment_type("Unknown"),
            crate::workouts::types::SegmentType::SteadyState
        );
    }

    #[test]
    fn test_workout_conversion_error_display() {
        let error = WorkoutConversionError::new("test error message");
        assert_eq!(
            format!("{}", error),
            "Workout conversion error: test error message"
        );
    }

    #[test]
    fn test_convert_tp_workouts_batch() {
        let workouts = vec![
            TPWorkout {
                id: 1,
                title: "Workout 1".to_string(),
                description: None,
                workout_type: "Bike".to_string(),
                workout_day: "2024-01-15".to_string(),
                total_time: Some(3600.0),
                tss_planned: None,
                if_planned: None,
                structure: None,
            },
            TPWorkout {
                id: 2,
                title: "Workout 2".to_string(),
                description: None,
                workout_type: "Bike".to_string(),
                workout_day: "2024-01-16".to_string(),
                total_time: Some(5400.0),
                tss_planned: None,
                if_planned: None,
                structure: None,
            },
        ];

        let results = convert_tp_workouts(workouts, Some(250));
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        let w1 = results[0].as_ref().unwrap();
        let w2 = results[1].as_ref().unwrap();
        assert_eq!(w1.name, "Workout 1");
        assert_eq!(w2.name, "Workout 2");
        assert_eq!(w1.total_duration_seconds, 3600);
        assert_eq!(w2.total_duration_seconds, 5400);
    }

    #[test]
    fn test_power_target_absolute_watts_conversion() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Absolute Power".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Interval".to_string(),
                    name: Some("Sweet spot".to_string()),
                    length: Some(1200.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "Power".to_string(),
                        min_value: Some(220.0),
                        max_value: Some(230.0),
                        unit: Some("Watts".to_string()),
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        // With FTP of 250, 225W avg would be 90% FTP
        let workout = tp_workout.to_workout(Some(250)).unwrap();

        assert_eq!(workout.segments.len(), 1);
        let segment = &workout.segments[0];

        // Should be converted to percent FTP since FTP was provided
        match &segment.power_target {
            crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                assert_eq!(*percent, 90); // 225 / 250 = 0.9 = 90%
            }
            _ => panic!("Expected PercentFtp target"),
        }
    }

    #[test]
    fn test_power_target_percent_ftp_conversion() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Percent FTP".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Interval".to_string(),
                    name: Some("Sweet spot".to_string()),
                    length: Some(1200.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "Power".to_string(),
                        min_value: Some(88.0),
                        max_value: Some(94.0),
                        unit: Some("PercentOfFtp".to_string()),
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();

        assert_eq!(workout.segments.len(), 1);
        let segment = &workout.segments[0];

        // Should preserve percent FTP target
        match &segment.power_target {
            crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                assert_eq!(*percent, 91); // (88 + 94) / 2 = 91
            }
            _ => panic!("Expected PercentFtp target"),
        }
    }

    // ========================================================================
    // Power Zone Handling Tests (T013)
    // ========================================================================

    #[test]
    fn test_power_zone_to_percent_range() {
        // Zone 1: Active Recovery
        assert_eq!(power_zone_to_percent_range(1), (0, 55));
        // Zone 2: Endurance
        assert_eq!(power_zone_to_percent_range(2), (55, 75));
        // Zone 3: Tempo
        assert_eq!(power_zone_to_percent_range(3), (75, 90));
        // Zone 4: Threshold
        assert_eq!(power_zone_to_percent_range(4), (90, 105));
        // Zone 5: VO2max
        assert_eq!(power_zone_to_percent_range(5), (105, 120));
        // Zone 6: Anaerobic
        assert_eq!(power_zone_to_percent_range(6), (120, 150));
        // Zone 7: Neuromuscular
        assert_eq!(power_zone_to_percent_range(7), (150, 200));
        // Invalid zone defaults to Zone 3
        assert_eq!(power_zone_to_percent_range(0), (75, 90));
        assert_eq!(power_zone_to_percent_range(8), (75, 90));
    }

    #[test]
    fn test_parse_power_zone() {
        // "Zone N" format
        assert_eq!(parse_power_zone("Zone 3"), Some(3));
        assert_eq!(parse_power_zone("zone 4"), Some(4));
        assert_eq!(parse_power_zone("Zone1"), Some(1));
        assert_eq!(parse_power_zone(" zone 5 "), Some(5));

        // "ZN" format
        assert_eq!(parse_power_zone("Z3"), Some(3));
        assert_eq!(parse_power_zone("z2"), Some(2));

        // Just number format
        assert_eq!(parse_power_zone("3"), Some(3));
        assert_eq!(parse_power_zone("7"), Some(7));

        // Invalid formats
        assert_eq!(parse_power_zone("Zone 8"), None);
        assert_eq!(parse_power_zone("Zone 0"), None);
        assert_eq!(parse_power_zone("invalid"), None);
        assert_eq!(parse_power_zone(""), None);
    }

    #[test]
    fn test_segment_type_uses_range_power() {
        assert!(segment_type_uses_range_power(SegmentType::Warmup));
        assert!(segment_type_uses_range_power(SegmentType::Cooldown));
        assert!(segment_type_uses_range_power(SegmentType::Ramp));
        assert!(!segment_type_uses_range_power(SegmentType::SteadyState));
        assert!(!segment_type_uses_range_power(SegmentType::Intervals));
        assert!(!segment_type_uses_range_power(SegmentType::FreeRide));
    }

    #[test]
    fn test_warmup_uses_range_power_target() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Warmup Test".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Warmup".to_string(),
                    name: Some("Easy warmup".to_string()),
                    length: Some(600.0),
                    length_metric: Some("Duration".to_string()),
                    targets: None, // No explicit target, should use default Range
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();
        assert_eq!(workout.segments.len(), 1);
        let segment = &workout.segments[0];

        // Should be a Range target for warmup (40% → 75%)
        match &segment.power_target {
            crate::workouts::types::PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                        crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                    ) => {
                        assert_eq!(*s, 40);
                        assert_eq!(*e, 75);
                    }
                    _ => panic!("Expected PercentFtp in Range"),
                }
            }
            _ => panic!("Expected Range target for warmup"),
        }
    }

    #[test]
    fn test_cooldown_uses_range_power_target() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Cooldown Test".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Cooldown".to_string(),
                    name: Some("Easy spin down".to_string()),
                    length: Some(300.0),
                    length_metric: Some("Duration".to_string()),
                    targets: None,
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();
        let segment = &workout.segments[0];

        // Should be a Range target for cooldown (65% → 40%)
        match &segment.power_target {
            crate::workouts::types::PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                        crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                    ) => {
                        assert_eq!(*s, 65);
                        assert_eq!(*e, 40);
                    }
                    _ => panic!("Expected PercentFtp in Range"),
                }
            }
            _ => panic!("Expected Range target for cooldown"),
        }
    }

    #[test]
    fn test_warmup_with_explicit_range_targets() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Warmup Ramp".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Warmup".to_string(),
                    name: Some("Progressive warmup".to_string()),
                    length: Some(600.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "Power".to_string(),
                        min_value: Some(50.0),
                        max_value: Some(80.0),
                        unit: Some("PercentOfFtp".to_string()),
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();
        let segment = &workout.segments[0];

        // Should use explicit range 50% → 80%
        match &segment.power_target {
            crate::workouts::types::PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                        crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                    ) => {
                        assert_eq!(*s, 50);
                        assert_eq!(*e, 80);
                    }
                    _ => panic!("Expected PercentFtp in Range"),
                }
            }
            _ => panic!("Expected Range target for warmup with explicit range"),
        }
    }

    #[test]
    fn test_power_zone_target_conversion() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Zone 4 Effort".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Interval".to_string(),
                    name: Some("Threshold work".to_string()),
                    length: Some(1200.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "PowerZone".to_string(),
                        min_value: Some(4.0), // Zone 4
                        max_value: Some(4.0),
                        unit: None,
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();
        let segment = &workout.segments[0];

        // Zone 4 should be average of 90-105% = 97%
        match &segment.power_target {
            crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                assert_eq!(*percent, 97); // (90 + 105) / 2 = 97
            }
            _ => panic!("Expected PercentFtp target for zone-based interval"),
        }
    }

    #[test]
    fn test_power_zone_for_ramp_uses_range() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Zone Ramp".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Ramp".to_string(),
                    name: Some("Zone 3 ramp".to_string()),
                    length: Some(300.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "PowerZone".to_string(),
                        min_value: Some(3.0), // Zone 3: 75-90%
                        max_value: Some(3.0),
                        unit: None,
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        let workout = tp_workout.to_workout(Some(250)).unwrap();
        let segment = &workout.segments[0];

        // Ramp with zone should use full zone range
        match &segment.power_target {
            crate::workouts::types::PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                        crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                    ) => {
                        assert_eq!(*s, 75); // Zone 3 min
                        assert_eq!(*e, 90); // Zone 3 max
                    }
                    _ => panic!("Expected PercentFtp in Range"),
                }
            }
            _ => panic!("Expected Range target for ramp with zone"),
        }
    }

    #[test]
    fn test_warmup_with_watts_range() {
        let tp_workout = TPWorkout {
            id: 12345,
            title: "Warmup Watts".to_string(),
            description: None,
            workout_type: "Bike".to_string(),
            workout_day: "2024-01-15".to_string(),
            total_time: None,
            tss_planned: None,
            if_planned: None,
            structure: Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![TPWorkoutStep {
                    step_type: "Warmup".to_string(),
                    name: Some("Progressive warmup".to_string()),
                    length: Some(600.0),
                    length_metric: Some("Duration".to_string()),
                    targets: Some(vec![TPWorkoutTarget {
                        target_type: "Power".to_string(),
                        min_value: Some(100.0), // 100W start
                        max_value: Some(200.0), // 200W end
                        unit: Some("Watts".to_string()),
                    }]),
                    steps: None,
                    reps: None,
                }],
            }),
        };

        // With FTP of 250, should convert 100W→40%, 200W→80%
        let workout = tp_workout.to_workout(Some(250)).unwrap();
        let segment = &workout.segments[0];

        match &segment.power_target {
            crate::workouts::types::PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                        crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                    ) => {
                        assert_eq!(*s, 40); // 100/250 = 40%
                        assert_eq!(*e, 80); // 200/250 = 80%
                    }
                    _ => panic!("Expected PercentFtp in Range"),
                }
            }
            _ => panic!("Expected Range target for warmup with watts range"),
        }
    }

    #[tokio::test]
    async fn test_upload_activity_with_invalid_fit_file() {
        let client = TrainingPeaksClient::new();
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let invalid_fit_data = vec![0u8; 5]; // Too small

        let result = client
            .upload_activity(&ride_id, &invalid_fit_data, Some("Test Ride"), None)
            .await;

        assert!(matches!(result, Err(SyncError::InvalidFitFile(_))));
    }

    // ========================================================================
    // Timeout Constant Tests
    // ========================================================================

    #[test]
    fn test_timeout_constants() {
        assert!(DEFAULT_TIMEOUT_SECS > 0);
        assert!(UPLOAD_TIMEOUT_SECS > DEFAULT_TIMEOUT_SECS);
    }

    #[tokio::test]
    async fn test_clear_token() {
        let client = TrainingPeaksClient::new();
        client.set_access_token("test_token".to_string()).await;
        assert!(client.is_configured());

        client.clear_token().await;
        assert!(!client.is_configured());
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
            "Id": "upload-12345678",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Processing",
            "Error": null
        }"#;

        Mock::given(method("POST"))
            .and(path("/file"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(201).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client
            .upload_activity(&ride_id, &fit_data, Some("Test Ride"), Some("A test ride"))
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.ride_id, ride_id);
        assert_eq!(record.platform, SyncPlatform::TrainingPeaks);
        assert_eq!(record.status, SyncRecordStatus::Uploading);
        assert_eq!(record.external_id, Some("upload-12345678".to_string()));
    }

    #[tokio::test]
    async fn test_upload_activity_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/file"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/file"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            "Message": "Bad Request",
            "ErrorCode": "VALIDATION_ERROR",
            "Errors": [
                {"Field": "file", "Code": "invalid"}
            ]
        }"#;

        Mock::given(method("POST"))
            .and(path("/file"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(
            matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("Bad Request"))
        );
    }

    #[tokio::test]
    async fn test_upload_activity_duplicate_detection() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "Message": "The activity already exists in your account.",
            "ErrorCode": "DUPLICATE",
            "Errors": []
        }"#;

        Mock::given(method("POST"))
            .and(path("/file"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data, None, None).await;

        assert!(matches!(
            result,
            Err(SyncError::DuplicateActivity(SyncPlatform::TrainingPeaks))
        ));
    }

    // ============================================================================
    // Check Upload Status Tests
    // ============================================================================

    #[tokio::test]
    async fn test_check_upload_status_processing() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": "12345",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Processing",
            "Error": null
        }"#;

        Mock::given(method("GET"))
            .and(path("/file/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UploadStatus::Processing);
    }

    #[tokio::test]
    async fn test_check_upload_status_ready() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": "12345",
            "FileId": 987654321,
            "WorkoutId": 123456,
            "Status": "Completed",
            "Error": null
        }"#;

        Mock::given(method("GET"))
            .and(path("/file/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UploadStatus::Ready { file_id: 987654321 });
    }

    #[tokio::test]
    async fn test_check_upload_status_error() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": "12345",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Failed",
            "Error": "Invalid file format"
        }"#;

        Mock::given(method("GET"))
            .and(path("/file/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            UploadStatus::Error {
                error: "Invalid file format".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_duplicate() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": "12345",
            "FileId": null,
            "WorkoutId": null,
            "Status": "Failed",
            "Error": "The activity appears to be a duplicate."
        }"#;

        Mock::given(method("GET"))
            .and(path("/file/12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("12345").await;

        assert!(result.is_ok());
        assert!(
            matches!(result.unwrap(), UploadStatus::Duplicate { error } if error.contains("duplicate"))
        );
    }

    #[tokio::test]
    async fn test_check_upload_status_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/file/99999"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.check_upload_status("99999").await;

        assert!(matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("not found")));
    }

    // ============================================================================
    // Get Athlete Tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_athlete_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": 12345678,
            "FirstName": "John",
            "LastName": "Doe",
            "Email": "john@example.com",
            "ProfilePhotoUrl": "https://example.com/photo.jpg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/athlete/profile"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(result.is_ok());
        let athlete = result.unwrap();
        assert_eq!(athlete.id, 12345678);
        assert_eq!(athlete.firstname, "John");
        assert_eq!(athlete.lastname, "Doe");
        assert_eq!(
            athlete.profile_photo_url,
            Some("https://example.com/photo.jpg".to_string())
        );
        assert_eq!(athlete.display_name(), "John Doe");
    }

    #[tokio::test]
    async fn test_get_athlete_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/athlete/profile"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_get_athlete_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/athlete/profile"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_athlete().await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    // ============================================================================
    // Deauthorize Tests
    // ============================================================================

    #[tokio::test]
    async fn test_deauthorize_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/oauth/deauthorize"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/oauth/deauthorize"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/oauth/deauthorize"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with 401 since token was already invalid
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    // ============================================================================
    // Get Scheduled Workouts Tests (T011)
    // ============================================================================

    #[tokio::test]
    async fn test_get_scheduled_workouts_success() {
        use wiremock::matchers::query_param;

        let mock_server = MockServer::start().await;

        let response_body = r#"[
            {
                "Id": 12345,
                "Title": "Morning Endurance Ride",
                "Description": "2 hour endurance ride",
                "WorkoutType": "Bike",
                "WorkoutDay": "2025-01-15",
                "TotalTime": 7200,
                "TSSPlanned": 100.0,
                "IFPlanned": 0.70,
                "Structure": null
            },
            {
                "Id": 12346,
                "Title": "Sweet Spot Intervals",
                "Description": "2x20 sweet spot",
                "WorkoutType": "Bike",
                "WorkoutDay": "2025-01-17",
                "TotalTime": 5400,
                "TSSPlanned": 80.0,
                "IFPlanned": 0.85,
                "Structure": null
            }
        ]"#;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .and(query_param("startDate", "2025-01-15"))
            .and(query_param("endDate", "2025-01-21"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let start_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 21).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(result.is_ok());
        let workouts = result.unwrap();
        assert_eq!(workouts.len(), 2);

        assert_eq!(workouts[0].id, 12345);
        assert_eq!(workouts[0].title, "Morning Endurance Ride");
        assert_eq!(workouts[0].workout_type, "Bike");
        assert_eq!(workouts[0].workout_day, "2025-01-15");
        assert_eq!(workouts[0].total_time, Some(7200.0));
        assert_eq!(workouts[0].tss_planned, Some(100.0));

        assert_eq!(workouts[1].id, 12346);
        assert_eq!(workouts[1].title, "Sweet Spot Intervals");
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_empty_result() {
        use wiremock::matchers::query_param;

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .and(query_param("startDate", "2025-02-01"))
            .and(query_param("endDate", "2025-02-07"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let start_date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 2, 7).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(result.is_ok());
        let workouts = result.unwrap();
        assert!(workouts.is_empty());
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 7).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 7).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "Message": "Invalid date range",
            "ErrorCode": "INVALID_PARAMETERS",
            "Errors": []
        }"#;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 7).unwrap();

        let result = client.get_scheduled_workouts(start_date, end_date).await;

        assert!(
            matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("Invalid date range"))
        );
    }

    #[tokio::test]
    async fn test_get_scheduled_workouts_with_structure() {
        use wiremock::matchers::query_param;

        let mock_server = MockServer::start().await;

        // Workout with full structure including steps
        let response_body = r#"[
            {
                "Id": 12347,
                "Title": "VO2max Intervals",
                "Description": "5x5 min VO2max with 5 min recovery",
                "WorkoutType": "Bike",
                "WorkoutDay": "2025-01-20",
                "TotalTime": 4500,
                "TSSPlanned": 120.0,
                "IFPlanned": 0.95,
                "Structure": {
                    "PrimaryLengthMetric": "Duration",
                    "PrimaryIntensityMetric": "Power",
                    "Steps": [
                        {
                            "Type": "Warmup",
                            "Name": "Warmup",
                            "Length": 600,
                            "LengthMetric": "Duration",
                            "Targets": [
                                {
                                    "Type": "Power",
                                    "MinValue": 100,
                                    "MaxValue": 150,
                                    "Unit": "Watts"
                                }
                            ]
                        },
                        {
                            "Type": "Interval",
                            "Name": "VO2max Work",
                            "Length": 300,
                            "LengthMetric": "Duration",
                            "Targets": [
                                {
                                    "Type": "Power",
                                    "MinValue": 320,
                                    "MaxValue": 350,
                                    "Unit": "Watts"
                                }
                            ]
                        }
                    ]
                }
            }
        ]"#;

        Mock::given(method("GET"))
            .and(path("/workouts"))
            .and(query_param("startDate", "2025-01-20"))
            .and(query_param("endDate", "2025-01-20"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

        let result = client.get_scheduled_workouts(date, date).await;

        assert!(result.is_ok());
        let workouts = result.unwrap();
        assert_eq!(workouts.len(), 1);

        let workout = &workouts[0];
        assert_eq!(workout.id, 12347);
        assert_eq!(workout.title, "VO2max Intervals");
        assert!(workout.structure.is_some());

        let structure = workout.structure.as_ref().unwrap();
        assert_eq!(structure.primary_length_metric, Some("Duration".to_string()));
        assert_eq!(structure.primary_intensity_metric, Some("Power".to_string()));
        assert_eq!(structure.steps.len(), 2);

        assert_eq!(structure.steps[0].step_type, "Warmup");
        assert_eq!(structure.steps[0].name, Some("Warmup".to_string()));
        assert_eq!(structure.steps[0].length, Some(600.0));

        assert_eq!(structure.steps[1].step_type, "Interval");
        assert_eq!(structure.steps[1].name, Some("VO2max Work".to_string()));
        assert_eq!(structure.steps[1].length, Some(300.0));
    }

    // ============================================================================
    // Get Workout Details Tests (T012)
    // ============================================================================

    #[tokio::test]
    async fn test_get_workout_details_without_token_returns_not_configured() {
        let client = TrainingPeaksClient::new();

        let result = client.get_workout_details(12345).await;

        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    #[tokio::test]
    async fn test_get_workout_details_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "Id": 12345,
            "Title": "Sweet Spot Intervals",
            "Description": "Build endurance with steady state efforts",
            "WorkoutType": "Bike",
            "WorkoutDay": "2025-01-15T00:00:00",
            "TotalTime": 3600.0,
            "TSSPlanned": 75.0,
            "IFPlanned": 0.85,
            "Structure": {
                "PrimaryLengthMetric": "Duration",
                "PrimaryIntensityMetric": "Power",
                "Steps": [
                    {
                        "Type": "Warmup",
                        "Name": "Easy spin",
                        "Length": 600.0,
                        "LengthMetric": "Duration"
                    },
                    {
                        "Type": "Interval",
                        "Name": "Sweet Spot",
                        "Length": 1200.0,
                        "LengthMetric": "Duration",
                        "Targets": [
                            {
                                "Type": "Power",
                                "MinValue": 250,
                                "MaxValue": 270,
                                "Unit": "Watts"
                            }
                        ]
                    },
                    {
                        "Type": "Rest",
                        "Name": "Recovery",
                        "Length": 300.0,
                        "LengthMetric": "Duration"
                    },
                    {
                        "Type": "Cooldown",
                        "Name": "Easy spin",
                        "Length": 600.0,
                        "LengthMetric": "Duration"
                    }
                ]
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/workouts/12345"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12345).await;

        assert!(result.is_ok());
        let workout = result.unwrap();
        assert_eq!(workout.id, 12345);
        assert_eq!(workout.title, "Sweet Spot Intervals");
        assert_eq!(
            workout.description,
            Some("Build endurance with steady state efforts".to_string())
        );
        assert_eq!(workout.workout_type, "Bike");
        assert_eq!(workout.total_time, Some(3600.0));
        assert_eq!(workout.tss_planned, Some(75.0));
        assert_eq!(workout.if_planned, Some(0.85));

        // Verify structure
        assert!(workout.structure.is_some());
        let structure = workout.structure.unwrap();
        assert_eq!(structure.primary_length_metric, Some("Duration".to_string()));
        assert_eq!(structure.primary_intensity_metric, Some("Power".to_string()));
        assert_eq!(structure.steps.len(), 4);

        // Verify steps
        assert_eq!(structure.steps[0].step_type, "Warmup");
        assert_eq!(structure.steps[0].name, Some("Easy spin".to_string()));
        assert_eq!(structure.steps[0].length, Some(600.0));

        assert_eq!(structure.steps[1].step_type, "Interval");
        assert_eq!(structure.steps[1].name, Some("Sweet Spot".to_string()));
        assert_eq!(structure.steps[1].length, Some(1200.0));
        assert!(structure.steps[1].targets.is_some());
        let targets = structure.steps[1].targets.as_ref().unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_type, "Power");
        assert_eq!(targets[0].min_value, Some(250.0));
        assert_eq!(targets[0].max_value, Some(270.0));
    }

    #[tokio::test]
    async fn test_get_workout_details_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts/99999"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(99999).await;

        assert!(
            matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("not found"))
        );
    }

    #[tokio::test]
    async fn test_get_workout_details_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts/12345"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12345).await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_get_workout_details_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/workouts/12345"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12345).await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_get_workout_details_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "Message": "Internal server error",
            "ErrorCode": "SERVER_ERROR",
            "Errors": []
        }"#;

        Mock::given(method("GET"))
            .and(path("/workouts/12345"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(500).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12345).await;

        assert!(
            matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("Internal server error"))
        );
    }

    #[tokio::test]
    async fn test_get_workout_details_with_repeat_steps() {
        let mock_server = MockServer::start().await;

        // Workout with nested repeat steps
        let response_body = r#"{
            "Id": 12346,
            "Title": "VO2max Repeats",
            "Description": "High intensity interval training",
            "WorkoutType": "Bike",
            "WorkoutDay": "2025-01-16T00:00:00",
            "TotalTime": 4200.0,
            "Structure": {
                "PrimaryLengthMetric": "Duration",
                "PrimaryIntensityMetric": "Power",
                "Steps": [
                    {
                        "Type": "Warmup",
                        "Name": "Easy warmup",
                        "Length": 600.0,
                        "LengthMetric": "Duration"
                    },
                    {
                        "Type": "Repeat",
                        "Reps": 5,
                        "Steps": [
                            {
                                "Type": "Interval",
                                "Name": "VO2max effort",
                                "Length": 180.0,
                                "LengthMetric": "Duration",
                                "Targets": [
                                    {
                                        "Type": "Power",
                                        "MinValue": 350,
                                        "MaxValue": 380,
                                        "Unit": "Watts"
                                    }
                                ]
                            },
                            {
                                "Type": "Rest",
                                "Name": "Recovery spin",
                                "Length": 180.0,
                                "LengthMetric": "Duration"
                            }
                        ]
                    },
                    {
                        "Type": "Cooldown",
                        "Name": "Easy spin",
                        "Length": 600.0,
                        "LengthMetric": "Duration"
                    }
                ]
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/workouts/12346"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12346).await;

        assert!(result.is_ok());
        let workout = result.unwrap();
        assert_eq!(workout.id, 12346);
        assert_eq!(workout.title, "VO2max Repeats");

        let structure = workout.structure.unwrap();
        assert_eq!(structure.steps.len(), 3);

        // Verify repeat step
        let repeat_step = &structure.steps[1];
        assert_eq!(repeat_step.step_type, "Repeat");
        assert_eq!(repeat_step.reps, Some(5));
        assert!(repeat_step.steps.is_some());

        let nested_steps = repeat_step.steps.as_ref().unwrap();
        assert_eq!(nested_steps.len(), 2);
        assert_eq!(nested_steps[0].step_type, "Interval");
        assert_eq!(nested_steps[0].name, Some("VO2max effort".to_string()));
        assert_eq!(nested_steps[1].step_type, "Rest");
    }

    #[tokio::test]
    async fn test_get_workout_details_without_structure() {
        let mock_server = MockServer::start().await;

        // Simple workout without structured data (e.g., free ride)
        let response_body = r#"{
            "Id": 12347,
            "Title": "Free Ride",
            "Description": "Easy endurance ride",
            "WorkoutType": "Bike",
            "WorkoutDay": "2025-01-17T00:00:00",
            "TotalTime": 7200.0,
            "TSSPlanned": 50.0
        }"#;

        Mock::given(method("GET"))
            .and(path("/workouts/12347"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = TrainingPeaksClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_workout_details(12347).await;

        assert!(result.is_ok());
        let workout = result.unwrap();
        assert_eq!(workout.id, 12347);
        assert_eq!(workout.title, "Free Ride");
        assert_eq!(workout.total_time, Some(7200.0));
        assert!(workout.structure.is_none());
    }
}
