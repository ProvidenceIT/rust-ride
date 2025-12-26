//! Head-up display overlay for the 3D view
//!
//! T054: Add weather controls to HUD
//! T094: Add landmark info overlay popup to HUD
//! T109: Add drafting visual indicator to HUD
//! T147: Integrate immersion effects with HUD intensity display

use super::landmarks::{Landmark, LandmarkType};
use super::npc::DraftingState;
use super::weather::{TimeOfDay, WeatherState, WeatherType};
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
    /// Whether to show weather controls
    pub show_weather_controls: bool,
    /// Whether to show time controls
    pub show_time_controls: bool,
    /// T109: Whether to show drafting indicator
    pub show_drafting_indicator: bool,
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
            show_weather_controls: true,
            show_time_controls: true,
            show_drafting_indicator: true,
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

    /// Format weather type for display (T054)
    pub fn format_weather(&self, weather: &WeatherState) -> String {
        let weather_name = match weather.weather {
            WeatherType::Clear => "Clear",
            WeatherType::Cloudy => "Cloudy",
            WeatherType::Rain => "Rain",
            WeatherType::HeavyRain => "Heavy Rain",
            WeatherType::Fog => "Fog",
            WeatherType::Snow => "Snow",
        };

        if weather.transition_progress < 1.0 {
            format!(
                "{} ({}%)",
                weather_name,
                (weather.transition_progress * 100.0) as u32
            )
        } else {
            weather_name.to_string()
        }
    }

    /// Format time of day for display (T054)
    pub fn format_time(&self, weather: &WeatherState) -> String {
        let hours = weather.time_hours as u32;
        let minutes = ((weather.time_hours - hours as f32) * 60.0) as u32;
        format!("{:02}:{:02}", hours, minutes)
    }

    /// Format time of day period for display (T054)
    pub fn format_time_period(&self, weather: &WeatherState) -> String {
        match weather.time_of_day {
            TimeOfDay::Dawn => "Dawn".to_string(),
            TimeOfDay::Day => "Day".to_string(),
            TimeOfDay::Dusk => "Dusk".to_string(),
            TimeOfDay::Night => "Night".to_string(),
        }
    }

    /// Format visibility for display (T054)
    pub fn format_visibility(&self, weather: &WeatherState, use_imperial: bool) -> String {
        if use_imperial {
            let miles = weather.visibility_meters / 1609.34;
            if miles < 1.0 {
                let feet = weather.visibility_meters * 3.281;
                format!("{:.0} ft visibility", feet)
            } else {
                format!("{:.1} mi visibility", miles)
            }
        } else if weather.visibility_meters < 1000.0 {
            format!("{:.0} m visibility", weather.visibility_meters)
        } else {
            format!("{:.1} km visibility", weather.visibility_meters / 1000.0)
        }
    }

    /// Get available weather options for UI controls (T054)
    pub fn weather_options() -> Vec<WeatherType> {
        vec![
            WeatherType::Clear,
            WeatherType::Cloudy,
            WeatherType::Rain,
            WeatherType::HeavyRain,
            WeatherType::Fog,
            WeatherType::Snow,
        ]
    }

    /// Get weather icon/emoji for display (T054)
    pub fn weather_icon(weather: WeatherType) -> &'static str {
        match weather {
            WeatherType::Clear => "sun",
            WeatherType::Cloudy => "cloud",
            WeatherType::Rain => "rain",
            WeatherType::HeavyRain => "storm",
            WeatherType::Fog => "fog",
            WeatherType::Snow => "snow",
        }
    }

    /// Get time period icon for display (T054)
    pub fn time_icon(time_of_day: TimeOfDay) -> &'static str {
        match time_of_day {
            TimeOfDay::Dawn => "sunrise",
            TimeOfDay::Day => "sun",
            TimeOfDay::Dusk => "sunset",
            TimeOfDay::Night => "moon",
        }
    }

    // ========== T094: Landmark Info Overlay ==========

    /// Get landmark type icon for display
    pub fn landmark_icon(landmark_type: LandmarkType) -> &'static str {
        match landmark_type {
            LandmarkType::Summit => "mountain",
            LandmarkType::Viewpoint => "eye",
            LandmarkType::Town => "home",
            LandmarkType::Historic => "landmark",
            LandmarkType::Sprint => "flag",
            LandmarkType::FeedZone => "utensils",
            LandmarkType::WaterFountain => "droplet",
            LandmarkType::RestArea => "bench",
            LandmarkType::Custom => "marker",
        }
    }

    /// Get landmark type color for display (RGB normalized)
    pub fn landmark_color(landmark_type: LandmarkType) -> [f32; 3] {
        match landmark_type {
            LandmarkType::Summit => [0.8, 0.2, 0.2],        // Red
            LandmarkType::Viewpoint => [0.2, 0.6, 0.8],     // Blue
            LandmarkType::Town => [0.6, 0.4, 0.2],          // Brown
            LandmarkType::Historic => [0.5, 0.3, 0.6],      // Purple
            LandmarkType::Sprint => [0.2, 0.8, 0.2],        // Green
            LandmarkType::FeedZone => [0.9, 0.6, 0.2],      // Orange
            LandmarkType::WaterFountain => [0.3, 0.5, 0.9], // Light blue
            LandmarkType::RestArea => [0.5, 0.5, 0.5],      // Gray
            LandmarkType::Custom => [0.7, 0.7, 0.3],        // Yellow
        }
    }

    /// Format landmark info for popup overlay
    pub fn format_landmark_info(
        landmark: &Landmark,
        distance_to: f64,
        use_imperial: bool,
    ) -> LandmarkInfo {
        let distance_str = if use_imperial {
            let feet = distance_to * 3.281;
            if feet < 5280.0 {
                format!("{:.0} ft ahead", feet)
            } else {
                format!("{:.2} mi ahead", feet / 5280.0)
            }
        } else if distance_to < 1000.0 {
            format!("{:.0} m ahead", distance_to)
        } else {
            format!("{:.2} km ahead", distance_to / 1000.0)
        };

        let elevation_str = if use_imperial {
            format!("{:.0} ft", landmark.elevation_meters * 3.281)
        } else {
            format!("{:.0} m", landmark.elevation_meters)
        };

        LandmarkInfo {
            name: landmark.name.clone(),
            landmark_type: landmark.landmark_type,
            description: landmark.description.clone(),
            distance_str,
            elevation_str,
            icon: Self::landmark_icon(landmark.landmark_type),
            color: Self::landmark_color(landmark.landmark_type),
        }
    }

    /// Format landmark discovery notification
    pub fn format_discovery_notification(landmark: &Landmark) -> String {
        format!(
            "Discovered: {} ({})",
            landmark.name,
            landmark.landmark_type.display_name()
        )
    }

    // ========== T109: Drafting Indicator ==========

    /// Get drafting indicator info for HUD display
    pub fn format_drafting_indicator(drafting: &DraftingState) -> Option<DraftingIndicator> {
        if !drafting.is_drafting {
            return None;
        }

        let intensity = Self::drafting_intensity(drafting.benefit_percent);
        let color = Self::drafting_color(drafting.benefit_percent);

        Some(DraftingIndicator {
            benefit_percent: drafting.benefit_percent,
            intensity,
            color,
            total_time_seconds: drafting.total_draft_time_seconds,
            energy_saved_kj: drafting.energy_saved_kj,
        })
    }

    /// Get drafting intensity level (for visual feedback)
    fn drafting_intensity(benefit_percent: f32) -> DraftingIntensity {
        if benefit_percent >= 28.0 {
            DraftingIntensity::Optimal
        } else if benefit_percent >= 24.0 {
            DraftingIntensity::Strong
        } else if benefit_percent >= 20.0 {
            DraftingIntensity::Moderate
        } else {
            DraftingIntensity::Light
        }
    }

    /// Get drafting indicator color (RGB normalized)
    fn drafting_color(benefit_percent: f32) -> [f32; 3] {
        if benefit_percent >= 28.0 {
            [0.2, 0.9, 0.3] // Bright green - optimal position
        } else if benefit_percent >= 24.0 {
            [0.4, 0.8, 0.3] // Green - strong draft
        } else if benefit_percent >= 20.0 {
            [0.9, 0.8, 0.2] // Yellow - moderate draft
        } else {
            [0.8, 0.6, 0.2] // Orange - light draft
        }
    }

    /// Format drafting benefit for display
    pub fn format_drafting_benefit(benefit_percent: f32) -> String {
        format!("-{:.0}% effort", benefit_percent)
    }

    /// Format drafting time for display
    pub fn format_drafting_time(total_seconds: f32) -> String {
        let minutes = (total_seconds / 60.0) as u32;
        let seconds = (total_seconds % 60.0) as u32;
        if minutes > 0 {
            format!("{}:{:02} drafting", minutes, seconds)
        } else {
            format!("{} sec drafting", seconds)
        }
    }

    /// Format energy saved for display
    pub fn format_energy_saved(kj: f32) -> String {
        format!("{:.1} kJ saved", kj)
    }

    /// Get drafting icon name
    pub fn drafting_icon(intensity: DraftingIntensity) -> &'static str {
        match intensity {
            DraftingIntensity::Optimal => "draft_optimal",
            DraftingIntensity::Strong => "draft_strong",
            DraftingIntensity::Moderate => "draft_moderate",
            DraftingIntensity::Light => "draft_light",
        }
    }
}

