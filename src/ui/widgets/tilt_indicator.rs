//! Tilt indicator widget for motion/rocker plate visualization.
//!
//! T141: Add simple tilt indicator widget to ride screen
//!
//! Displays current tilt angles from IMU sensor as a visual indicator.

use crate::sensors::{MotionSample, MotionSensorState};
use egui::{Color32, Painter, Pos2, Rect, Stroke, Ui, Vec2};

/// Configuration for the tilt indicator widget.
#[derive(Debug, Clone)]
pub struct TiltIndicatorConfig {
    /// Maximum tilt angle to display (degrees)
    pub max_tilt_degrees: f32,
    /// Size of the indicator in pixels
    pub size: f32,
    /// Show numerical values
    pub show_values: bool,
    /// Show grid lines
    pub show_grid: bool,
    /// Ring color
    pub ring_color: Color32,
    /// Indicator color
    pub indicator_color: Color32,
    /// Grid color
    pub grid_color: Color32,
}

impl Default for TiltIndicatorConfig {
    fn default() -> Self {
        Self {
            max_tilt_degrees: 15.0,
            size: 80.0,
            show_values: true,
            show_grid: true,
            ring_color: Color32::from_rgb(80, 80, 80),
            indicator_color: Color32::from_rgb(255, 165, 0), // Orange
            grid_color: Color32::from_rgb(50, 50, 50),
        }
    }
}

/// Tilt indicator widget for displaying rocker plate/IMU tilt.
pub struct TiltIndicator {
    config: TiltIndicatorConfig,
    /// Current tilt roll angle (degrees, left/right)
    current_roll: f32,
    /// Current tilt pitch angle (degrees, forward/backward)
    current_pitch: f32,
    /// Sensor connection state
    sensor_state: MotionSensorState,
    /// Smoothed roll for display
    smoothed_roll: f32,
    /// Smoothed pitch for display
    smoothed_pitch: f32,
}

impl TiltIndicator {
    /// Create a new tilt indicator with default config.
    pub fn new() -> Self {
        Self {
            config: TiltIndicatorConfig::default(),
            current_roll: 0.0,
            current_pitch: 0.0,
            sensor_state: MotionSensorState::Disconnected,
            smoothed_roll: 0.0,
            smoothed_pitch: 0.0,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: TiltIndicatorConfig) -> Self {
        Self {
            config,
            current_roll: 0.0,
            current_pitch: 0.0,
            sensor_state: MotionSensorState::Disconnected,
            smoothed_roll: 0.0,
            smoothed_pitch: 0.0,
        }
    }

    /// Update with new motion sample.
    pub fn update(&mut self, sample: &MotionSample) {
        self.current_roll = sample.tilt_degrees.0;
        self.current_pitch = sample.tilt_degrees.1;

        // Apply smoothing
        let alpha = 0.3;
        self.smoothed_roll = self.smoothed_roll * (1.0 - alpha) + self.current_roll * alpha;
        self.smoothed_pitch = self.smoothed_pitch * (1.0 - alpha) + self.current_pitch * alpha;
    }

    /// Update sensor connection state.
    pub fn set_sensor_state(&mut self, state: MotionSensorState) {
        self.sensor_state = state;
    }

    /// Get current roll angle.
    pub fn get_roll(&self) -> f32 {
        self.smoothed_roll
    }

    /// Get current pitch angle.
    pub fn get_pitch(&self) -> f32 {
        self.smoothed_pitch
    }

    /// Check if sensor is connected and providing data.
    pub fn is_active(&self) -> bool {
        self.sensor_state == MotionSensorState::Ready
    }

    /// Render the tilt indicator widget.
    pub fn show(&self, ui: &mut Ui) {
        let size = self.config.size;
        let (response, painter) = ui.allocate_painter(Vec2::splat(size), egui::Sense::hover());
        let rect = response.rect;
        let center = rect.center();
        let radius = size / 2.0 - 4.0;

        // Draw outer ring
        painter.circle_stroke(center, radius, Stroke::new(2.0, self.config.ring_color));

        // Draw grid if enabled
        if self.config.show_grid {
            self.draw_grid(&painter, center, radius);
        }

        // Draw tilt indicator
        if self.is_active() {
            self.draw_indicator(&painter, center, radius);
        } else {
            // Draw disconnected state
            self.draw_disconnected(&painter, center, radius);
        }

        // Show values if enabled
        if self.config.show_values && self.is_active() {
            self.draw_values(ui, rect);
        }
    }

    /// Draw grid lines.
    fn draw_grid(&self, painter: &Painter, center: Pos2, radius: f32) {
        let grid_stroke = Stroke::new(1.0, self.config.grid_color);

        // Draw concentric circles at 5 degree intervals
        for deg in [5.0, 10.0] {
            let r = (deg / self.config.max_tilt_degrees) * radius;
            if r < radius {
                painter.circle_stroke(center, r, grid_stroke);
            }
        }

        // Draw crosshairs
        painter.line_segment(
            [
                Pos2::new(center.x - radius, center.y),
                Pos2::new(center.x + radius, center.y),
            ],
            grid_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(center.x, center.y - radius),
                Pos2::new(center.x, center.y + radius),
            ],
            grid_stroke,
        );
    }

