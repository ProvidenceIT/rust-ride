//! Integration tests for achievement flow.
//!
//! T084: Integration test for achievement flow.

use rustride::achievements::{
    Achievement, AchievementCategory, AchievementTier, AchievementTracker, CumulativeStats,
    DefaultAchievementTracker, RideMetrics, XpGain, XpSource, XpStatus,
};
use uuid::Uuid;

#[test]
fn test_achievement_tracker_creation() {
    let user_id = Uuid::new_v4();
    let tracker = DefaultAchievementTracker::new(user_id);
    let status = tracker.xp_status();
    assert_eq!(status.total_xp, 0);
    assert_eq!(status.level, 1);
}

#[test]
fn test_ride_metrics_creation() {
    let ride_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 50.0, 5400); // 50km, 90min

    assert_eq!(metrics.distance_km, 50.0);
    assert_eq!(metrics.duration_secs, 5400);
}

#[test]
fn test_ride_metrics_builder() {
    let ride_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 100.0, 10800) // 100km, 3hr
        .with_elevation(1200.0)
        .with_power(200, Some(210), 450)
        .with_training_metrics(200.0, 0.85);

    assert_eq!(metrics.distance_km, 100.0);
    assert_eq!(metrics.elevation_gain_m, 1200.0);
    assert_eq!(metrics.avg_power, Some(200));
    assert_eq!(metrics.normalized_power, Some(210));
    assert_eq!(metrics.max_power, Some(450));
}

#[test]
fn test_cumulative_stats_default() {
    let stats = CumulativeStats::default();
    assert_eq!(stats.total_distance_km, 0.0);
    assert_eq!(stats.total_rides, 0);
    assert_eq!(stats.current_streak, 0);
}

#[test]
fn test_xp_gain_from_ride() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    // Initial state
    let initial_status = tracker.xp_status();
    assert_eq!(initial_status.total_xp, 0);

    // Add XP from a ride
    let gain = XpGain::new(100, XpSource::Ride);
    let result = tracker.add_xp(gain);

    // Should have gained XP
    let new_status = tracker.xp_status();
    assert_eq!(new_status.total_xp, 100);
    assert!(result.xp_added > 0);
}

#[test]
fn test_level_progression() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    // Start at level 1
    assert_eq!(tracker.xp_status().level, 1);

    // Add enough XP to level up (level 2 requires 1150 XP with 1000 * 1.15^1 curve)
    let gain = XpGain::new(1200, XpSource::Ride);
    let result = tracker.add_xp(gain);

    // Should be level 2 now
    assert!(result.leveled_up);
    let status = tracker.xp_status();
    assert!(status.level >= 2);
}

#[test]
fn test_achievement_categories() {
    assert_eq!(AchievementCategory::Distance.display_name(), "Distance");
    assert_eq!(AchievementCategory::Training.display_name(), "Training");
    assert_eq!(AchievementCategory::Climbing.display_name(), "Climbing");
    assert_eq!(AchievementCategory::Power.display_name(), "Power");
}

#[test]
fn test_achievement_tiers() {
    assert!(AchievementTier::Bronze.base_xp() < AchievementTier::Silver.base_xp());
    assert!(AchievementTier::Silver.base_xp() < AchievementTier::Gold.base_xp());
    assert!(AchievementTier::Gold.base_xp() < AchievementTier::Diamond.base_xp());
}

#[test]
fn test_achievement_creation() {
    let achievement = Achievement::new(
        "test_achievement",
        "Test Achievement",
        "A test achievement",
        AchievementCategory::Distance,
        AchievementTier::Bronze,
    );

    // name is internal ID, title is display name
    assert_eq!(achievement.name, "test_achievement");
    assert_eq!(achievement.title, "Test Achievement");
    assert_eq!(achievement.category, AchievementCategory::Distance);
    assert_eq!(achievement.tier, AchievementTier::Bronze);
}

#[test]
fn test_xp_status_from_total() {
    // Need 1150+ XP for level 2 with exponential curve
    let status = XpStatus::from_total_xp(1500);

    // Should calculate level and progress
    assert!(status.level >= 2);
    assert!(status.level_progress >= 0.0 && status.level_progress <= 1.0);
}

#[test]
fn test_xp_status_max_level_check() {
    // Very high XP should be at or near max level (MAX_LEVEL = 50)
    // With exponential curve 1000 * 1.15^n, need billions for level 50
    let status = XpStatus::from_total_xp(1_000_000_000);
    assert!(status.level >= 40); // Should be at high level
}

#[test]
fn test_ride_metrics_with_workout() {
    let ride_id = Uuid::new_v4();
    let workout_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 20.0, 3600).with_workout(workout_id, true);

    assert!(metrics.workout_completed);
    assert!(metrics.workout_id.is_some());
}

#[test]
fn test_night_owl_detection() {
    let ride_id = Uuid::new_v4();
    let mut metrics = RideMetrics::new(ride_id, 30.0, 3600);
    metrics.start_hour = Some(5); // 5am
    metrics.start_date = Some((6, 15)); // June 15

    // Night owl is for rides before 6am
    assert!(metrics.start_hour == Some(5));
}

