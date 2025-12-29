//! Consistency achievement checks.
//!
//! T039: Implement consistency achievement checks (streaks, daily).
//!
//! Checks achievements based on consistency patterns like daily streaks.

use super::AchievementChecker;
use crate::achievements::achievement::Achievement;
use crate::achievements::definitions;
use crate::achievements::earned::RideMetrics;
use crate::achievements::tracker::CumulativeStats;

/// Checker for consistency achievements.
#[derive(Debug, Default)]
pub struct ConsistencyChecker;

impl ConsistencyChecker {
    /// Create a new consistency checker.
    pub fn new() -> Self {
        Self
    }

    /// Check streak achievements.
    fn check_streaks(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let streak = stats.current_streak;

        if streak >= 3 {
            achievements.push(definitions::streak_3());
        }
        if streak >= 7 {
            achievements.push(definitions::streak_7());
        }
        if streak >= 14 {
            achievements.push(definitions::streak_14());
        }
        if streak >= 30 {
            achievements.push(definitions::streak_30());
        }
        if streak >= 100 {
            achievements.push(definitions::streak_100());
        }

        achievements
    }

    /// Check for perfect week (ride every day Mon-Sun).
    fn check_perfect_week(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // Check if all 7 days have rides
        if stats.rides_by_weekday.iter().all(|&count| count > 0) {
            achievements.push(definitions::perfect_week());
        }

        achievements
    }

    /// Check first of year achievement.
    fn check_first_of_year(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // First ride of the year check would need calendar tracking
        // For now, we check if it's early January
        if let Some((month, day)) = metrics.start_date {
            if month == 1 && day <= 7 {
                // Could be first of year - tracker handles dedup for repeatable
                achievements.push(definitions::first_of_year());
            }
        }

        achievements
    }
}

impl AchievementChecker for ConsistencyChecker {
    fn check(&self, metrics: &RideMetrics, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        achievements.extend(self.check_streaks(stats));
        achievements.extend(self.check_perfect_week(stats));
        achievements.extend(self.check_first_of_year(metrics));

        achievements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_stats() -> CumulativeStats {
        CumulativeStats::default()
    }

    fn make_metrics() -> RideMetrics {
        RideMetrics::new(Uuid::new_v4(), 20.0, 3600)
    }

    #[test]
    fn test_streak_achievements() {
        let checker = ConsistencyChecker::new();
        let metrics = make_metrics();

        // No streak
        let mut stats = make_stats();
        stats.current_streak = 2;
        let achievements = checker.check(&metrics, &stats);
        assert!(!achievements.iter().any(|a| a.name == "streak_3"));

        // 3-day streak
        stats.current_streak = 3;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "streak_3"));
        assert!(!achievements.iter().any(|a| a.name == "streak_7"));

        // 7-day streak
        stats.current_streak = 7;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "streak_3"));
        assert!(achievements.iter().any(|a| a.name == "streak_7"));
        assert!(!achievements.iter().any(|a| a.name == "streak_14"));
    }

    #[test]
    fn test_perfect_week() {
        let checker = ConsistencyChecker::new();
        let metrics = make_metrics();

        // Not all days
        let mut stats = make_stats();
        stats.rides_by_weekday = [1, 1, 0, 1, 1, 1, 1]; // Missing Wednesday
        let achievements = checker.check(&metrics, &stats);
        assert!(!achievements.iter().any(|a| a.name == "perfect_week"));

        // All days covered
        stats.rides_by_weekday = [1, 2, 1, 1, 1, 3, 1]; // All days have at least one ride
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "perfect_week"));
    }

    #[test]
    fn test_first_of_year() {
        let checker = ConsistencyChecker::new();
        let stats = make_stats();

        // January 1st
        let mut metrics = make_metrics();
        metrics.start_date = Some((1, 1));
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "first_of_year"));

        // Not early January
        metrics.start_date = Some((3, 15));
        let achievements = checker.check(&metrics, &stats);
        assert!(!achievements.iter().any(|a| a.name == "first_of_year"));
    }

    #[test]
    fn test_long_streak() {
        let checker = ConsistencyChecker::new();
        let metrics = make_metrics();

        let mut stats = make_stats();
        stats.current_streak = 100;
        let achievements = checker.check(&metrics, &stats);

        // Should have all streak achievements
        assert!(achievements.iter().any(|a| a.name == "streak_3"));
        assert!(achievements.iter().any(|a| a.name == "streak_7"));
        assert!(achievements.iter().any(|a| a.name == "streak_14"));
        assert!(achievements.iter().any(|a| a.name == "streak_30"));
        assert!(achievements.iter().any(|a| a.name == "streak_100"));
    }
}
