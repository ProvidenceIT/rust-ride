//! Achievement definitions library.
//!
//! T035: Define initial 50+ achievement definitions with XP values.
//!
//! This module contains all built-in achievement definitions organized by category.
//! Each achievement has a unique name, display title, description, category, tier,
//! and optional threshold for progress tracking.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use uuid::Uuid;

use super::achievement::Achievement;
use super::types::{AchievementCategory, AchievementTier};

/// Achievement definition with a stable ID based on name.
///
/// Uses a deterministic UUID based on the achievement name to ensure
/// consistent IDs across application restarts.
fn achievement_with_stable_id(
    name: &str,
    title: &str,
    description: &str,
    category: AchievementCategory,
    tier: AchievementTier,
) -> Achievement {
    // Create a deterministic UUID from the achievement name using a hash
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();

    // Create UUID bytes from hash (fills 16 bytes for UUID)
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&hash.to_le_bytes());
    // Use a second hash for remaining bytes
    let mut hasher2 = DefaultHasher::new();
    format!("achievement:{}", name).hash(&mut hasher2);
    let hash2 = hasher2.finish();
    bytes[8..16].copy_from_slice(&hash2.to_le_bytes());

    let id = Uuid::from_bytes(bytes);
    Achievement::new(name, title, description, category, tier).with_id(id)
}

// ============================================================================
// Distance Achievements (8 achievements)
// ============================================================================

/// First ride completion.
pub fn first_ride() -> Achievement {
    achievement_with_stable_id(
        "first_ride",
        "First Pedal Stroke",
        "Complete your first ride",
        AchievementCategory::Distance,
        AchievementTier::Bronze,
    )
}

/// Ride 10km in a single ride.
pub fn distance_10k() -> Achievement {
    achievement_with_stable_id(
        "distance_10k",
        "Getting Rolling",
        "Ride 10km in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Bronze,
    )
    .with_threshold(10.0)
}

/// Ride 25km in a single ride.
pub fn distance_25k() -> Achievement {
    achievement_with_stable_id(
        "distance_25k",
        "Quarter Century",
        "Ride 25km in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Bronze,
    )
    .with_threshold(25.0)
}

/// Ride 50km in a single ride.
pub fn distance_50k() -> Achievement {
    achievement_with_stable_id(
        "distance_50k",
        "Half Century",
        "Ride 50km in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Silver,
    )
    .with_threshold(50.0)
}

/// Ride 100km in a single ride (metric century).
pub fn metric_century() -> Achievement {
    achievement_with_stable_id(
        "metric_century",
        "Metric Century",
        "Ride 100km in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Gold,
    )
    .with_threshold(100.0)
}

/// Ride 160km in a single ride (imperial century).
pub fn imperial_century() -> Achievement {
    achievement_with_stable_id(
        "imperial_century",
        "Century Rider",
        "Ride 160km (100 miles) in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Diamond,
    )
    .with_threshold(160.0)
}

/// Ride 200km in a single ride.
pub fn double_metric_century() -> Achievement {
    achievement_with_stable_id(
        "double_metric_century",
        "Double Century",
        "Ride 200km in a single ride",
        AchievementCategory::Distance,
        AchievementTier::Legendary,
    )
    .with_threshold(200.0)
}

/// Accumulate 1000km total distance.
pub fn lifetime_1000k() -> Achievement {
    achievement_with_stable_id(
        "lifetime_1000k",
        "Thousand Kilometer Club",
        "Accumulate 1,000km total riding distance",
        AchievementCategory::Distance,
        AchievementTier::Silver,
    )
    .with_threshold(1000.0)
}

/// Accumulate 5000km total distance.
pub fn lifetime_5000k() -> Achievement {
    achievement_with_stable_id(
        "lifetime_5000k",
        "Road Warrior",
        "Accumulate 5,000km total riding distance",
        AchievementCategory::Distance,
        AchievementTier::Gold,
    )
    .with_threshold(5000.0)
}

