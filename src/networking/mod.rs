//! Networking module for LAN-based multiplayer
//!
//! Provides mDNS discovery, UDP metric synchronization, session management, and chat.

pub mod chat;
pub mod discovery;
pub mod protocol;
pub mod session;
pub mod sync;

// Re-export commonly used types
pub use chat::ChatService;
pub use discovery::{DiscoveryService, PeerInfo};
pub use protocol::{ProtocolMessage, RiderMetrics, RiderPosition};
pub use session::{Session, SessionManager, SessionState};
pub use sync::MetricSync;

/// Network configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// mDNS service type.
    pub service_type: String,
    /// Multicast address for metric sync.
    pub multicast_addr: String,
    /// Port for mDNS service.
    pub discovery_port: u16,
    /// Port for metric sync.
    pub sync_port: u16,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Disconnect timeout in milliseconds.
    pub disconnect_timeout_ms: u64,
    /// Metric update rate in Hz.
    pub metric_rate_hz: u8,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            service_type: discovery::SERVICE_TYPE.to_string(),
            multicast_addr: sync::MULTICAST_ADDR.to_string(),
            discovery_port: discovery::DEFAULT_PORT,
            sync_port: sync::SYNC_PORT,
            heartbeat_interval_ms: session::HEARTBEAT_INTERVAL_MS,
            disconnect_timeout_ms: session::DISCONNECT_TIMEOUT_MS,
            metric_rate_hz: sync::METRIC_RATE_HZ,
        }
    }
}

impl NetworkConfig {
    /// Create a new network configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }
}
