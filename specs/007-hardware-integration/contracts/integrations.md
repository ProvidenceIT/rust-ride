# Contract: Integrations Module

**Module**: `src/integrations/`
**Feature**: Hardware Integration
**Date**: 2025-12-26

This contract defines APIs for MQTT fan control, external display streaming, weather integration, and fitness platform sync.

---

## MQTT Fan Control (`src/integrations/mqtt/`)

### MQTT Client

```rust
/// MQTT broker connection management
pub trait MqttClient: Send + Sync {
    /// Connect to MQTT broker
    async fn connect(&self, config: &MqttConfig) -> Result<(), MqttError>;

    /// Disconnect from broker
    async fn disconnect(&self) -> Result<(), MqttError>;

    /// Check connection status
    fn is_connected(&self) -> bool;

    /// Publish message to topic
    async fn publish(&self, topic: &str, payload: &str, qos: QoS) -> Result<(), MqttError>;

    /// Subscribe to topic
    async fn subscribe(&self, topic: &str, qos: QoS) -> Result<(), MqttError>;

    /// Get connection events
    fn subscribe_events(&self) -> broadcast::Receiver<MqttEvent>;
}

pub enum QoS {
    AtMostOnce,  // 0
    AtLeastOnce, // 1
    ExactlyOnce, // 2
}

pub enum MqttEvent {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
    MessageReceived { topic: String, payload: String },
    Error { message: String },
}
```

### Fan Controller

```rust
/// Smart fan control based on training zones
pub trait FanController: Send + Sync {
    /// Initialize with profiles
    fn configure(&self, profiles: Vec<FanProfile>);

    /// Start fan control for a ride
    async fn start(&self) -> Result<(), MqttError>;

    /// Stop fan control
    async fn stop(&self) -> Result<(), MqttError>;

    /// Update current metrics (triggers fan speed evaluation)
    fn update_metrics(&self, power: u16, hr: Option<u8>);

    /// Manually set fan speed (override auto)
    async fn set_speed(&self, profile_id: &Uuid, speed: u8) -> Result<(), MqttError>;

    /// Get current fan states
    fn get_states(&self) -> HashMap<Uuid, FanState>;

    /// Test fan (cycle through speeds)
    async fn test_fan(&self, profile_id: &Uuid) -> Result<(), MqttError>;
}

pub struct FanState {
    pub profile_id: Uuid,
    pub current_speed: u8,
    pub last_zone: u8,
    pub auto_mode: bool,
    pub last_update: Instant,
}
```

---

## External Display Streaming (`src/integrations/streaming/`)

### WebSocket Server

```rust
/// WebSocket server for metrics streaming
pub trait StreamingServer: Send + Sync {
    /// Start the streaming server
    async fn start(&self, config: &StreamingConfig) -> Result<(), StreamingError>;

    /// Stop the server
    async fn stop(&self) -> Result<(), StreamingError>;

    /// Check if running
    fn is_running(&self) -> bool;

    /// Get server URL
    fn get_url(&self) -> Option<String>;

    /// Generate QR code for URL
    fn get_qr_code(&self) -> Option<QrCode>;

    /// Generate new PIN
    fn regenerate_pin(&self) -> String;

    /// Get current PIN
    fn get_pin(&self) -> Option<String>;

    /// Get connected sessions
    fn get_sessions(&self) -> Vec<StreamingSession>;

    /// Disconnect a session
    async fn disconnect_session(&self, session_id: &Uuid) -> Result<(), StreamingError>;

    /// Broadcast metrics update
    fn broadcast_metrics(&self, metrics: &StreamingMetrics);

    /// Subscribe to server events
    fn subscribe_events(&self) -> broadcast::Receiver<StreamingEvent>;
}

pub struct StreamingMetrics {
    pub power: Option<u16>,
    pub heart_rate: Option<u8>,
    pub cadence: Option<u8>,
    pub speed: Option<f32>,
    pub distance: Option<f32>,
    pub elapsed_time: Duration,
    pub current_interval: Option<String>,
    pub zone_name: Option<String>,
    pub gradient: Option<f32>,
    pub left_right_balance: Option<(f32, f32)>,
}

pub enum StreamingEvent {
    ServerStarted { url: String },
    ServerStopped,
    ClientConnected { session: StreamingSession },
    ClientAuthenticated { session_id: Uuid },
    ClientDisconnected { session_id: Uuid },
    AuthenticationFailed { client_ip: String },
}
```

### PIN Authentication

```rust
/// PIN-based authentication for streaming
pub trait PinAuthenticator: Send + Sync {
    /// Generate a new 6-digit PIN
    fn generate_pin(&self) -> String;

    /// Validate a PIN attempt
    fn validate_pin(&self, attempt: &str) -> bool;

    /// Get time until PIN expires
    fn time_until_expiry(&self) -> Duration;

    /// Check if PIN has expired
    fn is_expired(&self) -> bool;

    /// Regenerate PIN (invalidates old one)
    fn regenerate(&self) -> String;
}
```

---

## Weather Integration (`src/integrations/weather/`)

