//! TrainingPeaks connection settings screen.
//!
//! T019: Create dedicated TrainingPeaks settings screen with:
//! - Connection status display
//! - Connect/disconnect buttons
//! - Athlete profile display when connected

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::sync::trainingpeaks::AthleteProfile;
use crate::integrations::sync::{PlatformConfig, SyncPlatform};

use super::Screen;

/// TrainingPeaks brand color (teal)
const TRAININGPEAKS_TEAL: Color32 = Color32::from_rgb(0, 128, 128);

/// Connection state for TrainingPeaks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainingPeaksConnectionState {
    /// Not connected - show connect button
    Disconnected,
    /// OAuth flow in progress
    Connecting,
    /// Connected to TrainingPeaks
    Connected,
    /// Disconnecting in progress
    Disconnecting,
    /// Connection error
    Error(String),
}

impl Default for TrainingPeaksConnectionState {
    fn default() -> Self {
        TrainingPeaksConnectionState::Disconnected
    }
}

/// Actions that can result from the TrainingPeaks settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum TrainingPeaksSettingsAction {
    /// No action
    None,
    /// Navigate back
    Back,
    /// Start OAuth connection flow
    Connect,
    /// Disconnect from TrainingPeaks
    Disconnect,
    /// Toggle auto-sync setting
    ToggleAutoSync(bool),
}

/// TrainingPeaks settings screen state.
#[derive(Default)]
pub struct TrainingPeaksSettingsScreen {
    /// Current connection state
    pub connection_state: TrainingPeaksConnectionState,
    /// Athlete profile (when connected)
    pub athlete_profile: Option<AthleteProfile>,
    /// Platform configuration
    pub config: PlatformConfig,
    /// Number of pending uploads
    pub pending_uploads: usize,
    /// Last sync timestamp
    pub last_sync: Option<String>,
}

impl TrainingPeaksSettingsScreen {
    /// Create a new TrainingPeaks settings screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connection state.
    pub fn set_connection_state(&mut self, state: TrainingPeaksConnectionState) {
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
        matches!(self.connection_state, TrainingPeaksConnectionState::Connected)
    }

    /// Render the TrainingPeaks settings screen.
    pub fn show(&mut self, ui: &mut Ui) -> (Option<Screen>, TrainingPeaksSettingsAction) {
        let mut next_screen = None;
        let mut action = TrainingPeaksSettingsAction::None;

        ui.vertical(|ui| {
            // Header with back button
            ui.horizontal(|ui| {
                if ui.button("<- Back").clicked() {
                    next_screen = Some(Screen::Settings);
                    action = TrainingPeaksSettingsAction::Back;
                }
                ui.add_space(8.0);
                ui.heading(
                    RichText::new("TrainingPeaks")
                        .size(24.0)
                        .strong()
                        .color(TRAININGPEAKS_TEAL),
                );
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Main content
            match &self.connection_state {
                TrainingPeaksConnectionState::Disconnected => {
                    let connect_action = self.render_disconnected_state(ui);
                    if connect_action == TrainingPeaksSettingsAction::Connect {
                        action = connect_action;
                    }
                }
                TrainingPeaksConnectionState::Connecting => {
                    self.render_connecting_state(ui);
                }
                TrainingPeaksConnectionState::Connected => {
                    if let Some(disconnect_action) = self.render_connected_state(ui) {
                        action = disconnect_action;
                    }
                }
                TrainingPeaksConnectionState::Disconnecting => {
                    self.render_disconnecting_state(ui);
                }
                TrainingPeaksConnectionState::Error(error) => {
                    let error_action = self.render_error_state(ui, error.clone());
                    if error_action == TrainingPeaksSettingsAction::Connect {
                        action = error_action;
                    }
                }
            }
        });

        (next_screen, action)
    }

    /// Render the disconnected state with connect button.
    fn render_disconnected_state(&self, ui: &mut Ui) -> TrainingPeaksSettingsAction {
        let mut action = TrainingPeaksSettingsAction::None;

        // Information panel
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical_centered(|ui| {
                    // TrainingPeaks logo/icon placeholder
                    ui.label(RichText::new("TP").size(64.0).color(TRAININGPEAKS_TEAL).strong());

                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("Connect your TrainingPeaks account")
                            .size(18.0)
                            .strong(),
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(
                            "Sync your rides and download workout plans from TrainingPeaks.",
                        )
                        .weak(),
                    );

                    ui.add_space(24.0);

                    // Connect button
                    let connect_button = egui::Button::new(
                        RichText::new("Connect to TrainingPeaks")
                            .size(16.0)
                            .color(Color32::WHITE),
                    )
                    .fill(TRAININGPEAKS_TEAL)
                    .min_size(Vec2::new(220.0, 44.0));

                    if ui.add(connect_button).clicked() {
                        action = TrainingPeaksSettingsAction::Connect;
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
                    "Download structured workout plans",
                    "Sync workout library with your coach",
                    "Secure OAuth 2.0 authentication",
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("*").color(TRAININGPEAKS_TEAL));
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
                    ui.label(RichText::new("Connecting to TrainingPeaks...").size(16.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Complete authorization in your browser")
                            .weak(),
                    );
                });
            });
    }

