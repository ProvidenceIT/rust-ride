//! Weather system for dynamic weather effects and time-of-day simulation.

pub mod particles;
pub mod skybox;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Import weather provider types for the bridge
use crate::integrations::weather::{WeatherData, WeatherProvider, WeatherUnits};

/// Weather condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WeatherType {
    #[default]
    Clear,
    Cloudy,
    Rain,
    HeavyRain,
    Fog,
    Snow,
}

impl WeatherType {
    /// Get visibility distance in meters for this weather type
    pub fn visibility(&self) -> f32 {
        match self {
            Self::Clear => 10000.0,
            Self::Cloudy => 8000.0,
            Self::Rain => 3000.0,
            Self::HeavyRain => 1000.0,
            Self::Fog => 200.0,
            Self::Snow => 2000.0,
        }
    }

    /// Get particle density for this weather type (0.0-1.0)
    pub fn particle_density(&self) -> f32 {
        match self {
            Self::Clear => 0.0,
            Self::Cloudy => 0.0,
            Self::Rain => 0.5,
            Self::HeavyRain => 1.0,
            Self::Fog => 0.3,
            Self::Snow => 0.6,
        }
    }
}

/// Time of day period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeOfDay {
    Dawn,
    #[default]
    Day,
    Dusk,
    Night,
}

impl TimeOfDay {
    /// Get time of day from hours (0.0-24.0)
    pub fn from_hours(hours: f32) -> Self {
        let hours = hours % 24.0;
        match hours {
            h if (5.0..7.0).contains(&h) => Self::Dawn,
            h if (7.0..17.0).contains(&h) => Self::Day,
            h if (17.0..19.0).contains(&h) => Self::Dusk,
            _ => Self::Night,
        }
    }

    /// Get ambient light intensity (0.0-1.0)
    /// Ordering: Day (1.0) > Dawn (0.6) > Dusk (0.4) > Night (0.1)
    pub fn ambient_intensity(&self) -> f32 {
        match self {
            Self::Dawn => 0.6,
            Self::Day => 1.0,
            Self::Dusk => 0.4,
            Self::Night => 0.1,
        }
    }
}

/// Complete weather state for the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherState {
    /// Current weather type
    pub weather: WeatherType,
    /// Weather transition progress (0.0 = previous, 1.0 = current)
    pub transition_progress: f32,
    /// Previous weather (for transitions)
    pub previous_weather: Option<WeatherType>,
    /// Current time of day
    pub time_of_day: TimeOfDay,
    /// Exact time (0.0-24.0 hours)
    pub time_hours: f32,
    /// Whether time progresses realistically
    pub realistic_time: bool,
    /// Visibility distance in meters (affected by fog/rain)
    pub visibility_meters: f32,
    /// Wind speed in km/h (for visual effects)
    pub wind_speed_kmh: f32,
    /// Wind direction in degrees (0 = north)
    pub wind_direction_degrees: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            weather: WeatherType::default(),
            transition_progress: 1.0,
            previous_weather: None,
            time_of_day: TimeOfDay::default(),
            time_hours: 12.0,
            realistic_time: false,
            visibility_meters: 10000.0,
            wind_speed_kmh: 5.0,
            wind_direction_degrees: 0.0,
        }
    }
}

impl WeatherState {
    /// Create a new weather state with specific conditions
    pub fn new(weather: WeatherType, time_hours: f32) -> Self {
        let time_of_day = TimeOfDay::from_hours(time_hours);
        Self {
            weather,
            transition_progress: 1.0,
            previous_weather: None,
            time_of_day,
            time_hours,
            realistic_time: false,
            visibility_meters: weather.visibility(),
            wind_speed_kmh: 5.0,
            wind_direction_degrees: 0.0,
        }
    }

    /// Start transitioning to a new weather type
    pub fn transition_to(&mut self, new_weather: WeatherType) {
        if new_weather != self.weather {
            self.previous_weather = Some(self.weather);
            self.weather = new_weather;
            self.transition_progress = 0.0;
        }
    }

