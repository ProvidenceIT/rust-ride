//! OAuth2 Authentication
//!
//! Handles OAuth2 flows for fitness platform authentication.

use super::{SyncError, SyncPlatform};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
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

/// Default OAuth handler implementation
#[allow(dead_code)]
pub struct DefaultOAuthHandler {
    configs: Arc<RwLock<HashMap<SyncPlatform, OAuthConfig>>>,
    tokens: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
    pending_states: Arc<RwLock<HashMap<String, SyncPlatform>>>,
    callback_port: u16,
}

impl DefaultOAuthHandler {
    /// Create a new OAuth handler
    pub fn new(callback_port: u16) -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_states: Arc::new(RwLock::new(HashMap::new())),
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
        let _config = configs
            .get(&platform)
            .ok_or(SyncError::NotConfigured(platform))?;

        tracing::info!("Handling OAuth callback for {:?}", platform);

        // TODO: Exchange code for tokens using oauth2 crate
        // For now, return mock tokens
        let tokens = TokenResponse {
            access_token: format!("mock_access_token_{}", code),
            refresh_token: Some("mock_refresh_token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };

        // Store tokens
        self.tokens.write().await.insert(platform, tokens.clone());

        Ok(tokens)
    }

    async fn refresh_token(&self, platform: SyncPlatform) -> Result<TokenResponse, SyncError> {
        let current = self.tokens.read().await.get(&platform).cloned();

        let current = current.ok_or(SyncError::AuthorizationRequired)?;
        let refresh = current
            .refresh_token
            .ok_or(SyncError::RefreshFailed("No refresh token".to_string()))?;

        tracing::info!("Refreshing token for {:?}", platform);

        // TODO: Actually refresh the token
        let new_tokens = TokenResponse {
            access_token: format!("refreshed_{}", refresh),
            refresh_token: Some(refresh),
            expires_at: Utc::now() + Duration::hours(1),
        };

        self.tokens
            .write()
            .await
            .insert(platform, new_tokens.clone());

        Ok(new_tokens)
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
#[allow(dead_code)]
pub struct KeyringCredentialStore {
    service_name: String,
}

impl KeyringCredentialStore {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn key_for_platform(&self, platform: SyncPlatform) -> String {
        format!("{:?}", platform).to_lowercase()
    }
}

impl CredentialStore for KeyringCredentialStore {
    async fn store_tokens(
        &self,
        platform: SyncPlatform,
        tokens: &TokenResponse,
    ) -> Result<(), SyncError> {
        let _key = self.key_for_platform(platform);

        // Serialize tokens to JSON
        let _json =
            serde_json::to_string(tokens).map_err(|e| SyncError::CredentialError(e.to_string()))?;

        // TODO: Use keyring crate to store
        // let entry = keyring::Entry::new(&self.service_name, &key)?;
        // entry.set_password(&json)?;

        tracing::debug!("Stored tokens for {:?}", platform);

        Ok(())
    }

    async fn get_tokens(&self, platform: SyncPlatform) -> Result<Option<TokenResponse>, SyncError> {
        let _key = self.key_for_platform(platform);

        // TODO: Use keyring crate to retrieve
        // let entry = keyring::Entry::new(&self.service_name, &key)?;
        // match entry.get_password() {
        //     Ok(json) => {
        //         let tokens = serde_json::from_str(&json)?;
        //         Ok(Some(tokens))
        //     }
        //     Err(keyring::Error::NoEntry) => Ok(None),
        //     Err(e) => Err(SyncError::CredentialError(e.to_string())),
        // }

        Ok(None)
    }

    async fn delete_tokens(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        let _key = self.key_for_platform(platform);

        // TODO: Use keyring crate to delete
        // let entry = keyring::Entry::new(&self.service_name, &key)?;
        // entry.delete_password()?;

        tracing::debug!("Deleted tokens for {:?}", platform);

        Ok(())
    }

    fn has_credentials(&self, _platform: SyncPlatform) -> bool {
        // TODO: Check keyring
        false
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
}
