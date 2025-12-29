//! Power curve chart widget for power profiles.
//!
//! T055: Create power curve chart widget for power profile visualization.
//!
//! Similar to pdc_chart.rs but works with PowerProfile types and
//! supports reference curve overlays and lifetime comparisons.

use egui::{Color32, Response, RichText, Ui};
use egui_plot::{Line, Plot, PlotPoints};

use crate::power_profile::{
    duration_label, PowerProfile, ProfileComparer, ReferenceLevel, PROFILE_DURATIONS,
};

/// Configuration for power curve chart display.
#[derive(Debug, Clone)]
pub struct PowerCurveConfig {
    /// Chart height in pixels.
    pub height: f32,
    /// Whether to use logarithmic X-axis.
    pub log_x: bool,
    /// Whether to show data points.
    pub show_points: bool,
    /// Color for the primary curve.
    pub primary_color: Color32,
    /// Color for the secondary/lifetime curve.
    pub secondary_color: Color32,
    /// Color for reference curves.
    pub reference_color: Color32,
    /// Allow zoom/pan interaction.
    pub interactive: bool,
}

impl Default for PowerCurveConfig {
    fn default() -> Self {
        Self {
            height: 250.0,
            log_x: true,
            show_points: true,
            primary_color: Color32::from_rgb(100, 200, 255),
            secondary_color: Color32::from_rgb(100, 100, 200),
            reference_color: Color32::from_rgba_unmultiplied(255, 200, 100, 150),
            interactive: false,
        }
    }
}

/// Power curve chart widget for profile visualization.
pub struct PowerCurveChart<'a> {
    /// Primary profile (e.g., rolling 90-day).
    profile: &'a PowerProfile,
    /// Optional secondary profile (e.g., lifetime bests).
    secondary: Option<&'a PowerProfile>,
    /// Optional reference level for comparison.
    reference_level: Option<ReferenceLevel>,
    /// User weight for reference curve calculation.
    weight_kg: f64,
    /// Use female reference curves.
    use_female_reference: bool,
    /// Chart configuration.
    config: PowerCurveConfig,
}

impl<'a> PowerCurveChart<'a> {
    /// Create a new power curve chart.
    pub fn new(profile: &'a PowerProfile, weight_kg: f64) -> Self {
        Self {
            profile,
            secondary: None,
            reference_level: None,
            weight_kg,
            use_female_reference: false,
            config: PowerCurveConfig::default(),
        }
    }

    /// Add secondary profile (e.g., lifetime bests).
    pub fn with_secondary(mut self, profile: &'a PowerProfile) -> Self {
        self.secondary = Some(profile);
        self
    }

    /// Add reference curve overlay.
    pub fn with_reference(mut self, level: ReferenceLevel) -> Self {
        self.reference_level = Some(level);
        self
    }

    /// Set female reference curves.
    pub fn female_reference(mut self, female: bool) -> Self {
        self.use_female_reference = female;
        self
    }

    /// Set chart height.
    pub fn height(mut self, height: f32) -> Self {
        self.config.height = height;
        self
    }

    /// Enable interactive zoom/pan.
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.config.interactive = enabled;
        self
    }

    /// Set custom configuration.
    pub fn with_config(mut self, config: PowerCurveConfig) -> Self {
        self.config = config;
        self
    }

    /// Show the chart.
    pub fn show(self, ui: &mut Ui) -> Response {
        if self.profile.points.is_empty() {
            return ui.label(
                RichText::new("No power data. Complete rides to build your power profile.")
                    .color(Color32::from_gray(120)),
            );
        }

        // Build plot lines
        let primary_line =
            self.build_profile_line(self.profile, "Current", self.config.primary_color);

        let secondary_line = self
            .secondary
            .map(|p| self.build_profile_line(p, "Lifetime", self.config.secondary_color));

        let reference_line = self
            .reference_level
            .map(|level| self.build_reference_line(level));

        // Configure plot
        let mut plot = Plot::new("power_curve_chart")
            .height(self.config.height)
            .allow_drag(self.config.interactive)
            .allow_zoom(self.config.interactive)
            .allow_scroll(self.config.interactive)
            .show_x(true)
            .show_y(true)
            .x_axis_label("Duration")
            .y_axis_label("Power (W)")
            .label_formatter(|name, value| self.format_tooltip(name, value.x, value.y));

        if self.config.log_x {
            plot = plot.x_axis_formatter(|mark, _range| self.format_duration_axis(mark.value));
        }

        plot.show(ui, |plot_ui| {
            // Reference first (behind)
            if let Some(ref_line) = reference_line {
                plot_ui.line(ref_line);
            }

            // Secondary next
            if let Some(sec_line) = secondary_line {
                plot_ui.line(sec_line);
            }

            // Primary on top
            plot_ui.line(primary_line);
        })
        .response
    }

    /// Build a line from a power profile.
    fn build_profile_line(
        &self,
        profile: &PowerProfile,
        name: &str,
        color: Color32,
    ) -> Line<'static> {
        let coords: Vec<[f64; 2]> = profile
            .points
            .iter()
            .filter(|p| PROFILE_DURATIONS.contains(&p.duration_secs))
            .map(|p| {
                let x = if self.config.log_x {
                    (p.duration_secs as f64).ln()
                } else {
                    p.duration_secs as f64
                };
                [x, p.power_watts as f64]
            })
            .collect();

        Line::new(name.to_string(), PlotPoints::new(coords))
            .color(color)
            .width(2.0)
    }

    /// Build a reference curve line.
    fn build_reference_line(&self, level: ReferenceLevel) -> Line<'static> {
        let comparer = ProfileComparer::new(self.weight_kg, self.use_female_reference);
        let curve = comparer.reference_curve(level);
        let reference_points = curve.full_curve(self.weight_kg);

        let coords: Vec<[f64; 2]> = reference_points
            .iter()
            .map(|(duration, power)| {
                let x = if self.config.log_x {
                    (*duration as f64).ln()
                } else {
                    *duration as f64
                };
                [x, *power as f64]
            })
            .collect();

        Line::new(
            format!("Ref: {}", level.display_name()),
            PlotPoints::new(coords),
        )
        .color(self.config.reference_color)
        .width(1.5)
    }

    /// Format tooltip.
    fn format_tooltip(&self, name: &str, log_secs: f64, power: f64) -> String {
        let secs = if self.config.log_x {
            log_secs.exp() as u32
        } else {
            log_secs as u32
        };

        let wpk = power / self.weight_kg;

        if name.is_empty() {
            format!(
                "{}: {}W ({:.2} W/kg)",
                duration_label(secs),
                power.round() as u16,
                wpk
            )
        } else {
            format!(
                "{}\n{}: {}W ({:.2} W/kg)",
                name,
                duration_label(secs),
                power.round() as u16,
                wpk
            )
        }
    }

    /// Format duration axis label.
    fn format_duration_axis(&self, log_secs: f64) -> String {
        let secs = log_secs.exp() as u32;
        duration_label(secs)
    }
}

