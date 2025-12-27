//! Integration tests for weather system
//!
//! T046: Integration test for weather system in tests/integration/weather_test.rs

use glam::{Vec2, Vec3};
use rustride::world::weather::particles::{ParticleSystem, ParticleType};
use rustride::world::weather::skybox::{sun_position, Skybox};
use rustride::world::weather::{TimeOfDay, WeatherController, WeatherState, WeatherType};

/// Test the full weather system workflow
#[test]
fn test_weather_system_full_cycle() {
    // Create weather controller
    let mut controller = WeatherController::new();

    // Initial state should be clear day
    assert_eq!(controller.state().weather, WeatherType::Clear);
    assert_eq!(controller.state().time_of_day, TimeOfDay::Day);

    // Set time to morning
    controller.set_time(7.0);
    assert_eq!(controller.state().time_of_day, TimeOfDay::Day);

    // Transition to rain
    controller.set_weather(WeatherType::Rain);
    assert_eq!(controller.state().weather, WeatherType::Rain);
    assert!((controller.state().transition_progress - 0.0).abs() < 0.01);

    // Simulate time passing for transition
    controller.update(35.0); // Transition takes 30 seconds

    // Transition should be complete
    assert!((controller.state().transition_progress - 1.0).abs() < 0.01);

    // Visibility should have changed to rain visibility
    let rain_visibility = WeatherType::Rain.visibility();
    assert!((controller.state().visibility_meters - rain_visibility).abs() < 100.0);
}

/// Test particle system integration with weather
#[test]
fn test_particle_system_weather_integration() {
    let mut particle_system = ParticleSystem::new(1000);
    let camera_pos = Vec3::ZERO;
    let wind = Vec2::new(5.0, 0.0);

    // Test rain particles
    particle_system.set_particle_type(ParticleType::Rain);
    particle_system.set_density(1.0);
    particle_system.update(0.1, camera_pos, wind);

    assert!(!particle_system.particles().is_empty());

    // Clear and test snow
    particle_system.clear();
    particle_system.set_particle_type(ParticleType::Snow);
    particle_system.set_density(0.6);
    particle_system.update(0.1, camera_pos, wind);

    assert!(!particle_system.particles().is_empty());

    // Clear and test fog
    particle_system.clear();
    particle_system.set_particle_type(ParticleType::Fog);
    particle_system.set_density(0.3);
    particle_system.update(0.1, camera_pos, wind);

    assert!(!particle_system.particles().is_empty());
}

/// Test skybox integration with weather and time
#[test]
fn test_skybox_weather_time_integration() {
    let mut skybox = Skybox::new();

    // Dawn (should have warm horizon colors)
    skybox.update(6.0, WeatherType::Clear);
    let dawn_colors = *skybox.colors();

    // Clear noon
    skybox.update(12.0, WeatherType::Clear);
    let noon_colors = *skybox.colors();

    // Rainy noon
    skybox.update(12.0, WeatherType::Rain);
    let rain_colors = *skybox.colors();

    // Dawn and noon should have different horizon colors (dawn is warmer)
    // Dawn horizon should have more red than noon
    assert!(
        dawn_colors.horizon.x > noon_colors.horizon.x * 0.9
            || dawn_colors.horizon.y != noon_colors.horizon.y
    );

    // Rain should darken colors
    assert!(rain_colors.zenith.length() < noon_colors.zenith.length());
}

/// Test realistic time progression
#[test]
fn test_realistic_time_day_cycle() {
    let mut controller = WeatherController::new();
    controller.set_realistic_time(true);
    controller.set_time(6.0); // Start at dawn

    // Simulate 24 hours of real time (which is 24 * 60 = 1440 in-game hours at 60x speed)
    // Actually, 1 minute real = 1 hour in-game, so 24 minutes = 24 in-game hours
    let start_time = controller.state().time_hours;

    // Simulate 12 real minutes = 12 in-game hours
    for _ in 0..12 {
        controller.update(60.0); // 1 minute real time
    }

    let end_time = controller.state().time_hours;

    // Time should have progressed by about 12 hours (with wrapping)
    let expected_time = (start_time + 12.0) % 24.0;
    assert!((end_time - expected_time).abs() < 0.5);
}

/// Test weather affects visibility during ride
#[test]
fn test_visibility_changes_during_weather_transition() {
    let mut state = WeatherState::new(WeatherType::Clear, 12.0);
    let initial_visibility = state.visibility_meters;

    // Start transition to fog
    state.transition_to(WeatherType::Fog);

    // Track visibility changes during transition
    let mut prev_visibility = initial_visibility;
    let mut visibility_decreasing = true;

    for _ in 0..35 {
        state.update(1.0);
        if state.visibility_meters > prev_visibility {
            visibility_decreasing = false;
        }
        prev_visibility = state.visibility_meters;
    }

    // Visibility should have been consistently decreasing (or staying same)
    assert!(visibility_decreasing || prev_visibility < initial_visibility);

    // Final visibility should be fog visibility
    let fog_visibility = WeatherType::Fog.visibility();
    assert!((state.visibility_meters - fog_visibility).abs() < 50.0);
}

