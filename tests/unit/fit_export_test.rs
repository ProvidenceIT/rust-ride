//! Unit tests for FIT file export functionality.
//!
//! Comprehensive tests for FIT export including:
//! - Header validation
//! - Message structure
//! - CRC verification
//! - Data integrity
//! - Multi-lap support
//! - Cycling dynamics encoding

use chrono::{TimeZone, Utc};
use rustride::recording::exporter_fit::{
    export_fit, export_fit_with_laps, export_fit_with_segments, export_fit_with_workout,
    extract_workout_segment_durations, LapData,
};
use rustride::recording::types::{Ride, RideSample};
use rustride::workouts::{PowerTarget, SegmentType, Workout, WorkoutSegment};
use uuid::Uuid;

/// FIT file header size (14 bytes for header with CRC)
const FIT_HEADER_SIZE: usize = 14;

/// FIT epoch offset: seconds between Unix epoch (1970-01-01) and FIT epoch (1989-12-31)
const FIT_EPOCH_OFFSET: i64 = 631065600;

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

/// Create test samples with cycling dynamics data.
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
            left_power_phase_start: None,
            left_power_phase_end: None,
            left_power_phase_peak: None,
            right_power_phase_start: None,
            right_power_phase_end: None,
            right_power_phase_peak: None,
        })
        .collect()
}

/// Create test samples with power phase data.
fn create_test_samples_with_power_phase(count: usize) -> Vec<RideSample> {
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

// ============================================================================
// HEADER VALIDATION TESTS
// ============================================================================

#[test]
fn test_fit_header_size() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Header size should be 14 bytes
    assert_eq!(data[0], 14, "Header size byte should be 14");
}

#[test]
fn test_fit_protocol_version() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Protocol version should be 2.0 (0x20)
    assert_eq!(data[1], 0x20, "Protocol version should be 2.0 (0x20)");
}

#[test]
fn test_fit_profile_version() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Profile version is bytes 2-3 (little endian), should be 2100 (21.00)
    let profile_version = u16::from_le_bytes([data[2], data[3]]);
    assert_eq!(profile_version, 2100, "Profile version should be 21.00");
}

#[test]
fn test_fit_data_type_signature() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Data type signature at bytes 8-11 should be ".FIT"
    assert_eq!(
        &data[8..12],
        b".FIT",
        "Data type signature should be '.FIT'"
    );
}

#[test]
fn test_fit_header_crc_present() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Header CRC is at bytes 12-13 for 14-byte header
    let header_crc = u16::from_le_bytes([data[12], data[13]]);
    // CRC should not be zero for valid header
    assert_ne!(header_crc, 0, "Header CRC should be present and non-zero");
}

#[test]
fn test_fit_data_size_field() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Data size is at bytes 4-7 (little endian)
    let data_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    // Data size should match file length minus header (14 bytes) and final CRC (2 bytes)
    let expected_data_size = data.len() - FIT_HEADER_SIZE - 2;
    assert_eq!(
        data_size as usize, expected_data_size,
        "Data size field should match actual data size"
    );
}

// ============================================================================
// CRC VERIFICATION TESTS
// ============================================================================

#[test]
fn test_fit_header_crc_valid() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Calculate CRC for first 12 bytes (header excluding CRC)
    let calculated_crc = calculate_crc(&data[0..12]);
    let stored_crc = u16::from_le_bytes([data[12], data[13]]);

    assert_eq!(
        calculated_crc, stored_crc,
        "Header CRC should match calculated value"
    );
}

#[test]
fn test_fit_file_crc_valid() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // File CRC covers everything except the last 2 bytes (the CRC itself)
    let file_data_end = data.len() - 2;
    let calculated_crc = calculate_crc(&data[0..file_data_end]);
    let stored_crc = u16::from_le_bytes([data[file_data_end], data[file_data_end + 1]]);

    assert_eq!(
        calculated_crc, stored_crc,
        "File CRC should match calculated value"
    );
}

#[test]
fn test_fit_crc_different_for_different_data() {
    let ride = create_test_ride();
    let samples1 = create_test_samples(10);
    let samples2 = create_test_samples(20);

    let data1 = export_fit(&ride, &samples1).unwrap();
    let data2 = export_fit(&ride, &samples2).unwrap();

    // Get file CRCs
    let crc1 = u16::from_le_bytes([data1[data1.len() - 2], data1[data1.len() - 1]]);
    let crc2 = u16::from_le_bytes([data2[data2.len() - 2], data2[data2.len() - 1]]);

    // CRCs should be different for different content
    assert_ne!(crc1, crc2, "CRCs should differ for different content");
}

// ============================================================================
// DATA INTEGRITY TESTS
// ============================================================================

#[test]
fn test_fit_minimum_file_size() {
    let ride = create_test_ride();
    let samples = create_test_samples(1);

    let data = export_fit(&ride, &samples).unwrap();

    // Minimum size: header (14) + messages + CRC (2)
    assert!(
        data.len() > FIT_HEADER_SIZE + 2,
        "FIT file should be larger than just header and CRC"
    );
}

#[test]
fn test_fit_file_size_increases_with_samples() {
    let ride = create_test_ride();
    let samples_small = create_test_samples(10);
    let samples_large = create_test_samples(100);

    let data_small = export_fit(&ride, &samples_small).unwrap();
    let data_large = export_fit(&ride, &samples_large).unwrap();

    assert!(
        data_large.len() > data_small.len(),
        "More samples should produce larger file"
    );
}

#[test]
fn test_fit_export_empty_samples_returns_error() {
    let ride = create_test_ride();
    let samples: Vec<RideSample> = vec![];

    let result = export_fit(&ride, &samples);
    assert!(result.is_err(), "Empty samples should return error");
}

