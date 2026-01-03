//! MQTT Integration for Smart Fan Control
//!
//! Provides MQTT client and fan controller for smart home integration.

pub mod client;
pub mod fan;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export main types
pub use client::{ConnectionState, DefaultMqttClient, FanTestResult, MqttClient, MqttTestResult, test_fan, test_mqtt_connection};
pub use fan::{
    DefaultFanController, FanController, FanProfile, FanProfileSettings, FanState, PayloadFormat,
    // Database helper functions
    delete_fan_profile, load_active_fan_profile, load_fan_profiles, save_fan_profile, save_fan_profiles,
};

// Re-export credential store
pub use crate::integrations::mqtt::MqttCredentialStore;

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

    #[error("Credential error: {0}")]
    CredentialError(String),
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
    /// Connection timeout in seconds (how long to wait for initial connection)
    #[serde(default = "default_connection_timeout_secs")]
    pub connection_timeout_secs: u32,
    /// Maximum reconnection attempts before giving up (None = unlimited)
    #[serde(default)]
    pub max_reconnect_attempts: Option<u32>,
}

fn default_connection_timeout_secs() -> u32 {
    30
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
            connection_timeout_secs: default_connection_timeout_secs(),
            max_reconnect_attempts: None, // Unlimited by default
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
    /// Disconnected from broker (clean disconnect, no auto-reconnect)
    Disconnected,
    /// Connection lost unexpectedly (will trigger auto-reconnect if enabled)
    ConnectionLost { reason: String },
    /// Attempting to reconnect
    Reconnecting { attempt: u32 },
    /// Reconnection failed after reaching max attempts
    ReconnectionFailed { attempts: u32, reason: String },
    /// Message received on subscribed topic
    MessageReceived { topic: String, payload: String },
    /// Error occurred
    Error { message: String },
}

/// Service name used for keyring entries
const MQTT_KEYRING_SERVICE: &str = "RustRide-MQTT";

/// Secure storage for MQTT broker passwords using the OS keyring.
///
/// This provides platform-specific secure storage:
/// - Windows: Windows Credential Manager
/// - macOS: macOS Keychain
/// - Linux: Secret Service (via libsecret)
pub struct MqttCredentialStore {
    service_name: String,
}

impl Default for MqttCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MqttCredentialStore {
    /// Create a new MQTT credential store with default service name.
    pub fn new() -> Self {
        Self {
            service_name: MQTT_KEYRING_SERVICE.to_string(),
        }
    }

    /// Create a new MQTT credential store with a custom service name.
    /// Useful for testing or multiple instances.
    pub fn with_service_name(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Get the keyring key for a given username and host.
    /// The key uniquely identifies the broker credentials.
    fn key_for_credentials(&self, username: &str, host: &str) -> String {
        format!("{}@{}", username, host)
    }

    /// Create a keyring entry for the given credentials.
    fn entry_for_credentials(
        &self,
        username: &str,
        host: &str,
    ) -> Result<keyring::Entry, MqttError> {
        let key = self.key_for_credentials(username, host);
        keyring::Entry::new(&self.service_name, &key)
            .map_err(|e| MqttError::CredentialError(format!("Failed to create keyring entry: {}", e)))
    }

    /// Store a password for the given username and host.
    ///
    /// # Arguments
    /// * `username` - The MQTT broker username
    /// * `host` - The MQTT broker hostname
    /// * `password` - The password to store
    pub fn store_password(
        &self,
        username: &str,
        host: &str,
        password: &str,
    ) -> Result<(), MqttError> {
        let entry = self.entry_for_credentials(username, host)?;
        entry
            .set_password(password)
            .map_err(|e| MqttError::CredentialError(format!("Failed to store password: {}", e)))?;

        tracing::debug!("Stored MQTT password for {}@{} in OS keyring", username, host);
        Ok(())
    }

    /// Retrieve the password for the given username and host.
    ///
    /// # Arguments
    /// * `username` - The MQTT broker username
    /// * `host` - The MQTT broker hostname
    ///
    /// # Returns
    /// * `Ok(Some(password))` - Password was found
    /// * `Ok(None)` - No password stored for this username/host
    /// * `Err(MqttError)` - An error occurred accessing the keyring
    pub fn get_password(&self, username: &str, host: &str) -> Result<Option<String>, MqttError> {
        let entry = self.entry_for_credentials(username, host)?;

        match entry.get_password() {
            Ok(password) => {
                tracing::debug!("Retrieved MQTT password for {}@{} from OS keyring", username, host);
                Ok(Some(password))
            }
            Err(keyring::Error::NoEntry) => {
                tracing::debug!("No MQTT password found for {}@{} in OS keyring", username, host);
                Ok(None)
            }
            Err(e) => {
                tracing::error!("Failed to retrieve MQTT password for {}@{}: {}", username, host, e);
                Err(MqttError::CredentialError(format!(
                    "Failed to retrieve password: {}",
                    e
                )))
            }
        }
    }

    /// Delete the password for the given username and host.
    ///
    /// # Arguments
    /// * `username` - The MQTT broker username
    /// * `host` - The MQTT broker hostname
    pub fn delete_password(&self, username: &str, host: &str) -> Result<(), MqttError> {
        let entry = self.entry_for_credentials(username, host)?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted MQTT password for {}@{} from OS keyring", username, host);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Already deleted or never existed - not an error
                tracing::debug!("No MQTT password to delete for {}@{}", username, host);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to delete MQTT password for {}@{}: {}", username, host, e);
                Err(MqttError::CredentialError(format!(
                    "Failed to delete password: {}",
                    e
                )))
            }
        }
    }
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
        // Verify new fields have correct defaults
        assert_eq!(config.connection_timeout_secs, 30);
        assert!(config.max_reconnect_attempts.is_none());
    }

    #[test]
    fn test_config_with_max_reconnect_attempts() {
        let config = MqttConfig {
            max_reconnect_attempts: Some(10),
            connection_timeout_secs: 60,
            ..Default::default()
        };
        assert_eq!(config.max_reconnect_attempts, Some(10));
        assert_eq!(config.connection_timeout_secs, 60);
    }

    #[test]
    fn test_credential_store_default() {
        let store = MqttCredentialStore::new();
        // Default service name should be set
        assert_eq!(store.service_name, MQTT_KEYRING_SERVICE);
    }

    #[test]
    fn test_credential_store_custom_service() {
        let store = MqttCredentialStore::with_service_name("TestService");
        assert_eq!(store.service_name, "TestService");
    }

    #[test]
    fn test_credential_store_key_generation() {
        let store = MqttCredentialStore::new();

        // Key should be in format "username@host"
        let key = store.key_for_credentials("testuser", "broker.example.com");
        assert_eq!(key, "testuser@broker.example.com");

        // Different host should produce different key
        let key2 = store.key_for_credentials("testuser", "other-broker.local");
        assert_eq!(key2, "testuser@other-broker.local");
        assert_ne!(key, key2);

        // Different user should produce different key
        let key3 = store.key_for_credentials("admin", "broker.example.com");
        assert_eq!(key3, "admin@broker.example.com");
        assert_ne!(key, key3);
    }

    #[test]
    fn test_credential_error_display() {
        let error = MqttError::CredentialError("test error".to_string());
        assert_eq!(format!("{}", error), "Credential error: test error");
    }
}
