//! Sync Service for managing platform connections and uploads.
//!
//! T104: SyncService implementation
//!
//! Implements an async actor pattern for managing:
//! - Platform connections (Strava, etc.)
//! - Token refresh scheduling
//! - Upload queue management
//! - Persistent queue with offline support

use super::oauth::{CredentialStore, KeyringCredentialStore, OAuthHandler, TokenResponse, TokenStatus};
use super::strava::StravaClient;
use super::{PlatformConfig, SyncConfig, SyncError, SyncPlatform, SyncRecord, SyncRecordStatus};
use crate::storage::sync_store::{
    delete_fit_from_queue, load_fit_from_queue, save_fit_for_queue, StoredPlatformSync,
    StoredUploadQueueEntry, SyncStore,
};
use crate::storage::Database;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

/// How often to check for token refresh (in seconds)
const TOKEN_REFRESH_CHECK_INTERVAL_SECS: u64 = 60;

/// How long before token expiry to proactively refresh (in minutes)
const TOKEN_REFRESH_BUFFER_MINUTES: i64 = 5;

/// Maximum number of consecutive refresh failures before giving up
const MAX_REFRESH_FAILURES: u32 = 3;

/// Delay between refresh retries (in seconds)
const REFRESH_RETRY_DELAY_SECS: u64 = 30;

/// How often to process the upload queue (in seconds)
const QUEUE_PROCESS_INTERVAL_SECS: u64 = 30;

/// How often to check connectivity (in seconds)
const CONNECTIVITY_CHECK_INTERVAL_SECS: u64 = 60;

/// Maximum number of upload retries before giving up
const MAX_UPLOAD_RETRIES: i32 = 5;

/// Base delay for exponential backoff (in seconds)
const BASE_RETRY_DELAY_SECS: i64 = 30;

/// URL to use for connectivity checks
const CONNECTIVITY_CHECK_URL: &str = "https://www.strava.com";

/// Events emitted by the SyncService for external consumers
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// Token was refreshed successfully
    TokenRefreshed {
        platform: SyncPlatform,
        expires_at: DateTime<Utc>,
    },
    /// Token refresh failed, will retry
    TokenRefreshFailed {
        platform: SyncPlatform,
        error: String,
        retry_count: u32,
    },
    /// Re-authorization is required (refresh token invalid/expired)
    ReauthorizationRequired {
        platform: SyncPlatform,
    },
    /// Platform was connected
    PlatformConnected {
        platform: SyncPlatform,
    },
    /// Platform was disconnected
    PlatformDisconnected {
        platform: SyncPlatform,
    },
    /// Upload started
    UploadStarted {
        record_id: Uuid,
        ride_id: Uuid,
        platform: SyncPlatform,
    },
    /// Upload completed successfully
    UploadCompleted {
        record_id: Uuid,
        ride_id: Uuid,
        platform: SyncPlatform,
        external_id: Option<String>,
        external_url: Option<String>,
    },
    /// Upload failed
    UploadFailed {
        record_id: Uuid,
        ride_id: Uuid,
        platform: SyncPlatform,
        error: String,
        retry_count: i32,
        will_retry: bool,
    },
    /// Manual retry queued for a failed upload
    UploadRetryQueued {
        record_id: Uuid,
        ride_id: Uuid,
        platform: SyncPlatform,
        retry_count: i32,
    },
    /// Connectivity changed
    ConnectivityChanged {
        is_online: bool,
    },
    /// Queue processing started
    QueueProcessingStarted {
        pending_count: usize,
    },
}

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
        user_id: Option<Uuid>,
        response: oneshot::Sender<Result<(), SyncError>>,
    },
    /// Internal message to trigger token refresh check
    CheckTokenRefresh,
    /// Internal message to process the upload queue
    ProcessQueue,
    /// Internal message to check connectivity
    CheckConnectivity,
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
    /// Event receiver for subscribing to sync events
    event_receiver: Arc<RwLock<Option<mpsc::Receiver<SyncEvent>>>>,
}

impl SyncServiceHandle {
    /// Subscribe to sync events.
    ///
    /// Returns a receiver for sync events. Only one subscriber is supported.
    /// Calling this again will return None if already subscribed.
    pub async fn subscribe_events(&self) -> Option<mpsc::Receiver<SyncEvent>> {
        self.event_receiver.write().await.take()
    }

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
    ///
    /// If `user_id` is provided, the configuration will be persisted to the database.
    pub async fn update_config(
        &self,
        platform: SyncPlatform,
        config: PlatformConfig,
    ) -> Result<(), SyncError> {
        self.update_config_with_user(platform, config, None).await
    }

