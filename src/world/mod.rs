//! 3D Virtual World Module
//!
//! This module provides a Zwift-like 3D virtual cycling environment.
//! It integrates with the sensor system to receive power data and renders
//! an avatar moving through a virtual world at a speed proportional to power output.

pub mod avatar;
pub mod camera;
pub mod hud;
pub mod physics;
pub mod renderer;
pub mod route;
pub mod scene;
pub mod terrain;
pub mod worlds;

// 3D World & Content feature modules
pub mod achievements;
pub mod creator;
pub mod import;
pub mod landmarks;
pub mod npc;
pub mod procedural;
pub mod segments;
pub mod weather;

use std::sync::Arc;

use glam::Vec3;
use thiserror::Error;

use avatar::{Avatar, AvatarConfig};
use camera::Camera;
use hud::Hud;
use physics::PhysicsEngine;
use renderer::Renderer;
use route::{Route, StoredRoute, StoredWaypoint, Waypoint};
use scene::Scene;
use terrain::{ImportedRouteTerrain, Road, Terrain, TerrainStyle};
use worlds::{RouteDefinition, TimeOfDay, WorldDefinition, WorldTheme};

/// Errors that can occur in the 3D world module
#[derive(Debug, Error)]
pub enum WorldError {
    #[error("GPU initialization failed: {0}")]
    GpuInitError(String),

    #[error("Shader compilation failed: {0}")]
    ShaderError(String),

    #[error("Asset loading failed: {0}")]
    AssetError(String),

    #[error("World definition not found: {0}")]
    WorldNotFound(String),

    #[error("Route not found: {0}")]
    RouteNotFound(String),

    #[error("Insufficient GPU memory")]
    OutOfMemory,

    #[error("Render error: {0}")]
    RenderError(String),
}

/// Statistics about the current ride in the 3D world
#[derive(Debug, Clone, Default)]
pub struct WorldStats {
    /// Current speed in meters per second
    pub speed_mps: f32,
    /// Total distance traveled in meters
    pub distance_meters: f32,
    /// Current elevation in meters
    pub elevation_meters: f32,
    /// Current gradient as a percentage
    pub gradient_percent: f32,
    /// Distance remaining on route in meters
    pub route_remaining_meters: f32,
}

/// The main 3D world controller
pub struct World3D {
    /// GPU renderer
    renderer: Option<Renderer>,
    /// Scene configuration
    scene: Scene,
    /// Camera controller
    camera: Camera,
    /// Avatar (cyclist)
    avatar: Avatar,
    /// Physics engine
    physics: PhysicsEngine,
    /// Current route
    route: Route,
    /// Terrain configuration
    terrain: Terrain,
    /// Road configuration
    road: Road,
    /// HUD overlay
    hud: Hud,
    /// Current world stats
    stats: WorldStats,
    /// World definition
    world_def: WorldDefinition,
    /// Is the 3D mode active
    active: bool,
    /// Render width
    width: u32,
    /// Render height
    height: u32,
}

impl std::fmt::Debug for World3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World3D")
            .field("active", &self.active)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("world", &self.world_def.name)
            .field("route", &self.route.name)
            .finish()
    }
}

impl World3D {
    /// Create a new 3D world (without GPU - call init_renderer later)
    pub fn new(
        world_def: WorldDefinition,
        route_def: &RouteDefinition,
        avatar_config: AvatarConfig,
        rider_mass_kg: f32,
    ) -> Result<Self, WorldError> {
        // Create route from definition
        let route = worlds::create_basic_route(route_def);

        // Create avatar
        let avatar = Avatar::new(avatar_config);

        // Create physics engine
        let physics = PhysicsEngine::new(rider_mass_kg);

        // Create scene with default configuration
        let scene = Scene::new();

        // Create camera
        let camera = Camera::default();

        // Create terrain and road
        let terrain = Terrain::default();
        let road = Road::default();

        // Create HUD
        let hud = Hud::new();

        Ok(Self {
            renderer: None,
            scene,
            camera,
            avatar,
            physics,
            route,
            terrain,
            road,
            hud,
            stats: WorldStats::default(),
            world_def,
            active: false,
            width: 800,
            height: 600,
        })
    }

