//! TrainingPeaks API Integration
//!
//! T001: Create TrainingPeaks API client module with OAuth and activity upload support.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
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
}