    /// Update platform configuration with user ID for database persistence.
    ///
    /// This persists the configuration to the database if a user_id is provided.
    pub async fn update_config_with_user(
        &self,
        platform: SyncPlatform,
        config: PlatformConfig,
        user_id: Option<Uuid>,
    ) -> Result<(), SyncError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(SyncMessage::UpdateConfig {
                platform,
                config,
                user_id,
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

/// Tracks token refresh state for a platform
#[derive(Debug, Clone, Default)]
struct TokenRefreshState {
    /// Number of consecutive refresh failures
    failure_count: u32,
    /// Time of last refresh attempt
    last_attempt: Option<DateTime<Utc>>,
    /// Whether re-authorization has been requested
    reauth_requested: bool,
}

/// Sync Service that manages platform connections and uploads.
///
/// This service runs as an async actor, receiving messages through a channel.
/// It manages OAuth connections for platforms like Strava, handles token refresh,
/// and processes an upload queue for syncing rides.
pub struct SyncService<O: OAuthHandler + 'static, C: CredentialStore + 'static> {
    /// Message receiver
    receiver: mpsc::Receiver<SyncMessage>,
    /// Message sender for internal use (e.g., scheduling refresh checks)
    sender: mpsc::Sender<SyncMessage>,
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
    /// Upload queue (in-memory cache of database queue)
    upload_queue: Vec<UploadQueueEntry>,
    /// Sync records (in-memory cache)
    sync_records: HashMap<Uuid, SyncRecord>,
    /// Last sync times per platform
    last_sync: HashMap<SyncPlatform, DateTime<Utc>>,
    /// Token refresh state per platform
    refresh_state: HashMap<SyncPlatform, TokenRefreshState>,
    /// Event sender for notifying external consumers
    event_sender: mpsc::Sender<SyncEvent>,
    /// Database path for persistent storage
    db_path: Option<PathBuf>,
    /// Current connectivity status
    is_online: bool,
    /// Whether the queue is currently being processed
    is_processing_queue: bool,
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
        Self::spawn_with_db(oauth_handler, credential_store, config, None)
    }

    /// Create a new SyncService with database persistence.
    ///
    /// The service will start running in the background immediately and
    /// will persist the upload queue to the specified database.
    pub fn spawn_with_db(
        oauth_handler: Arc<O>,
        credential_store: Arc<C>,
        config: SyncConfig,
        db_path: Option<PathBuf>,
    ) -> SyncServiceHandle {
        let (sender, receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(32);

        // Initialize platform clients
        let mut clients = HashMap::new();
        clients.insert(SyncPlatform::Strava, PlatformClient::Strava(StravaClient::new()));

        let service = Self {
            receiver,
            sender: sender.clone(),
            oauth_handler,
            credential_store,
            clients,
            configs: Arc::new(RwLock::new(config)),
            connected: HashMap::new(),
            upload_queue: Vec::new(),
            sync_records: HashMap::new(),
            last_sync: HashMap::new(),
            refresh_state: HashMap::new(),
            event_sender,
            db_path,
            is_online: true, // Assume online initially
            is_processing_queue: false,
        };

        // Spawn the service actor
        tokio::spawn(service.run());

        // Spawn the token refresh scheduler
        let refresh_sender = sender.clone();
        tokio::spawn(async move {
            Self::token_refresh_scheduler(refresh_sender).await;
        });

        // Spawn the queue processor scheduler
        let queue_sender = sender.clone();
        tokio::spawn(async move {
            Self::queue_processor_scheduler(queue_sender).await;
        });

        // Spawn the connectivity checker
        let connectivity_sender = sender.clone();
        tokio::spawn(async move {
            Self::connectivity_checker_scheduler(connectivity_sender).await;
        });

        SyncServiceHandle {
            sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
        }
    }

    /// Background task that periodically triggers queue processing.
    async fn queue_processor_scheduler(sender: mpsc::Sender<SyncMessage>) {
        let mut interval = tokio::time::interval(Duration::from_secs(QUEUE_PROCESS_INTERVAL_SECS));

        loop {
            interval.tick().await;

            if sender.send(SyncMessage::ProcessQueue).await.is_err() {
                tracing::debug!("Queue processor scheduler stopping: service channel closed");
                break;
            }
        }
    }

    /// Background task that periodically checks connectivity.
    async fn connectivity_checker_scheduler(sender: mpsc::Sender<SyncMessage>) {
        let mut interval = tokio::time::interval(Duration::from_secs(CONNECTIVITY_CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            if sender.send(SyncMessage::CheckConnectivity).await.is_err() {
                tracing::debug!("Connectivity checker stopping: service channel closed");
                break;
            }
        }
    }

    /// Background task that periodically triggers token refresh checks.
    async fn token_refresh_scheduler(sender: mpsc::Sender<SyncMessage>) {
        let mut interval = tokio::time::interval(Duration::from_secs(TOKEN_REFRESH_CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            // Send a check token refresh message to the service
            if sender.send(SyncMessage::CheckTokenRefresh).await.is_err() {
                // Service has shut down
                tracing::debug!("Token refresh scheduler stopping: service channel closed");
                break;
            }
        }
    }

    /// Run the service event loop
    async fn run(mut self) {
        tracing::info!("SyncService started");

        // Load stored credentials on startup
        self.load_stored_credentials().await;

        // Load pending uploads from database on startup
        self.load_pending_uploads_from_db().await;

        // Check initial connectivity
        self.check_connectivity().await;

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
                    user_id,
                    response,
                } => {
                    let result = self.handle_update_config(platform, config, user_id).await;
                    let _ = response.send(result);
                }
                SyncMessage::CheckTokenRefresh => {
                    self.handle_token_refresh_check().await;
                }
                SyncMessage::ProcessQueue => {
                    self.process_upload_queue().await;
                }
                SyncMessage::CheckConnectivity => {
                    self.check_connectivity().await;
                }
                SyncMessage::Shutdown => {
                    tracing::info!("SyncService shutting down");
                    break;
                }
            }
        }

        tracing::info!("SyncService stopped");
    }

    /// Load pending uploads from the database on startup.
    async fn load_pending_uploads_from_db(&mut self) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => {
                tracing::debug!("No database path configured, skipping pending upload load");
                return;
            }
        };

        match Database::open(&db_path) {
            Ok(db) => {
                let conn = db.connection();
                let store = SyncStore::new(conn);

                // Initialize the upload queue table if needed
                if let Err(e) = store.init_upload_queue_table() {
                    tracing::warn!("Failed to initialize upload queue table: {}", e);
                    return;
                }

                match store.get_pending_entries() {
                    Ok(entries) => {
                        tracing::info!("Loaded {} pending uploads from database", entries.len());

                        for entry in entries {
                            // Convert stored entry to UploadQueueEntry
                            if let Some(queue_entry) = self.stored_entry_to_queue_entry(&entry) {
                                self.upload_queue.push(queue_entry.clone());
                                self.sync_records.insert(queue_entry.record.id, queue_entry.record);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load pending uploads: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to open database for loading pending uploads: {}", e);
            }
        }
    }

    /// Convert a stored upload queue entry to an in-memory UploadQueueEntry.
    fn stored_entry_to_queue_entry(&self, stored: &StoredUploadQueueEntry) -> Option<UploadQueueEntry> {
        // Parse platform
        let platform = match stored.platform.as_str() {
            "Strava" => SyncPlatform::Strava,
            "GarminConnect" => SyncPlatform::GarminConnect,
            "TrainingPeaks" => SyncPlatform::TrainingPeaks,
            "IntervalsIcu" => SyncPlatform::IntervalsIcu,
            _ => {
                tracing::warn!("Unknown platform in stored entry: {}", stored.platform);
                return None;
            }
        };

        // Load FIT data from file
        let fit_data = match load_fit_from_queue(&stored.fit_file_path) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to load FIT file for queue entry {}: {}", stored.id, e);
                return None;
            }
        };

        // Parse status
        let status = match stored.status.as_str() {
            "pending" => SyncRecordStatus::Pending,
            "uploading" => SyncRecordStatus::Uploading,
            "completed" => SyncRecordStatus::Completed,
            "failed" => SyncRecordStatus::Failed,
            "cancelled" => SyncRecordStatus::Cancelled,
            _ => SyncRecordStatus::Pending,
        };

        // Parse created_at
        let created_at = DateTime::parse_from_rfc3339(&stored.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // Parse completed_at
        let completed_at = stored.completed_at.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        // Parse next_retry
        let next_retry = stored.next_retry_at.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        let record = SyncRecord {
            id: stored.id,
            ride_id: stored.ride_id,
            platform,
            status,
            external_id: stored.external_activity_id.clone(),
            external_url: stored.external_activity_url.clone(),
            created_at,
            completed_at,
            error_message: stored.error_message.clone(),
            retry_count: stored.retry_count as u32,
        };

        Some(UploadQueueEntry {
            record,
            fit_data,
            activity_name: stored.activity_name.clone(),
            next_retry,
        })
    }

    /// Check network connectivity by attempting to reach Strava's API.
    async fn check_connectivity(&mut self) {
        let was_online = self.is_online;

        // Try a simple HTTP HEAD request to check connectivity
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        match client.head(CONNECTIVITY_CHECK_URL).send().await {
            Ok(_) => {
                self.is_online = true;
                if !was_online {
                    tracing::info!("Network connectivity restored");
                    self.emit_event(SyncEvent::ConnectivityChanged { is_online: true });

                    // Trigger immediate queue processing when coming back online
                    if let Err(e) = self.sender.try_send(SyncMessage::ProcessQueue) {
                        tracing::debug!("Failed to trigger queue processing after reconnect: {:?}", e);
                    }
                }
            }
            Err(e) => {
                self.is_online = false;
                if was_online {
                    tracing::warn!("Network connectivity lost: {}", e);
                    self.emit_event(SyncEvent::ConnectivityChanged { is_online: false });
                }
            }
        }
    }

    /// Process the upload queue, attempting to upload pending entries.
    async fn process_upload_queue(&mut self) {
        // Skip if already processing or offline
        if self.is_processing_queue || !self.is_online {
            return;
        }

        // Get pending entries that are ready to process
        let pending_entries: Vec<UploadQueueEntry> = self
            .upload_queue
            .iter()
            .filter(|e| {
                e.record.status == SyncRecordStatus::Pending && self.is_entry_ready_to_process(e)
            })
            .cloned()
            .collect();

        if pending_entries.is_empty() {
            return;
        }

        self.is_processing_queue = true;
        self.emit_event(SyncEvent::QueueProcessingStarted {
            pending_count: pending_entries.len(),
        });

        tracing::info!("Processing {} pending uploads", pending_entries.len());

        for entry in pending_entries {
            // Check if platform is connected
            if !self.connected.contains_key(&entry.record.platform) {
                tracing::debug!(
                    "Skipping upload {} - platform {:?} not connected",
                    entry.record.id,
                    entry.record.platform
                );
                continue;
            }

            // Check connectivity again before each upload
            if !self.is_online {
                tracing::info!("Upload queue processing stopped - offline");
                break;
            }

            self.process_single_upload(&entry).await;
        }

        self.is_processing_queue = false;
    }

    /// Check if an upload queue entry is ready to be processed (respecting retry delay).
    fn is_entry_ready_to_process(&self, entry: &UploadQueueEntry) -> bool {
        if let Some(next_retry) = entry.next_retry {
            Utc::now() >= next_retry
        } else {
            true
        }
    }

    /// Process a single upload from the queue.
    async fn process_single_upload(&mut self, entry: &UploadQueueEntry) {
        let record_id = entry.record.id;
        let ride_id = entry.record.ride_id;
        let platform = entry.record.platform;

        // Emit upload started event
        self.emit_event(SyncEvent::UploadStarted {
            record_id,
            ride_id,
            platform,
        });

        // Update status to uploading
        self.update_queue_entry_status(record_id, SyncRecordStatus::Uploading, None);

        // Attempt the upload
        let result = match platform {
            SyncPlatform::Strava => {
                if let Some(PlatformClient::Strava(client)) = self.clients.get(&platform) {
                    client
                        .upload_activity(
                            &entry.fit_data,
                            entry.activity_name.as_deref(),
                            None, // description
                        )
                        .await
                } else {
                    Err(SyncError::NotConfigured(platform))
                }
            }
            _ => Err(SyncError::NotConfigured(platform)),
        };

        match result {
            Ok(upload_record) => {
                tracing::info!(
                    "Upload {} completed successfully (external_id: {:?})",
                    record_id,
                    upload_record.external_id
                );

                // Update queue entry with success
                self.mark_upload_completed(
                    record_id,
                    upload_record.external_id.clone(),
                    upload_record.external_url.clone(),
                );

                // Emit success event
                self.emit_event(SyncEvent::UploadCompleted {
                    record_id,
                    ride_id,
                    platform,
                    external_id: upload_record.external_id,
                    external_url: upload_record.external_url,
                });
            }
            Err(e) => {
                let error_msg = e.to_string();
                let current_retry = self
                    .upload_queue
                    .iter()
                    .find(|e| e.record.id == record_id)
                    .map(|e| e.record.retry_count as i32)
                    .unwrap_or(0);

                let will_retry = current_retry < MAX_UPLOAD_RETRIES && self.is_retryable_error(&e);

                tracing::warn!(
                    "Upload {} failed (attempt {}/{}): {} (will_retry: {})",
                    record_id,
                    current_retry + 1,
                    MAX_UPLOAD_RETRIES,
                    error_msg,
                    will_retry
                );

                if will_retry {
                    // Calculate next retry time with exponential backoff
                    let backoff = BASE_RETRY_DELAY_SECS * (2_i64.pow(current_retry as u32));
                    let next_retry = Utc::now() + ChronoDuration::seconds(backoff);
                    self.mark_upload_failed_with_retry(record_id, &error_msg, next_retry);
                } else {
                    // Permanent failure
                    self.mark_upload_permanently_failed(record_id, &error_msg);
                }

                // Emit failure event
                self.emit_event(SyncEvent::UploadFailed {
                    record_id,
                    ride_id,
                    platform,
                    error: error_msg,
                    retry_count: current_retry + 1,
                    will_retry,
                });
            }
        }
    }

    /// Check if an error is retryable (e.g., network issues).
    fn is_retryable_error(&self, error: &SyncError) -> bool {
        matches!(
            error,
            SyncError::NetworkError(_) | SyncError::ApiError(_)
        )
    }

    /// Update queue entry status in memory and database.
    fn update_queue_entry_status(
        &mut self,
        record_id: Uuid,
        status: SyncRecordStatus,
        error_message: Option<&str>,
    ) {
        // Update in-memory queue
        if let Some(entry) = self.upload_queue.iter_mut().find(|e| e.record.id == record_id) {
            entry.record.status = status;
            entry.record.error_message = error_message.map(String::from);
        }

        // Update sync records
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            record.status = status;
            record.error_message = error_message.map(String::from);
        }

        // Update database
        self.persist_queue_entry_status(record_id, status, error_message);
    }

    /// Mark an upload as completed.
    fn mark_upload_completed(
        &mut self,
        record_id: Uuid,
        external_id: Option<String>,
        external_url: Option<String>,
    ) {
        // Update in-memory queue
        if let Some(entry) = self.upload_queue.iter_mut().find(|e| e.record.id == record_id) {
            entry.record.status = SyncRecordStatus::Completed;
            entry.record.external_id = external_id.clone();
            entry.record.external_url = external_url.clone();
            entry.record.completed_at = Some(Utc::now());
        }

        // Update sync records
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            record.status = SyncRecordStatus::Completed;
            record.external_id = external_id.clone();
            record.external_url = external_url.clone();
            record.completed_at = Some(Utc::now());
        }

        // Update database
        self.persist_queue_entry_completed(record_id, external_id.as_deref(), external_url.as_deref());
    }

    /// Mark an upload as failed with retry scheduled.
    fn mark_upload_failed_with_retry(
        &mut self,
        record_id: Uuid,
        error_message: &str,
        next_retry: DateTime<Utc>,
    ) {
        // Update in-memory queue
        if let Some(entry) = self.upload_queue.iter_mut().find(|e| e.record.id == record_id) {
            entry.record.status = SyncRecordStatus::Pending; // Back to pending for retry
            entry.record.error_message = Some(error_message.to_string());
            entry.record.retry_count += 1;
            entry.next_retry = Some(next_retry);
        }

        // Update sync records
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            record.status = SyncRecordStatus::Pending;
            record.error_message = Some(error_message.to_string());
            record.retry_count += 1;
        }

        // Update database
        self.persist_queue_entry_failed(record_id, error_message, Some(next_retry));
    }

