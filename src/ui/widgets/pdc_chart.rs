//! Power Duration Curve chart widget.
//!
//! T033: Create PDC chart widget using egui_plot
//! T034: Implement date range filter for PDC
//! T035: Implement hover tooltip showing power/duration

use egui::{Response, Ui};
use egui_plot::{Line, Plot, PlotPoints};

use crate::metrics::analytics::pdc::{PdcPoint, PowerDurationCurve};

/// Date range filter options for PDC display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdcDateFilter {
    /// Last 30 days
    Last30Days,
    /// Last 60 days
    Last60Days,
    /// Last 90 days
    #[default]
    Last90Days,
    /// All time
    AllTime,
}

impl PdcDateFilter {
    /// Get display label for the filter.
    pub fn label(&self) -> &'static str {
        match self {
            PdcDateFilter::Last30Days => "30 Days",
            PdcDateFilter::Last60Days => "60 Days",
            PdcDateFilter::Last90Days => "90 Days",
            PdcDateFilter::AllTime => "All Time",
        }
    }

    /// Get days count for the filter.
    pub fn days(&self) -> Option<u32> {
        match self {
            PdcDateFilter::Last30Days => Some(30),
            PdcDateFilter::Last60Days => Some(60),
            PdcDateFilter::Last90Days => Some(90),
            PdcDateFilter::AllTime => None,
        }
    }
}

/// Power Duration Curve chart widget.
pub struct PdcChart<'a> {
    /// The PDC data to display
    pdc: &'a PowerDurationCurve,
    /// Chart height
    height: f32,
    /// Whether to use logarithmic X-axis
    log_x: bool,
    /// Optional comparison PDC (e.g., previous period)
    comparison: Option<&'a PowerDurationCurve>,
    /// T139: Whether to allow pinch-zoom interaction
    allow_zoom: bool,
    /// T139: Whether to allow drag panning
    allow_drag: bool,
}

impl<'a> PdcChart<'a> {
    /// Create a new PDC chart.
    pub fn new(pdc: &'a PowerDurationCurve) -> Self {
        Self {
            pdc,
            height: 250.0,
            log_x: true,
            comparison: None,
            allow_zoom: false,
            allow_drag: false,
        }
    }

    /// T139: Enable touch pinch-zoom and scroll-wheel zoom.
    pub fn allow_zoom(mut self, allow: bool) -> Self {
        self.allow_zoom = allow;
        self
    }

    /// T139: Enable touch/mouse drag panning.
    pub fn allow_drag(mut self, allow: bool) -> Self {
        self.allow_drag = allow;
        self
    }

    /// T139: Enable all touch gestures (zoom + drag).
    pub fn touch_enabled(mut self, enabled: bool) -> Self {
        self.allow_zoom = enabled;
        self.allow_drag = enabled;
        self
    }

    /// Set chart height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set logarithmic X-axis.
    pub fn log_x(mut self, log_x: bool) -> Self {
        self.log_x = log_x;
        self
    }

    /// Set comparison PDC for overlay.
    pub fn with_comparison(mut self, comparison: &'a PowerDurationCurve) -> Self {
        self.comparison = Some(comparison);
        self
    }

    /// Show the chart in the UI.
    pub fn show(self, ui: &mut Ui) -> Response {
        let points = self.pdc.points();

        if points.is_empty() {
            return ui.label(
                "No power data available. Complete some rides to build your Power Duration Curve.",
            );
        }

        // Convert PDC points to plot points
        let plot_points = self.points_to_plot(points);
        let line = Line::new("Current PDC", plot_points); // Default blue color

        // T139: Configure touch/zoom interactions
        let mut plot = Plot::new("pdc_chart")
            .height(self.height)
            .allow_drag(self.allow_drag)
            .allow_zoom(self.allow_zoom)
            .allow_scroll(self.allow_zoom) // Enable scroll-wheel zoom if zoom is enabled
            .show_x(true)
            .show_y(true)
            .x_axis_label("Duration")
            .y_axis_label("Power (W)")
            .label_formatter(|name, value| {
                if name.is_empty() {
                    format_duration_power(value.x, value.y)
                } else {
                    format!("{}\n{}", name, format_duration_power(value.x, value.y))
                }
            });

        if self.log_x {
            plot = plot.x_axis_formatter(|mark, _range| format_duration_axis(mark.value));
        }

        plot.show(ui, |plot_ui| {
            plot_ui.line(line);

            // Show comparison if available
            if let Some(comp) = self.comparison {
                let comp_points = self.points_to_plot(comp.points());
                let comp_line = Line::new("Previous", comp_points); // Default gray
                plot_ui.line(comp_line);
            }
        })
        .response
    }

