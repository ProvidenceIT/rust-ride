//! Weather Integration
//!
//! Provides current weather data from external APIs.

pub mod provider;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export main types
pub use provider::WeatherProvider;

/// Weather-related errors
#[derive(Debug, Error)]
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
    NetworkError(String),
}

/// Weather configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// Whether weather is enabled
    pub enabled: bool,
    /// API key (stored in keyring in production)
    pub api_key_configured: bool,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Temperature units
    pub units: WeatherUnits,
    /// Refresh interval in minutes
    pub refresh_interval_minutes: u32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key_configured: false,
            latitude: 0.0,
            longitude: 0.0,
            units: WeatherUnits::Metric,
            refresh_interval_minutes: 30,
        }
    }
}

/// Temperature units
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherUnits {
    /// Celsius, km/h
    Metric,
    /// Fahrenheit, mph
    Imperial,
}

/// Current weather data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    /// Temperature in configured units
    pub temperature: f32,
    /// "Feels like" temperature
    pub feels_like: f32,
    /// Humidity percentage (0-100)
    pub humidity: u8,
    /// Weather condition
    pub condition: WeatherCondition,
    /// Weather description
    pub description: String,
    /// Wind speed in configured units
    pub wind_speed: f32,
    /// Wind direction in degrees
    pub wind_direction: u16,
    /// Atmospheric pressure in hPa
    pub pressure: u16,
    /// Visibility in meters
    pub visibility: u32,
    /// UV index
    pub uv_index: Option<f32>,
    /// When this data was fetched
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

/// Weather conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Overcast,
    Fog,
    LightRain,
    Rain,
    HeavyRain,
    Thunderstorm,
    Snow,
    Sleet,
    Hail,
    Windy,
}

impl WeatherCondition {
    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "☀️",
            WeatherCondition::PartlyCloudy => "⛅",
            WeatherCondition::Cloudy => "☁️",
            WeatherCondition::Overcast => "☁️",
            WeatherCondition::Fog => "🌫️",
            WeatherCondition::LightRain => "🌦️",
            WeatherCondition::Rain => "🌧️",
            WeatherCondition::HeavyRain => "🌧️",
            WeatherCondition::Thunderstorm => "⛈️",
            WeatherCondition::Snow => "🌨️",
            WeatherCondition::Sleet => "🌨️",
            WeatherCondition::Hail => "🌨️",
            WeatherCondition::Windy => "💨",
        }
    }

    /// Get icon name
    pub fn icon_name(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "sun",
            WeatherCondition::PartlyCloudy => "cloud-sun",
            WeatherCondition::Cloudy => "cloud",
            WeatherCondition::Overcast => "clouds",
            WeatherCondition::Fog => "fog",
            WeatherCondition::LightRain => "cloud-drizzle",
            WeatherCondition::Rain => "cloud-rain",
            WeatherCondition::HeavyRain => "cloud-rain-heavy",
            WeatherCondition::Thunderstorm => "cloud-lightning",
            WeatherCondition::Snow => "cloud-snow",
            WeatherCondition::Sleet => "cloud-sleet",
            WeatherCondition::Hail => "cloud-hail",
            WeatherCondition::Windy => "wind",
        }
    }
}

impl WeatherData {
    /// Format temperature with unit
    pub fn formatted_temperature(&self, units: WeatherUnits) -> String {
        match units {
            WeatherUnits::Metric => format!("{:.0}°C", self.temperature),
            WeatherUnits::Imperial => format!("{:.0}°F", self.temperature),
        }
    }

    /// Format wind speed with unit
    pub fn formatted_wind(&self, units: WeatherUnits) -> String {
        match units {
            WeatherUnits::Metric => format!("{:.0} km/h", self.wind_speed),
            WeatherUnits::Imperial => format!("{:.0} mph", self.wind_speed),
        }
    }

    /// Get wind direction as cardinal
    pub fn wind_cardinal(&self) -> &'static str {
        match self.wind_direction {
            0..=22 | 338..=360 => "N",
            23..=67 => "NE",
            68..=112 => "E",
            113..=157 => "SE",
            158..=202 => "S",
            203..=247 => "SW",
            248..=292 => "W",
            293..=337 => "NW",
            _ => "N",
        }
    }

    /// Check if data is stale (older than given minutes)
    pub fn is_stale(&self, max_age_minutes: u32) -> bool {
        let age = chrono::Utc::now() - self.fetched_at;
        age > chrono::Duration::minutes(max_age_minutes as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_condition_emoji() {
        assert_eq!(WeatherCondition::Clear.emoji(), "☀️");
        assert_eq!(WeatherCondition::Rain.emoji(), "🌧️");
    }

    #[test]
    fn test_wind_cardinal() {
        let data = WeatherData {
            temperature: 20.0,
            feels_like: 20.0,
            humidity: 50,
            condition: WeatherCondition::Clear,
            description: "Clear sky".to_string(),
            wind_speed: 10.0,
            wind_direction: 0,
            pressure: 1013,
            visibility: 10000,
            uv_index: None,
            fetched_at: chrono::Utc::now(),
        };

        assert_eq!(data.wind_cardinal(), "N");
    }

    #[test]
    fn test_formatted_temperature() {
        let data = WeatherData {
            temperature: 25.0,
            feels_like: 27.0,
            humidity: 60,
            condition: WeatherCondition::Clear,
            description: "Clear".to_string(),
            wind_speed: 5.0,
            wind_direction: 180,
            pressure: 1015,
            visibility: 10000,
            uv_index: Some(6.0),
            fetched_at: chrono::Utc::now(),
        };

        assert_eq!(data.formatted_temperature(WeatherUnits::Metric), "25°C");
        assert_eq!(data.formatted_temperature(WeatherUnits::Imperial), "25°F");
    }
}
