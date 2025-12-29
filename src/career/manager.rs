//! Career manager implementation.
//!
//! T075: Create CareerManager trait implementation.

use rusqlite::Connection;

use super::cosmetics::{CosmeticInventory, CosmeticType, EquippedCosmetics};
use super::events::{CareerEventQueue, LevelUpEvent, UnlockedReward};
use super::levels::{all_rewards, rewards_for_level, rewards_up_to_level};
use super::rewards::Reward;
use super::status::{CareerStatus, XpGainResult};
use crate::storage::rewards_store::RewardsStore;
use crate::storage::xp_store::XpStore;

/// Error type for career operations.
#[derive(Debug)]
pub enum CareerError {
    /// Database error.
    Database(String),
    /// XP operation error.
    XpError(String),
    /// Invalid operation.
    InvalidOperation(String),
}

impl std::fmt::Display for CareerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::XpError(msg) => write!(f, "XP error: {}", msg),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for CareerError {}

impl From<rusqlite::Error> for CareerError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}

/// Result type for career operations.
pub type CareerResult<T> = Result<T, CareerError>;

/// Manager for career progression.
#[derive(Debug)]
pub struct CareerManager {
    /// Current user ID.
    user_id: i64,
    /// Current career status.
    status: CareerStatus,
    /// Cosmetic inventory.
    inventory: CosmeticInventory,
    /// Pending events queue.
    event_queue: CareerEventQueue,
    /// All available rewards.
    all_rewards: Vec<Reward>,
}

impl CareerManager {
    /// Create a new career manager.
    pub fn new(user_id: i64) -> Self {
        Self {
            user_id,
            status: CareerStatus::new(user_id, 0),
            inventory: CosmeticInventory::with_defaults(),
            event_queue: CareerEventQueue::new(),
            all_rewards: all_rewards(),
        }
    }

    /// Create a builder.
    pub fn builder() -> CareerManagerBuilder {
        CareerManagerBuilder::default()
    }

    /// Load current state from database.
    pub fn load(&mut self, conn: &Connection) -> CareerResult<()> {
        // Load XP
        let xp_record = XpStore::get_or_create(conn, self.user_id)?;
        let total_xp = xp_record.total_xp;

        self.status = CareerStatus::new(self.user_id, total_xp);

        // Load unlocked rewards
        let reward_records = RewardsStore::get_unlocked(conn, self.user_id)?;
        for record in reward_records {
            self.inventory.unlock(&record.reward_id);
            self.status.unlocked_rewards.push(record.reward_id);
        }

        // Ensure all rewards up to current level are unlocked
        let should_unlock = rewards_up_to_level(self.status.current_level);
        for reward in should_unlock {
            if !self.inventory.is_unlocked(&reward.id) {
                self.inventory.unlock(&reward.id);
                self.status.unlocked_rewards.push(reward.id.clone());
                // Persist the reward
                let _ = RewardsStore::record_unlock(
                    conn,
                    self.user_id,
                    reward.reward_type.to_string(),
                    &reward.id,
                    self.status.current_level,
                );
            }
        }

        Ok(())
    }

    /// Save current state to database.
    pub fn save(&self, conn: &Connection) -> CareerResult<()> {
        XpStore::update(conn, self.user_id, self.status.total_xp, self.status.current_level)?;
        Ok(())
    }

    /// Add XP and process level ups.
    pub fn add_xp(&mut self, conn: &Connection, xp: u64) -> CareerResult<XpGainResult> {
        let result = self.status.add_xp(xp, &self.all_rewards);

        // Process level up if it happened
        if result.is_level_up {
            // Create level up event
            let mut event = LevelUpEvent::new(
                result.old_level,
                result.new_level,
                result.xp_gained,
                result.new_total,
            );

            // Unlock rewards
            for reward in &result.new_rewards {
                let unlocked = UnlockedReward::from_reward(reward, self.status.current_level);
                event.unlocked_rewards.push(unlocked);

                // Persist reward
                RewardsStore::record_unlock(
                    conn,
                    self.user_id,
                    reward.reward_type.to_string(),
                    &reward.id,
                    self.status.current_level,
                )?;

                // Add to inventory
                self.inventory.unlock(&reward.id);
            }

            self.event_queue.push_level_up(event);
        }

        // Save XP
        self.save(conn)?;

        Ok(result)
    }

    /// Get current career status.
    pub fn status(&self) -> &CareerStatus {
        &self.status
    }

    /// Get cosmetic inventory.
    pub fn inventory(&self) -> &CosmeticInventory {
        &self.inventory
    }

    /// Get equipped cosmetics.
    pub fn equipped(&self) -> &EquippedCosmetics {
        self.inventory.equipped()
    }

    /// Equip a cosmetic item.
    pub fn equip(&mut self, item_type: CosmeticType, item_id: &str) -> bool {
        self.inventory.equip(item_type, item_id)
    }

    /// Check if there are pending events.
    pub fn has_pending_events(&self) -> bool {
        self.event_queue.has_pending()
    }

    /// Pop the next level up event.
    pub fn pop_level_up(&mut self) -> Option<LevelUpEvent> {
        self.event_queue.pop_level_up()
    }

    /// Get rewards available at the next level.
    pub fn next_level_rewards(&self) -> Vec<Reward> {
        if self.status.is_max_level() {
            Vec::new()
        } else {
            rewards_for_level(self.status.current_level + 1)
        }
    }

