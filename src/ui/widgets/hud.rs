//! HUD overlay widgets for active rides.
//!
//! T009-3.5: Minimal connection quality indicator during active rides.
//! Shows compact sensor connection status with visual warning when quality degrades.

use std::collections::HashMap;

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use crate::sensors::quality::{QualityLevel, QualityStats};

/// Colors for HUD connection quality states.
struct HudQualityColors {
    /// Primary indicator color.
    indicator: Color32,
    /// Background color.
    background: Color32,
    /// Text color.
    text: Color32,
}

impl HudQualityColors {
    /// Get colors based on whether quality needs attention.
    fn for_state(needs_attention: bool, is_degraded: bool) -> Self {
        if needs_attention {
            Self {
                indicator: Color32::from_rgb(234, 67, 53),     // Red
                background: Color32::from_rgba_unmultiplied(234, 67, 53, 30),
                text: Color32::from_rgb(255, 150, 140),
            }
        } else if is_degraded {
            Self {
                indicator: Color32::from_rgb(251, 188, 4),     // Yellow/amber
                background: Color32::from_rgba_unmultiplied(251, 188, 4, 25),
                text: Color32::from_rgb(255, 210, 100),
            }
        } else {
            Self {
                indicator: Color32::from_rgb(52, 168, 83),     // Green
                background: Color32::from_rgba_unmultiplied(52, 168, 83, 20),
                text: Color32::from_rgb(160, 220, 170),
            }
        }
    }

    /// Get color for a specific quality level.
    fn for_level(level: QualityLevel) -> Color32 {
        match level {
            QualityLevel::Excellent => Color32::from_rgb(52, 168, 83),     // Green
            QualityLevel::Good => Color32::from_rgb(102, 187, 106),        // Light green
            QualityLevel::Fair => Color32::from_rgb(251, 188, 4),          // Yellow/amber
            QualityLevel::Poor => Color32::from_rgb(234, 67, 53),          // Red
        }
    }
}

/// Configuration for HUD connection quality indicator.
#[derive(Debug, Clone)]
pub struct HudConnectionQualityConfig {
    /// Minimum width of the indicator.
    pub min_width: f32,
    /// Height of the indicator bar.
    pub height: f32,
    /// Bar spacing.
    pub bar_spacing: f32,
    /// Bar width.
    pub bar_width: f32,
    /// Whether to animate warnings.
    pub animate_warnings: bool,
    /// Flash frequency for warnings (Hz).
    pub warning_flash_hz: f32,
    /// Position anchor (left-top corner offset from parent's top-right).
    pub anchor_offset: Vec2,
}

impl Default for HudConnectionQualityConfig {
    fn default() -> Self {
        Self {
            min_width: 80.0,
            height: 24.0,
            bar_spacing: 2.0,
            bar_width: 3.0,
            animate_warnings: true,
            warning_flash_hz: 2.0,
            anchor_offset: Vec2::new(-16.0, 16.0),
        }
    }
}

impl HudConnectionQualityConfig {
    /// Create a more compact configuration.
    pub fn compact() -> Self {
        Self {
            min_width: 60.0,
            height: 20.0,
            bar_spacing: 1.5,
            bar_width: 2.5,
            ..Self::default()
        }
    }
}

/// Aggregated connection quality state for HUD display.
#[derive(Debug, Clone, Default)]
pub struct HudConnectionState {
    /// Quality stats per sensor device_id.
    pub quality_stats: HashMap<String, QualityStats>,
    /// Whether any sensor has poor quality.
    pub has_poor_quality: bool,
    /// Whether any sensor has degraded (fair or poor) quality.
    pub has_degraded_quality: bool,
    /// Count of connected sensors.
    pub connected_count: usize,
    /// Worst quality level among all sensors.
    pub worst_level: Option<QualityLevel>,
    /// Device ID of worst sensor (for tooltip).
    pub worst_device_id: Option<String>,
}

