//! Sync Service for managing platform connections and uploads.
//!
//! T104: SyncService implementation
//!
//! Implements an async actor pattern for managing:
//! - Platform connections (Strava, etc.)
//! - Token refresh scheduling
//! - Upload queue management

use super::oauth::{CredentialStore, KeyringCredentialStore, OAuthHandler, TokenResponse, TokenStatus};
use super::strava::StravaClient;
use super::{PlatformConfig, SyncConfig, SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

/// Message types for the SyncService actor
#[derive(Debug)]
pub enum SyncMessage {
    /// Connect to a platform via OAuth
    Connect {
        platform: SyncPlatform,
        response: oneshot::Sender<Result<(), SyncError>>,
    },
    /// Disconnect from a platform
    Disconnect {
        platform: SyncPlatform,
        response: oneshot::Sender<Result<(), SyncError>>,
    },
    /// Get connection status for a platform
    GetStatus {
        platform: SyncPlatform,
        response: oneshot::Sender<PlatformStatus>,
    },
    /// Get all connected platforms
    GetConnectedPlatforms {
        response: oneshot::Sender<Vec<SyncPlatform>>,
    },
    /// Queue an upload
    QueueUpload {
        ride_id: Uuid,
        platform: SyncPlatform,
        fit_data: Vec<u8>,
        activity_name: Option<String>,
        response: oneshot::Sender<Result<SyncRecord, SyncError>>,
    },
    /// Retry a failed upload
    RetryUpload {
        record_id: Uuid,
        response: oneshot::Sender<Result<SyncRecord, SyncError>>,
    },
    /// Cancel a pending upload
    CancelUpload {
        record_id: Uuid,
        response: oneshot::Sender<bool>,
    },
    /// Get sync records for a ride
    GetSyncRecords {
        ride_id: Uuid,
        response: oneshot::Sender<Vec<SyncRecord>>,
    },
    /// Update platform configuration
    UpdateConfig {
        platform: SyncPlatform,
        config: PlatformConfig,
        response: oneshot::Sender<Result<(), SyncError>>,
    },
    /// Shutdown the service
    Shutdown,
}

/// Status of a platform connection
#[derive(Debug, Clone)]
pub struct PlatformStatus {
    /// Whether the platform is connected
    pub connected: bool,
    /// Token status if connected
    pub token_status: Option<TokenStatus>,
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Platform configuration
    pub config: PlatformConfig,
    /// Number of pending uploads
    pub pending_uploads: usize,
}

/// Upload queue entry
#[derive(Debug, Clone)]
pub struct UploadQueueEntry {
    /// Sync record
    pub record: SyncRecord,
    /// FIT file data
    pub fit_data: Vec<u8>,
    /// Activity name
    pub activity_name: Option<String>,
    /// Next retry time (if retrying)
    pub next_retry: Option<DateTime<Utc>>,
}

/// Handle to the SyncService for sending messages
#[derive(Clone)]
pub struct SyncServiceHandle {
    sender: mpsc::Sender<SyncMessage>,
}

impl SyncServiceHandle {
    /// Connect to a platform via OAuth
    ///
    /// This initiates the OAuth flow for the given platform.
    pub async fn connect(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::Connect {
                platform,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))?
    }

    /// Disconnect from a platform
    ///
    /// This revokes the OAuth tokens and clears stored credentials.
    pub async fn disconnect(&self, platform: SyncPlatform) -> Result<(), SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::Disconnect {
                platform,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))?
    }

    /// Get the connection status for a platform
    pub async fn get_status(&self, platform: SyncPlatform) -> Result<PlatformStatus, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::GetStatus {
                platform,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))
    }

    /// Get all connected platforms
    pub async fn get_connected_platforms(&self) -> Result<Vec<SyncPlatform>, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::GetConnectedPlatforms { response: tx })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))
    }

    /// Queue an activity upload to a platform
    ///
    /// Returns a SyncRecord for tracking the upload status.
    pub async fn queue_upload(
        &self,
        ride_id: Uuid,
        platform: SyncPlatform,
        fit_data: Vec<u8>,
        activity_name: Option<String>,
    ) -> Result<SyncRecord, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::QueueUpload {
                ride_id,
                platform,
                fit_data,
                activity_name,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))?
    }

    /// Retry a failed upload
    pub async fn retry_upload(&self, record_id: Uuid) -> Result<SyncRecord, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::RetryUpload {
                record_id,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))?
    }

    /// Cancel a pending upload
    pub async fn cancel_upload(&self, record_id: Uuid) -> Result<bool, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::CancelUpload {
                record_id,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))
    }

    /// Get sync records for a ride
    pub async fn get_sync_records(&self, ride_id: Uuid) -> Result<Vec<SyncRecord>, SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::GetSyncRecords {
                ride_id,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))
    }

    /// Update platform configuration
    pub async fn update_config(
        &self,
        platform: SyncPlatform,
        config: PlatformConfig,
    ) -> Result<(), SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::UpdateConfig {
                platform,
                config,
                response: tx,
            })
            .await
            .map_err(|_| SyncError::ApiError("Service unavailable".to_string()))?;
        rx.await
            .map_err(|_| SyncError::ApiError("Service response failed".to_string()))?
    }

    /// Shutdown the service gracefully
    pub async fn shutdown(&self) -> Result<(), SyncError> {
        self.sender
            .send(SyncMessage::Shutdown)
            .await
            .map_err(|_| SyncError::ApiError("Service already stopped".to_string()))
    }
}

