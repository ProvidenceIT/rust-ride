//! CSV export functionality for ride data.
//!
//! T098: Implement CSV export of raw samples

use crate::recording::types::{ExportError, Ride, RideSample};
use chrono::Duration;
use std::io::Write;

/// Export ride samples to CSV format.
pub fn export_csv(ride: &Ride, samples: &[RideSample]) -> Result<String, ExportError> {
    if samples.is_empty() {
        return Err(ExportError::NoData);
    }

    let mut output = Vec::new();

    // Write header
    writeln!(
        output,
        "timestamp,elapsed_seconds,power_watts,cadence_rpm,heart_rate_bpm,speed_kmh,distance_meters,calories,target_power"
    )
    .map_err(|e| ExportError::WriteFailed(e.to_string()))?;

    // Write data rows
    for sample in samples {
        let timestamp = ride.started_at + Duration::seconds(sample.elapsed_seconds as i64);
        writeln!(
            output,
            "{},{},{},{},{},{},{:.1},{},{}",
            timestamp.to_rfc3339(),
            sample.elapsed_seconds,
            sample.power_watts.map_or(String::new(), |v| v.to_string()),
            sample.cadence_rpm.map_or(String::new(), |v| v.to_string()),
            sample
                .heart_rate_bpm
                .map_or(String::new(), |v| v.to_string()),
            sample
                .speed_kmh
                .map_or(String::new(), |v| format!("{:.2}", v)),
            sample.distance_meters,
            sample.calories,
            sample.target_power.map_or(String::new(), |v| v.to_string()),
        )
        .map_err(|e| ExportError::WriteFailed(e.to_string()))?;
    }

    String::from_utf8(output).map_err(|e| ExportError::WriteFailed(e.to_string()))
}

/// Export ride summary to CSV format.
pub fn export_summary_csv(ride: &Ride) -> Result<String, ExportError> {
    let mut output = Vec::new();

    // Write header
    writeln!(
        output,
        "started_at,ended_at,duration_seconds,distance_meters,avg_power,max_power,normalized_power,intensity_factor,tss,avg_hr,max_hr,avg_cadence,calories,ftp"
    )
    .map_err(|e| ExportError::WriteFailed(e.to_string()))?;

    // Write data row
    writeln!(
        output,
        "{},{},{},{:.1},{},{},{},{:.2},{:.1},{},{},{},{},{}",
        ride.started_at.to_rfc3339(),
        ride.ended_at.map_or(String::new(), |dt| dt.to_rfc3339()),
        ride.duration_seconds,
        ride.distance_meters,
        ride.avg_power.map_or(String::new(), |v| v.to_string()),
        ride.max_power.map_or(String::new(), |v| v.to_string()),
        ride.normalized_power
            .map_or(String::new(), |v| v.to_string()),
        ride.intensity_factor.unwrap_or(0.0),
        ride.tss.unwrap_or(0.0),
        ride.avg_hr.map_or(String::new(), |v| v.to_string()),
        ride.max_hr.map_or(String::new(), |v| v.to_string()),
        ride.avg_cadence.map_or(String::new(), |v| v.to_string()),
        ride.calories,
        ride.ftp_at_ride,
    )
    .map_err(|e| ExportError::WriteFailed(e.to_string()))?;

    String::from_utf8(output).map_err(|e| ExportError::WriteFailed(e.to_string()))
}

/// Export a ride to CSV and write to a file.
pub fn export_csv_to_file(
    ride: &Ride,
    samples: &[RideSample],
    path: &std::path::Path,
) -> Result<(), ExportError> {
    let content = export_csv(ride, samples)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a ride CSV export.
pub fn generate_csv_filename(ride: &Ride) -> String {
    let timestamp = ride.started_at.format("%Y%m%d_%H%M%S");
    format!("RustRide_{}.csv", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_ride() -> Ride {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.started_at = Utc::now();
        ride.ended_at = Some(Utc::now());
        ride.duration_seconds = 3600;
        ride.distance_meters = 30000.0;
        ride.avg_power = Some(180);
        ride.max_power = Some(350);
        ride.normalized_power = Some(190);
        ride.intensity_factor = Some(0.95);
        ride.tss = Some(85.0);
        ride.avg_hr = Some(145);
        ride.max_hr = Some(175);
        ride.avg_cadence = Some(85);
        ride.calories = 720;
        ride
    }

    fn create_test_samples(count: usize) -> Vec<RideSample> {
        (0..count)
            .map(|i| RideSample {
                elapsed_seconds: i as u32,
                power_watts: Some(180 + (i % 30) as u16),
                cadence_rpm: Some(85),
                heart_rate_bpm: Some(145),
                speed_kmh: Some(30.0),
                distance_meters: i as f64 * 8.33,
                calories: (i as f64 * 0.2) as u32,
                resistance_level: None,
                target_power: Some(180),
                trainer_grade: None,
            })
            .collect()
    }

    #[test]
    fn test_export_csv_generates_content() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let result = export_csv(&ride, &samples);
        assert!(result.is_ok());

        let csv = result.unwrap();
        let lines: Vec<&str> = csv.lines().collect();

        // Header + 10 data rows
        assert_eq!(lines.len(), 11);
    }

    #[test]
    fn test_export_csv_has_header() {
        let ride = create_test_ride();
        let samples = create_test_samples(5);

        let csv = export_csv(&ride, &samples).unwrap();
        let header = csv.lines().next().unwrap();

        assert!(header.contains("timestamp"));
        assert!(header.contains("power_watts"));
        assert!(header.contains("heart_rate_bpm"));
        assert!(header.contains("cadence_rpm"));
        assert!(header.contains("distance_meters"));
    }

    #[test]
    fn test_export_csv_data_format() {
        let ride = create_test_ride();
        let samples = create_test_samples(3);

        let csv = export_csv(&ride, &samples).unwrap();
        let data_row = csv.lines().nth(1).unwrap();

        // Should contain comma-separated values
        let fields: Vec<&str> = data_row.split(',').collect();
        assert_eq!(fields.len(), 9);

        // elapsed_seconds should be "0"
        assert_eq!(fields[1], "0");
    }

    #[test]
    fn test_export_csv_empty_samples_error() {
        let ride = create_test_ride();
        let samples: Vec<RideSample> = vec![];

        let result = export_csv(&ride, &samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_summary_csv() {
        let ride = create_test_ride();

        let result = export_summary_csv(&ride);
        assert!(result.is_ok());

        let csv = result.unwrap();
        let lines: Vec<&str> = csv.lines().collect();

        // Header + 1 data row
        assert_eq!(lines.len(), 2);

        let header = lines[0];
        assert!(header.contains("avg_power"));
        assert!(header.contains("tss"));
    }

    #[test]
    fn test_generate_filename() {
        let ride = create_test_ride();
        let filename = generate_csv_filename(&ride);

        assert!(filename.starts_with("RustRide_"));
        assert!(filename.ends_with(".csv"));
    }

    #[test]
    fn test_csv_handles_missing_data() {
        let ride = create_test_ride();
        let samples = vec![RideSample {
            elapsed_seconds: 0,
            power_watts: Some(200),
            cadence_rpm: None,    // Missing
            heart_rate_bpm: None, // Missing
            speed_kmh: Some(30.0),
            distance_meters: 0.0,
            calories: 0,
            resistance_level: None,
            target_power: None, // Missing
            trainer_grade: None,
        }];

        let csv = export_csv(&ride, &samples).unwrap();
        let data_row = csv.lines().nth(1).unwrap();

        // Should have empty fields for missing data
        assert!(data_row.contains(",,"));
    }
}
