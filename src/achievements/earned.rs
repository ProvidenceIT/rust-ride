//! Earned achievement and ride metrics structures.
//!
//! T033: Create EarnedAchievement and RideMetrics structs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{AchievementCategory, AchievementTier};

/// An achievement that has been earned by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarnedAchievement {
    /// Database row ID.
    pub id: Option<i64>,
    /// User who earned it.
    pub user_id: Uuid,
    /// Achievement that was earned.
    pub achievement_id: Uuid,
    /// When it was earned.
    pub earned_at: DateTime<Utc>,
    /// Ride during which it was earned (if applicable).
    pub ride_id: Option<Uuid>,
    /// XP awarded (may differ from base if bonuses applied).
    pub xp_awarded: u32,
    /// Progress value at time of earning (for threshold achievements).
    pub progress_value: Option<f64>,
}

impl EarnedAchievement {
    /// Create a new earned achievement record.
    pub fn new(user_id: Uuid, achievement_id: Uuid, xp_awarded: u32) -> Self {
        Self {
            id: None,
            user_id,
            achievement_id,
            earned_at: Utc::now(),
            ride_id: None,
            xp_awarded,
            progress_value: None,
        }
    }

    /// Associate with a ride.
    pub fn with_ride(mut self, ride_id: Uuid) -> Self {
        self.ride_id = Some(ride_id);
        self
    }

    /// Set progress value.
    pub fn with_progress(mut self, value: f64) -> Self {
        self.progress_value = Some(value);
        self
    }

    /// Set database ID (after insertion).
    pub fn with_db_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }
}

/// Metrics from a single ride used for achievement evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RideMetrics {
    /// Ride UUID.
    pub ride_id: Uuid,
    /// Total distance in km.
    pub distance_km: f64,
    /// Total duration in seconds.
    pub duration_secs: u32,
    /// Elevation gain in meters.
    pub elevation_gain_m: f64,
    /// Average power in watts.
    pub avg_power: Option<u16>,
    /// Normalized power in watts.
    pub normalized_power: Option<u16>,
    /// Max power in watts.
    pub max_power: Option<u16>,
    /// 20-minute power in watts (for FTP achievements).
    pub twenty_min_power: Option<u16>,
    /// Number of power PRs set in this ride.
    pub power_prs: Option<u32>,
    /// Average heart rate.
    pub avg_hr: Option<u8>,
    /// Max heart rate.
    pub max_hr: Option<u8>,
    /// Average cadence.
    pub avg_cadence: Option<u8>,
    /// Calories burned.
    pub calories: Option<u32>,
    /// Whether a workout was completed.
    pub workout_completed: bool,
    /// Workout ID if applicable.
    pub workout_id: Option<Uuid>,
    /// Workout compliance percentage (0-100).
    pub workout_compliance: Option<f32>,
    /// Training Stress Score.
    pub tss: Option<f32>,
    /// Intensity Factor.
    pub intensity_factor: Option<f32>,
    /// Route ID if applicable.
    pub route_id: Option<Uuid>,
    /// Whether a GPX route was used.
    pub has_route: bool,
    /// Maximum gradient encountered on route.
    pub max_gradient: Option<f32>,
    /// Whether this was a race.
    pub is_race: bool,
    /// Race position (if applicable).
    pub race_position: Option<u32>,
    /// Number of race participants (if applicable).
    pub race_participants: Option<u32>,
    /// Hour when ride started (0-23).
    pub start_hour: Option<u8>,
    /// Date when ride started (month, day).
    pub start_date: Option<(u8, u8)>,
}

impl RideMetrics {
    /// Create metrics from basic ride data.
    pub fn new(ride_id: Uuid, distance_km: f64, duration_secs: u32) -> Self {
        Self {
            ride_id,
            distance_km,
            duration_secs,
            ..Default::default()
        }
    }

    /// Set elevation gain.
    pub fn with_elevation(mut self, gain_m: f64) -> Self {
        self.elevation_gain_m = gain_m;
        self
    }

