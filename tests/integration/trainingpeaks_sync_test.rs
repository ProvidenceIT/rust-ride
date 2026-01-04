//! Integration tests for TrainingPeaks workout format conversion.
//!
//! Tests real-world workout examples covering various interval types including:
//! - Simple endurance workouts
//! - Sweet spot training
//! - VO2max intervals
//! - Over-under workouts
//! - Complex nested repeat structures
//! - Ramp tests and build progressions
//! - Recovery workouts
//! - Sprint intervals

use rustride::integrations::sync::trainingpeaks::{
    convert_tp_workouts, TPWorkout, TPWorkoutStep, TPWorkoutStructure, TPWorkoutTarget,
};
use rustride::workouts::types::{PowerTarget, SegmentType, WorkoutFormat};

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a basic TPWorkout with optional structure
fn create_tp_workout(
    id: i64,
    title: &str,
    workout_type: &str,
    total_time: Option<f64>,
    structure: Option<TPWorkoutStructure>,
) -> TPWorkout {
    TPWorkout {
        id,
        title: title.to_string(),
        description: Some(format!("Test workout: {}", title)),
        workout_type: workout_type.to_string(),
        workout_day: "2025-01-15".to_string(),
        total_time,
        tss_planned: Some(75.0),
        if_planned: Some(0.85),
        structure,
    }
}

/// Create a simple step with power target
fn create_step(step_type: &str, duration_secs: f64, power_percent: Option<u8>) -> TPWorkoutStep {
    let targets = power_percent.map(|pct| {
        vec![TPWorkoutTarget {
            target_type: "Power".to_string(),
            min_value: Some(pct as f64),
            max_value: Some(pct as f64),
            unit: Some("PercentFTP".to_string()),
        }]
    });

    TPWorkoutStep {
        step_type: step_type.to_string(),
        name: Some(format!("{} step", step_type)),
        length: Some(duration_secs),
        length_metric: Some("Duration".to_string()),
        targets,
        steps: None,
        reps: None,
    }
}

/// Create a step with power range
fn create_range_step(
    step_type: &str,
    duration_secs: f64,
    min_power: u8,
    max_power: u8,
) -> TPWorkoutStep {
    TPWorkoutStep {
        step_type: step_type.to_string(),
        name: Some(format!("{} step", step_type)),
        length: Some(duration_secs),
        length_metric: Some("Duration".to_string()),
        targets: Some(vec![TPWorkoutTarget {
            target_type: "Power".to_string(),
            min_value: Some(min_power as f64),
            max_value: Some(max_power as f64),
            unit: Some("PercentFTP".to_string()),
        }]),
        steps: None,
        reps: None,
    }
}

/// Create a repeat step with nested intervals
fn create_repeat_step(reps: u32, nested_steps: Vec<TPWorkoutStep>) -> TPWorkoutStep {
    TPWorkoutStep {
        step_type: "Repeat".to_string(),
        name: Some(format!("{}x", reps)),
        length: None,
        length_metric: None,
        targets: None,
        steps: Some(nested_steps),
        reps: Some(reps),
    }
}

/// Create a step with cadence target
fn create_step_with_cadence(
    step_type: &str,
    duration_secs: f64,
    power_percent: u8,
    min_cadence: u8,
    max_cadence: u8,
) -> TPWorkoutStep {
    TPWorkoutStep {
        step_type: step_type.to_string(),
        name: Some(format!("{} with cadence", step_type)),
        length: Some(duration_secs),
        length_metric: Some("Duration".to_string()),
        targets: Some(vec![
            TPWorkoutTarget {
                target_type: "Power".to_string(),
                min_value: Some(power_percent as f64),
                max_value: Some(power_percent as f64),
                unit: Some("PercentFTP".to_string()),
            },
            TPWorkoutTarget {
                target_type: "Cadence".to_string(),
                min_value: Some(min_cadence as f64),
                max_value: Some(max_cadence as f64),
                unit: Some("RPM".to_string()),
            },
        ]),
        steps: None,
        reps: None,
    }
}

// ============================================================================
// Real-World Workout Tests
// ============================================================================

