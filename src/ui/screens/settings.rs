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

/// T064: Audio alert settings for voice alerts and notifications.
#[derive(Debug, Clone)]
pub struct AudioAlertSettings {
    /// Master voice alerts enabled
    pub voice_alerts_enabled: bool,
    /// Volume for voice alerts (0.0-1.0)
    pub voice_volume: f32,
    /// Speech rate (0.5-2.0, 1.0 is normal)
    pub speech_rate: f32,
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
}

impl Default for AudioAlertSettings {
    fn default() -> Self {
        Self {
            voice_alerts_enabled: true,
            voice_volume: 0.8,
            speech_rate: 1.0,
            workout_alerts_enabled: true,
            zone_alerts_enabled: true,
            sensor_alerts_enabled: true,
            achievement_alerts_enabled: true,
            countdown_threshold_secs: 10,
            zone_debounce_secs: 5,
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
    /// The mapping slot being configured (device_id, button_index)
    pub learning_target: Option<(Uuid, usize)>,
    /// Last learned button code
    pub learned_button_code: Option<u8>,
    /// Action being selected for new mapping
    pub selecting_action_for: Option<(Uuid, u8)>,
}

impl Default for HidSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            devices: Vec::new(),
            device_configs: Vec::new(),
            selected_device: None,
            learning_mode: false,
            learning_target: None,
            learned_button_code: None,
            selecting_action_for: None,
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
            learning_target: None,
            learned_button_code: None,
            selecting_action_for: None,
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

/// Actions that can result from the settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsAction {
    /// No action
    None,
    /// Save changes and go back
    Save,
    /// Cancel changes and go back
    Cancel,
}

impl SettingsScreen {
    /// Create a new settings screen with the given profile.
    pub fn new(profile: UserProfile) -> Self {
        let ftp_input = profile.ftp.to_string();
        let max_hr_input = profile.max_hr.map(|v| v.to_string()).unwrap_or_default();
        let resting_hr_input = profile
            .resting_hr
            .map(|v| v.to_string())
            .unwrap_or_default();
        let weight_input = format!("{:.1}", profile.weight_kg);
        let height_input = profile.height_cm.map(|v| v.to_string()).unwrap_or_default();
        let auto_power = !profile.power_zones.custom;
        let auto_hr = profile.hr_zones.as_ref().map(|z| !z.custom).unwrap_or(true);

        Self {
            original_profile: profile.clone(),
            profile,
            has_changes: false,
            error_message: None,
            show_power_zones: false,
            show_hr_zones: false,
            auto_calculate_power_zones: auto_power,
            auto_calculate_hr_zones: auto_hr,
            ftp_input,
            max_hr_input,
            resting_hr_input,
            weight_input,
            height_input,
            ftp_confidence: None,
            rider_type: None,
            power_profile: None,
            immersion_settings: ImmersionSettings::default(),
            incline_config: InclineConfig::default(),
            incline_rider_weight_input: "75.0".to_string(),
            incline_bike_weight_input: "10.0".to_string(),
            audio_alert_settings: AudioAlertSettings::default(),
            show_audio_alerts: false,
            mqtt_config: MqttConfig::default(),
            fan_profiles: vec![FanProfile::default()],
            show_mqtt: false,
            show_fan_profiles: false,
            mqtt_port_input: "1883".to_string(),
            editing_fan_profile: None,
            weather_config: WeatherConfig::default(),
            show_weather: false,
            weather_lat_input: "0.0".to_string(),
            weather_lon_input: "0.0".to_string(),
            sync_config: SyncConfig::default(),
            show_sync: false,
            platform_states: vec![
                (SyncPlatform::Strava, false),
                (SyncPlatform::GarminConnect, false),
                (SyncPlatform::TrainingPeaks, false),
                (SyncPlatform::IntervalsIcu, false),
            ],
            hid_settings: HidSettings::default(),
            show_hid: false,
            accessibility_settings: AccessibilitySettings::default(),
            show_accessibility: false,
            theme_preference: ThemePreference::FollowSystem,
            locale_settings: LocaleSettings::default(),
            restart_onboarding_requested: false,
            tv_mode_enabled: false,
            tv_mode_font_scale: 2.0,
        }
    }

    /// Set weather configuration.
    /// T100: Weather settings management.
    pub fn set_weather_config(&mut self, config: WeatherConfig) {
        self.weather_lat_input = format!("{:.4}", config.latitude);
        self.weather_lon_input = format!("{:.4}", config.longitude);
        self.weather_config = config;
    }

    /// Get current weather configuration.
    pub fn get_weather_config(&self) -> &WeatherConfig {
        &self.weather_config
    }

    /// Set incline configuration.
    pub fn set_incline_config(&mut self, config: InclineConfig) {
        self.incline_rider_weight_input = format!("{:.1}", config.rider_weight_kg);
        self.incline_bike_weight_input = format!("{:.1}", config.bike_weight_kg);
        self.incline_config = config;
    }

    /// Get current incline configuration.
    pub fn get_incline_config(&self) -> &InclineConfig {
        &self.incline_config
    }

    /// Set FTP confidence from auto-detection.
    /// T076: Display current FTP with confidence on profile screen.
    pub fn set_ftp_confidence(&mut self, confidence: Option<FtpConfidence>) {
        self.ftp_confidence = confidence;
    }

    /// Set rider type classification.
    /// T124: Add rider type display to profile screen.
    pub fn set_rider_type(&mut self, rider_type: Option<RiderType>, profile: Option<PowerProfile>) {
        self.rider_type = rider_type;
        self.power_profile = profile;
    }

    /// Update the profile (e.g., after loading from database).
    pub fn set_profile(&mut self, profile: UserProfile) {
        self.original_profile = profile.clone();
        self.profile = profile;
        self.has_changes = false;
        self.sync_inputs();
    }

    /// Sync input buffers from profile values.
    fn sync_inputs(&mut self) {
        self.ftp_input = self.profile.ftp.to_string();
        self.max_hr_input = self
            .profile
            .max_hr
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.resting_hr_input = self
            .profile
            .resting_hr
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.weight_input = format!("{:.1}", self.profile.weight_kg);
        self.height_input = self
            .profile
            .height_cm
            .map(|v| v.to_string())
            .unwrap_or_default();
    }

