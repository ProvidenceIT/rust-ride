//! User profile and application configuration.
//!
//! T010: Implement Config loading from TOML
//! T016: Define UserProfile struct with FTP, zones, preferences

use crate::audio::AudioConfig;
use crate::metrics::zones::{CadenceZones, HRZones, PowerZones};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Unit system preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    /// Metric units (km/h, kg, km)
    #[default]
    Metric,
    /// Imperial units (mph, lbs, miles)
    Imperial,
}

impl std::fmt::Display for Units {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Units::Metric => write!(f, "Metric"),
            Units::Imperial => write!(f, "Imperial"),
        }
    }
}

impl Units {
    // Conversion constants
    const KM_TO_MILES: f64 = 0.621371;
    const METERS_TO_FEET: f64 = 3.28084;
    const KG_TO_LBS: f64 = 2.20462;

    /// Convert speed from m/s to display units.
    /// Returns (value, unit_label)
    pub fn format_speed(&self, meters_per_second: f64) -> (f64, &'static str) {
        match self {
            Units::Metric => {
                let kmh = meters_per_second * 3.6;
                (kmh, "km/h")
            }
            Units::Imperial => {
                let mph = meters_per_second * 3.6 * Self::KM_TO_MILES;
                (mph, "mph")
            }
        }
    }

    /// Convert distance from meters to display units.
    /// Returns (value, unit_label)
    pub fn format_distance(&self, meters: f64) -> (f64, &'static str) {
        match self {
            Units::Metric => {
                if meters < 1000.0 {
                    (meters, "m")
                } else {
                    (meters / 1000.0, "km")
                }
            }
            Units::Imperial => {
                let miles = meters / 1000.0 * Self::KM_TO_MILES;
                if miles < 0.1 {
                    let feet = meters * Self::METERS_TO_FEET;
                    (feet, "ft")
                } else {
                    (miles, "mi")
                }
            }
        }
    }

    /// Convert elevation from meters to display units.
    /// Returns (value, unit_label)
    pub fn format_elevation(&self, meters: f64) -> (f64, &'static str) {
        match self {
            Units::Metric => (meters, "m"),
            Units::Imperial => {
                let feet = meters * Self::METERS_TO_FEET;
                (feet, "ft")
            }
        }
    }

    /// Convert weight from kg to display units.
    /// Returns (value, unit_label)
    pub fn format_weight(&self, kilograms: f64) -> (f64, &'static str) {
        match self {
            Units::Metric => (kilograms, "kg"),
            Units::Imperial => {
                let lbs = kilograms * Self::KG_TO_LBS;
                (lbs, "lbs")
            }
        }
    }

    /// Convert temperature from Celsius to display units.
    /// Returns (value, unit_label)
    pub fn format_temperature(&self, celsius: f64) -> (f64, &'static str) {
        match self {
            Units::Metric => (celsius, "°C"),
            Units::Imperial => {
                let fahrenheit = celsius * 9.0 / 5.0 + 32.0;
                (fahrenheit, "°F")
            }
        }
    }

    /// Get the speed unit label.
    pub fn speed_unit(&self) -> &'static str {
        match self {
            Units::Metric => "km/h",
            Units::Imperial => "mph",
        }
    }

    /// Get the distance unit label (for longer distances).
    pub fn distance_unit(&self) -> &'static str {
        match self {
            Units::Metric => "km",
            Units::Imperial => "mi",
        }
    }

    /// Get the elevation unit label.
    pub fn elevation_unit(&self) -> &'static str {
        match self {
            Units::Metric => "m",
            Units::Imperial => "ft",
        }
    }

    /// Get the weight unit label.
    pub fn weight_unit(&self) -> &'static str {
        match self {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        }
    }

    /// Get the temperature unit label.
    pub fn temperature_unit(&self) -> &'static str {
        match self {
            Units::Metric => "°C",
            Units::Imperial => "°F",
        }
    }

    /// Convert input weight to kg for storage.
    pub fn weight_to_kg(&self, value: f64) -> f64 {
        match self {
            Units::Metric => value,
            Units::Imperial => value / Self::KG_TO_LBS,
        }
    }

    /// Convert input elevation to meters for storage.
    pub fn elevation_to_meters(&self, value: f64) -> f64 {
        match self {
            Units::Metric => value,
            Units::Imperial => value / Self::METERS_TO_FEET,
        }
    }

    /// Convert input distance to meters for storage.
    pub fn distance_to_meters(&self, value: f64, is_large: bool) -> f64 {
        match self {
            Units::Metric => {
                if is_large {
                    value * 1000.0 // km to m
                } else {
                    value // already meters
                }
            }
            Units::Imperial => {
                if is_large {
                    value / Self::KM_TO_MILES * 1000.0 // miles to m
                } else {
                    value / Self::METERS_TO_FEET // feet to m
                }
            }
        }
    }
}

