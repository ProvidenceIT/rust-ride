//! WebSocket server for companion app connectivity.
//!
//! Implements a WebSocket server using tokio-tungstenite that listens
//! for connections from mobile companion apps over LAN.
//!
//! ## Features
//!
//! - Binds to 0.0.0.0 for LAN accessibility
//! - Supports multiple concurrent client connections
//! - Optional PIN-based authentication
//! - Real-time metrics broadcasting at 1Hz
//! - mDNS service advertisement for auto-discovery
//!
//! ## Implementation Details
//!
//! The server runs as a separate async task and communicates with the
//! main application via channels. Authenticated clients receive metrics
//! updates and can send control commands.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use chrono::Utc;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

use super::discovery::CompanionMdnsAdvertiser;
use super::handlers::handle_request;
use super::qr::{CompanionQrCode, QrCodeError};
use super::streaming::{MetricsStreamer, MetricsStreamerConfig};
use super::types::{
    CompanionClient, CompanionConfig, CompanionError, CompanionErrorCode, CompanionEvent,
    CompanionRequest, CompanionResponse,
};

/// Channel capacity for client message queues.
const CLIENT_CHANNEL_CAPACITY: usize = 64;

/// Shutdown signal for the server.
#[derive(Debug, Clone)]
enum ServerCommand {
    /// Shutdown the server gracefully.
    Shutdown,
    /// Disconnect a specific client.
    DisconnectClient(Uuid),
}

/// Internal client state maintained by the server.
#[derive(Debug)]
struct ClientState {
    /// Session ID for this client.
    session_id: Uuid,
    /// When the client connected.
    connected_at: chrono::DateTime<Utc>,
    /// Client IP address.
    remote_addr: SocketAddr,
    /// Whether the client is authenticated.
    is_authenticated: bool,
    /// Whether the client is subscribed to metrics.
    subscribed_to_metrics: bool,
    /// Channel to send messages to this client.
    tx: mpsc::Sender<CompanionResponse>,
}

impl ClientState {
    /// Convert to the public CompanionClient type.
    fn to_companion_client(&self) -> CompanionClient {
        CompanionClient {
            session_id: self.session_id,
            connected_at: self.connected_at.to_rfc3339(),
            remote_addr: self.remote_addr.to_string(),
            is_authenticated: self.is_authenticated,
            subscribed_to_metrics: self.subscribed_to_metrics,
        }
    }
}

/// The companion WebSocket server.
///
/// Manages client connections and coordinates message passing between
/// the desktop application and connected mobile companions.
pub struct CompanionServer {
    /// Server configuration.
    config: CompanionConfig,
    /// Whether the server is currently running.
    is_running: Arc<RwLock<bool>>,
    /// Connected clients by session ID.
    clients: Arc<RwLock<HashMap<Uuid, ClientState>>>,
    /// Current PIN for authentication (if enabled).
    current_pin: Arc<RwLock<Option<String>>>,
    /// Channel for broadcasting events to clients.
    event_tx: broadcast::Sender<CompanionEvent>,
    /// Server's listening address (when running).
    server_addr: Arc<RwLock<Option<SocketAddr>>>,
    /// Channel to send commands to the server task.
    command_tx: Arc<RwLock<Option<mpsc::Sender<ServerCommand>>>>,
    /// mDNS service advertiser for auto-discovery.
    mdns_advertiser: CompanionMdnsAdvertiser,
    /// Optional daemon state for workout control integration.
    daemon_state: Option<Arc<RwLock<super::state::DaemonState>>>,
}

