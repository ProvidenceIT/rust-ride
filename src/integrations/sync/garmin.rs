//! Garmin Connect API Integration
//!
//! T105: Implement Garmin Connect API upload.
//!
//! This module provides comprehensive error handling for all Garmin Connect API
//! interactions, including:
//! - Rate limiting with retry-after support
//! - Token expiry and refresh handling
//! - Automatic token refresh on expiry
//! - Duplicate activity detection
//! - Network error recovery
//! - FIT file validation
//!
//! ## Upload Processing Model
//!
//! Garmin Connect uses **synchronous** upload processing, unlike Strava which
//! processes uploads asynchronously:
//!
//! - **Garmin Connect**: The upload API immediately returns the activity ID
//!   (or error) in the response. No status polling is required. The upload
//!   completes in a single request/response cycle.
//!
//! - **Strava**: Returns an upload_id immediately, then processes the file
//!   asynchronously. Requires calling `check_upload_status()` to poll for
//!   completion.
//!
//! This means GarminClient does NOT need a `check_upload_status()` method.
//! The `upload_activity()` method returns a `SyncRecord` with `status: Completed`
//! when successful, or an error if the upload fails.

use super::{SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::Utc;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum number of automatic token refresh attempts per API call
const MAX_AUTO_REFRESH_ATTEMPTS: u32 = 1;

/// Result type for token refresh operations
pub type TokenRefreshResult = Pin<Box<dyn Future<Output = Result<String, SyncError>> + Send>>;

/// Trait for providing token refresh functionality to GarminClient.
///
/// Implement this trait to enable automatic token refresh when the access token
/// expires during API calls. When a `TokenExpired` error is encountered, the
/// client will call `refresh_token()` to get a new access token and retry the
/// request.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// struct MyTokenRefresher {
///     oauth_handler: Arc<OAuthHandler>,
/// }
///
/// impl TokenRefresher for MyTokenRefresher {
///     fn refresh_token(&self) -> TokenRefreshResult {
///         let handler = self.oauth_handler.clone();
///         Box::pin(async move {
///             let tokens = handler.refresh_token(SyncPlatform::GarminConnect).await?;
///             Ok(tokens.access_token)
///         })
///     }
/// }
/// ```
pub trait TokenRefresher: Send + Sync {
    /// Refresh the access token and return the new token.
    ///
    /// This method is called when the client encounters a `TokenExpired` error.
    /// It should:
    /// 1. Use the stored refresh token to get new access/refresh tokens
    /// 2. Store the new tokens appropriately
    /// 3. Return the new access token
    ///
    /// # Returns
    /// A future that resolves to the new access token or an error.
    ///
    /// # Errors
    /// - `AuthorizationRequired` - If the refresh token is also expired
    /// - `RefreshFailed` - If the refresh request fails for other reasons
    /// - `NetworkError` - If the refresh request fails due to network issues
    fn refresh_token(&self) -> TokenRefreshResult;
}

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Upload timeout in seconds (longer for file uploads)
const UPLOAD_TIMEOUT_SECS: u64 = 120;

/// Default Garmin Connect API base URL
const GARMIN_API_BASE_URL: &str = "https://connect.garmin.com/modern/proxy";

/// Default Garmin Connect OAuth base URL (for token revocation)
const GARMIN_OAUTH_BASE_URL: &str = "https://connect.garmin.com/oauth-service/oauth";

/// FIT file header magic bytes
const FIT_HEADER_SIZE: u8 = 14;
const FIT_HEADER_SIGNATURE: &[u8] = b".FIT";

/// Default retry delay in seconds when rate limited without Retry-After header
const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;

/// Maximum retry attempts for transient errors
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Error categorization for retry and recovery logic
///
/// Classifies errors into categories that help determine the appropriate
/// recovery strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Transient errors that should be retried (network issues, timeouts)
    Transient,
    /// Rate limit errors that require waiting before retry
    RateLimited,
    /// Authentication errors requiring token refresh or re-authorization
    Authentication,
    /// Client errors that should not be retried (invalid data, bad request)
    Client,
    /// Server errors that may be retried after a delay
    Server,
    /// Permanent errors that should not be retried (duplicate activity)
    Permanent,
}

impl ErrorCategory {
    /// Check if errors in this category should be retried
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Transient | ErrorCategory::RateLimited | ErrorCategory::Server
        )
    }

    /// Get the recommended initial retry delay in seconds
    pub fn initial_retry_delay_secs(&self) -> u64 {
        match self {
            ErrorCategory::Transient => 5,
            ErrorCategory::RateLimited => DEFAULT_RATE_LIMIT_RETRY_SECS,
            ErrorCategory::Server => 30,
            _ => 0, // Non-retryable
        }
    }

    /// Get the maximum number of retry attempts for this category
    pub fn max_retry_attempts(&self) -> u32 {
        match self {
            ErrorCategory::Transient => MAX_RETRY_ATTEMPTS,
            ErrorCategory::RateLimited => 1, // Wait and retry once
            ErrorCategory::Server => 2,
            _ => 0, // Non-retryable
        }
    }
}

/// Rate limit information extracted from API response
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Seconds to wait before retrying (from Retry-After header or default)
    pub retry_after_secs: u64,
    /// Whether this is a hard limit (daily) vs soft limit (per-minute)
    pub is_hard_limit: bool,
}

impl Default for RateLimitInfo {
    fn default() -> Self {
        Self {
            retry_after_secs: DEFAULT_RATE_LIMIT_RETRY_SECS,
            is_hard_limit: false,
        }
    }
}

impl RateLimitInfo {
    /// Create rate limit info from HTTP response headers
    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> Self {
        // Try to parse Retry-After header
        let retry_after_secs = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_RATE_LIMIT_RETRY_SECS);

        // Check for daily limit indicator (varies by API)
        let is_hard_limit = headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|remaining| remaining == 0)
            .unwrap_or(false);

        Self {
            retry_after_secs,
            is_hard_limit,
        }
    }

    /// Get the retry delay as a Duration
    pub fn retry_delay(&self) -> Duration {
        Duration::from_secs(self.retry_after_secs)
    }
}

/// Extension trait for SyncError to support error categorization and retry logic
pub trait SyncErrorExt {
    /// Get the error category for retry/recovery logic
    fn category(&self) -> ErrorCategory;

    /// Check if this error should be retried
    fn is_retryable(&self) -> bool;

    /// Check if this error requires authentication refresh
    fn requires_auth_refresh(&self) -> bool;

    /// Check if this error is due to rate limiting
    fn is_rate_limited(&self) -> bool;

    /// Check if this error is due to a duplicate activity
    fn is_duplicate(&self) -> bool;

    /// Get the recommended retry delay in seconds
    fn retry_delay_secs(&self) -> u64;
}

impl SyncErrorExt for SyncError {
    fn category(&self) -> ErrorCategory {
        match self {
            SyncError::NotConfigured(_) => ErrorCategory::Client,
            SyncError::AuthorizationRequired => ErrorCategory::Authentication,
            SyncError::TokenExpired => ErrorCategory::Authentication,
            SyncError::RefreshFailed(_) => ErrorCategory::Authentication,
            SyncError::UploadFailed(msg) => {
                // Check for server errors in the message
                if msg.contains("500") || msg.contains("502") || msg.contains("503") {
                    ErrorCategory::Server
                } else {
                    ErrorCategory::Client
                }
            }
            SyncError::ApiError(msg) => {
                if msg.contains("500") || msg.contains("502") || msg.contains("503") {
                    ErrorCategory::Server
                } else {
                    ErrorCategory::Client
                }
            }
            SyncError::CredentialError(_) => ErrorCategory::Permanent,
            SyncError::NetworkError(_) => ErrorCategory::Transient,
            SyncError::DuplicateActivity(_) => ErrorCategory::Permanent,
            SyncError::InvalidFitFile(_) => ErrorCategory::Client,
            SyncError::Timeout(_) => ErrorCategory::Transient,
            SyncError::RateLimited => ErrorCategory::RateLimited,
        }
    }

    fn is_retryable(&self) -> bool {
        self.category().is_retryable()
    }

    fn requires_auth_refresh(&self) -> bool {
        matches!(self.category(), ErrorCategory::Authentication)
    }

    fn is_rate_limited(&self) -> bool {
        matches!(self, SyncError::RateLimited)
    }

    fn is_duplicate(&self) -> bool {
        matches!(self, SyncError::DuplicateActivity(_))
    }

    fn retry_delay_secs(&self) -> u64 {
        self.category().initial_retry_delay_secs()
    }
}

