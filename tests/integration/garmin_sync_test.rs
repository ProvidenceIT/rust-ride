//! Integration tests for Garmin Connect sync flow.
//!
//! Tests the end-to-end flow of connecting, uploading, and disconnecting
//! from Garmin Connect, using mocked HTTP responses via wiremock.

use chrono::{Duration, Utc};
use rustride::integrations::sync::{
    ErrorCategory, GarminClient, GarminUserProfile, PlatformConfig, SyncConfig, SyncError,
    SyncErrorExt, SyncEvent, SyncPlatform, SyncRecordStatus,
};
use rustride::integrations::sync::oauth::{
    AuthorizationUrl, CredentialStore, OAuthHandler, TokenResponse, TokenStatus,
};
use rustride::integrations::sync::service::{SyncService, SyncServiceHandle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Mock OAuth Handler for Garmin Connect
// ============================================================================

/// Mock OAuth handler for Garmin Connect testing
struct GarminMockOAuthHandler {
    /// Simulated tokens per platform
    tokens: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
    /// Whether to simulate refresh failure
    simulate_refresh_failure: AtomicBool,
    /// Whether to require re-authorization
    simulate_reauth_required: AtomicBool,
    /// Count of refresh attempts
    refresh_count: AtomicU32,
}

impl GarminMockOAuthHandler {
    fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            simulate_refresh_failure: AtomicBool::new(false),
            simulate_reauth_required: AtomicBool::new(false),
            refresh_count: AtomicU32::new(0),
        }
    }

    /// Pre-configure with valid tokens for Garmin Connect
    async fn set_garmin_tokens(&self, tokens: TokenResponse) {
        self.tokens
            .write()
            .await
            .insert(SyncPlatform::GarminConnect, tokens);
    }

    fn set_refresh_failure(&self, should_fail: bool) {
        self.simulate_refresh_failure.store(should_fail, Ordering::SeqCst);
    }

    fn set_reauth_required(&self, required: bool) {
        self.simulate_reauth_required.store(required, Ordering::SeqCst);
    }

    fn get_refresh_count(&self) -> u32 {
        self.refresh_count.load(Ordering::SeqCst)
    }
}

impl OAuthHandler for GarminMockOAuthHandler {
    async fn start_authorization(
        &self,
        platform: SyncPlatform,
    ) -> Result<AuthorizationUrl, SyncError> {
        Ok(AuthorizationUrl {
            url: format!("https://connect.garmin.com/oauthConfirm?platform={:?}", platform),
            state: "garmin_test_state".to_string(),
        })
    }

