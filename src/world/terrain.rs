//! Terrain and road rendering

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