/// Garmin Connect upload API response
///
/// When uploading a FIT file, Garmin Connect returns detailed information
/// about the created activity/activities.
#[derive(Debug, Deserialize)]
struct GarminUploadResponse {
    /// Detailed information about the uploaded activities
    #[serde(rename = "detailedImportResult")]
    detailed_import_result: DetailedImportResult,
}

/// Detailed import result from Garmin Connect
#[derive(Debug, Deserialize)]
struct DetailedImportResult {
    /// Upload UUID assigned by Garmin
    #[serde(rename = "uploadUuid")]
    upload_uuid: Option<UploadUuid>,
    /// List of created activities
    #[serde(default)]
    successes: Vec<GarminUploadSuccess>,
    /// List of failed uploads
    #[serde(default)]
    failures: Vec<GarminUploadFailure>,
}

/// Upload UUID wrapper
#[derive(Debug, Deserialize)]
struct UploadUuid {
    /// The actual UUID string
    uuid: String,
}

/// Successful activity creation from Garmin upload
#[derive(Debug, Deserialize)]
struct GarminUploadSuccess {
    /// Internal activity ID assigned by Garmin
    #[serde(rename = "internalId")]
    internal_id: u64,
    /// External ID (matches what we sent)
    #[serde(rename = "externalId")]
    #[allow(dead_code)]
    external_id: Option<String>,
}

/// Failed upload information
#[derive(Debug, Deserialize)]
struct GarminUploadFailure {
    /// Internal activity ID (if partially processed)
    #[serde(rename = "internalId")]
    #[allow(dead_code)]
    internal_id: Option<u64>,
    /// External ID
    #[serde(rename = "externalId")]
    #[allow(dead_code)]
    external_id: Option<String>,
    /// Error messages
    #[serde(default)]
    messages: Vec<GarminUploadMessage>,
}

/// Upload message/error from Garmin
#[derive(Debug, Deserialize)]
struct GarminUploadMessage {
    /// Error code
    #[allow(dead_code)]
    code: Option<i32>,
    /// Error content/message
    content: Option<String>,
}

/// Garmin API error response
#[derive(Debug, Deserialize)]
struct GarminApiError {
    /// Error message
    message: Option<String>,
    /// Error code
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<String>,
    /// Detailed errors
    #[serde(default)]
    errors: Vec<GarminFieldError>,
}

/// Garmin field-level error detail
#[derive(Debug, Deserialize)]
struct GarminFieldError {
    /// Error message
    message: Option<String>,
    /// Field path
    #[allow(dead_code)]
    path: Option<String>,
}

/// Garmin social profile API response
///
/// Response from /userprofile-service/socialProfile endpoint
#[derive(Debug, Deserialize)]
struct GarminSocialProfileResponse {
    /// User ID
    #[serde(rename = "id")]
    user_id: u64,
    /// Display name (username)
    #[serde(rename = "displayName")]
    display_name: String,
    /// Full name (optional)
    #[serde(rename = "fullName")]
    full_name: Option<String>,
    /// Profile image URL (small)
    #[serde(rename = "profileImageUrlSmall")]
    profile_image_url_small: Option<String>,
    /// Profile image URL (medium)
    #[serde(rename = "profileImageUrlMedium")]
    profile_image_url_medium: Option<String>,
    /// Profile image URL (large)
    #[serde(rename = "profileImageUrlLarge")]
    profile_image_url_large: Option<String>,
}

impl std::fmt::Display for GarminApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref message) = self.message {
            if self.errors.is_empty() {
                write!(f, "{}", message)
            } else {
                let details: Vec<String> = self
                    .errors
                    .iter()
                    .filter_map(|e| e.message.clone())
                    .collect();
                if details.is_empty() {
                    write!(f, "{}", message)
                } else {
                    write!(f, "{} ({})", message, details.join(", "))
                }
            }
        } else if !self.errors.is_empty() {
            let details: Vec<String> = self
                .errors
                .iter()
                .filter_map(|e| e.message.clone())
                .collect();
            write!(f, "{}", details.join(", "))
        } else {
            write!(f, "Unknown Garmin API error")
        }
    }
}

