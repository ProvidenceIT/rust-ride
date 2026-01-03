//! Integration tests for workout export functionality.
//!
//! These tests verify that exported workout files (ZWO and MRC formats)
//! can be parsed back and produce equivalent workouts.
//!
//! T056: Integration test for ZWO export round-trip
//! T057: Integration test for MRC export round-trip

use rustride::workouts::{
    export_mrc, export_mrc_with_ftp, export_zwo, parse_mrc, parse_zwo, CadenceTarget, PowerTarget,
    SegmentType, Workout, WorkoutExportError, WorkoutSegment,
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a simple workout with a single steady state segment.
fn create_simple_workout(name: &str, duration_seconds: u32, power_percent: u8) -> Workout {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::SteadyState,
        duration_seconds,
        power_target: PowerTarget::percent_ftp(power_percent),
        cadence_target: None,
        text_event: None,
    }];
    Workout::new(name.to_string(), segments)
}

/// Creates a complex multi-segment workout with all segment types.
fn create_complex_workout() -> Workout {
    let segments = vec![
        // Warmup: 10 minutes, 40% -> 70%
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 600,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(70),
            ),
            cadence_target: None,
            text_event: Some("Easy spin to warm up".to_string()),
        },
        // Sweet spot: 15 minutes at 88%
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 900,
            power_target: PowerTarget::percent_ftp(88),
            cadence_target: Some(CadenceTarget {
                min_rpm: 85,
                max_rpm: 95,
            }),
            text_event: Some("Sweet spot effort".to_string()),
        },
        // Recovery: 5 minutes at 50%
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: None,
        },
        // Ramp up: 3 minutes, 80% -> 100%
        WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 180,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(80),
                PowerTarget::percent_ftp(100),
            ),
            cadence_target: None,
            text_event: Some("Build to threshold".to_string()),
        },
        // Threshold: 10 minutes at 100%
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 600,
            power_target: PowerTarget::percent_ftp(100),
            cadence_target: Some(CadenceTarget {
                min_rpm: 90,
                max_rpm: 90,
            }),
            text_event: Some("Hold threshold".to_string()),
        },
        // FreeRide: 5 minutes
        WorkoutSegment {
            segment_type: SegmentType::FreeRide,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(0),
            cadence_target: None,
            text_event: Some("Ride easy".to_string()),
        },
        // Cooldown: 10 minutes, 60% -> 40%
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 600,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: Some("Cool down".to_string()),
        },
    ];

    let mut workout = Workout::new("Complex Training Ride".to_string(), segments);
    workout.author = Some("RustRide Trainer".to_string());
    workout.description =
        Some("A complex workout testing all segment types and features".to_string());
    workout.tags = vec![
        "Threshold".to_string(),
        "Sweet Spot".to_string(),
        "Complex".to_string(),
    ];
    workout
}

/// Creates a workout simulating interval training.
fn create_interval_workout() -> Workout {
    let mut segments = Vec::new();

    // Warmup
    segments.push(WorkoutSegment {
        segment_type: SegmentType::Warmup,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(40),
            PowerTarget::percent_ftp(60),
        ),
        cadence_target: None,
        text_event: Some("Warm up".to_string()),
    });

    // 8 x (30s ON / 30s OFF) intervals
    for i in 0..8 {
        // ON interval (high power)
        segments.push(WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(120),
            cadence_target: Some(CadenceTarget {
                min_rpm: 95,
                max_rpm: 105,
            }),
            text_event: Some(format!("Interval {} - GO!", i + 1)),
        });

        // OFF interval (recovery)
        segments.push(WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: None,
        });
    }

    // Cooldown
    segments.push(WorkoutSegment {
        segment_type: SegmentType::Cooldown,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(60),
            PowerTarget::percent_ftp(40),
        ),
        cadence_target: None,
        text_event: Some("Cool down".to_string()),
    });

    let mut workout = Workout::new("8x30s Intervals".to_string(), segments);
    workout.description = Some("High intensity interval training".to_string());
    workout.tags = vec!["HIIT".to_string(), "Intervals".to_string()];
    workout
}

// =============================================================================
// ZWO Export Round-Trip Tests
// =============================================================================