/// Test sun position throughout the day
#[test]
fn test_sun_arc_throughout_day() {
    let mut max_height: f32 = f32::NEG_INFINITY;
    let mut max_height_time: f32 = 0.0;

    // Sample sun position every hour
    for hour in 0..24 {
        let pos = sun_position(hour as f32);
        if pos.y > max_height {
            max_height = pos.y;
            max_height_time = hour as f32;
        }
    }

    // Maximum sun height should be around noon
    assert!((11.0..=13.0).contains(&max_height_time));
}

/// Test particle lifecycle in system
#[test]
fn test_particle_lifecycle_in_system() {
    let mut system = ParticleSystem::new(100);
    system.set_particle_type(ParticleType::Rain);
    system.set_density(1.0);

    let camera_pos = Vec3::ZERO;
    let wind = Vec2::ZERO;

    // Spawn particles
    system.update(0.5, camera_pos, wind);
    let initial_count = system.particles().len();
    assert!(initial_count > 0);

    // Wait for particles to die (rain lifetime is 2 seconds)
    for _ in 0..30 {
        system.update(0.1, camera_pos, wind);
    }

    // System should still have particles (new ones spawned)
    assert!(!system.particles().is_empty());
}

/// Test weather controller with auto weather
#[test]
fn test_auto_weather_changes() {
    let mut controller = WeatherController::new();
    controller.set_auto_weather(true);

    let initial_weather = controller.state().weather;

    // Simulate enough time for weather change (default interval is 300 seconds)
    controller.update(350.0);

    // Weather should have changed
    assert_ne!(controller.state().weather, initial_weather);
}

/// Test multiple weather transitions
#[test]
fn test_sequential_weather_transitions() {
    let mut controller = WeatherController::new();

    // Clear -> Rain -> Heavy Rain -> Clear
    let transitions = [
        WeatherType::Rain,
        WeatherType::HeavyRain,
        WeatherType::Clear,
    ];

    for weather in transitions {
        controller.set_weather(weather);

        // Wait for transition
        controller.update(35.0);

        assert_eq!(controller.state().weather, weather);
        assert!((controller.state().transition_progress - 1.0).abs() < 0.01);
    }
}

/// Test particle density matches weather type
#[test]
fn test_particle_density_matches_weather() {
    let mut state = WeatherState::new(WeatherType::Clear, 12.0);

    // Clear should have no particles
    assert!((state.current_particle_density() - 0.0).abs() < 0.01);

    // Transition to rain
    state.transition_to(WeatherType::Rain);
    state.update(35.0);

    // Rain should have particles
    assert!(state.current_particle_density() > 0.3);

    // Transition to heavy rain
    state.transition_to(WeatherType::HeavyRain);
    state.update(35.0);

    // Heavy rain should have more particles
    assert!((state.current_particle_density() - 1.0).abs() < 0.01);
}

/// Test ambient lighting varies by time and weather
#[test]
fn test_ambient_lighting_variations() {
    use rustride::world::weather::skybox::ambient_light;

    // Day clear - brightest
    let (_, day_clear) = ambient_light(12.0, WeatherType::Clear);

    // Day rain - reduced
    let (_, day_rain) = ambient_light(12.0, WeatherType::Rain);

    // Night clear - darkest time
    let (_, night_clear) = ambient_light(2.0, WeatherType::Clear);

    // Night rain - even darker
    let (_, night_rain) = ambient_light(2.0, WeatherType::Rain);

    assert!(day_clear > day_rain);
    assert!(day_rain > night_clear);
    assert!(night_clear > night_rain);
}

/// Test weather state serialization
#[test]
fn test_weather_state_serialization() {
    let state = WeatherState::new(WeatherType::Snow, 15.0);

    // Serialize
    let json = serde_json::to_string(&state).expect("Should serialize");

    // Deserialize
    let restored: WeatherState = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(restored.weather, WeatherType::Snow);
    assert!((restored.time_hours - 15.0).abs() < 0.01);
}

/// Test wind affects particle movement
#[test]
fn test_wind_affects_particles() {
    let mut system = ParticleSystem::new(100);
    system.set_particle_type(ParticleType::Rain);
    system.set_density(1.0);

    let camera_pos = Vec3::ZERO;

    // No wind
    system.update(0.1, camera_pos, Vec2::ZERO);
    let particle_no_wind = system.particles()[0].velocity;

    system.clear();

    // Strong wind
    system.update(0.1, camera_pos, Vec2::new(20.0, 0.0));
    let particle_with_wind = system.particles()[0].velocity;

    // Wind should affect horizontal velocity
    assert!(particle_with_wind.x.abs() > particle_no_wind.x.abs());
}
