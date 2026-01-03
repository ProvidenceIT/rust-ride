//! Connection quality indicator widget.
//!
//! T009-3.3: Visual indicator showing connection quality with signal bars,
//! color coding (green/yellow/red), and tooltip with details.

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use crate::sensors::quality::{QualityLevel, QualityStats};

/// Colors for connection quality levels.
struct QualityColors {
    /// Primary bar color.
    bar: Color32,
    /// Background for inactive bars.
    inactive: Color32,
    /// Text color for tooltip.
    text: Color32,
}

impl QualityColors {
    /// Get colors for a quality level.
    fn for_level(level: QualityLevel) -> Self {
        match level {
            QualityLevel::Excellent => Self {
                bar: Color32::from_rgb(52, 168, 83),     // Green
                inactive: Color32::from_rgb(40, 60, 45),
                text: Color32::from_rgb(140, 220, 160),
            },
            QualityLevel::Good => Self {
                bar: Color32::from_rgb(102, 187, 106),   // Light green
                inactive: Color32::from_rgb(45, 60, 48),
                text: Color32::from_rgb(160, 220, 170),
            },
            QualityLevel::Fair => Self {
                bar: Color32::from_rgb(251, 188, 4),     // Yellow/amber
                inactive: Color32::from_rgb(70, 60, 35),
                text: Color32::from_rgb(255, 210, 100),
            },
            QualityLevel::Poor => Self {
                bar: Color32::from_rgb(234, 67, 53),     // Red
                inactive: Color32::from_rgb(70, 40, 38),
                text: Color32::from_rgb(255, 150, 140),
            },
        }
    }
}

/// Configuration for connection quality indicator.
#[derive(Debug, Clone)]
pub struct ConnectionQualityIndicatorConfig {
    /// Size of the widget (width).
    pub width: f32,
    /// Height of the tallest bar.
    pub max_bar_height: f32,
    /// Width of each bar.
    pub bar_width: f32,
    /// Spacing between bars.
    pub bar_spacing: f32,
    /// Corner radius for bars.
    pub bar_corner_radius: f32,
    /// Whether to show tooltip on hover.
    pub show_tooltip: bool,
    /// Whether to animate degraded quality.
    pub animate_degraded: bool,
}

impl Default for ConnectionQualityIndicatorConfig {
    fn default() -> Self {
        Self {
            width: 24.0,
            max_bar_height: 16.0,
            bar_width: 4.0,
            bar_spacing: 2.0,
            bar_corner_radius: 1.0,
            show_tooltip: true,
            animate_degraded: true,
        }
    }
}

impl ConnectionQualityIndicatorConfig {
    /// Create a compact configuration for status bars.
    pub fn compact() -> Self {
        Self {
            width: 16.0,
            max_bar_height: 12.0,
            bar_width: 3.0,
            bar_spacing: 1.5,
            bar_corner_radius: 0.5,
            show_tooltip: true,
            animate_degraded: true,
        }
    }

    /// Create a larger configuration for detailed views.
    pub fn large() -> Self {
        Self {
            width: 32.0,
            max_bar_height: 20.0,
            bar_width: 6.0,
            bar_spacing: 3.0,
            bar_corner_radius: 1.5,
            show_tooltip: true,
            animate_degraded: true,
        }
    }
}

/// Connection quality indicator widget.
///
/// Displays signal bars (1-4) with color coding based on connection quality.
/// Shows a tooltip with detailed metrics on hover.
pub struct ConnectionQualityIndicator {
    /// Quality statistics to display.
    stats: Option<QualityStats>,
    /// Widget configuration.
    config: ConnectionQualityIndicatorConfig,
    /// Override for signal bars (for testing/preview).
    override_bars: Option<u8>,
    /// Override for quality level (for testing/preview).
    override_level: Option<QualityLevel>,
    /// Label to show next to the indicator.
    label: Option<String>,
}

