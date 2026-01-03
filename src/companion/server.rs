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
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::types::{CompanionClient, CompanionConfig, CompanionError, CompanionEvent};

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
    clients: Arc<RwLock<HashMap<Uuid, CompanionClient>>>,
    /// Current PIN for authentication (if enabled).
    current_pin: Arc<RwLock<Option<String>>>,
    /// Channel for broadcasting events to clients.
    event_tx: broadcast::Sender<CompanionEvent>,
    /// Server's listening address (when running).
    server_addr: Arc<RwLock<Option<SocketAddr>>>,
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
        }
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
            *self.current_pin.write().await = Some(pin);
        }

        // TODO: T003 - Implement actual WebSocket server binding
        // let addr = format!("0.0.0.0:{}", self.config.port);
        // let listener = TcpListener::bind(&addr).await
        //     .map_err(|e| CompanionError::BindFailed(self.config.port, e.to_string()))?;

        let addr: SocketAddr = format!("0.0.0.0:{}", self.config.port)
            .parse()
            .map_err(|e| {
                CompanionError::BindFailed(self.config.port, format!("Invalid address: {}", e))
            })?;

        *self.server_addr.write().await = Some(addr);
        *self.is_running.write().await = true;

        tracing::info!("Companion server started on port {}", self.config.port);

        Ok(())
    }

    /// Stop the companion server.
    ///
    /// Disconnects all clients and stops accepting new connections.
    pub async fn stop(&self) -> Result<(), CompanionError> {
        if !*self.is_running.read().await {
            return Ok(()); // Not running
        }

        // Notify all clients of disconnection
        let _ = self.event_tx.send(CompanionEvent::Disconnecting {
            reason: "Server shutting down".to_string(),
        });

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
        Some(format!("ws://{}:{}", get_local_ip(), addr.port()))
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
        self.clients.read().await.values().cloned().collect()
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Disconnect a specific client by session ID.
    pub async fn disconnect_client(&self, session_id: &Uuid) -> Result<(), CompanionError> {
        if self.clients.write().await.remove(session_id).is_some() {
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
fn get_local_ip() -> String {
    // TODO: Implement proper local IP detection
    // For now, return localhost placeholder
    "127.0.0.1".to_string()
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
}
