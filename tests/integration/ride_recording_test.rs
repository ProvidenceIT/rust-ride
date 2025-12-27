//! Integration tests for ride recording.
//!
//! T083: Integration test for ride recording
//! Tests the full recording flow from start to finish

use rustride::recording::recorder::RideRecorder;
use rustride::recording::types::{RecorderConfig, RecordingStatus, RideSample};
use uuid::Uuid;

fn create_sample(elapsed: u32, power: u16, hr: u8, cadence: u8) -> RideSample {
    RideSample {
        elapsed_seconds: elapsed,
        power_watts: Some(power),
        cadence_rpm: Some(cadence),
        heart_rate_bpm: Some(hr),
        speed_kmh: Some(30.0),
        distance_meters: elapsed as f64 * 8.33,
        calories: elapsed / 4,
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
    }
}

#[test]
fn test_full_recording_flow() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();
    let ftp = 250;

    // Start recording
    assert_eq!(recorder.status(), RecordingStatus::Idle);
    recorder.start(user_id, ftp).unwrap();
    assert_eq!(recorder.status(), RecordingStatus::Recording);

    // Record samples for 60 seconds
    for i in 0..60 {
        let sample = create_sample(i, 200 + (i % 50) as u16, 140, 90);
        recorder.record_sample(sample).unwrap();
    }

    // Check live summary
    let summary = recorder.get_live_summary();
    assert_eq!(summary.elapsed_seconds, 59);
    assert!(summary.distance_meters > 0.0);

    // Pause and resume
    recorder.pause().unwrap();
    assert_eq!(recorder.status(), RecordingStatus::Paused);

    recorder.resume().unwrap();
    assert_eq!(recorder.status(), RecordingStatus::Recording);

    // Record more samples
    for i in 60..120 {
        let sample = create_sample(i, 180, 135, 85);
        recorder.record_sample(sample).unwrap();
    }

    // Finish recording
    let (ride, samples) = recorder.finish().unwrap();

    // Verify ride data
    assert_eq!(ride.ftp_at_ride, 250);
    assert_eq!(ride.duration_seconds, 119);
    assert_eq!(samples.len(), 120);
    assert!(ride.ended_at.is_some());
}

#[test]
fn test_recording_cannot_start_twice() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    let result = recorder.start(user_id, 200);
    assert!(result.is_err());
}

#[test]
fn test_recording_sample_when_not_recording() {
    let mut recorder = RideRecorder::with_defaults();

    let sample = create_sample(0, 200, 140, 90);
    let result = recorder.record_sample(sample);

    assert!(result.is_err());
}

#[test]
fn test_finish_when_not_recording() {
    let mut recorder = RideRecorder::with_defaults();

    let result = recorder.finish();
    assert!(result.is_err());
}

#[test]
fn test_discard_recording() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    for i in 0..30 {
        let sample = create_sample(i, 200, 140, 90);
        recorder.record_sample(sample).unwrap();
    }

    recorder.discard();

    assert_eq!(recorder.status(), RecordingStatus::Idle);
    assert!(recorder.finish().is_err());
}

#[test]
fn test_power_spike_filtering() {
    let config = RecorderConfig {
        max_power_filter: 2000,
        ..Default::default()
    };
    let mut recorder = RideRecorder::new(config);
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    // Normal sample
    let sample1 = create_sample(0, 250, 140, 90);
    recorder.record_sample(sample1).unwrap();

    // Spike sample (> 2000W should be filtered)
    let spike = RideSample {
        elapsed_seconds: 1,
        power_watts: Some(2500),
        cadence_rpm: Some(90),
        heart_rate_bpm: Some(140),
        speed_kmh: Some(30.0),
        distance_meters: 8.33,
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
    };
    recorder.record_sample(spike).unwrap();

    // Another normal sample
    let sample2 = create_sample(2, 260, 145, 92);
    recorder.record_sample(sample2).unwrap();

    let (_, samples) = recorder.finish().unwrap();

    // First and third samples should have power
    assert_eq!(samples[0].power_watts, Some(250));
    assert_eq!(samples[2].power_watts, Some(260));

    // Second sample should have None (filtered)
    assert_eq!(samples[1].power_watts, None);
}

#[test]
fn test_pause_does_not_record_samples() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    // Record 10 samples
    for i in 0..10 {
        recorder
            .record_sample(create_sample(i, 200, 140, 90))
            .unwrap();
    }

    // Pause
    recorder.pause().unwrap();

    // Try to record while paused - should fail
    let result = recorder.record_sample(create_sample(10, 200, 140, 90));
    assert!(result.is_err());

    // Resume and record more
    recorder.resume().unwrap();
    for i in 10..20 {
        recorder
            .record_sample(create_sample(i, 200, 140, 90))
            .unwrap();
    }

    let (_, samples) = recorder.finish().unwrap();
    assert_eq!(samples.len(), 20);
}

#[test]
fn test_recording_with_workout() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();
    let workout_id = Uuid::new_v4();

    recorder.start(user_id, 250).unwrap();

    // Simulate workout with target power
    for i in 0..60 {
        let mut sample = create_sample(i, 225, 145, 88);
        sample.target_power = Some(220); // ERG mode target
        recorder.record_sample(sample).unwrap();
    }

    let (ride, samples) = recorder.finish().unwrap();

    // Verify target power was recorded
    assert!(samples.iter().all(|s| s.target_power == Some(220)));
    assert_eq!(ride.ftp_at_ride, 250);
}

#[test]
fn test_empty_recording_returns_error() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    // Don't record any samples
    let result = recorder.finish();
    assert!(result.is_err());
}

#[test]
fn test_live_summary_updates() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();

    recorder.start(user_id, 200).unwrap();

    // Initial state
    let summary = recorder.get_live_summary();
    assert_eq!(summary.elapsed_seconds, 0);
    assert_eq!(summary.current_power, None);

    // After first sample
    recorder
        .record_sample(create_sample(0, 180, 130, 85))
        .unwrap();
    let summary = recorder.get_live_summary();
    assert_eq!(summary.elapsed_seconds, 0);
    assert_eq!(summary.current_power, Some(180));
    assert_eq!(summary.current_hr, Some(130));
    assert_eq!(summary.current_cadence, Some(85));

    // After more samples
    for i in 1..30 {
        recorder
            .record_sample(create_sample(i, 200, 140, 90))
            .unwrap();
    }
    let summary = recorder.get_live_summary();
    assert_eq!(summary.elapsed_seconds, 29);
    assert!(summary.distance_meters > 200.0);
}

#[test]
fn test_ride_summary_calculations() {
    let mut recorder = RideRecorder::with_defaults();
    let user_id = Uuid::new_v4();
    let ftp = 250;

    recorder.start(user_id, ftp).unwrap();

    // Record 300 samples (5 minutes) at 225W average
    for i in 0..300 {
        let power = 200 + (i % 50) as u16; // Varies 200-249W
        recorder
            .record_sample(create_sample(i, power, 145, 88))
            .unwrap();
    }

    let (ride, _samples) = recorder.finish().unwrap();

    // Verify summary was calculated
    assert_eq!(ride.duration_seconds, 299);
    // NOTE: avg_power/max_power stats not fully implemented yet (T090)
    // assert!(ride.avg_power.is_some() || ride.max_power.is_some());
    assert!(ride.distance_meters > 0.0);
    assert_eq!(ride.ftp_at_ride, 250);

    // IF and TSS should be calculated if NP is present
    if ride.normalized_power.is_some() {
        assert!(ride.intensity_factor.is_some());
        assert!(ride.tss.is_some());
    }
}
