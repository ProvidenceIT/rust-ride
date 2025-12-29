//! Power profile manager for coordinating profile updates.
//!
//! T051: Create PowerProfileManager trait implementation.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::analysis::ProfileAnalysis;
use super::lifetime::LifetimeBestTracker;
use super::profile::{PowerProfile, PowerProfilePoint};
use super::rider_type::{RiderClassification, RiderType};
use super::rolling::{RidePowerData, RollingWindowUpdate, RollingWindowUpdater};
use super::types::ProfileType;

/// Result of processing a ride's power data.
#[derive(Debug, Clone)]
pub struct RideProcessResult {
    /// New rolling window PRs.
    pub rolling_prs: Vec<PowerProfilePoint>,
    /// New lifetime PRs.
    pub lifetime_prs: Vec<PowerProfilePoint>,
    /// Whether the rider type classification changed.
    pub classification_changed: bool,
    /// Current rider classification after processing.
    pub classification: Option<RiderClassification>,
}

impl RideProcessResult {
    /// Check if any new PRs were achieved.
    pub fn has_new_prs(&self) -> bool {
        !self.rolling_prs.is_empty() || !self.lifetime_prs.is_empty()
    }

    /// Get total count of new PRs.
    pub fn total_pr_count(&self) -> usize {
        self.rolling_prs.len() + self.lifetime_prs.len()
    }

    /// Get only the lifetime PRs (excluding rolling duplicates).
    pub fn unique_lifetime_prs(&self) -> Vec<&PowerProfilePoint> {
        self.lifetime_prs
            .iter()
            .filter(|lp| {
                !self
                    .rolling_prs
                    .iter()
                    .any(|rp| rp.duration_secs == lp.duration_secs)
            })
            .collect()
    }
}

/// Manager for power profiles (both rolling and lifetime).
pub struct PowerProfileManager {
    /// User this manager is for.
    #[allow(dead_code)]
    user_id: Uuid,
    /// Rolling window profile updater.
    rolling: RollingWindowUpdater,
    /// Lifetime best tracker.
    lifetime: LifetimeBestTracker,
    /// User's weight for W/kg calculations.
    weight_kg: Option<f64>,
    /// Current rider classification.
    classification: Option<RiderClassification>,
}