/// Accumulate 10000km total distance.
pub fn lifetime_10000k() -> Achievement {
    achievement_with_stable_id(
        "lifetime_10000k",
        "Iron Legs",
        "Accumulate 10,000km total riding distance",
        AchievementCategory::Distance,
        AchievementTier::Diamond,
    )
    .with_threshold(10000.0)
}

// ============================================================================
// Climbing Achievements (8 achievements)
// ============================================================================

/// Climb 100m in a single ride.
pub fn climb_100m() -> Achievement {
    achievement_with_stable_id(
        "climb_100m",
        "Hill Finder",
        "Climb 100m elevation in a single ride",
        AchievementCategory::Climbing,
        AchievementTier::Bronze,
    )
    .with_threshold(100.0)
}

/// Climb 500m in a single ride.
pub fn climb_500m() -> Achievement {
    achievement_with_stable_id(
        "climb_500m",
        "Mountain Goat",
        "Climb 500m elevation in a single ride",
        AchievementCategory::Climbing,
        AchievementTier::Silver,
    )
    .with_threshold(500.0)
}

/// Climb 1000m in a single ride.
pub fn climb_1000m() -> Achievement {
    achievement_with_stable_id(
        "climb_1000m",
        "Summit Seeker",
        "Climb 1,000m elevation in a single ride",
        AchievementCategory::Climbing,
        AchievementTier::Gold,
    )
    .with_threshold(1000.0)
}

/// Climb 2000m in a single ride.
pub fn climb_2000m() -> Achievement {
    achievement_with_stable_id(
        "climb_2000m",
        "Alpine Legend",
        "Climb 2,000m elevation in a single ride",
        AchievementCategory::Climbing,
        AchievementTier::Diamond,
    )
    .with_threshold(2000.0)
}

/// Climb 3000m in a single ride (Everesting attempt territory).
pub fn climb_3000m() -> Achievement {
    achievement_with_stable_id(
        "climb_3000m",
        "Cloud Walker",
        "Climb 3,000m elevation in a single ride",
        AchievementCategory::Climbing,
        AchievementTier::Legendary,
    )
    .with_threshold(3000.0)
}

/// Accumulate 10000m total climbing.
pub fn lifetime_climb_10k() -> Achievement {
    achievement_with_stable_id(
        "lifetime_climb_10k",
        "Climbing Enthusiast",
        "Accumulate 10,000m total elevation gain",
        AchievementCategory::Climbing,
        AchievementTier::Silver,
    )
    .with_threshold(10000.0)
}

/// Accumulate 50000m total climbing.
pub fn lifetime_climb_50k() -> Achievement {
    achievement_with_stable_id(
        "lifetime_climb_50k",
        "Vertical Veteran",
        "Accumulate 50,000m total elevation gain",
        AchievementCategory::Climbing,
        AchievementTier::Gold,
    )
    .with_threshold(50000.0)
}

/// Everest elevation in total climbing (8849m cumulative).
pub fn everest_challenge() -> Achievement {
    achievement_with_stable_id(
        "everest_challenge",
        "Virtual Everest",
        "Climb the height of Mount Everest (8,849m) cumulatively",
        AchievementCategory::Climbing,
        AchievementTier::Diamond,
    )
    .with_threshold(8849.0)
}

// ============================================================================
// Consistency Achievements (10 achievements)
// ============================================================================

/// Ride 3 days in a row.
pub fn streak_3() -> Achievement {
    achievement_with_stable_id(
        "streak_3",
        "Hat Trick",
        "Ride 3 days in a row",
        AchievementCategory::Consistency,
        AchievementTier::Bronze,
    )
    .with_threshold(3.0)
}

/// Ride 7 days in a row.
pub fn streak_7() -> Achievement {
    achievement_with_stable_id(
        "streak_7",
        "Week Warrior",
        "Ride 7 days in a row",
        AchievementCategory::Consistency,
        AchievementTier::Silver,
    )
    .with_threshold(7.0)
}

