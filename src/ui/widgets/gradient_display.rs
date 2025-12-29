//! Gradient display widget showing current grade and route progress.
//!
//! T028: Create gradient display widget showing current grade.

use egui::{Color32, FontId, Pos2, Rect, RichText, Stroke, Ui, Vec2};

/// Configuration for the gradient display widget.
#[derive(Debug, Clone)]
pub struct GradientDisplayConfig {
    /// Whether to show the route progress bar
    pub show_progress: bool,
    /// Whether to show upcoming gradient preview
    pub show_preview: bool,
    /// Height of the widget in pixels
    pub height: f32,
}

impl Default for GradientDisplayConfig {
    fn default() -> Self {
        Self {
            show_progress: true,
            show_preview: true,
            height: 60.0,
        }
    }
}

/// Gradient display widget showing current grade and optional route progress.
pub struct GradientDisplay;

impl GradientDisplay {
    /// Get color for a gradient value.
    ///
    /// Green for flat/downhill, yellow for moderate, red for steep.
    fn gradient_color(gradient: f32) -> Color32 {
        let abs_grade = gradient.abs();

        if gradient < -2.0 {
            // Downhill - blue tones
            let t = (abs_grade / 15.0).min(1.0);
            Color32::from_rgb(
                (100.0 - 50.0 * t) as u8,
                (180.0 - 80.0 * t) as u8,
                (255.0) as u8,
            )
        } else if gradient < 2.0 {
            // Flat - green
            Color32::from_rgb(76, 175, 80)
        } else if gradient < 5.0 {
            // Moderate - yellow-green
            let t = (gradient - 2.0) / 3.0;
            Color32::from_rgb(
                (76.0 + 179.0 * t) as u8,
                (175.0 + 29.0 * t) as u8,
                (80.0 - 8.0 * t) as u8,
            )
        } else if gradient < 10.0 {
            // Steep - orange
            let t = (gradient - 5.0) / 5.0;
            Color32::from_rgb(
                255,
                (204.0 - 102.0 * t) as u8,
                (72.0 - 40.0 * t) as u8,
            )
        } else {
            // Very steep - red
            let t = ((gradient - 10.0) / 10.0).min(1.0);
            Color32::from_rgb(255, (102.0 - 60.0 * t) as u8, (32.0 - 32.0 * t) as u8)
        }
    }

    /// Render the main gradient display.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `current_gradient` - Current gradient in percent
    /// * `distance_m` - Current distance traveled in meters
    /// * `total_distance_m` - Total route distance in meters (None if no route)
    /// * `elevation_m` - Current elevation in meters
    /// * `config` - Display configuration
    pub fn show(
        ui: &mut Ui,
        current_gradient: f32,
        distance_m: Option<f64>,
        total_distance_m: Option<f64>,
        elevation_m: Option<f64>,
        config: &GradientDisplayConfig,
    ) {
        let available_width = ui.available_width();
        let height = config.height;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, height),
            egui::Sense::hover(),
        );

        let rect = response.rect;
        let bg_color = Color32::from_gray(30);
        painter.rect_filled(rect, 4.0, bg_color);

        // Gradient value and icon
        let gradient_color = Self::gradient_color(current_gradient);
        let gradient_text = format!("{:+.1}%", current_gradient);

        // Draw gradient icon (uphill/downhill arrow)
        let icon_center = Pos2::new(rect.min.x + 25.0, rect.center().y);
        Self::draw_gradient_icon(&painter, icon_center, current_gradient, gradient_color);

        // Draw gradient value
        painter.text(
            Pos2::new(rect.min.x + 50.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            gradient_text,
            FontId::proportional(24.0),
            gradient_color,
        );

        // Draw elevation if available
        if let Some(elev) = elevation_m {
            painter.text(
                Pos2::new(rect.max.x - 10.0, rect.center().y - 10.0),
                egui::Align2::RIGHT_CENTER,
                format!("{:.0}m", elev),
                FontId::proportional(14.0),
                Color32::from_gray(180),
            );
            painter.text(
                Pos2::new(rect.max.x - 10.0, rect.center().y + 8.0),
                egui::Align2::RIGHT_CENTER,
                "elevation",
                FontId::proportional(10.0),
                Color32::from_gray(120),
            );
        }

        // Draw progress bar if route is loaded
        if config.show_progress {
            if let (Some(dist), Some(total)) = (distance_m, total_distance_m) {
                let progress_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + 4.0, rect.max.y - 8.0),
                    Vec2::new(available_width - 8.0, 4.0),
                );

                // Background
                painter.rect_filled(progress_rect, 2.0, Color32::from_gray(50));

                // Progress fill
                let progress = (dist / total).clamp(0.0, 1.0) as f32;
                let progress_width = progress_rect.width() * progress;
                let fill_rect = Rect::from_min_size(
                    progress_rect.min,
                    Vec2::new(progress_width, progress_rect.height()),
                );
                painter.rect_filled(fill_rect, 2.0, gradient_color);
            }
        }
    }

    /// Draw a gradient indicator icon (arrow showing slope direction).
    fn draw_gradient_icon(painter: &egui::Painter, center: Pos2, gradient: f32, color: Color32) {
        let size = 16.0;
        let angle = (gradient / 100.0).atan(); // Convert percent to radians

        // Draw a line representing the slope
        let half_len = size / 2.0;
        let dx = half_len * angle.cos();
        let dy = half_len * angle.sin();

        let start = Pos2::new(center.x - dx, center.y + dy);
        let end = Pos2::new(center.x + dx, center.y - dy);

        painter.line_segment([start, end], Stroke::new(3.0, color));

        // Add small arrow head at the end
        let arrow_size = 4.0;
        let arrow_angle = std::f32::consts::PI / 6.0;

        let arrow_left = Pos2::new(
            end.x - arrow_size * (angle + arrow_angle).cos(),
            end.y + arrow_size * (angle + arrow_angle).sin(),
        );
        let arrow_right = Pos2::new(
            end.x - arrow_size * (angle - arrow_angle).cos(),
            end.y + arrow_size * (angle - arrow_angle).sin(),
        );

        painter.line_segment([end, arrow_left], Stroke::new(2.0, color));
        painter.line_segment([end, arrow_right], Stroke::new(2.0, color));
    }

    /// Render a compact gradient badge for status bars.
    pub fn badge(ui: &mut Ui, gradient: f32) {
        let color = Self::gradient_color(gradient);
        let text = format!("{:+.1}%", gradient);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Grade:").weak().size(12.0));
            ui.label(RichText::new(text).color(color).strong().size(14.0));
        });
    }

    /// Render a mini gradient indicator (just the colored value).
    pub fn mini(ui: &mut Ui, gradient: f32) {
        let color = Self::gradient_color(gradient);
        let text = format!("{:+.1}%", gradient);
        ui.label(RichText::new(text).color(color).strong());
    }

    /// Render distance remaining.
    pub fn distance_remaining(ui: &mut Ui, distance_m: f64, total_m: f64) {
        let remaining_m = total_m - distance_m;
        let remaining_km = remaining_m / 1000.0;
        let progress_pct = ((distance_m / total_m) * 100.0).clamp(0.0, 100.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{:.1} km", remaining_km)).strong());
            ui.label(RichText::new("remaining").weak().size(11.0));
            ui.label(RichText::new(format!("({:.0}%)", progress_pct)).weak());
        });
    }
}