    /// Update the weather state
    pub fn update(&mut self, delta_time: f32) {
        // Update transition progress
        if self.transition_progress < 1.0 {
            self.transition_progress = (self.transition_progress + delta_time / 30.0).min(1.0);

            // Interpolate visibility
            if let Some(prev) = self.previous_weather {
                let prev_vis = prev.visibility();
                let curr_vis = self.weather.visibility();
                self.visibility_meters =
                    prev_vis + (curr_vis - prev_vis) * self.transition_progress;
            }
        } else {
            self.visibility_meters = self.weather.visibility();
        }

        // Update time if realistic time is enabled
        if self.realistic_time {
            // Time passes at 60x real speed (1 minute real = 1 hour in-game)
            self.time_hours = (self.time_hours + delta_time / 60.0) % 24.0;
            self.time_of_day = TimeOfDay::from_hours(self.time_hours);
        }
    }

    /// Set exact time of day
    pub fn set_time(&mut self, hours: f32) {
        self.time_hours = hours % 24.0;
        self.time_of_day = TimeOfDay::from_hours(self.time_hours);
    }

    /// Get current particle density (accounting for transition)
    pub fn current_particle_density(&self) -> f32 {
        if let Some(prev) = self.previous_weather {
            let prev_density = prev.particle_density();
            let curr_density = self.weather.particle_density();
            prev_density + (curr_density - prev_density) * self.transition_progress
        } else {
            self.weather.particle_density()
        }
    }
}

/// Weather controller manages weather state and rendering
pub struct WeatherController {
    state: WeatherState,
    auto_weather_enabled: bool,
    weather_change_timer: f32,
    weather_change_interval: f32,
}

impl WeatherController {
    /// Create a new weather controller
    pub fn new() -> Self {
        Self {
            state: WeatherState::default(),
            auto_weather_enabled: false,
            weather_change_timer: 0.0,
            weather_change_interval: 300.0, // 5 minutes
        }
    }

    /// Get current weather state
    pub fn state(&self) -> &WeatherState {
        &self.state
    }

    /// Get mutable reference to state
    pub fn state_mut(&mut self) -> &mut WeatherState {
        &mut self.state
    }

    /// Set weather type (starts transition)
    pub fn set_weather(&mut self, weather: WeatherType) {
        self.state.transition_to(weather);
    }

    /// Set time of day
    pub fn set_time(&mut self, hours: f32) {
        self.state.set_time(hours);
    }

    /// Enable/disable realistic time progression
    pub fn set_realistic_time(&mut self, enabled: bool) {
        self.state.realistic_time = enabled;
    }

    /// Enable/disable automatic weather changes
    pub fn set_auto_weather(&mut self, enabled: bool) {
        self.auto_weather_enabled = enabled;
    }

    /// Update weather state
    pub fn update(&mut self, delta_time: f32) {
        self.state.update(delta_time);

        // Handle automatic weather changes
        if self.auto_weather_enabled {
            self.weather_change_timer += delta_time;
            if self.weather_change_timer >= self.weather_change_interval {
                self.weather_change_timer = 0.0;
                self.random_weather_change();
            }
        }
    }

    /// Change to a random weather type
    fn random_weather_change(&mut self) {
        // Simple random selection (in production, use probability-based selection)
        let current = self.state.weather;
        let next = match current {
            WeatherType::Clear => WeatherType::Cloudy,
            WeatherType::Cloudy => WeatherType::Rain,
            WeatherType::Rain => WeatherType::Clear,
            WeatherType::HeavyRain => WeatherType::Rain,
            WeatherType::Fog => WeatherType::Clear,
            WeatherType::Snow => WeatherType::Cloudy,
        };
        self.set_weather(next);
    }
}

impl Default for WeatherController {
    fn default() -> Self {
        Self::new()
    }
}

/// Bridge between weather API data and the 3D world rendering system.
///
/// The `WeatherBridge` connects fetched weather data from external APIs
/// (like OpenWeatherMap) to the `WeatherController`, updating the 3D world
/// state when new data arrives. It handles:
///
/// - Converting API weather conditions to world weather types
/// - Converting wind speed units (m/s or mph) to km/h for the world
/// - Triggering smooth weather transition animations
///
/// # Example
///
/// ```ignore
/// let provider = Arc::new(OpenWeatherMapProvider::new());
/// let mut controller = WeatherController::new();
/// let bridge = WeatherBridge::new(provider.clone(), WeatherUnits::Metric);
///
/// // Sync weather data to the controller
/// if let Err(e) = bridge.sync(&mut controller).await {
///     tracing::warn!("Failed to sync weather: {}", e);
/// }
/// ```
pub struct WeatherBridge<P: WeatherProvider> {
    /// Weather data provider (e.g., OpenWeatherMapProvider)
    provider: Arc<P>,
    /// Units used by the provider for wind speed conversion
    units: WeatherUnits,
    /// Last synced weather type (to detect changes)
    last_weather_type: Option<WeatherType>,
}