/// Garmin Connect API client
///
/// Provides methods for interacting with the Garmin Connect API, including:
/// - FIT file upload
/// - User profile retrieval
/// - Token deauthorization
///
/// ## Automatic Token Refresh
///
/// The client supports automatic token refresh when access tokens expire.
/// When a `TokenRefresher` is set via `set_token_refresher()`, the client
/// will automatically attempt to refresh the token and retry the request
/// when a `TokenExpired` error is encountered.
///
/// ```ignore
/// let client = GarminClient::new();
/// client.set_token_refresher(Arc::new(my_refresher));
/// // Now API calls will automatically refresh the token if needed
/// ```
#[allow(dead_code)]
pub struct GarminClient {
    /// Access token for API calls
    access_token: Arc<RwLock<Option<String>>>,
    /// API base URL (for /upload-service, /userprofile-service, etc.)
    base_url: String,
    /// OAuth base URL (for /revoke endpoint)
    oauth_base_url: String,
    /// HTTP client for API requests
    http_client: Client,
    /// Optional token refresher for automatic token refresh
    token_refresher: Arc<RwLock<Option<Arc<dyn TokenRefresher>>>>,
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
            oauth_base_url: GARMIN_OAUTH_BASE_URL.to_string(),
            http_client,
            token_refresher: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new Garmin Connect client with custom base URLs (for testing)
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
            token_refresher: Arc::new(RwLock::new(None)),
        }
    }

    /// Set a token refresher for automatic token refresh.
    ///
    /// When a token refresher is set, the client will automatically attempt
    /// to refresh the access token and retry the request when a `TokenExpired`
    /// error is encountered during API calls.
    ///
    /// # Arguments
    /// * `refresher` - The token refresher implementation
    ///
    /// # Example
    /// ```ignore
    /// let client = GarminClient::new();
    /// client.set_token_refresher(Arc::new(my_refresher)).await;
    /// ```
    pub async fn set_token_refresher(&self, refresher: Arc<dyn TokenRefresher>) {
        *self.token_refresher.write().await = Some(refresher);
    }

    /// Clear the token refresher.
    ///
    /// After calling this, the client will no longer attempt automatic
    /// token refresh on `TokenExpired` errors.
    pub async fn clear_token_refresher(&self) {
        *self.token_refresher.write().await = None;
    }

    /// Check if a token refresher is configured.
    pub fn has_token_refresher(&self) -> bool {
        self.token_refresher
            .try_read()
            .map(|r| r.is_some())
            .unwrap_or(false)
    }

    /// Attempt to refresh the access token using the configured token refresher.
    ///
    /// This method is called internally when an API call fails with `TokenExpired`
    /// and a token refresher is configured.
    ///
    /// # Returns
    /// - `Ok(())` if the token was refreshed successfully
    /// - `Err(SyncError)` if no refresher is configured or refresh failed
    async fn attempt_token_refresh(&self) -> Result<(), SyncError> {
        let refresher = {
            let guard = self.token_refresher.read().await;
            guard.clone()
        };

        match refresher {
            Some(refresher) => {
                tracing::info!("Attempting automatic token refresh for Garmin Connect");

                match refresher.refresh_token().await {
                    Ok(new_token) => {
                        tracing::info!("Automatic token refresh successful");
                        self.set_access_token(new_token).await;
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("Automatic token refresh failed: {}", e);
                        Err(e)
                    }
                }
            }
            None => {
                tracing::debug!("No token refresher configured, cannot auto-refresh");
                Err(SyncError::TokenExpired)
            }
        }
    }

    /// Execute an async operation with automatic token refresh on expiry.
    ///
    /// This helper method wraps an API call and automatically handles token
    /// refresh when the operation fails with `TokenExpired`. If a token refresher
    /// is configured and the refresh succeeds, the operation is retried once.
    ///
    /// # Type Parameters
    /// * `F` - The async factory function that creates the operation
    /// * `T` - The return type of the operation
    ///
    /// # Arguments
    /// * `operation_name` - Name of the operation for logging
    /// * `operation_factory` - Factory function that creates the async operation
    ///
    /// # Returns
    /// The result of the operation, or an error if the operation and refresh both fail
    async fn with_auto_refresh<F, Fut, T>(
        &self,
        operation_name: &str,
        operation_factory: F,
    ) -> Result<T, SyncError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, SyncError>>,
    {
        let mut attempts = 0;

        loop {
            match operation_factory().await {
                Ok(result) => return Ok(result),
                Err(SyncError::TokenExpired) if attempts < MAX_AUTO_REFRESH_ATTEMPTS => {
                    attempts += 1;
                    tracing::info!(
                        "Token expired during {}, attempting refresh (attempt {}/{})",
                        operation_name,
                        attempts,
                        MAX_AUTO_REFRESH_ATTEMPTS
                    );

                    // Try to refresh the token
                    match self.attempt_token_refresh().await {
                        Ok(()) => {
                            tracing::info!("Token refreshed, retrying {}", operation_name);
                            // Continue loop to retry the operation
                        }
                        Err(refresh_error) => {
                            tracing::warn!(
                                "Token refresh failed during {}: {}",
                                operation_name,
                                refresh_error
                            );
                            // If refresh fails with AuthorizationRequired, propagate that
                            // Otherwise, return the original TokenExpired error
                            return Err(match refresh_error {
                                SyncError::AuthorizationRequired => SyncError::AuthorizationRequired,
                                _ => SyncError::TokenExpired,
                            });
                        }
                    }
                }
                Err(e) => return Err(e),
            }
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

    /// Handle common HTTP status code errors with consistent logging and error mapping.
    ///
    /// Returns `Some(SyncError)` for known error status codes, or `None` if the
    /// status code should be handled differently by the caller.
    ///
    /// This method handles:
    /// - 429 Too Many Requests → RateLimited
    /// - 401 Unauthorized → TokenExpired
    /// - 403 Forbidden → TokenExpired (often means invalid token)
    /// - 409 Conflict → DuplicateActivity
    fn handle_error_status(
        status: reqwest::StatusCode,
        context: &str,
    ) -> Option<SyncError> {
        match status {
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                tracing::warn!("Garmin Connect API rate limit exceeded during {}", context);
                Some(SyncError::RateLimited)
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                tracing::warn!(
                    "Garmin Connect API returned 401 Unauthorized during {} - token may be expired or revoked",
                    context
                );
                Some(SyncError::TokenExpired)
            }
            reqwest::StatusCode::FORBIDDEN => {
                tracing::warn!(
                    "Garmin Connect API returned 403 Forbidden during {} - token may be invalid",
                    context
                );
                Some(SyncError::TokenExpired)
            }
            reqwest::StatusCode::CONFLICT => {
                tracing::info!("Garmin Connect returned 409 Conflict during {} - likely duplicate", context);
                Some(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))
            }
            _ => None,
        }
    }

    /// Map a reqwest error to a SyncError with appropriate categorization.
    ///
    /// This method handles common network error cases:
    /// - Timeout errors → Timeout
    /// - Connection errors → NetworkError (connection failed)
    /// - Other errors → NetworkError (request failed)
    fn map_request_error(err: reqwest::Error, timeout_secs: u64, context: &str) -> SyncError {
        if err.is_timeout() {
            tracing::warn!(
                "Garmin Connect {} request timed out after {} seconds",
                context,
                timeout_secs
            );
            SyncError::Timeout(timeout_secs)
        } else if err.is_connect() {
            tracing::warn!("Failed to connect to Garmin Connect during {}: {}", context, err);
            SyncError::NetworkError(format!("Connection failed: {}", err))
        } else {
            tracing::warn!("Failed to send {} request to Garmin Connect: {}", context, err);
            SyncError::NetworkError(format!("Request failed: {}", err))
        }
    }

    /// Parse error response body and create appropriate SyncError.
    ///
    /// Attempts to parse the body as a Garmin API error response. If parsing
    /// succeeds, checks for duplicate activity indicators before returning
    /// a generic upload error.
    fn parse_error_response(
        status: reqwest::StatusCode,
        body: &str,
        context: &str,
    ) -> SyncError {
        // Try to parse as Garmin error response
        if let Ok(error_response) = serde_json::from_str::<GarminApiError>(body) {
            let error_msg = error_response.to_string();

            // Check for duplicate activity error
            if Self::is_duplicate_error(&error_msg) {
                tracing::info!("Garmin Connect {} detected as duplicate: {}", context, error_msg);
                return SyncError::DuplicateActivity(SyncPlatform::GarminConnect);
            }

            tracing::error!("Garmin Connect {} failed: {}", context, error_msg);
            return SyncError::UploadFailed(format!("Garmin error: {}", error_msg));
        }

        // Fall back to generic error with status code
        tracing::error!(
            "Garmin Connect {} failed with status {}: {}",
            context,
            status,
            body
        );
        SyncError::UploadFailed(format!("Failed with status {}: {}", status, body))
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
            || lower.contains("identical file")
    }

    /// Upload a FIT file to Garmin Connect
    ///
    /// Returns the sync record with upload status. Garmin Connect processes uploads
    /// **synchronously** (unlike Strava which is async), so the record will have
    /// status `Completed` with an `external_id` containing the activity ID if
    /// successful.
    ///
    /// ## Processing Model
    ///
    /// Unlike Strava, Garmin Connect does NOT require status polling. The upload
    /// API immediately returns the activity ID (or error) in the response:
    ///
    /// - Returns `SyncRecordStatus::Completed` with activity ID on success
    /// - Returns appropriate error on failure (no "processing" state)
    ///
    /// This is different from Strava's async model which requires calling
    /// `check_upload_status()` after `upload_activity()`.
    ///
    /// # Arguments
    /// * `ride_id` - The local ride ID
    /// * `fit_data` - The FIT file data as bytes
    ///
    /// # Returns
    /// A SyncRecord with status `Completed` and activity_id in external_id field
    ///
    /// # Errors
    /// * `InvalidFitFile` - If the FIT file is malformed or too small
    /// * `DuplicateActivity` - If the activity was already uploaded to Garmin Connect
    /// * `RateLimited` - If Garmin's rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn upload_activity(
        &self,
        ride_id: &Uuid,
        fit_data: &[u8],
    ) -> Result<SyncRecord, SyncError> {
        // Validate FIT file before attempting upload
        Self::validate_fit_file(fit_data)?;

        let token = self.get_access_token().await?;

        let record_id = Uuid::new_v4();

        tracing::info!(
            "Uploading activity {} to Garmin Connect (record: {}, size: {} bytes)",
            ride_id,
            record_id,
            fit_data.len()
        );

        // Build multipart form
        // Use ride_id as external reference for correlation
        let filename = format!("{}.fit", ride_id);

        // Create the file part with proper MIME type
        let file_part = Part::bytes(fit_data.to_vec())
            .file_name(filename)
            .mime_str("application/octet-stream")
            .map_err(|e| SyncError::UploadFailed(format!("Failed to create file part: {}", e)))?;

        // Build the multipart form
        let form = Form::new().part("file", file_part);

        // Send the upload request with extended timeout for file uploads
        // Garmin Connect upload endpoint: /upload-service/upload/.fit
        let url = format!("{}/upload-service/upload/.fit", self.base_url);
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
                        "Garmin Connect upload request timed out after {} seconds",
                        UPLOAD_TIMEOUT_SECS
                    );
                    SyncError::Timeout(UPLOAD_TIMEOUT_SECS)
                } else if e.is_connect() {
                    tracing::warn!("Failed to connect to Garmin Connect: {}", e);
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
            tracing::warn!("Garmin Connect API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "Garmin Connect API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle forbidden (403) - often means token issues
        if status_code == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!("Garmin Connect API returned 403 Forbidden - token may be invalid");
            return Err(SyncError::TokenExpired);
        }

        // Handle conflict (409) - usually means duplicate
        if status_code == reqwest::StatusCode::CONFLICT {
            tracing::info!("Activity {} already exists on Garmin Connect (409 Conflict)", ride_id);
            return Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect));
        }

        // Handle other errors
        if !status_code.is_success() {
            // Try to parse as Garmin error response
            if let Ok(error_response) = serde_json::from_str::<GarminApiError>(&body) {
                let error_msg = error_response.to_string();

                // Check for duplicate activity error
                if Self::is_duplicate_error(&error_msg) {
                    tracing::info!("Activity {} already exists on Garmin Connect", ride_id);
                    return Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect));
                }

                tracing::error!("Garmin Connect upload failed: {}", error_msg);
                return Err(SyncError::UploadFailed(format!(
                    "Garmin error: {}",
                    error_msg
                )));
            }
            // Fall back to generic error
            tracing::error!(
                "Garmin Connect upload failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::UploadFailed(format!(
                "Upload failed with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let upload_response: GarminUploadResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::warn!("Failed to parse Garmin upload response: {}. Body: {}", e, body);
            SyncError::UploadFailed(format!("Failed to parse upload response: {}", e))
        })?;

        // Check for failures in the response
        if !upload_response.detailed_import_result.failures.is_empty() {
            let failure = &upload_response.detailed_import_result.failures[0];
            let error_msgs: Vec<String> = failure
                .messages
                .iter()
                .filter_map(|m| m.content.clone())
                .collect();
            let error_msg = if error_msgs.is_empty() {
                "Unknown upload failure".to_string()
            } else {
                error_msgs.join("; ")
            };

            // Check for duplicate in failure messages
            if Self::is_duplicate_error(&error_msg) {
                tracing::info!("Activity {} already exists on Garmin Connect", ride_id);
                return Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect));
            }

            tracing::error!("Garmin Connect upload failed: {}", error_msg);
            return Err(SyncError::UploadFailed(error_msg));
        }

        // Extract activity ID from successful upload
        let (external_id, external_url) = if let Some(success) =
            upload_response.detailed_import_result.successes.first()
        {
            let activity_id = success.internal_id.to_string();
            let activity_url = format!(
                "https://connect.garmin.com/modern/activity/{}",
                success.internal_id
            );
            tracing::info!(
                "Garmin Connect upload successful, activity_id: {}",
                success.internal_id
            );
            (Some(activity_id), Some(activity_url))
        } else if let Some(ref upload_uuid) = upload_response.detailed_import_result.upload_uuid {
            // If no success but we have an upload UUID, use that
            tracing::info!(
                "Garmin Connect upload accepted, upload_uuid: {}",
                upload_uuid.uuid
            );
            (Some(upload_uuid.uuid.clone()), None)
        } else {
            tracing::warn!("Garmin Connect upload succeeded but no activity ID returned");
            (None, None)
        };

        // Create record with Completed status (Garmin processes synchronously)
        let record = SyncRecord {
            id: record_id,
            ride_id: *ride_id,
            platform: SyncPlatform::GarminConnect,
            status: SyncRecordStatus::Completed,
            external_id,
            external_url,
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
            error_message: None,
            retry_count: 0,
        };

        tracing::debug!("Garmin Connect upload completed: {:?}", record);

        Ok(record)
    }

    /// Get user profile
    ///
    /// Fetches the authenticated user's profile from Garmin Connect.
    ///
    /// # Returns
    /// The user profile including id, display name, and profile image URL
    ///
    /// # Errors
    /// * `NotConfigured` - If no access token is set
    /// * `RateLimited` - If Garmin's rate limit was exceeded
    /// * `TokenExpired` - If the access token is invalid or expired
    /// * `Timeout` - If the request timed out
    /// * `NetworkError` - If a network error occurred
    pub async fn get_user_profile(&self) -> Result<GarminUserProfile, SyncError> {
        let token = self.get_access_token().await?;

        tracing::debug!("Fetching Garmin Connect user profile");

        let url = format!("{}/userprofile-service/socialProfile", self.base_url);

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
                    SyncError::NetworkError(format!("Failed to fetch user profile: {}", e))
                }
            })?;

        let status_code = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!("Garmin Connect API rate limit exceeded");
            return Err(SyncError::RateLimited);
        }

        // Handle unauthorized (401)
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                "Garmin Connect API returned 401 Unauthorized - token may be expired or revoked"
            );
            return Err(SyncError::TokenExpired);
        }

        // Handle forbidden (403) - often means token issues
        if status_code == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!("Garmin Connect API returned 403 Forbidden - token may be invalid");
            return Err(SyncError::TokenExpired);
        }

        // Handle other errors
        if !status_code.is_success() {
            if let Ok(error_response) = serde_json::from_str::<GarminApiError>(&body) {
                tracing::error!("Garmin Connect profile fetch failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "Garmin error: {}",
                    error_response
                )));
            }
            tracing::error!(
                "Garmin Connect profile fetch failed with status {}: {}",
                status_code,
                body
            );
            return Err(SyncError::ApiError(format!(
                "Failed to fetch profile with status {}: {}",
                status_code, body
            )));
        }

        // Parse successful response
        let profile_response: GarminSocialProfileResponse =
            serde_json::from_str(&body).map_err(|e| {
                SyncError::ApiError(format!("Failed to parse profile response: {}", e))
            })?;

        tracing::info!(
            "Fetched Garmin Connect user profile: {} (id: {})",
            profile_response.display_name,
            profile_response.user_id
        );

        // Convert to GarminUserProfile
        // Use medium image URL if available, fallback to small, then large
        let profile_image_url = profile_response
            .profile_image_url_medium
            .or(profile_response.profile_image_url_small)
            .or(profile_response.profile_image_url_large);

        Ok(GarminUserProfile {
            user_id: profile_response.user_id,
            display_name: profile_response.display_name,
            full_name: profile_response.full_name,
            profile_image_url,
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

    /// Deauthorize application
    ///
    /// Revokes the application's access to the user's Garmin Connect account by POSTing
    /// to the revoke endpoint. This invalidates the access token on Garmin's side
    /// and clears the local token.
    ///
    /// Note: Garmin Connect's OAuth 2.0 implementation may not support standard
    /// token revocation. This method attempts to call the revoke endpoint but
    /// will succeed regardless if the local token is cleared.
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
            .ok_or(SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

        tracing::info!("Deauthorizing Garmin Connect");

        // POST to Garmin's revoke endpoint
        // Note: Garmin Connect OAuth may not support standard revocation
        let url = format!("{}/revoke", self.oauth_base_url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .send()
            .await;

        // Always clear local token, even if the API call fails
        // This ensures the user can disconnect even with network issues
        self.clear_token().await;

        // Now handle the response
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    "Failed to call Garmin Connect revoke endpoint: {}. Local token cleared.",
                    e
                );
                // Still consider this a success since local token is cleared
                return Ok(());
            }
        };

        let status_code = response.status();

        // Handle rate limiting (429 Too Many Requests)
        if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
            tracing::warn!(
                "Garmin Connect API rate limit exceeded during deauthorization. Local token cleared."
            );
            // Token is already cleared locally, so this is still a success from user perspective
            return Ok(());
        }

        // Handle unauthorized (401) - token was already invalid/revoked
        if status_code == reqwest::StatusCode::UNAUTHORIZED {
            tracing::info!("Garmin Connect token was already invalid or revoked. Local token cleared.");
            return Ok(());
        }

        // Handle not found (404) - endpoint may not exist
        if status_code == reqwest::StatusCode::NOT_FOUND {
            tracing::info!(
                "Garmin Connect revoke endpoint not found. Local token cleared."
            );
            return Ok(());
        }

        // Handle other errors
        if !status_code.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Ok(error_response) = serde_json::from_str::<GarminApiError>(&body) {
                tracing::warn!(
                    "Garmin Connect revoke returned error: {}. Local token cleared.",
                    error_response
                );
            } else {
                tracing::warn!(
                    "Garmin Connect revoke returned status {}: {}. Local token cleared.",
                    status_code,
                    body
                );
            }
            // Still consider this a success since local token is cleared
            return Ok(());
        }

        tracing::info!("Successfully deauthorized from Garmin Connect");

        Ok(())
    }

    // ========================================================================
    // Auto-Refresh Wrappers
    //
    // These methods wrap the base API methods and automatically handle token
    // refresh when the access token expires.
    // ========================================================================

    /// Upload a FIT file to Garmin Connect with automatic token refresh.
    ///
    /// This method wraps `upload_activity` and automatically attempts to refresh
    /// the access token and retry if the upload fails with `TokenExpired`.
    ///
    /// Requires a token refresher to be set via `set_token_refresher()`.
    ///
    /// # Arguments
    /// * `ride_id` - The local ride ID
    /// * `fit_data` - The FIT file data as bytes
    ///
    /// # Returns
    /// A SyncRecord with the activity_id in external_id field
    ///
    /// # Errors
    /// Same as `upload_activity`, plus:
    /// * `AuthorizationRequired` - If token refresh fails because re-authorization is needed
    pub async fn upload_activity_with_refresh(
        &self,
        ride_id: Uuid,
        fit_data: Vec<u8>,
    ) -> Result<SyncRecord, SyncError> {
        self.with_auto_refresh("upload_activity", || {
            let ride_id = ride_id;
            let fit_data = fit_data.clone();
            async move { self.upload_activity(&ride_id, &fit_data).await }
        })
        .await
    }

    /// Get user profile with automatic token refresh.
    ///
    /// This method wraps `get_user_profile` and automatically attempts to refresh
    /// the access token and retry if the request fails with `TokenExpired`.
    ///
    /// Requires a token refresher to be set via `set_token_refresher()`.
    ///
    /// # Returns
    /// The user profile including id, display name, and profile image URL
    ///
    /// # Errors
    /// Same as `get_user_profile`, plus:
    /// * `AuthorizationRequired` - If token refresh fails because re-authorization is needed
    pub async fn get_user_profile_with_refresh(&self) -> Result<GarminUserProfile, SyncError> {
        self.with_auto_refresh("get_user_profile", || async move {
            self.get_user_profile().await
        })
        .await
    }
}

