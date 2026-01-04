//! Weather Data Provider
//!
//! Fetches weather data from OpenWeatherMap API with background refresh support.

use super::{WeatherCondition, WeatherConfig, WeatherData, WeatherError, WeatherUnits};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// HTTP request timeout in seconds
const HTTP_TIMEOUT_SECS: u64 = 10;

/// Initial backoff delay in seconds after first failure
const INITIAL_BACKOFF_SECS: u64 = 60;

/// Maximum backoff delay in seconds (15 minutes)
const MAX_BACKOFF_SECS: u64 = 900;

/// Trait for weather providers
pub trait WeatherProvider: Send + Sync {
    /// Configure the provider
    fn configure(&self, config: WeatherConfig);

    /// Fetch current weather (uses cache if valid)
    fn get_weather(
        &self,
    ) -> impl std::future::Future<Output = Result<WeatherData, WeatherError>> + Send;

    /// Force refresh (ignore cache)
    fn refresh(
        &self,
    ) -> impl std::future::Future<Output = Result<WeatherData, WeatherError>> + Send;

    /// Check if weather data is available
    fn is_available(&self) -> bool;

    /// Get cached data (even if stale)
    fn get_cached(&self) -> Option<WeatherData>;

    /// Get last fetch time
    fn last_updated(&self) -> Option<DateTime<Utc>>;
}

