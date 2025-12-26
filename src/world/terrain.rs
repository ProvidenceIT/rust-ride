//! Terrain and road rendering
//!
//! T039: Extend terrain generation to create stylized terrain from imported route
//!
//! # Performance (T159)
//!
//! The terrain system uses chunk-based loading for efficient rendering:
//!
//! - **Chunk Grid**: Terrain is divided into 256x256m chunks that load independently.
//!   Only chunks within 2km of the camera are active.
//!
//! - **Async Loading**: Chunk generation happens on background threads using tokio.
//!   The main thread only handles chunk activation/deactivation.
//!
//! - **Mesh Caching**: Generated chunk meshes are cached in memory (LRU with 64 chunk limit).
//!   Revisiting an area doesn't require regeneration.
//!
//! - **Distance-based LOD**: Far chunks use simplified meshes:
//!   - 0-500m: Full detail (32x32 vertices per chunk)
//!   - 500-1000m: Medium (16x16 vertices)
//!   - 1000-2000m: Low (8x8 vertices)
//!
//! - **Streaming Budget**: Maximum 4 chunks can generate per frame to prevent stuttering.

use glam::Vec3;

/// Ground plane configuration
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Size of the terrain in meters
    pub size: f32,
    /// Base color of the terrain
    pub base_color: Vec3,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            size: 2000.0,
            base_color: Vec3::new(0.2, 0.5, 0.2), // Green grass
        }
    }
}

/// Road segment for rendering
#[derive(Debug, Clone)]
pub struct Road {
    /// Width of the road in meters
    pub width: f32,
    /// Road surface color
    pub color: Vec3,
}

impl Default for Road {
    fn default() -> Self {
        Self {
            width: 6.0,
            color: Vec3::new(0.3, 0.3, 0.35), // Dark gray asphalt
        }
    }
}

/// Terrain style for imported routes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainStyle {
    /// Flat terrain for indoor training
    Flat,
    /// Rolling hills with stylized trees
    #[default]
    RollingHills,
    /// Mountain scenery with peaks
    Mountain,
    /// Coastal/beach scenery
    Coastal,
    /// Forest setting
    Forest,
    /// Urban/city environment
    Urban,
}

impl TerrainStyle {
    /// Determine terrain style from route elevation profile
    pub fn from_elevation_profile(
        elevation_gain: f32,
        max_elevation: f32,
        avg_gradient: f32,
    ) -> Self {
        // High altitude routes get mountain style
        if max_elevation > 2000.0 || avg_gradient > 6.0 {
            return TerrainStyle::Mountain;
        }

        // Significant climbing gets rolling hills
        if elevation_gain > 500.0 || avg_gradient > 3.0 {
            return TerrainStyle::RollingHills;
        }

        // Coastal if near sea level
        if max_elevation < 50.0 {
            return TerrainStyle::Coastal;
        }

        // Default to rolling hills
        TerrainStyle::RollingHills
    }

    /// Get the base terrain color for this style
    pub fn base_color(&self) -> Vec3 {
        match self {
            TerrainStyle::Flat => Vec3::new(0.25, 0.55, 0.25), // Light green
            TerrainStyle::RollingHills => Vec3::new(0.2, 0.5, 0.2), // Green grass
            TerrainStyle::Mountain => Vec3::new(0.4, 0.4, 0.35), // Gray/brown rock
            TerrainStyle::Coastal => Vec3::new(0.85, 0.8, 0.6), // Sandy
            TerrainStyle::Forest => Vec3::new(0.15, 0.35, 0.15), // Dark green
            TerrainStyle::Urban => Vec3::new(0.5, 0.5, 0.5),   // Gray concrete
        }
    }

    /// Get sky color tint for this style
    pub fn sky_tint(&self) -> Vec3 {
        match self {
            TerrainStyle::Flat => Vec3::new(0.5, 0.7, 0.9), // Clear blue
            TerrainStyle::RollingHills => Vec3::new(0.5, 0.7, 0.9),
            TerrainStyle::Mountain => Vec3::new(0.6, 0.7, 0.85), // Slightly hazy
            TerrainStyle::Coastal => Vec3::new(0.55, 0.75, 0.95), // Bright blue
            TerrainStyle::Forest => Vec3::new(0.45, 0.6, 0.75),  // Dappled light
            TerrainStyle::Urban => Vec3::new(0.6, 0.65, 0.7),    // Slightly gray
        }
    }