/// Garmin user profile
#[derive(Debug, Clone)]
pub struct GarminUserProfile {
    /// User ID
    pub user_id: u64,
    /// Display name (username)
    pub display_name: String,
    /// Full name (optional, e.g., "John Doe")
    pub full_name: Option<String>,
    /// Profile image URL (optional)
    pub profile_image_url: Option<String>,
}

impl GarminUserProfile {
    /// Get a human-readable name for display
    ///
    /// Returns full_name if available, otherwise display_name
    pub fn readable_name(&self) -> &str {
        self.full_name.as_deref().unwrap_or(&self.display_name)
    }
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

    // ========================================================================
    // Client Tests
    // ========================================================================

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
        let custom_oauth_url = "http://localhost:8081/oauth".to_string();
        let client = GarminClient::with_base_url(custom_url.clone(), custom_oauth_url);
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

    // ========================================================================
    // Error Category Tests
    // ========================================================================

    #[test]
    fn test_error_category_is_retryable() {
        assert!(ErrorCategory::Transient.is_retryable());
        assert!(ErrorCategory::RateLimited.is_retryable());
        assert!(ErrorCategory::Server.is_retryable());
        assert!(!ErrorCategory::Client.is_retryable());
        assert!(!ErrorCategory::Authentication.is_retryable());
        assert!(!ErrorCategory::Permanent.is_retryable());
    }

