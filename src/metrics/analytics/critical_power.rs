//! Critical Power / W' Model calculation.
//!
//! The CP model provides:
//! - Critical Power (CP): The highest power sustainable indefinitely
//! - W' (W-prime): Anaerobic work capacity in joules
//! - Time-to-exhaustion predictions at any power above CP

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::pdc::PowerDurationCurve;

/// Critical Power model parameters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpModel {
    /// Critical Power in watts.
    pub cp: u16,
    /// W' (anaerobic capacity) in joules.
    pub w_prime: u32,
    /// Model fit quality (R² value, 0-1).
    pub r_squared: f32,
}

/// Model fitting errors.
#[derive(Debug, Error)]
pub enum CpFitError {
    /// Insufficient data points for fitting.
    #[error("Insufficient data points (need at least 3, got {0})")]
    InsufficientData(usize),

    /// Invalid duration range in data.
    #[error("Invalid duration range (need 2-20 minute efforts)")]
    InvalidDurationRange,

    /// Model fitting failed.
    #[error("Model fitting failed: {0}")]
    FittingFailed(String),
}

/// CP model fitter.
pub struct CpFitter {
    /// Minimum duration for fitting (default: 120s / 2 min).
    min_duration: u32,
    /// Maximum duration for fitting (default: 1200s / 20 min).
    max_duration: u32,
}

impl CpFitter {
    /// Create with default settings (2-20 min range).
    pub fn new() -> Self {
        Self {
            min_duration: 120,
            max_duration: 1200,
        }
    }

    /// Create with custom duration range.
    pub fn with_range(min_secs: u32, max_secs: u32) -> Self {
        Self {
            min_duration: min_secs,
            max_duration: max_secs,
        }
    }

    /// Fit CP model from PDC points.
    pub fn fit(&self, pdc: &PowerDurationCurve) -> Result<CpModel, CpFitError> {
        let points: Vec<(u32, u16)> = pdc
            .points()
            .iter()
            .filter(|p| {
                p.duration_secs >= self.min_duration && p.duration_secs <= self.max_duration
            })
            .map(|p| (p.duration_secs, p.power_watts))
            .collect();

        self.fit_points(&points)
    }

    /// Fit from explicit (duration, power) pairs.
    pub fn fit_points(&self, points: &[(u32, u16)]) -> Result<CpModel, CpFitError> {
        if points.len() < 3 {
            return Err(CpFitError::InsufficientData(points.len()));
        }

        // Validate duration range
        let has_valid_range = points
            .iter()
            .any(|(d, _)| *d >= self.min_duration && *d <= self.max_duration);
        if !has_valid_range {
            return Err(CpFitError::InvalidDurationRange);
        }

        // Transform to linear form: Work = CP × time + W'
        // where work = power × duration
        let work_time_pairs: Vec<(f64, f64)> = points
            .iter()
            .map(|(d, p)| (*d as f64, *p as f64 * *d as f64))
            .collect();

        // Linear regression: work = slope * time + intercept
        // slope = CP, intercept = W'
        let (slope, intercept, r_squared) = linear_regression(&work_time_pairs)?;

        // Validate results
        if slope <= 0.0 || intercept <= 0.0 {
            return Err(CpFitError::FittingFailed(
                "Invalid CP/W' values (must be positive)".to_string(),
            ));
        }

        Ok(CpModel {
            cp: slope.round() as u16,
            w_prime: intercept.round() as u32,
            r_squared: r_squared as f32,
        })
    }
}

impl Default for CpFitter {
    fn default() -> Self {
        Self::new()
    }
}

impl CpModel {
    /// Predict time to exhaustion at given power.
    /// Returns None if power <= CP (theoretically infinite).
    pub fn time_to_exhaustion(&self, power_watts: u16) -> Option<Duration> {
        if power_watts <= self.cp {
            return None; // Infinite (sustainable)
        }

        let tte_secs = self.w_prime as f64 / (power_watts as f64 - self.cp as f64);
        Some(Duration::from_secs_f64(tte_secs))
    }

    /// Predict sustainable power for given duration.
    pub fn power_at_duration(&self, duration: Duration) -> u16 {
        let secs = duration.as_secs_f64();
        if secs <= 0.0 {
            return 0;
        }

        let power = self.cp as f64 + self.w_prime as f64 / secs;
        power.round() as u16
    }

    /// Calculate remaining W' after work at given power/duration.
    /// Returns negative if W' would be depleted.
    pub fn w_prime_remaining(&self, power_watts: u16, duration: Duration) -> i32 {
        if power_watts <= self.cp {
            return self.w_prime as i32; // No W' depletion below CP
        }

        let work_above_cp = (power_watts as f64 - self.cp as f64) * duration.as_secs_f64();
        self.w_prime as i32 - work_above_cp.round() as i32
    }
}

