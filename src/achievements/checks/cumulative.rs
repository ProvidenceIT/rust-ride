//! Cumulative achievement checks.
//!
//! T038: Implement cumulative achievement checks (lifetime totals).
//!
//! Checks achievements based on lifetime accumulated statistics.

use super::AchievementChecker;
use crate::achievements::achievement::Achievement;
use crate::achievements::definitions;
use crate::achievements::earned::RideMetrics;
use crate::achievements::tracker::CumulativeStats;

/// Checker for cumulative/lifetime achievements.
#[derive(Debug, Default)]
pub struct CumulativeChecker;

impl CumulativeChecker {
    /// Create a new cumulative checker.
    pub fn new() -> Self {
        Self
    }

    /// Check lifetime distance achievements.
    fn check_lifetime_distance(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let total_km = stats.total_distance_km;

        if total_km >= 1000.0 {
            achievements.push(definitions::lifetime_1000k());
        }
        if total_km >= 5000.0 {
            achievements.push(definitions::lifetime_5000k());
        }
        if total_km >= 10000.0 {
            achievements.push(definitions::lifetime_10000k());
        }

        achievements
    }

    /// Check lifetime climbing achievements.
    fn check_lifetime_climbing(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let total_elevation = stats.total_elevation_m;

        if total_elevation >= 8849.0 {
            achievements.push(definitions::everest_challenge());
        }
        if total_elevation >= 10000.0 {
            achievements.push(definitions::lifetime_climb_10k());
        }
        if total_elevation >= 50000.0 {
            achievements.push(definitions::lifetime_climb_50k());
        }

        achievements
    }

    /// Check total rides achievements.
    fn check_total_rides(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let total_rides = stats.total_rides;

        if total_rides >= 10 {
            achievements.push(definitions::rides_10());
        }
        if total_rides >= 50 {
            achievements.push(definitions::rides_50());
        }
        if total_rides >= 100 {
            achievements.push(definitions::rides_100());
        }
        if total_rides >= 500 {
            achievements.push(definitions::rides_500());
        }
        if total_rides >= 1000 {
            achievements.push(definitions::rides_1000());
        }

        achievements
    }

    /// Check total workouts achievements.
    fn check_total_workouts(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let total_workouts = stats.total_workouts;

        if total_workouts >= 10 {
            achievements.push(definitions::workouts_10());
        }
        if total_workouts >= 50 {
            achievements.push(definitions::workouts_50());
        }
        if total_workouts >= 100 {
            achievements.push(definitions::workouts_100());
        }

        achievements
    }

    /// Check total time achievements.
    fn check_total_time(&self, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let total_hours = stats.total_time_secs as f64 / 3600.0;

        if total_hours >= 100.0 {
            achievements.push(definitions::time_100h());
        }

        achievements
    }
}

impl AchievementChecker for CumulativeChecker {
    fn check(&self, _metrics: &RideMetrics, stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        achievements.extend(self.check_lifetime_distance(stats));
        achievements.extend(self.check_lifetime_climbing(stats));
        achievements.extend(self.check_total_rides(stats));
        achievements.extend(self.check_total_workouts(stats));
        achievements.extend(self.check_total_time(stats));

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
        RideMetrics::new(Uuid::new_v4(), 0.0, 0)
    }

    #[test]
    fn test_lifetime_distance_achievements() {
        let checker = CumulativeChecker::new();
        let metrics = make_metrics();

        // Under threshold
        let mut stats = make_stats();
        stats.total_distance_km = 500.0;
        let achievements = checker.check(&metrics, &stats);
        assert!(!achievements.iter().any(|a| a.name == "lifetime_1000k"));

        // At threshold
        stats.total_distance_km = 1000.0;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "lifetime_1000k"));
        assert!(!achievements.iter().any(|a| a.name == "lifetime_5000k"));

        // Higher thresholds
        stats.total_distance_km = 5500.0;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "lifetime_1000k"));
        assert!(achievements.iter().any(|a| a.name == "lifetime_5000k"));
        assert!(!achievements.iter().any(|a| a.name == "lifetime_10000k"));
    }

    #[test]
    fn test_lifetime_climbing_achievements() {
        let checker = CumulativeChecker::new();
        let metrics = make_metrics();

        let mut stats = make_stats();
        stats.total_elevation_m = 9000.0;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "everest_challenge"));
    }

    #[test]
    fn test_total_rides_achievements() {
        let checker = CumulativeChecker::new();
        let metrics = make_metrics();

        let mut stats = make_stats();
        stats.total_rides = 55;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "rides_10"));
        assert!(achievements.iter().any(|a| a.name == "rides_50"));
        assert!(!achievements.iter().any(|a| a.name == "rides_100"));
    }

    #[test]
    fn test_total_workouts_achievements() {
        let checker = CumulativeChecker::new();
        let metrics = make_metrics();

        let mut stats = make_stats();
        stats.total_workouts = 75;
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "workouts_10"));
        assert!(achievements.iter().any(|a| a.name == "workouts_50"));
        assert!(!achievements.iter().any(|a| a.name == "workouts_100"));
    }

    #[test]
    fn test_total_time_achievements() {
        let checker = CumulativeChecker::new();
        let metrics = make_metrics();

        let mut stats = make_stats();
        stats.total_time_secs = 100 * 3600; // 100 hours
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "time_100h"));
    }
}
