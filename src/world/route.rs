//! Route definitions for virtual worlds
//!
//! Routes define paths through virtual worlds with waypoints,
//! distance, and elevation data.

use chrono::{DateTime, Utc};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Source of imported route data (T018)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteSource {
    /// GPX file import
    Gpx,
    /// FIT file import
    Fit,
    /// TCX file import
    Tcx,
    /// User-created route in world creator
    Custom,
    /// Pre-built famous route
    Famous,
    /// Procedurally generated
    Procedural,
}

impl std::fmt::Display for RouteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteSource::Gpx => write!(f, "gpx"),
            RouteSource::Fit => write!(f, "fit"),
            RouteSource::Tcx => write!(f, "tcx"),
            RouteSource::Custom => write!(f, "custom"),
            RouteSource::Famous => write!(f, "famous"),
            RouteSource::Procedural => write!(f, "procedural"),
        }
    }
}

impl std::str::FromStr for RouteSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gpx" => Ok(RouteSource::Gpx),
            "fit" => Ok(RouteSource::Fit),
            "tcx" => Ok(RouteSource::Tcx),
            "custom" => Ok(RouteSource::Custom),
            "famous" => Ok(RouteSource::Famous),
            "procedural" => Ok(RouteSource::Procedural),
            _ => Err(format!("Unknown route source: {}", s)),
        }
    }
}

/// Difficulty modifier for NPC speed adjustments (T024)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DifficultyModifier {
    /// Base speed multiplier (1.0 = normal)
    pub speed_multiplier: f32,
    /// Additional power percentage required for climbs
    pub climb_penalty: f32,
    /// Drafting benefit reduction (0.0 = full draft, 1.0 = no draft)
    pub draft_reduction: f32,
    /// Recovery rate modifier
    pub recovery_rate: f32,
}

impl Default for DifficultyModifier {
    fn default() -> Self {
        Self {
            speed_multiplier: 1.0,
            climb_penalty: 0.0,
            draft_reduction: 0.0,
            recovery_rate: 1.0,
        }
    }
}

impl DifficultyModifier {
    /// Create an easy difficulty modifier
    pub fn easy() -> Self {
        Self {
            speed_multiplier: 0.85,
            climb_penalty: -0.1,
            draft_reduction: 0.0,
            recovery_rate: 1.2,
        }
    }

    /// Create a normal difficulty modifier
    pub fn normal() -> Self {
        Self::default()
    }

    /// Create a hard difficulty modifier
    pub fn hard() -> Self {
        Self {
            speed_multiplier: 1.15,
            climb_penalty: 0.1,
            draft_reduction: 0.3,
            recovery_rate: 0.85,
        }
    }

    /// Create an extreme difficulty modifier
    pub fn extreme() -> Self {
        Self {
            speed_multiplier: 1.3,
            climb_penalty: 0.2,
            draft_reduction: 0.5,
            recovery_rate: 0.7,
        }
    }

    /// Apply difficulty to a base speed
    pub fn apply_to_speed(&self, base_speed: f32) -> f32 {
        base_speed * self.speed_multiplier
    }

    /// Apply difficulty to climbing power requirement
    pub fn apply_to_climb_power(&self, base_power: f32, gradient: f32) -> f32 {
        if gradient > 0.0 {
            base_power * (1.0 + self.climb_penalty * gradient / 10.0)
        } else {
            base_power
        }
    }

    /// Calculate effective draft benefit
    pub fn effective_draft_benefit(&self, base_benefit: f32) -> f32 {
        base_benefit * (1.0 - self.draft_reduction)
    }
}

// ========== T096-T097: Gradient Scaling & Adaptive Difficulty ==========

/// Gradient scaling mode for route difficulty adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GradientScalingMode {
    /// No scaling - original gradients
    #[default]
    Original,
    /// Fixed percentage scaling (e.g., 50% = half gradient)
    Fixed,
    /// Adaptive scaling based on FTP
    Adaptive,
}

/// T096-T097: Gradient scaler for route difficulty adjustment
///
/// This allows users to modify route gradients to make climbs easier or harder.
/// Can use fixed percentage scaling or adaptive scaling based on FTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientScaler {
    /// Scaling mode
    pub mode: GradientScalingMode,
    /// Fixed scale factor (0.0 to 2.0, where 1.0 = original, 0.5 = half, 2.0 = double)
    pub fixed_scale: f32,
    /// User's FTP for adaptive scaling (watts)
    pub user_ftp: u16,
    /// Target FTP for adaptive scaling (watts) - gradients scaled to feel like this FTP
    pub target_ftp: u16,
    /// Minimum gradient after scaling (prevents weird flat sections)
    pub min_gradient: f32,
    /// Maximum gradient after scaling (prevents unrealistic steepness)
    pub max_gradient: f32,
}

