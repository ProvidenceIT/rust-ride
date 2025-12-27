//! Cycling dynamics display widget.
//!
//! T051: Create dynamics_display.rs with L/R balance arc visualization
//!
//! This widget displays cycling dynamics data including:
//! - Left/Right power balance as an arc visualization
//! - Pedal smoothness percentages
//! - Torque effectiveness percentages

use egui::{Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};

use crate::sensors::{CyclingDynamicsData, DynamicsAverages, PowerPhase};

/// Widget for displaying cycling dynamics data.
pub struct DynamicsDisplay {
    /// Current dynamics data
    current_data: Option<CyclingDynamicsData>,
    /// Session averages
    session_averages: DynamicsAverages,
    /// Show extended metrics (smoothness, torque effectiveness)
    show_extended: bool,
    /// Widget size
    size: Vec2,
}

impl DynamicsDisplay {
    /// Create a new dynamics display widget.
    pub fn new() -> Self {
        Self {
            current_data: None,
            session_averages: DynamicsAverages::default(),
            show_extended: true,
            size: Vec2::new(200.0, 150.0),
        }
    }

    /// Set the widget size.
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    /// Set whether to show extended metrics.
    pub fn with_extended(mut self, show: bool) -> Self {
        self.show_extended = show;
        self
    }

    /// Update with new dynamics data.
    pub fn update(&mut self, data: Option<CyclingDynamicsData>) {
        self.current_data = data;
    }

    /// Update session averages.
    pub fn update_averages(&mut self, averages: DynamicsAverages) {
        self.session_averages = averages;
    }

