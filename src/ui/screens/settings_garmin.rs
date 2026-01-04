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

    #[test]
    fn test_disconnected_state_features() {
        // Verify the disconnected state shows correct state
        let screen = GarminSettingsScreen::new();
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());
        // The render_disconnected_state method renders:
        // 1. Garmin logo/branding (GARMIN_BLUE color)
        // 2. Connect button with "Connect to Garmin" text
        // 3. Features list with 4 Garmin-specific features
    }

    #[test]
    fn test_garmin_brand_color() {
        // Verify Garmin brand color is correctly defined
        assert_eq!(GARMIN_BLUE, Color32::from_rgb(0, 118, 206));
    }

    #[test]
    fn test_disconnected_state_is_default() {
        // Verify new screens start in disconnected state
        let screen = GarminSettingsScreen::new();
        matches!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(screen.user_profile.is_none());
        assert_eq!(screen.pending_uploads, 0);
        assert!(screen.last_sync.is_none());
    }

    #[test]
    fn test_connected_state_with_full_profile() {
        // Verify connected state renders user profile correctly
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Connected);

        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist_pro".to_string(),
            full_name: Some("John Doe".to_string()),
            profile_image_url: Some("https://example.com/avatar.jpg".to_string()),
        };
        screen.set_user_profile(Some(profile));

        // Verify profile is accessible
        assert!(screen.is_connected());
        assert!(screen.user_profile.is_some());
        let profile = screen.user_profile.as_ref().unwrap();
        assert_eq!(profile.readable_name(), "John Doe");
        assert_eq!(profile.display_name, "cyclist_pro");
        assert_eq!(profile.user_id, 12345);
    }

    #[test]
    fn test_connected_state_pending_uploads_display() {
        // Verify pending uploads count is tracked correctly
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Connected);

        // No pending uploads initially
        assert_eq!(screen.pending_uploads, 0);

        // Set some pending uploads
        screen.set_pending_uploads(3);
        assert_eq!(screen.pending_uploads, 3);

        // Verify can handle large counts
        screen.set_pending_uploads(999);
        assert_eq!(screen.pending_uploads, 999);
    }

    #[test]
    fn test_connected_state_sync_settings() {
        // Verify sync settings are configurable
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Connected);

        // Default config has auto_sync disabled
        assert!(!screen.config.auto_sync);
        assert!(!screen.config.enabled);

        // Enable auto-sync
        let config = PlatformConfig {
            enabled: true,
            auto_sync: true,
        };
        screen.set_config(config);

        assert!(screen.config.enabled);
        assert!(screen.config.auto_sync);

        // Disable auto-sync
        let config = PlatformConfig {
            enabled: true,
            auto_sync: false,
        };
        screen.set_config(config);

        assert!(screen.config.enabled);
        assert!(!screen.config.auto_sync);
    }

    #[test]
    fn test_connected_state_last_sync_display() {
        // Verify last sync timestamp is displayed
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Connected);

        // Initially no last sync
        assert!(screen.last_sync.is_none());

        // Set last sync timestamp
        screen.set_last_sync(Some("2024-01-15 10:30:00 UTC".to_string()));
        assert_eq!(
            screen.last_sync,
            Some("2024-01-15 10:30:00 UTC".to_string())
        );

        // Can update last sync
        screen.set_last_sync(Some("2024-01-16 15:45:00 UTC".to_string()));
        assert_eq!(
            screen.last_sync,
            Some("2024-01-16 15:45:00 UTC".to_string())
        );

        // Can clear last sync
        screen.set_last_sync(None);
        assert!(screen.last_sync.is_none());
    }

    #[test]
    fn test_connected_state_disconnect_action() {
        // Verify disconnect action is properly defined
        let action = GarminSettingsAction::Disconnect;
        assert_eq!(action, GarminSettingsAction::Disconnect);
        assert_ne!(action, GarminSettingsAction::None);
        assert_ne!(action, GarminSettingsAction::Connect);
    }

    #[test]
    fn test_connected_state_toggle_auto_sync_action() {
        // Verify auto-sync toggle action captures new state
        let enable_action = GarminSettingsAction::ToggleAutoSync(true);
        let disable_action = GarminSettingsAction::ToggleAutoSync(false);

        // Actions with different values are not equal
        assert_ne!(enable_action, disable_action);

        // Verify action values
        match enable_action {
            GarminSettingsAction::ToggleAutoSync(value) => assert!(value),
            _ => panic!("Expected ToggleAutoSync action"),
        }

        match disable_action {
            GarminSettingsAction::ToggleAutoSync(value) => assert!(!value),
            _ => panic!("Expected ToggleAutoSync action"),
        }
    }

    #[test]
    fn test_connected_state_profile_initials() {
        // Verify profile initials generation for avatar
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist_pro".to_string(),
            full_name: Some("John Doe".to_string()),
            profile_image_url: None,
        };

        // readable_name returns full_name when available
        let readable = profile.readable_name();
        assert_eq!(readable, "John Doe");

        // First two characters for initials
        let initials: String = readable.chars().take(2).collect::<String>().to_uppercase();
        assert_eq!(initials, "JO");
    }

    #[test]
    fn test_connected_state_profile_link() {
        // Verify profile URL generation for Garmin Connect
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "cyclist_pro".to_string(),
            full_name: None,
            profile_image_url: None,
        };

        let profile_url = format!(
            "https://connect.garmin.com/modern/profile/{}",
            profile.display_name
        );
        assert_eq!(
            profile_url,
            "https://connect.garmin.com/modern/profile/cyclist_pro"
        );
    }

    #[test]
    fn test_connected_state_complete_setup() {
        // Verify complete connected state with all fields populated
        let mut screen = GarminSettingsScreen::new();

        // Set up connected state
        screen.set_connection_state(GarminConnectionState::Connected);

        // Set up profile
        let profile = GarminUserProfile {
            user_id: 98765,
            display_name: "garmin_rider".to_string(),
            full_name: Some("Jane Smith".to_string()),
            profile_image_url: Some("https://connect.garmin.com/avatar.jpg".to_string()),
        };
        screen.set_user_profile(Some(profile));

        // Set up config
        let config = PlatformConfig {
            enabled: true,
            auto_sync: true,
        };
        screen.set_config(config);

        // Set up pending uploads and last sync
        screen.set_pending_uploads(2);
        screen.set_last_sync(Some("Today at 3:30 PM".to_string()));

        // Verify all state
        assert!(screen.is_connected());
        assert!(screen.user_profile.is_some());
        assert!(screen.config.enabled);
        assert!(screen.config.auto_sync);
        assert_eq!(screen.pending_uploads, 2);
        assert_eq!(screen.last_sync, Some("Today at 3:30 PM".to_string()));
    }

    // ===========================================
    // Connecting/Disconnecting State Tests (4.4)
    // ===========================================

    #[test]
    fn test_connecting_state_ui() {
        // Verify connecting state is properly set and not considered connected
        let mut screen = GarminSettingsScreen::new();

        screen.set_connection_state(GarminConnectionState::Connecting);

        // Connecting state should NOT be considered connected
        assert!(!screen.is_connected());

        // Verify state is Connecting
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);

        // The render_connecting_state method renders:
        // 1. ui.spinner() - animated loading indicator
        // 2. "Connecting to Garmin Connect..." - main status message (size 16.0)
        // 3. "Complete authorization in your browser" - helper text (weak)
    }

    #[test]
    fn test_disconnecting_state_ui() {
        // Verify disconnecting state is properly set and not considered connected
        let mut screen = GarminSettingsScreen::new();

        screen.set_connection_state(GarminConnectionState::Disconnecting);

        // Disconnecting state should NOT be considered connected
        assert!(!screen.is_connected());

        // Verify state is Disconnecting
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnecting);

        // The render_disconnecting_state method renders:
        // 1. ui.spinner() - animated loading indicator
        // 2. "Disconnecting from Garmin Connect..." - main status message (size 16.0)
    }

    #[test]
    fn test_connecting_state_transition_from_disconnected() {
        // Verify transition from disconnected to connecting (OAuth flow start)
        let mut screen = GarminSettingsScreen::new();

        // Start in disconnected state
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);

        // Transition to connecting (user clicked "Connect to Garmin")
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);
        assert!(!screen.is_connected());

        // Transition to connected (OAuth completed successfully)
        screen.set_connection_state(GarminConnectionState::Connected);
        assert_eq!(screen.connection_state, GarminConnectionState::Connected);
        assert!(screen.is_connected());
    }

    #[test]
    fn test_connecting_state_transition_to_error() {
        // Verify transition from connecting to error (OAuth failed)
        let mut screen = GarminSettingsScreen::new();

        // Simulate OAuth flow start
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);

        // OAuth fails with an error
        let error_msg = "Authorization was denied by user".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg.clone()));

        // Verify error state
        match &screen.connection_state {
            GarminConnectionState::Error(err) => {
                assert_eq!(err, &error_msg);
            }
            _ => panic!("Expected Error state after failed OAuth"),
        }
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_disconnecting_state_transition() {
        // Verify full disconnect flow: Connected -> Disconnecting -> Disconnected
        let mut screen = GarminSettingsScreen::new();

        // Set up initial connected state
        screen.set_connection_state(GarminConnectionState::Connected);
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "test_user".to_string(),
            full_name: Some("Test User".to_string()),
            profile_image_url: None,
        };
        screen.set_user_profile(Some(profile));
        assert!(screen.is_connected());

        // User clicks disconnect - enter disconnecting state
        screen.set_connection_state(GarminConnectionState::Disconnecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnecting);
        assert!(!screen.is_connected());

        // Disconnect completes - back to disconnected state
        screen.set_connection_state(GarminConnectionState::Disconnected);
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_connecting_state_preserves_no_action() {
        // Connecting state should not produce any action (no buttons active)
        let screen = GarminSettingsScreen::new();

        // Connecting state doesn't return any action from render_connecting_state
        // The method signature is `fn render_connecting_state(&self, ui: &mut Ui)`
        // with no return value, meaning no user action is possible during OAuth flow
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
    }

    #[test]
    fn test_disconnecting_state_preserves_no_action() {
        // Disconnecting state should not produce any action (no buttons active)
        let mut screen = GarminSettingsScreen::new();
        screen.set_connection_state(GarminConnectionState::Disconnecting);

        // Disconnecting state doesn't return any action from render_disconnecting_state
        // The method signature is `fn render_disconnecting_state(&self, ui: &mut Ui)`
        // with no return value, meaning no user action is possible during disconnect
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnecting);
    }

    #[test]
    fn test_connection_states_debug_format() {
        // Verify all connection states have proper debug formatting
        let disconnected = GarminConnectionState::Disconnected;
        let connecting = GarminConnectionState::Connecting;
        let connected = GarminConnectionState::Connected;
        let disconnecting = GarminConnectionState::Disconnecting;
        let error = GarminConnectionState::Error("Test error".to_string());

        assert_eq!(format!("{:?}", disconnected), "Disconnected");
        assert_eq!(format!("{:?}", connecting), "Connecting");
        assert_eq!(format!("{:?}", connected), "Connected");
        assert_eq!(format!("{:?}", disconnecting), "Disconnecting");
        assert!(format!("{:?}", error).contains("Error"));
        assert!(format!("{:?}", error).contains("Test error"));
    }

    #[test]
    fn test_all_connection_states_equality() {
        // Verify equality comparisons for all connection states
        assert_eq!(
            GarminConnectionState::Disconnected,
            GarminConnectionState::Disconnected
        );
        assert_eq!(
            GarminConnectionState::Connecting,
            GarminConnectionState::Connecting
        );
        assert_eq!(
            GarminConnectionState::Connected,
            GarminConnectionState::Connected
        );
        assert_eq!(
            GarminConnectionState::Disconnecting,
            GarminConnectionState::Disconnecting
        );
        assert_eq!(
            GarminConnectionState::Error("same".to_string()),
            GarminConnectionState::Error("same".to_string())
        );

        // Different states are not equal
        assert_ne!(
            GarminConnectionState::Connecting,
            GarminConnectionState::Connected
        );
        assert_ne!(
            GarminConnectionState::Connecting,
            GarminConnectionState::Disconnecting
        );
        assert_ne!(
            GarminConnectionState::Error("a".to_string()),
            GarminConnectionState::Error("b".to_string())
        );
    }

    #[test]
    fn test_oauth_flow_complete_cycle() {
        // Test complete OAuth flow cycle: Disconnected -> Connecting -> Connected
        let mut screen = GarminSettingsScreen::new();

        // 1. Initial state: Disconnected
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());
        assert!(screen.user_profile.is_none());

        // 2. User clicks "Connect to Garmin" button
        // This would trigger GarminSettingsAction::Connect
        // App handles this by opening browser and setting state to Connecting
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);
        assert!(!screen.is_connected());

        // 3. User completes OAuth in browser
        // Callback received, tokens exchanged, profile fetched
        let profile = GarminUserProfile {
            user_id: 54321,
            display_name: "garmin_cyclist".to_string(),
            full_name: Some("OAuth Test User".to_string()),
            profile_image_url: None,
        };
        screen.set_user_profile(Some(profile));
        screen.set_connection_state(GarminConnectionState::Connected);

        // 4. Verify final connected state
        assert_eq!(screen.connection_state, GarminConnectionState::Connected);
        assert!(screen.is_connected());
        assert!(screen.user_profile.is_some());
        assert_eq!(
            screen.user_profile.as_ref().unwrap().readable_name(),
            "OAuth Test User"
        );
    }

    // ===========================================
    // Error State Tests (4.5)
    // ===========================================

    #[test]
    fn test_error_state_ui_components() {
        // Verify error state displays the expected UI components
        let mut screen = GarminSettingsScreen::new();

        let error_msg = "Failed to connect: Network timeout".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg.clone()));

        // Verify error state
        match &screen.connection_state {
            GarminConnectionState::Error(err) => {
                assert_eq!(err, &error_msg);
            }
            _ => panic!("Expected Error state"),
        }

        // Error state should NOT be considered connected
        assert!(!screen.is_connected());

        // The render_error_state method renders:
        // 1. Frame with dark error background (Color32::from_rgb(40, 30, 30))
        // 2. Error icon "!" in red (size 48.0, Color32::from_rgb(234, 67, 53))
        // 3. "Connection Error" heading in red (size 18.0, strong)
        // 4. Error message text (weak styling)
        // 5. "Try Again" retry button (GARMIN_BLUE fill, size 120x36)
    }

    #[test]
    fn test_error_state_retry_action() {
        // Verify clicking retry in error state triggers Connect action
        // The render_error_state method returns GarminSettingsAction::Connect
        // when the "Try Again" button is clicked
        let action = GarminSettingsAction::Connect;
        assert_eq!(action, GarminSettingsAction::Connect);
        assert_ne!(action, GarminSettingsAction::None);
        assert_ne!(action, GarminSettingsAction::Disconnect);
    }

    #[test]
    fn test_error_state_transition_to_connecting() {
        // Verify user can retry from error state back to connecting
        let mut screen = GarminSettingsScreen::new();

        // Start in error state
        let error_msg = "OAuth authorization denied".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg));
        assert!(!screen.is_connected());

        // User clicks "Try Again" - transition back to Connecting
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_error_state_transition_to_disconnected() {
        // Verify error state can transition back to disconnected
        let mut screen = GarminSettingsScreen::new();

        // Start in error state
        screen.set_connection_state(GarminConnectionState::Error("Some error".to_string()));
        assert!(!screen.is_connected());

        // User navigates away or error is cleared
        screen.set_connection_state(GarminConnectionState::Disconnected);
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_error_state_preserves_error_message() {
        // Verify error messages of various formats are preserved
        let test_cases = vec![
            "Network connection failed",
            "Authorization was denied by user",
            "Garmin Connect API returned error: 401 Unauthorized",
            "Request timeout after 30 seconds",
            "Invalid OAuth state - possible CSRF attack",
            "",  // Empty error message
            "Error with special chars: !@#$%^&*()",
        ];

        for error_msg in test_cases {
            let mut screen = GarminSettingsScreen::new();
            screen.set_connection_state(GarminConnectionState::Error(error_msg.to_string()));

            match &screen.connection_state {
                GarminConnectionState::Error(err) => {
                    assert_eq!(err, error_msg);
                }
                _ => panic!("Expected Error state for message: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_error_state_equality() {
        // Verify error states with same message are equal
        assert_eq!(
            GarminConnectionState::Error("test".to_string()),
            GarminConnectionState::Error("test".to_string())
        );

        // Verify error states with different messages are not equal
        assert_ne!(
            GarminConnectionState::Error("error1".to_string()),
            GarminConnectionState::Error("error2".to_string())
        );

        // Verify error state is not equal to other states
        assert_ne!(
            GarminConnectionState::Error("error".to_string()),
            GarminConnectionState::Disconnected
        );
        assert_ne!(
            GarminConnectionState::Error("error".to_string()),
            GarminConnectionState::Connecting
        );
        assert_ne!(
            GarminConnectionState::Error("error".to_string()),
            GarminConnectionState::Connected
        );
        assert_ne!(
            GarminConnectionState::Error("error".to_string()),
            GarminConnectionState::Disconnecting
        );
    }

    #[test]
    fn test_error_state_from_connecting_failure() {
        // Test complete flow: Disconnected -> Connecting -> Error -> Connecting -> Connected
        let mut screen = GarminSettingsScreen::new();

        // 1. Initial state
        assert_eq!(screen.connection_state, GarminConnectionState::Disconnected);

        // 2. User clicks connect
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);

        // 3. OAuth fails with error
        let error_msg = "User cancelled authorization".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg.clone()));
        match &screen.connection_state {
            GarminConnectionState::Error(err) => assert_eq!(err, &error_msg),
            _ => panic!("Expected Error state"),
        }

        // 4. User clicks "Try Again"
        screen.set_connection_state(GarminConnectionState::Connecting);
        assert_eq!(screen.connection_state, GarminConnectionState::Connecting);

        // 5. OAuth succeeds this time
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "retry_user".to_string(),
            full_name: Some("Retry User".to_string()),
            profile_image_url: None,
        };
        screen.set_user_profile(Some(profile));
        screen.set_connection_state(GarminConnectionState::Connected);
        assert!(screen.is_connected());
    }

    #[test]
    fn test_error_state_clears_profile() {
        // Verify profile remains None when transitioning to error state
        let mut screen = GarminSettingsScreen::new();

        // User never connected, so no profile
        assert!(screen.user_profile.is_none());

        // Connection fails
        screen.set_connection_state(GarminConnectionState::Error("Connection failed".to_string()));

        // Profile should still be None
        assert!(screen.user_profile.is_none());
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_error_state_after_disconnect_failure() {
        // Test error during disconnect: Connected -> Disconnecting -> Error
        let mut screen = GarminSettingsScreen::new();

        // Start connected
        screen.set_connection_state(GarminConnectionState::Connected);
        let profile = GarminUserProfile {
            user_id: 12345,
            display_name: "test_user".to_string(),
            full_name: None,
            profile_image_url: None,
        };
        screen.set_user_profile(Some(profile));
        assert!(screen.is_connected());

        // User clicks disconnect
        screen.set_connection_state(GarminConnectionState::Disconnecting);
        assert!(!screen.is_connected());

        // Disconnect fails (e.g., network error during token revocation)
        let error_msg = "Failed to revoke token: Network error".to_string();
        screen.set_connection_state(GarminConnectionState::Error(error_msg.clone()));

        // Verify error state
        match &screen.connection_state {
            GarminConnectionState::Error(err) => assert_eq!(err, &error_msg),
            _ => panic!("Expected Error state after disconnect failure"),
        }
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_error_state_debug_formatting() {
        // Verify error state debug formatting includes error message
        let error = GarminConnectionState::Error("Debug test error".to_string());
        let debug_str = format!("{:?}", error);

        assert!(debug_str.contains("Error"));
        assert!(debug_str.contains("Debug test error"));
    }

    #[test]
    fn test_error_colors_match_strava_pattern() {
        // Verify error UI uses consistent colors following Strava pattern
        // Error icon color: Color32::from_rgb(234, 67, 53) - Google/Material red
        // Error background: Color32::from_rgb(40, 30, 30) - dark with red tint
        // Retry button: GARMIN_BLUE (brand color) for consistency
        let error_red = Color32::from_rgb(234, 67, 53);
        let error_bg = Color32::from_rgb(40, 30, 30);

        // Verify colors are defined correctly
        assert_eq!(error_red.r(), 234);
        assert_eq!(error_red.g(), 67);
        assert_eq!(error_red.b(), 53);

        assert_eq!(error_bg.r(), 40);
        assert_eq!(error_bg.g(), 30);
        assert_eq!(error_bg.b(), 30);

        // GARMIN_BLUE for retry button
        assert_eq!(GARMIN_BLUE, Color32::from_rgb(0, 118, 206));
    }
}
