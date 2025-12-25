//! Analytics update triggers.
//!
//! T038: Trigger PDC update after ride save
//! T060: Trigger CP recalculation when PDC updates in 2-20min range
//! T093: Trigger daily training load update after ride save
//! T103: Trigger VO2max recalculation when 5-min PDC updates

use chrono::{NaiveDate, Utc};

use super::critical_power::CpFitter;
use super::pdc::{MmpCalculator, PdcPoint, PowerDurationCurve};
use super::training_load::{DailyLoad, TrainingLoadCalculator};
use super::vo2max::{Vo2maxCalculator, Vo2maxResult};
use super::CpModel;

/// Result from analytics trigger operations.
#[derive(Debug, Default)]
pub struct TriggerResult {
    /// PDC points that were updated
    pub pdc_updated: Vec<PdcPoint>,
    /// Whether CP model was recalculated
    pub cp_recalculated: bool,
    /// New CP model if recalculated
    pub new_cp_model: Option<CpModel>,
    /// Whether training load was updated
    pub training_load_updated: bool,
    /// New daily load if updated
    pub new_daily_load: Option<DailyLoad>,
    /// Whether VO2max was recalculated
    pub vo2max_recalculated: bool,
    /// New VO2max if recalculated
    pub new_vo2max: Option<Vo2maxResult>,
}

/// Analytics triggers for post-ride-save updates.
///
/// This struct provides methods to update analytics after a ride is saved.
#[allow(dead_code)]
pub struct AnalyticsTriggers {
    /// User's weight in kg for VO2max calculations
    weight_kg: f32,
    /// User's age for VO2max percentile (for future percentile lookup)
    age: u8,
    /// Whether user is male (for future percentile calculation)
    is_male: bool,
}

impl AnalyticsTriggers {
    /// Create a new triggers instance.
    pub fn new(weight_kg: f32, age: u8, is_male: bool) -> Self {
        Self {
            weight_kg,
            age,
            is_male,
        }
    }

    /// T038: Update PDC from ride power samples.
    ///
    /// Calculates MMP (maximal mean power) for standard durations and returns
    /// points that improved upon the existing PDC.
    pub fn update_pdc_from_ride(
        &self,
        power_samples: &[u16],
        existing_pdc: &PowerDurationCurve,
    ) -> Vec<PdcPoint> {
        if power_samples.is_empty() {
            return Vec::new();
        }

        // Calculate MMP for this ride
        let calculator = MmpCalculator::standard();
        let ride_points = calculator.calculate(power_samples);

        // Find points that improved upon the existing PDC
        let mut improved_points = Vec::new();
        for point in ride_points {
            let existing_power = existing_pdc.power_at(point.duration_secs);
            if existing_power.map(|p| point.power_watts > p).unwrap_or(true) {
                improved_points.push(point);
            }
        }

        improved_points
    }

    /// T060: Check if CP should be recalculated and do so if needed.
    ///
    /// CP is recalculated when PDC points in the 2-20 minute range are updated.
    pub fn maybe_recalculate_cp(
        &self,
        updated_pdc_points: &[PdcPoint],
        full_pdc: &PowerDurationCurve,
    ) -> Option<CpModel> {
        // Check if any updates were in the CP-relevant range (2-20 minutes)
        let cp_range_updated = updated_pdc_points.iter().any(|p| {
            p.duration_secs >= 120 && p.duration_secs <= 1200 // 2-20 min
        });

        if !cp_range_updated {
            return None;
        }

        // Check if we have sufficient data for CP calculation
        if !full_pdc.has_sufficient_data_for_cp() {
            return None;
        }

        // Fit new CP model
        let fitter = CpFitter::default();
        fitter.fit(full_pdc).ok()
    }

    /// T093: Update daily training load after ride save.
    ///
    /// Calculates new ATL/CTL/TSB based on today's TSS.
    pub fn update_training_load(
        &self,
        today_tss: f32,
        yesterday_load: Option<&DailyLoad>,
    ) -> DailyLoad {
        let calculator = TrainingLoadCalculator::new();
        let prev = yesterday_load.copied().unwrap_or_default();
        calculator.calculate_day(prev, today_tss)
    }

    /// T103: Check if VO2max should be recalculated and do so if needed.
    ///
    /// VO2max is recalculated when the 5-minute power PDC point is updated.
    pub fn maybe_recalculate_vo2max(
        &self,
        updated_pdc_points: &[PdcPoint],
        full_pdc: &PowerDurationCurve,
    ) -> Option<Vo2maxResult> {
        // Check if 5-minute power was updated
        let five_min_updated = updated_pdc_points.iter().any(|p| p.duration_secs == 300);

        if !five_min_updated {
            return None;
        }

        // Get the 5-minute power
        let five_min_power = full_pdc.power_at(300)?;

        // Calculate VO2max using Hawley-Noakes formula
        let calculator = Vo2maxCalculator::new(self.weight_kg);
        Some(calculator.from_five_minute_power(five_min_power))
    }

