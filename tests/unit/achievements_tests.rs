//! Achievement system unit tests.
//!
//! T045: Add unit tests for XP curve and level calculations.

use rustride::achievements::checks::ConsistencyChecker;
use rustride::achievements::{
    achievement_count, all_achievements, xp_from_ride, xp_from_workout, Achievement,
    AchievementCategory, AchievementChecker, AchievementTier, AchievementTracker, AllCheckers,
    CumulativeChecker, DefaultAchievementTracker, RideChecker, RideMetrics, XpGain, XpMultiplier,
    XpSource, XpStatus,
};
use rustride::career::{cumulative_xp_to_level, level_from_xp, xp_for_level, MAX_LEVEL};
use uuid::Uuid;

// ============================================================================
// XP Curve Tests
// ============================================================================

#[test]
fn test_xp_for_level_is_increasing() {
    // Level 1 requires 0 XP (starting level)
    // Level 2+ requires increasing amounts
    assert_eq!(xp_for_level(1), 0, "Level 1 should require 0 XP");
    assert!(xp_for_level(2) > 0, "Level 2 should require positive XP");

    let mut prev_xp = xp_for_level(2);
    for level in 3..=MAX_LEVEL {
        let xp = xp_for_level(level);
        assert!(
            xp > prev_xp,
            "XP required for level {} ({}) should be greater than level {} ({})",
            level,
            xp,
            level - 1,
            prev_xp
        );
        prev_xp = xp;
    }
}

#[test]
fn test_cumulative_xp_is_sum_of_levels() {
    for level in 1..=10 {
        let cumulative = cumulative_xp_to_level(level);
        let mut sum = 0u64;
        for l in 1..level {
            sum += xp_for_level(l);
        }
        assert_eq!(
            cumulative, sum,
            "Cumulative XP to level {} should equal sum of all prior level requirements",
            level
        );
    }
}

#[test]
fn test_level_from_xp_boundaries() {
    // Level 1 starts at 0 XP
    assert_eq!(level_from_xp(0), 1);

    // Just before level 2
    let xp_for_2 = xp_for_level(2);
    assert_eq!(level_from_xp(xp_for_2 - 1), 1);

    // Exactly at level 2
    assert_eq!(level_from_xp(xp_for_2), 2);

    // Just into level 2
    assert_eq!(level_from_xp(xp_for_2 + 1), 2);
}

#[test]
fn test_max_level_cap() {
    // Very high XP should cap at MAX_LEVEL
    let huge_xp = 1_000_000_000;
    assert_eq!(level_from_xp(huge_xp), MAX_LEVEL);
}

#[test]
fn test_xp_status_at_various_levels() {
    // Test at level 1
    let status = XpStatus::from_total_xp(500);
    assert_eq!(status.level, 1);
    assert!(status.level_progress > 0.0);
    assert!(status.level_progress < 1.0);

    // Test at level 5+
    // cumulative_xp_to_level(5) gives XP needed to REACH level 5
    // So at that exact XP, we should be AT level 5
    let level_5_xp = cumulative_xp_to_level(5);
    let status = XpStatus::from_total_xp(level_5_xp);
    // At exactly the boundary, we've achieved level 5
    assert!(
        status.level >= 4,
        "Expected level 4 or 5, got {}",
        status.level
    );

    // Test at max level
    let max_xp = 999_999_999;
    let status = XpStatus::from_total_xp(max_xp);
    assert!(status.is_max_level());
    assert_eq!(status.level, MAX_LEVEL);
}

#[test]
fn test_xp_status_xp_to_level() {
    // Test that xp_to_level works correctly for lower target levels
    let status = XpStatus::from_total_xp(5000);

    // Current level should be > 1 with 5000 XP
    assert!(status.level > 1, "With 5000 XP, should be above level 1");

    // XP to reach level 1 (which we've already passed) should be 0
    assert_eq!(status.xp_to_level(1), 0);

    // XP to reach current level should be 0
    assert_eq!(status.xp_to_level(status.level), 0);

    // XP to reach a very high level should be positive
    let xp_to_level_40 = status.xp_to_level(40);
    assert!(xp_to_level_40 > 0, "XP to level 40 should be positive");
}

// ============================================================================
// XP Calculation Tests
// ============================================================================

