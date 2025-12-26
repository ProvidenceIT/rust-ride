//! Route browser screen for viewing and selecting imported routes.
//!
//! T041: Create route browser UI screen in src/ui/screens/route_browser.rs
//! T086: Add famous routes category to route browser
//! T098: Add difficulty modifier UI to route settings
//! T105: Add route recommendations section to UI

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui, Vec2};

use crate::storage::config::Units;
use crate::world::route::{
    GradientScaler, GradientScalingMode, RecommendationCriteria, RouteRecommendation,
    RouteRecommender, RouteSource, StoredRoute, TrainingGoalType,
};
use crate::world::worlds::famous_routes::FamousRoutesLibrary;

use super::route_import::format_route_source;
use super::Screen;

/// Route browser screen state.
#[derive(Default)]
pub struct RouteBrowserScreen {
    /// Available routes
    pub routes: Vec<StoredRoute>,
    /// Selected route index
    pub selected_index: Option<usize>,
    /// Search query
    pub search_query: String,
    /// Filter by source type
    pub source_filter: Option<RouteSource>,
    /// Sort order
    pub sort_order: RouteSortOrder,
    /// Unit preference
    pub units: Units,
    /// Show delete confirmation dialog
    pub show_delete_confirm: bool,
    /// Route to delete
    pub pending_delete: Option<uuid::Uuid>,
    /// T098: Gradient scaler for difficulty adjustment
    pub gradient_scaler: GradientScaler,
    /// T098: Show difficulty settings panel
    pub show_difficulty_settings: bool,
    /// T105: Route recommender instance
    pub recommender: RouteRecommender,
    /// T105: Show recommendations panel
    pub show_recommendations: bool,
    /// T105: Current recommendation criteria
    pub recommendation_criteria: RecommendationCriteria,
    /// T105: Cached recommendations
    pub cached_recommendations: Vec<RouteRecommendation>,
}

/// Sort order for routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RouteSortOrder {
    #[default]
    NameAsc,
    NameDesc,
    DistanceAsc,
    DistanceDesc,
    ElevationAsc,
    ElevationDesc,
    DateAsc,
    DateDesc,
}

/// Action from route browser.
#[derive(Debug, Clone)]
pub enum RouteBrowserAction {
    /// Navigate to a screen
    Navigate(Screen),
    /// Start a ride on selected route with difficulty settings
    StartRide(StoredRoute, GradientScaler),
    /// Delete a route
    DeleteRoute(uuid::Uuid),
    /// Navigate to import screen
    ImportRoute,
}

impl RouteBrowserScreen {
    /// Create a new route browser screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the routes list.
    pub fn set_routes(&mut self, routes: Vec<StoredRoute>) {
        self.routes = routes;
        self.sort_routes();
    }

    /// Set routes and include famous routes automatically.
    ///
    /// This combines user-imported routes with the built-in famous pro cycling
    /// routes (L'Alpe d'Huez, Mont Ventoux, Stelvio, etc.) for a complete library.
    pub fn set_routes_with_famous(&mut self, mut routes: Vec<StoredRoute>) {
        // Add famous routes from the library
        let famous_library = FamousRoutesLibrary::new();
        let famous_routes = famous_library.as_stored_routes();
        routes.extend(famous_routes);

        self.routes = routes;
        self.sort_routes();
    }

    /// Get just the famous routes (for display in a dedicated section).
    pub fn get_famous_routes() -> Vec<StoredRoute> {
        FamousRoutesLibrary::new().as_stored_routes()
    }

    /// Set the unit preference.
    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    /// T105: Update recommendations based on current criteria.
    pub fn update_recommendations(&mut self) {
        self.cached_recommendations = self
            .recommender
            .recommend(&self.routes, &self.recommendation_criteria);
    }

    /// T105: Mark a route as recently ridden (for variety scoring).
    pub fn mark_route_ridden(&mut self, route_id: uuid::Uuid) {
        self.recommender.mark_ridden(route_id);
    }

