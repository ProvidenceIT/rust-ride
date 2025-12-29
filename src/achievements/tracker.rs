//! Achievement tracker trait and implementation.
//!
//! T036: Create AchievementTracker trait implementation.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate, Utc};
use uuid::Uuid;

use super::achievement::Achievement;
use super::earned::{AchievementProgress, AchievementSummary, EarnedAchievement, RideMetrics};
use super::notifications::{AchievementNotification, NotificationQueue};
use super::xp::{XpAddResult, XpGain, XpSource, XpStatus};

/// Trait for achievement tracking.
pub trait AchievementTracker {
    /// Check if an achievement has been earned.
    fn is_earned(&self, achievement_id: Uuid) -> bool;

    /// Get progress for an achievement.
    fn get_progress(&self, achievement_id: Uuid) -> Option<AchievementProgress>;

    /// Award an achievement.
    fn award(&mut self, achievement: &Achievement, ride_id: Option<Uuid>) -> Option<EarnedAchievement>;

    /// Process ride completion and check for achievements.
    fn process_ride(&mut self, metrics: &RideMetrics) -> Vec<EarnedAchievement>;

    /// Get current XP status.
    fn xp_status(&self) -> XpStatus;

    /// Get achievement summary.
    fn summary(&self) -> AchievementSummary;
}

/// Cumulative statistics tracked for achievement evaluation.
#[derive(Debug, Clone, Default)]
pub struct CumulativeStats {
    /// Total distance ridden (km).
    pub total_distance_km: f64,
    /// Total elevation gained (m).
    pub total_elevation_m: f64,
    /// Total ride time (seconds).
    pub total_time_secs: u64,
    /// Total rides completed.
    pub total_rides: u32,
    /// Total workouts completed.
    pub total_workouts: u32,
    /// Current daily streak.
    pub current_streak: u32,
    /// Longest streak ever.
    pub longest_streak: u32,
    /// Last ride date (for streak calculation).
    pub last_ride_date: Option<NaiveDate>,
    /// Rides per weekday (Monday = 0).
    pub rides_by_weekday: [u32; 7],
    /// Maximum values.
    pub max_distance_km: f64,
    pub max_elevation_m: f64,
    pub max_power: u16,
    pub max_duration_mins: u32,
}

impl CumulativeStats {
    /// Update stats from a ride.
    pub fn update_from_ride(&mut self, metrics: &RideMetrics) {
        self.total_distance_km += metrics.distance_km;
        self.total_elevation_m += metrics.elevation_gain_m;
        self.total_time_secs += metrics.duration_secs as u64;
        self.total_rides += 1;

        if metrics.workout_completed {
            self.total_workouts += 1;
        }

        // Update maximums
        if metrics.distance_km > self.max_distance_km {
            self.max_distance_km = metrics.distance_km;
        }
        if metrics.elevation_gain_m > self.max_elevation_m {
            self.max_elevation_m = metrics.elevation_gain_m;
        }
        if let Some(max_power) = metrics.max_power {
            if max_power > self.max_power {
                self.max_power = max_power;
            }
        }
        let duration_mins = metrics.duration_secs / 60;
        if duration_mins > self.max_duration_mins {
            self.max_duration_mins = duration_mins;
        }

        // Update streak
        self.update_streak();
    }

    /// Update daily streak based on current date.
    fn update_streak(&mut self) {
        let today = Utc::now().date_naive();

        match self.last_ride_date {
            Some(last) => {
                let days_diff = (today - last).num_days();
                if days_diff == 1 {
                    // Consecutive day, extend streak
                    self.current_streak += 1;
                } else if days_diff > 1 {
                    // Streak broken
                    self.current_streak = 1;
                }
                // days_diff == 0 means same day, streak unchanged
            }
            None => {
                // First ride
                self.current_streak = 1;
            }
        }

        if self.current_streak > self.longest_streak {
            self.longest_streak = self.current_streak;
        }

        self.last_ride_date = Some(today);

        // Update weekday count
        let weekday = today.weekday().num_days_from_monday() as usize;
        self.rides_by_weekday[weekday] += 1;
    }
}

