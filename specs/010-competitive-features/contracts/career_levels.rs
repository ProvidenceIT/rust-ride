//! Career Levels Contract
//!
//! Defines the interface for career progression and rewards.

use chrono::{DateTime, Utc};

/// Maximum career level.
pub const MAX_LEVEL: u32 = 50;

/// XP curve base multiplier.
pub const XP_BASE: f64 = 1000.0;

/// XP curve growth rate.
pub const XP_GROWTH_RATE: f64 = 1.15;

/// Calculate XP required for a specific level.
pub fn xp_for_level(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    (XP_BASE * XP_GROWTH_RATE.powi(level as i32 - 1)) as u64
}

/// Calculate cumulative XP to reach a level from level 1.
pub fn cumulative_xp_to_level(level: u32) -> u64 {
    (1..level).map(xp_for_level).sum()
}

/// Calculate level from total XP.
pub fn level_from_xp(total_xp: u64) -> u32 {
    let mut level = 1u32;
    let mut cumulative = 0u64;

    while level < MAX_LEVEL {
        let next_level_xp = xp_for_level(level + 1);
        if cumulative + next_level_xp > total_xp {
            break;
        }
        cumulative += next_level_xp;
        level += 1;
    }

    level
}

/// User's career progression status.
#[derive(Debug, Clone)]
pub struct CareerStatus {
    /// Total accumulated XP
    pub total_xp: u64,
    /// Current level (1-50)
    pub current_level: u32,
    /// XP into current level
    pub xp_in_current_level: u64,
    /// XP needed to reach next level
    pub xp_to_next_level: u64,
    /// Progress to next level (0.0-1.0)
    pub level_progress: f32,
    /// Whether at max level
    pub is_max_level: bool,
}

/// Type of unlockable reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A reward that can be unlocked.
#[derive(Debug, Clone)]
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
}

/// Record of an unlocked reward.
#[derive(Debug, Clone)]
pub struct UnlockedReward {
    /// The reward definition
    pub reward: Reward,
    /// When unlocked
    pub unlocked_at: DateTime<Utc>,
    /// Level when unlocked
    pub unlocked_at_level: u32,
}

/// Level up event for notifications.
#[derive(Debug, Clone)]
pub struct LevelUpEvent {
    /// Previous level
    pub old_level: u32,
    /// New level
    pub new_level: u32,
    /// Rewards unlocked at new level
    pub new_rewards: Vec<Reward>,
    /// Whether this is a milestone level (5, 10, 25, 50)
    pub is_milestone: bool,
}

/// Milestone levels for special rewards.
pub const MILESTONE_LEVELS: &[u32] = &[5, 10, 15, 20, 25, 30, 40, 50];

/// Manages career progression and rewards.
pub trait CareerManager: Send + Sync {
    /// Get current career status.
    fn get_career_status(&self) -> CareerStatus;

    /// Add XP and check for level up.
    ///
    /// # Returns
    /// Level up event if user leveled up
    fn add_xp(&mut self, xp: u32) -> Option<LevelUpEvent>;

    /// Get all rewards (locked and unlocked).
    fn get_all_rewards(&self) -> Vec<Reward>;

    /// Get rewards available at a specific level.
    fn get_rewards_at_level(&self, level: u32) -> Vec<Reward>;

    /// Get all unlocked rewards for user.
    fn get_unlocked_rewards(&self) -> Vec<UnlockedReward>;

    /// Check if a reward is unlocked.
    fn is_reward_unlocked(&self, reward_id: &str) -> bool;

    /// Get rewards that will be unlocked at next level.
    fn get_next_level_rewards(&self) -> Vec<Reward>;

    /// Apply a reward (e.g., set as active jersey color).
    fn apply_reward(&mut self, reward_id: &str) -> Result<(), CareerError>;

    /// Get currently applied reward of a type.
    fn get_active_reward(&self, reward_type: RewardType) -> Option<UnlockedReward>;

    /// Check if level is a milestone level.
    fn is_milestone_level(level: u32) -> bool {
        MILESTONE_LEVELS.contains(&level)
    }
}

/// Errors from career operations.
#[derive(Debug, Clone)]
pub enum CareerError {
    /// Reward not found
    RewardNotFound(String),
    /// Reward not unlocked yet
    RewardLocked(String, u32), // (reward_id, required_level)
    /// Storage error
    StorageError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_curve() {
        // Level 1 requires 0 XP
        assert_eq!(xp_for_level(1), 0);

        // Level 2 requires 1000 XP
        assert_eq!(xp_for_level(2), 1000);

        // Level 10 should require ~4,000 XP
        let level_10 = xp_for_level(10);
        assert!(level_10 > 3500 && level_10 < 4500);

        // Level 50 should require ~500,000+ XP
        let level_50 = xp_for_level(50);
        assert!(level_50 > 400_000);
    }

    #[test]
    fn test_level_from_xp() {
        assert_eq!(level_from_xp(0), 1);
        assert_eq!(level_from_xp(500), 1);
        assert_eq!(level_from_xp(1000), 2);
        assert_eq!(level_from_xp(2500), 3);
    }

    #[test]
    fn test_cumulative_xp() {
        // Cumulative to level 1 is 0
        assert_eq!(cumulative_xp_to_level(1), 0);

        // Cumulative to level 2 is xp_for_level(2) = 1000
        assert_eq!(cumulative_xp_to_level(2), 1000);

        // Should be monotonically increasing
        let level_10 = cumulative_xp_to_level(10);
        let level_20 = cumulative_xp_to_level(20);
        assert!(level_20 > level_10);
    }
}
