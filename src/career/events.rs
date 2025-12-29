//! Career event definitions.
//!
//! T072: Create LevelUpEvent and UnlockedReward structs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::rewards::Reward;

/// Event fired when the user levels up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpEvent {
    /// Previous level.
    pub old_level: u32,
    /// New level.
    pub new_level: u32,
    /// Number of levels gained.
    pub levels_gained: u32,
    /// New level title.
    pub level_title: String,
    /// XP that triggered the level up.
    pub xp_gained: u64,
    /// Total XP after level up.
    pub total_xp: u64,
    /// When the level up occurred.
    pub occurred_at: DateTime<Utc>,
    /// Rewards unlocked at this level.
    pub unlocked_rewards: Vec<UnlockedReward>,
    /// Whether this is a milestone level (10, 20, 30, 40, 50).
    pub is_milestone: bool,
}

impl LevelUpEvent {
    /// Create a new level up event.
    pub fn new(old_level: u32, new_level: u32, xp_gained: u64, total_xp: u64) -> Self {
        Self {
            old_level,
            new_level,
            levels_gained: new_level.saturating_sub(old_level),
            level_title: super::status::level_title(new_level).to_string(),
            xp_gained,
            total_xp,
            occurred_at: Utc::now(),
            unlocked_rewards: Vec::new(),
            is_milestone: is_milestone_level(new_level),
        }
    }

    /// Add unlocked rewards to the event.
    pub fn with_rewards(mut self, rewards: Vec<UnlockedReward>) -> Self {
        self.unlocked_rewards = rewards;
        self
    }

    /// Get notification message.
    pub fn notification_message(&self) -> String {
        if self.is_milestone {
            format!(
                "🎉 Milestone Level {}! You're now a {}!",
                self.new_level, self.level_title
            )
        } else if self.levels_gained > 1 {
            format!(
                "Level {}! Gained {} levels!",
                self.new_level, self.levels_gained
            )
        } else {
            format!("Level {}! {}", self.new_level, self.level_title)
        }
    }

    /// Get celebratory message based on level.
    pub fn celebration_message(&self) -> &'static str {
        match self.new_level {
            1..=9 => "Keep riding!",
            10 => "Double digits! You're getting serious!",
            20 => "Dedicated cyclist status achieved!",
            25 => "Quarter century! You're a veteran now!",
            30 => "Thirty levels of determination!",
            40 => "Champion level! Elite status!",
            50 => "MAXIMUM LEVEL! You are a LEGEND!",
            _ => "Great progress!",
        }
    }

    /// Check if any rewards were unlocked.
    pub fn has_rewards(&self) -> bool {
        !self.unlocked_rewards.is_empty()
    }
}

/// A reward that was just unlocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockedReward {
    /// Reward ID.
    pub reward_id: String,
    /// Reward type.
    pub reward_type: super::rewards::RewardType,
    /// Reward name.
    pub name: String,
    /// Reward description.
    pub description: String,
    /// Level at which it was unlocked.
    pub unlocked_at_level: u32,
    /// When it was unlocked.
    pub unlocked_at: DateTime<Utc>,
    /// Whether this was a milestone reward.
    pub is_milestone: bool,
    /// Optional color value.
    pub color: Option<String>,
    /// Optional asset path.
    pub asset_path: Option<String>,
}

impl UnlockedReward {
    /// Create from a reward definition.
    pub fn from_reward(reward: &Reward, unlocked_at_level: u32) -> Self {
        Self {
            reward_id: reward.id.clone(),
            reward_type: reward.reward_type,
            name: reward.name.clone(),
            description: reward.description.clone(),
            unlocked_at_level,
            unlocked_at: Utc::now(),
            is_milestone: reward.is_milestone,
            color: reward.color.clone(),
            asset_path: reward.asset_path.clone(),
        }
    }

