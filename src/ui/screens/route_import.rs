//! Route import screen for importing GPX, FIT, and TCX files.
//!
//! T040: Create route import UI screen in src/ui/screens/route_import.rs

use egui::{Align, Color32, Layout, ProgressBar, RichText, Ui, Vec2};

use crate::storage::config::Units;
use crate::world::import::{
    FileFormat, GeoBounds, ImportError, ImportProgress, ImportStage, RoutePreview,
};
use crate::world::route::RouteSource;

use super::Screen;

/// Route import screen state.
#[derive(Default)]
pub struct RouteImportScreen {
    /// Currently selected file path
    pub selected_file: Option<String>,
    /// Preview of the route (before full import)
    pub preview: Option<RoutePreview>,
    /// Whether we're currently importing
    pub is_importing: bool,
    /// Import progress
    pub progress: Option<ImportProgress>,
    /// Error message to display
    pub error_message: Option<String>,
    /// Success message after import
    pub success_message: Option<String>,
    /// Name override for the route
    pub name_override: String,
    /// Whether to fetch missing elevation data
    pub fetch_elevation: bool,
    /// Unit preference for display
    pub units: Units,
    /// Show file browser dialog
    pub show_file_dialog: bool,
}

/// Result from route import screen actions.
#[derive(Debug, Clone)]
pub enum RouteImportAction {
    /// Import completed successfully
    ImportComplete {
        route_id: uuid::Uuid,
        route_name: String,
    },
    /// Navigate to another screen
    Navigate(Screen),
    /// Request to browse for file
    BrowseFiles,
    /// Request to start import
    StartImport { path: String, name: Option<String> },
}

impl RouteImportScreen {
    /// Create a new route import screen.
    pub fn new() -> Self {
        Self {
            fetch_elevation: true,
            ..Default::default()
        }
    }

    /// Set the unit preference.
    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    /// Set the selected file and clear previous state.
    pub fn set_file(&mut self, path: String) {
        self.selected_file = Some(path);
        self.preview = None;
        self.error_message = None;
        self.success_message = None;
        self.name_override.clear();
    }

    /// Set the route preview.
    pub fn set_preview(&mut self, preview: RoutePreview) {
        self.name_override = preview.name.clone();
        self.preview = Some(preview);
    }

    /// Set import progress.
    pub fn set_progress(&mut self, progress: ImportProgress) {
        self.progress = Some(progress);
    }

    /// Set error message.
    pub fn set_error(&mut self, error: ImportError) {
        self.is_importing = false;
        self.progress = None;
        self.error_message = Some(format_import_error(&error));
    }

    /// Set success after import.
    pub fn set_success(&mut self, route_name: &str) {
        self.is_importing = false;
        self.progress = None;
        self.success_message = Some(format!("Successfully imported '{}'", route_name));
    }

    /// Clear any error state.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Reset the screen for a new import.
    pub fn reset(&mut self) {
        self.selected_file = None;
        self.preview = None;
        self.is_importing = false;
        self.progress = None;
        self.error_message = None;
        self.success_message = None;
        self.name_override.clear();
    }

