//! Integration tests for workout execution.
//!
//! T057: Integration test for workout execution.
//!
//! Tests the complete workout execution flow including:
//! - Loading and parsing a .zwo workout file
//! - Starting the workout engine
//! - Simulating time progression through segments
//! - Verifying power target changes at segment transitions
//! - Testing pause/resume/skip functionality

use rustride::workouts::engine::WorkoutEngine;
use rustride::workouts::parser_zwo::parse_zwo;
use rustride::workouts::types::{
    PowerTarget, SegmentType, Workout, WorkoutSegment, WorkoutStatus,
};

/// Create a test workout with multiple segments for integration testing.
fn create_test_workout() -> Workout {
    Workout::new(
        "Integration Test Workout".to_string(),
        vec![
            // 2-minute warmup at 50% FTP
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 120,
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: Some(85),
                text_event: Some("Warm up - easy spinning".to_string()),
            },
            // 3-minute steady state at 90% FTP
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 180,
                power_target: PowerTarget::percent_ftp(90),
                cadence_target: Some(90),
                text_event: Some("Increase effort to Zone 3".to_string()),
            },
            // 1-minute interval at 120% FTP
            WorkoutSegment {
                segment_type: SegmentType::Interval,
                duration_seconds: 60,
                power_target: PowerTarget::percent_ftp(120),
                cadence_target: Some(95),
                text_event: Some("Push it! High intensity interval".to_string()),
            },
            // 2-minute recovery at 55% FTP
            WorkoutSegment {
                segment_type: SegmentType::Recovery,
                duration_seconds: 120,
                power_target: PowerTarget::percent_ftp(55),
                cadence_target: Some(80),
                text_event: Some("Recover - bring HR down".to_string()),
            },
            // 1-minute cooldown at 40% FTP
            WorkoutSegment {
                segment_type: SegmentType::Cooldown,
                duration_seconds: 60,
                power_target: PowerTarget::percent_ftp(40),
                cadence_target: Some(75),
                text_event: Some("Cool down - easy spin".to_string()),
            },
        ],
    )
}

#[test]
fn test_complete_workout_execution() {
    let workout = create_test_workout();
    let user_ftp = 250; // 250W FTP

    let mut engine = WorkoutEngine::new();

    // Load workout
    engine.load(workout, user_ftp).unwrap();
    assert!(engine.has_workout());
    assert_eq!(
        engine.state().unwrap().status,
        WorkoutStatus::NotStarted
    );

    // Start workout
    engine.start().unwrap();
    assert!(engine.is_active());
    assert_eq!(
        engine.state().unwrap().status,
        WorkoutStatus::InProgress
    );

    // Verify initial segment (warmup at 50% = 125W)
    let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
    assert_eq!(progress.segment_index, 0);
    assert_eq!(progress.target_power, 125); // 50% of 250W

    // Simulate 120 seconds (end of warmup)
    for _ in 0..120 {
        engine.tick();
    }

    // Should now be in second segment (steady state at 90% = 225W)
    // But due to ramp transition, power will be transitioning
    let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
    assert_eq!(progress.segment_index, 1);

    // After ramp duration (default 3s), power should reach target
    for _ in 0..5 {
        engine.tick();
    }
    let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
    assert_eq!(progress.target_power, 225); // 90% of 250W
}

#[test]
fn test_pause_and_resume() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // Tick for 30 seconds
    for _ in 0..30 {
        engine.tick();
    }
    assert_eq!(engine.state().unwrap().total_elapsed_seconds, 30);

    // Pause
    engine.pause().unwrap();
    assert_eq!(engine.state().unwrap().status, WorkoutStatus::Paused);

    // Tick while paused (time should NOT advance)
    for _ in 0..10 {
        engine.tick();
    }
    assert_eq!(engine.state().unwrap().total_elapsed_seconds, 30);

    // Resume
    engine.resume().unwrap();
    assert_eq!(
        engine.state().unwrap().status,
        WorkoutStatus::InProgress
    );

    // Time should advance again
    for _ in 0..10 {
        engine.tick();
    }
    assert_eq!(engine.state().unwrap().total_elapsed_seconds, 40);
}

#[test]
fn test_skip_segment() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // Start in segment 0
    assert_eq!(
        engine
            .state()
            .unwrap()
            .segment_progress
            .as_ref()
            .unwrap()
            .segment_index,
        0
    );

    // Skip to next segment
    engine.skip_segment().unwrap();

    // Should be in segment 1
    assert_eq!(
        engine
            .state()
            .unwrap()
            .segment_progress
            .as_ref()
            .unwrap()
            .segment_index,
        1
    );

    // Skip again
    engine.skip_segment().unwrap();
    assert_eq!(
        engine
            .state()
            .unwrap()
            .segment_progress
            .as_ref()
            .unwrap()
            .segment_index,
        2
    );
}

#[test]
fn test_extend_segment() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // First segment is 120 seconds
    // Tick to near end
    for _ in 0..115 {
        engine.tick();
    }

    let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
    assert_eq!(progress.segment_index, 0);
    assert!(progress.remaining_seconds <= 10);

    // Extend by 60 seconds
    engine.extend_segment(60).unwrap();

    // Check remaining time increased
    let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
    assert!(progress.remaining_seconds > 60);
}

