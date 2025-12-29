//! XP calculation and level progression logic.
//!
//! T040: Create XP calculation and level progression logic.

use serde::{Deserialize, Serialize};

// Re-export career-level functions for convenience
pub use crate::career::{cumulative_xp_to_level, level_from_xp, xp_for_level, MAX_LEVEL};

/// XP gain multipliers for different activities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XpMultiplier {
    /// No multiplier (1.0x).
    None,
    /// First ride of the day bonus (1.25x).
    FirstRideOfDay,
    /// Weekly streak bonus (1.5x).
    WeeklyStreak,
    /// Event participation bonus (2.0x).
    EventBonus,
    /// Custom multiplier.
    Custom(f32),
}

impl XpMultiplier {
    /// Get the multiplier value.
    pub fn value(&self) -> f32 {
        match self {
            Self::None => 1.0,
            Self::FirstRideOfDay => 1.25,
            Self::WeeklyStreak => 1.5,
            Self::EventBonus => 2.0,
            Self::Custom(v) => *v,
        }
    }
}

/// XP source tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum XpSource {
    /// XP from completing a ride.
    Ride,
    /// XP from earning an achievement.
    Achievement,
    /// XP from completing a workout.
    Workout,
    /// XP from completing a training plan week.
    TrainingPlan,
    /// XP from participation in events.
    Event,
    /// XP from challenges.
    Challenge,
    /// XP from streak bonuses.
    Streak,
    /// XP adjustment (admin/correction).
    Adjustment,
}

/// Record of XP gained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XpGain {
    /// Base XP amount.
    pub base_xp: u32,
    /// Multiplier applied.
    pub multiplier: f32,
    /// Final XP after multiplier.
    pub final_xp: u32,
    /// Source of the XP.
    pub source: XpSource,
    /// Optional description.
    pub description: Option<String>,
}

impl XpGain {
    /// Create a new XP gain record.
    pub fn new(base_xp: u32, source: XpSource) -> Self {
        Self {
            base_xp,
            multiplier: 1.0,
            final_xp: base_xp,
            source,
            description: None,
        }
    }

    /// Apply a multiplier.
    pub fn with_multiplier(mut self, multiplier: XpMultiplier) -> Self {
        self.multiplier = multiplier.value();
        self.final_xp = (self.base_xp as f32 * self.multiplier) as u32;
        self
    }

    /// Add a description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Calculate XP from ride metrics.
pub fn xp_from_ride(distance_km: f64, duration_mins: u32, elevation_m: f64) -> u32 {
    let base_xp = (distance_km * 10.0) as u32; // 10 XP per km
    let time_bonus = (duration_mins / 10) * 5; // 5 XP per 10 minutes
    let elevation_bonus = (elevation_m / 100.0 * 20.0) as u32; // 20 XP per 100m climbed

    base_xp + time_bonus + elevation_bonus
}

/// Calculate XP from workout completion.
pub fn xp_from_workout(duration_mins: u32, tss: Option<f32>, completed: bool) -> u32 {
    if !completed {
        // Partial credit for incomplete workouts
        return (duration_mins / 10) * 10;
    }

    let base_xp = (duration_mins as f32 * 2.0) as u32; // 2 XP per minute
    let tss_bonus = tss.map(|t| (t * 0.5) as u32).unwrap_or(0); // 0.5 XP per TSS

    base_xp + tss_bonus + 50 // +50 bonus for completion
}

/// User XP status summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XpStatus {
    /// Total lifetime XP.
    pub total_xp: u64,
    /// Current level.
    pub level: u32,
    /// XP into current level.
    pub xp_into_level: u64,
    /// XP needed for next level.
    pub xp_for_next: u64,
    /// Progress to next level (0.0 - 1.0).
    pub level_progress: f32,
}