impl<P: WeatherProvider> WeatherBridge<P> {
    /// Create a new weather bridge with the given provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The weather data provider to use
    /// * `units` - The units the provider returns (affects wind speed conversion)
    pub fn new(provider: Arc<P>, units: WeatherUnits) -> Self {
        Self {
            provider,
            units,
            last_weather_type: None,
        }
    }

    /// Sync weather data from the provider to the controller.
    ///
    /// This method:
    /// 1. Fetches current weather data from the provider
    /// 2. Converts the weather condition to a world WeatherType
    /// 3. Applies wind speed (converted to km/h) and direction
    /// 4. Triggers a weather transition if the condition changed
    ///
    /// # Arguments
    ///
    /// * `controller` - The weather controller to update
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if weather was synced and changed,
    /// `Ok(false)` if synced but unchanged, or an error if fetch failed.
    pub async fn sync(
        &mut self,
        controller: &mut WeatherController,
    ) -> Result<bool, crate::integrations::weather::WeatherError> {
        // Fetch current weather data
        let weather_data = self.provider.get_weather().await?;

        // Apply weather data to controller
        let changed = self.apply_weather_data(controller, &weather_data);

        Ok(changed)
    }

    /// Sync using cached data only (no API call).
    ///
    /// This is useful for initial sync when you don't want to trigger
    /// a fresh API call, or when operating in offline mode.
    ///
    /// # Arguments
    ///
    /// * `controller` - The weather controller to update
    ///
    /// # Returns
    ///
    /// Returns `Some(true)` if cached data was applied and weather changed,
    /// `Some(false)` if applied but unchanged, or `None` if no cached data.
    pub fn sync_from_cache(&mut self, controller: &mut WeatherController) -> Option<bool> {
        let weather_data = self.provider.get_cached()?;
        Some(self.apply_weather_data(controller, &weather_data))
    }

    /// Apply weather data to the controller, returning whether weather type changed.
    fn apply_weather_data(
        &mut self,
        controller: &mut WeatherController,
        weather_data: &WeatherData,
    ) -> bool {
        // Convert weather condition to world weather type
        let weather_type = weather_data.condition.to_weather_type();

        // Convert wind speed to km/h
        let wind_speed_kmh = self.convert_wind_speed(weather_data.wind_speed);

        // Apply wind data to the state
        let state = controller.state_mut();
        state.wind_speed_kmh = wind_speed_kmh;
        state.wind_direction_degrees = weather_data.wind_direction as f32;

        // Check if weather type changed (triggers transition animation)
        let weather_changed = self.last_weather_type != Some(weather_type);
        if weather_changed {
            tracing::info!(
                previous = ?self.last_weather_type,
                new = ?weather_type,
                wind_kmh = wind_speed_kmh,
                wind_dir = weather_data.wind_direction,
                "Weather condition changed, triggering transition"
            );
            controller.set_weather(weather_type);
            self.last_weather_type = Some(weather_type);
        } else {
            tracing::debug!(
                weather = ?weather_type,
                wind_kmh = wind_speed_kmh,
                wind_dir = weather_data.wind_direction,
                "Weather synced (no change)"
            );
        }

        weather_changed
    }

    /// Convert wind speed from provider units to km/h.
    ///
    /// OpenWeatherMap returns:
    /// - Metric: m/s (multiply by 3.6 to get km/h)
    /// - Imperial: mph (multiply by 1.60934 to get km/h)
    fn convert_wind_speed(&self, wind_speed: f32) -> f32 {
        match self.units {
            WeatherUnits::Metric => wind_speed * 3.6, // m/s to km/h
            WeatherUnits::Imperial => wind_speed * 1.60934, // mph to km/h
        }
    }

    /// Get the current weather type (if synced).
    pub fn current_weather_type(&self) -> Option<WeatherType> {
        self.last_weather_type
    }

    /// Get a reference to the provider.
    pub fn provider(&self) -> &Arc<P> {
        &self.provider
    }