/// Formatted landmark info for HUD display
#[derive(Debug, Clone)]
pub struct LandmarkInfo {
    /// Landmark name
    pub name: String,
    /// Type of landmark
    pub landmark_type: LandmarkType,
    /// Optional description
    pub description: Option<String>,
    /// Formatted distance string
    pub distance_str: String,
    /// Formatted elevation string
    pub elevation_str: String,
    /// Icon name for display
    pub icon: &'static str,
    /// RGB color for display
    pub color: [f32; 3],
}

/// T109: Drafting indicator for HUD display
#[derive(Debug, Clone)]
pub struct DraftingIndicator {
    /// Current drafting benefit percentage (20-30%)
    pub benefit_percent: f32,
    /// Intensity level for visual feedback
    pub intensity: DraftingIntensity,
    /// RGB color for display
    pub color: [f32; 3],
    /// Total time spent drafting this ride (seconds)
    pub total_time_seconds: f32,
    /// Estimated energy saved (kJ)
    pub energy_saved_kj: f32,
}

/// T109: Drafting intensity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftingIntensity {
    /// Light draft (20-22% benefit)
    Light,
    /// Moderate draft (22-24% benefit)
    Moderate,
    /// Strong draft (24-28% benefit)
    Strong,
    /// Optimal position (28-30% benefit)
    Optimal,
}

