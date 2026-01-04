//! Weather Integration
//!
//! Provides current weather data from external APIs.

pub mod provider;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Import WeatherType for mapping from API conditions to 3D world types
use crate::world::weather::WeatherType;

// Re-export main types
pub use provider::{
    OpenWeatherMapProvider, WeatherProvider, WeatherRefreshHandle, WeatherRefreshScheduler,
};

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

    #[error("Credential error: {0}")]
    CredentialError(String),
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

    /// Convert to 3D world WeatherType for driving visual effects.
    ///
    /// Maps the 13 API weather conditions to the 6 world weather types:
    /// - Clear/Windy -> Clear (wind affects particles, not visibility)
    /// - PartlyCloudy/Cloudy/Overcast -> Cloudy
    /// - LightRain/Rain -> Rain
    /// - HeavyRain/Thunderstorm/Hail -> HeavyRain (intense precipitation)
    /// - Fog -> Fog
    /// - Snow/Sleet -> Snow (winter precipitation)
    pub fn to_weather_type(&self) -> WeatherType {
        match self {
            // Clear sky conditions (wind handled via wind_speed parameter)
            WeatherCondition::Clear | WeatherCondition::Windy => WeatherType::Clear,

            // Cloudy conditions (varying cloud coverage)
            WeatherCondition::PartlyCloudy
            | WeatherCondition::Cloudy
            | WeatherCondition::Overcast => WeatherType::Cloudy,

            // Light to moderate rain
            WeatherCondition::LightRain | WeatherCondition::Rain => WeatherType::Rain,

            // Heavy/intense precipitation (includes thunderstorms and hail)
            WeatherCondition::HeavyRain
            | WeatherCondition::Thunderstorm
            | WeatherCondition::Hail => WeatherType::HeavyRain,

            // Low visibility conditions
            WeatherCondition::Fog => WeatherType::Fog,

            // Winter precipitation (snow and sleet share snow effects)
            WeatherCondition::Snow | WeatherCondition::Sleet => WeatherType::Snow,
        }
    }
}

impl WeatherData {
    /// Create default weather data for fallback when API is unavailable.
    ///
    /// Returns clear weather with mild temperature:
    /// - 20°C (68°F) temperature
    /// - Clear sky
    /// - Calm wind (0 km/h)
    /// - 50% humidity
    /// - Normal atmospheric pressure
    ///
    /// # Arguments
    /// * `units` - The temperature units to use for the default values
    pub fn default_weather(units: WeatherUnits) -> Self {
        let (temperature, feels_like) = match units {
            WeatherUnits::Metric => (20.0, 20.0),   // 20°C
            WeatherUnits::Imperial => (68.0, 68.0), // 68°F
        };

        Self {
            temperature,
            feels_like,
            humidity: 50,
            condition: WeatherCondition::Clear,
            description: "Clear (default)".to_string(),
            wind_speed: 0.0, // Calm wind
            wind_direction: 0,
            pressure: 1013, // Standard atmospheric pressure
            visibility: 10000,
            uv_index: None,
            fetched_at: chrono::Utc::now(),
        }
    }

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

/// Service name used for keyring entries
const WEATHER_KEYRING_SERVICE: &str = "RustRide-Weather";

/// Keyring key for the OpenWeatherMap API key
const WEATHER_API_KEY_ENTRY: &str = "openweathermap-api-key";

/// Secure storage for weather API keys using the OS keyring.
///
/// This provides platform-specific secure storage:
/// - Windows: Windows Credential Manager
/// - macOS: macOS Keychain
/// - Linux: Secret Service (via libsecret)
pub struct WeatherCredentialStore {
    service_name: String,
}

impl Default for WeatherCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherCredentialStore {
    /// Create a new weather credential store with default service name.
    pub fn new() -> Self {
        Self {
            service_name: WEATHER_KEYRING_SERVICE.to_string(),
        }
    }