impl Default for ConnectionQualityIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionQualityIndicator {
    /// Create a new connection quality indicator.
    pub fn new() -> Self {
        Self {
            stats: None,
            config: ConnectionQualityIndicatorConfig::default(),
            override_bars: None,
            override_level: None,
            label: None,
        }
    }

    /// Create an indicator with quality statistics.
    pub fn with_stats(mut self, stats: QualityStats) -> Self {
        self.stats = Some(stats);
        self
    }

    /// Set custom configuration.
    pub fn with_config(mut self, config: ConnectionQualityIndicatorConfig) -> Self {
        self.config = config;
        self
    }

    /// Set to compact mode.
    pub fn compact(mut self) -> Self {
        self.config = ConnectionQualityIndicatorConfig::compact();
        self
    }

    /// Set to large mode.
    pub fn large(mut self) -> Self {
        self.config = ConnectionQualityIndicatorConfig::large();
        self
    }

    /// Override signal bars for preview/testing.
    pub fn with_bars(mut self, bars: u8) -> Self {
        self.override_bars = Some(bars.clamp(0, 4));
        self
    }

    /// Override quality level for preview/testing.
    pub fn with_level(mut self, level: QualityLevel) -> Self {
        self.override_level = Some(level);
        self
    }

    /// Set a label to display next to the indicator.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Disable tooltip.
    pub fn without_tooltip(mut self) -> Self {
        self.config.show_tooltip = false;
        self
    }

    /// Get the effective signal bars (from stats or override).
    fn get_signal_bars(&self) -> u8 {
        if let Some(bars) = self.override_bars {
            return bars;
        }
        self.stats.as_ref().map(|s| s.signal_bars).unwrap_or(0)
    }

    /// Get the effective quality level (from stats or override).
    fn get_quality_level(&self) -> QualityLevel {
        if let Some(level) = self.override_level {
            return level;
        }
        self.stats.as_ref().map(|s| s.level).unwrap_or(QualityLevel::Poor)
    }

