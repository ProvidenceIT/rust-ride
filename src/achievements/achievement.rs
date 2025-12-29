//! Achievement definition structure.
//!
//! T032: Create Achievement struct with XP value.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{AchievementCategory, AchievementTier, SECRET_XP_MULTIPLIER};

/// An achievement definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier.
    pub id: Uuid,
    /// Internal name (for code reference).
    pub name: String,
    /// Display title shown to the user.
    pub title: String,
    /// Description of how to earn the achievement.
    pub description: String,
    /// Category for grouping.
    pub category: AchievementCategory,
    /// Difficulty tier determining base XP.
    pub tier: AchievementTier,
    /// Base XP value (before multipliers).
    pub xp_value: u32,
    /// Whether this is a secret (hidden until earned).
    pub is_secret: bool,
    /// Optional icon name/path.
    pub icon: Option<String>,
    /// Threshold value for progress (e.g., 100 for "Ride 100km").
    pub threshold: Option<f64>,
    /// Whether this achievement can be earned multiple times.
    pub repeatable: bool,
}

impl Achievement {
    /// Create a new achievement with default values.
    pub fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        category: AchievementCategory,
        tier: AchievementTier,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            title: title.into(),
            description: description.into(),
            category,
            tier,
            xp_value: tier.base_xp(),
            is_secret: false,
            icon: None,
            threshold: None,
            repeatable: false,
        }
    }

    /// Set the achievement as secret.
    pub fn secret(mut self) -> Self {
        self.is_secret = true;
        self
    }

    /// Set a threshold value for progress-based achievements.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Set custom XP value (overrides tier-based default).
    pub fn with_xp(mut self, xp: u32) -> Self {
        self.xp_value = xp;
        self
    }

    /// Set icon.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set as repeatable.
    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }

    /// Calculate effective XP value (including secret multiplier).
    pub fn effective_xp(&self) -> u32 {
        if self.is_secret {
            (self.xp_value as f32 * SECRET_XP_MULTIPLIER) as u32
        } else {
            self.xp_value
        }
    }

    /// Create with a specific ID (for loading from database).
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

impl PartialEq for Achievement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Achievement {}

impl std::hash::Hash for Achievement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_creation() {
        let achievement = Achievement::new(
            "first_ride",
            "First Ride",
            "Complete your first ride",
            AchievementCategory::Training,
            AchievementTier::Bronze,
        );

        assert_eq!(achievement.title, "First Ride");
        assert_eq!(achievement.xp_value, 100); // Bronze tier
        assert!(!achievement.is_secret);
    }

    #[test]
    fn test_achievement_secret_multiplier() {
        let achievement = Achievement::new(
            "secret_climb",
            "Mystery Summit",
            "Complete a secret challenge",
            AchievementCategory::Special,
            AchievementTier::Gold,
        )
        .secret();

        assert!(achievement.is_secret);
        // 500 * 1.5 = 750
        assert_eq!(achievement.effective_xp(), 750);
    }

    #[test]
    fn test_achievement_with_threshold() {
        let achievement = Achievement::new(
            "century",
            "Century Rider",
            "Ride 100km in a single ride",
            AchievementCategory::Distance,
            AchievementTier::Gold,
        )
        .with_threshold(100.0);

        assert_eq!(achievement.threshold, Some(100.0));
    }

    #[test]
    fn test_achievement_equality() {
        let id = Uuid::new_v4();
        let a1 = Achievement::new("test", "Test", "Test", AchievementCategory::Training, AchievementTier::Bronze)
            .with_id(id);
        let a2 = Achievement::new("test", "Test", "Test", AchievementCategory::Training, AchievementTier::Bronze)
            .with_id(id);

        assert_eq!(a1, a2);
    }
}
