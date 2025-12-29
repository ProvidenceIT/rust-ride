//! 90-day rolling window power profile calculation.
//!
//! T049: Implement 90-day rolling window profile calculation.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::profile::{PowerProfile, PowerProfilePoint};
use super::types::{ProfileType, PROFILE_DURATIONS};

/// Configuration for rolling window profile calculation.
#[derive(Debug, Clone)]
pub struct RollingWindowConfig {
    /// Number of days to include in the rolling window.
    pub window_days: i64,
    /// Minimum number of rides required for valid profile.
    pub min_rides: usize,
    /// Standard durations to track.
    pub durations: Vec<u32>,
}

impl Default for RollingWindowConfig {
    fn default() -> Self {
        Self {
            window_days: 90,
            min_rides: 3,
            durations: PROFILE_DURATIONS.to_vec(),
        }
    }
}

impl RollingWindowConfig {
    /// Create a 90-day rolling window config.
    pub fn ninety_days() -> Self {
        Self::default()
    }

    /// Create a custom window config.
    pub fn with_days(days: i64) -> Self {
        Self {
            window_days: days,
            ..Default::default()
        }
    }

    /// Get the cutoff date for the rolling window.
    pub fn cutoff_date(&self) -> DateTime<Utc> {
        Utc::now() - Duration::days(self.window_days)
    }
}

/// A ride's power data for rolling window calculation.
#[derive(Debug, Clone)]
pub struct RidePowerData {
    /// Unique ride identifier.
    pub ride_id: Uuid,
    /// When the ride occurred.
    pub ride_date: DateTime<Utc>,
    /// MMP values at standard durations (duration_secs, power_watts).
    pub mmp_values: Vec<(u32, u16)>,
}

impl RidePowerData {
    /// Create new ride power data.
    pub fn new(ride_id: Uuid, ride_date: DateTime<Utc>, mmp_values: Vec<(u32, u16)>) -> Self {
        Self {
            ride_id,
            ride_date,
            mmp_values,
        }
    }

    /// Get power at a specific duration.
    pub fn power_at(&self, duration_secs: u32) -> Option<u16> {
        self.mmp_values
            .iter()
            .find(|(d, _)| *d == duration_secs)
            .map(|(_, p)| *p)
    }
}

/// Calculator for 90-day rolling window power profiles.
pub struct RollingWindowCalculator {
    config: RollingWindowConfig,
}

impl RollingWindowCalculator {
    /// Create a new calculator with default config.
    pub fn new() -> Self {
        Self {
            config: RollingWindowConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: RollingWindowConfig) -> Self {
        Self { config }
    }

    /// Calculate rolling window profile from ride history.
    ///
    /// Takes all rides and filters to those within the rolling window,
    /// then finds the best power at each duration.
    pub fn calculate(&self, user_id: Uuid, rides: &[RidePowerData]) -> PowerProfile {
        let cutoff = self.config.cutoff_date();

        // Filter to rides within the window
        let window_rides: Vec<_> = rides.iter().filter(|r| r.ride_date >= cutoff).collect();

        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        // Find best power at each duration
        for &duration in &self.config.durations {
            let best = self.find_best_at_duration(&window_rides, duration);
            if let Some((power, ride_id, achieved_at)) = best {
                let point = PowerProfilePoint::with_timestamp(duration, power, achieved_at)
                    .with_ride(ride_id);
                profile.update_point(point);
            }
        }

        profile
    }

    /// Find the best power at a specific duration from a set of rides.
    fn find_best_at_duration(
        &self,
        rides: &[&RidePowerData],
        duration_secs: u32,
    ) -> Option<(u16, Uuid, DateTime<Utc>)> {
        let mut best: Option<(u16, Uuid, DateTime<Utc>)> = None;

        for ride in rides {
            if let Some(power) = ride.power_at(duration_secs) {
                let is_better = best.map_or(true, |(best_power, _, _)| power > best_power);
                if is_better {
                    best = Some((power, ride.ride_id, ride.ride_date));
                }
            }
        }

        best
    }

    /// Check if there's sufficient data for a valid profile.
    pub fn has_sufficient_data(&self, rides: &[RidePowerData]) -> bool {
        let cutoff = self.config.cutoff_date();
        let window_rides = rides.iter().filter(|r| r.ride_date >= cutoff).count();
        window_rides >= self.config.min_rides
    }

    /// Get rides that are within the rolling window.
    pub fn filter_to_window<'a>(&self, rides: &'a [RidePowerData]) -> Vec<&'a RidePowerData> {
        let cutoff = self.config.cutoff_date();
        rides.iter().filter(|r| r.ride_date >= cutoff).collect()
    }

    /// Get count of rides in the window.
    pub fn ride_count_in_window(&self, rides: &[RidePowerData]) -> usize {
        self.filter_to_window(rides).len()
    }

    /// Get the window configuration.
    pub fn config(&self) -> &RollingWindowConfig {
        &self.config
    }
}