impl Default for GradientScaler {
    fn default() -> Self {
        Self {
            mode: GradientScalingMode::Original,
            fixed_scale: 1.0,
            user_ftp: 200,
            target_ftp: 200,
            min_gradient: -20.0,
            max_gradient: 25.0,
        }
    }
}

impl GradientScaler {
    /// Create a scaler with 50% gradient reduction
    pub fn half_gradient() -> Self {
        Self {
            mode: GradientScalingMode::Fixed,
            fixed_scale: 0.5,
            ..Default::default()
        }
    }

    /// Create a scaler with double gradients
    pub fn double_gradient() -> Self {
        Self {
            mode: GradientScalingMode::Fixed,
            fixed_scale: 2.0,
            ..Default::default()
        }
    }

    /// Create an adaptive scaler based on FTP
    ///
    /// Routes will feel like the user has `target_ftp` when they actually have `user_ftp`.
    /// For example, a 250W rider who sets target_ftp to 300W will experience easier gradients.
    pub fn adaptive(user_ftp: u16, target_ftp: u16) -> Self {
        Self {
            mode: GradientScalingMode::Adaptive,
            user_ftp,
            target_ftp,
            ..Default::default()
        }
    }

    /// Apply scaling to a gradient value
    ///
    /// Returns the scaled gradient, clamped to min/max bounds.
    pub fn scale_gradient(&self, original_gradient: f32) -> f32 {
        let scaled = match self.mode {
            GradientScalingMode::Original => original_gradient,
            GradientScalingMode::Fixed => original_gradient * self.fixed_scale,
            GradientScalingMode::Adaptive => {
                // Scale based on FTP ratio
                // If user has lower FTP than target, reduce gradients
                // If user has higher FTP than target, increase gradients
                if self.user_ftp == 0 || self.target_ftp == 0 {
                    original_gradient
                } else {
                    let ratio = self.target_ftp as f32 / self.user_ftp as f32;
                    original_gradient * ratio
                }
            }
        };

        // Clamp to reasonable bounds
        scaled.clamp(self.min_gradient, self.max_gradient)
    }

    /// Apply scaling to an entire elevation profile
    ///
    /// Takes (distance, elevation) pairs and returns modified elevations.
    pub fn scale_elevation_profile(&self, profile: &[(f64, f32)]) -> Vec<(f64, f32)> {
        if profile.is_empty() || matches!(self.mode, GradientScalingMode::Original) {
            return profile.to_vec();
        }

        let mut result = Vec::with_capacity(profile.len());
        let mut current_elevation = profile[0].1;
        result.push((profile[0].0, current_elevation));

        for i in 1..profile.len() {
            let (distance, _original_elevation) = profile[i];
            let prev_distance = profile[i - 1].0;
            let delta_distance = distance - prev_distance;

            if delta_distance > 0.0 {
                // Calculate original gradient
                let original_delta_elevation = profile[i].1 - profile[i - 1].1;
                let original_gradient =
                    (original_delta_elevation as f64 / delta_distance * 100.0) as f32;

                // Apply scaling to gradient
                let scaled_gradient = self.scale_gradient(original_gradient);

                // Calculate new elevation from scaled gradient
                let scaled_delta_elevation = (scaled_gradient / 100.0) * delta_distance as f32;
                current_elevation += scaled_delta_elevation;
            }

            result.push((distance, current_elevation));
        }

        result
    }

    /// Get the effective scale factor for display purposes
    pub fn effective_scale(&self) -> f32 {
        match self.mode {
            GradientScalingMode::Original => 1.0,
            GradientScalingMode::Fixed => self.fixed_scale,
            GradientScalingMode::Adaptive => {
                if self.user_ftp == 0 {
                    1.0
                } else {
                    self.target_ftp as f32 / self.user_ftp as f32
                }
            }
        }
    }

    /// Get display description
    pub fn description(&self) -> String {
        match self.mode {
            GradientScalingMode::Original => "Original gradients".to_string(),
            GradientScalingMode::Fixed => {
                format!("{}% gradient", (self.fixed_scale * 100.0) as i32)
            }
            GradientScalingMode::Adaptive => {
                format!("Adaptive ({}W → {}W)", self.user_ftp, self.target_ftp)
            }
        }
    }
}

// ========== T101-T104: Route Recommendation System ==========

/// Training goal type for route recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrainingGoalType {
    /// Endurance/base building - long, low-intensity
    Endurance,
    /// Climbing strength - elevation-focused
    Climbing,
    /// Speed/power - flat, high-intensity
    Speed,
    /// Recovery ride - short, easy
    Recovery,
    /// Interval training - varied terrain
    Intervals,
}

/// Criteria for route recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationCriteria {
    /// Training goal type
    pub goal: TrainingGoalType,
    /// Available time in minutes
    pub available_time_minutes: u32,
    /// User's FTP for intensity calculations
    pub user_ftp: u16,
    /// User's average speed (km/h) for time estimation
    pub avg_speed_kmh: f32,
    /// Prefer routes not ridden recently
    pub prefer_variety: bool,
    /// Maximum gradient user can handle (based on skill)
    pub max_comfortable_gradient: f32,
}