    /// Render the settings screen.
    pub fn show(&mut self, ui: &mut Ui) -> SettingsAction {
        let mut action = SettingsAction::None;

        // Header
        ui.horizontal(|ui| {
            ui.heading("Settings");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.has_changes,
                        egui::Button::new("Save").fill(Color32::from_rgb(52, 168, 83)),
                    )
                    .clicked()
                    && self.validate()
                {
                    action = SettingsAction::Save;
                }

                if ui.button("Cancel").clicked() {
                    action = SettingsAction::Cancel;
                }
            });
        });

        ui.separator();

        // Error message
        if let Some(ref error) = self.error_message {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⚠ {}", error)).color(Color32::from_rgb(234, 67, 53)),
                );
            });
            ui.add_space(8.0);
        }

        // Scrollable content
        ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Profile section
            self.render_profile_section(ui);

            ui.add_space(16.0);

            // T124: Rider type section
            self.render_rider_type_section(ui);

            ui.add_space(16.0);

            // Power zones section
            self.render_power_zones_section(ui);

            ui.add_space(16.0);

            // HR zones section
            self.render_hr_zones_section(ui);

            ui.add_space(16.0);

            // Preferences section
            self.render_preferences_section(ui);

            ui.add_space(16.0);

            // T100: Weather settings section
            self.render_weather_section(ui);

            ui.add_space(16.0);

            // T146: Immersion effects section
            self.render_immersion_section(ui);

            ui.add_space(16.0);

            // T064: Audio alerts section
            self.render_audio_alerts_section(ui);

            ui.add_space(16.0);

            // T042: Incline mode section
            self.render_incline_section(ui);

            ui.add_space(16.0);

            // T072: MQTT settings section
            self.render_mqtt_section(ui);

            ui.add_space(16.0);

            // T073: Fan profile configuration section
            self.render_fan_profiles_section(ui);

            ui.add_space(16.0);

            // T109: Platform sync configuration section
            self.render_sync_section(ui);

            ui.add_space(16.0);

            // T092: HID device settings section
            self.render_hid_section(ui);

            ui.add_space(16.0);

            // T047, T060, T065, T113, T131: Accessibility settings section
            self.render_accessibility_section(ui);

            ui.add_space(32.0);
        });

        action
    }

    /// Render the profile section.
    fn render_profile_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Profile").size(18.0).strong());
            ui.add_space(8.0);

            egui::Grid::new("profile_grid")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    // Name
                    ui.label("Name:");
                    let name_response = ui.add(
                        egui::TextEdit::singleline(&mut self.profile.name).desired_width(200.0),
                    );
                    if name_response.changed() {
                        self.has_changes = true;
                    }
                    ui.end_row();

                    // FTP with confidence indicator (T076)
                    ui.label("FTP (watts):");
                    ui.horizontal(|ui| {
                        let ftp_response = ui.add(
                            egui::TextEdit::singleline(&mut self.ftp_input).desired_width(100.0),
                        );
                        if ftp_response.changed() {
                            self.has_changes = true;
                            if let Ok(ftp) = self.ftp_input.parse::<u16>() {
                                if UserProfile::validate_ftp(ftp) {
                                    let _ = self.profile.set_ftp(ftp);
                                    if self.auto_calculate_power_zones {
                                        self.profile.power_zones = PowerZones::from_ftp(ftp);
                                    }
                                    self.error_message = None;
                                } else {
                                    self.error_message =
                                        Some("FTP must be between 50 and 600 watts".to_string());
                                }
                            }
                        }

                        // Show confidence badge if auto-detected
                        if let Some(confidence) = &self.ftp_confidence {
                            let (color, label) = match confidence {
                                FtpConfidence::High => (Color32::from_rgb(50, 205, 50), "High"),
                                FtpConfidence::Medium => (Color32::from_rgb(255, 165, 0), "Medium"),
                                FtpConfidence::Low => (Color32::from_rgb(220, 20, 60), "Low"),
                            };
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("({} confidence)", label))
                                    .color(color)
                                    .small(),
                            );
                        }
                    });
                    ui.end_row();

                    // Weight
                    ui.label("Weight (kg):");
                    let weight_response = ui.add(
                        egui::TextEdit::singleline(&mut self.weight_input).desired_width(100.0),
                    );
                    if weight_response.changed() {
                        self.has_changes = true;
                        if let Ok(weight) = self.weight_input.parse::<f32>() {
                            if UserProfile::validate_weight(weight) {
                                self.profile.weight_kg = weight;
                                self.error_message = None;
                            } else {
                                self.error_message =
                                    Some("Weight must be between 30 and 200 kg".to_string());
                            }
                        }
                    }
                    ui.end_row();

                    // Height
                    ui.label("Height (cm):");
                    let height_response = ui.add(
                        egui::TextEdit::singleline(&mut self.height_input).desired_width(100.0),
                    );
                    if height_response.changed() {
                        self.has_changes = true;
                        if self.height_input.is_empty() {
                            self.profile.height_cm = None;
                        } else if let Ok(height) = self.height_input.parse::<u16>() {
                            if (100..=250).contains(&height) {
                                self.profile.height_cm = Some(height);
                                self.error_message = None;
                            } else {
                                self.error_message =
                                    Some("Height must be between 100 and 250 cm".to_string());
                            }
                        }
                    }
                    ui.end_row();

                    // Max HR
                    ui.label("Max HR (bpm):");
                    let max_hr_response = ui.add(
                        egui::TextEdit::singleline(&mut self.max_hr_input).desired_width(100.0),
                    );
                    if max_hr_response.changed() {
                        self.has_changes = true;
                        self.update_hr_zones();
                    }
                    ui.end_row();

                    // Resting HR
                    ui.label("Resting HR (bpm):");
                    let resting_hr_response = ui.add(
                        egui::TextEdit::singleline(&mut self.resting_hr_input).desired_width(100.0),
                    );
                    if resting_hr_response.changed() {
                        self.has_changes = true;
                        self.update_hr_zones();
                    }
                    ui.end_row();
                });
        });
    }

    /// Update HR zones from max/resting HR inputs.
    fn update_hr_zones(&mut self) {
        let max_hr = if self.max_hr_input.is_empty() {
            None
        } else {
            self.max_hr_input.parse::<u8>().ok()
        };

        let resting_hr = if self.resting_hr_input.is_empty() {
            None
        } else {
            self.resting_hr_input.parse::<u8>().ok()
        };

        self.profile.set_heart_rate(max_hr, resting_hr);

        if self.auto_calculate_hr_zones {
            if let (Some(max), Some(rest)) = (max_hr, resting_hr) {
                if max > rest {
                    self.profile.hr_zones = Some(HRZones::from_hr(max, rest));
                }
            }
        }
    }

    /// Render the power zones section.
    fn render_power_zones_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Power Zones").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_power_zones {
                            "Hide"
                        } else {
                            "Show"
                        })
                        .clicked()
                    {
                        self.show_power_zones = !self.show_power_zones;
                    }
                });
            });

            ui.add_space(4.0);

            // Auto-calculate checkbox
            if ui
                .checkbox(
                    &mut self.auto_calculate_power_zones,
                    "Auto-calculate from FTP",
                )
                .changed()
            {
                self.has_changes = true;
                self.profile.power_zones.custom = !self.auto_calculate_power_zones;
                if self.auto_calculate_power_zones {
                    self.profile.power_zones = PowerZones::from_ftp(self.profile.ftp);
                }
            }

            if self.show_power_zones {
                ui.add_space(8.0);
                self.render_power_zones_table(ui);
            }
        });
    }

    /// Render the power zones table.
    fn render_power_zones_table(&self, ui: &mut Ui) {
        egui::Grid::new("power_zones_grid")
            .num_columns(4)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Zone").strong());
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("% FTP").strong());
                ui.label(RichText::new("Watts").strong());
                ui.end_row();

                let zones = self.profile.power_zones.all_zones();
                for zone in zones {
                    ui.label(format!("Z{}", zone.zone));
                    ui.label(&zone.name);
                    ui.label(format!("{}%-{}%", zone.min_percent, zone.max_percent));
                    let watts_str = if zone.max_watts > 1000 {
                        format!("{}+", zone.min_watts)
                    } else {
                        format!("{}-{}", zone.min_watts, zone.max_watts)
                    };
                    ui.label(watts_str);
                    ui.end_row();
                }
            });
    }

    /// Render the HR zones section.
    fn render_hr_zones_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Heart Rate Zones").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_hr_zones { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_hr_zones = !self.show_hr_zones;
                    }
                });
            });

            if self.profile.hr_zones.is_none() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Enter Max HR and Resting HR to calculate zones")
                        .weak()
                        .italics(),
                );
            } else {
                ui.add_space(4.0);

                // Auto-calculate checkbox
                if ui
                    .checkbox(
                        &mut self.auto_calculate_hr_zones,
                        "Auto-calculate from Max/Resting HR",
                    )
                    .changed()
                {
                    self.has_changes = true;
                    if let Some(ref mut zones) = self.profile.hr_zones {
                        zones.custom = !self.auto_calculate_hr_zones;
                    }
                    if self.auto_calculate_hr_zones {
                        self.update_hr_zones();
                    }
                }

                if self.show_hr_zones {
                    ui.add_space(8.0);
                    self.render_hr_zones_table(ui);
                }
            }
        });
    }

    /// Render the HR zones table.
    fn render_hr_zones_table(&self, ui: &mut Ui) {
        if let Some(ref zones) = self.profile.hr_zones {
            egui::Grid::new("hr_zones_grid")
                .num_columns(4)
                .striped(true)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Zone").strong());
                    ui.label(RichText::new("Name").strong());
                    ui.label(RichText::new("% HRR").strong());
                    ui.label(RichText::new("BPM").strong());
                    ui.end_row();

                    let all_zones = zones.all_zones();
                    for zone in all_zones {
                        ui.label(format!("Z{}", zone.zone));
                        ui.label(&zone.name);
                        // Calculate percentage
                        let hrr = self.profile.max_hr.unwrap_or(180)
                            - self.profile.resting_hr.unwrap_or(60);
                        let rest = self.profile.resting_hr.unwrap_or(60);
                        let min_pct = ((zone.min_bpm - rest) as f32 / hrr as f32 * 100.0) as u8;
                        let max_pct = ((zone.max_bpm - rest) as f32 / hrr as f32 * 100.0) as u8;
                        ui.label(format!("{}%-{}%", min_pct, max_pct));
                        ui.label(format!("{}-{}", zone.min_bpm, zone.max_bpm));
                        ui.end_row();
                    }
                });
        }
    }

    /// Render the preferences section.
    fn render_preferences_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Preferences").size(18.0).strong());
            ui.add_space(8.0);

            egui::Grid::new("preferences_grid")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    // Units
                    ui.label("Units:");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.profile.units == Units::Metric,
                                "Metric (km, kg)",
                            )
                            .clicked()
                        {
                            self.profile.units = Units::Metric;
                            self.has_changes = true;
                        }
                        if ui
                            .selectable_label(
                                self.profile.units == Units::Imperial,
                                "Imperial (mi, lbs)",
                            )
                            .clicked()
                        {
                            self.profile.units = Units::Imperial;
                            self.has_changes = true;
                        }
                    });
                    ui.end_row();

                    // Theme
                    ui.label("Theme:");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.profile.theme == Theme::Dark, "Dark")
                            .clicked()
                        {
                            self.profile.theme = Theme::Dark;
                            self.has_changes = true;
                        }
                        if ui
                            .selectable_label(self.profile.theme == Theme::Light, "Light")
                            .clicked()
                        {
                            self.profile.theme = Theme::Light;
                            self.has_changes = true;
                        }
                    });
                    ui.end_row();
                });
        });
    }

    /// Render the weather settings section.
    /// T100: Add weather settings to settings screen.
    fn render_weather_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Weather Display").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_weather { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_weather = !self.show_weather;
                    }
                });
            });

            ui.add_space(8.0);

            // Master weather toggle
            if ui
                .checkbox(&mut self.weather_config.enabled, "Enable weather display")
                .on_hover_text("Show current weather conditions during rides")
                .changed()
            {
                self.has_changes = true;
            }

            if self.show_weather {
                ui.add_enabled_ui(self.weather_config.enabled, |ui| {
                    ui.add_space(8.0);

                    // Units selection
                    ui.horizontal(|ui| {
                        ui.label("Temperature units:");
                        if ui
                            .selectable_label(
                                self.weather_config.units == WeatherUnits::Metric,
                                "Celsius (°C)",
                            )
                            .clicked()
                        {
                            self.weather_config.units = WeatherUnits::Metric;
                            self.has_changes = true;
                        }
                        if ui
                            .selectable_label(
                                self.weather_config.units == WeatherUnits::Imperial,
                                "Fahrenheit (°F)",
                            )
                            .clicked()
                        {
                            self.weather_config.units = WeatherUnits::Imperial;
                            self.has_changes = true;
                        }
                    });

                    ui.add_space(8.0);

                    // Location settings
                    ui.label(RichText::new("Location").strong());
                    ui.add_space(4.0);

                    egui::Grid::new("weather_location_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            // Latitude
                            ui.label("Latitude:");
                            let lat_response = ui.add(
                                egui::TextEdit::singleline(&mut self.weather_lat_input)
                                    .desired_width(100.0),
                            );
                            if lat_response.changed() {
                                if let Ok(lat) = self.weather_lat_input.parse::<f64>() {
                                    if (-90.0..=90.0).contains(&lat) {
                                        self.weather_config.latitude = lat;
                                        self.has_changes = true;
                                    }
                                }
                            }
                            ui.end_row();

                            // Longitude
                            ui.label("Longitude:");
                            let lon_response = ui.add(
                                egui::TextEdit::singleline(&mut self.weather_lon_input)
                                    .desired_width(100.0),
                            );
                            if lon_response.changed() {
                                if let Ok(lon) = self.weather_lon_input.parse::<f64>() {
                                    if (-180.0..=180.0).contains(&lon) {
                                        self.weather_config.longitude = lon;
                                        self.has_changes = true;
                                    }
                                }
                            }
                            ui.end_row();
                        });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Tip: Search for your city on maps.google.com and copy coordinates from the URL")
                            .weak()
                            .small(),
                    );

                    ui.add_space(8.0);

                    // API settings
                    ui.label(RichText::new("API Settings").strong());
                    ui.add_space(4.0);

                    // API key status
                    ui.horizontal(|ui| {
                        ui.label("API Key:");
                        if self.weather_config.api_key_configured {
                            ui.label(
                                RichText::new("Configured")
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        } else {
                            ui.label(
                                RichText::new("Not configured")
                                    .color(Color32::from_rgb(234, 67, 53)),
                            );
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Get a free API key from openweathermap.org")
                            .weak()
                            .small(),
                    );

                    ui.add_space(8.0);

                    // Refresh interval
                    ui.horizontal(|ui| {
                        ui.label("Refresh interval:");
                        let mut refresh = self.weather_config.refresh_interval_minutes as i32;
                        if ui
                            .add(
                                egui::Slider::new(&mut refresh, 10..=120)
                                    .suffix(" min"),
                            )
                            .on_hover_text("How often to fetch updated weather data")
                            .changed()
                        {
                            self.weather_config.refresh_interval_minutes = refresh as u32;
                            self.has_changes = true;
                        }
                    });
                });
            }
        });
    }

    /// Render the immersion effects section.
    /// T146: Add immersion effect toggles to settings.
    fn render_immersion_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Immersion Effects").size(18.0).strong());
            ui.add_space(8.0);

            // Visual effects group
            ui.label(RichText::new("Visual Effects").strong());
            ui.add_space(4.0);

            // Master visual toggle
            if ui
                .checkbox(
                    &mut self.immersion_settings.visual_effects_enabled,
                    "Enable visual effects",
                )
                .on_hover_text("Effort-based visual feedback during riding")
                .changed()
            {
                self.has_changes = true;
            }

            // Sub-options (indented, disabled if master is off)
            ui.add_enabled_ui(self.immersion_settings.visual_effects_enabled, |ui| {
                ui.indent("visual_sub", |ui| {
                    if ui
                        .checkbox(
                            &mut self.immersion_settings.vignette_enabled,
                            "Vignette effect",
                        )
                        .on_hover_text("Darkening around screen edges at high effort")
                        .changed()
                    {
                        self.has_changes = true;
                    }

                    if ui
                        .checkbox(
                            &mut self.immersion_settings.color_grading_enabled,
                            "Color grading",
                        )
                        .on_hover_text("Color shifts and desaturation at extreme effort")
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });
            });

            ui.add_space(12.0);

            // Audio effects group
            ui.label(RichText::new("Audio Effects").strong());
            ui.add_space(4.0);

            // Master audio toggle
            if ui
                .checkbox(
                    &mut self.immersion_settings.audio_effects_enabled,
                    "Enable audio effects",
                )
                .on_hover_text("Contextual audio feedback during riding")
                .changed()
            {
                self.has_changes = true;
            }

            // Audio volume slider
            ui.add_enabled_ui(self.immersion_settings.audio_effects_enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Volume:");
                    if ui
                        .add(
                            egui::Slider::new(&mut self.immersion_settings.audio_volume, 0.0..=1.0)
                                .show_value(true)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        )
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });

                ui.indent("audio_sub", |ui| {
                    // Effort sounds
                    ui.label(RichText::new("Effort Sounds").small().weak());
                    if ui
                        .checkbox(
                            &mut self.immersion_settings.breathing_sounds_enabled,
                            "Breathing sounds",
                        )
                        .on_hover_text("Heavy breathing at high effort (>75% FTP)")
                        .changed()
                    {
                        self.has_changes = true;
                    }

                    if ui
                        .checkbox(
                            &mut self.immersion_settings.heartbeat_sounds_enabled,
                            "Heartbeat sounds",
                        )
                        .on_hover_text("Heartbeat at very high effort (>95% FTP)")
                        .changed()
                    {
                        self.has_changes = true;
                    }

                    ui.add_space(4.0);

                    // Environment sounds
                    ui.label(RichText::new("Environment").small().weak());
                    if ui
                        .checkbox(
                            &mut self.immersion_settings.environment_audio_enabled,
                            "Environmental sounds",
                        )
                        .on_hover_text("Wind, rain, birds, and other ambient sounds")
                        .changed()
                    {
                        self.has_changes = true;
                    }

                    if ui
                        .checkbox(
                            &mut self.immersion_settings.cyclist_audio_enabled,
                            "Cycling sounds",
                        )
                        .on_hover_text("Tire roll and drivetrain sounds based on speed")
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });
            });
        });
    }

    /// Render the audio alerts settings section.
    /// T064: Add audio alert settings to settings screen.
    fn render_audio_alerts_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Voice Alerts").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_audio_alerts {
                            "Hide"
                        } else {
                            "Show"
                        })
                        .clicked()
                    {
                        self.show_audio_alerts = !self.show_audio_alerts;
                    }
                });
            });

            ui.add_space(8.0);

            // Master voice alerts toggle
            if ui
                .checkbox(
                    &mut self.audio_alert_settings.voice_alerts_enabled,
                    "Enable voice alerts",
                )
                .on_hover_text("Spoken alerts during workouts and rides")
                .changed()
            {
                self.has_changes = true;
            }

            // Voice settings (shown when enabled)
            ui.add_enabled_ui(self.audio_alert_settings.voice_alerts_enabled, |ui| {
                ui.add_space(8.0);

                // Volume slider
                ui.horizontal(|ui| {
                    ui.label("Volume:");
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut self.audio_alert_settings.voice_volume,
                                0.0..=1.0,
                            )
                            .show_value(true)
                            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        )
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });

                // Speech rate slider
                ui.horizontal(|ui| {
                    ui.label("Speech rate:");
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut self.audio_alert_settings.speech_rate,
                                0.5..=2.0,
                            )
                            .show_value(true)
                            .custom_formatter(|v, _| format!("{:.1}x", v)),
                        )
                        .on_hover_text("Speed of voice alerts (1.0x = normal)")
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });

                if self.show_audio_alerts {
                    ui.add_space(12.0);

                    // Alert categories
                    ui.label(RichText::new("Alert Categories").strong());
                    ui.add_space(4.0);

                    // Workout alerts
                    ui.indent("audio_alerts_sub", |ui| {
                        if ui
                            .checkbox(
                                &mut self.audio_alert_settings.workout_alerts_enabled,
                                "Workout alerts",
                            )
                            .on_hover_text("Start, interval changes, countdown, complete")
                            .changed()
                        {
                            self.has_changes = true;
                        }

                        // Countdown threshold
                        ui.add_enabled_ui(self.audio_alert_settings.workout_alerts_enabled, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(20.0);
                                ui.label("Countdown from:");
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.audio_alert_settings.countdown_threshold_secs,
                                            3..=30,
                                        )
                                        .suffix("s"),
                                    )
                                    .changed()
                                {
                                    self.has_changes = true;
                                }
                            });
                        });

                        if ui
                            .checkbox(
                                &mut self.audio_alert_settings.zone_alerts_enabled,
                                "Zone change alerts",
                            )
                            .on_hover_text("Power zone and heart rate zone changes")
                            .changed()
                        {
                            self.has_changes = true;
                        }

                        // Zone debounce
                        ui.add_enabled_ui(self.audio_alert_settings.zone_alerts_enabled, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(20.0);
                                ui.label("Minimum interval:");
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut self.audio_alert_settings.zone_debounce_secs,
                                            1..=30,
                                        )
                                        .suffix("s"),
                                    )
                                    .on_hover_text("Prevent rapid zone change announcements")
                                    .changed()
                                {
                                    self.has_changes = true;
                                }
                            });
                        });

                        if ui
                            .checkbox(
                                &mut self.audio_alert_settings.sensor_alerts_enabled,
                                "Sensor alerts",
                            )
                            .on_hover_text("Connection, disconnection, low battery")
                            .changed()
                        {
                            self.has_changes = true;
                        }

                        if ui
                            .checkbox(
                                &mut self.audio_alert_settings.achievement_alerts_enabled,
                                "Achievement alerts",
                            )
                            .on_hover_text("Personal records, milestones, achievements")
                            .changed()
                        {
                            self.has_changes = true;
                        }
                    });
                }
            });
        });
    }

    /// Render the incline/slope mode settings section.
    /// T042: Update settings UI for incline mode.
    fn render_incline_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Incline/Slope Mode").size(18.0).strong());
            ui.add_space(8.0);

            // Master enable toggle
            if ui
                .checkbox(
                    &mut self.incline_config.enabled,
                    "Enable incline simulation",
                )
                .on_hover_text("Simulate gradient resistance on compatible smart trainers")
                .changed()
            {
                self.has_changes = true;
            }

            // Sub-options (indented, disabled if master is off)
            ui.add_enabled_ui(self.incline_config.enabled, |ui| {
                ui.add_space(8.0);

                // Intensity slider
                ui.horizontal(|ui| {
                    ui.label("Intensity:");
                    if ui
                        .add(
                            egui::Slider::new(&mut self.incline_config.intensity, 0.5..=1.5)
                                .show_value(true)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                        )
                        .on_hover_text("Scale gradient effect (50% = half, 150% = enhanced)")
                        .changed()
                    {
                        self.has_changes = true;
                    }
                });

                ui.add_space(8.0);

                // Weight settings
                ui.label(RichText::new("Weight Settings").strong());
                ui.add_space(4.0);

                egui::Grid::new("incline_weight_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        // Rider weight
                        ui.label("Rider weight (kg):");
                        let rider_response = ui.add(
                            egui::TextEdit::singleline(&mut self.incline_rider_weight_input)
                                .desired_width(80.0),
                        );
                        if rider_response.changed() {
                            self.has_changes = true;
                            if let Ok(weight) = self.incline_rider_weight_input.parse::<f32>() {
                                if (30.0..=200.0).contains(&weight) {
                                    self.incline_config.rider_weight_kg = weight;
                                }
                            }
                        }
                        ui.end_row();

                        // Bike weight
                        ui.label("Bike weight (kg):");
                        let bike_response = ui.add(
                            egui::TextEdit::singleline(&mut self.incline_bike_weight_input)
                                .desired_width(80.0),
                        );
                        if bike_response.changed() {
                            self.has_changes = true;
                            if let Ok(weight) = self.incline_bike_weight_input.parse::<f32>() {
                                if (5.0..=30.0).contains(&weight) {
                                    self.incline_config.bike_weight_kg = weight;
                                }
                            }
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);

                // Gradient limits
                ui.label(RichText::new("Gradient Limits").strong());
                ui.add_space(4.0);

                egui::Grid::new("incline_limits_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        // Max gradient
                        ui.label("Maximum gradient (%):");
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut self.incline_config.max_gradient,
                                    5.0..=25.0,
                                )
                                .show_value(true)
                                .suffix("%"),
                            )
                            .on_hover_text("Maximum uphill gradient to simulate")
                            .changed()
                        {
                            self.has_changes = true;
                        }
                        ui.end_row();

                        // Min gradient (downhill)
                        ui.label("Minimum gradient (%):");
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut self.incline_config.min_gradient,
                                    -15.0..=0.0,
                                )
                                .show_value(true)
                                .suffix("%"),
                            )
                            .on_hover_text("Maximum downhill gradient (negative)")
                            .changed()
                        {
                            self.has_changes = true;
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);

                // Advanced settings
                ui.label(RichText::new("Advanced").strong());
                ui.add_space(4.0);

                // Smoothing duration
                ui.horizontal(|ui| {
                    ui.label("Smoothing:");
                    let mut smoothing_sec =
                        self.incline_config.smoothing_duration_ms as f32 / 1000.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut smoothing_sec, 0.5..=5.0)
                                .show_value(true)
                                .suffix("s"),
                        )
                        .on_hover_text("Transition time between gradient changes")
                        .changed()
                    {
                        self.incline_config.smoothing_duration_ms = (smoothing_sec * 1000.0) as u32;
                        self.has_changes = true;
                    }
                });

                // Enable downhill toggle
                if ui
                    .checkbox(
                        &mut self.incline_config.enable_downhill,
                        "Enable downhill simulation",
                    )
                    .on_hover_text("Reduce resistance on descents (trainer support required)")
                    .changed()
                {
                    self.has_changes = true;
                }
            });
        });
    }

    /// Render the MQTT settings section.
    /// T072: Add MQTT settings to settings screen.
    fn render_mqtt_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Smart Home (MQTT)").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_mqtt { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_mqtt = !self.show_mqtt;
                    }
                });
            });

            ui.add_space(8.0);

            // Master MQTT toggle
            if ui
                .checkbox(&mut self.mqtt_config.enabled, "Enable MQTT integration")
                .on_hover_text("Connect to MQTT broker for smart home integration")
                .changed()
            {
                self.has_changes = true;
            }

            if self.show_mqtt {
                ui.add_enabled_ui(self.mqtt_config.enabled, |ui| {
                    ui.add_space(8.0);

                    // Broker settings
                    ui.label(RichText::new("Broker Settings").strong());
                    ui.add_space(4.0);

                    egui::Grid::new("mqtt_broker_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            // Broker host
                            ui.label("Broker host:");
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut self.mqtt_config.broker_host)
                                        .desired_width(200.0),
                                )
                                .on_hover_text("MQTT broker hostname or IP address")
                                .changed()
                            {
                                self.has_changes = true;
                            }
                            ui.end_row();

                            // Broker port
                            ui.label("Port:");
                            let port_response = ui.add(
                                egui::TextEdit::singleline(&mut self.mqtt_port_input)
                                    .desired_width(80.0),
                            );
                            if port_response.changed() {
                                if let Ok(port) = self.mqtt_port_input.parse::<u16>() {
                                    self.mqtt_config.broker_port = port;
                                    self.has_changes = true;
                                }
                            }
                            ui.end_row();

                            // Username
                            ui.label("Username:");
                            let mut username_str =
                                self.mqtt_config.username.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut username_str)
                                        .desired_width(150.0),
                                )
                                .on_hover_text("Leave empty for anonymous connection")
                                .changed()
                            {
                                self.mqtt_config.username = if username_str.is_empty() {
                                    None
                                } else {
                                    Some(username_str)
                                };
                                self.has_changes = true;
                            }
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    // Security settings
                    ui.label(RichText::new("Security").strong());
                    ui.add_space(4.0);

                    // TLS toggle
                    if ui
                        .checkbox(&mut self.mqtt_config.use_tls, "Use TLS/SSL")
                        .on_hover_text("Enable encrypted connection (port 8883)")
                        .changed()
                    {
                        // Auto-update port when TLS changes
                        if self.mqtt_config.use_tls && self.mqtt_config.broker_port == 1883 {
                            self.mqtt_config.broker_port = 8883;
                            self.mqtt_port_input = "8883".to_string();
                        } else if !self.mqtt_config.use_tls && self.mqtt_config.broker_port == 8883
                        {
                            self.mqtt_config.broker_port = 1883;
                            self.mqtt_port_input = "1883".to_string();
                        }
                        self.has_changes = true;
                    }

                    ui.add_space(8.0);

                    // Connection settings
                    ui.label(RichText::new("Connection").strong());
                    ui.add_space(4.0);

                    egui::Grid::new("mqtt_connection_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            // Reconnect interval
                            ui.label("Reconnect interval:");
                            let mut reconnect = self.mqtt_config.reconnect_interval_secs as i32;
                            if ui
                                .add(egui::Slider::new(&mut reconnect, 1..=60).suffix("s"))
                                .changed()
                            {
                                self.mqtt_config.reconnect_interval_secs = reconnect as u32;
                                self.has_changes = true;
                            }
                            ui.end_row();

                            // Keep-alive
                            ui.label("Keep-alive:");
                            let mut keep_alive = self.mqtt_config.keep_alive_secs as i32;
                            if ui
                                .add(egui::Slider::new(&mut keep_alive, 10..=300).suffix("s"))
                                .changed()
                            {
                                self.mqtt_config.keep_alive_secs = keep_alive as u16;
                                self.has_changes = true;
                            }
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    // Test connection button
                    ui.horizontal(|ui| {
                        if ui
                            .button("Test Connection")
                            .on_hover_text("Test MQTT broker connection")
                            .clicked()
                        {
                            // TODO: Implement connection test
                            tracing::info!(
                                "Testing MQTT connection to {}:{}",
                                self.mqtt_config.broker_host,
                                self.mqtt_config.broker_port
                            );
                        }
                    });
                });
            }
        });
    }

    /// Render the fan profile configuration section.
    /// T073: Add fan profile configuration UI.
    fn render_fan_profiles_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Smart Fan Control").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_fan_profiles {
                            "Hide"
                        } else {
                            "Show"
                        })
                        .clicked()
                    {
                        self.show_fan_profiles = !self.show_fan_profiles;
                    }
                });
            });

            ui.add_space(8.0);

            // Show MQTT requirement if not enabled
            if !self.mqtt_config.enabled {
                ui.label(
                    RichText::new("Enable MQTT integration above to use smart fan control")
                        .weak()
                        .italics(),
                );
                return;
            }

            if self.show_fan_profiles {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Fan profiles automatically adjust fan speed based on your training zone.",
                    )
                    .weak()
                    .small(),
                );

                ui.add_space(8.0);

                // List existing profiles
                let mut remove_idx = None;
                let profiles_len = self.fan_profiles.len();

                for (idx, profile) in self.fan_profiles.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        // Profile name (editable)
                        ui.label(RichText::new(&profile.name).strong());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Remove button (only if more than one profile)
                            if profiles_len > 1
                                && ui
                                    .small_button("✕")
                                    .on_hover_text("Remove profile")
                                    .clicked()
                            {
                                remove_idx = Some(idx);
                            }

                            // Edit button
                            let editing = self.editing_fan_profile == Some(idx);
                            if ui
                                .small_button(if editing { "Done" } else { "Edit" })
                                .clicked()
                            {
                                self.editing_fan_profile = if editing { None } else { Some(idx) };
                            }
                        });
                    });

                    // Show expanded editor if editing this profile
                    if self.editing_fan_profile == Some(idx) {
                        ui.indent("fan_profile_editor", |ui| {
                            // Profile name
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut profile.name)
                                            .desired_width(150.0),
                                    )
                                    .changed()
                                {
                                    self.has_changes = true;
                                }
                            });

                            // MQTT topic
                            ui.horizontal(|ui| {
                                ui.label("MQTT topic:");
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut profile.mqtt_topic)
                                            .desired_width(250.0),
                                    )
                                    .on_hover_text("e.g., home/fan/living_room")
                                    .changed()
                                {
                                    self.has_changes = true;
                                }
                            });

                            // Use /set suffix
                            if ui
                                .checkbox(&mut profile.use_set_suffix, "Append /set to topic")
                                .on_hover_text("Common for Home Assistant MQTT topics")
                                .changed()
                            {
                                self.has_changes = true;
                            }

                            // Payload format
                            ui.horizontal(|ui| {
                                ui.label("Payload format:");
                                egui::ComboBox::from_id_salt(format!("payload_fmt_{}", idx))
                                    .selected_text(match profile.payload_format {
                                        PayloadFormat::SpeedOnly => "Speed only",
                                        PayloadFormat::JsonSpeed => "JSON {speed}",
                                        PayloadFormat::JsonSpeedOnOff => "JSON {speed, on}",
                                        PayloadFormat::Percentage => "Percentage",
                                    })
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_value(
                                                &mut profile.payload_format,
                                                PayloadFormat::SpeedOnly,
                                                "Speed only (e.g., 75)",
                                            )
                                            .clicked()
                                        {
                                            self.has_changes = true;
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut profile.payload_format,
                                                PayloadFormat::JsonSpeed,
                                                "JSON {speed} (e.g., {\"speed\": 75})",
                                            )
                                            .clicked()
                                        {
                                            self.has_changes = true;
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut profile.payload_format,
                                                PayloadFormat::JsonSpeedOnOff,
                                                "JSON {speed, on}",
                                            )
                                            .clicked()
                                        {
                                            self.has_changes = true;
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut profile.payload_format,
                                                PayloadFormat::Percentage,
                                                "Percentage (e.g., 75%)",
                                            )
                                            .clicked()
                                        {
                                            self.has_changes = true;
                                        }
                                    });
                            });

                            // Use power zones or HR zones
                            if ui
                                .checkbox(&mut profile.use_power_zones, "Use power zones")
                                .on_hover_text(
                                    "Use power zones (checked) or heart rate zones (unchecked)",
                                )
                                .changed()
                            {
                                self.has_changes = true;
                            }

                            // Change delay
                            ui.horizontal(|ui| {
                                ui.label("Change delay:");
                                let mut delay = profile.change_delay_secs as i32;
                                if ui
                                    .add(egui::Slider::new(&mut delay, 0..=30).suffix("s"))
                                    .on_hover_text("Minimum time between speed changes")
                                    .changed()
                                {
                                    profile.change_delay_secs = delay as u8;
                                    self.has_changes = true;
                                }
                            });

                            ui.add_space(8.0);

                            // Zone to speed mapping
                            ui.label(RichText::new("Zone-to-Speed Mapping").strong());
                            ui.add_space(4.0);

                            egui::Grid::new(format!("zone_speed_grid_{}", idx))
                                .num_columns(8)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    // Header row
                                    for z in 1..=7 {
                                        ui.label(RichText::new(format!("Z{}", z)).small());
                                    }
                                    ui.end_row();

                                    // Speed sliders row
                                    for z in 0..7 {
                                        let mut speed = profile.zone_speeds[z] as i32;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut speed)
                                                    .speed(1.0)
                                                    .range(0..=100)
                                                    .suffix("%"),
                                            )
                                            .changed()
                                        {
                                            profile.zone_speeds[z] = speed as u8;
                                            self.has_changes = true;
                                        }
                                    }
                                    ui.end_row();
                                });

                            ui.add_space(4.0);

                            // Quick presets
                            ui.horizontal(|ui| {
                                ui.label("Presets:");
                                if ui
                                    .small_button("Off → Full")
                                    .on_hover_text("Zone 1 = off, Zone 7 = 100%")
                                    .clicked()
                                {
                                    profile.zone_speeds = [0, 20, 40, 60, 80, 90, 100];
                                    self.has_changes = true;
                                }
                                if ui
                                    .small_button("Gradual")
                                    .on_hover_text("Gradual increase")
                                    .clicked()
                                {
                                    profile.zone_speeds = [10, 25, 40, 55, 70, 85, 100];
                                    self.has_changes = true;
                                }
                                if ui
                                    .small_button("Threshold Only")
                                    .on_hover_text("Fan on only at high zones")
                                    .clicked()
                                {
                                    profile.zone_speeds = [0, 0, 0, 25, 50, 75, 100];
                                    self.has_changes = true;
                                }
                            });
                        });
                    }

                    ui.separator();
                }

                // Handle removal
                if let Some(idx) = remove_idx {
                    self.fan_profiles.remove(idx);
                    self.has_changes = true;
                    if self.editing_fan_profile == Some(idx) {
                        self.editing_fan_profile = None;
                    }
                }

                ui.add_space(8.0);

                // Add new profile button
                if ui.button("+ Add Fan Profile").clicked() {
                    let new_profile = FanProfile {
                        id: Uuid::new_v4(),
                        name: format!("Fan {}", self.fan_profiles.len() + 1),
                        mqtt_topic: "home/fan/new".to_string(),
                        ..FanProfile::default()
                    };
                    self.fan_profiles.push(new_profile);
                    self.editing_fan_profile = Some(self.fan_profiles.len() - 1);
                    self.has_changes = true;
                }
            }
        });
    }

    /// Render the platform sync settings section.
    /// T109: Add platform connection UI for Strava/Garmin.
    fn render_sync_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Fitness Platform Sync").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_sync { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_sync = !self.show_sync;
                    }
                });
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Connect to fitness platforms to automatically sync your rides")
                    .weak()
                    .size(12.0),
            );

            if self.show_sync {
                ui.add_space(12.0);

                // Platform list
                let platforms = [
                    SyncPlatform::Strava,
                    SyncPlatform::GarminConnect,
                    SyncPlatform::TrainingPeaks,
                    SyncPlatform::IntervalsIcu,
                ];

                for platform in platforms {
                    // Get values before closure to avoid borrow conflicts
                    let auto_sync_val = self
                        .sync_config
                        .platforms
                        .get(&platform)
                        .map(|c| c.auto_sync)
                        .unwrap_or(false);
                    let is_connected = self
                        .platform_states
                        .iter()
                        .find(|(p, _)| *p == platform)
                        .map(|(_, connected)| *connected)
                        .unwrap_or(false);

                    ui.horizontal(|ui| {
                        // Platform name with icon
                        let (icon, icon_color) = match platform {
                            SyncPlatform::Strava => ("", Color32::from_rgb(252, 82, 0)),
                            SyncPlatform::GarminConnect => ("", Color32::from_rgb(0, 135, 200)),
                            SyncPlatform::TrainingPeaks => ("", Color32::from_rgb(0, 102, 51)),
                            SyncPlatform::IntervalsIcu => ("", Color32::from_rgb(255, 193, 7)),
                            #[cfg(target_os = "macos")]
                            SyncPlatform::HealthKit => ("", Color32::from_rgb(255, 59, 48)),
                        };

                        ui.label(RichText::new(icon).color(icon_color));
                        ui.label(RichText::new(platform.display_name()).size(14.0).strong());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Connect/Disconnect button
                            if is_connected {
                                ui.label(
                                    RichText::new("Connected")
                                        .color(Color32::from_rgb(52, 168, 83)),
                                );
                                if ui
                                    .small_button("Disconnect")
                                    .on_hover_text("Remove authorization")
                                    .clicked()
                                {
                                    // Mark for disconnect
                                    if let Some((_, connected)) = self
                                        .platform_states
                                        .iter_mut()
                                        .find(|(p, _)| *p == platform)
                                    {
                                        *connected = false;
                                    }
                                    self.has_changes = true;
                                }
                            } else if ui
                                .button("Connect")
                                .on_hover_text(format!("Connect to {}", platform.display_name()))
                                .clicked()
                            {
                                // TODO: Trigger OAuth flow
                                tracing::info!("Connect to {:?}", platform);
                            }

                            // Auto-sync checkbox
                            if is_connected {
                                let mut auto_sync = auto_sync_val;
                                if ui
                                    .checkbox(&mut auto_sync, "Auto-sync")
                                    .on_hover_text("Automatically sync rides after completion")
                                    .changed()
                                {
                                    if let Some(config) =
                                        self.sync_config.platforms.get_mut(&platform)
                                    {
                                        config.auto_sync = auto_sync;
                                    }
                                    self.has_changes = true;
                                }
                            }
                        });
                    });

                    ui.add_space(8.0);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new(
                        "Note: Connecting opens your browser for secure OAuth authorization.",
                    )
                    .weak()
                    .small(),
                );
            }
        });
    }

    /// Set sync configuration.
    /// T109: Sync settings management.
    pub fn set_sync_config(&mut self, config: SyncConfig) {
        self.sync_config = config;
    }

    /// Get current sync configuration.
    pub fn get_sync_config(&self) -> &SyncConfig {
        &self.sync_config
    }

    /// Update platform connected state.
    pub fn set_platform_connected(&mut self, platform: SyncPlatform, connected: bool) {
        if let Some((_, state)) = self
            .platform_states
            .iter_mut()
            .find(|(p, _)| *p == platform)
        {
            *state = connected;
        } else {
            self.platform_states.push((platform, connected));
        }
    }

    /// Get connected platforms.
    pub fn get_connected_platforms(&self) -> Vec<SyncPlatform> {
        self.platform_states
            .iter()
            .filter_map(|(p, connected)| if *connected { Some(*p) } else { None })
            .collect()
    }

    /// Validate the current profile.
    fn validate(&mut self) -> bool {
        // Validate FTP
        if let Ok(ftp) = self.ftp_input.parse::<u16>() {
            if !UserProfile::validate_ftp(ftp) {
                self.error_message = Some("FTP must be between 50 and 600 watts".to_string());
                return false;
            }
        } else {
            self.error_message = Some("Invalid FTP value".to_string());
            return false;
        }

        // Validate weight
        if let Ok(weight) = self.weight_input.parse::<f32>() {
            if !UserProfile::validate_weight(weight) {
                self.error_message = Some("Weight must be between 30 and 200 kg".to_string());
                return false;
            }
        } else {
            self.error_message = Some("Invalid weight value".to_string());
            return false;
        }

        // Validate height (optional)
        if !self.height_input.is_empty() {
            if let Ok(height) = self.height_input.parse::<u16>() {
                if !(100..=250).contains(&height) {
                    self.error_message = Some("Height must be between 100 and 250 cm".to_string());
                    return false;
                }
            } else {
                self.error_message = Some("Invalid height value".to_string());
                return false;
            }
        }

        // Validate max HR (optional)
        if !self.max_hr_input.is_empty() {
            if let Ok(hr) = self.max_hr_input.parse::<u8>() {
                if !(100..=220).contains(&hr) {
                    self.error_message = Some("Max HR must be between 100 and 220 bpm".to_string());
                    return false;
                }
            } else {
                self.error_message = Some("Invalid max HR value".to_string());
                return false;
            }
        }

        // Validate resting HR (optional)
        if !self.resting_hr_input.is_empty() {
            if let Ok(hr) = self.resting_hr_input.parse::<u8>() {
                if !(30..=100).contains(&hr) {
                    self.error_message =
                        Some("Resting HR must be between 30 and 100 bpm".to_string());
                    return false;
                }
            } else {
                self.error_message = Some("Invalid resting HR value".to_string());
                return false;
            }
        }

        // Validate name
        if self.profile.name.trim().is_empty() {
            self.error_message = Some("Name cannot be empty".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    /// Get the edited profile.
    pub fn get_profile(&self) -> &UserProfile {
        &self.profile
    }

    /// Reset to original profile.
    pub fn reset(&mut self) {
        self.profile = self.original_profile.clone();
        self.has_changes = false;
        self.error_message = None;
        self.sync_inputs();
    }

    /// Render the rider type section.
    ///
    /// T124: Add rider type display to profile screen.
    fn render_rider_type_section(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Rider Classification").size(18.0).strong());
            ui.add_space(8.0);

            if let Some(rider_type) = &self.rider_type {
                // Rider type display
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ui.label(
                        RichText::new(rider_type.name())
                            .strong()
                            .color(Color32::from_rgb(66, 133, 244)),
                    );
                });

                ui.add_space(4.0);

                // Description
                ui.label(RichText::new(rider_type.description()).weak().italics());

                ui.add_space(8.0);

                // Training focus recommendation
                ui.label("Suggested focus:");
                ui.label(
                    RichText::new(rider_type.training_focus())
                        .color(Color32::from_rgb(52, 168, 83)),
                );

                // Power profile if available
                if let Some(profile) = &self.power_profile {
                    ui.add_space(12.0);
                    ui.label(RichText::new("Power Profile").strong());
                    ui.add_space(4.0);

                    egui::Grid::new("power_profile_grid")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Neuromuscular (5s):");
                            self.render_profile_bar(ui, profile.neuromuscular);
                            ui.end_row();

                            ui.label("Anaerobic (1min):");
                            self.render_profile_bar(ui, profile.anaerobic);
                            ui.end_row();

                            ui.label("VO2max (5min):");
                            self.render_profile_bar(ui, profile.vo2max);
                            ui.end_row();

                            ui.label("Threshold (20min):");
                            self.render_profile_bar(ui, profile.threshold);
                            ui.end_row();
                        });
                }
            } else {
                ui.label(
                    RichText::new("Rider classification requires more ride data")
                        .weak()
                        .italics(),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Record rides with varied efforts to build your power profile")
                        .weak()
                        .small(),
                );
            }
        });
    }

    /// Render a power profile bar.
    fn render_profile_bar(&self, ui: &mut Ui, value: f32) {
        let width = 150.0;
        let height = 12.0;

        // Background
        let (rect, _response) =
            ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::hover());

        ui.painter().rect_filled(rect, 2.0, Color32::from_gray(60));

        // Fill based on value (0.0-1.5 typical range, 1.0 = average)
        let fill_width = (value.min(1.5) / 1.5 * width).max(0.0);
        let fill_color = if value > 1.1 {
            Color32::from_rgb(52, 168, 83) // Green for strength
        } else if value > 0.9 {
            Color32::from_rgb(66, 133, 244) // Blue for average
        } else {
            Color32::from_rgb(234, 67, 53) // Red for weakness
        };

        let fill_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, height));
        ui.painter().rect_filled(fill_rect, 2.0, fill_color);

        // Value label
        ui.label(RichText::new(format!("{:.2}", value)).small());
    }

    /// Render the HID device settings section.
    /// T092: Add HID device list and button mapping UI with learning mode.
    fn render_hid_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("USB Button Devices").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_hid { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_hid = !self.show_hid;
                    }
                });
            });

            ui.add_space(8.0);

            // Master HID toggle
            if ui
                .checkbox(
                    &mut self.hid_settings.enabled,
                    "Enable USB button device support",
                )
                .on_hover_text("Use USB HID devices like Stream Deck for button control")
                .changed()
            {
                self.has_changes = true;
            }

            if self.show_hid {
                ui.add_enabled_ui(self.hid_settings.enabled, |ui| {
                    ui.add_space(8.0);

                    // Device list
                    self.render_hid_device_list(ui);

                    ui.add_space(12.0);

                    // Button mappings for selected device
                    self.render_hid_button_mappings(ui);
                });
            }
        });
    }

    /// Render the list of detected HID devices.
    fn render_hid_device_list(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Detected Devices").strong());
        ui.add_space(4.0);

        if self.hid_settings.devices.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("No USB button devices detected")
                        .weak()
                        .italics(),
                );
                if ui
                    .small_button("Scan")
                    .on_hover_text("Scan for connected USB devices")
                    .clicked()
                {
                    // TODO: Trigger device scan via app
                    tracing::info!("Scanning for HID devices");
                }
            });
        } else {
            for device in self.hid_settings.devices.clone() {
                ui.horizontal(|ui| {
                    // Device status indicator
                    let (status_color, status_text) = match &device.status {
                        HidDeviceStatus::Detected => (Color32::from_rgb(255, 193, 7), "Detected"),
                        HidDeviceStatus::Opening => (Color32::from_rgb(66, 133, 244), "Opening..."),
                        HidDeviceStatus::Open => (Color32::from_rgb(52, 168, 83), "Connected"),
                        HidDeviceStatus::Error(_) => (Color32::from_rgb(234, 67, 53), "Error"),
                        HidDeviceStatus::Disconnected => {
                            (Color32::from_rgb(158, 158, 158), "Disconnected")
                        }
                    };

                    ui.label(RichText::new("●").color(status_color));

                    // Device name and path
                    let is_selected = self.hid_settings.selected_device == Some(device.id);
                    let device_label = if device.is_known {
                        RichText::new(&device.name).strong()
                    } else {
                        RichText::new(&device.name)
                    };

                    if ui
                        .selectable_label(is_selected, device_label)
                        .on_hover_text(format!(
                            "{} ({})\nStatus: {}",
                            device.name,
                            device.display_path(),
                            status_text
                        ))
                        .clicked()
                    {
                        self.hid_settings.selected_device = Some(device.id);
                    }

                    // Button count if known
                    if let Some(count) = device.button_count {
                        ui.label(RichText::new(format!("{} buttons", count)).weak().small());
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Enable/disable toggle
                        let config = self.hid_settings.get_device_config(&device.id);
                        let mut enabled = config.map(|c| c.enabled).unwrap_or(true);
                        if ui
                            .checkbox(&mut enabled, "")
                            .on_hover_text("Enable this device")
                            .changed()
                        {
                            let device_clone = device.clone();
                            let cfg = self.hid_settings.get_or_create_device_config(&device_clone);
                            cfg.enabled = enabled;
                            self.has_changes = true;
                        }
                    });
                });
            }

            ui.add_space(4.0);
            if ui
                .small_button("Refresh")
                .on_hover_text("Refresh device list")
                .clicked()
            {
                tracing::info!("Refreshing HID device list");
            }
        }
    }

    /// Render button mappings for the selected device.
    fn render_hid_button_mappings(&mut self, ui: &mut Ui) {
        let Some(selected_id) = self.hid_settings.selected_device else {
            ui.label(RichText::new("Select a device to configure button mappings").weak());
            return;
        };

        let device = self
            .hid_settings
            .devices
            .iter()
            .find(|d| d.id == selected_id)
            .cloned();

        let Some(device) = device else {
            return;
        };

        ui.label(
            RichText::new(format!("Button Mappings - {}", device.name))
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);

        // Learning mode indicator
        if self.hid_settings.learning_mode {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new("Press a button on the device...")
                        .color(Color32::from_rgb(66, 133, 244)),
                );
                if ui.small_button("Cancel").clicked() {
                    self.hid_settings.learning_mode = false;
                    self.hid_settings.learning_target = None;
                }
            });

            // Check if button was learned
            if let Some(button_code) = self.hid_settings.learned_button_code.take() {
                self.hid_settings.learning_mode = false;
                self.hid_settings.selecting_action_for = Some((selected_id, button_code));
            }

            ui.add_space(8.0);
        }

        // Action selection modal
        if let Some((_device_id, button_code)) = self.hid_settings.selecting_action_for {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!("Select action for Button #{}", button_code)).strong(),
                );
                ui.add_space(4.0);

                // Show action categories
                let actions = ButtonAction::all_actions();
                let mut selected_action = None;

                egui::Grid::new("action_selection_grid")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for (idx, action) in actions.iter().enumerate() {
                            if ui.button(action.display_name()).clicked() {
                                selected_action = Some(action.clone());
                            }
                            if (idx + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(4.0);
                if ui.button("Cancel").clicked() {
                    self.hid_settings.selecting_action_for = None;
                }

                // Add mapping if action selected
                if let Some(action) = selected_action {
                    let device_clone = device.clone();
                    let config = self.hid_settings.get_or_create_device_config(&device_clone);
                    // Remove existing mapping for this button if any
                    config.mappings.retain(|m| m.button_code != button_code);
                    // Add new mapping
                    config.mappings.push(crate::hid::ButtonMappingConfig {
                        button_code,
                        action,
                        label: None,
                    });
                    self.hid_settings.selecting_action_for = None;
                    self.has_changes = true;
                }
            });

            ui.add_space(8.0);
        }

        // Current mappings table
        let config = self.hid_settings.get_device_config(&selected_id).cloned();
        let mappings = config.map(|c| c.mappings).unwrap_or_default();

        if mappings.is_empty() && !self.hid_settings.learning_mode {
            ui.label(
                RichText::new("No button mappings configured")
                    .weak()
                    .italics(),
            );
        } else {
            egui::Grid::new("button_mappings_grid")
                .num_columns(4)
                .striped(true)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Button").strong());
                    ui.label(RichText::new("Action").strong());
                    ui.label(RichText::new("Label").strong());
                    ui.label(""); // Actions column
                    ui.end_row();

                    let mut remove_button: Option<u8> = None;
                    for mapping in &mappings {
                        ui.label(format!("#{}", mapping.button_code));
                        ui.label(mapping.action.display_name());
                        ui.label(mapping.label.as_deref().unwrap_or("-"));

                        if ui
                            .small_button("✕")
                            .on_hover_text("Remove mapping")
                            .clicked()
                        {
                            remove_button = Some(mapping.button_code);
                        }
                        ui.end_row();
                    }

                    // Handle removal
                    if let Some(button_code) = remove_button {
                        let device_clone = device.clone();
                        let cfg = self.hid_settings.get_or_create_device_config(&device_clone);
                        cfg.mappings.retain(|m| m.button_code != button_code);
                        self.has_changes = true;
                    }
                });
        }

        ui.add_space(8.0);

        // Add mapping button
        if !self.hid_settings.learning_mode
            && self.hid_settings.selecting_action_for.is_none()
            && ui
                .button("+ Add Button Mapping")
                .on_hover_text("Press a button on the device to map it to an action")
                .clicked()
        {
            self.hid_settings.learning_mode = true;
            self.hid_settings.learning_target = Some((selected_id, 0));
            self.hid_settings.learned_button_code = None;
            // TODO: Signal to HID handler to start learning mode
            tracing::info!("Started button learning mode for device {:?}", selected_id);
        }
    }

    /// Set HID configuration.
    /// T092: HID settings management.
    pub fn set_hid_config(&mut self, config: HidConfig) {
        self.hid_settings = HidSettings::from_config(&config);
    }

    /// Get current HID configuration.
    pub fn get_hid_config(&self) -> HidConfig {
        self.hid_settings.to_config()
    }

    /// Update detected HID devices.
    pub fn set_hid_devices(&mut self, devices: Vec<HidDevice>) {
        self.hid_settings.set_devices(devices);
    }

    /// Set learned button code from HID handler.
    pub fn set_learned_button(&mut self, button_code: u8) {
        if self.hid_settings.learning_mode {
            self.hid_settings.learned_button_code = Some(button_code);
        }
    }

    /// Render the accessibility settings section.
    /// T047, T060, T065, T113, T131: Comprehensive accessibility settings UI.
    fn render_accessibility_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Accessibility").size(18.0).strong());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(if self.show_accessibility {
                            "Hide"
                        } else {
                            "Show"
                        })
                        .clicked()
                    {
                        self.show_accessibility = !self.show_accessibility;
                    }
                });
            });

            ui.add_space(8.0);

            if self.show_accessibility {
                // T065: Theme preference
                ui.label(RichText::new("Theme").strong());
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.theme_preference == ThemePreference::FollowSystem,
                            "Follow System",
                        )
                        .on_hover_text("Automatically match your system's light/dark mode")
                        .clicked()
                    {
                        self.theme_preference = ThemePreference::FollowSystem;
                        self.has_changes = true;
                    }
                    if ui
                        .selectable_label(self.theme_preference == ThemePreference::Light, "Light")
                        .clicked()
                    {
                        self.theme_preference = ThemePreference::Light;
                        self.has_changes = true;
                    }
                    if ui
                        .selectable_label(self.theme_preference == ThemePreference::Dark, "Dark")
                        .clicked()
                    {
                        self.theme_preference = ThemePreference::Dark;
                        self.has_changes = true;
                    }
                });

                ui.add_space(12.0);

                // T047: Colorblind mode
                ui.label(RichText::new("Color Vision").strong());
                ui.horizontal(|ui| {
                    let modes = [
                        ("Normal", "normal"),
                        ("Protanopia", "protanopia"),
                        ("Deuteranopia", "deuteranopia"),
                        ("Tritanopia", "tritanopia"),
                    ];
                    for (label, mode) in modes {
                        if ui
                            .selectable_label(self.accessibility_settings.color_mode == mode, label)
                            .clicked()
                        {
                            self.accessibility_settings.color_mode = mode.to_string();
                            self.has_changes = true;
                        }
                    }
                });

                ui.add_space(8.0);

                // T047: High contrast mode
                if ui
                    .checkbox(
                        &mut self.accessibility_settings.high_contrast,
                        "High Contrast Mode",
                    )
                    .on_hover_text("WCAG AAA compliant 7:1 contrast ratio")
                    .changed()
                {
                    self.has_changes = true;
                }

                // Reduce motion
                if ui
                    .checkbox(
                        &mut self.accessibility_settings.reduce_motion,
                        "Reduce Motion",
                    )
                    .on_hover_text("Minimize animations for motion sensitivity")
                    .changed()
                {
                    self.has_changes = true;
                }

                ui.add_space(12.0);

                // T113: Language selection
                ui.label(RichText::new("Language").strong());
                let languages = [
                    ("English", "en-US"),
                    ("Español", "es"),
                    ("Français", "fr"),
                    ("Deutsch", "de"),
                    ("Italiano", "it"),
                ];
                ui.horizontal(|ui| {
                    for (label, code) in languages {
                        if ui
                            .selectable_label(self.locale_settings.language == code, label)
                            .clicked()
                        {
                            self.locale_settings.language = code.to_string();
                            self.has_changes = true;
                        }
                    }
                });

                ui.add_space(12.0);

                // T131: Voice control settings
                ui.label(RichText::new("Voice Control").strong());
                if ui
                    .checkbox(
                        &mut self.accessibility_settings.voice_control_enabled,
                        "Enable Voice Control",
                    )
                    .on_hover_text(
                        "Control the app with voice commands (start, pause, resume, end, skip)",
                    )
                    .changed()
                {
                    self.has_changes = true;
                }

                // Screen reader support
                if ui
                    .checkbox(
                        &mut self.accessibility_settings.screen_reader_enabled,
                        "Screen Reader Optimizations",
                    )
                    .on_hover_text("Optimize for NVDA, VoiceOver, and Orca screen readers")
                    .changed()
                {
                    self.has_changes = true;
                }

                ui.add_space(12.0);

                // T092: TV Mode settings
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("TV Mode").strong());
                ui.label(
                    RichText::new("Optimized for large displays (65\"+) at 3+ meter distance")
                        .color(Color32::GRAY)
                        .small(),
                );

                if ui
                    .checkbox(&mut self.tv_mode_enabled, "Enable TV Mode")
                    .on_hover_text("Enlarge fonts and simplify layout for viewing from a distance")
                    .changed()
                {
                    self.has_changes = true;
                }

                if self.tv_mode_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Font Scale:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.tv_mode_font_scale, 1.5..=3.0)
                                    .text("x")
                                    .step_by(0.1),
                            )
                            .changed()
                        {
                            self.has_changes = true;
                        }
                    });
                }

                ui.add_space(12.0);

                // T060: Restart onboarding
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Onboarding").strong());
                if ui
                    .button("Restart Onboarding Tutorial")
                    .on_hover_text("Start the onboarding wizard again to re-configure your setup")
                    .clicked()
                {
                    self.restart_onboarding_requested = true;
                    self.has_changes = true;
                }

                if self.restart_onboarding_requested {
                    ui.label(
                        RichText::new("Onboarding will restart when you save settings")
                            .color(Color32::from_rgb(251, 188, 4))
                            .small(),
                    );
                }
            }
        });
    }

    /// Set accessibility configuration.
    pub fn set_accessibility_config(&mut self, config: AccessibilitySettings) {
        self.accessibility_settings = config;
    }

    /// Get current accessibility configuration.
    pub fn get_accessibility_config(&self) -> &AccessibilitySettings {
        &self.accessibility_settings
    }

    /// Set locale configuration.
    pub fn set_locale_config(&mut self, config: LocaleSettings) {
        self.locale_settings = config;
    }

    /// Get current locale configuration.
    pub fn get_locale_config(&self) -> &LocaleSettings {
        &self.locale_settings
    }

    /// Check if onboarding restart was requested.
    pub fn should_restart_onboarding(&self) -> bool {
        self.restart_onboarding_requested
    }

    /// Clear the restart onboarding flag.
    pub fn clear_restart_onboarding(&mut self) {
        self.restart_onboarding_requested = false;
    }
}
