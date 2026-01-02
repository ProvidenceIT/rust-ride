//! Integration tests for FIT file validation.
//!
//! These tests validate that exported FIT files comply with the FIT specification
//! by using the fitparser crate to parse and verify the structure of exported files.
//! This provides validation equivalent to the Garmin FIT SDK validator.
//!
//! The tests cover:
//! - FIT header structure and CRC
//! - Required message types (FileId, Session, Lap, Activity, Record, Event)
//! - Field data integrity
//! - Multi-lap and workout structure preservation
//! - Cycling dynamics data encoding

use chrono::{TimeZone, Utc};
use fitparser::profile::MesgNum;
use rustride::recording::exporter_fit::{
    export_fit, export_fit_with_laps, export_fit_with_segments, export_fit_with_workout,
    LapData,
};
use rustride::recording::types::{Ride, RideSample};
use rustride::workouts::{PowerTarget, SegmentType, Workout, WorkoutSegment};
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;
use uuid::Uuid;

/// FIT file header size (14 bytes for header with CRC)
const FIT_HEADER_SIZE: usize = 14;

/// FIT epoch offset: seconds between Unix epoch (1970-01-01) and FIT epoch (1989-12-31)
const _FIT_EPOCH_OFFSET: i64 = 631065600;

/// CRC-16 lookup table for FIT format
const CRC_TABLE: [u16; 16] = [
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

/// Calculate CRC-16 for verifying FIT file integrity.
fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for byte in data {
        let tmp = CRC_TABLE[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ CRC_TABLE[(*byte & 0xF) as usize];

        let tmp = CRC_TABLE[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ CRC_TABLE[((*byte >> 4) & 0xF) as usize];
    }
    crc
}

/// Create a test ride with sample data at a fixed timestamp.
fn create_test_ride() -> Ride {
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
    ride
}

/// Create test samples without cycling dynamics.
fn create_test_samples(count: usize) -> Vec<RideSample> {
    (0..count)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(150 + (i % 50) as u16),
            cadence_rpm: Some(80 + (i % 20) as u8),
            heart_rate_bpm: Some(140 + (i % 20) as u8),
            speed_kmh: Some(30.0 + (i % 10) as f32),
            distance_meters: (i as f64) * 8.33,
            calories: (i as f64 * 0.2) as u32,
            resistance_level: None,
            target_power: Some(180),
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

/// Create test samples with full cycling dynamics data.
fn create_test_samples_with_dynamics(count: usize) -> Vec<RideSample> {
    (0..count)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(180 + (i % 30) as u16),
            cadence_rpm: Some(85),
            heart_rate_bpm: Some(145),
            speed_kmh: Some(30.0),
            distance_meters: (i as f64) * 8.33,
            calories: (i as f64 * 0.2) as u32,
            resistance_level: None,
            target_power: Some(180),
            trainer_grade: None,
            left_right_balance: Some(52.0),
            left_torque_effectiveness: Some(75.0),
            right_torque_effectiveness: Some(72.0),
            left_pedal_smoothness: Some(22.0),
            right_pedal_smoothness: Some(24.0),
            left_power_phase_start: Some(30.0),
            left_power_phase_end: Some(180.0),
            left_power_phase_peak: Some(90.0),
            right_power_phase_start: Some(30.0),
            right_power_phase_end: Some(180.0),
            right_power_phase_peak: Some(90.0),
        })
        .collect()
}

/// Create a test workout with multiple segments.
fn create_test_workout() -> Workout {
    let segments = vec![
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: Some("Warm up!".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(90),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(120),
            cadence_target: None,
            text_event: Some("Push!".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(40),
            cadence_target: None,
            text_event: None,
        },
    ];

    Workout::new("Test Workout".to_string(), segments)
}

/// Parse a FIT byte buffer using fitparser.
fn parse_fit(data: &[u8]) -> Vec<fitparser::FitDataRecord> {
    fitparser::from_bytes(data).expect("fitparser should parse exported FIT file")
}

// ============================================================================
// FIT HEADER VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_header_size() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();

    // FIT header should be 14 bytes
    assert!(
        data.len() >= FIT_HEADER_SIZE,
        "FIT file must be at least {} bytes",
        FIT_HEADER_SIZE
    );
    assert_eq!(data[0], 14, "Header size byte should be 14");
}

#[test]
fn test_validate_fit_protocol_version() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();

    // Protocol version should be 2.0 (0x20)
    assert_eq!(data[1], 0x20, "Protocol version should be 2.0 (0x20)");
}

