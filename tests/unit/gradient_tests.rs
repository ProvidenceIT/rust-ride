//! Unit tests for gradient simulation module.
//!
//! T031: Unit tests for gradient calculator and related functionality.

use rustride::gradient::{
    GradientCalculator, GradientController, GradientResult, GradientSettings, GradientSmoother,
    GpxRoute, RoutePoint, GradientSegment,
};

// ========== GradientSettings Tests ==========

#[test]
fn test_gradient_settings_default() {
    let settings = GradientSettings::default();

    assert_eq!(settings.difficulty, 1.0);
    assert_eq!(settings.max_gradient, 15.0);
    assert_eq!(settings.min_gradient, -15.0);
    assert_eq!(settings.smoothing_secs, 3);
    assert!((settings.rolling_resistance - 0.004).abs() < 0.0001);
}

#[test]
fn test_gradient_settings_with_difficulty() {
    let settings = GradientSettings::with_difficulty(50);

    assert!((settings.difficulty - 0.5).abs() < 0.001);
}

#[test]
fn test_gradient_settings_clamp() {
    let settings = GradientSettings::default();

    // Within range
    assert_eq!(settings.clamp_gradient(5.0), 5.0);
    assert_eq!(settings.clamp_gradient(-5.0), -5.0);

    // Exceeds max
    assert_eq!(settings.clamp_gradient(20.0), 15.0);
    assert_eq!(settings.clamp_gradient(100.0), 15.0);

    // Exceeds min
    assert_eq!(settings.clamp_gradient(-20.0), -15.0);
    assert_eq!(settings.clamp_gradient(-100.0), -15.0);
}

#[test]
fn test_gradient_settings_apply_difficulty() {
    let mut settings = GradientSettings::default();

    // Full difficulty
    assert_eq!(settings.apply_difficulty(10.0), 10.0);

    // Half difficulty
    settings.difficulty = 0.5;
    assert_eq!(settings.apply_difficulty(10.0), 5.0);

    // Zero difficulty
    settings.difficulty = 0.0;
    assert_eq!(settings.apply_difficulty(10.0), 0.0);
}

#[test]
fn test_gradient_settings_effective_gradient() {
    let mut settings = GradientSettings::default();
    settings.difficulty = 0.5;

    // 10% gradient at 50% difficulty = 5% effective
    assert_eq!(settings.effective_gradient(10.0), 5.0);

    // 20% gradient clamped to 15%, then 50% difficulty = 7.5%
    assert_eq!(settings.effective_gradient(20.0), 7.5);

    // Negative gradient
    assert_eq!(settings.effective_gradient(-10.0), -5.0);
}

#[test]
fn test_gradient_settings_validation() {
    // Valid settings
    let valid = GradientSettings::default();
    assert!(valid.validate().is_ok());

    // Invalid difficulty
    let invalid = GradientSettings {
        difficulty: 1.5,
        ..GradientSettings::default()
    };
    assert!(invalid.validate().is_err());

    // Invalid max gradient
    let invalid = GradientSettings {
        max_gradient: 30.0,
        ..GradientSettings::default()
    };
    assert!(invalid.validate().is_err());

    // Invalid min gradient
    let invalid = GradientSettings {
        min_gradient: -30.0,
        ..GradientSettings::default()
    };
    assert!(invalid.validate().is_err());

    // Invalid smoothing
    let invalid = GradientSettings {
        smoothing_secs: 15,
        ..GradientSettings::default()
    };
    assert!(invalid.validate().is_err());
}

// ========== RoutePoint Tests ==========

#[test]
fn test_route_point_new() {
    let point = RoutePoint::new(100.0, 50.0);

    assert_eq!(point.distance_m, 100.0);
    assert_eq!(point.elevation_m, 50.0);
    assert!(point.lat.is_none());
    assert!(point.lon.is_none());
}

#[test]
fn test_route_point_with_coords() {
    let point = RoutePoint::with_coords(100.0, 50.0, 45.0, -122.0);

    assert_eq!(point.distance_m, 100.0);
    assert_eq!(point.elevation_m, 50.0);
    assert_eq!(point.lat, Some(45.0));
    assert_eq!(point.lon, Some(-122.0));
}

// ========== GradientSegment Tests ==========

