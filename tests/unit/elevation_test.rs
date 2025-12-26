//! Unit tests for elevation service
//!
//! T029: Unit test for elevation service client

use rustride::world::import::elevation::ElevationService;

#[test]
fn test_elevation_service_creation() {
    let service = ElevationService::new();
    // Service should be created successfully
    // We can't easily verify internal state, but creation should not panic
    assert!(true);
}

#[test]
fn test_elevation_service_with_custom_url() {
    let service = ElevationService::with_url("http://localhost:8080/api");
    // Custom URL service should be created successfully
    assert!(true);
}

#[test]
fn test_elevation_service_default() {
    let service = ElevationService::default();
    // Default should work same as new()
    assert!(true);
}

// Note: Async tests for actual elevation fetching would require:
// 1. A mock HTTP server or
// 2. Real network access to open-elevation.com
//
// The actual fetch_elevation functionality is tested implicitly
// through integration tests that use the full import workflow.
//
// Key behaviors tested by integration tests:
// - Batch processing of points
// - Error handling for failed requests
// - Points with existing elevation are not re-fetched

/// Test that the service correctly identifies points missing elevation
#[tokio::test]
async fn test_elevation_service_skips_existing() {
    use rustride::world::import::GpsPoint;

    // Create points where all have elevation already
    let mut points = vec![
        GpsPoint {
            latitude: 45.5,
            longitude: -122.5,
            elevation: Some(100.0),
            timestamp: None,
        },
        GpsPoint {
            latitude: 45.51,
            longitude: -122.51,
            elevation: Some(110.0),
            timestamp: None,
        },
    ];

    // Use a URL that doesn't exist - since all points have elevation,
    // no network request should be made
    let service = ElevationService::with_url("http://invalid-url-that-does-not-exist.local/api");

    // This should return 0 (no points fetched) since all have elevation
    let result = service.fetch_elevation(&mut points).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    // Elevations should remain unchanged
    assert_eq!(points[0].elevation, Some(100.0));
    assert_eq!(points[1].elevation, Some(110.0));
}

/// Test that the service can handle empty point arrays
#[tokio::test]
async fn test_elevation_service_empty_points() {
    let mut points: Vec<rustride::world::import::GpsPoint> = vec![];
    let service = ElevationService::new();

    let result = service.fetch_elevation(&mut points).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}
