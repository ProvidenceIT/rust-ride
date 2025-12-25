//! Avatar customization screen.
//!
//! T128: Create avatar customization screen
//! T129: Add jersey color picker (10+ color options)
//! T130: Add bike style selector with previews
//! T131: Show live avatar preview as customization changes
//! T132: Wire Save button to persist avatar config

use egui::{Color32, RichText, Ui, Vec2};

use crate::world::avatar::{AvatarConfig, BikeStyle};

use super::Screen;

/// Predefined jersey colors for the color picker.
const JERSEY_COLORS: &[([u8; 3], &str)] = &[
    ([255, 0, 0], "Red"),
    ([0, 128, 0], "Green"),
    ([0, 0, 255], "Blue"),
    ([255, 255, 0], "Yellow"),
    ([255, 165, 0], "Orange"),
    ([128, 0, 128], "Purple"),
    ([255, 192, 203], "Pink"),
    ([0, 255, 255], "Cyan"),
    ([255, 255, 255], "White"),
    ([0, 0, 0], "Black"),
    ([128, 128, 128], "Gray"),
    ([139, 69, 19], "Brown"),
];

/// Avatar customization screen state.
#[derive(Clone)]
pub struct AvatarScreen {
    /// Current avatar configuration being edited
    config: AvatarConfig,
    /// Whether changes have been made
    has_changes: bool,
    /// Status message to display
    status_message: Option<(String, bool)>, // (message, is_error)
}

impl Default for AvatarScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarScreen {
    /// Create a new avatar customization screen.
    pub fn new() -> Self {
        Self {
            config: AvatarConfig::default(),
            has_changes: false,
            status_message: None,
        }
    }

    /// Load an existing avatar configuration.
    pub fn load_config(&mut self, config: AvatarConfig) {
        self.config = config;
        self.has_changes = false;
        self.status_message = None;
    }

    /// Get the current configuration.
    pub fn get_config(&self) -> &AvatarConfig {
        &self.config
    }

