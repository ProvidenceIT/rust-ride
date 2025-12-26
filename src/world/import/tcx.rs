//! TCX file parser for route import.

use super::{GpsPoint, ImportError};
use chrono::{DateTime, Utc};
use quick_xml::de::from_str;
use serde::Deserialize;

/// Parse TCX file content to GPS points
pub fn parse_tcx(content: &[u8]) -> Result<Vec<GpsPoint>, ImportError> {
    let content_str = std::str::from_utf8(content)
        .map_err(|e| ImportError::ParseError(format!("Invalid UTF-8: {}", e)))?;

    let tcx: TrainingCenterDatabase = from_str(content_str)
        .map_err(|e| ImportError::ParseError(format!("TCX parse error: {}", e)))?;

    let mut points = Vec::new();

    if let Some(activities) = tcx.activities {
        for activity in activities.activity {
            for lap in activity.lap {
                if let Some(track) = lap.track {
                    for trackpoint in track.trackpoint {
                        if let Some(position) = trackpoint.position {
                            points.push(GpsPoint {
                                latitude: position.latitude_degrees,
                                longitude: position.longitude_degrees,
                                elevation: trackpoint.altitude_meters.map(|a| a as f32),
                                timestamp: trackpoint.time.and_then(|t| {
                                    DateTime::parse_from_rfc3339(&t)
                                        .ok()
                                        .map(|dt| dt.with_timezone(&Utc))
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    // Also try courses
    if points.is_empty() {
        if let Some(courses) = tcx.courses {
            for course in courses.course {
                if let Some(track) = course.track {
                    for trackpoint in track.trackpoint {
                        if let Some(position) = trackpoint.position {
                            points.push(GpsPoint {
                                latitude: position.latitude_degrees,
                                longitude: position.longitude_degrees,
                                elevation: trackpoint.altitude_meters.map(|a| a as f32),
                                timestamp: trackpoint.time.and_then(|t| {
                                    DateTime::parse_from_rfc3339(&t)
                                        .ok()
                                        .map(|dt| dt.with_timezone(&Utc))
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    if points.is_empty() {
        return Err(ImportError::ParseError(
            "No GPS points found in TCX file".to_string(),
        ));
    }

    Ok(points)
}

/// Extract route/activity name from TCX file
pub fn extract_name(content: &[u8]) -> Option<String> {
    let content_str = std::str::from_utf8(content).ok()?;
    let tcx: TrainingCenterDatabase = from_str(content_str).ok()?;

    // Try activity name first
    if let Some(activities) = tcx.activities {
        if let Some(activity) = activities.activity.first() {
            if let Some(notes) = &activity.notes {
                return Some(notes.clone());
            }
            // Use sport as fallback
            return Some(activity.sport.clone());
        }
    }

    // Try course name
    if let Some(courses) = tcx.courses {
        if let Some(course) = courses.course.first() {
            return Some(course.name.clone());
        }
    }

    None
}

// TCX XML structures

#[derive(Debug, Deserialize)]
#[serde(rename = "TrainingCenterDatabase")]
struct TrainingCenterDatabase {
    #[serde(rename = "Activities")]
    activities: Option<Activities>,
    #[serde(rename = "Courses")]
    courses: Option<Courses>,
}

#[derive(Debug, Deserialize)]
struct Activities {
    #[serde(rename = "Activity", default)]
    activity: Vec<Activity>,
}

#[derive(Debug, Deserialize)]
struct Activity {
    #[serde(rename = "@Sport")]
    sport: String,
    #[serde(rename = "Notes")]
    notes: Option<String>,
    #[serde(rename = "Lap", default)]
    lap: Vec<Lap>,
}

#[derive(Debug, Deserialize)]
struct Lap {
    #[serde(rename = "Track")]
    track: Option<Track>,
}

#[derive(Debug, Deserialize)]
struct Track {
    #[serde(rename = "Trackpoint", default)]
    trackpoint: Vec<Trackpoint>,
}

#[derive(Debug, Deserialize)]
struct Trackpoint {
    #[serde(rename = "Time")]
    time: Option<String>,
    #[serde(rename = "Position")]
    position: Option<Position>,
    #[serde(rename = "AltitudeMeters")]
    altitude_meters: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Position {
    #[serde(rename = "LatitudeDegrees")]
    latitude_degrees: f64,
    #[serde(rename = "LongitudeDegrees")]
    longitude_degrees: f64,
}

#[derive(Debug, Deserialize)]
struct Courses {
    #[serde(rename = "Course", default)]
    course: Vec<Course>,
}

#[derive(Debug, Deserialize)]
struct Course {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Track")]
    track: Option<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TCX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities>
    <Activity Sport="Cycling">
      <Notes>Test Ride</Notes>
      <Lap>
        <Track>
          <Trackpoint>
            <Time>2024-01-01T00:00:00Z</Time>
            <Position>
              <LatitudeDegrees>45.5</LatitudeDegrees>
              <LongitudeDegrees>-122.5</LongitudeDegrees>
            </Position>
            <AltitudeMeters>100</AltitudeMeters>
          </Trackpoint>
          <Trackpoint>
            <Time>2024-01-01T00:01:00Z</Time>
            <Position>
              <LatitudeDegrees>45.51</LatitudeDegrees>
              <LongitudeDegrees>-122.51</LongitudeDegrees>
            </Position>
            <AltitudeMeters>110</AltitudeMeters>
          </Trackpoint>
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>"#;

    #[test]
    fn test_parse_tcx_basic() {
        let points = parse_tcx(SAMPLE_TCX.as_bytes()).unwrap();
        assert_eq!(points.len(), 2);
        assert!((points[0].latitude - 45.5).abs() < 0.001);
        assert!((points[0].longitude - (-122.5)).abs() < 0.001);
        assert_eq!(points[0].elevation, Some(100.0));
    }

    #[test]
    fn test_extract_name_tcx() {
        let name = extract_name(SAMPLE_TCX.as_bytes());
        assert_eq!(name, Some("Test Ride".to_string()));
    }
}
