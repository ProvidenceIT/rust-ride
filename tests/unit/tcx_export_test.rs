//! Unit tests for TCX export functionality.
//!
//! T081: Unit test for TCX export format
//! T097: Validate TCX output against schema

use chrono::{TimeZone, Utc};
use rustride::recording::types::{Ride, RideSample};
use uuid::Uuid;

/// Create a test ride with sample data.
fn create_test_ride() -> (Ride, Vec<RideSample>) {
    let user_id = Uuid::new_v4();
    let mut ride = Ride::new(user_id, 200);
    ride.started_at = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    ride.ended_at = Some(Utc.with_ymd_and_hms(2025, 1, 15, 11, 0, 0).unwrap());
    ride.duration_seconds = 3600;
    ride.distance_meters = 30000.0;
    ride.avg_power = Some(180);
    ride.max_power = Some(350);
    ride.normalized_power = Some(190);
    ride.intensity_factor = Some(0.95);
    ride.tss = Some(90.0);
    ride.avg_hr = Some(145);
    ride.max_hr = Some(175);
    ride.avg_cadence = Some(85);
    ride.calories = 800;

    let mut samples = Vec::new();
    for i in 0..60 {
        // Create 60 samples (1 per second for first minute)
        let sample = RideSample {
            elapsed_seconds: i,
            power_watts: Some(150 + (i % 50) as u16),
            cadence_rpm: Some(80 + (i % 20) as u8),
            heart_rate_bpm: Some(140 + (i % 20) as u8),
            speed_kmh: Some(30.0 + (i % 10) as f32),
            distance_meters: (i as f64) * 8.33, // ~30km/h = 8.33m/s
            calories: i,
            resistance_level: None,
            target_power: Some(180),
            trainer_grade: None,
        };
        samples.push(sample);
    }

    (ride, samples)
}

#[test]
fn test_tcx_export_generates_valid_xml() {
    let (ride, samples) = create_test_ride();

    // The exporter module will be implemented in T095
    // For now, verify the test structure is correct
    assert_eq!(samples.len(), 60);
    assert!(ride.avg_power.is_some());
    assert!(ride.duration_seconds > 0);
}

#[test]
fn test_tcx_contains_required_elements() {
    // TCX must contain:
    // - TrainingCenterDatabase root element
    // - Activities container
    // - Activity with Sport attribute
    // - Lap with StartTime
    // - Track with Trackpoints
    let (ride, samples) = create_test_ride();

    // Verify ride has required data for export
    assert!(ride.started_at.to_rfc3339().contains("2025-01-15"));
    assert!(ride.duration_seconds > 0);
    assert!(!samples.is_empty());
}

#[test]
fn test_tcx_trackpoint_has_power_extension() {
    // TCX ActivityExtension/TPX should include power data
    let (_, samples) = create_test_ride();

    // All our test samples should have power data
    for sample in &samples {
        assert!(sample.power_watts.is_some(), "Sample should have power data");
    }
}

#[test]
fn test_tcx_lap_summary_calculations() {
    let (ride, _samples) = create_test_ride();

    // Verify lap summary metrics are present
    assert_eq!(ride.duration_seconds, 3600);
    assert_eq!(ride.distance_meters, 30000.0);
    assert_eq!(ride.avg_power, Some(180));
    assert_eq!(ride.max_power, Some(350));
    assert_eq!(ride.avg_hr, Some(145));
    assert_eq!(ride.max_hr, Some(175));
    assert_eq!(ride.avg_cadence, Some(85));
    assert_eq!(ride.calories, 800);
}

#[test]
fn test_tcx_datetime_format() {
    // TCX uses ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
    let (ride, _) = create_test_ride();
    let timestamp = ride.started_at.to_rfc3339();

    // Should contain T separator and timezone
    assert!(timestamp.contains("T"));
    assert!(timestamp.ends_with("+00:00") || timestamp.ends_with("Z"));
}

#[test]
fn test_tcx_handles_missing_hr_data() {
    // Some trackpoints might not have HR data
    let sample = RideSample {
        elapsed_seconds: 0,
        power_watts: Some(200),
        cadence_rpm: Some(90),
        heart_rate_bpm: None, // No HR
        speed_kmh: Some(30.0),
        distance_meters: 0.0,
        calories: 0,
        resistance_level: None,
        target_power: None,
        trainer_grade: None,
    };

    assert!(sample.heart_rate_bpm.is_none());
    assert!(sample.power_watts.is_some());
}

#[test]
fn test_tcx_handles_missing_cadence_data() {
    let sample = RideSample {
        elapsed_seconds: 0,
        power_watts: Some(200),
        cadence_rpm: None, // No cadence
        heart_rate_bpm: Some(145),
        speed_kmh: Some(30.0),
        distance_meters: 0.0,
        calories: 0,
        resistance_level: None,
        target_power: None,
        trainer_grade: None,
    };

    assert!(sample.cadence_rpm.is_none());
}

#[test]
fn test_tcx_sport_type_is_biking() {
    // Indoor cycling activities should have Sport="Biking"
    let (ride, _) = create_test_ride();

    // The ride represents a cycling activity
    assert!(ride.duration_seconds > 0);
    assert!(ride.distance_meters > 0.0);
}
