//! Career status tracking.
//!
//! T071: Create CareerStatus struct.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::rewards::Reward;
use super::xp_curve::{cumulative_xp_to_level, level_from_xp, xp_for_level, MAX_LEVEL};

/// Current career status for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerStatus {
    /// User identifier.
    pub user_id: i64,
    /// Total accumulated XP.
    pub total_xp: u64,
    /// Current career level (1-50).
    pub current_level: u32,
    /// XP progress into current level.
    pub xp_into_level: u64,
    /// XP needed to reach next level.
    pub xp_to_next: u64,
    /// Progress percentage (0.0 to 1.0).
    pub progress: f32,
    /// Title for current level.
    pub level_title: String,
    /// When XP was last updated.
    pub updated_at: DateTime<Utc>,
    /// IDs of unlocked rewards.
    pub unlocked_rewards: Vec<String>,
}

impl CareerStatus {
    /// Create a new career status from XP.
    pub fn new(user_id: i64, total_xp: u64) -> Self {
        let current_level = level_from_xp(total_xp);
        let xp_at_level = cumulative_xp_to_level(current_level);
        let xp_into_level = total_xp.saturating_sub(xp_at_level);
        let xp_to_next = if current_level >= MAX_LEVEL {
            0
        } else {
            xp_for_level(current_level + 1).saturating_sub(xp_into_level)
        };

        let progress = if current_level >= MAX_LEVEL {
            1.0
        } else {
            let xp_needed = xp_for_level(current_level + 1);
            if xp_needed > 0 {
                xp_into_level as f32 / xp_needed as f32
            } else {
                0.0
            }
        };

        Self {
            user_id,
            total_xp,
            current_level,
            xp_into_level,
            xp_to_next,
            progress,
            level_title: level_title(current_level).to_string(),
            updated_at: Utc::now(),
            unlocked_rewards: Vec::new(),
        }
    }

    /// Create with unlocked rewards.
    pub fn with_rewards(mut self, rewards: Vec<String>) -> Self {
        self.unlocked_rewards = rewards;
        self
    }

    /// Update from new XP total.
    pub fn update(&mut self, total_xp: u64) {
        let new_level = level_from_xp(total_xp);
        self.total_xp = total_xp;
        self.current_level = new_level;

        let xp_at_level = cumulative_xp_to_level(new_level);
        self.xp_into_level = total_xp.saturating_sub(xp_at_level);

        if new_level >= MAX_LEVEL {
            self.xp_to_next = 0;
            self.progress = 1.0;
        } else {
            let xp_needed = xp_for_level(new_level + 1);
            self.xp_to_next = xp_needed.saturating_sub(self.xp_into_level);
            self.progress = if xp_needed > 0 {
                self.xp_into_level as f32 / xp_needed as f32
            } else {
                0.0
            };
        }

        self.level_title = level_title(new_level).to_string();
        self.updated_at = Utc::now();
    }

    /// Add XP and return new rewards.
    pub fn add_xp(&mut self, xp: u64, all_rewards: &[Reward]) -> XpGainResult {
        let old_level = self.current_level;
        let new_total = self.total_xp + xp;

        self.update(new_total);

        let levels_gained = self.current_level.saturating_sub(old_level);
        let is_level_up = levels_gained > 0;

        // Find newly unlocked rewards
        let new_rewards: Vec<Reward> = all_rewards
            .iter()
            .filter(|r| r.unlock_level > old_level && r.unlock_level <= self.current_level)
            .cloned()
            .collect();

        // Add new reward IDs to unlocked list
        for reward in &new_rewards {
            if !self.unlocked_rewards.contains(&reward.id) {
                self.unlocked_rewards.push(reward.id.clone());
            }
        }

        XpGainResult {
            xp_gained: xp,
            new_total,
            old_level,
            new_level: self.current_level,
            levels_gained,
            is_level_up,
            new_rewards,
        }
    }

    /// Check if the user has reached max level.
    pub fn is_max_level(&self) -> bool {
        self.current_level >= MAX_LEVEL
    }

    /// Get a progress bar representation (0.0 to 1.0).
    pub fn progress_bar(&self) -> f32 {
        self.progress.clamp(0.0, 1.0)
    }

