//! Route definitions for virtual worlds
//!
//! Routes define paths through virtual worlds with waypoints,
//! distance, and elevation data.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A point along a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    /// 3D position (x, y, z where z is elevation)
    pub position: Vec3,
    /// Distance from route start in meters
    pub distance_from_start: f32,
    /// Gradient at this point as a percentage
    pub gradient_percent: f32,
    /// Road surface type
    #[serde(default)]
    pub surface_type: SurfaceType,
}

/// Road surface types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceType {
    #[default]
    Asphalt,
    Concrete,
    Cobblestone,
    Gravel,
    Dirt,
}

/// A complete route through a virtual world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Route identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Total route distance in meters
    pub total_distance: f32,
    /// Ordered list of waypoints
    pub waypoints: Vec<Waypoint>,
    /// Elevation profile (elevation values at regular intervals)
    #[serde(default)]
    pub elevation_profile: Vec<f32>,
}

impl Route {
    /// Get position and heading at a given distance along the route
    ///
    /// # Arguments
    /// * `distance` - Distance from start in meters
    ///
    /// # Returns
    /// Tuple of (position, heading_radians)
    pub fn get_position(&self, distance: f32) -> (Vec3, f32) {
        if self.waypoints.is_empty() {
            return (Vec3::ZERO, 0.0);
        }

        // Clamp distance to route bounds
        let distance = distance.clamp(0.0, self.total_distance);

        // Find the waypoint segment we're on
        let mut prev_wp = &self.waypoints[0];
        for wp in &self.waypoints[1..] {
            if wp.distance_from_start >= distance {
                // Interpolate between prev_wp and wp
                let segment_length = wp.distance_from_start - prev_wp.distance_from_start;
                if segment_length > 0.0 {
                    let t = (distance - prev_wp.distance_from_start) / segment_length;
                    let position = prev_wp.position.lerp(wp.position, t);

                    // Calculate heading from direction
                    let direction = wp.position - prev_wp.position;
                    let heading = direction.z.atan2(direction.x);

                    return (position, heading);
                }
                return (prev_wp.position, 0.0);
            }
            prev_wp = wp;
        }

        // Past the end of the route
        let last = self.waypoints.last().unwrap();
        (last.position, 0.0)
    }

    /// Get gradient at a given distance along the route
    pub fn get_gradient(&self, distance: f32) -> f32 {
        if self.waypoints.is_empty() {
            return 0.0;
        }

        let distance = distance.clamp(0.0, self.total_distance);

        // Find the waypoint we're at or past
        for wp in &self.waypoints {
            if wp.distance_from_start >= distance {
                return wp.gradient_percent;
            }
        }

        self.waypoints
            .last()
            .map(|w| w.gradient_percent)
            .unwrap_or(0.0)
    }

    /// Get elevation at a given distance along the route
    pub fn get_elevation(&self, distance: f32) -> f32 {
        let (position, _) = self.get_position(distance);
        position.y // Y is typically elevation in our coordinate system
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route() -> Route {
        Route {
            id: "test".to_string(),
            name: "Test Route".to_string(),
            total_distance: 1000.0,
            waypoints: vec![
                Waypoint {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    distance_from_start: 0.0,
                    gradient_percent: 0.0,
                    surface_type: SurfaceType::Asphalt,
                },
                Waypoint {
                    position: Vec3::new(500.0, 10.0, 0.0),
                    distance_from_start: 500.0,
                    gradient_percent: 2.0,
                    surface_type: SurfaceType::Asphalt,
                },
                Waypoint {
                    position: Vec3::new(1000.0, 0.0, 0.0),
                    distance_from_start: 1000.0,
                    gradient_percent: -2.0,
                    surface_type: SurfaceType::Asphalt,
                },
            ],
            elevation_profile: vec![],
        }
    }

    #[test]
    fn test_get_position_start() {
        let route = create_test_route();
        let (pos, _) = route.get_position(0.0);
        assert_eq!(pos, Vec3::ZERO);
    }

    #[test]
    fn test_get_position_middle() {
        let route = create_test_route();
        let (pos, _) = route.get_position(250.0);
        assert!(pos.x > 0.0 && pos.x < 500.0);
    }

    #[test]
    fn test_get_gradient() {
        let route = create_test_route();
        assert_eq!(route.get_gradient(0.0), 0.0);
        assert_eq!(route.get_gradient(500.0), 2.0);
    }
}
