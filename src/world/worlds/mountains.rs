//! Mountains world definition

use super::{RouteDefinition, RouteDifficulty, TimeOfDay, WorldDefinition, WorldTheme};

/// Get the mountains world definition
pub fn get_definition() -> WorldDefinition {
    WorldDefinition {
        id: "mountains".to_string(),
        name: "Alpine Mountains".to_string(),
        description: "Challenging climbs through alpine terrain with switchbacks, snow-capped peaks, and stunning vistas."
            .to_string(),
        theme: WorldTheme::Mountains,
        preview_image: "assets/worlds/mountains/preview.png".to_string(),
        assets_path: "assets/worlds/mountains/".to_string(),
        default_route: "valley_climb".to_string(),
        time_of_day: TimeOfDay::Morning,
        routes: vec![
            RouteDefinition {
                id: "valley_climb".to_string(),
                name: "Valley Climb".to_string(),
                distance_meters: 15000.0,
                elevation_gain_meters: 800.0,
                difficulty: RouteDifficulty::Challenging,
                is_loop: false,
                waypoints_file: Some("routes/valley_climb.json".to_string()),
            },
            RouteDefinition {
                id: "summit_challenge".to_string(),
                name: "Summit Challenge".to_string(),
                distance_meters: 20000.0,
                elevation_gain_meters: 1500.0,
                difficulty: RouteDifficulty::Extreme,
                is_loop: false,
                waypoints_file: Some("routes/summit_challenge.json".to_string()),
            },
            RouteDefinition {
                id: "meadow_loop".to_string(),
                name: "Alpine Meadow Loop".to_string(),
                distance_meters: 10000.0,
                elevation_gain_meters: 300.0,
                difficulty: RouteDifficulty::Moderate,
                is_loop: true,
                waypoints_file: Some("routes/meadow_loop.json".to_string()),
            },
        ],
    }
}
