//! Weather Data Provider
//!
//! Fetches weather data from OpenWeatherMap API.

use super::{WeatherCondition, WeatherConfig, WeatherData, WeatherError, WeatherUnits};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// Default weather provider using OpenWeatherMap
pub struct OpenWeatherMapProvider {
    config: Arc<RwLock<WeatherConfig>>,
    api_key: Arc<RwLock<Option<String>>>,
    cached_data: Arc<RwLock<Option<WeatherData>>>,
    last_fetch: Arc<RwLock<Option<DateTime<Utc>>>>,
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

        tracing::debug!("Fetching weather data from OpenWeatherMap");

        // Make the HTTP request
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| WeatherError::NetworkError(e.to_string()))?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                return Err(WeatherError::RateLimited);
            }
            return Err(WeatherError::RequestFailed(format!(
                "HTTP {} - {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown error")
            )));
        }

        // Parse JSON response
        let owm_response: OwmResponse = response
            .json()
            .await
            .map_err(|e| WeatherError::InvalidResponse(format!("Failed to parse JSON: {}", e)))?;

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

        // Cache the result
        *self.cached_data.write().await = Some(weather_data.clone());
        *self.last_fetch.write().await = Some(Utc::now());

        Ok(weather_data)
    }

    /// Check if cache is valid
    async fn is_cache_valid(&self) -> bool {
        let config = self.config.read().await;
        let cached = self.cached_data.read().await;

        if let Some(data) = cached.as_ref() {
            !data.is_stale(config.refresh_interval_minutes)
        } else {
            false
        }
    }
}

impl WeatherProvider for OpenWeatherMapProvider {
    fn configure(&self, config: WeatherConfig) {
        if let Ok(mut c) = self.config.try_write() {
            *c = config;
        }
    }

    async fn get_weather(&self) -> Result<WeatherData, WeatherError> {
        // Check cache first
        if self.is_cache_valid().await {
            if let Some(data) = self.get_cached() {
                return Ok(data);
            }
        }

        // Fetch fresh data
        self.fetch_from_api().await
    }

    async fn refresh(&self) -> Result<WeatherData, WeatherError> {
        self.fetch_from_api().await
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
}