    async fn handle_callback(
        &self,
        _code: &str,
        _state: &str,
    ) -> Result<TokenResponse, SyncError> {
        let tokens = TokenResponse {
            access_token: "garmin_mock_access_token".to_string(),
            refresh_token: Some("garmin_mock_refresh_token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };
        // Store the tokens
        self.tokens
            .write()
            .await
            .insert(SyncPlatform::GarminConnect, tokens.clone());
        Ok(tokens)
    }

    async fn refresh_token(&self, platform: SyncPlatform) -> Result<TokenResponse, SyncError> {
        self.refresh_count.fetch_add(1, Ordering::SeqCst);

        if self.simulate_reauth_required.load(Ordering::SeqCst) {
            return Err(SyncError::AuthorizationRequired);
        }

        if self.simulate_refresh_failure.load(Ordering::SeqCst) {
            return Err(SyncError::RefreshFailed("Simulated refresh failure".to_string()));
        }

        let new_tokens = TokenResponse {
            access_token: format!("garmin_refreshed_token_{}", self.get_refresh_count()),
            refresh_token: Some("garmin_mock_refresh_token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };

        self.tokens.write().await.insert(platform, new_tokens.clone());
        Ok(new_tokens)
    }

    fn is_authorized(&self, platform: SyncPlatform) -> bool {
        self.tokens
            .try_read()
            .map(|t| t.contains_key(&platform))
            .unwrap_or(false)
    }

    fn get_token_status(&self, platform: SyncPlatform) -> TokenStatus {
        if let Ok(tokens) = self.tokens.try_read() {
            if let Some(token) = tokens.get(&platform) {
                let now = Utc::now();
                if token.expires_at <= now {
                    TokenStatus::Expired
                } else if token.expires_at <= now + Duration::minutes(5) {
                    TokenStatus::NeedsRefresh
                } else {
                    let expires_in = (token.expires_at - now).to_std().unwrap_or_default();
                    TokenStatus::Valid { expires_in }
                }
            } else {
                TokenStatus::NotConfigured
            }
        } else {
            TokenStatus::NotConfigured
        }
    }

    async fn revoke(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        self.tokens.write().await.remove(&platform);
        Ok(())
    }
}

// ============================================================================
// Mock Credential Store
// ============================================================================

/// Mock credential store for testing
struct GarminMockCredentialStore {
    credentials: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
}

impl GarminMockCredentialStore {
    fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CredentialStore for GarminMockCredentialStore {
    async fn store_tokens(
        &self,
        platform: SyncPlatform,
        tokens: &TokenResponse,
    ) -> Result<(), SyncError> {
        self.credentials.write().await.insert(platform, tokens.clone());
        Ok(())
    }

    async fn get_tokens(
        &self,
        platform: SyncPlatform,
    ) -> Result<Option<TokenResponse>, SyncError> {
        Ok(self.credentials.read().await.get(&platform).cloned())
    }

    async fn delete_tokens(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        self.credentials.write().await.remove(&platform);
        Ok(())
    }

    fn has_credentials(&self, platform: SyncPlatform) -> bool {
        self.credentials
            .try_read()
            .map(|c| c.contains_key(&platform))
            .unwrap_or(false)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test sync service with mock handlers
fn create_garmin_test_service(
    oauth_handler: Arc<GarminMockOAuthHandler>,
    credential_store: Arc<GarminMockCredentialStore>,
) -> SyncServiceHandle {
    let config = SyncConfig::default();
    SyncService::spawn(oauth_handler, credential_store, config)
}

/// Generate valid FIT file data for testing
fn generate_valid_fit_data() -> Vec<u8> {
    // FIT file with 14-byte header
    let mut data = vec![
        14,           // Header size (14 bytes)
        0x10,         // Protocol version
        0x00, 0x00,   // Profile version
        0x00, 0x00, 0x00, 0x00, // Data size (placeholder)
        b'.', b'F', b'I', b'T', // ".FIT" signature
        0x00, 0x00,   // CRC (placeholder)
    ];
    // Add some dummy record data
    data.extend_from_slice(&[0u8; 100]);
    data
}

/// Generate minimal (12-byte header) FIT file data for testing
fn generate_minimal_fit_data() -> Vec<u8> {
    let mut data = vec![
        12,           // Header size (12 bytes)
        0x10,         // Protocol version
        0x00, 0x00,   // Profile version
        0x00, 0x00, 0x00, 0x00, // Data size (placeholder)
        b'.', b'F', b'I', b'T', // ".FIT" signature
    ];
    // Add some dummy record data
    data.extend_from_slice(&[0u8; 50]);
    data
}

/// Create a successful Garmin upload response JSON
fn create_upload_success_response(activity_id: u64) -> String {
    serde_json::json!({
        "detailedImportResult": {
            "uploadUuid": {
                "uuid": "test-upload-uuid"
            },
            "successes": [{
                "internalId": activity_id,
                "externalId": null
            }],
            "failures": []
        }
    })
    .to_string()
}

/// Create a duplicate activity error response
fn create_duplicate_error_response() -> String {
    serde_json::json!({
        "detailedImportResult": {
            "uploadUuid": null,
            "successes": [],
            "failures": [{
                "internalId": null,
                "externalId": null,
                "messages": [{
                    "code": 202,
                    "content": "Duplicate activity detected"
                }]
            }]
        }
    })
    .to_string()
}

/// Create a user profile response
fn create_profile_response(user_id: u64, display_name: &str) -> String {
    serde_json::json!({
        "id": user_id,
        "displayName": display_name,
        "fullName": format!("{} User", display_name),
        "profileImageUrlSmall": format!("https://connect.garmin.com/profile/{}/small.jpg", user_id),
        "profileImageUrlMedium": format!("https://connect.garmin.com/profile/{}/medium.jpg", user_id),
        "profileImageUrlLarge": null
    })
    .to_string()
}

// ============================================================================
// End-to-End Flow Tests
// ============================================================================

/// Test the complete connect flow for Garmin Connect
#[tokio::test]
async fn test_garmin_connect_flow_start_authorization() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());
    let handle = create_garmin_test_service(oauth_handler.clone(), credential_store);

    // Initially not connected
    let status = handle.get_status(SyncPlatform::GarminConnect).await.unwrap();
    assert!(!status.connected, "Should not be connected initially");

    // Start authorization - returns URL to open in browser
    let auth_url = oauth_handler
        .start_authorization(SyncPlatform::GarminConnect)
        .await
        .unwrap();
    assert!(
        auth_url.url.contains("connect.garmin.com"),
        "Auth URL should point to Garmin Connect"
    );
    assert!(!auth_url.state.is_empty(), "Auth state should be set");

    handle.shutdown().await.unwrap();
}

/// Test the OAuth callback handling for Garmin Connect
#[tokio::test]
async fn test_garmin_connect_flow_handle_callback() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());
    let _handle = create_garmin_test_service(oauth_handler.clone(), credential_store.clone());

    // Simulate OAuth callback after user authorization
    let tokens = oauth_handler
        .handle_callback("mock_auth_code", "garmin_test_state")
        .await
        .unwrap();

    assert!(!tokens.access_token.is_empty(), "Access token should be returned");
    assert!(tokens.refresh_token.is_some(), "Refresh token should be returned");
    assert!(tokens.expires_at > Utc::now(), "Token should not be expired");

    // Store credentials
    credential_store
        .store_tokens(SyncPlatform::GarminConnect, &tokens)
        .await
        .unwrap();

    // Verify credentials are stored
    assert!(
        credential_store.has_credentials(SyncPlatform::GarminConnect),
        "Credentials should be stored after callback"
    );

    // Verify oauth handler knows about the tokens
    assert!(
        oauth_handler.is_authorized(SyncPlatform::GarminConnect),
        "Handler should show as authorized after callback"
    );
}

/// Test the complete upload flow for Garmin Connect with mocked HTTP
#[tokio::test]
async fn test_garmin_upload_flow_success() {
    let mock_server = MockServer::start().await;

    // Setup mock for upload endpoint
    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .and(bearer_token("test_access_token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(12345678))
        )
        .mount(&mock_server)
        .await;

    // Create client with mock server URL
    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    // Generate valid FIT data
    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();

    // Upload activity
    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(result.is_ok(), "Upload should succeed: {:?}", result.err());

    let record = result.unwrap();
    assert_eq!(record.status, SyncRecordStatus::Completed);
    assert_eq!(record.platform, SyncPlatform::GarminConnect);
    assert_eq!(record.external_id, Some("12345678".to_string()));
    assert!(record.external_url.is_some());
    assert!(
        record.external_url.as_ref().unwrap().contains("12345678"),
        "URL should contain activity ID"
    );
}

/// Test upload with duplicate activity detection
#[tokio::test]
async fn test_garmin_upload_flow_duplicate() {
    let mock_server = MockServer::start().await;

    // Setup mock for duplicate response
    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_duplicate_error_response())
        )
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();

    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(
        matches!(result, Err(SyncError::DuplicateActivity(SyncPlatform::GarminConnect))),
        "Should detect duplicate: {:?}",
        result
    );
}

/// Test upload with 409 Conflict status (duplicate detection via HTTP status)
#[tokio::test]
async fn test_garmin_upload_flow_409_conflict() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();

    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(
        matches!(result, Err(SyncError::DuplicateActivity(_))),
        "409 should trigger duplicate error: {:?}",
        result
    );
}

/// Test upload with rate limiting (429)
#[tokio::test]
async fn test_garmin_upload_flow_rate_limited() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
        )
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();

    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(
        matches!(result, Err(SyncError::RateLimited)),
        "429 should trigger rate limited error: {:?}",
        result
    );
}