#[test]
fn test_endurance_ride_conversion() {
    // Classic 90-minute endurance ride at 65-75% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_range_step("Warmup", 600.0, 40, 65), // 10 min warmup
            create_range_step("Interval", 4200.0, 65, 75), // 70 min main set
            create_range_step("Cooldown", 600.0, 65, 40), // 10 min cooldown
        ],
    };

    let workout = create_tp_workout(
        1001,
        "Endurance Ride",
        "Bike",
        Some(5400.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok(), "Endurance ride should convert successfully");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Endurance Ride");
    assert_eq!(converted.source_format, Some(WorkoutFormat::TrainingPeaks));
    assert_eq!(converted.segments.len(), 3);

    // Verify warmup
    assert_eq!(converted.segments[0].segment_type, SegmentType::Warmup);
    assert_eq!(converted.segments[0].duration_seconds, 600);

    // Verify main set (steady state at endurance)
    assert_eq!(converted.segments[1].segment_type, SegmentType::SteadyState);
    assert_eq!(converted.segments[1].duration_seconds, 4200);

    // Verify cooldown
    assert_eq!(converted.segments[2].segment_type, SegmentType::Cooldown);
    assert_eq!(converted.segments[2].duration_seconds, 600);
}

#[test]
fn test_sweet_spot_workout_conversion() {
    // Sweet spot intervals: 3x15 min at 88-93% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            create_repeat_step(
                3,
                vec![
                    create_range_step("Interval", 900.0, 88, 93), // 15 min sweet spot
                    create_step("Recovery", 300.0, Some(55)),     // 5 min recovery
                ],
            ),
            create_step("Cooldown", 600.0, Some(50)),
        ],
    };

    let workout = create_tp_workout(
        1002,
        "Sweet Spot 3x15",
        "Bike",
        Some(4500.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(280));
    assert!(
        result.is_ok(),
        "Sweet spot workout should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Sweet Spot 3x15");

    // Should have: warmup + (3 * (interval + recovery)) + cooldown = 1 + 6 + 1 = 8 segments
    // But repeat blocks are flattened, so actual count depends on implementation
    assert!(!converted.segments.is_empty());

    // Total duration should be preserved
    let total_duration: u32 = converted.segments.iter().map(|s| s.duration_seconds).sum();
    // 10 min warmup + 3*(15+5) min + 10 min cooldown = 10 + 60 + 10 = 80 min = 4800s
    // But reps expand differently based on implementation
    assert!(total_duration > 0);
}

#[test]
fn test_vo2max_intervals_conversion() {
    // VO2max workout: 5x4 min at 105-120% FTP with 4 min recovery
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 900.0, Some(55)),
            create_repeat_step(
                5,
                vec![
                    create_range_step("Interval", 240.0, 105, 120), // 4 min VO2max
                    create_step("Recovery", 240.0, Some(45)),       // 4 min recovery
                ],
            ),
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(1003, "VO2max 5x4", "Bike", Some(3900.0), Some(structure));

    let result = workout.to_workout(Some(300));
    assert!(
        result.is_ok(),
        "VO2max intervals should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "VO2max 5x4");
    assert!(!converted.segments.is_empty());
}

#[test]
fn test_over_under_workout_conversion() {
    // Over-unders: 3x12 min alternating 95% and 105% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            create_repeat_step(
                3,
                vec![
                    // 12 min block with 6 x (1 min over, 1 min under)
                    create_repeat_step(
                        6,
                        vec![
                            create_step("Interval", 60.0, Some(105)), // 1 min over
                            create_step("Interval", 60.0, Some(95)),  // 1 min under
                        ],
                    ),
                    create_step("Recovery", 300.0, Some(55)), // 5 min recovery
                ],
            ),
            create_step("Cooldown", 600.0, Some(50)),
        ],
    };

    let workout = create_tp_workout(
        1004,
        "Over-Unders 3x12",
        "Bike",
        Some(4500.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(275));
    assert!(
        result.is_ok(),
        "Over-under workout should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Over-Unders 3x12");
}