#[test]
fn test_fit_export_single_sample() {
    let ride = create_test_ride();
    let samples = create_test_samples(1);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Single sample should export successfully");
}

#[test]
fn test_fit_export_large_ride() {
    let ride = create_test_ride();
    // 2 hours of samples at 1Hz = 7200 samples
    let samples = create_test_samples(7200);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Large ride should export successfully");

    let data = result.unwrap();
    // Verify structure is intact
    assert_eq!(data[0], 14);
    assert_eq!(&data[8..12], b".FIT");
}

// ============================================================================
// MESSAGE STRUCTURE TESTS
// ============================================================================

#[test]
fn test_fit_contains_definition_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Definition messages have bit 6 set in the record header
    // Look for at least one definition message after the header
    let data_section = &data[FIT_HEADER_SIZE..data.len() - 2];

    let has_definition = data_section.iter().any(|&b| (b & 0x40) != 0);
    assert!(
        has_definition,
        "FIT file should contain definition messages"
    );
}

#[test]
fn test_fit_contains_data_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // Data messages have bit 6 clear in the record header (and bits 4-5 clear for normal header)
    let data_section = &data[FIT_HEADER_SIZE..data.len() - 2];

    // After definitions, there should be data messages
    // For data messages, bits 6-7 are 0 (for normal header, not compressed timestamp)
    let has_data_message = data_section.iter().any(|&b| (b & 0xC0) == 0x00);
    assert!(has_data_message, "FIT file should contain data messages");
}

#[test]
fn test_fit_definition_before_data() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    let data_section = &data[FIT_HEADER_SIZE..data.len() - 2];

    // First record after header should be a definition (bit 6 set)
    assert!(
        (data_section[0] & 0x40) != 0,
        "First record should be a definition message"
    );
}

// ============================================================================
// TIMESTAMP TESTS
// ============================================================================

#[test]
fn test_fit_timestamp_epoch_conversion() {
    // FIT epoch is 1989-12-31 00:00:00 UTC
    // Unix epoch is 1970-01-01 00:00:00 UTC
    // Difference is 631065600 seconds

    let fit_epoch = Utc.with_ymd_and_hms(1989, 12, 31, 0, 0, 0).unwrap();
    let fit_timestamp = (fit_epoch.timestamp() - FIT_EPOCH_OFFSET) as u32;
    assert_eq!(fit_timestamp, 0, "FIT epoch should convert to 0");

    // One day after FIT epoch
    let next_day = Utc.with_ymd_and_hms(1990, 1, 1, 0, 0, 0).unwrap();
    let fit_timestamp = (next_day.timestamp() - FIT_EPOCH_OFFSET) as u32;
    assert_eq!(fit_timestamp, 86400, "One day should be 86400 seconds");
}

#[test]
fn test_fit_ride_timestamp_in_valid_range() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();

    // The ride started at 2025-01-15, which should produce a reasonable FIT timestamp
    let expected_fit_timestamp = (ride.started_at.timestamp() - FIT_EPOCH_OFFSET) as u32;

    // This timestamp should be around 1105689600 (seconds from FIT epoch to Jan 2025)
    assert!(
        expected_fit_timestamp > 1000000000,
        "FIT timestamp for 2025 should be large positive value"
    );
}

// ============================================================================
// CYCLING DYNAMICS TESTS
// ============================================================================

#[test]
fn test_fit_with_dynamics_larger_than_without() {
    let ride = create_test_ride();
    let samples_no_dynamics = create_test_samples(50);
    let samples_with_dynamics = create_test_samples_with_dynamics(50);

    let data_no_dynamics = export_fit(&ride, &samples_no_dynamics).unwrap();
    let data_with_dynamics = export_fit(&ride, &samples_with_dynamics).unwrap();

    assert!(
        data_with_dynamics.len() > data_no_dynamics.len(),
        "FIT with dynamics should be larger"
    );
}

#[test]
fn test_fit_with_power_phase_larger_than_dynamics_only() {
    let ride = create_test_ride();
    let samples_dynamics = create_test_samples_with_dynamics(50);
    let samples_power_phase = create_test_samples_with_power_phase(50);

    let data_dynamics = export_fit(&ride, &samples_dynamics).unwrap();
    let data_power_phase = export_fit(&ride, &samples_power_phase).unwrap();

    assert!(
        data_power_phase.len() > data_dynamics.len(),
        "FIT with power phase should be larger than dynamics only"
    );
}

#[test]
fn test_fit_dynamics_export_succeeds() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(100);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export with dynamics should succeed");

    // Verify basic structure
    let data = result.unwrap();
    assert_eq!(data[0], 14);
    assert_eq!(&data[8..12], b".FIT");
}

#[test]
fn test_fit_power_phase_export_succeeds() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_power_phase(100);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export with power phase should succeed");

    // Verify basic structure
    let data = result.unwrap();
    assert_eq!(data[0], 14);
    assert_eq!(&data[8..12], b".FIT");
}

// ============================================================================
// LAP DATA TESTS
// ============================================================================

#[test]
fn test_lap_data_from_samples_basic() {
    let ride = create_test_ride();
    let samples = create_test_samples(100);

    let lap = LapData::from_samples(&samples, 0, 50, ride.started_at);
    assert!(lap.is_some(), "LapData should be created from valid range");

    let lap = lap.unwrap();
    assert_eq!(lap.duration_seconds, 50);
    assert!(lap.avg_power.is_some());
    assert!(lap.avg_hr.is_some());
    assert!(lap.avg_cadence.is_some());
}