/// Test upload with token expiry (401)
#[tokio::test]
async fn test_garmin_upload_flow_token_expired() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("expired_token".to_string()).await;

    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();

    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(
        matches!(result, Err(SyncError::TokenExpired)),
        "401 should trigger token expired error: {:?}",
        result
    );
}

/// Test FIT file validation - invalid file
#[tokio::test]
async fn test_garmin_upload_flow_invalid_fit_file() {
    let client = GarminClient::new();
    client.set_access_token("test_token".to_string()).await;

    // Create invalid FIT data (missing signature)
    let invalid_fit_data = vec![14, 0x10, 0, 0, 0, 0, 0, 0, b'X', b'X', b'X', b'X', 0, 0];

    let ride_id = Uuid::new_v4();
    let result = client.upload_activity(&ride_id, &invalid_fit_data).await;

    assert!(
        matches!(result, Err(SyncError::InvalidFitFile(_))),
        "Invalid FIT file should be rejected: {:?}",
        result
    );
}

/// Test getting user profile with mocked HTTP
#[tokio::test]
async fn test_garmin_get_profile_flow() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/userprofile-service/socialProfile"))
        .and(bearer_token("test_access_token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_profile_response(987654, "TestUser"))
        )
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    let profile = client.get_user_profile().await.unwrap();

    assert_eq!(profile.user_id, 987654);
    assert_eq!(profile.display_name, "TestUser");
    assert_eq!(profile.full_name, Some("TestUser User".to_string()));
    assert!(profile.profile_image_url.is_some());
    assert_eq!(profile.readable_name(), "TestUser User");
}

