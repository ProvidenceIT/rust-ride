//! Ride summary screen implementation.
//!
//! T101: Create ride summary screen with stats display
//! T102: Implement export button with format selection
//! T103: Implement save/discard controls
//! T108: Add sync button with platform selection

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::sync::{SyncPlatform, SyncRecordStatus};
use crate::recording::types::{ExportFormat, Ride, RideSample};

/// Ride summary screen state.
pub struct RideSummaryScreen {
    /// The completed ride
    pub ride: Option<Ride>,
    /// Ride samples for export
    pub samples: Vec<RideSample>,
    /// Show export dialog
    pub show_export_dialog: bool,
    /// Selected export format
    pub export_format: ExportFormat,
    /// Export status message
    pub export_status: Option<String>,
    /// Whether the ride has been saved
    pub is_saved: bool,
    /// Notes text
    pub notes: String,
    /// T108: Show sync dialog
    pub show_sync_dialog: bool,
    /// T108: Selected platforms for sync
    pub selected_platforms: Vec<SyncPlatform>,
    /// T108: Sync status for each platform
    pub sync_status: Vec<(SyncPlatform, SyncRecordStatus)>,
    /// T108: Connected/authorized platforms
    pub connected_platforms: Vec<SyncPlatform>,
}

impl Default for RideSummaryScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RideSummaryScreen {
    /// Create a new ride summary screen.
    pub fn new() -> Self {
        Self {
            ride: None,
            samples: Vec::new(),
            show_export_dialog: false,
            export_format: ExportFormat::Tcx,
            export_status: None,
            is_saved: false,
            notes: String::new(),
            show_sync_dialog: false,
            selected_platforms: Vec::new(),
            sync_status: Vec::new(),
            connected_platforms: Vec::new(),
        }
    }

    /// Set the ride data to display.
    pub fn set_ride(&mut self, ride: Ride, samples: Vec<RideSample>) {
        self.notes = ride.notes.clone().unwrap_or_default();
        self.ride = Some(ride);
        self.samples = samples;
        self.is_saved = false;
        self.export_status = None;
    }

    /// Clear the screen.
    pub fn clear(&mut self) {
        self.ride = None;
        self.samples.clear();
        self.show_export_dialog = false;
        self.export_status = None;
        self.is_saved = false;
        self.notes.clear();
        self.show_sync_dialog = false;
        self.selected_platforms.clear();
        self.sync_status.clear();
    }

    /// T108: Set connected platforms for sync.
    pub fn set_connected_platforms(&mut self, platforms: Vec<SyncPlatform>) {
        self.connected_platforms = platforms;
    }

    /// T108: Update sync status for a platform.
    pub fn update_sync_status(&mut self, platform: SyncPlatform, status: SyncRecordStatus) {
        if let Some(entry) = self.sync_status.iter_mut().find(|(p, _)| *p == platform) {
            entry.1 = status;
        } else {
            self.sync_status.push((platform, status));
        }
    }

    /// T108: Get sync status for a platform.
    pub fn get_sync_status(&self, platform: SyncPlatform) -> Option<SyncRecordStatus> {
        self.sync_status
            .iter()
            .find(|(p, _)| *p == platform)
            .map(|(_, s)| *s)
    }