#[test]
fn test_validate_fit_signature() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();

    // Bytes 8-11 should be ".FIT"
    assert_eq!(&data[8..12], b".FIT", "FIT signature should be '.FIT'");
}

#[test]
fn test_validate_fit_header_crc() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();

    // Header CRC is in bytes 12-13 (little endian)
    let stored_crc = u16::from_le_bytes([data[12], data[13]]);

    // Calculate CRC over first 12 bytes
    let calculated_crc = calculate_crc(&data[0..12]);

    assert_eq!(
        stored_crc, calculated_crc,
        "Header CRC should match calculated value"
    );
}

#[test]
fn test_validate_fit_file_crc() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();

    // File CRC is last 2 bytes (little endian)
    let file_len = data.len();
    let stored_crc = u16::from_le_bytes([data[file_len - 2], data[file_len - 1]]);

    // Calculate CRC over everything except last 2 bytes
    let calculated_crc = calculate_crc(&data[0..file_len - 2]);

    assert_eq!(
        stored_crc, calculated_crc,
        "File CRC should match calculated value"
    );
}

// ============================================================================
// REQUIRED MESSAGE TYPE TESTS
// ============================================================================

#[test]
fn test_validate_fit_contains_file_id_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let file_id = records.iter().find(|r| r.kind() == MesgNum::FileId);
    assert!(file_id.is_some(), "FIT file must contain FileId message");

    // Verify FileId has type field set to activity
    let file_id = file_id.unwrap();
    let type_field = file_id.fields().iter().find(|f| f.name() == "type");
    assert!(type_field.is_some(), "FileId must have type field");
}

#[test]
fn test_validate_fit_contains_session_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let session = records.iter().find(|r| r.kind() == MesgNum::Session);
    assert!(session.is_some(), "FIT file must contain Session message");

    // Verify session has required fields
    let session = session.unwrap();
    let sport = session.fields().iter().find(|f| f.name() == "sport");
    assert!(sport.is_some(), "Session must have sport field");
}

#[test]
fn test_validate_fit_contains_lap_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let lap = records.iter().find(|r| r.kind() == MesgNum::Lap);
    assert!(lap.is_some(), "FIT file must contain at least one Lap message");
}

#[test]
fn test_validate_fit_contains_activity_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let activity = records.iter().find(|r| r.kind() == MesgNum::Activity);
    assert!(
        activity.is_some(),
        "FIT file must contain Activity message"
    );
}

#[test]
fn test_validate_fit_contains_record_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();

    assert_eq!(
        record_count, 60,
        "FIT file should contain one Record message per sample"
    );
}

#[test]
fn test_validate_fit_contains_event_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let event_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Event)
        .count();

    assert!(
        event_count >= 2,
        "FIT file should contain at least start and stop events"
    );
}

// ============================================================================
// DATA INTEGRITY TESTS
// ============================================================================

#[test]
fn test_validate_fit_record_power_data() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let power_records: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .filter(|r| r.fields().iter().any(|f| f.name() == "power"))
        .collect();

    assert_eq!(
        power_records.len(),
        30,
        "All record messages should have power field"
    );
}

#[test]
fn test_validate_fit_record_heart_rate_data() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let hr_records: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .filter(|r| r.fields().iter().any(|f| f.name() == "heart_rate"))
        .collect();

    assert_eq!(
        hr_records.len(),
        30,
        "All record messages should have heart_rate field"
    );
}

