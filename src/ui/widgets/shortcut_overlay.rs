//! Keyboard shortcut help overlay.
//!
//! Displays available keyboard shortcuts when triggered by F1 or ? key.

use crate::input::keyboard::{KeyAction, KeyboardHandler};
use egui::{Align2, Area, Color32, Frame, Key, Order, RichText, Ui, Vec2};

/// Overlay for displaying keyboard shortcuts.
pub struct ShortcutOverlay {
    /// Whether the overlay is visible
    visible: bool,
}

impl Default for ShortcutOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcutOverlay {
    /// Create a new shortcut overlay.
    pub fn new() -> Self {
        Self { visible: false }
    }

    /// Show the overlay.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the overlay.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle the overlay visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if the overlay is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render the overlay.
    pub fn render(&mut self, ctx: &egui::Context, keyboard: &KeyboardHandler) {
        if !self.visible {
            return;
        }

        // Check for escape to close
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.visible = false;
            return;
        }

        // Semi-transparent background
        let screen_rect = ctx.input(|i| i.viewport_rect());
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            egui::Id::new("shortcut_overlay_bg"),
        ));
        painter.rect_filled(
            screen_rect,
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );

        // Centered modal
        Area::new(egui::Id::new("shortcut_overlay"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::popup(ui.style())
                    .fill(Color32::from_rgb(30, 30, 30))
                    .corner_radius(8.0)
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        self.render_content(ui, keyboard);
                    });
            });
    }

    /// Render the content of the overlay.
    fn render_content(&mut self, ui: &mut Ui, keyboard: &KeyboardHandler) {
        ui.vertical(|ui| {
            ui.heading(RichText::new("Keyboard Shortcuts").size(24.0).strong());
            ui.add_space(10.0);

            // Group shortcuts by category
            let categories = [
                (
                    "Navigation",
                    vec![
                        KeyAction::FocusNext,
                        KeyAction::FocusPrevious,
                        KeyAction::Activate,
                        KeyAction::Cancel,
                    ],
                ),
                (
                    "Display",
                    vec![
                        KeyAction::ToggleTvMode,
                        KeyAction::ToggleFlowMode,
                        KeyAction::CycleFlowMetric,
                    ],
                ),
                (
                    "Ride Control",
                    vec![
                        KeyAction::StartRide,
                        KeyAction::PauseRide,
                        KeyAction::EndRide,
                        KeyAction::SkipInterval,
                    ],
                ),
                (
                    "Other",
                    vec![
                        KeyAction::AnnounceMetrics,
                        KeyAction::ShowShortcuts,
                        KeyAction::OpenSettings,
                    ],
                ),
            ];

            for (category, actions) in categories {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(category)
                        .size(16.0)
                        .strong()
                        .color(Color32::from_rgb(100, 150, 255)),
                );
                ui.add_space(5.0);

                for action in actions {
                    if let Some(shortcut) = keyboard.shortcut_for(action) {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(shortcut.display())
                                    .monospace()
                                    .color(Color32::from_rgb(200, 200, 100)),
                            );
                            ui.add_space(20.0);
                            ui.label(action.description());
                        });
                    }
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Press").color(Color32::GRAY));
                ui.label(
                    RichText::new("Esc")
                        .monospace()
                        .color(Color32::from_rgb(200, 200, 100)),
                );
                ui.label(RichText::new("to close").color(Color32::GRAY));
            });
        });
    }
}