#[test]
fn test_zwo_round_trip_simple_workout() {
    let original = create_simple_workout("Simple Steady State", 300, 75);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    // Verify name preserved
    assert_eq!(parsed.name, original.name);

    // Verify segment count
    assert_eq!(parsed.segments.len(), original.segments.len());

    // Verify duration
    assert_eq!(
        parsed.segments[0].duration_seconds,
        original.segments[0].duration_seconds
    );

    // Verify power target
    match &parsed.segments[0].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 75),
        _ => panic!("Expected PercentFtp power target"),
    }
}

#[test]
fn test_zwo_round_trip_complex_workout() {
    let original = create_complex_workout();

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    // Verify name
    assert_eq!(parsed.name, "Complex Training Ride");

    // Verify metadata
    assert_eq!(parsed.author.as_deref(), Some("RustRide Trainer"));
    assert!(parsed
        .description
        .as_ref()
        .map(|d| d.contains("complex workout"))
        .unwrap_or(false));

    // Verify segment count
    assert_eq!(parsed.segments.len(), original.segments.len());

    // Verify total duration
    assert_eq!(parsed.total_duration_seconds, original.total_duration_seconds);

    // Verify tags
    assert_eq!(parsed.tags.len(), original.tags.len());
    for tag in &original.tags {
        assert!(parsed.tags.contains(tag), "Tag '{}' should be present", tag);
    }

    // Verify segment types preserved
    let expected_types = [
        SegmentType::Warmup,
        SegmentType::SteadyState,
        SegmentType::SteadyState,
        SegmentType::Ramp,
        SegmentType::SteadyState,
        SegmentType::FreeRide,
        SegmentType::Cooldown,
    ];

    for (i, expected_type) in expected_types.iter().enumerate() {
        assert_eq!(
            parsed.segments[i].segment_type, *expected_type,
            "Segment {} should be {:?}",
            i, expected_type
        );
    }

    // Verify individual segment durations
    let expected_durations = [600, 900, 300, 180, 600, 300, 600];
    for (i, expected_dur) in expected_durations.iter().enumerate() {
        assert_eq!(
            parsed.segments[i].duration_seconds, *expected_dur,
            "Segment {} should have duration {}",
            i, expected_dur
        );
    }
}

#[test]
fn test_zwo_round_trip_interval_workout() {
    let original = create_interval_workout();

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    // Verify name
    assert_eq!(parsed.name, "8x30s Intervals");

    // Verify segment count (warmup + 16 intervals + cooldown = 18)
    assert_eq!(parsed.segments.len(), original.segments.len());
    assert_eq!(parsed.segments.len(), 18);

    // Verify total duration: 300 (warmup) + 16*30 (intervals) + 300 (cooldown) = 1080s
    assert_eq!(parsed.total_duration_seconds, 1080);
}

#[test]
fn test_zwo_round_trip_warmup_power_range() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Warmup,
        duration_seconds: 600,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(40),
            PowerTarget::percent_ftp(70),
        ),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Warmup Test".to_string(), segments);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::Warmup);
    assert_eq!(parsed.segments[0].duration_seconds, 600);

    match &parsed.segments[0].power_target {
        PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
            (
                PowerTarget::PercentFtp { percent: start_pct },
                PowerTarget::PercentFtp { percent: end_pct },
            ) => {
                assert_eq!(*start_pct, 40);
                assert_eq!(*end_pct, 70);
            }
            _ => panic!("Expected PercentFtp in range"),
        },
        _ => panic!("Expected Range power target"),
    }
}

#[test]
fn test_zwo_round_trip_cooldown_power_range() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Cooldown,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(60),
            PowerTarget::percent_ftp(40),
        ),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Cooldown Test".to_string(), segments);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::Cooldown);

    match &parsed.segments[0].power_target {
        PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
            (
                PowerTarget::PercentFtp { percent: start_pct },
                PowerTarget::PercentFtp { percent: end_pct },
            ) => {
                assert_eq!(*start_pct, 60);
                assert_eq!(*end_pct, 40);
            }
            _ => panic!("Expected PercentFtp in range"),
        },
        _ => panic!("Expected Range power target"),
    }
}

#[test]
fn test_zwo_round_trip_cadence_range() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::SteadyState,
        duration_seconds: 300,
        power_target: PowerTarget::percent_ftp(85),
        cadence_target: Some(CadenceTarget {
            min_rpm: 80,
            max_rpm: 100,
        }),
        text_event: None,
    }];
    let original = Workout::new("Cadence Test".to_string(), segments);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    let cadence = parsed.segments[0]
        .cadence_target
        .as_ref()
        .expect("Should have cadence");
    assert_eq!(cadence.min_rpm, 80);
    assert_eq!(cadence.max_rpm, 100);
}