    /// Render the widget.
    pub fn show(&self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Power Balance").size(14.0).strong());
            ui.add_space(4.0);

            // Balance arc visualization
            self.draw_balance_arc(ui);

            if self.show_extended {
                ui.add_space(8.0);
                self.draw_extended_metrics(ui);
            }

            // T129: Power phase (force vector) visualization if available
            if let Some(ref data) = self.current_data {
                if data.has_power_phases() {
                    ui.add_space(8.0);
                    self.draw_power_phases(ui, data);
                }
            }
        });
    }

    /// Draw an arc using line segments.
    fn draw_arc_segments(
        painter: &egui::Painter,
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: Stroke,
        segments: usize,
    ) {
        if segments == 0 {
            return;
        }
        let angle_step = (end_angle - start_angle) / segments as f32;
        for i in 0..segments {
            let a1 = start_angle + i as f32 * angle_step;
            let a2 = start_angle + (i + 1) as f32 * angle_step;
            let p1 = Pos2::new(center.x + radius * a1.cos(), center.y - radius * a1.sin());
            let p2 = Pos2::new(center.x + radius * a2.cos(), center.y - radius * a2.sin());
            painter.line_segment([p1, p2], stroke);
        }
    }

    /// Draw the L/R balance arc visualization.
    fn draw_balance_arc(&self, ui: &mut Ui) {
        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(self.size.x, 60.0), egui::Sense::hover());
        let painter = ui.painter();

        let center = Pos2::new(rect.center().x, rect.max.y - 10.0);
        let radius = 40.0;

        // Get balance values
        let (left_percent, right_percent) = match &self.current_data {
            Some(data) => (data.balance.left_percent, data.balance.right_percent),
            None => (50.0, 50.0),
        };

        // Background arc (gray) - semi-circle from PI to 0
        let arc_stroke = Stroke::new(8.0, Color32::from_gray(60));
        Self::draw_arc_segments(
            painter,
            center,
            radius,
            0.0,
            std::f32::consts::PI,
            arc_stroke,
            32,
        );

        // Calculate arc angles based on balance
        // Left is from PI to center (PI/2), right is from center (PI/2) to 0
        let center_angle = std::f32::consts::PI / 2.0;
        let balance_angle = (left_percent - 50.0) / 50.0 * (std::f32::consts::PI / 2.0);
        let indicator_angle = center_angle - balance_angle;

        // Left side arc (blue) - from PI to PI/2
        let left_color = if left_percent > 52.0 {
            Color32::from_rgb(100, 149, 237) // Cornflower blue for imbalance
        } else {
            Color32::from_rgb(70, 130, 180) // Steel blue
        };
        Self::draw_arc_segments(
            painter,
            center,
            radius,
            center_angle,
            std::f32::consts::PI,
            Stroke::new(8.0, left_color),
            16,
        );

        // Right side arc (orange/red) - from PI/2 to 0
        let right_color = if right_percent > 52.0 {
            Color32::from_rgb(255, 140, 0) // Dark orange for imbalance
        } else {
            Color32::from_rgb(210, 105, 30) // Chocolate
        };
        Self::draw_arc_segments(
            painter,
            center,
            radius,
            0.0,
            center_angle,
            Stroke::new(8.0, right_color),
            16,
        );

        // Balance indicator line
        let indicator_length = 30.0;
        let indicator_end = Pos2::new(
            center.x + indicator_length * indicator_angle.cos(),
            center.y - indicator_length * indicator_angle.sin(),
        );
        painter.line_segment([center, indicator_end], Stroke::new(3.0, Color32::WHITE));

        // Center dot
        painter.circle_filled(center, 5.0, Color32::WHITE);

        // Labels
        let text_color = Color32::WHITE;
        let label_y = rect.max.y - 5.0;

        // Left label
        painter.text(
            Pos2::new(rect.min.x + 15.0, label_y),
            egui::Align2::CENTER_CENTER,
            format!("L {:.0}%", left_percent),
            egui::FontId::proportional(12.0),
            if left_percent > right_percent {
                left_color
            } else {
                text_color
            },
        );

        // Right label
        painter.text(
            Pos2::new(rect.max.x - 15.0, label_y),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}% R", right_percent),
            egui::FontId::proportional(12.0),
            if right_percent > left_percent {
                right_color
            } else {
                text_color
            },
        );

        // Imbalance indicator in center
        let imbalance = left_percent - right_percent;
        if imbalance.abs() > 2.0 {
            let imbalance_text = if imbalance > 0.0 {
                format!("L +{:.0}%", imbalance)
            } else {
                format!("R +{:.0}%", -imbalance)
            };
            painter.text(
                Pos2::new(center.x, center.y - radius - 15.0),
                egui::Align2::CENTER_CENTER,
                imbalance_text,
                egui::FontId::proportional(11.0),
                Color32::YELLOW,
            );
        }
    }

    /// Draw extended metrics (smoothness, torque effectiveness).
    fn draw_extended_metrics(&self, ui: &mut Ui) {
        let (smoothness, torque_eff) = match &self.current_data {
            Some(data) => (&data.smoothness, &data.torque_effectiveness),
            None => return,
        };

        // Only show if we have non-zero values
        if smoothness.combined_percent == 0.0 && torque_eff.combined_percent == 0.0 {
            return;
        }

        ui.horizontal(|ui| {
            // Pedal Smoothness
            ui.vertical(|ui| {
                ui.label(RichText::new("Smoothness").size(11.0).weak());
                ui.horizontal(|ui| {
                    ui.label(format!("L: {:.0}%", smoothness.left_percent));
                    ui.label(format!("R: {:.0}%", smoothness.right_percent));
                });
            });

            ui.add_space(16.0);

            // Torque Effectiveness
            ui.vertical(|ui| {
                ui.label(RichText::new("Torque Eff.").size(11.0).weak());
                ui.horizontal(|ui| {
                    ui.label(format!("L: {:.0}%", torque_eff.left_percent));
                    ui.label(format!("R: {:.0}%", torque_eff.right_percent));
                });
            });
        });
    }

    /// T129: Draw power phase visualization (force vectors).
    ///
    /// Shows the pedal stroke power phase as arcs around a crank circle,
    /// indicating where in the rotation power is being applied.
    fn draw_power_phases(&self, ui: &mut Ui, data: &CyclingDynamicsData) {
        ui.label(RichText::new("Power Phase").size(11.0).weak());

        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(self.size.x, 80.0), egui::Sense::hover());
        let painter = ui.painter();

        // Draw two crank circles side by side (left and right)
        let radius = 25.0;
        let left_center = Pos2::new(rect.min.x + 50.0, rect.center().y);
        let right_center = Pos2::new(rect.max.x - 50.0, rect.center().y);

        // Left pedal power phase
        if let Some(ref phase) = data.left_power_phase {
            Self::draw_power_phase_circle(
                painter,
                left_center,
                radius,
                phase,
                Color32::from_rgb(70, 130, 180),
            );
            painter.text(
                Pos2::new(left_center.x, rect.max.y - 5.0),
                egui::Align2::CENTER_CENTER,
                format!("L: {:.0}°-{:.0}°", phase.start_angle, phase.end_angle),
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        } else {
            Self::draw_empty_circle(painter, left_center, radius);
            painter.text(
                Pos2::new(left_center.x, rect.max.y - 5.0),
                egui::Align2::CENTER_CENTER,
                "L: --",
                egui::FontId::proportional(10.0),
                Color32::GRAY,
            );
        }

        // Right pedal power phase
        if let Some(ref phase) = data.right_power_phase {
            Self::draw_power_phase_circle(
                painter,
                right_center,
                radius,
                phase,
                Color32::from_rgb(210, 105, 30),
            );
            painter.text(
                Pos2::new(right_center.x, rect.max.y - 5.0),
                egui::Align2::CENTER_CENTER,
                format!("R: {:.0}°-{:.0}°", phase.start_angle, phase.end_angle),
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        } else {
            Self::draw_empty_circle(painter, right_center, radius);
            painter.text(
                Pos2::new(right_center.x, rect.max.y - 5.0),
                egui::Align2::CENTER_CENTER,
                "R: --",
                egui::FontId::proportional(10.0),
                Color32::GRAY,
            );
        }
    }

    /// Draw a power phase circle with arc highlighting the power zone.
    fn draw_power_phase_circle(
        painter: &egui::Painter,
        center: Pos2,
        radius: f32,
        phase: &PowerPhase,
        color: Color32,
    ) {
        // Background circle (gray)
        painter.circle_stroke(center, radius, Stroke::new(3.0, Color32::from_gray(50)));

        // Power phase arc
        // Convert angles from degrees (0° = top dead center, clockwise) to radians for rendering
        // In egui, 0 radians = right, angles go counter-clockwise
        // So we need to convert: render_angle = PI/2 - (degrees * PI / 180)
        let start_rad = std::f32::consts::PI / 2.0 - phase.start_angle.to_radians();
        let end_rad = std::f32::consts::PI / 2.0 - phase.end_angle.to_radians();

        // Draw power phase arc
        Self::draw_arc_segments(
            painter,
            center,
            radius,
            end_rad,
            start_rad,
            Stroke::new(4.0, color),
            24,
        );

        // Peak power indicator (if available)
        if let Some(peak) = phase.peak_angle {
            let peak_rad = std::f32::consts::PI / 2.0 - peak.to_radians();
            let peak_pos = Pos2::new(
                center.x + radius * peak_rad.cos(),
                center.y - radius * peak_rad.sin(),
            );
            painter.circle_filled(peak_pos, 4.0, Color32::WHITE);
        }

        // Center dot
        painter.circle_filled(center, 3.0, Color32::from_gray(100));
    }

    /// Draw an empty placeholder circle.
    fn draw_empty_circle(painter: &egui::Painter, center: Pos2, radius: f32) {
        painter.circle_stroke(center, radius, Stroke::new(2.0, Color32::from_gray(40)));
        painter.circle_filled(center, 3.0, Color32::from_gray(60));
    }
}