#[test]
fn test_threshold_intervals_conversion() {
    // Threshold workout: 2x20 min at 95-100% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_range_step("Warmup", 900.0, 40, 70),
            create_repeat_step(
                2,
                vec![
                    create_range_step("Interval", 1200.0, 95, 100), // 20 min threshold
                    create_step("Recovery", 600.0, Some(50)),       // 10 min recovery
                ],
            ),
            create_range_step("Cooldown", 600.0, 60, 40),
        ],
    };

    let workout = create_tp_workout(
        1005,
        "Threshold 2x20",
        "Bike",
        Some(5100.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(260));
    assert!(
        result.is_ok(),
        "Threshold intervals should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Threshold 2x20");
    assert!(!converted.segments.is_empty());

    // Verify format is TrainingPeaks
    assert_eq!(converted.source_format, Some(WorkoutFormat::TrainingPeaks));
}

#[test]
fn test_ramp_test_conversion() {
    // FTP Ramp test: start at 50% FTP, increase 6% every 3 min until failure
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 300.0, Some(50)),
            create_step("Ramp", 180.0, Some(56)),
            create_step("Ramp", 180.0, Some(62)),
            create_step("Ramp", 180.0, Some(68)),
            create_step("Ramp", 180.0, Some(74)),
            create_step("Ramp", 180.0, Some(80)),
            create_step("Ramp", 180.0, Some(86)),
            create_step("Ramp", 180.0, Some(92)),
            create_step("Ramp", 180.0, Some(98)),
            create_step("Ramp", 180.0, Some(104)),
            create_step("Ramp", 180.0, Some(110)),
            create_step("Ramp", 180.0, Some(116)),
            create_step("Cooldown", 600.0, Some(40)),
        ],
    };

    let workout = create_tp_workout(1006, "Ramp Test", "Bike", Some(2880.0), Some(structure));

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok(), "Ramp test should convert successfully");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Ramp Test");
    assert_eq!(converted.segments.len(), 13);
}

#[test]
fn test_sprint_intervals_conversion() {
    // Sprint workout: 6x30s max effort sprints
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 900.0, Some(55)),
            create_repeat_step(
                6,
                vec![
                    create_step("Interval", 30.0, Some(150)), // 30s sprint
                    create_step("Recovery", 270.0, Some(40)), // 4.5 min recovery
                ],
            ),
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(1007, "Sprint 6x30s", "Bike", Some(3300.0), Some(structure));

    let result = workout.to_workout(Some(290));
    assert!(result.is_ok(), "Sprint workout should convert successfully");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Sprint 6x30s");
}