/// Sync Service that manages platform connections and uploads.
///
/// This service runs as an async actor, receiving messages through a channel.
/// It manages OAuth connections for platforms like Strava, handles token refresh,
/// and processes an upload queue for syncing rides.
pub struct SyncService<O: OAuthHandler + 'static, C: CredentialStore + 'static> {
    /// Message receiver
    receiver: mpsc::Receiver<SyncMessage>,
    /// OAuth handler for authentication
    oauth_handler: Arc<O>,
    /// Credential store for secure token storage
    credential_store: Arc<C>,
    /// Platform clients
    clients: HashMap<SyncPlatform, PlatformClient>,
    /// Platform configurations
    configs: Arc<RwLock<SyncConfig>>,
    /// Connected platforms with tokens
    connected: HashMap<SyncPlatform, TokenResponse>,
    /// Upload queue
    upload_queue: Vec<UploadQueueEntry>,
    /// Sync records (in-memory, will be persisted in future subtask)
    sync_records: HashMap<Uuid, SyncRecord>,
    /// Last sync times per platform
    last_sync: HashMap<SyncPlatform, DateTime<Utc>>,
}

/// Client for a specific platform
enum PlatformClient {
    Strava(StravaClient),
}

impl<O: OAuthHandler + Send + Sync + 'static, C: CredentialStore + Send + Sync + 'static>
    SyncService<O, C>
{
    /// Create a new SyncService and return a handle for sending messages.
    ///
    /// The service will start running in the background immediately.
    pub fn spawn(
        oauth_handler: Arc<O>,
        credential_store: Arc<C>,
        config: SyncConfig,
    ) -> SyncServiceHandle {
        let (sender, receiver) = mpsc::channel(64);

        // Initialize platform clients
        let mut clients = HashMap::new();
        clients.insert(SyncPlatform::Strava, PlatformClient::Strava(StravaClient::new()));

        let service = Self {
            receiver,
            oauth_handler,
            credential_store,
            clients,
            configs: Arc::new(RwLock::new(config)),
            connected: HashMap::new(),
            upload_queue: Vec::new(),
            sync_records: HashMap::new(),
            last_sync: HashMap::new(),
        };

        // Spawn the service actor
        tokio::spawn(service.run());

        SyncServiceHandle { sender }
    }

    /// Run the service event loop
    async fn run(mut self) {
        tracing::info!("SyncService started");

        // Load stored credentials on startup
        self.load_stored_credentials().await;

        while let Some(message) = self.receiver.recv().await {
            match message {
                SyncMessage::Connect { platform, response } => {
                    let result = self.handle_connect(platform).await;
                    let _ = response.send(result);
                }
                SyncMessage::Disconnect { platform, response } => {
                    let result = self.handle_disconnect(platform).await;
                    let _ = response.send(result);
                }
                SyncMessage::GetStatus { platform, response } => {
                    let status = self.handle_get_status(platform).await;
                    let _ = response.send(status);
                }
                SyncMessage::GetConnectedPlatforms { response } => {
                    let platforms = self.handle_get_connected_platforms();
                    let _ = response.send(platforms);
                }
                SyncMessage::QueueUpload {
                    ride_id,
                    platform,
                    fit_data,
                    activity_name,
                    response,
                } => {
                    let result = self
                        .handle_queue_upload(ride_id, platform, fit_data, activity_name)
                        .await;
                    let _ = response.send(result);
                }
                SyncMessage::RetryUpload { record_id, response } => {
                    let result = self.handle_retry_upload(record_id).await;
                    let _ = response.send(result);
                }
                SyncMessage::CancelUpload { record_id, response } => {
                    let result = self.handle_cancel_upload(record_id);
                    let _ = response.send(result);
                }
                SyncMessage::GetSyncRecords { ride_id, response } => {
                    let records = self.handle_get_sync_records(ride_id);
                    let _ = response.send(records);
                }
                SyncMessage::UpdateConfig {
                    platform,
                    config,
                    response,
                } => {
                    let result = self.handle_update_config(platform, config).await;
                    let _ = response.send(result);
                }
                SyncMessage::Shutdown => {
                    tracing::info!("SyncService shutting down");
                    break;
                }
            }
        }

        tracing::info!("SyncService stopped");
    }

    /// Load stored credentials from the credential store on startup
    async fn load_stored_credentials(&mut self) {
        tracing::debug!("Loading stored credentials");

        for platform in [SyncPlatform::Strava] {
            if self.credential_store.has_credentials(platform) {
                match self.credential_store.get_tokens(platform).await {
                    Ok(Some(tokens)) => {
                        tracing::info!("Loaded stored credentials for {:?}", platform);

                        // Set token on the appropriate client
                        if let Some(client) = self.clients.get(&platform) {
                            match client {
                                PlatformClient::Strava(strava) => {
                                    strava.set_access_token(tokens.access_token.clone()).await;
                                }
                            }
                        }

                        self.connected.insert(platform, tokens);
                    }
                    Ok(None) => {
                        tracing::debug!("No stored credentials for {:?}", platform);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load credentials for {:?}: {}", platform, e);
                    }
                }
            }
        }
    }

    /// Handle platform connection request
    async fn handle_connect(&mut self, platform: SyncPlatform) -> Result<(), SyncError> {
        tracing::info!("Connecting to {:?}", platform);

        // Check if already connected
        if self.connected.contains_key(&platform) {
            tracing::debug!("{:?} already connected", platform);
            return Ok(());
        }

        // Start OAuth flow
        let auth_url = self.oauth_handler.start_authorization(platform).await?;
        tracing::info!("Authorization URL: {}", auth_url.url);

        // Note: The actual callback handling would be done separately
        // through the OAuthCallbackServer. This method just initiates the flow.
        // The actual token storage happens when handle_callback is called.

        Ok(())
    }

    /// Handle platform disconnection request
    async fn handle_disconnect(&mut self, platform: SyncPlatform) -> Result<(), SyncError> {
        tracing::info!("Disconnecting from {:?}", platform);

        // Deauthorize on the platform
        if let Some(client) = self.clients.get(&platform) {
            match client {
                PlatformClient::Strava(strava) => {
                    if let Err(e) = strava.deauthorize().await {
                        tracing::warn!("Deauthorization failed for {:?}: {}", platform, e);
                        // Continue anyway - we still want to clear local credentials
                    }
                }
            }
        }

        // Clear stored credentials
        self.credential_store.delete_tokens(platform).await?;

        // Remove from connected map
        self.connected.remove(&platform);

        // Revoke in OAuth handler
        self.oauth_handler.revoke(platform).await?;

        tracing::info!("Disconnected from {:?}", platform);
        Ok(())
    }

    /// Handle get status request
    async fn handle_get_status(&self, platform: SyncPlatform) -> PlatformStatus {
        let connected = self.connected.contains_key(&platform);
        let token_status = if connected {
            Some(self.oauth_handler.get_token_status(platform))
        } else {
            None
        };
        let last_sync = self.last_sync.get(&platform).cloned();

        let config = {
            let configs = self.configs.read().await;
            configs
                .platforms
                .get(&platform)
                .cloned()
                .unwrap_or_default()
        };

        let pending_uploads = self
            .upload_queue
            .iter()
            .filter(|e| e.record.platform == platform)
            .count();

        PlatformStatus {
            connected,
            token_status,
            last_sync,
            config,
            pending_uploads,
        }
    }

    /// Handle get connected platforms request
    fn handle_get_connected_platforms(&self) -> Vec<SyncPlatform> {
        self.connected.keys().cloned().collect()
    }

    /// Handle queue upload request
    async fn handle_queue_upload(
        &mut self,
        ride_id: Uuid,
        platform: SyncPlatform,
        fit_data: Vec<u8>,
        activity_name: Option<String>,
    ) -> Result<SyncRecord, SyncError> {
        // Check if platform is connected
        if !self.connected.contains_key(&platform) {
            return Err(SyncError::AuthorizationRequired);
        }

        // Create sync record
        let record = SyncRecord {
            id: Uuid::new_v4(),
            ride_id,
            platform,
            status: SyncRecordStatus::Pending,
            external_id: None,
            external_url: None,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
            retry_count: 0,
        };

        // Add to queue
        let entry = UploadQueueEntry {
            record: record.clone(),
            fit_data,
            activity_name,
            next_retry: None,
        };
        self.upload_queue.push(entry);

        // Store record
        self.sync_records.insert(record.id, record.clone());

        tracing::info!(
            "Queued upload for ride {} to {:?} (record: {})",
            ride_id,
            platform,
            record.id
        );

        // Try to process immediately if possible
        // Note: Actual processing will be handled in the upload queue processor (subtask 2.4)

        Ok(record)
    }

    /// Handle retry upload request
    async fn handle_retry_upload(&mut self, record_id: Uuid) -> Result<SyncRecord, SyncError> {
        // Find the record
        let record = self
            .sync_records
            .get_mut(&record_id)
            .ok_or_else(|| SyncError::ApiError("Record not found".to_string()))?;

        // Check if it can be retried
        if record.status != SyncRecordStatus::Failed {
            return Err(SyncError::ApiError(
                "Only failed uploads can be retried".to_string(),
            ));
        }

        // Reset status
        record.status = SyncRecordStatus::Pending;
        record.error_message = None;
        record.retry_count += 1;

        // Find and update in queue or re-add
        let queue_entry = self
            .upload_queue
            .iter_mut()
            .find(|e| e.record.id == record_id);

        if let Some(entry) = queue_entry {
            entry.record.status = SyncRecordStatus::Pending;
            entry.record.retry_count = record.retry_count;
            entry.next_retry = None;
        }

        tracing::info!("Queued retry for upload {}", record_id);

        Ok(record.clone())
    }

    /// Handle cancel upload request
    fn handle_cancel_upload(&mut self, record_id: Uuid) -> bool {
        // Remove from queue
        let initial_len = self.upload_queue.len();
        self.upload_queue.retain(|e| e.record.id != record_id);

        // Update record status if found
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            if record.status == SyncRecordStatus::Pending {
                record.status = SyncRecordStatus::Cancelled;
                return true;
            }
        }

        self.upload_queue.len() < initial_len
    }

    /// Handle get sync records request
    fn handle_get_sync_records(&self, ride_id: Uuid) -> Vec<SyncRecord> {
        self.sync_records
            .values()
            .filter(|r| r.ride_id == ride_id)
            .cloned()
            .collect()
    }

    /// Handle update config request
    async fn handle_update_config(
        &mut self,
        platform: SyncPlatform,
        config: PlatformConfig,
    ) -> Result<(), SyncError> {
        let mut configs = self.configs.write().await;
        configs.platforms.insert(platform, config);
        tracing::info!("Updated config for {:?}", platform);
        Ok(())
    }
}

