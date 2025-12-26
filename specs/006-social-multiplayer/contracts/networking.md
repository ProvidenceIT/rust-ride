# Contract: Networking Module

**Module**: `src/networking/`
**Purpose**: LAN peer discovery and real-time communication

## Public API

### Discovery Service

```rust
/// mDNS-based service discovery for RustRide instances on LAN
pub struct DiscoveryService {
    // Internal daemon handle
}

impl DiscoveryService {
    /// Start the discovery service
    /// Registers this instance and begins listening for peers
    pub fn start(rider_profile: &RiderProfile) -> Result<Self, NetworkError>;

    /// Stop the discovery service
    pub fn stop(&self) -> Result<(), NetworkError>;

    /// Get currently discovered peers
    pub fn peers(&self) -> Vec<DiscoveredPeer>;

    /// Subscribe to peer discovery events
    pub fn subscribe(&self) -> Receiver<DiscoveryEvent>;
}

pub struct DiscoveredPeer {
    pub rider_id: Uuid,
    pub display_name: String,
    pub address: SocketAddr,
    pub session_id: Option<Uuid>,  // If hosting a group ride
    pub last_seen: Instant,
}

pub enum DiscoveryEvent {
    PeerDiscovered(DiscoveredPeer),
    PeerUpdated(DiscoveredPeer),
    PeerLost(Uuid),
}
```

### Session Manager

```rust
/// Manages group ride sessions
pub struct SessionManager {
    // Internal state
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(discovery: Arc<DiscoveryService>) -> Self;

    /// Host a new group ride session
    pub fn host_session(&self, config: SessionConfig) -> Result<Session, NetworkError>;

    /// Join an existing session
    pub fn join_session(&self, session_id: Uuid) -> Result<Session, NetworkError>;

    /// Leave current session
    pub fn leave_session(&self) -> Result<(), NetworkError>;

    /// Get current session (if any)
    pub fn current_session(&self) -> Option<&Session>;
}

pub struct SessionConfig {
    pub name: Option<String>,
    pub world_id: String,
    pub max_participants: u8,
}

pub struct Session {
    pub id: Uuid,
    pub host_rider_id: Uuid,
    pub participants: Vec<Participant>,
    pub started_at: DateTime<Utc>,
}

pub struct Participant {
    pub rider_id: Uuid,
    pub display_name: String,
    pub current_metrics: Option<RiderMetrics>,
    pub position: Option<Vec3>,
    pub connected: bool,
}
```

### Metric Sync

```rust
/// Real-time metric synchronization via UDP
pub struct MetricSync {
    // Internal UDP socket and state
}

impl MetricSync {
    /// Start metric synchronization for a session
    pub fn start(session: &Session) -> Result<Self, NetworkError>;

    /// Stop synchronization
    pub fn stop(&self) -> Result<(), NetworkError>;

    /// Broadcast local rider metrics
    pub fn broadcast_metrics(&self, metrics: &RiderMetrics) -> Result<(), NetworkError>;

    /// Subscribe to incoming metrics from peers
    pub fn subscribe(&self) -> Receiver<(Uuid, RiderMetrics)>;
}

pub struct RiderMetrics {
    pub power_watts: u16,
    pub cadence_rpm: u8,
    pub heart_rate_bpm: u8,
    pub speed_kmh: f32,
    pub position: Vec3,
    pub timestamp_ms: u64,
}
```

### Chat

```rust
/// Group ride chat functionality
pub struct ChatService {
    // Internal state
}

impl ChatService {
    /// Start chat for a session
    pub fn start(session: &Session) -> Result<Self, NetworkError>;

    /// Send a message
    pub fn send(&self, message: &str) -> Result<Uuid, NetworkError>;

    /// Subscribe to incoming messages
    pub fn subscribe(&self) -> Receiver<ChatMessage>;

    /// Get message history
    pub fn history(&self) -> Vec<ChatMessage>;
}

pub struct ChatMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub text: String,
    pub sent_at: DateTime<Utc>,
    pub delivered: bool,
}
```

## Protocol Messages

```rust
/// UDP protocol messages (serialized with bincode)
#[derive(Serialize, Deserialize)]
pub enum ProtocolMessage {
    // Session management
    SessionAnnounce {
        session_id: Uuid,
        host_name: String,
        world_id: String,
        participant_count: u8,
    },
    SessionJoin {
        session_id: Uuid,
        rider_id: Uuid,
        rider_name: String,
    },
    SessionJoinAck {
        session_id: Uuid,
        success: bool,
        participants: Vec<ParticipantInfo>,
    },
    SessionLeave {
        session_id: Uuid,
        rider_id: Uuid,
    },

    // Metrics (high frequency)
    MetricUpdate {
        rider_id: Uuid,
        metrics: RiderMetrics,
        sequence: u32,
    },

    // Chat
    ChatSend {
        msg_id: Uuid,
        text: String,
    },
    ChatAck {
        msg_id: Uuid,
    },

    // Heartbeat
    Ping {
        timestamp_ms: u64,
    },
    Pong {
        timestamp_ms: u64,
        clock_offset_ms: i64,
    },
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Discovery service failed: {0}")]
    DiscoveryFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Session full")]
    SessionFull,

    #[error("Already in session")]
    AlreadyInSession,

    #[error("Not in session")]
    NotInSession,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## Configuration

```rust
pub struct NetworkConfig {
    /// mDNS service type
    pub service_type: String,  // Default: "_rustride._udp.local."

    /// UDP multicast address for metrics
    pub multicast_addr: SocketAddr,  // Default: 239.255.42.42:7878

    /// Heartbeat interval
    pub heartbeat_interval: Duration,  // Default: 1 second

    /// Disconnect timeout (missed heartbeats)
    pub disconnect_timeout: Duration,  // Default: 5 seconds

    /// Metric broadcast rate
    pub metric_rate_hz: u8,  // Default: 20
}
```
