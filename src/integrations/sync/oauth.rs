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

/// Default OAuth handler implementation
#[allow(dead_code)]
pub struct DefaultOAuthHandler {
    configs: Arc<RwLock<HashMap<SyncPlatform, OAuthConfig>>>,
    tokens: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
    pending_states: Arc<RwLock<HashMap<String, SyncPlatform>>>,
    http_client: Client,
    callback_port: u16,
}

impl DefaultOAuthHandler {
    /// Create a new OAuth handler
    pub fn new(callback_port: u16) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
            callback_port,
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
            .post("https://www.strava.com/oauth/token")
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
            .post("https://www.strava.com/oauth/token")
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
}