/// Ride 14 days in a row.
pub fn streak_14() -> Achievement {
    achievement_with_stable_id(
        "streak_14",
        "Fortnight Fighter",
        "Ride 14 days in a row",
        AchievementCategory::Consistency,
        AchievementTier::Gold,
    )
    .with_threshold(14.0)
}

/// Ride 30 days in a row.
pub fn streak_30() -> Achievement {
    achievement_with_stable_id(
        "streak_30",
        "Monthly Master",
        "Ride 30 days in a row",
        AchievementCategory::Consistency,
        AchievementTier::Diamond,
    )
    .with_threshold(30.0)
}

/// Ride 100 days in a row.
pub fn streak_100() -> Achievement {
    achievement_with_stable_id(
        "streak_100",
        "Century Streak",
        "Ride 100 days in a row",
        AchievementCategory::Consistency,
        AchievementTier::Legendary,
    )
    .with_threshold(100.0)
}

/// Complete 10 rides.
pub fn rides_10() -> Achievement {
    achievement_with_stable_id(
        "rides_10",
        "Getting Started",
        "Complete 10 rides",
        AchievementCategory::Consistency,
        AchievementTier::Bronze,
    )
    .with_threshold(10.0)
}

/// Complete 50 rides.
pub fn rides_50() -> Achievement {
    achievement_with_stable_id(
        "rides_50",
        "Regular Rider",
        "Complete 50 rides",
        AchievementCategory::Consistency,
        AchievementTier::Silver,
    )
    .with_threshold(50.0)
}

/// Complete 100 rides.
pub fn rides_100() -> Achievement {
    achievement_with_stable_id(
        "rides_100",
        "Century of Rides",
        "Complete 100 rides",
        AchievementCategory::Consistency,
        AchievementTier::Gold,
    )
    .with_threshold(100.0)
}

/// Complete 500 rides.
pub fn rides_500() -> Achievement {
    achievement_with_stable_id(
        "rides_500",
        "Dedicated Cyclist",
        "Complete 500 rides",
        AchievementCategory::Consistency,
        AchievementTier::Diamond,
    )
    .with_threshold(500.0)
}

/// Complete 1000 rides.
pub fn rides_1000() -> Achievement {
    achievement_with_stable_id(
        "rides_1000",
        "Thousand Rides",
        "Complete 1,000 rides",
        AchievementCategory::Consistency,
        AchievementTier::Legendary,
    )
    .with_threshold(1000.0)
}

// ============================================================================
// Training Achievements (8 achievements)
// ============================================================================

/// Complete first workout.
pub fn first_workout() -> Achievement {
    achievement_with_stable_id(
        "first_workout",
        "Training Begins",
        "Complete your first structured workout",
        AchievementCategory::Training,
        AchievementTier::Bronze,
    )
}

/// Complete 10 workouts.
pub fn workouts_10() -> Achievement {
    achievement_with_stable_id(
        "workouts_10",
        "Workout Regular",
        "Complete 10 structured workouts",
        AchievementCategory::Training,
        AchievementTier::Silver,
    )
    .with_threshold(10.0)
}

/// Complete 50 workouts.
pub fn workouts_50() -> Achievement {
    achievement_with_stable_id(
        "workouts_50",
        "Training Devotee",
        "Complete 50 structured workouts",
        AchievementCategory::Training,
        AchievementTier::Gold,
    )
    .with_threshold(50.0)
}

/// Complete 100 workouts.
pub fn workouts_100() -> Achievement {
    achievement_with_stable_id(
        "workouts_100",
        "Workout Champion",
        "Complete 100 structured workouts",
        AchievementCategory::Training,
        AchievementTier::Diamond,
    )
    .with_threshold(100.0)
}

/// Complete a workout with 100% compliance.
pub fn perfect_workout() -> Achievement {
    achievement_with_stable_id(
        "perfect_workout",
        "Perfect Execution",
        "Complete a workout with 100% target compliance",
        AchievementCategory::Training,
        AchievementTier::Gold,
    )
}