    /// Show the connection quality indicator.
    pub fn show(&self, ui: &mut Ui) -> ConnectionQualityIndicatorResponse {
        let signal_bars = self.get_signal_bars();
        let quality_level = self.get_quality_level();

        // Calculate widget dimensions
        let total_bar_width = (self.config.bar_width + self.config.bar_spacing) * 4.0 - self.config.bar_spacing;
        let widget_height = self.config.max_bar_height;

        let widget_size = if self.label.is_some() {
            // Extra width for label
            Vec2::new(total_bar_width + 40.0, widget_height)
        } else {
            Vec2::new(total_bar_width, widget_height)
        };

        let (rect, response) = ui.allocate_exact_size(widget_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            self.draw_bars(ui, rect, signal_bars, quality_level);

            // Draw label if present
            if let Some(ref label) = self.label {
                let colors = QualityColors::for_level(quality_level);
                let label_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x + total_bar_width + 4.0, rect.min.y),
                    rect.max,
                );
                ui.painter().text(
                    label_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(11.0),
                    colors.text,
                );
            }
        }

        // Show tooltip on hover
        let response = if self.config.show_tooltip {
            if let Some(ref stats) = self.stats {
                response.on_hover_ui(|ui| {
                    self.draw_tooltip(ui, stats);
                })
            } else {
                response.on_hover_text(format!(
                    "Signal: {} ({})",
                    quality_level,
                    format_signal_bars(signal_bars)
                ))
            }
        } else {
            response
        };

        ConnectionQualityIndicatorResponse {
            response,
            signal_bars,
            quality_level,
            needs_attention: quality_level == QualityLevel::Poor,
        }
    }

    /// Draw the signal bars.
    fn draw_bars(&self, ui: &mut Ui, rect: Rect, active_bars: u8, level: QualityLevel) {
        let painter = ui.painter();
        let colors = QualityColors::for_level(level);

        // Bar heights as fraction of max height (increasing left to right)
        let bar_heights = [0.3, 0.5, 0.75, 1.0];

        // Check if we need to animate (poor quality pulsing)
        let should_pulse = self.config.animate_degraded && level == QualityLevel::Poor;
        let alpha_modifier = if should_pulse {
            let time = ui.ctx().input(|i| i.time);
            let pulse = ((time * 2.5).sin() * 0.3 + 0.7) as f32;
            pulse
        } else {
            1.0
        };

        for i in 0..4 {
            let bar_index = i as u8;
            let is_active = bar_index < active_bars;

            let bar_height = bar_heights[i] * self.config.max_bar_height;

            // Calculate bar position (left to right)
            let bar_x = rect.min.x + (self.config.bar_width + self.config.bar_spacing) * i as f32;
            let bar_y = rect.max.y - bar_height;

            let bar_rect = Rect::from_min_size(
                Pos2::new(bar_x, bar_y),
                Vec2::new(self.config.bar_width, bar_height),
            );

            // Determine bar color
            let bar_color = if is_active {
                if alpha_modifier < 1.0 {
                    // Apply pulse animation
                    let Color32 {
                        r, g, b, a,
                    } = colors.bar;
                    Color32::from_rgba_unmultiplied(r, g, b, (a as f32 * alpha_modifier) as u8)
                } else {
                    colors.bar
                }
            } else {
                colors.inactive
            };

            painter.rect_filled(bar_rect, self.config.bar_corner_radius, bar_color);
        }

        // Request repaint for animation
        if should_pulse {
            ui.ctx().request_repaint();
        }
    }

    /// Draw the tooltip content.
    fn draw_tooltip(&self, ui: &mut Ui, stats: &QualityStats) {
        ui.spacing_mut().item_spacing.y = 4.0;

        // Header with quality level
        ui.horizontal(|ui| {
            ui.label(RichText::new("Connection Quality").strong());
        });

        ui.separator();

        // Overall score and level
        let colors = QualityColors::for_level(stats.level);
        ui.horizontal(|ui| {
            ui.label(format_signal_bars(stats.signal_bars));
            ui.label(RichText::new(format!("{} ({}%)", stats.level, stats.score)).color(colors.text));
        });

        ui.add_space(4.0);

        // Detailed metrics
        if let Some(rssi) = stats.metrics.rssi_avg {
            ui.horizontal(|ui| {
                ui.label(RichText::new("RSSI:").weak());
                let rssi_color = rssi_to_color(rssi);
                ui.label(RichText::new(format!("{} dBm", rssi)).color(rssi_color));
            });
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("Data rate:").weak());
            ui.label(format!("{:.1} pkt/s", stats.metrics.data_rate));
        });

        if stats.metrics.packet_loss_rate > 0.0 {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Packet loss:").weak());
                let loss_color = if stats.metrics.packet_loss_rate > 5.0 {
                    Color32::from_rgb(234, 67, 53)
                } else if stats.metrics.packet_loss_rate > 2.0 {
                    Color32::from_rgb(251, 188, 4)
                } else {
                    Color32::GRAY
                };
                ui.label(RichText::new(format!("{:.1}%", stats.metrics.packet_loss_rate)).color(loss_color));
            });
        }

        if let Some(latency) = stats.metrics.latency_avg_ms {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Latency:").weak());
                let latency_color = if latency > 200 {
                    Color32::from_rgb(234, 67, 53)
                } else if latency > 100 {
                    Color32::from_rgb(251, 188, 4)
                } else {
                    Color32::GRAY
                };
                ui.label(RichText::new(format!("{} ms", latency)).color(latency_color));
            });
        }

        // Component scores
        ui.add_space(4.0);
        ui.label(RichText::new("Component scores:").weak().small());

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Signal: {}", stats.metrics.rssi_score)).small());
            ui.separator();
            ui.label(RichText::new(format!("Rate: {}", stats.metrics.data_rate_score)).small());
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Loss: {}", stats.metrics.packet_loss_score)).small());
            ui.separator();
            ui.label(RichText::new(format!("Latency: {}", stats.metrics.latency_score)).small());
        });

        // Uptime
        ui.add_space(4.0);
        let uptime_secs = stats.uptime.as_secs();
        if uptime_secs >= 60 {
            ui.label(RichText::new(format!("Monitoring: {}m {}s", uptime_secs / 60, uptime_secs % 60)).weak().small());
        } else {
            ui.label(RichText::new(format!("Monitoring: {}s", uptime_secs)).weak().small());
        }
    }
}