#[test]
fn test_zwo_round_trip_special_characters() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::SteadyState,
        duration_seconds: 60,
        power_target: PowerTarget::percent_ftp(75),
        cadence_target: None,
        text_event: None,
    }];
    let mut original = Workout::new("Test & <Special> Chars".to_string(), segments);
    original.author = Some("Author with & <special> chars".to_string());
    original.description = Some("Description with \"quotes\" and & ampersands".to_string());

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    assert_eq!(parsed.name, "Test & <Special> Chars");
    assert_eq!(
        parsed.author.as_deref(),
        Some("Author with & <special> chars")
    );
    assert!(parsed
        .description
        .as_ref()
        .map(|d| d.contains("\"quotes\""))
        .unwrap_or(false));
}

#[test]
fn test_zwo_round_trip_freeride() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::FreeRide,
        duration_seconds: 900,
        power_target: PowerTarget::percent_ftp(0),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("FreeRide Test".to_string(), segments);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::FreeRide);
    assert_eq!(parsed.segments[0].duration_seconds, 900);
}

#[test]
fn test_zwo_round_trip_ramp() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Ramp,
        duration_seconds: 240,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(50),
            PowerTarget::percent_ftp(100),
        ),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Ramp Test".to_string(), segments);

    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let parsed = parse_zwo(&zwo_content).expect("Should parse exported ZWO");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::Ramp);
    assert_eq!(parsed.segments[0].duration_seconds, 240);

    match &parsed.segments[0].power_target {
        PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
            (
                PowerTarget::PercentFtp { percent: start_pct },
                PowerTarget::PercentFtp { percent: end_pct },
            ) => {
                assert_eq!(*start_pct, 50);
                assert_eq!(*end_pct, 100);
            }
            _ => panic!("Expected PercentFtp in range"),
        },
        _ => panic!("Expected Range power target"),
    }
}

#[test]
fn test_zwo_export_empty_workout_error() {
    let workout = Workout::new("Empty".to_string(), vec![]);
    let result = export_zwo(&workout);
    assert!(matches!(result, Err(WorkoutExportError::EmptyWorkout)));
}

// =============================================================================
// MRC Export Round-Trip Tests
// =============================================================================

#[test]
fn test_mrc_round_trip_simple_workout() {
    let original = create_simple_workout("Simple Steady State", 300, 75);

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    // Verify name preserved
    assert_eq!(parsed.name, original.name);

    // Verify segment count
    assert_eq!(parsed.segments.len(), original.segments.len());

    // Verify duration
    assert_eq!(
        parsed.segments[0].duration_seconds,
        original.segments[0].duration_seconds
    );

    // Verify power target
    match &parsed.segments[0].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 75),
        _ => panic!("Expected PercentFtp power target"),
    }
}

#[test]
fn test_mrc_round_trip_complex_workout() {
    let original = create_complex_workout();

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    // Verify name
    assert_eq!(parsed.name, "Complex Training Ride");

    // Verify description
    assert!(parsed
        .description
        .as_ref()
        .map(|d| d.contains("complex workout"))
        .unwrap_or(false));

    // Verify segment count
    assert_eq!(parsed.segments.len(), original.segments.len());

    // Verify total duration
    assert_eq!(parsed.total_duration_seconds, original.total_duration_seconds);

    // Verify segment types (MRC uses Warmup/Cooldown for ramps, SteadyState for constant)
    assert_eq!(parsed.segments[0].segment_type, SegmentType::Warmup);
    assert_eq!(parsed.segments[1].segment_type, SegmentType::SteadyState);
    assert_eq!(parsed.segments[2].segment_type, SegmentType::SteadyState);
    assert_eq!(parsed.segments[3].segment_type, SegmentType::Warmup); // Ramp up in MRC is warmup
    assert_eq!(parsed.segments[4].segment_type, SegmentType::SteadyState);
    // FreeRide becomes SteadyState at 0%
    assert_eq!(parsed.segments[5].segment_type, SegmentType::SteadyState);
    assert_eq!(parsed.segments[6].segment_type, SegmentType::Cooldown);

    // Verify individual segment durations
    let expected_durations = [600, 900, 300, 180, 600, 300, 600];
    for (i, expected_dur) in expected_durations.iter().enumerate() {
        assert_eq!(
            parsed.segments[i].duration_seconds, *expected_dur,
            "Segment {} should have duration {}",
            i, expected_dur
        );
    }
}

