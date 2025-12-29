//! Adapter for integrating with existing MmpCalculator.
//!
//! T052: Integrate with existing MmpCalculator from metrics/analytics/pdc.rs.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::metrics::analytics::pdc::{MmpCalculator, PdcPoint, PowerDurationCurve};
use super::profile::{PowerProfile, PowerProfilePoint};
use super::rolling::RidePowerData;
use super::types::{ProfileType, PROFILE_DURATIONS};

/// Adapter for converting between PDC module types and power profile types.
pub struct MmpAdapter;

impl MmpAdapter {
    /// Calculate MMP values for standard profile durations from power samples.
    ///
    /// Uses the existing MmpCalculator infrastructure.
    pub fn calculate_profile_mmp(power_samples: &[u16]) -> Vec<(u32, u16)> {
        let calculator = MmpCalculator::new(&PROFILE_DURATIONS);
        let points = calculator.calculate(power_samples);

        points
            .into_iter()
            .map(|p| (p.duration_secs, p.power_watts))
            .collect()
    }

    /// Calculate MMP with sensor gap interpolation for better accuracy.
    pub fn calculate_profile_mmp_with_interpolation(power_samples: &[u16]) -> Vec<(u32, u16)> {
        let calculator = MmpCalculator::new(&PROFILE_DURATIONS);
        let points = calculator.calculate_with_interpolation(power_samples);

        points
            .into_iter()
            .map(|p| (p.duration_secs, p.power_watts))
            .collect()
    }

    /// Convert PDC points to profile MMP values.
    pub fn pdc_points_to_mmp(points: &[PdcPoint]) -> Vec<(u32, u16)> {
        points
            .iter()
            .filter(|p| PROFILE_DURATIONS.contains(&p.duration_secs))
            .map(|p| (p.duration_secs, p.power_watts))
            .collect()
    }

    /// Convert profile MMP values to PDC points.
    pub fn mmp_to_pdc_points(mmp_values: &[(u32, u16)]) -> Vec<PdcPoint> {
        mmp_values
            .iter()
            .map(|&(duration, power)| PdcPoint {
                duration_secs: duration,
                power_watts: power,
            })
            .collect()
    }

    /// Create RidePowerData from power samples.
    pub fn create_ride_data(
        ride_id: Uuid,
        ride_date: DateTime<Utc>,
        power_samples: &[u16],
    ) -> RidePowerData {
        let mmp_values = Self::calculate_profile_mmp_with_interpolation(power_samples);
        RidePowerData::new(ride_id, ride_date, mmp_values)
    }

    /// Convert PowerProfile to PowerDurationCurve.
    pub fn profile_to_pdc(profile: &PowerProfile) -> PowerDurationCurve {
        let points: Vec<PdcPoint> = profile
            .points
            .iter()
            .map(|p| PdcPoint {
                duration_secs: p.duration_secs,
                power_watts: p.power_watts,
            })
            .collect();

        PowerDurationCurve::from_points(points)
    }

    /// Convert PowerDurationCurve to PowerProfile.
    pub fn pdc_to_profile(pdc: &PowerDurationCurve, user_id: Uuid, profile_type: ProfileType) -> PowerProfile {
        let points: Vec<PowerProfilePoint> = pdc
            .points()
            .iter()
            .filter(|p| PROFILE_DURATIONS.contains(&p.duration_secs))
            .map(|p| PowerProfilePoint::new(p.duration_secs, p.power_watts))
            .collect();

        PowerProfile::with_points(user_id, profile_type, points)
    }

    /// Extract profile durations from a full PDC.
    ///
    /// The existing PDC may have many more durations than we need for profiling.
    /// This extracts just the standard profile durations.
    pub fn extract_profile_durations(pdc: &PowerDurationCurve) -> Vec<(u32, u16)> {
        PROFILE_DURATIONS
            .iter()
            .filter_map(|&duration| {
                pdc.power_at(duration).map(|power| (duration, power))
            })
            .collect()
    }
}

/// Helper for batch processing rides with MMP calculation.
pub struct RideMmpProcessor {
    calculator: MmpCalculator,
}

impl RideMmpProcessor {
    /// Create a new processor for profile durations.
    pub fn new() -> Self {
        Self {
            calculator: MmpCalculator::new(&PROFILE_DURATIONS),
        }
    }

    /// Process a single ride's power samples.
    pub fn process_ride(
        &self,
        ride_id: Uuid,
        ride_date: DateTime<Utc>,
        power_samples: &[u16],
    ) -> RidePowerData {
        let points = self.calculator.calculate_with_interpolation(power_samples);
        let mmp_values: Vec<(u32, u16)> = points
            .into_iter()
            .map(|p| (p.duration_secs, p.power_watts))
            .collect();

        RidePowerData::new(ride_id, ride_date, mmp_values)
    }