impl Default for RecommendationCriteria {
    fn default() -> Self {
        Self {
            goal: TrainingGoalType::Endurance,
            available_time_minutes: 60,
            user_ftp: 200,
            avg_speed_kmh: 25.0,
            prefer_variety: true,
            max_comfortable_gradient: 12.0,
        }
    }
}

/// A route recommendation with match score and reasoning
#[derive(Debug, Clone)]
pub struct RouteRecommendation {
    /// The recommended route
    pub route: StoredRoute,
    /// Match score (0.0 to 1.0, higher is better)
    pub score: f32,
    /// Estimated duration in minutes
    pub estimated_duration_minutes: u32,
    /// Why this route was recommended
    pub reasons: Vec<String>,
    /// Whether this was recently ridden
    pub recently_ridden: bool,
}

/// Route recommendation engine
#[derive(Debug, Clone, Default)]
pub struct RouteRecommender {
    /// Recently ridden route IDs (for variety scoring)
    pub recently_ridden: Vec<Uuid>,
}

impl RouteRecommender {
    /// Create a new recommender
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a recently ridden route for variety scoring
    pub fn mark_ridden(&mut self, route_id: Uuid) {
        if !self.recently_ridden.contains(&route_id) {
            self.recently_ridden.push(route_id);
            // Keep only last 20 rides
            if self.recently_ridden.len() > 20 {
                self.recently_ridden.remove(0);
            }
        }
    }

    /// Get route recommendations based on criteria
    pub fn recommend(
        &self,
        routes: &[StoredRoute],
        criteria: &RecommendationCriteria,
    ) -> Vec<RouteRecommendation> {
        let mut recommendations: Vec<RouteRecommendation> = routes
            .iter()
            .map(|route| self.score_route(route, criteria))
            .filter(|rec| rec.score > 0.1) // Filter out very poor matches
            .collect();

        // Sort by score (highest first)
        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Return top 10 recommendations
        recommendations.truncate(10);
        recommendations
    }

    /// Score a single route against criteria
    fn score_route(
        &self,
        route: &StoredRoute,
        criteria: &RecommendationCriteria,
    ) -> RouteRecommendation {
        let mut score = 0.0_f32;
        let mut reasons = Vec::new();

        // Estimate duration based on distance and average speed
        let distance_km = route.distance_meters / 1000.0;
        let climbing_factor = 1.0 + (route.elevation_gain_meters / 1000.0 * 0.1); // 10% slower per 1000m climbing
        let estimated_hours = (distance_km as f32 / criteria.avg_speed_kmh) * climbing_factor;
        let estimated_duration = (estimated_hours * 60.0) as u32;

        // T103: Time-based filtering
        let time_diff = (estimated_duration as i32 - criteria.available_time_minutes as i32).abs();
        let time_tolerance = criteria.available_time_minutes as f32 * 0.2; // 20% tolerance
        if time_diff as f32 <= time_tolerance {
            score += 0.3;
            reasons.push("Good time fit".to_string());
        } else if time_diff as f32 <= time_tolerance * 2.0 {
            score += 0.15;
            reasons.push("Acceptable duration".to_string());
        }

        // T102: Goal-based matching
        match criteria.goal {
            TrainingGoalType::Endurance => {
                // Prefer longer routes with moderate gradients
                if distance_km > 50.0 {
                    score += 0.2;
                    reasons.push("Long distance for endurance".to_string());
                }
                if route.avg_gradient_percent < 5.0 {
                    score += 0.15;
                    reasons.push("Moderate terrain".to_string());
                }
            }
            TrainingGoalType::Climbing => {
                // Prefer routes with significant climbing
                if route.elevation_gain_meters > 500.0 {
                    score += 0.25;
                    reasons.push("Good climbing".to_string());
                }
                if route.max_gradient_percent > 8.0
                    && route.max_gradient_percent <= criteria.max_comfortable_gradient
                {
                    score += 0.15;
                    reasons.push("Challenging gradients".to_string());
                }
            }
            TrainingGoalType::Speed => {
                // Prefer flat routes
                if route.avg_gradient_percent < 2.0 {
                    score += 0.25;
                    reasons.push("Flat for speed work".to_string());
                }
                if route.elevation_gain_meters < 200.0 {
                    score += 0.15;
                    reasons.push("Minimal climbing".to_string());
                }
            }
            TrainingGoalType::Recovery => {
                // Prefer short, easy routes
                if distance_km < 30.0 {
                    score += 0.2;
                    reasons.push("Short distance".to_string());
                }
                if route.max_gradient_percent < 5.0 {
                    score += 0.2;
                    reasons.push("Easy gradients".to_string());
                }
            }
            TrainingGoalType::Intervals => {
                // Prefer routes with varied terrain
                let gradient_range = route.max_gradient_percent - route.avg_gradient_percent;
                if gradient_range > 3.0 {
                    score += 0.2;
                    reasons.push("Varied terrain for intervals".to_string());
                }
                if distance_km > 20.0 && distance_km < 60.0 {
                    score += 0.15;
                    reasons.push("Good length for interval work".to_string());
                }
            }
        }

        // Check gradient comfort
        if route.max_gradient_percent <= criteria.max_comfortable_gradient {
            score += 0.1;
        } else {
            score -= 0.2; // Penalize routes that are too steep
            reasons.push("May exceed comfort level".to_string());
        }

        // Variety bonus (not recently ridden)
        let recently_ridden = self.recently_ridden.contains(&route.id);
        if criteria.prefer_variety && !recently_ridden {
            score += 0.1;
            reasons.push("Fresh route".to_string());
        }

        // Famous routes get a small bonus
        if route.source == RouteSource::Famous {
            score += 0.05;
            reasons.push("Famous cycling route".to_string());
        }

        RouteRecommendation {
            route: route.clone(),
            score: score.clamp(0.0, 1.0),
            estimated_duration_minutes: estimated_duration,
            reasons,
            recently_ridden,
        }
    }