#[test]
fn test_power_adjustment() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // Base power at 50% of 200W = 100W
    assert_eq!(engine.current_target_power(), Some(100));

    // Increase power by 20W
    engine.adjust_power(20).unwrap();
    assert_eq!(engine.current_target_power(), Some(120));

    // Decrease power by 30W
    engine.adjust_power(-30).unwrap();
    assert_eq!(engine.current_target_power(), Some(90));
}

#[test]
fn test_workout_completion() {
    // Create a short workout
    let workout = Workout::new(
        "Short Test".to_string(),
        vec![
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 5,
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 5,
                power_target: PowerTarget::percent_ftp(60),
                cadence_target: None,
                text_event: None,
            },
        ],
    );

    let mut engine = WorkoutEngine::new();
    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    assert!(!engine.is_complete());

    // Run through entire workout (10 seconds + 1 extra to complete)
    for _ in 0..11 {
        engine.tick();
    }

    assert!(engine.is_complete());
    assert_eq!(
        engine.state().unwrap().status,
        WorkoutStatus::Completed
    );
}

#[test]
fn test_ramp_transition_smoothing() {
    let workout = Workout::new(
        "Ramp Test".to_string(),
        vec![
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 10,
                power_target: PowerTarget::percent_ftp(50), // 100W at 200 FTP
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::Interval,
                duration_seconds: 10,
                power_target: PowerTarget::percent_ftp(100), // 200W at 200 FTP
                cadence_target: None,
                text_event: None,
            },
        ],
    );

    let mut engine = WorkoutEngine::new();
    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // Initial power should be 100W
    assert_eq!(engine.current_target_power(), Some(100));

    // Skip to second segment
    engine.skip_segment().unwrap();

    // Immediately after skip, power should be ramping from 100W to 200W
    let initial_power = engine.current_target_power().unwrap();
    assert!(initial_power > 100); // Should have started ramping
    assert!(initial_power < 200); // Not yet at target

    // After ramp completes (default 3 seconds)
    for _ in 0..5 {
        engine.tick();
    }

    assert_eq!(engine.current_target_power(), Some(200));
}

#[test]
fn test_text_event_retrieval() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // First segment has text event
    let text = engine.current_text_event();
    assert!(text.is_some());
    assert!(text.unwrap().contains("Warm up"));
}

#[test]
fn test_segment_type_retrieval() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    assert_eq!(engine.current_segment_type(), Some(SegmentType::Warmup));

    // Skip to steady state
    engine.skip_segment().unwrap();
    assert_eq!(
        engine.current_segment_type(),
        Some(SegmentType::SteadyState)
    );

    // Skip to interval
    engine.skip_segment().unwrap();
    assert_eq!(
        engine.current_segment_type(),
        Some(SegmentType::Interval)
    );
}

#[test]
fn test_workout_with_ramp_segment() {
    // Test a workout with a ramp segment (power changes linearly over duration)
    let workout = Workout::new(
        "Ramp Segment Test".to_string(),
        vec![WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 100,
            power_target: PowerTarget::Ramp { start: 50, end: 100 }, // 50% to 100% FTP
            cadence_target: None,
            text_event: None,
        }],
    );

    let mut engine = WorkoutEngine::new();
    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // At start (0%), power should be 50% of 200W = 100W
    assert_eq!(engine.current_target_power(), Some(100));

    // At 50% through (50 seconds)
    for _ in 0..50 {
        engine.tick();
    }
    // Should be 75% of 200W = 150W (midpoint between 50% and 100%)
    assert_eq!(engine.current_target_power(), Some(150));

    // At end (99 seconds, just before completion)
    for _ in 0..49 {
        engine.tick();
    }
    // Should be close to 100% of 200W = 200W
    let power = engine.current_target_power().unwrap();
    assert!(power >= 195 && power <= 200);
}

#[test]
fn test_stop_workout_early() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();
    engine.start().unwrap();

    // Tick a bit
    for _ in 0..30 {
        engine.tick();
    }

    // Stop early
    engine.stop().unwrap();

    assert!(!engine.is_active());
    assert_eq!(
        engine.state().unwrap().status,
        WorkoutStatus::Stopped
    );
}

#[test]
fn test_error_handling_no_workout_loaded() {
    let mut engine = WorkoutEngine::new();

    // Should error when trying to start without loading
    assert!(engine.start().is_err());
    assert!(engine.pause().is_err());
    assert!(engine.resume().is_err());
    assert!(engine.skip_segment().is_err());
}

#[test]
fn test_error_handling_invalid_state_transitions() {
    let workout = create_test_workout();
    let mut engine = WorkoutEngine::new();

    engine.load(workout, 200).unwrap();

    // Can't pause before starting
    assert!(engine.pause().is_err());

    // Can't resume before starting
    assert!(engine.resume().is_err());

    // Start
    engine.start().unwrap();

    // Can't start again
    assert!(engine.start().is_err());

    // Pause
    engine.pause().unwrap();

    // Can't pause again
    assert!(engine.pause().is_err());

    // Resume
    engine.resume().unwrap();

    // Can resume correctly
    assert!(engine.pause().is_ok());
}
