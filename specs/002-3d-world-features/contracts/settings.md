# Contract: Settings Module

**Module**: `src/storage/config.rs`, `src/ui/screens/settings.rs`, `src/ui/theme.rs`
**Feature**: 002-3d-world-features
**Status**: Extension of existing modules

## Purpose

Complete the settings system with full user profile configuration, training zone calculations, display preferences (units, theme), and persistence across sessions.

## Public Interface

### UserSettings

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub profile: UserProfile,
    pub display: DisplaySettings,
    pub training: TrainingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub ftp: u16,                    // Functional Threshold Power (watts)
    pub max_heart_rate: u8,          // Maximum heart rate (bpm)
    pub weight_kg: f32,              // Weight in kilograms
    pub height_cm: Option<u16>,      // Height in centimeters (optional)
    pub birth_year: Option<u16>,     // For age-based calculations (optional)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub unit_system: UnitSystem,
    pub theme: Theme,
    pub show_heart_rate: bool,
    pub show_cadence: bool,
    pub show_speed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSettings {
    pub power_zones: PowerZoneConfig,
    pub hr_zones: HrZoneConfig,
    pub auto_lap_distance: Option<f32>,  // Auto-lap every X km (None = disabled)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum UnitSystem {
    Metric,
    Imperial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}
```

### Power Zones

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZoneConfig {
    pub zones: [PowerZone; 7],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZone {
    pub name: String,
    pub min_percent: u8,    // % of FTP
    pub max_percent: u8,    // % of FTP
    pub color: [u8; 3],     // RGB color
}

impl PowerZoneConfig {
    /// Create standard Coggan power zones from FTP
    pub fn from_ftp(ftp: u16) -> Self {
        Self {
            zones: [
                PowerZone { name: "Recovery".into(), min_percent: 0, max_percent: 55, color: [128, 128, 128] },
                PowerZone { name: "Endurance".into(), min_percent: 55, max_percent: 75, color: [0, 128, 255] },
                PowerZone { name: "Tempo".into(), min_percent: 75, max_percent: 90, color: [0, 255, 128] },
                PowerZone { name: "Threshold".into(), min_percent: 90, max_percent: 105, color: [255, 255, 0] },
                PowerZone { name: "VO2max".into(), min_percent: 105, max_percent: 120, color: [255, 165, 0] },
                PowerZone { name: "Anaerobic".into(), min_percent: 120, max_percent: 150, color: [255, 0, 0] },
                PowerZone { name: "Neuromuscular".into(), min_percent: 150, max_percent: 255, color: [128, 0, 128] },
            ],
        }
    }

    /// Get zone number (1-7) for given power and FTP
    pub fn get_zone(&self, power: u16, ftp: u16) -> u8;

    /// Get zone bounds in watts for given FTP
    pub fn get_zone_watts(&self, zone: u8, ftp: u16) -> (u16, u16);
}
```