```rust
/// Weather data provider
pub trait WeatherProvider: Send + Sync {
    /// Configure provider
    fn configure(&self, config: WeatherConfig);

    /// Fetch current weather (uses cache if valid)
    async fn get_weather(&self) -> Result<WeatherData, WeatherError>;

    /// Force refresh (ignore cache)
    async fn refresh(&self) -> Result<WeatherData, WeatherError>;

    /// Check if weather data is available
    fn is_available(&self) -> bool;

    /// Get cached data (even if stale)
    fn get_cached(&self) -> Option<WeatherData>;

    /// Get last fetch time
    fn last_updated(&self) -> Option<DateTime<Utc>>;
}

#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    #[error("API key not configured")]
    ApiKeyMissing,

    #[error("Location not configured")]
    LocationMissing,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
```

---

## Fitness Platform Sync (`src/integrations/sync/`)

### OAuth Handler

```rust
/// OAuth 2.0 flow handler
pub trait OAuthHandler: Send + Sync {
    /// Start OAuth authorization flow
    async fn start_authorization(&self, platform: SyncPlatform) -> Result<AuthorizationUrl, SyncError>;

    /// Handle OAuth callback
    async fn handle_callback(&self, code: &str, state: &str) -> Result<TokenResponse, SyncError>;

    /// Refresh access token
    async fn refresh_token(&self, platform: SyncPlatform) -> Result<TokenResponse, SyncError>;

    /// Check if platform is authorized
    fn is_authorized(&self, platform: SyncPlatform) -> bool;

    /// Get token status
    fn get_token_status(&self, platform: SyncPlatform) -> TokenStatus;

    /// Revoke authorization
    async fn revoke(&self, platform: SyncPlatform) -> Result<(), SyncError>;
}

pub struct AuthorizationUrl {
    pub url: String,
    pub state: String,
}

pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

pub enum TokenStatus {
    Valid { expires_in: Duration },
    Expired,
    NeedsRefresh,
    NotConfigured,
}
```

### Platform Uploader

```rust
/// Upload rides to fitness platforms
pub trait PlatformUploader: Send + Sync {
    /// Upload a ride to platform
    async fn upload(&self, platform: SyncPlatform, ride: &Ride, fit_data: &[u8]) -> Result<SyncRecord, SyncError>;

    /// Get upload status
    fn get_status(&self, record_id: &Uuid) -> Option<SyncRecordStatus>;

    /// Retry failed upload
    async fn retry(&self, record_id: &Uuid) -> Result<SyncRecord, SyncError>;

    /// Get sync history for a ride
    fn get_sync_history(&self, ride_id: &Uuid) -> Vec<SyncRecord>;

    /// Cancel pending upload
    fn cancel(&self, record_id: &Uuid) -> bool;
}

/// Platform-specific API implementations
pub trait GarminConnect {
    async fn upload_fit(&self, fit_data: &[u8]) -> Result<String, SyncError>;
    async fn get_activity(&self, activity_id: &str) -> Result<GarminActivity, SyncError>;
}

pub trait StravaApi {
    async fn upload_activity(&self, fit_data: &[u8], name: &str) -> Result<String, SyncError>;
    async fn get_upload_status(&self, upload_id: &str) -> Result<UploadStatus, SyncError>;
}

#[cfg(target_os = "macos")]
pub trait HealthKitSync {
    async fn save_workout(&self, ride: &Ride) -> Result<(), SyncError>;
    async fn request_authorization(&self) -> Result<(), SyncError>;
}
```

### Credential Storage

```rust
/// Secure credential storage using OS keyring
pub trait CredentialStore: Send + Sync {
    /// Store OAuth tokens
    async fn store_tokens(&self, platform: SyncPlatform, tokens: &TokenResponse) -> Result<(), SyncError>;

    /// Retrieve OAuth tokens
    async fn get_tokens(&self, platform: SyncPlatform) -> Result<Option<TokenResponse>, SyncError>;

    /// Delete tokens
    async fn delete_tokens(&self, platform: SyncPlatform) -> Result<(), SyncError>;

    /// Check if credentials exist
    fn has_credentials(&self, platform: SyncPlatform) -> bool;
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Publish failed: {0}")]
    PublishFailed(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Broker error: {0}")]
    BrokerError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StreamingError {
    #[error("Server bind failed: {0}")]
    BindFailed(String),

    #[error("Server not running")]
    NotRunning,

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Platform not configured: {0:?}")]
    NotConfigured(SyncPlatform),

    #[error("Authorization required")]
    AuthorizationRequired,

    #[error("Token expired")]
    TokenExpired,

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Credential storage error: {0}")]
    CredentialError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
```

---

## Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub enabled: bool,
    pub broker_host: String,
    pub broker_port: u16,
    pub use_tls: bool,
    pub username: Option<String>,
    // password stored in keyring, not config
    pub client_id: String,
    pub reconnect_interval_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub port: u16,
    pub require_pin: bool,
    pub pin_expiry_minutes: u32,
    pub update_interval_ms: u32,
    pub metrics_to_stream: Vec<StreamMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub enabled: bool,
    // api_key stored in keyring
    pub latitude: f64,
    pub longitude: f64,
    pub units: WeatherUnits,
    pub refresh_interval_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub platforms: HashMap<SyncPlatform, PlatformConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub enabled: bool,
    pub auto_sync: bool,
}
```