    /// Run all triggers after a ride save.
    ///
    /// This is the main entry point for post-ride analytics updates.
    pub fn run_all_triggers(
        &self,
        power_samples: &[u16],
        ride_tss: Option<f32>,
        existing_pdc: &PowerDurationCurve,
        yesterday_load: Option<&DailyLoad>,
    ) -> TriggerResult {
        let mut result = TriggerResult::default();

        // T038: Update PDC
        let updated_points = self.update_pdc_from_ride(power_samples, existing_pdc);
        result.pdc_updated = updated_points.clone();

        if !updated_points.is_empty() {
            // Create updated PDC for subsequent calculations
            let mut full_pdc = existing_pdc.clone();
            full_pdc.update(&updated_points);

            // T060: Check if CP needs recalculation
            if let Some(cp_model) = self.maybe_recalculate_cp(&updated_points, &full_pdc) {
                result.cp_recalculated = true;
                result.new_cp_model = Some(cp_model);
            }

            // T103: Check if VO2max needs recalculation
            if let Some(vo2max) = self.maybe_recalculate_vo2max(&updated_points, &full_pdc) {
                result.vo2max_recalculated = true;
                result.new_vo2max = Some(vo2max);
            }
        }

        // T093: Update training load
        if let Some(tss) = ride_tss {
            let daily_load = self.update_training_load(tss, yesterday_load);
            result.training_load_updated = true;
            result.new_daily_load = Some(daily_load);
        }

        result
    }
}

/// Helper to get yesterday's date.
pub fn yesterday() -> NaiveDate {
    Utc::now().date_naive() - chrono::Duration::days(1)
}

/// Helper to get today's date.
pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_pdc_from_ride() {
        let triggers = AnalyticsTriggers::new(75.0, 35, true);

        // Empty existing PDC
        let existing = PowerDurationCurve::new();

        // Simulate a 10-minute ride with constant 200W
        let power_samples: Vec<u16> = vec![200; 600]; // 10 minutes at 1 sample/sec

        let updated = triggers.update_pdc_from_ride(&power_samples, &existing);

        // Should have updates for durations up to 600 seconds
        assert!(!updated.is_empty());

        // Check that 5-minute power is correct
        let five_min = updated.iter().find(|p| p.duration_secs == 300);
        assert!(five_min.is_some());
        assert_eq!(five_min.unwrap().power_watts, 200);
    }

    #[test]
    fn test_no_update_for_lower_power() {
        let triggers = AnalyticsTriggers::new(75.0, 35, true);

        // Existing PDC with 250W 5-minute power
        let existing = PowerDurationCurve::from_points(vec![PdcPoint {
            duration_secs: 300,
            power_watts: 250,
        }]);

        // New ride with only 200W
        let power_samples: Vec<u16> = vec![200; 600];

        let updated = triggers.update_pdc_from_ride(&power_samples, &existing);

        // 5-minute point should NOT be in updates (200 < 250)
        let five_min = updated.iter().find(|p| p.duration_secs == 300);
        assert!(five_min.is_none());
    }

    #[test]
    fn test_training_load_update() {
        let triggers = AnalyticsTriggers::new(75.0, 35, true);

        // No previous load (cold start)
        let load = triggers.update_training_load(80.0, None);

        assert!(load.tss > 0.0);
        assert!(load.atl > 0.0);
        assert!(load.ctl > 0.0);
    }

    #[test]
    fn test_cp_recalculation_trigger() {
        let triggers = AnalyticsTriggers::new(75.0, 35, true);

        // PDC with sufficient data for CP
        let pdc = PowerDurationCurve::from_points(vec![
            PdcPoint {
                duration_secs: 180,
                power_watts: 320,
            }, // 3 min
            PdcPoint {
                duration_secs: 300,
                power_watts: 290,
            }, // 5 min
            PdcPoint {
                duration_secs: 600,
                power_watts: 270,
            }, // 10 min
            PdcPoint {
                duration_secs: 1200,
                power_watts: 250,
            }, // 20 min
        ]);

        // Update in CP range
        let updated_points = vec![PdcPoint {
            duration_secs: 600,
            power_watts: 275,
        }];

        let cp = triggers.maybe_recalculate_cp(&updated_points, &pdc);
        assert!(cp.is_some());
    }

    #[test]
    fn test_vo2max_recalculation_trigger() {
        let triggers = AnalyticsTriggers::new(75.0, 35, true);

        // PDC with 5-minute power
        let pdc = PowerDurationCurve::from_points(vec![PdcPoint {
            duration_secs: 300,
            power_watts: 350,
        }]);

        // 5-min power was updated
        let updated_points = vec![PdcPoint {
            duration_secs: 300,
            power_watts: 350,
        }];

        let vo2max = triggers.maybe_recalculate_vo2max(&updated_points, &pdc);
        assert!(vo2max.is_some());
        assert!(vo2max.unwrap().vo2max > 0.0);
    }
}
