//! Third-person camera for following the avatar

use glam::{Mat4, Vec3};

/// Third-person camera that follows the avatar
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec3,
    /// Point the camera is looking at
    pub target: Vec3,
    /// Up vector
    pub up: Vec3,
    /// Field of view in degrees
    pub fov_degrees: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Distance behind the avatar
    follow_distance: f32,
    /// Height above the avatar
    follow_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 5.0, -10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_degrees: 60.0,
            near: 0.1,
            far: 1000.0,
            follow_distance: 8.0,
            follow_height: 3.0,
        }
    }
}

impl Camera {
    /// Update camera position to follow the avatar
    ///
    /// # Arguments
    /// * `avatar_position` - Current position of the avatar
    /// * `avatar_direction` - Direction the avatar is facing (normalized)
    pub fn follow(&mut self, avatar_position: Vec3, avatar_direction: Vec3) {
        // Position camera behind and above the avatar
        let offset = -avatar_direction * self.follow_distance + Vec3::Y * self.follow_height;
        self.position = avatar_position + offset;
        self.target = avatar_position + Vec3::Y * 1.5; // Look at avatar's upper body
    }

    /// Get the view matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get the projection matrix
    ///
    /// # Arguments
    /// * `aspect_ratio` - Width / Height of the viewport
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_degrees.to_radians(),
            aspect_ratio,
            self.near,
            self.far,
        )
    }

    /// Get the combined view-projection matrix
    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }
}