    /// Get XP formatted for display.
    pub fn xp_display(&self) -> String {
        if self.is_max_level() {
            format!("{} XP (Max Level)", format_xp(self.total_xp))
        } else {
            format!(
                "{} / {} XP",
                format_xp(self.xp_into_level),
                format_xp(xp_for_level(self.current_level + 1))
            )
        }
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Level {} {} ({} XP)",
            self.current_level,
            self.level_title,
            format_xp(self.total_xp)
        )
    }
}

/// Result of gaining XP.
#[derive(Debug, Clone)]
pub struct XpGainResult {
    /// XP gained.
    pub xp_gained: u64,
    /// New total XP.
    pub new_total: u64,
    /// Level before XP gain.
    pub old_level: u32,
    /// Level after XP gain.
    pub new_level: u32,
    /// Number of levels gained.
    pub levels_gained: u32,
    /// Whether this resulted in a level up.
    pub is_level_up: bool,
    /// Rewards unlocked by this level up.
    pub new_rewards: Vec<Reward>,
}

impl XpGainResult {
    /// Get display message for the XP gain.
    pub fn message(&self) -> String {
        if self.is_level_up {
            if self.levels_gained > 1 {
                format!(
                    "+{} XP! {} levels gained! Now level {}",
                    self.xp_gained, self.levels_gained, self.new_level
                )
            } else {
                format!(
                    "+{} XP! Level Up! Now level {}",
                    self.xp_gained, self.new_level
                )
            }
        } else {
            format!("+{} XP", self.xp_gained)
        }
    }

    /// Check if rewards were unlocked.
    pub fn has_rewards(&self) -> bool {
        !self.new_rewards.is_empty()
    }
}

/// Get title for a level.
pub fn level_title(level: u32) -> &'static str {
    match level {
        1 => "Beginner",
        2..=4 => "Novice",
        5..=9 => "Enthusiast",
        10..=14 => "Regular",
        15..=19 => "Dedicated",
        20..=24 => "Committed",
        25..=29 => "Veteran",
        30..=34 => "Expert",
        35..=39 => "Elite",
        40..=44 => "Champion",
        45..=49 => "Master",
        50 => "Legend",
        _ => "Unknown",
    }
}

/// Format XP for display.
fn format_xp(xp: u64) -> String {
    if xp >= 1_000_000 {
        format!("{:.1}M", xp as f64 / 1_000_000.0)
    } else if xp >= 1_000 {
        format!("{:.1}K", xp as f64 / 1_000.0)
    } else {
        xp.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::RewardType;

    #[test]
    fn test_career_status_new() {
        let status = CareerStatus::new(1, 0);

        assert_eq!(status.current_level, 1);
        assert_eq!(status.total_xp, 0);
        assert_eq!(status.level_title, "Beginner");
    }

    #[test]
    fn test_career_status_update() {
        let mut status = CareerStatus::new(1, 0);

        // Add enough XP to reach level 2
        status.update(1200);

        assert_eq!(status.current_level, 2);
        assert!(status.xp_into_level > 0);
    }

    #[test]
    fn test_add_xp_level_up() {
        let mut status = CareerStatus::new(1, 1100);
        let rewards = vec![Reward::new(
            "test_reward",
            RewardType::JerseyColor,
            "Test",
            "Test reward",
            2,
        )];

        let result = status.add_xp(100, &rewards);

        assert!(result.is_level_up);
        assert_eq!(result.old_level, 1);
        assert_eq!(result.new_level, 2);
        assert_eq!(result.new_rewards.len(), 1);
    }

    #[test]
    fn test_level_titles() {
        assert_eq!(level_title(1), "Beginner");
        assert_eq!(level_title(10), "Regular");
        assert_eq!(level_title(25), "Veteran");
        assert_eq!(level_title(50), "Legend");
    }

    #[test]
    fn test_format_xp() {
        assert_eq!(format_xp(500), "500");
        assert_eq!(format_xp(1500), "1.5K");
        assert_eq!(format_xp(1_500_000), "1.5M");
    }

    #[test]
    fn test_xp_display() {
        let status = CareerStatus::new(1, 500);
        let display = status.xp_display();
        assert!(display.contains("500"));
    }
}
