//! Unit tests for GPX file parsing
//!
//! T026: Unit test for GPX parsing

use rustride::world::import::gpx::{extract_name, parse_gpx};
use std::fs;

const SAMPLE_GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <metadata>
    <name>Metadata Name</name>
  </metadata>
  <trk>
    <name>Track Name</name>
    <trkseg>
      <trkpt lat="45.5" lon="-122.5">
        <ele>100</ele>
        <time>2024-01-01T00:00:00Z</time>
      </trkpt>
      <trkpt lat="45.51" lon="-122.51">
        <ele>110</ele>
        <time>2024-01-01T00:01:00Z</time>
      </trkpt>
      <trkpt lat="45.52" lon="-122.52">
        <ele>120</ele>
        <time>2024-01-01T00:02:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

const ROUTE_GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <rte>
    <name>Route Name</name>
    <rtept lat="45.5" lon="-122.5">
      <ele>100</ele>
    </rtept>
    <rtept lat="45.51" lon="-122.51">
      <ele>110</ele>
    </rtept>
  </rte>
</gpx>"#;

const WAYPOINTS_GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <wpt lat="45.5" lon="-122.5">
    <ele>100</ele>
    <name>Point 1</name>
  </wpt>
  <wpt lat="45.51" lon="-122.51">
    <ele>110</ele>
    <name>Point 2</name>
  </wpt>
</gpx>"#;

#[test]
fn test_parse_gpx_track() {
    let points = parse_gpx(SAMPLE_GPX.as_bytes()).unwrap();
    assert_eq!(points.len(), 3);
    assert!((points[0].latitude - 45.5).abs() < 0.001);
    assert!((points[0].longitude - (-122.5)).abs() < 0.001);
    assert_eq!(points[0].elevation, Some(100.0));
    assert!(points[0].timestamp.is_some());
}

#[test]
fn test_parse_gpx_route() {
    let points = parse_gpx(ROUTE_GPX.as_bytes()).unwrap();
    assert_eq!(points.len(), 2);
    assert!((points[0].latitude - 45.5).abs() < 0.001);
    assert_eq!(points[0].elevation, Some(100.0));
}

#[test]
fn test_parse_gpx_waypoints() {
    let points = parse_gpx(WAYPOINTS_GPX.as_bytes()).unwrap();
    assert_eq!(points.len(), 2);
    assert!((points[0].latitude - 45.5).abs() < 0.001);
}

#[test]
fn test_extract_name_track() {
    let name = extract_name(SAMPLE_GPX.as_bytes());
    assert_eq!(name, Some("Track Name".to_string()));
}

#[test]
fn test_extract_name_route() {
    let name = extract_name(ROUTE_GPX.as_bytes());
    assert_eq!(name, Some("Route Name".to_string()));
}

#[test]
fn test_parse_gpx_empty() {
    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test"></gpx>"#;
    let result = parse_gpx(empty.as_bytes());
    assert!(result.is_err());
}

#[test]
fn test_parse_gpx_invalid_xml() {
    let invalid = b"not valid xml";
    let result = parse_gpx(invalid);
    assert!(result.is_err());
}

#[test]
fn test_parse_sample_route_fixture() {
    // Parse the sample GPX fixture file
    let content = fs::read("tests/fixtures/routes/sample_route.gpx")
        .expect("Failed to read sample_route.gpx fixture");

    let points = parse_gpx(&content).expect("Failed to parse sample_route.gpx");

    // Verify we got points
    assert!(!points.is_empty());
    assert_eq!(points.len(), 10);

    // Verify first point (Berlin area)
    assert!((points[0].latitude - 52.52).abs() < 0.01);
    assert!((points[0].longitude - 13.405).abs() < 0.01);
    assert_eq!(points[0].elevation, Some(35.0));
}

#[test]
fn test_parse_short_loop_fixture() {
    // Parse the short loop GPX fixture file
    let content = fs::read("tests/fixtures/routes/short_loop.gpx")
        .expect("Failed to read short_loop.gpx fixture");

    let points = parse_gpx(&content).expect("Failed to parse short_loop.gpx");

    // Verify we got points
    assert_eq!(points.len(), 5);

    // Verify it's a loop (first and last points are the same)
    let first = &points[0];
    let last = &points[points.len() - 1];
    assert!((first.latitude - last.latitude).abs() < 0.0001);
    assert!((first.longitude - last.longitude).abs() < 0.0001);
}

#[test]
fn test_parse_famous_routes() {
    // Parse Alpe d'Huez fixture
    let content = fs::read("tests/fixtures/famous_routes/alpe_dhuez.gpx")
        .expect("Failed to read alpe_dhuez.gpx");

    let points = parse_gpx(&content).expect("Failed to parse alpe_dhuez.gpx");
    assert!(!points.is_empty());

    // Verify elevation range (720m to 1791m)
    let min_elev = points
        .iter()
        .filter_map(|p| p.elevation)
        .fold(f32::INFINITY, f32::min);
    let max_elev = points
        .iter()
        .filter_map(|p| p.elevation)
        .fold(f32::NEG_INFINITY, f32::max);

    assert!((700.0..=750.0).contains(&min_elev));
    assert!((1750.0..=1800.0).contains(&max_elev));

    // Verify name extraction
    let name = extract_name(&content);
    assert_eq!(name, Some("Alpe d'Huez".to_string()));
}
