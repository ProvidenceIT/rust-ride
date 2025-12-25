//! World definitions and loading

use serde::{Deserialize, Serialize};

use super::route::Route;
use super::WorldError;

pub mod coastal;
pub mod countryside;
pub mod mountains;

/// Difficulty rating for routes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RouteDifficulty {
    #[default]
    Easy,
    Moderate,
    Challenging,
    Extreme,
}

/// Time of day for lighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TimeOfDay {
    Dawn,
    #[default]
    Morning,
    Noon,
    Afternoon,
    Sunset,
    Night,
}

/// World theme category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorldTheme {
    #[default]
    Countryside,
    Mountains,
    Coastal,
    Urban,
    Desert,
}

/// Definition of a route within a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    /// Route identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Total distance in meters
    pub distance_meters: f32,
    /// Total elevation gain in meters
    pub elevation_gain_meters: f32,
    /// Difficulty rating
    pub difficulty: RouteDifficulty,
    /// Whether the route is a loop
    #[serde(default)]
    pub is_loop: bool,
    /// Path to waypoints file (relative to world assets)
    #[serde(default)]
    pub waypoints_file: Option<String>,
}

/// Definition of a virtual world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDefinition {
    /// World identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// World theme
    pub theme: WorldTheme,
    /// Path to preview image
    pub preview_image: String,
    /// Path to world assets directory
    pub assets_path: String,
    /// Default route ID
    pub default_route: String,
    /// Time of day setting
    #[serde(default)]
    pub time_of_day: TimeOfDay,
    /// Available routes
    pub routes: Vec<RouteDefinition>,
}

impl WorldDefinition {
    /// Get a route definition by ID
    pub fn get_route(&self, route_id: &str) -> Option<&RouteDefinition> {
        self.routes.iter().find(|r| r.id == route_id)
    }

    /// Get the default route
    pub fn get_default_route(&self) -> Option<&RouteDefinition> {
        self.get_route(&self.default_route)
    }
}

/// Load a world definition from JSON
pub fn load_world_definition(json: &str) -> Result<WorldDefinition, WorldError> {
    serde_json::from_str(json).map_err(|e| WorldError::AssetError(e.to_string()))
}

/// Get all built-in world definitions
pub fn get_builtin_worlds() -> Vec<WorldDefinition> {
    vec![
        countryside::get_definition(),
        mountains::get_definition(),
        coastal::get_definition(),
    ]
}

/// Create a basic route from a route definition (placeholder waypoints)
pub fn create_basic_route(def: &RouteDefinition) -> Route {
    use super::route::Waypoint;
    use glam::Vec3;

    // Create a simple straight route as placeholder
    let num_waypoints = 10;
    let _segment_distance = def.distance_meters / (num_waypoints as f32 - 1.0);

    let waypoints: Vec<Waypoint> = (0..num_waypoints)
        .map(|i| {
            let t = i as f32 / (num_waypoints as f32 - 1.0);
            let distance = t * def.distance_meters;
            let elevation = t * def.elevation_gain_meters * 0.5; // Gradual climb then descent

            Waypoint {
                position: Vec3::new(distance, elevation, 0.0),
                distance_from_start: distance,
                gradient_percent: if t < 0.5 {
                    (def.elevation_gain_meters / (def.distance_meters * 0.5)) * 100.0
                } else {
                    -(def.elevation_gain_meters / (def.distance_meters * 0.5)) * 100.0
                },
                surface_type: super::route::SurfaceType::Asphalt,
            }
        })
        .collect();

    Route {
        id: def.id.clone(),
        name: def.name.clone(),
        total_distance: def.distance_meters,
        waypoints,
        elevation_profile: vec![],
    }
}
