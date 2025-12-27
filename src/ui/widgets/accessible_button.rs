//! Accessible button widget with 44x44 minimum touch target.
//!
//! Provides WCAG 2.1 compliant touch targets with visual feedback.

use egui::{Color32, Response, Sense, StrokeKind, Ui, Vec2, Widget};

/// Minimum touch target size per WCAG 2.1 guidelines (44x44 CSS pixels).
pub const MIN_TOUCH_TARGET: f32 = 44.0;

/// Configuration for accessible button appearance.
#[derive(Debug, Clone)]
pub struct AccessibleButtonStyle {
    /// Minimum size for touch targets
    pub min_size: Vec2,
    /// Normal background color
    pub bg_color: Color32,
    /// Hovered background color
    pub bg_hover: Color32,
    /// Pressed background color
    pub bg_pressed: Color32,
    /// Text color
    pub text_color: Color32,
    /// Border radius
    pub rounding: f32,
    /// Whether to show touch feedback ripple
    pub show_ripple: bool,
}

impl Default for AccessibleButtonStyle {
    fn default() -> Self {
        Self {
            min_size: Vec2::splat(MIN_TOUCH_TARGET),
            bg_color: Color32::from_rgb(60, 60, 60),
            bg_hover: Color32::from_rgb(80, 80, 80),
            bg_pressed: Color32::from_rgb(40, 40, 40),
            text_color: Color32::WHITE,
            rounding: 4.0,
            show_ripple: true,
        }
    }
}

impl AccessibleButtonStyle {
    /// Create a primary action button style.
    pub fn primary() -> Self {
        Self {
            bg_color: Color32::from_rgb(66, 133, 244),
            bg_hover: Color32::from_rgb(86, 153, 255),
            bg_pressed: Color32::from_rgb(46, 113, 224),
            ..Default::default()
        }
    }

    /// Create a secondary/subtle button style.
    pub fn secondary() -> Self {
        Self {
            bg_color: Color32::from_rgba_unmultiplied(100, 100, 100, 100),
            bg_hover: Color32::from_rgba_unmultiplied(120, 120, 120, 150),
            bg_pressed: Color32::from_rgba_unmultiplied(80, 80, 80, 100),
            ..Default::default()
        }
    }

    /// Create a danger/destructive action button style.
    pub fn danger() -> Self {
        Self {
            bg_color: Color32::from_rgb(220, 53, 69),
            bg_hover: Color32::from_rgb(240, 73, 89),
            bg_pressed: Color32::from_rgb(200, 33, 49),
            ..Default::default()
        }
    }
}

/// An accessible button with minimum 44x44 touch target.
pub struct AccessibleButton<'a> {
    /// Button text
    text: &'a str,
    /// Accessible label for screen readers (if different from text)
    accessible_label: Option<&'a str>,
    /// Button style
    style: AccessibleButtonStyle,
    /// Whether the button is enabled
    enabled: bool,
    /// Optional icon (rendered before text)
    icon: Option<&'a str>,
}

impl<'a> AccessibleButton<'a> {
    /// Create a new accessible button with the given text.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            accessible_label: None,
            style: AccessibleButtonStyle::default(),
            enabled: true,
            icon: None,
        }
    }

    /// Set the accessible label for screen readers.
    pub fn accessible_label(mut self, label: &'a str) -> Self {
        self.accessible_label = Some(label);
        self
    }

    /// Set the button style.
    pub fn style(mut self, style: AccessibleButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Use primary button style.
    pub fn primary(mut self) -> Self {
        self.style = AccessibleButtonStyle::primary();
        self
    }

    /// Use secondary button style.
    pub fn secondary(mut self) -> Self {
        self.style = AccessibleButtonStyle::secondary();
        self
    }

    /// Use danger button style.
    pub fn danger(mut self) -> Self {
        self.style = AccessibleButtonStyle::danger();
        self
    }

    /// Set whether the button is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set an icon to display before the text.
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the minimum size.
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.style.min_size = size;
        self
    }
}