impl CompanionServer {
    /// Create a new companion server with the given configuration.
    pub fn new(config: CompanionConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            current_pin: Arc::new(RwLock::new(None)),
            event_tx,
            server_addr: Arc::new(RwLock::new(None)),
            command_tx: Arc::new(RwLock::new(None)),
            mdns_advertiser: CompanionMdnsAdvertiser::new(),
            daemon_state: None,
        }
    }

    /// Create a new companion server with daemon state integration.
    ///
    /// This constructor enables workout control commands (pause, resume, skip, stop)
    /// to directly interact with the daemon's session state.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `daemon_state` - Shared daemon state for workout control
    pub fn with_daemon_state(
        config: CompanionConfig,
        daemon_state: Arc<RwLock<super::state::DaemonState>>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            config,
            is_running: Arc::new(RwLock::new(false)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            current_pin: Arc::new(RwLock::new(None)),
            event_tx,
            server_addr: Arc::new(RwLock::new(None)),
            command_tx: Arc::new(RwLock::new(None)),
            mdns_advertiser: CompanionMdnsAdvertiser::new(),
            daemon_state: Some(daemon_state),
        }
    }

    /// Set the daemon state for workout control integration.
    ///
    /// This enables workout control commands to directly interact with
    /// the daemon's session state.
    pub fn set_daemon_state(&mut self, daemon_state: Arc<RwLock<super::state::DaemonState>>) {
        self.daemon_state = Some(daemon_state);
    }

    /// Get the daemon state if available.
    pub fn daemon_state(&self) -> Option<Arc<RwLock<super::state::DaemonState>>> {
        self.daemon_state.clone()
    }

    /// Start the companion server.
    ///
    /// Binds to the configured port and begins accepting connections.
    /// This method returns immediately; the server runs in background tasks.
    pub async fn start(&self) -> Result<(), CompanionError> {
        if !self.config.enabled {
            return Err(CompanionError::ServerNotRunning);
        }

        if *self.is_running.read().await {
            return Ok(()); // Already running
        }

        // Generate initial PIN if required
        if self.config.require_pin {
            let pin = Self::generate_pin();
            *self.current_pin.write().await = Some(pin.clone());
            tracing::info!("Companion server PIN: {}", pin);
        }

        // Bind to 0.0.0.0 for LAN accessibility
        let addr = format!("0.0.0.0:{}", self.config.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| CompanionError::BindFailed(self.config.port, e.to_string()))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| CompanionError::BindFailed(self.config.port, e.to_string()))?;

        *self.server_addr.write().await = Some(local_addr);
        *self.is_running.write().await = true;

        // Create command channel for server control
        let (command_tx, command_rx) = mpsc::channel(16);
        *self.command_tx.write().await = Some(command_tx);

        // Clone state for the accept loop
        let clients = Arc::clone(&self.clients);
        let current_pin = Arc::clone(&self.current_pin);
        let is_running = Arc::clone(&self.is_running);
        let event_tx = self.event_tx.clone();
        let max_connections = self.config.max_connections;
        let require_pin = self.config.require_pin;
        let daemon_state = self.daemon_state.clone();

        // Spawn the main accept loop
        tokio::spawn(async move {
            Self::run_accept_loop(
                listener,
                command_rx,
                clients,
                current_pin,
                is_running,
                event_tx,
                max_connections,
                require_pin,
                daemon_state,
            )
            .await;
        });

        tracing::info!("Companion server started on {}", local_addr);

        // Start mDNS advertisement for auto-discovery
        if let Err(e) = self.mdns_advertiser.start(local_addr.port(), None).await {
            tracing::warn!("Failed to start mDNS advertisement: {}. Server will still work but won't be auto-discoverable.", e);
        }

        Ok(())
    }

    /// Run the main accept loop for incoming connections.
    async fn run_accept_loop(
        listener: TcpListener,
        mut command_rx: mpsc::Receiver<ServerCommand>,
        clients: Arc<RwLock<HashMap<Uuid, ClientState>>>,
        current_pin: Arc<RwLock<Option<String>>>,
        is_running: Arc<RwLock<bool>>,
        event_tx: broadcast::Sender<CompanionEvent>,
        max_connections: u8,
        require_pin: bool,
        daemon_state: Option<Arc<RwLock<super::state::DaemonState>>>,
    ) {
        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            // Check max connections
                            let current_count = clients.read().await.len();
                            if current_count >= max_connections as usize {
                                tracing::warn!(
                                    "Rejecting connection from {}: max connections ({}) reached",
                                    addr,
                                    max_connections
                                );
                                continue;
                            }

                            tracing::info!("New companion connection from {}", addr);

                            // Clone state for this connection
                            let clients = Arc::clone(&clients);
                            let current_pin = Arc::clone(&current_pin);
                            let event_tx = event_tx.clone();
                            let daemon_state = daemon_state.clone();

                            // Spawn a task to handle this client
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    stream,
                                    addr,
                                    clients,
                                    current_pin,
                                    event_tx,
                                    require_pin,
                                    daemon_state,
                                ).await {
                                    tracing::error!("Client {} error: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                // Handle server commands
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        ServerCommand::Shutdown => {
                            tracing::info!("Companion server shutting down");
                            break;
                        }
                        ServerCommand::DisconnectClient(session_id) => {
                            if let Some(client) = clients.write().await.remove(&session_id) {
                                // The client will be disconnected when its task notices
                                // the channel is closed
                                drop(client.tx);
                            }
                        }
                    }
                }
            }
        }

        *is_running.write().await = false;
    }

    /// Handle a single client connection.
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        clients: Arc<RwLock<HashMap<Uuid, ClientState>>>,
        current_pin: Arc<RwLock<Option<String>>>,
        event_tx: broadcast::Sender<CompanionEvent>,
        require_pin: bool,
        daemon_state: Option<Arc<RwLock<super::state::DaemonState>>>,
    ) -> Result<(), CompanionError> {
        // Upgrade to WebSocket
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .map_err(|e| CompanionError::WebSocketError(e.to_string()))?;

        let (ws_sender, mut ws_receiver) = ws_stream.split();

        // Create client state
        let session_id = Uuid::new_v4();
        let (client_tx, mut client_rx) = mpsc::channel::<CompanionResponse>(CLIENT_CHANNEL_CAPACITY);

        let client_state = ClientState {
            session_id,
            connected_at: Utc::now(),
            remote_addr: addr,
            is_authenticated: !require_pin, // Auto-authenticated if PIN not required
            subscribed_to_metrics: false,
            tx: client_tx.clone(),
        };

        // Register the client
        clients
            .write()
            .await
            .insert(session_id, client_state);

        tracing::info!("Client {} registered with session {}", addr, session_id);

        // Wrap sender in Arc<Mutex> for shared access
        let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));
        let ws_sender_clone = Arc::clone(&ws_sender);

        // Subscribe to events for this client
        let mut event_rx = event_tx.subscribe();

        // Spawn task to forward responses and events to client
        let clients_for_forward = Arc::clone(&clients);
        let forward_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Forward responses from handlers
                    Some(response) = client_rx.recv() => {
                        if let Err(e) = Self::send_message(&ws_sender_clone, &response).await {
                            tracing::error!("Failed to send response to client: {}", e);
                            break;
                        }
                    }

                    // Forward events if subscribed
                    Ok(event) = event_rx.recv() => {
                        // Check if client is subscribed to metrics
                        let is_subscribed = clients_for_forward
                            .read()
                            .await
                            .get(&session_id)
                            .map(|c| c.subscribed_to_metrics && c.is_authenticated)
                            .unwrap_or(false);

                        if is_subscribed {
                            if let Err(e) = Self::send_event(&ws_sender_clone, &event).await {
                                tracing::error!("Failed to send event to client: {}", e);
                                break;
                            }
                        }
                    }

                    else => break,
                }
            }
        });

        // Main receive loop
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    // Parse the request
                    match serde_json::from_str::<CompanionRequest>(&text) {
                        Ok(request) => {
                            // Get current client state
                            let is_authenticated = clients
                                .read()
                                .await
                                .get(&session_id)
                                .map(|c| c.is_authenticated)
                                .unwrap_or(false);

                            // Handle authentication specially
                            let response = match &request {
                                CompanionRequest::Auth { pin } => {
                                    Self::handle_auth(
                                        &clients,
                                        &current_pin,
                                        session_id,
                                        pin,
                                        require_pin,
                                    )
                                    .await
                                }
                                CompanionRequest::SubscribeMetrics if is_authenticated => {
                                    // Update subscription state
                                    if let Some(client) = clients.write().await.get_mut(&session_id)
                                    {
                                        client.subscribed_to_metrics = true;
                                    }
                                    CompanionResponse::SubscribedMetrics
                                }
                                CompanionRequest::UnsubscribeMetrics if is_authenticated => {
                                    // Update subscription state
                                    if let Some(client) = clients.write().await.get_mut(&session_id)
                                    {
                                        client.subscribed_to_metrics = false;
                                    }
                                    CompanionResponse::UnsubscribedMetrics
                                }
                                _ => handle_request(request, session_id, is_authenticated, daemon_state.clone()).await,
                            };

                            // Send response through the client channel
                            if client_tx.send(response).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let response = CompanionResponse::Error {
                                code: CompanionErrorCode::InvalidParams,
                                message: format!("Invalid request format: {}", e),
                            };
                            if client_tx.send(response).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    // Respond with pong
                    let mut sender = ws_sender.lock().await;
                    if sender.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client {} closing connection", addr);
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket error from {}: {}", addr, e);
                    break;
                }
                _ => {}
            }
        }

        // Clean up
        forward_task.abort();
        clients.write().await.remove(&session_id);
        tracing::info!("Client {} (session {}) disconnected", addr, session_id);

        Ok(())
    }

    /// Handle authentication request.
    async fn handle_auth(
        clients: &Arc<RwLock<HashMap<Uuid, ClientState>>>,
        current_pin: &Arc<RwLock<Option<String>>>,
        session_id: Uuid,
        pin: &str,
        require_pin: bool,
    ) -> CompanionResponse {
        // If PIN not required, auto-authenticate
        if !require_pin {
            if let Some(client) = clients.write().await.get_mut(&session_id) {
                client.is_authenticated = true;
            }
            return CompanionResponse::AuthOk { session_id };
        }

        // Validate PIN
        let expected_pin = current_pin.read().await.clone();
        match expected_pin {
            Some(expected) if expected == pin => {
                if let Some(client) = clients.write().await.get_mut(&session_id) {
                    client.is_authenticated = true;
                }
                tracing::info!("Client {} authenticated successfully", session_id);
                CompanionResponse::AuthOk { session_id }
            }
            _ => {
                tracing::warn!("Authentication failed for session {}", session_id);
                CompanionResponse::AuthFailed {
                    reason: "Invalid PIN".to_string(),
                }
            }
        }
    }

    /// Send a response message to a client.
    async fn send_message(
        sender: &Arc<tokio::sync::Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        response: &CompanionResponse,
    ) -> Result<(), CompanionError> {
        let json = serde_json::to_string(response)
            .map_err(|e| CompanionError::InternalError(e.to_string()))?;

        sender
            .lock()
            .await
            .send(Message::Text(json))
            .await
            .map_err(|e| CompanionError::WebSocketError(e.to_string()))
    }

    /// Send an event message to a client.
    async fn send_event(
        sender: &Arc<tokio::sync::Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
        event: &CompanionEvent,
    ) -> Result<(), CompanionError> {
        let json = serde_json::to_string(event)
            .map_err(|e| CompanionError::InternalError(e.to_string()))?;

        sender
            .lock()
            .await
            .send(Message::Text(json))
            .await
            .map_err(|e| CompanionError::WebSocketError(e.to_string()))
    }

    /// Stop the companion server.
    ///
    /// Disconnects all clients and stops accepting new connections.
    pub async fn stop(&self) -> Result<(), CompanionError> {
        if !*self.is_running.read().await {
            return Ok(()); // Not running
        }

        // Stop mDNS advertisement
        if let Err(e) = self.mdns_advertiser.stop().await {
            tracing::warn!("Failed to stop mDNS advertisement: {}", e);
        }

        // Notify all clients of disconnection
        let _ = self.event_tx.send(CompanionEvent::Disconnecting {
            reason: "Server shutting down".to_string(),
        });

        // Send shutdown command to the accept loop
        if let Some(tx) = self.command_tx.read().await.as_ref() {
            let _ = tx.send(ServerCommand::Shutdown).await;
        }

        // Clear command channel
        *self.command_tx.write().await = None;

        // Clear all clients
        self.clients.write().await.clear();

        *self.is_running.write().await = false;
        *self.server_addr.write().await = None;

        tracing::info!("Companion server stopped");

        Ok(())
    }

    /// Check if the server is running.
    pub fn is_running(&self) -> bool {
        self.is_running.try_read().map(|r| *r).unwrap_or(false)
    }

    /// Get the server's WebSocket URL.
    ///
    /// Returns `None` if the server is not running.
    pub fn get_url(&self) -> Option<String> {
        let addr = self.server_addr.try_read().ok()?.as_ref().cloned()?;
        let ip = get_local_ip();
        Some(format!("ws://{}:{}", ip, addr.port()))
    }

    /// Get the current PIN.
    ///
    /// Returns `None` if PIN authentication is disabled or server not running.
    pub fn get_pin(&self) -> Option<String> {
        if !self.config.require_pin {
            return None;
        }
        self.current_pin.try_read().ok()?.clone()
    }

    /// Regenerate the authentication PIN.
    ///
    /// Returns the new PIN.
    pub async fn regenerate_pin(&self) -> String {
        let pin = Self::generate_pin();
        *self.current_pin.write().await = Some(pin.clone());
        tracing::info!("Companion PIN regenerated");
        pin
    }

    /// Get a list of connected clients.
    pub async fn get_clients(&self) -> Vec<CompanionClient> {
        self.clients
            .read()
            .await
            .values()
            .map(|c| c.to_companion_client())
            .collect()
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Disconnect a specific client by session ID.
    pub async fn disconnect_client(&self, session_id: &Uuid) -> Result<(), CompanionError> {
        // Send disconnect command to server
        if let Some(tx) = self.command_tx.read().await.as_ref() {
            let _ = tx.send(ServerCommand::DisconnectClient(*session_id)).await;
            tracing::info!("Disconnected companion client: {}", session_id);
            Ok(())
        } else {
            Err(CompanionError::SessionNotFound(*session_id))
        }
    }

    /// Broadcast a metrics event to all subscribed clients.
    pub fn broadcast_metrics(&self, event: CompanionEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Subscribe to server events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<CompanionEvent> {
        self.event_tx.subscribe()
    }

    /// Get server configuration.
    pub fn config(&self) -> &CompanionConfig {
        &self.config
    }

    /// Create a metrics streamer connected to this server's event broadcast.
    ///
    /// The metrics streamer will broadcast metrics events at 1Hz to all
    /// authenticated clients subscribed to metrics updates. The streamer
    /// uses the server's internal event channel for broadcasting.
    ///
    /// # Arguments
    ///
    /// * `config` - Optional configuration for the streamer. If None, uses defaults.
    ///
    /// # Returns
    ///
    /// A new `MetricsStreamer` instance that broadcasts to this server's clients.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let server = CompanionServer::new(config);
    /// let streamer = server.create_metrics_streamer(None);
    ///
    /// // Start the streamer when a session begins
    /// streamer.start().await;
    ///
    /// // Update metrics from sensor data
    /// streamer.update_power(250).await;
    /// streamer.update_heart_rate(145).await;
    ///
    /// // Stop when session ends
    /// streamer.stop().await;
    /// ```
    pub fn create_metrics_streamer(&self, config: Option<MetricsStreamerConfig>) -> MetricsStreamer {
        MetricsStreamer::new(self.event_tx.clone(), config)
    }

    /// Check if mDNS advertisement is active.
    pub async fn is_mdns_running(&self) -> bool {
        self.mdns_advertiser.is_running().await
    }

    /// Get the mDNS service type being advertised.
    pub fn mdns_service_type(&self) -> &'static str {
        self.mdns_advertiser.service_type()
    }

    /// Get the companion protocol version.
    pub fn protocol_version(&self) -> &'static str {
        self.mdns_advertiser.protocol_version()
    }

    /// Generate a QR code for mobile app pairing.
    ///
    /// The QR code contains the WebSocket URL and optional PIN
    /// for easy scanning and connection from the mobile companion app.
    ///
    /// # Returns
    ///
    /// A `CompanionQrCode` that can be rendered in ASCII or SVG format,
    /// or an error if the server is not running.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let server = CompanionServer::new(config);
    /// server.start().await?;
    ///
    /// if let Ok(qr) = server.generate_qr_code() {
    ///     // Display ASCII in terminal
    ///     println!("{}", qr.to_ascii());
    ///
    ///     // Get SVG for UI rendering
    ///     let svg = qr.to_svg();
    /// }
    /// ```
    pub fn generate_qr_code(&self) -> Result<CompanionQrCode, QrCodeError> {
        let url = self.get_url().ok_or_else(|| {
            QrCodeError::GenerationFailed("Server not running".to_string())
        })?;
        let pin = self.get_pin();
        CompanionQrCode::from_url_and_pin(url, pin)
    }

    /// Get the QR code as an ASCII string.
    ///
    /// Convenience method that generates and renders the QR code
    /// as ASCII art suitable for terminal display.
    ///
    /// # Returns
    ///
    /// The ASCII representation of the QR code, or an error if
    /// the server is not running.
    pub fn get_qr_ascii(&self) -> Result<String, QrCodeError> {
        Ok(self.generate_qr_code()?.to_ascii())
    }

    /// Get the QR code as an SVG string.
    ///
    /// Convenience method that generates and renders the QR code
    /// as SVG suitable for desktop UI display.
    ///
    /// # Returns
    ///
    /// The SVG representation of the QR code, or an error if
    /// the server is not running.
    pub fn get_qr_svg(&self) -> Result<String, QrCodeError> {
        Ok(self.generate_qr_code()?.to_svg())
    }

    /// Get the QR code as an SVG string with custom colors.
    ///
    /// # Arguments
    ///
    /// * `dark_color` - Hex color for dark modules (e.g., "#000000")
    /// * `light_color` - Hex color for light modules (e.g., "#ffffff")
    /// * `min_size` - Minimum dimensions in pixels
    ///
    /// # Returns
    ///
    /// The SVG representation of the QR code, or an error if
    /// the server is not running.
    pub fn get_qr_svg_custom(
        &self,
        dark_color: &str,
        light_color: &str,
        min_size: u32,
    ) -> Result<String, QrCodeError> {
        Ok(self
            .generate_qr_code()?
            .to_svg_custom(dark_color, light_color, min_size))
    }

    /// Generate a random 6-digit PIN.
    fn generate_pin() -> String {
        use rand::Rng;
        let pin: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
        format!("{:06}", pin)
    }
}