    /// Create a new weather credential store with a custom service name.
    /// Useful for testing or multiple instances.
    pub fn with_service_name(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Create a keyring entry for the API key.
    fn entry(&self) -> Result<keyring::Entry, WeatherError> {
        keyring::Entry::new(&self.service_name, WEATHER_API_KEY_ENTRY)
            .map_err(|e| WeatherError::CredentialError(format!("Failed to create keyring entry: {}", e)))
    }

    /// Store the OpenWeatherMap API key securely.
    ///
    /// # Arguments
    /// * `api_key` - The API key to store
    pub fn save_api_key(&self, api_key: &str) -> Result<(), WeatherError> {
        let entry = self.entry()?;
        entry
            .set_password(api_key)
            .map_err(|e| WeatherError::CredentialError(format!("Failed to store API key: {}", e)))?;

        tracing::debug!("Stored weather API key in OS keyring");
        Ok(())
    }

    /// Retrieve the OpenWeatherMap API key from secure storage.
    ///
    /// # Returns
    /// * `Ok(Some(key))` - API key was found
    /// * `Ok(None)` - No API key stored
    /// * `Err(WeatherError)` - An error occurred accessing the keyring
    pub fn load_api_key(&self) -> Result<Option<String>, WeatherError> {
        let entry = self.entry()?;

        match entry.get_password() {
            Ok(key) => {
                tracing::debug!("Retrieved weather API key from OS keyring");
                Ok(Some(key))
            }
            Err(keyring::Error::NoEntry) => {
                tracing::debug!("No weather API key found in OS keyring");
                Ok(None)
            }
            Err(e) => {
                tracing::error!("Failed to retrieve weather API key: {}", e);
                Err(WeatherError::CredentialError(format!(
                    "Failed to retrieve API key: {}",
                    e
                )))
            }
        }
    }

    /// Delete the stored API key.
    pub fn clear_api_key(&self) -> Result<(), WeatherError> {
        let entry = self.entry()?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted weather API key from OS keyring");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Already deleted or never existed - not an error
                tracing::debug!("No weather API key to delete");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to delete weather API key: {}", e);
                Err(WeatherError::CredentialError(format!(
                    "Failed to delete API key: {}",
                    e
                )))
            }
        }
    }

