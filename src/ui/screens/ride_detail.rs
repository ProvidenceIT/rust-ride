//! Ride detail screen implementation.
//!
//! T133: Create ride detail screen with full statistics
//! T134: Display power curve/distribution (placeholder)
//! T135: Display HR data
//! T136: Show lap/segment breakdown (if workout)
//! T137: Implement export and delete actions
//! T142: Add motion visualization to post-ride analysis
//! 3.4: Add sync status display and retry button for failed syncs

use chrono::Local;
use egui::{Align, Color32, CursorIcon, Layout, Pos2, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::recording::types::{ExportFormat, Ride, RideSample};
use crate::sensors::MotionSample;
use crate::storage::config::Units;
use crate::ui::theme::zone_colors;

use super::ride_history::RideSyncStatus;

/// Strava brand color
const STRAVA_ORANGE: Color32 = Color32::from_rgb(252, 82, 0);

/// Actions that can result from the ride detail screen.
#[derive(Debug, Clone, PartialEq)]
pub enum RideDetailAction {
    /// No action
    None,
    /// Go back to history
    GoBack,
    /// Export ride
    Export(ExportFormat),
    /// Delete ride
    Delete,
    /// Retry sync for failed upload
    RetrySync {
        /// Platform to retry (e.g., "Strava")
        platform: String,
    },
}

/// Ride detail screen state.
pub struct RideDetailScreen {
    /// The ride being displayed
    pub ride: Option<Ride>,
    /// Ride samples (for charts)
    pub samples: Vec<RideSample>,
    /// T142: Motion samples (for motion visualization)
    pub motion_samples: Vec<MotionSample>,
    /// Show delete confirmation
    pub show_delete_dialog: bool,
    /// Show export dialog
    pub show_export_dialog: bool,
    /// Selected export format
    pub export_format: ExportFormat,
    /// Unit preference
    pub units: Units,
    /// FTP at time of ride (for zone calculations)
    pub ftp: u16,
    /// 3.4: Sync status for this ride
    pub sync_status: RideSyncStatus,
}

impl Default for RideDetailScreen {
    fn default() -> Self {
        Self {
            ride: None,
            samples: Vec::new(),
            motion_samples: Vec::new(),
            show_delete_dialog: false,
            show_export_dialog: false,
            export_format: ExportFormat::Tcx,
            units: Units::Metric,
            ftp: 200,
            sync_status: RideSyncStatus::NotSynced,
        }
    }
}

impl RideDetailScreen {
    /// Create a new ride detail screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the ride to display.
    pub fn set_ride(&mut self, ride: Ride, samples: Vec<RideSample>) {
        self.ftp = ride.ftp_at_ride;
        self.ride = Some(ride);
        self.samples = samples;
        self.motion_samples.clear();
        self.show_delete_dialog = false;
        self.show_export_dialog = false;
        self.sync_status = RideSyncStatus::NotSynced;
    }

    /// 3.4: Set the sync status for this ride.
    pub fn set_sync_status(&mut self, status: RideSyncStatus) {
        self.sync_status = status;
    }

    /// 3.4: Get the sync status for this ride.
    pub fn get_sync_status(&self) -> &RideSyncStatus {
        &self.sync_status
    }

    /// T142: Set motion samples for visualization.
    pub fn set_motion_samples(&mut self, samples: Vec<MotionSample>) {
        self.motion_samples = samples;
    }

    /// T142: Check if motion data is available.
    pub fn has_motion_data(&self) -> bool {
        !self.motion_samples.is_empty()
    }

    /// Render the ride detail screen.
    pub fn show(&mut self, ui: &mut Ui) -> RideDetailAction {
        let mut action = RideDetailAction::None;

        let Some(ref ride) = self.ride else {
            ui.centered_and_justified(|ui| {
                ui.label("No ride selected");
            });
            return action;
        };

        // Header
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                action = RideDetailAction::GoBack;
            }

            let local = ride.started_at.with_timezone(&Local);
            ui.heading(local.format("%B %d, %Y at %H:%M").to_string());

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(RichText::new("Delete").color(Color32::from_rgb(234, 67, 53)))
                    .clicked()
                {
                    self.show_delete_dialog = true;
                }

                if ui.button("Export").clicked() {
                    self.show_export_dialog = true;
                }
            });
        });

        ui.separator();

        // Main content
        let mut sync_retry_action: Option<RideDetailAction> = None;
        ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Summary stats grid
            self.render_summary_grid(ui, ride);

            ui.add_space(16.0);

            // 3.4: Sync status section (only show if not NotSynced)
            if !matches!(self.sync_status, RideSyncStatus::NotSynced) {
                if let Some(retry_action) = self.render_sync_section(ui) {
                    sync_retry_action = Some(retry_action);
                }
                ui.add_space(16.0);
            }

            // Power metrics
            self.render_power_section(ui, ride);

            ui.add_space(16.0);

            // Heart rate metrics
            self.render_hr_section(ui, ride);

            ui.add_space(16.0);

            // Power distribution (placeholder)
            self.render_power_distribution(ui);

            ui.add_space(16.0);

            // T142: Motion data visualization
            if self.has_motion_data() {
                self.render_motion_section(ui);
                ui.add_space(16.0);
            }

            // Notes
            if let Some(ref notes) = ride.notes {
                self.render_notes_section(ui, notes);
            }

            ui.add_space(32.0);
        });

        // Handle sync retry action
        if let Some(retry_action) = sync_retry_action {
            action = retry_action;
        }

        // Delete confirmation dialog
        if self.show_delete_dialog {
            if let Some(delete_action) = self.render_delete_dialog(ui) {
                if delete_action {
                    action = RideDetailAction::Delete;
                }
                self.show_delete_dialog = false;
            }
        }

        // Export dialog
        if self.show_export_dialog {
            if let Some(export_format) = self.render_export_dialog(ui) {
                action = RideDetailAction::Export(export_format);
                self.show_export_dialog = false;
            }
        }

        action
    }

    /// Render the summary statistics grid.
    fn render_summary_grid(&self, ui: &mut Ui, ride: &Ride) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Summary").size(18.0).strong());
            ui.add_space(8.0);

            egui::Grid::new("summary_grid")
                .num_columns(4)
                .spacing([32.0, 8.0])
                .show(ui, |ui| {
                    // Row 1: Duration, Distance, Avg Speed, Calories
                    self.render_metric_cell(
                        ui,
                        "Duration",
                        &self.format_duration(ride.duration_seconds),
                    );

                    let (dist, dist_unit) = match self.units {
                        Units::Metric => (ride.distance_meters / 1000.0, "km"),
                        Units::Imperial => (ride.distance_meters / 1000.0 * 0.621371, "mi"),
                    };
                    self.render_metric_cell(ui, "Distance", &format!("{:.2} {}", dist, dist_unit));

                    // Calculate average speed
                    let avg_speed = if ride.duration_seconds > 0 {
                        ride.distance_meters / ride.duration_seconds as f64 * 3.6
                    // km/h
                    } else {
                        0.0
                    };
                    let (speed, speed_unit) = match self.units {
                        Units::Metric => (avg_speed, "km/h"),
                        Units::Imperial => (avg_speed * 0.621371, "mph"),
                    };
                    self.render_metric_cell(
                        ui,
                        "Avg Speed",
                        &format!("{:.1} {}", speed, speed_unit),
                    );

                    self.render_metric_cell(ui, "Calories", &format!("{} kcal", ride.calories));
                    ui.end_row();

                    // Row 2: TSS, IF, Work
                    let tss_str = ride
                        .tss
                        .map(|t| format!("{:.0}", t))
                        .unwrap_or_else(|| "--".to_string());
                    self.render_metric_cell(ui, "TSS", &tss_str);

                    let if_str = ride
                        .intensity_factor
                        .map(|i| format!("{:.2}", i))
                        .unwrap_or_else(|| "--".to_string());
                    self.render_metric_cell(ui, "IF", &if_str);

                    // Calculate work (kJ)
                    let work_kj = ride
                        .avg_power
                        .map(|p| p as f32 * ride.duration_seconds as f32 / 1000.0);
                    let work_str = work_kj
                        .map(|w| format!("{:.0} kJ", w))
                        .unwrap_or_else(|| "--".to_string());
                    self.render_metric_cell(ui, "Work", &work_str);

                    self.render_metric_cell(ui, "FTP", &format!("{} W", ride.ftp_at_ride));
                    ui.end_row();
                });
        });
    }

    /// Render a single metric cell.
    fn render_metric_cell(&self, ui: &mut Ui, label: &str, value: &str) {
        ui.vertical(|ui| {
            ui.label(RichText::new(value).size(18.0).strong());
            ui.label(RichText::new(label).size(12.0).weak());
        });
    }

    /// Render the power section.
    fn render_power_section(&self, ui: &mut Ui, ride: &Ride) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Power").size(18.0).strong());
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Average Power
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        let avg_power = ride
                            .avg_power
                            .map(|p| format!("{} W", p))
                            .unwrap_or_else(|| "--".to_string());
                        ui.label(RichText::new(avg_power).size(24.0).strong());
                        ui.label(RichText::new("Average").weak());
                    });
                });

                // Max Power
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        let max_power = ride
                            .max_power
                            .map(|p| format!("{} W", p))
                            .unwrap_or_else(|| "--".to_string());
                        ui.label(RichText::new(max_power).size(24.0).strong());
                        ui.label(RichText::new("Max").weak());
                    });
                });

                // Normalized Power