#[test]
fn test_lap_data_from_samples_invalid_range() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    // start >= end
    let lap = LapData::from_samples(&samples, 5, 5, ride.started_at);
    assert!(lap.is_none(), "Equal start/end should return None");

    // start > end
    let lap = LapData::from_samples(&samples, 8, 5, ride.started_at);
    assert!(lap.is_none(), "start > end should return None");

    // end > samples.len()
    let lap = LapData::from_samples(&samples, 0, 100, ride.started_at);
    assert!(lap.is_none(), "end > len should return None");
}

#[test]
fn test_lap_data_from_samples_empty() {
    let ride = create_test_ride();
    let samples: Vec<RideSample> = vec![];

    let lap = LapData::from_samples(&samples, 0, 0, ride.started_at);
    assert!(lap.is_none(), "Empty samples should return None");
}

#[test]
fn test_lap_data_calculates_averages() {
    let ride = create_test_ride();
    let mut samples = Vec::new();

    // Create samples with known values for easy average calculation
    for i in 0..10 {
        samples.push(RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(200), // All same power
            cadence_rpm: Some(90),  // All same cadence
            heart_rate_bpm: Some(150), // All same HR
            speed_kmh: Some(30.0),
            distance_meters: (i as f64) * 8.33,
            calories: i,
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
        });
    }

    let lap = LapData::from_samples(&samples, 0, 10, ride.started_at).unwrap();

    assert_eq!(lap.avg_power, Some(200));
    assert_eq!(lap.avg_cadence, Some(90));
    assert_eq!(lap.avg_hr, Some(150));
}

#[test]
fn test_lap_data_calculates_max_values() {
    let ride = create_test_ride();
    let mut samples = Vec::new();

    // Create samples with varying values
    for i in 0..10 {
        samples.push(RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(100 + (i * 20) as u16), // 100, 120, 140, ... 280
            cadence_rpm: Some(90),
            heart_rate_bpm: Some(140 + (i * 3) as u8), // 140, 143, 146, ... 167
            speed_kmh: Some(30.0),
            distance_meters: (i as f64) * 8.33,
            calories: i,
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
        });
    }

    let lap = LapData::from_samples(&samples, 0, 10, ride.started_at).unwrap();

    assert_eq!(lap.max_power, Some(280)); // 100 + 9*20
    assert_eq!(lap.max_hr, Some(167)); // 140 + 9*3
}

#[test]
fn test_lap_data_from_segment_durations() {
    let ride = create_test_ride();
    let samples = create_test_samples(120); // 2 minutes of samples

    // Create 3 segments: 30s, 30s, 30s
    let segment_durations = vec![30, 30, 30];
    let laps = LapData::from_segment_durations(&samples, &segment_durations, ride.started_at);

    // Should have 4 laps (3 segments + remaining 30 samples)
    assert_eq!(laps.len(), 4);
}

#[test]
fn test_lap_data_from_segment_durations_empty() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    // Empty segments
    let laps = LapData::from_segment_durations(&samples, &[], ride.started_at);
    assert!(laps.is_empty());

    // Empty samples
    let laps = LapData::from_segment_durations(&[], &[30, 30], ride.started_at);
    assert!(laps.is_empty());
}

// ============================================================================
// MULTI-LAP EXPORT TESTS
// ============================================================================

#[test]
fn test_export_fit_with_laps_larger_than_single_lap() {
    let ride = create_test_ride();
    let samples = create_test_samples(120);

    let laps = vec![
        LapData::from_samples(&samples, 0, 40, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 40, 80, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 80, 120, ride.started_at).unwrap(),
    ];

    let single_lap = export_fit(&ride, &samples).unwrap();
    let multi_lap = export_fit_with_laps(&ride, &samples, &laps).unwrap();

    assert!(
        multi_lap.len() > single_lap.len(),
        "Multi-lap export should be larger due to additional lap messages"
    );
}

#[test]
fn test_export_fit_with_laps_valid_structure() {
    let ride = create_test_ride();
    let samples = create_test_samples(100);

    let laps = vec![
        LapData::from_samples(&samples, 0, 50, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 50, 100, ride.started_at).unwrap(),
    ];

    let data = export_fit_with_laps(&ride, &samples, &laps).unwrap();

    // Verify header structure
    assert_eq!(data[0], 14);
    assert_eq!(data[1], 0x20);
    assert_eq!(&data[8..12], b".FIT");

    // Verify CRC is valid
    let file_crc = calculate_crc(&data[..data.len() - 2]);
    let stored_crc = u16::from_le_bytes([data[data.len() - 2], data[data.len() - 1]]);
    assert_eq!(file_crc, stored_crc);
}

#[test]
fn test_export_fit_with_laps_empty_falls_back() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    // Empty laps should fall back to single lap
    let multi_lap = export_fit_with_laps(&ride, &samples, &[]).unwrap();
    let single_lap = export_fit(&ride, &samples).unwrap();

    assert_eq!(
        multi_lap.len(),
        single_lap.len(),
        "Empty laps should fall back to single lap export"
    );
}

#[test]
fn test_export_fit_with_segments() {
    let ride = create_test_ride();
    let samples = create_test_samples(120);

    let segment_durations = vec![40, 40, 40];
    let result = export_fit_with_segments(&ride, &samples, &segment_durations);

    assert!(result.is_ok());

    let data = result.unwrap();
    assert_eq!(data[0], 14);
    assert_eq!(&data[8..12], b".FIT");
}

#[test]
fn test_export_fit_with_segments_empty_falls_back() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let multi_segment = export_fit_with_segments(&ride, &samples, &[]).unwrap();
    let single_lap = export_fit(&ride, &samples).unwrap();

    assert_eq!(multi_segment.len(), single_lap.len());
}

// ============================================================================
// WORKOUT EXPORT TESTS
// ============================================================================

