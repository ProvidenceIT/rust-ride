//! Ride screen implementation.
//!
//! T048: Implement ride screen layout with metric panels
//! T051: Implement "End Ride" button
//! T053: Implement keyboard shortcut: Space for pause

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::metrics::calculator::AggregatedMetrics;
use crate::recording::types::RecordingStatus;
use crate::ui::theme::zone_colors;
use crate::ui::widgets::{MetricDisplay, MetricSize};

use super::Screen;

/// Ride mode (free ride or structured workout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RideMode {
    #[default]
    FreeRide,
    Workout,
}

/// Ride screen state.
pub struct RideScreen {
    /// Current ride mode
    pub mode: RideMode,
    /// Recording status
    pub recording_status: RecordingStatus,
    /// Is ride paused
    pub is_paused: bool,
    /// Current metrics
    pub metrics: AggregatedMetrics,
    /// Elapsed time in seconds
    pub elapsed_seconds: u32,
    /// Show end ride confirmation
    pub show_end_dialog: bool,
}

impl Default for RideScreen {
    fn default() -> Self {
        Self {
            mode: RideMode::FreeRide,
            recording_status: RecordingStatus::Idle,
            is_paused: false,
            metrics: AggregatedMetrics::default(),
            elapsed_seconds: 0,
            show_end_dialog: false,
        }
    }
}

impl RideScreen {
    /// Create a new ride screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a free ride.
    pub fn start_free_ride(&mut self) {
        self.mode = RideMode::FreeRide;
        self.recording_status = RecordingStatus::Recording;
        self.is_paused = false;
        self.elapsed_seconds = 0;
        self.metrics = AggregatedMetrics::default();
    }

