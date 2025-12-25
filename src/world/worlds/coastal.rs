//! Coastal world definition

use super::{RouteDefinition, RouteDifficulty, TimeOfDay, WorldDefinition, WorldTheme};

/// Get the coastal world definition
pub fn get_definition() -> WorldDefinition {
    WorldDefinition {
        id: "coastal".to_string(),
        name: "Coastal Paradise".to_string(),
        description: "Scenic ocean views with palm trees, beaches, and warm coastal breezes."
            .to_string(),
        theme: WorldTheme::Coastal,
        preview_image: "assets/worlds/coastal/preview.png".to_string(),
        assets_path: "assets/worlds/coastal/".to_string(),
        default_route: "beach_cruise".to_string(),
        time_of_day: TimeOfDay::Afternoon,
        routes: vec![
            RouteDefinition {
                id: "beach_cruise".to_string(),
                name: "Beach Cruise".to_string(),
                distance_meters: 18000.0,
                elevation_gain_meters: 50.0,
                difficulty: RouteDifficulty::Easy,
                is_loop: true,
                waypoints_file: Some("routes/beach_cruise.json".to_string()),
            },
            RouteDefinition {
                id: "cliff_road".to_string(),
                name: "Cliff Road".to_string(),
                distance_meters: 22000.0,
                elevation_gain_meters: 450.0,
                difficulty: RouteDifficulty::Moderate,
                is_loop: false,
                waypoints_file: Some("routes/cliff_road.json".to_string()),
            },
            RouteDefinition {
                id: "harbor_loop".to_string(),
                name: "Harbor Loop".to_string(),
                distance_meters: 8000.0,
                elevation_gain_meters: 30.0,
                difficulty: RouteDifficulty::Easy,
                is_loop: true,
                waypoints_file: Some("routes/harbor_loop.json".to_string()),
            },
            RouteDefinition {
                id: "lighthouse_climb".to_string(),
                name: "Lighthouse Climb".to_string(),
                distance_meters: 12000.0,
                elevation_gain_meters: 350.0,
                difficulty: RouteDifficulty::Challenging,
                is_loop: false,
                waypoints_file: Some("routes/lighthouse_climb.json".to_string()),
            },
        ],
    }
}
