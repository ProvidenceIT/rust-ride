//! VO2max Estimation from power data.
//!
//! Estimates VO2max without physiological testing using:
//! - Hawley-Noakes formula based on power:weight ratio
//! - Correlation with Critical Power and FTP

use serde::{Deserialize, Serialize};

/// VO2max estimation result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vo2maxResult {
    /// Estimated VO2max in ml/kg/min.
    pub vo2max: f32,
    /// Fitness classification.
    pub classification: FitnessLevel,
    /// Estimation method used.
    pub method: Vo2maxMethod,
}

/// Fitness classification based on VO2max.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitnessLevel {
    /// VO2max < 35: Untrained.
    Untrained,
    /// VO2max 35-45: Recreational.
    Recreational,
    /// VO2max 45-55: Trained.
    Trained,
    /// VO2max 55-65: Well-trained.
    WellTrained,
    /// VO2max 65-75: Elite.
    Elite,
    /// VO2max > 75: World-class.
    WorldClass,
}

/// Method used for VO2max estimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vo2maxMethod {
    /// Based on 5-minute max power (Hawley-Noakes).
    FiveMinutePower,
    /// Based on FTP and body weight.
    FtpBased,
    /// Based on Critical Power.
    CriticalPowerBased,
}

/// VO2max calculator.
pub struct Vo2maxCalculator {
    /// Athlete body weight in kg.
    weight_kg: f32,
}

impl Vo2maxCalculator {
    /// Create a new calculator with athlete weight.
    pub fn new(weight_kg: f32) -> Self {
        Self { weight_kg }
    }

    /// Estimate VO2max from 5-minute max power using Hawley-Noakes formula.
    /// Formula: VO2max = (10.8 × P5min / weight) + 7
    pub fn from_five_minute_power(&self, power_5min: u16) -> Vo2maxResult {
        let power_per_kg = power_5min as f32 / self.weight_kg;
        let vo2max = 10.8 * power_per_kg + 7.0;

        Vo2maxResult {
            vo2max,
            classification: Self::classify(vo2max),
            method: Vo2maxMethod::FiveMinutePower,
        }
    }

    /// Estimate VO2max from FTP.
    /// Uses empirical correlation: VO2max ≈ FTP/weight × 12 + 3.5
    pub fn from_ftp(&self, ftp: u16) -> Vo2maxResult {
        let ftp_per_kg = ftp as f32 / self.weight_kg;
        let vo2max = ftp_per_kg * 12.0 + 3.5;

        Vo2maxResult {
            vo2max,
            classification: Self::classify(vo2max),
            method: Vo2maxMethod::FtpBased,
        }
    }

    /// Estimate VO2max from Critical Power.
    /// CP correlates closely with VO2max: VO2max ≈ CP/weight × 12.5 + 2
    pub fn from_critical_power(&self, cp: u16) -> Vo2maxResult {
        let cp_per_kg = cp as f32 / self.weight_kg;
        let vo2max = cp_per_kg * 12.5 + 2.0;

        Vo2maxResult {
            vo2max,
            classification: Self::classify(vo2max),
            method: Vo2maxMethod::CriticalPowerBased,
        }
    }

    /// Classify fitness level based on VO2max.
    fn classify(vo2max: f32) -> FitnessLevel {
        if vo2max < 35.0 {
            FitnessLevel::Untrained
        } else if vo2max < 45.0 {
            FitnessLevel::Recreational
        } else if vo2max < 55.0 {
            FitnessLevel::Trained
        } else if vo2max < 65.0 {
            FitnessLevel::WellTrained
        } else if vo2max < 75.0 {
            FitnessLevel::Elite
        } else {
            FitnessLevel::WorldClass
        }
    }
}

impl FitnessLevel {
    /// Get descriptive text for the fitness level.
    pub fn description(&self) -> &'static str {
        match self {
            FitnessLevel::Untrained => "Untrained - Consider starting with easy endurance rides",
            FitnessLevel::Recreational => "Recreational - Good base fitness for cycling",
            FitnessLevel::Trained => "Trained - Solid aerobic capacity",
            FitnessLevel::WellTrained => "Well-trained - Competitive amateur level",
            FitnessLevel::Elite => "Elite - Professional or high-level amateur",
            FitnessLevel::WorldClass => "World-class - Top-tier athletic capacity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T094: Unit test for Hawley-Noakes VO2max formula
    #[test]
    fn test_hawley_noakes_formula() {
        let calc = Vo2maxCalculator::new(70.0);

        // 5-min power of 350W for 70kg rider
        // VO2max = 10.8 × (350/70) + 7 = 10.8 × 5 + 7 = 61
        let result = calc.from_five_minute_power(350);
        assert!((result.vo2max - 61.0).abs() < 0.5);
        assert_eq!(result.classification, FitnessLevel::WellTrained);
    }

    // T095: Unit test for FTP-based VO2max estimation
    #[test]
    fn test_ftp_based_estimation() {
        let calc = Vo2maxCalculator::new(70.0);

        // FTP of 280W for 70kg rider = 4.0 W/kg
        // VO2max ≈ 4.0 × 12 + 3.5 = 51.5
        let result = calc.from_ftp(280);
        assert!((result.vo2max - 51.5).abs() < 0.5);
        assert_eq!(result.classification, FitnessLevel::Trained);
    }

    // T096: Unit test for fitness level classification
    #[test]
    fn test_fitness_classification() {
        assert_eq!(Vo2maxCalculator::classify(30.0), FitnessLevel::Untrained);
        assert_eq!(Vo2maxCalculator::classify(40.0), FitnessLevel::Recreational);
        assert_eq!(Vo2maxCalculator::classify(50.0), FitnessLevel::Trained);
        assert_eq!(Vo2maxCalculator::classify(60.0), FitnessLevel::WellTrained);
        assert_eq!(Vo2maxCalculator::classify(70.0), FitnessLevel::Elite);
        assert_eq!(Vo2maxCalculator::classify(80.0), FitnessLevel::WorldClass);
    }

    #[test]
    fn test_cp_based_estimation() {
        let calc = Vo2maxCalculator::new(70.0);

        // CP of 260W for 70kg rider = 3.71 W/kg
        // VO2max ≈ 3.71 × 12.5 + 2 = 48.4
        let result = calc.from_critical_power(260);
        assert!((result.vo2max - 48.4).abs() < 0.5);
        assert_eq!(result.method, Vo2maxMethod::CriticalPowerBased);
    }
}
