//! Settings screen implementation.
//!
//! T120: Create settings screen with profile section
//! T121: Implement FTP, max HR, resting HR, weight, height inputs
//! T122: Implement power zone editor with auto-calculate toggle
//! T123: Implement HR zone editor
//! T124: Implement unit preference toggle (metric/imperial)
//! T125: Implement theme toggle (dark/light)
//! T076: Display current FTP with confidence on profile screen
//! T124: Add rider type display to profile screen
//! T146: Add immersion effect toggles to settings
//! T064: Add audio alert settings to settings screen
//! T092: Add HID device list and button mapping UI with learning mode

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui};
use std::collections::HashMap;

use crate::audio::{AlertCategory, AlertType, AudioConfig, VoiceInfo};
use crate::ui::settings::{AudioSettingsAction, AudioSettingsPanel, AudioSettingsPanelConfig, AudioTestType};
use crate::hid::{ButtonAction, HidConfig, HidDevice, HidDeviceConfig, HidDeviceStatus};
use crate::integrations::mqtt::{FanProfile, MqttConfig, PayloadFormat};
use crate::integrations::sync::{SyncConfig, SyncPlatform};
use crate::integrations::weather::{WeatherConfig, WeatherUnits};
use crate::metrics::analytics::{FtpConfidence, PowerProfile, RiderType};
use crate::metrics::zones::{HRZones, PowerZones};
use crate::sensors::InclineConfig;
use crate::storage::config::{
    AccessibilitySettings, LocaleSettings, Theme, ThemePreference, Units, UserProfile,
};
use uuid::Uuid;

/// Settings screen state.
pub struct SettingsScreen {
    /// Current user profile being edited
    pub profile: UserProfile,
    /// Original profile (for cancel/reset)
    original_profile: UserProfile,
    /// Has unsaved changes
    pub has_changes: bool,
    /// Validation error message
    pub error_message: Option<String>,
    /// Show zone editor
    show_power_zones: bool,
    show_hr_zones: bool,
    /// Auto-calculate zones from FTP/HR
    auto_calculate_power_zones: bool,
    auto_calculate_hr_zones: bool,
    /// Temporary input buffers for numeric fields
    ftp_input: String,
    max_hr_input: String,
    resting_hr_input: String,
    weight_input: String,
    height_input: String,
    /// T076: FTP confidence from auto-detection
    pub ftp_confidence: Option<FtpConfidence>,
    /// T124: Rider type classification
    pub rider_type: Option<RiderType>,
    /// Power profile for radar display
    pub power_profile: Option<PowerProfile>,
    /// T146: Immersion effect settings
    pub immersion_settings: ImmersionSettings,
    /// T042: Incline/slope mode settings
    pub incline_config: InclineConfig,
    /// Input buffers for incline settings
    incline_rider_weight_input: String,
    incline_bike_weight_input: String,
    /// T064: Audio alert settings
    pub audio_alert_settings: AudioAlertSettings,
    /// Show/hide audio alerts section
    show_audio_alerts: bool,
    /// Available system voices for TTS
    available_voices: Vec<VoiceInfo>,
    /// Audio engine configuration (volume levels, mute states)
    pub audio_config: AudioConfig,
    /// Show/hide audio settings section
    show_audio_settings: bool,
    /// T072: MQTT configuration
    pub mqtt_config: MqttConfig,
    /// T073: Fan profiles for zone-based fan control
    pub fan_profiles: Vec<FanProfile>,
    /// Show/hide MQTT section
    show_mqtt: bool,
    /// Show/hide fan profiles section
    show_fan_profiles: bool,
    /// MQTT broker port input buffer
    mqtt_port_input: String,
    /// Editing fan profile (index, if editing)
    editing_fan_profile: Option<usize>,
    /// T100: Weather configuration
    pub weather_config: WeatherConfig,
    /// T100: Show/hide weather section
    show_weather: bool,
    /// T100: Weather latitude input buffer
    weather_lat_input: String,
    /// T100: Weather longitude input buffer
    weather_lon_input: String,
    /// T109: Sync/platform configuration
    pub sync_config: SyncConfig,
    /// T109: Show/hide sync section
    show_sync: bool,
    /// T109: Connected platform states (for display)
    pub platform_states: Vec<(SyncPlatform, bool)>,
    /// T092: HID device settings
    pub hid_settings: HidSettings,
    /// T092: Show/hide HID section
    show_hid: bool,
    /// T047, T060, T065, T113, T131: Accessibility settings
    pub accessibility_settings: AccessibilitySettings,
    /// T047: Show/hide accessibility section
    show_accessibility: bool,
    /// T065: Theme preference (Follow System, Light, Dark)
    pub theme_preference: ThemePreference,
    /// T113: Locale/language settings
    pub locale_settings: LocaleSettings,
    /// T060: Restart onboarding flag
    pub restart_onboarding_requested: bool,
    /// T092: TV Mode enabled
    pub tv_mode_enabled: bool,
    /// T092: TV Mode font scale (1.5-3.0)
    pub tv_mode_font_scale: f32,
}