/// Default implementation of achievement tracker.
#[derive(Debug)]
pub struct DefaultAchievementTracker {
    /// User ID.
    user_id: Uuid,
    /// All known achievements.
    achievements: HashMap<Uuid, Achievement>,
    /// Earned achievement IDs.
    earned: HashMap<Uuid, EarnedAchievement>,
    /// Progress for in-progress achievements.
    progress: HashMap<Uuid, AchievementProgress>,
    /// Notification queue.
    notifications: NotificationQueue,
    /// Cumulative statistics.
    stats: CumulativeStats,
    /// Total XP.
    total_xp: u64,
}

impl DefaultAchievementTracker {
    /// Create a new tracker.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            achievements: HashMap::new(),
            earned: HashMap::new(),
            progress: HashMap::new(),
            notifications: NotificationQueue::new(),
            stats: CumulativeStats::default(),
            total_xp: 0,
        }
    }

    /// Load with existing state.
    pub fn with_state(
        user_id: Uuid,
        earned: Vec<EarnedAchievement>,
        stats: CumulativeStats,
        total_xp: u64,
    ) -> Self {
        let mut tracker = Self::new(user_id);
        tracker.stats = stats;
        tracker.total_xp = total_xp;

        for e in earned {
            tracker.earned.insert(e.achievement_id, e);
        }

        tracker
    }

    /// Register an achievement definition.
    pub fn register_achievement(&mut self, achievement: Achievement) {
        self.achievements.insert(achievement.id, achievement);
    }

    /// Register multiple achievements.
    pub fn register_achievements(&mut self, achievements: impl IntoIterator<Item = Achievement>) {
        for achievement in achievements {
            self.register_achievement(achievement);
        }
    }

    /// Get notification queue.
    pub fn notifications(&self) -> &NotificationQueue {
        &self.notifications
    }

    /// Get mutable notification queue.
    pub fn notifications_mut(&mut self) -> &mut NotificationQueue {
        &mut self.notifications
    }

    /// Get cumulative stats.
    pub fn stats(&self) -> &CumulativeStats {
        &self.stats
    }

    /// Add XP and return result.
    pub fn add_xp(&mut self, gain: XpGain) -> XpAddResult {
        let previous_xp = self.total_xp;
        self.total_xp += gain.final_xp as u64;
        XpAddResult::from_xp_change(previous_xp, self.total_xp, gain.final_xp)
    }

    /// Get all earned achievements.
    pub fn earned_achievements(&self) -> Vec<&EarnedAchievement> {
        self.earned.values().collect()
    }

    /// Get achievement by ID.
    pub fn get_achievement(&self, id: Uuid) -> Option<&Achievement> {
        self.achievements.get(&id)
    }
}

impl AchievementTracker for DefaultAchievementTracker {
    fn is_earned(&self, achievement_id: Uuid) -> bool {
        self.earned.contains_key(&achievement_id)
    }

    fn get_progress(&self, achievement_id: Uuid) -> Option<AchievementProgress> {
        self.progress.get(&achievement_id).cloned()
    }

    fn award(&mut self, achievement: &Achievement, ride_id: Option<Uuid>) -> Option<EarnedAchievement> {
        // Check if already earned (unless repeatable)
        if !achievement.repeatable && self.is_earned(achievement.id) {
            return None;
        }

        // Create earned record
        let xp_awarded = achievement.effective_xp();
        let mut earned = EarnedAchievement::new(self.user_id, achievement.id, xp_awarded);

        if let Some(rid) = ride_id {
            earned = earned.with_ride(rid);
        }

        // Store and add XP
        self.earned.insert(achievement.id, earned.clone());
        let xp_gain = XpGain::new(xp_awarded, XpSource::Achievement)
            .with_description(&achievement.title);
        self.add_xp(xp_gain);

        // Create notification
        let notification = AchievementNotification::new(
            achievement.id,
            &achievement.title,
            &achievement.description,
            achievement.category,
            achievement.tier,
            xp_awarded,
        );
        self.notifications.push(notification);

        Some(earned)
    }

