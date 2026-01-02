//! Strava connection settings screen.
//!
//! T109: Create dedicated Strava settings screen with:
//! - Connection status display
//! - Connect/disconnect buttons
//! - Athlete profile display when connected

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::sync::strava::AthleteProfile;
use crate::integrations::sync::{PlatformConfig, SyncPlatform};

use super::Screen;

/// Strava brand color
const STRAVA_ORANGE: Color32 = Color32::from_rgb(252, 82, 0);

/// Connection state for Strava
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StravaConnectionState {
    /// Not connected - show connect button
    Disconnected,
    /// OAuth flow in progress
    Connecting,
    /// Connected to Strava
    Connected,
    /// Disconnecting in progress
    Disconnecting,
    /// Connection error
    Error(String),
}

impl Default for StravaConnectionState {
    fn default() -> Self {
        StravaConnectionState::Disconnected
    }
}

/// Actions that can result from the Strava settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum StravaSettingsAction {
    /// No action
    None,
    /// Navigate back
    Back,
    /// Start OAuth connection flow
    Connect,
    /// Disconnect from Strava
    Disconnect,
    /// Toggle auto-sync setting
    ToggleAutoSync(bool),
}

/// Strava settings screen state.
#[derive(Default)]
pub struct StravaSettingsScreen {
    /// Current connection state
    pub connection_state: StravaConnectionState,
    /// Athlete profile (when connected)
    pub athlete_profile: Option<AthleteProfile>,
    /// Platform configuration
    pub config: PlatformConfig,
    /// Number of pending uploads
    pub pending_uploads: usize,
    /// Last sync timestamp
    pub last_sync: Option<String>,
}

impl StravaSettingsScreen {
    /// Create a new Strava settings screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connection state.
    pub fn set_connection_state(&mut self, state: StravaConnectionState) {
        self.connection_state = state;
    }

    /// Set the athlete profile (when connected).
    pub fn set_athlete_profile(&mut self, profile: Option<AthleteProfile>) {
        self.athlete_profile = profile;
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
        matches!(self.connection_state, StravaConnectionState::Connected)
    }