    /// Sort routes based on current sort order.
    fn sort_routes(&mut self) {
        match self.sort_order {
            RouteSortOrder::NameAsc => {
                self.routes
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            RouteSortOrder::NameDesc => {
                self.routes
                    .sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()));
            }
            RouteSortOrder::DistanceAsc => {
                self.routes
                    .sort_by(|a, b| a.distance_meters.partial_cmp(&b.distance_meters).unwrap());
            }
            RouteSortOrder::DistanceDesc => {
                self.routes
                    .sort_by(|a, b| b.distance_meters.partial_cmp(&a.distance_meters).unwrap());
            }
            RouteSortOrder::ElevationAsc => {
                self.routes.sort_by(|a, b| {
                    a.elevation_gain_meters
                        .partial_cmp(&b.elevation_gain_meters)
                        .unwrap()
                });
            }
            RouteSortOrder::ElevationDesc => {
                self.routes.sort_by(|a, b| {
                    b.elevation_gain_meters
                        .partial_cmp(&a.elevation_gain_meters)
                        .unwrap()
                });
            }
            RouteSortOrder::DateAsc => {
                self.routes.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
            RouteSortOrder::DateDesc => {
                self.routes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }
    }

    /// Get filtered routes based on search and filter settings.
    fn filter_routes(&self) -> Vec<&StoredRoute> {
        self.routes
            .iter()
            .filter(|route| {
                // Filter by search query
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !route.name.to_lowercase().contains(&query) {
                        return false;
                    }
                }

                // Filter by source type
                if let Some(source) = self.source_filter {
                    if route.source != source {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Render the route browser screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<RouteBrowserAction> {
        let mut action = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    action = Some(RouteBrowserAction::Navigate(Screen::Home));
                }
                ui.heading("Route Library");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Import Route").clicked() {
                        action = Some(RouteBrowserAction::ImportRoute);
                    }
                });
            });

            ui.add_space(8.0);