    /// Get quick recommendations by goal (simplified)
    pub fn quick_recommend(
        &self,
        routes: &[StoredRoute],
        goal: TrainingGoalType,
        time_minutes: u32,
    ) -> Vec<RouteRecommendation> {
        let criteria = RecommendationCriteria {
            goal,
            available_time_minutes: time_minutes,
            ..Default::default()
        };
        self.recommend(routes, &criteria)
    }
}

/// Stored route with metadata for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRoute {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Route source type
    pub source: RouteSource,
    /// Total distance in meters
    pub distance_meters: f64,
    /// Total elevation gain in meters
    pub elevation_gain_meters: f32,
    /// Maximum elevation in meters
    pub max_elevation_meters: f32,
    /// Minimum elevation in meters
    pub min_elevation_meters: f32,
    /// Average gradient as percentage
    pub avg_gradient_percent: f32,
    /// Maximum gradient as percentage
    pub max_gradient_percent: f32,
    /// Original source file path (if imported)
    pub source_file: Option<String>,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last modified
    pub updated_at: DateTime<Utc>,
}

impl StoredRoute {
    /// Create a new stored route
    pub fn new(name: String, source: RouteSource) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            source,
            distance_meters: 0.0,
            elevation_gain_meters: 0.0,
            max_elevation_meters: 0.0,
            min_elevation_meters: 0.0,
            avg_gradient_percent: 0.0,
            max_gradient_percent: 0.0,
            source_file: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update route statistics from waypoints
    pub fn update_stats(&mut self, waypoints: &[StoredWaypoint]) {
        if waypoints.is_empty() {
            return;
        }

        let mut total_gain = 0.0f32;
        let mut max_elev = f32::MIN;
        let mut min_elev = f32::MAX;
        let mut max_gradient = 0.0f32;
        let mut total_gradient = 0.0f32;

        for (i, wp) in waypoints.iter().enumerate() {
            max_elev = max_elev.max(wp.elevation_meters);
            min_elev = min_elev.min(wp.elevation_meters);
            max_gradient = max_gradient.max(wp.gradient_percent.abs());
            total_gradient += wp.gradient_percent.abs();

            if i > 0 {
                let elev_diff = wp.elevation_meters - waypoints[i - 1].elevation_meters;
                if elev_diff > 0.0 {
                    total_gain += elev_diff;
                }
            }
        }

        if let Some(last) = waypoints.last() {
            self.distance_meters = last.distance_from_start as f64;
        }

        self.elevation_gain_meters = total_gain;
        self.max_elevation_meters = max_elev;
        self.min_elevation_meters = min_elev;
        self.max_gradient_percent = max_gradient;
        self.avg_gradient_percent = if !waypoints.is_empty() {
            total_gradient / waypoints.len() as f32
        } else {
            0.0
        };
        self.updated_at = Utc::now();
    }
}

/// Stored waypoint for database persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredWaypoint {
    /// Unique identifier
    pub id: Uuid,
    /// Route this waypoint belongs to
    pub route_id: Uuid,
    /// Sequence number (0-indexed)
    pub sequence: u32,
    /// Latitude in degrees
    pub latitude: f64,
    /// Longitude in degrees
    pub longitude: f64,
    /// Elevation in meters
    pub elevation_meters: f32,
    /// Distance from route start in meters
    pub distance_from_start: f32,
    /// Gradient at this point as percentage
    pub gradient_percent: f32,
    /// Surface type at this point
    pub surface_type: SurfaceType,
}

