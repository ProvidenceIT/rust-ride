//! Metric display widget for showing training metrics.
//!
//! T049: Implement metric display widget (large readable numbers)

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

/// A widget for displaying a single training metric.
pub struct MetricDisplay<'a> {
    /// The metric value to display
    value: String,
    /// The unit label
    unit: &'a str,
    /// The metric name/label
    label: &'a str,
    /// Optional zone color
    zone_color: Option<Color32>,
    /// Size multiplier
    size: MetricSize,
}

/// Size variants for metric display.
#[derive(Debug, Clone, Copy, Default)]
pub enum MetricSize {
    /// Small metric (secondary display)
    Small,
    /// Medium metric (standard display)
    #[default]
    Medium,
    /// Large metric (primary focus)
    Large,
}

impl MetricSize {
    fn value_size(&self) -> f32 {
        match self {
            MetricSize::Small => 28.0,
            MetricSize::Medium => 42.0,
            MetricSize::Large => 64.0,
        }
    }

    fn unit_size(&self) -> f32 {
        match self {
            MetricSize::Small => 12.0,
            MetricSize::Medium => 14.0,
            MetricSize::Large => 18.0,
        }
    }

    fn label_size(&self) -> f32 {
        match self {
            MetricSize::Small => 11.0,
            MetricSize::Medium => 13.0,
            MetricSize::Large => 15.0,
        }
    }
}

impl<'a> MetricDisplay<'a> {
    /// Create a new metric display.
    pub fn new(value: impl Into<String>, unit: &'a str, label: &'a str) -> Self {
        Self {
            value: value.into(),
            unit,
            label,
            zone_color: None,
            size: MetricSize::default(),
        }
    }

    /// Create a metric display for power in watts.
    pub fn power(watts: Option<u16>) -> Self {
        let value = watts
            .map(|w| w.to_string())
            .unwrap_or_else(|| "--".to_string());
        Self::new(value, "W", "Power")
    }

    /// Create a metric display for heart rate.
    pub fn heart_rate(bpm: Option<u8>) -> Self {
        let value = bpm
            .map(|b| b.to_string())
            .unwrap_or_else(|| "--".to_string());
        Self::new(value, "bpm", "Heart Rate")
    }

    /// Create a metric display for cadence.
    pub fn cadence(rpm: Option<u8>) -> Self {
        let value = rpm
            .map(|r| r.to_string())
            .unwrap_or_else(|| "--".to_string());
        Self::new(value, "rpm", "Cadence")
    }

    /// Create a metric display for speed.
    pub fn speed(speed_kmh: Option<f32>) -> Self {
        let value = speed_kmh
            .map(|s| format!("{:.1}", s))
            .unwrap_or_else(|| "--".to_string());
        Self::new(value, "km/h", "Speed")
    }

    /// Create a metric display for distance.
    pub fn distance(meters: f64) -> Self {
        let km = meters / 1000.0;
        Self::new(format!("{:.2}", km), "km", "Distance")
    }

    /// Create a metric display for duration.
    pub fn duration(seconds: u32) -> Self {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        let value = if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{}:{:02}", minutes, secs)
        };

        Self::new(value, "", "Time")
    }

    /// Set a zone color for the metric.
    pub fn with_zone_color(mut self, color: Color32) -> Self {
        self.zone_color = Some(color);
        self
    }

    /// Set the display size.
    pub fn with_size(mut self, size: MetricSize) -> Self {
        self.size = size;
        self
    }

    /// Render the metric display.
    pub fn show(self, ui: &mut Ui) {
        let min_size = match self.size {
            MetricSize::Small => Vec2::new(80.0, 60.0),
            MetricSize::Medium => Vec2::new(120.0, 80.0),
            MetricSize::Large => Vec2::new(180.0, 100.0),
        };

        egui::Frame::new().inner_margin(8.0).show(ui, |ui| {
            ui.set_min_size(min_size);

            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                // Label
                ui.label(
                    RichText::new(self.label)
                        .size(self.size.label_size())
                        .weak(),
                );

                ui.add_space(4.0);

                // Value with optional zone color
                let value_text = RichText::new(&self.value)
                    .size(self.size.value_size())
                    .strong();

                let value_text = if let Some(color) = self.zone_color {
                    value_text.color(color)
                } else {
                    value_text
                };

                // Value and unit on same line
                ui.horizontal(|ui| {
                    ui.label(value_text);
                    if !self.unit.is_empty() {
                        ui.label(RichText::new(self.unit).size(self.size.unit_size()).weak());
                    }
                });
            });
        });
    }
}