impl Default for RollingWindowCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Update result from adding a new ride to the rolling window.
#[derive(Debug, Clone)]
pub struct RollingWindowUpdate {
    /// Points that were improved (new PRs within the window).
    pub improved_points: Vec<PowerProfilePoint>,
    /// Points that were removed (best ride fell out of window).
    pub expired_points: Vec<u32>,
    /// Whether the profile changed at all.
    pub changed: bool,
}

impl RollingWindowUpdate {
    /// Create an empty update (no changes).
    pub fn no_change() -> Self {
        Self {
            improved_points: Vec::new(),
            expired_points: Vec::new(),
            changed: false,
        }
    }

    /// Check if any points improved.
    pub fn has_improvements(&self) -> bool {
        !self.improved_points.is_empty()
    }

    /// Check if any points expired.
    pub fn has_expirations(&self) -> bool {
        !self.expired_points.is_empty()
    }
}

/// Incremental updater for rolling window profiles.
///
/// Maintains state to efficiently update the profile when new rides are added
/// or old rides fall out of the window.
pub struct RollingWindowUpdater {
    calculator: RollingWindowCalculator,
    /// Current profile.
    current_profile: PowerProfile,
    /// All rides with their power data.
    rides: Vec<RidePowerData>,
}

impl RollingWindowUpdater {
    /// Create a new updater.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            calculator: RollingWindowCalculator::new(),
            current_profile: PowerProfile::new(user_id, ProfileType::Current),
            rides: Vec::new(),
        }
    }

    /// Create with existing ride data.
    pub fn with_rides(user_id: Uuid, rides: Vec<RidePowerData>) -> Self {
        let calculator = RollingWindowCalculator::new();
        let current_profile = calculator.calculate(user_id, &rides);
        Self {
            calculator,
            current_profile,
            rides,
        }
    }

    /// Add a new ride and update the profile.
    pub fn add_ride(&mut self, ride: RidePowerData) -> RollingWindowUpdate {
        // Check for improvements from the new ride
        let mut improved_points = Vec::new();

        for (duration, power) in &ride.mmp_values {
            let current_power = self.current_profile.power_at_duration(*duration);
            let is_improvement = current_power.map_or(true, |cp| *power > cp);

            if is_improvement {
                let point = PowerProfilePoint::with_timestamp(*duration, *power, ride.ride_date)
                    .with_ride(ride.ride_id);
                self.current_profile.update_point(point.clone());
                improved_points.push(point);
            }
        }

        self.rides.push(ride);

        RollingWindowUpdate {
            changed: !improved_points.is_empty(),
            improved_points,
            expired_points: Vec::new(),
        }
    }

    /// Recalculate the profile, checking for expired rides.
    ///
    /// Call this periodically (e.g., daily) to remove rides that have
    /// fallen out of the rolling window.
    pub fn recalculate(&mut self) -> RollingWindowUpdate {
        let old_profile = self.current_profile.clone();
        let user_id = self.current_profile.user_id;

        // Recalculate from scratch
        self.current_profile = self.calculator.calculate(user_id, &self.rides);

        // Find expired points (were in old profile but not in new)
        let mut expired_points = Vec::new();
        for old_point in &old_profile.points {
            let new_power = self
                .current_profile
                .power_at_duration(old_point.duration_secs);
            if new_power.is_none() || new_power < Some(old_point.power_watts) {
                expired_points.push(old_point.duration_secs);
            }
        }

        // Find improved points (better in new than old)
        let mut improved_points = Vec::new();
        for new_point in &self.current_profile.points {
            let old_power = old_profile.power_at_duration(new_point.duration_secs);
            if old_power.is_none() || old_power < Some(new_point.power_watts) {
                improved_points.push(new_point.clone());
            }
        }

        let changed = !expired_points.is_empty() || !improved_points.is_empty();

        RollingWindowUpdate {
            improved_points,
            expired_points,
            changed,
        }
    }

    /// Get the current rolling window profile.
    pub fn profile(&self) -> &PowerProfile {
        &self.current_profile
    }

    /// Get mutable reference to the profile.
    pub fn profile_mut(&mut self) -> &mut PowerProfile {
        &mut self.current_profile
    }

    /// Get count of rides in the current window.
    pub fn ride_count(&self) -> usize {
        self.calculator.ride_count_in_window(&self.rides)
    }

    /// Check if sufficient data exists for analysis.
    pub fn has_sufficient_data(&self) -> bool {
        self.calculator.has_sufficient_data(&self.rides)
    }

    /// Prune rides that are older than the window (to save memory).
    pub fn prune_old_rides(&mut self) {
        let cutoff = self.calculator.config().cutoff_date();
        self.rides.retain(|r| r.ride_date >= cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ride(
        days_ago: i64,
        power_5s: u16,
        power_60s: u16,
        power_1200s: u16,
    ) -> RidePowerData {
        let ride_date = Utc::now() - Duration::days(days_ago);
        RidePowerData::new(
            Uuid::new_v4(),
            ride_date,
            vec![(5, power_5s), (60, power_60s), (1200, power_1200s)],
        )
    }

    #[test]
    fn test_rolling_window_config() {
        let config = RollingWindowConfig::ninety_days();
        assert_eq!(config.window_days, 90);

        let cutoff = config.cutoff_date();
        let expected = Utc::now() - Duration::days(90);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn test_calculate_rolling_profile() {
        let calculator = RollingWindowCalculator::new();
        let user_id = Uuid::new_v4();

        let rides = vec![
            create_test_ride(10, 800, 400, 250),   // Recent, good power
            create_test_ride(30, 900, 350, 240),   // Older, best sprint
            create_test_ride(100, 1000, 500, 300), // Too old, should be excluded
        ];

        let profile = calculator.calculate(user_id, &rides);

        // Should have the best from rides within 90 days
        assert_eq!(profile.power_at_duration(5), Some(900)); // From 30-day-old ride
        assert_eq!(profile.power_at_duration(60), Some(400)); // From 10-day-old ride
        assert_eq!(profile.power_at_duration(1200), Some(250)); // From 10-day-old ride

        // 100-day-old ride should be excluded
        assert_ne!(profile.power_at_duration(5), Some(1000));
    }

    #[test]
    fn test_rolling_window_updater() {
        let user_id = Uuid::new_v4();
        let mut updater = RollingWindowUpdater::new(user_id);

        // Add first ride
        let ride1 = create_test_ride(1, 800, 400, 250);
        let update1 = updater.add_ride(ride1);
        assert!(update1.has_improvements());
        assert_eq!(update1.improved_points.len(), 3);

        // Add ride with some improvements
        let ride2 = create_test_ride(2, 850, 380, 260);
        let update2 = updater.add_ride(ride2);
        assert!(update2.has_improvements());
        // Should improve 5s (850 > 800) and 1200s (260 > 250)
        assert_eq!(update2.improved_points.len(), 2);

        // Check final profile
        assert_eq!(updater.profile().power_at_duration(5), Some(850));
        assert_eq!(updater.profile().power_at_duration(60), Some(400));
        assert_eq!(updater.profile().power_at_duration(1200), Some(260));
    }

    #[test]
    fn test_sufficient_data_check() {
        let calculator = RollingWindowCalculator::new();

        let few_rides = vec![
            create_test_ride(10, 800, 400, 250),
            create_test_ride(20, 850, 420, 260),
        ];
        assert!(!calculator.has_sufficient_data(&few_rides));

        let enough_rides = vec![
            create_test_ride(10, 800, 400, 250),
            create_test_ride(20, 850, 420, 260),
            create_test_ride(30, 820, 410, 255),
        ];
        assert!(calculator.has_sufficient_data(&enough_rides));
    }

    #[test]
    fn test_ride_count_in_window() {
        let calculator = RollingWindowCalculator::new();

        let rides = vec![
            create_test_ride(10, 800, 400, 250),  // In window
            create_test_ride(50, 850, 420, 260),  // In window
            create_test_ride(100, 820, 410, 255), // Out of window
        ];

        assert_eq!(calculator.ride_count_in_window(&rides), 2);
    }
}
