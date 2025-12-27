//! SmO2 display widget for muscle oxygen monitoring.
//!
//! T116: SmO2 percentage gauge and trend line display

use egui::{Align, Color32, Layout, Rect, RichText, Stroke, Ui, Vec2};

use crate::sensors::smo2::{MuscleLocation, SmO2Reading};

/// SmO2 display size variants.
#[derive(Debug, Clone, Copy, Default)]
pub enum SmO2WidgetSize {
    /// Compact display (just percentage)
    Compact,
    /// Standard display with gauge
    #[default]
    Standard,
    /// Detailed display with trend
    Detailed,
}

/// Widget for displaying SmO2 reading.
pub struct SmO2Display<'a> {
    /// Current reading
    reading: &'a SmO2Reading,
    /// Display size
    size: SmO2WidgetSize,
    /// Recent readings for trend (optional)
    history: Option<&'a [SmO2Reading]>,
    /// Show location label
    show_location: bool,
}

impl<'a> SmO2Display<'a> {
    /// Create a new SmO2 display widget.
    pub fn new(reading: &'a SmO2Reading) -> Self {
        Self {
            reading,
            size: SmO2WidgetSize::default(),
            history: None,
            show_location: true,
        }
    }

    /// Set display size.
    pub fn with_size(mut self, size: SmO2WidgetSize) -> Self {
        self.size = size;
        self
    }

    /// Set history for trend display.
    pub fn with_history(mut self, history: &'a [SmO2Reading]) -> Self {
        self.history = Some(history);
        self
    }

    /// Set whether to show location label.
    pub fn with_location_label(mut self, show: bool) -> Self {
        self.show_location = show;
        self
    }

    /// Get color for SmO2 value.
    fn smo2_color(&self) -> Color32 {
        let smo2 = self.reading.smo2_percent;
        if smo2 >= 70.0 {
            // High oxygenation - green
            Color32::from_rgb(52, 168, 83)
        } else if smo2 >= 50.0 {
            // Moderate - yellow
            Color32::from_rgb(251, 188, 4)
        } else if smo2 >= 35.0 {
            // Low - orange
            Color32::from_rgb(255, 109, 0)
        } else {
            // Very low - red (desaturated)
            Color32::from_rgb(234, 67, 53)
        }
    }

