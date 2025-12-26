//! Achievement definitions.

use super::{Achievement, AchievementCategory, AchievementTier};

/// Get all achievement definitions
pub fn all_achievements() -> Vec<Achievement> {
    let mut achievements = Vec::new();

    // Distance achievements
    achievements.extend(distance_achievements());

    // Climbing achievements
    achievements.extend(climbing_achievements());

    // Consistency achievements
    achievements.extend(consistency_achievements());

    // Competition achievements
    achievements.extend(competition_achievements());

    // Exploration achievements
    achievements.extend(exploration_achievements());

    // Training achievements
    achievements.extend(training_achievements());

    // Special achievements
    achievements.extend(special_achievements());

    achievements
}

fn distance_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "first_ride",
            "First Pedal",
            "Complete your first ride",
            AchievementCategory::Distance,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "distance_100km",
            "Century Club",
            "Ride a total of 100 kilometers",
            AchievementCategory::Distance,
            AchievementTier::Bronze,
        )
        .with_target(100_000.0), // meters
        Achievement::new(
            "distance_500km",
            "Road Warrior",
            "Ride a total of 500 kilometers",
            AchievementCategory::Distance,
            AchievementTier::Silver,
        )
        .with_target(500_000.0),
        Achievement::new(
            "distance_1000km",
            "Kilometer King",
            "Ride a total of 1,000 kilometers",
            AchievementCategory::Distance,
            AchievementTier::Gold,
        )
        .with_target(1_000_000.0),
        Achievement::new(
            "distance_5000km",
            "Tour de Force",
            "Ride a total of 5,000 kilometers",
            AchievementCategory::Distance,
            AchievementTier::Diamond,
        )
        .with_target(5_000_000.0),
        Achievement::new(
            "distance_10000km",
            "Around the World",
            "Ride a total of 10,000 kilometers",
            AchievementCategory::Distance,
            AchievementTier::Legendary,
        )
        .with_target(10_000_000.0),
        Achievement::new(
            "single_100km",
            "Century Ride",
            "Complete a single ride of 100km or more",
            AchievementCategory::Distance,
            AchievementTier::Gold,
        )
        .with_target(100_000.0),
    ]
}

fn climbing_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "first_climb",
            "Hill Hunter",
            "Complete your first climb",
            AchievementCategory::Climbing,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "elevation_1000m",
            "Sky High",
            "Climb a total of 1,000 meters",
            AchievementCategory::Climbing,
            AchievementTier::Bronze,
        )
        .with_target(1000.0),
        Achievement::new(
            "elevation_10000m",
            "Mountain Goat",
            "Climb a total of 10,000 meters",
            AchievementCategory::Climbing,
            AchievementTier::Silver,
        )
        .with_target(10_000.0),
        Achievement::new(
            "elevation_everest",
            "Everesting Prep",
            "Climb a total of 8,848 meters (height of Everest)",
            AchievementCategory::Climbing,
            AchievementTier::Gold,
        )
        .with_target(8848.0),
        Achievement::new(
            "elevation_50000m",
            "Cloud Walker",
            "Climb a total of 50,000 meters",
            AchievementCategory::Climbing,
            AchievementTier::Diamond,
        )
        .with_target(50_000.0),
        Achievement::new(
            "hc_climb",
            "Hors Categorie",
            "Complete an HC category climb",
            AchievementCategory::Climbing,
            AchievementTier::Gold,
        ),
        Achievement::new(
            "all_categories",
            "Category Crusher",
            "Complete climbs in all categories (HC, 1, 2, 3, 4)",
            AchievementCategory::Climbing,
            AchievementTier::Diamond,
        ),
    ]
}

fn consistency_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "streak_3",
            "Getting Started",
            "Ride 3 days in a row",
            AchievementCategory::Consistency,
            AchievementTier::Bronze,
        )
        .with_target(3.0),
        Achievement::new(
            "streak_7",
            "Week Warrior",
            "Ride 7 days in a row",
            AchievementCategory::Consistency,
            AchievementTier::Silver,
        )
        .with_target(7.0),
        Achievement::new(
            "streak_30",
            "Monthly Motivation",
            "Ride 30 days in a row",
            AchievementCategory::Consistency,
            AchievementTier::Gold,
        )
        .with_target(30.0),
        Achievement::new(
            "streak_100",
            "Iron Legs",
            "Ride 100 days in a row",
            AchievementCategory::Consistency,
            AchievementTier::Legendary,
        )
        .with_target(100.0),
        Achievement::new(
            "rides_10",
            "Regular Rider",
            "Complete 10 rides",
            AchievementCategory::Consistency,
            AchievementTier::Bronze,
        )
        .with_target(10.0),
        Achievement::new(
            "rides_100",
            "Centurion",
            "Complete 100 rides",
            AchievementCategory::Consistency,
            AchievementTier::Gold,
        )
        .with_target(100.0),
        Achievement::new(
            "rides_500",
            "Half Millennium",
            "Complete 500 rides",
            AchievementCategory::Consistency,
            AchievementTier::Diamond,
        )
        .with_target(500.0),
    ]
}

fn competition_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "first_segment",
            "Segment Seeker",
            "Complete your first segment",
            AchievementCategory::Competition,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "pb_segment",
            "Personal Best",
            "Set a personal best on any segment",
            AchievementCategory::Competition,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "top_10_segment",
            "Top 10",
            "Finish in the top 10 on any segment",
            AchievementCategory::Competition,
            AchievementTier::Silver,
        ),
        Achievement::new(
            "first_place",
            "Gold Medal",
            "Take first place on any segment",
            AchievementCategory::Competition,
            AchievementTier::Gold,
        ),
        Achievement::new(
            "segments_50",
            "Segment Collector",
            "Complete 50 different segments",
            AchievementCategory::Competition,
            AchievementTier::Silver,
        )
        .with_target(50.0),
        Achievement::new(
            "sprint_win",
            "Sprint King",
            "Win a sprint segment",
            AchievementCategory::Competition,
            AchievementTier::Silver,
        ),
        Achievement::new(
            "kom",
            "King of the Mountain",
            "Take the KOM on a categorized climb",
            AchievementCategory::Competition,
            AchievementTier::Diamond,
        ),
    ]
}