impl DraftingIntensity {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            DraftingIntensity::Light => "Light Draft",
            DraftingIntensity::Moderate => "Drafting",
            DraftingIntensity::Strong => "Strong Draft",
            DraftingIntensity::Optimal => "Sweet Spot!",
        }
    }
}

// ========== T147: Immersion Effect HUD Integration ==========

/// Effort intensity level for HUD display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortIntensity {
    /// Recovery/easy effort (<55% FTP)
    Easy,
    /// Endurance effort (55-75% FTP)
    Endurance,
    /// Tempo effort (75-85% FTP)
    Tempo,
    /// Hard effort (85-100% FTP)
    Hard,
    /// Threshold effort (100-120% FTP)
    Threshold,
    /// Maximum effort (>120% FTP)
    Maximum,
}

impl EffortIntensity {
    /// Get intensity from effort level (normalized to FTP)
    pub fn from_effort(effort_level: f32) -> Self {
        if effort_level >= 1.2 {
            Self::Maximum
        } else if effort_level >= 1.0 {
            Self::Threshold
        } else if effort_level >= 0.85 {
            Self::Hard
        } else if effort_level >= 0.75 {
            Self::Tempo
        } else if effort_level >= 0.55 {
            Self::Endurance
        } else {
            Self::Easy
        }
    }