    /// Mark an upload as permanently failed (no more retries).
    fn mark_upload_permanently_failed(&mut self, record_id: Uuid, error_message: &str) {
        // Update in-memory queue
        if let Some(entry) = self.upload_queue.iter_mut().find(|e| e.record.id == record_id) {
            entry.record.status = SyncRecordStatus::Failed;
            entry.record.error_message = Some(error_message.to_string());
        }

        // Update sync records
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            record.status = SyncRecordStatus::Failed;
            record.error_message = Some(error_message.to_string());
        }

        // Update database
        self.persist_queue_entry_permanently_failed(record_id, error_message);
    }

    /// Persist queue entry status to database.
    fn persist_queue_entry_status(
        &self,
        record_id: Uuid,
        status: SyncRecordStatus,
        error_message: Option<&str>,
    ) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        let status_str = match status {
            SyncRecordStatus::Pending => "pending",
            SyncRecordStatus::Uploading => "uploading",
            SyncRecordStatus::Completed => "completed",
            SyncRecordStatus::Failed => "failed",
            SyncRecordStatus::Cancelled => "cancelled",
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());
            if let Err(e) = store.update_status(&record_id, status_str, error_message) {
                tracing::warn!("Failed to persist queue entry status: {}", e);
            }
        }
    }

    /// Persist completed upload to database.
    fn persist_queue_entry_completed(
        &self,
        record_id: Uuid,
        external_id: Option<&str>,
        external_url: Option<&str>,
    ) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());
            if let Err(e) = store.mark_completed(&record_id, external_id, external_url) {
                tracing::warn!("Failed to persist completed upload: {}", e);
            }
        }
    }

    /// Persist failed upload with retry info to database.
    fn persist_queue_entry_failed(
        &self,
        record_id: Uuid,
        error_message: &str,
        next_retry: Option<DateTime<Utc>>,
    ) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());
            if let Err(e) = store.mark_failed(&record_id, error_message, next_retry) {
                tracing::warn!("Failed to persist failed upload: {}", e);
            }
        }
    }

    /// Persist permanently failed upload to database.
    fn persist_queue_entry_permanently_failed(&self, record_id: Uuid, error_message: &str) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());
            if let Err(e) = store.mark_permanently_failed(&record_id, error_message) {
                tracing::warn!("Failed to persist permanently failed upload: {}", e);
            }
        }
    }

    /// Handle token refresh check for all connected platforms.
    ///
    /// This is called periodically by the token refresh scheduler to proactively
    /// refresh tokens before they expire.
    async fn handle_token_refresh_check(&mut self) {
        // Get list of connected platforms
        let platforms: Vec<SyncPlatform> = self.connected.keys().cloned().collect();

        for platform in platforms {
            self.check_and_refresh_token(platform).await;
        }
    }

    /// Check token status for a platform and refresh if needed.
    async fn check_and_refresh_token(&mut self, platform: SyncPlatform) {
        // Get current token
        let token = match self.connected.get(&platform) {
            Some(t) => t.clone(),
            None => return,
        };

        // Get refresh state
        let refresh_state = self.refresh_state.entry(platform).or_default().clone();

        // If re-auth has already been requested and not resolved, skip
        if refresh_state.reauth_requested {
            return;
        }

        // Check if we should attempt refresh based on retry delay
        if let Some(last_attempt) = refresh_state.last_attempt {
            let retry_delay = ChronoDuration::seconds(REFRESH_RETRY_DELAY_SECS as i64);
            if Utc::now() < last_attempt + retry_delay && refresh_state.failure_count > 0 {
                // Too soon to retry after a failure
                return;
            }
        }

        // Check token status
        let now = Utc::now();
        let refresh_threshold = now + ChronoDuration::minutes(TOKEN_REFRESH_BUFFER_MINUTES);

        if token.expires_at > refresh_threshold {
            // Token still valid, no refresh needed
            // Reset failure count on success
            if let Some(state) = self.refresh_state.get_mut(&platform) {
                if state.failure_count > 0 {
                    state.failure_count = 0;
                    state.last_attempt = None;
                }
            }
            return;
        }

        tracing::info!(
            "Token for {:?} expires at {}, proactively refreshing",
            platform,
            token.expires_at
        );

        // Update last attempt time
        if let Some(state) = self.refresh_state.get_mut(&platform) {
            state.last_attempt = Some(now);
        }

        // Attempt to refresh the token
        match self.oauth_handler.refresh_token(platform).await {
            Ok(new_tokens) => {
                tracing::info!(
                    "Successfully refreshed token for {:?}, new expiry: {}",
                    platform,
                    new_tokens.expires_at
                );

                // Update the client's access token
                if let Some(client) = self.clients.get(&platform) {
                    match client {
                        PlatformClient::Strava(strava) => {
                            strava.set_access_token(new_tokens.access_token.clone()).await;
                        }
                    }
                }

                // Store new tokens in credential store
                if let Err(e) = self.credential_store.store_tokens(platform, &new_tokens).await {
                    tracing::warn!("Failed to store refreshed tokens for {:?}: {}", platform, e);
                }

                // Update connected tokens
                let expires_at = new_tokens.expires_at;
                self.connected.insert(platform, new_tokens);

                // Reset refresh state
                if let Some(state) = self.refresh_state.get_mut(&platform) {
                    state.failure_count = 0;
                    state.last_attempt = None;
                    state.reauth_requested = false;
                }

                // Emit success event
                self.emit_event(SyncEvent::TokenRefreshed { platform, expires_at });
            }
            Err(SyncError::AuthorizationRequired) => {
                tracing::warn!(
                    "Re-authorization required for {:?}, refresh token is invalid/expired",
                    platform
                );

                // Mark re-auth as requested
                if let Some(state) = self.refresh_state.get_mut(&platform) {
                    state.reauth_requested = true;
                }

                // Emit re-authorization event
                self.emit_event(SyncEvent::ReauthorizationRequired { platform });
            }
            Err(e) => {
                let failure_count = {
                    let state = self.refresh_state.entry(platform).or_default();
                    state.failure_count += 1;
                    state.failure_count
                };

                let error_msg = e.to_string();
                tracing::warn!(
                    "Token refresh failed for {:?} (attempt {}/{}): {}",
                    platform,
                    failure_count,
                    MAX_REFRESH_FAILURES,
                    error_msg
                );

                // Emit failure event
                self.emit_event(SyncEvent::TokenRefreshFailed {
                    platform,
                    error: error_msg,
                    retry_count: failure_count,
                });

                // If max failures reached, require re-authorization
                if failure_count >= MAX_REFRESH_FAILURES {
                    tracing::error!(
                        "Max refresh failures reached for {:?}, re-authorization required",
                        platform
                    );

                    if let Some(state) = self.refresh_state.get_mut(&platform) {
                        state.reauth_requested = true;
                    }

                    self.emit_event(SyncEvent::ReauthorizationRequired { platform });
                }
            }
        }
    }

    /// Emit an event to subscribers.
    fn emit_event(&self, event: SyncEvent) {
        // Use try_send to avoid blocking if no one is listening or buffer is full
        if let Err(e) = self.event_sender.try_send(event) {
            match e {
                mpsc::error::TrySendError::Full(event) => {
                    tracing::debug!("Event channel full, dropping event: {:?}", event);
                }
                mpsc::error::TrySendError::Closed(_) => {
                    // No subscriber, this is fine
                }
            }
        }
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

        // Get platform name for storage
        let platform_name = match platform {
            SyncPlatform::Strava => "Strava",
            SyncPlatform::GarminConnect => "GarminConnect",
            SyncPlatform::TrainingPeaks => "TrainingPeaks",
            SyncPlatform::IntervalsIcu => "IntervalsIcu",
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => "HealthKit",
        };

        // Save FIT data to file and persist to database if available
        let fit_file_path = if self.db_path.is_some() {
            match save_fit_for_queue(&ride_id, platform_name, &fit_data) {
                Ok(path) => {
                    // Persist to database
                    let stored_entry = StoredUploadQueueEntry {
                        id: record.id,
                        ride_id,
                        platform: platform_name.to_string(),
                        fit_file_path: path.to_string_lossy().to_string(),
                        activity_name: activity_name.clone(),
                        status: "pending".to_string(),
                        error_message: None,
                        external_activity_id: None,
                        external_activity_url: None,
                        retry_count: 0,
                        next_retry_at: None,
                        created_at: record.created_at.to_rfc3339(),
                        completed_at: None,
                    };

                    if let Some(db_path) = &self.db_path {
                        if let Ok(db) = Database::open(db_path) {
                            let store = SyncStore::new(db.connection());
                            if let Err(e) = store.init_upload_queue_table() {
                                tracing::warn!("Failed to init upload queue table: {}", e);
                            }
                            if let Err(e) = store.add_to_queue(&stored_entry) {
                                tracing::warn!("Failed to persist upload to database: {}", e);
                            }
                        }
                    }

                    Some(path)
                }
                Err(e) => {
                    tracing::warn!("Failed to save FIT file for queue: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Add to in-memory queue
        let entry = UploadQueueEntry {
            record: record.clone(),
            fit_data,
            activity_name,
            next_retry: None,
        };
        self.upload_queue.push(entry);

        // Store record in memory
        self.sync_records.insert(record.id, record.clone());

        tracing::info!(
            "Queued upload for ride {} to {:?} (record: {}, persisted: {})",
            ride_id,
            platform,
            record.id,
            fit_file_path.is_some()
        );

        // Trigger queue processing if online
        if self.is_online {
            if let Err(e) = self.sender.try_send(SyncMessage::ProcessQueue) {
                tracing::debug!("Failed to trigger immediate queue processing: {:?}", e);
            }
        }

        Ok(record)
    }

    /// Handle retry upload request.
    ///
    /// Allows manual retry of a failed upload. This resets the upload status to pending
    /// and clears the next_retry delay so it will be processed immediately.
    /// The retry_count is incremented to track manual retries.
    async fn handle_retry_upload(&mut self, record_id: Uuid) -> Result<SyncRecord, SyncError> {
        // First, try to find the record in memory
        let record_opt = self.sync_records.get(&record_id).cloned();

        let record = if let Some(record) = record_opt {
            record
        } else {
            // Try to load from database if not in memory
            if let Some(entry) = self.load_queue_entry_from_db(&record_id) {
                // Add to in-memory structures
                self.sync_records.insert(record_id, entry.record.clone());
                self.upload_queue.push(entry.clone());
                entry.record
            } else {
                return Err(SyncError::ApiError("Record not found".to_string()));
            }
        };

        // Check if it can be retried
        if record.status != SyncRecordStatus::Failed {
            return Err(SyncError::ApiError(
                "Only failed uploads can be retried".to_string(),
            ));
        }

        // Check if max retries exceeded - for manual retry we allow one more attempt
        // but warn if we've exceeded the automatic retry limit
        let current_retry_count = record.retry_count as i32;
        if current_retry_count >= MAX_UPLOAD_RETRIES * 2 {
            // Even manual retries have a limit (double the automatic limit)
            return Err(SyncError::ApiError(format!(
                "Maximum retry attempts ({}) exceeded. Upload permanently failed.",
                MAX_UPLOAD_RETRIES * 2
            )));
        }

        // Update the in-memory record
        if let Some(mem_record) = self.sync_records.get_mut(&record_id) {
            mem_record.status = SyncRecordStatus::Pending;
            mem_record.error_message = None;
            mem_record.retry_count += 1;
        }

        // Find and update in queue
        if let Some(entry) = self.upload_queue.iter_mut().find(|e| e.record.id == record_id) {
            entry.record.status = SyncRecordStatus::Pending;
            entry.record.error_message = None;
            entry.record.retry_count += 1;
            entry.next_retry = None; // Clear retry delay for immediate processing
        }

        // Persist retry reset to database
        self.persist_retry_reset(record_id);

        let updated_record = self.sync_records.get(&record_id).cloned().unwrap_or(record);

        tracing::info!(
            "Manual retry queued for upload {} (attempt {})",
            record_id,
            updated_record.retry_count
        );

        // Emit event for retry
        self.emit_event(SyncEvent::UploadRetryQueued {
            record_id,
            ride_id: updated_record.ride_id,
            platform: updated_record.platform,
            retry_count: updated_record.retry_count as i32,
        });

        // Trigger immediate queue processing if online
        if self.is_online {
            if let Err(e) = self.sender.try_send(SyncMessage::ProcessQueue) {
                tracing::debug!("Failed to trigger immediate queue processing for retry: {:?}", e);
            }
        }

        Ok(updated_record)
    }

    /// Load a queue entry from the database by record ID.
    fn load_queue_entry_from_db(&self, record_id: &Uuid) -> Option<UploadQueueEntry> {
        let db_path = self.db_path.as_ref()?;

        let db = Database::open(db_path).ok()?;
        let store = SyncStore::new(db.connection());
        let stored = store.get_queue_entry(record_id).ok()??;

        self.stored_entry_to_queue_entry(&stored)
    }

    /// Persist a retry reset to the database.
    fn persist_retry_reset(&self, record_id: Uuid) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());

            // First reset the status
            if let Err(e) = store.reset_for_retry(&record_id) {
                tracing::warn!("Failed to reset entry for retry in database: {}", e);
            }

            // Then increment the retry count
            if let Err(e) = store.increment_retry_count(&record_id) {
                tracing::warn!("Failed to increment retry count in database: {}", e);
            }
        }
    }

    /// Handle cancel upload request
    fn handle_cancel_upload(&mut self, record_id: Uuid) -> bool {
        // Remove from in-memory queue
        let initial_len = self.upload_queue.len();
        self.upload_queue.retain(|e| e.record.id != record_id);

        // Update record status if found
        if let Some(record) = self.sync_records.get_mut(&record_id) {
            if record.status == SyncRecordStatus::Pending {
                record.status = SyncRecordStatus::Cancelled;

                // Persist cancellation to database
                if let Some(db_path) = &self.db_path {
                    if let Ok(db) = Database::open(db_path) {
                        let store = SyncStore::new(db.connection());
                        if let Err(e) = store.cancel_entry(&record_id) {
                            tracing::warn!("Failed to persist upload cancellation: {}", e);
                        }
                    }
                }

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
        user_id: Option<Uuid>,
    ) -> Result<(), SyncError> {
        // Update in-memory config
        {
            let mut configs = self.configs.write().await;
            configs.platforms.insert(platform, config.clone());
        }

        // Persist to database if user_id provided and database is configured
        if let Some(user_id) = user_id {
            if self.db_path.is_some() {
                self.persist_platform_config(user_id, platform, &config);
            }
        }

        tracing::info!(
            "Updated config for {:?}: enabled={}, auto_sync={}",
            platform,
            config.enabled,
            config.auto_sync
        );
        Ok(())
    }

    /// Persist platform configuration to the database.
    fn persist_platform_config(
        &self,
        user_id: Uuid,
        platform: SyncPlatform,
        config: &PlatformConfig,
    ) {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return,
        };

        let platform_name = match platform {
            SyncPlatform::Strava => "Strava",
            SyncPlatform::GarminConnect => "GarminConnect",
            SyncPlatform::TrainingPeaks => "TrainingPeaks",
            SyncPlatform::IntervalsIcu => "IntervalsIcu",
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => "HealthKit",
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());

            // First try to update existing record
            match store.update_platform_config(&user_id, platform_name, config.enabled, config.auto_sync) {
                Ok(true) => {
                    tracing::debug!(
                        "Updated platform config in database: {:?} enabled={} auto_sync={}",
                        platform,
                        config.enabled,
                        config.auto_sync
                    );
                }
                Ok(false) => {
                    // No existing record, create a new one
                    let now = Utc::now().to_rfc3339();
                    let sync_record = StoredPlatformSync {
                        id: Uuid::new_v4(),
                        user_id,
                        platform: platform_name.to_string(),
                        is_enabled: config.enabled,
                        auto_upload: config.auto_sync,
                        athlete_id: None,
                        last_sync_at: None,
                        created_at: now.clone(),
                        updated_at: now,
                    };

                    if let Err(e) = store.upsert_platform_sync(&sync_record) {
                        tracing::warn!("Failed to create platform sync record: {}", e);
                    } else {
                        tracing::debug!(
                            "Created platform config in database: {:?} enabled={} auto_sync={}",
                            platform,
                            config.enabled,
                            config.auto_sync
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to update platform config in database: {}", e);
                }
            }
        }
    }

    /// Load platform configuration from the database.
    ///
    /// Returns the platform configuration if found, otherwise returns default config.
    pub fn load_platform_config(
        &self,
        user_id: &Uuid,
        platform: SyncPlatform,
    ) -> PlatformConfig {
        let db_path = match &self.db_path {
            Some(path) => path.clone(),
            None => return PlatformConfig::default(),
        };

        let platform_name = match platform {
            SyncPlatform::Strava => "Strava",
            SyncPlatform::GarminConnect => "GarminConnect",
            SyncPlatform::TrainingPeaks => "TrainingPeaks",
            SyncPlatform::IntervalsIcu => "IntervalsIcu",
            #[cfg(target_os = "macos")]
            SyncPlatform::HealthKit => "HealthKit",
        };

        if let Ok(db) = Database::open(&db_path) {
            let store = SyncStore::new(db.connection());

            match store.get_platform_sync(user_id, platform_name) {
                Ok(Some(sync)) => {
                    tracing::debug!(
                        "Loaded platform config from database: {:?} enabled={} auto_sync={}",
                        platform,
                        sync.is_enabled,
                        sync.auto_upload
                    );
                    return PlatformConfig {
                        enabled: sync.is_enabled,
                        auto_sync: sync.auto_upload,
                    };
                }
                Ok(None) => {
                    tracing::debug!("No platform config found in database for {:?}", platform);
                }
                Err(e) => {
                    tracing::warn!("Failed to load platform config from database: {}", e);
                }
            }
        }

        PlatformConfig::default()
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

/// Create a SyncService with database persistence for upload queue.
///
/// The upload queue will be persisted to the specified database path,
/// allowing pending uploads to survive app restarts.
pub fn create_sync_service_with_db(
    oauth_handler: super::oauth::DefaultOAuthHandler,
    config: SyncConfig,
    db_path: PathBuf,
) -> SyncServiceHandle {
    let credential_store = KeyringCredentialStore::new("RustRide");
    SyncService::spawn_with_db(
        Arc::new(oauth_handler),
        Arc::new(credential_store),
        config,
        Some(db_path),
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

    #[tokio::test]
    async fn test_event_subscription() {
        let oauth_handler = DefaultOAuthHandler::new(8894);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        // Subscribe to events
        let event_rx = handle.subscribe_events().await;
        assert!(event_rx.is_some());

        // Second subscription should return None
        let event_rx2 = handle.subscribe_events().await;
        assert!(event_rx2.is_none());

        handle.shutdown().await.unwrap();
    }

    #[test]
    fn test_token_refresh_constants() {
        // Verify constants are reasonable
        assert!(TOKEN_REFRESH_CHECK_INTERVAL_SECS > 0);
        assert!(TOKEN_REFRESH_CHECK_INTERVAL_SECS <= 300); // At most 5 minutes

        assert!(TOKEN_REFRESH_BUFFER_MINUTES > 0);
        assert!(TOKEN_REFRESH_BUFFER_MINUTES <= 30); // At most 30 minutes

        assert!(MAX_REFRESH_FAILURES >= 1);
        assert!(MAX_REFRESH_FAILURES <= 10);

        assert!(REFRESH_RETRY_DELAY_SECS > 0);
        assert!(REFRESH_RETRY_DELAY_SECS <= 300); // At most 5 minutes
    }

    #[test]
    fn test_token_refresh_state_default() {
        let state = TokenRefreshState::default();
        assert_eq!(state.failure_count, 0);
        assert!(state.last_attempt.is_none());
        assert!(!state.reauth_requested);
    }

    #[test]
    fn test_sync_event_debug() {
        // Verify SyncEvent variants can be formatted
        let event = SyncEvent::TokenRefreshed {
            platform: SyncPlatform::Strava,
            expires_at: Utc::now(),
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("TokenRefreshed"));
        assert!(debug_str.contains("Strava"));

        let event = SyncEvent::TokenRefreshFailed {
            platform: SyncPlatform::Strava,
            error: "Network error".to_string(),
            retry_count: 1,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("TokenRefreshFailed"));
        assert!(debug_str.contains("Network error"));

        let event = SyncEvent::ReauthorizationRequired {
            platform: SyncPlatform::Strava,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ReauthorizationRequired"));
    }

    #[test]
    fn test_sync_event_clone() {
        let event = SyncEvent::TokenRefreshed {
            platform: SyncPlatform::Strava,
            expires_at: Utc::now(),
        };
        let cloned = event.clone();
        match (event, cloned) {
            (
                SyncEvent::TokenRefreshed { platform: p1, .. },
                SyncEvent::TokenRefreshed { platform: p2, .. },
            ) => {
                assert_eq!(p1, p2);
            }
            _ => panic!("Clone produced different variant"),
        }
    }

    // ========== Retry Functionality Tests ==========

    #[test]
    fn test_upload_retry_constants() {
        // Verify retry constants are reasonable
        assert!(MAX_UPLOAD_RETRIES >= 1);
        assert!(MAX_UPLOAD_RETRIES <= 10);

        assert!(BASE_RETRY_DELAY_SECS > 0);
        assert!(BASE_RETRY_DELAY_SECS <= 120); // At most 2 minutes base delay
    }

    #[test]
    fn test_exponential_backoff_formula() {
        // Test the exponential backoff formula used in process_single_upload
        let base = BASE_RETRY_DELAY_SECS;

        // Verify formula produces expected delays
        for retry in 0..MAX_UPLOAD_RETRIES {
            let backoff = base * (2_i64.pow(retry as u32));
            // Backoff should be reasonable (not overflow and stay under 1 hour)
            assert!(backoff > 0);
            assert!(backoff <= 3600, "Backoff {} at retry {} exceeds 1 hour", backoff, retry);
        }
    }

    #[test]
    fn test_upload_retry_queued_event_debug() {
        let event = SyncEvent::UploadRetryQueued {
            record_id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            retry_count: 3,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("UploadRetryQueued"));
        assert!(debug_str.contains("Strava"));
        assert!(debug_str.contains("retry_count: 3"));
    }

    #[test]
    fn test_upload_failed_event_with_retry_info() {
        let event = SyncEvent::UploadFailed {
            record_id: Uuid::new_v4(),
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            error: "Network timeout".to_string(),
            retry_count: 2,
            will_retry: true,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("UploadFailed"));
        assert!(debug_str.contains("will_retry: true"));
        assert!(debug_str.contains("retry_count: 2"));
    }

    #[tokio::test]
    async fn test_retry_upload_not_found() {
        let oauth_handler = DefaultOAuthHandler::new(8895);
        let config = SyncConfig::default();
        let handle = create_sync_service(oauth_handler, config);

        // Try to retry a non-existent upload
        let result = handle.retry_upload(Uuid::new_v4()).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Record not found"));

        handle.shutdown().await.unwrap();
    }

    #[test]
    fn test_manual_retry_limit() {
        // Manual retries are allowed up to 2x the automatic limit
        let max_manual_retries = MAX_UPLOAD_RETRIES * 2;
        assert!(max_manual_retries >= 10);
        assert!(max_manual_retries <= 20);
    }
}
