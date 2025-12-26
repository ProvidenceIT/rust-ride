//! Unit tests for segment timing
//!
//! T065: Unit test for segment timing in tests/unit/segment_timing_test.rs

use rustride::world::segments::{Segment, SegmentCategory, SegmentTime};
use rustride::world::segments::timing::{ActiveTiming, SegmentTimer, TimingState};
use uuid::Uuid;

/// Test segment creation with gradient
#[test]
fn test_segment_creation() {
    let route_id = Uuid::new_v4();
    let segment = Segment::new(
        route_id,
        "Test Climb".to_string(),
        1000.0,
        3000.0,
        200.0, // 200m elevation over 2000m = 10% gradient
    );

    assert_eq!(segment.route_id, route_id);
    assert_eq!(segment.name, "Test Climb");
    assert!((segment.length_meters - 2000.0).abs() < 0.01);
    assert!((segment.avg_gradient_percent - 10.0).abs() < 0.1);
}

/// Test segment category classification
#[test]
fn test_segment_category_sprint() {
    // Low gradient = sprint
    let cat = SegmentCategory::from_profile(20.0, 3000.0);
    assert_eq!(cat, Some(SegmentCategory::Sprint));
}

#[test]
fn test_segment_category_cat4() {
    // Cat4: climb_score 50-100
    // 150m over 1500m = 10% gradient, climb_score = 150 * 10 / 100 = 15... too low
    // Need: score > 50, e.g., 80m over 400m = 20%, score = 80 * 20 / 100 = 16... still low
    // Actually formula: climb_score = elevation_gain * avg_gradient / 100
    // Need score 50-100: e.g., 100m gain at 8% = 100 * 8 / 100 = 8... too low
    // Let's use 200m over 1000m = 20%, score = 200 * 20 / 100 = 40... still too low
    // Try 150m over 500m = 30%, score = 150 * 30 / 100 = 45... close
    // Try 180m over 500m = 36%, score = 180 * 36 / 100 = 64.8 (Cat4!)
    let cat = SegmentCategory::from_profile(180.0, 500.0);
    assert_eq!(cat, Some(SegmentCategory::Cat4));
}

#[test]
fn test_segment_category_cat3() {
    // Cat3: score 100-200
    // 300m over 700m = 42.8%, score = 300 * 42.8 / 100 = 128.5 (Cat3!)
    let cat = SegmentCategory::from_profile(300.0, 700.0);
    assert_eq!(cat, Some(SegmentCategory::Cat3));
}

#[test]
fn test_segment_category_cat2() {
    // Cat2: score 200-400
    // 500m over 1000m = 50%, score = 500 * 50 / 100 = 250 (Cat2!)
    let cat = SegmentCategory::from_profile(500.0, 1000.0);
    assert_eq!(cat, Some(SegmentCategory::Cat2));
}

#[test]
fn test_segment_category_cat1() {
    // Cat1: score 400-800
    // 800m over 2000m = 40%, score = 800 * 40 / 100 = 320... Cat2
    // Try 1000m over 2500m = 40%, score = 1000 * 40 / 100 = 400 (Cat1)
    // Actually need > 400, so 1000m over 2400m = 41.7%, score = 416
    let cat = SegmentCategory::from_profile(1000.0, 2400.0);
    assert_eq!(cat, Some(SegmentCategory::Cat1));
}

#[test]
fn test_segment_category_hc() {
    // HC: score > 800
    // 1500m over 3000m = 50%, score = 1500 * 50 / 100 = 750... not quite
    // 2000m over 4000m = 50%, score = 2000 * 50 / 100 = 1000 (HC!)
    let cat = SegmentCategory::from_profile(2000.0, 4000.0);
    assert_eq!(cat, Some(SegmentCategory::HC));
}