impl StoredWaypoint {
    /// Create a new stored waypoint
    pub fn new(
        route_id: Uuid,
        sequence: u32,
        latitude: f64,
        longitude: f64,
        elevation_meters: f32,
        distance_from_start: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            route_id,
            sequence,
            latitude,
            longitude,
            elevation_meters,
            distance_from_start,
            gradient_percent: 0.0,
            surface_type: SurfaceType::Asphalt,
        }
    }

    /// Set the gradient
    pub fn with_gradient(mut self, gradient: f32) -> Self {
        self.gradient_percent = gradient;
        self
    }

    /// Set the surface type
    pub fn with_surface(mut self, surface: SurfaceType) -> Self {
        self.surface_type = surface;
        self
    }
}

/// A point along a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    /// 3D position (x, y, z where z is elevation)
    pub position: Vec3,
    /// Distance from route start in meters
    pub distance_from_start: f32,
    /// Gradient at this point as a percentage
    pub gradient_percent: f32,
    /// Road surface type
    #[serde(default)]
    pub surface_type: SurfaceType,
}

/// Road surface types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceType {
    #[default]
    Asphalt,
    Concrete,
    Cobblestone,
    Gravel,
    Dirt,
}

/// A complete route through a virtual world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Route identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Total route distance in meters
    pub total_distance: f32,
    /// Ordered list of waypoints
    pub waypoints: Vec<Waypoint>,
    /// Elevation profile (elevation values at regular intervals)
    #[serde(default)]
    pub elevation_profile: Vec<f32>,
}

impl Route {
    /// Get position and heading at a given distance along the route
    ///
    /// # Arguments
    /// * `distance` - Distance from start in meters
    ///
    /// # Returns
    /// Tuple of (position, heading_radians)
    pub fn get_position(&self, distance: f32) -> (Vec3, f32) {
        if self.waypoints.is_empty() {
            return (Vec3::ZERO, 0.0);
        }

        // Clamp distance to route bounds
        let distance = distance.clamp(0.0, self.total_distance);

        // Find the waypoint segment we're on
        let mut prev_wp = &self.waypoints[0];
        for wp in &self.waypoints[1..] {
            if wp.distance_from_start >= distance {
                // Interpolate between prev_wp and wp
                let segment_length = wp.distance_from_start - prev_wp.distance_from_start;
                if segment_length > 0.0 {
                    let t = (distance - prev_wp.distance_from_start) / segment_length;
                    let position = prev_wp.position.lerp(wp.position, t);

                    // Calculate heading from direction
                    let direction = wp.position - prev_wp.position;
                    let heading = direction.z.atan2(direction.x);

                    return (position, heading);
                }
                return (prev_wp.position, 0.0);
            }
            prev_wp = wp;
        }

        // Past the end of the route
        let last = self.waypoints.last().unwrap();
        (last.position, 0.0)
    }

    /// Get gradient at a given distance along the route
    pub fn get_gradient(&self, distance: f32) -> f32 {
        if self.waypoints.is_empty() {
            return 0.0;
        }

        let distance = distance.clamp(0.0, self.total_distance);

        // Find the waypoint we're at or past
        for wp in &self.waypoints {
            if wp.distance_from_start >= distance {
                return wp.gradient_percent;
            }
        }

        self.waypoints
            .last()
            .map(|w| w.gradient_percent)
            .unwrap_or(0.0)
    }

