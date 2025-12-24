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
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_3s_power: true,
            show_normalized_power: true,
            show_zone_colors: true,
            font_scale: 1.0,
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

    let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::IoError(e.to_string()))?;

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

    let content = toml::to_string_pretty(config).map_err(|e| ConfigError::SerializeError(e.to_string()))?;

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