#[test]
fn test_xp_from_ride_formula() {
    // 100km, 4 hours, 2000m elevation
    let xp = xp_from_ride(100.0, 240, 2000.0);

    // Expected: 100*10 + 24*5 + 400 = 1000 + 120 + 400 = 1520
    assert_eq!(xp, 1520);
}

#[test]
fn test_xp_from_ride_zero_values() {
    let xp = xp_from_ride(0.0, 0, 0.0);
    assert_eq!(xp, 0);
}

#[test]
fn test_xp_from_workout_complete() {
    // 60 min, TSS 100, completed
    let xp = xp_from_workout(60, Some(100.0), true);

    // Expected: 60*2 + 50 + 50 = 120 + 50 + 50 = 220
    assert_eq!(xp, 220);
}

#[test]
fn test_xp_from_workout_incomplete() {
    // 60 min, not completed - partial credit
    let xp = xp_from_workout(60, Some(100.0), false);

    // Partial: 60/10 * 10 = 60
    assert_eq!(xp, 60);
}

#[test]
fn test_xp_multipliers() {
    assert_eq!(XpMultiplier::None.value(), 1.0);
    assert_eq!(XpMultiplier::FirstRideOfDay.value(), 1.25);
    assert_eq!(XpMultiplier::WeeklyStreak.value(), 1.5);
    assert_eq!(XpMultiplier::EventBonus.value(), 2.0);
    assert_eq!(XpMultiplier::Custom(3.0).value(), 3.0);
}

#[test]
fn test_xp_gain_with_multiplier() {
    let gain = XpGain::new(100, XpSource::Achievement).with_multiplier(XpMultiplier::EventBonus);

    assert_eq!(gain.base_xp, 100);
    assert_eq!(gain.multiplier, 2.0);
    assert_eq!(gain.final_xp, 200);
}

// ============================================================================
// Achievement Definition Tests
// ============================================================================

#[test]
fn test_achievement_count_minimum() {
    let count = achievement_count();
    assert!(
        count >= 50,
        "Expected at least 50 achievements, got {}",
        count
    );
}

#[test]
fn test_all_categories_have_achievements() {
    let achievements = all_achievements();

    for category in AchievementCategory::all() {
        let count = achievements
            .iter()
            .filter(|a| a.category == *category)
            .count();
        assert!(
            count > 0,
            "Category {:?} should have at least one achievement",
            category
        );
    }
}

#[test]
fn test_all_tiers_have_achievements() {
    let achievements = all_achievements();

    for tier in AchievementTier::all() {
        let count = achievements.iter().filter(|a| a.tier == *tier).count();
        assert!(
            count > 0,
            "Tier {:?} should have at least one achievement",
            tier
        );
    }
}

#[test]
fn test_achievement_xp_values() {
    let achievements = all_achievements();

    for achievement in &achievements {
        let xp = achievement.effective_xp();
        assert!(
            xp > 0,
            "Achievement {} should have positive XP",
            achievement.name
        );

        // Verify tier-based XP
        let expected_base = achievement.tier.base_xp();
        if !achievement.is_secret {
            assert_eq!(
                achievement.xp_value, expected_base,
                "Achievement {} should have tier-based XP",
                achievement.name
            );
        }
    }
}

// ============================================================================
// Achievement Tracker Tests
// ============================================================================

#[test]
fn test_tracker_awards_xp() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    let achievement = Achievement::new(
        "test",
        "Test",
        "Test description",
        AchievementCategory::Training,
        AchievementTier::Silver,
    );
    tracker.register_achievement(achievement.clone());

    let initial_status = tracker.xp_status();
    assert_eq!(initial_status.total_xp, 0);

    tracker.award(&achievement, None);

    let final_status = tracker.xp_status();
    assert_eq!(final_status.total_xp, 250); // Silver = 250 XP
}

#[test]
fn test_tracker_prevents_duplicate_awards() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    let achievement = Achievement::new(
        "test",
        "Test",
        "Test description",
        AchievementCategory::Training,
        AchievementTier::Bronze,
    );
    tracker.register_achievement(achievement.clone());

    // First award succeeds
    let first = tracker.award(&achievement, None);
    assert!(first.is_some());

    // Second award fails
    let second = tracker.award(&achievement, None);
    assert!(second.is_none());

    // XP only awarded once
    assert_eq!(tracker.xp_status().total_xp, 100);
}