    /// Initialize the GPU renderer
    ///
    /// This must be called with the wgpu device and queue from eframe's render state
    /// before rendering can occur.
    pub fn init_renderer(
        &mut self,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Result<(), WorldError> {
        self.width = width;
        self.height = height;
        self.renderer = Some(Renderer::new(device, queue, width, height)?);
        self.active = true;
        Ok(())
    }

    /// Check if renderer is initialized
    pub fn is_initialized(&self) -> bool {
        self.renderer.is_some()
    }

    /// Update world state (called each frame)
    ///
    /// # Arguments
    /// * `power_watts` - Current power reading from sensors
    /// * `cadence` - Current cadence reading from sensors (optional)
    /// * `delta_time` - Time since last update in seconds
    pub fn update(&mut self, power_watts: u16, cadence: Option<u8>, delta_time: f32) {
        if !self.active {
            return;
        }

        // Get current gradient from route
        let gradient = self.route.get_gradient(self.avatar.distance_traveled());

        // Calculate speed from power and gradient
        let speed_mps = self.physics.calculate_speed(power_watts, gradient);

        // Update avatar position
        self.avatar.update(speed_mps, &self.route, delta_time);

        // Update cadence animation if available
        if let Some(cad) = cadence {
            self.avatar.set_cadence(cad);
        }

        // Update camera to follow avatar
        self.camera
            .follow(self.avatar.position, self.avatar.direction());

        // Update stats
        let distance_traveled = self.avatar.distance_traveled();
        self.stats = WorldStats {
            speed_mps,
            distance_meters: distance_traveled,
            elevation_meters: self.route.get_elevation(distance_traveled),
            gradient_percent: gradient,
            route_remaining_meters: (self.route.total_distance - distance_traveled).max(0.0),
        };
    }

    /// Render the 3D world
    ///
    /// This renders the scene to an internal texture. Use `get_texture_id()` to
    /// get the egui TextureId for display.
    pub fn render(&mut self) {
        let Some(ref mut renderer) = self.renderer else {
            return;
        };

        // Get route waypoints as Vec3 positions
        let waypoints: Vec<Vec3> = self.route.waypoints.iter().map(|wp| wp.position).collect();

        // Get avatar color from config
        let jersey = self.avatar.config.jersey_color;
        let avatar_color = [
            jersey[0] as f32 / 255.0,
            jersey[1] as f32 / 255.0,
            jersey[2] as f32 / 255.0,
        ];

        // Render the scene
        renderer.render(
            &self.scene,
            &self.camera,
            &self.terrain,
            &self.road,
            &waypoints,
            self.avatar.position,
            self.avatar.rotation,
            avatar_color,
        );
    }

    /// Register the rendered texture with egui and return the TextureId
    ///
    /// This must be called after `render()` to get the texture for display.
    /// The texture handle is cached and reused on subsequent calls.
    #[allow(unused_variables)]
    pub fn register_texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureId> {
        let _renderer = self.renderer.as_ref()?;

        // For now, return None - proper egui texture integration requires
        // accessing eframe's wgpu render state which we don't have here.
        // The integration will be done in the ride screen.
        None
    }

    /// Get current ride stats for HUD display
    pub fn get_stats(&self) -> WorldStats {
        self.stats.clone()
    }

    /// Get HUD reference for formatting
    pub fn hud(&self) -> &Hud {
        &self.hud
    }

    /// Get route progress (0.0 - 1.0)
    pub fn get_route_progress(&self) -> f32 {
        if self.route.total_distance > 0.0 {
            self.avatar.distance_traveled() / self.route.total_distance
        } else {
            0.0
        }
    }

    /// Get the current world definition
    pub fn world_definition(&self) -> &WorldDefinition {
        &self.world_def
    }

    /// Get the current route
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// Resize the render target
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;

        if let Some(ref mut renderer) = self.renderer {
            renderer.resize(width, height);
        }
    }

    /// Reset the avatar to the start of the route
    pub fn reset(&mut self) {
        self.avatar.reset();
        self.stats = WorldStats::default();
    }

    /// Set rider mass (from settings)
    pub fn set_rider_mass(&mut self, mass_kg: f32) {
        self.physics.set_rider_mass(mass_kg);
    }