    /// Render the Strava settings screen.
    pub fn show(&mut self, ui: &mut Ui) -> (Option<Screen>, StravaSettingsAction) {
        let mut next_screen = None;
        let mut action = StravaSettingsAction::None;

        ui.vertical(|ui| {
            // Header with back button
            ui.horizontal(|ui| {
                if ui.button("<- Back").clicked() {
                    next_screen = Some(Screen::Settings);
                    action = StravaSettingsAction::Back;
                }
                ui.add_space(8.0);
                ui.heading(
                    RichText::new("Strava")
                        .size(24.0)
                        .strong()
                        .color(STRAVA_ORANGE),
                );
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Main content
            match &self.connection_state {
                StravaConnectionState::Disconnected => {
                    let connect_action = self.render_disconnected_state(ui);
                    if connect_action == StravaSettingsAction::Connect {
                        action = connect_action;
                    }
                }
                StravaConnectionState::Connecting => {
                    self.render_connecting_state(ui);
                }
                StravaConnectionState::Connected => {
                    if let Some(disconnect_action) = self.render_connected_state(ui) {
                        action = disconnect_action;
                    }
                }
                StravaConnectionState::Disconnecting => {
                    self.render_disconnecting_state(ui);
                }
                StravaConnectionState::Error(error) => {
                    let error_action = self.render_error_state(ui, error.clone());
                    if error_action == StravaSettingsAction::Connect {
                        action = error_action;
                    }
                }
            }
        });

        (next_screen, action)
    }

    /// Render the disconnected state with connect button.
    fn render_disconnected_state(&self, ui: &mut Ui) -> StravaSettingsAction {
        let mut action = StravaSettingsAction::None;

        // Information panel
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    // Strava logo/icon placeholder
                    ui.label(RichText::new("S").size(64.0).color(STRAVA_ORANGE).strong());

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("Connect your Strava account")
                            .size(18.0)
                            .strong(),
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(
                            "Automatically upload your rides to Strava after each session.",
                        )
                        .weak(),
                    );

                    ui.add_space(24.0);

                    // Connect button
                    let connect_button = egui::Button::new(
                        RichText::new("Connect to Strava")
                            .size(16.0)
                            .color(Color32::WHITE),
                    )
                    .fill(STRAVA_ORANGE)
                    .min_size(Vec2::new(200.0, 44.0));

                    if ui.add(connect_button).clicked() {
                        action = StravaSettingsAction::Connect;
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
                    "Upload pending rides when back online",
                    "Manual retry for failed uploads",
                    "Secure OAuth 2.0 authentication",
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("*").color(STRAVA_ORANGE));
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
                    ui.label(RichText::new("Connecting to Strava...").size(16.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Complete authorization in your browser")
                            .weak(),
                    );
                });
            });
    }

    /// Render the connected state with athlete profile.
    fn render_connected_state(&mut self, ui: &mut Ui) -> Option<StravaSettingsAction> {
        let mut action = None;

        // Athlete profile section
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
                                if let Some(ref profile) = self.athlete_profile {
                                    // Display initials as avatar placeholder
                                    let initials = format!(
                                        "{}{}",
                                        profile.firstname.chars().next().unwrap_or('?'),
                                        profile.lastname.chars().next().unwrap_or('?')
                                    );
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
                        if let Some(ref profile) = self.athlete_profile {
                            // Athlete name
                            ui.label(
                                RichText::new(profile.display_name())
                                    .size(18.0)
                                    .strong(),
                            );

                            // Username if available
                            if let Some(ref username) = profile.username {
                                ui.label(
                                    RichText::new(format!("@{}", username))
                                        .color(Color32::GRAY),
                                );
                            }
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
                                RichText::new("Connected to Strava")
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        });
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Disconnect button
                        if ui
                            .button("Disconnect")
                            .on_hover_text("Remove authorization from Strava")
                            .clicked()
                        {
                            action = Some(StravaSettingsAction::Disconnect);
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
                        action = Some(StravaSettingsAction::ToggleAutoSync(auto_sync));
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

        // Strava profile link
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.label(RichText::new("View your profile on Strava").weak());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(ref profile) = self.athlete_profile {
                            let profile_url = format!(
                                "https://www.strava.com/athletes/{}",
                                profile.id
                            );
                            if ui.small_button("Open").clicked() {
                                if let Err(e) = open::that(&profile_url) {
                                    tracing::warn!("Failed to open Strava profile: {}", e);
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
                    ui.label(RichText::new("Disconnecting from Strava...").size(16.0));
                });
            });
    }

    /// Render the error state with retry button.
    fn render_error_state(&self, ui: &mut Ui, error: String) -> StravaSettingsAction {
        let mut action = StravaSettingsAction::None;

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
                    .fill(STRAVA_ORANGE)
                    .min_size(Vec2::new(120.0, 36.0));

                    if ui.add(retry_button).clicked() {
                        action = StravaSettingsAction::Connect;
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
    fn test_strava_settings_screen_creation() {
        let screen = StravaSettingsScreen::new();
        assert!(!screen.is_connected());
        assert!(screen.athlete_profile.is_none());
        assert_eq!(screen.pending_uploads, 0);
    }

    #[test]
    fn test_connection_state_transitions() {
        let mut screen = StravaSettingsScreen::new();

        // Initial state
        assert_eq!(screen.connection_state, StravaConnectionState::Disconnected);
        assert!(!screen.is_connected());

        // Connecting state
        screen.set_connection_state(StravaConnectionState::Connecting);
        assert!(!screen.is_connected());

        // Connected state
        screen.set_connection_state(StravaConnectionState::Connected);
        assert!(screen.is_connected());

        // Disconnected again
        screen.set_connection_state(StravaConnectionState::Disconnected);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_athlete_profile_display() {
        let mut screen = StravaSettingsScreen::new();
        screen.set_connection_state(StravaConnectionState::Connected);

        let profile = AthleteProfile {
            id: 12345,
            username: Some("cyclist_pro".to_string()),
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_medium: None,
        };

        screen.set_athlete_profile(Some(profile.clone()));

        assert!(screen.athlete_profile.is_some());
        assert_eq!(screen.athlete_profile.as_ref().unwrap().display_name(), "cyclist_pro");
    }

    #[test]
    fn test_athlete_profile_without_username() {
        let profile = AthleteProfile {
            id: 12345,
            username: None,
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_medium: None,
        };

        assert_eq!(profile.display_name(), "John Doe");
    }

    #[test]
    fn test_pending_uploads_display() {
        let mut screen = StravaSettingsScreen::new();
        assert_eq!(screen.pending_uploads, 0);

        screen.set_pending_uploads(5);
        assert_eq!(screen.pending_uploads, 5);
    }

    #[test]
    fn test_auto_sync_config() {
        let mut screen = StravaSettingsScreen::new();
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
        let mut screen = StravaSettingsScreen::new();

        let error_msg = "Network connection failed".to_string();
        screen.set_connection_state(StravaConnectionState::Error(error_msg.clone()));

        match &screen.connection_state {
            StravaConnectionState::Error(err) => {
                assert_eq!(err, &error_msg);
            }
            _ => panic!("Expected Error state"),
        }

        assert!(!screen.is_connected());
    }

    #[test]
    fn test_settings_action_equality() {
        assert_eq!(StravaSettingsAction::None, StravaSettingsAction::None);
        assert_eq!(StravaSettingsAction::Connect, StravaSettingsAction::Connect);
        assert_eq!(StravaSettingsAction::Disconnect, StravaSettingsAction::Disconnect);
        assert_eq!(
            StravaSettingsAction::ToggleAutoSync(true),
            StravaSettingsAction::ToggleAutoSync(true)
        );
        assert_ne!(
            StravaSettingsAction::ToggleAutoSync(true),
            StravaSettingsAction::ToggleAutoSync(false)
        );
    }
}
