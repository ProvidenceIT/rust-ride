//! FTP prediction from workout history.
//!
//! T022-T030: FTP prediction implementation

use std::sync::Arc;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::{MlError, PredictionSource};
use crate::metrics::analytics::{
    FtpConfidence, FtpDetector, FtpMethod, PdcPoint as AnalyticsPdcPoint, PowerDurationCurve,
};

/// FTP prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpPredictionResult {
    /// Predicted FTP in watts
    pub predicted_ftp: u16,
    /// Confidence level
    pub confidence: FtpConfidence,
    /// Method used for prediction
    pub method_used: FtpMethod,
    /// Efforts that support this prediction
    pub supporting_efforts: Vec<SupportingEffort>,
    /// Whether prediction differs significantly from current FTP
    pub differs_from_current: bool,
    /// Percentage difference from current FTP
    pub difference_percent: f32,
    /// Source of prediction (cloud, cache, local)
    pub source: PredictionSource,
}

/// An effort that supports the FTP prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportingEffort {
    /// Ride ID
    pub ride_id: Uuid,
    /// Duration of the effort in seconds
    pub duration_secs: u32,
    /// Power in watts
    pub power_watts: u16,
    /// Date of the ride
    pub ride_date: NaiveDate,
}

/// FTP predictor with cloud and local fallback.
pub struct FtpPredictor {
    client: Option<Arc<MlClient>>,
    local_detector: FtpDetector,
}

impl FtpPredictor {
    /// Create a new FTP predictor with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
            local_detector: FtpDetector::new(),
        }
    }

    /// Create a new FTP predictor for local-only predictions.
    pub fn local_only() -> Self {
        Self {
            client: None,
            local_detector: FtpDetector::new(),
        }
    }

    /// Predict FTP from ride history.
    ///
    /// Attempts cloud prediction first, falls back to local detection.
    pub async fn predict(
        &self,
        _user_id: Uuid,
        rides: &[RideSummary],
        current_ftp: u16,
    ) -> Result<FtpPredictionResult, MlError> {
        // Check minimum data requirements
        if rides.len() < 5 {
            return Err(MlError::InsufficientData {
                message: format!("Only {} rides available, need at least 5", rides.len()),
                guidance: "Complete more varied-intensity rides to enable FTP prediction.".into(),
            });
        }

        // Try cloud prediction if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
            // For now, fall through to local prediction
        }

        // Fall back to local prediction
        self.predict_local(rides, current_ftp)
    }

    /// Predict FTP using local algorithm only.
    pub fn predict_local(
        &self,
        rides: &[RideSummary],
        current_ftp: u16,
    ) -> Result<FtpPredictionResult, MlError> {
        // Collect PDC points from rides
        let pdc_points: Vec<AnalyticsPdcPoint> = rides
            .iter()
            .flat_map(|r| {
                r.pdc_points.iter().map(|p| AnalyticsPdcPoint {
                    duration_secs: p.duration_secs,
                    power_watts: p.power_watts,
                })
            })
            .collect();

        if pdc_points.is_empty() {
            return Err(MlError::InsufficientData {
                message: "No power duration curve data available".into(),
                guidance: "Record rides with consistent power efforts to build your power curve."
                    .into(),
            });
        }

        // Build PDC and use existing FTP detector
        let pdc = PowerDurationCurve::from_points(pdc_points);
        let estimate =
            self.local_detector
                .detect(&pdc)
                .ok_or_else(|| MlError::InsufficientData {
                    message: "Could not estimate FTP from available data".into(),
                    guidance: "Record longer efforts (20+ minutes) at high intensity.".into(),
                })?;

        let supporting_efforts: Vec<SupportingEffort> = rides
            .iter()
            .filter_map(|r| {
                r.pdc_points
                    .iter()
                    .find(|p| p.duration_secs >= 1200)
                    .map(|p| SupportingEffort {
                        ride_id: r.ride_id,
                        duration_secs: p.duration_secs,
                        power_watts: p.power_watts,
                        ride_date: r.date,
                    })
            })
            .take(5)
            .collect();

        let difference_percent = if current_ftp > 0 {
            ((estimate.ftp_watts as f32 - current_ftp as f32) / current_ftp as f32) * 100.0
        } else {
            0.0
        };

        Ok(FtpPredictionResult {
            predicted_ftp: estimate.ftp_watts,
            confidence: estimate.confidence,
            method_used: estimate.method,
            supporting_efforts,
            differs_from_current: difference_percent.abs() > 3.0,
            difference_percent,
            source: PredictionSource::LocalFallback,
        })
    }

    /// Check if prediction warrants user notification.
    pub fn should_notify(&self, prediction: &FtpPredictionResult, current_ftp: u16) -> bool {
        prediction.differs_from_current
            && prediction.difference_percent.abs() > 3.0
            && prediction.confidence != FtpConfidence::Low
            && current_ftp > 0
    }
}

