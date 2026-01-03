//! Companion server module for mobile app connectivity.
//!
//! This module provides a WebSocket server that allows the RustRide mobile
//! companion app to connect over LAN for remote workout control, real-time
//! metrics streaming, and ride history access.
//!
//! ## Architecture
//!
//! The companion server consists of:
//! - `server`: WebSocket server using tokio-tungstenite for LAN connectivity
//! - `handlers`: Request handlers for workout control, metrics, and history
//! - `types`: Message types for WebSocket communication
//! - `discovery`: mDNS service advertisement for auto-discovery
//! - `streaming`: Real-time metrics streaming at 1Hz to connected clients
//!
//! ## Authentication
//!
//! The server supports optional PIN-based authentication. When enabled,
//! clients must send an authentication message with the correct PIN before
//! receiving metrics or sending commands.
//!
//! ## Discovery
//!
//! The server advertises itself via mDNS as `_rustride._tcp.local` for
//! automatic discovery by the mobile app on the same LAN. TXT records
//! include the port number and protocol version.
//!
//! ## Metrics Streaming
//!
//! The streaming module provides 1Hz metrics broadcasting to authenticated
//! clients. It subscribes to sensor events and aggregates power, heart rate,
//! cadence, speed, distance, and calorie data.
//!
//! ## Feature: Mobile Companion App (014)

pub mod discovery;
pub mod handlers;
pub mod server;
pub mod streaming;
pub mod types;

// Re-export commonly used types
pub use discovery::{CompanionMdnsAdvertiser, COMPANION_PROTOCOL_VERSION, COMPANION_SERVICE_TYPE};
pub use server::CompanionServer;
pub use streaming::{MetricsStreamer, MetricsStreamerConfig, SensorEventProcessor, StreamingMetrics};
pub use types::{
    CompanionConfig, CompanionError, CompanionEvent, CompanionRequest, CompanionResponse,
};