    /// Convert PDC points to plot points, optionally with log X.
    fn points_to_plot(&self, points: &[PdcPoint]) -> PlotPoints<'_> {
        let coords: Vec<[f64; 2]> = points
            .iter()
            .map(|p| {
                let x = if self.log_x {
                    (p.duration_secs as f64).ln()
                } else {
                    p.duration_secs as f64
                };
                [x, p.power_watts as f64]
            })
            .collect();
        PlotPoints::new(coords)
    }
}

/// Key power durations to highlight in the chart.
pub struct KeyPowers {
    /// 5-second power (neuromuscular)
    pub p5s: Option<u16>,
    /// 1-minute power (anaerobic)
    pub p1m: Option<u16>,
    /// 5-minute power (VO2max)
    pub p5m: Option<u16>,
    /// 20-minute power (threshold)
    pub p20m: Option<u16>,
    /// 60-minute power (endurance)
    pub p60m: Option<u16>,
}

impl KeyPowers {
    /// Extract key powers from a PDC.
    pub fn from_pdc(pdc: &PowerDurationCurve) -> Self {
        Self {
            p5s: pdc.power_at(5),
            p1m: pdc.power_at(60),
            p5m: pdc.power_at(300),
            p20m: pdc.power_at(1200),
            p60m: pdc.power_at(3600),
        }
    }

    /// Show key powers in a compact grid.
    pub fn show(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("5s");
                ui.strong(format_power(self.p5s));
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label("1min");
                ui.strong(format_power(self.p1m));
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label("5min");
                ui.strong(format_power(self.p5m));
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label("20min");
                ui.strong(format_power(self.p20m));
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label("60min");
                ui.strong(format_power(self.p60m));
            });
        });
    }
}

/// Format power value.
fn format_power(power: Option<u16>) -> String {
    power.map_or("--".to_string(), |p| format!("{}W", p))
}

/// Format duration from log scale for axis labels.
fn format_duration_axis(log_secs: f64) -> String {
    let secs = log_secs.exp();
    format_duration_secs(secs as u32)
}

/// Format duration and power for tooltip.
fn format_duration_power(log_secs: f64, power: f64) -> String {
    let secs = log_secs.exp();
    format!(
        "{}: {}W",
        format_duration_secs(secs as u32),
        power.round() as u16
    )
}

/// Format seconds to human-readable duration.
fn format_duration_secs(secs: u32) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remainder = secs % 60;
        if remainder == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m{}s", mins, remainder)
        }
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h{}m", hours, mins)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_secs() {
        assert_eq!(format_duration_secs(5), "5s");
        assert_eq!(format_duration_secs(30), "30s");
        assert_eq!(format_duration_secs(60), "1m");
        assert_eq!(format_duration_secs(90), "1m30s");
        assert_eq!(format_duration_secs(300), "5m");
        assert_eq!(format_duration_secs(3600), "1h");
        assert_eq!(format_duration_secs(5400), "1h30m");
    }

    #[test]
    fn test_key_powers_from_pdc() {
        let points = vec![
            PdcPoint {
                duration_secs: 5,
                power_watts: 1000,
            },
            PdcPoint {
                duration_secs: 60,
                power_watts: 500,
            },
            PdcPoint {
                duration_secs: 300,
                power_watts: 350,
            },
            PdcPoint {
                duration_secs: 1200,
                power_watts: 280,
            },
            PdcPoint {
                duration_secs: 3600,
                power_watts: 240,
            },
        ];
        let pdc = PowerDurationCurve::from_points(points);
        let keys = KeyPowers::from_pdc(&pdc);

        assert_eq!(keys.p5s, Some(1000));
        assert_eq!(keys.p1m, Some(500));
        assert_eq!(keys.p5m, Some(350));
        assert_eq!(keys.p20m, Some(280));
        assert_eq!(keys.p60m, Some(240));
    }

    #[test]
    fn test_date_filter() {
        assert_eq!(PdcDateFilter::Last30Days.days(), Some(30));
        assert_eq!(PdcDateFilter::AllTime.days(), None);
    }
}
