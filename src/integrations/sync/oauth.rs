//! OAuth2 Authentication
//!
//! Handles OAuth2 flows for fitness platform authentication.

use super::{SyncError, SyncPlatform};
use chrono::{DateTime, Duration, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;

/// OAuth authorization URL response
#[derive(Debug, Clone)]
pub struct AuthorizationUrl {
    /// URL to redirect user to
    pub url: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// Token response from OAuth flow
#[derive(Debug, Clone)]
pub struct TokenResponse {
    /// Access token for API calls
    pub access_token: String,
    /// Refresh token for getting new access tokens
    pub refresh_token: Option<String>,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
}

/// Token status
#[derive(Debug, Clone)]
pub enum TokenStatus {
    /// Token is valid
    Valid { expires_in: std::time::Duration },
    /// Token has expired
    Expired,
    /// Token needs refresh (expires soon)
    NeedsRefresh,
    /// No token configured
    NotConfigured,
}

/// Trait for OAuth handling
pub trait OAuthHandler: Send + Sync {
    /// Start OAuth authorization flow
    fn start_authorization(
        &self,
        platform: SyncPlatform,
    ) -> impl std::future::Future<Output = Result<AuthorizationUrl, SyncError>> + Send;

    /// Handle OAuth callback
    fn handle_callback(
        &self,
        code: &str,
        state: &str,
    ) -> impl std::future::Future<Output = Result<TokenResponse, SyncError>> + Send;

    /// Refresh access token
    fn refresh_token(
        &self,
        platform: SyncPlatform,
    ) -> impl std::future::Future<Output = Result<TokenResponse, SyncError>> + Send;

    /// Check if platform is authorized
    fn is_authorized(&self, platform: SyncPlatform) -> bool;

    /// Get token status
    fn get_token_status(&self, platform: SyncPlatform) -> TokenStatus;

    /// Revoke authorization
    fn revoke(
        &self,
        platform: SyncPlatform,
    ) -> impl std::future::Future<Output = Result<(), SyncError>> + Send;
}

/// Trait for secure credential storage
pub trait CredentialStore: Send + Sync {
    /// Store OAuth tokens
    fn store_tokens(
        &self,
        platform: SyncPlatform,
        tokens: &TokenResponse,
    ) -> impl std::future::Future<Output = Result<(), SyncError>> + Send;

    /// Retrieve OAuth tokens
    fn get_tokens(
        &self,
        platform: SyncPlatform,
    ) -> impl std::future::Future<Output = Result<Option<TokenResponse>, SyncError>> + Send;

    /// Delete tokens
    fn delete_tokens(
        &self,
        platform: SyncPlatform,
    ) -> impl std::future::Future<Output = Result<(), SyncError>> + Send;

    /// Check if credentials exist
    fn has_credentials(&self, platform: SyncPlatform) -> bool;
}

/// Platform OAuth configuration
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

/// Strava OAuth token response from the token endpoint
#[derive(Debug, Deserialize)]
struct StravaTokenResponse {
    /// The access token for API calls
    access_token: String,
    /// The refresh token for getting new access tokens
    refresh_token: String,
    /// When the access token expires (Unix timestamp)
    expires_at: i64,
    /// Token type (usually "Bearer")
    #[allow(dead_code)]
    token_type: String,
}

/// Strava OAuth error response
#[derive(Debug, Deserialize)]
struct StravaErrorResponse {
    /// Error message
    message: String,
    /// Error field details (optional)
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
    field: String,
    /// Error code
    code: String,
}

impl std::fmt::Display for StravaErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.is_empty() {
            write!(f, "{}", self.message)
        } else {
            let details: Vec<String> = self
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.field, e.code))
                .collect();
            write!(f, "{} ({})", self.message, details.join(", "))
        }
    }
}

/// TrainingPeaks OAuth token response from the token endpoint
/// TrainingPeaks uses standard OAuth2 response format with snake_case field names
#[derive(Debug, Deserialize)]
struct TrainingPeaksTokenResponse {
    /// The access token for API calls
    access_token: String,
    /// The refresh token for getting new access tokens
    refresh_token: String,
    /// Token lifetime in seconds
    expires_in: i64,
    /// Token type (usually "Bearer")
    #[allow(dead_code)]
    token_type: String,
}

/// TrainingPeaks OAuth error response
#[derive(Debug, Deserialize)]
struct TrainingPeaksOAuthErrorResponse {
    /// Error type (e.g., "invalid_grant", "invalid_client")
    error: String,
    /// Human-readable error description
    #[serde(default)]
    error_description: Option<String>,
}

impl std::fmt::Display for TrainingPeaksOAuthErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref description) = self.error_description {
            write!(f, "{}: {}", self.error, description)
        } else {
            write!(f, "{}", self.error)
        }
    }
}

/// Garmin Connect OAuth token response from the token endpoint
/// Garmin uses standard OAuth 2.0 response format with snake_case field names
#[derive(Debug, Deserialize)]
struct GarminTokenResponse {
    /// The access token for API calls
    access_token: String,
    /// The refresh token for getting new access tokens
    refresh_token: String,
    /// Token lifetime in seconds
    expires_in: i64,
    /// Token type (usually "Bearer")
    #[allow(dead_code)]
    token_type: String,
    /// Optional scope (may be returned by Garmin)
    #[allow(dead_code)]
    #[serde(default)]
    scope: Option<String>,
}

/// Garmin Connect OAuth error response
/// Garmin uses standard OAuth 2.0 error format
#[derive(Debug, Deserialize)]
struct GarminOAuthErrorResponse {
    /// Error type (e.g., "invalid_grant", "invalid_client", "invalid_request")
    error: String,
    /// Human-readable error description
    #[serde(default)]
    error_description: Option<String>,
}

impl std::fmt::Display for GarminOAuthErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref description) = self.error_description {
            write!(f, "{}: {}", self.error, description)
        } else {
            write!(f, "{}", self.error)
        }
    }
}

/// Default OAuth handler implementation
#[allow(dead_code)]
pub struct DefaultOAuthHandler {
    configs: Arc<RwLock<HashMap<SyncPlatform, OAuthConfig>>>,
    tokens: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
    pending_states: Arc<RwLock<HashMap<String, SyncPlatform>>>,
    http_client: Client,
    callback_port: u16,
    /// Base URL for Strava OAuth API (overridable for testing)
    strava_token_url: String,
    /// Base URL for TrainingPeaks OAuth API (overridable for testing)
    trainingpeaks_token_url: String,
    /// Base URL for Garmin Connect OAuth API (overridable for testing)
    garmin_token_url: String,
}

/// Default Strava OAuth token URL
const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";

/// Default TrainingPeaks OAuth token URL
const TRAININGPEAKS_TOKEN_URL: &str = "https://oauth.trainingpeaks.com/oauth/token";

/// Default Garmin Connect OAuth token URL
/// Garmin uses a proxy endpoint for OAuth token exchange
const GARMIN_TOKEN_URL: &str = "https://connect.garmin.com/oauth-service/oauth/token";