/// Complete a 2+ hour workout.
pub fn endurance_workout() -> Achievement {
    achievement_with_stable_id(
        "endurance_workout",
        "Endurance Engine",
        "Complete a workout lasting 2 hours or more",
        AchievementCategory::Training,
        AchievementTier::Gold,
    )
    .with_threshold(120.0)
}

/// Complete a training plan.
pub fn plan_complete() -> Achievement {
    achievement_with_stable_id(
        "plan_complete",
        "Plan Finisher",
        "Complete an entire training plan",
        AchievementCategory::Training,
        AchievementTier::Diamond,
    )
}

/// Complete 3 training plans.
pub fn plans_three() -> Achievement {
    achievement_with_stable_id(
        "plans_three",
        "Structured Athlete",
        "Complete 3 training plans",
        AchievementCategory::Training,
        AchievementTier::Legendary,
    )
    .with_threshold(3.0)
}

// ============================================================================
// Power Achievements (8 achievements)
// ============================================================================

/// Set your first power PR.
pub fn first_power_pr() -> Achievement {
    achievement_with_stable_id(
        "first_power_pr",
        "Power Unlocked",
        "Set your first power personal record",
        AchievementCategory::Power,
        AchievementTier::Bronze,
    )
}

/// Achieve 200W average for 20 minutes.
pub fn ftp_200() -> Achievement {
    achievement_with_stable_id(
        "ftp_200",
        "Breaking 200",
        "Achieve 200W average power for 20 minutes",
        AchievementCategory::Power,
        AchievementTier::Silver,
    )
    .with_threshold(200.0)
}

/// Achieve 250W average for 20 minutes.
pub fn ftp_250() -> Achievement {
    achievement_with_stable_id(
        "ftp_250",
        "Quarter Kilowatt",
        "Achieve 250W average power for 20 minutes",
        AchievementCategory::Power,
        AchievementTier::Gold,
    )
    .with_threshold(250.0)
}

/// Achieve 300W average for 20 minutes.
pub fn ftp_300() -> Achievement {
    achievement_with_stable_id(
        "ftp_300",
        "Category Racer",
        "Achieve 300W average power for 20 minutes",
        AchievementCategory::Power,
        AchievementTier::Diamond,
    )
    .with_threshold(300.0)
}

/// Achieve 400W average for 20 minutes.
pub fn ftp_400() -> Achievement {
    achievement_with_stable_id(
        "ftp_400",
        "Pro Power",
        "Achieve 400W average power for 20 minutes",
        AchievementCategory::Power,
        AchievementTier::Legendary,
    )
    .with_threshold(400.0)
}

/// Hit 1000W peak power.
pub fn peak_1000w() -> Achievement {
    achievement_with_stable_id(
        "peak_1000w",
        "Kilowatt Club",
        "Achieve 1000W peak power",
        AchievementCategory::Power,
        AchievementTier::Gold,
    )
    .with_threshold(1000.0)
}

/// Hit 1500W peak power.
pub fn peak_1500w() -> Achievement {
    achievement_with_stable_id(
        "peak_1500w",
        "Sprint King",
        "Achieve 1500W peak power",
        AchievementCategory::Power,
        AchievementTier::Diamond,
    )
    .with_threshold(1500.0)
}

/// Set 5 power PRs in a single ride.
pub fn multi_pr() -> Achievement {
    achievement_with_stable_id(
        "multi_pr",
        "PR Parade",
        "Set 5 or more power PRs in a single ride",
        AchievementCategory::Power,
        AchievementTier::Gold,
    )
    .with_threshold(5.0)
}

// ============================================================================
// Competition Achievements (6 achievements)
// ============================================================================

/// Complete first race.
pub fn first_race() -> Achievement {
    achievement_with_stable_id(
        "first_race",
        "Race Ready",
        "Complete your first race",
        AchievementCategory::Competition,
        AchievementTier::Bronze,
    )
}

/// Win a race (1st place).
pub fn race_winner() -> Achievement {
    achievement_with_stable_id(
        "race_winner",
        "Victory Lap",
        "Win a race (1st place)",
        AchievementCategory::Competition,
        AchievementTier::Gold,
    )
}