/// OpenWeatherMap API response (simplified)
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct OwmResponse {
    main: OwmMain,
    weather: Vec<OwmWeather>,
    wind: OwmWind,
    visibility: Option<u32>,
    sys: Option<OwmSys>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct OwmMain {
    temp: f32,
    feels_like: f32,
    humidity: u8,
    pressure: u16,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct OwmWeather {
    id: u32,
    main: String,
    description: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct OwmWind {
    speed: f32,
    deg: Option<u16>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct OwmSys {
    country: Option<String>,
}

/// Tracks error state for exponential backoff
#[derive(Debug, Default)]
struct ErrorState {
    /// Time of last error
    last_error_time: Option<DateTime<Utc>>,
    /// Number of consecutive failures
    consecutive_failures: u32,
}

/// Default weather provider using OpenWeatherMap
pub struct OpenWeatherMapProvider {
    config: Arc<RwLock<WeatherConfig>>,
    api_key: Arc<RwLock<Option<String>>>,
    cached_data: Arc<RwLock<Option<WeatherData>>>,
    last_fetch: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Error state for exponential backoff
    error_state: Arc<RwLock<ErrorState>>,
}

impl Default for OpenWeatherMapProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenWeatherMapProvider {
    /// Create a new provider
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(WeatherConfig::default())),
            api_key: Arc::new(RwLock::new(None)),
            cached_data: Arc::new(RwLock::new(None)),
            last_fetch: Arc::new(RwLock::new(None)),
            error_state: Arc::new(RwLock::new(ErrorState::default())),
        }
    }

    /// Set API key (typically loaded from keyring)
    pub async fn set_api_key(&self, key: String) {
        *self.api_key.write().await = Some(key);
    }

    /// Map OpenWeatherMap condition code to our condition
    fn map_condition(code: u32) -> WeatherCondition {
        match code {
            200..=232 => WeatherCondition::Thunderstorm,
            300..=321 => WeatherCondition::LightRain,
            500..=504 => WeatherCondition::Rain,
            511 => WeatherCondition::Sleet,
            520..=531 => WeatherCondition::HeavyRain,
            600..=622 => WeatherCondition::Snow,
            701..=762 => WeatherCondition::Fog,
            771 | 781 => WeatherCondition::Windy,
            800 => WeatherCondition::Clear,
            801 => WeatherCondition::PartlyCloudy,
            802 => WeatherCondition::Cloudy,
            803..=804 => WeatherCondition::Overcast,
            _ => WeatherCondition::Clear,
        }
    }

    /// Build API URL
    fn build_url(&self, config: &WeatherConfig, api_key: &str) -> String {
        let units = match config.units {
            WeatherUnits::Metric => "metric",
            WeatherUnits::Imperial => "imperial",
        };

        format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&units={}&appid={}",
            config.latitude, config.longitude, units, api_key
        )
    }

    /// Fetch from API
    async fn fetch_from_api(&self) -> Result<WeatherData, WeatherError> {
        let config = self.config.read().await;
        let api_key = self.api_key.read().await;

        let api_key = api_key.as_ref().ok_or(WeatherError::ApiKeyMissing)?;

        if config.latitude == 0.0 && config.longitude == 0.0 {
            return Err(WeatherError::LocationMissing);
        }

        let url = self.build_url(&config, api_key);

        tracing::info!(
            lat = config.latitude,
            lon = config.longitude,
            units = ?config.units,
            "Fetching weather data from OpenWeatherMap"
        );

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to build HTTP client");
                WeatherError::NetworkError(format!("Failed to build HTTP client: {}", e))
            })?;

        // Make the HTTP request with timing
        let start_time = std::time::Instant::now();
        let response = client.get(&url).send().await.map_err(|e| {
            let elapsed = start_time.elapsed();
            if e.is_timeout() {
                tracing::warn!(
                    elapsed_ms = elapsed.as_millis() as u64,
                    timeout_secs = HTTP_TIMEOUT_SECS,
                    "Weather API request timed out"
                );
                WeatherError::NetworkError(format!(
                    "Request timed out after {} seconds",
                    HTTP_TIMEOUT_SECS
                ))
            } else if e.is_connect() {
                tracing::warn!(
                    error = %e,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "Failed to connect to weather API"
                );
                WeatherError::NetworkError(format!("Connection failed: {}", e))
            } else {
                tracing::warn!(
                    error = %e,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "Weather API network error"
                );
                WeatherError::NetworkError(e.to_string())
            }
        })?;

        let elapsed = start_time.elapsed();
        let status = response.status();

        tracing::debug!(
            status = status.as_u16(),
            elapsed_ms = elapsed.as_millis() as u64,
            "Weather API response received"
        );

        // Check for HTTP errors
        if !status.is_success() {
            if status.as_u16() == 429 {
                tracing::warn!("Weather API rate limit exceeded (HTTP 429)");
                return Err(WeatherError::RateLimited);
            }
            let reason = status.canonical_reason().unwrap_or("Unknown error");
            tracing::error!(
                status = status.as_u16(),
                reason = reason,
                "Weather API request failed"
            );
            return Err(WeatherError::RequestFailed(format!(
                "HTTP {} - {}",
                status.as_u16(),
                reason
            )));
        }

        // Parse JSON response
        let owm_response: OwmResponse = response.json().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to parse weather API JSON response");
            WeatherError::InvalidResponse(format!("Failed to parse JSON: {}", e))
        })?;

        // Extract condition from first weather entry
        let condition = owm_response
            .weather
            .first()
            .map(|w| Self::map_condition(w.id))
            .unwrap_or(WeatherCondition::Clear);

        let description = owm_response
            .weather
            .first()
            .map(|w| w.description.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Map to WeatherData
        let weather_data = WeatherData {
            temperature: owm_response.main.temp,
            feels_like: owm_response.main.feels_like,
            humidity: owm_response.main.humidity,
            condition,
            description,
            wind_speed: owm_response.wind.speed,
            wind_direction: owm_response.wind.deg.unwrap_or(0),
            pressure: owm_response.main.pressure,
            visibility: owm_response.visibility.unwrap_or(10000),
            uv_index: None, // UV index requires separate API call
            fetched_at: Utc::now(),
        };

        tracing::debug!(
            "Weather fetched: {}°, {:?}",
            weather_data.temperature,
            weather_data.condition
        );

        // Cache the result and reset error state on success
        *self.cached_data.write().await = Some(weather_data.clone());
        *self.last_fetch.write().await = Some(Utc::now());
        self.record_success().await;

        Ok(weather_data)
    }

    /// Check if cache is valid (respects refresh_interval_minutes from config)
    async fn is_cache_valid(&self) -> bool {
        let config = self.config.read().await;
        let cached = self.cached_data.read().await;

        if let Some(data) = cached.as_ref() {
            let is_fresh = !data.is_stale(config.refresh_interval_minutes);
            if is_fresh {
                tracing::trace!(
                    refresh_interval = config.refresh_interval_minutes,
                    fetched_at = %data.fetched_at,
                    "Cache is still valid"
                );
            }
            is_fresh
        } else {
            false
        }
    }

    /// Calculate exponential backoff delay based on consecutive failures.
    /// Returns delay in seconds: 60, 120, 240, 480, 900 (capped at 15 min)
    fn calculate_backoff_secs(consecutive_failures: u32) -> u64 {
        if consecutive_failures == 0 {
            return 0;
        }
        // Exponential backoff: initial_delay * 2^(failures-1)
        // 60, 120, 240, 480, 960 -> capped at 900
        let backoff = INITIAL_BACKOFF_SECS.saturating_mul(1 << (consecutive_failures - 1).min(10));
        backoff.min(MAX_BACKOFF_SECS)
    }

    /// Check if we should retry based on exponential backoff
    async fn should_retry(&self) -> bool {
        let error_state = self.error_state.read().await;

        // If no errors, always allow retry
        if error_state.consecutive_failures == 0 {
            return true;
        }

        let Some(last_error) = error_state.last_error_time else {
            return true;
        };

        let backoff_secs = Self::calculate_backoff_secs(error_state.consecutive_failures);
        let elapsed = (Utc::now() - last_error).num_seconds() as u64;

        if elapsed >= backoff_secs {
            tracing::debug!(
                consecutive_failures = error_state.consecutive_failures,
                backoff_secs = backoff_secs,
                elapsed_secs = elapsed,
                "Backoff period expired, allowing retry"
            );
            true
        } else {
            tracing::trace!(
                consecutive_failures = error_state.consecutive_failures,
                backoff_secs = backoff_secs,
                elapsed_secs = elapsed,
                remaining_secs = backoff_secs - elapsed,
                "Still in backoff period"
            );
            false
        }
    }

    /// Record a successful fetch, resetting error state
    async fn record_success(&self) {
        let mut error_state = self.error_state.write().await;
        if error_state.consecutive_failures > 0 {
            tracing::debug!(
                previous_failures = error_state.consecutive_failures,
                "Weather fetch succeeded, resetting error state"
            );
        }
        error_state.last_error_time = None;
        error_state.consecutive_failures = 0;
    }

    /// Record a failure for exponential backoff
    async fn record_failure(&self, error: &WeatherError) {
        let mut error_state = self.error_state.write().await;
        error_state.consecutive_failures = error_state.consecutive_failures.saturating_add(1);
        error_state.last_error_time = Some(Utc::now());

        let next_backoff = Self::calculate_backoff_secs(error_state.consecutive_failures);
        tracing::warn!(
            error = %error,
            consecutive_failures = error_state.consecutive_failures,
            next_retry_in_secs = next_backoff,
            "Weather API failure recorded, applying exponential backoff"
        );
    }

    /// Get the current consecutive failure count (for testing/monitoring)
    #[allow(dead_code)]
    pub async fn consecutive_failures(&self) -> u32 {
        self.error_state.read().await.consecutive_failures
    }
}

