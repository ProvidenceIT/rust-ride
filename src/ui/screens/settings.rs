//! Settings screen implementation.
//!
//! T120: Create settings screen with profile section
//! T121: Implement FTP, max HR, resting HR, weight, height inputs
//! T122: Implement power zone editor with auto-calculate toggle
//! T123: Implement HR zone editor
//! T124: Implement unit preference toggle (metric/imperial)
//! T125: Implement theme toggle (dark/light)

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui};

use crate::metrics::zones::{HRZones, PowerZones};
use crate::storage::config::{Theme, Units, UserProfile};


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
        let resting_hr_input = profile.resting_hr.map(|v| v.to_string()).unwrap_or_default();
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
        }
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
        self.max_hr_input = self.profile.max_hr.map(|v| v.to_string()).unwrap_or_default();
        self.resting_hr_input = self.profile.resting_hr.map(|v| v.to_string()).unwrap_or_default();
        self.weight_input = format!("{:.1}", self.profile.weight_kg);
        self.height_input = self.profile.height_cm.map(|v| v.to_string()).unwrap_or_default();
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
                {
                    if self.validate() {
                        action = SettingsAction::Save;
                    }
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
                ui.label(RichText::new(format!("⚠ {}", error)).color(Color32::from_rgb(234, 67, 53)));
            });
            ui.add_space(8.0);
        }

        // Scrollable content
        ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Profile section
            self.render_profile_section(ui);

            ui.add_space(16.0);

            // Power zones section
            self.render_power_zones_section(ui);

            ui.add_space(16.0);

            // HR zones section
            self.render_hr_zones_section(ui);

            ui.add_space(16.0);

            // Preferences section
            self.render_preferences_section(ui);

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
                        egui::TextEdit::singleline(&mut self.profile.name)
                            .desired_width(200.0),
                    );
                    if name_response.changed() {
                        self.has_changes = true;
                    }
                    ui.end_row();

                    // FTP
                    ui.label("FTP (watts):");
                    let ftp_response = ui.add(
                        egui::TextEdit::singleline(&mut self.ftp_input)
                            .desired_width(100.0),
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
                                self.error_message = Some("FTP must be between 50 and 600 watts".to_string());
                            }
                        }
                    }
                    ui.end_row();

                    // Weight
                    ui.label("Weight (kg):");
                    let weight_response = ui.add(
                        egui::TextEdit::singleline(&mut self.weight_input)
                            .desired_width(100.0),
                    );
                    if weight_response.changed() {
                        self.has_changes = true;
                        if let Ok(weight) = self.weight_input.parse::<f32>() {
                            if UserProfile::validate_weight(weight) {
                                self.profile.weight_kg = weight;
                                self.error_message = None;
                            } else {
                                self.error_message = Some("Weight must be between 30 and 200 kg".to_string());
                            }
                        }
                    }
                    ui.end_row();

                    // Height
                    ui.label("Height (cm):");
                    let height_response = ui.add(
                        egui::TextEdit::singleline(&mut self.height_input)
                            .desired_width(100.0),
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
                                self.error_message = Some("Height must be between 100 and 250 cm".to_string());
                            }
                        }
                    }
                    ui.end_row();

                    // Max HR
                    ui.label("Max HR (bpm):");
                    let max_hr_response = ui.add(
                        egui::TextEdit::singleline(&mut self.max_hr_input)
                            .desired_width(100.0),
                    );
                    if max_hr_response.changed() {
                        self.has_changes = true;
                        self.update_hr_zones();
                    }
                    ui.end_row();

                    // Resting HR
                    ui.label("Resting HR (bpm):");
                    let resting_hr_response = ui.add(
                        egui::TextEdit::singleline(&mut self.resting_hr_input)
                            .desired_width(100.0),
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
                    if ui.button(if self.show_power_zones { "Hide" } else { "Show" }).clicked() {
                        self.show_power_zones = !self.show_power_zones;
                    }
                });
            });

            ui.add_space(4.0);

            // Auto-calculate checkbox
            if ui
                .checkbox(&mut self.auto_calculate_power_zones, "Auto-calculate from FTP")
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
                    if ui.button(if self.show_hr_zones { "Hide" } else { "Show" }).clicked() {
                        self.show_hr_zones = !self.show_hr_zones;
                    }
                });
            });

            if self.profile.hr_zones.is_none() {
                ui.add_space(4.0);
                ui.label(RichText::new("Enter Max HR and Resting HR to calculate zones").weak().italics());
            } else {
                ui.add_space(4.0);

                // Auto-calculate checkbox
                if ui
                    .checkbox(&mut self.auto_calculate_hr_zones, "Auto-calculate from Max/Resting HR")
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
                        let hrr = self.profile.max_hr.unwrap_or(180) - self.profile.resting_hr.unwrap_or(60);
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
                            .selectable_label(self.profile.units == Units::Metric, "Metric (km, kg)")
                            .clicked()
                        {
                            self.profile.units = Units::Metric;
                            self.has_changes = true;
                        }
                        if ui
                            .selectable_label(self.profile.units == Units::Imperial, "Imperial (mi, lbs)")
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
                    self.error_message = Some("Resting HR must be between 30 and 100 bpm".to_string());
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
}
