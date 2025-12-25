//! Rider Type Classification based on power profile.
//!
//! Analyzes Power Duration Curve to classify riders into types:
//! - Sprinter: Excels at short, high-power efforts
//! - Pursuiter: Strong 1-5 minute power
//! - Time Trialist: Excellent sustained power
//! - All-Rounder: Balanced across all durations

use serde::{Deserialize, Serialize};

use super::pdc::PowerDurationCurve;

/// Rider power profile (normalized to FTP).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PowerProfile {
    /// 5-second power as % of FTP (neuromuscular).
    pub neuromuscular: f32,
    /// 1-minute power as % of FTP (anaerobic).
    pub anaerobic: f32,
    /// 5-minute power as % of FTP (VO2max).
    pub vo2max: f32,
    /// FTP as % of FTP (always 100, reference point).
    pub threshold: f32,
}

/// Rider type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiderType {
    /// High neuromuscular power (5s > 180% FTP).
    Sprinter,
    /// Strong anaerobic capacity (1min > 130% FTP).
    Pursuiter,
    /// Excellent sustained power (5min close to FTP).
    TimeTrialist,
    /// Balanced profile across all durations.
    AllRounder,
    /// Insufficient data to classify.
    Unknown,
}

/// Rider classifier.
pub struct RiderClassifier {
    /// FTP for normalization.
    ftp: u16,
}

impl RiderClassifier {
    /// Create classifier with rider's FTP.
    pub fn new(ftp: u16) -> Self {
        Self { ftp }
    }

    /// Calculate power profile from PDC.
    pub fn profile_from_pdc(&self, pdc: &PowerDurationCurve) -> PowerProfile {
        let ftp = self.ftp as f32;
        if ftp <= 0.0 {
            return PowerProfile::default();
        }

        // Get power at key durations
        let p5s = pdc.power_at(5).unwrap_or(0) as f32;
        let p1m = pdc.power_at(60).unwrap_or(0) as f32;
        let p5m = pdc.power_at(300).unwrap_or(0) as f32;

        PowerProfile {
            neuromuscular: (p5s / ftp) * 100.0,
            anaerobic: (p1m / ftp) * 100.0,
            vo2max: (p5m / ftp) * 100.0,
            threshold: 100.0,
        }
    }

    /// Classify rider type from power profile.
    pub fn classify(&self, profile: &PowerProfile) -> RiderType {
        // Check for insufficient data
        if profile.neuromuscular <= 0.0 || profile.anaerobic <= 0.0 || profile.vo2max <= 0.0 {
            return RiderType::Unknown;
        }

        // Sprinter: Very high 5s power (>180% FTP)
        if profile.neuromuscular > 180.0 {
            return RiderType::Sprinter;
        }

        // Pursuiter: High 1-min power (>130% FTP) with good 5s
        if profile.anaerobic > 130.0 && profile.neuromuscular > 150.0 {
            return RiderType::Pursuiter;
        }

        // Time Trialist: 5-min power close to FTP (>85% FTP) with lower sprint
        if profile.vo2max > 85.0 && profile.neuromuscular < 160.0 {
            return RiderType::TimeTrialist;
        }

        // All-Rounder: Balanced profile
        RiderType::AllRounder
    }

    /// Classify directly from PDC.
    pub fn classify_from_pdc(&self, pdc: &PowerDurationCurve) -> RiderType {
        let profile = self.profile_from_pdc(pdc);
        self.classify(&profile)
    }
}

impl RiderType {
    /// Get the display name for this rider type.
    pub fn name(&self) -> &'static str {
        match self {
            RiderType::Sprinter => "Sprinter",
            RiderType::Pursuiter => "Pursuiter",
            RiderType::TimeTrialist => "Time Trialist",
            RiderType::AllRounder => "All-Rounder",
            RiderType::Unknown => "Unknown",
        }
    }

    /// Get a brief description of this rider type.
    pub fn description(&self) -> &'static str {
        match self {
            RiderType::Sprinter => "Explosive power specialist with excellent short burst ability",
            RiderType::Pursuiter => "Strong anaerobic capacity, excels at 1-5 minute efforts",
            RiderType::TimeTrialist => "Outstanding sustained power for long steady efforts",
            RiderType::AllRounder => "Balanced power profile across all durations",
            RiderType::Unknown => "Insufficient data for classification",
        }
    }

    /// Get the suggested training focus for this rider type.
    pub fn training_focus(&self) -> &'static str {
        match self {
            RiderType::Sprinter => "Threshold & VO2max intervals to build sustained power",
            RiderType::Pursuiter => "Sweet spot & FTP work to extend endurance",
            RiderType::TimeTrialist => "Sprint & VO2max sessions for race versatility",
            RiderType::AllRounder => "Target-specific training based on event demands",
            RiderType::Unknown => "Record more varied efforts to build profile",
        }
    }

    /// Get training recommendations for this rider type.
    pub fn training_recommendations(&self) -> &'static str {
        match self {
            RiderType::Sprinter => {
                "Focus on threshold and VO2max work to improve sustained power. \
                 Your sprint is a strength - maintain it with occasional neuromuscular work."
            }
            RiderType::Pursuiter => {
                "Develop your threshold power with sweet spot and FTP intervals. \
                 Your anaerobic capacity is excellent for attacks and short climbs."
            }
            RiderType::TimeTrialist => {
                "Consider adding some sprint and VO2max work for versatility. \
                 Your sustained power is your strength - great for time trials and long climbs."
            }
            RiderType::AllRounder => {
                "Your balanced profile suits many race types. \
                 Focus training on the demands of your target events."
            }
            RiderType::Unknown => {
                "Not enough data to classify rider type. \
                 Complete more rides with varied intensity to build a power profile."
            }
        }
    }

    /// Get typical race types suited for this rider.
    pub fn suited_events(&self) -> &'static str {
        match self {
            RiderType::Sprinter => "Criteriums, flat road races, track sprint events",
            RiderType::Pursuiter => "Track pursuit, short time trials, uphill finishes",
            RiderType::TimeTrialist => "Time trials, long climbs, breakaways",
            RiderType::AllRounder => "Stage races, hilly road races, multi-discipline events",
            RiderType::Unknown => "Complete more rides to determine suited events",
        }
    }
}