impl DefaultOAuthHandler {
    /// Create a new OAuth handler
    pub fn new(callback_port: u16) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            callback_port,
            strava_token_url: STRAVA_TOKEN_URL.to_string(),
            trainingpeaks_token_url: TRAININGPEAKS_TOKEN_URL.to_string(),
            garmin_token_url: GARMIN_TOKEN_URL.to_string(),
        }
    }

    /// Create a new OAuth handler with a custom Strava token URL (for testing)
    #[cfg(test)]
    pub fn with_strava_token_url(callback_port: u16, strava_token_url: String) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            callback_port,
            strava_token_url,
            trainingpeaks_token_url: TRAININGPEAKS_TOKEN_URL.to_string(),
            garmin_token_url: GARMIN_TOKEN_URL.to_string(),
        }
    }

    /// Create a new OAuth handler with custom token URLs (for testing)
    #[cfg(test)]
    pub fn with_token_urls(
        callback_port: u16,
        strava_token_url: String,
        trainingpeaks_token_url: String,
    ) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            callback_port,
            strava_token_url,
            trainingpeaks_token_url,
            garmin_token_url: GARMIN_TOKEN_URL.to_string(),
        }
    }

    /// Create a new OAuth handler with all custom token URLs (for testing)
    #[cfg(test)]
    pub fn with_all_token_urls(
        callback_port: u16,
        strava_token_url: String,
        trainingpeaks_token_url: String,
        garmin_token_url: String,
    ) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            callback_port,
            strava_token_url,
            trainingpeaks_token_url,
            garmin_token_url,
        }
    }

    /// Configure OAuth for a platform
    pub async fn configure(&self, platform: SyncPlatform, config: OAuthConfig) {
        self.configs.write().await.insert(platform, config);
    }

    /// Generate a random state string
    fn generate_state() -> String {
        use std::time::SystemTime;

        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        format!("{:x}", seed)
    }

    /// Build authorization URL for platform
    fn build_auth_url(config: &OAuthConfig, platform: SyncPlatform, state: &str) -> String {
        let scopes = config.scopes.join(",");

        match platform {
            SyncPlatform::Strava => {
                format!(
                    "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
                    platform.auth_url_base(),
                    config.client_id,
                    urlencoding::encode(&config.redirect_uri),
                    urlencoding::encode(&scopes),
                    state
                )
            }
            SyncPlatform::GarminConnect => {
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
                    platform.auth_url_base(),
                    config.client_id,
                    urlencoding::encode(&config.redirect_uri),
                    state
                )
            }
            _ => {
                format!(
                    "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
                    platform.auth_url_base(),
                    config.client_id,
                    urlencoding::encode(&config.redirect_uri),
                    urlencoding::encode(&scopes),
                    state
                )
            }
        }
    }
}

impl OAuthHandler for DefaultOAuthHandler {
    async fn start_authorization(
        &self,
        platform: SyncPlatform,
    ) -> Result<AuthorizationUrl, SyncError> {
        if !platform.uses_oauth() {
            return Err(SyncError::NotConfigured(platform));
        }

        let configs = self.configs.read().await;
        let config = configs
            .get(&platform)
            .ok_or(SyncError::NotConfigured(platform))?;

        let state = Self::generate_state();

        // Store pending state
        self.pending_states
            .write()
            .await
            .insert(state.clone(), platform);

        let url = Self::build_auth_url(config, platform, &state);

        tracing::info!("Starting OAuth flow for {:?}", platform);

        Ok(AuthorizationUrl { url, state })
    }

    async fn handle_callback(&self, code: &str, state: &str) -> Result<TokenResponse, SyncError> {
        // Verify state
        let pending = self.pending_states.write().await.remove(state);
        let platform = pending.ok_or(SyncError::AuthorizationRequired)?;

        let configs = self.configs.read().await;
        let config = configs
            .get(&platform)
            .ok_or(SyncError::NotConfigured(platform))?;

        tracing::info!("Handling OAuth callback for {:?}", platform);

        // Exchange authorization code for tokens
        let tokens = match platform {
            SyncPlatform::Strava => {
                self.exchange_strava_token(code, config).await?
            }
            SyncPlatform::TrainingPeaks => {
                self.exchange_trainingpeaks_token(code, config).await?
            }
            SyncPlatform::GarminConnect => {
                self.exchange_garmin_token(code, config).await?
            }
            _ => {
                // For other platforms, return an error until implemented
                return Err(SyncError::NotConfigured(platform));
            }
        };

        // Store tokens
        self.tokens.write().await.insert(platform, tokens.clone());

        Ok(tokens)
    }