            // Search and filter bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.add_sized(
                    Vec2::new(200.0, 20.0),
                    egui::TextEdit::singleline(&mut self.search_query),
                );

                ui.separator();

                // T098: Difficulty settings button
                if ui
                    .button(format!(
                        "⚙ Difficulty: {}",
                        self.gradient_scaler.description()
                    ))
                    .clicked()
                {
                    self.show_difficulty_settings = !self.show_difficulty_settings;
                }

                // T105: Recommendations button
                let rec_button_text = if self.show_recommendations {
                    "★ Recommendations ▼"
                } else {
                    "★ Recommendations"
                };
                if ui.button(rec_button_text).clicked() {
                    self.show_recommendations = !self.show_recommendations;
                    if self.show_recommendations {
                        self.update_recommendations();
                    }
                }

                ui.separator();

                ui.label("Source:");
                let current_filter = self.source_filter.map(format_route_source).unwrap_or("All");
                egui::ComboBox::from_id_salt("source_filter")
                    .selected_text(current_filter)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.source_filter.is_none(), "All")
                            .clicked()
                        {
                            self.source_filter = None;
                        }
                        if ui
                            .selectable_label(self.source_filter == Some(RouteSource::Gpx), "GPX")
                            .clicked()
                        {
                            self.source_filter = Some(RouteSource::Gpx);
                        }
                        if ui
                            .selectable_label(self.source_filter == Some(RouteSource::Fit), "FIT")
                            .clicked()
                        {
                            self.source_filter = Some(RouteSource::Fit);
                        }
                        if ui
                            .selectable_label(self.source_filter == Some(RouteSource::Tcx), "TCX")
                            .clicked()
                        {
                            self.source_filter = Some(RouteSource::Tcx);
                        }
                        if ui
                            .selectable_label(
                                self.source_filter == Some(RouteSource::Famous),
                                "Famous",
                            )
                            .clicked()
                        {
                            self.source_filter = Some(RouteSource::Famous);
                        }
                    });

                ui.separator();

                ui.label("Sort:");
                let sort_text = match self.sort_order {
                    RouteSortOrder::NameAsc => "Name (A-Z)",
                    RouteSortOrder::NameDesc => "Name (Z-A)",
                    RouteSortOrder::DistanceAsc => "Distance (Low-High)",
                    RouteSortOrder::DistanceDesc => "Distance (High-Low)",
                    RouteSortOrder::ElevationAsc => "Elevation (Low-High)",
                    RouteSortOrder::ElevationDesc => "Elevation (High-Low)",
                    RouteSortOrder::DateAsc => "Date (Oldest)",
                    RouteSortOrder::DateDesc => "Date (Newest)",
                };
                egui::ComboBox::from_id_salt("sort_order")
                    .selected_text(sort_text)
                    .show_ui(ui, |ui| {
                        let orders = [
                            (RouteSortOrder::NameAsc, "Name (A-Z)"),
                            (RouteSortOrder::NameDesc, "Name (Z-A)"),
                            (RouteSortOrder::DistanceDesc, "Distance (High-Low)"),
                            (RouteSortOrder::DistanceAsc, "Distance (Low-High)"),
                            (RouteSortOrder::ElevationDesc, "Elevation (High-Low)"),
                            (RouteSortOrder::ElevationAsc, "Elevation (Low-High)"),
                            (RouteSortOrder::DateDesc, "Date (Newest)"),
                            (RouteSortOrder::DateAsc, "Date (Oldest)"),
                        ];
                        for (order, label) in orders {
                            if ui
                                .selectable_label(self.sort_order == order, label)
                                .clicked()
                            {
                                self.sort_order = order;
                                self.sort_routes();
                            }
                        }
                    });

                if ui.button("Clear").clicked() {
                    self.search_query.clear();
                    self.source_filter = None;
                }
            });

            ui.add_space(8.0);

            // T098: Difficulty settings panel (collapsible)
            if self.show_difficulty_settings {
                ui.group(|ui| {
                    ui.heading("Difficulty Settings");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        egui::ComboBox::from_id_salt("difficulty_mode")
                            .selected_text(match self.gradient_scaler.mode {
                                GradientScalingMode::Original => "Original",
                                GradientScalingMode::Fixed => "Fixed %",
                                GradientScalingMode::Adaptive => "Adaptive (FTP)",
                            })
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.gradient_scaler.mode == GradientScalingMode::Original,
                                        "Original",
                                    )
                                    .clicked()
                                {
                                    self.gradient_scaler.mode = GradientScalingMode::Original;
                                }
                                if ui
                                    .selectable_label(
                                        self.gradient_scaler.mode == GradientScalingMode::Fixed,
                                        "Fixed %",
                                    )
                                    .clicked()
                                {
                                    self.gradient_scaler.mode = GradientScalingMode::Fixed;
                                }
                                if ui
                                    .selectable_label(
                                        self.gradient_scaler.mode == GradientScalingMode::Adaptive,
                                        "Adaptive (FTP)",
                                    )
                                    .clicked()
                                {
                                    self.gradient_scaler.mode = GradientScalingMode::Adaptive;
                                }
                            });
                    });

                    match self.gradient_scaler.mode {
                        GradientScalingMode::Original => {
                            ui.label("Routes will use original gradients.");
                        }
                        GradientScalingMode::Fixed => {
                            ui.horizontal(|ui| {
                                ui.label("Gradient scale:");
                                let mut scale_pct =
                                    (self.gradient_scaler.fixed_scale * 100.0) as i32;
                                ui.add(egui::Slider::new(&mut scale_pct, 25..=200).suffix("%"));
                                self.gradient_scaler.fixed_scale = scale_pct as f32 / 100.0;
                            });
                            ui.label(format!(
                                "A 10% gradient will feel like {}%",
                                (10.0 * self.gradient_scaler.fixed_scale) as i32
                            ));
                        }
                        GradientScalingMode::Adaptive => {
                            ui.horizontal(|ui| {
                                ui.label("Your FTP:");
                                let mut ftp = self.gradient_scaler.user_ftp as i32;
                                ui.add(egui::Slider::new(&mut ftp, 100..=500).suffix("W"));
                                self.gradient_scaler.user_ftp = ftp as u16;
                            });
                            ui.horizontal(|ui| {
                                ui.label("Target FTP:");
                                let mut target = self.gradient_scaler.target_ftp as i32;
                                ui.add(egui::Slider::new(&mut target, 100..=500).suffix("W"));
                                self.gradient_scaler.target_ftp = target as u16;
                            });
                            let scale = self.gradient_scaler.effective_scale();
                            ui.label(format!(
                                "Routes scaled to {:.0}% (feel like {}W rider)",
                                scale * 100.0,
                                self.gradient_scaler.target_ftp
                            ));
                        }
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Reset to Original").clicked() {
                            self.gradient_scaler = GradientScaler::default();
                        }
                        if ui.button("50% Easier").clicked() {
                            self.gradient_scaler = GradientScaler::half_gradient();
                        }
                        if ui.button("2x Harder").clicked() {
                            self.gradient_scaler = GradientScaler::double_gradient();
                        }
                    });
                });
                ui.add_space(8.0);
            }

            // T105: Recommendations panel (collapsible)
            if self.show_recommendations {
                if let Some(rec_action) = self.render_recommendations_panel(ui) {
                    action = Some(rec_action);
                }
                ui.add_space(8.0);
            }

            ui.separator();

            // Two-column layout: route list | preview
            ui.columns(2, |columns| {
                // Left column: Route list
                columns[0].vertical(|ui| {
                    ui.heading("Routes");
                    ui.add_space(8.0);

                    // Clone filtered routes to avoid borrow issues
                    let filtered: Vec<StoredRoute> =
                        self.filter_routes().into_iter().cloned().collect();
                    let is_routes_empty = self.routes.is_empty();

                    if filtered.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            if is_routes_empty {
                                ui.label(RichText::new("No routes imported yet").weak());
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("Click 'Import Route' to add routes").weak(),
                                );
                            } else {
                                ui.label(RichText::new("No routes match your search").weak());
                            }
                        });
                    } else {
                        let mut new_selection = None;
                        ScrollArea::vertical()
                            .max_height(ui.available_height() - 40.0)
                            .show(ui, |ui| {
                                for (i, route) in filtered.iter().enumerate() {
                                    let is_selected = self.selected_index == Some(i);
                                    if self.render_route_item(ui, route, is_selected) {
                                        new_selection = Some(i);
                                    }
                                    ui.add_space(4.0);
                                }
                            });
                        if let Some(idx) = new_selection {
                            self.selected_index = Some(idx);
                        }
                    }
                });

                // Right column: Route details
                columns[1].vertical(|ui| {
                    ui.heading("Details");
                    ui.add_space(8.0);

                    // Clone the selected route to avoid borrow issues
                    let selected_route: Option<StoredRoute> = {
                        let filtered = self.filter_routes();
                        self.selected_index
                            .and_then(|idx| filtered.get(idx).map(|r| (*r).clone()))
                    };

                    if let Some(route) = selected_route {
                        if let Some(act) = self.render_route_details(ui, &route) {
                            action = Some(act);
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(RichText::new("Select a route to view details").weak());
                        });
                    }
                });
            });
        });

        // Delete confirmation dialog
        if self.show_delete_confirm {
            if let Some(act) = self.render_delete_dialog(ui) {
                action = Some(act);
            }
        }

        action
    }

    /// Render a single route item in the list.
    fn render_route_item(&self, ui: &mut Ui, route: &StoredRoute, is_selected: bool) -> bool {
        let mut clicked = false;

        let fill_color = if is_selected {
            Color32::from_rgb(66, 133, 244).linear_multiply(0.3)
        } else {
            ui.visuals().faint_bg_color
        };

        let frame = egui::Frame::new()
            .fill(fill_color)
            .inner_margin(12.0)
            .corner_radius(8.0);

        let response = frame
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 8.0);

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        // Route name
                        ui.label(RichText::new(&route.name).size(14.0).strong());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Source badge
                            ui.label(
                                RichText::new(format_route_source(route.source))
                                    .size(10.0)
                                    .color(source_color(route.source)),
                            );
                        });
                    });

                    ui.add_space(4.0);

                    // Stats row
                    ui.horizontal(|ui| {
                        // Distance
                        let (dist, unit) = match self.units {
                            Units::Metric => (route.distance_meters / 1000.0, "km"),
                            Units::Imperial => (route.distance_meters / 1000.0 * 0.621371, "mi"),
                        };
                        ui.label(RichText::new(format!("{:.1} {}", dist, unit)).weak());

                        ui.separator();

                        // Elevation gain
                        let (elev, elev_unit) = match self.units {
                            Units::Metric => (route.elevation_gain_meters, "m"),
                            Units::Imperial => (route.elevation_gain_meters * 3.28084, "ft"),
                        };
                        ui.label(RichText::new(format!("{:.0} {} gain", elev, elev_unit)).weak());

                        ui.separator();

                        // Max gradient
                        ui.label(
                            RichText::new(format!("{:.1}% max", route.max_gradient_percent)).weak(),
                        );
                    });
                });
            })
            .response;

        if response.interact(egui::Sense::click()).clicked() {
            clicked = true;
        }

        clicked
    }

    /// Render route details panel.
    fn render_route_details(
        &mut self,
        ui: &mut Ui,
        route: &StoredRoute,
    ) -> Option<RouteBrowserAction> {
        let mut action = None;

        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            // Route name
            ui.label(RichText::new(&route.name).size(18.0).strong());

            // Description
            if let Some(ref desc) = route.description {
                ui.add_space(4.0);
                ui.label(RichText::new(desc).weak());
            }

            ui.add_space(8.0);

            // Source and date
            ui.horizontal(|ui| {
                ui.label("Source:");
                ui.label(
                    RichText::new(format_route_source(route.source))
                        .color(source_color(route.source)),
                );
            });

            if let Some(ref file) = route.source_file {
                ui.horizontal(|ui| {
                    ui.label("File:");
                    ui.label(RichText::new(file).monospace().weak());
                });
            }

            ui.horizontal(|ui| {
                ui.label("Imported:");
                ui.label(
                    RichText::new(route.created_at.format("%Y-%m-%d %H:%M").to_string()).weak(),
                );
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Statistics
            ui.label(RichText::new("Statistics").strong());
            ui.add_space(4.0);

            // Distance
            let (dist, dist_unit) = match self.units {
                Units::Metric => (route.distance_meters / 1000.0, "km"),
                Units::Imperial => (route.distance_meters / 1000.0 * 0.621371, "mi"),
            };
            ui.horizontal(|ui| {
                ui.label("Distance:");
                ui.label(RichText::new(format!("{:.2} {}", dist, dist_unit)).strong());
            });

            // Elevation gain
            let (elev, elev_unit) = match self.units {
                Units::Metric => (route.elevation_gain_meters, "m"),
                Units::Imperial => (route.elevation_gain_meters * 3.28084, "ft"),
            };
            ui.horizontal(|ui| {
                ui.label("Elevation Gain:");
                ui.label(RichText::new(format!("{:.0} {}", elev, elev_unit)).strong());
            });

            // Elevation range
            let (min_elev, max_elev, elev_unit) = match self.units {
                Units::Metric => (route.min_elevation_meters, route.max_elevation_meters, "m"),
                Units::Imperial => (
                    route.min_elevation_meters * 3.28084,
                    route.max_elevation_meters * 3.28084,
                    "ft",
                ),
            };
            ui.horizontal(|ui| {
                ui.label("Elevation Range:");
                ui.label(
                    RichText::new(format!("{:.0} - {:.0} {}", min_elev, max_elev, elev_unit))
                        .strong(),
                );
            });

            // Gradient
            ui.horizontal(|ui| {
                ui.label("Avg Gradient:");
                ui.label(RichText::new(format!("{:.1}%", route.avg_gradient_percent)).strong());
            });
            ui.horizontal(|ui| {
                ui.label("Max Gradient:");
                ui.label(
                    RichText::new(format!("{:.1}%", route.max_gradient_percent))
                        .strong()
                        .color(gradient_color(route.max_gradient_percent)),
                );
            });

            ui.add_space(16.0);

            // Elevation profile placeholder
            ui.label(RichText::new("Elevation Profile").strong());
            self.render_elevation_profile_placeholder(ui, route);

            ui.add_space(16.0);

            // Action buttons
            ui.horizontal(|ui| {
                // Start ride button
                if ui
                    .add_sized(
                        Vec2::new(120.0, 40.0),
                        egui::Button::new(RichText::new("Start Ride").size(14.0))
                            .fill(Color32::from_rgb(52, 168, 83)),
                    )
                    .clicked()
                {
                    action = Some(RouteBrowserAction::StartRide(
                        route.clone(),
                        self.gradient_scaler.clone(),
                    ));
                }

                ui.add_space(8.0);

                // Delete button
                if ui
                    .add_sized(
                        Vec2::new(80.0, 40.0),
                        egui::Button::new(RichText::new("Delete").size(14.0))
                            .fill(Color32::from_rgb(234, 67, 53)),
                    )
                    .clicked()
                {
                    self.pending_delete = Some(route.id);
                    self.show_delete_confirm = true;
                }
            });
        });

        action
    }

    /// Render a placeholder elevation profile visualization.
    fn render_elevation_profile_placeholder(&self, ui: &mut Ui, route: &StoredRoute) {
        let available_width = ui.available_width() - 16.0;
        let height = 60.0;

        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available_width, height), egui::Sense::hover());

        // Draw background
        ui.painter().rect_filled(rect, 4.0, Color32::from_gray(40));

        // Draw a simple profile based on elevation data
        let num_segments = 30;
        let segment_width = available_width / num_segments as f32;

        let range = route.max_elevation_meters - route.min_elevation_meters;
        let _range = if range < 10.0 { 10.0 } else { range };

        for i in 0..num_segments {
            // Simulate a profile based on average gradient
            let t = i as f32 / num_segments as f32;
            let phase = t * std::f32::consts::PI;
            let h = (phase.sin() * route.avg_gradient_percent.abs() / 20.0).clamp(0.1, 0.9);
            let bar_height = h * height * 0.8;

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.min.x + i as f32 * segment_width + 1.0,
                    rect.max.y - bar_height,
                ),
                Vec2::new(segment_width - 2.0, bar_height),
            );

            let color = gradient_color(route.avg_gradient_percent * h * 2.0);
            ui.painter().rect_filled(bar_rect, 2.0, color);
        }
    }

    /// Render delete confirmation dialog.
    fn render_delete_dialog(&mut self, ui: &mut Ui) -> Option<RouteBrowserAction> {
        let mut action = None;

        egui::Window::new("Delete Route")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::new(300.0, 120.0));

                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label("Are you sure you want to delete this route?");
                    ui.label(RichText::new("This action cannot be undone.").weak());
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_delete_confirm = false;
                            self.pending_delete = None;
                        }

                        ui.add_space(16.0);

                        if ui
                            .add(egui::Button::new("Delete").fill(Color32::from_rgb(234, 67, 53)))
                            .clicked()
                        {
                            if let Some(id) = self.pending_delete {
                                action = Some(RouteBrowserAction::DeleteRoute(id));
                            }
                            self.show_delete_confirm = false;
                            self.pending_delete = None;
                            self.selected_index = None;
                        }
                    });
                });
            });

        // Close on Escape
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_delete_confirm = false;
            self.pending_delete = None;
        }

        action
    }

    /// T105: Render the recommendations panel.
    fn render_recommendations_panel(&mut self, ui: &mut Ui) -> Option<RouteBrowserAction> {
        let mut action = None;
        let mut needs_update = false;

        ui.group(|ui| {
            ui.heading("Route Recommendations");
            ui.add_space(4.0);

            // Criteria controls
            ui.horizontal(|ui| {
                ui.label("Goal:");
                let goal_text = match self.recommendation_criteria.goal {
                    TrainingGoalType::Endurance => "Endurance",
                    TrainingGoalType::Climbing => "Climbing",
                    TrainingGoalType::Speed => "Speed",
                    TrainingGoalType::Recovery => "Recovery",
                    TrainingGoalType::Intervals => "Intervals",
                };
                egui::ComboBox::from_id_salt("rec_goal")
                    .selected_text(goal_text)
                    .show_ui(ui, |ui| {
                        for (goal, label) in [
                            (TrainingGoalType::Endurance, "Endurance"),
                            (TrainingGoalType::Climbing, "Climbing"),
                            (TrainingGoalType::Speed, "Speed"),
                            (TrainingGoalType::Recovery, "Recovery"),
                            (TrainingGoalType::Intervals, "Intervals"),
                        ] {
                            if ui
                                .selectable_label(self.recommendation_criteria.goal == goal, label)
                                .clicked()
                            {
                                self.recommendation_criteria.goal = goal;
                                needs_update = true;
                            }
                        }
                    });

                ui.separator();

                ui.label("Time:");
                let mut time = self.recommendation_criteria.available_time_minutes as i32;
                if ui
                    .add(egui::Slider::new(&mut time, 15..=300).suffix(" min"))
                    .changed()
                {
                    self.recommendation_criteria.available_time_minutes = time as u32;
                    needs_update = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Your FTP:");
                let mut ftp = self.recommendation_criteria.user_ftp as i32;
                if ui
                    .add(egui::Slider::new(&mut ftp, 100..=500).suffix(" W"))
                    .changed()
                {
                    self.recommendation_criteria.user_ftp = ftp as u16;
                    needs_update = true;
                }

                ui.separator();

                ui.label("Avg Speed:");
                let mut speed = self.recommendation_criteria.avg_speed_kmh;
                let (speed_label, speed_range) = match self.units {
                    Units::Metric => ("km/h", 15.0..=45.0),
                    Units::Imperial => ("mph", 10.0..=30.0),
                };
                if ui
                    .add(egui::Slider::new(&mut speed, speed_range).suffix(speed_label))
                    .changed()
                {
                    self.recommendation_criteria.avg_speed_kmh = speed;
                    needs_update = true;
                }
            });

            ui.horizontal(|ui| {
                let mut variety = self.recommendation_criteria.prefer_variety;
                if ui.checkbox(&mut variety, "Prefer fresh routes").changed() {
                    self.recommendation_criteria.prefer_variety = variety;
                    needs_update = true;
                }

                ui.separator();

                ui.label("Max gradient:");
                let mut max_grad = self.recommendation_criteria.max_comfortable_gradient;
                if ui
                    .add(egui::Slider::new(&mut max_grad, 5.0..=25.0).suffix("%"))
                    .changed()
                {
                    self.recommendation_criteria.max_comfortable_gradient = max_grad;
                    needs_update = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Recommendations list
            if self.cached_recommendations.is_empty() {
                ui.label(
                    RichText::new("No matching routes found. Try adjusting your criteria.").weak(),
                );
            } else {
                ui.label(
                    RichText::new(format!(
                        "Found {} recommendations:",
                        self.cached_recommendations.len()
                    ))
                    .strong(),
                );

                ui.add_space(4.0);

                // Clone to avoid borrow issues
                let recommendations: Vec<_> = self.cached_recommendations.to_vec();

                ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for rec in recommendations.iter().take(5) {
                            let card_response = self.render_recommendation_card(ui, rec);
                            if card_response {
                                action = Some(RouteBrowserAction::StartRide(
                                    rec.route.clone(),
                                    self.gradient_scaler.clone(),
                                ));
                            }
                        }
                    });
                });
            }
        });

        if needs_update {
            self.update_recommendations();
        }

        action
    }

    /// T105: Render a single recommendation card.
    fn render_recommendation_card(&self, ui: &mut Ui, rec: &RouteRecommendation) -> bool {
        let mut clicked = false;

        let fill = if rec.recently_ridden {
            Color32::from_gray(50)
        } else {
            Color32::from_rgb(40, 60, 80)
        };

        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(12.0)
            .corner_radius(8.0);

        let response = frame
            .show(ui, |ui| {
                ui.set_min_width(180.0);
                ui.set_max_width(200.0);

                ui.vertical(|ui| {
                    // Score badge
                    let score_color = if rec.score > 0.6 {
                        Color32::from_rgb(76, 175, 80)
                    } else if rec.score > 0.4 {
                        Color32::from_rgb(255, 193, 7)
                    } else {
                        Color32::from_rgb(158, 158, 158)
                    };
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:.0}%", rec.score * 100.0))
                                .size(12.0)
                                .color(score_color),
                        );
                        if rec.recently_ridden {
                            ui.label(RichText::new("(ridden)").size(10.0).weak());
                        }
                    });

                    // Route name
                    ui.label(RichText::new(&rec.route.name).size(13.0).strong());

                    ui.add_space(4.0);

                    // Duration
                    let hours = rec.estimated_duration_minutes / 60;
                    let mins = rec.estimated_duration_minutes % 60;
                    let duration_str = if hours > 0 {
                        format!("~{}h {}m", hours, mins)
                    } else {
                        format!("~{} min", mins)
                    };
                    ui.label(RichText::new(duration_str).size(11.0).weak());

                    // Distance
                    let (dist, unit) = match self.units {
                        Units::Metric => (rec.route.distance_meters / 1000.0, "km"),
                        Units::Imperial => (rec.route.distance_meters / 1000.0 * 0.621371, "mi"),
                    };
                    ui.label(
                        RichText::new(format!("{:.1} {}", dist, unit))
                            .size(11.0)
                            .weak(),
                    );

                    ui.add_space(4.0);

                    // Reasons (first 2)
                    for reason in rec.reasons.iter().take(2) {
                        ui.label(
                            RichText::new(format!("• {}", reason))
                                .size(10.0)
                                .color(Color32::from_rgb(144, 202, 249)),
                        );
                    }

                    ui.add_space(8.0);

                    // Start button
                    if ui.button("Start Ride").clicked() {
                        clicked = true;
                    }
                });
            })
            .response;

        if response.interact(egui::Sense::click()).clicked() {
            // Card click could select the route, but button handles start
        }

        clicked
    }
}