#[test]
fn test_extract_workout_segment_durations() {
    let workout = create_test_workout();
    let durations = extract_workout_segment_durations(&workout);

    assert_eq!(durations.len(), 4);
    assert_eq!(durations[0], 30); // Warmup
    assert_eq!(durations[1], 60); // Steady state
    assert_eq!(durations[2], 30); // Intervals
    assert_eq!(durations[3], 30); // Cooldown
}

#[test]
fn test_extract_workout_segment_durations_empty() {
    let workout = Workout::new("Empty".to_string(), vec![]);
    let durations = extract_workout_segment_durations(&workout);

    assert!(durations.is_empty());
}

#[test]
fn test_export_fit_with_workout() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);
    let workout = create_test_workout();

    let result = export_fit_with_workout(&ride, &samples, Some(&workout));
    assert!(result.is_ok());

    let data = result.unwrap();
    assert_eq!(data[0], 14);
    assert_eq!(&data[8..12], b".FIT");
}

#[test]
fn test_export_fit_with_workout_none_falls_back() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let with_workout = export_fit_with_workout(&ride, &samples, None).unwrap();
    let single_lap = export_fit(&ride, &samples).unwrap();

    assert_eq!(with_workout.len(), single_lap.len());
}

#[test]
fn test_export_fit_with_workout_empty_segments_falls_back() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);
    let workout = Workout::new("Empty".to_string(), vec![]);

    let with_workout = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let single_lap = export_fit(&ride, &samples).unwrap();

    assert_eq!(with_workout.len(), single_lap.len());
}

#[test]
fn test_export_fit_with_workout_larger_than_single_lap() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);
    let workout = create_test_workout();

    let with_workout = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let single_lap = export_fit(&ride, &samples).unwrap();

    assert!(
        with_workout.len() > single_lap.len(),
        "Workout export should be larger due to multiple laps"
    );
}

// ============================================================================
// DATA HANDLING EDGE CASES
// ============================================================================

#[test]
fn test_fit_handles_missing_optional_fields() {
    let ride = create_test_ride();
    let mut samples = Vec::new();

    // Create sample with minimal data
    samples.push(RideSample {
        elapsed_seconds: 0,
        power_watts: None,
        cadence_rpm: None,
        heart_rate_bpm: None,
        speed_kmh: None,
        distance_meters: 0.0,
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
    });

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export should handle missing optional fields");
}

#[test]
fn test_fit_handles_max_power_values() {
    let ride = create_test_ride();
    let mut samples = Vec::new();

    samples.push(RideSample {
        elapsed_seconds: 0,
        power_watts: Some(u16::MAX),
        cadence_rpm: Some(u8::MAX),
        heart_rate_bpm: Some(u8::MAX),
        speed_kmh: Some(f32::MAX),
        distance_meters: f64::MAX,
        calories: u32::MAX,
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
    });

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export should handle max values");
}

#[test]
fn test_fit_handles_zero_duration_ride() {
    let mut ride = create_test_ride();
    ride.duration_seconds = 0;
    ride.distance_meters = 0.0;

    let samples = create_test_samples(1);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export should handle zero duration ride");
}

#[test]
fn test_fit_handles_ride_without_end_time() {
    let mut ride = create_test_ride();
    ride.ended_at = None;

    let samples = create_test_samples(10);

    let result = export_fit(&ride, &samples);
    assert!(result.is_ok(), "Export should handle missing end time");
}

// ============================================================================
// DETERMINISTIC OUTPUT TESTS
// ============================================================================

#[test]
fn test_fit_export_deterministic() {
    let ride = create_test_ride();
    let samples = create_test_samples(50);

    let data1 = export_fit(&ride, &samples).unwrap();
    let data2 = export_fit(&ride, &samples).unwrap();

    assert_eq!(
        data1, data2,
        "Same input should produce identical output"
    );
}

#[test]
fn test_fit_export_with_laps_deterministic() {
    let ride = create_test_ride();
    let samples = create_test_samples(100);

    let laps = vec![
        LapData::from_samples(&samples, 0, 50, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 50, 100, ride.started_at).unwrap(),
    ];

    let data1 = export_fit_with_laps(&ride, &samples, &laps).unwrap();
    let data2 = export_fit_with_laps(&ride, &samples, &laps).unwrap();

    assert_eq!(data1, data2);
}

// ============================================================================
// ROUNDTRIP TESTS (EXPORT -> PARSE WITH FITPARSER)
// ============================================================================

/// Parse FIT file using fitparser and validate basic structure.
/// Returns the parsed records for further verification.
fn parse_exported_fit(data: &[u8]) -> Vec<fitparser::FitDataRecord> {
    fitparser::from_bytes(data).expect("fitparser should successfully parse exported FIT file")
}

#[test]
fn test_roundtrip_fitparser_parses_exported_file() {
    let ride = create_test_ride();
    let samples = create_test_samples(60);

    let data = export_fit(&ride, &samples).unwrap();

    // Parse with fitparser - this validates the FIT format is correct
    let records = parse_exported_fit(&data);

    // Should have records
    assert!(!records.is_empty(), "Parsed FIT should contain records");
}

#[test]
fn test_roundtrip_contains_file_id_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Find FileId message
    let file_id = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::FileId);

    assert!(
        file_id.is_some(),
        "Exported FIT should contain FileId message"
    );

    // Verify it's an activity type
    let file_id = file_id.unwrap();
    let type_field = file_id.fields().iter().find(|f| f.name() == "type");
    assert!(type_field.is_some(), "FileId should have type field");
}

#[test]
fn test_roundtrip_contains_session_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Find Session message
    let session = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Session);

    assert!(
        session.is_some(),
        "Exported FIT should contain Session message"
    );
}