/// Configuration for a specific alert type (voice vs. sound)
#[derive(Debug, Clone)]
pub struct AlertTypeConfig {
    /// Whether this alert uses voice announcement
    pub use_voice: bool,
    /// Whether this alert plays a sound effect
    pub play_sound: bool,
}

impl Default for AlertTypeConfig {
    fn default() -> Self {
        Self {
            use_voice: true,
            play_sound: true,
        }
    }
}

impl AlertTypeConfig {
    /// Create with voice only (no sound effect)
    pub fn voice_only() -> Self {
        Self {
            use_voice: true,
            play_sound: false,
        }
    }

    /// Create with sound only (no voice)
    pub fn sound_only() -> Self {
        Self {
            use_voice: false,
            play_sound: true,
        }
    }
}

/// T064: Audio alert settings for voice alerts and notifications.
#[derive(Debug, Clone)]
pub struct AudioAlertSettings {
    /// Master voice alerts enabled
    pub voice_alerts_enabled: bool,
    /// Volume for voice alerts (0.0-1.0)
    pub voice_volume: f32,
    /// Speech rate (0.5-2.0, 1.0 is normal)
    pub speech_rate: f32,
    /// Preferred voice ID (system-specific identifier)
    pub preferred_voice: Option<String>,
    /// Workout alerts enabled (start, intervals, countdown, complete)
    pub workout_alerts_enabled: bool,
    /// Zone change alerts enabled (power zone, HR zone changes)
    pub zone_alerts_enabled: bool,
    /// Sensor alerts enabled (connect, disconnect, low battery)
    pub sensor_alerts_enabled: bool,
    /// Achievement alerts enabled (PRs, milestones)
    pub achievement_alerts_enabled: bool,
    /// Interval countdown threshold (seconds before interval change)
    pub countdown_threshold_secs: u32,
    /// Zone change debounce time (minimum seconds between zone alerts)
    pub zone_debounce_secs: u32,
    /// Per-alert-type voice/sound configuration
    pub alert_type_configs: HashMap<AlertType, AlertTypeConfig>,
}

impl AudioAlertSettings {
    /// Get the list of user-configurable alert types
    /// These are the most common alerts users may want to customize
    fn configurable_alert_types() -> Vec<AlertType> {
        vec![
            // Workout alerts
            AlertType::WorkoutStart,
            AlertType::IntervalChange,
            AlertType::IntervalCountdown,
            AlertType::WorkoutComplete,
            AlertType::RecoveryStart,
            // Power alerts
            AlertType::PowerZoneChange,
            AlertType::PowerTooHigh,
            AlertType::PowerTooLow,
            // Heart rate alerts
            AlertType::HeartRateZoneChange,
            AlertType::HeartRateTooHigh,
            AlertType::HeartRateTooLow,
            // Sensor alerts
            AlertType::SensorConnected,
            AlertType::SensorDisconnected,
            // Milestone alerts
            AlertType::DistanceMilestone,
            AlertType::TimeMilestone,
        ]
    }

    /// Create default alert type configurations
    fn default_alert_type_configs() -> HashMap<AlertType, AlertTypeConfig> {
        let mut configs = HashMap::new();
        for alert_type in Self::configurable_alert_types() {
            configs.insert(alert_type, AlertTypeConfig::default());
        }
        configs
    }

    /// Get config for an alert type, with defaults
    pub fn get_alert_type_config(&self, alert_type: AlertType) -> AlertTypeConfig {
        self.alert_type_configs
            .get(&alert_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Set config for an alert type
    pub fn set_alert_type_config(&mut self, alert_type: AlertType, config: AlertTypeConfig) {
        self.alert_type_configs.insert(alert_type, config);
    }
}

impl Default for AudioAlertSettings {
    fn default() -> Self {
        Self {
            voice_alerts_enabled: true,
            voice_volume: 0.8,
            speech_rate: 1.0,
            preferred_voice: None,
            workout_alerts_enabled: true,
            zone_alerts_enabled: true,
            sensor_alerts_enabled: true,
            achievement_alerts_enabled: true,
            countdown_threshold_secs: 10,
            zone_debounce_secs: 5,
            alert_type_configs: AudioAlertSettings::default_alert_type_configs(),
        }
    }
}

/// T092: HID device settings for button mapping UI.
#[derive(Debug, Clone)]
pub struct HidSettings {
    /// Master HID support enabled
    pub enabled: bool,
    /// Detected HID devices
    pub devices: Vec<HidDevice>,
    /// Device configurations (persisted)
    pub device_configs: Vec<HidDeviceConfig>,
    /// Currently selected device for mapping
    pub selected_device: Option<Uuid>,
    /// Learning mode: waiting for button press
    pub learning_mode: bool,
    /// The device for which learning mode was started
    pub learning_device_id: Option<Uuid>,
    /// The mapping slot being configured (device_id, button_index)
    pub learning_target: Option<(Uuid, usize)>,
    /// Last learned button code
    pub learned_button_code: Option<u8>,
    /// Action being selected for new mapping
    pub selecting_action_for: Option<(Uuid, u8)>,
    /// Flag indicating a device scan was requested
    pub scan_requested: bool,
    /// Flag indicating learning mode start was requested (device_id)
    pub learning_mode_start_requested: Option<Uuid>,
    /// Flag indicating learning mode cancel was requested
    pub learning_mode_cancel_requested: bool,
}

impl Default for HidSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: Vec::new(),
            device_configs: Vec::new(),
            selected_device: None,
            learning_mode: false,
            learning_device_id: None,
            learning_target: None,
            learned_button_code: None,
            selecting_action_for: None,
            scan_requested: false,
            learning_mode_start_requested: None,
            learning_mode_cancel_requested: false,
        }
    }
}