impl Default for DynamicsDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Compact balance bar widget for inline display.
pub struct BalanceBar {
    left_percent: f32,
    right_percent: f32,
    width: f32,
    height: f32,
}

impl BalanceBar {
    /// Create a new balance bar.
    pub fn new(left_percent: f32, right_percent: f32) -> Self {
        Self {
            left_percent: left_percent.clamp(0.0, 100.0),
            right_percent: right_percent.clamp(0.0, 100.0),
            width: 100.0,
            height: 16.0,
        }
    }

    /// Set the bar dimensions.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Render the balance bar.
    pub fn show(&self, ui: &mut Ui) {
        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(self.width, self.height), egui::Sense::hover());
        let painter = ui.painter();

        let total = self.left_percent + self.right_percent;
        if total == 0.0 {
            return;
        }

        let left_width = (self.left_percent / total) * rect.width();
        let center = rect.center().x;

        // Background
        painter.rect_filled(rect, 2.0, Color32::from_gray(40));

        // Left bar (grows from center to left)
        let left_rect = Rect::from_min_max(
            Pos2::new(center - left_width / 2.0 * 2.0, rect.min.y),
            Pos2::new(center, rect.max.y),
        );
        painter.rect_filled(left_rect, 2.0, Color32::from_rgb(70, 130, 180));

        // Right bar (grows from center to right)
        let right_rect = Rect::from_min_max(
            Pos2::new(center, rect.min.y),
            Pos2::new(center + (rect.width() - left_width / 2.0 * 2.0), rect.max.y),
        );
        painter.rect_filled(right_rect, 2.0, Color32::from_rgb(210, 105, 30));

        // Center line
        painter.vline(center, rect.y_range(), Stroke::new(1.0, Color32::WHITE));

        // Labels
        painter.text(
            Pos2::new(rect.min.x + 3.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{:.0}", self.left_percent),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );

        painter.text(
            Pos2::new(rect.max.x - 3.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{:.0}", self.right_percent),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamics_display_new() {
        let display = DynamicsDisplay::new();
        assert!(display.current_data.is_none());
        assert!(display.show_extended);
    }

    #[test]
    fn test_dynamics_display_update() {
        let mut display = DynamicsDisplay::new();
        let data = CyclingDynamicsData::default();
        display.update(Some(data));
        assert!(display.current_data.is_some());
    }

    #[test]
    fn test_balance_bar_new() {
        let bar = BalanceBar::new(52.0, 48.0);
        assert_eq!(bar.left_percent, 52.0);
        assert_eq!(bar.right_percent, 48.0);
    }

    #[test]
    fn test_balance_bar_clamping() {
        let bar = BalanceBar::new(150.0, -10.0);
        assert_eq!(bar.left_percent, 100.0);
        assert_eq!(bar.right_percent, 0.0);
    }
}