impl Widget for AccessibleButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            text,
            accessible_label,
            style,
            enabled,
            icon,
        } = self;

        // Calculate size ensuring minimum touch target
        let text_galley = ui.painter().layout_no_wrap(
            text.to_string(),
            egui::FontId::default(),
            Color32::WHITE,
        );
        let text_size = text_galley.size();

        let icon_width = if icon.is_some() { 24.0 } else { 0.0 };
        let padding = Vec2::new(16.0, 8.0);
        let content_size = Vec2::new(
            text_size.x + icon_width + padding.x * 2.0,
            text_size.y + padding.y * 2.0,
        );

        // Ensure minimum touch target size
        let size = Vec2::new(
            content_size.x.max(style.min_size.x),
            content_size.y.max(style.min_size.y),
        );

        let (rect, response) = ui.allocate_exact_size(size, Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = if !enabled {
                ui.visuals().widgets.inactive
            } else if response.is_pointer_button_down_on() {
                ui.visuals().widgets.active
            } else if response.hovered() {
                ui.visuals().widgets.hovered
            } else {
                ui.visuals().widgets.inactive
            };

            // Determine background color
            let bg_color = if !enabled {
                style.bg_color.gamma_multiply(0.5)
            } else if response.is_pointer_button_down_on() {
                style.bg_pressed
            } else if response.hovered() {
                style.bg_hover
            } else {
                style.bg_color
            };

            // Draw button background
            ui.painter().rect_filled(rect, style.rounding, bg_color);

            // Draw focus indicator if keyboard focused
            if response.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    style.rounding,
                    egui::Stroke::new(2.0, Color32::from_rgb(66, 133, 244)),
                    StrokeKind::Middle,
                );
            }

            // Draw text centered
            let text_color = if enabled {
                style.text_color
            } else {
                style.text_color.gamma_multiply(0.5)
            };

            let text_pos = rect.center() - text_size / 2.0;

            if let Some(icon_str) = icon {
                // Draw icon + text
                let icon_pos = egui::pos2(
                    rect.center().x - (text_size.x + icon_width) / 2.0,
                    rect.center().y - text_size.y / 2.0,
                );
                ui.painter().text(
                    icon_pos,
                    egui::Align2::LEFT_TOP,
                    icon_str,
                    egui::FontId::default(),
                    text_color,
                );
                ui.painter().text(
                    egui::pos2(icon_pos.x + icon_width, icon_pos.y),
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::default(),
                    text_color,
                );
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::default(),
                    text_color,
                );
            }

            // Touch feedback ripple effect
            if style.show_ripple && response.is_pointer_button_down_on() {
                let ripple_color = Color32::from_rgba_unmultiplied(255, 255, 255, 30);
                ui.painter().circle_filled(
                    response.interact_pointer_pos().unwrap_or(rect.center()),
                    rect.width().min(rect.height()) / 2.0,
                    ripple_color,
                );
            }
        }

        // Set accessible name for screen readers
        let label = accessible_label.unwrap_or(text);
        response.on_hover_text(label)
    }
}

/// Helper function to create an accessible button.
pub fn accessible_button(text: &str) -> AccessibleButton<'_> {
    AccessibleButton::new(text)
}

/// Accessible icon button with minimum touch target.
pub struct AccessibleIconButton<'a> {
    /// Icon character or emoji
    icon: &'a str,
    /// Accessible label for screen readers
    accessible_label: &'a str,
    /// Button style
    style: AccessibleButtonStyle,
    /// Whether the button is enabled
    enabled: bool,
    /// Size of the icon
    icon_size: f32,
}

impl<'a> AccessibleIconButton<'a> {
    /// Create a new accessible icon button.
    pub fn new(icon: &'a str, accessible_label: &'a str) -> Self {
        Self {
            icon,
            accessible_label,
            style: AccessibleButtonStyle::default(),
            enabled: true,
            icon_size: 24.0,
        }
    }

    /// Set the button style.
    pub fn style(mut self, style: AccessibleButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Set whether the button is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the icon size.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }
}

impl Widget for AccessibleIconButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            icon,
            accessible_label,
            style,
            enabled,
            icon_size,
        } = self;

        // Ensure minimum touch target size
        let size = Vec2::splat(style.min_size.x.max(icon_size + 16.0));

        let (rect, response) = ui.allocate_exact_size(size, Sense::click());

        if ui.is_rect_visible(rect) {
            // Determine background color
            let bg_color = if !enabled {
                style.bg_color.gamma_multiply(0.5)
            } else if response.is_pointer_button_down_on() {
                style.bg_pressed
            } else if response.hovered() {
                style.bg_hover
            } else {
                style.bg_color
            };

            // Draw circular button background
            ui.painter().circle_filled(rect.center(), size.x / 2.0, bg_color);

            // Draw focus indicator if keyboard focused
            if response.has_focus() {
                ui.painter().circle_stroke(
                    rect.center(),
                    size.x / 2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(66, 133, 244)),
                );
            }

            // Draw icon centered
            let text_color = if enabled {
                style.text_color
            } else {
                style.text_color.gamma_multiply(0.5)
            };

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(icon_size),
                text_color,
            );
        }

        response.on_hover_text(accessible_label)
    }
}

/// Helper function to create an accessible icon button.
pub fn accessible_icon_button<'a>(icon: &'a str, label: &'a str) -> AccessibleIconButton<'a> {
    AccessibleIconButton::new(icon, label)
}