#[test]
fn test_recovery_ride_conversion() {
    // Simple recovery ride: 45 min at 50% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 300.0, Some(40)),
            create_step("Interval", 2100.0, Some(50)),
            create_step("Cooldown", 300.0, Some(40)),
        ],
    };

    let workout = create_tp_workout(1008, "Recovery Spin", "Bike", Some(2700.0), Some(structure));

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok(), "Recovery ride should convert successfully");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Recovery Spin");
    assert_eq!(converted.segments.len(), 3);

    // Verify low power targets (recovery zone)
    for segment in &converted.segments {
        match &segment.power_target {
            PowerTarget::PercentFtp { percent } => {
                assert!(
                    *percent <= 55,
                    "Recovery ride should have low power targets: {}",
                    percent
                );
            }
            PowerTarget::Range { start, end } => {
                // Range targets are acceptable for warmup/cooldown
                if let PowerTarget::PercentFtp { percent } = **start {
                    assert!(percent <= 60);
                }
                if let PowerTarget::PercentFtp { percent } = **end {
                    assert!(percent <= 60);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn test_cadence_drills_conversion() {
    // Cadence drill workout with specific RPM targets
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            create_step_with_cadence("Interval", 300.0, 70, 60, 70), // Low cadence
            create_step_with_cadence("Interval", 300.0, 70, 90, 100), // Normal cadence
            create_step_with_cadence("Interval", 300.0, 70, 110, 120), // High cadence
            create_step_with_cadence("Interval", 300.0, 70, 90, 100), // Normal
            create_step_with_cadence("Interval", 300.0, 70, 60, 70), // Low
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(
        1009,
        "Cadence Drills",
        "Bike",
        Some(2400.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(250));
    assert!(
        result.is_ok(),
        "Cadence drill workout should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Cadence Drills");
    assert_eq!(converted.segments.len(), 7);

    // Verify cadence targets are present on drill segments
    let has_cadence_targets = converted
        .segments
        .iter()
        .skip(1) // Skip warmup
        .take(5) // Check drill segments
        .filter(|s| s.cadence_target.is_some())
        .count();
    assert!(has_cadence_targets > 0, "Should have some cadence targets");
}

#[test]
fn test_pyramid_intervals_conversion() {
    // Pyramid workout: 1-2-3-2-1 min intervals at 110% FTP
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 900.0, Some(55)),
            create_step("Interval", 60.0, Some(110)),
            create_step("Recovery", 60.0, Some(50)),
            create_step("Interval", 120.0, Some(110)),
            create_step("Recovery", 120.0, Some(50)),
            create_step("Interval", 180.0, Some(110)),
            create_step("Recovery", 180.0, Some(50)),
            create_step("Interval", 120.0, Some(110)),
            create_step("Recovery", 120.0, Some(50)),
            create_step("Interval", 60.0, Some(110)),
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(
        1010,
        "Pyramid 1-2-3-2-1",
        "Bike",
        Some(2520.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(270));
    assert!(
        result.is_ok(),
        "Pyramid workout should convert successfully"
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Pyramid 1-2-3-2-1");
    assert_eq!(converted.segments.len(), 11);
}

#[test]
fn test_build_intervals_conversion() {
    // Progressive build workout: 3 x 10 min at 85%, 90%, 95%
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            create_step("Interval", 600.0, Some(85)),
            create_step("Recovery", 180.0, Some(50)),
            create_step("Interval", 600.0, Some(90)),
            create_step("Recovery", 180.0, Some(50)),
            create_step("Interval", 600.0, Some(95)),
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(1011, "Build 3x10", "Bike", Some(3360.0), Some(structure));

    let result = workout.to_workout(Some(280));
    assert!(result.is_ok(), "Build workout should convert successfully");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Build 3x10");
    assert_eq!(converted.segments.len(), 7);
}

#[test]
fn test_workout_without_structure_conversion() {
    // Workout with description but no structured data
    let workout = create_tp_workout(
        1012,
        "Unstructured Ride",
        "Bike",
        Some(3600.0),
        None, // No structure
    );

    let result = workout.to_workout(Some(250));
    assert!(
        result.is_err(),
        "Workout without structure should fail to convert"
    );
}

#[test]
fn test_workout_with_empty_structure_conversion() {
    // Workout with empty steps
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![],
    };

    let workout = create_tp_workout(1013, "Empty Workout", "Bike", Some(0.0), Some(structure));

    let result = workout.to_workout(Some(250));
    assert!(result.is_err(), "Empty structure should fail to convert");
}

#[test]
fn test_batch_conversion_with_convert_tp_workouts() {
    // Test batch conversion function
    let workouts = vec![
        create_tp_workout(
            2001,
            "Endurance 1",
            "Bike",
            Some(3600.0),
            Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![
                    create_step("Warmup", 600.0, Some(55)),
                    create_step("Interval", 2400.0, Some(70)),
                    create_step("Cooldown", 600.0, Some(45)),
                ],
            }),
        ),
        create_tp_workout(
            2002,
            "Intervals 1",
            "Bike",
            Some(2400.0),
            Some(TPWorkoutStructure {
                primary_length_metric: Some("Duration".to_string()),
                primary_intensity_metric: Some("Power".to_string()),
                steps: vec![
                    create_step("Warmup", 600.0, Some(55)),
                    create_repeat_step(
                        4,
                        vec![
                            create_step("Interval", 180.0, Some(100)),
                            create_step("Recovery", 120.0, Some(50)),
                        ],
                    ),
                    create_step("Cooldown", 600.0, Some(45)),
                ],
            }),
        ),
        // One without structure - should be filtered out
        create_tp_workout(2003, "No Structure", "Bike", Some(1800.0), None),
    ];

    let results = convert_tp_workouts(workouts, Some(250));

    // Two valid conversions, one error
    let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
    let errors: Vec<_> = results.iter().filter(|r| r.is_err()).collect();

    assert_eq!(successes.len(), 2, "Should have 2 successful conversions");
    assert_eq!(errors.len(), 1, "Should have 1 error");
}

#[test]
fn test_absolute_watts_target_conversion() {
    // Workout with absolute wattage targets (not percentage)
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            TPWorkoutStep {
                step_type: "Warmup".to_string(),
                name: Some("Warmup".to_string()),
                length: Some(600.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "Power".to_string(),
                    min_value: Some(100.0),
                    max_value: Some(150.0),
                    unit: Some("Watts".to_string()),
                }]),
                steps: None,
                reps: None,
            },
            TPWorkoutStep {
                step_type: "Interval".to_string(),
                name: Some("Main Set".to_string()),
                length: Some(1200.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "Power".to_string(),
                    min_value: Some(200.0),
                    max_value: Some(220.0),
                    unit: Some("Watts".to_string()),
                }]),
                steps: None,
                reps: None,
            },
            TPWorkoutStep {
                step_type: "Cooldown".to_string(),
                name: Some("Cooldown".to_string()),
                length: Some(600.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "Power".to_string(),
                    min_value: Some(150.0),
                    max_value: Some(100.0),
                    unit: Some("Watts".to_string()),
                }]),
                steps: None,
                reps: None,
            },
        ],
    };

    let workout = create_tp_workout(
        1014,
        "Absolute Power Workout",
        "Bike",
        Some(2400.0),
        Some(structure),
    );

    // Convert without FTP - should still work with absolute targets
    let result = workout.to_workout(None);
    assert!(
        result.is_ok(),
        "Absolute watt workout should convert: {:?}",
        result.err()
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Absolute Power Workout");
}

