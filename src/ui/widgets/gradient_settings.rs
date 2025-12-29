//! Gradient settings UI panel widget.
//!
//! T027: Create gradient settings UI panel for adjusting simulation parameters.

use egui::{Color32, RichText, Slider, Ui};

use crate::gradient::GradientSettings;

/// Actions that can be triggered from the gradient settings panel.
#[derive(Debug, Clone, PartialEq)]
pub enum GradientSettingsAction {
    /// Settings were modified
    SettingsChanged(GradientSettings),
    /// User requested to load a GPX route
    LoadRoute,
    /// User requested to clear the current route
    ClearRoute,
}

/// Response from rendering the gradient settings panel.
#[derive(Debug, Default)]
pub struct GradientSettingsResponse {
    /// Action triggered by user interaction
    pub action: Option<GradientSettingsAction>,
}

/// Gradient settings panel for configuring simulation parameters.
pub struct GradientSettingsPanel;

impl GradientSettingsPanel {
    /// Render the gradient settings panel.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `settings` - Mutable reference to the gradient settings
    /// * `route_loaded` - Whether a route is currently loaded
    /// * `route_name` - Optional name of the loaded route
    ///
    /// # Returns
    /// Response containing any triggered action
    pub fn show(
        ui: &mut Ui,
        settings: &mut GradientSettings,
        route_loaded: bool,
        route_name: Option<&str>,
    ) -> GradientSettingsResponse {
        let mut response = GradientSettingsResponse::default();
        let original_settings = settings.clone();

        ui.heading("Gradient Simulation");
        ui.separator();

        // Route status section
        ui.horizontal(|ui| {
            ui.label("Route:");
            if route_loaded {
                if let Some(name) = route_name {
                    ui.label(RichText::new(name).strong().color(Color32::LIGHT_GREEN));
                } else {
                    ui.label(RichText::new("Loaded").color(Color32::LIGHT_GREEN));
                }
                if ui.button("Clear").clicked() {
                    response.action = Some(GradientSettingsAction::ClearRoute);
                }
            } else {
                ui.label(RichText::new("None").weak());
                if ui.button("Load GPX...").clicked() {
                    response.action = Some(GradientSettingsAction::LoadRoute);
                }
            }
        });

        ui.add_space(8.0);

        // Difficulty slider (0-100%)
        ui.horizontal(|ui| {
            ui.label("Trainer Difficulty:");
            let difficulty_pct = (settings.difficulty * 100.0).round() as i32;
            ui.label(RichText::new(format!("{}%", difficulty_pct)).strong());
        });
        let mut difficulty_pct = settings.difficulty * 100.0;
        if ui
            .add(Slider::new(&mut difficulty_pct, 0.0..=100.0).suffix("%"))
            .changed()
        {
            settings.difficulty = difficulty_pct / 100.0;
        }
        ui.label(
            RichText::new("Higher = harder climbs")
                .weak()
                .size(11.0),
        );

        ui.add_space(8.0);

        // Max gradient slider
        ui.horizontal(|ui| {
            ui.label("Max Uphill Gradient:");
            ui.label(RichText::new(format!("{:.0}%", settings.max_gradient)).strong());
        });
        ui.add(Slider::new(&mut settings.max_gradient, 5.0..=25.0).suffix("%"));

        ui.add_space(4.0);

        // Min gradient slider
        ui.horizontal(|ui| {
            ui.label("Max Downhill Gradient:");
            ui.label(RichText::new(format!("{:.0}%", settings.min_gradient)).strong());
        });
        ui.add(Slider::new(&mut settings.min_gradient, -25.0..=-5.0).suffix("%"));

        ui.add_space(8.0);

        // Smoothing slider
        ui.horizontal(|ui| {
            ui.label("Gradient Smoothing:");
            ui.label(RichText::new(format!("{}s", settings.smoothing_secs)).strong());
        });
        let mut smoothing = settings.smoothing_secs as f32;
        if ui
            .add(Slider::new(&mut smoothing, 1.0..=10.0).suffix("s"))
            .changed()
        {
            settings.smoothing_secs = smoothing.round() as u8;
        }
        ui.label(
            RichText::new("Smooths sudden gradient changes")
                .weak()
                .size(11.0),
        );

        ui.add_space(8.0);

        // Rolling resistance (advanced)
        ui.collapsing("Advanced", |ui| {
            ui.horizontal(|ui| {
                ui.label("Rolling Resistance (Crr):");
                ui.label(RichText::new(format!("{:.4}", settings.rolling_resistance)).strong());
            });
            ui.add(Slider::new(&mut settings.rolling_resistance, 0.002..=0.008).step_by(0.0005));
            ui.label(
                RichText::new("Lower = smoother roads, Higher = rougher terrain")
                    .weak()
                    .size(11.0),
            );
        });

        // Check if settings changed
        if (settings.difficulty != original_settings.difficulty
            || settings.max_gradient != original_settings.max_gradient
            || settings.min_gradient != original_settings.min_gradient
            || settings.smoothing_secs != original_settings.smoothing_secs
            || (settings.rolling_resistance - original_settings.rolling_resistance).abs() > 0.00001)
            && response.action.is_none() {
                response.action = Some(GradientSettingsAction::SettingsChanged(settings.clone()));
            }

        response
    }

    /// Render a compact difficulty slider for the ride screen.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `difficulty` - Current difficulty (0.0-1.0)
    ///
    /// # Returns
    /// New difficulty value if changed
    pub fn compact_difficulty(ui: &mut Ui, difficulty: f32) -> Option<f32> {
        let mut new_difficulty = difficulty;
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Difficulty:");
            let pct = (difficulty * 100.0).round() as i32;
            ui.label(RichText::new(format!("{}%", pct)).strong());

            let mut difficulty_pct = difficulty * 100.0;
            if ui
                .add(Slider::new(&mut difficulty_pct, 0.0..=100.0).show_value(false))
                .changed()
            {
                new_difficulty = difficulty_pct / 100.0;
                changed = true;
            }
        });

        if changed {
            Some(new_difficulty)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_settings_action_eq() {
        let settings = GradientSettings::default();
        let action1 = GradientSettingsAction::SettingsChanged(settings.clone());
        let action2 = GradientSettingsAction::SettingsChanged(settings);
        assert_eq!(action1, action2);

        assert_eq!(GradientSettingsAction::LoadRoute, GradientSettingsAction::LoadRoute);
        assert_eq!(GradientSettingsAction::ClearRoute, GradientSettingsAction::ClearRoute);
    }

    #[test]
    fn test_gradient_settings_response_default() {
        let response = GradientSettingsResponse::default();
        assert!(response.action.is_none());
    }
}
