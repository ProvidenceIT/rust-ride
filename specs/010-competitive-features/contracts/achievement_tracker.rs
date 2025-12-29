//! Achievement Tracker Contract
//!
//! Defines the interface for tracking achievements and XP.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// An achievement that can be earned.
#[derive(Debug, Clone)]
pub struct Achievement {
    /// Unique key identifier
    pub key: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: AchievementCategory,
    /// Tier determining base XP
    pub tier: AchievementTier,
    /// Target value for cumulative achievements
    pub target: Option<f64>,
    /// Whether hidden until earned
    pub is_secret: bool,
    /// XP awarded (overrides tier default if set)
    pub xp_value: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementCategory {
    Distance,
    Climbing,
    Consistency,
    Competition,
    Exploration,
    Training,
    Special,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementTier {
    Bronze,    // 100 XP
    Silver,    // 250 XP
    Gold,      // 500 XP
    Diamond,   // 1000 XP
    Legendary, // 2500 XP
}

impl AchievementTier {
    pub fn base_xp(&self) -> u32 {
        match self {
            AchievementTier::Bronze => 100,
            AchievementTier::Silver => 250,
            AchievementTier::Gold => 500,
            AchievementTier::Diamond => 1000,
            AchievementTier::Legendary => 2500,
        }
    }
}

/// A user's earned achievement record.
#[derive(Debug, Clone)]
pub struct EarnedAchievement {
    /// Achievement key
    pub achievement_key: String,
    /// When earned
    pub earned_at: DateTime<Utc>,
    /// Triggering ride (if applicable)
    pub ride_id: Option<Uuid>,
    /// XP awarded
    pub xp_awarded: u32,
}

/// A pending achievement notification.
#[derive(Debug, Clone)]
pub struct AchievementNotification {
    /// Achievement that was earned
    pub achievement: Achievement,
    /// XP awarded
    pub xp_awarded: u32,
    /// When earned
    pub earned_at: DateTime<Utc>,
}

/// User's XP and level status.
#[derive(Debug, Clone)]
pub struct UserLevel {
    /// Total accumulated XP
    pub total_xp: u64,
    /// Current level (1-50)
    pub current_level: u32,
    /// XP needed to reach next level
    pub xp_to_next_level: u64,
    /// Progress to next level (0.0-1.0)
    pub level_progress: f32,
}

/// Tracks achievement progress and awards.
pub trait AchievementTracker: Send + Sync {
    /// Check for newly earned achievements based on ride data.
    ///
    /// # Arguments
    /// * `ride_id` - UUID of completed ride
    /// * `ride_data` - Metrics from the ride
    ///
    /// # Returns
    /// List of newly earned achievements
    fn check_ride_achievements(
        &mut self,
        ride_id: Uuid,
        ride_data: &RideMetrics,
    ) -> Vec<EarnedAchievement>;

    /// Check for cumulative achievements (distance, time, etc.).
    fn check_cumulative_achievements(&mut self) -> Vec<EarnedAchievement>;

    /// Queue a notification for later display.
    fn queue_notification(&mut self, notification: AchievementNotification);

    /// Get pending notifications (for break points).
    fn get_pending_notifications(&mut self) -> Vec<AchievementNotification>;

    /// Check if there are pending notifications.
    fn has_pending_notifications(&self) -> bool;

    /// Get user's current XP and level.
    fn get_user_level(&self) -> UserLevel;

    /// Award XP and check for level up.
    ///
    /// # Returns
    /// New level if user leveled up
    fn award_xp(&mut self, xp: u32) -> Option<u32>;

    /// Get all earned achievements for a user.
    fn get_earned_achievements(&self) -> Vec<EarnedAchievement>;

    /// Get progress on a cumulative achievement.
    fn get_achievement_progress(&self, key: &str) -> Option<(f64, f64)>; // (current, target)

    /// Get all achievement definitions.
    fn get_all_achievements(&self) -> &[Achievement];
}

/// Ride metrics needed for achievement checks.
#[derive(Debug, Clone, Default)]
pub struct RideMetrics {
    /// Ride duration (seconds)
    pub duration_secs: u32,
    /// Distance (meters)
    pub distance_m: f64,
    /// Elevation gain (meters)
    pub elevation_m: f64,
    /// Average power (watts)
    pub avg_power: Option<u16>,
    /// Max power (watts)
    pub max_power: Option<u16>,
    /// Normalized power (watts)
    pub normalized_power: Option<u16>,
    /// TSS
    pub tss: Option<f32>,
    /// Whether a structured workout was completed
    pub completed_workout: bool,
    /// Power records set (duration_secs, watts)
    pub power_records: Vec<(u32, u16)>,
}
