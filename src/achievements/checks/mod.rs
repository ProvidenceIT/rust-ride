//! Achievement check modules.
//!
//! Each module provides functions to check if achievements should be awarded
//! based on ride data and cumulative statistics.

pub mod career;
pub mod consistency;
pub mod cumulative;
pub mod power;
pub mod ride;
pub mod training;

pub use career::{all_career_achievements, CareerChecker, CareerSummary};
pub use consistency::ConsistencyChecker;
pub use cumulative::CumulativeChecker;
pub use power::{all_power_achievements, PowerProfileChecker};
pub use ride::RideChecker;
pub use training::{all_training_achievements, TrainingChecker, TrainingPlanSummary};

use super::achievement::Achievement;
use super::earned::RideMetrics;
use super::tracker::CumulativeStats;

/// Trait for achievement checkers.
pub trait AchievementChecker {
    /// Check achievements and return those that should be awarded.
    fn check(&self, metrics: &RideMetrics, stats: &CumulativeStats) -> Vec<Achievement>;
}

/// Combined checker that runs all achievement checks.
#[derive(Default)]
pub struct AllCheckers {
    ride: RideChecker,
    cumulative: CumulativeChecker,
    consistency: ConsistencyChecker,
}

impl AllCheckers {
    /// Create a new combined checker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check all achievement types.
    pub fn check_all(&self, metrics: &RideMetrics, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        achievements.extend(self.ride.check(metrics, stats));
        achievements.extend(self.cumulative.check(metrics, stats));
        achievements.extend(self.consistency.check(metrics, stats));

        achievements
    }
}