impl PowerProfileManager {
    /// Create a new profile manager for a user.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            rolling: RollingWindowUpdater::new(user_id),
            lifetime: LifetimeBestTracker::new(user_id),
            weight_kg: None,
            classification: None,
        }
    }

    /// Create with existing profiles and ride data.
    pub fn with_data(
        user_id: Uuid,
        _rolling_profile: PowerProfile,
        lifetime_profile: PowerProfile,
        rides: Vec<RidePowerData>,
        weight_kg: Option<f64>,
    ) -> Self {
        let mut manager = Self {
            user_id,
            rolling: RollingWindowUpdater::with_rides(user_id, rides),
            lifetime: LifetimeBestTracker::with_profile(lifetime_profile),
            weight_kg,
            classification: None,
        };

        // Update classification
        manager.update_classification();

        manager
    }

    /// Set user weight for W/kg calculations.
    pub fn set_weight(&mut self, weight_kg: f64) {
        self.weight_kg = Some(weight_kg);
        self.update_classification();
    }

    /// Process a new ride's power data.
    ///
    /// Updates both rolling and lifetime profiles, recalculates classification.
    pub fn process_ride(
        &mut self,
        ride_id: Uuid,
        ride_date: DateTime<Utc>,
        mmp_values: Vec<(u32, u16)>,
    ) -> RideProcessResult {
        // Create ride data for rolling window
        let ride_data = RidePowerData::new(ride_id, ride_date, mmp_values.clone());

        // Update rolling window profile
        let rolling_update = self.rolling.add_ride(ride_data);

        // Check for lifetime bests
        let lifetime_result = self.lifetime.check_ride(ride_id, ride_date, &mmp_values);

        // Convert lifetime bests to PowerProfilePoints
        let lifetime_prs: Vec<PowerProfilePoint> = lifetime_result
            .new_bests
            .iter()
            .map(|lb| {
                PowerProfilePoint::with_timestamp(lb.duration_secs, lb.power_watts, lb.achieved_at)
                    .with_ride(ride_id)
            })
            .collect();

        // Check if classification changed
        let old_classification = self.classification.as_ref().map(|c| c.rider_type);
        self.update_classification();
        let new_classification = self.classification.as_ref().map(|c| c.rider_type);
        let classification_changed = old_classification != new_classification;

        RideProcessResult {
            rolling_prs: rolling_update.improved_points,
            lifetime_prs,
            classification_changed,
            classification: self.classification.clone(),
        }
    }

    /// Recalculate profiles (e.g., after date change).
    pub fn recalculate(&mut self) -> RollingWindowUpdate {
        let update = self.rolling.recalculate();
        self.update_classification();
        update
    }

    /// Get the current rolling window profile.
    pub fn rolling_profile(&self) -> &PowerProfile {
        self.rolling.profile()
    }

    /// Get the lifetime best profile.
    pub fn lifetime_profile(&self) -> &PowerProfile {
        self.lifetime.profile()
    }

    /// Get profile analysis for the rolling window.
    pub fn analyze_rolling(&self) -> ProfileAnalysis {
        ProfileAnalysis::from_profile(self.rolling.profile(), self.weight_kg)
    }

    /// Get profile analysis for lifetime bests.
    pub fn analyze_lifetime(&self) -> ProfileAnalysis {
        ProfileAnalysis::from_profile(self.lifetime.profile(), self.weight_kg)
    }

    /// Get current rider classification.
    pub fn classification(&self) -> Option<&RiderClassification> {
        self.classification.as_ref()
    }

    /// Get rider type from current classification.
    pub fn rider_type(&self) -> RiderType {
        self.classification
            .as_ref()
            .map(|c| c.rider_type)
            .unwrap_or(RiderType::Unknown)
    }

    /// Check if sufficient data exists for analysis.
    pub fn has_sufficient_data(&self) -> bool {
        self.rolling.has_sufficient_data()
    }

    /// Get count of rides in the rolling window.
    pub fn ride_count(&self) -> usize {
        self.rolling.ride_count()
    }

    /// Compare rolling profile to lifetime bests.
    ///
    /// Returns percentage of lifetime best at each duration.
    pub fn compare_rolling_to_lifetime(&self) -> Vec<(u32, f64)> {
        let rolling = self.rolling.profile();
        let mmp_values: Vec<(u32, u16)> = rolling
            .points
            .iter()
            .map(|p| (p.duration_secs, p.power_watts))
            .collect();

        self.lifetime.compare_to_lifetime(&mmp_values)
    }

    /// Get estimated FTP from rolling profile.
    pub fn estimated_ftp_rolling(&self) -> Option<u16> {
        self.rolling.profile().estimated_ftp()
    }

    /// Get estimated FTP from lifetime bests.
    pub fn estimated_ftp_lifetime(&self) -> Option<u16> {
        self.lifetime.estimated_ftp()
    }

    /// Get W/kg at FTP for rolling profile.
    pub fn watts_per_kg_ftp(&self) -> Option<f64> {
        self.estimated_ftp_rolling()
            .and_then(|ftp| self.weight_kg.map(|w| ftp as f64 / w))
    }

    /// Prune old ride data to save memory.
    pub fn prune_old_data(&mut self) {
        self.rolling.prune_old_rides();
    }

    /// Update rider classification based on current profile.
    fn update_classification(&mut self) {
        let analysis = self.analyze_rolling();
        self.classification = Some(RiderClassification::from_analysis(
            &analysis,
            self.weight_kg,
        ));
    }
}

/// Builder for PowerProfileManager with fluent API.
pub struct PowerProfileManagerBuilder {
    user_id: Uuid,
    rolling_profile: Option<PowerProfile>,
    lifetime_profile: Option<PowerProfile>,
    rides: Vec<RidePowerData>,
    weight_kg: Option<f64>,
}

