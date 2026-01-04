//! Garmin Connect settings screen.
//!
//! Provides UI for managing Garmin Connect integration:
//! - Connection status display
//! - Connect/disconnect buttons
//! - User profile display when connected
//! - Auto-sync toggle and settings

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::sync::garmin::GarminUserProfile;
use crate::integrations::sync::PlatformConfig;

use super::Screen;

/// Garmin Connect brand color (official Garmin blue)
const GARMIN_BLUE: Color32 = Color32::from_rgb(0, 118, 206);

/// Connection state for Garmin Connect
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GarminConnectionState {
    /// Not connected - show connect button
    Disconnected,
    /// OAuth flow in progress
    Connecting,
    /// Connected to Garmin Connect
    Connected,
    /// Disconnecting in progress
    Disconnecting,
    /// Connection error
    Error(String),
}

impl Default for GarminConnectionState {
    fn default() -> Self {
        GarminConnectionState::Disconnected
    }
}

/// Actions that can result from the Garmin settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum GarminSettingsAction {
    /// No action
    None,
    /// Navigate back
    Back,
    /// Start OAuth connection flow
    Connect,
    /// Disconnect from Garmin Connect
    Disconnect,
    /// Toggle auto-sync setting
    ToggleAutoSync(bool),
}

/// Garmin Connect settings screen state.
#[derive(Default)]
pub struct GarminSettingsScreen {
    /// Current connection state
    pub connection_state: GarminConnectionState,
    /// User profile (when connected)
    pub user_profile: Option<GarminUserProfile>,
    /// Platform configuration
    pub config: PlatformConfig,
    /// Number of pending uploads
    pub pending_uploads: usize,
    /// Last sync timestamp
    pub last_sync: Option<String>,
}

impl GarminSettingsScreen {
    /// Create a new Garmin settings screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connection state.
    pub fn set_connection_state(&mut self, state: GarminConnectionState) {
        self.connection_state = state;
    }

    /// Set the user profile (when connected).
    pub fn set_user_profile(&mut self, profile: Option<GarminUserProfile>) {
        self.user_profile = profile;
    }

    /// Set the platform configuration.
    pub fn set_config(&mut self, config: PlatformConfig) {
        self.config = config;
    }

    /// Set the number of pending uploads.
    pub fn set_pending_uploads(&mut self, count: usize) {
        self.pending_uploads = count;
    }

    /// Set the last sync timestamp.
    pub fn set_last_sync(&mut self, timestamp: Option<String>) {
        self.last_sync = timestamp;
    }