    /// Get zone description for SmO2 value.
    fn zone_description(&self) -> &'static str {
        let smo2 = self.reading.smo2_percent;
        if smo2 >= 70.0 {
            "Recovery"
        } else if smo2 >= 55.0 {
            "Aerobic"
        } else if smo2 >= 40.0 {
            "Threshold"
        } else {
            "Anaerobic"
        }
    }

    /// Render compact display.
    fn show_compact(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{:.0}%", self.reading.smo2_percent))
                    .size(18.0)
                    .color(self.smo2_color())
                    .strong(),
            );
            if self.show_location {
                ui.label(
                    RichText::new(self.reading.location.short_name())
                        .size(10.0)
                        .weak(),
                );
            }
        });
    }

    /// Render standard display with gauge.
    fn show_standard(&self, ui: &mut Ui) {
        let min_size = Vec2::new(120.0, 100.0);

        egui::Frame::new()
            .inner_margin(12.0)
            .corner_radius(8.0)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .show(ui, |ui| {
                ui.set_min_size(min_size);

                ui.vertical(|ui| {
                    // Location header
                    if self.show_location {
                        ui.label(
                            RichText::new(self.reading.location.display_name())
                                .size(12.0)
                                .weak(),
                        );
                        ui.add_space(4.0);
                    }

                    // Main percentage
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:.0}", self.reading.smo2_percent))
                                .size(36.0)
                                .color(self.smo2_color())
                                .strong(),
                        );
                        ui.label(RichText::new("%").size(14.0).color(self.smo2_color()));
                    });

                    // Zone description
                    ui.label(RichText::new(self.zone_description()).size(11.0).weak());

                    // Simple gauge bar
                    let gauge_height = 8.0;
                    let gauge_width = ui.available_width();
                    let (rect, _response) = ui.allocate_exact_size(
                        Vec2::new(gauge_width, gauge_height),
                        egui::Sense::hover(),
                    );

                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();

                        // Background
                        painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);

                        // Fill based on percentage
                        let fill_width = rect.width() * (self.reading.smo2_percent / 100.0);
                        let fill_rect =
                            Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
                        painter.rect_filled(fill_rect, 4.0, self.smo2_color());
                    }

                    // Signal quality if available
                    if let Some(quality) = self.reading.signal_quality {
                        ui.add_space(4.0);
                        let quality_color = if quality >= 80 {
                            Color32::from_rgb(52, 168, 83)
                        } else if quality >= 50 {
                            Color32::from_rgb(251, 188, 4)
                        } else {
                            Color32::from_rgb(234, 67, 53)
                        };
                        ui.label(
                            RichText::new(format!("Signal: {}%", quality))
                                .size(10.0)
                                .color(quality_color),
                        );
                    }
                });
            });
    }

    /// Render detailed display with trend line.
    fn show_detailed(&self, ui: &mut Ui) {
        let min_size = Vec2::new(200.0, 160.0);

        egui::Frame::new()
            .inner_margin(12.0)
            .corner_radius(8.0)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .show(ui, |ui| {
                ui.set_min_size(min_size);

                ui.vertical(|ui| {
                    // Header with location
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("SmO2").size(14.0).strong());
                        if self.show_location {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(self.reading.location.display_name())
                                        .size(12.0)
                                        .weak(),
                                );
                            });
                        }
                    });

                    ui.add_space(8.0);

                    // Main value
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:.1}%", self.reading.smo2_percent))
                                .size(32.0)
                                .color(self.smo2_color())
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(self.zone_description())
                                    .size(12.0)
                                    .color(self.smo2_color()),
                            );
                            if let Some(thb) = self.reading.thb {
                                ui.label(
                                    RichText::new(format!("THb: {:.1} g/dL", thb))
                                        .size(10.0)
                                        .weak(),
                                );
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Trend chart if history available
                    if let Some(history) = self.history {
                        if !history.is_empty() {
                            self.render_trend_chart(ui, history);
                        }
                    }

                    // Signal quality
                    if let Some(quality) = self.reading.signal_quality {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Signal:").size(10.0).weak());
                            let bars = (quality as f32 / 25.0).ceil() as i32;
                            for i in 0..4 {
                                let bar_color = if i < bars {
                                    if quality >= 80 {
                                        Color32::from_rgb(52, 168, 83)
                                    } else if quality >= 50 {
                                        Color32::from_rgb(251, 188, 4)
                                    } else {
                                        Color32::from_rgb(234, 67, 53)
                                    }
                                } else {
                                    ui.visuals().faint_bg_color
                                };
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(4.0, 8.0 + (i as f32 * 2.0)),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 1.0, bar_color);
                                ui.add_space(1.0);
                            }
                        });
                    }
                });
            });
    }

    /// Render a simple trend chart.
    fn render_trend_chart(&self, ui: &mut Ui, history: &[SmO2Reading]) {
        let chart_height = 40.0;
        let chart_width = ui.available_width();

        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(chart_width, chart_height), egui::Sense::hover());

        if ui.is_rect_visible(rect) && history.len() > 1 {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);

            // Find min/max for scaling
            let values: Vec<f32> = history.iter().map(|r| r.smo2_percent).collect();
            let min_val = values.iter().cloned().fold(f32::MAX, f32::min).max(0.0);
            let max_val = values.iter().cloned().fold(f32::MIN, f32::max).min(100.0);
            let range = (max_val - min_val).max(10.0); // At least 10% range

            // Draw line
            let points: Vec<egui::Pos2> = values
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let x = rect.left() + (i as f32 / (values.len() - 1) as f32) * rect.width();
                    let y = rect.bottom() - ((v - min_val) / range) * rect.height();
                    egui::Pos2::new(x, y)
                })
                .collect();

            if points.len() >= 2 {
                for window in points.windows(2) {
                    painter
                        .line_segment([window[0], window[1]], Stroke::new(2.0, self.smo2_color()));
                }
            }
        }
    }

    /// Render the widget.
    pub fn show(self, ui: &mut Ui) {
        match self.size {
            SmO2WidgetSize::Compact => self.show_compact(ui),
            SmO2WidgetSize::Standard => self.show_standard(ui),
            SmO2WidgetSize::Detailed => self.show_detailed(ui),
        }
    }
}

/// Placeholder widget when no SmO2 sensor connected.
pub struct SmO2Placeholder {
    message: String,
}

impl SmO2Placeholder {
    /// Create a "not connected" placeholder.
    pub fn not_connected() -> Self {
        Self {
            message: "No SmO2 sensor".to_string(),
        }
    }

    /// Create a "connecting" placeholder.
    pub fn connecting() -> Self {
        Self {
            message: "Connecting...".to_string(),
        }
    }

    /// Render the placeholder.
    pub fn show(self, ui: &mut Ui) {
        ui.label(RichText::new(self.message).size(12.0).weak());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_reading() -> SmO2Reading {
        SmO2Reading::new(Uuid::new_v4(), MuscleLocation::LeftQuad, 65.0)
            .with_thb(12.5)
            .with_signal_quality(85)
    }

    #[test]
    fn test_smo2_display_creation() {
        let reading = sample_reading();
        let display = SmO2Display::new(&reading);
        assert!(display.show_location);
    }

    #[test]
    fn test_smo2_color_zones() {
        let high = SmO2Reading::new(Uuid::new_v4(), MuscleLocation::LeftQuad, 75.0);
        let display_high = SmO2Display::new(&high);
        assert_eq!(display_high.smo2_color(), Color32::from_rgb(52, 168, 83));

        let low = SmO2Reading::new(Uuid::new_v4(), MuscleLocation::LeftQuad, 30.0);
        let display_low = SmO2Display::new(&low);
        assert_eq!(display_low.smo2_color(), Color32::from_rgb(234, 67, 53));
    }

    #[test]
    fn test_zone_descriptions() {
        let recovery = SmO2Reading::new(Uuid::new_v4(), MuscleLocation::LeftQuad, 72.0);
        assert_eq!(SmO2Display::new(&recovery).zone_description(), "Recovery");

        let threshold = SmO2Reading::new(Uuid::new_v4(), MuscleLocation::LeftQuad, 45.0);
        assert_eq!(SmO2Display::new(&threshold).zone_description(), "Threshold");
    }
}