#[test]
fn test_mrc_round_trip_interval_workout() {
    let original = create_interval_workout();

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    // Verify name
    assert_eq!(parsed.name, "8x30s Intervals");

    // Verify segment count (warmup + 16 intervals + cooldown = 18)
    assert_eq!(parsed.segments.len(), original.segments.len());
    assert_eq!(parsed.segments.len(), 18);

    // Verify total duration
    assert_eq!(parsed.total_duration_seconds, 1080);
}

#[test]
fn test_mrc_round_trip_warmup_with_range() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Warmup,
        duration_seconds: 600,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(40),
            PowerTarget::percent_ftp(70),
        ),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Warmup Ramp".to_string(), segments);

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::Warmup);
    assert_eq!(parsed.segments[0].duration_seconds, 600);

    match &parsed.segments[0].power_target {
        PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
            (
                PowerTarget::PercentFtp { percent: start_pct },
                PowerTarget::PercentFtp { percent: end_pct },
            ) => {
                assert_eq!(*start_pct, 40);
                assert_eq!(*end_pct, 70);
            }
            _ => panic!("Expected PercentFtp in range"),
        },
        _ => panic!("Expected Range power target"),
    }
}

#[test]
fn test_mrc_round_trip_cooldown_with_range() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Cooldown,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(70),
            PowerTarget::percent_ftp(40),
        ),
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Cooldown Ramp".to_string(), segments);

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    assert_eq!(parsed.segments[0].segment_type, SegmentType::Cooldown);
    assert_eq!(parsed.segments[0].duration_seconds, 300);

    match &parsed.segments[0].power_target {
        PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
            (
                PowerTarget::PercentFtp { percent: start_pct },
                PowerTarget::PercentFtp { percent: end_pct },
            ) => {
                assert_eq!(*start_pct, 70);
                assert_eq!(*end_pct, 40);
            }
            _ => panic!("Expected PercentFtp in range"),
        },
        _ => panic!("Expected Range power target"),
    }
}

#[test]
fn test_mrc_round_trip_with_text_events() {
    let segments = vec![
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(60),
            ),
            cadence_target: None,
            text_event: Some("Warm up!".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 600,
            power_target: PowerTarget::percent_ftp(88),
            cadence_target: None,
            text_event: Some("Main effort".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: Some("Cool down".to_string()),
        },
    ];
    let original = Workout::new("Text Event Test".to_string(), segments);

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    // Verify text events preserved
    assert_eq!(parsed.segments[0].text_event.as_deref(), Some("Warm up!"));
    assert_eq!(
        parsed.segments[1].text_event.as_deref(),
        Some("Main effort")
    );
    assert_eq!(parsed.segments[2].text_event.as_deref(), Some("Cool down"));
}

#[test]
fn test_mrc_round_trip_with_description() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::SteadyState,
        duration_seconds: 600,
        power_target: PowerTarget::percent_ftp(75),
        cadence_target: None,
        text_event: None,
    }];
    let mut original = Workout::new("Described Workout".to_string(), segments);
    original.description = Some("A test workout with a description".to_string());

    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    assert_eq!(parsed.name, "Described Workout");
    assert_eq!(
        parsed.description.as_deref(),
        Some("A test workout with a description")
    );
}

#[test]
fn test_mrc_round_trip_with_ftp_conversion() {
    // Test absolute power conversion with specific FTP
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::SteadyState,
        duration_seconds: 300,
        power_target: PowerTarget::absolute(225), // 225W with 300W FTP = 75%
        cadence_target: None,
        text_event: None,
    }];
    let original = Workout::new("Absolute Power Test".to_string(), segments);

    let mrc_content = export_mrc_with_ftp(&original, Some(300)).expect("Should export MRC");
    let parsed = parse_mrc(&mrc_content).expect("Should parse exported MRC");

    // 225W with 300W FTP = 75%
    match &parsed.segments[0].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 75),
        _ => panic!("Expected PercentFtp power target"),
    }
}

#[test]
fn test_mrc_export_empty_workout_error() {
    let workout = Workout::new("Empty".to_string(), vec![]);
    let result = export_mrc(&workout);
    assert!(matches!(result, Err(WorkoutExportError::EmptyWorkout)));
}

