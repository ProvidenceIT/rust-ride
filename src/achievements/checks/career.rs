//! Career-related achievement checks.
//!
//! T081: Add career-related achievements (level milestones).
//!
//! Checks achievements related to career progression and level milestones.

use crate::achievements::achievement::Achievement;
use crate::achievements::types::{AchievementCategory, AchievementTier};

/// Summary of career status for achievement checking.
#[derive(Debug, Clone, Default)]
pub struct CareerSummary {
    /// Current career level.
    pub current_level: u32,
    /// Total XP earned.
    pub total_xp: u64,
    /// Total rewards unlocked.
    pub rewards_unlocked: u32,
    /// Total cosmetics equipped.
    pub cosmetics_equipped: u32,
}

/// Checker for career achievements.
#[derive(Debug, Default)]
pub struct CareerChecker;

impl CareerChecker {
    /// Create a new career checker.
    pub fn new() -> Self {
        Self
    }

    /// Check achievements based on career progress.
    pub fn check(&self, summary: &CareerSummary) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // Level milestone achievements
        if summary.current_level >= 5 {
            achievements.push(level_5());
        }
        if summary.current_level >= 10 {
            achievements.push(level_10());
        }
        if summary.current_level >= 15 {
            achievements.push(level_15());
        }
        if summary.current_level >= 20 {
            achievements.push(level_20());
        }
        if summary.current_level >= 25 {
            achievements.push(level_25());
        }
        if summary.current_level >= 30 {
            achievements.push(level_30());
        }
        if summary.current_level >= 35 {
            achievements.push(level_35());
        }
        if summary.current_level >= 40 {
            achievements.push(level_40());
        }
        if summary.current_level >= 45 {
            achievements.push(level_45());
        }
        if summary.current_level >= 50 {
            achievements.push(level_50());
        }

        // XP milestones
        if summary.total_xp >= 10_000 {
            achievements.push(xp_10k());
        }
        if summary.total_xp >= 50_000 {
            achievements.push(xp_50k());
        }
        if summary.total_xp >= 100_000 {
            achievements.push(xp_100k());
        }
        if summary.total_xp >= 500_000 {
            achievements.push(xp_500k());
        }
        if summary.total_xp >= 1_000_000 {
            achievements.push(xp_1m());
        }

        // Reward collection
        if summary.rewards_unlocked >= 10 {
            achievements.push(collector_10());
        }
        if summary.rewards_unlocked >= 25 {
            achievements.push(collector_25());
        }
        if summary.rewards_unlocked >= 50 {
            achievements.push(collector_50());
        }

        achievements
    }

    /// Check for a level up.
    pub fn check_level_up(&self, old_level: u32, new_level: u32) -> Vec<Achievement> {
        let summary = CareerSummary {
            current_level: new_level,
            ..Default::default()
        };

        // Only return achievements for levels just crossed
        self.check(&summary)
            .into_iter()
            .filter(|a| {
                // Get the level threshold from the achievement
                if let Some(threshold) = a.threshold {
                    let level = threshold as u32;
                    level > old_level && level <= new_level
                } else {
                    false
                }
            })
            .collect()
    }
}

//
// Level Milestone Achievements
//

/// Level 5 achievement.
pub fn level_5() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000001").unwrap(),
        name: "career_level_5".to_string(),
        title: "Rising Rider".to_string(),
        description: "Reach career level 5".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: Some(5.0),
        is_secret: false,
        icon: Some("level_5".to_string()),
        repeatable: false,
    }
}

/// Level 10 achievement.
pub fn level_10() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000002").unwrap(),
        name: "career_level_10".to_string(),
        title: "Double Digits".to_string(),
        description: "Reach career level 10".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(10.0),
        is_secret: false,
        icon: Some("level_10".to_string()),
        repeatable: false,
    }
}

/// Level 15 achievement.
pub fn level_15() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000003").unwrap(),
        name: "career_level_15".to_string(),
        title: "Dedicated Cyclist".to_string(),
        description: "Reach career level 15".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Silver,
        xp_value: 150,
        threshold: Some(15.0),
        is_secret: false,
        icon: Some("level_15".to_string()),
        repeatable: false,
    }
}