/// Get the local IP address for LAN connectivity.
///
/// This function attempts to find a non-loopback IPv4 address that can
/// be used for LAN connections from mobile devices.
pub fn get_local_ip() -> String {
    // Try to get the first non-loopback IPv4 address
    if let Ok(interfaces) = get_network_interfaces() {
        for (_, ip) in interfaces {
            if !ip.is_loopback() && ip.is_ipv4() {
                return ip.to_string();
            }
        }
    }

    // Fallback: try to connect to a public address and get local addr
    // This doesn't actually send any traffic
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }

    // Ultimate fallback
    "127.0.0.1".to_string()
}

/// Get network interfaces with their IP addresses.
fn get_network_interfaces() -> std::io::Result<Vec<(String, IpAddr)>> {
    let mut result = Vec::new();

    // Use platform-specific code to enumerate interfaces
    #[cfg(target_os = "windows")]
    {
        // On Windows, we use the fallback UDP socket approach
        // A full implementation would use GetAdaptersAddresses Win32 API
        return Ok(result);
    }

    #[cfg(unix)]
    {
        use std::ffi::CStr;
        use std::mem;
        use std::ptr;

        extern "C" {
            fn getifaddrs(ifap: *mut *mut libc::ifaddrs) -> libc::c_int;
            fn freeifaddrs(ifa: *mut libc::ifaddrs);
        }

        unsafe {
            let mut addrs: *mut libc::ifaddrs = ptr::null_mut();
            if getifaddrs(&mut addrs) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut addr = addrs;
            while !addr.is_null() {
                let ifa = &*addr;

                if !ifa.ifa_addr.is_null() {
                    let family = (*ifa.ifa_addr).sa_family as i32;

                    if family == libc::AF_INET {
                        let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().to_string();
                        let sockaddr = ifa.ifa_addr as *const libc::sockaddr_in;
                        let ip_bytes = (*sockaddr).sin_addr.s_addr.to_ne_bytes();
                        let ip = IpAddr::V4(Ipv4Addr::new(
                            ip_bytes[0],
                            ip_bytes[1],
                            ip_bytes[2],
                            ip_bytes[3],
                        ));
                        result.push((name, ip));
                    }
                }

                addr = ifa.ifa_next;
            }

            freeifaddrs(addrs);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = CompanionConfig::default();
        let server = CompanionServer::new(config);
        assert!(!server.is_running());
    }

    #[test]
    fn test_pin_generation() {
        let pin = CompanionServer::generate_pin();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_get_local_ip() {
        let ip = get_local_ip();
        // Should return a valid IP address
        assert!(!ip.is_empty());
        // Should be parseable as an IP
        assert!(ip.parse::<std::net::IpAddr>().is_ok());
    }

    #[tokio::test]
    async fn test_start_disabled() {
        let config = CompanionConfig {
            enabled: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        let result = server.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_not_running() {
        let config = CompanionConfig::default();
        let server = CompanionServer::new(config);
        // Should not error when stopping a server that's not running
        assert!(server.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_start_and_stop() {
        let config = CompanionConfig {
            enabled: true,
            port: 0, // Use any available port
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);

        // Start server
        let result = server.start().await;
        assert!(result.is_ok(), "Failed to start server: {:?}", result);
        assert!(server.is_running());

        // Get URL
        let url = server.get_url();
        assert!(url.is_some());

        // Stop server
        let result = server.stop().await;
        assert!(result.is_ok());
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_pin_management() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: true,
            ..Default::default()
        };
        let server = CompanionServer::new(config);

        // Start server to generate PIN
        server.start().await.unwrap();

        // Should have a PIN
        let pin = server.get_pin();
        assert!(pin.is_some());
        let original_pin = pin.unwrap();
        assert_eq!(original_pin.len(), 6);

        // Regenerate PIN
        let new_pin = server.regenerate_pin().await;
        assert_eq!(new_pin.len(), 6);
        // New PIN should be different (with very high probability)
        // Note: There's a tiny chance this could fail if the same PIN is generated
        // assert_ne!(new_pin, original_pin);

        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_no_clients_initially() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        assert_eq!(server.client_count().await, 0);
        assert!(server.get_clients().await.is_empty());

        server.stop().await.unwrap();
    }

    #[test]
    fn test_qr_code_not_running() {
        let config = CompanionConfig::default();
        let server = CompanionServer::new(config);

        // Should fail when server not running
        let result = server.generate_qr_code();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_qr_code_generation_without_pin() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        // Should generate QR code
        let qr = server.generate_qr_code();
        assert!(qr.is_ok());

        let qr = qr.unwrap();
        let data = qr.connection_data();

        // Should contain the URL
        assert!(data.url.starts_with("ws://"));

        // PIN should be None when not required
        assert!(data.pin.is_none());

        // ASCII and SVG should work
        let ascii = qr.to_ascii();
        assert!(!ascii.is_empty());

        let svg = qr.to_svg();
        assert!(svg.contains("<svg"));

        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_qr_code_generation_with_pin() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: true,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        // Should generate QR code with PIN
        let qr = server.generate_qr_code().unwrap();
        let data = qr.connection_data();

        // Should contain URL and PIN
        assert!(data.url.starts_with("ws://"));
        assert!(data.pin.is_some());
        assert_eq!(data.pin.as_ref().unwrap().len(), 6);

        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_qr_ascii_convenience() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        let ascii = server.get_qr_ascii();
        assert!(ascii.is_ok());
        assert!(!ascii.unwrap().is_empty());

        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_qr_svg_convenience() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        let svg = server.get_qr_svg();
        assert!(svg.is_ok());
        assert!(svg.unwrap().contains("<svg"));

        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_qr_svg_custom_colors() {
        let config = CompanionConfig {
            enabled: true,
            port: 0,
            require_pin: false,
            ..Default::default()
        };
        let server = CompanionServer::new(config);
        server.start().await.unwrap();

        let svg = server.get_qr_svg_custom("#333333", "#eeeeee", 250);
        assert!(svg.is_ok());
        let svg = svg.unwrap();
        assert!(svg.contains("#333333"));
        assert!(svg.contains("#eeeeee"));

        server.stop().await.unwrap();
    }
}