impl HudConnectionState {
    /// Create a new HUD connection state from quality stats.
    pub fn from_stats(stats: Vec<QualityStats>) -> Self {
        let mut state = Self {
            quality_stats: HashMap::new(),
            has_poor_quality: false,
            has_degraded_quality: false,
            connected_count: stats.len(),
            worst_level: None,
            worst_device_id: None,
        };

        for stat in stats {
            // Track worst quality
            let is_worse = state.worst_level
                .map(|current| stat.level < current)
                .unwrap_or(true);

            if is_worse {
                state.worst_level = Some(stat.level);
                state.worst_device_id = Some(stat.device_id.clone());
            }

            // Track degraded/poor flags
            if stat.level == QualityLevel::Poor {
                state.has_poor_quality = true;
            }
            if stat.level <= QualityLevel::Fair {
                state.has_degraded_quality = true;
            }

            state.quality_stats.insert(stat.device_id.clone(), stat);
        }

        state
    }

    /// Update with new quality stats.
    pub fn update(&mut self, stats: Vec<QualityStats>) {
        *self = Self::from_stats(stats);
    }

    /// Check if the indicator should be visible.
    pub fn should_show(&self) -> bool {
        self.connected_count > 0
    }

    /// Check if warning animation should be active.
    pub fn should_animate_warning(&self) -> bool {
        self.has_poor_quality
    }

    /// Get overall signal bars (based on worst sensor).
    pub fn overall_signal_bars(&self) -> u8 {
        self.worst_level.map(|l| l.signal_bars()).unwrap_or(0)
    }

    /// Get a summary string for the connection state.
    pub fn summary(&self) -> String {
        if self.connected_count == 0 {
            return "No sensors".to_string();
        }

        if self.has_poor_quality {
            format!("{} sensor(s) - Poor signal", self.connected_count)
        } else if self.has_degraded_quality {
            format!("{} sensor(s) - Degraded signal", self.connected_count)
        } else {
            format!("{} sensor(s) connected", self.connected_count)
        }
    }
}

/// Minimal HUD connection quality indicator for ride overlay.
///
/// Shows a compact connection quality status that's visible but unobtrusive
/// during active rides. Flashes warning when connection quality degrades.
pub struct HudConnectionQualityIndicator {
    /// Current connection state.
    state: HudConnectionState,
    /// Widget configuration.
    config: HudConnectionQualityConfig,
}

impl Default for HudConnectionQualityIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl HudConnectionQualityIndicator {
    /// Create a new HUD connection quality indicator.
    pub fn new() -> Self {
        Self {
            state: HudConnectionState::default(),
            config: HudConnectionQualityConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: HudConnectionQualityConfig) -> Self {
        Self {
            state: HudConnectionState::default(),
            config,
        }
    }

    /// Update the connection state.
    pub fn update_state(&mut self, state: HudConnectionState) {
        self.state = state;
    }

    /// Update from quality stats vector.
    pub fn update_from_stats(&mut self, stats: Vec<QualityStats>) {
        self.state = HudConnectionState::from_stats(stats);
    }

    /// Set to compact mode.
    pub fn compact(mut self) -> Self {
        self.config = HudConnectionQualityConfig::compact();
        self
    }

    /// Show the HUD indicator.
    ///
    /// Returns the response and whether a warning is currently shown.
    pub fn show(&self, ui: &mut Ui) -> HudConnectionQualityResponse {
        // Don't show if no sensors connected
        if !self.state.should_show() {
            return HudConnectionQualityResponse {
                response: None,
                showing_warning: false,
                needs_attention: false,
            };
        }

        let needs_attention = self.state.has_poor_quality;
        let is_degraded = self.state.has_degraded_quality;
        let colors = HudQualityColors::for_state(needs_attention, is_degraded);

        // Calculate warning flash state
        let show_warning_flash = if self.config.animate_warnings && needs_attention {
            let time = ui.ctx().input(|i| i.time);
            let flash_period = 1.0 / self.config.warning_flash_hz as f64;
            (time / flash_period) as i32 % 2 == 0
        } else {
            needs_attention
        };

        // Calculate widget size
        let signal_bars = self.state.overall_signal_bars();
        let bars_width = (self.config.bar_width + self.config.bar_spacing) * 4.0;
        let text_width = if needs_attention { 45.0 } else { 30.0 };
        let widget_width = bars_width + text_width + 8.0;
        let widget_size = Vec2::new(widget_width.max(self.config.min_width), self.config.height);

        let (rect, response) = ui.allocate_exact_size(widget_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            self.draw_indicator(ui, rect, signal_bars, &colors, show_warning_flash, needs_attention);
        }

        // Show tooltip on hover
        let response = response.on_hover_ui(|ui| {
            self.draw_tooltip(ui);
        });

        // Request repaint for animation
        if self.config.animate_warnings && needs_attention {
            ui.ctx().request_repaint();
        }

        HudConnectionQualityResponse {
            response: Some(response),
            showing_warning: show_warning_flash,
            needs_attention,
        }
    }

    /// Draw the compact indicator.
    fn draw_indicator(
        &self,
        ui: &mut Ui,
        rect: Rect,
        signal_bars: u8,
        colors: &HudQualityColors,
        show_warning_flash: bool,
        needs_attention: bool,
    ) {
        let painter = ui.painter();

        // Draw background with rounded corners
        let bg_color = if show_warning_flash && needs_attention {
            Color32::from_rgba_unmultiplied(234, 67, 53, 50)
        } else {
            colors.background
        };
        painter.rect_filled(rect, 4.0, bg_color);

        // Draw border for warning state
        if needs_attention {
            let border_color = if show_warning_flash {
                Color32::from_rgb(234, 67, 53)
            } else {
                Color32::from_rgba_unmultiplied(234, 67, 53, 100)
            };
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0, border_color));
        }