/// Level 20 achievement.
pub fn level_20() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000004").unwrap(),
        name: "career_level_20".to_string(),
        title: "Committed Rider".to_string(),
        description: "Reach career level 20".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(20.0),
        is_secret: false,
        icon: Some("level_20".to_string()),
        repeatable: false,
    }
}

/// Level 25 achievement.
pub fn level_25() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000005").unwrap(),
        name: "career_level_25".to_string(),
        title: "Quarter Century".to_string(),
        description: "Reach career level 25".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Gold,
        xp_value: 250,
        threshold: Some(25.0),
        is_secret: false,
        icon: Some("level_25".to_string()),
        repeatable: false,
    }
}

/// Level 30 achievement.
pub fn level_30() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000006").unwrap(),
        name: "career_level_30".to_string(),
        title: "Expert Status".to_string(),
        description: "Reach career level 30".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Gold,
        xp_value: 300,
        threshold: Some(30.0),
        is_secret: false,
        icon: Some("level_30".to_string()),
        repeatable: false,
    }
}

/// Level 35 achievement.
pub fn level_35() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000007").unwrap(),
        name: "career_level_35".to_string(),
        title: "Elite Rider".to_string(),
        description: "Reach career level 35".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Diamond,
        xp_value: 350,
        threshold: Some(35.0),
        is_secret: false,
        icon: Some("level_35".to_string()),
        repeatable: false,
    }
}

/// Level 40 achievement.
pub fn level_40() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000008").unwrap(),
        name: "career_level_40".to_string(),
        title: "Champion".to_string(),
        description: "Reach career level 40".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Diamond,
        xp_value: 400,
        threshold: Some(40.0),
        is_secret: false,
        icon: Some("level_40".to_string()),
        repeatable: false,
    }
}

/// Level 45 achievement.
pub fn level_45() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000009").unwrap(),
        name: "career_level_45".to_string(),
        title: "Master Cyclist".to_string(),
        description: "Reach career level 45".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Diamond,
        xp_value: 450,
        threshold: Some(45.0),
        is_secret: false,
        icon: Some("level_45".to_string()),
        repeatable: false,
    }
}

/// Level 50 achievement.
pub fn level_50() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000010").unwrap(),
        name: "career_level_50".to_string(),
        title: "Cycling Legend".to_string(),
        description: "Reach the maximum career level 50".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Legendary,
        xp_value: 1000,
        threshold: Some(50.0),
        is_secret: false,
        icon: Some("level_50".to_string()),
        repeatable: false,
    }
}

//
// XP Milestone Achievements
//

/// 10K XP achievement.
pub fn xp_10k() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000011").unwrap(),
        name: "career_xp_10k".to_string(),
        title: "XP Earner".to_string(),
        description: "Earn 10,000 total XP".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: Some(10000.0),
        is_secret: false,
        icon: Some("xp_10k".to_string()),
        repeatable: false,
    }
}

/// 50K XP achievement.
pub fn xp_50k() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000012").unwrap(),
        name: "career_xp_50k".to_string(),
        title: "XP Accumulator".to_string(),
        description: "Earn 50,000 total XP".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(50000.0),
        is_secret: false,
        icon: Some("xp_50k".to_string()),
        repeatable: false,
    }
}

/// 100K XP achievement.
pub fn xp_100k() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000013").unwrap(),
        name: "career_xp_100k".to_string(),
        title: "Century of XP".to_string(),
        description: "Earn 100,000 total XP".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(100000.0),
        is_secret: false,
        icon: Some("xp_100k".to_string()),
        repeatable: false,
    }
}

/// 500K XP achievement.
pub fn xp_500k() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000014").unwrap(),
        name: "career_xp_500k".to_string(),
        title: "Half Million XP".to_string(),
        description: "Earn 500,000 total XP".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Diamond,
        xp_value: 500,
        threshold: Some(500000.0),
        is_secret: false,
        icon: Some("xp_500k".to_string()),
        repeatable: false,
    }
}

