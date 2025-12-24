//! Ride summary screen implementation.
//!
//! T101: Create ride summary screen with stats display
//! T102: Implement export button with format selection
//! T103: Implement save/discard controls

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::recording::types::{ExportFormat, Ride, RideSample};

use super::Screen;

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

        action
    }

    /// Render the stats grid.
    fn render_stats_grid(&self, ui: &mut Ui, ride: &Ride) {
        let panel_color = ui.visuals().faint_bg_color;

        // Duration and Distance row
        ui.horizontal(|ui| {
            self.render_stat_panel(ui, "Duration", &format_duration(ride.duration_seconds), panel_color);
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
                &ride.intensity_factor.map_or("-".to_string(), |v| format!("{:.2}", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "FTP",
                &format!("{} W", ride.ftp_at_ride),
                panel_color,
            );
        });

        ui.add_space(8.0);

        // HR stats row
        ui.horizontal(|ui| {
            self.render_stat_panel(
                ui,
                "Avg HR",
                &ride.avg_hr.map_or("-".to_string(), |v| format!("{} bpm", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Max HR",
                &ride.max_hr.map_or("-".to_string(), |v| format!("{} bpm", v)),
                panel_color,
            );
            self.render_stat_panel(
                ui,
                "Avg Cadence",
                &ride.avg_cadence.map_or("-".to_string(), |v| format!("{} rpm", v)),
                panel_color,
            );
        });
    }

    /// Render a single stat panel.
    fn render_stat_panel(&self, ui: &mut Ui, label: &str, value: &str, fill: Color32) {
        let available_width = (ui.available_width() - 16.0) / 3.0;

        egui::Frame::none()
            .fill(fill)
            .inner_margin(12.0)
            .rounding(8.0)
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