impl WeatherProvider for OpenWeatherMapProvider {
    fn configure(&self, config: WeatherConfig) {
        if let Ok(mut c) = self.config.try_write() {
            *c = config;
        }
    }

    async fn get_weather(&self) -> Result<WeatherData, WeatherError> {
        // Check cache first - respects refresh_interval_minutes
        if self.is_cache_valid().await {
            if let Some(data) = self.get_cached() {
                tracing::trace!("Returning cached weather data");
                return Ok(data);
            }
        }

        // Check if we're in a backoff period due to previous failures
        if !self.should_retry().await {
            // If we have cached data (even stale), return it during backoff
            if let Some(data) = self.get_cached() {
                tracing::debug!(
                    "Returning stale cached data during backoff period"
                );
                return Ok(data);
            }
            // No cached data available and in backoff - return error
            return Err(WeatherError::NetworkError(
                "In backoff period after previous failures".to_string(),
            ));
        }

        // Fetch fresh data
        match self.fetch_from_api().await {
            Ok(data) => Ok(data),
            Err(error) => {
                self.record_failure(&error).await;
                Err(error)
            }
        }
    }

    async fn refresh(&self) -> Result<WeatherData, WeatherError> {
        // Force refresh - check backoff but don't use cache
        if !self.should_retry().await {
            return Err(WeatherError::NetworkError(
                "In backoff period after previous failures".to_string(),
            ));
        }

        match self.fetch_from_api().await {
            Ok(data) => Ok(data),
            Err(error) => {
                self.record_failure(&error).await;
                Err(error)
            }
        }
    }

