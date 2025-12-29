//! Integration tests for gradient-responsive ride simulation.
//!
//! T083: Integration test for gradient ride simulation.

use rustride::gradient::{
    GpxRoute, GradientController, GradientResult, GradientSettings, GradientSmoother, RoutePoint,
};

#[test]
fn test_gradient_controller_creation() {
    let controller = GradientController::new(10); // 10 Hz update rate
    assert!(!controller.has_route());
    assert_eq!(controller.progress(), 0.0);
}

#[test]
fn test_gradient_smoother_basic() {
    let smoother = GradientSmoother::new(5);
    assert_eq!(smoother.current(), 0.0);
}

#[test]
fn test_gradient_smoother_with_values() {
    let mut smoother = GradientSmoother::new(3);
    smoother.add(3.0);
    smoother.add(6.0);
    smoother.add(9.0);
    // Average of 3, 6, 9 = 6.0
    assert!((smoother.current() - 6.0).abs() < 0.1);
}

#[test]
fn test_gradient_settings_defaults() {
    let settings = GradientSettings::default();
    assert!(settings.max_gradient > 0.0);
    assert!(settings.min_gradient < 0.0);
    assert!(settings.difficulty > 0.0);
}

#[test]
fn test_gradient_settings_clamp_gradient() {
    let settings = GradientSettings::default();

    // Test clamping above max
    let clamped = settings.clamp_gradient(30.0);
    assert!(clamped <= settings.max_gradient);

    // Test clamping below min
    let clamped = settings.clamp_gradient(-25.0);
    assert!(clamped >= settings.min_gradient);

    // Test within range stays unchanged
    let clamped = settings.clamp_gradient(5.0);
    assert_eq!(clamped, 5.0);
}

#[test]
fn test_gradient_settings_validation() {
    // Valid settings
    let valid = GradientSettings::default();
    assert!(valid.validate().is_ok());

    // Invalid: difficulty > 1.0
    let invalid = GradientSettings {
        difficulty: 1.5,
        ..Default::default()
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn test_gpx_route_creation() {
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 105.0),
        RoutePoint::new(200.0, 108.0),
    ];

    let route = GpxRoute::from_points("test", points).unwrap();
    assert_eq!(route.total_distance_m, 200.0);
}

#[test]
fn test_gpx_route_get_gradient_at() {
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 105.0), // 5% grade
        RoutePoint::new(200.0, 115.0), // 10% grade
        RoutePoint::new(300.0, 112.5), // -2.5% grade
    ];

    let route = GpxRoute::from_points("test", points).unwrap();

    // In middle of first segment (should be ~5%)
    let gradient = route.get_gradient_at(50.0);
    assert!(gradient > 4.0 && gradient < 6.0);
}

#[test]
fn test_gradient_controller_load_route() {
    let mut controller = GradientController::new(10);

    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(1000.0, 180.0),
        RoutePoint::new(2000.0, 230.0),
    ];

    let route = GpxRoute::from_points("test", points).unwrap();
    controller.load_route(route).unwrap();

    assert!(controller.has_route());
    assert!(controller.has_elevation_data());
    assert_eq!(controller.total_distance(), Some(2000.0));
}

#[test]
fn test_gradient_controller_update() {
    let mut controller = GradientController::new(10);

    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(1000.0, 180.0), // 8% grade
        RoutePoint::new(2000.0, 230.0), // 5% grade
    ];

    let route = GpxRoute::from_points("test", points).unwrap();
    controller.load_route(route).unwrap();

    let settings = GradientSettings::default();

    // Update position
    let result = controller.update(500.0, &settings);
    assert!(result.is_ok());

    let (gradient_result, _command) = result.unwrap();
    assert!(gradient_result.raw_gradient > 0.0);
}

#[test]
fn test_gradient_simulation_full_ride() {
    let mut controller = GradientController::new(10);

    // Create a route simulating a short hill climb then descent
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(500.0, 140.0),  // 8% climb
        RoutePoint::new(1000.0, 180.0), // 8% climb
        RoutePoint::new(1500.0, 200.0), // 4% climb
        RoutePoint::new(2000.0, 190.0), // -2% descent
        RoutePoint::new(2500.0, 170.0), // -4% descent
    ];

    let route = GpxRoute::from_points("test", points).unwrap();
    controller.load_route(route).unwrap();

    let settings = GradientSettings::default();

    // Simulate riding the route
    let mut positive_count = 0;
    let mut negative_count = 0;

    for distance in (0..2500).step_by(100) {
        let result = controller.update(distance as f64, &settings);
        if let Ok((gradient_result, _)) = result {
            if gradient_result.raw_gradient > 1.0 {
                positive_count += 1;
            } else if gradient_result.raw_gradient < -1.0 {
                negative_count += 1;
            }
        }
    }

    // Should have both climbs and descents
    assert!(positive_count > 0);
    assert!(negative_count > 0);
}

#[test]
fn test_gradient_controller_progress_tracking() {
    let mut controller = GradientController::new(10);

    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(1000.0, 150.0),
        RoutePoint::new(2000.0, 150.0),
    ];

    let route = GpxRoute::from_points("test", points).unwrap();
    controller.load_route(route).unwrap();

    let settings = GradientSettings::default();

    // Initially at 0%
    assert_eq!(controller.progress(), 0.0);

    // After updating to 1000m on a 2000m route
    let _ = controller.update(1000.0, &settings);
    let progress = controller.progress();
    assert!((progress - 0.5).abs() < 0.1); // ~50%

    // After completing
    let _ = controller.update(2000.0, &settings);
    let progress = controller.progress();
    assert!((progress - 1.0).abs() < 0.1); // ~100%
}

#[test]
fn test_gradient_controller_no_route_error() {
    let mut controller = GradientController::new(10);
    let settings = GradientSettings::default();

    // Without loading a route, should return error
    let result = controller.update(100.0, &settings);
    assert!(result.is_err());
}

#[test]
fn test_gradient_result_structure() {
    let result = GradientResult::from_raw(5.0, 5.0, 5.0, 5.0);
    assert_eq!(result.raw_gradient, 5.0);
    assert_eq!(result.capped_gradient, 5.0);
    assert_eq!(result.effective_gradient, 5.0);
    assert_eq!(result.smoothed_gradient, 5.0);
}

#[test]
fn test_gradient_settings_apply_difficulty() {
    let settings = GradientSettings {
        difficulty: 0.5,
        ..Default::default()
    };

    // 10% gradient with 0.5 difficulty = 5% effective
    let effective = settings.apply_difficulty(10.0);
    assert!((effective - 5.0).abs() < 0.1);
}

#[test]
fn test_gradient_smoother_reset() {
    let mut smoother = GradientSmoother::new(5);
    smoother.add(10.0);
    smoother.add(10.0);
    assert!(smoother.current() > 0.0);

    smoother.reset();
    assert_eq!(smoother.current(), 0.0);
}

#[test]
fn test_gradient_settings_with_difficulty() {
    let settings = GradientSettings::with_difficulty(50);
    assert!((settings.difficulty - 0.5).abs() < 0.01);
}

#[test]
fn test_gradient_effective_gradient() {
    let settings = GradientSettings {
        difficulty: 0.5,
        max_gradient: 10.0,
        min_gradient: -10.0,
        ..Default::default()
    };

    // 20% gradient clamped to 10%, then 50% difficulty = 5%
    let effective = settings.effective_gradient(20.0);
    assert!((effective - 5.0).abs() < 0.1);
}