/// Summary of a ride for FTP prediction.
#[derive(Debug, Clone)]
pub struct RideSummary {
    pub ride_id: Uuid,
    pub date: NaiveDate,
    pub duration_seconds: u32,
    pub avg_power: Option<u16>,
    pub normalized_power: Option<u16>,
    pub max_power: Option<u16>,
    pub tss: Option<f32>,
    pub pdc_points: Vec<PdcPoint>,
}

/// Power duration curve point.
#[derive(Debug, Clone)]
pub struct PdcPoint {
    pub duration_secs: u32,
    pub power_watts: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rides() -> Vec<RideSummary> {
        (0..10)
            .map(|i| RideSummary {
                ride_id: Uuid::new_v4(),
                date: NaiveDate::from_ymd_opt(2025, 1, 1 + i).unwrap(),
                duration_seconds: 3600,
                avg_power: Some(200 + i as u16 * 5),
                normalized_power: Some(210 + i as u16 * 5),
                max_power: Some(450),
                tss: Some(75.0),
                pdc_points: vec![
                    PdcPoint {
                        duration_secs: 5,
                        power_watts: 450,
                    },
                    PdcPoint {
                        duration_secs: 60,
                        power_watts: 350,
                    },
                    PdcPoint {
                        duration_secs: 300,
                        power_watts: 280,
                    },
                    PdcPoint {
                        duration_secs: 1200,
                        power_watts: 260,
                    },
                    PdcPoint {
                        duration_secs: 2700,
                        power_watts: 250,
                    },
                ],
            })
            .collect()
    }

    #[test]
    fn test_local_prediction() {
        let predictor = FtpPredictor::local_only();
        let rides = create_test_rides();

        let result = predictor.predict_local(&rides, 250).unwrap();

        assert!(result.predicted_ftp > 0);
        assert!(!result.supporting_efforts.is_empty());
        assert_eq!(result.source, PredictionSource::LocalFallback);
    }

    #[test]
    fn test_insufficient_data() {
        let predictor = FtpPredictor::local_only();
        let rides: Vec<RideSummary> = vec![]; // Empty rides

        let result = predictor.predict_local(&rides, 250);
        assert!(matches!(result, Err(MlError::InsufficientData { .. })));
    }

    #[test]
    fn test_should_notify() {
        let predictor = FtpPredictor::local_only();

        let result = FtpPredictionResult {
            predicted_ftp: 280,
            confidence: FtpConfidence::High,
            method_used: FtpMethod::ExtendedDuration,
            supporting_efforts: vec![],
            differs_from_current: true,
            difference_percent: 12.0,
            source: PredictionSource::LocalFallback,
        };

        assert!(predictor.should_notify(&result, 250));

        // Low confidence should not notify
        let low_conf = FtpPredictionResult {
            confidence: FtpConfidence::Low,
            ..result.clone()
        };
        assert!(!predictor.should_notify(&low_conf, 250));

        // Small difference should not notify
        let small_diff = FtpPredictionResult {
            differs_from_current: false,
            difference_percent: 2.0,
            ..result
        };
        assert!(!predictor.should_notify(&small_diff, 250));
    }
}