    fn is_available(&self) -> bool {
        self.api_key
            .try_read()
            .map(|k| k.is_some())
            .unwrap_or(false)
            && self.config.try_read().map(|c| c.enabled).unwrap_or(false)
    }

    fn get_cached(&self) -> Option<WeatherData> {
        self.cached_data.try_read().ok()?.clone()
    }

    fn last_updated(&self) -> Option<DateTime<Utc>> {
        *self.last_fetch.try_read().ok()?
    }
}

/// Handle for controlling the weather refresh scheduler.
///
/// This handle can be used to stop the background refresh task gracefully.
/// When dropped, the background task will continue running until explicitly stopped.
#[derive(Clone)]
pub struct WeatherRefreshHandle {
    /// Cancellation token to signal shutdown
    cancel_token: CancellationToken,
}

impl WeatherRefreshHandle {
    /// Stop the background refresh task gracefully.
    ///
    /// This signals the background task to stop at the next opportunity.
    /// The method returns immediately; the actual task may take up to one
    /// refresh interval to stop.
    pub fn stop(&self) {
        tracing::info!("Weather refresh scheduler stopping");
        self.cancel_token.cancel();
    }

    /// Check if the scheduler has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

/// Background scheduler for periodically refreshing weather data.
///
/// This scheduler spawns a background tokio task that refreshes weather data
/// at the configured interval. It respects the weather enabled setting and
/// handles shutdown gracefully.
///
/// # Example
///
/// ```ignore
/// let provider = Arc::new(OpenWeatherMapProvider::new());
/// let config = WeatherConfig::default();
/// provider.set_api_key("your_api_key".to_string()).await;
///
/// let handle = WeatherRefreshScheduler::spawn(provider, config);
///
/// // ... later, when shutting down
/// handle.stop();
/// ```
pub struct WeatherRefreshScheduler;

impl WeatherRefreshScheduler {
    /// Spawn a background task that periodically refreshes weather data.
    ///
    /// The task will:
    /// - Wait for the configured refresh interval between fetches
    /// - Only fetch when weather is enabled in the config
    /// - Stop gracefully when the handle's `stop()` method is called
    /// - Continue running even if individual fetches fail (with exponential backoff)
    ///
    /// # Arguments
    ///
    /// * `provider` - The weather provider to use for fetching data
    /// * `config` - Initial weather configuration
    ///
    /// # Returns
    ///
    /// A handle that can be used to stop the background task.
    pub fn spawn(
        provider: Arc<OpenWeatherMapProvider>,
        config: WeatherConfig,
    ) -> WeatherRefreshHandle {
        let cancel_token = CancellationToken::new();
        let handle = WeatherRefreshHandle {
            cancel_token: cancel_token.clone(),
        };

        // Configure the provider with the initial config
        provider.configure(config.clone());

        // Calculate the refresh interval
        let refresh_interval = Duration::from_secs(config.refresh_interval_minutes as u64 * 60);

        tracing::info!(
            refresh_interval_mins = config.refresh_interval_minutes,
            enabled = config.enabled,
            "Weather refresh scheduler starting"
        );

        // Spawn the background refresh task
        tokio::spawn(Self::refresh_loop(
            provider,
            refresh_interval,
            cancel_token,
        ));

        handle
    }

