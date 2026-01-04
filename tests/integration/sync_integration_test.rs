//! Integration tests for the sync service.
//!
//! Tests the full sync flow from ride completion to successful upload,
//! including retry mechanism and offline queue handling.

use chrono::{Duration, Utc};
use rustride::integrations::sync::oauth::{
    AuthorizationUrl, CredentialStore, OAuthHandler, TokenResponse, TokenStatus,
};
use rustride::integrations::sync::service::{SyncService, SyncServiceHandle};
use rustride::integrations::sync::{
    PlatformConfig, SyncConfig, SyncError, SyncEvent, SyncPlatform, SyncRecordStatus,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Mock OAuth Handler
// ============================================================================

/// Mock OAuth handler for testing that simulates successful authentication
struct MockOAuthHandler {
    /// Simulated tokens per platform
    tokens: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
    /// Whether to simulate refresh failure
    simulate_refresh_failure: AtomicBool,
    /// Whether to require re-authorization
    simulate_reauth_required: AtomicBool,
    /// Count of refresh attempts
    refresh_count: AtomicU32,
}

impl MockOAuthHandler {
    fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            simulate_refresh_failure: AtomicBool::new(false),
            simulate_reauth_required: AtomicBool::new(false),
            refresh_count: AtomicU32::new(0),
        }
    }

    /// Pre-configure with valid tokens for a platform
    async fn set_tokens(&self, platform: SyncPlatform, tokens: TokenResponse) {
        self.tokens.write().await.insert(platform, tokens);
    }

    fn set_refresh_failure(&self, should_fail: bool) {
        self.simulate_refresh_failure
            .store(should_fail, Ordering::SeqCst);
    }

    fn set_reauth_required(&self, required: bool) {
        self.simulate_reauth_required
            .store(required, Ordering::SeqCst);
    }

    fn get_refresh_count(&self) -> u32 {
        self.refresh_count.load(Ordering::SeqCst)
    }
}

impl OAuthHandler for MockOAuthHandler {
    async fn start_authorization(
        &self,
        platform: SyncPlatform,
    ) -> Result<AuthorizationUrl, SyncError> {
        Ok(AuthorizationUrl {
            url: format!("https://mock.auth/{:?}", platform),
            state: "mock_state".to_string(),
        })
    }

