//! Training Load calculations (ATL/CTL/ACWR).
//!
//! Implements the Performance Management Chart (PMC) model:
//! - ATL (Acute Training Load): 7-day exponentially weighted moving average
//! - CTL (Chronic Training Load): 42-day exponentially weighted moving average
//! - TSB (Training Stress Balance): CTL - ATL
//! - ACWR (Acute:Chronic Workload Ratio): ATL / CTL

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Daily training load values.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DailyLoad {
    /// Total TSS for the day.
    pub tss: f32,
    /// Acute Training Load (7-day EWMA).
    pub atl: f32,
    /// Chronic Training Load (42-day EWMA).
    pub ctl: f32,
    /// Training Stress Balance (CTL - ATL).
    pub tsb: f32,
}

/// ACWR status thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcwrStatus {
    /// ACWR < 0.8: Undertrained (detraining risk).
    Undertrained,
    /// ACWR 0.8 - 1.3: Optimal training zone.
    Optimal,
    /// ACWR 1.3 - 1.5: Caution zone.
    Caution,
    /// ACWR > 1.5: High injury risk.
    HighRisk,
}

/// Acute:Chronic Workload Ratio result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Acwr {
    /// The ratio value.
    pub ratio: f32,
    /// Status classification.
    pub status: AcwrStatus,
}

/// Training load calculator.
pub struct TrainingLoadCalculator {
    /// ATL decay constant (default: 7 days).
    atl_days: f32,
    /// CTL decay constant (default: 42 days).
    ctl_days: f32,
}

impl TrainingLoadCalculator {
    /// Create with default constants (7/42 day).
    pub fn new() -> Self {
        Self {
            atl_days: 7.0,
            ctl_days: 42.0,
        }
    }

    /// Create with custom decay constants.
    pub fn with_constants(atl_days: f32, ctl_days: f32) -> Self {
        Self { atl_days, ctl_days }
    }

    /// Calculate training load for a date given previous day's values and today's TSS.
    pub fn calculate_day(&self, prev: DailyLoad, today_tss: f32) -> DailyLoad {
        // EWMA formula: new = old × (1 - k) + value × k
        // where k = 2 / (N + 1)
        let atl_k = 2.0 / (self.atl_days + 1.0);
        let ctl_k = 2.0 / (self.ctl_days + 1.0);

        let new_atl = prev.atl * (1.0 - atl_k) + today_tss * atl_k;
        let new_ctl = prev.ctl * (1.0 - ctl_k) + today_tss * ctl_k;
        let new_tsb = new_ctl - new_atl;

        DailyLoad {
            tss: today_tss,
            atl: new_atl,
            ctl: new_ctl,
            tsb: new_tsb,
        }
    }

    /// Calculate full history from daily TSS values.
    pub fn calculate_history(&self, daily_tss: &[(NaiveDate, f32)]) -> Vec<(NaiveDate, DailyLoad)> {
        if daily_tss.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(daily_tss.len());
        let mut prev = DailyLoad::default();

        for (date, tss) in daily_tss {
            let load = self.calculate_day(prev, *tss);
            results.push((*date, load));
            prev = load;
        }

        results
    }

    /// Calculate ACWR from current ATL/CTL.
    pub fn acwr(&self, atl: f32, ctl: f32) -> Acwr {
        let ratio = if ctl > 0.0 { atl / ctl } else { 0.0 };

        let status = if ratio < 0.8 {
            AcwrStatus::Undertrained
        } else if ratio <= 1.3 {
            AcwrStatus::Optimal
        } else if ratio <= 1.5 {
            AcwrStatus::Caution
        } else {
            AcwrStatus::HighRisk
        };

        Acwr { ratio, status }
    }

    /// Check if we have enough data for meaningful ACWR (28+ days).
    pub fn has_sufficient_history(&self, days: usize) -> bool {
        days >= 28
    }
}

impl Default for TrainingLoadCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Acwr {
    /// Get color for UI display (RGB).
    pub fn color(&self) -> (u8, u8, u8) {
        match self.status {
            AcwrStatus::Undertrained => (100, 149, 237), // Cornflower blue
            AcwrStatus::Optimal => (50, 205, 50),        // Lime green
            AcwrStatus::Caution => (255, 165, 0),        // Orange
            AcwrStatus::HighRisk => (220, 20, 60),       // Crimson
        }
    }