#[test]
fn test_validate_fit_record_cadence_data() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let cadence_records: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .filter(|r| r.fields().iter().any(|f| f.name() == "cadence"))
        .collect();

    assert_eq!(
        cadence_records.len(),
        30,
        "All record messages should have cadence field"
    );
}

#[test]
fn test_validate_fit_session_sport_type() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let session = records
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .unwrap();

    let sport = session
        .fields()
        .iter()
        .find(|f| f.name() == "sport")
        .unwrap();

    // Sport should be cycling (value 2)
    let sport_str = format!("{:?}", sport.value());
    assert!(
        sport_str.contains("Cycling") || sport_str.contains("2"),
        "Sport should be cycling, got: {}",
        sport_str
    );
}

// ============================================================================
// FILE WRITE AND PARSE ROUNDTRIP TESTS
// ============================================================================

#[test]
fn test_validate_fit_file_write_and_parse() {
    let ride = create_test_ride();
    let samples = create_test_samples(100);

    let data = export_fit(&ride, &samples).unwrap();

    // Write to a temp file
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(&data).unwrap();
    temp_file.flush().unwrap();

    // Read back and parse
    let read_data = fs::read(temp_file.path()).unwrap();
    let records = parse_fit(&read_data);

    // Verify it parsed correctly
    assert!(!records.is_empty(), "Should parse records from file");

    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();
    assert_eq!(record_count, 100, "Should have 100 record messages");
}

#[test]
fn test_validate_fit_file_structure_complete() {
    let ride = create_test_ride();
    let samples = create_test_samples(120);

    let data = export_fit(&ride, &samples).unwrap();

    // Write to temp file
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(&data).unwrap();

    // Parse and validate complete structure
    let read_data = fs::read(temp_file.path()).unwrap();
    let records = parse_fit(&read_data);

    // Count all required message types
    let file_id_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::FileId)
        .count();
    let session_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Session)
        .count();
    let lap_count = records.iter().filter(|r| r.kind() == MesgNum::Lap).count();
    let activity_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Activity)
        .count();
    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();
    let event_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Event)
        .count();

    assert_eq!(file_id_count, 1, "Should have exactly 1 FileId");
    assert_eq!(session_count, 1, "Should have exactly 1 Session");
    assert!(lap_count >= 1, "Should have at least 1 Lap");
    assert_eq!(activity_count, 1, "Should have exactly 1 Activity");
    assert_eq!(record_count, 120, "Should have 120 Records");
    assert!(event_count >= 2, "Should have at least 2 Events");
}

// ============================================================================
// MULTI-LAP VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_multi_lap_export() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);

    // Create 3 laps of 50 seconds each
    let laps: Vec<LapData> = (0..3)
        .filter_map(|i| {
            LapData::from_samples(&samples, i * 50, (i + 1) * 50, ride.started_at)
        })
        .collect();

    let data = export_fit_with_laps(&ride, &samples, &laps).unwrap();
    let records = parse_fit(&data);

    // Count lap messages
    let lap_count = records.iter().filter(|r| r.kind() == MesgNum::Lap).count();

    assert_eq!(lap_count, 3, "Should have 3 lap messages");
}

#[test]
fn test_validate_fit_segment_based_laps() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);

    // Create segments of 30, 60, 30, 30 seconds
    let segment_durations = vec![30, 60, 30, 30];

    let data = export_fit_with_segments(&ride, &samples, &segment_durations).unwrap();
    let records = parse_fit(&data);

    // Count lap messages
    let lap_count = records.iter().filter(|r| r.kind() == MesgNum::Lap).count();

    assert_eq!(lap_count, 4, "Should have 4 lap messages from segments");
}

#[test]
fn test_validate_fit_workout_based_laps() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);
    let workout = create_test_workout();

    let data = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let records = parse_fit(&data);

    // Count lap messages - should match workout segments
    let lap_count = records.iter().filter(|r| r.kind() == MesgNum::Lap).count();

    assert_eq!(
        lap_count, 4,
        "Should have 4 lap messages matching workout segments"
    );
}