    #[test]
    fn test_error_category_initial_retry_delay() {
        assert_eq!(ErrorCategory::Transient.initial_retry_delay_secs(), 5);
        assert_eq!(
            ErrorCategory::RateLimited.initial_retry_delay_secs(),
            DEFAULT_RATE_LIMIT_RETRY_SECS
        );
        assert_eq!(ErrorCategory::Server.initial_retry_delay_secs(), 30);
        assert_eq!(ErrorCategory::Client.initial_retry_delay_secs(), 0);
        assert_eq!(ErrorCategory::Permanent.initial_retry_delay_secs(), 0);
    }

    #[test]
    fn test_error_category_max_retry_attempts() {
        assert_eq!(ErrorCategory::Transient.max_retry_attempts(), MAX_RETRY_ATTEMPTS);
        assert_eq!(ErrorCategory::RateLimited.max_retry_attempts(), 1);
        assert_eq!(ErrorCategory::Server.max_retry_attempts(), 2);
        assert_eq!(ErrorCategory::Client.max_retry_attempts(), 0);
        assert_eq!(ErrorCategory::Permanent.max_retry_attempts(), 0);
    }

    // ========================================================================
    // Rate Limit Info Tests
    // ========================================================================

    #[test]
    fn test_rate_limit_info_default() {
        let info = RateLimitInfo::default();
        assert_eq!(info.retry_after_secs, DEFAULT_RATE_LIMIT_RETRY_SECS);
        assert!(!info.is_hard_limit);
    }

    #[test]
    fn test_rate_limit_info_retry_delay() {
        let info = RateLimitInfo {
            retry_after_secs: 120,
            is_hard_limit: false,
        };
        assert_eq!(info.retry_delay(), Duration::from_secs(120));
    }