/// UI theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Dark theme (default)
    #[default]
    Dark,
    /// Light theme
    Light,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Dark => write!(f, "Dark"),
            Theme::Light => write!(f, "Light"),
        }
    }
}

// ============================================================================
// UX & Accessibility Types (Feature 008)
// ============================================================================

/// Theme selection preference with system-following option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    /// Follow system dark/light mode
    #[default]
    FollowSystem,
    /// Always use light theme
    Light,
    /// Always use dark theme
    Dark,
}

impl std::fmt::Display for ThemePreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemePreference::FollowSystem => write!(f, "Follow System"),
            ThemePreference::Light => write!(f, "Light"),
            ThemePreference::Dark => write!(f, "Dark"),
        }
    }
}

/// Voice control activation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceActivation {
    /// Always listening for commands
    AlwaysOn,
    /// Requires push-to-talk key
    PushToTalk,
    /// Disabled
    #[default]
    Off,
}

/// Focus indicator style options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusIndicatorStyle {
    /// Standard 2px outline
    #[default]
    Standard,
    /// Bold 4px outline for visibility
    Bold,
    /// Animated pulsing outline
    Animated,
}

/// Accessibility configuration stored per user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    /// Colorblind mode selection (uses ColorMode from accessibility module)
    pub color_mode: String, // "normal", "protanopia", "deuteranopia", "tritanopia"

    /// High contrast mode enabled
    pub high_contrast: bool,

    /// Screen reader optimizations enabled
    pub screen_reader_enabled: bool,

    /// Voice control enabled
    pub voice_control_enabled: bool,

    /// Voice control activation mode
    pub voice_activation: VoiceActivation,

    /// Focus indicator style
    pub focus_indicator: FocusIndicatorStyle,

    /// Reduce motion animations
    pub reduce_motion: bool,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            color_mode: "normal".to_string(),
            high_contrast: false,
            screen_reader_enabled: false,
            voice_control_enabled: false,
            voice_activation: VoiceActivation::Off,
            focus_indicator: FocusIndicatorStyle::Standard,
            reduce_motion: false,
        }
    }
}

/// Audio feedback settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCueSettings {
    /// Master audio enabled
    pub enabled: bool,

    /// Audio cue volume (0.0 - 1.0)
    pub volume: f32,

    /// Play sound on interval transitions
    pub interval_cues: bool,

    /// Play sound on zone changes
    pub zone_change_cues: bool,

    /// Play sound on workout start/end
    pub workout_cues: bool,

    /// Enable countdown beeps before intervals
    pub countdown_enabled: bool,

    /// Number of countdown beeps (3, 5, or 10 seconds)
    pub countdown_seconds: u8,

    /// Custom audio profile selection
    pub profile: AudioProfile,
}

impl Default for AudioCueSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.7,
            interval_cues: true,
            zone_change_cues: true,
            workout_cues: true,
            countdown_enabled: true,
            countdown_seconds: 3,
            profile: AudioProfile::Simple,
        }
    }
}

/// Audio profile presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProfile {
    /// Simple beep tones
    #[default]
    Simple,
    /// Melodic tones
    Melodic,
    /// Minimal (only critical alerts)
    Minimal,
    /// Custom (user-defined frequencies)
    Custom,
}

