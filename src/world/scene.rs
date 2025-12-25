//! Scene graph for the 3D world

use glam::{Quat, Vec3};

/// Transform component for scene objects
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Position in world space
    pub position: Vec3,
    /// Rotation as a quaternion
    pub rotation: Quat,
    /// Scale factor
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Create a transform at the given position
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Create a transform with position and rotation
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }
}

/// Handle to a loaded model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelHandle(pub u32);

/// A scenery object in the world
#[derive(Debug, Clone)]
pub struct SceneryObject {
    /// Model to render
    pub model: ModelHandle,
    /// Transform in world space
    pub transform: Transform,
    /// Distance at which to use lower LOD
    pub lod_distance: f32,
}

/// Lighting configuration
#[derive(Debug, Clone)]
pub struct Lighting {
    /// Direction of the sun/main light
    pub sun_direction: Vec3,
    /// Ambient light color and intensity
    pub ambient_color: Vec3,
    /// Sun light color and intensity
    pub sun_color: Vec3,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(0.5, 1.0, 0.3).normalize(),
            ambient_color: Vec3::new(0.3, 0.3, 0.35),
            sun_color: Vec3::new(1.0, 0.95, 0.9),
        }
    }
}

/// Sky configuration
#[derive(Debug, Clone)]
pub struct Sky {
    /// Top color of sky gradient
    pub top_color: Vec3,
    /// Horizon color of sky gradient
    pub horizon_color: Vec3,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            top_color: Vec3::new(0.4, 0.6, 1.0),
            horizon_color: Vec3::new(0.8, 0.85, 0.95),
        }
    }
}

/// The complete scene to render
#[derive(Debug, Default)]
pub struct Scene {
    /// Scenery objects (trees, buildings, etc.)
    pub scenery: Vec<SceneryObject>,
    /// Lighting configuration
    pub lighting: Lighting,
    /// Sky configuration
    pub sky: Sky,
}

impl Scene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a scenery object to the scene
    pub fn add_scenery(&mut self, object: SceneryObject) {
        self.scenery.push(object);
    }

    /// Clear all scenery objects
    pub fn clear_scenery(&mut self) {
        self.scenery.clear();
    }
}
