//! Lifetime best power tracking.
//!
//! T050: Implement lifetime best tracking.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::profile::{PowerProfile, PowerProfilePoint};
use super::types::{ProfileType, PROFILE_DURATIONS};

/// Type alias for ride history data: (ride_id, ride_date, mmp_values).
/// MMP values are pairs of (duration_secs, power_watts).
pub type RideHistoryData = (Uuid, DateTime<Utc>, Vec<(u32, u16)>);

/// A lifetime best record at a specific duration.
#[derive(Debug, Clone)]
pub struct LifetimeBest {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Best power achieved (watts).
    pub power_watts: u16,
    /// When this best was achieved.
    pub achieved_at: DateTime<Utc>,
    /// Which ride this came from.
    pub ride_id: Option<Uuid>,
    /// Previous best power (if any).
    pub previous_best: Option<u16>,
}

impl LifetimeBest {
    /// Create a new lifetime best.
    pub fn new(
        duration_secs: u32,
        power_watts: u16,
        achieved_at: DateTime<Utc>,
        ride_id: Option<Uuid>,
    ) -> Self {
        Self {
            duration_secs,
            power_watts,
            achieved_at,
            ride_id,
            previous_best: None,
        }
    }

    /// Create with previous best for comparison.
    pub fn with_previous(mut self, previous: u16) -> Self {
        self.previous_best = Some(previous);
        self
    }

    /// Calculate improvement over previous best.
    pub fn improvement_watts(&self) -> Option<u16> {
        self.previous_best
            .map(|prev| self.power_watts.saturating_sub(prev))
    }

    /// Calculate improvement percentage.
    pub fn improvement_percent(&self) -> Option<f64> {
        self.previous_best.map(|prev| {
            if prev > 0 {
                ((self.power_watts as f64 - prev as f64) / prev as f64) * 100.0
            } else {
                0.0
            }
        })
    }
}

/// Result of checking for new lifetime bests.
#[derive(Debug, Clone)]
pub struct LifetimeCheckResult {
    /// New lifetime bests achieved.
    pub new_bests: Vec<LifetimeBest>,
    /// Total number of durations checked.
    pub durations_checked: usize,
}

impl LifetimeCheckResult {
    /// Check if any new bests were achieved.
    pub fn has_new_bests(&self) -> bool {
        !self.new_bests.is_empty()
    }

    /// Get count of new PRs.
    pub fn new_pr_count(&self) -> usize {
        self.new_bests.len()
    }