impl PowerProfileManagerBuilder {
    /// Create a new builder.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            rolling_profile: None,
            lifetime_profile: None,
            rides: Vec::new(),
            weight_kg: None,
        }
    }

    /// Set the rolling profile.
    pub fn rolling_profile(mut self, profile: PowerProfile) -> Self {
        self.rolling_profile = Some(profile);
        self
    }

    /// Set the lifetime profile.
    pub fn lifetime_profile(mut self, profile: PowerProfile) -> Self {
        self.lifetime_profile = Some(profile);
        self
    }

    /// Add rides for rolling window calculation.
    pub fn rides(mut self, rides: Vec<RidePowerData>) -> Self {
        self.rides = rides;
        self
    }

    /// Set user weight.
    pub fn weight_kg(mut self, weight: f64) -> Self {
        self.weight_kg = Some(weight);
        self
    }

    /// Build the manager.
    pub fn build(self) -> PowerProfileManager {
        let rolling_profile = self
            .rolling_profile
            .unwrap_or_else(|| PowerProfile::new(self.user_id, ProfileType::Current));
        let lifetime_profile = self
            .lifetime_profile
            .unwrap_or_else(|| PowerProfile::new(self.user_id, ProfileType::Lifetime));

        PowerProfileManager::with_data(
            self.user_id,
            rolling_profile,
            lifetime_profile,
            self.rides,
            self.weight_kg,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_mmp(power_5s: u16, power_60s: u16, power_1200s: u16) -> Vec<(u32, u16)> {
        vec![(5, power_5s), (60, power_60s), (1200, power_1200s)]
    }

    #[test]
    fn test_process_ride() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        let ride_id = Uuid::new_v4();
        let now = Utc::now();

        // First ride
        let result1 = manager.process_ride(ride_id, now, create_test_mmp(800, 400, 250));
        assert!(result1.has_new_prs());
        assert_eq!(result1.total_pr_count(), 6); // 3 rolling + 3 lifetime

        // Second ride with some improvements
        let ride_id2 = Uuid::new_v4();
        let result2 = manager.process_ride(
            ride_id2,
            now + Duration::days(1),
            create_test_mmp(850, 380, 260),
        );

        // Should have 2 rolling PRs (5s, 1200s) and 2 lifetime PRs (5s, 1200s)
        assert!(result2.has_new_prs());
    }

    #[test]
    fn test_compare_rolling_to_lifetime() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);

        // Add a ride that will be in both rolling and lifetime
        let now = Utc::now();
        manager.process_ride(Uuid::new_v4(), now, create_test_mmp(1000, 500, 300));

        // Add a ride with lower power (won't beat lifetime, but still in rolling)
        manager.process_ride(
            Uuid::new_v4(),
            now + Duration::days(1),
            create_test_mmp(900, 480, 290),
        );

        let comparison = manager.compare_rolling_to_lifetime();

        // All should be at or near 100% since rolling matches lifetime
        for (_duration, pct) in &comparison {
            assert!(*pct >= 90.0 && *pct <= 100.0);
        }
    }

    #[test]
    fn test_classification_update() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        // Add a "sprinter" profile (strong short, weak long)
        let now = Utc::now();
        manager.process_ride(
            Uuid::new_v4(),
            now,
            vec![
                (5, 1200), // Very strong sprint
                (15, 900),
                (30, 600),
                (60, 450),
                (180, 350),
                (300, 300),
                (600, 270),
                (1200, 250),
                (3600, 200),
            ],
        );

        // Should classify as Sprinter
        assert!(manager.classification().is_some());
        assert_eq!(manager.rider_type(), RiderType::Sprinter);
    }

    #[test]
    fn test_builder() {
        let user_id = Uuid::new_v4();

        let manager = PowerProfileManagerBuilder::new(user_id)
            .weight_kg(75.0)
            .build();

        assert_eq!(manager.user_id, user_id);
        assert_eq!(manager.weight_kg, Some(75.0));
    }

    #[test]
    fn test_estimated_ftp() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        let now = Utc::now();
        manager.process_ride(
            Uuid::new_v4(),
            now,
            vec![(1200, 300)], // 20-min power
        );

        // FTP should be ~95% of 20-min power
        let ftp = manager.estimated_ftp_rolling();
        assert_eq!(ftp, Some(285)); // 300 * 0.95 = 285
    }

    #[test]
    fn test_watts_per_kg() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        let now = Utc::now();
        manager.process_ride(
            Uuid::new_v4(),
            now,
            vec![(1200, 280)], // 20-min power = 280W -> FTP ~266W
        );

        let wpk = manager.watts_per_kg_ftp();
        assert!(wpk.is_some());
        // 266W / 70kg = ~3.8 W/kg
        assert!((wpk.unwrap() - 3.8).abs() < 0.1);
    }
}
