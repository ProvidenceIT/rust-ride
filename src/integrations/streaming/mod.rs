//! WebSocket Streaming for External Display
//!
//! Provides real-time metrics streaming to secondary devices.

pub mod pin;
pub mod server;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use pin::{DefaultPinAuthenticator, PinAuthenticator};
pub use server::{DefaultStreamingServer, QrCodeData, StreamingServer};

/// Streaming-related errors
#[derive(Debug, Error)]
pub enum StreamingError {
    #[error("Server bind failed: {0}")]
    BindFailed(String),

    #[error("Server not running")]
    NotRunning,

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Authentication failed")]
    AuthenticationFailed,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Whether streaming is enabled
    pub enabled: bool,
    /// Port to listen on
    pub port: u16,
    /// Require PIN authentication
    pub require_pin: bool,
    /// PIN expiry time in minutes
    pub pin_expiry_minutes: u32,
    /// Metrics update interval in milliseconds
    pub update_interval_ms: u32,
    /// Which metrics to stream
    pub metrics_to_stream: Vec<StreamMetric>,
    /// Allow connections from any IP (not just local network)
    pub allow_remote: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
            require_pin: true,
            pin_expiry_minutes: 60,
            update_interval_ms: 1000,
            metrics_to_stream: vec![
                StreamMetric::Power,
                StreamMetric::HeartRate,
                StreamMetric::Cadence,
                StreamMetric::Speed,
                StreamMetric::Distance,
                StreamMetric::ElapsedTime,
            ],
            allow_remote: false,
        }
    }
}

/// Metrics that can be streamed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamMetric {
    Power,
    HeartRate,
    Cadence,
    Speed,
    Distance,
    ElapsedTime,
    CurrentInterval,
    ZoneName,
    Gradient,
    LeftRightBalance,
    Calories,
    NormalizedPower,
    IntensityFactor,
}

/// Data packet sent to streaming clients
#[derive(Debug, Clone, Serialize)]
pub struct StreamingMetrics {
    /// Timestamp in milliseconds since ride start
    pub timestamp_ms: u64,
    /// Current power in watts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<u16>,
    /// Current heart rate in BPM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heart_rate: Option<u8>,
    /// Current cadence in RPM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence: Option<u8>,
    /// Current speed in km/h
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// Total distance in meters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f32>,
    /// Elapsed time
    pub elapsed_time: Duration,
    /// Current workout interval name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_interval: Option<String>,
    /// Current power zone name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_name: Option<String>,
    /// Current gradient in percent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<f32>,
    /// L/R power balance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_right_balance: Option<(f32, f32)>,
    /// Calories burned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<u32>,
    /// Normalized power
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_power: Option<u16>,
    /// Intensity factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity_factor: Option<f32>,
}

impl Default for StreamingMetrics {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            power: None,
            heart_rate: None,
            cadence: None,
            speed: None,
            distance: None,
            elapsed_time: Duration::ZERO,
            current_interval: None,
            zone_name: None,
            gradient: None,
            left_right_balance: None,
            calories: None,
            normalized_power: None,
            intensity_factor: None,
        }
    }
}

/// Connected streaming session
#[derive(Debug, Clone)]
pub struct StreamingSession {
    /// Unique session ID
    pub id: Uuid,
    /// Client IP address
    pub client_ip: String,
    /// User agent if available
    pub user_agent: Option<String>,
    /// When the session was created
    pub connected_at: std::time::Instant,
    /// Whether authenticated
    pub authenticated: bool,
    /// Last activity timestamp
    pub last_activity: std::time::Instant,
}

/// Streaming server events
#[derive(Debug, Clone)]
pub enum StreamingEvent {
    /// Server started
    ServerStarted { url: String },
    /// Server stopped
    ServerStopped,
    /// Client connected (before auth)
    ClientConnected { session: StreamingSession },
    /// Client authenticated successfully
    ClientAuthenticated { session_id: Uuid },
    /// Client disconnected
    ClientDisconnected { session_id: Uuid },
    /// Authentication failed
    AuthenticationFailed { client_ip: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = StreamingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 8080);
        assert!(config.require_pin);
    }

    #[test]
    fn test_metrics_serialization() {
        let metrics = StreamingMetrics {
            power: Some(200),
            heart_rate: Some(140),
            ..Default::default()
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"power\":200"));
        assert!(json.contains("\"heart_rate\":140"));
        // None values should be skipped
        assert!(!json.contains("cadence"));
    }
}