// =============================================================================
// Cross-Format Tests
// =============================================================================

#[test]
fn test_workout_structure_preserved_across_formats() {
    // Create a workout and verify both formats preserve essential structure
    let segments = vec![
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(60),
            ),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 600,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        },
    ];
    let original = Workout::new("Cross Format Test".to_string(), segments);

    // Export to both formats
    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let mrc_content = export_mrc(&original).expect("Should export MRC");

    // Parse back both formats
    let zwo_parsed = parse_zwo(&zwo_content).expect("Should parse ZWO");
    let mrc_parsed = parse_mrc(&mrc_content).expect("Should parse MRC");

    // Both should have same segment count
    assert_eq!(zwo_parsed.segments.len(), mrc_parsed.segments.len());

    // Both should have same total duration
    assert_eq!(
        zwo_parsed.total_duration_seconds,
        mrc_parsed.total_duration_seconds
    );

    // Both should preserve workout name
    assert_eq!(zwo_parsed.name, mrc_parsed.name);
}

#[test]
fn test_long_endurance_workout_round_trip() {
    // Test a long workout (2 hours)
    let segments = vec![
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 600, // 10 min
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(65),
            ),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 6000, // 100 min at Z2
            power_target: PowerTarget::percent_ftp(65),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 600, // 10 min
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(65),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        },
    ];
    let original = Workout::new("Long Endurance Ride".to_string(), segments);

    // ZWO round-trip
    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let zwo_parsed = parse_zwo(&zwo_content).expect("Should parse ZWO");
    assert_eq!(zwo_parsed.total_duration_seconds, 7200); // 2 hours

    // MRC round-trip
    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let mrc_parsed = parse_mrc(&mrc_content).expect("Should parse MRC");
    assert_eq!(mrc_parsed.total_duration_seconds, 7200); // 2 hours
}

#[test]
fn test_many_segments_round_trip() {
    // Test workout with many segments (50+ segments)
    let mut segments = Vec::new();

    // Warmup
    segments.push(WorkoutSegment {
        segment_type: SegmentType::Warmup,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(40),
            PowerTarget::percent_ftp(60),
        ),
        cadence_target: None,
        text_event: None,
    });

    // 50 x 30s intervals
    for i in 0..50 {
        segments.push(WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(50 + (i % 50) as u8),
            cadence_target: None,
            text_event: None,
        });
    }

    // Cooldown
    segments.push(WorkoutSegment {
        segment_type: SegmentType::Cooldown,
        duration_seconds: 300,
        power_target: PowerTarget::range(
            PowerTarget::percent_ftp(60),
            PowerTarget::percent_ftp(40),
        ),
        cadence_target: None,
        text_event: None,
    });

    let original = Workout::new("Many Segments".to_string(), segments);

    // ZWO round-trip
    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let zwo_parsed = parse_zwo(&zwo_content).expect("Should parse ZWO");
    assert_eq!(zwo_parsed.segments.len(), 52); // 1 warmup + 50 intervals + 1 cooldown

    // MRC round-trip
    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let mrc_parsed = parse_mrc(&mrc_content).expect("Should parse MRC");
    assert_eq!(mrc_parsed.segments.len(), 52);
}

#[test]
fn test_extreme_power_values_round_trip() {
    let segments = vec![
        // Very low power (recovery)
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(25),
            cadence_target: None,
            text_event: None,
        },
        // Very high power (sprint)
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(200),
            cadence_target: None,
            text_event: None,
        },
    ];
    let original = Workout::new("Extreme Power Test".to_string(), segments);

    // ZWO round-trip
    let zwo_content = export_zwo(&original).expect("Should export ZWO");
    let zwo_parsed = parse_zwo(&zwo_content).expect("Should parse ZWO");

    match &zwo_parsed.segments[0].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 25),
        _ => panic!("Expected PercentFtp"),
    }
    match &zwo_parsed.segments[1].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 200),
        _ => panic!("Expected PercentFtp"),
    }

    // MRC round-trip
    let mrc_content = export_mrc(&original).expect("Should export MRC");
    let mrc_parsed = parse_mrc(&mrc_content).expect("Should parse MRC");

    match &mrc_parsed.segments[0].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 25),
        _ => panic!("Expected PercentFtp"),
    }
    match &mrc_parsed.segments[1].power_target {
        PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 200),
        _ => panic!("Expected PercentFtp"),
    }
}
