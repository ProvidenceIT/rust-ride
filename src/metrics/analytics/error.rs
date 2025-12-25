//! Analytics error types.
//!
//! T004: Create AnalyticsError types with thiserror

use thiserror::Error;

/// Errors that can occur during analytics calculations.
#[derive(Debug, Error)]
pub enum AnalyticsError {
    /// Insufficient data to perform calculation.
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Calculation failed.
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(#[from] rusqlite::Error),
}

/// Result type for analytics operations.
pub type AnalyticsResult<T> = Result<T, AnalyticsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_data_error() {
        let err = AnalyticsError::InsufficientData("Need 3 max efforts".to_string());
        assert!(err.to_string().contains("Need 3 max efforts"));
    }

    #[test]
    fn test_invalid_input_error() {
        let err = AnalyticsError::InvalidInput("Power cannot be negative".to_string());
        assert!(err.to_string().contains("Power cannot be negative"));
    }
}