#[test]
fn test_gradient_segment_new() {
    let segment = GradientSegment::new(0.0, 100.0, 5.0);

    assert_eq!(segment.start_distance_m, 0.0);
    assert_eq!(segment.end_distance_m, 100.0);
    assert_eq!(segment.gradient_percent, 5.0);
}

#[test]
fn test_gradient_segment_length() {
    let segment = GradientSegment::new(50.0, 150.0, 3.0);

    assert_eq!(segment.length_m(), 100.0);
}

#[test]
fn test_gradient_segment_contains() {
    let segment = GradientSegment::new(100.0, 200.0, 5.0);

    assert!(!segment.contains(50.0));  // Before start
    assert!(segment.contains(100.0));   // At start
    assert!(segment.contains(150.0));   // In middle
    assert!(!segment.contains(200.0)); // At end (exclusive)
    assert!(!segment.contains(250.0)); // After end
}

// ========== GpxRoute Tests ==========

fn create_test_route() -> GpxRoute {
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 110.0),   // 10% grade
        RoutePoint::new(200.0, 120.0),   // 10% grade
        RoutePoint::new(300.0, 115.0),   // -5% grade (descent)
        RoutePoint::new(400.0, 110.0),   // -5% grade
    ];
    GpxRoute::from_points("Test Route", points).unwrap()
}

#[test]
fn test_gpx_route_from_points() {
    let route = create_test_route();

    assert_eq!(route.name, "Test Route");
    assert_eq!(route.total_distance_m, 400.0);
    assert!(route.has_elevation_data());
}

#[test]
fn test_gpx_route_elevation_stats() {
    let route = create_test_route();

    // Total gain: 10 + 10 = 20m (0→100→200m sections)
    assert!((route.total_elevation_m - 20.0).abs() < 1.0);

    // Total descent: 5 + 5 = 10m (200→300→400m sections)
    assert!((route.total_descent_m - 10.0).abs() < 1.0);
}

#[test]
fn test_gpx_route_has_elevation() {
    let route = create_test_route();
    assert!(route.has_elevation_data());

    // Flat route should not have "meaningful" elevation
    let flat_points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 100.0),
        RoutePoint::new(200.0, 100.0),
    ];
    let flat_route = GpxRoute::from_points("Flat", flat_points).unwrap();
    assert!(!flat_route.has_elevation_data());
}

#[test]
fn test_gpx_route_get_gradient_at() {
    let route = create_test_route();

    // Should be positive (uphill) at start
    let gradient_50 = route.get_gradient_at(50.0);
    assert!(gradient_50 > 5.0, "Expected uphill gradient, got {}", gradient_50);

    // Should be negative (downhill) in descent section
    let gradient_350 = route.get_gradient_at(350.0);
    assert!(gradient_350 < 0.0, "Expected downhill gradient, got {}", gradient_350);
}

#[test]
fn test_gpx_route_get_elevation_at() {
    let route = create_test_route();

    // At start
    assert!((route.get_elevation_at(0.0).unwrap() - 100.0).abs() < 0.1);

    // At end
    assert!((route.get_elevation_at(400.0).unwrap() - 110.0).abs() < 0.1);

    // Interpolated value at midpoint of first segment
    let mid_elevation = route.get_elevation_at(50.0).unwrap();
    assert!(mid_elevation > 100.0 && mid_elevation < 110.0,
        "Expected interpolated elevation between 100 and 110, got {}", mid_elevation);
}

#[test]
fn test_gpx_route_progress() {
    let route = create_test_route();

    assert_eq!(route.get_progress(0.0), 0.0);
    assert!((route.get_progress(200.0) - 0.5).abs() < 0.01);
    assert_eq!(route.get_progress(400.0), 1.0);
}

#[test]
fn test_gpx_route_insufficient_points() {
    // Single point should fail
    let result = GpxRoute::from_points("Short", vec![RoutePoint::new(0.0, 100.0)]);
    assert!(result.is_err());
}

#[test]
fn test_gpx_route_summary() {
    let route = create_test_route();
    let summary = route.summary();

    assert_eq!(summary.total_distance_m, 400.0);
    assert!((summary.total_elevation_m - 20.0).abs() < 1.0);
    assert!((summary.total_descent_m - 10.0).abs() < 1.0);
    assert!(summary.max_gradient > 0.0);
    assert!(summary.min_gradient < 0.0);
}

