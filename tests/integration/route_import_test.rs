//! Integration tests for route import workflow
//!
//! T030: Integration test for route import workflow

use rustride::storage::database::Database;
use rustride::world::import::gpx::parse_gpx;
use rustride::world::import::tcx::parse_tcx;
use rustride::world::import::{
    calculate_gradient, detect_format, gps_to_world_coords, haversine_distance, simplify_route,
    FileFormat, GpsPoint, ImportOptions,
};
use rustride::world::route::{RouteSource, StoredRoute, StoredWaypoint};
use std::fs;

#[test]
fn test_detect_format_gpx() {
    let format = detect_format(std::path::Path::new("route.gpx")).unwrap();
    assert_eq!(format, FileFormat::Gpx);
}

#[test]
fn test_detect_format_fit() {
    let format = detect_format(std::path::Path::new("activity.fit")).unwrap();
    assert_eq!(format, FileFormat::Fit);
}

#[test]
fn test_detect_format_tcx() {
    let format = detect_format(std::path::Path::new("workout.tcx")).unwrap();
    assert_eq!(format, FileFormat::Tcx);
}

#[test]
fn test_detect_format_unknown() {
    let result = detect_format(std::path::Path::new("file.unknown"));
    assert!(result.is_err());
}

#[test]
fn test_detect_format_case_insensitive() {
    let format1 = detect_format(std::path::Path::new("route.GPX")).unwrap();
    let format2 = detect_format(std::path::Path::new("route.Gpx")).unwrap();
    assert_eq!(format1, FileFormat::Gpx);
    assert_eq!(format2, FileFormat::Gpx);
}

#[test]
fn test_gps_to_world_coords_origin() {
    // At origin, world coords should be (0, 0)
    let (x, y) = gps_to_world_coords(45.0, -122.0, 45.0, -122.0);
    assert!(x.abs() < 0.1);
    assert!(y.abs() < 0.1);
}

#[test]
fn test_gps_to_world_coords_offset() {
    // 1 degree longitude at equator is about 111km
    let (x, y) = gps_to_world_coords(0.0, 1.0, 0.0, 0.0);
    // Web Mercator gives different scaling, but should be positive for east
    assert!(x > 0.0);
    // Y should be near 0 since we're at same latitude
    assert!(y.abs() < 0.1);
}

#[test]
fn test_haversine_distance_zero() {
    let d = haversine_distance(45.0, -122.0, 45.0, -122.0);
    assert!(d < 0.001);
}

#[test]
fn test_haversine_distance_known() {
    // Portland, OR to Seattle, WA is about 234 km
    let d = haversine_distance(45.5231, -122.6765, 47.6062, -122.3321);
    // Should be approximately 234,000 meters
    assert!(d > 200_000.0 && d < 250_000.0);
}

#[test]
fn test_calculate_gradient() {
    // 10m rise over 100m horizontal = 10%
    let gradient = calculate_gradient(10.0, 100.0);
    assert!((gradient - 10.0).abs() < 0.1);
}

#[test]
fn test_calculate_gradient_flat() {
    // No rise = 0% gradient
    let gradient = calculate_gradient(0.0, 100.0);
    assert!(gradient.abs() < 0.01);
}

#[test]
fn test_calculate_gradient_zero_distance() {
    // Zero distance should return 0 (not divide by zero)
    let gradient = calculate_gradient(10.0, 0.0);
    assert!(gradient.abs() < 0.01);
}

#[test]
fn test_simplify_route_small() {
    // Small routes shouldn't be simplified
    let points = vec![
        GpsPoint {
            latitude: 45.0,
            longitude: -122.0,
            elevation: Some(100.0),
            timestamp: None,
        },
        GpsPoint {
            latitude: 45.1,
            longitude: -122.1,
            elevation: Some(110.0),
            timestamp: None,
        },
    ];
    let simplified = simplify_route(&points, 100.0);
    assert_eq!(simplified.len(), 2);
}

#[test]
fn test_simplify_route_removes_collinear() {
    // Points on a straight line should be simplified
    let points = vec![
        GpsPoint {
            latitude: 45.0,
            longitude: -122.0,
            elevation: Some(100.0),
            timestamp: None,
        },
        GpsPoint {
            latitude: 45.05,
            longitude: -122.05,
            elevation: Some(100.0),
            timestamp: None,
        },
        GpsPoint {
            latitude: 45.1,
            longitude: -122.1,
            elevation: Some(100.0),
            timestamp: None,
        },
    ];
    // With a high epsilon, middle point should be removed
    let simplified = simplify_route(&points, 10000.0);
    assert_eq!(simplified.len(), 2);
}

