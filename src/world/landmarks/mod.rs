//! Landmark and point-of-interest system.

pub mod discovery;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Landmark types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LandmarkType {
    /// Mountain summit
    Summit,
    /// Scenic viewpoint
    Viewpoint,
    /// Town/village
    Town,
    /// Historic site
    Historic,
    /// Sprint point
    Sprint,
    /// Feed zone / refreshment
    FeedZone,
    /// Water fountain
    WaterFountain,
    /// Rest area
    RestArea,
    /// Custom user-created
    Custom,
}

impl LandmarkType {
    /// Get icon name for this landmark type
    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Summit => "summit",
            Self::Viewpoint => "viewpoint",
            Self::Town => "town",
            Self::Historic => "historic",
            Self::Sprint => "sprint",
            Self::FeedZone => "feed",
            Self::WaterFountain => "water",
            Self::RestArea => "rest",
            Self::Custom => "marker",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Summit => "Summit",
            Self::Viewpoint => "Viewpoint",
            Self::Town => "Town",
            Self::Historic => "Historic Site",
            Self::Sprint => "Sprint",
            Self::FeedZone => "Feed Zone",
            Self::WaterFountain => "Water",
            Self::RestArea => "Rest Area",
            Self::Custom => "Custom",
        }
    }
}

/// A landmark/POI on a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    /// Unique identifier
    pub id: Uuid,
    /// Route this landmark belongs to
    pub route_id: Option<Uuid>,
    /// Type of landmark
    pub landmark_type: LandmarkType,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// GPS latitude
    pub latitude: f64,
    /// GPS longitude
    pub longitude: f64,
    /// Elevation in meters
    pub elevation_meters: f32,
    /// Distance from route start (if on route)
    pub distance_meters: Option<f64>,
    /// When created
    pub created_at: DateTime<Utc>,
}

impl Landmark {
    /// Create a new landmark
    pub fn new(
        landmark_type: LandmarkType,
        name: String,
        latitude: f64,
        longitude: f64,
        elevation: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            route_id: None,
            landmark_type,
            name,
            description: None,
            latitude,
            longitude,
            elevation_meters: elevation,
            distance_meters: None,
            created_at: Utc::now(),
        }
    }

    /// Add to a route at a specific distance
    pub fn on_route(mut self, route_id: Uuid, distance: f64) -> Self {
        self.route_id = Some(route_id);
        self.distance_meters = Some(distance);
        self
    }

    /// Add description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
}

/// Landmark visibility settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandmarkSettings {
    /// Whether to show landmarks
    pub enabled: bool,
    /// Maximum number of visible landmarks at once
    pub max_visible: u8,
    /// Distance ahead to show landmarks (meters)
    pub preview_distance: f64,
    /// Which types to show
    pub visible_types: Vec<LandmarkType>,
    /// Whether to announce landmarks
    pub audio_announcements: bool,
}

impl Default for LandmarkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_visible: 5,
            preview_distance: 500.0,
            visible_types: vec![
                LandmarkType::Summit,
                LandmarkType::Viewpoint,
                LandmarkType::Town,
                LandmarkType::Sprint,
            ],
            audio_announcements: true,
        }
    }
}

/// Landmark manager
pub struct LandmarkManager {
    /// All landmarks on current route
    landmarks: Vec<Landmark>,
    /// Discovered landmarks this ride
    discovered: Vec<Uuid>,
    /// Settings
    settings: LandmarkSettings,
    /// Last announced landmark
    last_announced: Option<Uuid>,
}

impl LandmarkManager {
    /// Create landmark manager
    pub fn new(settings: LandmarkSettings) -> Self {
        Self {
            landmarks: Vec::new(),
            discovered: Vec::new(),
            settings,
            last_announced: None,
        }
    }

    /// Load landmarks for a route
    pub fn load_route(&mut self, landmarks: Vec<Landmark>) {
        self.landmarks = landmarks;
        self.discovered.clear();
        self.last_announced = None;
    }

    /// Get visible landmarks based on current position
    pub fn visible(&self, current_distance: f64) -> Vec<&Landmark> {
        if !self.settings.enabled {
            return Vec::new();
        }

        let max_distance = current_distance + self.settings.preview_distance;

        self.landmarks
            .iter()
            .filter(|lm| {
                if let Some(dist) = lm.distance_meters {
                    dist >= current_distance
                        && dist <= max_distance
                        && self.settings.visible_types.contains(&lm.landmark_type)
                } else {
                    false
                }
            })
            .take(self.settings.max_visible as usize)
            .collect()
    }

    /// Get upcoming landmark (if any)
    pub fn next_landmark(&self, current_distance: f64) -> Option<&Landmark> {
        self.landmarks
            .iter()
            .filter(|lm| {
                lm.distance_meters
                    .map(|d| d > current_distance)
                    .unwrap_or(false)
            })
            .min_by(|a, b| a.distance_meters.partial_cmp(&b.distance_meters).unwrap())
    }

    /// Check if user reached a landmark (within 10m)
    pub fn check_discovery(&mut self, current_distance: f64) -> Option<&Landmark> {
        let discovery_radius = 10.0;

        for landmark in &self.landmarks {
            if let Some(lm_distance) = landmark.distance_meters {
                let distance_to = (lm_distance - current_distance).abs();

                if distance_to <= discovery_radius && !self.discovered.contains(&landmark.id) {
                    self.discovered.push(landmark.id);
                    return Some(landmark);
                }
            }
        }

        None
    }

    /// Get all discovered landmarks this ride
    pub fn discovered(&self) -> Vec<&Landmark> {
        self.landmarks
            .iter()
            .filter(|lm| self.discovered.contains(&lm.id))
            .collect()
    }

    /// Get discovery count
    pub fn discovery_count(&self) -> (usize, usize) {
        (self.discovered.len(), self.landmarks.len())
    }

    /// Reset for new ride
    pub fn reset(&mut self) {
        self.discovered.clear();
        self.last_announced = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landmark_creation() {
        let lm = Landmark::new(
            LandmarkType::Summit,
            "Alpe d'Huez".to_string(),
            45.0913,
            6.0714,
            1850.0,
        );

        assert_eq!(lm.landmark_type, LandmarkType::Summit);
        assert!(lm.route_id.is_none());
    }

    #[test]
    fn test_landmark_manager_visibility() {
        let mut manager = LandmarkManager::new(LandmarkSettings::default());

        let lm1 = Landmark::new(LandmarkType::Summit, "Peak 1".to_string(), 0.0, 0.0, 1000.0)
            .on_route(Uuid::new_v4(), 1000.0);

        let lm2 = Landmark::new(LandmarkType::Summit, "Peak 2".to_string(), 0.0, 0.0, 2000.0)
            .on_route(Uuid::new_v4(), 5000.0);

        manager.load_route(vec![lm1, lm2]);

        // At distance 800m, should see Peak 1 (at 1000m, within 500m preview)
        let visible = manager.visible(800.0);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "Peak 1");
    }
}