// ========== GradientSmoother Tests ==========

#[test]
fn test_gradient_smoother_creation() {
    let smoother = GradientSmoother::new(3);
    assert_eq!(smoother.current(), 0.0);
}

#[test]
fn test_gradient_smoother_from_seconds() {
    let smoother = GradientSmoother::from_seconds(3, 10); // 3 seconds at 10 Hz = 30 samples
    assert_eq!(smoother.current(), 0.0);
}

#[test]
fn test_gradient_smoother_single_value() {
    let mut smoother = GradientSmoother::new(3);
    smoother.add(10.0);

    // With only one value, should return that value
    assert!((smoother.current() - 10.0).abs() < 0.1);
}

#[test]
fn test_gradient_smoother_averaging() {
    let mut smoother = GradientSmoother::new(3);

    smoother.add(0.0);
    smoother.add(5.0);
    smoother.add(10.0);

    // Average of 0, 5, 10 = 5
    let avg = smoother.current();
    assert!((avg - 5.0).abs() < 0.5, "Expected ~5.0, got {}", avg);
}

#[test]
fn test_gradient_smoother_window_size() {
    let mut smoother = GradientSmoother::new(2);

    smoother.add(0.0);
    smoother.add(10.0);
    smoother.add(20.0); // Should push out the 0.0

    // Average of 10, 20 = 15
    let avg = smoother.current();
    assert!((avg - 15.0).abs() < 0.5, "Expected ~15.0, got {}", avg);
}

#[test]
fn test_gradient_smoother_reset() {
    let mut smoother = GradientSmoother::new(3);

    smoother.add(10.0);
    smoother.add(20.0);
    smoother.reset();

    assert_eq!(smoother.current(), 0.0);
}

// ========== GradientController Tests ==========

#[test]
fn test_gradient_controller_new() {
    let controller = GradientController::new(10);

    assert!(!controller.has_route());
    assert_eq!(controller.current_distance(), 0.0);
}

#[test]
fn test_gradient_controller_default() {
    let controller = GradientController::default();

    assert!(!controller.has_route());
}

#[test]
fn test_gradient_controller_load_route() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();

    controller.load_route(route).unwrap();

    assert!(controller.has_route());
    assert!(controller.has_elevation_data());
    assert_eq!(controller.total_distance(), Some(400.0));
}

#[test]
fn test_gradient_controller_update() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();
    let settings = GradientSettings::default();

    controller.load_route(route).unwrap();

    let (result, command) = controller.update(50.0, &settings).unwrap();

    // Should have a positive gradient (climbing section)
    assert!(result.raw_gradient > 0.0);
    // First update should generate a command
    assert!(command.is_some());
}

#[test]
fn test_gradient_controller_unload_route() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();

    controller.load_route(route).unwrap();
    assert!(controller.has_route());

    controller.unload_route();
    assert!(!controller.has_route());
}

#[test]
fn test_gradient_controller_no_route_error() {
    let mut controller = GradientController::new(10);
    let settings = GradientSettings::default();

    let result = controller.update(100.0, &settings);

    assert!(result.is_err());
}

#[test]
fn test_gradient_controller_position_out_of_bounds() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();
    let settings = GradientSettings::default();

    controller.load_route(route).unwrap();

    let result = controller.update(500.0, &settings); // Route is 400m
    assert!(result.is_err());
}

#[test]
fn test_gradient_controller_progress() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();
    let settings = GradientSettings::default();

    controller.load_route(route).unwrap();
    controller.update(200.0, &settings).unwrap();

    assert!((controller.progress() - 0.5).abs() < 0.01);
}

#[test]
fn test_gradient_controller_current_elevation() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();
    let settings = GradientSettings::default();

    controller.load_route(route).unwrap();
    controller.update(100.0, &settings).unwrap();

    let elevation = controller.current_elevation();
    assert!(elevation.is_some());
    assert!((elevation.unwrap() - 110.0).abs() < 1.0);
}

