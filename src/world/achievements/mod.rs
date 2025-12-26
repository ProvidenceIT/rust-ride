//! Achievement and collectible system.

pub mod collectibles;
pub mod definitions;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Achievement category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    /// Distance-based achievements
    Distance,
    /// Elevation-based achievements
    Climbing,
    /// Time/consistency achievements
    Consistency,
    /// Segment/leaderboard achievements
    Competition,
    /// Exploration achievements
    Exploration,
    /// Workout achievements
    Training,
    /// Social achievements
    Social,
    /// Special/seasonal achievements
    Special,
}

/// Achievement tier/difficulty
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AchievementTier {
    /// Easy to obtain
    Bronze,
    /// Moderate difficulty
    Silver,
    /// Challenging
    Gold,
    /// Very challenging
    Diamond,
    /// Expert level
    Legendary,
}

impl AchievementTier {
    /// Get XP reward for tier
    pub fn xp_reward(&self) -> u32 {
        match self {
            Self::Bronze => 100,
            Self::Silver => 250,
            Self::Gold => 500,
            Self::Diamond => 1000,
            Self::Legendary => 2500,
        }
    }
}

/// Achievement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier
    pub id: Uuid,
    /// Short code/key
    pub key: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: AchievementCategory,
    /// Difficulty tier
    pub tier: AchievementTier,
    /// Icon name
    pub icon: String,
    /// Whether hidden until unlocked
    pub is_secret: bool,
    /// Target value for progress (if applicable)
    pub target_value: Option<f64>,
}

impl Achievement {
    /// Create new achievement
    pub fn new(
        key: &str,
        name: &str,
        description: &str,
        category: AchievementCategory,
        tier: AchievementTier,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            key: key.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category,
            tier,
            icon: format!("achievement_{}", key),
            is_secret: false,
            target_value: None,
        }
    }

    /// Set as secret achievement
    pub fn secret(mut self) -> Self {
        self.is_secret = true;
        self
    }

    /// Set target value for progress tracking
    pub fn with_target(mut self, target: f64) -> Self {
        self.target_value = Some(target);
        self
    }
}

/// User's progress on an achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementProgress {
    /// Achievement ID
    pub achievement_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Current value (for progress-based achievements)
    pub current_value: f64,
    /// Whether unlocked
    pub is_unlocked: bool,
    /// When unlocked (if applicable)
    pub unlocked_at: Option<DateTime<Utc>>,
}

impl AchievementProgress {
    /// Create new progress tracker
    pub fn new(achievement_id: Uuid, user_id: Uuid) -> Self {
        Self {
            achievement_id,
            user_id,
            current_value: 0.0,
            is_unlocked: false,
            unlocked_at: None,
        }
    }

    /// Update progress value
    pub fn update(&mut self, value: f64) {
        self.current_value = value;
    }

    /// Add to progress value
    pub fn increment(&mut self, amount: f64) {
        self.current_value += amount;
    }

    /// Mark as unlocked
    pub fn unlock(&mut self) {
        if !self.is_unlocked {
            self.is_unlocked = true;
            self.unlocked_at = Some(Utc::now());
        }
    }

    /// Get progress percentage (0..100)
    pub fn percentage(&self, target: f64) -> f32 {
        if target <= 0.0 {
            return if self.is_unlocked { 100.0 } else { 0.0 };
        }
        ((self.current_value / target) * 100.0).min(100.0) as f32
    }
}

/// Achievement unlocked event
#[derive(Debug, Clone)]
pub struct AchievementUnlocked {
    /// Achievement that was unlocked
    pub achievement: Achievement,
    /// User who unlocked it
    pub user_id: Uuid,
    /// When unlocked
    pub unlocked_at: DateTime<Utc>,
    /// XP earned
    pub xp_earned: u32,
}

/// Achievement manager
pub struct AchievementManager {
    /// All available achievements
    achievements: Vec<Achievement>,
    /// User's progress on achievements
    progress: std::collections::HashMap<Uuid, AchievementProgress>,
    /// User ID
    user_id: Uuid,
    /// Total XP earned
    total_xp: u32,
    /// Queue of recently unlocked achievements
    unlock_queue: Vec<AchievementUnlocked>,
}

impl AchievementManager {
    /// Create achievement manager
    pub fn new(user_id: Uuid) -> Self {
        Self {
            achievements: definitions::all_achievements(),
            progress: std::collections::HashMap::new(),
            user_id,
            total_xp: 0,
            unlock_queue: Vec::new(),
        }
    }