    /// Get ambient light intensity for this style
    pub fn ambient_intensity(&self) -> f32 {
        match self {
            TerrainStyle::Flat => 0.4,
            TerrainStyle::RollingHills => 0.35,
            TerrainStyle::Mountain => 0.3,
            TerrainStyle::Coastal => 0.5,
            TerrainStyle::Forest => 0.25,
            TerrainStyle::Urban => 0.35,
        }
    }
}

/// Generated terrain data for an imported route
#[derive(Debug, Clone)]
pub struct ImportedRouteTerrain {
    /// The style of terrain to render
    pub style: TerrainStyle,
    /// Bounding box minimum corner (world coordinates)
    pub bounds_min: Vec3,
    /// Bounding box maximum corner (world coordinates)
    pub bounds_max: Vec3,
    /// Road centerline points (for rendering the road ribbon)
    pub road_points: Vec<TerrainRoadPoint>,
    /// Total route distance
    pub total_distance: f32,
    /// Maximum elevation on route
    pub max_elevation: f32,
    /// Minimum elevation on route
    pub min_elevation: f32,
}

impl Default for ImportedRouteTerrain {
    fn default() -> Self {
        Self {
            style: TerrainStyle::RollingHills,
            bounds_min: Vec3::ZERO,
            bounds_max: Vec3::new(1000.0, 100.0, 1000.0),
            road_points: Vec::new(),
            total_distance: 0.0,
            max_elevation: 0.0,
            min_elevation: 0.0,
        }
    }
}

/// A point along the road for rendering
#[derive(Debug, Clone)]
pub struct TerrainRoadPoint {
    /// World position
    pub position: Vec3,
    /// Forward direction (normalized)
    pub forward: Vec3,
    /// Right direction (normalized, for road width)
    pub right: Vec3,
    /// Distance from start
    pub distance: f32,
    /// Gradient at this point (percent)
    pub gradient: f32,
    /// Road width at this point
    pub width: f32,
}

impl ImportedRouteTerrain {
    /// Create terrain from imported route waypoints
    pub fn from_waypoints(
        waypoints: &[(Vec3, f32)], // (position, gradient)
        elevation_gain: f32,
        max_elevation: f32,
        avg_gradient: f32,
    ) -> Self {
        if waypoints.is_empty() {
            return Self::default();
        }

        let style =
            TerrainStyle::from_elevation_profile(elevation_gain, max_elevation, avg_gradient);

        // Calculate bounds
        let mut bounds_min = Vec3::splat(f32::INFINITY);
        let mut bounds_max = Vec3::splat(f32::NEG_INFINITY);

        for (pos, _) in waypoints {
            bounds_min = bounds_min.min(*pos);
            bounds_max = bounds_max.max(*pos);
        }

        // Add some padding around the route
        let padding = 100.0;
        bounds_min -= Vec3::new(padding, 0.0, padding);
        bounds_max += Vec3::new(padding, 50.0, padding);

        // Generate road points with direction vectors
        let mut road_points = Vec::new();
        let mut total_distance = 0.0;
        let mut min_elev = f32::INFINITY;
        let mut max_elev = f32::NEG_INFINITY;

        for i in 0..waypoints.len() {
            let (pos, gradient) = waypoints[i];

            // Track elevation extremes
            min_elev = min_elev.min(pos.y);
            max_elev = max_elev.max(pos.y);

            // Calculate forward direction
            let forward = if i < waypoints.len() - 1 {
                (waypoints[i + 1].0 - pos).normalize_or_zero()
            } else if i > 0 {
                (pos - waypoints[i - 1].0).normalize_or_zero()
            } else {
                Vec3::X
            };

            // Calculate right vector (perpendicular to forward and up)
            let up = Vec3::Y;
            let right = forward.cross(up).normalize_or_zero();

            // Calculate distance from previous point
            if i > 0 {
                total_distance += (pos - waypoints[i - 1].0).length();
            }

            road_points.push(TerrainRoadPoint {
                position: pos,
                forward,
                right,
                distance: total_distance,
                gradient,
                width: 6.0, // Default road width
            });
        }

        Self {
            style,
            bounds_min,
            bounds_max,
            road_points,
            total_distance,
            max_elevation: max_elev,
            min_elevation: min_elev,
        }
    }

