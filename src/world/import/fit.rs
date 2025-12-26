//! FIT file parser for route import.

use super::{GpsPoint, ImportError};
use chrono::{DateTime, Utc};

/// Parse FIT file content to GPS points
pub fn parse_fit(content: &[u8]) -> Result<Vec<GpsPoint>, ImportError> {
    let mut points = Vec::new();

    // Parse FIT file
    let fit_data = fitparser::from_bytes(content)
        .map_err(|e| ImportError::ParseError(format!("FIT parse error: {}", e)))?;

    for record in fit_data {
        // Look for record messages with position data
        if record.kind() == fitparser::profile::MesgNum::Record {
            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;
            let mut elevation: Option<f32> = None;
            let mut timestamp: Option<DateTime<Utc>> = None;

            for field in record.fields() {
                match field.name() {
                    "position_lat" => {
                        if let fitparser::Value::SInt32(v) = field.value() {
                            // Convert semicircles to degrees
                            lat = Some(*v as f64 * (180.0 / 2_147_483_648.0));
                        }
                    }
                    "position_long" => {
                        if let fitparser::Value::SInt32(v) = field.value() {
                            lon = Some(*v as f64 * (180.0 / 2_147_483_648.0));
                        }
                    }
                    "altitude" | "enhanced_altitude" => {
                        if let fitparser::Value::Float64(v) = field.value() {
                            elevation = Some(*v as f32);
                        }
                    }
                    "timestamp" => {
                        if let fitparser::Value::Timestamp(t) = field.value() {
                            timestamp = Some((*t).into());
                        }
                    }
                    _ => {}
                }
            }

            if let (Some(latitude), Some(longitude)) = (lat, lon) {
                // Filter out invalid coordinates (0,0 or extreme values)
                if latitude.abs() > 0.0001
                    && longitude.abs() > 0.0001
                    && latitude.abs() <= 90.0
                    && longitude.abs() <= 180.0
                {
                    points.push(GpsPoint {
                        latitude,
                        longitude,
                        elevation,
                        timestamp,
                    });
                }
            }
        }
    }

    if points.is_empty() {
        return Err(ImportError::ParseError(
            "No GPS points found in FIT file".to_string(),
        ));
    }

    Ok(points)
}

/// Extract activity name from FIT file
pub fn extract_name(content: &[u8]) -> Option<String> {
    let fit_data = fitparser::from_bytes(content).ok()?;

    for record in fit_data {
        // Look for session or lap messages with name
        if record.kind() == fitparser::profile::MesgNum::Session
            || record.kind() == fitparser::profile::MesgNum::Activity
        {
            for field in record.fields() {
                if field.name() == "sport" {
                    if let fitparser::Value::String(name) = field.value() {
                        return Some(name.clone());
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: FIT file tests would require actual binary FIT test files
    // These tests are placeholders that will be expanded with test fixtures

    #[test]
    fn test_empty_fit_returns_error() {
        let result = parse_fit(&[]);
        assert!(result.is_err());
    }
}