impl XpStatus {
    /// Calculate status from total XP.
    pub fn from_total_xp(total_xp: u64) -> Self {
        let level = level_from_xp(total_xp);
        let xp_at_level = cumulative_xp_to_level(level);
        let xp_into_level = total_xp.saturating_sub(xp_at_level);

        let xp_for_next = if level >= MAX_LEVEL {
            0
        } else {
            xp_for_level(level + 1)
        };

        let level_progress = if xp_for_next > 0 {
            (xp_into_level as f32 / xp_for_next as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };

        Self {
            total_xp,
            level,
            xp_into_level,
            xp_for_next,
            level_progress,
        }
    }

    /// Check if at max level.
    pub fn is_max_level(&self) -> bool {
        self.level >= MAX_LEVEL
    }

    /// Calculate XP remaining to reach a target level.
    pub fn xp_to_level(&self, target_level: u32) -> u64 {
        if target_level <= self.level {
            return 0;
        }
        let target_xp = cumulative_xp_to_level(target_level);
        target_xp.saturating_sub(self.total_xp)
    }
}

/// Result of adding XP.
#[derive(Debug, Clone)]
pub struct XpAddResult {
    /// New total XP.
    pub new_total: u64,
    /// XP that was added.
    pub xp_added: u32,
    /// Previous level.
    pub previous_level: u32,
    /// New level.
    pub new_level: u32,
    /// Whether a level up occurred.
    pub leveled_up: bool,
    /// Number of levels gained.
    pub levels_gained: u32,
}

impl XpAddResult {
    /// Create from before/after XP values.
    pub fn from_xp_change(previous_xp: u64, new_xp: u64, xp_added: u32) -> Self {
        let previous_level = level_from_xp(previous_xp);
        let new_level = level_from_xp(new_xp);

        Self {
            new_total: new_xp,
            xp_added,
            previous_level,
            new_level,
            leveled_up: new_level > previous_level,
            levels_gained: new_level.saturating_sub(previous_level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_multiplier() {
        assert_eq!(XpMultiplier::None.value(), 1.0);
        assert_eq!(XpMultiplier::FirstRideOfDay.value(), 1.25);
        assert_eq!(XpMultiplier::WeeklyStreak.value(), 1.5);
        assert_eq!(XpMultiplier::Custom(2.5).value(), 2.5);
    }

    #[test]
    fn test_xp_gain() {
        let gain =
            XpGain::new(100, XpSource::Achievement).with_multiplier(XpMultiplier::FirstRideOfDay);

        assert_eq!(gain.base_xp, 100);
        assert_eq!(gain.final_xp, 125);
    }

    #[test]
    fn test_xp_from_ride() {
        // 50km, 2 hours, 500m climbing
        let xp = xp_from_ride(50.0, 120, 500.0);

        // 50*10 + 12*5 + 100 = 500 + 60 + 100 = 660
        assert_eq!(xp, 660);
    }

    #[test]
    fn test_xp_from_workout() {
        // 60 min workout completed with TSS 80
        let xp = xp_from_workout(60, Some(80.0), true);

        // 60*2 + 80*0.5 + 50 = 120 + 40 + 50 = 210
        assert_eq!(xp, 210);
    }

    #[test]
    fn test_xp_status() {
        let status = XpStatus::from_total_xp(1500);

        // 1500 XP should be level 2+ (level 2 starts at 1000)
        assert!(status.level >= 2);
        assert!(status.level_progress >= 0.0);
        assert!(status.level_progress <= 1.0);
    }

    #[test]
    fn test_xp_add_result() {
        // xp_for_level(2) = 1150, so need >= 1150 for level 2
        let result = XpAddResult::from_xp_change(1000, 1200, 200);

        // Should have leveled up (1150 XP = level 2)
        assert!(result.leveled_up);
        assert_eq!(result.levels_gained, 1);
    }

    #[test]
    fn test_xp_status_max_level() {
        // Very high XP
        let status = XpStatus::from_total_xp(1_000_000_000);

        assert!(status.is_max_level());
        assert_eq!(status.level, MAX_LEVEL);
    }
}