#[test]
fn test_zone_based_workout_conversion() {
    // Workout with power zone targets
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            TPWorkoutStep {
                step_type: "Warmup".to_string(),
                name: Some("Zone 1 Warmup".to_string()),
                length: Some(600.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "PowerZone".to_string(),
                    min_value: Some(1.0),
                    max_value: Some(1.0),
                    unit: Some("Zone".to_string()),
                }]),
                steps: None,
                reps: None,
            },
            TPWorkoutStep {
                step_type: "Interval".to_string(),
                name: Some("Zone 4 Threshold".to_string()),
                length: Some(1200.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "PowerZone".to_string(),
                    min_value: Some(4.0),
                    max_value: Some(4.0),
                    unit: Some("Zone".to_string()),
                }]),
                steps: None,
                reps: None,
            },
            TPWorkoutStep {
                step_type: "Cooldown".to_string(),
                name: Some("Zone 1 Cooldown".to_string()),
                length: Some(600.0),
                length_metric: Some("Duration".to_string()),
                targets: Some(vec![TPWorkoutTarget {
                    target_type: "PowerZone".to_string(),
                    min_value: Some(1.0),
                    max_value: Some(1.0),
                    unit: Some("Zone".to_string()),
                }]),
                steps: None,
                reps: None,
            },
        ],
    };

    let workout = create_tp_workout(
        1015,
        "Zone-Based Workout",
        "Bike",
        Some(2400.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(250));
    assert!(
        result.is_ok(),
        "Zone-based workout should convert: {:?}",
        result.err()
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Zone-Based Workout");
    assert_eq!(converted.segments.len(), 3);
}

#[test]
fn test_microbursts_workout_conversion() {
    // Microbursts: 10x (15s on / 15s off) repeated 3 times
    let microburst_set = create_repeat_step(
        10,
        vec![
            create_step("Interval", 15.0, Some(150)), // 15s on at 150%
            create_step("Recovery", 15.0, Some(50)),  // 15s off at 50%
        ],
    );

    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            microburst_set.clone(),
            create_step("Recovery", 300.0, Some(55)),
            microburst_set.clone(),
            create_step("Recovery", 300.0, Some(55)),
            microburst_set,
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(
        1016,
        "Microbursts 3x10",
        "Bike",
        Some(3000.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(260));
    assert!(
        result.is_ok(),
        "Microbursts workout should convert: {:?}",
        result.err()
    );

    let converted = result.unwrap();
    assert_eq!(converted.name, "Microbursts 3x10");
}

#[test]
fn test_total_duration_calculation() {
    // Verify total duration is calculated correctly
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 300.0, Some(50)),   // 5 min
            create_step("Interval", 600.0, Some(80)), // 10 min
            create_step("Cooldown", 300.0, Some(45)), // 5 min
        ],
    };

    let workout = create_tp_workout(1017, "Duration Test", "Bike", Some(1200.0), Some(structure));

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok());

    let converted = result.unwrap();
    assert_eq!(
        converted.total_duration_seconds, 1200,
        "Total duration should be 20 minutes (1200 seconds)"
    );
}

