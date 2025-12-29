//! Career progression module for long-term gamification.
//!
//! Provides 50-level career progression with cosmetic rewards unlocked at
//! milestone levels. Builds on the XP system from achievements.

mod cosmetics;
mod events;
mod levels;
mod manager;
mod rewards;
mod status;
mod xp_curve;

pub use cosmetics::{CosmeticInventory, CosmeticItem, CosmeticType, EquippedCosmetics};
pub use events::{CareerEventQueue, LevelUpEvent, UnlockedReward, is_milestone_level};
pub use levels::{all_level_rewards, all_rewards, next_milestone, rewards_for_level, rewards_up_to_level, LevelDefinition};
pub use manager::{CareerManager, CareerManagerBuilder, CareerError, CareerResult, MilestoneProgress};
pub use rewards::{Reward, RewardType};
pub use status::{CareerStatus, XpGainResult, level_title};
pub use xp_curve::{cumulative_xp_to_level, level_from_xp, level_progress, xp_for_level, xp_to_next_level, MAX_LEVEL, XP_BASE, XP_GROWTH_RATE};