/// Podium finish (top 3).
pub fn podium_finish() -> Achievement {
    achievement_with_stable_id(
        "podium_finish",
        "Podium Finisher",
        "Finish on the podium (top 3)",
        AchievementCategory::Competition,
        AchievementTier::Silver,
    )
}

/// Complete 10 races.
pub fn races_10() -> Achievement {
    achievement_with_stable_id(
        "races_10",
        "Racing Regular",
        "Complete 10 races",
        AchievementCategory::Competition,
        AchievementTier::Silver,
    )
    .with_threshold(10.0)
}

/// Complete 50 races.
pub fn races_50() -> Achievement {
    achievement_with_stable_id(
        "races_50",
        "Racing Veteran",
        "Complete 50 races",
        AchievementCategory::Competition,
        AchievementTier::Gold,
    )
    .with_threshold(50.0)
}

/// Win 10 races.
pub fn wins_10() -> Achievement {
    achievement_with_stable_id(
        "wins_10",
        "Serial Winner",
        "Win 10 races",
        AchievementCategory::Competition,
        AchievementTier::Diamond,
    )
    .with_threshold(10.0)
}

// ============================================================================
// Exploration Achievements (6 achievements)
// ============================================================================

/// Complete first route with gradient simulation.
pub fn first_route() -> Achievement {
    achievement_with_stable_id(
        "first_route",
        "Route Explorer",
        "Complete your first GPX route with gradient simulation",
        AchievementCategory::Exploration,
        AchievementTier::Bronze,
    )
}

/// Complete 10 different routes.
pub fn routes_10() -> Achievement {
    achievement_with_stable_id(
        "routes_10",
        "Route Collector",
        "Complete 10 different routes",
        AchievementCategory::Exploration,
        AchievementTier::Silver,
    )
    .with_threshold(10.0)
}

/// Complete 50 different routes.
pub fn routes_50() -> Achievement {
    achievement_with_stable_id(
        "routes_50",
        "Route Master",
        "Complete 50 different routes",
        AchievementCategory::Exploration,
        AchievementTier::Gold,
    )
    .with_threshold(50.0)
}

/// Complete a route with >15% max gradient.
pub fn steep_route() -> Achievement {
    achievement_with_stable_id(
        "steep_route",
        "Wall Climber",
        "Complete a route with maximum gradient over 15%",
        AchievementCategory::Exploration,
        AchievementTier::Gold,
    )
    .with_threshold(15.0)
}

/// Complete 5 routes in a week.
pub fn weekly_explorer() -> Achievement {
    achievement_with_stable_id(
        "weekly_explorer",
        "Weekly Explorer",
        "Complete 5 different routes in a single week",
        AchievementCategory::Exploration,
        AchievementTier::Silver,
    )
    .with_threshold(5.0)
}

/// Complete a route over 100km with gradient simulation.
pub fn epic_route() -> Achievement {
    achievement_with_stable_id(
        "epic_route",
        "Epic Journey",
        "Complete a route over 100km with gradient simulation",
        AchievementCategory::Exploration,
        AchievementTier::Diamond,
    )
    .with_threshold(100.0)
}

// ============================================================================
// Special/Secret Achievements (8 achievements)
// ============================================================================

/// Ride at midnight.
pub fn night_owl() -> Achievement {
    achievement_with_stable_id(
        "night_owl",
        "Night Owl",
        "Complete a ride starting between midnight and 4am",
        AchievementCategory::Special,
        AchievementTier::Silver,
    )
    .secret()
}

/// Ride on New Year's Day.
pub fn new_year_rider() -> Achievement {
    achievement_with_stable_id(
        "new_year_rider",
        "New Year's Resolution",
        "Complete a ride on January 1st",
        AchievementCategory::Special,
        AchievementTier::Bronze,
    )
    .secret()
}