/// Response from showing a connection quality indicator.
pub struct ConnectionQualityIndicatorResponse {
    /// The egui response.
    pub response: egui::Response,
    /// Number of signal bars displayed (0-4).
    pub signal_bars: u8,
    /// Quality level.
    pub quality_level: QualityLevel,
    /// Whether the connection needs attention (poor quality).
    pub needs_attention: bool,
}

/// Compact connection quality indicator for status bars.
pub struct CompactConnectionQualityIndicator {
    /// Quality statistics.
    stats: Option<QualityStats>,
    /// Override bars for preview.
    bars: Option<u8>,
}

impl CompactConnectionQualityIndicator {
    /// Create a new compact indicator.
    pub fn new() -> Self {
        Self {
            stats: None,
            bars: None,
        }
    }

    /// Create with quality stats.
    pub fn with_stats(mut self, stats: QualityStats) -> Self {
        self.stats = Some(stats);
        self
    }

    /// Override signal bars.
    pub fn with_bars(mut self, bars: u8) -> Self {
        self.bars = Some(bars.clamp(0, 4));
        self
    }

    /// Get the effective signal bars.
    fn get_signal_bars(&self) -> u8 {
        if let Some(bars) = self.bars {
            return bars;
        }
        self.stats.as_ref().map(|s| s.signal_bars).unwrap_or(0)
    }

    /// Get the effective quality level.
    fn get_quality_level(&self) -> QualityLevel {
        self.stats.as_ref().map(|s| s.level).unwrap_or(QualityLevel::Poor)
    }

    /// Show the compact indicator.
    pub fn show(&self, ui: &mut Ui) -> egui::Response {
        let signal_bars = self.get_signal_bars();
        let level = self.get_quality_level();

        let size = Vec2::new(12.0, 10.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let colors = QualityColors::for_level(level);

            let bar_width = 2.0;
            let bar_spacing = 1.0;
            let bar_heights = [0.3, 0.5, 0.75, 1.0];

            for i in 0..4 {
                let is_active = (i as u8) < signal_bars;
                let bar_height = bar_heights[i] * size.y;
                let bar_x = rect.min.x + (bar_width + bar_spacing) * i as f32;
                let bar_y = rect.max.y - bar_height;

                let bar_rect = Rect::from_min_size(
                    Pos2::new(bar_x, bar_y),
                    Vec2::new(bar_width, bar_height),
                );

                let bar_color = if is_active { colors.bar } else { colors.inactive };
                painter.rect_filled(bar_rect, 0.5, bar_color);
            }
        }

        // Add tooltip
        let tooltip = if let Some(ref stats) = self.stats {
            stats.summary()
        } else {
            format_signal_bars(signal_bars)
        };

        response.on_hover_text(tooltip)
    }
}

impl Default for CompactConnectionQualityIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// A preview widget showing all quality levels for testing/demo.
pub struct ConnectionQualityPreview;

impl ConnectionQualityPreview {
    /// Show a preview of all quality levels.
    pub fn show(ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Quality levels: ");

            for (bars, level) in [
                (4, QualityLevel::Excellent),
                (3, QualityLevel::Good),
                (2, QualityLevel::Fair),
                (1, QualityLevel::Poor),
            ] {
                ConnectionQualityIndicator::new()
                    .with_bars(bars)
                    .with_level(level)
                    .without_tooltip()
                    .show(ui);
                ui.add_space(8.0);
            }
        });
    }
}

