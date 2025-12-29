//! GPX route handling for gradient simulation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::gradient::calculator::GradientCalculator;
use crate::gradient::error::GradientError;
use crate::gradient::types::{GradientSegment, RoutePoint};

/// A parsed GPX route with gradient data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpxRoute {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Original file path
    pub source_file: Option<PathBuf>,
    /// Total route distance in meters
    pub total_distance_m: f64,
    /// Total elevation gain in meters
    pub total_elevation_m: f64,
    /// Total descent in meters
    pub total_descent_m: f64,
    /// Parsed route points
    pub points: Vec<RoutePoint>,
    /// Pre-calculated gradient segments
    pub segments: Vec<GradientSegment>,
    /// When imported
    pub imported_at: DateTime<Utc>,
}

impl GpxRoute {
    /// Create a new route from points.
    ///
    /// Automatically calculates gradient segments and elevation statistics.
    pub fn from_points(
        name: impl Into<String>,
        points: Vec<RoutePoint>,
    ) -> Result<Self, GradientError> {
        if points.len() < 2 {
            return Err(GradientError::InsufficientPoints {
                found: points.len(),
                minimum: 2,
            });
        }

        let total_distance_m = points.last().map(|p| p.distance_m).unwrap_or(0.0);
        let total_elevation_m = GradientCalculator::total_elevation_gain(&points);
        let total_descent_m = GradientCalculator::total_descent(&points);

        // Calculate segments with 0.5% merge threshold
        let segments = GradientCalculator::calculate_segments(&points, 0.5);

        Ok(Self {
            id: Uuid::new_v4(),
            name: name.into(),
            source_file: None,
            total_distance_m,
            total_elevation_m,
            total_descent_m,
            points,
            segments,
            imported_at: Utc::now(),
        })
    }

    /// Check if the route has elevation data.
    pub fn has_elevation_data(&self) -> bool {
        // Check if there's any variation in elevation
        if self.points.len() < 2 {
            return false;
        }

        let first_elevation = self.points[0].elevation_m;
        self.points
            .iter()
            .any(|p| (p.elevation_m - first_elevation).abs() > 0.1)
    }

    /// Get the gradient at a specific distance along the route.
    pub fn get_gradient_at(&self, distance_m: f64) -> f32 {
        GradientCalculator::get_gradient_at(&self.segments, distance_m)
    }

    /// Get the elevation at a specific distance.
    pub fn get_elevation_at(&self, distance_m: f64) -> Option<f32> {
        // Find the two points surrounding this distance and interpolate
        if self.points.is_empty() {
            return None;
        }

        if distance_m <= 0.0 {
            return Some(self.points[0].elevation_m);
        }

        if distance_m >= self.total_distance_m {
            return self.points.last().map(|p| p.elevation_m);
        }

        // Find the enclosing points
        for window in self.points.windows(2) {
            let p1 = &window[0];
            let p2 = &window[1];

            if distance_m >= p1.distance_m && distance_m <= p2.distance_m {
                // Linear interpolation
                let t = (distance_m - p1.distance_m) / (p2.distance_m - p1.distance_m);
                let elevation = p1.elevation_m + (p2.elevation_m - p1.elevation_m) * t as f32;
                return Some(elevation);
            }
        }

        None
    }

    /// Get progress through the route as a percentage (0.0 to 1.0).
    pub fn get_progress(&self, distance_m: f64) -> f32 {
        if self.total_distance_m <= 0.0 {
            return 0.0;
        }
        (distance_m / self.total_distance_m).clamp(0.0, 1.0) as f32
    }

    /// Get summary statistics for the route.
    pub fn summary(&self) -> RouteSummary {
        let avg_gradient = if !self.segments.is_empty() {
            let total_weighted: f64 = self
                .segments
                .iter()
                .map(|s| s.gradient_percent as f64 * s.length_m())
                .sum();
            (total_weighted / self.total_distance_m) as f32
        } else {
            0.0
        };

        let max_gradient = self
            .segments
            .iter()
            .map(|s| s.gradient_percent)
            .fold(f32::NEG_INFINITY, f32::max);

        let min_gradient = self
            .segments
            .iter()
            .map(|s| s.gradient_percent)
            .fold(f32::INFINITY, f32::min);

        RouteSummary {
            total_distance_m: self.total_distance_m,
            total_elevation_m: self.total_elevation_m,
            total_descent_m: self.total_descent_m,
            avg_gradient,
            max_gradient: if max_gradient.is_finite() {
                max_gradient
            } else {
                0.0
            },
            min_gradient: if min_gradient.is_finite() {
                min_gradient
            } else {
                0.0
            },
            segment_count: self.segments.len(),
        }
    }
}

/// Summary statistics for a route.
#[derive(Debug, Clone)]
pub struct RouteSummary {
    pub total_distance_m: f64,
    pub total_elevation_m: f64,
    pub total_descent_m: f64,
    pub avg_gradient: f32,
    pub max_gradient: f32,
    pub min_gradient: f32,
    pub segment_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route() -> GpxRoute {
        let points = vec![
            RoutePoint::new(0.0, 100.0),
            RoutePoint::new(100.0, 110.0), // 10% grade
            RoutePoint::new(200.0, 120.0), // 10% grade
            RoutePoint::new(300.0, 115.0), // -5% grade (descent)
            RoutePoint::new(400.0, 110.0), // -5% grade
            RoutePoint::new(500.0, 110.0), // flat
        ];
        GpxRoute::from_points("Test Route", points).unwrap()
    }

    #[test]
    fn test_route_creation() {
        let route = create_test_route();
        assert_eq!(route.name, "Test Route");
        assert_eq!(route.total_distance_m, 500.0);
        assert!((route.total_elevation_m - 20.0).abs() < 0.1);
        assert!((route.total_descent_m - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_has_elevation() {
        let route = create_test_route();
        assert!(route.has_elevation_data());

        // Flat route
        let flat_points = vec![
            RoutePoint::new(0.0, 100.0),
            RoutePoint::new(100.0, 100.0),
            RoutePoint::new(200.0, 100.0),
        ];
        let flat_route = GpxRoute::from_points("Flat", flat_points).unwrap();
        assert!(!flat_route.has_elevation_data());
    }

    #[test]
    fn test_get_gradient_at() {
        let route = create_test_route();

        // Should be around 10% at the start (climbing)
        let gradient_50 = route.get_gradient_at(50.0);
        assert!(gradient_50 > 5.0);

        // Should be negative in the descent section
        let gradient_350 = route.get_gradient_at(350.0);
        assert!(gradient_350 < 0.0);
    }

    #[test]
    fn test_get_elevation_at() {
        let route = create_test_route();

        assert!((route.get_elevation_at(0.0).unwrap() - 100.0).abs() < 0.1);
        assert!((route.get_elevation_at(500.0).unwrap() - 110.0).abs() < 0.1);

        // Interpolated value
        let mid_elevation = route.get_elevation_at(50.0).unwrap();
        assert!(mid_elevation > 100.0 && mid_elevation < 110.0);
    }

    #[test]
    fn test_progress() {
        let route = create_test_route();

        assert_eq!(route.get_progress(0.0), 0.0);
        assert!((route.get_progress(250.0) - 0.5).abs() < 0.01);
        assert_eq!(route.get_progress(500.0), 1.0);
    }

    #[test]
    fn test_insufficient_points() {
        let result = GpxRoute::from_points("Short", vec![RoutePoint::new(0.0, 100.0)]);
        assert!(matches!(
            result,
            Err(GradientError::InsufficientPoints { .. })
        ));
    }
}