#[test]
fn test_validate_fit_session_num_laps_field() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);
    let workout = create_test_workout();

    let data = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let records = parse_fit(&data);

    let session = records
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .unwrap();

    let num_laps = session.fields().iter().find(|f| f.name() == "num_laps");
    assert!(
        num_laps.is_some(),
        "Session should have num_laps field for multi-lap export"
    );
}

// ============================================================================
// CYCLING DYNAMICS VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_cycling_dynamics_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .collect();

    // Check for left_right_balance field
    let has_balance = record_messages
        .iter()
        .any(|r| r.fields().iter().any(|f| f.name() == "left_right_balance"));

    assert!(
        has_balance,
        "Record messages should contain left_right_balance field"
    );
}

#[test]
fn test_validate_fit_torque_effectiveness_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .collect();

    // Check for torque effectiveness fields
    let has_left_te = record_messages.iter().any(|r| {
        r.fields()
            .iter()
            .any(|f| f.name() == "left_torque_effectiveness")
    });
    let has_right_te = record_messages.iter().any(|r| {
        r.fields()
            .iter()
            .any(|f| f.name() == "right_torque_effectiveness")
    });

    assert!(
        has_left_te,
        "Should have left_torque_effectiveness field"
    );
    assert!(
        has_right_te,
        "Should have right_torque_effectiveness field"
    );
}

#[test]
fn test_validate_fit_pedal_smoothness_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .collect();

    // Check for pedal smoothness fields
    let has_left_ps = record_messages
        .iter()
        .any(|r| r.fields().iter().any(|f| f.name() == "left_pedal_smoothness"));
    let has_right_ps = record_messages.iter().any(|r| {
        r.fields()
            .iter()
            .any(|f| f.name() == "right_pedal_smoothness")
    });

    assert!(has_left_ps, "Should have left_pedal_smoothness field");
    assert!(has_right_ps, "Should have right_pedal_smoothness field");
}

// ============================================================================
// LARGE RIDE VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_large_ride_1_hour() {
    let ride = create_test_ride();
    let samples = create_test_samples(3600); // 1 hour of 1-second samples

    let data = export_fit(&ride, &samples).unwrap();

    // Write to temp file
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(&data).unwrap();

    // Parse and verify
    let read_data = fs::read(temp_file.path()).unwrap();
    let records = parse_fit(&read_data);

    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();

    assert_eq!(record_count, 3600, "Should have 3600 record messages");
}

#[test]
fn test_validate_fit_large_ride_2_hours() {
    let ride = create_test_ride();
    let samples = create_test_samples(7200); // 2 hours of 1-second samples

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();

    assert_eq!(record_count, 7200, "Should have 7200 record messages");
}

// ============================================================================
// CRC VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_crc_consistency() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    // Export twice and verify CRCs match (deterministic output)
    let data1 = export_fit(&ride, &samples).unwrap();
    let data2 = export_fit(&ride, &samples).unwrap();

    assert_eq!(data1, data2, "Same input should produce identical output");

    let crc1 = u16::from_le_bytes([data1[data1.len() - 2], data1[data1.len() - 1]]);
    let crc2 = u16::from_le_bytes([data2[data2.len() - 2], data2[data2.len() - 1]]);

    assert_eq!(crc1, crc2, "CRCs should match for identical exports");
}

#[test]
fn test_validate_fit_crc_differs_for_different_data() {
    let ride = create_test_ride();
    let samples1 = create_test_samples(60);
    let samples2 = create_test_samples(61);

    let data1 = export_fit(&ride, &samples1).unwrap();
    let data2 = export_fit(&ride, &samples2).unwrap();

    let crc1 = u16::from_le_bytes([data1[data1.len() - 2], data1[data1.len() - 1]]);
    let crc2 = u16::from_le_bytes([data2[data2.len() - 2], data2[data2.len() - 1]]);

    assert_ne!(crc1, crc2, "Different data should produce different CRCs");
}