    /// Update the units setting (e.g., if user changes preference).
    pub fn set_units(&mut self, units: WeatherUnits) {
        self.units = units;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_state_default() {
        let state = WeatherState::default();
        assert_eq!(state.weather, WeatherType::Clear);
        assert_eq!(state.time_of_day, TimeOfDay::Day);
        assert!((state.time_hours - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_weather_transition() {
        let mut state = WeatherState::default();
        state.transition_to(WeatherType::Rain);
        assert_eq!(state.weather, WeatherType::Rain);
        assert_eq!(state.previous_weather, Some(WeatherType::Clear));
        assert!((state.transition_progress - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_time_of_day_from_hours() {
        assert_eq!(TimeOfDay::from_hours(3.0), TimeOfDay::Night);
        assert_eq!(TimeOfDay::from_hours(6.0), TimeOfDay::Dawn);
        assert_eq!(TimeOfDay::from_hours(12.0), TimeOfDay::Day);
        assert_eq!(TimeOfDay::from_hours(18.0), TimeOfDay::Dusk);
        assert_eq!(TimeOfDay::from_hours(22.0), TimeOfDay::Night);
    }

    // ========== WeatherBridge Tests ==========

    use crate::integrations::weather::{WeatherCondition, WeatherError};
    use chrono::Utc;
    use std::sync::RwLock;

    /// Mock weather provider for testing WeatherBridge
    struct MockWeatherProvider {
        data: RwLock<Option<WeatherData>>,
        should_fail: RwLock<bool>,
    }

    impl MockWeatherProvider {
        fn new() -> Self {
            Self {
                data: RwLock::new(None),
                should_fail: RwLock::new(false),
            }
        }

        fn set_weather_data(&self, data: WeatherData) {
            *self.data.write().unwrap() = Some(data);
        }

        fn set_should_fail(&self, fail: bool) {
            *self.should_fail.write().unwrap() = fail;
        }
    }

    impl crate::integrations::weather::WeatherProvider for MockWeatherProvider {
        fn configure(&self, _config: crate::integrations::weather::WeatherConfig) {}

        async fn get_weather(&self) -> Result<WeatherData, WeatherError> {
            if *self.should_fail.read().unwrap() {
                return Err(WeatherError::NetworkError("Mock failure".to_string()));
            }
            self.data
                .read()
                .unwrap()
                .clone()
                .ok_or(WeatherError::NetworkError("No mock data".to_string()))
        }

        async fn refresh(&self) -> Result<WeatherData, WeatherError> {
            self.get_weather().await
        }

        fn is_available(&self) -> bool {
            self.data.read().unwrap().is_some()
        }

        fn get_cached(&self) -> Option<WeatherData> {
            self.data.read().unwrap().clone()
        }

        fn last_updated(&self) -> Option<chrono::DateTime<Utc>> {
            self.data.read().unwrap().as_ref().map(|d| d.fetched_at)
        }
    }

    fn create_test_weather_data(condition: WeatherCondition, wind_speed: f32, wind_dir: u16) -> WeatherData {
        WeatherData {
            temperature: 20.0,
            feels_like: 18.0,
            humidity: 65,
            condition,
            description: "Test weather".to_string(),
            wind_speed,
            wind_direction: wind_dir,
            pressure: 1013,
            visibility: 10000,
            uv_index: None,
            fetched_at: Utc::now(),
        }
    }

    #[test]
    fn test_weather_bridge_creation() {
        let provider = Arc::new(MockWeatherProvider::new());
        let bridge: WeatherBridge<MockWeatherProvider> = WeatherBridge::new(provider, WeatherUnits::Metric);

        assert!(bridge.current_weather_type().is_none());
    }

    #[test]
    fn test_weather_bridge_wind_speed_conversion_metric() {
        let provider = Arc::new(MockWeatherProvider::new());
        let bridge: WeatherBridge<MockWeatherProvider> = WeatherBridge::new(provider, WeatherUnits::Metric);

        // 10 m/s should convert to 36 km/h
        let converted = bridge.convert_wind_speed(10.0);
        assert!((converted - 36.0).abs() < 0.001);

        // 5 m/s should convert to 18 km/h
        let converted = bridge.convert_wind_speed(5.0);
        assert!((converted - 18.0).abs() < 0.001);
    }

    #[test]
    fn test_weather_bridge_wind_speed_conversion_imperial() {
        let provider = Arc::new(MockWeatherProvider::new());
        let bridge: WeatherBridge<MockWeatherProvider> = WeatherBridge::new(provider, WeatherUnits::Imperial);

        // 10 mph should convert to ~16.0934 km/h
        let converted = bridge.convert_wind_speed(10.0);
        assert!((converted - 16.0934).abs() < 0.001);

        // 60 mph should convert to ~96.56 km/h
        let converted = bridge.convert_wind_speed(60.0);
        assert!((converted - 96.5604).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_weather_bridge_sync_applies_weather_type() {
        let provider = Arc::new(MockWeatherProvider::new());
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Rain,
            5.0, // m/s
            180, // degrees (south)
        ));

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        let changed = bridge.sync(&mut controller).await.unwrap();

        assert!(changed);
        assert_eq!(controller.state().weather, WeatherType::Rain);
        assert_eq!(bridge.current_weather_type(), Some(WeatherType::Rain));
    }

    #[tokio::test]
    async fn test_weather_bridge_sync_applies_wind_data() {
        let provider = Arc::new(MockWeatherProvider::new());
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Clear,
            10.0, // 10 m/s = 36 km/h
            270,  // degrees (west)
        ));

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        bridge.sync(&mut controller).await.unwrap();

        let state = controller.state();
        assert!((state.wind_speed_kmh - 36.0).abs() < 0.001);
        assert!((state.wind_direction_degrees - 270.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_weather_bridge_sync_no_change_returns_false() {
        let provider = Arc::new(MockWeatherProvider::new());
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Clear,
            5.0,
            0,
        ));

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        // First sync - should report change
        let changed1 = bridge.sync(&mut controller).await.unwrap();
        assert!(changed1);

        // Second sync with same weather - should report no change
        let changed2 = bridge.sync(&mut controller).await.unwrap();
        assert!(!changed2);
    }

    #[tokio::test]
    async fn test_weather_bridge_sync_weather_transition() {
        let provider = Arc::new(MockWeatherProvider::new());
        let mut bridge = WeatherBridge::new(Arc::clone(&provider), WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        // First: Clear weather
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Clear,
            5.0,
            0,
        ));
        bridge.sync(&mut controller).await.unwrap();
        assert_eq!(controller.state().weather, WeatherType::Clear);
        assert_eq!(controller.state().previous_weather, None);
        assert!((controller.state().transition_progress - 1.0).abs() < 0.001);

        // Second: Change to Rain - should trigger transition
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Rain,
            8.0,
            90,
        ));
        let changed = bridge.sync(&mut controller).await.unwrap();

        assert!(changed);
        assert_eq!(controller.state().weather, WeatherType::Rain);
        assert_eq!(controller.state().previous_weather, Some(WeatherType::Clear));
        assert!((controller.state().transition_progress - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_weather_bridge_sync_error() {
        let provider = Arc::new(MockWeatherProvider::new());
        provider.set_should_fail(true);

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        let result = bridge.sync(&mut controller).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_weather_bridge_sync_from_cache() {
        let provider = Arc::new(MockWeatherProvider::new());
        provider.set_weather_data(create_test_weather_data(
            WeatherCondition::Cloudy,
            3.0,
            45,
        ));

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        let result = bridge.sync_from_cache(&mut controller);

        assert_eq!(result, Some(true));
        assert_eq!(controller.state().weather, WeatherType::Cloudy);
    }

    #[test]
    fn test_weather_bridge_sync_from_cache_no_data() {
        let provider = Arc::new(MockWeatherProvider::new());
        // Don't set any data

        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);
        let mut controller = WeatherController::new();

        let result = bridge.sync_from_cache(&mut controller);

        assert_eq!(result, None);
    }

    #[test]
    fn test_weather_bridge_set_units() {
        let provider = Arc::new(MockWeatherProvider::new());
        let mut bridge = WeatherBridge::new(provider, WeatherUnits::Metric);

        // Initially metric
        assert!((bridge.convert_wind_speed(10.0) - 36.0).abs() < 0.001);

        // Change to imperial
        bridge.set_units(WeatherUnits::Imperial);
        assert!((bridge.convert_wind_speed(10.0) - 16.0934).abs() < 0.001);
    }

    #[test]
    fn test_weather_bridge_provider_access() {
        let provider = Arc::new(MockWeatherProvider::new());
        let bridge = WeatherBridge::new(Arc::clone(&provider), WeatherUnits::Metric);

        // Should be able to access the provider through the bridge
        assert!(!bridge.provider().is_available()); // No data set yet
    }
}
