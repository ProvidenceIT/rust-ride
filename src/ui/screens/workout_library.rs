//! Workout library screen implementation.
//!
//! T073: Implement workout library list with name, duration, TSS
//! T074: Implement workout import file picker
//! T075: Implement workout preview with profile graph

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::workouts::types::{Workout, WorkoutFormat, WorkoutParseError};

use super::Screen;

/// Workout library screen state.
#[derive(Default)]
pub struct WorkoutLibraryScreen {
    /// List of available workouts
    pub workouts: Vec<Workout>,
    /// Selected workout index
    pub selected_index: Option<usize>,
    /// Search filter
    pub search_query: String,
    /// Filter by tag
    pub tag_filter: Option<String>,
    /// Show import dialog
    pub show_import_dialog: bool,
    /// Error message to display
    pub error_message: Option<String>,
    /// Show detailed error dialog
    pub show_error_dialog: bool,
    /// Detailed error info
    pub error_details: Option<WorkoutImportError>,
}

/// Detailed information about a workout import error.
#[derive(Debug, Clone)]
pub struct WorkoutImportError {
    /// The filename that failed to import
    pub filename: String,
    /// The error that occurred
    pub error_message: String,
    /// Helpful suggestions for the user
    pub suggestions: Vec<String>,
}

impl WorkoutLibraryScreen {
    /// Create a new workout library screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the workouts list.
    pub fn set_workouts(&mut self, workouts: Vec<Workout>) {
        self.workouts = workouts;
    }

    /// Add a workout to the library.
    pub fn add_workout(&mut self, workout: Workout) {
        self.workouts.push(workout);
    }

    /// Handle a workout parse error with user-friendly messaging.
    pub fn handle_parse_error(&mut self, filename: &str, error: WorkoutParseError) {
        let (error_message, suggestions) = match &error {
            WorkoutParseError::IoError(msg) => (
                format!("Could not read file: {}", msg),
                vec![
                    "Check that the file exists and is not corrupted".to_string(),
                    "Ensure you have permission to read the file".to_string(),
                ],
            ),
            WorkoutParseError::InvalidXml(msg) => (
                format!("Invalid XML format: {}", msg),
                vec![
                    "The file may be corrupted or not a valid workout file".to_string(),
                    "Try re-exporting the workout from the original application".to_string(),
                    "Make sure the file has a .zwo extension for Zwift workouts".to_string(),
                ],
            ),
            WorkoutParseError::UnsupportedFormat(msg) => (
                format!("Unsupported workout format: {}", msg),
                vec![
                    "This file type may not be supported".to_string(),
                    "Supported formats: .zwo (Zwift), .mrc (TrainerRoad)".to_string(),
                    "Try converting the workout to a supported format".to_string(),
                ],
            ),
            WorkoutParseError::MissingField(field) => (
                format!("Missing required field: {}", field),
                vec![
                    "The workout file is missing required data".to_string(),
                    "Try re-exporting from the original application".to_string(),
                    format!("Required field: {}", field),
                ],
            ),
            WorkoutParseError::InvalidValue { field, value } => (
                format!("Invalid value '{}' for field '{}'", value, field),
                vec![
                    "The workout file may have been manually edited incorrectly".to_string(),
                    "Try using the original workout file".to_string(),
                ],
            ),
            WorkoutParseError::EmptyWorkout => (
                "The workout file contains no segments".to_string(),
                vec![
                    "The workout file appears to be empty".to_string(),
                    "Try re-exporting from the original application".to_string(),
                ],
            ),
        };

        self.error_details = Some(WorkoutImportError {
            filename: filename.to_string(),
            error_message,
            suggestions,
        });
        self.show_error_dialog = true;
        self.error_message = Some(format!("Failed to import: {}", filename));
    }