    /// Get elevation at a given distance along the route
    pub fn get_elevation(&self, distance: f32) -> f32 {
        let (position, _) = self.get_position(distance);
        position.y // Y is typically elevation in our coordinate system
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route() -> Route {
        Route {
            id: "test".to_string(),
            name: "Test Route".to_string(),
            total_distance: 1000.0,
            waypoints: vec![
                Waypoint {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    distance_from_start: 0.0,
                    gradient_percent: 0.0,
                    surface_type: SurfaceType::Asphalt,
                },
                Waypoint {
                    position: Vec3::new(500.0, 10.0, 0.0),
                    distance_from_start: 500.0,
                    gradient_percent: 2.0,
                    surface_type: SurfaceType::Asphalt,
                },
                Waypoint {
                    position: Vec3::new(1000.0, 0.0, 0.0),
                    distance_from_start: 1000.0,
                    gradient_percent: -2.0,
                    surface_type: SurfaceType::Asphalt,
                },
            ],
            elevation_profile: vec![],
        }
    }

    #[test]
    fn test_get_position_start() {
        let route = create_test_route();
        let (pos, _) = route.get_position(0.0);
        assert_eq!(pos, Vec3::ZERO);
    }

    #[test]
    fn test_get_position_middle() {
        let route = create_test_route();
        let (pos, _) = route.get_position(250.0);
        assert!(pos.x > 0.0 && pos.x < 500.0);
    }

    #[test]
    fn test_get_gradient() {
        let route = create_test_route();
        assert_eq!(route.get_gradient(0.0), 0.0);
        assert_eq!(route.get_gradient(500.0), 2.0);
    }

    #[test]
    fn test_route_source_display() {
        assert_eq!(RouteSource::Gpx.to_string(), "gpx");
        assert_eq!(RouteSource::Fit.to_string(), "fit");
        assert_eq!(RouteSource::Tcx.to_string(), "tcx");
        assert_eq!(RouteSource::Custom.to_string(), "custom");
        assert_eq!(RouteSource::Famous.to_string(), "famous");
        assert_eq!(RouteSource::Procedural.to_string(), "procedural");
    }

    #[test]
    fn test_route_source_from_str() {
        assert_eq!("gpx".parse::<RouteSource>().unwrap(), RouteSource::Gpx);
        assert_eq!("GPX".parse::<RouteSource>().unwrap(), RouteSource::Gpx);
        assert_eq!("fit".parse::<RouteSource>().unwrap(), RouteSource::Fit);
        assert_eq!("tcx".parse::<RouteSource>().unwrap(), RouteSource::Tcx);
        assert!("invalid".parse::<RouteSource>().is_err());
    }

    #[test]
    fn test_difficulty_modifier_defaults() {
        let normal = DifficultyModifier::normal();
        assert_eq!(normal.speed_multiplier, 1.0);
        assert_eq!(normal.climb_penalty, 0.0);
        assert_eq!(normal.draft_reduction, 0.0);
        assert_eq!(normal.recovery_rate, 1.0);
    }

    #[test]
    fn test_difficulty_modifier_presets() {
        let easy = DifficultyModifier::easy();
        let hard = DifficultyModifier::hard();
        let extreme = DifficultyModifier::extreme();

        assert!(easy.speed_multiplier < 1.0);
        assert!(hard.speed_multiplier > 1.0);
        assert!(extreme.speed_multiplier > hard.speed_multiplier);
    }

    #[test]
    fn test_difficulty_apply_to_speed() {
        let hard = DifficultyModifier::hard();
        let base_speed = 10.0;
        let adjusted = hard.apply_to_speed(base_speed);
        assert!(adjusted > base_speed);
    }

    #[test]
    fn test_difficulty_apply_to_climb_power() {
        let hard = DifficultyModifier::hard();
        let base_power = 200.0;

        // On flat, no extra power needed
        let flat_power = hard.apply_to_climb_power(base_power, 0.0);
        assert_eq!(flat_power, base_power);

        // On climb, extra power needed
        let climb_power = hard.apply_to_climb_power(base_power, 10.0);
        assert!(climb_power > base_power);
    }

    #[test]
    fn test_stored_route_new() {
        let route = StoredRoute::new("Test Route".to_string(), RouteSource::Gpx);
        assert_eq!(route.name, "Test Route");
        assert_eq!(route.source, RouteSource::Gpx);
        assert_eq!(route.distance_meters, 0.0);
    }

    #[test]
    fn test_stored_route_update_stats() {
        let mut route = StoredRoute::new("Test Route".to_string(), RouteSource::Gpx);
        let route_id = route.id;

        let waypoints = vec![
            StoredWaypoint::new(route_id, 0, 0.0, 0.0, 100.0, 0.0).with_gradient(0.0),
            StoredWaypoint::new(route_id, 1, 0.001, 0.001, 150.0, 500.0).with_gradient(10.0),
            StoredWaypoint::new(route_id, 2, 0.002, 0.002, 120.0, 1000.0).with_gradient(-6.0),
        ];

        route.update_stats(&waypoints);

        assert_eq!(route.distance_meters, 1000.0);
        assert_eq!(route.elevation_gain_meters, 50.0); // Only uphill counts
        assert_eq!(route.max_elevation_meters, 150.0);
        assert_eq!(route.min_elevation_meters, 100.0);
        assert_eq!(route.max_gradient_percent, 10.0);
    }

    #[test]
    fn test_stored_waypoint_new() {
        let route_id = Uuid::new_v4();
        let wp = StoredWaypoint::new(route_id, 0, 45.5, -122.6, 100.0, 0.0);

        assert_eq!(wp.route_id, route_id);
        assert_eq!(wp.sequence, 0);
        assert_eq!(wp.latitude, 45.5);
        assert_eq!(wp.longitude, -122.6);
        assert_eq!(wp.elevation_meters, 100.0);
        assert_eq!(wp.surface_type, SurfaceType::Asphalt);
    }

    #[test]
    fn test_stored_waypoint_builder() {
        let route_id = Uuid::new_v4();
        let wp = StoredWaypoint::new(route_id, 0, 45.5, -122.6, 100.0, 0.0)
            .with_gradient(5.0)
            .with_surface(SurfaceType::Gravel);

        assert_eq!(wp.gradient_percent, 5.0);
        assert_eq!(wp.surface_type, SurfaceType::Gravel);
    }

    // ========== T096-T097: Gradient Scaler Tests ==========

    #[test]
    fn test_gradient_scaler_default() {
        let scaler = GradientScaler::default();
        assert_eq!(scaler.mode, GradientScalingMode::Original);
        assert_eq!(scaler.fixed_scale, 1.0);
    }

    #[test]
    fn test_gradient_scaler_original() {
        let scaler = GradientScaler::default();
        assert_eq!(scaler.scale_gradient(10.0), 10.0);
        assert_eq!(scaler.scale_gradient(-5.0), -5.0);
    }

    #[test]
    fn test_gradient_scaler_half() {
        let scaler = GradientScaler::half_gradient();
        assert_eq!(scaler.scale_gradient(10.0), 5.0);
        assert_eq!(scaler.scale_gradient(-8.0), -4.0);
    }

    #[test]
    fn test_gradient_scaler_double() {
        let scaler = GradientScaler::double_gradient();
        assert_eq!(scaler.scale_gradient(10.0), 20.0);
    }

    #[test]
    fn test_gradient_scaler_adaptive() {
        // User has 200W FTP, target 300W = 1.5x scaling
        let scaler = GradientScaler::adaptive(200, 300);
        assert_eq!(scaler.scale_gradient(10.0), 15.0);

        // User has 300W FTP, target 200W = 0.67x scaling
        let scaler2 = GradientScaler::adaptive(300, 200);
        let scaled = scaler2.scale_gradient(10.0);
        assert!((scaled - 6.67).abs() < 0.1);
    }

    #[test]
    fn test_gradient_scaler_clamping() {
        let scaler = GradientScaler::double_gradient();
        // Original 15% doubled = 30%, but max is 25%
        assert_eq!(scaler.scale_gradient(15.0), 25.0);
    }

    #[test]
    fn test_gradient_scaler_elevation_profile() {
        let profile = vec![
            (0.0, 100.0),
            (100.0, 110.0), // 10% gradient
            (200.0, 120.0), // 10% gradient
        ];

        let scaler = GradientScaler::half_gradient();
        let scaled = scaler.scale_elevation_profile(&profile);

        // Should start at same elevation
        assert_eq!(scaled[0].1, 100.0);
        // Gradients halved means half the elevation gain
        assert!((scaled[1].1 - 105.0).abs() < 0.1);
        assert!((scaled[2].1 - 110.0).abs() < 0.1);
    }

    #[test]
    fn test_gradient_scaler_description() {
        let original = GradientScaler::default();
        assert_eq!(original.description(), "Original gradients");

        let half = GradientScaler::half_gradient();
        assert_eq!(half.description(), "50% gradient");

        let adaptive = GradientScaler::adaptive(200, 300);
        assert_eq!(adaptive.description(), "Adaptive (200W → 300W)");
    }

    // ========== T101-T104: Route Recommender Tests ==========

    fn create_test_stored_routes() -> Vec<StoredRoute> {
        let now = Utc::now();
        vec![
            // Long flat endurance route
            StoredRoute {
                id: Uuid::new_v4(),
                name: "Flat Century".to_string(),
                description: Some("Long flat route".to_string()),
                source: RouteSource::Famous,
                distance_meters: 100_000.0,
                elevation_gain_meters: 200.0,
                max_elevation_meters: 150.0,
                min_elevation_meters: 100.0,
                avg_gradient_percent: 0.5,
                max_gradient_percent: 3.0,
                source_file: None,
                created_at: now,
                updated_at: now,
            },
            // Mountain climb
            StoredRoute {
                id: Uuid::new_v4(),
                name: "Mountain Pass".to_string(),
                description: Some("Steep climb".to_string()),
                source: RouteSource::Gpx,
                distance_meters: 20_000.0,
                elevation_gain_meters: 1200.0,
                max_elevation_meters: 2000.0,
                min_elevation_meters: 800.0,
                avg_gradient_percent: 8.0,
                max_gradient_percent: 15.0,
                source_file: None,
                created_at: now,
                updated_at: now,
            },
            // Recovery spin
            StoredRoute {
                id: Uuid::new_v4(),
                name: "Easy Loop".to_string(),
                description: Some("Recovery ride".to_string()),
                source: RouteSource::Custom,
                distance_meters: 15_000.0,
                elevation_gain_meters: 50.0,
                max_elevation_meters: 120.0,
                min_elevation_meters: 100.0,
                avg_gradient_percent: 0.3,
                max_gradient_percent: 2.0,
                source_file: None,
                created_at: now,
                updated_at: now,
            },
            // Interval route
            StoredRoute {
                id: Uuid::new_v4(),
                name: "Rolling Hills".to_string(),
                description: Some("Varied terrain".to_string()),
                source: RouteSource::Gpx,
                distance_meters: 40_000.0,
                elevation_gain_meters: 600.0,
                max_elevation_meters: 400.0,
                min_elevation_meters: 100.0,
                avg_gradient_percent: 3.0,
                max_gradient_percent: 10.0,
                source_file: None,
                created_at: now,
                updated_at: now,
            },
        ]
    }

    #[test]
    fn test_recommender_new() {
        let recommender = RouteRecommender::new();
        assert!(recommender.recently_ridden.is_empty());
    }

    #[test]
    fn test_recommender_mark_ridden() {
        let mut recommender = RouteRecommender::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        recommender.mark_ridden(id1);
        assert_eq!(recommender.recently_ridden.len(), 1);

        recommender.mark_ridden(id1); // Duplicate
        assert_eq!(recommender.recently_ridden.len(), 1);

        recommender.mark_ridden(id2);
        assert_eq!(recommender.recently_ridden.len(), 2);
    }

    #[test]
    fn test_recommender_mark_ridden_limit() {
        let mut recommender = RouteRecommender::new();

        // Add 25 routes (should keep only last 20)
        for _ in 0..25 {
            recommender.mark_ridden(Uuid::new_v4());
        }

        assert_eq!(recommender.recently_ridden.len(), 20);
    }

    #[test]
    fn test_recommender_endurance_goal() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let criteria = RecommendationCriteria {
            goal: TrainingGoalType::Endurance,
            available_time_minutes: 180, // 3 hours
            ..Default::default()
        };

        let recommendations = recommender.recommend(&routes, &criteria);

        // Should return some recommendations
        assert!(!recommendations.is_empty());

        // Flat Century should be in top recommendations
        let top_names: Vec<&str> = recommendations
            .iter()
            .take(2)
            .map(|r| r.route.name.as_str())
            .collect();
        assert!(top_names.contains(&"Flat Century"));
    }