impl PowerProfile {
    /// Get the strongest area relative to typical values.
    pub fn strongest_area(&self) -> &'static str {
        let mut best = ("Threshold", 0.0f32);

        // Compare to typical ratios
        let nm_score = self.neuromuscular / 170.0; // Typical 170%
        let an_score = self.anaerobic / 120.0; // Typical 120%
        let vo2_score = self.vo2max / 100.0; // Typical 100%

        if nm_score > best.1 {
            best = ("Neuromuscular (5s)", nm_score);
        }
        if an_score > best.1 {
            best = ("Anaerobic (1min)", an_score);
        }
        if vo2_score > best.1 {
            best = ("VO2max (5min)", vo2_score);
        }

        best.0
    }

    /// Get the weakest area that could be improved.
    pub fn weakest_area(&self) -> &'static str {
        let mut worst = ("Threshold", f32::MAX);

        let nm_score = self.neuromuscular / 170.0;
        let an_score = self.anaerobic / 120.0;
        let vo2_score = self.vo2max / 100.0;

        if nm_score < worst.1 && nm_score > 0.0 {
            worst = ("Neuromuscular (5s)", nm_score);
        }
        if an_score < worst.1 && an_score > 0.0 {
            worst = ("Anaerobic (1min)", an_score);
        }
        if vo2_score < worst.1 && vo2_score > 0.0 {
            worst = ("VO2max (5min)", vo2_score);
        }

        worst.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::analytics::pdc::PdcPoint;

    // T104: Unit test for Sprinter classification
    #[test]
    fn test_sprinter_classification() {
        let classifier = RiderClassifier::new(250);

        // Sprinter: 5s = 500W (200% FTP), 1min = 300W, 5min = 210W
        let profile = PowerProfile {
            neuromuscular: 200.0,
            anaerobic: 120.0,
            vo2max: 84.0,
            threshold: 100.0,
        };

        assert_eq!(classifier.classify(&profile), RiderType::Sprinter);
    }

    // T105: Unit test for Time Trialist classification
    #[test]
    fn test_time_trialist_classification() {
        let classifier = RiderClassifier::new(280);

        // TT: 5s = 420W (150% FTP), 1min = 336W, 5min = 252W (90% FTP)
        let profile = PowerProfile {
            neuromuscular: 150.0,
            anaerobic: 120.0,
            vo2max: 90.0,
            threshold: 100.0,
        };

        assert_eq!(classifier.classify(&profile), RiderType::TimeTrialist);
    }

    // T106: Unit test for All-Rounder classification
    #[test]
    fn test_all_rounder_classification() {
        let classifier = RiderClassifier::new(260);

        // Balanced profile
        let profile = PowerProfile {
            neuromuscular: 170.0,
            anaerobic: 125.0,
            vo2max: 95.0,
            threshold: 100.0,
        };

        assert_eq!(classifier.classify(&profile), RiderType::AllRounder);
    }

    #[test]
    fn test_profile_from_pdc() {
        let classifier = RiderClassifier::new(200);

        let points = vec![
            PdcPoint {
                duration_secs: 5,
                power_watts: 400,
            }, // 200%
            PdcPoint {
                duration_secs: 60,
                power_watts: 260,
            }, // 130%
            PdcPoint {
                duration_secs: 300,
                power_watts: 180,
            }, // 90%
        ];
        let pdc = PowerDurationCurve::from_points(points);
        let profile = classifier.profile_from_pdc(&pdc);

        assert!((profile.neuromuscular - 200.0).abs() < 0.1);
        assert!((profile.anaerobic - 130.0).abs() < 0.1);
        assert!((profile.vo2max - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_unknown_with_missing_data() {
        let classifier = RiderClassifier::new(200);

        let profile = PowerProfile {
            neuromuscular: 0.0,
            anaerobic: 120.0,
            vo2max: 90.0,
            threshold: 100.0,
        };

        assert_eq!(classifier.classify(&profile), RiderType::Unknown);
    }
}