    /// Check if an API key is stored without retrieving it.
    pub fn has_api_key(&self) -> bool {
        match self.load_api_key() {
            Ok(Some(_)) => true,
            _ => false,
        }
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

    #[test]
    fn test_credential_store_default() {
        let store = WeatherCredentialStore::new();
        assert_eq!(store.service_name, WEATHER_KEYRING_SERVICE);
    }

    #[test]
    fn test_credential_store_custom_service() {
        let custom_name = "RustRide-Weather-Test";
        let store = WeatherCredentialStore::with_service_name(custom_name);
        assert_eq!(store.service_name, custom_name);
    }

    #[test]
    fn test_credential_store_default_impl() {
        let store = WeatherCredentialStore::default();
        assert_eq!(store.service_name, WEATHER_KEYRING_SERVICE);
    }

    #[test]
    fn test_credential_error_display() {
        let error = WeatherError::CredentialError("Test error".to_string());
        assert!(error.to_string().contains("Credential error"));
        assert!(error.to_string().contains("Test error"));
    }

    #[test]
    fn test_weather_condition_to_weather_type_clear() {
        use crate::world::weather::WeatherType;
        assert_eq!(WeatherCondition::Clear.to_weather_type(), WeatherType::Clear);
        assert_eq!(WeatherCondition::Windy.to_weather_type(), WeatherType::Clear);
    }

    #[test]
    fn test_weather_condition_to_weather_type_cloudy() {
        use crate::world::weather::WeatherType;
        assert_eq!(
            WeatherCondition::PartlyCloudy.to_weather_type(),
            WeatherType::Cloudy
        );
        assert_eq!(WeatherCondition::Cloudy.to_weather_type(), WeatherType::Cloudy);
        assert_eq!(
            WeatherCondition::Overcast.to_weather_type(),
            WeatherType::Cloudy
        );
    }

    #[test]
    fn test_weather_condition_to_weather_type_rain() {
        use crate::world::weather::WeatherType;
        assert_eq!(WeatherCondition::LightRain.to_weather_type(), WeatherType::Rain);
        assert_eq!(WeatherCondition::Rain.to_weather_type(), WeatherType::Rain);
    }

    #[test]
    fn test_weather_condition_to_weather_type_heavy_rain() {
        use crate::world::weather::WeatherType;
        assert_eq!(
            WeatherCondition::HeavyRain.to_weather_type(),
            WeatherType::HeavyRain
        );
        assert_eq!(
            WeatherCondition::Thunderstorm.to_weather_type(),
            WeatherType::HeavyRain
        );
        assert_eq!(WeatherCondition::Hail.to_weather_type(), WeatherType::HeavyRain);
    }

    #[test]
    fn test_weather_condition_to_weather_type_fog() {
        use crate::world::weather::WeatherType;
        assert_eq!(WeatherCondition::Fog.to_weather_type(), WeatherType::Fog);
    }

    #[test]
    fn test_weather_condition_to_weather_type_snow() {
        use crate::world::weather::WeatherType;
        assert_eq!(WeatherCondition::Snow.to_weather_type(), WeatherType::Snow);
        assert_eq!(WeatherCondition::Sleet.to_weather_type(), WeatherType::Snow);
    }

    #[test]
    fn test_weather_condition_all_variants_mapped() {
        use crate::world::weather::WeatherType;
        // Ensure all 13 variants map to valid WeatherType values
        let conditions = [
            WeatherCondition::Clear,
            WeatherCondition::PartlyCloudy,
            WeatherCondition::Cloudy,
            WeatherCondition::Overcast,
            WeatherCondition::Fog,
            WeatherCondition::LightRain,
            WeatherCondition::Rain,
            WeatherCondition::HeavyRain,
            WeatherCondition::Thunderstorm,
            WeatherCondition::Snow,
            WeatherCondition::Sleet,
            WeatherCondition::Hail,
            WeatherCondition::Windy,
        ];

        for condition in conditions {
            let weather_type = condition.to_weather_type();
            // Verify each maps to one of the 6 valid types
            assert!(matches!(
                weather_type,
                WeatherType::Clear
                    | WeatherType::Cloudy
                    | WeatherType::Rain
                    | WeatherType::HeavyRain
                    | WeatherType::Fog
                    | WeatherType::Snow
            ));
        }
    }

    #[test]
    fn test_default_weather_metric() {
        let weather = WeatherData::default_weather(WeatherUnits::Metric);

        // Check temperature is 20°C
        assert!((weather.temperature - 20.0).abs() < 0.1);
        assert!((weather.feels_like - 20.0).abs() < 0.1);

        // Check clear conditions
        assert_eq!(weather.condition, WeatherCondition::Clear);
        assert_eq!(weather.description, "Clear (default)");

        // Check calm wind
        assert!((weather.wind_speed - 0.0).abs() < 0.1);

        // Check other defaults
        assert_eq!(weather.humidity, 50);
        assert_eq!(weather.pressure, 1013);
        assert_eq!(weather.visibility, 10000);
    }

    #[test]
    fn test_default_weather_imperial() {
        let weather = WeatherData::default_weather(WeatherUnits::Imperial);

        // Check temperature is 68°F
        assert!((weather.temperature - 68.0).abs() < 0.1);
        assert!((weather.feels_like - 68.0).abs() < 0.1);

        // Check clear conditions
        assert_eq!(weather.condition, WeatherCondition::Clear);
    }

    #[test]
    fn test_default_weather_has_current_timestamp() {
        let before = chrono::Utc::now();
        let weather = WeatherData::default_weather(WeatherUnits::Metric);
        let after = chrono::Utc::now();

        // Timestamp should be between before and after
        assert!(weather.fetched_at >= before);
        assert!(weather.fetched_at <= after);
    }
}
