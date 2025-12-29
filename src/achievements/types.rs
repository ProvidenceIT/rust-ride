//! Achievement type definitions.

use serde::{Deserialize, Serialize};

/// Category for grouping achievements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    /// Distance-based achievements (km milestones)
    Distance,
    /// Climbing achievements (elevation gain)
    Climbing,
    /// Consistency achievements (streaks, daily rides)
    Consistency,
    /// Competition achievements (races, PRs)
    Competition,
    /// Exploration achievements (routes, landmarks)
    Exploration,
    /// Training achievements (workout completion)
    Training,
    /// Special achievements (unique accomplishments)
    Special,
    /// Power-related achievements (power records, zones)
    Power,
}

impl AchievementCategory {
    /// Get display name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Distance => "Distance",
            Self::Climbing => "Climbing",
            Self::Consistency => "Consistency",
            Self::Competition => "Competition",
            Self::Exploration => "Exploration",
            Self::Training => "Training",
            Self::Special => "Special",
            Self::Power => "Power",
        }
    }

    /// Get all category variants.
    pub fn all() -> &'static [AchievementCategory] {
        &[
            Self::Distance,
            Self::Climbing,
            Self::Consistency,
            Self::Competition,
            Self::Exploration,
            Self::Training,
            Self::Special,
            Self::Power,
        ]
    }
}

/// Difficulty tier determining base XP value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AchievementTier {
    /// Bronze tier: 100 XP base
    Bronze,
    /// Silver tier: 250 XP base
    Silver,
    /// Gold tier: 500 XP base
    Gold,
    /// Diamond tier: 1000 XP base
    Diamond,
    /// Legendary tier: 2500 XP base
    Legendary,
}

impl AchievementTier {
    /// Get the base XP value for this tier.
    pub fn base_xp(&self) -> u32 {
        match self {
            Self::Bronze => 100,
            Self::Silver => 250,
            Self::Gold => 500,
            Self::Diamond => 1000,
            Self::Legendary => 2500,
        }
    }

    /// Get display name for the tier.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Diamond => "Diamond",
            Self::Legendary => "Legendary",
        }
    }

    /// Get all tier variants in order.
    pub fn all() -> &'static [AchievementTier] {
        &[
            Self::Bronze,
            Self::Silver,
            Self::Gold,
            Self::Diamond,
            Self::Legendary,
        ]
    }
}

/// Multiplier for secret achievement XP.
pub const SECRET_XP_MULTIPLIER: f32 = 1.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_xp_values() {
        assert_eq!(AchievementTier::Bronze.base_xp(), 100);
        assert_eq!(AchievementTier::Silver.base_xp(), 250);
        assert_eq!(AchievementTier::Gold.base_xp(), 500);
        assert_eq!(AchievementTier::Diamond.base_xp(), 1000);
        assert_eq!(AchievementTier::Legendary.base_xp(), 2500);
    }

    #[test]
    fn test_tier_ordering() {
        assert!(AchievementTier::Bronze < AchievementTier::Silver);
        assert!(AchievementTier::Silver < AchievementTier::Gold);
        assert!(AchievementTier::Gold < AchievementTier::Diamond);
        assert!(AchievementTier::Diamond < AchievementTier::Legendary);
    }

    #[test]
    fn test_all_categories() {
        let categories = AchievementCategory::all();
        assert_eq!(categories.len(), 8);
    }
}