fn exploration_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "first_landmark",
            "Sightseer",
            "Discover your first landmark",
            AchievementCategory::Exploration,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "landmarks_10",
            "Explorer",
            "Discover 10 landmarks",
            AchievementCategory::Exploration,
            AchievementTier::Bronze,
        )
        .with_target(10.0),
        Achievement::new(
            "landmarks_50",
            "Adventurer",
            "Discover 50 landmarks",
            AchievementCategory::Exploration,
            AchievementTier::Silver,
        )
        .with_target(50.0),
        Achievement::new(
            "landmarks_100",
            "World Traveler",
            "Discover 100 landmarks",
            AchievementCategory::Exploration,
            AchievementTier::Gold,
        )
        .with_target(100.0),
        Achievement::new(
            "routes_5",
            "Route Finder",
            "Ride 5 different routes",
            AchievementCategory::Exploration,
            AchievementTier::Bronze,
        )
        .with_target(5.0),
        Achievement::new(
            "routes_25",
            "Pathfinder",
            "Ride 25 different routes",
            AchievementCategory::Exploration,
            AchievementTier::Silver,
        )
        .with_target(25.0),
        Achievement::new(
            "pro_route",
            "Following the Pros",
            "Complete a famous pro cycling route",
            AchievementCategory::Exploration,
            AchievementTier::Gold,
        ),
    ]
}

fn training_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "first_workout",
            "Structured Start",
            "Complete your first structured workout",
            AchievementCategory::Training,
            AchievementTier::Bronze,
        ),
        Achievement::new(
            "workouts_10",
            "Training Committed",
            "Complete 10 structured workouts",
            AchievementCategory::Training,
            AchievementTier::Bronze,
        )
        .with_target(10.0),
        Achievement::new(
            "workouts_50",
            "Training Dedicated",
            "Complete 50 structured workouts",
            AchievementCategory::Training,
            AchievementTier::Silver,
        )
        .with_target(50.0),
        Achievement::new(
            "ftp_increase",
            "Getting Stronger",
            "Increase your FTP from a test",
            AchievementCategory::Training,
            AchievementTier::Silver,
        ),
        Achievement::new(
            "zone_5_30min",
            "Threshold Crusher",
            "Spend 30 minutes in Zone 5",
            AchievementCategory::Training,
            AchievementTier::Gold,
        )
        .with_target(1800.0), // seconds
        Achievement::new(
            "tss_1000",
            "Training Load",
            "Accumulate 1,000 TSS",
            AchievementCategory::Training,
            AchievementTier::Silver,
        )
        .with_target(1000.0),
        Achievement::new(
            "tss_10000",
            "Serious Athlete",
            "Accumulate 10,000 TSS",
            AchievementCategory::Training,
            AchievementTier::Diamond,
        )
        .with_target(10_000.0),
    ]
}

fn special_achievements() -> Vec<Achievement> {
    vec![
        Achievement::new(
            "early_bird",
            "Early Bird",
            "Start a ride before 6am",
            AchievementCategory::Special,
            AchievementTier::Bronze,
        )
        .secret(),
        Achievement::new(
            "night_owl",
            "Night Owl",
            "Ride after midnight",
            AchievementCategory::Special,
            AchievementTier::Bronze,
        )
        .secret(),
        Achievement::new(
            "new_year",
            "New Year's Resolution",
            "Complete a ride on January 1st",
            AchievementCategory::Special,
            AchievementTier::Silver,
        )
        .secret(),
        Achievement::new(
            "holiday_rider",
            "No Days Off",
            "Complete a ride on a major holiday",
            AchievementCategory::Special,
            AchievementTier::Bronze,
        )
        .secret(),
        Achievement::new(
            "long_ride_3h",
            "Endurance",
            "Complete a ride longer than 3 hours",
            AchievementCategory::Special,
            AchievementTier::Silver,
        )
        .with_target(10800.0), // 3 hours in seconds
        Achievement::new(
            "all_weather",
            "All Weather Rider",
            "Ride in all weather conditions",
            AchievementCategory::Special,
            AchievementTier::Gold,
        )
        .secret(),
        Achievement::new(
            "dawn_dusk",
            "Dawn to Dusk",
            "Ride through sunrise and sunset in one session",
            AchievementCategory::Special,
            AchievementTier::Gold,
        )
        .secret(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_achievements() {
        let achievements = all_achievements();

        // Should have multiple achievements
        assert!(achievements.len() > 30);

        // All should have unique keys
        let mut keys: Vec<_> = achievements.iter().map(|a| &a.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), achievements.len());

        // Should cover all categories
        let categories: std::collections::HashSet<_> =
            achievements.iter().map(|a| a.category).collect();
        assert!(categories.contains(&AchievementCategory::Distance));
        assert!(categories.contains(&AchievementCategory::Climbing));
        assert!(categories.contains(&AchievementCategory::Consistency));
    }

    #[test]
    fn test_achievement_tiers_balanced() {
        let achievements = all_achievements();

        let bronze = achievements
            .iter()
            .filter(|a| a.tier == AchievementTier::Bronze)
            .count();
        let legendary = achievements
            .iter()
            .filter(|a| a.tier == AchievementTier::Legendary)
            .count();

        // Should have more bronze than legendary (pyramid structure)
        assert!(bronze > legendary);
    }
}