    /// Set power metrics.
    pub fn with_power(mut self, avg: u16, np: Option<u16>, max: u16) -> Self {
        self.avg_power = Some(avg);
        self.normalized_power = np;
        self.max_power = Some(max);
        self
    }

    /// Set heart rate metrics.
    pub fn with_hr(mut self, avg: u8, max: u8) -> Self {
        self.avg_hr = Some(avg);
        self.max_hr = Some(max);
        self
    }

    /// Set workout completion.
    pub fn with_workout(mut self, workout_id: Uuid, completed: bool) -> Self {
        self.workout_id = Some(workout_id);
        self.workout_completed = completed;
        self
    }

    /// Set training metrics.
    pub fn with_training_metrics(mut self, tss: f32, if_: f32) -> Self {
        self.tss = Some(tss);
        self.intensity_factor = Some(if_);
        self
    }

    /// Set race results.
    pub fn with_race(mut self, position: u32, participants: u32) -> Self {
        self.is_race = true;
        self.race_position = Some(position);
        self.race_participants = Some(participants);
        self
    }

    /// Calculate speed in km/h.
    pub fn avg_speed_kmh(&self) -> f64 {
        if self.duration_secs == 0 {
            return 0.0;
        }
        self.distance_km / (self.duration_secs as f64 / 3600.0)
    }

    /// Check if power data is available.
    pub fn has_power(&self) -> bool {
        self.avg_power.is_some()
    }

    /// Check if HR data is available.
    pub fn has_hr(&self) -> bool {
        self.avg_hr.is_some()
    }
}

/// Summary of a user's achievement progress.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementProgress {
    /// Achievement ID.
    pub achievement_id: Uuid,
    /// Current progress value.
    pub current_value: f64,
    /// Target threshold.
    pub target_value: f64,
    /// Progress as percentage (0.0 - 1.0).
    pub progress_percent: f32,
    /// Whether already earned.
    pub is_earned: bool,
}

impl AchievementProgress {
    /// Create new progress tracker.
    pub fn new(achievement_id: Uuid, target: f64) -> Self {
        Self {
            achievement_id,
            current_value: 0.0,
            target_value: target,
            progress_percent: 0.0,
            is_earned: false,
        }
    }

    /// Update progress value.
    pub fn update(&mut self, value: f64) {
        self.current_value = value;
        self.progress_percent = if self.target_value > 0.0 {
            (value / self.target_value).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        if self.current_value >= self.target_value {
            self.is_earned = true;
        }
    }
}

/// Summary statistics for achievements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementSummary {
    /// Total achievements earned.
    pub total_earned: u32,
    /// Achievements per category.
    pub by_category: std::collections::HashMap<AchievementCategory, u32>,
    /// Achievements per tier.
    pub by_tier: std::collections::HashMap<AchievementTier, u32>,
    /// Total XP from achievements.
    pub total_xp: u64,
    /// Most recent achievement earned.
    pub most_recent: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_earned_achievement() {
        let user_id = Uuid::new_v4();
        let achievement_id = Uuid::new_v4();

        let earned = EarnedAchievement::new(user_id, achievement_id, 500);

        assert_eq!(earned.user_id, user_id);
        assert_eq!(earned.achievement_id, achievement_id);
        assert_eq!(earned.xp_awarded, 500);
        assert!(earned.ride_id.is_none());
    }

    #[test]
    fn test_ride_metrics() {
        let ride_id = Uuid::new_v4();
        let metrics = RideMetrics::new(ride_id, 50.0, 7200) // 50km in 2 hours
            .with_elevation(500.0)
            .with_power(200, Some(210), 450);

        assert_eq!(metrics.distance_km, 50.0);
        assert_eq!(metrics.elevation_gain_m, 500.0);
        assert_eq!(metrics.avg_power, Some(200));
        assert!((metrics.avg_speed_kmh() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_achievement_progress() {
        let achievement_id = Uuid::new_v4();
        let mut progress = AchievementProgress::new(achievement_id, 100.0);

        progress.update(50.0);
        assert!((progress.progress_percent - 0.5).abs() < 0.01);
        assert!(!progress.is_earned);

        progress.update(100.0);
        assert!(progress.is_earned);
    }
}