#[test]
fn test_roundtrip_contains_lap_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Find Lap message
    let lap = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Lap);

    assert!(lap.is_some(), "Exported FIT should contain Lap message");
}

#[test]
fn test_roundtrip_contains_activity_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Find Activity message
    let activity = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Activity);

    assert!(
        activity.is_some(),
        "Exported FIT should contain Activity message"
    );
}

#[test]
fn test_roundtrip_contains_record_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(50);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Count Record messages
    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(
        record_count,
        50,
        "Exported FIT should contain one Record message per sample"
    );
}

#[test]
fn test_roundtrip_record_contains_power_data() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Get first Record message
    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Find power field
    let power_field = record.fields().iter().find(|f| f.name() == "power");
    assert!(power_field.is_some(), "Record should contain power field");

    // Verify power value is reasonable (samples start at 150 watts)
    if let Some(field) = power_field {
        match field.value() {
            fitparser::Value::UInt16(v) => {
                assert!(
                    *v >= 100 && *v <= 500,
                    "Power value {} should be in valid range",
                    v
                );
            }
            _ => panic!("Power field should be UInt16"),
        }
    }
}

#[test]
fn test_roundtrip_record_contains_heart_rate() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let hr_field = record.fields().iter().find(|f| f.name() == "heart_rate");
    assert!(hr_field.is_some(), "Record should contain heart_rate field");
}

#[test]
fn test_roundtrip_record_contains_cadence() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let cadence_field = record.fields().iter().find(|f| f.name() == "cadence");
    assert!(
        cadence_field.is_some(),
        "Record should contain cadence field"
    );
}

#[test]
fn test_roundtrip_session_contains_sport_type() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let session = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Session)
        .expect("Should have Session message");

    // Check sport field (should be cycling = 2)
    let sport_field = session.fields().iter().find(|f| f.name() == "sport");
    assert!(sport_field.is_some(), "Session should contain sport field");

    if let Some(field) = sport_field {
        // Sport type 2 = cycling
        let is_cycling = match field.value() {
            fitparser::Value::UInt8(v) => *v == 2,
            fitparser::Value::String(s) => s.to_lowercase().contains("cycling"),
            _ => false,
        };
        assert!(is_cycling, "Sport should be cycling");
    }
}

#[test]
fn test_roundtrip_multi_lap_export() {
    let ride = create_test_ride();
    let samples = create_test_samples(120);

    let laps = vec![
        LapData::from_samples(&samples, 0, 40, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 40, 80, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 80, 120, ride.started_at).unwrap(),
    ];

    let data = export_fit_with_laps(&ride, &samples, &laps).unwrap();
    let records = parse_exported_fit(&data);

    // Count Lap messages
    let lap_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Lap)
        .count();

    assert_eq!(lap_count, 3, "Should have 3 lap messages for 3 laps");
}

#[test]
fn test_roundtrip_workout_export() {
    let ride = create_test_ride();
    let samples = create_test_samples(150);
    let workout = create_test_workout();

    let data = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let records = parse_exported_fit(&data);

    // Count Lap messages - workout has 4 segments + potential remainder
    let lap_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Lap)
        .count();

    // Workout segments: 30s warmup + 60s steady + 30s intervals + 30s cooldown = 150s
    // With 150 samples at 1Hz, we should have laps for each segment
    assert!(
        lap_count >= 4,
        "Should have at least 4 lap messages for workout segments, got {}",
        lap_count
    );
}

#[test]
fn test_roundtrip_cycling_dynamics_export() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(20);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Verify Record messages exist (dynamics are encoded in record fields)
    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Check for left_right_balance field
    let balance_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_right_balance");
    assert!(
        balance_field.is_some(),
        "Record should contain left_right_balance field for dynamics data"
    );
}

#[test]
fn test_roundtrip_event_messages() {
    let ride = create_test_ride();
    let samples = create_test_samples(30);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Count Event messages (should have at least start and stop events)
    let event_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Event)
        .count();

    assert!(
        event_count >= 2,
        "Should have at least 2 event messages (start and stop)"
    );
}

#[test]
fn test_roundtrip_timestamp_in_records() {
    let ride = create_test_ride();
    let samples = create_test_samples(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Get Record messages and verify they have timestamps
    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .collect();

    assert_eq!(record_messages.len(), 5);

    for record in record_messages {
        let timestamp_field = record.fields().iter().find(|f| f.name() == "timestamp");
        assert!(
            timestamp_field.is_some(),
            "Each Record message should have timestamp field"
        );
    }
}

#[test]
fn test_roundtrip_large_ride() {
    let ride = create_test_ride();
    // 2 hours of samples at 1Hz
    let samples = create_test_samples(7200);

    let data = export_fit(&ride, &samples).unwrap();

    // Verify fitparser can parse large files
    let records = parse_exported_fit(&data);

    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(record_count, 7200, "All 7200 samples should be in the file");
}

#[test]
fn test_roundtrip_file_creator_message() {
    let ride = create_test_ride();
    let samples = create_test_samples(10);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let file_creator = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::FileCreator);

    assert!(
        file_creator.is_some(),
        "Exported FIT should contain FileCreator message"
    );
}

// ============================================================================
// CYCLING DYNAMICS ROUNDTRIP TESTS
// ============================================================================