/// Test segment time creation
#[test]
fn test_segment_time_creation() {
    let segment_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    let time = SegmentTime::new(segment_id, user_id, ride_id, 120.5, 250);

    assert_eq!(time.segment_id, segment_id);
    assert_eq!(time.user_id, user_id);
    assert_eq!(time.ride_id, ride_id);
    assert!((time.time_seconds - 120.5).abs() < 0.01);
    assert_eq!(time.ftp_at_effort, 250);
    assert!(!time.is_personal_best); // Default is false
}

/// Test segment time with metrics
#[test]
fn test_segment_time_with_metrics() {
    let time = SegmentTime::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        100.0,
        250,
    ).with_metrics(Some(280), Some(175));

    assert_eq!(time.avg_power_watts, Some(280));
    assert_eq!(time.avg_heart_rate, Some(175));
}

/// Test active timing creation
#[test]
fn test_active_timing_creation() {
    let segment_id = Uuid::new_v4();
    let timing = ActiveTiming::new(segment_id, 10.0, Some(120.0));

    assert_eq!(timing.segment_id, segment_id);
    assert!((timing.start_time_seconds - 10.0).abs() < 0.01);
    assert!((timing.elapsed_seconds - 0.0).abs() < 0.01);
    assert_eq!(timing.target_time_seconds, Some(120.0));
}

/// Test active timing update with power and HR
#[test]
fn test_active_timing_update() {
    let mut timing = ActiveTiming::new(Uuid::new_v4(), 0.0, None);

    timing.update(10.0, Some(200), Some(150));
    assert!((timing.elapsed_seconds - 10.0).abs() < 0.01);
    assert!((timing.avg_power_watts - 200.0).abs() < 0.01);
    assert!((timing.avg_heart_rate - 150.0).abs() < 0.01);

    // Second update should average
    timing.update(20.0, Some(220), Some(160));
    assert!((timing.elapsed_seconds - 20.0).abs() < 0.01);
    assert!((timing.avg_power_watts - 210.0).abs() < 0.01);
    assert!((timing.avg_heart_rate - 155.0).abs() < 0.01);
}

/// Test delta vs target calculation
#[test]
fn test_active_timing_delta_vs_target() {
    let mut timing = ActiveTiming::new(Uuid::new_v4(), 0.0, Some(60.0));

    // At 30s, should be on pace
    timing.update(30.0, None, None);
    let delta = timing.delta_vs_target().unwrap();
    assert!((delta - (-30.0)).abs() < 0.01); // 30s elapsed, 60s target = -30s (30s ahead)

    // At 70s, behind pace
    timing.update(70.0, None, None);
    let delta = timing.delta_vs_target().unwrap();
    assert!((delta - 10.0).abs() < 0.01); // 70s elapsed, 60s target = +10s (behind)
}

/// Test segment timer inactive state
#[test]
fn test_segment_timer_inactive() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    // Before segment
    timer.update(
        500.0, 0.0, Some(200), Some(150),
        Uuid::new_v4(), Uuid::new_v4(), 250, None,
    );
    assert_eq!(timer.state(), TimingState::Inactive);
    assert!(timer.active().is_none());
}

/// Test segment timer approaching state
#[test]
fn test_segment_timer_approaching() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    // Within 200m of start
    timer.update(
        850.0, 5.0, Some(200), Some(150),
        Uuid::new_v4(), Uuid::new_v4(), 250, None,
    );
    assert_eq!(timer.state(), TimingState::Approaching);
    assert!(timer.distance_to_next().is_some());
    assert!((timer.distance_to_next().unwrap() - 150.0).abs() < 1.0);
}

/// Test segment timer active state
#[test]
fn test_segment_timer_active() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    // Enter segment
    timer.update(
        1500.0, 10.0, Some(200), Some(150),
        Uuid::new_v4(), Uuid::new_v4(), 250, None,
    );
    assert_eq!(timer.state(), TimingState::Active);
    assert!(timer.active().is_some());
}