    /// Check if there are unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_changes
    }

    /// Render the avatar customization screen.
    pub fn show(&mut self, ui: &mut Ui) -> Option<(Screen, Option<AvatarConfig>)> {
        let mut result = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("< Back").clicked() {
                    result = Some((Screen::Home, None));
                }
                ui.heading("Customize Avatar");
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(16.0);

            // Two-column layout: preview | customization options
            ui.columns(2, |columns| {
                // Left column: Avatar preview
                columns[0].vertical(|ui| {
                    ui.heading("Preview");
                    ui.add_space(8.0);
                    self.render_avatar_preview(ui);
                });

                // Right column: Customization options
                columns[1].vertical(|ui| {
                    // Jersey color picker
                    ui.heading("Jersey Color");
                    ui.add_space(8.0);
                    if self.render_color_picker(ui) {
                        self.has_changes = true;
                    }

                    ui.add_space(16.0);

                    // Bike style selector
                    ui.heading("Bike Style");
                    ui.add_space(8.0);
                    if self.render_bike_selector(ui) {
                        self.has_changes = true;
                    }

                    ui.add_space(24.0);

                    // Save button
                    ui.horizontal(|ui| {
                        let save_button = ui.add_sized(
                            Vec2::new(120.0, 40.0),
                            egui::Button::new(RichText::new("Save").size(16.0))
                                .fill(Color32::from_rgb(52, 168, 83)),
                        );

                        if save_button.clicked() {
                            result = Some((Screen::Home, Some(self.config.clone())));
                            self.has_changes = false;
                            self.status_message = Some(("Avatar saved!".to_string(), false));
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_sized(
                                Vec2::new(120.0, 40.0),
                                egui::Button::new(RichText::new("Reset").size(16.0)),
                            )
                            .clicked()
                        {
                            self.config = AvatarConfig::default();
                            self.has_changes = true;
                        }
                    });

                    // Status message
                    if let Some((msg, is_error)) = &self.status_message {
                        ui.add_space(8.0);
                        let color = if *is_error {
                            Color32::from_rgb(244, 67, 54)
                        } else {
                            Color32::from_rgb(76, 175, 80)
                        };
                        ui.label(RichText::new(msg).color(color));
                    }
                });
            });
        });

        result
    }

    /// Render the avatar preview.
    fn render_avatar_preview(&self, ui: &mut Ui) {
        let available_width = ui.available_width().min(300.0);
        let height = 350.0;

        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available_width, height), egui::Sense::hover());

        // Draw background
        ui.painter().rect_filled(rect, 8.0, Color32::from_gray(30));

        // Draw simplified avatar representation
        let center_x = rect.center().x;
        let base_y = rect.max.y - 40.0;

        // Draw bike
        let bike_color = match self.config.bike_style {
            BikeStyle::RoadBike => Color32::from_rgb(60, 60, 60),
            BikeStyle::TimeTrial => Color32::from_rgb(30, 30, 30),
            BikeStyle::Gravel => Color32::from_rgb(139, 90, 43),
        };

        // Bike frame (simplified triangle)
        let wheel_radius = 25.0;
        let wheel_y = base_y - wheel_radius;

        // Wheels
        ui.painter().circle_stroke(
            egui::pos2(center_x - 40.0, wheel_y),
            wheel_radius,
            egui::Stroke::new(3.0, bike_color),
        );
        ui.painter().circle_stroke(
            egui::pos2(center_x + 40.0, wheel_y),
            wheel_radius,
            egui::Stroke::new(3.0, bike_color),
        );

        // Frame
        let frame_points = [
            egui::pos2(center_x - 40.0, wheel_y),        // rear wheel
            egui::pos2(center_x, wheel_y - 40.0),        // seat tube top
            egui::pos2(center_x + 40.0, wheel_y),        // front wheel
            egui::pos2(center_x, wheel_y - 40.0),        // back to top
            egui::pos2(center_x + 20.0, wheel_y - 50.0), // handlebars
        ];
        for i in 0..frame_points.len() - 1 {
            ui.painter().line_segment(
                [frame_points[i], frame_points[i + 1]],
                egui::Stroke::new(3.0, bike_color),
            );
        }

        // Draw rider body (oval)
        let body_center = egui::pos2(center_x, wheel_y - 80.0);
        let jersey_color = Color32::from_rgb(
            self.config.jersey_color[0],
            self.config.jersey_color[1],
            self.config.jersey_color[2],
        );
        ui.painter().rect_filled(
            egui::Rect::from_center_size(body_center, Vec2::new(30.0, 50.0)),
            6.0,
            jersey_color,
        );

        // Draw head (circle)
        let head_center = egui::pos2(center_x, wheel_y - 120.0);
        let helmet_color = self
            .config
            .helmet_color
            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
            .unwrap_or(Color32::from_gray(80));
        ui.painter().circle_filled(head_center, 15.0, helmet_color);

        // Draw arms (lines to handlebars)
        let arm_color = Color32::from_rgb(255, 220, 180);
        ui.painter().line_segment(
            [
                egui::pos2(center_x + 10.0, wheel_y - 70.0),
                egui::pos2(center_x + 20.0, wheel_y - 50.0),
            ],
            egui::Stroke::new(4.0, arm_color),
        );
        ui.painter().line_segment(
            [
                egui::pos2(center_x - 10.0, wheel_y - 70.0),
                egui::pos2(center_x + 15.0, wheel_y - 55.0),
            ],
            egui::Stroke::new(4.0, arm_color),
        );

        // Bike style label
        let style_text = match self.config.bike_style {
            BikeStyle::RoadBike => "Road Bike",
            BikeStyle::TimeTrial => "Time Trial",
            BikeStyle::Gravel => "Gravel",
        };
        ui.painter().text(
            egui::pos2(rect.center().x, rect.max.y - 10.0),
            egui::Align2::CENTER_CENTER,
            style_text,
            egui::FontId::proportional(12.0),
            Color32::GRAY,
        );
    }

    /// Render the jersey color picker. Returns true if color changed.
    fn render_color_picker(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        // Display colors in a grid
        ui.horizontal_wrapped(|ui| {
            for (color, name) in JERSEY_COLORS {
                let is_selected = self.config.jersey_color == *color;
                let button_color = Color32::from_rgb(color[0], color[1], color[2]);

                let size = Vec2::splat(40.0);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

                // Draw color swatch
                ui.painter().rect_filled(rect, 4.0, button_color);

                // Draw selection border
                if is_selected {
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(3.0, Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );
                }

                // Tooltip
                response.clone().on_hover_text(*name);

                if response.clicked() {
                    self.config.jersey_color = *color;
                    changed = true;
                }
            }
        });

        // Show current color name
        let current_name = JERSEY_COLORS
            .iter()
            .find(|(c, _)| *c == self.config.jersey_color)
            .map(|(_, n)| *n)
            .unwrap_or("Custom");

        ui.add_space(4.0);
        ui.label(RichText::new(format!("Selected: {}", current_name)).weak());

        changed
    }

    /// Render the bike style selector. Returns true if style changed.
    fn render_bike_selector(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let styles = [
            (
                BikeStyle::RoadBike,
                "Road Bike",
                "Lightweight and fast, ideal for paved roads",
            ),
            (
                BikeStyle::TimeTrial,
                "Time Trial",
                "Aerodynamic design for maximum speed",
            ),
            (
                BikeStyle::Gravel,
                "Gravel",
                "Versatile bike for mixed terrain",
            ),
        ];

        for (style, name, description) in styles {
            let is_selected = self.config.bike_style == style;

            let fill_color = if is_selected {
                Color32::from_rgb(66, 133, 244).linear_multiply(0.3)
            } else {
                ui.visuals().faint_bg_color
            };

            let frame = egui::Frame::new()
                .fill(fill_color)
                .inner_margin(12.0)
                .corner_radius(6.0);

            let response = frame
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width() - 8.0);
                    ui.horizontal(|ui| {
                        // Radio-style indicator
                        let indicator_color = if is_selected {
                            Color32::from_rgb(66, 133, 244)
                        } else {
                            Color32::GRAY
                        };
                        let (indicator_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        ui.painter().circle_stroke(
                            indicator_rect.center(),
                            7.0,
                            egui::Stroke::new(2.0, indicator_color),
                        );
                        if is_selected {
                            ui.painter().circle_filled(
                                indicator_rect.center(),
                                4.0,
                                indicator_color,
                            );
                        }

                        ui.add_space(8.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).strong());
                            ui.label(RichText::new(description).size(11.0).weak());
                        });
                    });
                })
                .response;

            if response.interact(egui::Sense::click()).clicked() {
                self.config.bike_style = style;
                changed = true;
            }

            ui.add_space(4.0);
        }

        changed
    }
}
