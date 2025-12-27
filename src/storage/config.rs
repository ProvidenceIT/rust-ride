//! User profile and application configuration.
//!
//! T010: Implement Config loading from TOML
//! T016: Define UserProfile struct with FTP, zones, preferences

use crate::metrics::zones::{HRZones, PowerZones};
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
            units: Units::Metric,
            theme: Theme::Dark,
            created_at: now,
            updated_at: now,
        }
    }
}

impl UserProfile {
    /// Create a new user profile with the given name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Update FTP and recalculate power zones.
    pub fn set_ftp(&mut self, ftp: u16) -> Result<(), &'static str> {
        if !Self::validate_ftp(ftp) {
            return Err("FTP must be between 50 and 600 watts");
        }

        self.ftp = ftp;
        if !self.power_zones.custom {
            self.power_zones = PowerZones::from_ftp(ftp);
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update heart rate values and recalculate HR zones.
    pub fn set_heart_rate(&mut self, max_hr: Option<u8>, resting_hr: Option<u8>) {
        self.max_hr = max_hr;
        self.resting_hr = resting_hr;

        if let (Some(max), Some(rest)) = (max_hr, resting_hr) {
            if max > rest {
                self.hr_zones = Some(HRZones::from_hr(max, rest));
            }
        }
        self.updated_at = Utc::now();
    }

    /// Validate FTP value (50-600 watts).
    pub fn validate_ftp(ftp: u16) -> bool {
        (50..=600).contains(&ftp)
    }

    /// Validate weight value (30-200 kg).
    pub fn validate_weight(weight: f32) -> bool {
        (30.0..=200.0).contains(&weight)
    }

    /// Convert weight to the user's preferred units.
    pub fn display_weight(&self) -> (f32, &'static str) {
        match self.units {
            Units::Metric => (self.weight_kg, "kg"),
            Units::Imperial => (self.weight_kg * 2.20462, "lbs"),
        }
    }

    /// Convert speed to the user's preferred units.
    pub fn convert_speed(&self, speed_kmh: f32) -> (f32, &'static str) {
        match self.units {
            Units::Metric => (speed_kmh, "km/h"),
            Units::Imperial => (speed_kmh * 0.621371, "mph"),
        }
    }

    /// Convert distance to the user's preferred units.
    pub fn convert_distance(&self, distance_km: f64) -> (f64, &'static str) {
        match self.units {
            Units::Metric => (distance_km, "km"),
            Units::Imperial => (distance_km * 0.621371, "mi"),
        }
    }
}

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application version
    pub version: String,
    /// Data directory path
    #[serde(skip)]
    pub data_dir: PathBuf,
    /// Sensor settings
    pub sensors: SensorSettings,
    /// Recording settings
    pub recording: RecordingSettings,
    /// UI settings
    pub ui: UiSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: PathBuf::new(),
            sensors: SensorSettings::default(),
            recording: RecordingSettings::default(),
            ui: UiSettings::default(),
        }
    }
}

/// Sensor-related settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorSettings {
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Discovery timeout in seconds
    pub discovery_timeout_secs: u32,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u32,
}

impl Default for SensorSettings {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            discovery_timeout_secs: 30,
            connection_timeout_secs: 10,
        }
    }
}

/// Recording-related settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSettings {
    /// Auto-save interval in seconds
    pub autosave_interval_secs: u32,
    /// Maximum power filter (values above this are noise)
    pub max_power_filter: u16,
    /// Record zero-power samples
    pub record_zeros: bool,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            autosave_interval_secs: 30,
            max_power_filter: 2000,
            record_zeros: true,
        }
    }
}

/// Metric types that can be displayed on the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    /// Instantaneous power
    Power,
    /// 3-second power average
    Power3s,
    /// Heart rate
    HeartRate,
    /// Cadence
    Cadence,
    /// Speed
    Speed,
    /// Distance
    Distance,
    /// Elapsed time
    Duration,
    /// Calories
    Calories,
    /// Normalized power
    NormalizedPower,
    /// Training Stress Score
    Tss,
    /// Intensity Factor
    IntensityFactor,
    /// Current power zone
    PowerZone,
    /// Current HR zone
    HrZone,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Power => write!(f, "Power"),
            MetricType::Power3s => write!(f, "3s Power"),
            MetricType::HeartRate => write!(f, "Heart Rate"),
            MetricType::Cadence => write!(f, "Cadence"),
            MetricType::Speed => write!(f, "Speed"),
            MetricType::Distance => write!(f, "Distance"),
            MetricType::Duration => write!(f, "Duration"),
            MetricType::Calories => write!(f, "Calories"),
            MetricType::NormalizedPower => write!(f, "NP"),
            MetricType::Tss => write!(f, "TSS"),
            MetricType::IntensityFactor => write!(f, "IF"),
            MetricType::PowerZone => write!(f, "Power Zone"),
            MetricType::HrZone => write!(f, "HR Zone"),
        }
    }
}

/// Dashboard layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    /// Primary metrics (shown large, in order)
    pub primary_metrics: Vec<MetricType>,
    /// Secondary metrics (shown medium, in order)
    pub secondary_metrics: Vec<MetricType>,
    /// Tertiary metrics (shown small, in order)
    pub tertiary_metrics: Vec<MetricType>,
}

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            primary_metrics: vec![
                MetricType::Power,
                MetricType::HeartRate,
                MetricType::Cadence,
            ],
            secondary_metrics: vec![
                MetricType::Duration,
                MetricType::Distance,
                MetricType::Speed,
                MetricType::Power3s,
                MetricType::Calories,
            ],
            tertiary_metrics: vec![
                MetricType::NormalizedPower,
                MetricType::Tss,
                MetricType::IntensityFactor,
            ],
        }
    }
}

/// UI-related settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Show 3-second average power
    pub show_3s_power: bool,
    /// Show normalized power during ride
    pub show_normalized_power: bool,
    /// Show zone colors
    pub show_zone_colors: bool,
    /// Font scale multiplier
    pub font_scale: f32,
    /// Dashboard metric layout
    #[serde(default)]
    pub dashboard_layout: DashboardLayout,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_3s_power: true,
            show_normalized_power: true,
            show_zone_colors: true,
            font_scale: 1.0,
            dashboard_layout: DashboardLayout::default(),
        }
    }
}

/// Get the application data directory.
pub fn get_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "rustride", "RustRide")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get the configuration file path.
pub fn get_config_path() -> PathBuf {
    get_data_dir().join("config.toml")
}

/// Load application configuration from file.
pub fn load_config() -> Result<AppConfig, ConfigError> {
    let path = get_config_path();

    if !path.exists() {
        let config = AppConfig {
            data_dir: get_data_dir(),
            ..Default::default()
        };
        return Ok(config);
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| ConfigError::IoError(e.to_string()))?;

    let mut config: AppConfig =
        toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

    config.data_dir = get_data_dir();

    Ok(config)
}

/// Save application configuration to file.
pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let path = get_config_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::IoError(e.to_string()))?;
    }

    let content =
        toml::to_string_pretty(config).map_err(|e| ConfigError::SerializeError(e.to_string()))?;

    std::fs::write(&path, content).map_err(|e| ConfigError::IoError(e.to_string()))?;

    Ok(())
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialize error: {0}")]
    SerializeError(String),
}
