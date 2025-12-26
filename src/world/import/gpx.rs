//! GPX file parser for route import.

use super::{GpsPoint, ImportError};
use chrono::{DateTime, Utc};

/// Convert gpx Time to chrono DateTime
fn gpx_time_to_chrono(time: gpx::Time) -> Option<DateTime<Utc>> {
    // gpx::Time wraps time::OffsetDateTime, convert via string format
    let formatted = time.format().ok()?;
    DateTime::parse_from_rfc3339(&formatted)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Parse GPX file content to GPS points
pub fn parse_gpx(content: &[u8]) -> Result<Vec<GpsPoint>, ImportError> {
    let content_str = std::str::from_utf8(content)
        .map_err(|e| ImportError::ParseError(format!("Invalid UTF-8: {}", e)))?;

    let gpx_data: gpx::Gpx = gpx::read(content_str.as_bytes())
        .map_err(|e| ImportError::ParseError(format!("GPX parse error: {}", e)))?;

    let mut points = Vec::new();

    // Extract points from tracks
    for track in gpx_data.tracks {
        for segment in track.segments {
            for point in segment.points {
                points.push(GpsPoint {
                    latitude: point.point().y(),
                    longitude: point.point().x(),
                    elevation: point.elevation.map(|e| e as f32),
                    timestamp: point.time.and_then(gpx_time_to_chrono),
                });
            }
        }
    }

    // If no tracks, try routes
    if points.is_empty() {
        for route in gpx_data.routes {
            for point in route.points {
                points.push(GpsPoint {
                    latitude: point.point().y(),
                    longitude: point.point().x(),
                    elevation: point.elevation.map(|e| e as f32),
                    timestamp: point.time.and_then(gpx_time_to_chrono),
                });
            }
        }
    }

    // If still empty, try waypoints
    if points.is_empty() {
        for point in gpx_data.waypoints {
            points.push(GpsPoint {
                latitude: point.point().y(),
                longitude: point.point().x(),
                elevation: point.elevation.map(|e| e as f32),
                timestamp: point.time.and_then(gpx_time_to_chrono),
            });
        }
    }

    if points.is_empty() {
        return Err(ImportError::ParseError(
            "No GPS points found in GPX file".to_string(),
        ));
    }

    Ok(points)
}

/// Extract route name from GPX file
pub fn extract_name(content: &[u8]) -> Option<String> {
    let content_str = std::str::from_utf8(content).ok()?;
    let gpx_data: gpx::Gpx = gpx::read(content_str.as_bytes()).ok()?;

    // Try track name first
    if let Some(track) = gpx_data.tracks.first() {
        if let Some(name) = &track.name {
            return Some(name.clone());
        }
    }

    // Try route name
    if let Some(route) = gpx_data.routes.first() {
        if let Some(name) = &route.name {
            return Some(name.clone());
        }
    }

    // Try metadata name
    gpx_data.metadata.and_then(|m| m.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <trk>
    <name>Test Route</name>
    <trkseg>
      <trkpt lat="45.5" lon="-122.5">
        <ele>100</ele>
        <time>2024-01-01T00:00:00Z</time>
      </trkpt>
      <trkpt lat="45.51" lon="-122.51">
        <ele>110</ele>
        <time>2024-01-01T00:01:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

    #[test]
    fn test_parse_gpx_basic() {
        let points = parse_gpx(SAMPLE_GPX.as_bytes()).unwrap();
        assert_eq!(points.len(), 2);
        assert!((points[0].latitude - 45.5).abs() < 0.001);
        assert!((points[0].longitude - (-122.5)).abs() < 0.001);
        assert_eq!(points[0].elevation, Some(100.0));
    }

    #[test]
    fn test_extract_name() {
        let name = extract_name(SAMPLE_GPX.as_bytes());
        assert_eq!(name, Some("Test Route".to_string()));
    }
}