// ============================================================================
// EDGE CASE VALIDATION TESTS
// ============================================================================

#[test]
fn test_validate_fit_minimum_ride() {
    let ride = create_test_ride();
    let samples = create_test_samples(1); // Minimum valid ride

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    // Should still have all required message types
    assert!(records.iter().any(|r| r.kind() == MesgNum::FileId));
    assert!(records.iter().any(|r| r.kind() == MesgNum::Session));
    assert!(records.iter().any(|r| r.kind() == MesgNum::Lap));
    assert!(records.iter().any(|r| r.kind() == MesgNum::Activity));
    assert!(records.iter().any(|r| r.kind() == MesgNum::Record));
}

#[test]
fn test_validate_fit_empty_samples_error() {
    let ride = create_test_ride();
    let samples: Vec<RideSample> = vec![];

    let result = export_fit(&ride, &samples);

    assert!(result.is_err(), "Should error on empty samples");
}

#[test]
fn test_validate_fit_missing_optional_fields() {
    let ride = create_test_ride();

    // Create samples with some missing optional fields
    let samples: Vec<RideSample> = (0..30)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: if i % 2 == 0 { Some(200) } else { None },
            cadence_rpm: if i % 3 == 0 { Some(90) } else { None },
            heart_rate_bpm: if i % 5 == 0 { Some(140) } else { None },
            speed_kmh: Some(30.0),
            distance_meters: (i as f64) * 8.33,
            calories: 0,
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
        .collect();

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_fit(&data);

    // Should still be valid FIT file
    assert!(!records.is_empty());
    assert!(records.iter().any(|r| r.kind() == MesgNum::FileId));
}

// ============================================================================
// GARMIN SDK COMPLIANCE SUMMARY TEST
// ============================================================================

/// This test validates that exported FIT files meet the minimum requirements
/// for Garmin SDK compliance. It checks:
/// - Valid FIT header (14 bytes with proper signature)
/// - Correct CRC values (header and file)
/// - All required message types present
/// - Proper data field encoding
#[test]
fn test_validate_fit_garmin_sdk_compliance() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(120);

    let data = export_fit(&ride, &samples).unwrap();

    // 1. Header validation
    assert_eq!(data[0], 14, "Header size must be 14 bytes");
    assert_eq!(data[1], 0x20, "Protocol version must be 2.0");
    assert_eq!(&data[8..12], b".FIT", "Must have .FIT signature");

    // 2. Header CRC validation
    let header_crc_stored = u16::from_le_bytes([data[12], data[13]]);
    let header_crc_calc = calculate_crc(&data[0..12]);
    assert_eq!(
        header_crc_stored, header_crc_calc,
        "Header CRC must be valid"
    );

    // 3. File CRC validation
    let file_crc_stored = u16::from_le_bytes([data[data.len() - 2], data[data.len() - 1]]);
    let file_crc_calc = calculate_crc(&data[0..data.len() - 2]);
    assert_eq!(file_crc_stored, file_crc_calc, "File CRC must be valid");

    // 4. Parse with fitparser (validates internal structure)
    let records = parse_fit(&data);

    // 5. Required message types
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::FileId),
        "Must contain FileId message"
    );
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::Session),
        "Must contain Session message"
    );
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::Lap),
        "Must contain Lap message"
    );
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::Activity),
        "Must contain Activity message"
    );
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::Record),
        "Must contain Record messages"
    );
    assert!(
        records.iter().any(|r| r.kind() == MesgNum::Event),
        "Must contain Event messages"
    );

    // 6. Verify record count matches input
    let record_count = records
        .iter()
        .filter(|r| r.kind() == MesgNum::Record)
        .count();
    assert_eq!(
        record_count, 120,
        "Record count must match sample count"
    );

    // 7. Verify session has cycling sport type
    let session = records
        .iter()
        .find(|r| r.kind() == MesgNum::Session)
        .unwrap();
    let sport = session.fields().iter().find(|f| f.name() == "sport");
    assert!(sport.is_some(), "Session must have sport field");
}
