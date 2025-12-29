//! Integration with ride recording and save flow.
//!
//! T056: Integrate power profile updates into ride save flow.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// Note: LifetimeCheckResult is used indirectly through manager
use super::manager::{PowerProfileManager, RideProcessResult};
use super::mmp_adapter::MmpAdapter;
use super::rolling::RidePowerData;
use crate::recording::types::RideSample;

/// Extract power samples from ride samples.
///
/// Filters to valid power readings and returns as Vec<u16>.
pub fn extract_power_samples(samples: &[RideSample]) -> Vec<u16> {
    samples
        .iter()
        .filter_map(|s| s.power_watts)
        .collect()
}

/// Process a completed ride and update power profiles.
///
/// This is the main integration point for the ride save flow.
/// Call this after a ride is completed to update both rolling and lifetime profiles.
pub fn process_ride_for_profiles(
    manager: &mut PowerProfileManager,
    ride_id: Uuid,
    ride_date: DateTime<Utc>,
    samples: &[RideSample],
) -> RideProcessResult {
    let power_samples = extract_power_samples(samples);

    if power_samples.is_empty() {
        return RideProcessResult {
            rolling_prs: Vec::new(),
            lifetime_prs: Vec::new(),
            classification_changed: false,
            classification: manager.classification().cloned(),
        };
    }

    // Calculate MMP values for this ride
    let mmp_values = MmpAdapter::calculate_profile_mmp_with_interpolation(&power_samples);

    // Process through the manager
    manager.process_ride(ride_id, ride_date, mmp_values)
}

/// Create RidePowerData from ride samples.
///
/// Use this when loading historical rides to build the power profile.
pub fn ride_samples_to_power_data(
    ride_id: Uuid,
    ride_date: DateTime<Utc>,
    samples: &[RideSample],
) -> Option<RidePowerData> {
    let power_samples = extract_power_samples(samples);

    if power_samples.is_empty() {
        return None;
    }

    Some(MmpAdapter::create_ride_data(ride_id, ride_date, &power_samples))
}

/// Summary of power profile updates after a ride.
#[derive(Debug, Clone)]
pub struct PowerProfileUpdateSummary {
    /// Number of new rolling window PRs.
    pub rolling_pr_count: usize,
    /// Number of new lifetime PRs.
    pub lifetime_pr_count: usize,
    /// New estimated FTP (if changed).
    pub new_ftp: Option<u16>,
    /// Previous FTP for comparison.
    pub previous_ftp: Option<u16>,
    /// Whether rider classification changed.
    pub classification_changed: bool,
    /// New rider type name (if classification exists).
    pub rider_type_name: Option<String>,
    /// Durations where new PRs were achieved.
    pub pr_durations: Vec<String>,
}

impl PowerProfileUpdateSummary {
    /// Create from a ride process result.
    pub fn from_result(
        result: &RideProcessResult,
        previous_ftp: Option<u16>,
        current_ftp: Option<u16>,
    ) -> Self {
        let pr_durations: Vec<String> = result
            .rolling_prs
            .iter()
            .chain(result.lifetime_prs.iter())
            .map(|p| crate::power_profile::duration_label(p.duration_secs))
            .collect();

        Self {
            rolling_pr_count: result.rolling_prs.len(),
            lifetime_pr_count: result.lifetime_prs.len(),
            new_ftp: if current_ftp != previous_ftp { current_ftp } else { None },
            previous_ftp,
            classification_changed: result.classification_changed,
            rider_type_name: result.classification.as_ref().map(|c| c.rider_type.display_name().to_string()),
            pr_durations,
        }
    }

    /// Check if there are any notable updates.
    pub fn has_updates(&self) -> bool {
        self.rolling_pr_count > 0
            || self.lifetime_pr_count > 0
            || self.new_ftp.is_some()
            || self.classification_changed
    }

