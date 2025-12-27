//! Weather system for dynamic weather effects and time-of-day simulation.

pub mod particles;
pub mod skybox;

use serde::{Deserialize, Serialize};

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
}
