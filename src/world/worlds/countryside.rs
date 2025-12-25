//! Countryside world definition

use super::{RouteDefinition, RouteDifficulty, TimeOfDay, WorldDefinition, WorldTheme};

/// Get the countryside world definition
pub fn get_definition() -> WorldDefinition {
    WorldDefinition {
        id: "countryside".to_string(),
        name: "Rolling Countryside".to_string(),
        description:
            "Gentle hills and pastoral scenery with farms, forests, and quiet country roads."
                .to_string(),
        theme: WorldTheme::Countryside,
        preview_image: "assets/worlds/countryside/preview.png".to_string(),
        assets_path: "assets/worlds/countryside/".to_string(),
        default_route: "farm_loop".to_string(),
        time_of_day: TimeOfDay::Morning,
        routes: vec![
            RouteDefinition {
                id: "farm_loop".to_string(),
                name: "Farm Loop".to_string(),
                distance_meters: 12500.0,
                elevation_gain_meters: 120.0,
                difficulty: RouteDifficulty::Easy,
                is_loop: true,
                waypoints_file: Some("routes/farm_loop.json".to_string()),
            },
            RouteDefinition {
                id: "village_tour".to_string(),
                name: "Village Tour".to_string(),
                distance_meters: 25000.0,
                elevation_gain_meters: 280.0,
                difficulty: RouteDifficulty::Moderate,
                is_loop: true,
                waypoints_file: Some("routes/village_tour.json".to_string()),
            },
            RouteDefinition {
                id: "forest_path".to_string(),
                name: "Forest Path".to_string(),
                distance_meters: 8000.0,
                elevation_gain_meters: 150.0,
                difficulty: RouteDifficulty::Easy,
                is_loop: false,
                waypoints_file: Some("routes/forest_path.json".to_string()),
            },
        ],
    }
}