/// Test segment timer completed state
#[test]
fn test_segment_timer_completion() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // Enter segment
    timer.update(
        1500.0, 10.0, Some(200), Some(150),
        user_id, ride_id, 250, None,
    );
    assert_eq!(timer.state(), TimingState::Active);

    // Exit segment
    let completed = timer.update(
        2100.0, 70.0, Some(200), Some(150),
        user_id, ride_id, 250, None,
    );

    assert_eq!(timer.state(), TimingState::Completed);
    assert!(timer.active().is_none());
    assert!(completed.is_some());

    let time = completed.unwrap();
    assert!((time.time_seconds - 60.0).abs() < 0.01); // 70 - 10 = 60s
    assert!(time.is_personal_best); // First attempt is PB
}

/// Test personal best detection
#[test]
fn test_personal_best_detection() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // First attempt
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, None);
    let first = timer.update(2100.0, 60.0, None, None, user_id, ride_id, 250, None);
    assert!(first.unwrap().is_personal_best);

    // Reset and do faster attempt
    timer.reset();
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, Some(60.0)); // Previous PB: 60s
    let second = timer.update(2100.0, 50.0, None, None, user_id, ride_id, 250, Some(60.0));
    assert!(second.unwrap().is_personal_best); // 50s < 60s

    // Reset and do slower attempt
    timer.reset();
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, Some(50.0)); // Previous PB: 50s
    let third = timer.update(2100.0, 70.0, None, None, user_id, ride_id, 250, Some(50.0));
    assert!(!third.unwrap().is_personal_best); // 70s > 50s
}

/// Test segment timer with multiple segments
#[test]
fn test_timer_multiple_segments() {
    let segments = vec![
        Segment::new(Uuid::new_v4(), "Seg1".to_string(), 1000.0, 2000.0, 50.0),
        Segment::new(Uuid::new_v4(), "Seg2".to_string(), 3000.0, 4000.0, 100.0),
    ];
    let mut timer = SegmentTimer::new(segments);

    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // Complete first segment
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, None);
    timer.update(2100.0, 60.0, None, None, user_id, ride_id, 250, None);

    // Approach second segment
    timer.update(2850.0, 90.0, None, None, user_id, ride_id, 250, None);
    assert_eq!(timer.state(), TimingState::Approaching);

    // Complete second segment
    timer.update(3500.0, 100.0, None, None, user_id, ride_id, 250, None);
    assert_eq!(timer.state(), TimingState::Active);
}

/// Test completed times tracking
#[test]
fn test_completed_times_tracking() {
    let segments = vec![
        Segment::new(Uuid::new_v4(), "Seg1".to_string(), 1000.0, 2000.0, 50.0),
        Segment::new(Uuid::new_v4(), "Seg2".to_string(), 3000.0, 4000.0, 100.0),
    ];
    let mut timer = SegmentTimer::new(segments);

    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // Complete both segments
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, None);
    timer.update(2100.0, 60.0, None, None, user_id, ride_id, 250, None);

    timer.update(3500.0, 100.0, None, None, user_id, ride_id, 250, None);
    timer.update(4100.0, 180.0, None, None, user_id, ride_id, 250, None);

    assert_eq!(timer.completed_times().len(), 2);
}

/// Test timer reset
#[test]
fn test_timer_reset() {
    let segment = Segment::new(
        Uuid::new_v4(),
        "Test".to_string(),
        1000.0,
        2000.0,
        50.0,
    );
    let mut timer = SegmentTimer::new(vec![segment]);

    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();

    // Complete a segment
    timer.update(1500.0, 0.0, None, None, user_id, ride_id, 250, None);
    timer.update(2100.0, 60.0, None, None, user_id, ride_id, 250, None);

    assert!(!timer.completed_times().is_empty());

    // Reset
    timer.reset();

    assert_eq!(timer.state(), TimingState::Inactive);
    assert!(timer.active().is_none());
    assert!(timer.completed_times().is_empty());
}