/// Ride for exactly 1 hour.
pub fn precision_rider() -> Achievement {
    achievement_with_stable_id(
        "precision_rider",
        "Precision Rider",
        "Complete a ride lasting exactly 1 hour (within 30 seconds)",
        AchievementCategory::Special,
        AchievementTier::Silver,
    )
    .secret()
}

/// Ride with 42.195km distance (marathon).
pub fn marathon_distance() -> Achievement {
    achievement_with_stable_id(
        "marathon_distance",
        "Marathon Distance",
        "Complete a ride of exactly 42.195km (marathon distance)",
        AchievementCategory::Special,
        AchievementTier::Gold,
    )
    .secret()
    .with_threshold(42.195)
}

/// Complete 7 rides in 7 days starting on Monday.
pub fn perfect_week() -> Achievement {
    achievement_with_stable_id(
        "perfect_week",
        "Perfect Week",
        "Ride every day of the week, Monday through Sunday",
        AchievementCategory::Special,
        AchievementTier::Gold,
    )
    .secret()
}

/// First ride of the year.
pub fn first_of_year() -> Achievement {
    achievement_with_stable_id(
        "first_of_year",
        "Fresh Start",
        "Complete your first ride of the calendar year",
        AchievementCategory::Special,
        AchievementTier::Bronze,
    )
    .repeatable()
}

/// Ride on your birthday (requires profile setup).
pub fn birthday_ride() -> Achievement {
    achievement_with_stable_id(
        "birthday_ride",
        "Birthday Spin",
        "Complete a ride on your birthday",
        AchievementCategory::Special,
        AchievementTier::Silver,
    )
    .secret()
    .repeatable()
}

/// Complete 100 hours of riding.
pub fn time_100h() -> Achievement {
    achievement_with_stable_id(
        "time_100h",
        "Century of Hours",
        "Accumulate 100 hours of riding time",
        AchievementCategory::Special,
        AchievementTier::Diamond,
    )
    .with_threshold(100.0)
}

// ============================================================================
// All Achievements Collection
// ============================================================================

/// Get all built-in achievement definitions.
///
/// Returns a vector of all 62 achievement definitions organized by category.
pub fn all_achievements() -> Vec<Achievement> {
    vec![
        // Distance (10)
        first_ride(),
        distance_10k(),
        distance_25k(),
        distance_50k(),
        metric_century(),
        imperial_century(),
        double_metric_century(),
        lifetime_1000k(),
        lifetime_5000k(),
        lifetime_10000k(),
        // Climbing (8)
        climb_100m(),
        climb_500m(),
        climb_1000m(),
        climb_2000m(),
        climb_3000m(),
        lifetime_climb_10k(),
        lifetime_climb_50k(),
        everest_challenge(),
        // Consistency (10)
        streak_3(),
        streak_7(),
        streak_14(),
        streak_30(),
        streak_100(),
        rides_10(),
        rides_50(),
        rides_100(),
        rides_500(),
        rides_1000(),
        // Training (8)
        first_workout(),
        workouts_10(),
        workouts_50(),
        workouts_100(),
        perfect_workout(),
        endurance_workout(),
        plan_complete(),
        plans_three(),
        // Power (8)
        first_power_pr(),
        ftp_200(),
        ftp_250(),
        ftp_300(),
        ftp_400(),
        peak_1000w(),
        peak_1500w(),
        multi_pr(),
        // Competition (6)
        first_race(),
        race_winner(),
        podium_finish(),
        races_10(),
        races_50(),
        wins_10(),
        // Exploration (6)
        first_route(),
        routes_10(),
        routes_50(),
        steep_route(),
        weekly_explorer(),
        epic_route(),
        // Special (8)
        night_owl(),
        new_year_rider(),
        precision_rider(),
        marathon_distance(),
        perfect_week(),
        first_of_year(),
        birthday_ride(),
        time_100h(),
    ]
}

/// Get achievements by category.
pub fn achievements_by_category(category: AchievementCategory) -> Vec<Achievement> {
    all_achievements()
        .into_iter()
        .filter(|a| a.category == category)
        .collect()
}

