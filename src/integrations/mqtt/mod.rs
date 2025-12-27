//! MQTT Integration for Smart Fan Control
//!
//! Provides MQTT client and fan controller for smart home integration.

pub mod client;
pub mod fan;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use client::{DefaultMqttClient, MqttClient};
pub use fan::{DefaultFanController, FanController, FanProfile, FanState, PayloadFormat};

/// MQTT-related errors
#[derive(Debug, Error)]
pub enum MqttError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Publish failed: {0}")]
    PublishFailed(String),

    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Broker error: {0}")]
    BrokerError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// MQTT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// Whether MQTT is enabled
    pub enabled: bool,
    /// Broker hostname or IP
    pub broker_host: String,
    /// Broker port (default 1883, or 8883 for TLS)
    pub broker_port: u16,
    /// Use TLS/SSL
    pub use_tls: bool,
    /// Username for authentication (optional)
    pub username: Option<String>,
    /// Password is stored in OS keyring, not in config
    /// Client ID for MQTT connection
    pub client_id: String,
    /// Auto-reconnect interval in seconds
    pub reconnect_interval_secs: u32,
    /// Keep-alive interval in seconds
    pub keep_alive_secs: u16,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            use_tls: false,
            username: None,
            client_id: format!(
                "rustride-{}",
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("default")
            ),
            reconnect_interval_secs: 5,
            keep_alive_secs: 60,
        }
    }
}

/// Quality of Service levels for MQTT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoS {
    /// At most once delivery (fire and forget)
    AtMostOnce = 0,
    /// At least once delivery
    AtLeastOnce = 1,
    /// Exactly once delivery
    ExactlyOnce = 2,
}

/// MQTT connection events
#[derive(Debug, Clone)]
pub enum MqttEvent {
    /// Successfully connected to broker
    Connected,
    /// Disconnected from broker
    Disconnected,
    /// Attempting to reconnect
    Reconnecting { attempt: u32 },
    /// Message received on subscribed topic
    MessageReceived { topic: String, payload: String },
    /// Error occurred
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MqttConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.broker_host, "localhost");
        assert_eq!(config.broker_port, 1883);
        assert!(!config.use_tls);
    }
}