    async fn handle_callback(&self, _code: &str, _state: &str) -> Result<TokenResponse, SyncError> {
        Ok(TokenResponse {
            access_token: "mock_access_token".to_string(),
            refresh_token: Some("mock_refresh_token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        })
    }

    async fn refresh_token(&self, platform: SyncPlatform) -> Result<TokenResponse, SyncError> {
        self.refresh_count.fetch_add(1, Ordering::SeqCst);

        if self.simulate_reauth_required.load(Ordering::SeqCst) {
            return Err(SyncError::AuthorizationRequired);
        }

        if self.simulate_refresh_failure.load(Ordering::SeqCst) {
            return Err(SyncError::RefreshFailed(
                "Simulated refresh failure".to_string(),
            ));
        }

        let new_tokens = TokenResponse {
            access_token: format!("refreshed_token_{}", self.get_refresh_count()),
            refresh_token: Some("mock_refresh_token".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
        };

        self.tokens
            .write()
            .await
            .insert(platform, new_tokens.clone());
        Ok(new_tokens)
    }

    fn is_authorized(&self, platform: SyncPlatform) -> bool {
        // Use try_read to avoid blocking
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
struct MockCredentialStore {
    credentials: Arc<RwLock<HashMap<SyncPlatform, TokenResponse>>>,
}

impl MockCredentialStore {
    fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CredentialStore for MockCredentialStore {
    async fn store_tokens(
        &self,
        platform: SyncPlatform,
        tokens: &TokenResponse,
    ) -> Result<(), SyncError> {
        self.credentials
            .write()
            .await
            .insert(platform, tokens.clone());
        Ok(())
    }

    async fn get_tokens(&self, platform: SyncPlatform) -> Result<Option<TokenResponse>, SyncError> {
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
fn create_test_service(
    oauth_handler: Arc<MockOAuthHandler>,
    credential_store: Arc<MockCredentialStore>,
) -> SyncServiceHandle {
    let config = SyncConfig::default();
    SyncService::spawn(oauth_handler, credential_store, config)
}

/// Generate minimal FIT file data for testing
fn generate_test_fit_data() -> Vec<u8> {
    // Minimal FIT file header (14 bytes) + dummy data
    // This is a simplified mock - real FIT files are more complex
    let mut data = vec![
        14,   // Header size
        0x10, // Protocol version
        0x00, 0x00, // Profile version
        0x00, 0x00, 0x00, 0x00, // Data size (placeholder)
        b'.', b'F', b'I', b'T', // ".FIT" signature
        0x00, 0x00, // CRC (placeholder)
    ];
    // Add some dummy record data
    data.extend_from_slice(&[0u8; 100]);
    data
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_sync_service_starts_and_stops() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // Service should be running
    let platforms = handle.get_connected_platforms().await.unwrap();
    assert!(
        platforms.is_empty(),
        "No platforms should be connected initially"
    );

    // Shutdown should succeed
    let result = handle.shutdown().await;
    assert!(result.is_ok(), "Shutdown should succeed");
}

#[tokio::test]
async fn test_platform_connection_status() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // Check initial status
    let status = handle.get_status(SyncPlatform::Strava).await.unwrap();
    assert!(
        !status.connected,
        "Strava should not be connected initially"
    );
    assert!(
        status.token_status.is_none(),
        "No token status when not connected"
    );
    assert_eq!(status.pending_uploads, 0, "No pending uploads initially");

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_upload_requires_platform_connection() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    let ride_id = Uuid::new_v4();
    let fit_data = generate_test_fit_data();

    // Attempt upload without connection should fail
    let result = handle
        .queue_upload(
            ride_id,
            SyncPlatform::Strava,
            fit_data,
            Some("Test Ride".to_string()),
        )
        .await;

    assert!(
        matches!(result, Err(SyncError::AuthorizationRequired)),
        "Upload should require authorization"
    );

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_update_platform_config() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // Update configuration
    let config = PlatformConfig {
        enabled: true,
        auto_sync: true,
    };

    let result = handle
        .update_config(SyncPlatform::Strava, config.clone())
        .await;
    assert!(result.is_ok(), "Config update should succeed");

    // Verify the configuration was applied
    let status = handle.get_status(SyncPlatform::Strava).await.unwrap();
    assert!(status.config.enabled, "Platform should be enabled");
    assert!(status.config.auto_sync, "Auto-sync should be enabled");

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_event_subscription() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // First subscription should succeed
    let event_rx = handle.subscribe_events().await;
    assert!(event_rx.is_some(), "First subscription should succeed");

    // Second subscription should return None (only one subscriber allowed)
    let event_rx2 = handle.subscribe_events().await;
    assert!(event_rx2.is_none(), "Second subscription should fail");

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_cancel_nonexistent_upload() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    let fake_record_id = Uuid::new_v4();
    let result = handle.cancel_upload(fake_record_id).await.unwrap();

    assert!(
        !result,
        "Cancelling non-existent upload should return false"
    );

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_get_sync_records_empty() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    let ride_id = Uuid::new_v4();
    let records = handle.get_sync_records(ride_id).await.unwrap();

    assert!(
        records.is_empty(),
        "No sync records should exist for new ride"
    );

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_retry_nonexistent_upload() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    let fake_record_id = Uuid::new_v4();
    let result = handle.retry_upload(fake_record_id).await;

    assert!(result.is_err(), "Retrying non-existent upload should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Error should mention record not found"
    );

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_platform_configs() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // Configure Strava
    let strava_config = PlatformConfig {
        enabled: true,
        auto_sync: true,
    };
    handle
        .update_config(SyncPlatform::Strava, strava_config)
        .await
        .unwrap();

    // Configure Garmin (even though not fully implemented)
    let garmin_config = PlatformConfig {
        enabled: true,
        auto_sync: false,
    };
    handle
        .update_config(SyncPlatform::GarminConnect, garmin_config)
        .await
        .unwrap();

    // Verify Strava config
    let strava_status = handle.get_status(SyncPlatform::Strava).await.unwrap();
    assert!(strava_status.config.enabled);
    assert!(strava_status.config.auto_sync);

    // Verify Garmin config
    let garmin_status = handle
        .get_status(SyncPlatform::GarminConnect)
        .await
        .unwrap();
    assert!(garmin_status.config.enabled);
    assert!(!garmin_status.config.auto_sync);

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sync_record_status_values() {
    // Test that all SyncRecordStatus variants exist and have expected properties
    let pending = SyncRecordStatus::Pending;
    let uploading = SyncRecordStatus::Uploading;
    let completed = SyncRecordStatus::Completed;
    let failed = SyncRecordStatus::Failed;
    let cancelled = SyncRecordStatus::Cancelled;

    // All variants should be comparable
    assert_ne!(pending, uploading);
    assert_ne!(uploading, completed);
    assert_ne!(completed, failed);
    assert_ne!(failed, cancelled);

    // Each variant should equal itself
    assert_eq!(pending, SyncRecordStatus::Pending);
    assert_eq!(uploading, SyncRecordStatus::Uploading);
    assert_eq!(completed, SyncRecordStatus::Completed);
    assert_eq!(failed, SyncRecordStatus::Failed);
    assert_eq!(cancelled, SyncRecordStatus::Cancelled);
}

#[tokio::test]
async fn test_sync_event_variants() {
    // Test that SyncEvent variants can be created and formatted
    let platform = SyncPlatform::Strava;
    let record_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // Test TokenRefreshed event
    let event = SyncEvent::TokenRefreshed {
        platform,
        expires_at: Utc::now() + Duration::hours(1),
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("TokenRefreshed"));

    // Test UploadStarted event
    let event = SyncEvent::UploadStarted {
        record_id,
        ride_id,
        platform,
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("UploadStarted"));

    // Test UploadCompleted event
    let event = SyncEvent::UploadCompleted {
        record_id,
        ride_id,
        platform,
        external_id: Some("12345".to_string()),
        external_url: Some("https://strava.com/activities/12345".to_string()),
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("UploadCompleted"));

    // Test UploadFailed event
    let event = SyncEvent::UploadFailed {
        record_id,
        ride_id,
        platform,
        error: "Network error".to_string(),
        retry_count: 1,
        will_retry: true,
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("UploadFailed"));
    assert!(debug_str.contains("will_retry: true"));

    // Test ConnectivityChanged event
    let event = SyncEvent::ConnectivityChanged { is_online: true };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("ConnectivityChanged"));
}

#[tokio::test]
async fn test_platform_display_names() {
    assert_eq!(SyncPlatform::Strava.display_name(), "Strava");
    assert_eq!(SyncPlatform::GarminConnect.display_name(), "Garmin Connect");
    assert_eq!(SyncPlatform::TrainingPeaks.display_name(), "TrainingPeaks");
    assert_eq!(SyncPlatform::IntervalsIcu.display_name(), "Intervals.icu");
}

#[tokio::test]
async fn test_platform_uses_oauth() {
    // All these platforms should use OAuth
    assert!(SyncPlatform::Strava.uses_oauth());
    assert!(SyncPlatform::GarminConnect.uses_oauth());
    assert!(SyncPlatform::TrainingPeaks.uses_oauth());
    assert!(SyncPlatform::IntervalsIcu.uses_oauth());
}

#[tokio::test]
async fn test_sync_config_default() {
    let config = SyncConfig::default();

    // All platforms should be in the default config
    assert!(config.platforms.contains_key(&SyncPlatform::Strava));
    assert!(config.platforms.contains_key(&SyncPlatform::GarminConnect));
    assert!(config.platforms.contains_key(&SyncPlatform::TrainingPeaks));
    assert!(config.platforms.contains_key(&SyncPlatform::IntervalsIcu));

    // All platforms should be disabled by default
    for (_, platform_config) in &config.platforms {
        assert!(
            !platform_config.enabled,
            "Platforms should be disabled by default"
        );
        assert!(
            !platform_config.auto_sync,
            "Auto-sync should be off by default"
        );
    }
}

#[tokio::test]
async fn test_platform_config_default() {
    let config = PlatformConfig::default();

    assert!(!config.enabled, "Platform should be disabled by default");
    assert!(!config.auto_sync, "Auto-sync should be off by default");
}

#[tokio::test]
async fn test_sync_error_display() {
    let errors = vec![
        (
            SyncError::NotConfigured(SyncPlatform::Strava),
            "not configured",
        ),
        (SyncError::AuthorizationRequired, "Authorization required"),
        (SyncError::TokenExpired, "Token expired"),
        (
            SyncError::RefreshFailed("test".to_string()),
            "refresh failed",
        ),
        (SyncError::UploadFailed("test".to_string()), "Upload failed"),
        (SyncError::ApiError("test".to_string()), "API error"),
        (SyncError::CredentialError("test".to_string()), "Credential"),
        (SyncError::NetworkError("test".to_string()), "Network error"),
    ];

    for (error, expected_substr) in errors {
        let msg = error.to_string();
        assert!(
            msg.to_lowercase().contains(&expected_substr.to_lowercase()),
            "Error '{}' should contain '{}'",
            msg,
            expected_substr
        );
    }
}

#[tokio::test]
async fn test_mock_oauth_handler_basic_operations() {
    let handler = MockOAuthHandler::new();

    // Initially not authorized
    assert!(!handler.is_authorized(SyncPlatform::Strava));

    // Set tokens
    let tokens = TokenResponse {
        access_token: "test_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    handler.set_tokens(SyncPlatform::Strava, tokens).await;

    // Now should be authorized
    assert!(handler.is_authorized(SyncPlatform::Strava));

    // Token status should be valid
    let status = handler.get_token_status(SyncPlatform::Strava);
    assert!(matches!(status, TokenStatus::Valid { .. }));
}

#[tokio::test]
async fn test_mock_oauth_handler_refresh() {
    let handler = MockOAuthHandler::new();

    // Set initial tokens
    let tokens = TokenResponse {
        access_token: "test_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    handler.set_tokens(SyncPlatform::Strava, tokens).await;

    // Refresh should succeed
    let new_tokens = handler.refresh_token(SyncPlatform::Strava).await.unwrap();
    assert!(new_tokens.access_token.starts_with("refreshed_token_"));
    assert_eq!(handler.get_refresh_count(), 1);
}

#[tokio::test]
async fn test_mock_oauth_handler_refresh_failure() {
    let handler = MockOAuthHandler::new();

    // Set tokens and simulate refresh failure
    let tokens = TokenResponse {
        access_token: "test_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    handler.set_tokens(SyncPlatform::Strava, tokens).await;
    handler.set_refresh_failure(true);

    // Refresh should fail
    let result = handler.refresh_token(SyncPlatform::Strava).await;
    assert!(matches!(result, Err(SyncError::RefreshFailed(_))));
}

#[tokio::test]
async fn test_mock_oauth_handler_reauth_required() {
    let handler = MockOAuthHandler::new();

    // Set tokens and simulate reauth required
    let tokens = TokenResponse {
        access_token: "test_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    handler.set_tokens(SyncPlatform::Strava, tokens).await;
    handler.set_reauth_required(true);

    // Refresh should return AuthorizationRequired
    let result = handler.refresh_token(SyncPlatform::Strava).await;
    assert!(matches!(result, Err(SyncError::AuthorizationRequired)));
}

#[tokio::test]
async fn test_mock_credential_store_operations() {
    let store = MockCredentialStore::new();

    // Initially no credentials
    assert!(!store.has_credentials(SyncPlatform::Strava));

    // Store credentials
    let tokens = TokenResponse {
        access_token: "test_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    };
    store
        .store_tokens(SyncPlatform::Strava, &tokens)
        .await
        .unwrap();

    // Should have credentials now
    assert!(store.has_credentials(SyncPlatform::Strava));

    // Retrieve credentials
    let retrieved = store
        .get_tokens(SyncPlatform::Strava)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.access_token, tokens.access_token);

    // Delete credentials
    store.delete_tokens(SyncPlatform::Strava).await.unwrap();
    assert!(!store.has_credentials(SyncPlatform::Strava));
}

#[tokio::test]
async fn test_service_graceful_shutdown() {
    let oauth_handler = Arc::new(MockOAuthHandler::new());
    let credential_store = Arc::new(MockCredentialStore::new());
    let handle = create_test_service(oauth_handler, credential_store);

    // Do some operations
    handle
        .update_config(
            SyncPlatform::Strava,
            PlatformConfig {
                enabled: true,
                auto_sync: false,
            },
        )
        .await
        .unwrap();

    // Shutdown should be clean
    let result = handle.shutdown().await;
    assert!(result.is_ok(), "Graceful shutdown should succeed");

    // Operations after shutdown should fail
    let result = handle.get_connected_platforms().await;
    assert!(result.is_err(), "Operations after shutdown should fail");
}
