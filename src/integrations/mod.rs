//! External Integrations Module
//!
//! Provides integrations for MQTT (fan control), WebSocket streaming,
//! weather data, and fitness platform sync.

pub mod mqtt;
pub mod streaming;
pub mod sync;
pub mod weather;

// Re-export main types for convenience
pub use mqtt::{
    DefaultFanController, DefaultMqttClient, FanController, FanProfile, FanState, MqttClient,
    MqttConfig, MqttError, MqttEvent, PayloadFormat, QoS,
};
pub use streaming::{
    DefaultPinAuthenticator, DefaultStreamingServer, PinAuthenticator, QrCodeData, StreamingConfig,
    StreamingError, StreamingEvent, StreamingMetrics, StreamingServer, StreamingSession,
};
pub use sync::{CredentialStore, OAuthHandler, PlatformUploader, SyncConfig, SyncPlatform};
pub use weather::{WeatherConfig, WeatherData, WeatherProvider};