/// Test disconnect flow with deauthorization
#[tokio::test]
async fn test_garmin_disconnect_flow() {
    let mock_server = MockServer::start().await;

    // Revoke endpoint returns success
    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    assert!(client.is_configured(), "Client should be configured before disconnect");

    // Deauthorize
    let result = client.deauthorize().await;
    assert!(result.is_ok(), "Deauthorize should succeed: {:?}", result.err());

    // Token should be cleared
    assert!(!client.is_configured(), "Client should not be configured after disconnect");
}

/// Test disconnect flow when revoke endpoint fails
#[tokio::test]
async fn test_garmin_disconnect_flow_revoke_fails() {
    let mock_server = MockServer::start().await;

    // Revoke endpoint returns error
    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    // Deauthorize should still succeed (local token cleared)
    let result = client.deauthorize().await;
    assert!(result.is_ok(), "Deauthorize should succeed even if API fails");

    // Token should still be cleared
    assert!(!client.is_configured(), "Token should be cleared even on API error");
}

/// Test disconnect flow when revoke endpoint is not found (404)
#[tokio::test]
async fn test_garmin_disconnect_flow_endpoint_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_access_token".to_string()).await;

    let result = client.deauthorize().await;
    assert!(result.is_ok(), "Deauthorize should succeed on 404");
    assert!(!client.is_configured(), "Token should be cleared on 404");
}

// ============================================================================
// Complete E2E Scenario Tests
// ============================================================================