    /// Get recommendation text.
    pub fn recommendation(&self) -> &'static str {
        match self.status {
            AcwrStatus::Undertrained => {
                "Training load is low. Consider increasing training volume gradually."
            }
            AcwrStatus::Optimal => "Training load is in the optimal zone. Keep up the good work!",
            AcwrStatus::Caution => {
                "Training load is elevated. Monitor for signs of fatigue and consider recovery."
            }
            AcwrStatus::HighRisk => {
                "Training load spike detected. High injury risk. Reduce training intensity."
            }
        }
    }
}

impl DailyLoad {
    /// Create a new DailyLoad with initial values.
    pub fn new(tss: f32) -> Self {
        Self {
            tss,
            atl: tss,
            ctl: tss,
            tsb: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // T077: Unit test for EWMA ATL calculation (7-day)
    #[test]
    fn test_atl_calculation() {
        let calc = TrainingLoadCalculator::new();

        // Start with 0 load, add 100 TSS
        let day1 = calc.calculate_day(DailyLoad::default(), 100.0);

        // ATL should be 100 * (2/8) = 25
        assert!((day1.atl - 25.0).abs() < 0.1);

        // Add another 100 TSS
        let day2 = calc.calculate_day(day1, 100.0);
        // ATL should be 25 * 0.75 + 100 * 0.25 = 43.75
        assert!((day2.atl - 43.75).abs() < 0.1);
    }

    // T078: Unit test for EWMA CTL calculation (42-day)
    #[test]
    fn test_ctl_calculation() {
        let calc = TrainingLoadCalculator::new();

        // Start with 0 load, add 100 TSS
        let day1 = calc.calculate_day(DailyLoad::default(), 100.0);

        // CTL k = 2/43 ≈ 0.0465
        // CTL should be 100 * 0.0465 ≈ 4.65
        assert!((day1.ctl - 4.65).abs() < 0.1);
    }

    // T079: Unit test for ACWR calculation and status thresholds
    #[test]
    fn test_acwr_thresholds() {
        let calc = TrainingLoadCalculator::new();

        // Undertrained: ACWR < 0.8
        let under = calc.acwr(40.0, 100.0);
        assert_eq!(under.status, AcwrStatus::Undertrained);
        assert!((under.ratio - 0.4).abs() < 0.01);

        // Optimal: 0.8 - 1.3
        let optimal = calc.acwr(100.0, 100.0);
        assert_eq!(optimal.status, AcwrStatus::Optimal);

        // Caution: 1.3 - 1.5
        let caution = calc.acwr(140.0, 100.0);
        assert_eq!(caution.status, AcwrStatus::Caution);

        // High risk: > 1.5
        let high_risk = calc.acwr(200.0, 100.0);
        assert_eq!(high_risk.status, AcwrStatus::HighRisk);
    }

    // T080: Unit test for cold start handling (no historical data)
    #[test]
    fn test_cold_start() {
        let calc = TrainingLoadCalculator::new();

        // Empty history
        let history = calc.calculate_history(&[]);
        assert!(history.is_empty());

        // First day with TSS
        let daily_tss = vec![(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 100.0)];
        let history = calc.calculate_history(&daily_tss);

        assert_eq!(history.len(), 1);
        assert!(history[0].1.atl > 0.0);
        assert!(history[0].1.ctl > 0.0);

        // Check insufficient history flag
        assert!(!calc.has_sufficient_history(1));
        assert!(calc.has_sufficient_history(28));
    }

    #[test]
    fn test_tsb_calculation() {
        let calc = TrainingLoadCalculator::new();

        // Build up load over several days
        let mut prev = DailyLoad::default();
        for _ in 0..30 {
            prev = calc.calculate_day(prev, 100.0);
        }

        // TSB should be CTL - ATL
        assert!((prev.tsb - (prev.ctl - prev.atl)).abs() < 0.01);
    }

    #[test]
    fn test_acwr_colors() {
        let optimal = Acwr {
            ratio: 1.0,
            status: AcwrStatus::Optimal,
        };
        let (r, g, b) = optimal.color();
        assert_eq!((r, g, b), (50, 205, 50)); // Green

        let high_risk = Acwr {
            ratio: 2.0,
            status: AcwrStatus::HighRisk,
        };
        let (r, g, b) = high_risk.color();
        assert_eq!((r, g, b), (220, 20, 60)); // Red
    }
}