    /// Get display label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Endurance => "Endurance",
            Self::Tempo => "Tempo",
            Self::Hard => "Hard",
            Self::Threshold => "Threshold",
            Self::Maximum => "MAX!",
        }
    }

    /// Get color for display (RGB normalized)
    pub fn color(&self) -> [f32; 3] {
        match self {
            Self::Easy => [0.3, 0.7, 0.3],      // Green
            Self::Endurance => [0.4, 0.6, 0.8], // Blue
            Self::Tempo => [0.9, 0.7, 0.2],     // Yellow
            Self::Hard => [0.9, 0.5, 0.2],      // Orange
            Self::Threshold => [0.9, 0.3, 0.2], // Red
            Self::Maximum => [0.8, 0.1, 0.1],   // Dark red
        }
    }

    /// Get icon for display
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Easy => "heart_relaxed",
            Self::Endurance => "heart",
            Self::Tempo => "heart_fast",
            Self::Hard => "heart_racing",
            Self::Threshold => "fire",
            Self::Maximum => "fire_intense",
        }
    }
}

/// Immersion state for HUD display
#[derive(Debug, Clone)]
pub struct ImmersionHudState {
    /// Current effort level (0.0-2.0)
    pub effort_level: f32,
    /// Current intensity category
    pub intensity: EffortIntensity,
    /// Vignette intensity (0.0-1.0)
    pub vignette_active: bool,
    /// Color grading active
    pub color_grading_active: bool,
    /// Audio active
    pub audio_active: bool,
    /// Current audio sources playing
    pub active_sounds: Vec<String>,
}

impl Default for ImmersionHudState {
    fn default() -> Self {
        Self {
            effort_level: 0.0,
            intensity: EffortIntensity::Easy,
            vignette_active: false,
            color_grading_active: false,
            audio_active: false,
            active_sounds: Vec::new(),
        }
    }
}

impl ImmersionHudState {
    /// Update from effort level
    pub fn update(&mut self, effort_level: f32, vignette: f32, color_shift: f32) {
        self.effort_level = effort_level;
        self.intensity = EffortIntensity::from_effort(effort_level);
        self.vignette_active = vignette > 0.05;
        self.color_grading_active = color_shift > 0.02;
    }

    /// Format effort percentage
    pub fn format_effort(&self) -> String {
        format!("{:.0}%", self.effort_level * 100.0)
    }
}

impl Hud {
    /// T147: Format immersion intensity for display
    pub fn format_effort_intensity(effort_level: f32) -> EffortIntensityDisplay {
        let intensity = EffortIntensity::from_effort(effort_level);
        EffortIntensityDisplay {
            effort_percent: (effort_level * 100.0) as u16,
            intensity,
            label: intensity.label(),
            color: intensity.color(),
            icon: intensity.icon(),
        }
    }

    /// Get effort intensity icon
    pub fn effort_icon(intensity: EffortIntensity) -> &'static str {
        intensity.icon()
    }

    /// Get effort intensity color
    pub fn effort_color(intensity: EffortIntensity) -> [f32; 3] {
        intensity.color()
    }
}

/// Formatted effort intensity for HUD display
#[derive(Debug, Clone)]
pub struct EffortIntensityDisplay {
    /// Effort as percentage of FTP
    pub effort_percent: u16,
    /// Intensity level
    pub intensity: EffortIntensity,
    /// Display label
    pub label: &'static str,
    /// RGB color for display
    pub color: [f32; 3],
    /// Icon name
    pub icon: &'static str,
}