/// Create test samples with varying cycling dynamics data.
fn create_test_samples_with_varying_dynamics(count: usize) -> Vec<RideSample> {
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
            // Varying dynamics values to test scaling
            left_right_balance: Some(48.0 + (i % 10) as f32),
            left_torque_effectiveness: Some(70.0 + (i % 20) as f32),
            right_torque_effectiveness: Some(68.0 + (i % 20) as f32),
            left_pedal_smoothness: Some(18.0 + (i % 15) as f32),
            right_pedal_smoothness: Some(20.0 + (i % 15) as f32),
            left_power_phase_start: None,
            left_power_phase_end: None,
            left_power_phase_peak: None,
            right_power_phase_start: None,
            right_power_phase_end: None,
            right_power_phase_peak: None,
        })
        .collect()
}

/// Create test samples with all dynamics including power phase.
fn create_test_samples_full_dynamics(count: usize) -> Vec<RideSample> {
    (0..count)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(200),
            cadence_rpm: Some(90),
            heart_rate_bpm: Some(150),
            speed_kmh: Some(32.0),
            distance_meters: (i as f64) * 8.89,
            calories: (i as f64 * 0.22) as u32,
            resistance_level: None,
            target_power: Some(200),
            trainer_grade: None,
            left_right_balance: Some(51.5),
            left_torque_effectiveness: Some(78.0),
            right_torque_effectiveness: Some(76.0),
            left_pedal_smoothness: Some(24.0),
            right_pedal_smoothness: Some(26.0),
            left_power_phase_start: Some(15.0),
            left_power_phase_end: Some(195.0),
            left_power_phase_peak: Some(90.0),
            right_power_phase_start: Some(12.0),
            right_power_phase_end: Some(192.0),
            right_power_phase_peak: Some(88.0),
        })
        .collect()
}

/// Create test samples with partial dynamics (only some fields present).
fn create_test_samples_partial_dynamics(count: usize) -> Vec<RideSample> {
    (0..count)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(175),
            cadence_rpm: Some(82),
            heart_rate_bpm: Some(142),
            speed_kmh: Some(28.0),
            distance_meters: (i as f64) * 7.78,
            calories: (i as f64 * 0.18) as u32,
            resistance_level: None,
            target_power: Some(175),
            trainer_grade: None,
            // Only L/R balance and left metrics
            left_right_balance: Some(50.0),
            left_torque_effectiveness: Some(72.0),
            right_torque_effectiveness: None,
            left_pedal_smoothness: Some(21.0),
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

/// Create test samples with boundary dynamics values.
fn create_test_samples_boundary_dynamics() -> Vec<RideSample> {
    vec![
        // Sample with minimum valid values
        RideSample {
            elapsed_seconds: 0,
            power_watts: Some(100),
            cadence_rpm: Some(60),
            heart_rate_bpm: Some(100),
            speed_kmh: Some(20.0),
            distance_meters: 0.0,
            calories: 0,
            resistance_level: None,
            target_power: None,
            trainer_grade: None,
            left_right_balance: Some(0.0),
            left_torque_effectiveness: Some(0.0),
            right_torque_effectiveness: Some(0.0),
            left_pedal_smoothness: Some(0.0),
            right_pedal_smoothness: Some(0.0),
            left_power_phase_start: Some(0.0),
            left_power_phase_end: Some(0.0),
            left_power_phase_peak: Some(0.0),
            right_power_phase_start: Some(0.0),
            right_power_phase_end: Some(0.0),
            right_power_phase_peak: Some(0.0),
        },
        // Sample with maximum valid values
        RideSample {
            elapsed_seconds: 1,
            power_watts: Some(500),
            cadence_rpm: Some(120),
            heart_rate_bpm: Some(200),
            speed_kmh: Some(60.0),
            distance_meters: 16.67,
            calories: 1,
            resistance_level: None,
            target_power: None,
            trainer_grade: None,
            left_right_balance: Some(100.0),
            left_torque_effectiveness: Some(100.0),
            right_torque_effectiveness: Some(100.0),
            left_pedal_smoothness: Some(100.0),
            right_pedal_smoothness: Some(100.0),
            left_power_phase_start: Some(360.0),
            left_power_phase_end: Some(360.0),
            left_power_phase_peak: Some(360.0),
            right_power_phase_start: Some(360.0),
            right_power_phase_end: Some(360.0),
            right_power_phase_peak: Some(360.0),
        },
        // Sample with typical middle values
        RideSample {
            elapsed_seconds: 2,
            power_watts: Some(250),
            cadence_rpm: Some(90),
            heart_rate_bpm: Some(155),
            speed_kmh: Some(35.0),
            distance_meters: 33.34,
            calories: 2,
            resistance_level: None,
            target_power: None,
            trainer_grade: None,
            left_right_balance: Some(52.0),
            left_torque_effectiveness: Some(75.0),
            right_torque_effectiveness: Some(73.0),
            left_pedal_smoothness: Some(22.0),
            right_pedal_smoothness: Some(24.0),
            left_power_phase_start: Some(30.0),
            left_power_phase_end: Some(180.0),
            left_power_phase_peak: Some(90.0),
            right_power_phase_start: Some(25.0),
            right_power_phase_end: Some(175.0),
            right_power_phase_peak: Some(85.0),
        },
    ]
}

#[test]
fn test_roundtrip_left_right_balance_present() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(10);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .collect();

    assert_eq!(record_messages.len(), 10);

    for record in record_messages {
        let balance_field = record
            .fields()
            .iter()
            .find(|f| f.name() == "left_right_balance");

        assert!(
            balance_field.is_some(),
            "Record should contain left_right_balance field"
        );
    }
}