    /// Render the ride screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        // Handle keyboard shortcuts
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            self.is_paused = !self.is_paused;
        }

        ui.vertical(|ui| {
            // Top bar with ride controls
            self.render_top_bar(ui);

            ui.add_space(16.0);

            // Main metrics area
            self.render_main_metrics(ui);

            ui.add_space(16.0);

            // Secondary metrics
            self.render_secondary_metrics(ui);

            // Spacer
            ui.add_space(ui.available_height() - 60.0);

            // Bottom controls
            if let Some(screen) = self.render_bottom_controls(ui) {
                next_screen = Some(screen);
            }
        });

        // End ride confirmation dialog
        if self.show_end_dialog {
            if let Some(screen) = self.render_end_dialog(ui) {
                next_screen = Some(screen);
            }
        }

        next_screen
    }

    /// Render the top bar with ride info and controls.
    fn render_top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Ride mode indicator
            let mode_text = match self.mode {
                RideMode::FreeRide => "Free Ride",
                RideMode::Workout => "Workout",
            };
            ui.label(RichText::new(mode_text).size(18.0).strong());

            // Recording status
            let (status_icon, status_color) = match self.recording_status {
                RecordingStatus::Recording if !self.is_paused => {
                    ("●", Color32::from_rgb(234, 67, 53)) // Red = recording
                }
                RecordingStatus::Recording if self.is_paused => {
                    ("❚❚", Color32::from_rgb(251, 188, 4)) // Yellow = paused
                }
                RecordingStatus::Paused => ("❚❚", Color32::from_rgb(251, 188, 4)),
                _ => ("○", Color32::GRAY),
            };
            ui.label(RichText::new(status_icon).color(status_color));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Pause/Resume button
                let pause_text = if self.is_paused { "▶ Resume" } else { "❚❚ Pause" };
                if ui.button(pause_text).clicked() {
                    self.is_paused = !self.is_paused;
                }

                ui.label(RichText::new("(Space)").weak().small());
            });
        });

        ui.separator();
    }

    /// Render the main metrics (large display).
    fn render_main_metrics(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 600.0) / 2.0);

            // Power (primary metric)
            let power_color = self
                .metrics
                .power_zone
                .map(zone_colors::power_zone_color)
                .unwrap_or(Color32::WHITE);

            MetricDisplay::power(self.metrics.power_instant)
                .with_size(MetricSize::Large)
                .with_zone_color(power_color)
                .show(ui);

            ui.add_space(32.0);

            // Heart Rate
            let hr_color = self
                .metrics
                .hr_zone
                .map(zone_colors::hr_zone_color)
                .unwrap_or(Color32::WHITE);

            MetricDisplay::heart_rate(self.metrics.heart_rate)
                .with_size(MetricSize::Large)
                .with_zone_color(hr_color)
                .show(ui);

            ui.add_space(32.0);

            // Cadence
            MetricDisplay::cadence(self.metrics.cadence)
                .with_size(MetricSize::Large)
                .show(ui);
        });
    }

    /// Render the secondary metrics row.
    fn render_secondary_metrics(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 700.0) / 2.0);

            // Duration
            MetricDisplay::duration(self.elapsed_seconds)
                .with_size(MetricSize::Medium)
                .show(ui);

            ui.add_space(16.0);

            // Distance
            MetricDisplay::distance(self.metrics.distance)
                .with_size(MetricSize::Medium)
                .show(ui);

            ui.add_space(16.0);

            // Speed
            MetricDisplay::speed(self.metrics.speed)
                .with_size(MetricSize::Medium)
                .show(ui);

            ui.add_space(16.0);

            // 3s Power Average
            MetricDisplay::new(
                self.metrics
                    .power_3s_avg
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "--".to_string()),
                "W",
                "3s Avg",
            )
            .with_size(MetricSize::Medium)
            .show(ui);

            ui.add_space(16.0);

            // Calories
            MetricDisplay::new(self.metrics.calories.to_string(), "kcal", "Calories")
                .with_size(MetricSize::Medium)
                .show(ui);
        });

        // Third row: NP, TSS, IF (if available)
        if self.metrics.normalized_power.is_some() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 400.0) / 2.0);

                // Normalized Power
                if let Some(np) = self.metrics.normalized_power {
                    MetricDisplay::new(np.to_string(), "W", "NP")
                        .with_size(MetricSize::Small)
                        .show(ui);
                }

                ui.add_space(16.0);

                // TSS
                if let Some(tss) = self.metrics.tss {
                    MetricDisplay::new(format!("{:.0}", tss), "", "TSS")
                        .with_size(MetricSize::Small)
                        .show(ui);
                }

                ui.add_space(16.0);

                // Intensity Factor
                if let Some(if_val) = self.metrics.intensity_factor {
                    MetricDisplay::new(format!("{:.2}", if_val), "", "IF")
                        .with_size(MetricSize::Small)
                        .show(ui);
                }
            });
        }
    }

    /// Render the bottom control buttons.
    fn render_bottom_controls(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        ui.separator();

        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 200.0) / 2.0);

            if ui
                .add_sized(
                    Vec2::new(200.0, 48.0),
                    egui::Button::new(RichText::new("End Ride").size(18.0))
                        .fill(Color32::from_rgb(234, 67, 53)),
                )
                .clicked()
            {
                self.show_end_dialog = true;
            }
        });

        next_screen
    }

    /// Render the end ride confirmation dialog.
    fn render_end_dialog(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        egui::Window::new("End Ride?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(300.0, 150.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label("Are you sure you want to end this ride?");

                    ui.add_space(8.0);

                    // Show ride summary
                    let duration_min = self.elapsed_seconds / 60;
                    let distance_km = self.metrics.distance / 1000.0;
                    ui.label(
                        RichText::new(format!(
                            "Duration: {}:{:02} | Distance: {:.1} km",
                            duration_min,
                            self.elapsed_seconds % 60,
                            distance_km
                        ))
                        .weak(),
                    );

                    ui.add_space(24.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_end_dialog = false;
                        }

                        ui.add_space(16.0);

                        if ui.button("Discard").clicked() {
                            self.show_end_dialog = false;
                            next_screen = Some(Screen::Home);
                        }

                        ui.add_space(16.0);

                        if ui
                            .add(
                                egui::Button::new("Save & End")
                                    .fill(Color32::from_rgb(52, 168, 83)),
                            )
                            .clicked()
                        {
                            self.show_end_dialog = false;
                            // TODO: Save the ride
                            next_screen = Some(Screen::RideSummary);
                        }
                    });
                });
            });

        next_screen
    }
}
