//! MQTT Client Implementation
//!
//! Provides MQTT broker connection using rumqttc.

use super::{MqttConfig, MqttError, MqttEvent, QoS};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS as RumqttcQoS};
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
}

impl Default for DefaultMqttClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultMqttClient {
    /// Create a new MQTT client
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            config: Arc::new(RwLock::new(None)),
            event_tx,
            client: Arc::new(RwLock::new(None)),
            event_loop_handle: Arc::new(RwLock::new(None)),
            reconnect_enabled: Arc::new(AtomicBool::new(false)),
        }
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
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(event) => {
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
    /// Reconnecting events.
    fn spawn_reconnection_task(
        state: Arc<RwLock<ConnectionState>>,
        config: Arc<RwLock<Option<MqttConfig>>>,
        event_tx: broadcast::Sender<MqttEvent>,
        client: Arc<RwLock<Option<AsyncClient>>>,
        event_loop_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
        reconnect_enabled: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let mut attempt: u32 = 0;

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

                attempt += 1;
                *state.write().await = ConnectionState::Reconnecting { attempt };
                let _ = event_tx.send(MqttEvent::Reconnecting { attempt });

                tracing::info!(
                    "MQTT reconnection attempt {} to {}:{}",
                    attempt,
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

                // TODO (subtask 3.1): Add TLS configuration when use_tls is enabled
                // TODO (subtask 3.2): Add credentials from keyring when username is set

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

impl MqttClient for DefaultMqttClient {
    async fn connect(&self, config: &MqttConfig) -> Result<(), MqttError> {
        if !config.enabled {
            return Err(MqttError::ConfigError("MQTT is disabled".to_string()));
        }

        *self.state.write().await = ConnectionState::Connecting;
        *self.config.write().await = Some(config.clone());

        // Enable auto-reconnection for this connection
        self.reconnect_enabled.store(true, Ordering::SeqCst);

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

        // TODO (subtask 3.1): Add TLS configuration when use_tls is enabled
        // TODO (subtask 3.2): Add credentials from keyring when username is set

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
}
