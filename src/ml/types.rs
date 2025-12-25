//! Shared ML types and error definitions.
//!
//! T015: Create shared types for ML module

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for ML operations.
#[derive(Debug, Error)]
pub enum MlError {
    /// Cloud API error
    #[error("Cloud API error: {0}")]
    ApiError(String),

    /// Network unavailable
    #[error("Network unavailable - using cached predictions")]
    Offline,

    /// Insufficient data for prediction
    #[error("Insufficient data: {message}. {guidance}")]
    InsufficientData {
        /// What data is missing
        message: String,
        /// How to get more data
        guidance: String,
    },

    /// Rate limited by cloud API
    #[error("Rate limited - try again later")]
    RateLimited,

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl From<serde_json::Error> for MlError {
    fn from(err: serde_json::Error) -> Self {
        MlError::SerializationError(err.to_string())
    }
}

impl From<rusqlite::Error> for MlError {
    fn from(err: rusqlite::Error) -> Self {
        MlError::DatabaseError(err.to_string())
    }
}

/// Type of ML prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionType {
    /// FTP prediction from workout history
    FtpPrediction,
    /// Real-time fatigue state
    FatigueState,
    /// CTL/fitness performance forecast
    PerformanceForecast,
    /// Personalized workout difficulty
    DifficultyEstimate,
    /// Cadence optimization analysis
    CadenceAnalysis,
    /// Personalized adaptation model
    AdaptationModel,
}

impl PredictionType {
    /// Get the cache expiry duration for this prediction type.
    pub fn cache_expiry_hours(&self) -> u64 {
        match self {
            PredictionType::FtpPrediction => 24 * 7, // 7 days
            PredictionType::FatigueState => 24,       // 24 hours
            PredictionType::PerformanceForecast => 24, // 24 hours
            PredictionType::DifficultyEstimate => 1,  // 1 hour
            PredictionType::CadenceAnalysis => 24 * 7, // 7 days
            PredictionType::AdaptationModel => 24 * 7, // 7 days
        }
    }

    /// Get display name for this prediction type.
    pub fn display_name(&self) -> &'static str {
        match self {
            PredictionType::FtpPrediction => "FTP Prediction",
            PredictionType::FatigueState => "Fatigue State",
            PredictionType::PerformanceForecast => "Performance Forecast",
            PredictionType::DifficultyEstimate => "Difficulty Estimate",
            PredictionType::CadenceAnalysis => "Cadence Analysis",
            PredictionType::AdaptationModel => "Adaptation Model",
        }
    }
}

impl std::fmt::Display for PredictionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Source of ML prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionSource {
    /// Prediction from cloud ML service
    Cloud,
    /// Prediction retrieved from local cache
    Cached,
    /// Local fallback calculation (offline mode)
    LocalFallback,
}

impl PredictionSource {
    /// Whether this prediction may be stale.
    pub fn may_be_stale(&self) -> bool {
        matches!(self, PredictionSource::Cached | PredictionSource::LocalFallback)
    }

    /// Get display description.
    pub fn description(&self) -> &'static str {
        match self {
            PredictionSource::Cloud => "Live prediction",
            PredictionSource::Cached => "Cached prediction",
            PredictionSource::LocalFallback => "Offline calculation",
        }
    }
}

impl std::fmt::Display for PredictionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Confidence level for predictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confidence {
    /// Not enough data for confident prediction
    Insufficient,
    /// Low confidence (limited data)
    Low,
    /// Medium confidence (adequate data)
    Medium,
    /// High confidence (rich data)
    High,
}

impl Confidence {
    /// Get numeric representation (0.0 - 1.0).
    pub fn as_f32(&self) -> f32 {
        match self {
            Confidence::Insufficient => 0.0,
            Confidence::Low => 0.3,
            Confidence::Medium => 0.6,
            Confidence::High => 0.9,
        }
    }

    /// Create from numeric confidence.
    pub fn from_f32(value: f32) -> Self {
        if value < 0.2 {
            Confidence::Insufficient
        } else if value < 0.5 {
            Confidence::Low
        } else if value < 0.75 {
            Confidence::Medium
        } else {
            Confidence::High
        }
    }

    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Confidence::Insufficient => "Insufficient Data",
            Confidence::Low => "Low",
            Confidence::Medium => "Medium",
            Confidence::High => "High",
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_type_cache_expiry() {
        assert_eq!(PredictionType::FtpPrediction.cache_expiry_hours(), 168);
        assert_eq!(PredictionType::DifficultyEstimate.cache_expiry_hours(), 1);
    }

    #[test]
    fn test_confidence_ordering() {
        assert!(Confidence::High > Confidence::Medium);
        assert!(Confidence::Medium > Confidence::Low);
        assert!(Confidence::Low > Confidence::Insufficient);
    }

    #[test]
    fn test_confidence_from_f32() {
        assert_eq!(Confidence::from_f32(0.1), Confidence::Insufficient);
        assert_eq!(Confidence::from_f32(0.4), Confidence::Low);
        assert_eq!(Confidence::from_f32(0.6), Confidence::Medium);
        assert_eq!(Confidence::from_f32(0.9), Confidence::High);
    }

    #[test]
    fn test_prediction_source_staleness() {
        assert!(!PredictionSource::Cloud.may_be_stale());
        assert!(PredictionSource::Cached.may_be_stale());
        assert!(PredictionSource::LocalFallback.may_be_stale());
    }
}