#[test]
fn test_import_options_default() {
    let options = ImportOptions::default();
    assert!(options.name.is_none());
    assert!(options.fetch_elevation);
    assert_eq!(options.max_points, 10_000);
}

#[test]
fn test_full_gpx_import_workflow() {
    // Load GPX fixture
    let content = fs::read("tests/fixtures/routes/sample_route.gpx")
        .expect("Failed to read sample_route.gpx");

    // Parse GPX
    let points = parse_gpx(&content).expect("Failed to parse GPX");
    assert_eq!(points.len(), 10);

    // Extract name
    let name = rustride::world::import::gpx::extract_name(&content)
        .unwrap_or_else(|| "Unnamed Route".to_string());
    assert_eq!(name, "Sample Test Route");

    // Calculate total distance
    let mut total_distance = 0.0;
    for i in 1..points.len() {
        total_distance += haversine_distance(
            points[i - 1].latitude,
            points[i - 1].longitude,
            points[i].latitude,
            points[i].longitude,
        );
    }
    assert!(total_distance > 100.0); // Should be some positive distance

    // Calculate elevation gain
    let mut elevation_gain = 0.0f32;
    for i in 1..points.len() {
        if let (Some(prev), Some(curr)) = (points[i - 1].elevation, points[i].elevation) {
            let diff = curr - prev;
            if diff > 0.0 {
                elevation_gain += diff;
            }
        }
    }
    assert!(elevation_gain > 0.0);

    // Create stored route
    let mut route = StoredRoute::new(name, RouteSource::Gpx);
    route.source_file = Some("sample_route.gpx".to_string());

    // Create waypoints
    let mut waypoints = Vec::new();
    let _origin_lat = points[0].latitude;
    let _origin_lon = points[0].longitude;
    let mut cumulative_distance = 0.0;

    for (i, point) in points.iter().enumerate() {
        if i > 0 {
            cumulative_distance += haversine_distance(
                points[i - 1].latitude,
                points[i - 1].longitude,
                point.latitude,
                point.longitude,
            );
        }

        let gradient = if i > 0 {
            let dist = haversine_distance(
                points[i - 1].latitude,
                points[i - 1].longitude,
                point.latitude,
                point.longitude,
            );
            let elev_change =
                point.elevation.unwrap_or(0.0) - points[i - 1].elevation.unwrap_or(0.0);
            calculate_gradient(elev_change, dist)
        } else {
            0.0
        };

        let wp = StoredWaypoint::new(
            route.id,
            i as u32,
            point.latitude,
            point.longitude,
            point.elevation.unwrap_or(0.0),
            cumulative_distance as f32,
        )
        .with_gradient(gradient);

        waypoints.push(wp);
    }

    // Update route stats
    route.update_stats(&waypoints);

    // Verify route statistics
    assert!(route.distance_meters > 100.0);
    assert!(route.elevation_gain_meters > 0.0);
    assert!(route.max_elevation_meters > route.min_elevation_meters);
}

#[test]
fn test_full_tcx_import_workflow() {
    // Load TCX fixture
    let content = fs::read("tests/fixtures/routes/sample_route.tcx")
        .expect("Failed to read sample_route.tcx");

    // Parse TCX
    let points = parse_tcx(&content).expect("Failed to parse TCX");
    assert!(!points.is_empty());

    // Extract name
    let name = rustride::world::import::tcx::extract_name(&content)
        .unwrap_or_else(|| "Unnamed Route".to_string());
    assert_eq!(name, "Sample TCX Route");

    // Create stored route
    let route = StoredRoute::new(name, RouteSource::Tcx);

    // Verify route was created
    assert_eq!(route.source, RouteSource::Tcx);
}