    /// Render the ride summary screen.
    /// Returns (next_screen, should_save, should_discard)
    pub fn show(&mut self, ui: &mut Ui) -> RideSummaryAction {
        let mut action = RideSummaryAction::None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading("Ride Summary");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.is_saved {
                        ui.label(RichText::new("✓ Saved").color(Color32::from_rgb(52, 168, 83)));
                    }
                });
            });

            ui.add_space(16.0);

            if let Some(ride) = &self.ride {
                // Stats grid
                self.render_stats_grid(ui, ride);

                ui.add_space(16.0);

                // Notes section
                ui.label(RichText::new("Notes:").strong());
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.notes)
                        .desired_width(ui.available_width())
                        .desired_rows(3)
                        .hint_text("Add notes about your ride..."),
                );

                ui.add_space(16.0);

                // Export status
                if let Some(status) = &self.export_status {
                    ui.label(RichText::new(status).color(Color32::from_rgb(66, 133, 244)));
                    ui.add_space(8.0);
                }

                // Action buttons
                ui.horizontal(|ui| {
                    // Save button
                    let save_text = if self.is_saved { "Saved" } else { "Save Ride" };
                    if ui
                        .add_sized(
                            Vec2::new(120.0, 36.0),
                            egui::Button::new(RichText::new(save_text).size(14.0))
                                .fill(Color32::from_rgb(52, 168, 83)),
                        )
                        .clicked()
                    {
                        action = RideSummaryAction::Save;
                    }

                    ui.add_space(8.0);

                    // Export button
                    if ui
                        .add_sized(
                            Vec2::new(120.0, 36.0),
                            egui::Button::new(RichText::new("Export...").size(14.0))
                                .fill(Color32::from_rgb(66, 133, 244)),
                        )
                        .clicked()
                    {
                        self.show_export_dialog = true;
                    }

                    ui.add_space(8.0);

                    // T108: Sync button
                    let sync_btn_text = if self
                        .sync_status
                        .iter()
                        .any(|(_, s)| *s == SyncRecordStatus::Completed)
                    {
                        "Synced"
                    } else if self.connected_platforms.is_empty() {
                        "Sync"
                    } else {
                        "Sync..."
                    };
                    let sync_btn_enabled = !self.connected_platforms.is_empty();
                    if ui
                        .add_enabled(
                            sync_btn_enabled,
                            egui::Button::new(RichText::new(sync_btn_text).size(14.0))
                                .min_size(Vec2::new(100.0, 36.0))
                                .fill(Color32::from_rgb(255, 152, 0)),
                        )
                        .on_hover_text(if sync_btn_enabled {
                            "Sync to connected fitness platforms"
                        } else {
                            "Connect platforms in Settings first"
                        })
                        .clicked()
                    {
                        self.show_sync_dialog = true;
                    }

                    ui.add_space(8.0);

                    // Discard button
                    if ui
                        .add_sized(
                            Vec2::new(120.0, 36.0),
                            egui::Button::new(RichText::new("Discard").size(14.0))
                                .fill(Color32::from_rgb(234, 67, 53)),
                        )
                        .clicked()
                    {
                        action = RideSummaryAction::Discard;
                    }

                    ui.add_space(8.0);

                    // Home button
                    if ui
                        .add_sized(
                            Vec2::new(120.0, 36.0),
                            egui::Button::new(RichText::new("Home").size(14.0)),
                        )
                        .clicked()
                    {
                        action = RideSummaryAction::GoHome;
                    }
                });
            } else {
                ui.label("No ride data to display");
            }
        });

        // Export dialog
        if self.show_export_dialog {
            if let Some(export_action) = self.render_export_dialog(ui) {
                action = export_action;
            }
        }

        // T108: Sync dialog
        if self.show_sync_dialog {
            if let Some(sync_action) = self.render_sync_dialog(ui) {
                action = sync_action;
            }
        }

        action
    }

    /// Render the stats grid.
    fn render_stats_grid(&self, ui: &mut Ui, ride: &Ride) {
        let panel_color = ui.visuals().faint_bg_color;

        // Duration and Distance row
        ui.horizontal(|ui| {
            self.render_stat_panel(
                ui,
                "Duration",
                &format_duration(ride.duration_seconds),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Distance",
                &format!("{:.1} km", ride.distance_meters / 1000.0),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Calories",
                &format!("{} kcal", ride.calories),
                panel_color,
            );
        });

        ui.add_space(8.0);

        // Power stats row
        ui.horizontal(|ui| {
            self.render_stat_panel(
                ui,
                "Avg Power",
                &format_optional_power(ride.avg_power),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Max Power",
                &format_optional_power(ride.max_power),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Norm Power",
                &format_optional_power(ride.normalized_power),
                panel_color,
            );
        });

        ui.add_space(8.0);

        // TSS/IF row
        ui.horizontal(|ui| {
            self.render_stat_panel(
                ui,
                "TSS",
                &ride.tss.map_or("-".to_string(), |v| format!("{:.0}", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "IF",
                &ride
                    .intensity_factor
                    .map_or("-".to_string(), |v| format!("{:.2}", v)),
                panel_color,
            );
            self.render_stat_panel(ui, "FTP", &format!("{} W", ride.ftp_at_ride), panel_color);
        });

        ui.add_space(8.0);

        // HR stats row
        ui.horizontal(|ui| {
            self.render_stat_panel(
                ui,
                "Avg HR",
                &ride
                    .avg_hr
                    .map_or("-".to_string(), |v| format!("{} bpm", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Max HR",
                &ride
                    .max_hr
                    .map_or("-".to_string(), |v| format!("{} bpm", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Avg Cadence",
                &ride
                    .avg_cadence
                    .map_or("-".to_string(), |v| format!("{} rpm", v)),
                panel_color,
            );
        });

        // T053: Cycling dynamics row (only show if dynamics data available)
        if ride.avg_left_balance.is_some()
            || ride.avg_left_torque_eff.is_some()
            || ride.avg_left_smoothness.is_some()
        {
            ui.add_space(8.0);
            self.render_dynamics_row(ui, ride, panel_color);
        }
    }

    /// T053: Render cycling dynamics summary row.
    fn render_dynamics_row(&self, ui: &mut Ui, ride: &Ride, fill: Color32) {
        ui.horizontal(|ui| {
            // Power Balance
            let balance_text = if let Some(left) = ride.avg_left_balance {
                let right = 100.0 - left;
                format!("{:.0}/{:.0}", left, right)
            } else {
                "-".to_string()
            };
            self.render_stat_panel(ui, "L/R Balance", &balance_text, fill);

            // Torque Effectiveness
            let te_text =
                if ride.avg_left_torque_eff.is_some() || ride.avg_right_torque_eff.is_some() {
                    format!(
                        "L:{:.0}% R:{:.0}%",
                        ride.avg_left_torque_eff.unwrap_or(0.0),
                        ride.avg_right_torque_eff.unwrap_or(0.0)
                    )
                } else {
                    "-".to_string()
                };
            self.render_stat_panel(ui, "Torque Eff.", &te_text, fill);

            // Pedal Smoothness
            let ps_text =
                if ride.avg_left_smoothness.is_some() || ride.avg_right_smoothness.is_some() {
                    format!(
                        "L:{:.0}% R:{:.0}%",
                        ride.avg_left_smoothness.unwrap_or(0.0),
                        ride.avg_right_smoothness.unwrap_or(0.0)
                    )
                } else {
                    "-".to_string()
                };
            self.render_stat_panel(ui, "Smoothness", &ps_text, fill);
        });
    }

    /// Render a single stat panel.
    fn render_stat_panel(&self, ui: &mut Ui, label: &str, value: &str, fill: Color32) {
        let available_width = (ui.available_width() - 16.0) / 3.0;

        egui::Frame::new()
            .fill(fill)
            .inner_margin(12.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(available_width);
                ui.set_max_width(available_width);

                ui.vertical(|ui| {
                    ui.label(RichText::new(label).size(12.0).weak());
                    ui.label(RichText::new(value).size(20.0).strong());
                });
            });
    }

    /// Render the export dialog.
    fn render_export_dialog(&mut self, ui: &mut Ui) -> Option<RideSummaryAction> {
        let mut action = None;

        egui::Window::new("Export Ride")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(300.0, 200.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label("Select export format:");
                    ui.add_space(8.0);

                    // Format selection
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.export_format, ExportFormat::Tcx, "TCX (Strava/Garmin)");
                    });
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.export_format, ExportFormat::Csv, "CSV (Spreadsheet)");
                    });

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("TCX format is recommended for uploading to Strava, Garmin Connect, or TrainingPeaks.")
                            .weak()
                            .size(12.0),
                    );

                    ui.add_space(16.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button("Export").clicked() {
                            action = Some(RideSummaryAction::Export(self.export_format));
                            self.show_export_dialog = false;
                        }

                        ui.add_space(8.0);

                        if ui.button("Cancel").clicked() {
                            self.show_export_dialog = false;
                        }
                    });
                });
            });

        // Close on Escape
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_export_dialog = false;
        }

        action
    }

    /// T108: Render the sync dialog.
    fn render_sync_dialog(&mut self, ui: &mut Ui) -> Option<RideSummaryAction> {
        let mut action = None;

        egui::Window::new("Sync to Platforms")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(350.0, 250.0));

                ui.vertical(|ui| {
                    ui.add_space(8.0);

                    if self.connected_platforms.is_empty() {
                        ui.label(RichText::new("No platforms connected.").weak());
                        ui.label(
                            RichText::new("Connect to Strava or Garmin in Settings.")
                                .weak()
                                .size(12.0),
                        );
                    } else {
                        ui.label("Select platforms to sync:");
                        ui.add_space(8.0);

                        // Platform checkboxes
                        for platform in &self.connected_platforms {
                            let platform_name = platform.display_name();
                            let is_selected = self.selected_platforms.contains(platform);

                            // Check if already synced
                            let sync_status = self.get_sync_status(*platform);
                            let is_synced = sync_status == Some(SyncRecordStatus::Completed);
                            let is_syncing = sync_status == Some(SyncRecordStatus::Uploading);

                            ui.horizontal(|ui| {
                                let mut selected = is_selected;
                                if ui
                                    .add_enabled(
                                        !is_synced && !is_syncing,
                                        egui::Checkbox::new(&mut selected, platform_name),
                                    )
                                    .changed()
                                {
                                    if selected {
                                        if !self.selected_platforms.contains(platform) {
                                            self.selected_platforms.push(*platform);
                                        }
                                    } else {
                                        self.selected_platforms.retain(|p| p != platform);
                                    }
                                }

                                // Status indicator
                                if is_synced {
                                    ui.label(
                                        RichText::new("(Synced)")
                                            .color(Color32::from_rgb(52, 168, 83))
                                            .small(),
                                    );
                                } else if is_syncing {
                                    ui.label(
                                        RichText::new("(Syncing...)")
                                            .color(Color32::from_rgb(255, 152, 0))
                                            .small(),
                                    );
                                } else if sync_status == Some(SyncRecordStatus::Failed) {
                                    ui.label(
                                        RichText::new("(Failed)")
                                            .color(Color32::from_rgb(234, 67, 53))
                                            .small(),
                                    );
                                }
                            });
                        }

                        ui.add_space(16.0);

                        // T110: Sync status summary
                        if !self.sync_status.is_empty() {
                            ui.separator();
                            ui.add_space(8.0);
                            for (platform, status) in &self.sync_status {
                                let (icon, color) = match status {
                                    SyncRecordStatus::Completed => {
                                        ("", Color32::from_rgb(52, 168, 83))
                                    }
                                    SyncRecordStatus::Uploading => {
                                        ("", Color32::from_rgb(255, 152, 0))
                                    }
                                    SyncRecordStatus::Pending => {
                                        ("", Color32::from_rgb(66, 133, 244))
                                    }
                                    SyncRecordStatus::Failed => {
                                        ("", Color32::from_rgb(234, 67, 53))
                                    }
                                    SyncRecordStatus::Cancelled => ("", Color32::GRAY),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).color(color));
                                    ui.label(platform.display_name());
                                    ui.label(
                                        RichText::new(format!("{:?}", status)).color(color).small(),
                                    );
                                });
                            }
                            ui.add_space(8.0);
                        }
                    }

                    ui.add_space(16.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        let can_sync = !self.selected_platforms.is_empty();

                        if ui
                            .add_enabled(
                                can_sync,
                                egui::Button::new("Sync Selected")
                                    .fill(Color32::from_rgb(255, 152, 0)),
                            )
                            .clicked()
                        {
                            let platforms_to_sync = self.selected_platforms.clone();
                            action = Some(RideSummaryAction::SyncToPlatforms(platforms_to_sync));
                            self.show_sync_dialog = false;
                        }

                        ui.add_space(8.0);

                        if ui.button("Cancel").clicked() {
                            self.show_sync_dialog = false;
                        }
                    });
                });
            });

        // Close on Escape
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_sync_dialog = false;
        }

        action
    }

    /// Set the export status message.
    pub fn set_export_status(&mut self, message: &str) {
        self.export_status = Some(message.to_string());
    }

    /// Mark the ride as saved.
    pub fn mark_saved(&mut self) {
        self.is_saved = true;
    }

    /// Get the notes text.
    pub fn get_notes(&self) -> &str {
        &self.notes
    }
}

/// Actions returned from the ride summary screen.
#[derive(Debug, Clone, PartialEq)]
pub enum RideSummaryAction {
    /// No action
    None,
    /// Save the ride
    Save,
    /// Discard the ride
    Discard,
    /// Export to specified format
    Export(ExportFormat),
    /// Go to home screen
    GoHome,
    /// T108: Sync to specified platforms
    SyncToPlatforms(Vec<SyncPlatform>),
}

/// Format a duration in seconds to HH:MM:SS.
fn format_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

/// Format optional power value.
fn format_optional_power(power: Option<u16>) -> String {
    power.map_or("-".to_string(), |p| format!("{} W", p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_minutes_only() {
        assert_eq!(format_duration(65), "1:05");
        assert_eq!(format_duration(3599), "59:59");
    }

    #[test]
    fn test_format_duration_with_hours() {
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(7265), "2:01:05");
    }

    #[test]
    fn test_format_optional_power() {
        assert_eq!(format_optional_power(Some(200)), "200 W");
        assert_eq!(format_optional_power(None), "-");
    }
}