/// Format signal bars as a text representation.
fn format_signal_bars(bars: u8) -> String {
    match bars {
        4 => "▂▄▆█".to_string(),
        3 => "▂▄▆░".to_string(),
        2 => "▂▄░░".to_string(),
        1 => "▂░░░".to_string(),
        _ => "░░░░".to_string(),
    }
}

/// Get a color based on RSSI value.
fn rssi_to_color(rssi: i16) -> Color32 {
    if rssi >= -50 {
        Color32::from_rgb(52, 168, 83)     // Excellent - green
    } else if rssi >= -70 {
        Color32::from_rgb(102, 187, 106)   // Good - light green
    } else if rssi >= -85 {
        Color32::from_rgb(251, 188, 4)     // Fair - yellow
    } else {
        Color32::from_rgb(234, 67, 53)     // Poor - red
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_quality_colors_for_levels() {
        // Just verify colors are created without panicking
        let _excellent = QualityColors::for_level(QualityLevel::Excellent);
        let _good = QualityColors::for_level(QualityLevel::Good);
        let _fair = QualityColors::for_level(QualityLevel::Fair);
        let _poor = QualityColors::for_level(QualityLevel::Poor);
    }

    #[test]
    fn test_indicator_config_presets() {
        let default = ConnectionQualityIndicatorConfig::default();
        assert_eq!(default.width, 24.0);

        let compact = ConnectionQualityIndicatorConfig::compact();
        assert_eq!(compact.width, 16.0);

        let large = ConnectionQualityIndicatorConfig::large();
        assert_eq!(large.width, 32.0);
    }

    #[test]
    fn test_indicator_with_overrides() {
        let indicator = ConnectionQualityIndicator::new()
            .with_bars(3)
            .with_level(QualityLevel::Good);

        assert_eq!(indicator.get_signal_bars(), 3);
        assert_eq!(indicator.get_quality_level(), QualityLevel::Good);
    }

    #[test]
    fn test_indicator_bars_clamped() {
        let indicator = ConnectionQualityIndicator::new().with_bars(10);
        assert_eq!(indicator.get_signal_bars(), 4); // Clamped to max

        let indicator = ConnectionQualityIndicator::new().with_bars(0);
        assert_eq!(indicator.get_signal_bars(), 0);
    }

    #[test]
    fn test_format_signal_bars() {
        assert_eq!(format_signal_bars(4), "▂▄▆█");
        assert_eq!(format_signal_bars(3), "▂▄▆░");
        assert_eq!(format_signal_bars(2), "▂▄░░");
        assert_eq!(format_signal_bars(1), "▂░░░");
        assert_eq!(format_signal_bars(0), "░░░░");
    }

    #[test]
    fn test_rssi_to_color() {
        // Excellent signal
        let color = rssi_to_color(-40);
        assert_eq!(color, Color32::from_rgb(52, 168, 83));

        // Good signal
        let color = rssi_to_color(-60);
        assert_eq!(color, Color32::from_rgb(102, 187, 106));

        // Fair signal
        let color = rssi_to_color(-80);
        assert_eq!(color, Color32::from_rgb(251, 188, 4));

        // Poor signal
        let color = rssi_to_color(-90);
        assert_eq!(color, Color32::from_rgb(234, 67, 53));
    }

    #[test]
    fn test_compact_indicator() {
        let compact = CompactConnectionQualityIndicator::new()
            .with_bars(2);

        assert_eq!(compact.get_signal_bars(), 2);
        assert_eq!(compact.get_quality_level(), QualityLevel::Poor); // Default when no stats
    }

    #[test]
    fn test_indicator_default() {
        let indicator = ConnectionQualityIndicator::default();
        assert_eq!(indicator.get_signal_bars(), 0);
        assert_eq!(indicator.get_quality_level(), QualityLevel::Poor);
    }
}