    /// Get the most significant new best (largest improvement percentage).
    pub fn best_improvement(&self) -> Option<&LifetimeBest> {
        self.new_bests.iter().max_by(|a, b| {
            let a_pct = a.improvement_percent().unwrap_or(0.0);
            let b_pct = b.improvement_percent().unwrap_or(0.0);
            a_pct
                .partial_cmp(&b_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Tracker for lifetime best power values.
pub struct LifetimeBestTracker {
    /// The lifetime best profile.
    profile: PowerProfile,
    /// Standard durations to track.
    durations: Vec<u32>,
}

impl LifetimeBestTracker {
    /// Create a new tracker for a user.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            profile: PowerProfile::new(user_id, ProfileType::Lifetime),
            durations: PROFILE_DURATIONS.to_vec(),
        }
    }

    /// Create with existing profile data.
    pub fn with_profile(profile: PowerProfile) -> Self {
        Self {
            profile,
            durations: PROFILE_DURATIONS.to_vec(),
        }
    }

    /// Check ride MMP values against lifetime bests.
    ///
    /// Returns new lifetime bests achieved in this ride.
    pub fn check_ride(
        &mut self,
        ride_id: Uuid,
        ride_date: DateTime<Utc>,
        mmp_values: &[(u32, u16)],
    ) -> LifetimeCheckResult {
        let mut new_bests = Vec::new();

        for &(duration, power) in mmp_values {
            // Only track standard durations
            if !self.durations.contains(&duration) {
                continue;
            }

            let current_best = self.profile.power_at_duration(duration);
            let is_new_best = current_best.map_or(true, |best| power > best);

            if is_new_best {
                let mut lifetime_best =
                    LifetimeBest::new(duration, power, ride_date, Some(ride_id));

                if let Some(prev) = current_best {
                    lifetime_best = lifetime_best.with_previous(prev);
                }

                // Update the profile
                let point = PowerProfilePoint::with_timestamp(duration, power, ride_date)
                    .with_ride(ride_id);
                self.profile.update_point(point);

                new_bests.push(lifetime_best);
            }
        }

        LifetimeCheckResult {
            new_bests,
            durations_checked: mmp_values.len(),
        }
    }

    /// Get the lifetime best profile.
    pub fn profile(&self) -> &PowerProfile {
        &self.profile
    }

    /// Get mutable reference to profile.
    pub fn profile_mut(&mut self) -> &mut PowerProfile {
        &mut self.profile
    }

    /// Get best power at a duration.
    pub fn best_at(&self, duration_secs: u32) -> Option<u16> {
        self.profile.power_at_duration(duration_secs)
    }

    /// Get the full point (with timestamp and ride) at a duration.
    pub fn best_point_at(&self, duration_secs: u32) -> Option<&PowerProfilePoint> {
        self.profile.point_at_duration(duration_secs)
    }

    /// Get estimated lifetime best FTP.
    pub fn estimated_ftp(&self) -> Option<u16> {
        self.profile.estimated_ftp()
    }

    /// Check if profile has data at all standard durations.
    pub fn is_complete(&self) -> bool {
        self.profile.is_complete()
    }

    /// Get count of durations with data.
    pub fn duration_count(&self) -> usize {
        self.profile.point_count()
    }

    /// Compare current power values to lifetime bests.
    ///
    /// Returns percentage of lifetime best at each duration.
    pub fn compare_to_lifetime(&self, mmp_values: &[(u32, u16)]) -> Vec<(u32, f64)> {
        mmp_values
            .iter()
            .filter_map(|&(duration, power)| {
                self.best_at(duration).map(|best| {
                    let percentage = if best > 0 {
                        (power as f64 / best as f64) * 100.0
                    } else {
                        100.0
                    };
                    (duration, percentage)
                })
            })
            .collect()
    }
}

/// Load lifetime bests from ride history.
///
/// Processes all rides to build the lifetime best profile.
pub fn build_lifetime_from_history(user_id: Uuid, rides: &[RideHistoryData]) -> PowerProfile {
    let mut tracker = LifetimeBestTracker::new(user_id);

    for (ride_id, ride_date, mmp_values) in rides {
        tracker.check_ride(*ride_id, *ride_date, mmp_values);
    }

    // Take ownership of the profile
    tracker.profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_lifetime_best_creation() {
        let now = Utc::now();
        let best = LifetimeBest::new(300, 350, now, Some(Uuid::new_v4())).with_previous(320);

        assert_eq!(best.improvement_watts(), Some(30));
        assert!((best.improvement_percent().unwrap() - 9.375).abs() < 0.01);
    }

    #[test]
    fn test_lifetime_tracker_new_bests() {
        let user_id = Uuid::new_v4();
        let mut tracker = LifetimeBestTracker::new(user_id);
        let ride_id = Uuid::new_v4();
        let now = Utc::now();

        // First ride establishes all bests
        let result1 = tracker.check_ride(ride_id, now, &[(5, 800), (60, 400), (1200, 250)]);

        assert!(result1.has_new_bests());
        assert_eq!(result1.new_pr_count(), 3);

        // Second ride with improvements
        let ride_id2 = Uuid::new_v4();
        let result2 = tracker.check_ride(
            ride_id2,
            now + Duration::days(1),
            &[(5, 850), (60, 380), (1200, 260)], // Better 5s and 1200s
        );

        assert!(result2.has_new_bests());
        assert_eq!(result2.new_pr_count(), 2);

        // Check the improvements were recorded
        let best_5s = result2
            .new_bests
            .iter()
            .find(|b| b.duration_secs == 5)
            .unwrap();
        assert_eq!(best_5s.previous_best, Some(800));
        assert_eq!(best_5s.improvement_watts(), Some(50));
    }

    #[test]
    fn test_lifetime_no_improvement() {
        let user_id = Uuid::new_v4();
        let mut tracker = LifetimeBestTracker::new(user_id);
        let now = Utc::now();

        // Establish bests
        tracker.check_ride(Uuid::new_v4(), now, &[(5, 800), (60, 400)]);

        // Ride with no improvements
        let result = tracker.check_ride(
            Uuid::new_v4(),
            now + Duration::days(1),
            &[(5, 750), (60, 380)], // Both worse
        );

        assert!(!result.has_new_bests());
        assert_eq!(result.new_pr_count(), 0);

        // Profile should be unchanged
        assert_eq!(tracker.best_at(5), Some(800));
        assert_eq!(tracker.best_at(60), Some(400));
    }

    #[test]
    fn test_compare_to_lifetime() {
        let user_id = Uuid::new_v4();
        let mut tracker = LifetimeBestTracker::new(user_id);
        let now = Utc::now();

        tracker.check_ride(Uuid::new_v4(), now, &[(5, 1000), (60, 500), (1200, 250)]);

        let comparison = tracker.compare_to_lifetime(&[(5, 900), (60, 500), (1200, 275)]);

        // 5s: 900/1000 = 90%
        let pct_5s = comparison.iter().find(|(d, _)| *d == 5).unwrap().1;
        assert!((pct_5s - 90.0).abs() < 0.01);

        // 60s: 500/500 = 100%
        let pct_60s = comparison.iter().find(|(d, _)| *d == 60).unwrap().1;
        assert!((pct_60s - 100.0).abs() < 0.01);

        // 1200s: 275/250 = 110% (better than lifetime!)
        let pct_1200s = comparison.iter().find(|(d, _)| *d == 1200).unwrap().1;
        assert!((pct_1200s - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_build_from_history() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let rides = vec![
            (
                Uuid::new_v4(),
                now - Duration::days(100),
                vec![(5, 800), (60, 400)],
            ),
            (
                Uuid::new_v4(),
                now - Duration::days(50),
                vec![(5, 900), (60, 380)],
            ),
            (
                Uuid::new_v4(),
                now - Duration::days(10),
                vec![(5, 850), (60, 420)],
            ),
        ];

        let profile = build_lifetime_from_history(user_id, &rides);

        // Should have best from across all rides
        assert_eq!(profile.power_at_duration(5), Some(900)); // From 50-day ride
        assert_eq!(profile.power_at_duration(60), Some(420)); // From 10-day ride
    }

    #[test]
    fn test_best_improvement() {
        let user_id = Uuid::new_v4();
        let mut tracker = LifetimeBestTracker::new(user_id);
        let now = Utc::now();

        // Establish bests
        tracker.check_ride(Uuid::new_v4(), now, &[(5, 800), (60, 400), (1200, 250)]);

        // Second ride with improvements
        let result = tracker.check_ride(
            Uuid::new_v4(),
            now + Duration::days(1),
            &[(5, 880), (60, 420), (1200, 260)], // All improved by different amounts
        );

        // Find the best improvement (should be 5s with 10% improvement)
        let best = result.best_improvement().unwrap();
        assert_eq!(best.duration_secs, 5); // 80/800 = 10% > 20/400 = 5% > 10/250 = 4%
    }
}
