//! Ride-based achievement checks.
//!
//! T037: Implement ride achievement checks (distance, power, duration).
//!
//! Checks achievements that can be earned from a single ride's metrics.

use super::AchievementChecker;
use crate::achievements::achievement::Achievement;
use crate::achievements::definitions;
use crate::achievements::earned::RideMetrics;
use crate::achievements::tracker::CumulativeStats;

/// Checker for ride-based achievements.
#[derive(Debug, Default)]
pub struct RideChecker;

impl RideChecker {
    /// Create a new ride checker.
    pub fn new() -> Self {
        Self
    }

    /// Check distance achievements for a single ride.
    fn check_distance(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let distance = metrics.distance_km;

        // First ride (any distance > 0)
        if distance > 0.0 {
            // Note: first_ride should only be awarded once, tracker handles dedup
            achievements.push(definitions::first_ride());
        }

        // Distance milestones
        if distance >= 10.0 {
            achievements.push(definitions::distance_10k());
        }
        if distance >= 25.0 {
            achievements.push(definitions::distance_25k());
        }
        if distance >= 50.0 {
            achievements.push(definitions::distance_50k());
        }
        if distance >= 100.0 {
            achievements.push(definitions::metric_century());
        }
        if distance >= 160.0 {
            achievements.push(definitions::imperial_century());
        }
        if distance >= 200.0 {
            achievements.push(definitions::double_metric_century());
        }

        // Marathon distance (within 1km tolerance)
        if (distance - 42.195).abs() < 1.0 {
            achievements.push(definitions::marathon_distance());
        }

        achievements
    }

    /// Check climbing achievements for a single ride.
    fn check_climbing(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();
        let elevation = metrics.elevation_gain_m;

        if elevation >= 100.0 {
            achievements.push(definitions::climb_100m());
        }
        if elevation >= 500.0 {
            achievements.push(definitions::climb_500m());
        }
        if elevation >= 1000.0 {
            achievements.push(definitions::climb_1000m());
        }
        if elevation >= 2000.0 {
            achievements.push(definitions::climb_2000m());
        }
        if elevation >= 3000.0 {
            achievements.push(definitions::climb_3000m());
        }

        achievements
    }

    /// Check power achievements for a single ride.
    fn check_power(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // Check peak power achievements
        if let Some(max_power) = metrics.max_power {
            if max_power >= 1000 {
                achievements.push(definitions::peak_1000w());
            }
            if max_power >= 1500 {
                achievements.push(definitions::peak_1500w());
            }
        }

        // Check 20-minute power achievements
        if let Some(twenty_min_power) = metrics.twenty_min_power {
            if twenty_min_power >= 200 {
                achievements.push(definitions::ftp_200());
            }
            if twenty_min_power >= 250 {
                achievements.push(definitions::ftp_250());
            }
            if twenty_min_power >= 300 {
                achievements.push(definitions::ftp_300());
            }
            if twenty_min_power >= 400 {
                achievements.push(definitions::ftp_400());
            }
        }

        // Multiple power PRs
        if let Some(pr_count) = metrics.power_prs {
            if pr_count >= 1 {
                achievements.push(definitions::first_power_pr());
            }
            if pr_count >= 5 {
                achievements.push(definitions::multi_pr());
            }
        }

        achievements
    }

    /// Check workout achievements for a single ride.
    fn check_workout(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        if metrics.workout_completed {
            achievements.push(definitions::first_workout());

            // Perfect workout (100% compliance)
            if let Some(compliance) = metrics.workout_compliance {
                if (compliance - 100.0).abs() < 0.1 {
                    achievements.push(definitions::perfect_workout());
                }
            }
        }

        // Endurance workout (2+ hours)
        let duration_mins = metrics.duration_secs / 60;
        if metrics.workout_completed && duration_mins >= 120 {
            achievements.push(definitions::endurance_workout());
        }

        achievements
    }

    /// Check race achievements for a single ride.
    fn check_race(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        if metrics.is_race {
            achievements.push(definitions::first_race());

            if let Some(position) = metrics.race_position {
                if position == 1 {
                    achievements.push(definitions::race_winner());
                }
                if position <= 3 {
                    achievements.push(definitions::podium_finish());
                }
            }
        }

        achievements
    }

    /// Check route/exploration achievements for a single ride.
    fn check_route(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        if metrics.has_route {
            achievements.push(definitions::first_route());

            // Check for steep routes
            if let Some(max_gradient) = metrics.max_gradient {
                if max_gradient >= 15.0 {
                    achievements.push(definitions::steep_route());
                }
            }

            // Epic route (100km+ with gradient simulation)
            if metrics.distance_km >= 100.0 {
                achievements.push(definitions::epic_route());
            }
        }

        achievements
    }

