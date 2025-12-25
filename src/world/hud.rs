//! Head-up display overlay for the 3D view

use super::WorldStats;

/// HUD configuration and state
#[derive(Debug, Clone, Default)]
pub struct Hud {
    /// Whether to show speed
    pub show_speed: bool,
    /// Whether to show distance
    pub show_distance: bool,
    /// Whether to show elevation
    pub show_elevation: bool,
    /// Whether to show gradient
    pub show_gradient: bool,
    /// Whether to show route progress
    pub show_route_progress: bool,
}

impl Hud {
    /// Create a new HUD with all elements visible
    pub fn new() -> Self {
        Self {
            show_speed: true,
            show_distance: true,
            show_elevation: true,
            show_gradient: true,
            show_route_progress: true,
        }
    }

    /// Format speed for display
    pub fn format_speed(&self, stats: &WorldStats, use_imperial: bool) -> String {
        if use_imperial {
            let mph = stats.speed_mps * 2.237;
            format!("{:.1} mph", mph)
        } else {
            let kmh = stats.speed_mps * 3.6;
            format!("{:.1} km/h", kmh)
        }
    }

    /// Format distance for display
    pub fn format_distance(&self, stats: &WorldStats, use_imperial: bool) -> String {
        if use_imperial {
            let miles = stats.distance_meters / 1609.34;
            if miles < 1.0 {
                let feet = stats.distance_meters * 3.281;
                format!("{:.0} ft", feet)
            } else {
                format!("{:.2} mi", miles)
            }
        } else if stats.distance_meters < 1000.0 {
            format!("{:.0} m", stats.distance_meters)
        } else {
            format!("{:.2} km", stats.distance_meters / 1000.0)
        }
    }

    /// Format elevation for display
    pub fn format_elevation(&self, stats: &WorldStats, use_imperial: bool) -> String {
        if use_imperial {
            let feet = stats.elevation_meters * 3.281;
            format!("{:.0} ft", feet)
        } else {
            format!("{:.0} m", stats.elevation_meters)
        }
    }

    /// Format gradient for display
    pub fn format_gradient(&self, stats: &WorldStats) -> String {
        format!("{:.1}%", stats.gradient_percent)
    }

    /// Format remaining distance for display
    pub fn format_remaining(&self, stats: &WorldStats, use_imperial: bool) -> String {
        if use_imperial {
            let miles = stats.route_remaining_meters / 1609.34;
            format!("{:.2} mi to go", miles)
        } else if stats.route_remaining_meters < 1000.0 {
            format!("{:.0} m to go", stats.route_remaining_meters)
        } else {
            format!("{:.2} km to go", stats.route_remaining_meters / 1000.0)
        }
    }
}