    /// Render the route import screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<RouteImportAction> {
        let mut action = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    action = Some(RouteImportAction::Navigate(Screen::Home));
                }
                ui.heading("Import Route");
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // File selection area
            ui.group(|ui| {
                ui.set_min_width(ui.available_width() - 16.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Select Route File").size(16.0).strong());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Browse...").clicked() {
                            action = Some(RouteImportAction::BrowseFiles);
                        }
                    });
                });

                ui.add_space(8.0);

                // Supported formats
                ui.horizontal(|ui| {
                    ui.label("Supported formats:");
                    ui.label(
                        RichText::new(".gpx")
                            .monospace()
                            .color(format_color(FileFormat::Gpx)),
                    );
                    ui.label(
                        RichText::new(".fit")
                            .monospace()
                            .color(format_color(FileFormat::Fit)),
                    );
                    ui.label(
                        RichText::new(".tcx")
                            .monospace()
                            .color(format_color(FileFormat::Tcx)),
                    );
                });

                ui.add_space(8.0);

                // Selected file display
                if let Some(ref path) = self.selected_file {
                    ui.horizontal(|ui| {
                        ui.label("Selected:");
                        ui.label(RichText::new(path).monospace());
                    });
                } else {
                    ui.label(RichText::new("No file selected").weak());
                    ui.add_space(8.0);
                    ui.label("Drag and drop a route file here, or click Browse.");
                }
            });

            ui.add_space(16.0);

            // Error message
            if let Some(ref error) = self.error_message {
                self.render_error(ui, error);
                ui.add_space(8.0);
            }

            // Success message
            if let Some(ref success) = self.success_message {
                self.render_success(ui, success);
                ui.add_space(8.0);
            }

            // Route preview (if available)
            if let Some(ref preview) = self.preview.clone() {
                if let Some(act) = self.render_preview(ui, preview) {
                    action = Some(act);
                }
            }

            // Import progress
            if self.is_importing {
                if let Some(ref progress) = self.progress {
                    self.render_progress(ui, progress);
                }
            }
        });

        action
    }

    /// Render the route preview panel.
    fn render_preview(&mut self, ui: &mut Ui, preview: &RoutePreview) -> Option<RouteImportAction> {
        let mut action = None;

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Route Preview").size(16.0).strong());
            ui.add_space(8.0);

            // Route name (editable)
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.name_override);
            });

            ui.add_space(8.0);

            // Statistics in a grid-like layout
            ui.columns(2, |columns| {
                // Left column
                columns[0].vertical(|ui| {
                    // Distance
                    let (dist, dist_unit) = match self.units {
                        Units::Metric => (preview.distance_km, "km"),
                        Units::Imperial => (preview.distance_km * 0.621371, "mi"),
                    };
                    ui.horizontal(|ui| {
                        ui.label("Distance:");
                        ui.label(RichText::new(format!("{:.1} {}", dist, dist_unit)).strong());
                    });

                    // Elevation gain
                    let (elev, elev_unit) = match self.units {
                        Units::Metric => (preview.elevation_gain_m, "m"),
                        Units::Imperial => (preview.elevation_gain_m * 3.28084, "ft"),
                    };
                    ui.horizontal(|ui| {
                        ui.label("Elevation Gain:");
                        ui.label(RichText::new(format!("{:.0} {}", elev, elev_unit)).strong());
                    });
                });

                // Right column
                columns[1].vertical(|ui| {
                    // Point count
                    ui.horizontal(|ui| {
                        ui.label("GPS Points:");
                        ui.label(RichText::new(format!("{}", preview.point_count)).strong());
                    });

                    // Elevation data status
                    ui.horizontal(|ui| {
                        ui.label("Elevation Data:");
                        if preview.has_elevation {
                            ui.label(
                                RichText::new("Yes")
                                    .strong()
                                    .color(Color32::from_rgb(76, 175, 80)),
                            );
                        } else {
                            ui.label(
                                RichText::new("Missing")
                                    .strong()
                                    .color(Color32::from_rgb(255, 152, 0)),
                            );
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Bounds info
            self.render_bounds(ui, &preview.bounds);

            ui.add_space(8.0);

            // Elevation fetch option (if missing)
            if !preview.has_elevation {
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.fetch_elevation,
                        "Fetch elevation data from online service",
                    );
                });
                ui.label(
                    RichText::new("Note: Elevation fetching requires internet connection")
                        .weak()
                        .small(),
                );
                ui.add_space(8.0);
            }

            ui.add_space(8.0);

            // Import button
            ui.horizontal(|ui| {
                let button = egui::Button::new(RichText::new("Import Route").size(16.0))
                    .fill(Color32::from_rgb(52, 168, 83));

                if ui.add_sized(Vec2::new(150.0, 40.0), button).clicked() {
                    if let Some(ref path) = self.selected_file {
                        self.is_importing = true;
                        let name = if self.name_override.is_empty() {
                            None
                        } else {
                            Some(self.name_override.clone())
                        };
                        action = Some(RouteImportAction::StartImport {
                            path: path.clone(),
                            name,
                        });
                    }
                }

                ui.add_space(16.0);

                if ui.button("Cancel").clicked() {
                    self.reset();
                }
            });
        });

        action
    }

    /// Render geographic bounds info.
    fn render_bounds(&self, ui: &mut Ui, bounds: &GeoBounds) {
        ui.collapsing("Geographic Bounds", |ui| {
            ui.horizontal(|ui| {
                ui.label("Latitude:");
                ui.label(
                    RichText::new(format!("{:.4}° to {:.4}°", bounds.min_lat, bounds.max_lat))
                        .monospace(),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Longitude:");
                ui.label(
                    RichText::new(format!("{:.4}° to {:.4}°", bounds.min_lon, bounds.max_lon))
                        .monospace(),
                );
            });
        });
    }

    /// Render import progress.
    fn render_progress(&self, ui: &mut Ui, progress: &ImportProgress) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Importing...").strong());
                ui.spinner();
            });

            ui.add_space(8.0);

            // Stage indicator
            let stage_text = match progress.stage {
                ImportStage::Parsing => "Parsing file...",
                ImportStage::FetchingElevation => "Fetching elevation data...",
                ImportStage::GeneratingTerrain => "Generating terrain...",
                ImportStage::Saving => "Saving to database...",
            };
            ui.label(stage_text);

            ui.add_space(4.0);

            // Progress bar
            ui.add(ProgressBar::new(progress.percent / 100.0).show_percentage());

            if !progress.message.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&progress.message).weak().small());
            }
        });
    }

    /// Render error message.
    fn render_error(&self, ui: &mut Ui, error: &str) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Error")
                        .strong()
                        .color(Color32::from_rgb(234, 67, 53)),
                );
            });

            ui.add_space(4.0);
            ui.label(RichText::new(error).color(Color32::from_rgb(234, 67, 53)));
        });
    }

    /// Render success message.
    fn render_success(&self, ui: &mut Ui, message: &str) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Success")
                        .strong()
                        .color(Color32::from_rgb(76, 175, 80)),
                );
            });

            ui.add_space(4.0);
            ui.label(RichText::new(message).color(Color32::from_rgb(76, 175, 80)));

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Import Another").clicked() {
                    // Reset will be handled by the caller
                }
                ui.add_space(8.0);
                if ui.button("View Routes").clicked() {
                    // Navigate to route browser - handled by caller
                }
            });
        });
    }
}