    /// Draw the tilt indicator dot.
    fn draw_indicator(&self, painter: &Painter, center: Pos2, radius: f32) {
        // Map tilt angles to position
        let x_offset = (self.smoothed_roll / self.config.max_tilt_degrees) * radius;
        let y_offset = (-self.smoothed_pitch / self.config.max_tilt_degrees) * radius;

        // Clamp to circle
        let offset = Vec2::new(x_offset, y_offset);
        let offset_len = offset.length();
        let clamped_offset = if offset_len > radius {
            offset * (radius / offset_len)
        } else {
            offset
        };

        let dot_pos = center + clamped_offset;

        // Draw indicator dot
        let dot_radius = 6.0;
        painter.circle_filled(dot_pos, dot_radius, self.config.indicator_color);

        // Draw line from center to dot
        painter.line_segment(
            [center, dot_pos],
            Stroke::new(2.0, self.config.indicator_color),
        );
    }

    /// Draw disconnected state.
    fn draw_disconnected(&self, painter: &Painter, center: Pos2, _radius: f32) {
        // Draw X in center
        let size = 10.0;
        let stroke = Stroke::new(2.0, Color32::from_rgb(100, 100, 100));
        painter.line_segment(
            [
                Pos2::new(center.x - size, center.y - size),
                Pos2::new(center.x + size, center.y + size),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(center.x + size, center.y - size),
                Pos2::new(center.x - size, center.y + size),
            ],
            stroke,
        );
    }

    /// Draw numerical values.
    fn draw_values(&self, ui: &mut Ui, rect: Rect) {
        let text = format!(
            "R:{:+.1}° P:{:+.1}°",
            self.smoothed_roll, self.smoothed_pitch
        );

        ui.put(
            Rect::from_min_size(
                Pos2::new(rect.left(), rect.bottom() + 2.0),
                Vec2::new(rect.width(), 14.0),
            ),
            egui::Label::new(
                egui::RichText::new(text)
                    .small()
                    .color(Color32::from_rgb(150, 150, 150)),
            ),
        );
    }

    /// Render a compact version of the indicator.
    pub fn show_compact(&self, ui: &mut Ui) {
        let size = 50.0;
        let (response, painter) = ui.allocate_painter(Vec2::splat(size), egui::Sense::hover());
        let rect = response.rect;
        let center = rect.center();
        let radius = size / 2.0 - 2.0;

        // Draw outer ring
        painter.circle_stroke(center, radius, Stroke::new(1.0, self.config.ring_color));

        // Draw indicator if active
        if self.is_active() {
            let x_offset = (self.smoothed_roll / self.config.max_tilt_degrees) * radius;
            let y_offset = (-self.smoothed_pitch / self.config.max_tilt_degrees) * radius;

            let offset = Vec2::new(x_offset, y_offset);
            let offset_len = offset.length();
            let clamped_offset = if offset_len > radius {
                offset * (radius / offset_len)
            } else {
                offset
            };

            let dot_pos = center + clamped_offset;
            painter.circle_filled(dot_pos, 4.0, self.config.indicator_color);
        }
    }
}

impl Default for TiltIndicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::{Quaternion, Vector3};

    #[test]
    fn test_tilt_indicator_new() {
        let indicator = TiltIndicator::new();
        assert_eq!(indicator.current_roll, 0.0);
        assert_eq!(indicator.current_pitch, 0.0);
        assert!(!indicator.is_active());
    }

    #[test]
    fn test_tilt_indicator_update() {
        let mut indicator = TiltIndicator::new();
        indicator.set_sensor_state(MotionSensorState::Ready);

        let sample = MotionSample::new(
            0,
            Vector3::new(0.0, 0.0, 9.81),
            Vector3::zero(),
            Quaternion::from_euler(0.1, 0.05, 0.0),
        );

        indicator.update(&sample);

        // Should have updated roll/pitch
        assert!(indicator.smoothed_roll.abs() > 0.0 || indicator.smoothed_pitch.abs() > 0.0);
        assert!(indicator.is_active());
    }

    #[test]
    fn test_config_default() {
        let config = TiltIndicatorConfig::default();
        assert_eq!(config.max_tilt_degrees, 15.0);
        assert_eq!(config.size, 80.0);
        assert!(config.show_values);
        assert!(config.show_grid);
    }
}