/// Gradient profile preview showing upcoming gradients.
pub struct GradientPreview;

impl GradientPreview {
    /// Render a gradient profile preview chart.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `gradients` - Slice of upcoming gradient values
    /// * `current_index` - Index of current position in the gradients slice
    /// * `height` - Height of the preview chart
    pub fn show(ui: &mut Ui, gradients: &[f32], current_index: usize, height: f32) {
        if gradients.is_empty() {
            return;
        }

        let available_width = ui.available_width();

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, height),
            egui::Sense::hover(),
        );

        let rect = response.rect;
        let bg_color = Color32::from_gray(25);
        painter.rect_filled(rect, 2.0, bg_color);

        // Find gradient range for scaling
        let max_gradient = gradients.iter().cloned().fold(0.0_f32, f32::max).max(5.0);
        let min_gradient = gradients.iter().cloned().fold(0.0_f32, f32::min).min(-5.0);
        let range = max_gradient - min_gradient;

        // Calculate bar width
        let bar_count = gradients.len();
        let bar_width = available_width / bar_count as f32;
        let center_y = rect.min.y + height * (max_gradient / range);

        // Draw zero line
        painter.line_segment(
            [
                Pos2::new(rect.min.x, center_y),
                Pos2::new(rect.max.x, center_y),
            ],
            Stroke::new(1.0, Color32::from_gray(60)),
        );

        // Draw gradient bars
        for (i, &gradient) in gradients.iter().enumerate() {
            let x = rect.min.x + i as f32 * bar_width;
            let bar_height = (gradient / range) * height;

            let (bar_top, bar_bottom) = if gradient >= 0.0 {
                (center_y - bar_height, center_y)
            } else {
                (center_y, center_y - bar_height)
            };

            let bar_rect = Rect::from_min_max(
                Pos2::new(x, bar_top),
                Pos2::new(x + bar_width - 1.0, bar_bottom),
            );

            let color = if i == current_index {
                Color32::WHITE
            } else {
                GradientDisplay::gradient_color(gradient).linear_multiply(0.7)
            };

            painter.rect_filled(bar_rect, 0.0, color);
        }

        // Draw current position marker
        if current_index < gradients.len() {
            let marker_x = rect.min.x + current_index as f32 * bar_width + bar_width / 2.0;
            painter.line_segment(
                [
                    Pos2::new(marker_x, rect.min.y),
                    Pos2::new(marker_x, rect.max.y),
                ],
                Stroke::new(2.0, Color32::WHITE),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_color_flat() {
        let color = GradientDisplay::gradient_color(0.0);
        // Should be greenish
        assert!(color.g() > color.r());
    }

    #[test]
    fn test_gradient_color_steep() {
        let color = GradientDisplay::gradient_color(15.0);
        // Should be reddish
        assert!(color.r() > color.g());
    }

    #[test]
    fn test_gradient_color_downhill() {
        let color = GradientDisplay::gradient_color(-10.0);
        // Should be bluish
        assert!(color.b() > color.r());
    }

    #[test]
    fn test_config_default() {
        let config = GradientDisplayConfig::default();
        assert!(config.show_progress);
        assert!(config.show_preview);
        assert_eq!(config.height, 60.0);
    }
}