    /// Get all rewards the user hasn't unlocked yet.
    pub fn locked_rewards(&self) -> Vec<&Reward> {
        self.all_rewards
            .iter()
            .filter(|r| !self.inventory.is_unlocked(&r.id))
            .collect()
    }

    /// Get progress towards next milestone.
    pub fn milestone_progress(&self) -> Option<MilestoneProgress> {
        super::levels::next_milestone(self.status.current_level).map(|target| {
            let levels_remaining = target.saturating_sub(self.status.current_level);
            let levels_since_last = match target {
                10 => self.status.current_level,
                20 => self.status.current_level.saturating_sub(10),
                25 => self.status.current_level.saturating_sub(20),
                30 => self.status.current_level.saturating_sub(25),
                40 => self.status.current_level.saturating_sub(30),
                50 => self.status.current_level.saturating_sub(40),
                _ => self.status.current_level,
            };
            let total_levels = levels_remaining + levels_since_last;
            let progress = if total_levels > 0 {
                levels_since_last as f32 / total_levels as f32
            } else {
                0.0
            };

            MilestoneProgress {
                target_level: target,
                levels_remaining,
                progress,
            }
        })
    }
}

/// Builder for CareerManager.
#[derive(Default)]
pub struct CareerManagerBuilder {
    user_id: Option<i64>,
}

impl CareerManagerBuilder {
    /// Set user ID.
    pub fn user_id(mut self, id: i64) -> Self {
        self.user_id = Some(id);
        self
    }

    /// Build the manager.
    pub fn build(self) -> CareerManager {
        CareerManager::new(self.user_id.unwrap_or(1))
    }

    /// Build and load from database.
    pub fn build_and_load(self, conn: &Connection) -> CareerResult<CareerManager> {
        let mut manager = self.build();
        manager.load(conn)?;
        Ok(manager)
    }
}

/// Progress towards next milestone level.
#[derive(Debug, Clone)]
pub struct MilestoneProgress {
    /// Target milestone level.
    pub target_level: u32,
    /// Levels remaining to milestone.
    pub levels_remaining: u32,
    /// Progress as 0.0-1.0.
    pub progress: f32,
}

impl MilestoneProgress {
    /// Get display string.
    pub fn display(&self) -> String {
        format!(
            "{} levels to Level {} milestone",
            self.levels_remaining, self.target_level
        )
    }
}

/// Helper to convert RewardType to string for storage.
trait RewardTypeExt {
    fn to_string(&self) -> &'static str;
}

impl RewardTypeExt for super::rewards::RewardType {
    fn to_string(&self) -> &'static str {
        match self {
            super::rewards::RewardType::JerseyColor => "jersey_color",
            super::rewards::RewardType::BikeFrame => "bike_frame",
            super::rewards::RewardType::UiTheme => "ui_theme",
            super::rewards::RewardType::AccentColor => "accent_color",
            super::rewards::RewardType::ProfileBadge => "profile_badge",
            super::rewards::RewardType::WheelStyle => "wheel_style",
            super::rewards::RewardType::HelmetStyle => "helmet_style",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE user_xp (
                user_id INTEGER PRIMARY KEY,
                total_xp INTEGER NOT NULL DEFAULT 0,
                current_level INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();
        conn.execute(
            "CREATE TABLE user_rewards (
                user_id INTEGER NOT NULL,
                reward_type TEXT NOT NULL,
                reward_id TEXT NOT NULL,
                unlocked_at TEXT NOT NULL,
                unlocked_at_level INTEGER NOT NULL,
                PRIMARY KEY (user_id, reward_type, reward_id)
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_manager_creation() {
        let manager = CareerManager::new(1);

        assert_eq!(manager.status().current_level, 1);
        assert_eq!(manager.status().total_xp, 0);
    }

    #[test]
    fn test_builder() {
        let manager = CareerManager::builder()
            .user_id(42)
            .build();

        assert_eq!(manager.user_id, 42);
    }

    #[test]
    fn test_add_xp() {
        let conn = setup_test_db();
        let mut manager = CareerManager::new(1);

        let result = manager.add_xp(&conn, 500).unwrap();

        assert_eq!(result.xp_gained, 500);
        assert!(!result.is_level_up);
    }

    #[test]
    fn test_level_up() {
        let conn = setup_test_db();
        let mut manager = CareerManager::new(1);

        // Add enough XP to level up
        let result = manager.add_xp(&conn, 1500).unwrap();

        assert!(result.is_level_up);
        assert!(manager.has_pending_events());

        let event = manager.pop_level_up();
        assert!(event.is_some());
    }

    #[test]
    fn test_next_level_rewards() {
        let manager = CareerManager::new(1);
        let rewards = manager.next_level_rewards();

        // Level 2 has rewards
        assert!(!rewards.is_empty());
    }

    #[test]
    fn test_milestone_progress() {
        let manager = CareerManager::new(1);
        let progress = manager.milestone_progress();

        assert!(progress.is_some());
        let progress = progress.unwrap();
        assert_eq!(progress.target_level, 10);
        assert_eq!(progress.levels_remaining, 9);
    }

    #[test]
    fn test_equip() {
        let mut manager = CareerManager::new(1);

        // Can't equip locked item
        assert!(!manager.equip(CosmeticType::Jersey, "locked_jersey"));

        // Can equip default item
        assert!(manager.equip(CosmeticType::Jersey, "jersey_default"));
    }
}