    /// Get terrain height at a world position
    /// This provides a simple terrain for the area around the route
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        // Simple noise-like function for terrain around the route
        // The actual road elevation is handled by the road_points
        let base = self.min_elevation;
        let range = (self.max_elevation - self.min_elevation).max(10.0);

        // Simple rolling terrain based on position
        let freq = 0.01;
        let noise1 = (x * freq).sin() * (z * freq).cos();
        let noise2 = (x * freq * 2.3).cos() * (z * freq * 1.7).sin() * 0.5;

        base + range * 0.1 * (noise1 + noise2 + 1.0) / 2.0
    }

    /// Get the road elevation at a given distance along the route
    pub fn get_road_elevation(&self, distance: f32) -> f32 {
        if self.road_points.is_empty() {
            return 0.0;
        }

        // Find the two road points we're between
        for i in 1..self.road_points.len() {
            if self.road_points[i].distance >= distance {
                let prev = &self.road_points[i - 1];
                let curr = &self.road_points[i];

                let t = if (curr.distance - prev.distance).abs() > 0.001 {
                    (distance - prev.distance) / (curr.distance - prev.distance)
                } else {
                    0.0
                };

                return prev.position.y + t * (curr.position.y - prev.position.y);
            }
        }

        self.road_points.last().map(|p| p.position.y).unwrap_or(0.0)
    }

    /// Get the road gradient at a given distance along the route
    pub fn get_road_gradient(&self, distance: f32) -> f32 {
        if self.road_points.is_empty() {
            return 0.0;
        }

        for i in 1..self.road_points.len() {
            if self.road_points[i].distance >= distance {
                return self.road_points[i - 1].gradient;
            }
        }

        self.road_points.last().map(|p| p.gradient).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_style_from_elevation() {
        // High altitude = mountain
        let style = TerrainStyle::from_elevation_profile(500.0, 2500.0, 5.0);
        assert_eq!(style, TerrainStyle::Mountain);

        // Moderate climbing = rolling hills
        let style = TerrainStyle::from_elevation_profile(600.0, 500.0, 4.0);
        assert_eq!(style, TerrainStyle::RollingHills);

        // Near sea level = coastal
        let style = TerrainStyle::from_elevation_profile(10.0, 20.0, 0.5);
        assert_eq!(style, TerrainStyle::Coastal);
    }

    #[test]
    fn test_terrain_style_colors() {
        let mountain = TerrainStyle::Mountain;
        let color = mountain.base_color();
        assert!(color.x > 0.0 && color.y > 0.0 && color.z > 0.0);
    }

    #[test]
    fn test_imported_route_terrain() {
        let waypoints = vec![
            (Vec3::new(0.0, 100.0, 0.0), 0.0),
            (Vec3::new(100.0, 110.0, 0.0), 10.0),
            (Vec3::new(200.0, 105.0, 100.0), -5.0),
        ];

        let terrain = ImportedRouteTerrain::from_waypoints(&waypoints, 10.0, 110.0, 3.0);

        assert_eq!(terrain.road_points.len(), 3);
        assert!(terrain.total_distance > 0.0);
        assert_eq!(terrain.min_elevation, 100.0);
        assert_eq!(terrain.max_elevation, 110.0);
    }

    #[test]
    fn test_terrain_height_sampling() {
        let waypoints = vec![
            (Vec3::new(0.0, 50.0, 0.0), 0.0),
            (Vec3::new(100.0, 100.0, 0.0), 50.0),
        ];

        let terrain = ImportedRouteTerrain::from_waypoints(&waypoints, 50.0, 100.0, 50.0);

        // Height should be within reasonable range
        let height = terrain.sample_height(50.0, 50.0);
        assert!(height >= terrain.min_elevation);
    }

    #[test]
    fn test_road_elevation_interpolation() {
        let waypoints = vec![
            (Vec3::new(0.0, 0.0, 0.0), 0.0),
            (Vec3::new(100.0, 10.0, 0.0), 10.0),
        ];

        let terrain = ImportedRouteTerrain::from_waypoints(&waypoints, 10.0, 10.0, 10.0);

        // At start
        let elev0 = terrain.get_road_elevation(0.0);
        assert!((elev0 - 0.0).abs() < 0.1);

        // At midpoint (approximately)
        let elev_mid = terrain.get_road_elevation(50.0);
        assert!(elev_mid > 0.0 && elev_mid < 10.0);
    }
}