/// 1M XP achievement.
pub fn xp_1m() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000015").unwrap(),
        name: "career_xp_1m".to_string(),
        title: "XP Millionaire".to_string(),
        description: "Earn 1,000,000 total XP".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Legendary,
        xp_value: 1000,
        threshold: Some(1000000.0),
        is_secret: false,
        icon: Some("xp_1m".to_string()),
        repeatable: false,
    }
}

//
// Collection Achievements
//

/// 10 rewards collected.
pub fn collector_10() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000016").unwrap(),
        name: "career_collector_10".to_string(),
        title: "Collector".to_string(),
        description: "Unlock 10 cosmetic rewards".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: Some(10.0),
        is_secret: false,
        icon: Some("collector".to_string()),
        repeatable: false,
    }
}

/// 25 rewards collected.
pub fn collector_25() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000017").unwrap(),
        name: "career_collector_25".to_string(),
        title: "Avid Collector".to_string(),
        description: "Unlock 25 cosmetic rewards".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(25.0),
        is_secret: false,
        icon: Some("collector_25".to_string()),
        repeatable: false,
    }
}

/// 50 rewards collected.
pub fn collector_50() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000020-0000-4000-8000-000000000018").unwrap(),
        name: "career_collector_50".to_string(),
        title: "Master Collector".to_string(),
        description: "Unlock 50 cosmetic rewards".to_string(),
        category: AchievementCategory::Special,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(50.0),
        is_secret: false,
        icon: Some("collector_50".to_string()),
        repeatable: false,
    }
}

/// Get all career achievements.
pub fn all_career_achievements() -> Vec<Achievement> {
    vec![
        level_5(),
        level_10(),
        level_15(),
        level_20(),
        level_25(),
        level_30(),
        level_35(),
        level_40(),
        level_45(),
        level_50(),
        xp_10k(),
        xp_50k(),
        xp_100k(),
        xp_500k(),
        xp_1m(),
        collector_10(),
        collector_25(),
        collector_50(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_career_achievements() {
        let achievements = all_career_achievements();
        assert_eq!(achievements.len(), 18);

        // Check all have unique IDs
        let ids: std::collections::HashSet<_> = achievements.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), achievements.len());
    }

    #[test]
    fn test_level_achievement_check() {
        let checker = CareerChecker::new();
        let summary = CareerSummary {
            current_level: 10,
            ..Default::default()
        };

        let achievements = checker.check(&summary);

        assert!(achievements.iter().any(|a| a.name == "career_level_5"));
        assert!(achievements.iter().any(|a| a.name == "career_level_10"));
        assert!(!achievements.iter().any(|a| a.name == "career_level_15"));
    }

    #[test]
    fn test_xp_achievement_check() {
        let checker = CareerChecker::new();
        let summary = CareerSummary {
            total_xp: 75_000,
            ..Default::default()
        };

        let achievements = checker.check(&summary);

        assert!(achievements.iter().any(|a| a.name == "career_xp_10k"));
        assert!(achievements.iter().any(|a| a.name == "career_xp_50k"));
        assert!(!achievements.iter().any(|a| a.name == "career_xp_100k"));
    }

    #[test]
    fn test_level_up_check() {
        let checker = CareerChecker::new();
        let achievements = checker.check_level_up(9, 10);

        // Should get level 10 achievement (just crossed)
        assert!(achievements.iter().any(|a| a.name == "career_level_10"));
        // Should not get level 5 (crossed earlier)
        assert!(!achievements.iter().any(|a| a.name == "career_level_5"));
    }

    #[test]
    fn test_max_level_achievement() {
        let checker = CareerChecker::new();
        let summary = CareerSummary {
            current_level: 50,
            ..Default::default()
        };

        let achievements = checker.check(&summary);

        assert!(achievements.iter().any(|a| a.name == "career_level_50"));
        assert!(achievements
            .iter()
            .any(|a| a.tier == AchievementTier::Legendary));
    }
}
