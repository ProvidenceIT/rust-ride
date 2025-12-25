//! FTP Auto-Detection.
//!
//! Automatically estimates Functional Threshold Power from ride history
//! without requiring dedicated FTP tests.

use serde::{Deserialize, Serialize};

use super::critical_power::CpModel;
use super::pdc::PowerDurationCurve;

/// FTP detection confidence level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FtpConfidence {
    /// 3+ recent quality efforts.
    High,
    /// 2+ efforts.
    Medium,
    /// Limited data.
    Low,
}

/// Method used to detect FTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FtpMethod {
    /// 95% of 20-minute power.
    TwentyMinute,
    /// Average of extended duration efforts (45-60 min).
    ExtendedDuration,
    /// Derived from CP model.
    CriticalPower,
}

/// FTP estimate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpEstimate {
    /// Estimated FTP in watts.
    pub ftp_watts: u16,
    /// Detection method used.
    pub method: FtpMethod,
    /// Confidence level.
    pub confidence: FtpConfidence,
    /// Supporting data: (duration_secs, power_watts) pairs used.
    pub supporting_data: Vec<(u32, u16)>,
}

/// FTP detector.
pub struct FtpDetector {
    /// Minimum weeks of data required.
    #[allow(dead_code)]
    min_weeks: u8,
    /// How recent data must be (in days).
    #[allow(dead_code)]
    recency_days: u32,
    /// Significant change threshold (fraction).
    significant_change_threshold: f32,
}

impl FtpDetector {
    /// Create with default settings.
    pub fn new() -> Self {
        Self {
            min_weeks: 2,
            recency_days: 42,
            significant_change_threshold: 0.05,
        }
    }

    /// Detect FTP from PDC.
    pub fn detect(&self, pdc: &PowerDurationCurve) -> Option<FtpEstimate> {
        // Try extended duration first (more accurate)
        if let Some(estimate) = self.detect_extended_duration(pdc) {
            return Some(estimate);
        }

        // Fall back to 20-minute power
        self.detect_twenty_minute(pdc)
    }

    /// Detect using 20-minute power (95% rule).
    fn detect_twenty_minute(&self, pdc: &PowerDurationCurve) -> Option<FtpEstimate> {
        let power_20min = pdc.power_at(1200)?; // 20 minutes = 1200 seconds

        let ftp = (power_20min as f32 * 0.95).round() as u16;

        // Determine confidence based on actual data points (not extrapolated)
        // Need actual 5-min and 10-min data to have Medium confidence
        let tolerance = 60; // 1 minute tolerance for matching durations
        let has_5min = pdc.has_data_near(300, tolerance);
        let has_10min = pdc.has_data_near(600, tolerance);
        let confidence = if has_5min && has_10min {
            FtpConfidence::Medium
        } else {
            FtpConfidence::Low
        };

        Some(FtpEstimate {
            ftp_watts: ftp,
            method: FtpMethod::TwentyMinute,
            confidence,
            supporting_data: vec![(1200, power_20min)],
        })
    }

    /// Detect using extended duration efforts (45-60 min).
    /// Only uses actual data points, not extrapolated values.
    fn detect_extended_duration(&self, pdc: &PowerDurationCurve) -> Option<FtpEstimate> {
        // Use tolerance of 300s (5 min) to find actual extended efforts
        // This prevents extrapolation from shorter efforts being used
        let tolerance = 300;
        let power_45min = pdc.power_at_actual(2700, tolerance); // 45 minutes
        let power_60min = pdc.power_at_actual(3600, tolerance); // 60 minutes

        match (power_45min, power_60min) {
            (Some(p45), Some(p60)) => {
                let ftp = (p45 + p60) / 2;
                Some(FtpEstimate {
                    ftp_watts: ftp,
                    method: FtpMethod::ExtendedDuration,
                    confidence: FtpConfidence::High,
                    supporting_data: vec![(2700, p45), (3600, p60)],
                })
            }
            (Some(p45), None) => Some(FtpEstimate {
                ftp_watts: p45,
                method: FtpMethod::ExtendedDuration,
                confidence: FtpConfidence::Medium,
                supporting_data: vec![(2700, p45)],
            }),
            _ => None,
        }
    }