### Heart Rate Zones

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrZoneConfig {
    pub zones: [HrZone; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrZone {
    pub name: String,
    pub min_percent: u8,    // % of max HR
    pub max_percent: u8,    // % of max HR
    pub color: [u8; 3],     // RGB color
}

impl HrZoneConfig {
    /// Create standard 5-zone HR model from max HR
    pub fn from_max_hr(max_hr: u8) -> Self {
        Self {
            zones: [
                HrZone { name: "Recovery".into(), min_percent: 50, max_percent: 60, color: [128, 128, 128] },
                HrZone { name: "Endurance".into(), min_percent: 60, max_percent: 70, color: [0, 128, 255] },
                HrZone { name: "Tempo".into(), min_percent: 70, max_percent: 80, color: [0, 255, 128] },
                HrZone { name: "Threshold".into(), min_percent: 80, max_percent: 90, color: [255, 165, 0] },
                HrZone { name: "VO2max".into(), min_percent: 90, max_percent: 100, color: [255, 0, 0] },
            ],
        }
    }

    /// Get zone number (1-5) for given HR and max HR
    pub fn get_zone(&self, hr: u8, max_hr: u8) -> u8;
}
```

### Settings Manager

```rust
impl SettingsManager {
    /// Load settings from storage (or defaults if none exist)
    pub fn load() -> Result<UserSettings, SettingsError>;

    /// Save settings to persistent storage
    pub fn save(&self, settings: &UserSettings) -> Result<(), SettingsError>;

    /// Reset to default settings
    pub fn reset_to_defaults(&self) -> Result<UserSettings, SettingsError>;

    /// Update FTP and recalculate power zones
    pub fn update_ftp(&mut self, settings: &mut UserSettings, new_ftp: u16);

    /// Update max HR and recalculate HR zones
    pub fn update_max_hr(&mut self, settings: &mut UserSettings, new_max_hr: u8);
}
```

### Theme System

```rust
pub struct ThemeColors {
    pub background: Color32,
    pub surface: Color32,
    pub primary: Color32,
    pub secondary: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub border: Color32,
}

impl ThemeColors {
    pub fn dark() -> Self {
        ThemeColors {
            background: Color32::from_rgb(18, 18, 18),
            surface: Color32::from_rgb(30, 30, 30),
            primary: Color32::from_rgb(66, 165, 245),
            secondary: Color32::from_rgb(171, 71, 188),
            success: Color32::from_rgb(102, 187, 106),
            warning: Color32::from_rgb(255, 167, 38),
            error: Color32::from_rgb(239, 83, 80),
            text_primary: Color32::from_rgb(255, 255, 255),
            text_secondary: Color32::from_rgb(158, 158, 158),
            border: Color32::from_rgb(66, 66, 66),
        }
    }

    pub fn light() -> Self {
        ThemeColors {
            background: Color32::from_rgb(250, 250, 252),
            surface: Color32::from_rgb(255, 255, 255),
            primary: Color32::from_rgb(25, 118, 210),
            secondary: Color32::from_rgb(156, 39, 176),
            success: Color32::from_rgb(46, 125, 50),
            warning: Color32::from_rgb(245, 124, 0),
            error: Color32::from_rgb(211, 47, 47),
            text_primary: Color32::from_rgb(33, 33, 33),
            text_secondary: Color32::from_rgb(117, 117, 117),
            border: Color32::from_rgb(224, 224, 224),
        }
    }
}

/// Apply theme to egui context
pub fn apply_theme(ctx: &egui::Context, theme: Theme);
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to load settings: {0}")]
    LoadError(String),

    #[error("Failed to save settings: {0}")]
    SaveError(String),

    #[error("Invalid value for {field}: {message}")]
    ValidationError { field: String, message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## Storage Format

Settings stored as TOML in user config directory:

```toml
# ~/.config/rustride/settings.toml (Linux)
# ~/Library/Application Support/rustride/settings.toml (macOS)
# %APPDATA%\rustride\settings.toml (Windows)

[profile]
name = "Rider"
ftp = 250
max_heart_rate = 185
weight_kg = 75.0

[display]
unit_system = "Metric"
theme = "Dark"
show_heart_rate = true
show_cadence = true
show_speed = true

[training]
auto_lap_distance = 5.0

[training.power_zones]
# Custom zones if modified from defaults
```

## UI Components

### Settings Screen Layout

```
┌─────────────────────────────────────┐
│ Settings                       [←]  │
├─────────────────────────────────────┤
│ Profile                             │
│   Name: [______________]            │
│   FTP:  [250] W                     │
│   Max HR: [185] bpm                 │
│   Weight: [75.0] kg                 │
│                                     │
│ Display                             │
│   Units: [Metric ▼]                 │
│   Theme: [Dark ▼]                   │
│                                     │
│ Training Zones                      │
│   [View Power Zones]                │
│   [View HR Zones]                   │
│                                     │
│             [Save]  [Cancel]        │
└─────────────────────────────────────┘
```

## Implementation Notes

1. **Immediate preview**: Theme changes apply immediately without save
2. **Validation**: FTP 50-500W, Max HR 100-220, Weight 30-200kg
3. **Auto zone recalculation**: When FTP/MaxHR changes, zones recalculate
4. **Unit conversion**: All internal values in metric, convert for display only
5. **Settings migration**: Handle missing fields when loading old config files

## Testing Requirements

- Unit tests for zone calculations
- Unit tests for unit conversion
- Test settings persistence (save, load, migrate)
- Test theme application
