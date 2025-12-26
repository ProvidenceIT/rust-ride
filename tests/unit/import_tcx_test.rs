//! Unit tests for TCX file parsing
//!
//! T028: Unit test for TCX parsing

use rustride::world::import::tcx::{extract_name, parse_tcx};
use std::fs;

const SAMPLE_TCX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities>
    <Activity Sport="Biking">
      <Notes>Test Ride Activity</Notes>
      <Lap>
        <Track>
          <Trackpoint>
            <Time>2024-01-01T10:00:00Z</Time>
            <Position>
              <LatitudeDegrees>45.5</LatitudeDegrees>
              <LongitudeDegrees>-122.5</LongitudeDegrees>
            </Position>
            <AltitudeMeters>100</AltitudeMeters>
          </Trackpoint>
          <Trackpoint>
            <Time>2024-01-01T10:01:00Z</Time>
            <Position>
              <LatitudeDegrees>45.51</LatitudeDegrees>
              <LongitudeDegrees>-122.51</LongitudeDegrees>
            </Position>
            <AltitudeMeters>110</AltitudeMeters>
          </Trackpoint>
          <Trackpoint>
            <Time>2024-01-01T10:02:00Z</Time>
            <Position>
              <LatitudeDegrees>45.52</LatitudeDegrees>
              <LongitudeDegrees>-122.52</LongitudeDegrees>
            </Position>
            <AltitudeMeters>120</AltitudeMeters>
          </Trackpoint>
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>"#;

const COURSE_TCX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Courses>
    <Course>
      <Name>Test Course</Name>
      <Track>
        <Trackpoint>
          <Time>2024-01-01T10:00:00Z</Time>
          <Position>
            <LatitudeDegrees>45.5</LatitudeDegrees>
            <LongitudeDegrees>-122.5</LongitudeDegrees>
          </Position>
          <AltitudeMeters>100</AltitudeMeters>
        </Trackpoint>
        <Trackpoint>
          <Time>2024-01-01T10:01:00Z</Time>
          <Position>
            <LatitudeDegrees>45.51</LatitudeDegrees>
            <LongitudeDegrees>-122.51</LongitudeDegrees>
          </Position>
          <AltitudeMeters>110</AltitudeMeters>
        </Trackpoint>
      </Track>
    </Course>
  </Courses>
</TrainingCenterDatabase>"#;

#[test]
fn test_parse_tcx_activity() {
    let points = parse_tcx(SAMPLE_TCX.as_bytes()).unwrap();
    assert_eq!(points.len(), 3);
    assert!((points[0].latitude - 45.5).abs() < 0.001);
    assert!((points[0].longitude - (-122.5)).abs() < 0.001);
    assert_eq!(points[0].elevation, Some(100.0));
    assert!(points[0].timestamp.is_some());
}

#[test]
fn test_parse_tcx_course() {
    let points = parse_tcx(COURSE_TCX.as_bytes()).unwrap();
    assert_eq!(points.len(), 2);
    assert!((points[0].latitude - 45.5).abs() < 0.001);
    assert_eq!(points[0].elevation, Some(100.0));
}

#[test]
fn test_extract_name_activity() {
    let name = extract_name(SAMPLE_TCX.as_bytes());
    assert_eq!(name, Some("Test Ride Activity".to_string()));
}

#[test]
fn test_extract_name_course() {
    let name = extract_name(COURSE_TCX.as_bytes());
    assert_eq!(name, Some("Test Course".to_string()));
}

#[test]
fn test_parse_tcx_empty() {
    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
</TrainingCenterDatabase>"#;
    let result = parse_tcx(empty.as_bytes());
    assert!(result.is_err());
}

#[test]
fn test_parse_tcx_invalid_xml() {
    let invalid = b"not valid xml";
    let result = parse_tcx(invalid);
    assert!(result.is_err());
}

#[test]
fn test_parse_sample_route_tcx_fixture() {
    // Parse the sample TCX fixture file
    let content = fs::read("tests/fixtures/routes/sample_route.tcx")
        .expect("Failed to read sample_route.tcx fixture");

    let points = parse_tcx(&content).expect("Failed to parse sample_route.tcx");

    // Verify we got points
    assert!(!points.is_empty());
    assert_eq!(points.len(), 6);

    // Verify first point (Berlin area)
    assert!((points[0].latitude - 52.52).abs() < 0.01);
    assert!((points[0].longitude - 13.405).abs() < 0.01);
    assert_eq!(points[0].elevation, Some(35.0));

    // Verify elevation increases over the route
    let first_elev = points[0].elevation.unwrap();
    let last_elev = points[points.len() - 1].elevation.unwrap();
    assert!(last_elev > first_elev);
}

#[test]
fn test_parse_tcx_with_missing_position() {
    // TCX with trackpoint without position should be skipped
    let tcx_with_missing = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities>
    <Activity Sport="Biking">
      <Lap>
        <Track>
          <Trackpoint>
            <Time>2024-01-01T10:00:00Z</Time>
            <Position>
              <LatitudeDegrees>45.5</LatitudeDegrees>
              <LongitudeDegrees>-122.5</LongitudeDegrees>
            </Position>
            <AltitudeMeters>100</AltitudeMeters>
          </Trackpoint>
          <Trackpoint>
            <Time>2024-01-01T10:01:00Z</Time>
            <AltitudeMeters>110</AltitudeMeters>
          </Trackpoint>
          <Trackpoint>
            <Time>2024-01-01T10:02:00Z</Time>
            <Position>
              <LatitudeDegrees>45.52</LatitudeDegrees>
              <LongitudeDegrees>-122.52</LongitudeDegrees>
            </Position>
            <AltitudeMeters>120</AltitudeMeters>
          </Trackpoint>
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>"#;

    let points = parse_tcx(tcx_with_missing.as_bytes()).unwrap();
    // Should only have 2 points (middle one has no position)
    assert_eq!(points.len(), 2);
}
