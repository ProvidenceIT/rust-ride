//! Achievement and XP system module.
//!
//! Provides gamification through badges, XP accumulation, and level progression.
//! Achievements are awarded for completing milestones during training.

mod achievement;
pub mod checks;
pub mod definitions;
mod earned;
mod notifications;
mod tracker;
mod types;
mod xp;

pub use achievement::Achievement;
pub use checks::{
    AchievementChecker, AllCheckers, ConsistencyChecker, CumulativeChecker, RideChecker,
};
pub use definitions::{
    achievement_by_name, achievement_count, achievements_by_category, achievements_by_tier,
    all_achievements, secret_achievements, total_possible_xp,
};
pub use earned::{AchievementProgress, AchievementSummary, EarnedAchievement, RideMetrics};
pub use notifications::{AchievementNotification, LevelUpNotification, NotificationQueue};
pub use tracker::{AchievementTracker, CumulativeStats, DefaultAchievementTracker};
pub use types::{AchievementCategory, AchievementTier, SECRET_XP_MULTIPLIER};
pub use xp::{
    xp_from_ride, xp_from_workout, XpAddResult, XpGain, XpMultiplier, XpSource, XpStatus,
};