#[test]
fn test_database_route_storage_workflow() {
    // Create in-memory database
    let mut db = Database::open_in_memory().expect("Failed to create database");

    // Load GPX fixture
    let content = fs::read("tests/fixtures/routes/sample_route.gpx")
        .expect("Failed to read sample_route.gpx");
    let points = parse_gpx(&content).expect("Failed to parse GPX");
    let name = rustride::world::import::gpx::extract_name(&content)
        .unwrap_or_else(|| "Unnamed Route".to_string());

    // Create route
    let mut route = StoredRoute::new(name.clone(), RouteSource::Gpx);
    route.source_file = Some("sample_route.gpx".to_string());
    let route_id = route.id;

    // Create waypoints
    let mut waypoints = Vec::new();
    let mut cumulative_distance = 0.0;

    for (i, point) in points.iter().enumerate() {
        if i > 0 {
            cumulative_distance += haversine_distance(
                points[i - 1].latitude,
                points[i - 1].longitude,
                point.latitude,
                point.longitude,
            );
        }

        let wp = StoredWaypoint::new(
            route_id,
            i as u32,
            point.latitude,
            point.longitude,
            point.elevation.unwrap_or(0.0),
            cumulative_distance as f32,
        );
        waypoints.push(wp);
    }

    route.update_stats(&waypoints);

    // Store in database
    db.insert_route(&route).expect("Failed to insert route");
    db.insert_route_waypoints(&waypoints)
        .expect("Failed to insert waypoints");

    // Retrieve and verify
    let (retrieved_route, retrieved_waypoints) = db
        .get_route_with_waypoints(&route_id)
        .expect("Failed to get route")
        .expect("Route not found");

    assert_eq!(retrieved_route.name, name);
    assert_eq!(retrieved_route.source, RouteSource::Gpx);
    assert_eq!(retrieved_waypoints.len(), points.len());

    // Verify first waypoint coordinates
    assert!((retrieved_waypoints[0].latitude - points[0].latitude).abs() < 0.0001);
    assert!((retrieved_waypoints[0].longitude - points[0].longitude).abs() < 0.0001);

    // Test route listing
    let all_routes = db.list_routes(None).expect("Failed to list routes");
    assert_eq!(all_routes.len(), 1);

    let gpx_routes = db
        .list_routes(Some(RouteSource::Gpx))
        .expect("Failed to list GPX routes");
    assert_eq!(gpx_routes.len(), 1);

    let fit_routes = db
        .list_routes(Some(RouteSource::Fit))
        .expect("Failed to list FIT routes");
    assert_eq!(fit_routes.len(), 0);
}

#[test]
fn test_famous_route_import() {
    // Load famous route fixture
    let content = fs::read("tests/fixtures/famous_routes/mont_ventoux.gpx")
        .expect("Failed to read mont_ventoux.gpx");

    // Parse GPX
    let points = parse_gpx(&content).expect("Failed to parse GPX");
    assert!(!points.is_empty());

    // Extract name
    let name = rustride::world::import::gpx::extract_name(&content)
        .unwrap_or_else(|| "Unnamed Route".to_string());
    assert!(name.contains("Mont Ventoux"));

    // Verify elevation profile
    let min_elev = points
        .iter()
        .filter_map(|p| p.elevation)
        .fold(f32::INFINITY, f32::min);
    let max_elev = points
        .iter()
        .filter_map(|p| p.elevation)
        .fold(f32::NEG_INFINITY, f32::max);

    // Mont Ventoux: 310m start, 1912m summit
    assert!(min_elev < 500.0);
    assert!(max_elev > 1800.0);

    // Create and verify route
    let mut route = StoredRoute::new(name, RouteSource::Famous);

    // Create waypoints for stats calculation
    let mut waypoints = Vec::new();
    let mut cumulative_distance = 0.0;

    for (i, point) in points.iter().enumerate() {
        if i > 0 {
            cumulative_distance += haversine_distance(
                points[i - 1].latitude,
                points[i - 1].longitude,
                point.latitude,
                point.longitude,
            );
        }

        let gradient = if i > 0 {
            let dist = haversine_distance(
                points[i - 1].latitude,
                points[i - 1].longitude,
                point.latitude,
                point.longitude,
            );
            let elev_change =
                point.elevation.unwrap_or(0.0) - points[i - 1].elevation.unwrap_or(0.0);
            calculate_gradient(elev_change, dist)
        } else {
            0.0
        };

        let wp = StoredWaypoint::new(
            route.id,
            i as u32,
            point.latitude,
            point.longitude,
            point.elevation.unwrap_or(0.0),
            cumulative_distance as f32,
        )
        .with_gradient(gradient);

        waypoints.push(wp);
    }

    route.update_stats(&waypoints);

    // Verify significant climbing
    assert!(route.elevation_gain_meters > 1500.0);
    assert!(route.max_gradient_percent > 5.0);
}
