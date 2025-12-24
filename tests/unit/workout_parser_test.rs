//! Unit tests for workout parsers.
//!
//! T054: Unit test for .zwo parsing
//! T055: Unit test for .mrc parsing

use rustride::workouts::parser_mrc::parse_mrc;
use rustride::workouts::parser_zwo::parse_zwo;
use rustride::workouts::types::{PowerTarget, SegmentType, WorkoutFormat};

/// Sample ZWO workout XML for testing.
const SAMPLE_ZWO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<workout_file>
    <author>Test Author</author>
    <name>Test Workout</name>
    <description>A simple test workout</description>
    <sportType>bike</sportType>
    <tags>
        <tag name="Intervals"/>
        <tag name="FTP"/>
    </tags>
    <workout>
        <Warmup Duration="300" PowerLow="0.4" PowerHigh="0.7"/>
        <SteadyState Duration="600" Power="0.75" Cadence="90"/>
        <IntervalsT Repeat="4" OnDuration="30" OffDuration="30" OnPower="1.2" OffPower="0.5"/>
        <SteadyState Duration="300" Power="0.6"/>
        <Cooldown Duration="300" PowerLow="0.5" PowerHigh="0.3"/>
    </workout>
</workout_file>"#;

/// Sample MRC workout for testing.
const SAMPLE_MRC: &str = r#"[COURSE HEADER]
VERSION = 2
UNITS = ENGLISH
DESCRIPTION = Test MRC Workout
FILE NAME = test_workout
MINUTES PERCENT
[END COURSE HEADER]
[COURSE DATA]
0.00    50
5.00    50
5.00    75
10.00   75
10.00   100
15.00   100
15.00   50
20.00   50
[END COURSE DATA]
[COURSE TEXT]
0.00    "Warmup"
5.00    "Zone 3"
10.00   "Threshold"
15.00   "Cooldown"
[END COURSE TEXT]"#;

#[test]
fn test_parse_zwo_basic() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    assert_eq!(workout.name, "Test Workout");
    assert_eq!(workout.author.as_deref(), Some("Test Author"));
    assert_eq!(workout.description.as_deref(), Some("A simple test workout"));
    assert_eq!(workout.source_format, Some(WorkoutFormat::Zwo));
    assert!(!workout.segments.is_empty());
}

#[test]
fn test_parse_zwo_warmup() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    // First segment should be warmup
    let warmup = &workout.segments[0];
    assert_eq!(warmup.segment_type, SegmentType::Warmup);
    assert_eq!(warmup.duration_seconds, 300);

    // Should have a range power target (40% to 70%)
    match &warmup.power_target {
        PowerTarget::Range { start, end } => {
            match (start.as_ref(), end.as_ref()) {
                (PowerTarget::PercentFtp { percent: s }, PowerTarget::PercentFtp { percent: e }) => {
                    assert_eq!(*s, 40);
                    assert_eq!(*e, 70);
                }
                _ => panic!("Expected PercentFtp targets"),
            }
        }
        _ => panic!("Expected Range power target for warmup"),
    }
}

#[test]
fn test_parse_zwo_steady_state() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    // Second segment should be steady state at 75%
    let steady = &workout.segments[1];
    assert_eq!(steady.segment_type, SegmentType::SteadyState);
    assert_eq!(steady.duration_seconds, 600);

    match &steady.power_target {
        PowerTarget::PercentFtp { percent } => {
            assert_eq!(*percent, 75);
        }
        _ => panic!("Expected PercentFtp power target"),
    }
}

#[test]
fn test_parse_zwo_intervals() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    // After warmup and steady state, we should have interval segments
    // 4 repeats * 2 (on + off) = 8 segments
    let interval_segments: Vec<_> = workout
        .segments
        .iter()
        .filter(|s| s.segment_type == SegmentType::Intervals)
        .collect();

    assert_eq!(interval_segments.len(), 8);
}

#[test]
fn test_parse_zwo_cooldown() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    // Last segment should be cooldown
    let cooldown = workout.segments.last().expect("Should have segments");
    assert_eq!(cooldown.segment_type, SegmentType::Cooldown);
    assert_eq!(cooldown.duration_seconds, 300);
}

#[test]
fn test_parse_zwo_total_duration() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    // Warmup: 300s + SteadyState: 600s + Intervals: 4*60s=240s + SteadyState: 300s + Cooldown: 300s
    // Total: 300 + 600 + 240 + 300 + 300 = 1740s
    assert_eq!(workout.total_duration_seconds, 1740);
}

#[test]
fn test_parse_zwo_tags() {
    let workout = parse_zwo(SAMPLE_ZWO).expect("Should parse ZWO workout");

    assert!(workout.tags.contains(&"Intervals".to_string()));
    assert!(workout.tags.contains(&"FTP".to_string()));
}

#[test]
fn test_parse_zwo_empty() {
    let empty_zwo = r#"<?xml version="1.0"?>
<workout_file>
    <name>Empty Workout</name>
    <workout/>
</workout_file>"#;

    let result = parse_zwo(empty_zwo);
    assert!(result.is_err());
}

#[test]
fn test_parse_zwo_invalid_xml() {
    let invalid = "not valid xml at all";
    let result = parse_zwo(invalid);
    assert!(result.is_err());
}

// MRC Parser Tests

#[test]
fn test_parse_mrc_basic() {
    let workout = parse_mrc(SAMPLE_MRC).expect("Should parse MRC workout");

    assert_eq!(workout.name, "test_workout");
    assert_eq!(
        workout.description.as_deref(),
        Some("Test MRC Workout")
    );
    assert_eq!(workout.source_format, Some(WorkoutFormat::Mrc));
    assert!(!workout.segments.is_empty());
}

#[test]
fn test_parse_mrc_segments() {
    let workout = parse_mrc(SAMPLE_MRC).expect("Should parse MRC workout");

    // Should have 4 segments based on the course data
    // 0-5min: 50%, 5-10min: 75%, 10-15min: 100%, 15-20min: 50%
    assert_eq!(workout.segments.len(), 4);

    // Check first segment
    let first = &workout.segments[0];
    assert_eq!(first.duration_seconds, 300); // 5 minutes

    // Check power targets are percent FTP
    match &first.power_target {
        PowerTarget::PercentFtp { percent } => {
            assert_eq!(*percent, 50);
        }
        _ => panic!("Expected PercentFtp"),
    }
}

#[test]
fn test_parse_mrc_total_duration() {
    let workout = parse_mrc(SAMPLE_MRC).expect("Should parse MRC workout");

    // 20 minutes = 1200 seconds
    assert_eq!(workout.total_duration_seconds, 1200);
}

#[test]
fn test_parse_mrc_text_events() {
    let workout = parse_mrc(SAMPLE_MRC).expect("Should parse MRC workout");

    // First segment should have "Warmup" text
    assert_eq!(workout.segments[0].text_event.as_deref(), Some("Warmup"));
}

#[test]
fn test_parse_mrc_empty() {
    let empty_mrc = r#"[COURSE HEADER]
VERSION = 2
[END COURSE HEADER]
[COURSE DATA]
[END COURSE DATA]"#;

    let result = parse_mrc(empty_mrc);
    assert!(result.is_err());
}

#[test]
fn test_parse_mrc_invalid_format() {
    let invalid = "random text that is not MRC format";
    let result = parse_mrc(invalid);
    assert!(result.is_err());
}