    /// Get notification message.
    pub fn notification_message(&self) -> String {
        if self.is_milestone {
            format!("🏆 Milestone Unlock: {}!", self.name)
        } else {
            format!("🎁 Unlocked: {}", self.name)
        }
    }
}

/// Check if a level is a milestone level.
pub fn is_milestone_level(level: u32) -> bool {
    matches!(level, 10 | 20 | 25 | 30 | 40 | 50)
}

/// Queue for pending career events.
#[derive(Debug, Default)]
pub struct CareerEventQueue {
    /// Pending level up events.
    level_ups: Vec<LevelUpEvent>,
    /// Pending reward unlocks not part of a level up.
    rewards: Vec<UnlockedReward>,
}

impl CareerEventQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a level up event.
    pub fn push_level_up(&mut self, event: LevelUpEvent) {
        self.level_ups.push(event);
    }

    /// Add a reward unlock.
    pub fn push_reward(&mut self, reward: UnlockedReward) {
        self.rewards.push(reward);
    }

    /// Pop the next level up event.
    pub fn pop_level_up(&mut self) -> Option<LevelUpEvent> {
        if !self.level_ups.is_empty() {
            Some(self.level_ups.remove(0))
        } else {
            None
        }
    }

    /// Pop the next reward.
    pub fn pop_reward(&mut self) -> Option<UnlockedReward> {
        if !self.rewards.is_empty() {
            Some(self.rewards.remove(0))
        } else {
            None
        }
    }

    /// Check if there are pending events.
    pub fn has_pending(&self) -> bool {
        !self.level_ups.is_empty() || !self.rewards.is_empty()
    }

    /// Check if there are pending level ups.
    pub fn has_level_ups(&self) -> bool {
        !self.level_ups.is_empty()
    }

    /// Get count of pending events.
    pub fn pending_count(&self) -> usize {
        self.level_ups.len() + self.rewards.len()
    }

    /// Clear all pending events.
    pub fn clear(&mut self) {
        self.level_ups.clear();
        self.rewards.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::RewardType;

    #[test]
    fn test_level_up_event() {
        let event = LevelUpEvent::new(9, 10, 500, 10000);

        assert_eq!(event.old_level, 9);
        assert_eq!(event.new_level, 10);
        assert_eq!(event.levels_gained, 1);
        assert!(event.is_milestone);
    }

    #[test]
    fn test_multi_level_up() {
        let event = LevelUpEvent::new(5, 8, 3000, 15000);

        assert_eq!(event.levels_gained, 3);
        assert!(!event.is_milestone);
    }

    #[test]
    fn test_milestone_levels() {
        assert!(!is_milestone_level(5));
        assert!(is_milestone_level(10));
        assert!(is_milestone_level(25));
        assert!(is_milestone_level(50));
    }

    #[test]
    fn test_unlocked_reward() {
        let reward = Reward::new("test", RewardType::JerseyColor, "Test", "Test desc", 10)
            .with_color("#FF0000")
            .milestone();

        let unlocked = UnlockedReward::from_reward(&reward, 10);

        assert_eq!(unlocked.reward_id, "test");
        assert!(unlocked.is_milestone);
        assert_eq!(unlocked.color, Some("#FF0000".to_string()));
    }

    #[test]
    fn test_event_queue() {
        let mut queue = CareerEventQueue::new();

        assert!(!queue.has_pending());

        queue.push_level_up(LevelUpEvent::new(1, 2, 1000, 1000));
        assert!(queue.has_pending());
        assert!(queue.has_level_ups());

        let event = queue.pop_level_up();
        assert!(event.is_some());
        assert!(!queue.has_level_ups());
    }

    #[test]
    fn test_celebration_messages() {
        assert_eq!(LevelUpEvent::new(49, 50, 1000, 500000).celebration_message(), "MAXIMUM LEVEL! You are a LEGEND!");
        assert_eq!(LevelUpEvent::new(9, 10, 1000, 10000).celebration_message(), "Double digits! You're getting serious!");
    }
}
