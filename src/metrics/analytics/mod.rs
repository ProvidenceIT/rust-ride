//! Advanced training analytics module.
//!
//! T002: Create analytics module root with exports
//!
//! This module provides advanced training analytics calculations including:
//! - Power Duration Curve (PDC)
//! - Critical Power / W' Model
//! - FTP Auto-Detection
//! - Training Load (ATL/CTL/ACWR)
//! - VO2max Estimation
//! - Rider Type Classification
//! - Sweet Spot Recommendations

pub mod critical_power;
pub mod error;
pub mod ftp_detection;
pub mod pdc;
pub mod rider_type;
pub mod sweet_spot;
pub mod training_load;
pub mod triggers;
pub mod vo2max;

// Re-exports for convenience
pub use critical_power::{CpFitError, CpFitter, CpModel};
pub use error::{AnalyticsError, AnalyticsResult};
pub use ftp_detection::{FtpConfidence, FtpDetector, FtpEstimate, FtpMethod};
pub use pdc::{MmpCalculator, PdcPoint, PowerDurationCurve};
pub use rider_type::{PowerProfile, RiderClassifier, RiderType};
pub use sweet_spot::{IntensityZone, SweetSpotRecommender, WorkoutRecommendation};
pub use training_load::{Acwr, AcwrStatus, DailyLoad, TrainingLoadCalculator};
pub use triggers::{AnalyticsTriggers, TriggerResult};
pub use vo2max::{FitnessLevel, Vo2maxCalculator, Vo2maxResult};
