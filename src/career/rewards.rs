//! Career reward definitions.

use serde::{Deserialize, Serialize};

/// Type of unlockable reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RewardType {
    /// Jersey/kit color option
    JerseyColor,
    /// Bike frame style
    BikeFrame,
    /// UI color theme
    UiTheme,
    /// Accent color for UI elements
    AccentColor,
    /// Profile badge/icon
    ProfileBadge,
    /// Wheel style
    WheelStyle,
    /// Helmet style
    HelmetStyle,
}

impl RewardType {
    /// Get display name for the reward type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::JerseyColor => "Jersey Color",
            Self::BikeFrame => "Bike Frame",
            Self::UiTheme => "UI Theme",
            Self::AccentColor => "Accent Color",
            Self::ProfileBadge => "Profile Badge",
            Self::WheelStyle => "Wheel Style",
            Self::HelmetStyle => "Helmet Style",
        }
    }

    /// Get all reward types.
    pub fn all() -> &'static [RewardType] {
        &[
            Self::JerseyColor,
            Self::BikeFrame,
            Self::UiTheme,
            Self::AccentColor,
            Self::ProfileBadge,
            Self::WheelStyle,
            Self::HelmetStyle,
        ]
    }
}

/// A reward that can be unlocked through career progression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    /// Unique identifier
    pub id: String,
    /// Reward type
    pub reward_type: RewardType,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Level required to unlock
    pub unlock_level: u32,
    /// Whether this is a milestone reward (special unlock animation)
    pub is_milestone: bool,
    /// Optional color value (for color-type rewards)
    pub color: Option<String>,
    /// Optional asset path (for visual rewards)
    pub asset_path: Option<String>,
}

impl Reward {
    /// Create a new reward.
    pub fn new(
        id: impl Into<String>,
        reward_type: RewardType,
        name: impl Into<String>,
        description: impl Into<String>,
        unlock_level: u32,
    ) -> Self {
        Self {
            id: id.into(),
            reward_type,
            name: name.into(),
            description: description.into(),
            unlock_level,
            is_milestone: false,
            color: None,
            asset_path: None,
        }
    }

    /// Mark this reward as a milestone reward.
    pub fn milestone(mut self) -> Self {
        self.is_milestone = true;
        self
    }

    /// Set the color for this reward.
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the asset path for this reward.
    pub fn with_asset(mut self, path: impl Into<String>) -> Self {
        self.asset_path = Some(path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_types() {
        let types = RewardType::all();
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_reward_creation() {
        let reward = Reward::new(
            "blue_jersey",
            RewardType::JerseyColor,
            "Blue Jersey",
            "A sleek blue racing jersey",
            5,
        )
        .with_color("#0066CC")
        .milestone();

        assert_eq!(reward.id, "blue_jersey");
        assert_eq!(reward.unlock_level, 5);
        assert!(reward.is_milestone);
        assert_eq!(reward.color, Some("#0066CC".to_string()));
    }
}