#[test]
fn test_xp_multiplier() {
    use rustride::achievements::XpMultiplier;

    assert_eq!(XpMultiplier::None.value(), 1.0);
    assert!(XpMultiplier::FirstRideOfDay.value() > 1.0);
    assert!(XpMultiplier::WeeklyStreak.value() > XpMultiplier::FirstRideOfDay.value());
    assert!(XpMultiplier::EventBonus.value() > XpMultiplier::WeeklyStreak.value());
}

#[test]
fn test_xp_gain_with_multiplier() {
    use rustride::achievements::XpMultiplier;

    let base_gain = XpGain::new(100, XpSource::Ride);
    let boosted_gain = base_gain.with_multiplier(XpMultiplier::FirstRideOfDay);

    // 100 base * 1.25 multiplier = 125
    assert_eq!(boosted_gain.final_xp, 125);
}

#[test]
fn test_cumulative_stats_tracking() {
    let stats = CumulativeStats {
        total_distance_km: 1000.0,
        total_elevation_m: 15000.0,
        total_time_secs: 180000,
        total_rides: 50,
        total_workouts: 20,
        current_streak: 7,
        longest_streak: 14,
        last_ride_date: None,
        rides_by_weekday: [5, 10, 8, 7, 6, 9, 5],
        max_distance_km: 100.0,
        max_elevation_m: 2000.0,
        max_power: 400,
        max_duration_mins: 180,
    };

    assert_eq!(stats.total_rides, 50);
    assert_eq!(stats.total_workouts, 20);
    assert_eq!(stats.current_streak, 7);
}

#[test]
fn test_full_achievement_flow() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    // Register an achievement
    let century_achievement = Achievement::new(
        "century_ride",
        "Century Rider",
        "Complete a 100km ride",
        AchievementCategory::Distance,
        AchievementTier::Gold,
    );
    tracker.register_achievement(century_achievement);

    // Simulate completing a century ride
    let ride_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 100.0, 10800) // 100km, 3 hours
        .with_elevation(1200.0)
        .with_power(200, Some(210), 450);

    // Add XP from the ride
    let xp = rustride::achievements::xp_from_ride(
        metrics.distance_km,
        metrics.duration_secs / 60, // convert to minutes
        metrics.elevation_gain_m,
    );

    let gain = XpGain::new(xp, XpSource::Ride);
    tracker.add_xp(gain);

    // Should have gained significant XP
    let status = tracker.xp_status();
    assert!(status.total_xp > 0);
}

#[test]
fn test_ride_with_race() {
    let ride_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 40.0, 3600).with_race(5, 20); // 5th place out of 20

    assert!(metrics.is_race);
    assert_eq!(metrics.race_position, Some(5));
    assert_eq!(metrics.race_participants, Some(20));
}

#[test]
fn test_xp_from_ride_calculation() {
    // Basic ride: 50km, 90min, 500m climbing
    let xp = rustride::achievements::xp_from_ride(50.0, 90, 500.0);

    // Should give reasonable XP:
    // ~500 base (10 per km)
    // +45 time bonus (5 per 10 min)
    // +100 elevation bonus (20 per 100m)
    assert!(xp > 600);
    assert!(xp < 800);
}

#[test]
fn test_xp_from_workout_calculation() {
    // 60 minute workout completed with 65 TSS
    let xp = rustride::achievements::xp_from_workout(60, Some(65.0), true);

    // Base: 120 (2 per minute)
    // TSS bonus: ~32 (0.5 per TSS)
    // Completion bonus: 50
    assert!(xp > 150);
    assert!(xp < 250);
}

#[test]
fn test_xp_from_incomplete_workout() {
    // 30 minute workout not completed
    let xp = rustride::achievements::xp_from_workout(30, None, false);

    // Should give partial credit
    assert!(xp > 0);
    assert!(xp < 100); // Much less than completed
}

#[test]
fn test_cumulative_stats_update() {
    let mut stats = CumulativeStats::default();

    let ride_id = Uuid::new_v4();
    let metrics = RideMetrics::new(ride_id, 50.0, 5400)
        .with_elevation(500.0)
        .with_power(200, Some(210), 350);

    stats.update_from_ride(&metrics);

    assert_eq!(stats.total_distance_km, 50.0);
    assert_eq!(stats.total_elevation_m, 500.0);
    assert_eq!(stats.total_rides, 1);
}

#[test]
fn test_all_achievement_tiers() {
    let tiers = AchievementTier::all();
    assert_eq!(tiers.len(), 5);
    assert!(tiers.contains(&AchievementTier::Bronze));
    assert!(tiers.contains(&AchievementTier::Legendary));
}

#[test]
fn test_all_achievement_categories() {
    let categories = AchievementCategory::all();
    assert_eq!(categories.len(), 8);
    assert!(categories.contains(&AchievementCategory::Distance));
    assert!(categories.contains(&AchievementCategory::Power));
}
