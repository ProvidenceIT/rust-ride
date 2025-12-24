//! Home screen implementation.
//!
//! T043: Implement home screen with "Start Free Ride" and "Sensors" buttons

use egui::{Align, Layout, RichText, Ui, Vec2};

use super::Screen;

/// Home screen UI.
pub struct HomeScreen;

impl HomeScreen {
    /// Render the home screen and return the next screen if navigation requested.
    pub fn show(ui: &mut Ui) -> Option<Screen> {
        let mut next_screen = None;

        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // Title
            ui.label(RichText::new("RustRide").size(48.0).strong());
            ui.add_space(8.0);
            ui.label(
                RichText::new("Indoor Cycling Training")
                    .size(18.0)
                    .weak(),
            );

            ui.add_space(60.0);

            // Main action buttons
            let button_size = Vec2::new(280.0, 60.0);

            if ui
                .add_sized(
                    button_size,
                    egui::Button::new(RichText::new("Start Free Ride").size(20.0)),
                )
                .clicked()
            {
                next_screen = Some(Screen::Ride);
            }

            ui.add_space(16.0);

            if ui
                .add_sized(
                    button_size,
                    egui::Button::new(RichText::new("Workouts").size(20.0)),
                )
                .clicked()
            {
                next_screen = Some(Screen::WorkoutLibrary);
            }

            ui.add_space(40.0);

            // Secondary buttons in a row
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let small_button_size = Vec2::new(130.0, 44.0);

                ui.add_space((ui.available_width() - small_button_size.x * 3.0 - 32.0) / 2.0);

                if ui
                    .add_sized(
                        small_button_size,
                        egui::Button::new(RichText::new("Sensors").size(16.0)),
                    )
                    .clicked()
                {
                    next_screen = Some(Screen::SensorSetup);
                }

                ui.add_space(16.0);

                if ui
                    .add_sized(
                        small_button_size,
                        egui::Button::new(RichText::new("History").size(16.0)),
                    )
                    .clicked()
                {
                    next_screen = Some(Screen::RideHistory);
                }

                ui.add_space(16.0);

                if ui
                    .add_sized(
                        small_button_size,
                        egui::Button::new(RichText::new("Settings").size(16.0)),
                    )
                    .clicked()
                {
                    next_screen = Some(Screen::Settings);
                }
            });

            // Sensor status at bottom
            ui.add_space(40.0);
            ui.separator();
            ui.add_space(16.0);

            // TODO: Show connected sensor status
            ui.label(RichText::new("No sensors connected").weak());
        });

        next_screen
    }
}
