//! Metrics module for training calculations and zones.

pub mod analytics;
pub mod calculator;
pub mod smoothing;
pub mod zones;

pub use calculator::MetricsCalculator;
pub use zones::{HRZones, PowerZones};

// Re-export key analytics types for convenience
pub use analytics::{
    Acwr, AcwrStatus, CpFitError, CpFitter, CpModel, DailyLoad, FitnessLevel, FtpConfidence,
    FtpDetector, FtpEstimate, FtpMethod, IntensityZone, MmpCalculator, PdcPoint,
    PowerDurationCurve, PowerProfile, RiderClassifier, RiderType, SweetSpotRecommender,
    TrainingLoadCalculator, Vo2maxCalculator, Vo2maxResult, WorkoutRecommendation,
};