    /// Detect using CP model (more accurate if available).
    pub fn detect_from_cp(&self, cp_model: &CpModel) -> FtpEstimate {
        // FTP ≈ CP for well-trained athletes
        // Some add a small offset (e.g., CP + 5%)
        let ftp = cp_model.cp;

        FtpEstimate {
            ftp_watts: ftp,
            method: FtpMethod::CriticalPower,
            confidence: if cp_model.r_squared > 0.95 {
                FtpConfidence::High
            } else if cp_model.r_squared > 0.85 {
                FtpConfidence::Medium
            } else {
                FtpConfidence::Low
            },
            supporting_data: vec![],
        }
    }

    /// Check if FTP estimate differs significantly from current.
    pub fn is_significant_change(&self, current_ftp: u16, new_estimate: &FtpEstimate) -> bool {
        self.change_percent(current_ftp, new_estimate.ftp_watts)
            .abs()
            > self.significant_change_threshold
    }

    /// Get change percentage.
    pub fn change_percent(&self, current: u16, new: u16) -> f32 {
        if current == 0 {
            return 1.0;
        }
        (new as f32 - current as f32) / current as f32
    }
}

impl Default for FtpDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FtpEstimate {
    /// Check if this should trigger user notification.
    pub fn should_notify(&self, current_ftp: u16) -> bool {
        let detector = FtpDetector::new();
        detector.is_significant_change(current_ftp, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::analytics::pdc::PdcPoint;

    // T061: Unit test for FTP detection from 20-min power (95% rule)
    #[test]
    fn test_ftp_twenty_minute() {
        let points = vec![PdcPoint {
            duration_secs: 1200,
            power_watts: 280,
        }];
        let pdc = PowerDurationCurve::from_points(points);
        let detector = FtpDetector::new();

        let estimate = detector.detect(&pdc).unwrap();
        assert_eq!(estimate.method, FtpMethod::TwentyMinute);
        assert_eq!(estimate.ftp_watts, 266); // 280 * 0.95 = 266
    }

    // T062: Unit test for FTP detection from extended duration
    #[test]
    fn test_ftp_extended_duration() {
        let points = vec![
            PdcPoint {
                duration_secs: 2700,
                power_watts: 250,
            },
            PdcPoint {
                duration_secs: 3600,
                power_watts: 240,
            },
        ];
        let pdc = PowerDurationCurve::from_points(points);
        let detector = FtpDetector::new();

        let estimate = detector.detect(&pdc).unwrap();
        assert_eq!(estimate.method, FtpMethod::ExtendedDuration);
        assert_eq!(estimate.ftp_watts, 245); // (250 + 240) / 2
        assert_eq!(estimate.confidence, FtpConfidence::High);
    }

    // T063: Unit test for confidence level calculation
    #[test]
    fn test_confidence_levels() {
        // Only 20-min data -> Low confidence
        let points1 = vec![PdcPoint {
            duration_secs: 1200,
            power_watts: 280,
        }];
        let pdc1 = PowerDurationCurve::from_points(points1);
        let detector = FtpDetector::new();
        let estimate1 = detector.detect(&pdc1).unwrap();
        assert_eq!(estimate1.confidence, FtpConfidence::Low);

        // 20-min + 5-min + 10-min -> Medium confidence
        let points2 = vec![
            PdcPoint {
                duration_secs: 300,
                power_watts: 350,
            },
            PdcPoint {
                duration_secs: 600,
                power_watts: 320,
            },
            PdcPoint {
                duration_secs: 1200,
                power_watts: 280,
            },
        ];
        let pdc2 = PowerDurationCurve::from_points(points2);
        let estimate2 = detector.detect(&pdc2).unwrap();
        assert_eq!(estimate2.confidence, FtpConfidence::Medium);
    }

    // T064: Unit test for significant change detection (>5%)
    #[test]
    fn test_significant_change() {
        let detector = FtpDetector::new();

        let estimate = FtpEstimate {
            ftp_watts: 260,
            method: FtpMethod::TwentyMinute,
            confidence: FtpConfidence::Medium,
            supporting_data: vec![],
        };

        // 5% change should be significant
        assert!(detector.is_significant_change(245, &estimate)); // 245 -> 260 = 6.1%

        // Small change should not be significant
        let small_estimate = FtpEstimate {
            ftp_watts: 252,
            method: FtpMethod::TwentyMinute,
            confidence: FtpConfidence::Medium,
            supporting_data: vec![],
        };
        assert!(!detector.is_significant_change(250, &small_estimate)); // 250 -> 252 = 0.8%
    }
}