#[test]
fn test_gradient_controller_reset_position() {
    let mut controller = GradientController::new(10);
    let route = create_test_route();
    let settings = GradientSettings::default();

    controller.load_route(route).unwrap();
    controller.update(200.0, &settings).unwrap();

    controller.reset_position();
    assert_eq!(controller.current_distance(), 0.0);
}

// ========== GradientResult Tests ==========

#[test]
fn test_gradient_result_flat() {
    let result = GradientResult::flat();

    assert_eq!(result.raw_gradient, 0.0);
    assert_eq!(result.capped_gradient, 0.0);
    assert_eq!(result.effective_gradient, 0.0);
    assert_eq!(result.smoothed_gradient, 0.0);
}

#[test]
fn test_gradient_result_from_raw() {
    let result = GradientResult::from_raw(10.0, 10.0, 5.0, 4.5);

    assert_eq!(result.raw_gradient, 10.0);
    assert_eq!(result.capped_gradient, 10.0);
    assert_eq!(result.effective_gradient, 5.0);
    assert_eq!(result.smoothed_gradient, 4.5);
}

// ========== Integration Tests ==========

#[test]
fn test_full_gradient_simulation_flow() {
    // Create a route with hills
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(50.0, 105.0),     // 10% uphill
        RoutePoint::new(100.0, 115.0),    // 20% uphill
        RoutePoint::new(150.0, 110.0),    // 10% downhill
        RoutePoint::new(200.0, 100.0),    // 20% downhill
    ];

    let route = GpxRoute::from_points("Hill Climb", points).unwrap();
    let mut controller = GradientController::new(10);
    let settings = GradientSettings::with_difficulty(100);

    controller.load_route(route).unwrap();

    // Simulate riding through the route
    let mut max_gradient = 0.0_f32;
    let mut min_gradient = 0.0_f32;

    for i in 0..20 {
        let distance = i as f64 * 10.0; // Every 10 meters
        if let Ok((gradient_result, _)) = controller.update(distance, &settings) {
            max_gradient = max_gradient.max(gradient_result.raw_gradient);
            min_gradient = min_gradient.min(gradient_result.raw_gradient);
        }
    }

    // Should have detected both uphill and downhill sections
    assert!(max_gradient > 5.0, "Should have positive gradients, got {}", max_gradient);
    assert!(min_gradient < -5.0, "Should have negative gradients, got {}", min_gradient);
}

#[test]
fn test_gradient_with_different_difficulty_levels() {
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 110.0), // 10% uphill
    ];

    let route = GpxRoute::from_points("Test", points.clone()).unwrap();

    // Test at 100% difficulty
    let mut controller_100 = GradientController::new(10);
    controller_100.load_route(route).unwrap();
    let settings_100 = GradientSettings::with_difficulty(100);
    let result_100 = controller_100.update(50.0, &settings_100).unwrap();

    let route2 = GpxRoute::from_points("Test2", points).unwrap();

    // Test at 50% difficulty
    let mut controller_50 = GradientController::new(10);
    controller_50.load_route(route2).unwrap();
    let settings_50 = GradientSettings::with_difficulty(50);
    let result_50 = controller_50.update(50.0, &settings_50).unwrap();

    // The effective gradient at 50% difficulty should be half
    let ratio = result_50.0.effective_gradient / result_100.0.effective_gradient;
    assert!((ratio - 0.5).abs() < 0.1,
        "Expected 50% ratio, got {} (100%: {}, 50%: {})",
        ratio, result_100.0.effective_gradient, result_50.0.effective_gradient);
}

#[test]
fn test_gradient_capping() {
    // Very steep route that exceeds default limits
    let points = vec![
        RoutePoint::new(0.0, 100.0),
        RoutePoint::new(100.0, 150.0), // 50% gradient - very steep!
    ];

    let route = GpxRoute::from_points("Steep", points).unwrap();
    let mut controller = GradientController::new(10);
    let settings = GradientSettings::default(); // max 15%, min -15%

    controller.load_route(route).unwrap();
    let (result, _) = controller.update(50.0, &settings).unwrap();

    // Raw should be high
    assert!(result.raw_gradient > 20.0, "Expected high raw gradient");

    // Capped should be within limits
    assert!(result.capped_gradient <= 15.0,
        "Expected capped gradient <= 15%, got {}", result.capped_gradient);
}
