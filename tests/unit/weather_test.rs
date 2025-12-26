//! Unit tests for weather state transitions
//!
//! T044: Unit test for weather state transitions in tests/unit/weather_test.rs

use rustride::world::weather::{WeatherController, WeatherState, WeatherType, TimeOfDay};

#[test]
fn test_weather_state_default() {
    let state = WeatherState::default();
    assert_eq!(state.weather, WeatherType::Clear);
    assert_eq!(state.time_of_day, TimeOfDay::Day);
    assert!((state.time_hours - 12.0).abs() < 0.001);
    assert!((state.visibility_meters - 10000.0).abs() < 0.1);
}

#[test]
fn test_weather_state_new() {
    let state = WeatherState::new(WeatherType::Rain, 8.0);
    assert_eq!(state.weather, WeatherType::Rain);
    assert_eq!(state.time_of_day, TimeOfDay::Day);
    assert!((state.time_hours - 8.0).abs() < 0.001);
}

#[test]
fn test_weather_transition() {
    let mut state = WeatherState::default();
    assert_eq!(state.weather, WeatherType::Clear);

    state.transition_to(WeatherType::Rain);

    assert_eq!(state.weather, WeatherType::Rain);
    assert_eq!(state.previous_weather, Some(WeatherType::Clear));
    assert!((state.transition_progress - 0.0).abs() < 0.001);
}

#[test]
fn test_weather_transition_same_type_no_change() {
    let mut state = WeatherState::default();
    state.transition_to(WeatherType::Clear);

    // No transition should happen for same weather
    assert!(state.previous_weather.is_none());
    assert!((state.transition_progress - 1.0).abs() < 0.001);
}

#[test]
fn test_weather_transition_progress() {
    let mut state = WeatherState::default();
    state.transition_to(WeatherType::Fog);

    // Transition takes 30 seconds (default)
    state.update(15.0); // Half way
    assert!(state.transition_progress > 0.4 && state.transition_progress < 0.6);

    state.update(20.0); // Should complete
    assert!((state.transition_progress - 1.0).abs() < 0.001);
}

#[test]
fn test_visibility_during_transition() {
    let mut state = WeatherState::default();
    let clear_visibility = WeatherType::Clear.visibility();
    let fog_visibility = WeatherType::Fog.visibility();

    state.transition_to(WeatherType::Fog);

    // Initially should be at clear visibility
    assert!((state.visibility_meters - clear_visibility).abs() < 1.0);

    // Update to trigger transition
    state.update(15.0);

    // Should be somewhere between clear and fog visibility
    assert!(state.visibility_meters < clear_visibility);
    assert!(state.visibility_meters > fog_visibility);

    // Complete transition
    state.update(20.0);
    assert!((state.visibility_meters - fog_visibility).abs() < 1.0);
}

#[test]
fn test_time_of_day_from_hours() {
    assert_eq!(TimeOfDay::from_hours(3.0), TimeOfDay::Night);
    assert_eq!(TimeOfDay::from_hours(6.0), TimeOfDay::Dawn);
    assert_eq!(TimeOfDay::from_hours(10.0), TimeOfDay::Day);
    assert_eq!(TimeOfDay::from_hours(12.0), TimeOfDay::Day);
    assert_eq!(TimeOfDay::from_hours(18.0), TimeOfDay::Dusk);
    assert_eq!(TimeOfDay::from_hours(22.0), TimeOfDay::Night);
}

#[test]
fn test_time_of_day_wrapping() {
    // 25 hours should wrap to 1 hour (night)
    assert_eq!(TimeOfDay::from_hours(25.0), TimeOfDay::Night);
    // 30 hours should wrap to 6 hours (dawn)
    assert_eq!(TimeOfDay::from_hours(30.0), TimeOfDay::Dawn);
}

#[test]
fn test_set_time() {
    let mut state = WeatherState::default();
    state.set_time(6.5);

    assert!((state.time_hours - 6.5).abs() < 0.001);
    assert_eq!(state.time_of_day, TimeOfDay::Dawn);
}

#[test]
fn test_realistic_time_progression() {
    let mut state = WeatherState::default();
    state.realistic_time = true;
    state.time_hours = 12.0;

    // 60 seconds real = 1 hour in-game
    state.update(60.0);

    assert!((state.time_hours - 13.0).abs() < 0.01);
}

#[test]
fn test_realistic_time_wrapping() {
    let mut state = WeatherState::default();
    state.realistic_time = true;
    state.time_hours = 23.5;

    state.update(60.0); // +1 hour

    // Should wrap around
    assert!(state.time_hours < 1.0);
}

#[test]
fn test_weather_controller_new() {
    let controller = WeatherController::new();
    assert_eq!(controller.state().weather, WeatherType::Clear);
}

#[test]
fn test_weather_controller_set_weather() {
    let mut controller = WeatherController::new();
    controller.set_weather(WeatherType::HeavyRain);

    assert_eq!(controller.state().weather, WeatherType::HeavyRain);
}

#[test]
fn test_weather_controller_set_time() {
    let mut controller = WeatherController::new();
    controller.set_time(18.5);

    assert!((controller.state().time_hours - 18.5).abs() < 0.001);
    assert_eq!(controller.state().time_of_day, TimeOfDay::Dusk);
}

#[test]
fn test_weather_controller_realistic_time() {
    let mut controller = WeatherController::new();
    controller.set_realistic_time(true);
    controller.set_time(12.0);

    controller.update(60.0);

    assert!(controller.state().time_hours > 12.0);
}

#[test]
fn test_weather_controller_auto_weather() {
    let mut controller = WeatherController::new();
    controller.set_auto_weather(true);

    // Simulate 5 minutes (default change interval)
    controller.update(300.0);

    // Weather should have changed from Clear
    assert_ne!(controller.state().weather, WeatherType::Clear);
}

#[test]
fn test_particle_density_clear() {
    assert!((WeatherType::Clear.particle_density() - 0.0).abs() < 0.001);
}

#[test]
fn test_particle_density_rain() {
    assert!(WeatherType::Rain.particle_density() > 0.0);
    assert!(WeatherType::HeavyRain.particle_density() > WeatherType::Rain.particle_density());
}

#[test]
fn test_particle_density_during_transition() {
    let mut state = WeatherState::default();
    state.transition_to(WeatherType::Rain);

    // Initially should be 0 (clear)
    let initial = state.current_particle_density();
    assert!(initial < 0.1);

    // After transition
    state.update(35.0);
    let final_density = state.current_particle_density();
    assert!((final_density - WeatherType::Rain.particle_density()).abs() < 0.1);
}

#[test]
fn test_visibility_values() {
    assert!(WeatherType::Clear.visibility() > WeatherType::Cloudy.visibility());
    assert!(WeatherType::Cloudy.visibility() > WeatherType::Rain.visibility());
    assert!(WeatherType::Rain.visibility() > WeatherType::HeavyRain.visibility());
    assert!(WeatherType::HeavyRain.visibility() > WeatherType::Fog.visibility());
}

#[test]
fn test_ambient_intensity() {
    assert!(TimeOfDay::Day.ambient_intensity() > TimeOfDay::Dawn.ambient_intensity());
    assert!(TimeOfDay::Dawn.ambient_intensity() > TimeOfDay::Dusk.ambient_intensity());
    assert!(TimeOfDay::Dusk.ambient_intensity() > TimeOfDay::Night.ambient_intensity());
}

#[test]
fn test_wind_defaults() {
    let state = WeatherState::default();
    assert!(state.wind_speed_kmh > 0.0);
    assert!(state.wind_direction_degrees >= 0.0 && state.wind_direction_degrees < 360.0);
}