    /// The main refresh loop that runs in the background.
    async fn refresh_loop(
        provider: Arc<OpenWeatherMapProvider>,
        refresh_interval: Duration,
        cancel_token: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(refresh_interval);

        // Consume the first tick immediately (interval ticks immediately on first call)
        interval.tick().await;

        loop {
            tokio::select! {
                // Wait for cancellation
                _ = cancel_token.cancelled() => {
                    tracing::info!("Weather refresh scheduler stopped");
                    break;
                }

                // Wait for the next tick
                _ = interval.tick() => {
                    // Check if weather is enabled before fetching
                    if !provider.is_available() {
                        tracing::trace!("Weather refresh skipped: not available (disabled or no API key)");
                        continue;
                    }

                    // Attempt to refresh weather data
                    tracing::debug!("Weather refresh scheduler triggering background refresh");
                    match provider.refresh().await {
                        Ok(data) => {
                            tracing::debug!(
                                temp = data.temperature,
                                condition = ?data.condition,
                                "Weather data refreshed successfully"
                            );
                        }
                        Err(e) => {
                            // Errors are already logged and backoff is handled by the provider
                            tracing::trace!(
                                error = %e,
                                "Weather refresh failed (backoff handled by provider)"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Spawn a background task with a custom interval for testing.
    ///
    /// This method allows specifying a custom interval instead of reading from config,
    /// which is useful for testing with shorter intervals.
    #[cfg(test)]
    pub fn spawn_with_interval(
        provider: Arc<OpenWeatherMapProvider>,
        interval: Duration,
        enabled: bool,
    ) -> WeatherRefreshHandle {
        let cancel_token = CancellationToken::new();
        let handle = WeatherRefreshHandle {
            cancel_token: cancel_token.clone(),
        };

        // Configure the provider with minimal test config
        let config = WeatherConfig {
            enabled,
            api_key_configured: enabled,
            latitude: 51.5074,
            longitude: -0.1278,
            ..Default::default()
        };
        provider.configure(config);

        tracing::debug!(
            interval_secs = interval.as_secs(),
            enabled = enabled,
            "Weather refresh scheduler starting (test mode)"
        );

        tokio::spawn(Self::refresh_loop(
            provider,
            interval,
            cancel_token,
        ));

        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_mapping() {
        assert_eq!(
            OpenWeatherMapProvider::map_condition(800),
            WeatherCondition::Clear
        );
        assert_eq!(
            OpenWeatherMapProvider::map_condition(500),
            WeatherCondition::Rain
        );
        assert_eq!(
            OpenWeatherMapProvider::map_condition(200),
            WeatherCondition::Thunderstorm
        );
    }

    #[test]
    fn test_provider_creation() {
        let provider = OpenWeatherMapProvider::new();
        assert!(!provider.is_available());
        assert!(provider.get_cached().is_none());
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        // No failures = no backoff
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(0), 0);

        // First failure = 60 seconds (1 minute)
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(1), 60);

        // Second failure = 120 seconds (2 minutes)
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(2), 120);

        // Third failure = 240 seconds (4 minutes)
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(3), 240);

        // Fourth failure = 480 seconds (8 minutes)
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(4), 480);

        // Fifth failure = capped at 900 seconds (15 minutes)
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(5), 900);

        // Further failures stay capped at 900 seconds
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(10), 900);
        assert_eq!(OpenWeatherMapProvider::calculate_backoff_secs(100), 900);
    }

    #[tokio::test]
    async fn test_error_state_tracking() {
        let provider = OpenWeatherMapProvider::new();

        // Initially no failures
        assert_eq!(provider.consecutive_failures().await, 0);

        // Record a failure
        let error = WeatherError::NetworkError("test error".to_string());
        provider.record_failure(&error).await;
        assert_eq!(provider.consecutive_failures().await, 1);

        // Record another failure
        provider.record_failure(&error).await;
        assert_eq!(provider.consecutive_failures().await, 2);

        // Success resets failure count
        provider.record_success().await;
        assert_eq!(provider.consecutive_failures().await, 0);
    }

    #[tokio::test]
    async fn test_should_retry_no_failures() {
        let provider = OpenWeatherMapProvider::new();

        // Should always retry when no failures
        assert!(provider.should_retry().await);
    }

    #[tokio::test]
    async fn test_should_retry_during_backoff() {
        let provider = OpenWeatherMapProvider::new();

        // Record a failure
        let error = WeatherError::NetworkError("test error".to_string());
        provider.record_failure(&error).await;

        // Should NOT retry immediately (within backoff period)
        assert!(!provider.should_retry().await);
    }

    // ========== WeatherRefreshScheduler Tests ==========

    #[test]
    fn test_refresh_handle_stop() {
        // Create a cancellation token to verify behavior
        let cancel_token = CancellationToken::new();
        let handle = WeatherRefreshHandle {
            cancel_token: cancel_token.clone(),
        };

        // Initially not stopped
        assert!(!handle.is_stopped());

        // Stop the handle
        handle.stop();

        // Now should be stopped
        assert!(handle.is_stopped());
        assert!(cancel_token.is_cancelled());
    }

    #[test]
    fn test_refresh_handle_clone() {
        let cancel_token = CancellationToken::new();
        let handle = WeatherRefreshHandle {
            cancel_token: cancel_token.clone(),
        };

        let cloned = handle.clone();

        // Stopping original should also stop clone (same underlying token)
        handle.stop();
        assert!(cloned.is_stopped());
    }

    #[tokio::test]
    async fn test_scheduler_spawn_and_stop() {
        let provider = Arc::new(OpenWeatherMapProvider::new());

        // Use a short interval for testing
        let handle = WeatherRefreshScheduler::spawn_with_interval(
            provider,
            Duration::from_millis(50),
            false, // disabled, so won't actually fetch
        );

        // Give the scheduler a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Initially not stopped
        assert!(!handle.is_stopped());

        // Stop the scheduler
        handle.stop();

        // Should be marked as stopped
        assert!(handle.is_stopped());

        // Give the task time to actually stop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_scheduler_respects_enabled_flag() {
        let provider = Arc::new(OpenWeatherMapProvider::new());

        // Start scheduler with weather disabled
        let handle = WeatherRefreshScheduler::spawn_with_interval(
            Arc::clone(&provider),
            Duration::from_millis(50),
            false, // disabled
        );

        // Wait for a few intervals
        tokio::time::sleep(Duration::from_millis(150)).await;

        // No data should be cached because weather is disabled
        assert!(provider.get_cached().is_none());

        handle.stop();
    }

    #[tokio::test]
    async fn test_scheduler_spawn_with_config() {
        let provider = Arc::new(OpenWeatherMapProvider::new());

        let config = WeatherConfig {
            enabled: false,
            api_key_configured: false,
            latitude: 40.7128,
            longitude: -74.0060,
            refresh_interval_minutes: 15,
            ..Default::default()
        };

        let handle = WeatherRefreshScheduler::spawn(Arc::clone(&provider), config);

        // Give the scheduler a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Initially not stopped
        assert!(!handle.is_stopped());

        // Stop the scheduler
        handle.stop();
        assert!(handle.is_stopped());
    }

    #[tokio::test]
    async fn test_scheduler_graceful_shutdown() {
        let provider = Arc::new(OpenWeatherMapProvider::new());

        let handle = WeatherRefreshScheduler::spawn_with_interval(
            provider,
            Duration::from_secs(60), // long interval
            false,
        );

        // Stop immediately - should not wait for the full interval
        let start = std::time::Instant::now();
        handle.stop();

        // Wait for task to acknowledge cancellation
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should have stopped quickly, not waited 60 seconds
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(handle.is_stopped());
    }
}
