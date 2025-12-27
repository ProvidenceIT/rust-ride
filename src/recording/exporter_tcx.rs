//! TCX export functionality for ride data.
//!
//! T095: Implement TCX XML structure generation with quick-xml
//! T096: Include power data in TCX ActivityExtension/TPX

use crate::recording::types::{ExportError, Ride, RideSample};
use chrono::{DateTime, Duration, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

/// TCX XML namespaces
const NS_TCX: &str = "http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2";
const NS_TPX: &str = "http://www.garmin.com/xmlschemas/ActivityExtension/v2";
const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const SCHEMA_LOCATION: &str = "http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2 http://www.garmin.com/xmlschemas/TrainingCenterDatabasev2.xsd";

/// Export a ride to TCX format.
pub fn export_tcx(ride: &Ride, samples: &[RideSample]) -> Result<String, ExportError> {
    if samples.is_empty() {
        return Err(ExportError::NoData);
    }

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Root element
    let mut root = BytesStart::new("TrainingCenterDatabase");
    root.push_attribute(("xmlns", NS_TCX));
    root.push_attribute(("xmlns:ns3", NS_TPX));
    root.push_attribute(("xmlns:xsi", NS_XSI));
    root.push_attribute(("xsi:schemaLocation", SCHEMA_LOCATION));
    writer
        .write_event(Event::Start(root))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Activities container
    writer
        .write_event(Event::Start(BytesStart::new("Activities")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Single Activity
    let mut activity = BytesStart::new("Activity");
    activity.push_attribute(("Sport", "Biking"));
    writer
        .write_event(Event::Start(activity))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Activity Id (start time)
    write_element(&mut writer, "Id", &ride.started_at.to_rfc3339())?;

    // Write Lap(s) - for now, single lap for entire ride
    write_lap(&mut writer, ride, samples)?;

    // Close Activity
    writer
        .write_event(Event::End(BytesEnd::new("Activity")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Close Activities
    writer
        .write_event(Event::End(BytesEnd::new("Activities")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Close TrainingCenterDatabase
    writer
        .write_event(Event::End(BytesEnd::new("TrainingCenterDatabase")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).map_err(|e| ExportError::XmlError(e.to_string()))
}

/// Write a single lap element.
fn write_lap<W: std::io::Write>(
    writer: &mut Writer<W>,
    ride: &Ride,
    samples: &[RideSample],
) -> Result<(), ExportError> {
    let mut lap = BytesStart::new("Lap");
    lap.push_attribute(("StartTime", ride.started_at.to_rfc3339().as_str()));
    writer
        .write_event(Event::Start(lap))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // TotalTimeSeconds
    write_element(
        writer,
        "TotalTimeSeconds",
        &ride.duration_seconds.to_string(),
    )?;

    // DistanceMeters
    write_element(
        writer,
        "DistanceMeters",
        &format!("{:.1}", ride.distance_meters),
    )?;

    // MaximumSpeed (convert from km/h to m/s if available)
    // TODO: Calculate from samples

    // Calories
    write_element(writer, "Calories", &ride.calories.to_string())?;

    // AverageHeartRateBpm
    if let Some(avg_hr) = ride.avg_hr {
        write_heart_rate_element(writer, "AverageHeartRateBpm", avg_hr)?;
    }

    // MaximumHeartRateBpm
    if let Some(max_hr) = ride.max_hr {
        write_heart_rate_element(writer, "MaximumHeartRateBpm", max_hr)?;
    }

    // Intensity (Active/Resting)
    write_element(writer, "Intensity", "Active")?;

    // Cadence (average)
    if let Some(avg_cadence) = ride.avg_cadence {
        write_element(writer, "Cadence", &avg_cadence.to_string())?;
    }

    // TriggerMethod
    write_element(writer, "TriggerMethod", "Manual")?;

    // Track
    write_track(writer, ride.started_at, samples)?;

    // Extensions (power data)
    write_lap_extensions(writer, ride)?;

    // Close Lap
    writer
        .write_event(Event::End(BytesEnd::new("Lap")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write the track with trackpoints.
fn write_track<W: std::io::Write>(
    writer: &mut Writer<W>,
    start_time: DateTime<Utc>,
    samples: &[RideSample],
) -> Result<(), ExportError> {
    writer
        .write_event(Event::Start(BytesStart::new("Track")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    for sample in samples {
        write_trackpoint(writer, start_time, sample)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Track")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write a single trackpoint.
fn write_trackpoint<W: std::io::Write>(
    writer: &mut Writer<W>,
    start_time: DateTime<Utc>,
    sample: &RideSample,
) -> Result<(), ExportError> {
    writer
        .write_event(Event::Start(BytesStart::new("Trackpoint")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Time
    let sample_time = start_time + Duration::seconds(sample.elapsed_seconds as i64);
    write_element(writer, "Time", &sample_time.to_rfc3339())?;

    // DistanceMeters
    write_element(
        writer,
        "DistanceMeters",
        &format!("{:.1}", sample.distance_meters),
    )?;

    // HeartRateBpm
    if let Some(hr) = sample.heart_rate_bpm {
        write_heart_rate_element(writer, "HeartRateBpm", hr)?;
    }

    // Cadence
    if let Some(cadence) = sample.cadence_rpm {
        write_element(writer, "Cadence", &cadence.to_string())?;
    }

    // Extensions (power, speed)
    write_trackpoint_extensions(writer, sample)?;

    writer
        .write_event(Event::End(BytesEnd::new("Trackpoint")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write trackpoint extensions (power data).
fn write_trackpoint_extensions<W: std::io::Write>(
    writer: &mut Writer<W>,
    sample: &RideSample,
) -> Result<(), ExportError> {
    // Only write extensions if we have power or speed data
    if sample.power_watts.is_none() && sample.speed_kmh.is_none() {
        return Ok(());
    }

    writer
        .write_event(Event::Start(BytesStart::new("Extensions")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // TPX element for power/speed
    writer
        .write_event(Event::Start(BytesStart::new("ns3:TPX")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    // Speed in m/s (convert from km/h)
    if let Some(speed_kmh) = sample.speed_kmh {
        let speed_ms = speed_kmh / 3.6;
        write_element(writer, "ns3:Speed", &format!("{:.2}", speed_ms))?;
    }

    // Watts
    if let Some(power) = sample.power_watts {
        write_element(writer, "ns3:Watts", &power.to_string())?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("ns3:TPX")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    writer
        .write_event(Event::End(BytesEnd::new("Extensions")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write lap extensions (average/max power).
fn write_lap_extensions<W: std::io::Write>(
    writer: &mut Writer<W>,
    ride: &Ride,
) -> Result<(), ExportError> {
    // Only write if we have power data
    if ride.avg_power.is_none() && ride.max_power.is_none() {
        return Ok(());
    }

    writer
        .write_event(Event::Start(BytesStart::new("Extensions")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    writer
        .write_event(Event::Start(BytesStart::new("ns3:LX")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    if let Some(avg_power) = ride.avg_power {
        write_element(writer, "ns3:AvgWatts", &avg_power.to_string())?;
    }

    if let Some(max_power) = ride.max_power {
        write_element(writer, "ns3:MaxWatts", &max_power.to_string())?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("ns3:LX")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    writer
        .write_event(Event::End(BytesEnd::new("Extensions")))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write a simple element with text content.
fn write_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    value: &str,
) -> Result<(), ExportError> {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Write a heart rate element with Value sub-element.
fn write_heart_rate_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    value: u8,
) -> Result<(), ExportError> {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    write_element(writer, "Value", &value.to_string())?;

    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| ExportError::XmlError(e.to_string()))?;

    Ok(())
}

/// Export a ride to TCX and write to a file.
pub fn export_tcx_to_file(
    ride: &Ride,
    samples: &[RideSample],
    path: &std::path::Path,
) -> Result<(), ExportError> {
    let content = export_tcx(ride, samples)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a ride export.
pub fn generate_tcx_filename(ride: &Ride) -> String {
    let timestamp = ride.started_at.format("%Y%m%d_%H%M%S");
    format!("RustRide_{}.tcx", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                target_power: None,
                trainer_grade: None,
                left_right_balance: None,
                left_torque_effectiveness: None,
                right_torque_effectiveness: None,
                left_pedal_smoothness: None,
                right_pedal_smoothness: None,
                left_power_phase_start: None,
                left_power_phase_end: None,
                left_power_phase_peak: None,
                right_power_phase_start: None,
                right_power_phase_end: None,
                right_power_phase_peak: None,
            })
            .collect()
    }

    #[test]
    fn test_export_tcx_generates_xml() {
        let ride = create_test_ride();
        let samples = create_test_samples(60);

        let result = export_tcx(&ride, &samples);
        assert!(result.is_ok());

        let xml = result.unwrap();
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<TrainingCenterDatabase"));
        assert!(xml.contains("</TrainingCenterDatabase>"));
    }

    #[test]
    fn test_export_tcx_contains_activity() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<Activity Sport=\"Biking\">"));
        assert!(xml.contains("</Activity>"));
    }

    #[test]
    fn test_export_tcx_contains_lap() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<Lap"));
        assert!(xml.contains("<TotalTimeSeconds>3600</TotalTimeSeconds>"));
        assert!(xml.contains("<DistanceMeters>30000.0</DistanceMeters>"));
        assert!(xml.contains("<Calories>720</Calories>"));
    }

    #[test]
    fn test_export_tcx_contains_trackpoints() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<Track>"));
        assert!(xml.contains("<Trackpoint>"));
        assert!(xml.contains("</Trackpoint>"));
    }

    #[test]
    fn test_export_tcx_contains_power_extension() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<ns3:TPX>"));
        assert!(xml.contains("<ns3:Watts>"));
    }

    #[test]
    fn test_export_tcx_contains_heart_rate() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<HeartRateBpm>"));
        assert!(xml.contains("<Value>145</Value>"));
    }

    #[test]
    fn test_export_tcx_empty_samples_error() {
        let ride = create_test_ride();
        let samples: Vec<RideSample> = vec![];

        let result = export_tcx(&ride, &samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_filename() {
        let ride = create_test_ride();
        let filename = generate_tcx_filename(&ride);

        assert!(filename.starts_with("RustRide_"));
        assert!(filename.ends_with(".tcx"));
    }

    #[test]
    fn test_export_tcx_lap_extensions() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let xml = export_tcx(&ride, &samples).unwrap();

        assert!(xml.contains("<ns3:LX>"));
        assert!(xml.contains("<ns3:AvgWatts>180</ns3:AvgWatts>"));
        assert!(xml.contains("<ns3:MaxWatts>350</ns3:MaxWatts>"));
    }
}