/// Test complete connect -> upload -> disconnect flow
#[tokio::test]
async fn test_garmin_complete_e2e_flow() {
    let mock_server = MockServer::start().await;

    // Setup mocks for the complete flow
    Mock::given(method("GET"))
        .and(path("/userprofile-service/socialProfile"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_profile_response(111222, "E2EUser"))
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(99887766))
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );

    // Step 1: Connect (simulate OAuth callback)
    client.set_access_token("e2e_test_token".to_string()).await;
    assert!(client.is_configured(), "Client should be configured after connect");

    // Step 2: Fetch profile to verify connection
    let profile = client.get_user_profile().await.unwrap();
    assert_eq!(profile.display_name, "E2EUser");

    // Step 3: Upload an activity
    let fit_data = generate_valid_fit_data();
    let ride_id = Uuid::new_v4();
    let upload_result = client.upload_activity(&ride_id, &fit_data).await.unwrap();
    assert_eq!(upload_result.status, SyncRecordStatus::Completed);
    assert_eq!(upload_result.external_id, Some("99887766".to_string()));

    // Step 4: Disconnect
    client.deauthorize().await.unwrap();
    assert!(!client.is_configured(), "Client should be disconnected");

    // Step 5: Verify operations fail after disconnect
    let profile_result = client.get_user_profile().await;
    assert!(
        matches!(profile_result, Err(SyncError::NotConfigured(_))),
        "Profile fetch should fail when not configured"
    );
}

/// Test multiple uploads in sequence
#[tokio::test]
async fn test_garmin_multiple_uploads() {
    let mock_server = MockServer::start().await;

    // Mock returns different activity IDs for each upload
    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(11111111))
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(22222222))
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(33333333))
        )
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_token".to_string()).await;

    let fit_data = generate_valid_fit_data();

    // Upload 3 activities
    let mut external_ids = Vec::new();
    for _ in 0..3 {
        let ride_id = Uuid::new_v4();
        let result = client.upload_activity(&ride_id, &fit_data).await.unwrap();
        assert_eq!(result.status, SyncRecordStatus::Completed);
        if let Some(id) = result.external_id {
            external_ids.push(id);
        }
    }

    assert_eq!(external_ids.len(), 3, "All 3 uploads should have external IDs");
}

/// Test upload with minimal (12-byte header) FIT file
#[tokio::test]
async fn test_garmin_upload_minimal_fit_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/upload-service/upload/.fit"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(create_upload_success_response(44444444))
        )
        .mount(&mock_server)
        .await;

    let client = GarminClient::with_base_url(
        mock_server.uri(),
        format!("{}/oauth", mock_server.uri()),
    );
    client.set_access_token("test_token".to_string()).await;

    // Use minimal 12-byte header FIT data
    let fit_data = generate_minimal_fit_data();
    let ride_id = Uuid::new_v4();

    let result = client.upload_activity(&ride_id, &fit_data).await;
    assert!(result.is_ok(), "Minimal FIT header should be accepted: {:?}", result.err());
}

// ============================================================================
// SyncService Integration Tests
// ============================================================================

/// Test Garmin platform configuration through SyncService
#[tokio::test]
async fn test_garmin_service_platform_config() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());
    let handle = create_garmin_test_service(oauth_handler, credential_store);

    // Configure Garmin Connect
    let config = PlatformConfig {
        enabled: true,
        auto_sync: true,
    };
    handle
        .update_config(SyncPlatform::GarminConnect, config)
        .await
        .unwrap();

    // Verify configuration
    let status = handle.get_status(SyncPlatform::GarminConnect).await.unwrap();
    assert!(status.config.enabled, "Garmin should be enabled");
    assert!(status.config.auto_sync, "Auto-sync should be enabled");

    handle.shutdown().await.unwrap();
}

/// Test Garmin upload requires authorization
#[tokio::test]
async fn test_garmin_service_upload_requires_auth() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());
    let handle = create_garmin_test_service(oauth_handler, credential_store);

    let ride_id = Uuid::new_v4();
    let fit_data = generate_valid_fit_data();

    // Attempt upload without authorization
    let result = handle
        .queue_upload(ride_id, SyncPlatform::GarminConnect, fit_data, Some("Test Ride".to_string()))
        .await;

    assert!(
        matches!(result, Err(SyncError::AuthorizationRequired)),
        "Upload should require authorization: {:?}",
        result
    );

    handle.shutdown().await.unwrap();
}