    /// Render the connected state with athlete profile.
    fn render_connected_state(&mut self, ui: &mut Ui) -> Option<TrainingPeaksSettingsAction> {
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
                                RichText::new("Connected to TrainingPeaks")
                                    .color(Color32::from_rgb(52, 168, 83)),
                            );
                        });
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Disconnect button
                        if ui
                            .button("Disconnect")
                            .on_hover_text("Remove authorization from TrainingPeaks")
                            .clicked()
                        {
                            action = Some(TrainingPeaksSettingsAction::Disconnect);
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
                        action = Some(TrainingPeaksSettingsAction::ToggleAutoSync(auto_sync));
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

        // TrainingPeaks profile link
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.label(RichText::new("View your profile on TrainingPeaks").weak());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(ref profile) = self.athlete_profile {
                            let profile_url = format!(
                                "https://www.trainingpeaks.com/athlete/{}",
                                profile.id
                            );
                            if ui.small_button("Open").clicked() {
                                if let Err(e) = open::that(&profile_url) {
                                    tracing::warn!("Failed to open TrainingPeaks profile: {}", e);
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
                    ui.label(RichText::new("Disconnecting from TrainingPeaks...").size(16.0));
                });
            });
    }

    /// Render the error state with retry button.
    fn render_error_state(&self, ui: &mut Ui, error: String) -> TrainingPeaksSettingsAction {
        let mut action = TrainingPeaksSettingsAction::None;

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
                    .fill(TRAININGPEAKS_TEAL)
                    .min_size(Vec2::new(120.0, 36.0));

                    if ui.add(retry_button).clicked() {
                        action = TrainingPeaksSettingsAction::Connect;
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
    fn test_trainingpeaks_settings_screen_creation() {
        let screen = TrainingPeaksSettingsScreen::new();
        assert!(!screen.is_connected());
        assert!(screen.athlete_profile.is_none());
        assert_eq!(screen.pending_uploads, 0);
    }

    #[test]
    fn test_connection_state_transitions() {
        let mut screen = TrainingPeaksSettingsScreen::new();

        // Initial state
        assert_eq!(screen.connection_state, TrainingPeaksConnectionState::Disconnected);
        assert!(!screen.is_connected());

        // Connecting state
        screen.set_connection_state(TrainingPeaksConnectionState::Connecting);
        assert!(!screen.is_connected());

        // Connected state
        screen.set_connection_state(TrainingPeaksConnectionState::Connected);
        assert!(screen.is_connected());

        // Disconnected again
        screen.set_connection_state(TrainingPeaksConnectionState::Disconnected);
        assert!(!screen.is_connected());
    }

    #[test]
    fn test_athlete_profile_display() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        screen.set_connection_state(TrainingPeaksConnectionState::Connected);

        let profile = AthleteProfile {
            id: 12345,
            firstname: "John".to_string(),
            lastname: "Doe".to_string(),
            profile_photo_url: None,
        };

        screen.set_athlete_profile(Some(profile.clone()));

        assert!(screen.athlete_profile.is_some());
        assert_eq!(screen.athlete_profile.as_ref().unwrap().display_name(), "John Doe");
    }

    #[test]
    fn test_athlete_profile_display_name() {
        let profile = AthleteProfile {
            id: 12345,
            firstname: "Jane".to_string(),
            lastname: "Smith".to_string(),
            profile_photo_url: Some("https://example.com/photo.jpg".to_string()),
        };

        assert_eq!(profile.display_name(), "Jane Smith");
    }

    #[test]
    fn test_pending_uploads_display() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        assert_eq!(screen.pending_uploads, 0);

        screen.set_pending_uploads(5);
        assert_eq!(screen.pending_uploads, 5);
    }

    #[test]
    fn test_auto_sync_config() {
        let mut screen = TrainingPeaksSettingsScreen::new();
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
        let mut screen = TrainingPeaksSettingsScreen::new();

        let error_msg = "Network connection failed".to_string();
        screen.set_connection_state(TrainingPeaksConnectionState::Error(error_msg.clone()));

        match &screen.connection_state {
            TrainingPeaksConnectionState::Error(err) => {
                assert_eq!(err, &error_msg);
            }
            _ => panic!("Expected Error state"),
        }

        assert!(!screen.is_connected());
    }

    #[test]
    fn test_settings_action_equality() {
        assert_eq!(TrainingPeaksSettingsAction::None, TrainingPeaksSettingsAction::None);
        assert_eq!(TrainingPeaksSettingsAction::Connect, TrainingPeaksSettingsAction::Connect);
        assert_eq!(TrainingPeaksSettingsAction::Disconnect, TrainingPeaksSettingsAction::Disconnect);
        assert_eq!(
            TrainingPeaksSettingsAction::ToggleAutoSync(true),
            TrainingPeaksSettingsAction::ToggleAutoSync(true)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::ToggleAutoSync(true),
            TrainingPeaksSettingsAction::ToggleAutoSync(false)
        );
    }

    #[test]
    fn test_last_sync_timestamp() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        assert!(screen.last_sync.is_none());

        screen.set_last_sync(Some("2024-01-15 10:30 AM".to_string()));
        assert_eq!(screen.last_sync, Some("2024-01-15 10:30 AM".to_string()));

        screen.set_last_sync(None);
        assert!(screen.last_sync.is_none());
    }

    #[test]
    fn test_disconnecting_state() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        screen.set_connection_state(TrainingPeaksConnectionState::Disconnecting);

        assert!(!screen.is_connected());
        assert_eq!(screen.connection_state, TrainingPeaksConnectionState::Disconnecting);
    }
}
