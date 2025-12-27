//! Unit tests for ride history filtering.
//!
//! T127: Unit test for history filtering by date range

use chrono::{DateTime, Duration, Utc};
use rustride::recording::types::Ride;
use uuid::Uuid;

/// Test helper to create a ride with a specific date offset from a reference time.
fn create_ride_with_offset(user_id: Uuid, days_ago: i64, now: DateTime<Utc>) -> Ride {
    let mut ride = Ride::new(user_id, 200);
    ride.started_at = now - Duration::days(days_ago);
    ride.ended_at = Some(ride.started_at + Duration::hours(1));
    ride.duration_seconds = 3600;
    ride.distance_meters = 30000.0;
    ride.avg_power = Some(180);
    ride
}

#[test]
fn test_filter_rides_by_date_range() {
    let user_id = Uuid::new_v4();
    // Capture reference time once to avoid timing edge cases
    let now = Utc::now();

    let rides = [
        create_ride_with_offset(user_id, 1, now),  // Yesterday
        create_ride_with_offset(user_id, 7, now),  // Last week
        create_ride_with_offset(user_id, 30, now), // Last month
        create_ride_with_offset(user_id, 60, now), // Two months ago
    ];

    // Filter last 7 days (uses > to include rides exactly 7 days old)
    let week_ago = now - Duration::days(7);
    let last_week: Vec<_> = rides.iter().filter(|r| r.started_at >= week_ago).collect();
    assert_eq!(last_week.len(), 2);

    // Filter last 30 days
    let month_ago = now - Duration::days(30);
    let last_month: Vec<_> = rides.iter().filter(|r| r.started_at >= month_ago).collect();
    assert_eq!(last_month.len(), 3);

    // All time
    assert_eq!(rides.len(), 4);
}

#[test]
fn test_filter_rides_with_workout() {
    let user_id = Uuid::new_v4();

    let mut ride_free = Ride::new(user_id, 200);
    ride_free.workout_id = None;

    let mut ride_workout = Ride::new(user_id, 200);
    ride_workout.workout_id = Some(Uuid::new_v4());

    let rides = [ride_free, ride_workout];

    // Filter workout rides
    let workout_rides: Vec<_> = rides.iter().filter(|r| r.workout_id.is_some()).collect();
    assert_eq!(workout_rides.len(), 1);

    // Filter free rides
    let free_rides: Vec<_> = rides.iter().filter(|r| r.workout_id.is_none()).collect();
    assert_eq!(free_rides.len(), 1);
}

#[test]
fn test_calculate_weekly_totals() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let rides = [
        {
            let mut r = create_ride_with_offset(user_id, 1, now);
            r.duration_seconds = 3600;
            r.distance_meters = 30000.0;
            r.tss = Some(50.0);
            r
        },
        {
            let mut r = create_ride_with_offset(user_id, 2, now);
            r.duration_seconds = 5400;
            r.distance_meters = 45000.0;
            r.tss = Some(75.0);
            r
        },
        {
            let mut r = create_ride_with_offset(user_id, 3, now);
            r.duration_seconds = 1800;
            r.distance_meters = 15000.0;
            r.tss = Some(30.0);
            r
        },
    ];

    let total_duration: u32 = rides.iter().map(|r| r.duration_seconds).sum();
    let total_distance: f64 = rides.iter().map(|r| r.distance_meters).sum();
    let total_tss: f32 = rides.iter().filter_map(|r| r.tss).sum();

    assert_eq!(total_duration, 10800); // 3 hours
    assert_eq!(total_distance, 90000.0); // 90 km
    assert_eq!(total_tss, 155.0);
}

#[test]
fn test_sort_rides_by_date() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let mut rides = [
        create_ride_with_offset(user_id, 30, now),
        create_ride_with_offset(user_id, 1, now),
        create_ride_with_offset(user_id, 7, now),
    ];

    // Sort by date descending (most recent first)
    rides.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    assert!(rides[0].started_at > rides[1].started_at);
    assert!(rides[1].started_at > rides[2].started_at);
}