    /// Get all achievements
    pub fn achievements(&self) -> &[Achievement] {
        &self.achievements
    }

    /// Get achievements by category
    pub fn by_category(&self, category: AchievementCategory) -> Vec<&Achievement> {
        self.achievements
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }

    /// Get progress for an achievement
    pub fn get_progress(&self, achievement_id: Uuid) -> Option<&AchievementProgress> {
        self.progress.get(&achievement_id)
    }

    /// Update progress for an achievement
    pub fn update_progress(&mut self, key: &str, value: f64) -> Option<AchievementUnlocked> {
        let achievement = self.achievements.iter().find(|a| a.key == key)?;
        let id = achievement.id;

        let progress = self
            .progress
            .entry(id)
            .or_insert_with(|| AchievementProgress::new(id, self.user_id));

        if progress.is_unlocked {
            return None;
        }

        progress.update(value);

        // Check if achievement is now complete
        if let Some(target) = achievement.target_value {
            if value >= target {
                return self.unlock_achievement(key);
            }
        }

        None
    }

    /// Increment progress for an achievement
    pub fn increment_progress(&mut self, key: &str, amount: f64) -> Option<AchievementUnlocked> {
        let achievement = self.achievements.iter().find(|a| a.key == key)?;
        let id = achievement.id;

        let progress = self
            .progress
            .entry(id)
            .or_insert_with(|| AchievementProgress::new(id, self.user_id));

        if progress.is_unlocked {
            return None;
        }

        progress.increment(amount);

        // Check if achievement is now complete
        if let Some(target) = achievement.target_value {
            if progress.current_value >= target {
                return self.unlock_achievement(key);
            }
        }

        None
    }

    /// Unlock an achievement immediately
    pub fn unlock_achievement(&mut self, key: &str) -> Option<AchievementUnlocked> {
        let achievement = self.achievements.iter().find(|a| a.key == key)?.clone();
        let id = achievement.id;

        let progress = self
            .progress
            .entry(id)
            .or_insert_with(|| AchievementProgress::new(id, self.user_id));

        if progress.is_unlocked {
            return None;
        }

        progress.unlock();
        let xp = achievement.tier.xp_reward();
        self.total_xp += xp;

        let unlocked = AchievementUnlocked {
            achievement,
            user_id: self.user_id,
            unlocked_at: Utc::now(),
            xp_earned: xp,
        };

        self.unlock_queue.push(unlocked.clone());
        Some(unlocked)
    }

    /// Get and clear pending unlock notifications
    pub fn pop_unlocks(&mut self) -> Vec<AchievementUnlocked> {
        std::mem::take(&mut self.unlock_queue)
    }

    /// Get total XP
    pub fn total_xp(&self) -> u32 {
        self.total_xp
    }

    /// Get unlocked count
    pub fn unlocked_count(&self) -> usize {
        self.progress.values().filter(|p| p.is_unlocked).count()
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.achievements.is_empty() {
            return 0.0;
        }
        (self.unlocked_count() as f32 / self.achievements.len() as f32) * 100.0
    }

    /// Load progress from storage
    pub fn load_progress(&mut self, progress: Vec<AchievementProgress>) {
        for p in progress {
            if p.is_unlocked {
                if let Some(ach) = self.achievements.iter().find(|a| a.id == p.achievement_id) {
                    self.total_xp += ach.tier.xp_reward();
                }
            }
            self.progress.insert(p.achievement_id, p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_tiers() {
        assert!(AchievementTier::Bronze < AchievementTier::Silver);
        assert!(AchievementTier::Gold.xp_reward() > AchievementTier::Silver.xp_reward());
    }

    #[test]
    fn test_achievement_progress() {
        let mut progress = AchievementProgress::new(Uuid::new_v4(), Uuid::new_v4());

        progress.update(50.0);
        assert_eq!(progress.percentage(100.0), 50.0);

        progress.increment(25.0);
        assert_eq!(progress.percentage(100.0), 75.0);

        progress.unlock();
        assert!(progress.is_unlocked);
        assert!(progress.unlocked_at.is_some());
    }

    #[test]
    fn test_achievement_manager() {
        let user_id = Uuid::new_v4();
        let mut manager = AchievementManager::new(user_id);

        // Should have some default achievements
        assert!(!manager.achievements().is_empty());

        // Find a distance achievement and update it
        if let Some(ach) = manager
            .achievements()
            .iter()
            .find(|a| a.key == "first_ride")
        {
            let unlocked = manager.unlock_achievement("first_ride");
            assert!(unlocked.is_some());
            assert!(manager.total_xp() > 0);
        }
    }
}
