//! World selection screen for 3D virtual riding.
//!
//! T118: Create world selection screen
//! T119: Display world cards with preview images and descriptions
//! T120: Display route list when world is selected
//! T121: Show route distance and elevation profile
//! T122: Wire route selection to start 3D ride

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui, Vec2};

use crate::storage::config::Units;
use crate::world::worlds::{
    get_builtin_worlds, RouteDefinition, RouteDifficulty, TimeOfDay, WorldDefinition, WorldTheme,
};

use super::Screen;

/// World selection screen state.
#[derive(Default)]
pub struct WorldSelectScreen {
    /// Available worlds
    pub worlds: Vec<WorldDefinition>,
    /// Selected world index
    pub selected_world: Option<usize>,
    /// Selected route index within the world
    pub selected_route: Option<usize>,
    /// Unit preference
    pub units: Units,
}

/// Result from world selection.
#[derive(Debug, Clone)]
pub struct WorldRouteSelection {
    /// The selected world definition
    pub world: WorldDefinition,
    /// The selected route definition
    pub route: RouteDefinition,
}

impl WorldSelectScreen {
    /// Create a new world selection screen.
    pub fn new() -> Self {
        Self {
            worlds: get_builtin_worlds(),
            selected_world: None,
            selected_route: None,
            units: Units::Metric,
        }
    }

    /// Set the unit preference.
    pub fn set_units(&mut self, units: Units) {
        self.units = units;
    }

    /// Render the world selection screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<(Screen, Option<WorldRouteSelection>)> {
        let mut result = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("← Back").clicked() {
                    result = Some((Screen::Home, None));
                }
                ui.heading("Select World");
            });

            ui.add_space(8.0);
            ui.separator();

            // Two-column layout: world list | route details
            ui.columns(2, |columns| {
                // Left column: World list
                columns[0].vertical(|ui| {
                    ui.heading("Worlds");
                    ui.add_space(8.0);

                    ScrollArea::vertical()
                        .max_height(ui.available_height() - 60.0)
                        .show(ui, |ui| {
                            for (i, world) in self.worlds.iter().enumerate() {
                                let is_selected = self.selected_world == Some(i);
                                if self.render_world_card(ui, world, is_selected) {
                                    self.selected_world = Some(i);
                                    self.selected_route = Some(0); // Select default route
                                }
                                ui.add_space(4.0);
                            }
                        });
                });

                // Right column: Route selection and details
                columns[1].vertical(|ui| {
                    if let Some(world_idx) = self.selected_world {
                        if let Some(world) = self.worlds.get(world_idx).cloned() {
                            ui.heading(format!("{} - Routes", world.name));
                            ui.add_space(8.0);

                            // Route list
                            let mut new_route_selection = None;
                            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                for (i, route) in world.routes.iter().enumerate() {
                                    let is_selected = self.selected_route == Some(i);
                                    if self.render_route_item(ui, route, is_selected) {
                                        new_route_selection = Some(i);
                                    }
                                }
                            });
                            if let Some(idx) = new_route_selection {
                                self.selected_route = Some(idx);
                            }

                            ui.add_space(16.0);

                            // Selected route details
                            if let Some(route_idx) = self.selected_route {
                                if let Some(route) = world.routes.get(route_idx) {
                                    self.render_route_details(ui, route);

                                    ui.add_space(16.0);

                                    // Start ride button
                                    if ui
                                        .add_sized(
                                            Vec2::new(200.0, 50.0),
                                            egui::Button::new(
                                                RichText::new("Start 3D Ride").size(18.0),
                                            )
                                            .fill(Color32::from_rgb(52, 168, 83)),
                                        )
                                        .clicked()
                                    {
                                        let selection = WorldRouteSelection {
                                            world: world.clone(),
                                            route: route.clone(),
                                        };
                                        result = Some((Screen::Ride, Some(selection)));
                                    }
                                }
                            }
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new("Select a world to see available routes").weak(),
                            );
                        });
                    }
                });
            });
        });

        result
    }

    /// Render a world card.
    fn render_world_card(&self, ui: &mut Ui, world: &WorldDefinition, is_selected: bool) -> bool {
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
                    // World name and theme badge
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&world.name).size(16.0).strong());
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format_theme(world.theme))
                                .size(11.0)
                                .color(theme_color(world.theme)),
                        );
                    });

                    ui.add_space(4.0);

                    // Description
                    ui.label(RichText::new(&world.description).size(12.0).weak());

                    ui.add_space(4.0);

                    // Stats
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} routes", world.routes.len()))
                                .size(11.0)
                                .weak(),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format_time_of_day(world.time_of_day))
                                .size(11.0)
                                .weak(),
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

    /// Render a route item in the list.
    fn render_route_item(&self, ui: &mut Ui, route: &RouteDefinition, is_selected: bool) -> bool {
        let mut clicked = false;

        let fill_color = if is_selected {
            Color32::from_rgb(66, 133, 244).linear_multiply(0.2)
        } else {
            Color32::TRANSPARENT
        };

        let frame = egui::Frame::new()
            .fill(fill_color)
            .inner_margin(8.0)
            .corner_radius(4.0);

        let response = frame
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 8.0);

                ui.horizontal(|ui| {
                    // Route name
                    ui.label(RichText::new(&route.name).strong());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Difficulty badge
                        ui.label(
                            RichText::new(format_difficulty(route.difficulty))
                                .size(10.0)
                                .color(difficulty_color(route.difficulty)),
                        );

                        // Distance
                        let (dist, unit) = match self.units {
                            Units::Metric => (route.distance_meters / 1000.0, "km"),
                            Units::Imperial => (route.distance_meters / 1000.0 * 0.621371, "mi"),
                        };
                        ui.label(RichText::new(format!("{:.1} {}", dist, unit)).size(11.0));
                    });
                });
            })
            .response;

        if response.interact(egui::Sense::click()).clicked() {
            clicked = true;
        }

        clicked
    }

    /// Render detailed route information.
    fn render_route_details(&self, ui: &mut Ui, route: &RouteDefinition) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new(&route.name).size(18.0).strong());
            ui.add_space(8.0);

            // Distance
            let (dist, dist_unit) = match self.units {
                Units::Metric => (route.distance_meters / 1000.0, "km"),
                Units::Imperial => (route.distance_meters / 1000.0 * 0.621371, "mi"),
            };
            ui.horizontal(|ui| {
                ui.label("Distance:");
                ui.label(RichText::new(format!("{:.1} {}", dist, dist_unit)).strong());
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

            // Difficulty
            ui.horizontal(|ui| {
                ui.label("Difficulty:");
                ui.label(
                    RichText::new(format_difficulty(route.difficulty))
                        .strong()
                        .color(difficulty_color(route.difficulty)),
                );
            });

            // Route type
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.label(
                    RichText::new(if route.is_loop {
                        "Loop"
                    } else {
                        "Point-to-Point"
                    })
                    .strong(),
                );
            });

            ui.add_space(8.0);

            // Elevation profile visualization (simplified bar chart)
            ui.label(RichText::new("Elevation Profile").size(12.0).weak());
            self.render_elevation_profile(ui, route);
        });
    }

    /// Render a simplified elevation profile visualization.
    fn render_elevation_profile(&self, ui: &mut Ui, route: &RouteDefinition) {
        let available_width = ui.available_width() - 16.0;
        let height = 40.0;

        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available_width, height), egui::Sense::hover());

        // Draw background
        ui.painter().rect_filled(rect, 4.0, Color32::from_gray(40));

        // Draw a simplified profile based on difficulty
        let profile_color = difficulty_color(route.difficulty);
        let num_points = 20;
        let segment_width = available_width / num_points as f32;

        // Generate a simple profile shape based on difficulty
        let max_height = match route.difficulty {
            RouteDifficulty::Easy => 0.3,
            RouteDifficulty::Moderate => 0.5,
            RouteDifficulty::Challenging => 0.7,
            RouteDifficulty::Extreme => 0.9,
        };

        for i in 0..num_points {
            let t = i as f32 / num_points as f32;
            // Simple sine wave profile
            let h = (t * std::f32::consts::PI * 2.0).sin().abs() * max_height;
            let bar_height = h * height * 0.8;

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.min.x + i as f32 * segment_width + 1.0,
                    rect.max.y - bar_height,
                ),
                Vec2::new(segment_width - 2.0, bar_height),
            );

            ui.painter().rect_filled(bar_rect, 2.0, profile_color);
        }
    }
}