ui.group(|ui| {
                    ui.vertical(|ui| {
                        let np = ride
                            .normalized_power
                            .map(|p| format!("{} W", p))
                            .unwrap_or_else(|| "--".to_string());
                        ui.label(RichText::new(np).size(24.0).strong());
                        ui.label(RichText::new("Normalized").weak());
                    });
                });

                // Variability Index (NP/Avg)
                if let (Some(np), Some(avg)) = (ride.normalized_power, ride.avg_power) {
                    if avg > 0 {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                let vi = np as f32 / avg as f32;
                                ui.label(RichText::new(format!("{:.2}", vi)).size(24.0).strong());
                                ui.label(RichText::new("VI").weak());
                            });
                        });
                    }
                }
            });
        });
    }

    /// Render the heart rate section.
    fn render_hr_section(&self, ui: &mut Ui, ride: &Ride) {
        if ride.avg_hr.is_none() && ride.max_hr.is_none() {
            return;
        }

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Heart Rate").size(18.0).strong());
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Average HR
                if let Some(avg_hr) = ride.avg_hr {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("{} bpm", avg_hr)).size(24.0).strong());
                            ui.label(RichText::new("Average").weak());
                        });
                    });
                }

                // Max HR
                if let Some(max_hr) = ride.max_hr {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("{} bpm", max_hr)).size(24.0).strong());
                            ui.label(RichText::new("Max").weak());
                        });
                    });
                }

                // Average Cadence
                if let Some(avg_cad) = ride.avg_cadence {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("{} rpm", avg_cad))
                                    .size(24.0)
                                    .strong(),
                            );
                            ui.label(RichText::new("Avg Cadence").weak());
                        });
                    });
                }
            });
        });
    }

    /// Render power distribution (zone time).
    fn render_power_distribution(&self, ui: &mut Ui) {
        if self.samples.is_empty() {
            return;
        }

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Time in Zone").size(18.0).strong());
            ui.add_space(8.0);

            // Calculate time in each zone
            let mut zone_times = [0u32; 7];
            let power_zones = crate::metrics::zones::PowerZones::from_ftp(self.ftp);

            for sample in &self.samples {
                if let Some(power) = sample.power_watts {
                    let zone = power_zones.get_zone(power);
                    if (1..=7).contains(&zone) {
                        zone_times[(zone - 1) as usize] += 1;
                    }
                }
            }

            let total_time: u32 = zone_times.iter().sum();
            if total_time == 0 {
                ui.label(RichText::new("No power data available").weak().italics());
                return;
            }

            // Render zone bars
            let zone_names = ["Z1", "Z2", "Z3", "Z4", "Z5", "Z6", "Z7"];
            let bar_max_width = ui.available_width() - 100.0;

            for (i, &time) in zone_times.iter().enumerate() {
                let pct = time as f32 / total_time as f32;
                let minutes = time / 60;
                let seconds = time % 60;

                ui.horizontal(|ui| {
                    ui.label(RichText::new(zone_names[i]).size(12.0).strong());
                    ui.add_space(8.0);

                    // Draw bar
                    let bar_width = bar_max_width * pct;
                    let bar_color = zone_colors::power_zone_color((i + 1) as u8);

                    let (rect, _) = ui
                        .allocate_exact_size(Vec2::new(bar_max_width, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, Vec2::new(bar_width, 16.0)),
                        0.0,
                        bar_color,
                    );

                    // Draw border
                    ui.painter().rect_stroke(
                        egui::Rect::from_min_size(rect.min, Vec2::new(bar_max_width, 16.0)),
                        0.0,
                        Stroke::new(1.0, Color32::GRAY),
                    );

                    ui.add_space(8.0);
                    ui.label(format!("{}:{:02} ({:.0}%)", minutes, seconds, pct * 100.0));
                });
            }
        });
    }

    /// 3.4: Render sync status section with retry button for failed syncs.
    fn render_sync_section(&self, ui: &mut Ui) -> Option<RideDetailAction> {
        let mut action = None;

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Strava Sync").size(18.0).strong());
            ui.add_space(8.0);

            match &self.sync_status {
                RideSyncStatus::NotSynced => {
                    // This shouldn't be rendered, but handle it gracefully
                    ui.label(RichText::new("Not synced").weak());
                }
                RideSyncStatus::Pending { platform } => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            RichText::new(format!("Uploading to {}...", platform))
                                .color(self.sync_status.color()),
                        );
                    });
                }
                RideSyncStatus::Synced {
                    activity_id,
                    platform,
                } => {
                    ui.horizontal(|ui| {
                        // Strava icon
                        if platform.to_lowercase() == "strava" {
                            ui.label(RichText::new("S").color(STRAVA_ORANGE).size(18.0).strong());
                        } else {
                            ui.label(
                                RichText::new("✓")
                                    .color(self.sync_status.color())
                                    .size(18.0),
                            );
                        }
                        ui.add_space(8.0);

                        ui.label(
                            RichText::new(format!("Synced to {}", platform))
                                .color(self.sync_status.color()),
                        );

                        ui.add_space(16.0);

                        // View on Strava button
                        if let Some(url) = self.sync_status.activity_url() {
                            let view_button = egui::Button::new(
                                RichText::new("View on Strava")
                                    .size(12.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(STRAVA_ORANGE)
                            .min_size(Vec2::new(100.0, 24.0));

                            if ui
                                .add(view_button)
                                .on_hover_text(&url)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                if let Err(e) = open::that(&url) {
                                    tracing::warn!("Failed to open Strava activity: {}", e);
                                }
                            }
                        } else {
                            ui.label(
                                RichText::new(format!("Activity ID: {}", activity_id))
                                    .size(12.0)
                                    .weak(),
                            );
                        }
                    });
                }
                RideSyncStatus::Failed {
                    error,
                    retry_count,
                    platform,
                } => {
                    ui.vertical(|ui| {
                        // Error message
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("!")
                                    .color(self.sync_status.color())
                                    .size(18.0)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("Upload to {} failed", platform))
                                    .color(self.sync_status.color()),
                            );
                        });

                        ui.add_space(8.0);

                        // Error details
                        ui.horizontal(|ui| {
                            ui.add_space(26.0); // Align with text above
                            ui.label(RichText::new(format!("Error: {}", error)).weak().size(12.0));
                        });

                        if *retry_count > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(26.0);
                                ui.label(
                                    RichText::new(format!("Retry attempts: {}", retry_count))
                                        .weak()
                                        .size(12.0),
                                );
                            });
                        }

                        ui.add_space(12.0);

                        // Retry button
                        ui.horizontal(|ui| {
                            ui.add_space(26.0);
                            let retry_button = egui::Button::new(
                                RichText::new("↻ Retry Upload")
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            )
                            .fill(STRAVA_ORANGE)
                            .min_size(Vec2::new(120.0, 32.0));

                            if ui
                                .add(retry_button)
                                .on_hover_text(format!("Retry upload to {}", platform))
                                .clicked()
                            {
                                action = Some(RideDetailAction::RetrySync {
                                    platform: platform.clone(),
                                });
                            }
                        });
                    });
                }
                RideSyncStatus::Retrying { platform } => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            RichText::new(format!("Retrying upload to {}...", platform))
                                .color(self.sync_status.color()),
                        );
                    });
                }
            }
        });

        action
    }

    /// Render motion visualization section.
    fn render_motion_section(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Motion Data").size(18.0).strong());
            ui.add_space(8.0);

            if self.motion_samples.is_empty() {
                ui.label(RichText::new("No motion data available").weak().italics());
                return;
            }

            // Display motion data summary
            ui.label(format!("Motion samples: {}", self.motion_samples.len()));

            // Calculate acceleration stats
            let mut max_accel = 0.0f32;
            let mut avg_accel = 0.0f32;
            for sample in &self.motion_samples {
                let accel_magnitude =
                    (sample.accel_x.powi(2) + sample.accel_y.powi(2) + sample.accel_z.powi(2))
                        .sqrt();
                if accel_magnitude > max_accel {
                    max_accel = accel_magnitude;
                }
                avg_accel += accel_magnitude;
            }
            avg_accel /= self.motion_samples.len() as f32;

            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{:.2} m/s²", max_accel)).strong());
                        ui.label(RichText::new("Max Acceleration").weak());
                    });
                });

                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{:.2} m/s²", avg_accel)).strong());
                        ui.label(RichText::new("Avg Acceleration").weak());
                    });
                });
            });
        });
    }

    /// Render notes section.
    fn render_notes_section(&self, ui: &mut Ui, notes: &str) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Notes").size(18.0).strong());
            ui.add_space(8.0);

            ui.label(notes);
        });
    }

    /// Format duration in seconds to readable string.
    fn format_duration(&self, seconds: u32) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}h {:.0}m", hours, minutes)
        } else {
            format!("{:.0}m {:.0}s", minutes, secs)
        }
    }

    /// Render the delete confirmation dialog.
    fn render_delete_dialog(&self, ui: &mut Ui) -> Option<bool> {
        let mut result = None;

        let modal_id = egui::Id::new("delete_confirm_dialog");
        let is_open = ui.memory_mut(|mem| mem.data.get_temp::<bool>(modal_id).unwrap_or(true));

        if is_open {
            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, true));

            egui::Window::new("Delete Ride")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Are you sure you want to delete this ride? This cannot be undone.");

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            result = Some(false);
                            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, false));
                        }

                        if ui
                            .button(RichText::new("Delete").color(Color32::from_rgb(234, 67, 53)))
                            .clicked()
                        {
                            result = Some(true);
                            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, false));
                        }
                    });
                });
        }

        result
    }

    /// Render the export dialog.
    fn render_export_dialog(&self, ui: &mut Ui) -> Option<ExportFormat> {
        let mut result = None;

        let modal_id = egui::Id::new("export_dialog");
        let is_open = ui.memory_mut(|mem| mem.data.get_temp::<bool>(modal_id).unwrap_or(true));

        if is_open {
            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, true));

            egui::Window::new("Export Ride")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Select export format:");

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, false));
                        }

                        if ui.button("TCX").clicked() {
                            result = Some(ExportFormat::Tcx);
                            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, false));
                        }

                        if ui.button("CSV").clicked() {
                            result = Some(ExportFormat::Csv);
                            ui.memory_mut(|mem| mem.data.insert_temp(modal_id, false));
                        }
                    });
                });
        }

        result
    }
}