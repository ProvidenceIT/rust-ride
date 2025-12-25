//! Machine Learning coaching module.
//!
//! Provides AI/ML-powered coaching features including:
//! - FTP prediction from workout history
//! - Real-time fatigue detection during rides
//! - Adaptive workout recommendations
//! - Performance trend forecasting
//! - Workout difficulty estimation
//! - Cadence and technique analysis
//! - Training load adaptation

pub mod adaptation;
pub mod cache;
pub mod cadence_analysis;
pub mod client;
pub mod difficulty;
pub mod fatigue_detection;
pub mod ftp_prediction;
pub mod performance_forecast;
pub mod types;
pub mod workout_recommend;

// Re-exports for convenience
pub use adaptation::{AdaptationEngine, AdaptationModel, LoadRecommendation, ModelConfidence};
pub use cache::{CachedPrediction, MlCache};
pub use cadence_analysis::{CadenceAnalysis, CadenceAnalyzer, CadenceEfficiency, DegradationPattern};
pub use client::MlClient;
pub use difficulty::{DifficultyEstimate, DifficultyEstimator, DifficultyFactors};
pub use fatigue_detection::{
    AthleteBaseline, FatigueAnalysis, FatigueDetector, FatigueSeverity, FatigueState,
};
pub use ftp_prediction::{FtpPredictionResult, FtpPredictor, SupportingEffort};
pub use performance_forecast::{
    DetrainingRisk, EventReadiness, PerformanceForecaster, PerformanceProjection, ProjectedCtl,
};
pub use types::{MlError, PredictionSource, PredictionType};
pub use workout_recommend::{
    EnergySystem, RecommendationStatus, WorkoutRecommendation, WorkoutRecommender, WorkoutSource,
};