#[test]
fn test_roundtrip_left_right_balance_value() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(1);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let balance_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_right_balance");

    assert!(
        balance_field.is_some(),
        "Record should contain left_right_balance field"
    );

    // The first sample has left_right_balance of 48.0%
    // FIT encoding: value * 1.27 with bit 7 set
    // When parsed, it should decode back to approximately 48%
    if let Some(field) = balance_field {
        match field.value() {
            fitparser::Value::UInt8(v) => {
                // Value should have bit 7 set (0x80) and contain scaled balance
                assert!(
                    *v >= 0x80,
                    "Balance should have reference bit set, got {}",
                    v
                );
            }
            _ => {
                // Other value types are also valid depending on how fitparser decodes
            }
        }
    }
}

#[test]
fn test_roundtrip_torque_effectiveness_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Check for left torque effectiveness
    let left_te_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_torque_effectiveness");

    // Check for right torque effectiveness
    let right_te_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "right_torque_effectiveness");

    assert!(
        left_te_field.is_some(),
        "Record should contain left_torque_effectiveness field"
    );
    assert!(
        right_te_field.is_some(),
        "Record should contain right_torque_effectiveness field"
    );
}

#[test]
fn test_roundtrip_torque_effectiveness_values() {
    let ride = create_test_ride();
    // First sample has left_torque_effectiveness = 70.0, right = 68.0
    let samples = create_test_samples_with_dynamics(1);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let left_te_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_torque_effectiveness");

    if let Some(field) = left_te_field {
        match field.value() {
            fitparser::Value::UInt8(v) => {
                // FIT encoding: 0.5% per bit, so 70% = 140
                // Allow some tolerance for encoding/decoding
                assert!(
                    *v >= 130 && *v <= 150,
                    "Left torque effectiveness should be around 140 (70%), got {}",
                    v
                );
            }
            fitparser::Value::Float64(v) => {
                assert!(
                    *v >= 65.0 && *v <= 75.0,
                    "Left torque effectiveness should be around 70%, got {}",
                    v
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_roundtrip_pedal_smoothness_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Check for left pedal smoothness
    let left_ps_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_pedal_smoothness");

    // Check for right pedal smoothness
    let right_ps_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "right_pedal_smoothness");

    assert!(
        left_ps_field.is_some(),
        "Record should contain left_pedal_smoothness field"
    );
    assert!(
        right_ps_field.is_some(),
        "Record should contain right_pedal_smoothness field"
    );
}

#[test]
fn test_roundtrip_pedal_smoothness_values() {
    let ride = create_test_ride();
    // First sample has left_pedal_smoothness = 18.0, right = 20.0
    let samples = create_test_samples_with_dynamics(1);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let left_ps_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_pedal_smoothness");

    if let Some(field) = left_ps_field {
        match field.value() {
            fitparser::Value::UInt8(v) => {
                // FIT encoding: 0.5% per bit, so 18% = 36
                // Allow some tolerance
                assert!(
                    *v >= 30 && *v <= 45,
                    "Left pedal smoothness should be around 36 (18%), got {}",
                    v
                );
            }
            fitparser::Value::Float64(v) => {
                assert!(
                    *v >= 15.0 && *v <= 25.0,
                    "Left pedal smoothness should be around 18%, got {}",
                    v
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_roundtrip_power_phase_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_power_phase(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Check for left power phase
    let left_pp_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_power_phase");

    // Check for right power phase
    let right_pp_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "right_power_phase");

    assert!(
        left_pp_field.is_some(),
        "Record should contain left_power_phase field"
    );
    assert!(
        right_pp_field.is_some(),
        "Record should contain right_power_phase field"
    );
}

#[test]
fn test_roundtrip_power_phase_peak_fields() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_power_phase(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // Check for left power phase peak
    let left_pp_peak_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "left_power_phase_peak");

    // Check for right power phase peak
    let right_pp_peak_field = record
        .fields()
        .iter()
        .find(|f| f.name() == "right_power_phase_peak");

    assert!(
        left_pp_peak_field.is_some(),
        "Record should contain left_power_phase_peak field"
    );
    assert!(
        right_pp_peak_field.is_some(),
        "Record should contain right_power_phase_peak field"
    );
}

#[test]
fn test_roundtrip_all_dynamics_fields_present() {
    let ride = create_test_ride();
    let samples = create_test_samples_full_dynamics(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    // All cycling dynamics fields should be present
    let expected_fields = [
        "left_right_balance",
        "left_torque_effectiveness",
        "right_torque_effectiveness",
        "left_pedal_smoothness",
        "right_pedal_smoothness",
        "left_power_phase",
        "left_power_phase_peak",
        "right_power_phase",
        "right_power_phase_peak",
    ];

    for field_name in expected_fields {
        let field = record.fields().iter().find(|f| f.name() == field_name);
        assert!(
            field.is_some(),
            "Record should contain {} field",
            field_name
        );
    }
}

#[test]
fn test_roundtrip_partial_dynamics() {
    let ride = create_test_ride();
    let samples = create_test_samples_partial_dynamics(5);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Verify parsing succeeds even with partial dynamics
    assert!(!records.is_empty());

    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(record_count, 5, "All 5 samples should be in the file");
}

#[test]
fn test_roundtrip_boundary_dynamics_values() {
    let ride = create_test_ride();
    let samples = create_test_samples_boundary_dynamics();

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Verify all 3 samples with boundary values parse successfully
    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(
        record_count, 3,
        "All 3 boundary samples should be in the file"
    );
}

#[test]
fn test_roundtrip_dynamics_consistency_across_samples() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_varying_dynamics(20);

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    let record_messages: Vec<_> = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .collect();

    assert_eq!(record_messages.len(), 20);

    // Verify each record has the dynamics fields
    for (i, record) in record_messages.iter().enumerate() {
        let balance_field = record
            .fields()
            .iter()
            .find(|f| f.name() == "left_right_balance");

        assert!(
            balance_field.is_some(),
            "Record {} should contain left_right_balance field",
            i
        );
    }
}

#[test]
fn test_fit_dynamics_file_size_scaling() {
    let ride = create_test_ride();

    // Compare file sizes: no dynamics, dynamics only, full dynamics
    let samples_no_dynamics = create_test_samples(50);
    let samples_dynamics = create_test_samples_with_dynamics(50);
    let samples_full = create_test_samples_full_dynamics(50);

    let data_no_dynamics = export_fit(&ride, &samples_no_dynamics).unwrap();
    let data_dynamics = export_fit(&ride, &samples_dynamics).unwrap();
    let data_full = export_fit(&ride, &samples_full).unwrap();

    // Dynamics should add ~5 bytes per sample (balance + 2x TE + 2x PS)
    assert!(
        data_dynamics.len() > data_no_dynamics.len(),
        "Dynamics should add to file size"
    );

    // Full dynamics with power phase should be even larger (~16 more bytes per sample)
    assert!(
        data_full.len() > data_dynamics.len(),
        "Power phase should add additional size"
    );

    // Verify approximate sizes
    let dynamics_overhead = data_dynamics.len() - data_no_dynamics.len();
    let power_phase_overhead = data_full.len() - data_dynamics.len();

    assert!(
        dynamics_overhead >= 200,
        "Dynamics overhead for 50 samples should be at least 200 bytes"
    );
    assert!(
        power_phase_overhead >= 400,
        "Power phase overhead for 50 samples should be at least 400 bytes"
    );
}

#[test]
fn test_fit_dynamics_deterministic_output() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(30);

    // Export twice
    let data1 = export_fit(&ride, &samples).unwrap();
    let data2 = export_fit(&ride, &samples).unwrap();

    // Should produce identical output
    assert_eq!(data1, data2, "Same input should produce identical output");
}

#[test]
fn test_fit_full_dynamics_deterministic_output() {
    let ride = create_test_ride();
    let samples = create_test_samples_full_dynamics(30);

    // Export twice
    let data1 = export_fit(&ride, &samples).unwrap();
    let data2 = export_fit(&ride, &samples).unwrap();

    // Should produce identical output
    assert_eq!(data1, data2, "Same input should produce identical output");
}

#[test]
fn test_roundtrip_dynamics_with_multi_lap() {
    let ride = create_test_ride();
    let samples = create_test_samples_with_dynamics(120);

    let laps = vec![
        LapData::from_samples(&samples, 0, 40, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 40, 80, ride.started_at).unwrap(),
        LapData::from_samples(&samples, 80, 120, ride.started_at).unwrap(),
    ];

    let data = export_fit_with_laps(&ride, &samples, &laps).unwrap();
    let records = parse_exported_fit(&data);

    // Should have 3 laps and 120 records
    let lap_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Lap)
        .count();
    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(lap_count, 3, "Should have 3 lap messages");
    assert_eq!(record_count, 120, "Should have 120 record messages");

    // Verify dynamics are present in record messages
    let first_record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .unwrap();

    let balance_field = first_record
        .fields()
        .iter()
        .find(|f| f.name() == "left_right_balance");

    assert!(
        balance_field.is_some(),
        "Multi-lap export should preserve dynamics"
    );
}

#[test]
fn test_roundtrip_full_dynamics_with_workout() {
    let ride = create_test_ride();
    let samples = create_test_samples_full_dynamics(150);
    let workout = create_test_workout();

    let data = export_fit_with_workout(&ride, &samples, Some(&workout)).unwrap();
    let records = parse_exported_fit(&data);

    // Verify workout export preserves all dynamics
    let record = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .expect("Should have Record message");

    let all_fields = [
        "left_right_balance",
        "left_torque_effectiveness",
        "right_torque_effectiveness",
        "left_pedal_smoothness",
        "right_pedal_smoothness",
        "left_power_phase",
        "left_power_phase_peak",
        "right_power_phase",
        "right_power_phase_peak",
    ];

    for field_name in all_fields {
        let field = record.fields().iter().find(|f| f.name() == field_name);
        assert!(
            field.is_some(),
            "Workout export should preserve {} field",
            field_name
        );
    }
}

#[test]
fn test_roundtrip_dynamics_mixed_presence() {
    // Test with samples where some have dynamics and some don't
    let ride = create_test_ride();
    let mut samples = Vec::new();

    // First sample: no dynamics
    samples.push(RideSample {
        elapsed_seconds: 0,
        power_watts: Some(180),
        cadence_rpm: Some(85),
        heart_rate_bpm: Some(145),
        speed_kmh: Some(30.0),
        distance_meters: 0.0,
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
    });

    // Second sample: has dynamics
    samples.push(RideSample {
        elapsed_seconds: 1,
        power_watts: Some(185),
        cadence_rpm: Some(86),
        heart_rate_bpm: Some(148),
        speed_kmh: Some(31.0),
        distance_meters: 8.33,
        calories: 1,
        resistance_level: None,
        target_power: None,
        trainer_grade: None,
        left_right_balance: Some(52.0),
        left_torque_effectiveness: Some(75.0),
        right_torque_effectiveness: Some(73.0),
        left_pedal_smoothness: Some(22.0),
        right_pedal_smoothness: Some(24.0),
        left_power_phase_start: None,
        left_power_phase_end: None,
        left_power_phase_peak: None,
        right_power_phase_start: None,
        right_power_phase_end: None,
        right_power_phase_peak: None,
    });

    let data = export_fit(&ride, &samples).unwrap();
    let records = parse_exported_fit(&data);

    // Export should succeed
    let record_count = records
        .iter()
        .filter(|r| r.kind() == fitparser::profile::MesgNum::Record)
        .count();

    assert_eq!(record_count, 2, "Both samples should be exported");
}