    #[test]
    fn test_recommender_climbing_goal() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let criteria = RecommendationCriteria {
            goal: TrainingGoalType::Climbing,
            available_time_minutes: 90,
            max_comfortable_gradient: 15.0,
            ..Default::default()
        };

        let recommendations = recommender.recommend(&routes, &criteria);

        // Mountain Pass should be highly ranked
        let has_mountain = recommendations
            .iter()
            .take(3)
            .any(|r| r.route.name == "Mountain Pass");
        assert!(has_mountain);
    }

    #[test]
    fn test_recommender_recovery_goal() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let criteria = RecommendationCriteria {
            goal: TrainingGoalType::Recovery,
            available_time_minutes: 45,
            ..Default::default()
        };

        let recommendations = recommender.recommend(&routes, &criteria);

        // Easy Loop should be top recommendation
        assert!(!recommendations.is_empty());
        assert_eq!(recommendations[0].route.name, "Easy Loop");
    }

    #[test]
    fn test_recommender_variety_bonus() {
        let mut recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        // Mark Easy Loop as recently ridden
        let easy_loop_id = routes
            .iter()
            .find(|r| r.name == "Easy Loop")
            .map(|r| r.id)
            .unwrap();
        recommender.mark_ridden(easy_loop_id);

        let criteria = RecommendationCriteria {
            goal: TrainingGoalType::Recovery,
            available_time_minutes: 45,
            prefer_variety: true,
            ..Default::default()
        };

        let recommendations = recommender.recommend(&routes, &criteria);

        // Recently ridden route should be marked
        let easy_rec = recommendations.iter().find(|r| r.route.name == "Easy Loop");
        if let Some(rec) = easy_rec {
            assert!(rec.recently_ridden);
        }
    }

    #[test]
    fn test_recommender_quick_recommend() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let recommendations = recommender.quick_recommend(&routes, TrainingGoalType::Speed, 60);

        // Should return recommendations
        assert!(!recommendations.is_empty());

        // Flat routes should be preferred for speed
        let flat_in_top = recommendations
            .iter()
            .take(2)
            .any(|r| r.route.avg_gradient_percent < 2.0);
        assert!(flat_in_top);
    }

    #[test]
    fn test_recommender_score_clamping() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let criteria = RecommendationCriteria::default();
        let recommendations = recommender.recommend(&routes, &criteria);

        // All scores should be in valid range
        for rec in &recommendations {
            assert!(rec.score >= 0.0 && rec.score <= 1.0);
        }
    }

    #[test]
    fn test_recommender_duration_estimate() {
        let recommender = RouteRecommender::new();
        let routes = create_test_stored_routes();

        let criteria = RecommendationCriteria {
            avg_speed_kmh: 25.0,
            ..Default::default()
        };

        let recommendations = recommender.recommend(&routes, &criteria);

        // Find Flat Century (100km)
        let century = recommendations
            .iter()
            .find(|r| r.route.name == "Flat Century");

        if let Some(rec) = century {
            // 100km at 25km/h = 4 hours = 240 minutes (with small climbing factor)
            assert!(rec.estimated_duration_minutes > 200 && rec.estimated_duration_minutes < 280);
        }
    }

    #[test]
    fn test_recommendation_criteria_default() {
        let criteria = RecommendationCriteria::default();
        assert_eq!(criteria.goal, TrainingGoalType::Endurance);
        assert_eq!(criteria.available_time_minutes, 60);
        assert_eq!(criteria.user_ftp, 200);
        assert!(criteria.prefer_variety);
    }
}