        // Draw signal bars on the left
        let bar_heights = [0.3, 0.5, 0.75, 1.0];
        let bars_start_x = rect.min.x + 6.0;
        let bars_bottom_y = rect.max.y - 5.0;
        let max_bar_height = self.config.height - 10.0;

        for i in 0..4 {
            let is_active = (i as u8) < signal_bars;
            let bar_height = bar_heights[i] * max_bar_height;
            let bar_x = bars_start_x + (self.config.bar_width + self.config.bar_spacing) * i as f32;
            let bar_y = bars_bottom_y - bar_height;

            let bar_rect = Rect::from_min_size(
                Pos2::new(bar_x, bar_y),
                Vec2::new(self.config.bar_width, bar_height),
            );

            let bar_color = if is_active {
                if needs_attention && show_warning_flash {
                    Color32::from_rgb(234, 67, 53)
                } else {
                    colors.indicator
                }
            } else {
                Color32::from_gray(60)
            };

            painter.rect_filled(bar_rect, 1.0, bar_color);
        }

        // Draw status text/icon on the right
        let text_x = bars_start_x + (self.config.bar_width + self.config.bar_spacing) * 4.0 + 6.0;
        let text_y = rect.center().y;

        if needs_attention && show_warning_flash {
            // Show warning icon when flashing
            painter.text(
                Pos2::new(text_x, text_y),
                egui::Align2::LEFT_CENTER,
                "!",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(234, 67, 53),
            );
        } else {
            // Show sensor count
            let count_text = format!("{}", self.state.connected_count);
            painter.text(
                Pos2::new(text_x, text_y),
                egui::Align2::LEFT_CENTER,
                count_text,
                egui::FontId::proportional(12.0),
                colors.text,
            );
        }
    }

    /// Draw the tooltip content.
    fn draw_tooltip(&self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 4.0;

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Sensor Connections").strong());
        });

        ui.separator();

        // Overall status
        let status_text = self.state.summary();
        let status_color = if self.state.has_poor_quality {
            Color32::from_rgb(234, 67, 53)
        } else if self.state.has_degraded_quality {
            Color32::from_rgb(251, 188, 4)
        } else {
            Color32::from_rgb(52, 168, 83)
        };
        ui.label(RichText::new(status_text).color(status_color));

        ui.add_space(4.0);

        // Per-sensor status
        if !self.state.quality_stats.is_empty() {
            ui.label(RichText::new("Details:").weak().small());

            for stat in self.state.quality_stats.values() {
                ui.horizontal(|ui| {
                    // Signal bars icon
                    let bars_str = format_signal_bars(stat.signal_bars);
                    let level_color = HudQualityColors::for_level(stat.level);
                    ui.label(RichText::new(bars_str).color(level_color).small());

                    // Device name (truncated)
                    let name = if stat.device_id.len() > 20 {
                        format!("{}...", &stat.device_id[..17])
                    } else {
                        stat.device_id.clone()
                    };
                    ui.label(RichText::new(name).small());

                    // Quality level
                    ui.label(RichText::new(format!("({})", stat.level)).color(level_color).small());
                });
            }
        }

        // Warning message for poor connections
        if self.state.has_poor_quality {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("!").color(Color32::from_rgb(234, 67, 53)));
                ui.label(
                    RichText::new("Poor connection - move closer to sensor")
                        .small()
                        .color(Color32::from_rgb(234, 67, 53)),
                );
            });
        }
    }
}

