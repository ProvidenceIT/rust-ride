//! MQTT Client Implementation
//!
//! Provides MQTT broker connection using rumqttc.

use super::{MqttConfig, MqttCredentialStore, MqttError, MqttEvent, QoS};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS as RumqttcQoS, TlsConfiguration, Transport};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// Trait for MQTT client implementations
pub trait MqttClient: Send + Sync {
    /// Connect to the MQTT broker
    fn connect(
        &self,
        config: &MqttConfig,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Disconnect from the broker
    fn disconnect(&self) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Get the current connection state
    fn connection_state(&self) -> ConnectionState;

    /// Publish a message to a topic
    fn publish(
        &self,
        topic: &str,
        payload: &str,
        qos: QoS,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Subscribe to a topic
    fn subscribe(
        &self,
        topic: &str,
        qos: QoS,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Unsubscribe from a topic
    fn unsubscribe(
        &self,
        topic: &str,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Subscribe to connection events
    fn subscribe_events(&self) -> broadcast::Receiver<MqttEvent>;
}

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected and not attempting to connect
    Disconnected,
    /// Attempting initial connection
    Connecting,
    /// Successfully connected to broker
    Connected,
    /// Connection was lost, preparing to reconnect
    ConnectionLost,
    /// Actively attempting reconnection
    Reconnecting { attempt: u32 },
}

/// Default MQTT client implementation
pub struct DefaultMqttClient {
    state: Arc<RwLock<ConnectionState>>,
    config: Arc<RwLock<Option<MqttConfig>>>,
    event_tx: broadcast::Sender<MqttEvent>,
    /// The rumqttc async client for publishing and subscribing
    client: Arc<RwLock<Option<AsyncClient>>>,
    /// Handle to the spawned event loop task (for aborting on disconnect)
    event_loop_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Whether auto-reconnection is enabled (disabled on manual disconnect)
    reconnect_enabled: Arc<AtomicBool>,
    /// Current reconnection attempt counter (shared across reconnection tasks)
    reconnect_attempt: Arc<std::sync::atomic::AtomicU32>,
    /// Credential store for securely retrieving MQTT passwords from OS keyring
    credential_store: Arc<MqttCredentialStore>,
}

impl Default for DefaultMqttClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultMqttClient {
    /// Create a new MQTT client
    pub fn new() -> Self {
        Self::with_credential_store(MqttCredentialStore::new())
    }

    /// Create a new MQTT client with a custom credential store.
    /// Useful for testing or custom keyring service names.
    pub fn with_credential_store(credential_store: MqttCredentialStore) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            config: Arc::new(RwLock::new(None)),
            event_tx,
            client: Arc::new(RwLock::new(None)),
            event_loop_handle: Arc::new(RwLock::new(None)),
            reconnect_enabled: Arc::new(AtomicBool::new(false)),
            reconnect_attempt: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            credential_store: Arc::new(credential_store),
        }
    }

    /// Get a reference to the credential store for password management.
    /// Use this to store or delete passwords in the OS keyring.
    pub fn credential_store(&self) -> &MqttCredentialStore {
        &self.credential_store
    }

    /// Spawn a task to poll the event loop and handle MQTT events.
    /// When a recoverable error occurs and reconnection is enabled, this will
    /// automatically spawn a reconnection attempt.
    fn spawn_event_loop_task(
        mut event_loop: EventLoop,
        state: Arc<RwLock<ConnectionState>>,
        config: Arc<RwLock<Option<MqttConfig>>>,
        event_tx: broadcast::Sender<MqttEvent>,
        client: Arc<RwLock<Option<AsyncClient>>>,
        event_loop_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
        reconnect_enabled: Arc<AtomicBool>,
        reconnect_attempt: Arc<std::sync::atomic::AtomicU32>,
        credential_store: Arc<MqttCredentialStore>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(event) => {
                        // Reset reconnection counter on successful event (connection established)
                        if matches!(event, Event::Incoming(Incoming::ConnAck(_))) {
                            reconnect_attempt.store(0, Ordering::SeqCst);
                        }
                        Self::handle_event(event, &state, &event_tx).await;
                    }
                    Err(e) => {
                        let is_recoverable = Self::is_recoverable_error(&e);
                        Self::handle_connection_error(e, &state, &event_tx).await;

                        // If error is recoverable and reconnection is enabled, start reconnection
                        if is_recoverable && reconnect_enabled.load(Ordering::SeqCst) {
                            Self::spawn_reconnection_task(
                                Arc::clone(&state),
                                Arc::clone(&config),
                                event_tx.clone(),
                                Arc::clone(&client),
                                Arc::clone(&event_loop_handle),
                                Arc::clone(&reconnect_enabled),
                                Arc::clone(&reconnect_attempt),
                                Arc::clone(&credential_store),
                            );
                        }
                        break;
                    }
                }
            }
            tracing::debug!("MQTT event loop task ended");
        })
    }

    /// Handle connection errors from the event loop
    ///
    /// Distinguishes between recoverable connection errors (network issues, timeouts)
    /// and non-recoverable errors (auth failures, protocol errors).
    async fn handle_connection_error(
        error: rumqttc::ConnectionError,
        state: &Arc<RwLock<ConnectionState>>,
        event_tx: &broadcast::Sender<MqttEvent>,
    ) {
        let error_message = format!("{}", error);
        let is_recoverable = Self::is_recoverable_error(&error);

        tracing::error!(
            "MQTT connection error (recoverable={}): {:?}",
            is_recoverable,
            error
        );

        if is_recoverable {
            // Connection was lost but can be recovered - trigger reconnection
            *state.write().await = ConnectionState::ConnectionLost;
            let _ = event_tx.send(MqttEvent::ConnectionLost {
                reason: error_message.clone(),
            });
        } else {
            // Non-recoverable error - stay disconnected
            *state.write().await = ConnectionState::Disconnected;
            let _ = event_tx.send(MqttEvent::Error {
                message: error_message.clone(),
            });
            let _ = event_tx.send(MqttEvent::Disconnected);
        }
    }

    /// Determine if a connection error is recoverable (can be retried)
    fn is_recoverable_error(error: &rumqttc::ConnectionError) -> bool {
        use rumqttc::ConnectionError;

        match error {
            // Network issues are recoverable
            ConnectionError::Io(_) => true,
            // Timeout is recoverable
            ConnectionError::RequestsDone => true,
            // TLS errors might be recoverable (transient network issues)
            #[cfg(feature = "use-rustls")]
            ConnectionError::Tls(_) => true,
            // MQTT protocol errors are generally not recoverable
            ConnectionError::MqttState(state_error) => {
                // Most state errors indicate protocol issues, but some might be recoverable
                tracing::debug!("MQTT state error: {:?}", state_error);
                false
            }
            // Connection refused might be temporary (broker restarting)
            ConnectionError::ConnectionRefused(_) => true,
            // Timeout during connection is recoverable
            ConnectionError::Timeout(_) => true,
            // Flush timeout is recoverable
            ConnectionError::FlushTimeout => true,
            // Network unreachable - might become available later
            ConnectionError::NetworkUnreachable => true,
            // DNS resolution issues might be temporary
            ConnectionError::Resolve(_) => true,
            // Default to not recoverable for unknown errors
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    /// Handle an event from the MQTT event loop
    async fn handle_event(
        event: Event,
        state: &Arc<RwLock<ConnectionState>>,
        event_tx: &broadcast::Sender<MqttEvent>,
    ) {
        match event {
            Event::Incoming(incoming) => {
                Self::handle_incoming(incoming, state, event_tx).await;
            }
            Event::Outgoing(outgoing) => {
                tracing::trace!("MQTT outgoing: {:?}", outgoing);
            }
        }
    }

    /// Handle incoming MQTT packets
    async fn handle_incoming(
        incoming: Incoming,
        state: &Arc<RwLock<ConnectionState>>,
        event_tx: &broadcast::Sender<MqttEvent>,
    ) {
        match incoming {
            Incoming::ConnAck(connack) => {
                if connack.code == rumqttc::ConnectReturnCode::Success {
                    tracing::info!("MQTT connection acknowledged by broker");
                    *state.write().await = ConnectionState::Connected;
                    let _ = event_tx.send(MqttEvent::Connected);
                } else {
                    tracing::error!("MQTT connection rejected: {:?}", connack.code);
                    *state.write().await = ConnectionState::Disconnected;
                    let _ = event_tx.send(MqttEvent::Error {
                        message: format!("Connection rejected: {:?}", connack.code),
                    });
                }
            }
            Incoming::Publish(publish) => {
                let topic = publish.topic.clone();
                let payload = String::from_utf8_lossy(&publish.payload).to_string();
                tracing::debug!("MQTT message received on '{}': {}", topic, payload);
                let _ = event_tx.send(MqttEvent::MessageReceived { topic, payload });
            }
            Incoming::Disconnect => {
                // Broker-initiated disconnect is treated as connection lost (recoverable)
                // This could happen due to:
                // - Broker maintenance/restart
                // - Session timeout
                // - Duplicate client ID
                tracing::warn!("MQTT disconnect received from broker");
                *state.write().await = ConnectionState::ConnectionLost;
                let _ = event_tx.send(MqttEvent::ConnectionLost {
                    reason: "Broker sent disconnect".to_string(),
                });
            }
            Incoming::PingResp => {
                tracing::trace!("MQTT ping response received");
            }
            _ => {
                tracing::trace!("MQTT incoming event: {:?}", incoming);
            }
        }
    }

    /// Spawn a reconnection task that will attempt to reconnect to the broker.
    /// The task will wait for the configured reconnection interval, then attempt
    /// to create a new connection. It tracks reconnection attempts and emits
    /// Reconnecting events. Gives up after max_reconnect_attempts if configured.
    fn spawn_reconnection_task(
        state: Arc<RwLock<ConnectionState>>,
        config: Arc<RwLock<Option<MqttConfig>>>,
        event_tx: broadcast::Sender<MqttEvent>,
        client: Arc<RwLock<Option<AsyncClient>>>,
        event_loop_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
        reconnect_enabled: Arc<AtomicBool>,
        reconnect_attempt: Arc<std::sync::atomic::AtomicU32>,
        credential_store: Arc<MqttCredentialStore>,
    ) {
        tokio::spawn(async move {
            loop {
                // Check if reconnection is still enabled (user might disconnect manually)
                if !reconnect_enabled.load(Ordering::SeqCst) {
                    tracing::info!("MQTT reconnection disabled, stopping reconnection attempts");
                    *state.write().await = ConnectionState::Disconnected;
                    let _ = event_tx.send(MqttEvent::Disconnected);
                    break;
                }

                // Get the config, bail if it's been cleared
                let cfg = match config.read().await.clone() {
                    Some(cfg) => cfg,
                    None => {
                        tracing::warn!("MQTT config cleared, stopping reconnection");
                        *state.write().await = ConnectionState::Disconnected;
                        let _ = event_tx.send(MqttEvent::Disconnected);
                        break;
                    }
                };

                // Increment attempt counter
                let attempt = reconnect_attempt.fetch_add(1, Ordering::SeqCst) + 1;

                // Check if we've exceeded max reconnection attempts
                if let Some(max_attempts) = cfg.max_reconnect_attempts {
                    if attempt > max_attempts {
                        tracing::error!(
                            "MQTT reconnection failed after {} attempts (max: {})",
                            attempt - 1,
                            max_attempts
                        );
                        reconnect_enabled.store(false, Ordering::SeqCst);
                        *state.write().await = ConnectionState::Disconnected;
                        let _ = event_tx.send(MqttEvent::ReconnectionFailed {
                            attempts: attempt - 1,
                            reason: format!(
                                "Exceeded maximum reconnection attempts ({})",
                                max_attempts
                            ),
                        });
                        let _ = event_tx.send(MqttEvent::Disconnected);
                        break;
                    }
                }

                *state.write().await = ConnectionState::Reconnecting { attempt };
                let _ = event_tx.send(MqttEvent::Reconnecting { attempt });

                let max_info = cfg
                    .max_reconnect_attempts
                    .map(|m| format!(" of {}", m))
                    .unwrap_or_default();
                tracing::info!(
                    "MQTT reconnection attempt {}{} to {}:{}",
                    attempt,
                    max_info,
                    cfg.broker_host,
                    cfg.broker_port
                );

                // Wait for the configured reconnection interval before attempting
                let delay = Duration::from_secs(cfg.reconnect_interval_secs as u64);
                tokio::time::sleep(delay).await;

                // Check again if reconnection is still enabled after sleeping
                if !reconnect_enabled.load(Ordering::SeqCst) {
                    tracing::info!("MQTT reconnection disabled during delay, stopping");
                    *state.write().await = ConnectionState::Disconnected;
                    let _ = event_tx.send(MqttEvent::Disconnected);
                    break;
                }

                // Create new MQTT options and client
                let mut mqtt_options = MqttOptions::new(
                    &cfg.client_id,
                    &cfg.broker_host,
                    cfg.broker_port,
                );
                mqtt_options.set_keep_alive(Duration::from_secs(cfg.keep_alive_secs as u64));
                mqtt_options.set_connection_timeout(cfg.connection_timeout_secs.into());

                // Configure TLS when enabled (subtask 3.1)
                configure_tls(&mut mqtt_options, cfg.use_tls);

                // Configure credentials from keyring when username is set (subtask 3.2)
                configure_credentials(&mut mqtt_options, &cfg, &credential_store);

                // Create new AsyncClient and EventLoop
                let (new_client, eventloop) = AsyncClient::new(mqtt_options, 10);

                // Store the new client
                *client.write().await = Some(new_client);

                // Update state to connecting for this attempt
                *state.write().await = ConnectionState::Connecting;

                // Spawn new event loop task
                let handle = Self::spawn_event_loop_task(
                    eventloop,
                    Arc::clone(&state),
                    Arc::clone(&config),
                    event_tx.clone(),
                    Arc::clone(&client),
                    Arc::clone(&event_loop_handle),
                    Arc::clone(&reconnect_enabled),
                    Arc::clone(&reconnect_attempt),
                    Arc::clone(&credential_store),
                );
                *event_loop_handle.write().await = Some(handle);

                tracing::debug!("MQTT reconnection attempt {} initiated", attempt);

                // The event loop task will handle connection success/failure.
                // If it fails with a recoverable error, it will spawn another
                // reconnection task automatically, so we can exit this task.
                break;
            }
        });
    }
}

/// Convert our QoS enum to rumqttc's QoS enum
fn qos_to_rumqttc(qos: QoS) -> RumqttcQoS {
    match qos {
        QoS::AtMostOnce => RumqttcQoS::AtMostOnce,
        QoS::AtLeastOnce => RumqttcQoS::AtLeastOnce,
        QoS::ExactlyOnce => RumqttcQoS::ExactlyOnce,
    }
}

/// Configure TLS on MqttOptions when enabled
///
/// Uses the system's native root certificates for server verification.
/// This is appropriate for connecting to public MQTT brokers with valid certificates.
fn configure_tls(mqtt_options: &mut MqttOptions, use_tls: bool) {
    if use_tls {
        // Use TlsConfiguration::Simple which uses rustls with system-native CA certificates
        // This provides secure TLS without requiring custom certificates
        let tls_config = TlsConfiguration::default();
        mqtt_options.set_transport(Transport::Tls(tls_config));
        tracing::debug!("TLS enabled for MQTT connection");
    }
}

/// Configure credentials on MqttOptions when username is set.
///
/// Retrieves the password from the OS keyring for the given username and host.
/// If no password is stored or username is not set, no credentials are configured.
fn configure_credentials(
    mqtt_options: &mut MqttOptions,
    config: &MqttConfig,
    credential_store: &MqttCredentialStore,
) {
    if let Some(username) = &config.username {
        if username.is_empty() {
            return;
        }

        match credential_store.get_password(username, &config.broker_host) {
            Ok(Some(password)) => {
                mqtt_options.set_credentials(username, &password);
                tracing::debug!(
                    "Set MQTT credentials for user '{}' on broker '{}'",
                    username,
                    config.broker_host
                );
            }
            Ok(None) => {
                // No password stored - set username only (some brokers allow this)
                tracing::warn!(
                    "No password found in keyring for MQTT user '{}@{}', connecting without password",
                    username,
                    config.broker_host
                );
            }
            Err(e) => {
                // Log the error but don't fail - try to connect without credentials
                tracing::error!(
                    "Failed to retrieve MQTT password from keyring: {}. Connecting without credentials.",
                    e
                );
            }
        }
    }
}

impl MqttClient for DefaultMqttClient {
    async fn connect(&self, config: &MqttConfig) -> Result<(), MqttError> {
        if !config.enabled {
            return Err(MqttError::ConfigError("MQTT is disabled".to_string()));
        }

        *self.state.write().await = ConnectionState::Connecting;
        *self.config.write().await = Some(config.clone());

        // Enable auto-reconnection for this connection and reset attempt counter
        self.reconnect_enabled.store(true, Ordering::SeqCst);
        self.reconnect_attempt.store(0, Ordering::SeqCst);

        tracing::info!(
            "Connecting to MQTT broker at {}:{}",
            config.broker_host,
            config.broker_port
        );

        // Create MQTT options with broker configuration
        let mut mqtt_options = MqttOptions::new(
            &config.client_id,
            &config.broker_host,
            config.broker_port,
        );
        mqtt_options.set_keep_alive(Duration::from_secs(config.keep_alive_secs as u64));
        mqtt_options.set_connection_timeout(config.connection_timeout_secs.into());

        // Configure TLS when enabled (subtask 3.1)
        configure_tls(&mut mqtt_options, config.use_tls);

        // Configure credentials from keyring when username is set (subtask 3.2)
        configure_credentials(&mut mqtt_options, config, &self.credential_store);

        // Create the AsyncClient and EventLoop
        // Buffer size of 10 is sufficient for fan control operations
        let (client, eventloop) = AsyncClient::new(mqtt_options, 10);

        // Store the client for publishing/subscribing
        *self.client.write().await = Some(client);

        // Spawn the event loop task to poll for MQTT events
        // The actual connection is established when the event loop starts polling
        let handle = Self::spawn_event_loop_task(
            eventloop,
            Arc::clone(&self.state),
            Arc::clone(&self.config),
            self.event_tx.clone(),
            Arc::clone(&self.client),
            Arc::clone(&self.event_loop_handle),
            Arc::clone(&self.reconnect_enabled),
            Arc::clone(&self.reconnect_attempt),
            Arc::clone(&self.credential_store),
        );
        *self.event_loop_handle.write().await = Some(handle);

        tracing::info!("MQTT event loop started, waiting for connection...");

        Ok(())
    }

    async fn disconnect(&self) -> Result<(), MqttError> {
        // Disable auto-reconnection first to prevent reconnection tasks from running
        self.reconnect_enabled.store(false, Ordering::SeqCst);

        // Abort the event loop task if running
        if let Some(handle) = self.event_loop_handle.write().await.take() {
            handle.abort();
            tracing::debug!("MQTT event loop task aborted");
        }

        // Clear the client
        *self.client.write().await = None;

        *self.state.write().await = ConnectionState::Disconnected;
        *self.config.write().await = None;

        let _ = self.event_tx.send(MqttEvent::Disconnected);

        tracing::info!("Disconnected from MQTT broker");

        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Use try_read to avoid blocking
        if let Ok(state) = self.state.try_read() {
            matches!(*state, ConnectionState::Connected)
        } else {
            false
        }
    }

    fn connection_state(&self) -> ConnectionState {
        // Use try_read to avoid blocking, default to Disconnected if lock unavailable
        if let Ok(state) = self.state.try_read() {
            state.clone()
        } else {
            ConnectionState::Disconnected
        }
    }

    async fn publish(&self, topic: &str, payload: &str, qos: QoS) -> Result<(), MqttError> {
        if !self.is_connected() {
            return Err(MqttError::NotConnected);
        }

        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(MqttError::NotConnected)?;

        tracing::debug!("Publishing to {}: {}", topic, payload);

        // Convert our QoS to rumqttc QoS
        let rumqttc_qos = qos_to_rumqttc(qos);

        // Publish using rumqttc AsyncClient
        // retain=false: don't retain the message on the broker
        client
            .publish(topic, rumqttc_qos, false, payload.as_bytes())
            .await
            .map_err(|e| MqttError::PublishFailed(e.to_string()))?;

        Ok(())
    }

    async fn subscribe(&self, topic: &str, qos: QoS) -> Result<(), MqttError> {
        if !self.is_connected() {
            return Err(MqttError::NotConnected);
        }

        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(MqttError::NotConnected)?;

        tracing::debug!("Subscribing to {} with QoS {:?}", topic, qos);

        // Convert our QoS to rumqttc QoS
        let rumqttc_qos = qos_to_rumqttc(qos);

        // Subscribe using rumqttc AsyncClient
        client
            .subscribe(topic, rumqttc_qos)
            .await
            .map_err(|e| MqttError::SubscribeFailed(e.to_string()))?;

        Ok(())
    }

    async fn unsubscribe(&self, topic: &str) -> Result<(), MqttError> {
        if !self.is_connected() {
            return Err(MqttError::NotConnected);
        }

        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(MqttError::NotConnected)?;

        tracing::debug!("Unsubscribing from {}", topic);

        // Unsubscribe using rumqttc AsyncClient
        client
            .unsubscribe(topic)
            .await
            .map_err(|e| MqttError::SubscribeFailed(format!("Unsubscribe failed: {}", e)))?;

        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<MqttEvent> {
        self.event_tx.subscribe()
    }
}

/// Result of a connection test
#[derive(Debug, Clone)]
pub struct MqttTestResult {
    /// Whether the connection was successful
    pub success: bool,
    /// Message describing the result
    pub message: String,
    /// How long the test took
    pub duration_ms: u64,
}

/// Test an MQTT connection without affecting the main client state.
///
/// This function creates a temporary connection to the broker, waits for
/// a connection acknowledgment or timeout, then disconnects and returns the result.
/// It's designed to be used from the settings UI to validate configuration.
pub async fn test_mqtt_connection(config: &MqttConfig) -> MqttTestResult {
    use std::time::Instant;

    let start = Instant::now();

    if !config.enabled {
        return MqttTestResult {
            success: false,
            message: "MQTT is disabled in configuration".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    if config.broker_host.is_empty() {
        return MqttTestResult {
            success: false,
            message: "Broker host is not configured".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    // Create test client ID (unique for this test)
    let test_client_id = format!("{}-test-{}", config.client_id, uuid::Uuid::new_v4().as_simple());

    // Create MQTT options for the test connection
    let mut mqtt_options = MqttOptions::new(
        &test_client_id,
        &config.broker_host,
        config.broker_port,
    );
    mqtt_options.set_keep_alive(Duration::from_secs(config.keep_alive_secs as u64));

    // Use a shorter timeout for testing (max 10 seconds)
    let timeout_secs = std::cmp::min(config.connection_timeout_secs, 10);
    mqtt_options.set_connection_timeout(timeout_secs.into());

    // Configure TLS when enabled
    configure_tls(&mut mqtt_options, config.use_tls);

    // Configure credentials from keyring when username is set
    let credential_store = MqttCredentialStore::new();
    configure_credentials(&mut mqtt_options, config, &credential_store);

    tracing::info!(
        "Testing MQTT connection to {}:{} (timeout: {}s)",
        config.broker_host,
        config.broker_port,
        timeout_secs
    );

    // Create the test client and event loop
    let (_client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Poll the event loop with a timeout to wait for connection
    let timeout_duration = Duration::from_secs(timeout_secs as u64);

    match tokio::time::timeout(timeout_duration, async {
        // Poll events until we get a ConnAck or an error
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(connack))) => {
                    if connack.code == rumqttc::ConnectReturnCode::Success {
                        return Ok("Connection successful");
                    } else {
                        return Err(format!("Connection rejected: {:?}", connack.code));
                    }
                }
                Ok(_event) => {
                    // Continue polling for ConnAck
                    continue;
                }
                Err(e) => {
                    return Err(format!("Connection error: {}", e));
                }
            }
        }
    })
    .await
    {
        Ok(Ok(msg)) => {
            tracing::info!("MQTT test connection succeeded");
            MqttTestResult {
                success: true,
                message: msg.to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        Ok(Err(error_msg)) => {
            tracing::warn!("MQTT test connection failed: {}", error_msg);
            MqttTestResult {
                success: false,
                message: error_msg,
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        Err(_) => {
            tracing::warn!("MQTT test connection timed out after {}s", timeout_secs);
            MqttTestResult {
                success: false,
                message: format!(
                    "Connection timed out after {}s. Check broker address and port.",
                    timeout_secs
                ),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
    }
    // Note: The test client is dropped here, which will clean up the connection
}

/// Result of a fan test cycle.
#[derive(Debug, Clone)]
pub struct FanTestResult {
    /// Whether the test completed successfully
    pub success: bool,
    /// Status message
    pub message: String,
    /// Duration of the test in milliseconds
    pub duration_ms: u64,
    /// Current speed being tested (for progress updates)
    pub current_speed: u8,
}

/// Callback type for fan test progress updates.
pub type FanTestProgressCallback = Box<dyn Fn(u8) + Send + Sync>;

/// Test a fan by cycling through speeds without affecting the main client state.
///
/// This function creates a temporary MQTT connection, cycles through fan speeds
/// [25, 50, 75, 100, 50, 0] with delays between each, then disconnects.
/// It's designed to be used from the settings UI to validate fan configuration.
///
/// # Arguments
/// * `config` - MQTT broker configuration
/// * `profile` - Fan profile with topic and payload format
/// * `progress_callback` - Optional callback for progress updates (receives current speed)
pub async fn test_fan(
    config: &MqttConfig,
    profile: &super::fan::FanProfile,
    progress_callback: Option<FanTestProgressCallback>,
) -> FanTestResult {
    use std::time::Instant;

    let start = Instant::now();

    if !config.enabled {
        return FanTestResult {
            success: false,
            message: "MQTT is disabled in configuration".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            current_speed: 0,
        };
    }

    if config.broker_host.is_empty() {
        return FanTestResult {
            success: false,
            message: "Broker host is not configured".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            current_speed: 0,
        };
    }

    if profile.mqtt_topic.is_empty() {
        return FanTestResult {
            success: false,
            message: "Fan MQTT topic is not configured".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            current_speed: 0,
        };
    }

    // Create test client ID (unique for this test)
    let test_client_id = format!("{}-fantest-{}", config.client_id, uuid::Uuid::new_v4().as_simple());

    // Create MQTT options for the test connection
    let mut mqtt_options = MqttOptions::new(
        &test_client_id,
        &config.broker_host,
        config.broker_port,
    );
    mqtt_options.set_keep_alive(Duration::from_secs(config.keep_alive_secs as u64));

    // Use a shorter timeout for testing (max 10 seconds)
    let timeout_secs = std::cmp::min(config.connection_timeout_secs, 10);
    mqtt_options.set_connection_timeout(timeout_secs.into());

    // Configure TLS when enabled
    configure_tls(&mut mqtt_options, config.use_tls);

    // Configure credentials from keyring when username is set
    let credential_store = MqttCredentialStore::new();
    configure_credentials(&mut mqtt_options, config, &credential_store);

    tracing::info!(
        "Testing fan '{}' on topic '{}' via {}:{}",
        profile.name,
        profile.mqtt_topic,
        config.broker_host,
        config.broker_port
    );

    // Create the test client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // First, connect to the broker
    let connect_timeout = Duration::from_secs(timeout_secs as u64);
    let connect_result = tokio::time::timeout(connect_timeout, async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(connack))) => {
                    if connack.code == rumqttc::ConnectReturnCode::Success {
                        return Ok(());
                    } else {
                        return Err(format!("Connection rejected: {:?}", connack.code));
                    }
                }
                Ok(_event) => continue,
                Err(e) => return Err(format!("Connection error: {}", e)),
            }
        }
    })
    .await;

    match connect_result {
        Ok(Ok(())) => {
            tracing::info!("Connected to MQTT broker for fan test");
        }
        Ok(Err(e)) => {
            return FanTestResult {
                success: false,
                message: e,
                duration_ms: start.elapsed().as_millis() as u64,
                current_speed: 0,
            };
        }
        Err(_) => {
            return FanTestResult {
                success: false,
                message: format!("Connection timed out after {}s", timeout_secs),
                duration_ms: start.elapsed().as_millis() as u64,
                current_speed: 0,
            };
        }
    }

    // Cycle through speeds: 25, 50, 75, 100, 50, 0
    let speeds: [u8; 6] = [25, 50, 75, 100, 50, 0];
    let topic = profile.command_topic();

    for speed in speeds.iter() {
        // Notify progress callback
        if let Some(ref callback) = progress_callback {
            callback(*speed);
        }

        // Format and publish the speed command
        let is_on = *speed > 0;
        let payload = profile.format_payload(*speed, is_on);

        tracing::debug!("Fan test: setting speed to {}% (topic: {}, payload: {})", speed, topic, payload);

        if let Err(e) = client.publish(&topic, RumqttcQoS::AtLeastOnce, false, payload.as_bytes()).await {
            return FanTestResult {
                success: false,
                message: format!("Failed to publish speed {}: {}", speed, e),
                duration_ms: start.elapsed().as_millis() as u64,
                current_speed: *speed,
            };
        }

        // Poll the event loop to process the publish
        // Give it a short time to process the outgoing message
        let poll_result = tokio::time::timeout(Duration::from_millis(500), eventloop.poll()).await;
        if let Ok(Err(e)) = poll_result {
            tracing::warn!("Event loop error during fan test: {}", e);
            // Continue anyway, the publish may have still succeeded
        }

        // Wait 2 seconds before changing speed (unless this is the last speed)
        if *speed != 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    tracing::info!("Fan test completed successfully for '{}'", profile.name);

    FanTestResult {
        success: true,
        message: format!("Fan test completed for '{}'", profile.name),
        duration_ms: start.elapsed().as_millis() as u64,
        current_speed: 0,
    }
    // Note: The test client is dropped here, which will clean up the connection
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = DefaultMqttClient::new();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_qos_conversion() {
        assert!(matches!(qos_to_rumqttc(QoS::AtMostOnce), RumqttcQoS::AtMostOnce));
        assert!(matches!(qos_to_rumqttc(QoS::AtLeastOnce), RumqttcQoS::AtLeastOnce));
        assert!(matches!(qos_to_rumqttc(QoS::ExactlyOnce), RumqttcQoS::ExactlyOnce));
    }

    #[tokio::test]
    async fn test_connect_disabled() {
        let client = DefaultMqttClient::new();
        let config = MqttConfig {
            enabled: false,
            ..Default::default()
        };

        let result = client.connect(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_publish_not_connected() {
        let client = DefaultMqttClient::new();
        let result = client.publish("test", "payload", QoS::AtMostOnce).await;
        assert!(matches!(result, Err(MqttError::NotConnected)));
    }

    #[tokio::test]
    async fn test_subscribe_not_connected() {
        let client = DefaultMqttClient::new();
        let result = client.subscribe("test/topic", QoS::AtLeastOnce).await;
        assert!(matches!(result, Err(MqttError::NotConnected)));
    }

    #[tokio::test]
    async fn test_unsubscribe_not_connected() {
        let client = DefaultMqttClient::new();
        let result = client.unsubscribe("test/topic").await;
        assert!(matches!(result, Err(MqttError::NotConnected)));
    }

    #[test]
    fn test_connection_state_initial() {
        let client = DefaultMqttClient::new();
        assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_enum_equality() {
        // Verify ConnectionState variants are distinguishable
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Disconnected, ConnectionState::ConnectionLost);
        assert_ne!(ConnectionState::Connected, ConnectionState::Connecting);
        assert_eq!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 1 }
        );
        assert_ne!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 2 }
        );
    }

    #[test]
    fn test_is_recoverable_error_io() {
        use std::io::{Error as IoError, ErrorKind};
        let io_error = IoError::new(ErrorKind::ConnectionReset, "connection reset");
        let error = rumqttc::ConnectionError::Io(io_error);
        assert!(DefaultMqttClient::is_recoverable_error(&error));
    }

    #[test]
    fn test_is_recoverable_error_timeout() {
        let error = rumqttc::ConnectionError::FlushTimeout;
        assert!(DefaultMqttClient::is_recoverable_error(&error));
    }

    #[test]
    fn test_is_recoverable_error_network_unreachable() {
        let error = rumqttc::ConnectionError::NetworkUnreachable;
        assert!(DefaultMqttClient::is_recoverable_error(&error));
    }

    #[test]
    fn test_reconnect_enabled_initial_false() {
        let client = DefaultMqttClient::new();
        // Auto-reconnection is disabled by default
        assert!(!client.reconnect_enabled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_disconnect_disables_reconnection() {
        let client = DefaultMqttClient::new();
        // Manually enable reconnection
        client.reconnect_enabled.store(true, Ordering::SeqCst);
        assert!(client.reconnect_enabled.load(Ordering::SeqCst));

        // Disconnect should disable it
        let _ = client.disconnect().await;
        assert!(!client.reconnect_enabled.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reconnect_attempt_counter_initial_zero() {
        let client = DefaultMqttClient::new();
        // Reconnection attempt counter should start at 0
        assert_eq!(client.reconnect_attempt.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_config_connection_timeout_default() {
        let config = MqttConfig::default();
        // Default connection timeout should be 30 seconds
        assert_eq!(config.connection_timeout_secs, 30);
    }

    #[test]
    fn test_config_max_reconnect_attempts_default() {
        let config = MqttConfig::default();
        // Default max reconnection attempts should be None (unlimited)
        assert!(config.max_reconnect_attempts.is_none());
    }

    #[test]
    fn test_config_max_reconnect_attempts_set() {
        let config = MqttConfig {
            max_reconnect_attempts: Some(5),
            ..Default::default()
        };
        assert_eq!(config.max_reconnect_attempts, Some(5));
    }

    #[test]
    fn test_config_use_tls_default_false() {
        let config = MqttConfig::default();
        // TLS should be disabled by default
        assert!(!config.use_tls);
    }

    #[test]
    fn test_config_use_tls_enabled() {
        let config = MqttConfig {
            use_tls: true,
            broker_port: 8883, // Standard TLS port
            ..Default::default()
        };
        assert!(config.use_tls);
        assert_eq!(config.broker_port, 8883);
    }

    #[test]
    fn test_configure_tls_disabled() {
        // When TLS is disabled, MqttOptions should not have transport set
        let mut mqtt_options = MqttOptions::new("test-client", "localhost", 1883);
        configure_tls(&mut mqtt_options, false);
        // No panic or error means the function works correctly when TLS is disabled
        // We can't easily inspect the internal transport state, but the function should not panic
    }

    #[test]
    fn test_configure_tls_enabled() {
        // When TLS is enabled, MqttOptions should have TLS transport configured
        let mut mqtt_options = MqttOptions::new("test-client", "localhost", 8883);
        configure_tls(&mut mqtt_options, true);
        // No panic or error means the function works correctly when TLS is enabled
        // The actual TLS connection would be tested in integration tests
    }

    #[test]
    fn test_client_has_credential_store() {
        let client = DefaultMqttClient::new();
        // Client should have a credential store accessible
        let store = client.credential_store();
        // Just verify we can access it (the store itself is tested in mod.rs)
        assert_eq!(store.service_name, "RustRide-MQTT");
    }

    #[test]
    fn test_client_with_custom_credential_store() {
        let custom_store = MqttCredentialStore::with_service_name("CustomService");
        let client = DefaultMqttClient::with_credential_store(custom_store);
        assert_eq!(client.credential_store().service_name, "CustomService");
    }

    #[test]
    fn test_configure_credentials_no_username() {
        // When no username is set, credentials should not be configured
        let mut mqtt_options = MqttOptions::new("test-client", "localhost", 1883);
        let config = MqttConfig {
            username: None,
            ..Default::default()
        };
        let store = MqttCredentialStore::new();

        // Should not panic when no username is set
        configure_credentials(&mut mqtt_options, &config, &store);
        // Function completes without error
    }

    #[test]
    fn test_configure_credentials_empty_username() {
        // When username is empty, credentials should not be configured
        let mut mqtt_options = MqttOptions::new("test-client", "localhost", 1883);
        let config = MqttConfig {
            username: Some("".to_string()),
            ..Default::default()
        };
        let store = MqttCredentialStore::new();

        // Should not panic with empty username
        configure_credentials(&mut mqtt_options, &config, &store);
        // Function completes without error
    }

    #[test]
    fn test_configure_credentials_no_password_stored() {
        // When username is set but no password is stored, should log warning but not panic
        let mut mqtt_options = MqttOptions::new("test-client", "localhost", 1883);
        let config = MqttConfig {
            username: Some("testuser".to_string()),
            broker_host: "test-broker.local".to_string(),
            ..Default::default()
        };
        // Use a unique service name to avoid actual keyring access affecting other tests
        let store = MqttCredentialStore::with_service_name("RustRide-Test-NoPassword");

        // Should not panic when no password is stored
        // The function logs a warning but continues
        configure_credentials(&mut mqtt_options, &config, &store);
        // Function completes without error
    }
}