/// Compact power curve summary widget.
pub struct PowerCurveSummary<'a> {
    profile: &'a PowerProfile,
    weight_kg: f64,
}

impl<'a> PowerCurveSummary<'a> {
    /// Create a new summary widget.
    pub fn new(profile: &'a PowerProfile, weight_kg: f64) -> Self {
        Self { profile, weight_kg }
    }

    /// Show the summary.
    pub fn show(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.show_key_power(ui, "5s", 5);
            ui.separator();
            self.show_key_power(ui, "1 min", 60);
            ui.separator();
            self.show_key_power(ui, "5 min", 300);
            ui.separator();
            self.show_key_power(ui, "20 min", 1200);
        });
    }

    /// Show a single key power.
    fn show_key_power(&self, ui: &mut Ui, label: &str, duration_secs: u32) {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).weak().size(11.0));

            if let Some(power) = self.profile.power_at_duration(duration_secs) {
                let wpk = power as f64 / self.weight_kg;
                ui.label(RichText::new(format!("{}W", power)).strong());
                ui.label(RichText::new(format!("{:.2} W/kg", wpk)).weak().size(10.0));
            } else {
                ui.label(RichText::new("--").weak());
            }
        });
    }
}

/// Power improvement indicator widget.
pub struct PowerImprovement {
    /// Previous power value.
    pub previous: u16,
    /// Current power value.
    pub current: u16,
    /// Duration label.
    pub label: String,
}

impl PowerImprovement {
    /// Create a new improvement indicator.
    pub fn new(previous: u16, current: u16, label: impl Into<String>) -> Self {
        Self {
            previous,
            current,
            label: label.into(),
        }
    }

    /// Show the improvement.
    pub fn show(&self, ui: &mut Ui) {
        let diff = self.current as i32 - self.previous as i32;
        let pct = if self.previous > 0 {
            (diff as f64 / self.previous as f64) * 100.0
        } else {
            100.0
        };

        let color = if diff > 0 {
            Color32::from_rgb(100, 200, 100)
        } else if diff < 0 {
            Color32::from_rgb(200, 100, 100)
        } else {
            Color32::from_gray(150)
        };

        ui.horizontal(|ui| {
            ui.label(&self.label);
            ui.label(format!("{} → {}", self.previous, self.current));

            let arrow = if diff > 0 {
                "↑"
            } else if diff < 0 {
                "↓"
            } else {
                "="
            };
            ui.label(RichText::new(format!("{} {:+}W ({:+.1}%)", arrow, diff, pct)).color(color));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power_profile::{PowerProfilePoint, ProfileType};
    use uuid::Uuid;

    fn create_test_profile() -> PowerProfile {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);
        profile.update_point(PowerProfilePoint::new(5, 800));
        profile.update_point(PowerProfilePoint::new(60, 400));
        profile.update_point(PowerProfilePoint::new(300, 320));
        profile.update_point(PowerProfilePoint::new(1200, 280));
        profile
    }

    #[test]
    fn test_config_default() {
        let config = PowerCurveConfig::default();
        assert_eq!(config.height, 250.0);
        assert!(config.log_x);
        assert!(config.show_points);
    }

    #[test]
    fn test_chart_creation() {
        let profile = create_test_profile();
        let chart = PowerCurveChart::new(&profile, 70.0);
        assert!(!chart.profile.points.is_empty());
    }

    #[test]
    fn test_chart_with_secondary() {
        let profile = create_test_profile();
        let lifetime = create_test_profile();

        let chart = PowerCurveChart::new(&profile, 70.0).with_secondary(&lifetime);

        assert!(chart.secondary.is_some());
    }

    #[test]
    fn test_chart_with_reference() {
        let profile = create_test_profile();

        let chart = PowerCurveChart::new(&profile, 70.0).with_reference(ReferenceLevel::Trained);

        assert_eq!(chart.reference_level, Some(ReferenceLevel::Trained));
    }

    #[test]
    fn test_power_improvement() {
        let improvement = PowerImprovement::new(280, 300, "20 min");
        assert_eq!(improvement.previous, 280);
        assert_eq!(improvement.current, 300);
    }
}
