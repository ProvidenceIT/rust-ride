//! Ride screen implementation.
//!
//! T048: Implement ride screen layout with metric panels
//! T051: Implement "End Ride" button
//! T053: Implement keyboard shortcut: Space for pause
//! T076: Extend ride screen with workout progress bar
//! T077: Display current interval, target power, time remaining
//! T078: Implement pause/resume/skip interval buttons
//! T079: Implement keyboard shortcuts: +/- for power adjustment
//! T110: Implement full-screen mode toggle
//! T111: Implement configurable metric panel layout

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::metrics::calculator::AggregatedMetrics;
use crate::recording::types::RecordingStatus;
use crate::storage::config::{DashboardLayout, MetricType};
use crate::ui::theme::zone_colors;
use crate::ui::widgets::{MetricDisplay, MetricSize};
use crate::workouts::types::{SegmentProgress, SegmentType, Workout, WorkoutStatus};

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
    /// Loaded workout (if in workout mode)
    pub workout: Option<Workout>,
    /// Current workout status
    pub workout_status: WorkoutStatus,
    /// Current segment progress
    pub segment_progress: Option<SegmentProgress>,
    /// Target power (from workout engine)
    pub target_power: Option<u16>,
    /// Power offset adjustment
    pub power_offset: i16,
    /// Current segment type
    pub current_segment_type: Option<SegmentType>,
    /// Current segment text event
    pub text_event: Option<String>,
    /// Full-screen mode (hides top bar and shows only essential metrics)
    pub full_screen_mode: bool,
    /// Dashboard layout configuration
    pub dashboard_layout: DashboardLayout,
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
            workout: None,
            workout_status: WorkoutStatus::NotStarted,
            segment_progress: None,
            target_power: None,
            power_offset: 0,
            current_segment_type: None,
            text_event: None,
            full_screen_mode: false,
            dashboard_layout: DashboardLayout::default(),
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
        self.workout = None;
        self.workout_status = WorkoutStatus::NotStarted;
        self.segment_progress = None;
        self.target_power = None;
        self.power_offset = 0;
    }

    /// Start a workout ride.
    pub fn start_workout(&mut self, workout: Workout) {
        self.mode = RideMode::Workout;
        self.recording_status = RecordingStatus::Recording;
        self.is_paused = false;
        self.elapsed_seconds = 0;
        self.metrics = AggregatedMetrics::default();
        self.workout = Some(workout);
        self.workout_status = WorkoutStatus::InProgress;
        self.power_offset = 0;
    }

    /// Update workout progress from engine.
    pub fn update_workout_progress(
        &mut self,
        progress: Option<SegmentProgress>,
        target_power: Option<u16>,
        segment_type: Option<SegmentType>,
        text_event: Option<String>,
        status: WorkoutStatus,
    ) {
        self.segment_progress = progress;
        self.target_power = target_power;
        self.current_segment_type = segment_type;
        self.text_event = text_event;
        self.workout_status = status;
    }

    /// Render the ride screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        // Handle keyboard shortcuts
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            self.is_paused = !self.is_paused;
        }

        // Full-screen toggle (F key or Escape to exit)
        if ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.full_screen_mode = !self.full_screen_mode;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.full_screen_mode {
            self.full_screen_mode = false;
        }

        // Power offset shortcuts (+/-)
        if self.mode == RideMode::Workout {
            if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                self.power_offset += 5;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
                self.power_offset -= 5;
            }
        }

        if self.full_screen_mode {
            // Full-screen mode: show only essential metrics in large format
            self.render_full_screen_mode(ui);
        } else {
            // Normal mode: show all UI elements
            ui.vertical(|ui| {
                // Top bar with ride controls
                self.render_top_bar(ui);

                ui.add_space(8.0);

                // Workout progress bar (if in workout mode)
                if self.mode == RideMode::Workout {
                    self.render_workout_progress(ui);
                    ui.add_space(8.0);
                }

                // Main metrics area
                self.render_main_metrics(ui);

                ui.add_space(16.0);

                // Workout controls (if in workout mode)
                if self.mode == RideMode::Workout {
                    self.render_workout_controls(ui);
                    ui.add_space(8.0);
                }

                // Secondary metrics
                self.render_secondary_metrics(ui);

                // Spacer
                ui.add_space(ui.available_height() - 60.0);

                // Bottom controls
                if let Some(screen) = self.render_bottom_controls(ui) {
                    next_screen = Some(screen);
                }
            });
        }

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
                // Full-screen button
                if ui.button("⛶ Full Screen").clicked() {
                    self.full_screen_mode = true;
                }
                ui.label(RichText::new("(F)").weak().small());

                ui.separator();

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

    /// Render the workout progress bar and current segment info.
    fn render_workout_progress(&self, ui: &mut Ui) {
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            if let Some(ref workout) = self.workout {
                // Workout name and overall progress
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&workout.name).strong());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let total_elapsed = self.elapsed_seconds;
                        let total_duration = workout.total_duration_seconds;
                        let overall_progress = total_elapsed as f32 / total_duration.max(1) as f32;
                        ui.label(format!(
                            "{:.0}% complete",
                            (overall_progress * 100.0).min(100.0)
                        ));
                    });
                });

                ui.add_space(4.0);

                // Overall progress bar
                let total_progress =
                    self.elapsed_seconds as f32 / workout.total_duration_seconds.max(1) as f32;
                let progress_bar = egui::ProgressBar::new(total_progress.min(1.0))
                    .fill(Color32::from_rgb(66, 133, 244));
                ui.add(progress_bar);

                ui.add_space(8.0);

                // Current segment info
                if let Some(ref progress) = self.segment_progress {
                    ui.horizontal(|ui| {
                        // Segment type
                        let segment_name = self
                            .current_segment_type
                            .map(|t| format!("{}", t))
                            .unwrap_or_else(|| "Segment".to_string());
                        ui.label(RichText::new(segment_name).size(16.0));

                        ui.separator();

                        // Target power
                        if let Some(target) = self.target_power {
                            let offset_str = if self.power_offset != 0 {
                                format!(" ({:+}W)", self.power_offset)
                            } else {
                                String::new()
                            };
                            ui.label(
                                RichText::new(format!("Target: {}W{}", target, offset_str))
                                    .size(16.0)
                                    .color(Color32::from_rgb(251, 188, 4)),
                            );
                        }

                        ui.separator();

                        // Time remaining in segment
                        let remaining_min = progress.remaining_seconds / 60;
                        let remaining_sec = progress.remaining_seconds % 60;
                        ui.label(
                            RichText::new(format!("{}:{:02} remaining", remaining_min, remaining_sec))
                                .size(16.0),
                        );
                    });

                    // Segment progress bar
                    let segment_bar = egui::ProgressBar::new(progress.progress)
                        .fill(Color32::from_rgb(52, 168, 83));
                    ui.add(segment_bar);

                    // Text event message
                    if let Some(ref text) = self.text_event {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(text)
                                .size(14.0)
                                .italics()
                                .color(Color32::from_rgb(251, 188, 4)),
                        );
                    }
                }
            }
        });
    }

    /// Render workout-specific controls (skip, extend, power adjustment).
    fn render_workout_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 400.0) / 2.0);

            // Skip segment button
            if ui.button("⏭ Skip Segment").clicked() {
                // Signal to app to skip segment (handled in app.rs)
            }

            ui.add_space(8.0);

            // Extend segment button
            if ui.button("+30s").clicked() {
                // Signal to app to extend segment
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Power adjustment
            ui.label("Power:");

            if ui.button("-5W").clicked() {
                self.power_offset -= 5;
            }

            ui.label(
                RichText::new(format!("{:+}W", self.power_offset))
                    .color(if self.power_offset == 0 {
                        Color32::GRAY
                    } else if self.power_offset > 0 {
                        Color32::from_rgb(52, 168, 83)
                    } else {
                        Color32::from_rgb(234, 67, 53)
                    }),
            );

            if ui.button("+5W").clicked() {
                self.power_offset += 5;
            }

            if self.power_offset != 0 {
                if ui.button("Reset").clicked() {
                    self.power_offset = 0;
                }
            }

            ui.add_space(8.0);
            ui.label(RichText::new("(+/- keys)").weak().small());
        });
    }

    /// Render the full-screen mode with only essential metrics.
    fn render_full_screen_mode(&mut self, ui: &mut Ui) {
        // Fill background
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        ui.vertical_centered(|ui| {
            // Small hint at top
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button("Exit Full Screen (F/Esc)").clicked() {
                        self.full_screen_mode = false;
                    }

                    // Pause indicator
                    if self.is_paused {
                        ui.label(RichText::new("PAUSED").color(Color32::from_rgb(251, 188, 4)).size(14.0));
                    } else {
                        ui.label(RichText::new("●").color(Color32::from_rgb(234, 67, 53)));
                    }
                });
            });

            // Center the main metrics vertically
            let available_height = ui.available_height();
            let metrics_height = 300.0; // Approximate height of metrics
            ui.add_space((available_height - metrics_height) / 2.0);

            // Large power display (primary metric)
            let power_color = self
                .metrics
                .power_zone
                .map(zone_colors::power_zone_color)
                .unwrap_or(Color32::WHITE);

            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 800.0) / 2.0);

                // Power - extra large
                ui.vertical(|ui| {
                    ui.label(RichText::new("POWER").size(16.0).weak());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(
                                self.metrics.power_instant
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "--".to_string())
                            )
                                .size(96.0)
                                .color(power_color)
                                .strong(),
                        );
                        ui.label(RichText::new("W").size(32.0).weak());
                    });
                });

                ui.add_space(48.0);

                // Heart Rate
                let hr_color = self
                    .metrics
                    .hr_zone
                    .map(zone_colors::hr_zone_color)
                    .unwrap_or(Color32::WHITE);

                ui.vertical(|ui| {
                    ui.label(RichText::new("HR").size(16.0).weak());
                    ui.horizontal(|ui| {
                        let hr_text = self
                            .metrics
                            .heart_rate
                            .map(|hr| hr.to_string())
                            .unwrap_or_else(|| "--".to_string());
                        ui.label(
                            RichText::new(hr_text)
                                .size(96.0)
                                .color(hr_color)
                                .strong(),
                        );
                        ui.label(RichText::new("bpm").size(32.0).weak());
                    });
                });

                ui.add_space(48.0);

                // Cadence
                ui.vertical(|ui| {
                    ui.label(RichText::new("CADENCE").size(16.0).weak());
                    ui.horizontal(|ui| {
                        let cad_text = self
                            .metrics
                            .cadence
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "--".to_string());
                        ui.label(RichText::new(cad_text).size(96.0).strong());
                        ui.label(RichText::new("rpm").size(32.0).weak());
                    });
                });
            });

            ui.add_space(24.0);

            // Workout info if in workout mode
            if self.mode == RideMode::Workout {
                if let Some(target) = self.target_power {
                    let offset_str = if self.power_offset != 0 {
                        format!(" ({:+})", self.power_offset)
                    } else {
                        String::new()
                    };
                    ui.label(
                        RichText::new(format!("Target: {}W{}", target, offset_str))
                            .size(32.0)
                            .color(Color32::from_rgb(251, 188, 4)),
                    );
                }

                if let Some(ref progress) = self.segment_progress {
                    let remaining_min = progress.remaining_seconds / 60;
                    let remaining_sec = progress.remaining_seconds % 60;
                    ui.label(
                        RichText::new(format!("{}:{:02} remaining", remaining_min, remaining_sec))
                            .size(24.0),
                    );
                }
            }

            // Duration at bottom
            ui.add_space(24.0);
            let hours = self.elapsed_seconds / 3600;
            let minutes = (self.elapsed_seconds % 3600) / 60;
            let seconds = self.elapsed_seconds % 60;
            let duration_str = if hours > 0 {
                format!("{}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{}:{:02}", minutes, seconds)
            };
            ui.label(RichText::new(duration_str).size(48.0).weak());
        });
    }

    /// Set the dashboard layout from configuration.
    pub fn set_dashboard_layout(&mut self, layout: DashboardLayout) {
        self.dashboard_layout = layout;
    }

    /// Render a single metric based on its type.
    fn render_metric(&self, ui: &mut Ui, metric_type: MetricType, size: MetricSize) {
        match metric_type {
            MetricType::Power => {
                let power_color = self
                    .metrics
                    .power_zone
                    .map(zone_colors::power_zone_color)
                    .unwrap_or(Color32::WHITE);
                MetricDisplay::power(self.metrics.power_instant)
                    .with_size(size)
                    .with_zone_color(power_color)
                    .show(ui);
            }
            MetricType::Power3s => {
                MetricDisplay::new(
                    self.metrics
                        .power_3s_avg
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "--".to_string()),
                    "W",
                    "3s Avg",
                )
                .with_size(size)
                .show(ui);
            }
            MetricType::HeartRate => {
                let hr_color = self
                    .metrics
                    .hr_zone
                    .map(zone_colors::hr_zone_color)
                    .unwrap_or(Color32::WHITE);
                MetricDisplay::heart_rate(self.metrics.heart_rate)
                    .with_size(size)
                    .with_zone_color(hr_color)
                    .show(ui);
            }
            MetricType::Cadence => {
                MetricDisplay::cadence(self.metrics.cadence)
                    .with_size(size)
                    .show(ui);
            }
            MetricType::Speed => {
                MetricDisplay::speed(self.metrics.speed)
                    .with_size(size)
                    .show(ui);
            }
            MetricType::Distance => {
                MetricDisplay::distance(self.metrics.distance)
                    .with_size(size)
                    .show(ui);
            }
            MetricType::Duration => {
                MetricDisplay::duration(self.elapsed_seconds)
                    .with_size(size)
                    .show(ui);
            }
            MetricType::Calories => {
                MetricDisplay::new(self.metrics.calories.to_string(), "kcal", "Calories")
                    .with_size(size)
                    .show(ui);
            }
            MetricType::NormalizedPower => {
                if let Some(np) = self.metrics.normalized_power {
                    MetricDisplay::new(np.to_string(), "W", "NP")
                        .with_size(size)
                        .show(ui);
                } else {
                    MetricDisplay::new("--".to_string(), "W", "NP")
                        .with_size(size)
                        .show(ui);
                }
            }
            MetricType::Tss => {
                if let Some(tss) = self.metrics.tss {
                    MetricDisplay::new(format!("{:.0}", tss), "", "TSS")
                        .with_size(size)
                        .show(ui);
                } else {
                    MetricDisplay::new("--".to_string(), "", "TSS")
                        .with_size(size)
                        .show(ui);
                }
            }
            MetricType::IntensityFactor => {
                if let Some(if_val) = self.metrics.intensity_factor {
                    MetricDisplay::new(format!("{:.2}", if_val), "", "IF")
                        .with_size(size)
                        .show(ui);
                } else {
                    MetricDisplay::new("--".to_string(), "", "IF")
                        .with_size(size)
                        .show(ui);
                }
            }
            MetricType::PowerZone => {
                let zone_str = self
                    .metrics
                    .power_zone
                    .map(|z| format!("Z{}", z))
                    .unwrap_or_else(|| "--".to_string());
                let zone_color = self
                    .metrics
                    .power_zone
                    .map(zone_colors::power_zone_color)
                    .unwrap_or(Color32::GRAY);
                MetricDisplay::new(zone_str, "", "Power Zone")
                    .with_size(size)
                    .with_zone_color(zone_color)
                    .show(ui);
            }
            MetricType::HrZone => {
                let zone_str = self
                    .metrics
                    .hr_zone
                    .map(|z| format!("Z{}", z))
                    .unwrap_or_else(|| "--".to_string());
                let zone_color = self
                    .metrics
                    .hr_zone
                    .map(zone_colors::hr_zone_color)
                    .unwrap_or(Color32::GRAY);
                MetricDisplay::new(zone_str, "", "HR Zone")
                    .with_size(size)
                    .with_zone_color(zone_color)
                    .show(ui);
            }
        }
    }

    /// Render metrics from a configurable layout.
    #[allow(dead_code)]
    fn render_configurable_metrics(&self, ui: &mut Ui) {
        // Primary metrics (large)
        ui.horizontal(|ui| {
            let layout = &self.dashboard_layout;
            let metric_count = layout.primary_metrics.len();
            let total_width = (metric_count as f32) * 180.0 + ((metric_count - 1) as f32 * 32.0);
            ui.add_space((ui.available_width() - total_width) / 2.0);

            for (i, metric) in layout.primary_metrics.iter().enumerate() {
                self.render_metric(ui, *metric, MetricSize::Large);
                if i < layout.primary_metrics.len() - 1 {
                    ui.add_space(32.0);
                }
            }
        });

        ui.add_space(16.0);

        // Secondary metrics (medium)
        ui.horizontal(|ui| {
            let layout = &self.dashboard_layout;
            let metric_count = layout.secondary_metrics.len();
            let total_width = (metric_count as f32) * 120.0 + ((metric_count - 1) as f32 * 16.0);
            ui.add_space((ui.available_width() - total_width) / 2.0);

            for (i, metric) in layout.secondary_metrics.iter().enumerate() {
                self.render_metric(ui, *metric, MetricSize::Medium);
                if i < layout.secondary_metrics.len() - 1 {
                    ui.add_space(16.0);
                }
            }
        });

        // Tertiary metrics (small) - only if any exist
        if !self.dashboard_layout.tertiary_metrics.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let layout = &self.dashboard_layout;
                let metric_count = layout.tertiary_metrics.len();
                let total_width = (metric_count as f32) * 80.0 + ((metric_count - 1) as f32 * 16.0);
                ui.add_space((ui.available_width() - total_width) / 2.0);

                for (i, metric) in layout.tertiary_metrics.iter().enumerate() {
                    self.render_metric(ui, *metric, MetricSize::Small);
                    if i < layout.tertiary_metrics.len() - 1 {
                        ui.add_space(16.0);
                    }
                }
            });
        }
    }
}