/// Display mode for the ride screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    /// Standard dashboard layout
    #[default]
    Normal,
    /// Large display optimized (TV Mode)
    TvMode,
    /// Minimal distraction (Flow Mode)
    FlowMode,
}

/// Flow Mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowModeConfig {
    /// Primary metric to display in Flow Mode
    pub primary_metric: MetricType,

    /// Show 3D world background
    pub show_world: bool,

    /// Show interval notifications
    pub show_interval_alerts: bool,

    /// Notification display duration in seconds
    pub notification_duration_secs: f32,

    /// Overlay opacity (0.0 - 1.0)
    pub overlay_opacity: f32,
}

impl Default for FlowModeConfig {
    fn default() -> Self {
        Self {
            primary_metric: MetricType::Power,
            show_world: true,
            show_interval_alerts: true,
            notification_duration_secs: 3.0,
            overlay_opacity: 0.9,
        }
    }
}

/// Date format preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateFormat {
    #[default]
    /// YYYY-MM-DD
    Iso,
    /// MM/DD/YYYY
    UsFormat,
    /// DD/MM/YYYY
    EuFormat,
}

/// Number format preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    #[default]
    /// 1,234.56 (comma thousands, period decimal)
    Comma,
    /// 1.234,56 (period thousands, comma decimal)
    Period,
}

/// Language and locale settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleSettings {
    /// Selected language code (e.g., "en-US", "es", "fr")
    pub language: String,

    /// Use system language on startup
    pub follow_system: bool,

    /// Date format preference
    pub date_format: DateFormat,

    /// Number format (decimal separator)
    pub number_format: NumberFormat,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            language: "en-US".to_string(),
            follow_system: true,
            date_format: DateFormat::Iso,
            number_format: NumberFormat::Comma,
        }
    }
}

/// Extended user preferences including all UX & Accessibility settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Theme preference (extended from Theme to ThemePreference)
    pub theme_preference: ThemePreference,

    /// Accessibility settings
    pub accessibility: AccessibilitySettings,

    /// Audio feedback settings
    pub audio: AudioCueSettings,

    /// Current display mode
    pub display_mode: DisplayMode,

    /// Flow mode configuration
    pub flow_mode: FlowModeConfig,

    /// Locale/language settings
    pub locale: LocaleSettings,

    /// Active layout profile ID
    pub active_layout_id: Option<Uuid>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::FollowSystem,
            accessibility: AccessibilitySettings::default(),
            audio: AudioCueSettings::default(),
            display_mode: DisplayMode::Normal,
            flow_mode: FlowModeConfig::default(),
            locale: LocaleSettings::default(),
            active_layout_id: None,
        }
    }
}

/// User profile with physiological data and preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Functional Threshold Power in watts (50-600)
    pub ftp: u16,
    /// Maximum heart rate in bpm
    pub max_hr: Option<u8>,
    /// Resting heart rate in bpm
    pub resting_hr: Option<u8>,
    /// Weight in kilograms
    pub weight_kg: f32,
    /// Height in centimeters
    pub height_cm: Option<u16>,
    /// Power training zones
    #[serde(skip)]
    pub power_zones: PowerZones,
    /// Heart rate training zones
    #[serde(skip)]
    pub hr_zones: Option<HRZones>,
    /// Cadence training zones
    #[serde(skip)]
    pub cadence_zones: Option<CadenceZones>,
    /// Unit preference
    pub units: Units,
    /// Theme preference
    pub theme: Theme,
    /// Profile creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

impl Default for UserProfile {
    fn default() -> Self {
        let now = Utc::now();
        let ftp = 200;

        Self {
            id: Uuid::new_v4(),
            name: "Cyclist".to_string(),
            ftp,
            max_hr: None,
            resting_hr: None,
            weight_kg: 75.0,
            height_cm: None,
            power_zones: PowerZones::from_ftp(ftp),
            hr_zones: None,
            cadence_zones: None,
            units: Units::Metric,
            theme: Theme::Dark,
            created_at: now,
            updated_at: now,
        }
    }
}