/// Get achievements by tier.
pub fn achievements_by_tier(tier: AchievementTier) -> Vec<Achievement> {
    all_achievements()
        .into_iter()
        .filter(|a| a.tier == tier)
        .collect()
}

/// Get secret achievements only.
pub fn secret_achievements() -> Vec<Achievement> {
    all_achievements()
        .into_iter()
        .filter(|a| a.is_secret)
        .collect()
}

/// Get achievement by name.
pub fn achievement_by_name(name: &str) -> Option<Achievement> {
    all_achievements().into_iter().find(|a| a.name == name)
}

/// Total count of achievements.
pub fn achievement_count() -> usize {
    all_achievements().len()
}

/// Maximum possible XP from earning all achievements.
pub fn total_possible_xp() -> u32 {
    all_achievements().iter().map(|a| a.effective_xp()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_achievements_count() {
        let achievements = all_achievements();
        // We defined 62 achievements total
        assert!(
            achievements.len() >= 50,
            "Expected at least 50 achievements, got {}",
            achievements.len()
        );
    }

    #[test]
    fn test_unique_ids() {
        let achievements = all_achievements();
        let mut ids: Vec<_> = achievements.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            achievements.len(),
            "All achievement IDs must be unique"
        );
    }

    #[test]
    fn test_unique_names() {
        let achievements = all_achievements();
        let mut names: Vec<_> = achievements.iter().map(|a| &a.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            achievements.len(),
            "All achievement names must be unique"
        );
    }

    #[test]
    fn test_stable_ids() {
        // IDs should be deterministic based on name
        let a1 = first_ride();
        let a2 = first_ride();
        assert_eq!(a1.id, a2.id, "Achievement IDs should be stable");
    }

    #[test]
    fn test_by_category() {
        let distance = achievements_by_category(AchievementCategory::Distance);
        assert!(!distance.is_empty());
        assert!(distance
            .iter()
            .all(|a| a.category == AchievementCategory::Distance));
    }

    #[test]
    fn test_by_tier() {
        let bronze = achievements_by_tier(AchievementTier::Bronze);
        assert!(!bronze.is_empty());
        assert!(bronze.iter().all(|a| a.tier == AchievementTier::Bronze));
    }

    #[test]
    fn test_secret_achievements() {
        let secrets = secret_achievements();
        assert!(!secrets.is_empty());
        assert!(secrets.iter().all(|a| a.is_secret));
    }

    #[test]
    fn test_achievement_by_name() {
        let achievement = achievement_by_name("first_ride");
        assert!(achievement.is_some());
        assert_eq!(achievement.unwrap().title, "First Pedal Stroke");

        let missing = achievement_by_name("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_total_possible_xp() {
        let total = total_possible_xp();
        // Should be a reasonable total (sum of all achievement XP)
        assert!(total > 10000, "Total XP should be significant");
    }

    #[test]
    fn test_all_categories_represented() {
        for category in AchievementCategory::all() {
            let achievements = achievements_by_category(*category);
            assert!(
                !achievements.is_empty(),
                "Category {:?} should have achievements",
                category
            );
        }
    }

    #[test]
    fn test_all_tiers_represented() {
        for tier in AchievementTier::all() {
            let achievements = achievements_by_tier(*tier);
            assert!(
                !achievements.is_empty(),
                "Tier {:?} should have achievements",
                tier
            );
        }
    }

    #[test]
    fn test_secret_xp_multiplier() {
        let secret = night_owl();
        assert!(secret.is_secret);
        // Silver = 250 base, secret multiplier = 1.5, so 375 XP
        assert_eq!(secret.effective_xp(), 375);
    }

    #[test]
    fn test_threshold_achievements() {
        let century = metric_century();
        assert_eq!(century.threshold, Some(100.0));
    }

    #[test]
    fn test_repeatable_achievements() {
        let first_of_year = first_of_year();
        assert!(first_of_year.repeatable);

        let first_ride = first_ride();
        assert!(!first_ride.repeatable);
    }
}