    /// Clear any error state.
    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.error_details = None;
        self.show_error_dialog = false;
    }

    /// Render the workout library screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<(Screen, Option<Workout>)> {
        let mut result = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    result = Some((Screen::Home, None));
                }
                ui.heading("Workout Library");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Import").clicked() {
                        self.show_import_dialog = true;
                    }
                });
            });

            ui.add_space(8.0);

            // Search and filter bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                ui.separator();

                if ui.button("Clear").clicked() {
                    self.search_query.clear();
                    self.tag_filter = None;
                }
            });

            ui.add_space(8.0);

            // Error message
            if let Some(ref error) = self.error_message {
                ui.colored_label(Color32::from_rgb(234, 67, 53), error);
                ui.add_space(8.0);
            }

            ui.separator();

            // Two-column layout: workout list | preview
            ui.columns(2, |columns| {
                // Left column: Workout list
                columns[0].vertical(|ui| {
                    ui.heading("Workouts");
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height() - 60.0)
                        .show(ui, |ui| {
                            // Clone filtered workouts to avoid borrow conflicts
                            let filtered_workouts: Vec<_> =
                                self.filter_workouts().into_iter().cloned().collect();

                            if filtered_workouts.is_empty() {
                                ui.label(RichText::new("No workouts found").weak());
                                ui.label(
                                    RichText::new("Import a .zwo or .mrc file to get started")
                                        .weak(),
                                );
                            } else {
                                for (i, workout) in filtered_workouts.iter().enumerate() {
                                    let is_selected = self.selected_index == Some(i);
                                    if self.render_workout_item(ui, workout, is_selected) {
                                        self.selected_index = Some(i);
                                    }
                                }
                            }
                        });
                });

                // Right column: Preview
                columns[1].vertical(|ui| {
                    ui.heading("Preview");
                    ui.add_space(8.0);

                    if let Some(idx) = self.selected_index {
                        let filtered = self.filter_workouts();
                        if let Some(workout) = filtered.get(idx) {
                            if let Some((screen, selected)) =
                                self.render_workout_preview(ui, workout)
                            {
                                result = Some((screen, selected));
                            }
                        }
                    } else {
                        ui.label(RichText::new("Select a workout to preview").weak());
                    }
                });
            });
        });

        // Import dialog
        if self.show_import_dialog {
            self.render_import_dialog(ui);
        }

        // Error dialog
        if self.show_error_dialog {
            self.render_error_dialog(ui);
        }

        result
    }

    /// Get workouts filtered by search query and tag.
    fn filter_workouts(&self) -> Vec<&Workout> {
        self.workouts
            .iter()
            .filter(|w| {
                // Filter by search query
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !w.name.to_lowercase().contains(&query)
                        && !w
                            .description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                    {
                        return false;
                    }
                }

                // Filter by tag
                if let Some(ref tag) = self.tag_filter {
                    if !w.tags.contains(tag) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Render a single workout item in the list.
    fn render_workout_item(&self, ui: &mut Ui, workout: &Workout, is_selected: bool) -> bool {
        let mut clicked = false;

        let fill_color = if is_selected {
            Color32::from_rgb(66, 133, 244).linear_multiply(0.3)
        } else {
            ui.visuals().faint_bg_color
        };

        let frame = egui::Frame::new()
            .fill(fill_color)
            .inner_margin(12.0)
            .corner_radius(4.0);

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let response =
                ui.allocate_response(Vec2::new(ui.available_width(), 0.0), egui::Sense::click());

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&workout.name).strong());

                    ui.horizontal(|ui| {
                        // Duration
                        let duration_min = workout.total_duration_seconds / 60;
                        ui.label(RichText::new(format!("{}min", duration_min)).weak());

                        // TSS
                        if let Some(tss) = workout.estimated_tss {
                            ui.label(RichText::new(format!("TSS: {:.0}", tss)).weak());
                        }

                        // IF
                        if let Some(if_val) = workout.estimated_if {
                            ui.label(RichText::new(format!("IF: {:.2}", if_val)).weak());
                        }
                    });

                    // Tags
                    if !workout.tags.is_empty() {
                        ui.horizontal(|ui| {
                            for tag in &workout.tags {
                                ui.label(
                                    RichText::new(tag)
                                        .small()
                                        .color(Color32::from_rgb(66, 133, 244)),
                                );
                            }
                        });
                    }
                });
            });

            if response.clicked() {
                clicked = true;
            }
        });

        ui.add_space(4.0);

        clicked
    }

    /// Render the workout preview panel.
    fn render_workout_preview(
        &self,
        ui: &mut Ui,
        workout: &Workout,
    ) -> Option<(Screen, Option<Workout>)> {
        let mut result = None;

        // Workout info
        ui.label(RichText::new(&workout.name).size(18.0).strong());

        if let Some(ref author) = workout.author {
            ui.label(RichText::new(format!("by {}", author)).weak());
        }

        ui.add_space(8.0);

        if let Some(ref description) = workout.description {
            ui.label(description);
            ui.add_space(8.0);
        }

        // Stats
        ui.horizontal(|ui| {
            let duration_min = workout.total_duration_seconds / 60;
            ui.label(format!("Duration: {} min", duration_min));
        });

        if let Some(tss) = workout.estimated_tss {
            ui.label(format!("Estimated TSS: {:.0}", tss));
        }

        if let Some(if_val) = workout.estimated_if {
            ui.label(format!("Intensity Factor: {:.2}", if_val));
        }

        ui.add_space(8.0);

        // Segments summary
        ui.label(RichText::new("Segments:").strong());
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (i, segment) in workout.segments.iter().enumerate() {
                    let duration_min = segment.duration_seconds / 60;
                    let duration_sec = segment.duration_seconds % 60;

                    let power_text = match &segment.power_target {
                        crate::workouts::types::PowerTarget::Absolute { watts } => {
                            format!("{}W", watts)
                        }
                        crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                            format!("{}%", percent)
                        }
                        crate::workouts::types::PowerTarget::Range { start, end } => {
                            match (start.as_ref(), end.as_ref()) {
                                (
                                    crate::workouts::types::PowerTarget::PercentFtp { percent: s },
                                    crate::workouts::types::PowerTarget::PercentFtp { percent: e },
                                ) => format!("{}% → {}%", s, e),
                                _ => "Ramp".to_string(),
                            }
                        }
                    };

                    ui.label(format!(
                        "{}. {} - {}:{:02} @ {}",
                        i + 1,
                        segment.segment_type,
                        duration_min,
                        duration_sec,
                        power_text
                    ));
                }
            });

        ui.add_space(16.0);

        // Start workout button
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    Vec2::new(150.0, 40.0),
                    egui::Button::new(RichText::new("Start Workout").size(16.0))
                        .fill(Color32::from_rgb(52, 168, 83)),
                )
                .clicked()
            {
                result = Some((Screen::Ride, Some(workout.clone())));
            }
        });

        result
    }

    /// Render the import dialog.
    fn render_import_dialog(&mut self, ui: &mut Ui) {
        egui::Window::new("Import Workout")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(400.0, 200.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label("Supported formats:");
                    ui.label(RichText::new(".zwo (Zwift)").weak());
                    ui.label(RichText::new(".mrc (TrainerRoad/Generic)").weak());

                    ui.add_space(16.0);

                    ui.label("Drag and drop a workout file here,");
                    ui.label("or click Browse to select a file.");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Browse...").clicked() {
                            // TODO: Open file picker
                            // For now, just close the dialog
                            self.error_message = Some("File picker not yet implemented. Place .zwo or .mrc files in the workouts folder.".to_string());
                        }

                        ui.add_space(16.0);

                        if ui.button("Cancel").clicked() {
                            self.show_import_dialog = false;
                        }
                    });
                });
            });

        // Close dialog on click outside (simplified)
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_import_dialog = false;
        }
    }

    /// Render the error dialog for invalid workout files.
    fn render_error_dialog(&mut self, ui: &mut Ui) {
        let error = match &self.error_details {
            Some(e) => e.clone(),
            None => return,
        };

        egui::Window::new("Import Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(450.0, 250.0));

                ui.vertical(|ui| {
                    // Error icon and title
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("⚠")
                                .size(24.0)
                                .color(Color32::from_rgb(234, 67, 53)),
                        );
                        ui.label(
                            RichText::new("Failed to Import Workout")
                                .size(18.0)
                                .strong(),
                        );
                    });

                    ui.add_space(12.0);

                    // Filename
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("File:").strong());
                        ui.label(&error.filename);
                    });

                    ui.add_space(8.0);

                    // Error message
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width() - 8.0);
                        ui.label(
                            RichText::new(&error.error_message)
                                .color(Color32::from_rgb(234, 67, 53)),
                        );
                    });

                    ui.add_space(12.0);

                    // Suggestions
                    if !error.suggestions.is_empty() {
                        ui.label(RichText::new("Suggestions:").strong());
                        ui.add_space(4.0);

                        for suggestion in &error.suggestions {
                            ui.horizontal(|ui| {
                                ui.label("•");
                                ui.label(suggestion);
                            });
                        }
                    }

                    ui.add_space(16.0);

                    // Close button
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.clear_error();
                            }
                        });
                    });
                });
            });

        // Close on Escape
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.clear_error();
        }
    }
}

/// Format a workout format for display.
fn _format_workout_format(format: WorkoutFormat) -> &'static str {
    match format {
        WorkoutFormat::Zwo => "Zwift (.zwo)",
        WorkoutFormat::Mrc => "TrainerRoad (.mrc)",
        WorkoutFormat::Fit => "Garmin (.fit)",
        WorkoutFormat::Native => "RustRide",
    }
}