/// Get color for route source.
fn source_color(source: RouteSource) -> Color32 {
    match source {
        RouteSource::Gpx => Color32::from_rgb(76, 175, 80),
        RouteSource::Fit => Color32::from_rgb(66, 133, 244),
        RouteSource::Tcx => Color32::from_rgb(255, 152, 0),
        RouteSource::Custom => Color32::from_rgb(156, 39, 176),
        RouteSource::Famous => Color32::from_rgb(233, 30, 99),
        RouteSource::Procedural => Color32::from_rgb(0, 188, 212),
    }
}

/// Get color for gradient percentage.
fn gradient_color(gradient: f32) -> Color32 {
    let abs_gradient = gradient.abs();
    if abs_gradient < 3.0 {
        Color32::from_rgb(76, 175, 80) // Green - easy
    } else if abs_gradient < 6.0 {
        Color32::from_rgb(255, 193, 7) // Yellow - moderate
    } else if abs_gradient < 10.0 {
        Color32::from_rgb(255, 152, 0) // Orange - challenging
    } else {
        Color32::from_rgb(244, 67, 54) // Red - extreme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_browser_screen_new() {
        let screen = RouteBrowserScreen::new();
        assert!(screen.routes.is_empty());
        assert!(screen.selected_index.is_none());
        assert!(screen.search_query.is_empty());
    }

    #[test]
    fn test_set_routes() {
        let mut screen = RouteBrowserScreen::new();
        let routes = vec![
            StoredRoute::new("Route A".to_string(), RouteSource::Gpx),
            StoredRoute::new("Route B".to_string(), RouteSource::Fit),
        ];

        screen.set_routes(routes);

        assert_eq!(screen.routes.len(), 2);
    }

    #[test]
    fn test_filter_routes() {
        let mut screen = RouteBrowserScreen::new();
        let routes = vec![
            StoredRoute::new("Mountain Route".to_string(), RouteSource::Gpx),
            StoredRoute::new("Coastal Route".to_string(), RouteSource::Fit),
        ];

        screen.set_routes(routes);

        // No filter
        let filtered = screen.filter_routes();
        assert_eq!(filtered.len(), 2);

        // Search filter
        screen.search_query = "mountain".to_string();
        let filtered = screen.filter_routes();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Mountain Route");

        // Source filter
        screen.search_query.clear();
        screen.source_filter = Some(RouteSource::Fit);
        let filtered = screen.filter_routes();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Coastal Route");
    }

    #[test]
    fn test_sort_routes() {
        let mut screen = RouteBrowserScreen::new();

        let mut route_a = StoredRoute::new("Alpha".to_string(), RouteSource::Gpx);
        route_a.distance_meters = 5000.0;

        let mut route_b = StoredRoute::new("Beta".to_string(), RouteSource::Gpx);
        route_b.distance_meters = 10000.0;

        screen.set_routes(vec![route_b, route_a]);

        // Default sort is by name ascending
        assert_eq!(screen.routes[0].name, "Alpha");
        assert_eq!(screen.routes[1].name, "Beta");

        // Sort by distance descending
        screen.sort_order = RouteSortOrder::DistanceDesc;
        screen.sort_routes();
        assert_eq!(screen.routes[0].name, "Beta");
        assert_eq!(screen.routes[1].name, "Alpha");
    }

    #[test]
    fn test_gradient_color() {
        let green = gradient_color(2.0);
        let yellow = gradient_color(4.0);
        let orange = gradient_color(8.0);
        let red = gradient_color(12.0);

        // They should all be different colors
        assert_ne!(green, yellow);
        assert_ne!(yellow, orange);
        assert_ne!(orange, red);
    }

    #[test]
    fn test_source_color() {
        let gpx = source_color(RouteSource::Gpx);
        let fit = source_color(RouteSource::Fit);
        assert_ne!(gpx, fit);
    }
}