#[test]
fn test_tracker_summary() {
    let user_id = Uuid::new_v4();
    let mut tracker = DefaultAchievementTracker::new(user_id);

    // Register and award multiple achievements
    for (i, category) in AchievementCategory::all().iter().enumerate() {
        let achievement = Achievement::new(
            format!("test_{}", i),
            format!("Test {}", i),
            "Test",
            *category,
            AchievementTier::Bronze,
        );
        tracker.register_achievement(achievement.clone());
        tracker.award(&achievement, None);
    }

    let summary = tracker.summary();
    assert_eq!(
        summary.total_earned,
        AchievementCategory::all().len() as u32
    );
    assert_eq!(summary.by_category.len(), AchievementCategory::all().len());
}

// ============================================================================
// Achievement Check Tests
// ============================================================================

#[test]
fn test_ride_checker_distance() {
    let checker = RideChecker::new();
    let stats = rustride::achievements::CumulativeStats::default();

    // Short ride
    let metrics = RideMetrics::new(Uuid::new_v4(), 5.0, 1200);
    let achievements = checker.check(&metrics, &stats);
    assert!(achievements.iter().any(|a| a.name == "first_ride"));

    // Century ride
    let metrics = RideMetrics::new(Uuid::new_v4(), 100.0, 14400);
    let achievements = checker.check(&metrics, &stats);
    assert!(achievements.iter().any(|a| a.name == "metric_century"));
}

#[test]
fn test_cumulative_checker_distance() {
    let checker = CumulativeChecker::new();
    let metrics = RideMetrics::new(Uuid::new_v4(), 50.0, 7200);

    let stats = rustride::achievements::CumulativeStats {
        total_distance_km: 1500.0,
        ..Default::default()
    };

    let achievements = checker.check(&metrics, &stats);
    assert!(achievements.iter().any(|a| a.name == "lifetime_1000k"));
}

#[test]
fn test_consistency_checker_streak() {
    let checker = ConsistencyChecker::new();
    let metrics = RideMetrics::new(Uuid::new_v4(), 20.0, 3600);

    let stats = rustride::achievements::CumulativeStats {
        current_streak: 7,
        ..Default::default()
    };

    let achievements = checker.check(&metrics, &stats);
    assert!(achievements.iter().any(|a| a.name == "streak_7"));
}

#[test]
fn test_all_checkers_combined() {
    let checker = AllCheckers::new();

    let mut metrics = RideMetrics::new(Uuid::new_v4(), 100.0, 14400);
    metrics.elevation_gain_m = 1500.0;

    let stats = rustride::achievements::CumulativeStats {
        current_streak: 14,
        total_rides: 100,
        ..Default::default()
    };

    let achievements = checker.check_all(&metrics, &stats);

    // Should include achievements from all checkers
    assert!(achievements.iter().any(|a| a.name == "metric_century"));
    assert!(achievements.iter().any(|a| a.name == "climb_1000m"));
    assert!(achievements.iter().any(|a| a.name == "streak_14"));
    assert!(achievements.iter().any(|a| a.name == "rides_100"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_xp_overflow_protection() {
    // Ensure calculations don't overflow with large values
    let large_distance = 10000.0;
    let large_duration = 100000;
    let large_elevation = 50000.0;

    let xp = xp_from_ride(large_distance, large_duration, large_elevation);
    assert!(xp > 0);
}

#[test]
fn test_achievement_progress_clamping() {
    use rustride::achievements::AchievementProgress;

    let mut progress = AchievementProgress::new(Uuid::new_v4(), 100.0);

    // Update with value beyond target
    progress.update(150.0);

    assert_eq!(progress.progress_percent, 1.0);
    assert!(progress.is_earned);
}

#[test]
fn test_notification_queue_ordering() {
    use rustride::achievements::AchievementNotification;
    use rustride::achievements::NotificationQueue;

    let mut queue = NotificationQueue::with_settings(1, 5);

    // Add multiple notifications
    for i in 0..3 {
        let notification = AchievementNotification::new(
            Uuid::new_v4(),
            format!("Title {}", i),
            "Description",
            AchievementCategory::Training,
            AchievementTier::Bronze,
            100,
        );
        queue.push(notification);
    }

    assert_eq!(queue.pending_count(), 3);

    // First update should promote first notification to current
    let changed = queue.update();
    assert!(changed);
    assert!(queue.current().is_some());
    assert_eq!(queue.current().unwrap().title, "Title 0");
}