    /// Process multiple rides in batch.
    pub fn process_batch(
        &self,
        rides: &[(Uuid, DateTime<Utc>, Vec<u16>)],
    ) -> Vec<RidePowerData> {
        rides
            .iter()
            .map(|(id, date, samples)| self.process_ride(*id, *date, samples))
            .collect()
    }
}

impl Default for RideMmpProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_profile_mmp() {
        // 10 minutes at constant 250W
        let samples = vec![250u16; 600];

        let mmp = MmpAdapter::calculate_profile_mmp(&samples);

        // Should have values for durations <= 600s
        assert!(mmp.iter().any(|(d, _)| *d == 5));
        assert!(mmp.iter().any(|(d, _)| *d == 60));
        assert!(mmp.iter().any(|(d, _)| *d == 300));
        assert!(mmp.iter().any(|(d, _)| *d == 600));

        // All should be 250W
        for (_, power) in &mmp {
            assert_eq!(*power, 250);
        }
    }

    #[test]
    fn test_create_ride_data() {
        let ride_id = Uuid::new_v4();
        let ride_date = Utc::now();
        let samples = vec![300u16; 1200]; // 20 min at 300W

        let ride_data = MmpAdapter::create_ride_data(ride_id, ride_date, &samples);

        assert_eq!(ride_data.ride_id, ride_id);
        assert_eq!(ride_data.ride_date, ride_date);
        assert!(!ride_data.mmp_values.is_empty());

        // Check 5-min and 20-min values
        assert_eq!(ride_data.power_at(300), Some(300));
        assert_eq!(ride_data.power_at(1200), Some(300));
    }

    #[test]
    fn test_profile_to_pdc_conversion() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        profile.update_point(PowerProfilePoint::new(5, 800));
        profile.update_point(PowerProfilePoint::new(60, 400));
        profile.update_point(PowerProfilePoint::new(300, 320));

        let pdc = MmpAdapter::profile_to_pdc(&profile);

        assert_eq!(pdc.power_at(5), Some(800));
        assert_eq!(pdc.power_at(60), Some(400));
        assert_eq!(pdc.power_at(300), Some(320));
    }

    #[test]
    fn test_pdc_to_profile_conversion() {
        let points = vec![
            PdcPoint { duration_secs: 5, power_watts: 800 },
            PdcPoint { duration_secs: 10, power_watts: 700 }, // Not a standard duration
            PdcPoint { duration_secs: 60, power_watts: 400 },
        ];
        let pdc = PowerDurationCurve::from_points(points);

        let user_id = Uuid::new_v4();
        let profile = MmpAdapter::pdc_to_profile(&pdc, user_id, ProfileType::Current);

        // Should only include standard durations
        assert_eq!(profile.power_at_duration(5), Some(800));
        assert_eq!(profile.power_at_duration(60), Some(400));
        // 10s is not a standard duration, should not be included
        assert_eq!(profile.power_at_duration(10), None);
    }

    #[test]
    fn test_ride_processor_batch() {
        let processor = RideMmpProcessor::new();
        let now = Utc::now();

        let rides = vec![
            (Uuid::new_v4(), now, vec![200u16; 300]),
            (Uuid::new_v4(), now, vec![250u16; 600]),
        ];

        let results = processor.process_batch(&rides);

        assert_eq!(results.len(), 2);
        assert!(!results[0].mmp_values.is_empty());
        assert!(!results[1].mmp_values.is_empty());
    }

    #[test]
    fn test_extract_profile_durations() {
        let points = vec![
            PdcPoint { duration_secs: 1, power_watts: 1000 },  // Not standard
            PdcPoint { duration_secs: 5, power_watts: 800 },   // Standard
            PdcPoint { duration_secs: 10, power_watts: 700 },  // Not standard
            PdcPoint { duration_secs: 60, power_watts: 400 },  // Standard
            PdcPoint { duration_secs: 300, power_watts: 320 }, // Standard
        ];
        let pdc = PowerDurationCurve::from_points(points);

        let extracted = MmpAdapter::extract_profile_durations(&pdc);

        // PDC may interpolate values for other standard durations, but
        // we should at least have these exact values
        assert!(extracted.contains(&(5, 800)));
        assert!(extracted.contains(&(60, 400)));
        assert!(extracted.contains(&(300, 320)));
        // 1s and 10s are not standard profile durations
        assert!(!extracted.iter().any(|(d, _)| *d == 1 || *d == 10));
    }
}