    /// Check if world is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Get the output texture for reading pixels (for integration tests)
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        self.renderer.as_ref().map(|r| r.output_texture())
    }

    /// Create a World3D from an imported route (T042)
    ///
    /// This creates a 3D world using the route data from database storage,
    /// automatically generating appropriate terrain based on the route's
    /// elevation profile.
    pub fn from_imported_route(
        stored_route: &StoredRoute,
        waypoints: &[StoredWaypoint],
        avatar_config: AvatarConfig,
        rider_mass_kg: f32,
    ) -> Result<Self, WorldError> {
        if waypoints.is_empty() {
            return Err(WorldError::RouteNotFound(
                "Route has no waypoints".to_string(),
            ));
        }

        // Convert stored waypoints to route waypoints with 3D positions
        let route_waypoints = Self::convert_waypoints_to_3d(waypoints);

        // Create the Route
        let route = Route {
            id: stored_route.id.to_string(),
            name: stored_route.name.clone(),
            total_distance: stored_route.distance_meters as f32,
            waypoints: route_waypoints,
            elevation_profile: Self::extract_elevation_profile(waypoints),
        };

        // Determine terrain style from route characteristics
        let terrain_style = TerrainStyle::from_elevation_profile(
            stored_route.elevation_gain_meters,
            stored_route.max_elevation_meters,
            stored_route.avg_gradient_percent,
        );

        // Create world definition based on terrain style
        let world_def = Self::create_world_def_from_terrain(
            &stored_route.name,
            terrain_style,
            stored_route.distance_meters,
        );

        // Create terrain with appropriate style colors
        let terrain = Terrain {
            size: (stored_route.distance_meters as f32 * 1.5).max(2000.0),
            base_color: terrain_style.base_color(),
        };

        // Create avatar
        let avatar = Avatar::new(avatar_config);

        // Create physics engine
        let physics = PhysicsEngine::new(rider_mass_kg);

        // Create scene with terrain style lighting
        let mut scene = Scene::new();
        scene.lighting.ambient_color = terrain_style.sky_tint() * terrain_style.ambient_intensity();
        scene.sky.top_color = terrain_style.sky_tint();
        scene.sky.horizon_color = terrain_style.sky_tint() * 1.2;

        // Create camera
        let camera = Camera::default();

        // Create road
        let road = Road::default();

        // Create HUD
        let hud = Hud::new();

        Ok(Self {
            renderer: None,
            scene,
            camera,
            avatar,
            physics,
            route,
            terrain,
            road,
            hud,
            stats: WorldStats::default(),
            world_def,
            active: false,
            width: 800,
            height: 600,
        })
    }

    /// Convert stored waypoints to 3D route waypoints using GPS to world coordinate conversion
    fn convert_waypoints_to_3d(waypoints: &[StoredWaypoint]) -> Vec<Waypoint> {
        if waypoints.is_empty() {
            return Vec::new();
        }

        // Use first waypoint as origin
        let origin_lat = waypoints[0].latitude;
        let origin_lon = waypoints[0].longitude;

        waypoints
            .iter()
            .map(|wp| {
                // Convert GPS to local 3D coordinates
                let (x, z) =
                    import::gps_to_world_coords(wp.latitude, wp.longitude, origin_lat, origin_lon);

                Waypoint {
                    position: Vec3::new(x, wp.elevation_meters, z),
                    distance_from_start: wp.distance_from_start,
                    gradient_percent: wp.gradient_percent,
                    surface_type: wp.surface_type,
                }
            })
            .collect()
    }

    /// Extract elevation profile from waypoints
    fn extract_elevation_profile(waypoints: &[StoredWaypoint]) -> Vec<f32> {
        waypoints.iter().map(|wp| wp.elevation_meters).collect()
    }

    /// Create a world definition based on terrain style
    fn create_world_def_from_terrain(
        route_name: &str,
        terrain_style: TerrainStyle,
        distance_meters: f64,
    ) -> WorldDefinition {
        let theme = match terrain_style {
            TerrainStyle::Flat => WorldTheme::Countryside,
            TerrainStyle::RollingHills => WorldTheme::Countryside,
            TerrainStyle::Mountain => WorldTheme::Mountains,
            TerrainStyle::Coastal => WorldTheme::Coastal,
            TerrainStyle::Forest => WorldTheme::Countryside,
            TerrainStyle::Urban => WorldTheme::Urban,
        };

        let time_of_day = match terrain_style {
            TerrainStyle::Coastal => TimeOfDay::Morning,
            TerrainStyle::Mountain => TimeOfDay::Noon,
            TerrainStyle::Forest => TimeOfDay::Afternoon,
            _ => TimeOfDay::Morning,
        };

        WorldDefinition {
            id: format!("imported_{}", route_name.to_lowercase().replace(' ', "_")),
            name: format!("{} World", route_name),
            description: format!("Generated world for imported route: {}", route_name),
            theme,
            time_of_day,
            preview_image: String::new(), // No preview for imported routes
            assets_path: String::new(),   // No external assets
            default_route: "imported_route".to_string(),
            routes: vec![RouteDefinition {
                id: "imported_route".to_string(),
                name: route_name.to_string(),
                distance_meters: distance_meters as f32,
                elevation_gain_meters: 0.0, // Will be set from stored route
                difficulty: worlds::RouteDifficulty::Moderate,
                is_loop: false,
                waypoints_file: None,
            }],
        }
    }

    /// Get the imported route terrain data (for advanced rendering)
    pub fn get_imported_terrain(&self) -> Option<ImportedRouteTerrain> {
        // Build terrain data from current route waypoints
        if self.route.waypoints.is_empty() {
            return None;
        }

        let waypoint_data: Vec<(Vec3, f32)> = self
            .route
            .waypoints
            .iter()
            .map(|wp| (wp.position, wp.gradient_percent))
            .collect();

        let elevations: Vec<f32> = self
            .route
            .waypoints
            .iter()
            .map(|wp| wp.position.y)
            .collect();
        let elevation_gain = elevations
            .windows(2)
            .filter_map(|w| {
                let diff = w[1] - w[0];
                if diff > 0.0 {
                    Some(diff)
                } else {
                    None
                }
            })
            .sum();
        let max_elevation = elevations.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let avg_gradient = self
            .route
            .waypoints
            .iter()
            .map(|wp| wp.gradient_percent.abs())
            .sum::<f32>()
            / self.route.waypoints.len() as f32;

        Some(ImportedRouteTerrain::from_waypoints(
            &waypoint_data,
            elevation_gain,
            max_elevation,
            avg_gradient,
        ))
    }
}
