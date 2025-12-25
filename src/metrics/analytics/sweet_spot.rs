//! Sweet Spot Training Recommendations.
//!
//! Generates personalized workout recommendations based on:
//! - Current training load (ATL/CTL)
//! - Power profile strengths/weaknesses
//! - Recovery status (TSB)
//! - Training goals

use serde::{Deserialize, Serialize};

use super::training_load::{AcwrStatus, DailyLoad, TrainingLoadCalculator};

/// Workout intensity zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntensityZone {
    /// Recovery: < 55% FTP
    Recovery,
    /// Endurance: 55-75% FTP
    Endurance,
    /// Tempo: 75-87% FTP
    Tempo,
    /// Sweet Spot: 88-94% FTP
    SweetSpot,
    /// Threshold: 95-105% FTP
    Threshold,
    /// VO2max: 106-120% FTP
    Vo2max,
    /// Anaerobic: > 120% FTP
    Anaerobic,
}

/// Workout recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutRecommendation {
    /// Recommended intensity zone.
    pub zone: IntensityZone,
    /// Target duration in minutes.
    pub duration_min: u16,
    /// Expected TSS from this workout.
    pub expected_tss: u16,
    /// Recommendation rationale.
    pub rationale: String,
    /// Suggested workout structure.
    pub structure: String,
}

/// Sweet spot recommender based on training status.
pub struct SweetSpotRecommender {
    /// Athlete's FTP.
    ftp: u16,
    /// Training load calculator.
    load_calc: TrainingLoadCalculator,
}

impl SweetSpotRecommender {
    /// Create recommender with athlete's FTP.
    pub fn new(ftp: u16) -> Self {
        Self {
            ftp,
            load_calc: TrainingLoadCalculator::new(),
        }
    }

    /// Get workout recommendation based on current training status.
    pub fn recommend(&self, current_load: &DailyLoad) -> WorkoutRecommendation {
        let acwr = self.load_calc.acwr(current_load.atl, current_load.ctl);

        match acwr.status {
            AcwrStatus::Undertrained => self.build_recommendation(),
            AcwrStatus::Optimal => self.maintain_recommendation(current_load.tsb),
            AcwrStatus::Caution => self.moderate_recommendation(),
            AcwrStatus::HighRisk => self.recovery_recommendation(),
        }
    }

    /// Recommendation when undertrained - build load gradually.
    fn build_recommendation(&self) -> WorkoutRecommendation {
        WorkoutRecommendation {
            zone: IntensityZone::SweetSpot,
            duration_min: 60,
            expected_tss: 70,
            rationale: "Training load is low. Sweet spot work builds fitness efficiently \
                        without excessive fatigue."
                .to_string(),
            structure: format!(
                "Warm-up 10min, 2x20min @ {}W (88-94% FTP) with 5min recovery, Cool-down 10min",
                (self.ftp as f32 * 0.91).round() as u16
            ),
        }
    }

    /// Recommendation when optimal - maintain or slightly increase.
    fn maintain_recommendation(&self, tsb: f32) -> WorkoutRecommendation {
        if tsb > 10.0 {
            // Fresh - can do harder workout
            WorkoutRecommendation {
                zone: IntensityZone::Threshold,
                duration_min: 75,
                expected_tss: 90,
                rationale: "You're fresh with good fitness. Great time for a quality session."
                    .to_string(),
                structure: format!(
                    "Warm-up 15min, 3x10min @ {}W (FTP) with 5min recovery, Cool-down 10min",
                    self.ftp
                ),
            }
        } else if tsb > -10.0 {
            // Neutral - sweet spot
            WorkoutRecommendation {
                zone: IntensityZone::SweetSpot,
                duration_min: 90,
                expected_tss: 85,
                rationale: "Training load is optimal. Continue building with sweet spot work."
                    .to_string(),
                structure: format!(
                    "Warm-up 10min, 3x20min @ {}W (88-94% FTP) with 5min recovery, Cool-down 10min",
                    (self.ftp as f32 * 0.91).round() as u16
                ),
            }
        } else {
            // Slightly fatigued
            WorkoutRecommendation {
                zone: IntensityZone::Tempo,
                duration_min: 90,
                expected_tss: 65,
                rationale:
                    "Slight fatigue detected. Tempo work maintains fitness while recovering."
                        .to_string(),
                structure: format!(
                    "Warm-up 15min, 60min @ {}W (75-87% FTP), Cool-down 15min",
                    (self.ftp as f32 * 0.80).round() as u16
                ),
            }
        }
    }

    /// Recommendation when caution - reduce intensity.
    fn moderate_recommendation(&self) -> WorkoutRecommendation {
        WorkoutRecommendation {
            zone: IntensityZone::Endurance,
            duration_min: 60,
            expected_tss: 45,
            rationale: "Training load is elevated. Easy endurance ride to maintain fitness \
                        while recovering."
                .to_string(),
            structure: format!(
                "Steady riding @ {}W (55-75% FTP) for 60min",
                (self.ftp as f32 * 0.65).round() as u16
            ),
        }
    }

