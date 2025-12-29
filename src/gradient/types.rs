//! Gradient type definitions.

use serde::{Deserialize, Serialize};

/// A point along a route with position and elevation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePoint {
    /// Distance from start in meters
    pub distance_m: f64,
    /// Elevation in meters
    pub elevation_m: f32,
    /// Latitude (optional, for reference)
    pub lat: Option<f64>,
    /// Longitude (optional, for reference)
    pub lon: Option<f64>,
}

impl RoutePoint {
    /// Create a new route point.
    pub fn new(distance_m: f64, elevation_m: f32) -> Self {
        Self {
            distance_m,
            elevation_m,
            lat: None,
            lon: None,
        }
    }

    /// Create a route point with GPS coordinates.
    pub fn with_coords(distance_m: f64, elevation_m: f32, lat: f64, lon: f64) -> Self {
        Self {
            distance_m,
            elevation_m,
            lat: Some(lat),
            lon: Some(lon),
        }
    }
}

/// A segment of the route with calculated gradient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientSegment {
    /// Start distance in meters
    pub start_distance_m: f64,
    /// End distance in meters
    pub end_distance_m: f64,
    /// Gradient percentage (positive = uphill)
    pub gradient_percent: f32,
}

impl GradientSegment {
    /// Create a new gradient segment.
    pub fn new(start_m: f64, end_m: f64, gradient: f32) -> Self {
        Self {
            start_distance_m: start_m,
            end_distance_m: end_m,
            gradient_percent: gradient,
        }
    }

    /// Get the length of this segment in meters.
    pub fn length_m(&self) -> f64 {
        self.end_distance_m - self.start_distance_m
    }

    /// Check if a distance falls within this segment.
    pub fn contains(&self, distance_m: f64) -> bool {
        distance_m >= self.start_distance_m && distance_m < self.end_distance_m
    }
}

/// A gradient point for display/lookup.
#[derive(Debug, Clone, Copy)]
pub struct GradientPoint {
    /// Distance from route start (meters)
    pub distance_m: f64,
    /// Gradient at this point (percent, positive = uphill)
    pub gradient_percent: f32,
}

/// Result of gradient calculation for a position.
#[derive(Debug, Clone, Copy)]
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

impl GradientResult {
    /// Create a result with all gradients set to the same value.
    pub fn flat() -> Self {
        Self {
            raw_gradient: 0.0,
            capped_gradient: 0.0,
            effective_gradient: 0.0,
            smoothed_gradient: 0.0,
        }
    }

    /// Create a result from raw gradient with transformations applied.
    pub fn from_raw(
        raw: f32,
        capped: f32,
        effective: f32,
        smoothed: f32,
    ) -> Self {
        Self {
            raw_gradient: raw,
            capped_gradient: capped,
            effective_gradient: effective,
            smoothed_gradient: smoothed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_point() {
        let point = RoutePoint::new(1000.0, 150.0);
        assert_eq!(point.distance_m, 1000.0);
        assert_eq!(point.elevation_m, 150.0);
        assert!(point.lat.is_none());
    }

    #[test]
    fn test_gradient_segment() {
        let segment = GradientSegment::new(0.0, 100.0, 5.0);
        assert_eq!(segment.length_m(), 100.0);
        assert!(segment.contains(50.0));
        assert!(!segment.contains(100.0));
    }

    #[test]
    fn test_gradient_result() {
        let result = GradientResult::flat();
        assert_eq!(result.raw_gradient, 0.0);
        assert_eq!(result.effective_gradient, 0.0);
    }
}