/// Test event subscription for Garmin uploads
#[tokio::test]
async fn test_garmin_event_subscription() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());
    let handle = create_garmin_test_service(oauth_handler, credential_store);

    // Subscribe to events
    let event_rx = handle.subscribe_events().await;
    assert!(event_rx.is_some(), "First subscription should succeed");

    // Second subscription should fail
    let event_rx2 = handle.subscribe_events().await;
    assert!(event_rx2.is_none(), "Second subscription should fail");

    handle.shutdown().await.unwrap();
}

// ============================================================================
// Error Category and Retry Logic Tests
// ============================================================================

/// Test error categorization for Garmin-specific errors
#[tokio::test]
async fn test_garmin_error_categories() {
    // Rate limited
    let rate_limited = SyncError::RateLimited;
    assert_eq!(rate_limited.category(), ErrorCategory::RateLimited);
    assert!(rate_limited.is_retryable());
    assert!(rate_limited.is_rate_limited());
    assert!(!rate_limited.is_duplicate());

    // Duplicate activity
    let duplicate = SyncError::DuplicateActivity(SyncPlatform::GarminConnect);
    assert_eq!(duplicate.category(), ErrorCategory::Permanent);
    assert!(!duplicate.is_retryable());
    assert!(!duplicate.is_rate_limited());
    assert!(duplicate.is_duplicate());

    // Token expired
    let expired = SyncError::TokenExpired;
    assert_eq!(expired.category(), ErrorCategory::Authentication);
    assert!(!expired.is_retryable());
    assert!(expired.requires_auth_refresh());

    // Invalid FIT file
    let invalid = SyncError::InvalidFitFile("test".to_string());
    assert_eq!(invalid.category(), ErrorCategory::Client);
    assert!(!invalid.is_retryable());

    // Network error
    let network = SyncError::NetworkError("test".to_string());
    assert_eq!(network.category(), ErrorCategory::Transient);
    assert!(network.is_retryable());
}

/// Test retry delay for different error categories
#[tokio::test]
async fn test_garmin_error_retry_delays() {
    let rate_limited = SyncError::RateLimited;
    assert!(rate_limited.retry_delay_secs() >= 60);

    let network = SyncError::NetworkError("test".to_string());
    assert!(network.retry_delay_secs() > 0);
    assert!(network.retry_delay_secs() < rate_limited.retry_delay_secs());

    let duplicate = SyncError::DuplicateActivity(SyncPlatform::GarminConnect);
    assert_eq!(duplicate.retry_delay_secs(), 0); // Non-retryable
}

/// Test max retry attempts for different error categories
#[tokio::test]
async fn test_garmin_error_max_retries() {
    assert!(ErrorCategory::Transient.max_retry_attempts() > 0);
    assert!(ErrorCategory::RateLimited.max_retry_attempts() > 0);
    assert!(ErrorCategory::Server.max_retry_attempts() > 0);
    assert_eq!(ErrorCategory::Client.max_retry_attempts(), 0);
    assert_eq!(ErrorCategory::Authentication.max_retry_attempts(), 0);
    assert_eq!(ErrorCategory::Permanent.max_retry_attempts(), 0);
}

// ============================================================================
// Token Refresh Tests
// ============================================================================

/// Test token refresh flow
#[tokio::test]
async fn test_garmin_token_refresh_flow() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());
    let credential_store = Arc::new(GarminMockCredentialStore::new());

    // Set initial tokens
    let tokens = TokenResponse {
        access_token: "initial_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    oauth_handler.set_garmin_tokens(tokens.clone()).await;
    credential_store
        .store_tokens(SyncPlatform::GarminConnect, &tokens)
        .await
        .unwrap();

    // Verify initial state
    assert!(oauth_handler.is_authorized(SyncPlatform::GarminConnect));
    assert_eq!(oauth_handler.get_refresh_count(), 0);

    // Trigger token refresh
    let new_tokens = oauth_handler
        .refresh_token(SyncPlatform::GarminConnect)
        .await
        .unwrap();

    assert!(
        new_tokens.access_token.contains("refreshed"),
        "Token should be refreshed"
    );
    assert_eq!(oauth_handler.get_refresh_count(), 1);
}