    /// Check if currently connected.
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_state, GarminConnectionState::Connected)
    }

    /// Render the Garmin settings screen.
    pub fn show(&mut self, ui: &mut Ui) -> (Option<Screen>, GarminSettingsAction) {
        let mut next_screen = None;
        let mut action = GarminSettingsAction::None;

        ui.vertical(|ui| {
            // Header with back button
            ui.horizontal(|ui| {
                if ui.button("<- Back").clicked() {
                    next_screen = Some(Screen::Settings);
                    action = GarminSettingsAction::Back;
                }
                ui.add_space(8.0);
                ui.heading(
                    RichText::new("Garmin Connect")
                        .size(24.0)
                        .strong()
                        .color(GARMIN_BLUE),
                );
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Main content
            match &self.connection_state {
                GarminConnectionState::Disconnected => {
                    let connect_action = self.render_disconnected_state(ui);
                    if connect_action == GarminSettingsAction::Connect {
                        action = connect_action;
                    }
                }
                GarminConnectionState::Connecting => {
                    self.render_connecting_state(ui);
                }
                GarminConnectionState::Connected => {
                    if let Some(connected_action) = self.render_connected_state(ui) {
                        action = connected_action;
                    }
                }
                GarminConnectionState::Disconnecting => {
                    self.render_disconnecting_state(ui);
                }
                GarminConnectionState::Error(error) => {
                    let error_action = self.render_error_state(ui, error.clone());
                    if error_action == GarminSettingsAction::Connect {
                        action = error_action;
                    }
                }
            }
        });

        (next_screen, action)
    }

    /// Render the disconnected state with connect button.
    fn render_disconnected_state(&self, ui: &mut Ui) -> GarminSettingsAction {
        let mut action = GarminSettingsAction::None;

        // Information panel
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    // Garmin logo/icon placeholder
                    ui.label(RichText::new("G").size(64.0).color(GARMIN_BLUE).strong());

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("Connect your Garmin account")
                            .size(18.0)
                            .strong(),
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(
                            "Automatically upload your rides to Garmin Connect after each session.",
                        )
                        .weak(),
                    );

                    ui.add_space(24.0);

                    // Connect button
                    let connect_button = egui::Button::new(
                        RichText::new("Connect to Garmin")
                            .size(16.0)
                            .color(Color32::WHITE),
                    )
                    .fill(GARMIN_BLUE)
                    .min_size(Vec2::new(200.0, 44.0));

                    if ui.add(connect_button).clicked() {
                        action = GarminSettingsAction::Connect;
                    }

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("Opens your browser for secure OAuth authorization")
                            .weak()
                            .small(),
                    );
                });
            });

        ui.add_space(24.0);

        // Features list
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.label(RichText::new("Features").size(14.0).strong());
                ui.add_space(8.0);

                for feature in &[
                    "Automatic ride upload after each session",
                    "Native FIT file format for complete data preservation",
                    "Upload pending rides when back online",
                    "Sync with Garmin's training ecosystem",
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("*").color(GARMIN_BLUE));
                        ui.label(*feature);
                    });
                }
            });

        action
    }

    /// Render the connecting state with spinner.
    fn render_connecting_state(&self, ui: &mut Ui) {
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(24.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.add_space(16.0);
                    ui.label(RichText::new("Connecting to Garmin Connect...").size(16.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Complete authorization in your browser")
                            .weak(),
                    );
                });
            });
    }

    /// Render the connected state with user profile.
    fn render_connected_state(&mut self, ui: &mut Ui) -> Option<GarminSettingsAction> {
        let mut action = None;

        // User profile section
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Profile avatar placeholder
                    egui::Frame::new()
                        .fill(Color32::from_gray(60))
                        .corner_radius(24.0)
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(48.0, 48.0));
                            ui.centered_and_justified(|ui| {
                                if let Some(ref profile) = self.user_profile {
                                    // Display initials as avatar placeholder
                                    let initials = profile
                                        .readable_name()
                                        .chars()
                                        .take(2)
                                        .collect::<String>()
                                        .to_uppercase();
                                    ui.label(
                                        RichText::new(initials)
                                            .size(18.0)
                                            .color(Color32::WHITE),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("?")
                                            .size(18.0)
                                            .color(Color32::WHITE),
                                    );
                                }
                            });
                        });

                    ui.add_space(16.0);

                    ui.vertical(|ui| {
                        if let Some(ref profile) = self.user_profile {
                            // User display name
                            ui.label(
                                RichText::new(profile.readable_name())
                                    .size(18.0)
                                    .strong(),
                            );

                            // Display name/username
                            ui.label(
                                RichText::new(format!("@{}", profile.display_name))
                                    .color(Color32::GRAY),
                            );
                        } else {
                            ui.label(RichText::new("Connected").size(18.0).strong());
                        }

                        // Connection status
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("*")
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                            ui.label(
                                RichText::new("Connected to Garmin Connect")
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        });
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Disconnect button
                        if ui
                            .button("Disconnect")
                            .on_hover_text("Remove authorization from Garmin Connect")
                            .clicked()
                        {
                            action = Some(GarminSettingsAction::Disconnect);
                        }
                    });
                });
            });

        ui.add_space(16.0);

        // Sync settings section
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.label(RichText::new("Sync Settings").size(14.0).strong());
                ui.add_space(12.0);

                // Auto-sync toggle
                ui.horizontal(|ui| {
                    let mut auto_sync = self.config.auto_sync;
                    if ui
                        .checkbox(&mut auto_sync, "Auto-sync rides")
                        .on_hover_text("Automatically upload rides after each session")
                        .changed()
                    {
                        self.config.auto_sync = auto_sync;
                        action = Some(GarminSettingsAction::ToggleAutoSync(auto_sync));
                    }
                });

                ui.add_space(8.0);

                // Status information
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Pending uploads:").weak());
                    if self.pending_uploads > 0 {
                        ui.label(
                            RichText::new(format!("{}", self.pending_uploads))
                                .color(Color32::from_rgb(251, 188, 4)),
                        );
                    } else {
                        ui.label(RichText::new("0").weak());
                    }
                });

                if let Some(ref last_sync) = self.last_sync {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Last sync:").weak());
                        ui.label(RichText::new(last_sync).weak());
                    });
                }
            });

        ui.add_space(16.0);

        // Garmin Connect profile link
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.label(RichText::new("View your profile on Garmin Connect").weak());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(ref profile) = self.user_profile {
                            let profile_url = format!(
                                "https://connect.garmin.com/modern/profile/{}",
                                profile.display_name
                            );
                            if ui.small_button("Open").clicked() {
                                if let Err(e) = open::that(&profile_url) {
                                    tracing::warn!("Failed to open Garmin profile: {}", e);
                                }
                            }
                        }
                    });
                });
            });

        action
    }

    /// Render the disconnecting state with spinner.
    fn render_disconnecting_state(&self, ui: &mut Ui) {
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(24.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.add_space(16.0);
                    ui.label(RichText::new("Disconnecting from Garmin Connect...").size(16.0));
                });
            });
    }

    /// Render the error state with retry button.
    fn render_error_state(&self, ui: &mut Ui, error: String) -> GarminSettingsAction {
        let mut action = GarminSettingsAction::None;

        egui::Frame::new()
            .fill(Color32::from_rgb(40, 30, 30))
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("!")
                            .size(48.0)
                            .color(Color32::from_rgb(234, 67, 53)),
                    );

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("Connection Error")
                            .size(18.0)
                            .strong()
                            .color(Color32::from_rgb(234, 67, 53)),
                    );

                    ui.add_space(8.0);

                    ui.label(RichText::new(&error).weak());

                    ui.add_space(24.0);

                    // Retry button
                    let retry_button = egui::Button::new(
                        RichText::new("Try Again").size(14.0).color(Color32::WHITE),
                    )
                    .fill(GARMIN_BLUE)
                    .min_size(Vec2::new(120.0, 36.0));

                    if ui.add(retry_button).clicked() {
                        action = GarminSettingsAction::Connect;
                    }
                });
            });

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garmin_settings_screen_creation() {
        let screen = GarminSettingsScreen::new();
        assert!(!screen.is_connected());
        assert!(screen.user_profile.is_none());
        assert_eq!(screen.pending_uploads, 0);
    }

    #[test]
    fn test_connection_state_transitions() {
        let mut screen = GarminSettingsScreen::new();

        // Initial state
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());

        // Connecting state
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert!(!screen.is_connected());

        // Connected state
        screen.set_connection_state(GarminConnectionState::Connected);
        assert!(screen.is_connected());

        // Disconnected again
        screen.set_connection_state(GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_user_profile_display() {
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Connected);

        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist_pro".to_string(),
            full_name: Some("John Doe".to_string()),
            profile_image_url: None,
        };

        screen.set_user_profile(Some(profile.clone()));

        assert!(screen.user_profile.is_some());
        assert_eq!(screen.user_profile.as_ref().unwrap().readable_name(), "John Doe");
    }

    #[test]
    fn test_user_profile_without_full_name() {
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist_pro".to_string(),
            full_name: None,
            profile_image_url: None,
        };

        assert_eq!(profile.readable_name(), "cyclist_pro");
    }

    #[test]
    fn test_pending_uploads_display() {
        let mut screen = GarminSettingsScreen::new();
        assert_eq!(screen.pending_uploads, 0);

        screen.set_pending_uploads(5);
        assert_eq!(screen.pending_uploads, 5);
    }

    #[test]
    fn test_auto_sync_config() {
        let mut screen = GarminSettingsScreen::new();
        assert!(!screen.config.auto_sync);

        let config = PlatformConfig {
            enabled: true,
            auto_sync: true,
        };
        screen.set_config(config);

        assert!(screen.config.auto_sync);
        assert!(screen.config.enabled);
    }

    #[test]
    fn test_error_state() {
        let mut screen = GarminSettingsScreen::new();

        let error_msg = "Network connection failed".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg.clone()));

        match &screen.connection_state {
            GarminConnectionState::Error(err) => {
                assert_eq!(err, &error_msg);
            }
            _ => panic!("Expected Error state"),
        }

        assert!(!screen.is_connected());
    }

    #[test]
    fn test_settings_action_equality() {
        assert_eq!(GarminSettingsAction::None, GarminSettingsAction::None);
        assert_eq!(GarminSettingsAction::Connect, GarminSettingsAction::Connect);
        assert_eq!(GarminSettingsAction::Disconnect, GarminSettingsAction::Disconnect);
        assert_eq!(
            GarminSettingsAction::ToggleAutoSync(true),
            GarminSettingsAction::ToggleAutoSync(true)
        );
        assert_ne!(
            GarminSettingsAction::ToggleAutoSync(true),
            GarminSettingsAction::ToggleAutoSync(false)
        );
    }

    #[test]
    fn test_default_connection_state() {
        let state = GarminConnectionState::default();
        assert_eq!(state, GarminConnectionState::Disconnected);
    }

    #[test]
    fn test_last_sync_timestamp() {
        let mut screen = GarminSettingsScreen::new();
        assert!(screen.last_sync.is_none());

        screen.set_last_sync(Some("2024-01-15 10:30:00".to_string()));
        assert_eq!(screen.last_sync, Some("2024-01-15 10:30:00".to_string()));
    }

    #[test]
    fn test_connection_state_debug() {
        let state = GarminConnectionState::Connecting;
        assert_eq!(format!("{:?}", state), "Connecting");

        let error_state = GarminConnectionState::Error("test error".to_string());
        assert!(format!("{:?}", error_state).contains("test error"));
    }

    #[test]
    fn test_settings_action_debug() {
        let action = GarminSettingsAction::ToggleAutoSync(true);
        assert!(format!("{:?}", action).contains("ToggleAutoSync"));
        assert!(format!("{:?}", action).contains("true"));
    }
}