    fn process_ride(&mut self, metrics: &RideMetrics) -> Vec<EarnedAchievement> {
        // Update cumulative stats
        self.stats.update_from_ride(metrics);

        // Award XP for the ride
        let ride_xp = super::xp::xp_from_ride(
            metrics.distance_km,
            metrics.duration_secs / 60,
            metrics.elevation_gain_m,
        );
        let xp_gain = XpGain::new(ride_xp, XpSource::Ride);
        self.add_xp(xp_gain);

        // Check achievements (to be implemented by check modules)
        // For now, return empty - actual checks will be done by check modules
        Vec::new()
    }

    fn xp_status(&self) -> XpStatus {
        XpStatus::from_total_xp(self.total_xp)
    }

    fn summary(&self) -> AchievementSummary {
        let mut summary = AchievementSummary {
            total_earned: self.earned.len() as u32,
            total_xp: self.total_xp,
            ..Default::default()
        };

        // Count by category and tier
        for earned in self.earned.values() {
            if let Some(achievement) = self.achievements.get(&earned.achievement_id) {
                *summary.by_category.entry(achievement.category).or_insert(0) += 1;
                *summary.by_tier.entry(achievement.tier).or_insert(0) += 1;
            }
        }

        // Find most recent
        summary.most_recent = self.earned
            .values()
            .max_by_key(|e| e.earned_at)
            .map(|e| e.achievement_id);

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{AchievementCategory, AchievementTier};

    fn test_achievement() -> Achievement {
        Achievement::new(
            "test",
            "Test Achievement",
            "Test Description",
            AchievementCategory::Training,
            AchievementTier::Bronze,
        )
    }

    #[test]
    fn test_tracker_creation() {
        let user_id = Uuid::new_v4();
        let tracker = DefaultAchievementTracker::new(user_id);

        assert_eq!(tracker.total_xp, 0);
        assert!(tracker.earned.is_empty());
    }

    #[test]
    fn test_award_achievement() {
        let user_id = Uuid::new_v4();
        let mut tracker = DefaultAchievementTracker::new(user_id);

        let achievement = test_achievement();
        tracker.register_achievement(achievement.clone());

        let earned = tracker.award(&achievement, None);

        assert!(earned.is_some());
        assert!(tracker.is_earned(achievement.id));
        assert_eq!(tracker.total_xp, 100); // Bronze = 100 XP
    }

    #[test]
    fn test_no_duplicate_award() {
        let user_id = Uuid::new_v4();
        let mut tracker = DefaultAchievementTracker::new(user_id);

        let achievement = test_achievement();
        tracker.register_achievement(achievement.clone());

        tracker.award(&achievement, None);
        let second = tracker.award(&achievement, None);

        assert!(second.is_none());
        assert_eq!(tracker.total_xp, 100); // Only awarded once
    }

    #[test]
    fn test_xp_status() {
        let user_id = Uuid::new_v4();
        let mut tracker = DefaultAchievementTracker::new(user_id);

        let achievement = test_achievement();
        tracker.register_achievement(achievement.clone());
        tracker.award(&achievement, None);

        let status = tracker.xp_status();
        assert_eq!(status.total_xp, 100);
        assert_eq!(status.level, 1);
    }

    #[test]
    fn test_cumulative_stats() {
        let mut stats = CumulativeStats::default();

        let metrics = RideMetrics::new(Uuid::new_v4(), 50.0, 7200)
            .with_elevation(500.0);

        stats.update_from_ride(&metrics);

        assert_eq!(stats.total_distance_km, 50.0);
        assert_eq!(stats.total_elevation_m, 500.0);
        assert_eq!(stats.total_rides, 1);
        assert_eq!(stats.current_streak, 1);
    }
}
