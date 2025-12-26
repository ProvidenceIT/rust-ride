//! Procedural skybox with time-of-day transitions.

use super::{TimeOfDay, WeatherType};
use glam::Vec3;

/// Sky colors for rendering
#[derive(Debug, Clone, Copy)]
pub struct SkyColors {
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub sun: Vec3,
    pub fog: Vec3,
}

impl SkyColors {
    /// Get sky colors for given time and weather
    pub fn for_conditions(time_hours: f32, weather: WeatherType) -> Self {
        let time_of_day = TimeOfDay::from_hours(time_hours);
        let base_colors = Self::for_time(time_of_day, time_hours);

        // Modify colors based on weather
        match weather {
            WeatherType::Clear => base_colors,
            WeatherType::Cloudy => Self::desaturate(base_colors, 0.3),
            WeatherType::Rain | WeatherType::HeavyRain => {
                Self::darken(Self::desaturate(base_colors, 0.5), 0.3)
            }
            WeatherType::Fog => Self {
                zenith: Vec3::new(0.7, 0.7, 0.75),
                horizon: Vec3::new(0.8, 0.8, 0.82),
                sun: Vec3::new(0.9, 0.9, 0.85),
                fog: Vec3::new(0.8, 0.8, 0.82),
            },
            WeatherType::Snow => Self {
                zenith: Vec3::new(0.6, 0.65, 0.7),
                horizon: Vec3::new(0.85, 0.87, 0.9),
                sun: Vec3::new(0.9, 0.9, 0.85),
                fog: Vec3::new(0.9, 0.92, 0.95),
            },
        }
    }

    fn for_time(time_of_day: TimeOfDay, time_hours: f32) -> Self {
        match time_of_day {
            TimeOfDay::Dawn => {
                // Interpolate between night and morning
                let t = (time_hours - 5.0) / 2.0;
                Self::lerp(Self::night(), Self::morning(), t)
            }
            TimeOfDay::Day => Self::day(),
            TimeOfDay::Dusk => {
                // Interpolate between day and evening
                let t = (time_hours - 17.0) / 2.0;
                Self::lerp(Self::day(), Self::evening(), t)
            }
            TimeOfDay::Night => Self::night(),
        }
    }

    fn day() -> Self {
        Self {
            zenith: Vec3::new(0.3, 0.5, 0.85),
            horizon: Vec3::new(0.6, 0.75, 0.95),
            sun: Vec3::new(1.0, 0.95, 0.85),
            fog: Vec3::new(0.7, 0.8, 0.9),
        }
    }

    fn morning() -> Self {
        Self {
            zenith: Vec3::new(0.4, 0.5, 0.7),
            horizon: Vec3::new(0.95, 0.7, 0.5),
            sun: Vec3::new(1.0, 0.8, 0.5),
            fog: Vec3::new(0.9, 0.8, 0.7),
        }
    }

    fn evening() -> Self {
        Self {
            zenith: Vec3::new(0.3, 0.25, 0.5),
            horizon: Vec3::new(0.95, 0.5, 0.3),
            sun: Vec3::new(1.0, 0.6, 0.3),
            fog: Vec3::new(0.8, 0.6, 0.5),
        }
    }

    fn night() -> Self {
        Self {
            zenith: Vec3::new(0.02, 0.02, 0.08),
            horizon: Vec3::new(0.05, 0.05, 0.12),
            sun: Vec3::new(0.9, 0.9, 0.95), // Moon color
            fog: Vec3::new(0.1, 0.1, 0.15),
        }
    }

    fn lerp(a: Self, b: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            zenith: a.zenith.lerp(b.zenith, t),
            horizon: a.horizon.lerp(b.horizon, t),
            sun: a.sun.lerp(b.sun, t),
            fog: a.fog.lerp(b.fog, t),
        }
    }

    fn desaturate(colors: Self, amount: f32) -> Self {
        let desat = |c: Vec3| {
            let gray = c.x * 0.299 + c.y * 0.587 + c.z * 0.114;
            c.lerp(Vec3::splat(gray), amount)
        };
        Self {
            zenith: desat(colors.zenith),
            horizon: desat(colors.horizon),
            sun: desat(colors.sun),
            fog: desat(colors.fog),
        }
    }

    fn darken(colors: Self, amount: f32) -> Self {
        Self {
            zenith: colors.zenith * (1.0 - amount),
            horizon: colors.horizon * (1.0 - amount),
            sun: colors.sun * (1.0 - amount * 0.5),
            fog: colors.fog * (1.0 - amount * 0.5),
        }
    }
}

/// Calculate sun position from time
pub fn sun_position(time_hours: f32) -> Vec3 {
    // Sun moves from east to west
    // At 6am, sun is at horizon east
    // At 12pm, sun is overhead
    // At 18pm, sun is at horizon west

    let normalized_time = (time_hours - 6.0) / 12.0; // 0 at sunrise, 1 at sunset
    let angle = normalized_time * std::f32::consts::PI;

    // Sun is below horizon at night
    if !(5.0..=19.0).contains(&time_hours) {
        return Vec3::new(0.0, -1.0, 0.0);
    }

    Vec3::new(
        -angle.cos(),
        angle.sin().max(0.0),
        0.2, // Slight offset for more interesting lighting
    )
    .normalize()
}

/// Calculate ambient light based on time and weather
pub fn ambient_light(time_hours: f32, weather: WeatherType) -> (Vec3, f32) {
    let time_of_day = TimeOfDay::from_hours(time_hours);
    let base_intensity = time_of_day.ambient_intensity();

    let weather_multiplier = match weather {
        WeatherType::Clear => 1.0,
        WeatherType::Cloudy => 0.8,
        WeatherType::Rain => 0.5,
        WeatherType::HeavyRain => 0.3,
        WeatherType::Fog => 0.6,
        WeatherType::Snow => 0.7,
    };

    let intensity = base_intensity * weather_multiplier;
    let color = SkyColors::for_conditions(time_hours, weather).fog;

    (color, intensity)
}

/// Skybox renderer state
pub struct Skybox {
    current_colors: SkyColors,
}

impl Skybox {
    pub fn new() -> Self {
        Self {
            current_colors: SkyColors::day(),
        }
    }

    pub fn update(&mut self, time_hours: f32, weather: WeatherType) {
        self.current_colors = SkyColors::for_conditions(time_hours, weather);
    }

    pub fn colors(&self) -> &SkyColors {
        &self.current_colors
    }

    pub fn sun_direction(&self, time_hours: f32) -> Vec3 {
        sun_position(time_hours)
    }
}

impl Default for Skybox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sky_colors_day() {
        let colors = SkyColors::for_conditions(12.0, WeatherType::Clear);
        assert!(colors.zenith.y > 0.3); // Blue sky
    }

    #[test]
    fn test_sun_position_noon() {
        let pos = sun_position(12.0);
        assert!(pos.y > 0.9); // Sun is high
    }

    #[test]
    fn test_sun_position_night() {
        let pos = sun_position(2.0);
        assert!(pos.y < 0.0); // Sun below horizon
    }
}
