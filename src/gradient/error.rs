//! Gradient error types.

use std::fmt;
use uuid::Uuid;

/// Errors from gradient operations.
#[derive(Debug, Clone)]
pub enum GradientError {
    /// Route not found
    RouteNotFound(Uuid),
    /// Route has no elevation data
    NoElevationData,
    /// Position is beyond route length
    PositionOutOfBounds { position: f64, max: f64 },
    /// Route not loaded
    NoRouteLoaded,
    /// Invalid GPX file
    InvalidGpx(String),
    /// Route has insufficient points
    InsufficientPoints { found: usize, minimum: usize },
    /// FTMS communication error
    FtmsError(String),
}

impl fmt::Display for GradientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouteNotFound(id) => write!(f, "Route not found: {}", id),
            Self::NoElevationData => write!(f, "Route has no elevation data"),
            Self::PositionOutOfBounds { position, max } => {
                write!(f, "Position {} is beyond route end ({}m)", position, max)
            }
            Self::NoRouteLoaded => write!(f, "No route is currently loaded"),
            Self::InvalidGpx(msg) => write!(f, "Invalid GPX file: {}", msg),
            Self::InsufficientPoints { found, minimum } => {
                write!(f, "Route has {} points, minimum {} required", found, minimum)
            }
            Self::FtmsError(msg) => write!(f, "FTMS error: {}", msg),
        }
    }
}

impl std::error::Error for GradientError {}

impl From<std::io::Error> for GradientError {
    fn from(err: std::io::Error) -> Self {
        Self::InvalidGpx(err.to_string())
    }
}