/// Create a SyncService with default handlers
pub fn create_sync_service(
    oauth_handler: super::oauth::DefaultOAuthHandler,
    config: SyncConfig,
) -> SyncServiceHandle {
    let credential_store = KeyringCredentialStore::new("RustRide");
    SyncService::spawn(
        Arc::new(oauth_handler),
        Arc::new(credential_store),
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::sync::oauth::DefaultOAuthHandler;

    #[tokio::test]
    async fn test_sync_service_creation() {
        let oauth_handler = DefaultOAuthHandler::new(8888);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        // Service should be running
        let platforms = handle.get_connected_platforms().await.unwrap();
        assert!(platforms.is_empty());

        // Shutdown
        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_status_not_connected() {
        let oauth_handler = DefaultOAuthHandler::new(8889);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        let status = handle.get_status(SyncPlatform::Strava).await.unwrap();
        assert!(!status.connected);
        assert!(status.token_status.is_none());
        assert_eq!(status.pending_uploads, 0);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_queue_upload_requires_connection() {
        let oauth_handler = DefaultOAuthHandler::new(8890);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        let result = handle
            .queue_upload(
                Uuid::new_v4(),
                SyncPlatform::Strava,
                vec![0u8; 100],
                Some("Test Ride".to_string()),
            )
            .await;

        assert!(matches!(result, Err(SyncError::AuthorizationRequired)));

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_upload() {
        let oauth_handler = DefaultOAuthHandler::new(8891);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        let result = handle.cancel_upload(Uuid::new_v4()).await.unwrap();
        assert!(!result);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_config() {
        let oauth_handler = DefaultOAuthHandler::new(8892);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        let new_config = PlatformConfig {
            enabled: true,
            auto_sync: true,
        };

        handle
            .update_config(SyncPlatform::Strava, new_config.clone())
            .await
            .unwrap();

        let status = handle.get_status(SyncPlatform::Strava).await.unwrap();
        assert!(status.config.enabled);
        assert!(status.config.auto_sync);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_sync_records_empty() {
        let oauth_handler = DefaultOAuthHandler::new(8893);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        let records = handle.get_sync_records(Uuid::new_v4()).await.unwrap();
        assert!(records.is_empty());

        handle.shutdown().await.unwrap();
    }
}
