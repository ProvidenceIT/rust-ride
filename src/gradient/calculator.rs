//! Gradient calculation from elevation data.

use crate::gradient::types::{GradientSegment, RoutePoint};

/// Earth radius in meters for Haversine calculation.
#[allow(dead_code)]
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Calculate distance between two GPS coordinates using Haversine formula.
#[allow(dead_code)]
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_M * c
}

/// Gradient calculator for computing gradients from route points.
pub struct GradientCalculator;

impl GradientCalculator {
    /// Calculate gradient percentage between two elevation values over a distance.
    ///
    /// Returns the gradient as a percentage (e.g., 5.0 for 5% grade).
    pub fn calculate_gradient(elevation_change_m: f32, distance_m: f64) -> f32 {
        if distance_m <= 0.0 {
            return 0.0;
        }
        ((elevation_change_m as f64 / distance_m) * 100.0) as f32
    }

    /// Calculate gradient segments from a series of route points.
    ///
    /// Combines consecutive points with similar gradients to reduce segment count.
    pub fn calculate_segments(points: &[RoutePoint], merge_threshold: f32) -> Vec<GradientSegment> {
        if points.len() < 2 {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut current_start = 0;

        for i in 1..points.len() {
            let prev = &points[i - 1];
            let curr = &points[i];

            let distance = curr.distance_m - prev.distance_m;
            let elevation_change = curr.elevation_m - prev.elevation_m;
            let gradient = Self::calculate_gradient(elevation_change, distance);

            // Check if we should start a new segment
            let should_merge = if let Some(last) = segments.last() {
                let last_seg: &GradientSegment = last;
                (gradient - last_seg.gradient_percent).abs() <= merge_threshold
            } else {
                false
            };

            if should_merge {
                // Update the last segment's end point
                if let Some(last) = segments.last_mut() {
                    last.end_distance_m = curr.distance_m;
                    // Recalculate gradient for merged segment
                    let start_point = &points[current_start];
                    let total_distance = curr.distance_m - start_point.distance_m;
                    let total_elevation = curr.elevation_m - start_point.elevation_m;
                    last.gradient_percent =
                        Self::calculate_gradient(total_elevation, total_distance);
                }
            } else {
                // Start a new segment
                current_start = i - 1;
                segments.push(GradientSegment::new(
                    prev.distance_m,
                    curr.distance_m,
                    gradient,
                ));
            }
        }

        segments
    }

    /// Get the gradient at a specific distance along the route.
    pub fn get_gradient_at(segments: &[GradientSegment], distance_m: f64) -> f32 {
        // Binary search for the segment containing this distance
        match segments.binary_search_by(|seg| {
            if distance_m < seg.start_distance_m {
                std::cmp::Ordering::Greater
            } else if distance_m >= seg.end_distance_m {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(idx) => segments[idx].gradient_percent,
            Err(0) => {
                // Before first segment - return first gradient or 0
                segments.first().map(|s| s.gradient_percent).unwrap_or(0.0)
            }
            Err(idx) if idx >= segments.len() => {
                // After last segment - return last gradient or 0
                segments.last().map(|s| s.gradient_percent).unwrap_or(0.0)
            }
            Err(_) => 0.0,
        }
    }

    /// Calculate total elevation gain from route points.
    pub fn total_elevation_gain(points: &[RoutePoint]) -> f64 {
        if points.len() < 2 {
            return 0.0;
        }

        points
            .windows(2)
            .map(|pair| {
                let diff = pair[1].elevation_m - pair[0].elevation_m;
                if diff > 0.0 {
                    diff as f64
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// Calculate total descent from route points.
    pub fn total_descent(points: &[RoutePoint]) -> f64 {
        if points.len() < 2 {
            return 0.0;
        }

        points
            .windows(2)
            .map(|pair| {
                let diff = pair[0].elevation_m - pair[1].elevation_m;
                if diff > 0.0 {
                    diff as f64
                } else {
                    0.0
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // New York to Los Angeles (approximate)
        let distance = haversine_distance(40.7128, -74.0060, 34.0522, -118.2437);
        // Should be approximately 3944 km
        assert!((distance - 3_944_000.0).abs() < 50_000.0);
    }

    #[test]
    fn test_calculate_gradient() {
        // 10m rise over 100m = 10%
        assert!((GradientCalculator::calculate_gradient(10.0, 100.0) - 10.0).abs() < 0.01);

        // 5m rise over 200m = 2.5%
        assert!((GradientCalculator::calculate_gradient(5.0, 200.0) - 2.5).abs() < 0.01);

        // -10m (descent) over 100m = -10%
        assert!((GradientCalculator::calculate_gradient(-10.0, 100.0) - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn test_calculate_segments() {
        let points = vec![
            RoutePoint::new(0.0, 100.0),
            RoutePoint::new(100.0, 110.0), // 10% grade
            RoutePoint::new(200.0, 120.0), // 10% grade
            RoutePoint::new(300.0, 115.0), // -5% grade
            RoutePoint::new(400.0, 110.0), // -5% grade
        ];

        let segments = GradientCalculator::calculate_segments(&points, 1.0);

        // Should merge similar gradients
        assert!(segments.len() <= 4);
        assert!(segments.len() >= 2);
    }

    #[test]
    fn test_total_elevation() {
        let points = vec![
            RoutePoint::new(0.0, 100.0),
            RoutePoint::new(100.0, 150.0), // +50m
            RoutePoint::new(200.0, 120.0), // -30m (descent)
            RoutePoint::new(300.0, 180.0), // +60m
        ];

        let gain = GradientCalculator::total_elevation_gain(&points);
        assert!((gain - 110.0).abs() < 0.1); // 50 + 60 = 110m

        let descent = GradientCalculator::total_descent(&points);
        assert!((descent - 30.0).abs() < 0.1);
    }
}