    /// Exchange authorization code for tokens with Strava's OAuth endpoint
    async fn exchange_strava_token(
        &self,
        code: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::Strava))?;

        // Build the token request
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
        ];

        tracing::debug!("Exchanging authorization code with Strava token endpoint");

        let response = self
            .http_client
            .post(&self.strava_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to send token request: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as Strava error response
            if let Ok(error_response) = serde_json::from_str::<StravaErrorResponse>(&body) {
                tracing::error!("Strava token exchange failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "Strava OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!("Strava token exchange failed with status {}: {}", status, body);
            return Err(SyncError::ApiError(format!(
                "Strava OAuth failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let strava_response: StravaTokenResponse = serde_json::from_str(&body)
            .map_err(|e| SyncError::ApiError(format!("Failed to parse token response: {}", e)))?;

        // Convert Unix timestamp to DateTime<Utc>
        let expires_at = Utc
            .timestamp_opt(strava_response.expires_at, 0)
            .single()
            .unwrap_or_else(|| Utc::now() + Duration::hours(1));

        tracing::info!(
            "Successfully exchanged code for Strava tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: strava_response.access_token,
            refresh_token: Some(strava_response.refresh_token),
            expires_at,
        })
    }

    /// Exchange authorization code for tokens with TrainingPeaks' OAuth endpoint
    async fn exchange_trainingpeaks_token(
        &self,
        code: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        // Build the token request
        // TrainingPeaks uses standard OAuth2 form-encoded parameters
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", config.redirect_uri.as_str()),
        ];

        tracing::debug!("Exchanging authorization code with TrainingPeaks token endpoint");

        let response = self
            .http_client
            .post(&self.trainingpeaks_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to send token request: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as TrainingPeaks OAuth error response
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksOAuthErrorResponse>(&body) {
                tracing::error!("TrainingPeaks token exchange failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "TrainingPeaks OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!("TrainingPeaks token exchange failed with status {}: {}", status, body);
            return Err(SyncError::ApiError(format!(
                "TrainingPeaks OAuth failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let tp_response: TrainingPeaksTokenResponse = serde_json::from_str(&body)
            .map_err(|e| SyncError::ApiError(format!("Failed to parse token response: {}", e)))?;

        // TrainingPeaks returns expires_in (seconds until expiry), not expires_at
        let expires_at = Utc::now() + Duration::seconds(tp_response.expires_in);

        tracing::info!(
            "Successfully exchanged code for TrainingPeaks tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: tp_response.access_token,
            refresh_token: Some(tp_response.refresh_token),
            expires_at,
        })
    }

    /// Exchange authorization code for tokens with Garmin Connect's OAuth endpoint
    async fn exchange_garmin_token(
        &self,
        code: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

        // Build the token request
        // Garmin Connect uses standard OAuth2 form-encoded parameters
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", config.redirect_uri.as_str()),
        ];

        tracing::debug!("Exchanging authorization code with Garmin Connect token endpoint");

        let response = self
            .http_client
            .post(&self.garmin_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to send token request: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as Garmin OAuth error response
            if let Ok(error_response) = serde_json::from_str::<GarminOAuthErrorResponse>(&body) {
                tracing::error!("Garmin Connect token exchange failed: {}", error_response);
                return Err(SyncError::ApiError(format!(
                    "Garmin Connect OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!("Garmin Connect token exchange failed with status {}: {}", status, body);
            return Err(SyncError::ApiError(format!(
                "Garmin Connect OAuth failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let garmin_response: GarminTokenResponse = serde_json::from_str(&body)
            .map_err(|e| SyncError::ApiError(format!("Failed to parse token response: {}", e)))?;

        // Garmin returns expires_in (seconds until expiry), not expires_at
        let expires_at = Utc::now() + Duration::seconds(garmin_response.expires_in);

        tracing::info!(
            "Successfully exchanged code for Garmin Connect tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: garmin_response.access_token,
            refresh_token: Some(garmin_response.refresh_token),
            expires_at,
        })
    }

    async fn refresh_token(&self, platform: SyncPlatform) -> Result<TokenResponse, SyncError> {
        let current = self.tokens.read().await.get(&platform).cloned();

        let current = current.ok_or(SyncError::AuthorizationRequired)?;
        let refresh = current
            .refresh_token
            .ok_or(SyncError::RefreshFailed("No refresh token".to_string()))?;

        let configs = self.configs.read().await;
        let config = configs
            .get(&platform)
            .ok_or(SyncError::NotConfigured(platform))?;

        tracing::info!("Refreshing token for {:?}", platform);

        // Refresh tokens based on platform
        let new_tokens = match platform {
            SyncPlatform::Strava => self.refresh_strava_token(&refresh, config).await?,
            SyncPlatform::TrainingPeaks => self.refresh_trainingpeaks_token(&refresh, config).await?,
            SyncPlatform::GarminConnect => self.refresh_garmin_token(&refresh, config).await?,
            _ => {
                return Err(SyncError::NotConfigured(platform));
            }
        };

        self.tokens
            .write()
            .await
            .insert(platform, new_tokens.clone());

        Ok(new_tokens)
    }

    /// Refresh tokens with Strava's OAuth endpoint
    async fn refresh_strava_token(
        &self,
        refresh_token: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::Strava))?;

        // Build the refresh token request
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        tracing::debug!("Refreshing Strava access token");

        let response = self
            .http_client
            .post(&self.strava_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                SyncError::NetworkError(format!("Failed to send refresh token request: {}", e))
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as Strava error response
            if let Ok(error_response) = serde_json::from_str::<StravaErrorResponse>(&body) {
                tracing::error!("Strava token refresh failed: {}", error_response);

                // Check if the refresh token is invalid or expired
                // Strava returns "invalid" code for bad refresh tokens
                let requires_reauth = error_response.errors.iter().any(|e| {
                    e.field == "refresh_token"
                        && (e.code == "invalid" || e.code == "expired" || e.code == "revoked")
                });

                if requires_reauth {
                    tracing::warn!(
                        "Refresh token is invalid/expired, re-authorization required"
                    );
                    return Err(SyncError::AuthorizationRequired);
                }

                return Err(SyncError::RefreshFailed(format!(
                    "Strava OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!(
                "Strava token refresh failed with status {}: {}",
                status,
                body
            );
            return Err(SyncError::RefreshFailed(format!(
                "Strava refresh failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let strava_response: StravaTokenResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // Convert Unix timestamp to DateTime<Utc>
        let expires_at = Utc
            .timestamp_opt(strava_response.expires_at, 0)
            .single()
            .unwrap_or_else(|| Utc::now() + Duration::hours(1));

        tracing::info!(
            "Successfully refreshed Strava tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: strava_response.access_token,
            refresh_token: Some(strava_response.refresh_token),
            expires_at,
        })
    }

    /// Refresh tokens with TrainingPeaks' OAuth endpoint
    async fn refresh_trainingpeaks_token(
        &self,
        refresh_token: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::TrainingPeaks))?;

        // Build the refresh token request
        // TrainingPeaks uses standard OAuth2 form-encoded parameters
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        tracing::debug!("Refreshing TrainingPeaks access token");

        let response = self
            .http_client
            .post(&self.trainingpeaks_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                SyncError::NetworkError(format!("Failed to send refresh token request: {}", e))
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as TrainingPeaks OAuth error response
            if let Ok(error_response) = serde_json::from_str::<TrainingPeaksOAuthErrorResponse>(&body) {
                tracing::error!("TrainingPeaks token refresh failed: {}", error_response);

                // Check if the refresh token is invalid or expired
                // TrainingPeaks uses standard OAuth2 error codes
                let requires_reauth = error_response.error == "invalid_grant"
                    || error_response.error == "invalid_token"
                    || error_response.error == "unauthorized";

                if requires_reauth {
                    tracing::warn!(
                        "TrainingPeaks refresh token is invalid/expired, re-authorization required"
                    );
                    return Err(SyncError::AuthorizationRequired);
                }

                return Err(SyncError::RefreshFailed(format!(
                    "TrainingPeaks OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!(
                "TrainingPeaks token refresh failed with status {}: {}",
                status,
                body
            );
            return Err(SyncError::RefreshFailed(format!(
                "TrainingPeaks refresh failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let tp_response: TrainingPeaksTokenResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // TrainingPeaks returns expires_in (seconds until expiry), not expires_at
        let expires_at = Utc::now() + Duration::seconds(tp_response.expires_in);

        tracing::info!(
            "Successfully refreshed TrainingPeaks tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: tp_response.access_token,
            refresh_token: Some(tp_response.refresh_token),
            expires_at,
        })
    }

    /// Refresh tokens with Garmin Connect's OAuth endpoint
    async fn refresh_garmin_token(
        &self,
        refresh_token: &str,
        config: &OAuthConfig,
    ) -> Result<TokenResponse, SyncError> {
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured(SyncPlatform::GarminConnect))?;

        // Build the refresh token request
        // Garmin Connect uses standard OAuth2 form-encoded parameters
        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        tracing::debug!("Refreshing Garmin Connect access token");

        let response = self
            .http_client
            .post(&self.garmin_token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                SyncError::NetworkError(format!("Failed to send refresh token request: {}", e))
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SyncError::NetworkError(format!("Failed to read response body: {}", e)))?;

        if !status.is_success() {
            // Try to parse as Garmin OAuth error response
            if let Ok(error_response) = serde_json::from_str::<GarminOAuthErrorResponse>(&body) {
                tracing::error!("Garmin Connect token refresh failed: {}", error_response);

                // Check if the refresh token is invalid or expired
                // Garmin uses standard OAuth2 error codes
                let requires_reauth = error_response.error == "invalid_grant"
                    || error_response.error == "invalid_token"
                    || error_response.error == "unauthorized";

                if requires_reauth {
                    tracing::warn!(
                        "Garmin Connect refresh token is invalid/expired, re-authorization required"
                    );
                    return Err(SyncError::AuthorizationRequired);
                }

                return Err(SyncError::RefreshFailed(format!(
                    "Garmin Connect OAuth error: {}",
                    error_response
                )));
            }
            // Fall back to generic error
            tracing::error!(
                "Garmin Connect token refresh failed with status {}: {}",
                status,
                body
            );
            return Err(SyncError::RefreshFailed(format!(
                "Garmin Connect refresh failed with status {}: {}",
                status, body
            )));
        }

        // Parse successful response
        let garmin_response: GarminTokenResponse = serde_json::from_str(&body).map_err(|e| {
            SyncError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // Garmin returns expires_in (seconds until expiry), not expires_at
        let expires_at = Utc::now() + Duration::seconds(garmin_response.expires_in);

        tracing::info!(
            "Successfully refreshed Garmin Connect tokens (expires at: {})",
            expires_at
        );

        Ok(TokenResponse {
            access_token: garmin_response.access_token,
            refresh_token: Some(garmin_response.refresh_token),
            expires_at,
        })
    }

    fn is_authorized(&self, platform: SyncPlatform) -> bool {
        self.tokens
            .try_read()
            .map(|t| t.contains_key(&platform))
            .unwrap_or(false)
    }

    fn get_token_status(&self, platform: SyncPlatform) -> TokenStatus {
        let tokens = match self.tokens.try_read() {
            Ok(t) => t,
            Err(_) => return TokenStatus::NotConfigured,
        };

        match tokens.get(&platform) {
            None => TokenStatus::NotConfigured,
            Some(token) => {
                let now = Utc::now();
                if token.expires_at <= now {
                    TokenStatus::Expired
                } else if token.expires_at <= now + Duration::minutes(5) {
                    TokenStatus::NeedsRefresh
                } else {
                    let expires_in = (token.expires_at - now).to_std().unwrap_or_default();
                    TokenStatus::Valid { expires_in }
                }
            }
        }
    }

    async fn revoke(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        self.tokens.write().await.remove(&platform);
        tracing::info!("Revoked authorization for {:?}", platform);
        Ok(())
    }
}

/// Keyring-based credential store
///
/// Stores OAuth tokens securely using the OS credential store:
/// - Windows: Windows Credential Manager
/// - macOS: macOS Keychain
/// - Linux: Secret Service (via libsecret)
pub struct KeyringCredentialStore {
    service_name: String,
    /// Cache of which platforms have credentials (for fast synchronous checks)
    credentials_cache: Mutex<HashMap<SyncPlatform, bool>>,
}

impl KeyringCredentialStore {
    /// Create a new keyring credential store.
    ///
    /// # Arguments
    /// * `service_name` - The service name used for keyring entries (e.g., "RustRide")
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            credentials_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get the keyring username for a platform.
    fn key_for_platform(&self, platform: SyncPlatform) -> String {
        format!("{:?}", platform).to_lowercase()
    }

    /// Create a keyring entry for the given platform.
    fn entry_for_platform(
        &self,
        platform: SyncPlatform,
    ) -> Result<keyring::Entry, SyncError> {
        let key = self.key_for_platform(platform);
        keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| SyncError::CredentialError(format!("Failed to create keyring entry: {}", e)))
    }

    /// Update the credentials cache.
    fn update_cache(&self, platform: SyncPlatform, has_creds: bool) {
        if let Ok(mut cache) = self.credentials_cache.lock() {
            cache.insert(platform, has_creds);
        }
    }
}

impl CredentialStore for KeyringCredentialStore {
    async fn store_tokens(
        &self,
        platform: SyncPlatform,
        tokens: &TokenResponse,
    ) -> Result<(), SyncError> {
        // Serialize tokens to JSON
        let json = serde_json::to_string(tokens)
            .map_err(|e| SyncError::CredentialError(format!("Failed to serialize tokens: {}", e)))?;

        // Create keyring entry and store
        let entry = self.entry_for_platform(platform)?;
        entry
            .set_password(&json)
            .map_err(|e| SyncError::CredentialError(format!("Failed to store credentials: {}", e)))?;

        // Update cache
        self.update_cache(platform, true);

        tracing::debug!("Stored tokens for {:?} in OS keyring", platform);

        Ok(())
    }

    async fn get_tokens(&self, platform: SyncPlatform) -> Result<Option<TokenResponse>, SyncError> {
        let entry = self.entry_for_platform(platform)?;

        match entry.get_password() {
            Ok(json) => {
                // Parse stored JSON back to TokenResponse
                let tokens: StoredTokenResponse = serde_json::from_str(&json)
                    .map_err(|e| SyncError::CredentialError(format!("Failed to parse stored tokens: {}", e)))?;

                // Convert to TokenResponse
                let token_response = tokens.into_token_response()?;

                // Update cache
                self.update_cache(platform, true);

                tracing::debug!("Retrieved tokens for {:?} from OS keyring", platform);
                Ok(Some(token_response))
            }
            Err(keyring::Error::NoEntry) => {
                // No credentials stored - this is not an error
                self.update_cache(platform, false);
                tracing::debug!("No tokens found for {:?} in OS keyring", platform);
                Ok(None)
            }
            Err(e) => {
                // Other keyring errors
                tracing::error!("Failed to retrieve tokens for {:?}: {}", platform, e);
                Err(SyncError::CredentialError(format!(
                    "Failed to retrieve credentials: {}",
                    e
                )))
            }
        }
    }

    async fn delete_tokens(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        let entry = self.entry_for_platform(platform)?;

        match entry.delete_credential() {
            Ok(()) => {
                self.update_cache(platform, false);
                tracing::debug!("Deleted tokens for {:?} from OS keyring", platform);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Already deleted or never existed - not an error
                self.update_cache(platform, false);
                tracing::debug!("No tokens to delete for {:?}", platform);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to delete tokens for {:?}: {}", platform, e);
                Err(SyncError::CredentialError(format!(
                    "Failed to delete credentials: {}",
                    e
                )))
            }
        }
    }

    fn has_credentials(&self, platform: SyncPlatform) -> bool {
        // First check cache for fast lookup
        if let Ok(cache) = self.credentials_cache.lock() {
            if let Some(&has_creds) = cache.get(&platform) {
                return has_creds;
            }
        }

        // Cache miss - check keyring directly
        let has_creds = match self.entry_for_platform(platform) {
            Ok(entry) => entry.get_password().is_ok(),
            Err(_) => false,
        };

        // Update cache
        self.update_cache(platform, has_creds);

        has_creds
    }
}

/// Stored token response format for JSON serialization/deserialization.
/// This mirrors TokenResponse but with string dates for JSON compatibility.
#[derive(Debug, Deserialize)]
struct StoredTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: String, // RFC3339 formatted date string
}

impl StoredTokenResponse {
    /// Convert to TokenResponse, parsing the date string.
    fn into_token_response(self) -> Result<TokenResponse, SyncError> {
        let expires_at = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|e| SyncError::CredentialError(format!("Invalid expires_at date: {}", e)))?
            .with_timezone(&Utc);

        Ok(TokenResponse {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at,
        })
    }
}

// Serialize TokenResponse for storage
impl serde::Serialize for TokenResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TokenResponse", 3)?;
        state.serialize_field("access_token", &self.access_token)?;
        state.serialize_field("refresh_token", &self.refresh_token)?;
        state.serialize_field("expires_at", &self.expires_at.to_rfc3339())?;
        state.end()
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}

/// T104: OAuth callback server for handling redirects.
///
/// Starts a local HTTP server to receive the OAuth callback from the
/// authorization flow. Once the callback is received, it extracts the
/// authorization code and state, then shuts down.
#[allow(dead_code)]
pub struct OAuthCallbackServer {
    port: u16,
    base_url: String,
    callback_port: u16,
    service_name: String,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Callback result from OAuth flow
#[derive(Debug, Clone)]
pub struct CallbackResult {
    /// Authorization code
    pub code: String,
    /// State for CSRF verification
    pub state: String,
}

impl OAuthCallbackServer {
    /// Create a new callback server on the specified port.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            base_url: format!("http://localhost:{}", port),
            callback_port: port,
            service_name: "RustRide".to_string(),
            shutdown_tx: None,
        }
    }

    /// Get the redirect URI for OAuth configuration.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.port)
    }

    /// Start the callback server and wait for authorization.
    ///
    /// Returns the authorization code and state when received.
    /// The server automatically shuts down after receiving the callback.
    pub async fn wait_for_callback(&mut self) -> Result<CallbackResult, SyncError> {
        use std::net::SocketAddr;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            SyncError::NetworkError(format!("Failed to bind callback server: {}", e))
        })?;

        tracing::info!("OAuth callback server listening on {}", addr);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // Wait for connection or shutdown
        let result = tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let (reader, mut writer) = stream.into_split();
                        let mut reader = BufReader::new(reader);
                        let mut request_line = String::new();

                        reader.read_line(&mut request_line).await
                            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

                        // Parse the request
                        let result = Self::parse_callback_request(&request_line)?;

                        // Send response
                        let response = Self::success_html();
                        let http_response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.len(),
                            response
                        );

                        writer.write_all(http_response.as_bytes()).await
                            .map_err(|e| SyncError::NetworkError(e.to_string()))?;
                        writer.flush().await
                            .map_err(|e| SyncError::NetworkError(e.to_string()))?;

                        Ok(result)
                    }
                    Err(e) => Err(SyncError::NetworkError(format!("Accept failed: {}", e)))
                }
            }
            _ = &mut shutdown_rx => {
                Err(SyncError::AuthorizationRequired)
            }
        };

        self.shutdown_tx = None;
        result
    }

    /// Parse the callback request to extract code and state.
    fn parse_callback_request(request_line: &str) -> Result<CallbackResult, SyncError> {
        // Request line format: GET /callback?code=xxx&state=yyy HTTP/1.1
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(SyncError::AuthorizationRequired);
        }

        let path = parts[1];
        if !path.starts_with("/callback?") {
            return Err(SyncError::AuthorizationRequired);
        }

        let query = &path[10..]; // Skip "/callback?"
        let mut code = None;
        let mut state = None;

        for param in query.split('&') {
            let kv: Vec<&str> = param.split('=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "code" => code = Some(kv[1].to_string()),
                    "state" => state = Some(kv[1].to_string()),
                    _ => {}
                }
            }
        }

        match (code, state) {
            (Some(c), Some(s)) => Ok(CallbackResult { code: c, state: s }),
            _ => Err(SyncError::AuthorizationRequired),
        }
    }

    /// Generate success HTML response.
    fn success_html() -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Successful</title>
    <style>
        body { font-family: system-ui; text-align: center; padding: 50px; background: #f5f5f5; }
        .container { background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); max-width: 400px; margin: 0 auto; }
        h1 { color: #4CAF50; }
        p { color: #666; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Success!</h1>
        <p>Authorization complete. You can close this window and return to RustRide.</p>
    </div>
</body>
</html>"#.to_string()
    }

    /// Cancel waiting for callback.
    pub fn cancel(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_generation() {
        let state1 = DefaultOAuthHandler::generate_state();
        let _state2 = DefaultOAuthHandler::generate_state();

        assert!(!state1.is_empty());
        // States should be different (unless generated at exact same nanosecond)
        // Just check they're non-empty for now
    }

    #[tokio::test]
    async fn test_oauth_handler_creation() {
        let handler = DefaultOAuthHandler::new(8888);
        assert!(!handler.is_authorized(SyncPlatform::Strava));
    }

    #[test]
    fn test_token_status() {
        let handler = DefaultOAuthHandler::new(8888);
        let status = handler.get_token_status(SyncPlatform::Strava);
        assert!(matches!(status, TokenStatus::NotConfigured));
    }

    #[tokio::test]
    async fn test_refresh_token_without_tokens_returns_auth_required() {
        let handler = DefaultOAuthHandler::new(8888);

        // Configure Strava but don't store any tokens
        handler
            .configure(
                SyncPlatform::Strava,
                OAuthConfig {
                    client_id: "test_client".to_string(),
                    client_secret: Some("test_secret".to_string()),
                    redirect_uri: "http://localhost:8888/callback".to_string(),
                    scopes: vec!["activity:read_all".to_string()],
                },
            )
            .await;

        let result = handler.refresh_token(SyncPlatform::Strava).await;
        assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
    }

    #[tokio::test]
    async fn test_refresh_token_without_refresh_token_returns_error() {
        let handler = DefaultOAuthHandler::new(8888);

        // Configure Strava
        handler
            .configure(
                SyncPlatform::Strava,
                OAuthConfig {
                    client_id: "test_client".to_string(),
                    client_secret: Some("test_secret".to_string()),
                    redirect_uri: "http://localhost:8888/callback".to_string(),
                    scopes: vec!["activity:read_all".to_string()],
                },
            )
            .await;

        // Store tokens without refresh_token
        handler.tokens.write().await.insert(
            SyncPlatform::Strava,
            TokenResponse {
                access_token: "test_access".to_string(),
                refresh_token: None,
                expires_at: Utc::now() + Duration::hours(1),
            },
        );

        let result = handler.refresh_token(SyncPlatform::Strava).await;
        assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
    }

    #[tokio::test]
    async fn test_refresh_token_without_config_returns_not_configured() {
        let handler = DefaultOAuthHandler::new(8888);

        // Store tokens but don't configure the platform
        handler.tokens.write().await.insert(
            SyncPlatform::Strava,
            TokenResponse {
                access_token: "test_access".to_string(),
                refresh_token: Some("test_refresh".to_string()),
                expires_at: Utc::now() + Duration::hours(1),
            },
        );

        let result = handler.refresh_token(SyncPlatform::Strava).await;
        assert!(matches!(result, Err(SyncError::NotConfigured(_))));
    }

    // === KeyringCredentialStore Tests ===

    #[test]
    fn test_keyring_store_key_for_platform() {
        let store = KeyringCredentialStore::new("TestService");
        assert_eq!(store.key_for_platform(SyncPlatform::Strava), "strava");
        assert_eq!(
            store.key_for_platform(SyncPlatform::GarminConnect),
            "garminconnect"
        );
        assert_eq!(
            store.key_for_platform(SyncPlatform::TrainingPeaks),
            "trainingpeaks"
        );
    }

    #[test]
    fn test_token_response_serialization() {
        let token = TokenResponse {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            expires_at: Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&token).expect("Serialization should succeed");

        // Verify JSON contains expected fields
        assert!(json.contains("test_access_token"));
        assert!(json.contains("test_refresh_token"));
        assert!(json.contains("2025-06-15"));
    }

    #[test]
    fn test_token_response_serialization_without_refresh_token() {
        let token = TokenResponse {
            access_token: "test_access_token".to_string(),
            refresh_token: None,
            expires_at: Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap(),
        };

        let json = serde_json::to_string(&token).expect("Serialization should succeed");
        assert!(json.contains("test_access_token"));
        assert!(json.contains("null")); // refresh_token should be null
    }

    #[test]
    fn test_stored_token_response_deserialization() {
        let json = r#"{
            "access_token": "my_access_token",
            "refresh_token": "my_refresh_token",
            "expires_at": "2025-06-15T12:00:00+00:00"
        }"#;

        let stored: StoredTokenResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");
        let token = stored
            .into_token_response()
            .expect("Conversion should succeed");

        assert_eq!(token.access_token, "my_access_token");
        assert_eq!(token.refresh_token, Some("my_refresh_token".to_string()));
        assert_eq!(
            token.expires_at,
            Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_stored_token_response_deserialization_null_refresh() {
        let json = r#"{
            "access_token": "my_access_token",
            "refresh_token": null,
            "expires_at": "2025-06-15T12:00:00+00:00"
        }"#;

        let stored: StoredTokenResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");
        let token = stored
            .into_token_response()
            .expect("Conversion should succeed");

        assert_eq!(token.access_token, "my_access_token");
        assert_eq!(token.refresh_token, None);
    }

    #[test]
    fn test_stored_token_response_invalid_date() {
        let json = r#"{
            "access_token": "my_access_token",
            "refresh_token": "my_refresh_token",
            "expires_at": "not-a-valid-date"
        }"#;

        let stored: StoredTokenResponse =
            serde_json::from_str(json).expect("Deserialization should succeed");
        let result = stored.into_token_response();

        assert!(matches!(result, Err(SyncError::CredentialError(_))));
    }

    #[test]
    fn test_keyring_store_creation() {
        let store = KeyringCredentialStore::new("TestService");
        // Initially, credentials cache should be empty
        assert!(!store.has_credentials(SyncPlatform::Strava));
    }

    #[test]
    fn test_token_roundtrip_serialization() {
        // Create a token, serialize it, then deserialize and verify
        let original = TokenResponse {
            access_token: "access123".to_string(),
            refresh_token: Some("refresh456".to_string()),
            expires_at: Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap(),
        };

        // Serialize (this is what store_tokens does)
        let json = serde_json::to_string(&original).expect("Serialization should succeed");

        // Deserialize (this is what get_tokens does)
        let stored: StoredTokenResponse =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        let restored = stored
            .into_token_response()
            .expect("Conversion should succeed");

        // Verify roundtrip
        assert_eq!(original.access_token, restored.access_token);
        assert_eq!(original.refresh_token, restored.refresh_token);
        assert_eq!(original.expires_at, restored.expires_at);
    }

    // === HTTP Mocked Tests for Token Exchange and Refresh ===

    mod http_mocked_tests {
        use super::*;
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        /// Helper to set up handler with mock server URL
        async fn setup_handler_with_mock_server(
            mock_server: &MockServer,
        ) -> DefaultOAuthHandler {
            let handler = DefaultOAuthHandler::with_strava_token_url(
                8888,
                format!("{}/oauth/token", mock_server.uri()),
            );

            // Configure Strava OAuth
            handler
                .configure(
                    SyncPlatform::Strava,
                    OAuthConfig {
                        client_id: "test_client_id".to_string(),
                        client_secret: Some("test_client_secret".to_string()),
                        redirect_uri: "http://localhost:8888/callback".to_string(),
                        scopes: vec!["activity:read_all".to_string(), "activity:write".to_string()],
                    },
                )
                .await;

            // Add a pending state for the callback
            handler
                .pending_states
                .write()
                .await
                .insert("test_state".to_string(), SyncPlatform::Strava);

            handler
        }

        #[tokio::test]
        async fn test_token_exchange_success() {
            let mock_server = MockServer::start().await;

            // Set up mock for successful token exchange
            let expires_at = Utc::now().timestamp() + 21600; // 6 hours from now
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=authorization_code"))
                .and(body_string_contains("code=test_auth_code"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "token_type": "Bearer",
                    "access_token": "mock_access_token_12345",
                    "refresh_token": "mock_refresh_token_67890",
                    "expires_at": expires_at
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("test_auth_code", "test_state").await;

            assert!(result.is_ok(), "Token exchange should succeed");
            let token = result.unwrap();
            assert_eq!(token.access_token, "mock_access_token_12345");
            assert_eq!(token.refresh_token, Some("mock_refresh_token_67890".to_string()));
            assert!(handler.is_authorized(SyncPlatform::Strava));
        }

        #[tokio::test]
        async fn test_token_exchange_invalid_code_error() {
            let mock_server = MockServer::start().await;

            // Set up mock for error response (invalid authorization code)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "message": "Bad Request",
                    "errors": [{
                        "resource": "Application",
                        "field": "code",
                        "code": "invalid"
                    }]
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("invalid_code", "test_state").await;

            assert!(result.is_err());
            match result {
                Err(SyncError::ApiError(msg)) => {
                    assert!(msg.contains("Bad Request"), "Error should contain message");
                }
                _ => panic!("Expected ApiError, got {:?}", result),
            }
        }

        #[tokio::test]
        async fn test_token_exchange_invalid_json_response() {
            let mock_server = MockServer::start().await;

            // Set up mock that returns invalid JSON
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("test_code", "test_state").await;

            assert!(result.is_err());
            match result {
                Err(SyncError::ApiError(msg)) => {
                    assert!(msg.contains("Failed to parse"), "Error should mention parsing failure");
                }
                _ => panic!("Expected ApiError, got {:?}", result),
            }
        }

        #[tokio::test]
        async fn test_token_exchange_server_error() {
            let mock_server = MockServer::start().await;

            // Set up mock for server error (500)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("test_code", "test_state").await;

            assert!(result.is_err());
            match result {
                Err(SyncError::ApiError(msg)) => {
                    assert!(msg.contains("500"), "Error should contain status code");
                }
                _ => panic!("Expected ApiError, got {:?}", result),
            }
        }

        #[tokio::test]
        async fn test_token_exchange_invalid_state() {
            let mock_server = MockServer::start().await;
            let handler = setup_handler_with_mock_server(&mock_server).await;

            // Use wrong state - no HTTP call should be made
            let result = handler.handle_callback("test_code", "wrong_state").await;

            assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
        }

        #[tokio::test]
        async fn test_token_refresh_success() {
            let mock_server = MockServer::start().await;

            let new_expires_at = Utc::now().timestamp() + 21600; // 6 hours from now
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=refresh_token"))
                .and(body_string_contains("refresh_token=original_refresh_token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "token_type": "Bearer",
                    "access_token": "new_access_token_abc",
                    "refresh_token": "new_refresh_token_xyz",
                    "expires_at": new_expires_at
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            // Set up existing tokens
            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access_token".to_string(),
                    refresh_token: Some("original_refresh_token".to_string()),
                    expires_at: Utc::now() - Duration::hours(1), // Expired
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(result.is_ok(), "Token refresh should succeed");
            let token = result.unwrap();
            assert_eq!(token.access_token, "new_access_token_abc");
            assert_eq!(token.refresh_token, Some("new_refresh_token_xyz".to_string()));
        }

        #[tokio::test]
        async fn test_token_refresh_expired_refresh_token_requires_reauth() {
            let mock_server = MockServer::start().await;

            // Mock returns error indicating refresh token is expired
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=refresh_token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "message": "Bad Request",
                    "errors": [{
                        "resource": "RefreshToken",
                        "field": "refresh_token",
                        "code": "expired"
                    }]
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("expired_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
        }

        #[tokio::test]
        async fn test_token_refresh_invalid_refresh_token_requires_reauth() {
            let mock_server = MockServer::start().await;

            // Mock returns error indicating refresh token is invalid
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "message": "Bad Request",
                    "errors": [{
                        "resource": "RefreshToken",
                        "field": "refresh_token",
                        "code": "invalid"
                    }]
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("invalid_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
        }

        #[tokio::test]
        async fn test_token_refresh_revoked_refresh_token_requires_reauth() {
            let mock_server = MockServer::start().await;

            // Mock returns error indicating refresh token is revoked
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "message": "Bad Request",
                    "errors": [{
                        "resource": "RefreshToken",
                        "field": "refresh_token",
                        "code": "revoked"
                    }]
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("revoked_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
        }

        #[tokio::test]
        async fn test_token_refresh_generic_api_error() {
            let mock_server = MockServer::start().await;

            // Mock returns a generic API error (not refresh token specific)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "message": "Bad Request",
                    "errors": [{
                        "resource": "Application",
                        "field": "client_id",
                        "code": "invalid"
                    }]
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("valid_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
        }

        #[tokio::test]
        async fn test_token_refresh_server_error() {
            let mock_server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("valid_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::Strava).await;

            assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
        }

        #[tokio::test]
        async fn test_token_status_expired() {
            let handler = DefaultOAuthHandler::new(8888);

            // Store expired tokens
            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "expired_token".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let status = handler.get_token_status(SyncPlatform::Strava);
            assert!(matches!(status, TokenStatus::Expired));
        }

        #[tokio::test]
        async fn test_token_status_needs_refresh() {
            let handler = DefaultOAuthHandler::new(8888);

            // Store tokens expiring in 3 minutes (less than 5 minute threshold)
            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "expiring_soon".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    expires_at: Utc::now() + Duration::minutes(3),
                },
            );

            let status = handler.get_token_status(SyncPlatform::Strava);
            assert!(matches!(status, TokenStatus::NeedsRefresh));
        }

        #[tokio::test]
        async fn test_token_status_valid() {
            let handler = DefaultOAuthHandler::new(8888);

            // Store tokens valid for 6 hours
            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "valid_token".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    expires_at: Utc::now() + Duration::hours(6),
                },
            );

            let status = handler.get_token_status(SyncPlatform::Strava);
            match status {
                TokenStatus::Valid { expires_in } => {
                    // Should be approximately 6 hours (allow some tolerance)
                    assert!(expires_in.as_secs() > 5 * 60 * 60); // > 5 hours
                    assert!(expires_in.as_secs() <= 6 * 60 * 60 + 60); // <= 6h + 1min buffer
                }
                _ => panic!("Expected TokenStatus::Valid"),
            }
        }

        #[tokio::test]
        async fn test_start_authorization_builds_correct_url() {
            let handler = DefaultOAuthHandler::new(8888);

            handler
                .configure(
                    SyncPlatform::Strava,
                    OAuthConfig {
                        client_id: "my_client_id".to_string(),
                        client_secret: Some("my_secret".to_string()),
                        redirect_uri: "http://localhost:8888/callback".to_string(),
                        scopes: vec!["activity:read_all".to_string(), "activity:write".to_string()],
                    },
                )
                .await;

            let result = handler.start_authorization(SyncPlatform::Strava).await;

            assert!(result.is_ok());
            let auth_url = result.unwrap();
            assert!(auth_url.url.contains("client_id=my_client_id"));
            assert!(auth_url.url.contains("redirect_uri="));
            assert!(auth_url.url.contains("scope="));
            assert!(auth_url.url.contains("state="));
            assert!(!auth_url.state.is_empty());
        }

        #[tokio::test]
        async fn test_revoke_clears_tokens() {
            let handler = DefaultOAuthHandler::new(8888);

            // Store tokens
            handler.tokens.write().await.insert(
                SyncPlatform::Strava,
                TokenResponse {
                    access_token: "token".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    expires_at: Utc::now() + Duration::hours(1),
                },
            );

            assert!(handler.is_authorized(SyncPlatform::Strava));

            let result = handler.revoke(SyncPlatform::Strava).await;
            assert!(result.is_ok());
            assert!(!handler.is_authorized(SyncPlatform::Strava));
        }

        // === TrainingPeaks OAuth Tests ===

        /// Helper to set up handler with mock server URL for TrainingPeaks
        async fn setup_trainingpeaks_handler_with_mock_server(
            mock_server: &MockServer,
        ) -> DefaultOAuthHandler {
            let handler = DefaultOAuthHandler::with_token_urls(
                8888,
                STRAVA_TOKEN_URL.to_string(),
                format!("{}/oauth/token", mock_server.uri()),
            );

            // Configure TrainingPeaks OAuth
            handler
                .configure(
                    SyncPlatform::TrainingPeaks,
                    OAuthConfig {
                        client_id: "tp_client_id".to_string(),
                        client_secret: Some("tp_client_secret".to_string()),
                        redirect_uri: "http://localhost:8888/callback".to_string(),
                        scopes: vec!["athlete:profile".to_string(), "workouts:read".to_string(), "file:write".to_string()],
                    },
                )
                .await;

            // Add a pending state for the callback
            handler
                .pending_states
                .write()
                .await
                .insert("tp_test_state".to_string(), SyncPlatform::TrainingPeaks);

            handler
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_exchange_success() {
            let mock_server = MockServer::start().await;

            // Set up mock for successful token exchange
            // TrainingPeaks returns expires_in (seconds) instead of expires_at (timestamp)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=authorization_code"))
                .and(body_string_contains("code=tp_auth_code"))
                .and(body_string_contains("client_id=tp_client_id"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "token_type": "Bearer",
                    "access_token": "tp_access_token_12345",
                    "refresh_token": "tp_refresh_token_67890",
                    "expires_in": 21600  // 6 hours in seconds
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("tp_auth_code", "tp_test_state").await;

            assert!(result.is_ok(), "TrainingPeaks token exchange should succeed");
            let token = result.unwrap();
            assert_eq!(token.access_token, "tp_access_token_12345");
            assert_eq!(token.refresh_token, Some("tp_refresh_token_67890".to_string()));
            assert!(handler.is_authorized(SyncPlatform::TrainingPeaks));
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_exchange_invalid_code_error() {
            let mock_server = MockServer::start().await;

            // Set up mock for error response (invalid authorization code)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "error": "invalid_grant",
                    "error_description": "The authorization code is invalid or expired."
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("invalid_code", "tp_test_state").await;

            assert!(result.is_err());
            match result {
                Err(SyncError::ApiError(msg)) => {
                    assert!(msg.contains("invalid_grant"), "Error should contain error type");
                }
                _ => panic!("Expected ApiError, got {:?}", result),
            }
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_exchange_server_error() {
            let mock_server = MockServer::start().await;

            // Set up mock for server error (500)
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            let result = handler.handle_callback("test_code", "tp_test_state").await;

            assert!(result.is_err());
            match result {
                Err(SyncError::ApiError(msg)) => {
                    assert!(msg.contains("500"), "Error should contain status code");
                }
                _ => panic!("Expected ApiError, got {:?}", result),
            }
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_refresh_success() {
            let mock_server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=refresh_token"))
                .and(body_string_contains("refresh_token=tp_original_refresh"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "token_type": "Bearer",
                    "access_token": "tp_new_access_token",
                    "refresh_token": "tp_new_refresh_token",
                    "expires_in": 21600
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            // Set up existing tokens
            handler.tokens.write().await.insert(
                SyncPlatform::TrainingPeaks,
                TokenResponse {
                    access_token: "tp_old_access".to_string(),
                    refresh_token: Some("tp_original_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1), // Expired
                },
            );

            let result = handler.refresh_token(SyncPlatform::TrainingPeaks).await;

            assert!(result.is_ok(), "TrainingPeaks token refresh should succeed");
            let token = result.unwrap();
            assert_eq!(token.access_token, "tp_new_access_token");
            assert_eq!(token.refresh_token, Some("tp_new_refresh_token".to_string()));
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_refresh_invalid_grant_requires_reauth() {
            let mock_server = MockServer::start().await;

            // Mock returns error indicating refresh token is expired
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .and(body_string_contains("grant_type=refresh_token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "error": "invalid_grant",
                    "error_description": "The refresh token is expired or revoked."
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::TrainingPeaks,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("expired_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::TrainingPeaks).await;

            assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
        }

        #[tokio::test]
        async fn test_trainingpeaks_token_refresh_generic_error() {
            let mock_server = MockServer::start().await;

            // Mock returns a generic error
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "error": "server_error",
                    "error_description": "An internal error occurred."
                })))
                .expect(1)
                .mount(&mock_server)
                .await;

            let handler = setup_trainingpeaks_handler_with_mock_server(&mock_server).await;

            handler.tokens.write().await.insert(
                SyncPlatform::TrainingPeaks,
                TokenResponse {
                    access_token: "old_access".to_string(),
                    refresh_token: Some("valid_refresh".to_string()),
                    expires_at: Utc::now() - Duration::hours(1),
                },
            );

            let result = handler.refresh_token(SyncPlatform::TrainingPeaks).await;

            assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
        }

        #[tokio::test]
        async fn test_trainingpeaks_start_authorization_builds_correct_url() {
            let handler = DefaultOAuthHandler::new(8888);

            handler
                .configure(
                    SyncPlatform::TrainingPeaks,
                    OAuthConfig {
                        client_id: "tp_my_client_id".to_string(),
                        client_secret: Some("tp_my_secret".to_string()),
                        redirect_uri: "http://localhost:8888/callback".to_string(),
                        scopes: vec!["athlete:profile".to_string(), "workouts:read".to_string()],
                    },
                )
                .await;

            let result = handler.start_authorization(SyncPlatform::TrainingPeaks).await;

            assert!(result.is_ok());
            let auth_url = result.unwrap();
            assert!(auth_url.url.contains("client_id=tp_my_client_id"));
            assert!(auth_url.url.contains("oauth.trainingpeaks.com"));
            assert!(auth_url.url.contains("redirect_uri="));
            assert!(auth_url.url.contains("scope="));
            assert!(auth_url.url.contains("state="));
            assert!(!auth_url.state.is_empty());
        }
    }

    // === Credential Store Mock Tests ===

    mod credential_store_tests {
        use super::*;

        /// A mock credential store for testing that stores tokens in memory
        struct MockCredentialStore {
            tokens: std::sync::RwLock<HashMap<SyncPlatform, TokenResponse>>,
        }

        impl MockCredentialStore {
            fn new() -> Self {
                Self {
                    tokens: std::sync::RwLock::new(HashMap::new()),
                }
            }
        }

        impl CredentialStore for MockCredentialStore {
            async fn store_tokens(
                &self,
                platform: SyncPlatform,
                tokens: &TokenResponse,
            ) -> Result<(), SyncError> {
                self.tokens.write().unwrap().insert(platform, tokens.clone());
                Ok(())
            }

            async fn get_tokens(
                &self,
                platform: SyncPlatform,
            ) -> Result<Option<TokenResponse>, SyncError> {
                Ok(self.tokens.read().unwrap().get(&platform).cloned())
            }

            async fn delete_tokens(&self, platform: SyncPlatform) -> Result<(), SyncError> {
                self.tokens.write().unwrap().remove(&platform);
                Ok(())
            }

            fn has_credentials(&self, platform: SyncPlatform) -> bool {
                self.tokens.read().unwrap().contains_key(&platform)
            }
        }

        #[tokio::test]
        async fn test_mock_store_store_and_retrieve() {
            let store = MockCredentialStore::new();

            let tokens = TokenResponse {
                access_token: "test_access".to_string(),
                refresh_token: Some("test_refresh".to_string()),
                expires_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            };

            // Store tokens
            store.store_tokens(SyncPlatform::Strava, &tokens).await.unwrap();

            // Verify has_credentials
            assert!(store.has_credentials(SyncPlatform::Strava));
            assert!(!store.has_credentials(SyncPlatform::GarminConnect));

            // Retrieve and verify
            let retrieved = store.get_tokens(SyncPlatform::Strava).await.unwrap();
            assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            assert_eq!(retrieved.access_token, "test_access");
            assert_eq!(retrieved.refresh_token, Some("test_refresh".to_string()));
        }

        #[tokio::test]
        async fn test_mock_store_delete() {
            let store = MockCredentialStore::new();

            let tokens = TokenResponse {
                access_token: "test".to_string(),
                refresh_token: None,
                expires_at: Utc::now(),
            };

            store.store_tokens(SyncPlatform::Strava, &tokens).await.unwrap();
            assert!(store.has_credentials(SyncPlatform::Strava));

            store.delete_tokens(SyncPlatform::Strava).await.unwrap();
            assert!(!store.has_credentials(SyncPlatform::Strava));
        }

        #[tokio::test]
        async fn test_mock_store_get_nonexistent() {
            let store = MockCredentialStore::new();

            let result = store.get_tokens(SyncPlatform::Strava).await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_mock_store_overwrite() {
            let store = MockCredentialStore::new();

            let tokens1 = TokenResponse {
                access_token: "first".to_string(),
                refresh_token: Some("refresh1".to_string()),
                expires_at: Utc::now(),
            };

            let tokens2 = TokenResponse {
                access_token: "second".to_string(),
                refresh_token: Some("refresh2".to_string()),
                expires_at: Utc::now(),
            };

            store.store_tokens(SyncPlatform::Strava, &tokens1).await.unwrap();
            store.store_tokens(SyncPlatform::Strava, &tokens2).await.unwrap();

            let retrieved = store.get_tokens(SyncPlatform::Strava).await.unwrap().unwrap();
            assert_eq!(retrieved.access_token, "second");
        }

        #[tokio::test]
        async fn test_mock_store_multiple_platforms() {
            let store = MockCredentialStore::new();

            let strava_tokens = TokenResponse {
                access_token: "strava_token".to_string(),
                refresh_token: Some("strava_refresh".to_string()),
                expires_at: Utc::now(),
            };

            let garmin_tokens = TokenResponse {
                access_token: "garmin_token".to_string(),
                refresh_token: Some("garmin_refresh".to_string()),
                expires_at: Utc::now(),
            };

            store.store_tokens(SyncPlatform::Strava, &strava_tokens).await.unwrap();
            store.store_tokens(SyncPlatform::GarminConnect, &garmin_tokens).await.unwrap();

            assert!(store.has_credentials(SyncPlatform::Strava));
            assert!(store.has_credentials(SyncPlatform::GarminConnect));

            let strava = store.get_tokens(SyncPlatform::Strava).await.unwrap().unwrap();
            let garmin = store.get_tokens(SyncPlatform::GarminConnect).await.unwrap().unwrap();

            assert_eq!(strava.access_token, "strava_token");
            assert_eq!(garmin.access_token, "garmin_token");

            // Delete just Strava
            store.delete_tokens(SyncPlatform::Strava).await.unwrap();
            assert!(!store.has_credentials(SyncPlatform::Strava));
            assert!(store.has_credentials(SyncPlatform::GarminConnect));
        }

        // Test the keyring credential store cache behavior (without actual keyring)
        #[test]
        fn test_keyring_cache_update() {
            let store = KeyringCredentialStore::new("TestService");

            // Initially empty
            assert!(!store.has_credentials(SyncPlatform::Strava));

            // Manually update cache (simulating what store_tokens does)
            store.update_cache(SyncPlatform::Strava, true);

            // Check cache returns true
            if let Ok(cache) = store.credentials_cache.lock() {
                assert_eq!(cache.get(&SyncPlatform::Strava), Some(&true));
            }

            // Update to false
            store.update_cache(SyncPlatform::Strava, false);

            if let Ok(cache) = store.credentials_cache.lock() {
                assert_eq!(cache.get(&SyncPlatform::Strava), Some(&false));
            }
        }

        #[test]
        fn test_keyring_key_generation_all_platforms() {
            let store = KeyringCredentialStore::new("RustRide");

            // Test all platforms generate valid, distinct keys
            let strava_key = store.key_for_platform(SyncPlatform::Strava);
            let garmin_key = store.key_for_platform(SyncPlatform::GarminConnect);
            let tp_key = store.key_for_platform(SyncPlatform::TrainingPeaks);
            let intervals_key = store.key_for_platform(SyncPlatform::IntervalsIcu);

            // All should be lowercase
            assert!(strava_key.chars().all(|c| c.is_lowercase() || c.is_numeric()));
            assert!(garmin_key.chars().all(|c| c.is_lowercase() || c.is_numeric()));
            assert!(tp_key.chars().all(|c| c.is_lowercase() || c.is_numeric()));
            assert!(intervals_key.chars().all(|c| c.is_lowercase() || c.is_numeric()));

            // All should be distinct
            let keys = vec![&strava_key, &garmin_key, &tp_key, &intervals_key];
            let unique: std::collections::HashSet<_> = keys.iter().collect();
            assert_eq!(keys.len(), unique.len(), "All platform keys should be unique");
        }
    }

    // === Strava Response Parsing Tests ===

    mod response_parsing_tests {
        use super::*;

        #[test]
        fn test_strava_token_response_deserialization() {
            let json = r#"{
                "token_type": "Bearer",
                "access_token": "abc123",
                "refresh_token": "def456",
                "expires_at": 1704067200
            }"#;

            let response: StravaTokenResponse = serde_json::from_str(json).unwrap();
            assert_eq!(response.access_token, "abc123");
            assert_eq!(response.refresh_token, "def456");
            assert_eq!(response.expires_at, 1704067200);
            assert_eq!(response.token_type, "Bearer");
        }

        #[test]
        fn test_strava_error_response_simple() {
            let json = r#"{
                "message": "Bad Request",
                "errors": []
            }"#;

            let error: StravaErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.message, "Bad Request");
            assert!(error.errors.is_empty());
            assert_eq!(format!("{}", error), "Bad Request");
        }

        #[test]
        fn test_strava_error_response_with_field_errors() {
            let json = r#"{
                "message": "Bad Request",
                "errors": [
                    {"resource": "Application", "field": "code", "code": "invalid"},
                    {"resource": "Application", "field": "client_id", "code": "missing"}
                ]
            }"#;

            let error: StravaErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.message, "Bad Request");
            assert_eq!(error.errors.len(), 2);

            let display = format!("{}", error);
            assert!(display.contains("Bad Request"));
            assert!(display.contains("code: invalid"));
            assert!(display.contains("client_id: missing"));
        }

        #[test]
        fn test_strava_error_response_without_errors_field() {
            // Strava sometimes returns just the message
            let json = r#"{"message": "Authorization Error"}"#;

            let error: StravaErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.message, "Authorization Error");
            assert!(error.errors.is_empty()); // Default empty vec
        }

        // === TrainingPeaks Response Parsing Tests ===

        #[test]
        fn test_trainingpeaks_token_response_deserialization() {
            // TrainingPeaks uses standard OAuth2 snake_case format
            let json = r#"{
                "token_type": "Bearer",
                "access_token": "tp_abc123",
                "refresh_token": "tp_def456",
                "expires_in": 21600
            }"#;

            let response: TrainingPeaksTokenResponse = serde_json::from_str(json).unwrap();
            assert_eq!(response.access_token, "tp_abc123");
            assert_eq!(response.refresh_token, "tp_def456");
            assert_eq!(response.expires_in, 21600);
            assert_eq!(response.token_type, "Bearer");
        }

        #[test]
        fn test_trainingpeaks_error_response_with_description() {
            let json = r#"{
                "error": "invalid_grant",
                "error_description": "The authorization code is invalid or expired."
            }"#;

            let error: TrainingPeaksOAuthErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.error, "invalid_grant");
            assert_eq!(error.error_description, Some("The authorization code is invalid or expired.".to_string()));

            let display = format!("{}", error);
            assert!(display.contains("invalid_grant"));
            assert!(display.contains("The authorization code is invalid or expired."));
        }

        #[test]
        fn test_trainingpeaks_error_response_without_description() {
            // Some OAuth errors only have the error code
            let json = r#"{"error": "server_error"}"#;

            let error: TrainingPeaksOAuthErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.error, "server_error");
            assert_eq!(error.error_description, None);
            assert_eq!(format!("{}", error), "server_error");
        }

        #[test]
        fn test_trainingpeaks_error_response_invalid_client() {
            let json = r#"{
                "error": "invalid_client",
                "error_description": "The client_id or client_secret is incorrect."
            }"#;

            let error: TrainingPeaksOAuthErrorResponse = serde_json::from_str(json).unwrap();
            assert_eq!(error.error, "invalid_client");

            let display = format!("{}", error);
            assert!(display.contains("invalid_client"));
            assert!(display.contains("client_id or client_secret"));
        }
    }
}