/// Linear regression on (x, y) pairs.
/// Returns (slope, intercept, r_squared).
fn linear_regression(points: &[(f64, f64)]) -> Result<(f64, f64, f64), CpFitError> {
    let n = points.len() as f64;
    if n < 2.0 {
        return Err(CpFitError::FittingFailed(
            "Need at least 2 points for regression".to_string(),
        ));
    }

    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();
    let _sum_yy: f64 = points.iter().map(|(_, y)| y * y).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-10 {
        return Err(CpFitError::FittingFailed(
            "Singular matrix in regression".to_string(),
        ));
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;

    // Calculate R²
    let mean_y = sum_y / n;
    let ss_tot: f64 = points.iter().map(|(_, y)| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = points
        .iter()
        .map(|(x, y)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        })
        .sum();

    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    Ok((slope, intercept, r_squared))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::analytics::pdc::PdcPoint;

    // T046: Unit test for CP model fitting with known reference values
    #[test]
    fn test_cp_model_fitting() {
        // Known values: CP = 250W, W' = 20000J
        // Work = 250 * t + 20000
        // At 3 min (180s): P = 250 + 20000/180 = 361W
        // At 12 min (720s): P = 250 + 20000/720 = 278W
        // At 20 min (1200s): P = 250 + 20000/1200 = 267W

        let points = vec![
            PdcPoint {
                duration_secs: 180,
                power_watts: 361,
            },
            PdcPoint {
                duration_secs: 720,
                power_watts: 278,
            },
            PdcPoint {
                duration_secs: 1200,
                power_watts: 267,
            },
        ];

        let pdc = PowerDurationCurve::from_points(points);
        let fitter = CpFitter::new();
        let model = fitter.fit(&pdc).expect("Fitting should succeed");

        // Allow some tolerance
        assert!(
            (model.cp as i32 - 250).abs() <= 5,
            "CP should be ~250W, got {}",
            model.cp
        );
        assert!(
            (model.w_prime as i32 - 20000).abs() <= 1000,
            "W' should be ~20000J, got {}",
            model.w_prime
        );
        assert!(model.r_squared > 0.95, "R² should be high for perfect data");
    }

    // T047: Unit test for time_to_exhaustion calculation
    #[test]
    fn test_time_to_exhaustion() {
        let model = CpModel {
            cp: 250,
            w_prime: 20000,
            r_squared: 0.98,
        };

        // At CP, TTE is infinite
        assert!(model.time_to_exhaustion(250).is_none());
        assert!(model.time_to_exhaustion(200).is_none());

        // Above CP, TTE is finite
        // TTE at 300W = 20000 / (300 - 250) = 400s
        let tte = model.time_to_exhaustion(300).unwrap();
        assert_eq!(tte.as_secs(), 400);

        // TTE at 350W = 20000 / (350 - 250) = 200s
        let tte = model.time_to_exhaustion(350).unwrap();
        assert_eq!(tte.as_secs(), 200);
    }

    // T048: Unit test for power_at_duration calculation
    #[test]
    fn test_power_at_duration() {
        let model = CpModel {
            cp: 250,
            w_prime: 20000,
            r_squared: 0.98,
        };

        // P at 400s = 250 + 20000/400 = 300W
        let power = model.power_at_duration(Duration::from_secs(400));
        assert_eq!(power, 300);

        // P at 200s = 250 + 20000/200 = 350W
        let power = model.power_at_duration(Duration::from_secs(200));
        assert_eq!(power, 350);
    }

    // T049: Unit test for CpFitError on insufficient data
    #[test]
    fn test_cp_insufficient_data() {
        let points = vec![PdcPoint {
            duration_secs: 180,
            power_watts: 350,
        }];
        let pdc = PowerDurationCurve::from_points(points);
        let fitter = CpFitter::new();

        let result = fitter.fit(&pdc);
        assert!(matches!(result, Err(CpFitError::InsufficientData(1))));
    }

    #[test]
    fn test_w_prime_remaining() {
        let model = CpModel {
            cp: 250,
            w_prime: 20000,
            r_squared: 0.98,
        };

        // Below CP: no depletion
        let remaining = model.w_prime_remaining(200, Duration::from_secs(300));
        assert_eq!(remaining, 20000);

        // At 300W for 200s: depletes (300-250) * 200 = 10000J
        let remaining = model.w_prime_remaining(300, Duration::from_secs(200));
        assert_eq!(remaining, 10000);

        // At 300W for 500s: depletes 25000J (negative remaining)
        let remaining = model.w_prime_remaining(300, Duration::from_secs(500));
        assert!(remaining < 0);
    }
}