    /// Get a summary message for display.
    pub fn summary_message(&self) -> String {
        let mut parts = Vec::new();

        if self.lifetime_pr_count > 0 {
            parts.push(format!("{} lifetime PR(s)!", self.lifetime_pr_count));
        }

        if self.rolling_pr_count > 0 && self.lifetime_pr_count == 0 {
            parts.push(format!("{} 90-day PR(s)", self.rolling_pr_count));
        }

        if let Some(new_ftp) = self.new_ftp {
            if let Some(prev) = self.previous_ftp {
                let diff = new_ftp as i32 - prev as i32;
                parts.push(format!("FTP: {} → {}W ({:+}W)", prev, new_ftp, diff));
            } else {
                parts.push(format!("FTP: {}W", new_ftp));
            }
        }

        if self.classification_changed {
            if let Some(ref name) = self.rider_type_name {
                parts.push(format!("New classification: {}", name));
            }
        }

        if parts.is_empty() {
            "No power profile changes".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

/// Batch processor for loading historical rides into power profile.
pub struct HistoricalRideProcessor {
    manager: PowerProfileManager,
}

impl HistoricalRideProcessor {
    /// Create a new processor for a user.
    pub fn new(user_id: Uuid, weight_kg: f64) -> Self {
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(weight_kg);

        Self { manager }
    }

    /// Process a historical ride.
    pub fn process_ride(
        &mut self,
        ride_id: Uuid,
        ride_date: DateTime<Utc>,
        samples: &[RideSample],
    ) -> bool {
        let power_samples = extract_power_samples(samples);

        if power_samples.is_empty() {
            return false;
        }

        let mmp_values = MmpAdapter::calculate_profile_mmp(&power_samples);
        self.manager.process_ride(ride_id, ride_date, mmp_values);
        true
    }

    /// Get the completed manager.
    pub fn into_manager(self) -> PowerProfileManager {
        self.manager
    }

    /// Get reference to the manager.
    pub fn manager(&self) -> &PowerProfileManager {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::types::RideSample;

    fn create_test_samples(power_values: &[u16]) -> Vec<RideSample> {
        power_values
            .iter()
            .enumerate()
            .map(|(i, &power)| RideSample {
                elapsed_seconds: i as u32,
                power_watts: Some(power),
                cadence_rpm: Some(90),
                heart_rate_bpm: Some(140),
                speed_kmh: Some(30.0),
                distance_meters: i as f64 * 10.0,
                calories: i as u32,
                resistance_level: None,
                target_power: None,
                trainer_grade: None,
                left_right_balance: None,
                left_torque_effectiveness: None,
                right_torque_effectiveness: None,
                left_pedal_smoothness: None,
                right_pedal_smoothness: None,
                left_power_phase_start: None,
                left_power_phase_end: None,
                left_power_phase_peak: None,
                right_power_phase_start: None,
                right_power_phase_end: None,
                right_power_phase_peak: None,
            })
            .collect()
    }

    #[test]
    fn test_extract_power_samples() {
        let samples = create_test_samples(&[200, 250, 300, 280, 260]);
        let power = extract_power_samples(&samples);

        assert_eq!(power.len(), 5);
        assert_eq!(power[0], 200);
        assert_eq!(power[2], 300);
    }

    #[test]
    fn test_extract_power_samples_with_gaps() {
        let mut samples = create_test_samples(&[200, 250, 300]);
        samples[1].power_watts = None; // Create a gap

        let power = extract_power_samples(&samples);
        assert_eq!(power.len(), 2);
        assert_eq!(power[0], 200);
        assert_eq!(power[1], 300);
    }

    #[test]
    fn test_ride_samples_to_power_data() {
        let samples = create_test_samples(&[200; 600]); // 10 minutes
        let ride_id = Uuid::new_v4();
        let now = Utc::now();

        let power_data = ride_samples_to_power_data(ride_id, now, &samples);

        assert!(power_data.is_some());
        let data = power_data.unwrap();
        assert_eq!(data.ride_id, ride_id);
        assert!(!data.mmp_values.is_empty());
    }

    #[test]
    fn test_process_ride_for_profiles() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        let samples = create_test_samples(&[250; 1200]); // 20 minutes at 250W
        let ride_id = Uuid::new_v4();
        let now = Utc::now();

        let result = process_ride_for_profiles(&mut manager, ride_id, now, &samples);

        assert!(result.has_new_prs());
    }

    #[test]
    fn test_update_summary() {
        let user_id = Uuid::new_v4();
        let mut manager = PowerProfileManager::new(user_id);
        manager.set_weight(70.0);

        let samples = create_test_samples(&[280; 1200]); // 20 min at 280W
        let ride_id = Uuid::new_v4();
        let now = Utc::now();

        let result = process_ride_for_profiles(&mut manager, ride_id, now, &samples);
        let summary = PowerProfileUpdateSummary::from_result(&result, None, manager.estimated_ftp_rolling());

        assert!(summary.has_updates());
        assert!(!summary.summary_message().contains("No power profile changes"));
    }

    #[test]
    fn test_historical_processor() {
        let user_id = Uuid::new_v4();
        let mut processor = HistoricalRideProcessor::new(user_id, 70.0);

        // Process multiple rides
        for i in 0..5 {
            let samples = create_test_samples(&[250 + i as u16 * 10; 600]);
            let ride_id = Uuid::new_v4();
            let date = Utc::now() - chrono::Duration::days(i as i64 * 10);
            processor.process_ride(ride_id, date, &samples);
        }

        let manager = processor.into_manager();
        assert!(manager.has_sufficient_data());
    }
}
