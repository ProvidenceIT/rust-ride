//! Real-time metric synchronization over UDP.
//!
//! Broadcasts and receives rider metrics for group rides.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::protocol::{ProtocolMessage, RiderMetrics, RiderPosition, MAX_MESSAGE_SIZE};

/// Default multicast address for metric sync.
pub const MULTICAST_ADDR: &str = "239.255.42.42";

/// Default port for metric sync.
pub const SYNC_PORT: u16 = 7879;

/// Metric update rate in Hz.
pub const METRIC_RATE_HZ: u8 = 20;

/// Metric update event.
#[derive(Debug, Clone)]
pub struct MetricUpdateEvent {
    pub rider_id: Uuid,
    pub metrics: RiderMetrics,
    pub position: Option<RiderPosition>,
    pub sequence: u32,
}

/// Metric sync service.
pub struct MetricSync {
    session_id: Option<Uuid>,
    local_rider_id: Uuid,
    socket: Option<Arc<UdpSocket>>,
    peer_metrics: Arc<RwLock<HashMap<Uuid, MetricUpdateEvent>>>,
    event_tx: broadcast::Sender<MetricUpdateEvent>,
    sequence: Arc<std::sync::atomic::AtomicU32>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl MetricSync {
    /// Create a new metric sync service.
    pub fn new(rider_id: Uuid) -> Self {
        let (tx, _) = broadcast::channel(256);

        Self {
            session_id: None,
            local_rider_id: rider_id,
            socket: None,
            peer_metrics: Arc::new(RwLock::new(HashMap::new())),
            event_tx: tx,
            sequence: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the metric sync service.
    pub async fn start(&mut self, session_id: Uuid, port: u16) -> Result<(), SyncError> {
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(SyncError::AlreadyRunning);
        }

        self.session_id = Some(session_id);

        // Bind UDP socket
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
            .await
            .map_err(|e| SyncError::BindFailed(e.to_string()))?;

        // Join multicast group
        let multicast_addr: std::net::Ipv4Addr = MULTICAST_ADDR.parse().unwrap();
        socket
            .join_multicast_v4(multicast_addr, std::net::Ipv4Addr::UNSPECIFIED)
            .map_err(|e| SyncError::MulticastFailed(e.to_string()))?;

        let socket = Arc::new(socket);
        self.socket = Some(Arc::clone(&socket));
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        // Start receive loop
        let peer_metrics = Arc::clone(&self.peer_metrics);
        let event_tx = self.event_tx.clone();
        let local_rider_id = self.local_rider_id;
        let running = Arc::clone(&self.running);
        let expected_session_id = session_id;

        tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_MESSAGE_SIZE];

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                match socket.recv_from(&mut buf).await {
                    Ok((len, _addr)) => {
                        if let Ok(msg) = ProtocolMessage::from_bytes(&buf[..len]) {
                            match msg {
                                // Process metric updates
                                ProtocolMessage::MetricUpdate {
                                    session_id,
                                    rider_id,
                                    metrics,
                                    sequence,
                                } => {
                                    // Ignore our own messages and wrong sessions
                                    if rider_id == local_rider_id || session_id != expected_session_id {
                                        continue;
                                    }

                                    let event = MetricUpdateEvent {
                                        rider_id,
                                        metrics,
                                        position: None,
                                        sequence,
                                    };

                                    peer_metrics.write().unwrap().insert(rider_id, event.clone());
                                    let _ = event_tx.send(event);
                                }

                                // Process position updates
                                ProtocolMessage::PositionUpdate {
                                    session_id,
                                    rider_id,
                                    position,
                                    sequence,
                                } => {
                                    if rider_id == local_rider_id || session_id != expected_session_id {
                                        continue;
                                    }

                                    // Update position in existing metrics or create new
                                    let mut metrics_guard = peer_metrics.write().unwrap();
                                    if let Some(existing) = metrics_guard.get_mut(&rider_id) {
                                        existing.position = Some(position);
                                        existing.sequence = sequence;
                                        let _ = event_tx.send(existing.clone());
                                    }
                                }

                                _ => {} // Ignore other message types
                            }
                        }
                    }
                    Err(e) => {
                        if running.load(std::sync::atomic::Ordering::SeqCst) {
                            tracing::warn!("UDP receive error: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the metric sync service.
    pub fn stop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        self.socket = None;
        self.session_id = None;
        self.peer_metrics.write().unwrap().clear();
    }

    /// Broadcast local metrics.
    pub async fn broadcast_metrics(&self, metrics: RiderMetrics) -> Result<(), SyncError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or(SyncError::NotRunning)?;

        let session_id = self.session_id.ok_or(SyncError::NotRunning)?;

        let sequence = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let msg = ProtocolMessage::MetricUpdate {
            session_id,
            rider_id: self.local_rider_id,
            metrics,
            sequence,
        };

        let bytes = msg.to_bytes().map_err(|e| SyncError::SerializeFailed(e.to_string()))?;

        let multicast_addr = format!("{}:{}", MULTICAST_ADDR, SYNC_PORT);
        socket
            .send_to(&bytes, &multicast_addr)
            .await
            .map_err(|e| SyncError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Broadcast local position.
    pub async fn broadcast_position(&self, position: RiderPosition) -> Result<(), SyncError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or(SyncError::NotRunning)?;

        let session_id = self.session_id.ok_or(SyncError::NotRunning)?;

        let sequence = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let msg = ProtocolMessage::PositionUpdate {
            session_id,
            rider_id: self.local_rider_id,
            position,
            sequence,
        };

        let bytes = msg.to_bytes().map_err(|e| SyncError::SerializeFailed(e.to_string()))?;

        let multicast_addr = format!("{}:{}", MULTICAST_ADDR, SYNC_PORT);
        socket
            .send_to(&bytes, &multicast_addr)
            .await
            .map_err(|e| SyncError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Get current metrics for all peers.
    pub fn peer_metrics(&self) -> HashMap<Uuid, MetricUpdateEvent> {
        self.peer_metrics.read().unwrap().clone()
    }

    /// Get metrics for a specific peer.
    pub fn get_peer_metrics(&self, rider_id: &Uuid) -> Option<MetricUpdateEvent> {
        self.peer_metrics.read().unwrap().get(rider_id).cloned()
    }

    /// Subscribe to metric updates.
    pub fn subscribe(&self) -> broadcast::Receiver<MetricUpdateEvent> {
        self.event_tx.subscribe()
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Sync errors.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Already running")]
    AlreadyRunning,

    #[error("Not running")]
    NotRunning,

    #[error("Failed to bind: {0}")]
    BindFailed(String),

    #[error("Failed to join multicast: {0}")]
    MulticastFailed(String),

    #[error("Failed to serialize: {0}")]
    SerializeFailed(String),

    #[error("Failed to send: {0}")]
    SendFailed(String),
}
