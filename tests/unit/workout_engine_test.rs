//! Unit tests for WorkoutEngine state machine.
//!
//! T056: Unit test for WorkoutEngine state machine

use rustride::workouts::engine::WorkoutEngine;
use rustride::workouts::types::{PowerTarget, SegmentType, Workout, WorkoutSegment, WorkoutStatus};

fn create_test_workout() -> Workout {
    let segments = vec![
        WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 60,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(60),
            ),
            cadence_target: None,
            text_event: Some("Let's warm up!".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 120,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        },
        WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(120),
            cadence_target: None,
            text_event: Some("Go hard!".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: Some("Recover".to_string()),
        },
        WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 60,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        },
    ];

    Workout::new("Test Workout".to_string(), segments)
}

#[test]
fn test_workout_engine_load() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout.clone(), 200).expect("Should load workout");

    let state = engine.state();
    assert!(state.is_some());
    assert_eq!(state.unwrap().status, WorkoutStatus::NotStarted);
}

#[test]
fn test_workout_engine_start() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    let state = engine.state().unwrap();
    assert_eq!(state.status, WorkoutStatus::InProgress);
    assert!(state.segment_progress.is_some());
}

#[test]
fn test_workout_engine_pause_resume() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    engine.pause().expect("Should pause workout");
    assert_eq!(engine.state().unwrap().status, WorkoutStatus::Paused);

    engine.resume().expect("Should resume workout");
    assert_eq!(engine.state().unwrap().status, WorkoutStatus::InProgress);
}

#[test]
fn test_workout_engine_tick() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Tick for 30 seconds
    for _ in 0..30 {
        engine.tick();
    }

    let state = engine.state().unwrap();
    assert_eq!(state.total_elapsed_seconds, 30);
    assert_eq!(state.segment_progress.as_ref().unwrap().elapsed_seconds, 30);
}

#[test]
fn test_workout_engine_segment_transition() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Tick past first segment (60 seconds)
    for _ in 0..61 {
        engine.tick();
    }

    let state = engine.state().unwrap();
    // Should be in second segment now
    assert_eq!(state.segment_progress.as_ref().unwrap().segment_index, 1);
    assert_eq!(state.segment_progress.as_ref().unwrap().elapsed_seconds, 1);
}

#[test]
fn test_workout_engine_completion() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();
    let total_duration = workout.total_duration_seconds;

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Tick through entire workout
    for _ in 0..=total_duration {
        engine.tick();
    }

    let state = engine.state().unwrap();
    assert_eq!(state.status, WorkoutStatus::Completed);
}

#[test]
fn test_workout_engine_stop() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    for _ in 0..30 {
        engine.tick();
    }

    engine.stop().expect("Should stop workout");

    let state = engine.state().unwrap();
    assert_eq!(state.status, WorkoutStatus::Stopped);
}

#[test]
fn test_workout_engine_skip_segment() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Tick for 30 seconds
    for _ in 0..30 {
        engine.tick();
    }

    engine.skip_segment().expect("Should skip segment");

    let state = engine.state().unwrap();
    // Should be in second segment now
    assert_eq!(state.segment_progress.as_ref().unwrap().segment_index, 1);
    assert_eq!(state.segment_progress.as_ref().unwrap().elapsed_seconds, 0);
}

#[test]
fn test_workout_engine_extend_segment() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Tick for 30 seconds
    for _ in 0..30 {
        engine.tick();
    }

    engine.extend_segment(30).expect("Should extend segment");

    let state = engine.state().unwrap();
    // Remaining should be 30 (was 30) + 30 (extended) = 60
    assert_eq!(
        state.segment_progress.as_ref().unwrap().remaining_seconds,
        60
    );
}

#[test]
fn test_workout_engine_power_offset() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Adjust power +10W
    engine.adjust_power(10).expect("Should adjust power");

    let state = engine.state().unwrap();
    assert_eq!(state.power_offset, 10);

    // Adjust power -20W (net -10W)
    engine.adjust_power(-20).expect("Should adjust power");

    let state = engine.state().unwrap();
    assert_eq!(state.power_offset, -10);
}

#[test]
fn test_workout_engine_current_target_power() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // At start of warmup, target should be 40% of 200 = 80W
    let target = engine.current_target_power();
    assert_eq!(target, Some(80));

    // Tick halfway through warmup (30s of 60s)
    for _ in 0..30 {
        engine.tick();
    }

    // Target should be halfway between 40% and 60% = 50% = 100W
    let target = engine.current_target_power();
    assert_eq!(target, Some(100));

    // Tick to end of warmup and into steady state
    for _ in 0..31 {
        engine.tick();
    }

    // Steady state at 75% of 200 = 150W
    let target = engine.current_target_power();
    assert_eq!(target, Some(150));
}

#[test]
fn test_workout_engine_power_offset_affects_target() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // Skip to steady state (75% = 150W)
    engine.skip_segment().unwrap();

    let base_target = engine.current_target_power();
    assert_eq!(base_target, Some(150));

    // Add +25W offset
    engine.adjust_power(25).unwrap();

    let adjusted_target = engine.current_target_power();
    assert_eq!(adjusted_target, Some(175)); // 150 + 25
}

#[test]
fn test_workout_engine_no_workout_loaded() {
    let mut engine = WorkoutEngine::new();

    assert!(engine.start().is_err());
    assert!(engine.pause().is_err());
    assert!(engine.resume().is_err());
    assert!(engine.stop().is_err());
    assert!(engine.tick() == ());
    assert!(engine.current_target_power().is_none());
}

#[test]
fn test_workout_engine_text_event() {
    let mut engine = WorkoutEngine::new();
    let workout = create_test_workout();

    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    let text = engine.current_text_event();
    assert_eq!(text.as_deref(), Some("Let's warm up!"));

    // Skip to steady state (no text event)
    engine.skip_segment().unwrap();
    let text = engine.current_text_event();
    assert!(text.is_none());

    // Skip to interval on (has text event)
    engine.skip_segment().unwrap();
    let text = engine.current_text_event();
    assert_eq!(text.as_deref(), Some("Go hard!"));
}

#[test]
fn test_workout_engine_ramp_power_calculation() {
    let segments = vec![WorkoutSegment {
        segment_type: SegmentType::Ramp,
        duration_seconds: 100,
        power_target: PowerTarget::range(
            PowerTarget::absolute(100),
            PowerTarget::absolute(200),
        ),
        cadence_target: None,
        text_event: None,
    }];

    let workout = Workout::new("Ramp Test".to_string(), segments);

    let mut engine = WorkoutEngine::new();
    engine.load(workout, 200).expect("Should load workout");
    engine.start().expect("Should start workout");

    // At 0%, power should be 100W
    assert_eq!(engine.current_target_power(), Some(100));

    // At 50% (50 seconds), power should be 150W
    for _ in 0..50 {
        engine.tick();
    }
    assert_eq!(engine.current_target_power(), Some(150));

    // At 100% (100 seconds), power should be 200W
    for _ in 0..50 {
        engine.tick();
    }
    // Last tick is at 100s, which is exactly at the end
    assert_eq!(engine.current_target_power(), Some(200));
}