/// Format world theme for display.
fn format_theme(theme: WorldTheme) -> &'static str {
    match theme {
        WorldTheme::Countryside => "Countryside",
        WorldTheme::Mountains => "Mountains",
        WorldTheme::Coastal => "Coastal",
        WorldTheme::Urban => "Urban",
        WorldTheme::Desert => "Desert",
    }
}

/// Get color for world theme.
fn theme_color(theme: WorldTheme) -> Color32 {
    match theme {
        WorldTheme::Countryside => Color32::from_rgb(76, 175, 80),
        WorldTheme::Mountains => Color32::from_rgb(121, 134, 203),
        WorldTheme::Coastal => Color32::from_rgb(79, 195, 247),
        WorldTheme::Urban => Color32::from_rgb(158, 158, 158),
        WorldTheme::Desert => Color32::from_rgb(255, 183, 77),
    }
}

/// Format time of day for display.
fn format_time_of_day(time: TimeOfDay) -> &'static str {
    match time {
        TimeOfDay::Dawn => "Dawn",
        TimeOfDay::Morning => "Morning",
        TimeOfDay::Noon => "Noon",
        TimeOfDay::Afternoon => "Afternoon",
        TimeOfDay::Sunset => "Sunset",
        TimeOfDay::Night => "Night",
    }
}

/// Format difficulty for display.
fn format_difficulty(difficulty: RouteDifficulty) -> &'static str {
    match difficulty {
        RouteDifficulty::Easy => "Easy",
        RouteDifficulty::Moderate => "Moderate",
        RouteDifficulty::Challenging => "Challenging",
        RouteDifficulty::Extreme => "Extreme",
    }
}

/// Get color for difficulty level.
fn difficulty_color(difficulty: RouteDifficulty) -> Color32 {
    match difficulty {
        RouteDifficulty::Easy => Color32::from_rgb(76, 175, 80),
        RouteDifficulty::Moderate => Color32::from_rgb(255, 193, 7),
        RouteDifficulty::Challenging => Color32::from_rgb(255, 152, 0),
        RouteDifficulty::Extreme => Color32::from_rgb(244, 67, 54),
    }
}
