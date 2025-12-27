//! Unit tests for skybox rendering
//!
//! T045: Unit test for skybox rendering in tests/unit/skybox_test.rs

use rustride::world::weather::skybox::{ambient_light, sun_position, SkyColors, Skybox};
use rustride::world::weather::WeatherType;

#[test]
fn test_sky_colors_for_clear_day() {
    let colors = SkyColors::for_conditions(12.0, WeatherType::Clear);

    // Day sky should be blue (high blue component)
    assert!(colors.zenith.z > colors.zenith.x);
    assert!(colors.zenith.z > colors.zenith.y);
}

#[test]
fn test_sky_colors_for_clear_night() {
    let colors = SkyColors::for_conditions(2.0, WeatherType::Clear);

    // Night sky should be dark
    assert!(colors.zenith.x < 0.1);
    assert!(colors.zenith.y < 0.1);
    assert!(colors.zenith.z < 0.2);
}

#[test]
fn test_sky_colors_for_dawn() {
    let colors = SkyColors::for_conditions(6.0, WeatherType::Clear);

    // Dawn horizon should have warm colors (higher red/yellow)
    assert!(colors.horizon.x > colors.zenith.x);
}

#[test]
fn test_sky_colors_for_dusk() {
    let colors = SkyColors::for_conditions(18.0, WeatherType::Clear);

    // Dusk horizon should have warm colors
    assert!(colors.horizon.x > colors.zenith.x);
}

#[test]
fn test_sky_colors_cloudy_desaturation() {
    let clear = SkyColors::for_conditions(12.0, WeatherType::Clear);
    let cloudy = SkyColors::for_conditions(12.0, WeatherType::Cloudy);

    // Cloudy should be more gray (less difference between RGB components)
    let clear_range = (clear.zenith.x - clear.zenith.z).abs();
    let cloudy_range = (cloudy.zenith.x - cloudy.zenith.z).abs();

    assert!(cloudy_range < clear_range);
}

#[test]
fn test_sky_colors_rain_darkening() {
    let clear = SkyColors::for_conditions(12.0, WeatherType::Clear);
    let rain = SkyColors::for_conditions(12.0, WeatherType::Rain);

    // Rain should be darker
    assert!(rain.zenith.x < clear.zenith.x);
    assert!(rain.zenith.y < clear.zenith.y);
    assert!(rain.zenith.z < clear.zenith.z);
}

#[test]
fn test_sky_colors_fog() {
    let fog = SkyColors::for_conditions(12.0, WeatherType::Fog);

    // Fog should be gray/white-ish (similar RGB values)
    let range = (fog.zenith.x - fog.zenith.z).abs();
    assert!(range < 0.1);

    // And relatively bright
    assert!(fog.zenith.x > 0.6);
}

#[test]
fn test_sky_colors_snow() {
    let snow = SkyColors::for_conditions(12.0, WeatherType::Snow);

    // Snow sky should have slightly blue tint
    assert!(snow.zenith.z >= snow.zenith.x);
    assert!(snow.horizon.z >= snow.horizon.x);
}

#[test]
fn test_sun_position_sunrise() {
    let pos = sun_position(6.0);

    // Sun should be at horizon (y ≈ 0)
    assert!(pos.y.abs() < 0.2);
    // Sun should be in east (negative x)
    assert!(pos.x < 0.0);
}

#[test]
fn test_sun_position_noon() {
    let pos = sun_position(12.0);

    // Sun should be high
    assert!(pos.y > 0.9);
}

#[test]
fn test_sun_position_sunset() {
    let pos = sun_position(18.0);

    // Sun should be at horizon
    assert!(pos.y.abs() < 0.2);
    // Sun should be in west (positive x)
    assert!(pos.x > 0.0);
}

#[test]
fn test_sun_position_night() {
    let pos = sun_position(2.0);

    // Sun should be below horizon
    assert!(pos.y < 0.0);
}