/// Response from showing the HUD connection quality indicator.
pub struct HudConnectionQualityResponse {
    /// The egui response (None if not visible).
    pub response: Option<egui::Response>,
    /// Whether the warning flash is currently visible.
    pub showing_warning: bool,
    /// Whether any connection needs attention.
    pub needs_attention: bool,
}

/// Inline HUD sensor status for the ride top bar.
///
/// Even more minimal than HudConnectionQualityIndicator - just signal bars.
pub struct InlineHudSensorStatus {
    /// Connection state.
    state: HudConnectionState,
}

impl Default for InlineHudSensorStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineHudSensorStatus {
    /// Create a new inline sensor status.
    pub fn new() -> Self {
        Self {
            state: HudConnectionState::default(),
        }
    }

    /// Create from connection state.
    pub fn from_state(state: HudConnectionState) -> Self {
        Self { state }
    }

    /// Create from quality stats.
    pub fn from_stats(stats: Vec<QualityStats>) -> Self {
        Self {
            state: HudConnectionState::from_stats(stats),
        }
    }

    /// Show the inline status (returns true if warning is showing).
    pub fn show(&self, ui: &mut Ui) -> bool {
        if !self.state.should_show() {
            return false;
        }

        let needs_attention = self.state.has_poor_quality;
        let is_degraded = self.state.has_degraded_quality;

        // Calculate flash state
        let show_flash = if needs_attention {
            let time = ui.ctx().input(|i| i.time);
            (time * 2.5) as i32 % 2 == 0
        } else {
            false
        };

        // Signal bars
        let signal_bars = self.state.overall_signal_bars();
        let bars_str = format_signal_bars(signal_bars);

        let color = if needs_attention {
            if show_flash {
                Color32::from_rgb(234, 67, 53)
            } else {
                Color32::from_rgba_unmultiplied(234, 67, 53, 150)
            }
        } else if is_degraded {
            Color32::from_rgb(251, 188, 4)
        } else {
            Color32::from_rgb(52, 168, 83)
        };

        // Draw the indicator
        ui.horizontal(|ui| {
            // Warning icon for poor connections
            if needs_attention && show_flash {
                ui.label(RichText::new("!").color(Color32::from_rgb(234, 67, 53)).size(12.0));
            }

            // Signal bars
            let response = ui.label(RichText::new(&bars_str).color(color).size(11.0));

            // Tooltip
            response.on_hover_text(self.state.summary());
        });

        // Request repaint for animation
        if needs_attention {
            ui.ctx().request_repaint();
        }

        needs_attention && show_flash
    }
}

