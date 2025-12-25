//! Avatar representation in the 3D world

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::route::Route;

/// Bike style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BikeStyle {
    #[default]
    RoadBike,
    TimeTrial,
    Gravel,
}

/// Avatar animation state
#[derive(Debug, Clone, Copy, Default)]
pub enum AnimationState {
    #[default]
    Idle,
    Pedaling {
        cadence: f32,
    },
    Coasting,
}

/// Configuration for avatar appearance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarConfig {
    /// Primary jersey color as RGB
    pub jersey_color: [u8; 3],
    /// Bike style
    pub bike_style: BikeStyle,
    /// Secondary jersey color (optional)
    #[serde(default)]
    pub jersey_secondary: Option<[u8; 3]>,
    /// Helmet color (optional)
    #[serde(default)]
    pub helmet_color: Option<[u8; 3]>,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            jersey_color: [255, 0, 0], // Red
            bike_style: BikeStyle::RoadBike,
            jersey_secondary: None,
            helmet_color: None,
        }
    }
}

/// The avatar in the 3D world
#[derive(Debug, Clone)]
pub struct Avatar {
    /// Current position in world space
    pub position: Vec3,
    /// Current rotation (heading) in radians
    pub rotation: f32,
    /// Current animation state
    pub animation_state: AnimationState,
    /// Avatar configuration
    pub config: AvatarConfig,
    /// Distance traveled along route
    distance_on_route: f32,
}

impl Default for Avatar {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: 0.0,
            animation_state: AnimationState::Idle,
            config: AvatarConfig::default(),
            distance_on_route: 0.0,
        }
    }
}

impl Avatar {
    /// Create a new avatar with the given configuration
    pub fn new(config: AvatarConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Update avatar position based on speed and route
    ///
    /// # Arguments
    /// * `speed_mps` - Current speed in meters per second
    /// * `route` - The route being followed
    /// * `delta_time` - Time since last update in seconds
    pub fn update(&mut self, speed_mps: f32, route: &Route, delta_time: f32) {
        // Update distance along route
        self.distance_on_route += speed_mps * delta_time;

        // Wrap around if past the end of a loop route
        if self.distance_on_route > route.total_distance {
            self.distance_on_route %= route.total_distance;
        }

        // Get new position and heading from route
        let (position, heading) = route.get_position(self.distance_on_route);
        self.position = position;
        self.rotation = heading;

        // Update animation based on speed
        if speed_mps < 0.5 {
            self.animation_state = AnimationState::Idle;
        } else {
            // Estimate cadence from speed (simplified)
            let estimated_cadence = (speed_mps * 12.0).clamp(40.0, 120.0);
            self.animation_state = AnimationState::Pedaling {
                cadence: estimated_cadence,
            };
        }
    }

    /// Set pedaling cadence explicitly (from sensor data)
    pub fn set_cadence(&mut self, cadence: u8) {
        if cadence > 0 {
            self.animation_state = AnimationState::Pedaling {
                cadence: cadence as f32,
            };
        } else {
            self.animation_state = AnimationState::Coasting;
        }
    }

    /// Get the direction the avatar is facing
    pub fn direction(&self) -> Vec3 {
        Vec3::new(self.rotation.cos(), 0.0, self.rotation.sin())
    }

    /// Get current distance traveled on route
    pub fn distance_traveled(&self) -> f32 {
        self.distance_on_route
    }

    /// Reset position to start of route
    pub fn reset(&mut self) {
        self.distance_on_route = 0.0;
        self.position = Vec3::ZERO;
        self.rotation = 0.0;
        self.animation_state = AnimationState::Idle;
    }
}