/// Test token refresh failure requires reauth
#[tokio::test]
async fn test_garmin_token_refresh_requires_reauth() {
    let oauth_handler = Arc::new(GarminMockOAuthHandler::new());

    let tokens = TokenResponse {
        access_token: "initial_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    oauth_handler.set_garmin_tokens(tokens).await;
    oauth_handler.set_reauth_required(true);

    let result = oauth_handler
        .refresh_token(SyncPlatform::GarminConnect)
        .await;

    assert!(
        matches!(result, Err(SyncError::AuthorizationRequired)),
        "Should require reauthorization: {:?}",
        result
    );
}

// ============================================================================
// FIT File Validation Tests
// ============================================================================

/// Test FIT file validation accepts valid files
#[tokio::test]
async fn test_garmin_fit_validation_valid() {
    let valid_fit = generate_valid_fit_data();
    let result = GarminClient::validate_fit_file(&valid_fit);
    assert!(result.is_ok(), "Valid FIT file should pass validation");
}

/// Test FIT file validation rejects too small files
#[tokio::test]
async fn test_garmin_fit_validation_too_small() {
    let tiny_data = vec![1, 2, 3, 4, 5];
    let result = GarminClient::validate_fit_file(&tiny_data);
    assert!(
        matches!(result, Err(SyncError::InvalidFitFile(_))),
        "Too small file should fail: {:?}",
        result
    );
}

/// Test FIT file validation rejects missing signature
#[tokio::test]
async fn test_garmin_fit_validation_missing_signature() {
    let bad_signature = vec![14, 0x10, 0, 0, 0, 0, 0, 0, b'N', b'O', b'T', b'F', 0, 0, 0, 0];
    let result = GarminClient::validate_fit_file(&bad_signature);
    assert!(
        matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("signature")),
        "Missing signature should fail: {:?}",
        result
    );
}

/// Test FIT file validation rejects invalid header size
#[tokio::test]
async fn test_garmin_fit_validation_invalid_header() {
    // Header size 13 is invalid (must be 12 or 14)
    let bad_header = vec![13, 0x10, 0, 0, 0, 0, 0, 0, b'.', b'F', b'I', b'T', 0, 0, 0, 0];
    let result = GarminClient::validate_fit_file(&bad_header);
    assert!(
        matches!(result, Err(SyncError::InvalidFitFile(msg)) if msg.contains("header")),
        "Invalid header size should fail: {:?}",
        result
    );
}

// ============================================================================
// GarminUserProfile Tests
// ============================================================================

/// Test user profile readable name with full name
#[tokio::test]
async fn test_garmin_profile_readable_name_with_full_name() {
    let profile = GarminUserProfile {
        user_id: 12345,
        display_name: "jdoe".to_string(),
        full_name: Some("John Doe".to_string()),
        profile_image_url: None,
    };

    assert_eq!(profile.readable_name(), "John Doe");
}

/// Test user profile readable name without full name
#[tokio::test]
async fn test_garmin_profile_readable_name_without_full_name() {
    let profile = GarminUserProfile {
        user_id: 12345,
        display_name: "jdoe".to_string(),
        full_name: None,
        profile_image_url: None,
    };

    assert_eq!(profile.readable_name(), "jdoe");
}

// ============================================================================
// Platform Display Name Tests
// ============================================================================

/// Test Garmin Connect platform display name
#[tokio::test]
async fn test_garmin_platform_display_name() {
    assert_eq!(SyncPlatform::GarminConnect.display_name(), "Garmin Connect");
}

/// Test Garmin Connect uses OAuth
#[tokio::test]
async fn test_garmin_uses_oauth() {
    assert!(SyncPlatform::GarminConnect.uses_oauth());
}

/// Test Garmin Connect auth URL base
#[tokio::test]
async fn test_garmin_auth_url_base() {
    let auth_url = SyncPlatform::GarminConnect.auth_url_base();
    assert!(auth_url.contains("garmin.com"));
    assert!(auth_url.contains("oauth"));
}