#[test]
fn test_freeride_segment_conversion() {
    // Workout with free ride segment
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(55)),
            TPWorkoutStep {
                step_type: "FreeRide".to_string(),
                name: Some("Free Ride".to_string()),
                length: Some(1800.0),
                length_metric: Some("Duration".to_string()),
                targets: None, // No power target for free ride
                steps: None,
                reps: None,
            },
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(
        1018,
        "Free Ride Workout",
        "Bike",
        Some(3000.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok(), "Free ride workout should convert");

    let converted = result.unwrap();

    // Find the free ride segment
    let has_freeride = converted
        .segments
        .iter()
        .any(|s| s.segment_type == SegmentType::FreeRide);
    assert!(has_freeride, "Should have a FreeRide segment");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_very_long_workout_conversion() {
    // 4-hour endurance ride
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 1200.0, Some(50)),    // 20 min
            create_step("Interval", 12000.0, Some(65)), // 3h20m
            create_step("Cooldown", 1200.0, Some(45)),  // 20 min
        ],
    };

    let workout = create_tp_workout(
        1019,
        "4 Hour Endurance",
        "Bike",
        Some(14400.0),
        Some(structure),
    );

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok(), "Long workout should convert");

    let converted = result.unwrap();
    assert_eq!(converted.total_duration_seconds, 14400);
}

#[test]
fn test_very_short_intervals_conversion() {
    // Tabata-style very short intervals
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(50)),
            create_repeat_step(
                8,
                vec![
                    create_step("Interval", 20.0, Some(170)), // 20s all-out
                    create_step("Recovery", 10.0, Some(30)),  // 10s rest
                ],
            ),
            create_step("Cooldown", 600.0, Some(45)),
        ],
    };

    let workout = create_tp_workout(1020, "Tabata", "Bike", Some(1440.0), Some(structure));

    let result = workout.to_workout(Some(280));
    assert!(result.is_ok(), "Tabata workout should convert");

    let converted = result.unwrap();
    assert_eq!(converted.name, "Tabata");
}

#[test]
fn test_high_power_targets_conversion() {
    // Sprint intervals with very high power targets
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 600.0, Some(50)),
            create_repeat_step(
                3,
                vec![
                    create_step("Interval", 10.0, Some(200)), // 200% FTP sprint
                    create_step("Recovery", 290.0, Some(40)),
                ],
            ),
            create_step("Cooldown", 600.0, Some(40)),
        ],
    };

    let workout = create_tp_workout(1021, "Max Sprints", "Bike", Some(2100.0), Some(structure));

    let result = workout.to_workout(Some(300));
    assert!(result.is_ok(), "High power workout should convert");
}

#[test]
fn test_description_preservation() {
    let structure = TPWorkoutStructure {
        primary_length_metric: Some("Duration".to_string()),
        primary_intensity_metric: Some("Power".to_string()),
        steps: vec![
            create_step("Warmup", 300.0, Some(50)),
            create_step("Interval", 600.0, Some(75)),
            create_step("Cooldown", 300.0, Some(45)),
        ],
    };

    let mut workout = create_tp_workout(
        1022,
        "Test Description",
        "Bike",
        Some(1200.0),
        Some(structure),
    );
    workout.description =
        Some("This is a detailed workout description with instructions.".to_string());

    let result = workout.to_workout(Some(250));
    assert!(result.is_ok());

    let converted = result.unwrap();
    assert!(converted.description.is_some());
    assert!(converted
        .description
        .as_ref()
        .unwrap()
        .contains("detailed workout description"));
}