    /// Check time-based special achievements.
    fn check_special_time(&self, metrics: &RideMetrics) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // Night owl (midnight to 4am)
        if let Some(hour) = metrics.start_hour {
            if hour < 4 {
                achievements.push(definitions::night_owl());
            }
        }

        // New Year's Day
        if let Some((month, day)) = metrics.start_date {
            if month == 1 && day == 1 {
                achievements.push(definitions::new_year_rider());
            }
        }

        // Precision rider (exactly 1 hour within 30 seconds)
        let duration_secs = metrics.duration_secs;
        if (3570..=3630).contains(&duration_secs) {
            achievements.push(definitions::precision_rider());
        }

        achievements
    }
}

impl AchievementChecker for RideChecker {
    fn check(&self, metrics: &RideMetrics, _stats: &CumulativeStats) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        achievements.extend(self.check_distance(metrics));
        achievements.extend(self.check_climbing(metrics));
        achievements.extend(self.check_power(metrics));
        achievements.extend(self.check_workout(metrics));
        achievements.extend(self.check_race(metrics));
        achievements.extend(self.check_route(metrics));
        achievements.extend(self.check_special_time(metrics));

        achievements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_metrics(distance_km: f64, duration_secs: u32) -> RideMetrics {
        RideMetrics::new(Uuid::new_v4(), distance_km, duration_secs)
    }

    #[test]
    fn test_distance_achievements() {
        let checker = RideChecker::new();

        // Short ride
        let metrics = make_metrics(5.0, 1200);
        let stats = CumulativeStats::default();
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "first_ride"));
        assert!(!achievements.iter().any(|a| a.name == "distance_10k"));

        // 10km ride
        let metrics = make_metrics(10.0, 1800);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "distance_10k"));

        // Century ride
        let metrics = make_metrics(105.0, 14400);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "metric_century"));
    }

    #[test]
    fn test_climbing_achievements() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let metrics = make_metrics(30.0, 5400).with_elevation(550.0);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "climb_100m"));
        assert!(achievements.iter().any(|a| a.name == "climb_500m"));
        assert!(!achievements.iter().any(|a| a.name == "climb_1000m"));
    }

    #[test]
    fn test_power_achievements() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let mut metrics = make_metrics(50.0, 7200);
        metrics.max_power = Some(1200);
        metrics.twenty_min_power = Some(260);

        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "peak_1000w"));
        assert!(!achievements.iter().any(|a| a.name == "peak_1500w"));
        assert!(achievements.iter().any(|a| a.name == "ftp_200"));
        assert!(achievements.iter().any(|a| a.name == "ftp_250"));
        assert!(!achievements.iter().any(|a| a.name == "ftp_300"));
    }

    #[test]
    fn test_workout_achievements() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let mut metrics = make_metrics(25.0, 3600);
        metrics.workout_completed = true;
        metrics.workout_compliance = Some(100.0);

        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "first_workout"));
        assert!(achievements.iter().any(|a| a.name == "perfect_workout"));
    }

    #[test]
    fn test_night_owl_achievement() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let mut metrics = make_metrics(20.0, 3600);
        metrics.start_hour = Some(2); // 2am

        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "night_owl"));
    }

    #[test]
    fn test_precision_rider_achievement() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        // Exactly 1 hour
        let metrics = make_metrics(30.0, 3600);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "precision_rider"));

        // 1 hour + 20 seconds (still qualifies)
        let metrics = make_metrics(30.0, 3620);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "precision_rider"));

        // Too long
        let metrics = make_metrics(30.0, 3700);
        let achievements = checker.check(&metrics, &stats);
        assert!(!achievements.iter().any(|a| a.name == "precision_rider"));
    }

    #[test]
    fn test_marathon_distance() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let metrics = make_metrics(42.2, 5400);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "marathon_distance"));
    }

    #[test]
    fn test_race_achievements() {
        let checker = RideChecker::new();
        let stats = CumulativeStats::default();

        let mut metrics = make_metrics(40.0, 3600);
        metrics.is_race = true;
        metrics.race_position = Some(2);

        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "first_race"));
        assert!(achievements.iter().any(|a| a.name == "podium_finish"));
        assert!(!achievements.iter().any(|a| a.name == "race_winner"));

        // Winner
        metrics.race_position = Some(1);
        let achievements = checker.check(&metrics, &stats);
        assert!(achievements.iter().any(|a| a.name == "race_winner"));
    }
}