#[test]
fn test_sun_position_normalized() {
    for hour in [6.0, 9.0, 12.0, 15.0, 18.0] {
        let pos = sun_position(hour);
        let length = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "Sun position should be normalized at hour {}",
            hour
        );
    }
}

#[test]
fn test_ambient_light_day_clear() {
    let (color, intensity) = ambient_light(12.0, WeatherType::Clear);

    assert!(intensity > 0.9);
    assert!(color.x > 0.0 && color.y > 0.0 && color.z > 0.0);
}

#[test]
fn test_ambient_light_night() {
    let (_, intensity) = ambient_light(2.0, WeatherType::Clear);

    assert!(intensity < 0.2);
}

#[test]
fn test_ambient_light_rain_reduces_intensity() {
    let (_, clear_intensity) = ambient_light(12.0, WeatherType::Clear);
    let (_, rain_intensity) = ambient_light(12.0, WeatherType::Rain);

    assert!(rain_intensity < clear_intensity);
}

#[test]
fn test_ambient_light_heavy_rain_darker() {
    let (_, rain_intensity) = ambient_light(12.0, WeatherType::Rain);
    let (_, heavy_rain_intensity) = ambient_light(12.0, WeatherType::HeavyRain);

    assert!(heavy_rain_intensity < rain_intensity);
}

#[test]
fn test_skybox_new() {
    let skybox = Skybox::new();
    let colors = skybox.colors();

    // Default should be day colors
    assert!(colors.zenith.z > 0.5); // Blue sky
}

#[test]
fn test_skybox_update() {
    let mut skybox = Skybox::new();

    skybox.update(2.0, WeatherType::Clear);
    let night_colors = skybox.colors().zenith;

    skybox.update(12.0, WeatherType::Clear);
    let day_colors = skybox.colors().zenith;

    // Day should be brighter than night
    assert!(day_colors.x > night_colors.x);
    assert!(day_colors.y > night_colors.y);
    assert!(day_colors.z > night_colors.z);
}

#[test]
fn test_skybox_sun_direction() {
    let skybox = Skybox::new();

    let noon_dir = skybox.sun_direction(12.0);
    let sunrise_dir = skybox.sun_direction(6.0);

    // Noon sun should be higher than sunrise
    assert!(noon_dir.y > sunrise_dir.y);
}

#[test]
fn test_skybox_default() {
    let skybox = Skybox::default();
    let colors = skybox.colors();

    // Should have valid colors
    assert!(colors.zenith.x >= 0.0 && colors.zenith.x <= 1.0);
    assert!(colors.zenith.y >= 0.0 && colors.zenith.y <= 1.0);
    assert!(colors.zenith.z >= 0.0 && colors.zenith.z <= 1.0);
}

#[test]
fn test_time_transitions_smooth() {
    // Test that transitioning through dawn/dusk produces smooth color changes
    let colors_5am = SkyColors::for_conditions(5.0, WeatherType::Clear);
    let colors_6am = SkyColors::for_conditions(6.0, WeatherType::Clear);
    let colors_7am = SkyColors::for_conditions(7.0, WeatherType::Clear);

    // Colors should change gradually (no huge jumps)
    let diff_5_6 = (colors_5am.zenith.y - colors_6am.zenith.y).abs();
    let diff_6_7 = (colors_6am.zenith.y - colors_7am.zenith.y).abs();

    assert!(diff_5_6 < 0.5);
    assert!(diff_6_7 < 0.5);
}

#[test]
fn test_weather_affects_all_color_components() {
    let clear = SkyColors::for_conditions(12.0, WeatherType::Clear);
    let heavy_rain = SkyColors::for_conditions(12.0, WeatherType::HeavyRain);

    // Heavy rain should affect zenith, horizon, sun, and fog colors
    assert!(heavy_rain.zenith.length() < clear.zenith.length());
    assert!(heavy_rain.horizon.length() < clear.horizon.length());
}
