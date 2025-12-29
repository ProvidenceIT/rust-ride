//! Gradient Controller Contract
//!
//! Defines the interface for gradient-responsive resistance control.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Settings for gradient simulation behavior.
#[derive(Debug, Clone)]
pub struct GradientSettings {
    /// Trainer difficulty multiplier (0.0-1.0)
    pub difficulty: f32,
    /// Maximum positive gradient (percent)
    pub max_gradient: f32,
    /// Maximum negative gradient (percent)
    pub min_gradient: f32,
    /// Smoothing window (seconds)
    pub smoothing_secs: u8,
    /// Rolling resistance coefficient
    pub rolling_resistance: f32,
}

impl Default for GradientSettings {
    fn default() -> Self {
        Self {
            difficulty: 1.0,
            max_gradient: 15.0,
            min_gradient: -15.0,
            smoothing_secs: 3,
            rolling_resistance: 0.004,
        }
    }
}

/// A point along a route with distance and gradient.
#[derive(Debug, Clone)]
pub struct GradientPoint {
    /// Distance from route start (meters)
    pub distance_m: f64,
    /// Gradient at this point (percent, positive = uphill)
    pub gradient_percent: f32,
}

/// Result of gradient calculation for a position.
#[derive(Debug, Clone)]
pub struct GradientResult {
    /// Raw gradient from elevation data (percent)
    pub raw_gradient: f32,
    /// Gradient after applying caps (percent)
    pub capped_gradient: f32,
    /// Gradient after applying difficulty scaling (percent)
    pub effective_gradient: f32,
    /// Smoothed gradient for display (percent)
    pub smoothed_gradient: f32,
}

/// Controller for managing gradient-responsive resistance.
pub trait GradientController: Send + Sync {
    /// Load a route and prepare gradient data.
    ///
    /// # Arguments
    /// * `route_id` - UUID of the loaded GPX route
    ///
    /// # Returns
    /// * `Ok(())` if route loaded successfully
    /// * `Err` if route not found or has no elevation data
    fn load_route(&mut self, route_id: Uuid) -> Result<(), GradientError>;

    /// Update current position and get gradient.
    ///
    /// # Arguments
    /// * `distance_m` - Current distance along route (meters)
    /// * `settings` - User's gradient settings
    ///
    /// # Returns
    /// Gradient result with raw, capped, and effective values
    fn get_gradient_at(
        &self,
        distance_m: f64,
        settings: &GradientSettings,
    ) -> Result<GradientResult, GradientError>;

    /// Generate FTMS simulation command for current gradient.
    ///
    /// # Arguments
    /// * `gradient` - Effective gradient percentage
    /// * `settings` - User's gradient settings (for rolling resistance)
    ///
    /// # Returns
    /// FTMS Control Point command bytes
    fn build_resistance_command(
        &self,
        gradient: f32,
        settings: &GradientSettings,
    ) -> Vec<u8>;

    /// Check if route has sufficient elevation data.
    fn has_elevation_data(&self) -> bool;

    /// Get total route distance.
    fn total_distance(&self) -> Option<f64>;

    /// Get total elevation gain.
    fn total_elevation_gain(&self) -> Option<f64>;
}

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
}