impl HidSettings {
    /// Create from HidConfig
    pub fn from_config(config: &HidConfig) -> Self {
        Self {
            enabled: config.enabled,
            devices: Vec::new(),
            device_configs: config.devices.clone(),
            selected_device: None,
            learning_mode: false,
            learning_device_id: None,
            learning_target: None,
            learned_button_code: None,
            selecting_action_for: None,
            scan_requested: false,
            learning_mode_start_requested: None,
            learning_mode_cancel_requested: false,
        }
    }

    /// Update detected devices
    pub fn set_devices(&mut self, devices: Vec<HidDevice>) {
        self.devices = devices;
        // Auto-select first device if none selected
        if self.selected_device.is_none() && !self.devices.is_empty() {
            self.selected_device = Some(self.devices[0].id);
        }
    }

    /// Get config for a device
    pub fn get_device_config(&self, device_id: &Uuid) -> Option<&HidDeviceConfig> {
        self.device_configs
            .iter()
            .find(|c| &c.device_id == device_id)
    }

    /// Get or create mutable config for a device
    pub fn get_or_create_device_config(&mut self, device: &HidDevice) -> &mut HidDeviceConfig {
        let device_id = device.id;
        if !self.device_configs.iter().any(|c| c.device_id == device_id) {
            self.device_configs.push(HidDeviceConfig {
                device_id,
                vendor_id: device.vendor_id,
                product_id: device.product_id,
                name: device.name.clone(),
                enabled: true,
                mappings: Vec::new(),
            });
        }
        self.device_configs
            .iter_mut()
            .find(|c| c.device_id == device_id)
            .unwrap()
    }

    /// Convert to HidConfig for saving
    pub fn to_config(&self) -> HidConfig {
        HidConfig {
            enabled: self.enabled,
            devices: self.device_configs.clone(),
        }
    }
}

/// T146: Immersion effect settings
#[derive(Debug, Clone)]
pub struct ImmersionSettings {
    /// Enable visual immersion effects (vignette, color grading)
    pub visual_effects_enabled: bool,
    /// Enable audio effects (breathing, heartbeat, environment)
    pub audio_effects_enabled: bool,
    /// Enable effort-based vignette
    pub vignette_enabled: bool,
    /// Enable effort-based color grading
    pub color_grading_enabled: bool,
    /// Enable breathing sounds at high effort
    pub breathing_sounds_enabled: bool,
    /// Enable heartbeat sounds at very high effort
    pub heartbeat_sounds_enabled: bool,
    /// Enable environmental audio (wind, rain, birds)
    pub environment_audio_enabled: bool,
    /// Enable cyclist audio (tire roll, drivetrain)
    pub cyclist_audio_enabled: bool,
    /// Master audio volume (0.0-1.0)
    pub audio_volume: f32,
}

impl Default for ImmersionSettings {
    fn default() -> Self {
        Self {
            visual_effects_enabled: true,
            audio_effects_enabled: true,
            vignette_enabled: true,
            color_grading_enabled: true,
            breathing_sounds_enabled: true,
            heartbeat_sounds_enabled: true,
            environment_audio_enabled: true,
            cyclist_audio_enabled: true,
            audio_volume: 0.8,
        }
    }
}

/// Settings for testing voice preview.
#[derive(Debug, Clone, PartialEq)]
pub struct TestVoiceSettings {
    /// The voice ID to use (None = system default)
    pub voice_id: Option<String>,
    /// Volume level (0.0-1.0)
    pub volume: f32,
    /// Speech rate (0.5-2.0, 1.0 is normal)
    pub rate: f32,
}

/// Actions that can result from the settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsAction {
    /// No action
    None,
    /// Save changes and go back
    Save,
    /// Cancel changes and go back
    Cancel,
    /// Test/preview the current voice settings
    TestVoice(TestVoiceSettings),
    /// Test/preview an audio sound (countdown, achievement, milestone, etc.)
    TestAudio(AudioTestType),
    /// Audio configuration changed (auto-save)
    AudioConfigChanged(AudioConfig),
}