    /// Recommendation when high risk - recovery focus.
    fn recovery_recommendation(&self) -> WorkoutRecommendation {
        WorkoutRecommendation {
            zone: IntensityZone::Recovery,
            duration_min: 45,
            expected_tss: 25,
            rationale: "Training load spike detected. Recovery ride or rest day recommended \
                        to reduce injury risk."
                .to_string(),
            structure: format!(
                "Very easy spinning @ {}W (< 55% FTP) for 30-45min, or complete rest",
                (self.ftp as f32 * 0.50).round() as u16
            ),
        }
    }

    /// Calculate power range for a zone.
    pub fn zone_power_range(&self, zone: IntensityZone) -> (u16, u16) {
        let ftp = self.ftp as f32;
        match zone {
            IntensityZone::Recovery => (0, (ftp * 0.55).round() as u16),
            IntensityZone::Endurance => ((ftp * 0.55).round() as u16, (ftp * 0.75).round() as u16),
            IntensityZone::Tempo => ((ftp * 0.75).round() as u16, (ftp * 0.87).round() as u16),
            IntensityZone::SweetSpot => ((ftp * 0.88).round() as u16, (ftp * 0.94).round() as u16),
            IntensityZone::Threshold => ((ftp * 0.95).round() as u16, (ftp * 1.05).round() as u16),
            IntensityZone::Vo2max => ((ftp * 1.06).round() as u16, (ftp * 1.20).round() as u16),
            IntensityZone::Anaerobic => ((ftp * 1.21).round() as u16, u16::MAX),
        }
    }
}

impl IntensityZone {
    /// Get description of the zone.
    pub fn description(&self) -> &'static str {
        match self {
            IntensityZone::Recovery => "Active recovery to promote blood flow and adaptation",
            IntensityZone::Endurance => "Aerobic base building, fat metabolism, long duration",
            IntensityZone::Tempo => "Muscular endurance, sustained power improvement",
            IntensityZone::SweetSpot => "High training benefit with manageable fatigue",
            IntensityZone::Threshold => "FTP improvement, lactate tolerance",
            IntensityZone::Vo2max => "Aerobic capacity improvement, VO2max development",
            IntensityZone::Anaerobic => "Anaerobic capacity, short maximal efforts",
        }
    }

    /// Get % FTP range as string.
    pub fn ftp_range(&self) -> &'static str {
        match self {
            IntensityZone::Recovery => "< 55%",
            IntensityZone::Endurance => "55-75%",
            IntensityZone::Tempo => "75-87%",
            IntensityZone::SweetSpot => "88-94%",
            IntensityZone::Threshold => "95-105%",
            IntensityZone::Vo2max => "106-120%",
            IntensityZone::Anaerobic => "> 120%",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T115: Unit test for Sweet Spot zone calculation
    #[test]
    fn test_sweet_spot_power_range() {
        let recommender = SweetSpotRecommender::new(250);
        let (low, high) = recommender.zone_power_range(IntensityZone::SweetSpot);

        // 88-94% of 250 = 220-235
        assert_eq!(low, 220);
        assert_eq!(high, 235);
    }

    // T116: Unit test for recommendation based on ACWR status
    #[test]
    fn test_recommendation_undertrained() {
        let recommender = SweetSpotRecommender::new(250);
        let load = DailyLoad {
            tss: 50.0,
            atl: 40.0, // Low ATL
            ctl: 70.0, // Higher CTL = undertrained
            tsb: 30.0,
        };

        let rec = recommender.recommend(&load);
        assert_eq!(rec.zone, IntensityZone::SweetSpot);
        assert!(rec.rationale.contains("low"));
    }

    // T117: Unit test for recovery recommendation
    #[test]
    fn test_recommendation_high_risk() {
        let recommender = SweetSpotRecommender::new(250);
        let load = DailyLoad {
            tss: 150.0,
            atl: 120.0, // High ATL
            ctl: 60.0,  // Lower CTL = high ACWR
            tsb: -60.0,
        };

        let rec = recommender.recommend(&load);
        assert_eq!(rec.zone, IntensityZone::Recovery);
        assert!(rec.rationale.contains("spike") || rec.rationale.contains("Recovery"));
    }

    #[test]
    fn test_zone_power_ranges() {
        let recommender = SweetSpotRecommender::new(200);

        let (r_low, r_high) = recommender.zone_power_range(IntensityZone::Recovery);
        assert_eq!(r_low, 0);
        assert_eq!(r_high, 110); // 55% of 200

        let (e_low, e_high) = recommender.zone_power_range(IntensityZone::Endurance);
        assert_eq!(e_low, 110); // 55% of 200
        assert_eq!(e_high, 150); // 75% of 200

        let (t_low, t_high) = recommender.zone_power_range(IntensityZone::Threshold);
        assert_eq!(t_low, 190); // 95% of 200
        assert_eq!(t_high, 210); // 105% of 200
    }

    #[test]
    fn test_optimal_fresh_recommendation() {
        let recommender = SweetSpotRecommender::new(280);
        let load = DailyLoad {
            tss: 60.0,
            atl: 60.0,
            ctl: 60.0, // ACWR = 1.0 (optimal)
            tsb: 15.0, // Fresh
        };

        let rec = recommender.recommend(&load);
        assert_eq!(rec.zone, IntensityZone::Threshold);
        assert!(rec.rationale.contains("fresh"));
    }
}