/// Format signal bars as a text representation.
fn format_signal_bars(bars: u8) -> String {
    match bars {
        4 => "||||".to_string(),
        3 => "|||.".to_string(),
        2 => "||..".to_string(),
        1 => "|...".to_string(),
        _ => "....".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::sensors::quality::QualityMetrics;

    fn make_stats(device_id: &str, level: QualityLevel, score: u8) -> QualityStats {
        QualityStats {
            device_id: device_id.to_string(),
            score,
            level,
            metrics: QualityMetrics::default(),
            signal_bars: level.signal_bars(),
            uptime: Duration::from_secs(60),
            sample_count: 30,
        }
    }

    #[test]
    fn test_hud_connection_state_from_stats() {
        let stats = vec![
            make_stats("sensor_a", QualityLevel::Excellent, 90),
            make_stats("sensor_b", QualityLevel::Good, 75),
        ];

        let state = HudConnectionState::from_stats(stats);

        assert_eq!(state.connected_count, 2);
        assert!(!state.has_poor_quality);
        assert!(!state.has_degraded_quality);
        assert_eq!(state.worst_level, Some(QualityLevel::Good));
    }

    #[test]
    fn test_hud_connection_state_with_poor_quality() {
        let stats = vec![
            make_stats("sensor_a", QualityLevel::Excellent, 90),
            make_stats("sensor_b", QualityLevel::Poor, 20),
        ];

        let state = HudConnectionState::from_stats(stats);

        assert!(state.has_poor_quality);
        assert!(state.has_degraded_quality);
        assert_eq!(state.worst_level, Some(QualityLevel::Poor));
        assert!(state.should_animate_warning());
    }

    #[test]
    fn test_hud_connection_state_with_fair_quality() {
        let stats = vec![
            make_stats("sensor_a", QualityLevel::Excellent, 90),
            make_stats("sensor_b", QualityLevel::Fair, 50),
        ];

        let state = HudConnectionState::from_stats(stats);

        assert!(!state.has_poor_quality);
        assert!(state.has_degraded_quality);
        assert_eq!(state.worst_level, Some(QualityLevel::Fair));
        assert!(!state.should_animate_warning());
    }

    #[test]
    fn test_hud_connection_state_empty() {
        let state = HudConnectionState::from_stats(vec![]);

        assert_eq!(state.connected_count, 0);
        assert!(!state.should_show());
        assert_eq!(state.overall_signal_bars(), 0);
    }

    #[test]
    fn test_overall_signal_bars() {
        let stats = vec![
            make_stats("sensor_a", QualityLevel::Excellent, 90),
            make_stats("sensor_b", QualityLevel::Poor, 20),
        ];

        let state = HudConnectionState::from_stats(stats);

        // Should show bars for worst sensor (Poor = 1 bar)
        assert_eq!(state.overall_signal_bars(), 1);
    }

    #[test]
    fn test_format_signal_bars() {
        assert_eq!(format_signal_bars(4), "||||");
        assert_eq!(format_signal_bars(3), "|||.");
        assert_eq!(format_signal_bars(2), "||..");
        assert_eq!(format_signal_bars(1), "|...");
        assert_eq!(format_signal_bars(0), "....");
    }

    #[test]
    fn test_hud_colors_for_state() {
        // Poor quality - needs attention
        let colors = HudQualityColors::for_state(true, true);
        assert_eq!(colors.indicator, Color32::from_rgb(234, 67, 53));

        // Degraded but not poor
        let colors = HudQualityColors::for_state(false, true);
        assert_eq!(colors.indicator, Color32::from_rgb(251, 188, 4));

        // Good quality
        let colors = HudQualityColors::for_state(false, false);
        assert_eq!(colors.indicator, Color32::from_rgb(52, 168, 83));
    }

    #[test]
    fn test_hud_config_compact() {
        let config = HudConnectionQualityConfig::compact();

        assert!(config.min_width < HudConnectionQualityConfig::default().min_width);
        assert!(config.height < HudConnectionQualityConfig::default().height);
    }

    #[test]
    fn test_state_summary() {
        // No sensors
        let state = HudConnectionState::default();
        assert_eq!(state.summary(), "No sensors");

        // Good connection
        let stats = vec![make_stats("sensor_a", QualityLevel::Good, 75)];
        let state = HudConnectionState::from_stats(stats);
        assert!(state.summary().contains("connected"));

        // Poor connection
        let stats = vec![make_stats("sensor_a", QualityLevel::Poor, 20)];
        let state = HudConnectionState::from_stats(stats);
        assert!(state.summary().contains("Poor"));
    }

    #[test]
    fn test_indicator_default() {
        let indicator = HudConnectionQualityIndicator::new();
        assert!(!indicator.state.should_show());
    }

    #[test]
    fn test_inline_status_default() {
        let status = InlineHudSensorStatus::new();
        assert!(!status.state.should_show());
    }
}