    #[test]
    fn test_rate_limit_info_from_headers_with_retry_after() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("retry-after", "300".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.retry_after_secs, 300);
    }

    #[test]
    fn test_rate_limit_info_from_headers_no_retry_after() {
        let headers = reqwest::header::HeaderMap::new();

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.retry_after_secs, DEFAULT_RATE_LIMIT_RETRY_SECS);
    }

    #[test]
    fn test_rate_limit_info_from_headers_hard_limit() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "0".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert!(info.is_hard_limit);
    }

    #[test]
    fn test_rate_limit_info_from_headers_soft_limit() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "50".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert!(!info.is_hard_limit);
    }

    // ========================================================================
    // SyncErrorExt Tests
    // ========================================================================

    #[test]
    fn test_sync_error_ext_category_network_error() {
        let error = SyncError::NetworkError("connection failed".to_string());
        assert_eq!(error.category(), ErrorCategory::Transient);
        assert!(error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_timeout() {
        let error = SyncError::Timeout(60);
        assert_eq!(error.category(), ErrorCategory::Transient);
        assert!(error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_rate_limited() {
        let error = SyncError::RateLimited;
        assert_eq!(error.category(), ErrorCategory::RateLimited);
        assert!(error.is_retryable());
        assert!(error.is_rate_limited());
    }

    #[test]
    fn test_sync_error_ext_category_token_expired() {
        let error = SyncError::TokenExpired;
        assert_eq!(error.category(), ErrorCategory::Authentication);
        assert!(error.requires_auth_refresh());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_duplicate() {
        let error = SyncError::DuplicateActivity(SyncPlatform::GarminConnect);
        assert_eq!(error.category(), ErrorCategory::Permanent);
        assert!(error.is_duplicate());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_invalid_fit() {
        let error = SyncError::InvalidFitFile("bad file".to_string());
        assert_eq!(error.category(), ErrorCategory::Client);
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_server_error() {
        let error = SyncError::UploadFailed("status 500: Internal Server Error".to_string());
        assert_eq!(error.category(), ErrorCategory::Server);
        assert!(error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_category_client_error() {
        let error = SyncError::UploadFailed("Bad Request".to_string());
        assert_eq!(error.category(), ErrorCategory::Client);
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_sync_error_ext_retry_delay() {
        assert_eq!(SyncError::NetworkError("test".to_string()).retry_delay_secs(), 5);
        assert_eq!(SyncError::RateLimited.retry_delay_secs(), DEFAULT_RATE_LIMIT_RETRY_SECS);
        assert_eq!(
            SyncError::UploadFailed("500 error".to_string()).retry_delay_secs(),
            30
        );
        assert_eq!(SyncError::TokenExpired.retry_delay_secs(), 0);
    }

    // ========================================================================
    // Helper Method Tests
    // ========================================================================

    #[test]
    fn test_handle_error_status_rate_limit() {
        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "upload",
        );
        assert!(matches!(result, Some(SyncError::RateLimited)));
    }

    #[test]
    fn test_handle_error_status_unauthorized() {
        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::UNAUTHORIZED,
            "profile fetch",
        );
        assert!(matches!(result, Some(SyncError::TokenExpired)));
    }

    #[test]
    fn test_handle_error_status_forbidden() {
        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::FORBIDDEN,
            "upload",
        );
        assert!(matches!(result, Some(SyncError::TokenExpired)));
    }

    #[test]
    fn test_handle_error_status_conflict() {
        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::CONFLICT,
            "upload",
        );
        assert!(matches!(
            result,
            Some(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))
        ));
    }

    #[test]
    fn test_handle_error_status_other() {
        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::BAD_REQUEST,
            "upload",
        );
        assert!(result.is_none());

        let result = GarminClient::handle_error_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "upload",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_error_response_with_garmin_error() {
        let body = r#"{"message": "Invalid file format", "errors": []}"#;
        let error = GarminClient::parse_error_response(
            reqwest::StatusCode::BAD_REQUEST,
            body,
            "upload",
        );
        assert!(matches!(error, SyncError::UploadFailed(msg) if msg.contains("Invalid file format")));
    }

    #[test]
    fn test_parse_error_response_with_duplicate() {
        let body = r#"{"message": "Activity already exists", "errors": []}"#;
        let error = GarminClient::parse_error_response(
            reqwest::StatusCode::BAD_REQUEST,
            body,
            "upload",
        );
        assert!(matches!(
            error,
            SyncError::DuplicateActivity(SyncPlatform::GarminConnect)
        ));
    }

    #[test]
    fn test_parse_error_response_non_json() {
        let body = "Internal Server Error";
        let error = GarminClient::parse_error_response(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            body,
            "upload",
        );
        assert!(matches!(error, SyncError::UploadFailed(msg) if msg.contains("500")));
    }

    // ========================================================================
    // Upload Activity Tests (without token)
    // ========================================================================

    #[tokio::test]
    async fn test_upload_activity_without_token_returns_not_configured() {
        let client = GarminClient::new();
        let ride_id = Uuid::new_v4();
        // Create valid FIT data to ensure we get past validation
        let fit_data = create_valid_fit_header();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_upload_activity_with_invalid_fit_file() {
        let client = GarminClient::new();
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let invalid_fit_data = vec![0u8; 5]; // Too small

        let result = client.upload_activity(&ride_id, &invalid_fit_data).await;

        assert!(matches!(result, Err(SyncError::InvalidFitFile(_))));
    }

    // ========================================================================
    // Other API Method Tests
    // ========================================================================

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
    async fn test_deauthorize_without_token_returns_not_configured() {
        let client = GarminClient::new();

        let result = client.deauthorize().await;

        assert!(matches!(
            result,
            Err(SyncError::NotConfigured(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_deauthorize_clears_local_token() {
        let client = GarminClient::new();
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
    // Activity Type Tests
    // ========================================================================

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

    // ========================================================================
    // OAuth Scopes Tests
    // ========================================================================

    #[test]
    fn test_default_scopes() {
        let scopes = default_scopes();
        assert!(scopes.contains(&scopes::ACTIVITY_WRITE.to_string()));
        assert!(scopes.contains(&scopes::PROFILE_READ.to_string()));
        assert!(scopes.contains(&scopes::ACTIVITY_READ.to_string()));
        assert!(!scopes.contains(&scopes::DEVICE_READ.to_string()));
    }

    // ========================================================================
    // User Profile Tests
    // ========================================================================

    #[test]
    fn test_garmin_user_profile_readable_name_with_full_name() {
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist123".to_string(),
            full_name: Some("John Doe".to_string()),
            profile_image_url: None,
        };
        assert_eq!(profile.readable_name(), "John Doe");
    }

    #[test]
    fn test_garmin_user_profile_readable_name_without_full_name() {
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist123".to_string(),
            full_name: None,
            profile_image_url: None,
        };
        assert_eq!(profile.readable_name(), "cyclist123");
    }

    #[test]
    fn test_garmin_social_profile_response_deserialization() {
        let json = r#"{
            "id": 12345678,
            "displayName": "cyclist123",
            "fullName": "John Doe",
            "profileImageUrlSmall": "https://example.com/small.jpg",
            "profileImageUrlMedium": "https://example.com/medium.jpg",
            "profileImageUrlLarge": "https://example.com/large.jpg"
        }"#;

        let response: GarminSocialProfileResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.user_id, 12345678);
        assert_eq!(response.display_name, "cyclist123");
        assert_eq!(response.full_name, Some("John Doe".to_string()));
        assert_eq!(
            response.profile_image_url_small,
            Some("https://example.com/small.jpg".to_string())
        );
        assert_eq!(
            response.profile_image_url_medium,
            Some("https://example.com/medium.jpg".to_string())
        );
        assert_eq!(
            response.profile_image_url_large,
            Some("https://example.com/large.jpg".to_string())
        );
    }

    #[test]
    fn test_garmin_social_profile_response_minimal() {
        let json = r#"{
            "id": 99999,
            "displayName": "user99999"
        }"#;

        let response: GarminSocialProfileResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(response.user_id, 99999);
        assert_eq!(response.display_name, "user99999");
        assert!(response.full_name.is_none());
        assert!(response.profile_image_url_small.is_none());
        assert!(response.profile_image_url_medium.is_none());
        assert!(response.profile_image_url_large.is_none());
    }

    #[test]
    fn test_garmin_social_profile_response_ignores_extra_fields() {
        // Garmin API may return additional fields - ensure we handle them gracefully
        let json = r#"{
            "id": 12345678,
            "displayName": "cyclist123",
            "fullName": "John Doe",
            "userName": "cyclist123",
            "profileVisibility": "PUBLIC",
            "location": "San Francisco",
            "bio": "I love cycling!",
            "profileImageUrlSmall": "https://example.com/small.jpg"
        }"#;

        let response: GarminSocialProfileResponse = serde_json::from_str(json)
            .expect("Deserialization should succeed with extra fields");

        assert_eq!(response.user_id, 12345678);
        assert_eq!(response.display_name, "cyclist123");
    }

    #[test]
    fn test_scope_values() {
        assert_eq!(scopes::PROFILE_READ, "profile:read");
        assert_eq!(scopes::ACTIVITY_READ, "activity:read");
        assert_eq!(scopes::ACTIVITY_WRITE, "activity:write");
        assert_eq!(scopes::DEVICE_READ, "device:read");
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
        let result = GarminClient::validate_fit_file(&valid_fit);
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
        let result = GarminClient::validate_fit_file(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fit_file_too_small() {
        let tiny_data = vec![0u8; 5];
        let result = GarminClient::validate_fit_file(&tiny_data);
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
        let result = GarminClient::validate_fit_file(&data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("Invalid header size")));
    }

    #[test]
    fn test_validate_fit_file_missing_signature() {
        let mut data = vec![0u8; 16];
        data[0] = 14; // Header size
        // Missing ".FIT" signature - just zeros
        let result = GarminClient::validate_fit_file(&data);
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
        let result = GarminClient::validate_fit_file(&data);
        assert!(matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("truncated")));
    }

    // ========================================================================
    // Duplicate Detection Tests
    // ========================================================================

    #[test]
    fn test_is_duplicate_error_detection() {
        assert!(GarminClient::is_duplicate_error(
            "The activity appears to be a duplicate."
        ));
        assert!(GarminClient::is_duplicate_error("Activity already exists"));
        assert!(GarminClient::is_duplicate_error(
            "This file has already uploaded"
        ));
        assert!(GarminClient::is_duplicate_error("DUPLICATE activity detected"));
        assert!(GarminClient::is_duplicate_error("identical file detected"));

        // Should not match non-duplicate errors
        assert!(!GarminClient::is_duplicate_error("Invalid file format"));
        assert!(!GarminClient::is_duplicate_error("Rate limit exceeded"));
        assert!(!GarminClient::is_duplicate_error("Server error"));
    }

    // ========================================================================
    // Response Parsing Tests
    // ========================================================================

    #[test]
    fn test_garmin_api_error_display() {
        let error = GarminApiError {
            message: Some("Bad Request".to_string()),
            code: None,
            errors: vec![],
        };
        assert_eq!(format!("{}", error), "Bad Request");

        let error_with_details = GarminApiError {
            message: Some("Validation failed".to_string()),
            code: None,
            errors: vec![GarminFieldError {
                message: Some("Invalid file".to_string()),
                path: Some("file".to_string()),
            }],
        };
        assert_eq!(
            format!("{}", error_with_details),
            "Validation failed (Invalid file)"
        );

        let error_no_message = GarminApiError {
            message: None,
            code: None,
            errors: vec![GarminFieldError {
                message: Some("Error detail".to_string()),
                path: None,
            }],
        };
        assert_eq!(format!("{}", error_no_message), "Error detail");

        let error_empty = GarminApiError {
            message: None,
            code: None,
            errors: vec![],
        };
        assert_eq!(format!("{}", error_empty), "Unknown Garmin API error");
    }

    #[test]
    fn test_garmin_upload_response_deserialization() {
        let json = r#"{
            "detailedImportResult": {
                "uploadUuid": {
                    "uuid": "abc123-def456"
                },
                "successes": [
                    {
                        "internalId": 12345678,
                        "externalId": "test-ride-uuid"
                    }
                ],
                "failures": []
            }
        }"#;

        let response: GarminUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(
            response
                .detailed_import_result
                .upload_uuid
                .as_ref()
                .unwrap()
                .uuid,
            "abc123-def456"
        );
        assert_eq!(response.detailed_import_result.successes.len(), 1);
        assert_eq!(
            response.detailed_import_result.successes[0].internal_id,
            12345678
        );
        assert!(response.detailed_import_result.failures.is_empty());
    }

    #[test]
    fn test_garmin_upload_response_with_failure() {
        let json = r#"{
            "detailedImportResult": {
                "uploadUuid": null,
                "successes": [],
                "failures": [
                    {
                        "internalId": null,
                        "externalId": "test-ride",
                        "messages": [
                            {
                                "code": 409,
                                "content": "Duplicate activity detected"
                            }
                        ]
                    }
                ]
            }
        }"#;

        let response: GarminUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert!(response.detailed_import_result.successes.is_empty());
        assert_eq!(response.detailed_import_result.failures.len(), 1);
        assert_eq!(
            response.detailed_import_result.failures[0].messages[0].content,
            Some("Duplicate activity detected".to_string())
        );
    }

    #[test]
    fn test_garmin_upload_response_minimal() {
        // Minimal response with only required fields
        let json = r#"{
            "detailedImportResult": {
                "successes": [],
                "failures": []
            }
        }"#;

        let response: GarminUploadResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert!(response.detailed_import_result.upload_uuid.is_none());
        assert!(response.detailed_import_result.successes.is_empty());
        assert!(response.detailed_import_result.failures.is_empty());
    }

    // ========================================================================
    // Token Refresher Tests
    // ========================================================================

    #[test]
    fn test_client_has_no_token_refresher_by_default() {
        let client = GarminClient::new();
        assert!(!client.has_token_refresher());
    }

    #[tokio::test]
    async fn test_set_and_clear_token_refresher() {
        use std::sync::atomic::{AtomicBool, Ordering};

        /// Test token refresher that tracks if it was called
        struct TestRefresher {
            was_called: Arc<AtomicBool>,
        }

        impl TokenRefresher for TestRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.was_called.store(true, Ordering::SeqCst);
                Box::pin(async { Ok("new_token".to_string()) })
            }
        }

        let client = GarminClient::new();

        // Set a token refresher
        let was_called = Arc::new(AtomicBool::new(false));
        let refresher = Arc::new(TestRefresher {
            was_called: was_called.clone(),
        });
        client.set_token_refresher(refresher).await;

        assert!(client.has_token_refresher());

        // Clear it
        client.clear_token_refresher().await;
        assert!(!client.has_token_refresher());
    }

    #[tokio::test]
    async fn test_attempt_token_refresh_no_refresher() {
        let client = GarminClient::new();
        let result = client.attempt_token_refresh().await;
        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_attempt_token_refresh_success() {
        use std::sync::atomic::{AtomicU32, Ordering};

        /// Test token refresher that succeeds
        struct SuccessRefresher {
            call_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for SuccessRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("refreshed_token_123".to_string()) })
            }
        }

        let client = GarminClient::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let refresher = Arc::new(SuccessRefresher {
            call_count: call_count.clone(),
        });
        client.set_token_refresher(refresher).await;

        // Attempt refresh
        let result = client.attempt_token_refresh().await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Verify the token was updated
        let token = client.get_access_token().await.unwrap();
        assert_eq!(token, "refreshed_token_123");
    }

    #[tokio::test]
    async fn test_attempt_token_refresh_failure() {
        /// Test token refresher that fails
        struct FailingRefresher;

        impl TokenRefresher for FailingRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async {
                    Err(SyncError::RefreshFailed("Test refresh failure".to_string()))
                })
            }
        }

        let client = GarminClient::new();
        client
            .set_token_refresher(Arc::new(FailingRefresher))
            .await;

        let result = client.attempt_token_refresh().await;

        assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
    }

    #[tokio::test]
    async fn test_attempt_token_refresh_authorization_required() {
        /// Test token refresher that returns AuthorizationRequired
        struct ReauthRefresher;

        impl TokenRefresher for ReauthRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async { Err(SyncError::AuthorizationRequired) })
            }
        }

        let client = GarminClient::new();
        client.set_token_refresher(Arc::new(ReauthRefresher)).await;

        let result = client.attempt_token_refresh().await;

        assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
    }

    #[tokio::test]
    async fn test_with_auto_refresh_success_no_refresh_needed() {
        let client = GarminClient::new();

        // Operation succeeds without needing refresh
        let result = client
            .with_auto_refresh("test_op", || async { Ok(42) })
            .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_auto_refresh_non_token_error_propagated() {
        let client = GarminClient::new();

        // Operation fails with non-TokenExpired error
        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || async {
                Err(SyncError::NetworkError("test network error".to_string()))
            })
            .await;

        assert!(matches!(result, Err(SyncError::NetworkError(_))));
    }

    #[tokio::test]
    async fn test_with_auto_refresh_token_expired_no_refresher() {
        let client = GarminClient::new();

        // Operation fails with TokenExpired, no refresher configured
        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || async { Err(SyncError::TokenExpired) })
            .await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_with_auto_refresh_token_expired_refresh_succeeds_retry_succeeds() {
        use std::sync::atomic::{AtomicU32, Ordering};

        /// Test token refresher that succeeds
        struct SuccessRefresher;

        impl TokenRefresher for SuccessRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async { Ok("new_token".to_string()) })
            }
        }

        let client = GarminClient::new();
        client.set_token_refresher(Arc::new(SuccessRefresher)).await;

        // Track call count - first call fails, second succeeds
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count == 0 {
                        Err(SyncError::TokenExpired)
                    } else {
                        Ok(99)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(call_count.load(Ordering::SeqCst), 2); // Called twice: fail then succeed
    }

    #[tokio::test]
    async fn test_with_auto_refresh_token_expired_refresh_fails() {
        /// Test token refresher that fails
        struct FailingRefresher;

        impl TokenRefresher for FailingRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async {
                    Err(SyncError::RefreshFailed("refresh failed".to_string()))
                })
            }
        }

        let client = GarminClient::new();
        client.set_token_refresher(Arc::new(FailingRefresher)).await;

        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || async { Err(SyncError::TokenExpired) })
            .await;

        // Should return TokenExpired (original error) not RefreshFailed
        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_with_auto_refresh_reauth_required_propagated() {
        /// Test token refresher that returns AuthorizationRequired
        struct ReauthRefresher;

        impl TokenRefresher for ReauthRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async { Err(SyncError::AuthorizationRequired) })
            }
        }

        let client = GarminClient::new();
        client.set_token_refresher(Arc::new(ReauthRefresher)).await;

        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || async { Err(SyncError::TokenExpired) })
            .await;

        // AuthorizationRequired should be propagated
        assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
    }

    #[tokio::test]
    async fn test_with_auto_refresh_max_attempts_respected() {
        use std::sync::atomic::{AtomicU32, Ordering};

        /// Test token refresher that counts calls
        struct CountingRefresher {
            refresh_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for CountingRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.refresh_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("new_token".to_string()) })
            }
        }

        let client = GarminClient::new();
        let refresh_count = Arc::new(AtomicU32::new(0));
        client
            .set_token_refresher(Arc::new(CountingRefresher {
                refresh_count: refresh_count.clone(),
            }))
            .await;

        // Operation always fails with TokenExpired
        let result: Result<i32, SyncError> = client
            .with_auto_refresh("test_op", || async { Err(SyncError::TokenExpired) })
            .await;

        // Should only attempt refresh MAX_AUTO_REFRESH_ATTEMPTS times (1)
        assert_eq!(refresh_count.load(Ordering::SeqCst), MAX_AUTO_REFRESH_ATTEMPTS);
        assert!(matches!(result, Err(SyncError::TokenExpired)));
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
            "detailedImportResult": {
                "uploadUuid": {
                    "uuid": "upload-uuid-123"
                },
                "successes": [
                    {
                        "internalId": 98765432,
                        "externalId": "test-ride-uuid"
                    }
                ],
                "failures": []
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.ride_id, ride_id);
        assert_eq!(record.platform, SyncPlatform::GarminConnect);
        assert_eq!(record.status, SyncRecordStatus::Completed);
        assert_eq!(record.external_id, Some("98765432".to_string()));
        assert_eq!(
            record.external_url,
            Some("https://connect.garmin.com/modern/activity/98765432".to_string())
        );
    }

    #[tokio::test]
    async fn test_upload_activity_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_upload_activity_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_upload_activity_forbidden() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_upload_activity_conflict_duplicate() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(
            result,
            Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_upload_activity_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Invalid file format",
            "errors": [
                {"message": "File is corrupted", "path": "file"}
            ]
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(
            matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("Invalid file format"))
        );
    }

    #[tokio::test]
    async fn test_upload_activity_api_error_duplicate_detection() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Activity already exists in your library"
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(400).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(
            result,
            Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_upload_activity_generic_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("500")));
    }

    #[tokio::test]
    async fn test_upload_activity_with_failure_in_response() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "detailedImportResult": {
                "uploadUuid": null,
                "successes": [],
                "failures": [
                    {
                        "internalId": null,
                        "externalId": "test-ride",
                        "messages": [
                            {
                                "code": 409,
                                "content": "Duplicate activity detected"
                            }
                        ]
                    }
                ]
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        // Should detect duplicate from failure messages
        assert!(matches!(
            result,
            Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))
        ));
    }

    #[tokio::test]
    async fn test_upload_activity_with_non_duplicate_failure() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "detailedImportResult": {
                "uploadUuid": null,
                "successes": [],
                "failures": [
                    {
                        "internalId": null,
                        "externalId": "test-ride",
                        "messages": [
                            {
                                "code": 500,
                                "content": "Processing error occurred"
                            }
                        ]
                    }
                ]
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(
            matches!(result, Err(SyncError::UploadFailed(msg)) if msg.contains("Processing error"))
        );
    }

    #[tokio::test]
    async fn test_upload_activity_success_with_upload_uuid_only() {
        let mock_server = MockServer::start().await;

        // Response with upload UUID but no successes (might be async processing)
        let response_body = r#"{
            "detailedImportResult": {
                "uploadUuid": {
                    "uuid": "pending-upload-uuid"
                },
                "successes": [],
                "failures": []
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client.upload_activity(&ride_id, &fit_data).await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.external_id, Some("pending-upload-uuid".to_string()));
        assert!(record.external_url.is_none()); // No URL when only UUID available
    }

    // ============================================================================
    // Get User Profile Tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_user_profile_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 12345678,
            "displayName": "cyclist123",
            "fullName": "John Doe",
            "profileImageUrlSmall": "https://example.com/small.jpg",
            "profileImageUrlMedium": "https://example.com/medium.jpg",
            "profileImageUrlLarge": "https://example.com/large.jpg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.user_id, 12345678);
        assert_eq!(profile.display_name, "cyclist123");
        assert_eq!(profile.full_name, Some("John Doe".to_string()));
        // Should prefer medium image URL
        assert_eq!(
            profile.profile_image_url,
            Some("https://example.com/medium.jpg".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_user_profile_minimal_response() {
        let mock_server = MockServer::start().await;

        // Response with only required fields
        let response_body = r#"{
            "id": 99999,
            "displayName": "user99999"
        }"#;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.user_id, 99999);
        assert_eq!(profile.display_name, "user99999");
        assert!(profile.full_name.is_none());
        assert!(profile.profile_image_url.is_none());
        // Test readable_name when full_name is None
        assert_eq!(profile.readable_name(), "user99999");
    }

    #[tokio::test]
    async fn test_get_user_profile_with_small_image_only() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "id": 11111,
            "displayName": "user11111",
            "profileImageUrlSmall": "https://example.com/small.jpg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(result.is_ok());
        let profile = result.unwrap();
        // Should fallback to small image when medium not available
        assert_eq!(
            profile.profile_image_url,
            Some("https://example.com/small.jpg".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_user_profile_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(matches!(result, Err(SyncError::RateLimited)));
    }

    #[tokio::test]
    async fn test_get_user_profile_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_get_user_profile_forbidden() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(matches!(result, Err(SyncError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_get_user_profile_api_error() {
        let mock_server = MockServer::start().await;

        let error_body = r#"{
            "message": "Resource Not Found"
        }"#;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(404).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(
            matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("Resource Not Found"))
        );
    }

    #[tokio::test]
    async fn test_get_user_profile_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.get_user_profile().await;

        assert!(matches!(result, Err(SyncError::ApiError(msg)) if msg.contains("500")));
    }

    // ============================================================================
    // Deauthorize Tests
    // ============================================================================

    #[tokio::test]
    async fn test_deauthorize_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/revoke"))
            .and(bearer_token("test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with 401 since token was already invalid
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_deauthorize_not_found_still_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        // Deauthorize should succeed even with 404 since endpoint may not exist
        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    #[tokio::test]
    async fn test_deauthorize_server_error_still_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
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
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(403).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("test_token".to_string()).await;

        let result = client.deauthorize().await;

        assert!(result.is_ok());
        assert!(!client.is_configured());
    }

    // ============================================================================
    // Auto-Refresh Wrapper HTTP Tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_user_profile_with_refresh_success() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mock_server = MockServer::start().await;

        let profile_response = r#"{
            "id": 12345,
            "displayName": "TestUser",
            "fullName": "Test User",
            "profileImageUrlMedium": "https://example.com/image.jpg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .and(bearer_token("valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(profile_response))
            .mount(&mock_server)
            .await;

        /// Refresher that provides a valid token
        struct ValidTokenRefresher {
            call_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for ValidTokenRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("valid_token".to_string()) })
            }
        }

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("valid_token".to_string()).await;
        let call_count = Arc::new(AtomicU32::new(0));
        client
            .set_token_refresher(Arc::new(ValidTokenRefresher {
                call_count: call_count.clone(),
            }))
            .await;

        let result = client.get_user_profile_with_refresh().await;

        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.user_id, 12345);
        assert_eq!(profile.display_name, "TestUser");
        // Refresh should not have been called since token was valid
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_get_user_profile_with_refresh_expired_then_refreshed() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mock_server = MockServer::start().await;

        let profile_response = r#"{
            "id": 99999,
            "displayName": "RefreshedUser",
            "fullName": "Refreshed User"
        }"#;

        // First request with old token returns 401
        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .and(bearer_token("old_token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Second request with new token succeeds
        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .and(bearer_token("refreshed_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(profile_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        /// Refresher that provides a new token
        struct NewTokenRefresher {
            call_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for NewTokenRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("refreshed_token".to_string()) })
            }
        }

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("old_token".to_string()).await;
        let call_count = Arc::new(AtomicU32::new(0));
        client
            .set_token_refresher(Arc::new(NewTokenRefresher {
                call_count: call_count.clone(),
            }))
            .await;

        let result = client.get_user_profile_with_refresh().await;

        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.user_id, 99999);
        assert_eq!(profile.display_name, "RefreshedUser");
        // Refresh should have been called once
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_get_user_profile_with_refresh_fails_reauth_required() {
        let mock_server = MockServer::start().await;

        // Request returns 401
        Mock::given(method("GET"))
            .and(path("/userprofile-service/socialProfile"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        /// Refresher that indicates re-authorization is required
        struct ReauthRefresher;

        impl TokenRefresher for ReauthRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                Box::pin(async { Err(SyncError::AuthorizationRequired) })
            }
        }

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client.set_access_token("expired_token".to_string()).await;
        client.set_token_refresher(Arc::new(ReauthRefresher)).await;

        let result = client.get_user_profile_with_refresh().await;

        assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
    }

    #[tokio::test]
    async fn test_upload_activity_with_refresh_success() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mock_server = MockServer::start().await;

        let upload_response = r#"{
            "detailedImportResult": {
                "uploadUuid": {"uuid": "abc-123"},
                "successes": [{"internalId": 12345678}],
                "failures": []
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .and(bearer_token("valid_upload_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(upload_response))
            .mount(&mock_server)
            .await;

        /// Token refresher for upload test
        struct UploadRefresher {
            call_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for UploadRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("valid_upload_token".to_string()) })
            }
        }

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client
            .set_access_token("valid_upload_token".to_string())
            .await;
        let call_count = Arc::new(AtomicU32::new(0));
        client
            .set_token_refresher(Arc::new(UploadRefresher {
                call_count: call_count.clone(),
            }))
            .await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client
            .upload_activity_with_refresh(ride_id, fit_data)
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.external_id, Some("12345678".to_string()));
        // Refresh should not have been called since token was valid
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_upload_activity_with_refresh_expired_then_refreshed() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mock_server = MockServer::start().await;

        let upload_response = r#"{
            "detailedImportResult": {
                "successes": [{"internalId": 87654321}],
                "failures": []
            }
        }"#;

        // First request with old token returns 401
        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .and(bearer_token("expired_upload_token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Second request with refreshed token succeeds
        Mock::given(method("POST"))
            .and(path("/upload-service/upload/.fit"))
            .and(bearer_token("new_upload_token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(upload_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        /// Token refresher for upload retry test
        struct UploadRetryRefresher {
            call_count: Arc<AtomicU32>,
        }

        impl TokenRefresher for UploadRetryRefresher {
            fn refresh_token(&self) -> TokenRefreshResult {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok("new_upload_token".to_string()) })
            }
        }

        let client = GarminClient::with_base_url(mock_server.uri(), mock_server.uri());
        client
            .set_access_token("expired_upload_token".to_string())
            .await;
        let call_count = Arc::new(AtomicU32::new(0));
        client
            .set_token_refresher(Arc::new(UploadRetryRefresher {
                call_count: call_count.clone(),
            }))
            .await;

        let ride_id = Uuid::new_v4();
        let fit_data = create_valid_fit_data();

        let result = client
            .upload_activity_with_refresh(ride_id, fit_data)
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.external_id, Some("87654321".to_string()));
        // Refresh should have been called once
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