/// Format file extension color.
fn format_color(format: FileFormat) -> Color32 {
    match format {
        FileFormat::Gpx => Color32::from_rgb(76, 175, 80), // Green
        FileFormat::Fit => Color32::from_rgb(66, 133, 244), // Blue
        FileFormat::Tcx => Color32::from_rgb(255, 152, 0), // Orange
    }
}

/// Format import error for display.
fn format_import_error(error: &ImportError) -> String {
    match error {
        ImportError::FileNotFound(path) => {
            format!("File not found: {}", path.display())
        }
        ImportError::InvalidFormat(msg) => {
            format!("Invalid file format: {}", msg)
        }
        ImportError::ParseError(msg) => {
            format!("Failed to parse file: {}", msg)
        }
        ImportError::TooLarge { size_mb, max_mb } => {
            format!(
                "File too large: {:.1}MB exceeds maximum {:.1}MB",
                size_mb, max_mb
            )
        }
        ImportError::RouteTooLong {
            distance_km,
            max_km,
        } => {
            format!(
                "Route too long: {:.1}km exceeds maximum {:.1}km",
                distance_km, max_km
            )
        }
        ImportError::ElevationFetchFailed(msg) => {
            format!("Failed to fetch elevation data: {}", msg)
        }
        ImportError::DatabaseError(msg) => {
            format!("Database error: {}", msg)
        }
        ImportError::IoError(e) => {
            format!("IO error: {}", e)
        }
    }
}

/// Format route source for display.
pub fn format_route_source(source: RouteSource) -> &'static str {
    match source {
        RouteSource::Gpx => "GPX",
        RouteSource::Fit => "FIT",
        RouteSource::Tcx => "TCX",
        RouteSource::Custom => "Custom",
        RouteSource::Famous => "Famous",
        RouteSource::Procedural => "Generated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_import_screen_new() {
        let screen = RouteImportScreen::new();
        assert!(screen.selected_file.is_none());
        assert!(screen.preview.is_none());
        assert!(!screen.is_importing);
        assert!(screen.fetch_elevation);
    }

    #[test]
    fn test_set_file() {
        let mut screen = RouteImportScreen::new();
        screen.set_file("/path/to/route.gpx".to_string());

        assert_eq!(screen.selected_file, Some("/path/to/route.gpx".to_string()));
        assert!(screen.error_message.is_none());
    }

    #[test]
    fn test_set_preview() {
        let mut screen = RouteImportScreen::new();
        let preview = RoutePreview {
            name: "Test Route".to_string(),
            point_count: 100,
            distance_km: 25.0,
            elevation_gain_m: 500.0,
            has_elevation: true,
            bounds: GeoBounds::default(),
        };

        screen.set_preview(preview);

        assert!(screen.preview.is_some());
        assert_eq!(screen.name_override, "Test Route");
    }

    #[test]
    fn test_reset() {
        let mut screen = RouteImportScreen::new();
        screen.set_file("/path/to/route.gpx".to_string());
        screen.is_importing = true;
        screen.error_message = Some("Error".to_string());

        screen.reset();

        assert!(screen.selected_file.is_none());
        assert!(!screen.is_importing);
        assert!(screen.error_message.is_none());
    }

    #[test]
    fn test_format_import_error() {
        let error = ImportError::FileNotFound(std::path::PathBuf::from("/test/path.gpx"));
        let formatted = format_import_error(&error);
        assert!(formatted.contains("not found"));

        let error = ImportError::TooLarge {
            size_mb: 50.0,
            max_mb: 10.0,
        };
        let formatted = format_import_error(&error);
        assert!(formatted.contains("50.0MB"));
    }

    #[test]
    fn test_format_route_source() {
        assert_eq!(format_route_source(RouteSource::Gpx), "GPX");
        assert_eq!(format_route_source(RouteSource::Fit), "FIT");
        assert_eq!(format_route_source(RouteSource::Famous), "Famous");
    }
}
