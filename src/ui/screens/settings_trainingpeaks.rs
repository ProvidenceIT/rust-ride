//! TrainingPeaks connection settings screen.
//!
//! T019: Create dedicated TrainingPeaks settings screen with:
//! - Connection status display
//! - Connect/disconnect buttons
//! - Athlete profile display when connected

use egui::{Align, Color32, Layout, RichText, Ui, Vec2};

use crate::integrations::sync::trainingpeaks::AthleteProfile;
use crate::integrations::sync::{PlatformConfig, SyncPlatform, TrainingPeaksPlatformConfig};

use super::Screen;

/// TrainingPeaks brand color (teal)
const TRAININGPEAKS_TEAL: Color32 = Color32::from_rgb(0, 128, 128);

/// Options for how many days to look ahead for scheduled workouts.
const LOOKAHEAD_OPTIONS: &[i32] = &[7, 14, 21, 28, 30];

/// Options for how many days to look back for past workouts.
const LOOKBACK_OPTIONS: &[i32] = &[0, 3, 7, 14, 30];

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

/// Workout sync status for UI display.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WorkoutSyncStatus {
    /// No sync in progress, waiting for next scheduled sync
    #[default]
    Idle,
    /// Sync is in progress
    Syncing,
    /// Last sync completed successfully
    Success {
        /// Number of new workouts synced
        new_workouts: usize,
        /// Timestamp of successful sync
        timestamp: String,
    },
    /// Last sync failed
    Error(String),
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
    /// Toggle auto-sync rides setting
    ToggleAutoSyncRides(bool),
    /// Toggle sync workout plans setting
    ToggleSyncWorkoutPlans(bool),
    /// Change sync frequency (hours)
    SetSyncFrequency(u32),
    /// Update the full TrainingPeaks config
    UpdateConfig(TrainingPeaksPlatformConfig),
    /// Trigger manual workout plan sync now
    SyncWorkoutsNow,
    /// Set the number of days to look ahead for scheduled workouts
    SetLookaheadDays(i32),
    /// Set the number of days to look back for past workouts
    SetLookbackDays(i32),
}

/// TrainingPeaks settings screen state.
#[derive(Default)]
pub struct TrainingPeaksSettingsScreen {
    /// Current connection state
    pub connection_state: TrainingPeaksConnectionState,
    /// Athlete profile (when connected)
    pub athlete_profile: Option<AthleteProfile>,
    /// Base platform configuration (for backward compatibility)
    pub config: PlatformConfig,
    /// Extended TrainingPeaks-specific configuration
    pub tp_config: TrainingPeaksPlatformConfig,
    /// Number of pending uploads
    pub pending_uploads: usize,
    /// Last ride sync timestamp
    pub last_sync: Option<String>,
    /// Last workout plan sync timestamp
    pub last_workout_sync: Option<String>,
    /// Number of synced workouts
    pub synced_workouts_count: usize,
    /// Current workout sync status
    pub workout_sync_status: WorkoutSyncStatus,
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

    /// Set the TrainingPeaks-specific configuration.
    pub fn set_tp_config(&mut self, config: TrainingPeaksPlatformConfig) {
        self.tp_config = config;
    }

    /// Set the number of pending uploads.
    pub fn set_pending_uploads(&mut self, count: usize) {
        self.pending_uploads = count;
    }

    /// Set the last ride sync timestamp.
    pub fn set_last_sync(&mut self, timestamp: Option<String>) {
        self.last_sync = timestamp;
    }

    /// Set the last workout plan sync timestamp.
    pub fn set_last_workout_sync(&mut self, timestamp: Option<String>) {
        self.last_workout_sync = timestamp;
    }

    /// Set the number of synced workouts.
    pub fn set_synced_workouts_count(&mut self, count: usize) {
        self.synced_workouts_count = count;
    }

    /// Set the workout sync status.
    pub fn set_workout_sync_status(&mut self, status: WorkoutSyncStatus) {
        self.workout_sync_status = status;
    }

    /// Check if a workout sync is in progress.
    pub fn is_syncing_workouts(&self) -> bool {
        matches!(self.workout_sync_status, WorkoutSyncStatus::Syncing)
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

        // Ride Sync settings section
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.label(RichText::new("Ride Sync").size(14.0).strong());
                ui.add_space(12.0);

                // Auto-sync rides toggle
                ui.horizontal(|ui| {
                    let mut auto_sync_rides = self.tp_config.auto_sync_rides;
                    if ui
                        .checkbox(&mut auto_sync_rides, "Auto-sync rides")
                        .on_hover_text("Automatically upload rides after each session")
                        .changed()
                    {
                        self.tp_config.auto_sync_rides = auto_sync_rides;
                        self.config.auto_sync = auto_sync_rides;
                        action = Some(TrainingPeaksSettingsAction::ToggleAutoSyncRides(auto_sync_rides));
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
                        ui.label(RichText::new("Last ride sync:").weak());
                        ui.label(RichText::new(last_sync).weak());
                    });
                }
            });

        ui.add_space(16.0);

        // Workout Plan Sync settings section
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.label(RichText::new("Workout Plan Sync").size(14.0).strong());
                ui.add_space(12.0);

                // Sync workout plans toggle
                ui.horizontal(|ui| {
                    let mut sync_workouts = self.tp_config.sync_workout_plans;
                    if ui
                        .checkbox(&mut sync_workouts, "Sync workout plans")
                        .on_hover_text("Download scheduled workouts from TrainingPeaks")
                        .changed()
                    {
                        self.tp_config.sync_workout_plans = sync_workouts;
                        action = Some(TrainingPeaksSettingsAction::ToggleSyncWorkoutPlans(sync_workouts));
                    }
                });

                // Only show detailed options if workout sync is enabled
                if self.tp_config.sync_workout_plans {
                    ui.add_space(8.0);

                    // Sync frequency dropdown
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Auto-sync frequency:").weak());

                        let current_freq = self.tp_config.sync_frequency_hours;
                        let display = self.tp_config.sync_frequency_display();

                        egui::ComboBox::from_id_salt("sync_frequency")
                            .selected_text(display)
                            .show_ui(ui, |ui| {
                                for (hours, label) in TrainingPeaksPlatformConfig::sync_frequency_options() {
                                    if ui
                                        .selectable_value(&mut self.tp_config.sync_frequency_hours, *hours, *label)
                                        .clicked()
                                    {
                                        if self.tp_config.sync_frequency_hours != current_freq {
                                            action = Some(TrainingPeaksSettingsAction::SetSyncFrequency(*hours));
                                        }
                                    }
                                }
                            });
                    });

                    ui.add_space(12.0);

                    // Date range selection section
                    ui.label(RichText::new("Date Range").size(12.0).weak());
                    ui.add_space(4.0);

                    // Lookahead days selection
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Look ahead:").weak());

                        let current_lookahead = self.tp_config.lookahead_days;

                        egui::ComboBox::from_id_salt("lookahead_days")
                            .selected_text(format!("{} days", self.tp_config.lookahead_days))
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for days in LOOKAHEAD_OPTIONS {
                                    if ui
                                        .selectable_value(&mut self.tp_config.lookahead_days, *days, format!("{} days", days))
                                        .clicked()
                                    {
                                        if self.tp_config.lookahead_days != current_lookahead {
                                            action = Some(TrainingPeaksSettingsAction::SetLookaheadDays(*days));
                                        }
                                    }
                                }
                            });

                        ui.label(RichText::new("(future workouts)").weak().small());
                    });

                    ui.add_space(4.0);

                    // Lookback days selection
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Look back:").weak());

                        let current_lookback = self.tp_config.lookback_days;

                        egui::ComboBox::from_id_salt("lookback_days")
                            .selected_text(format!("{} days", self.tp_config.lookback_days))
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for days in LOOKBACK_OPTIONS {
                                    if ui
                                        .selectable_value(&mut self.tp_config.lookback_days, *days, format!("{} days", days))
                                        .clicked()
                                    {
                                        if self.tp_config.lookback_days != current_lookback {
                                            action = Some(TrainingPeaksSettingsAction::SetLookbackDays(*days));
                                        }
                                    }
                                }
                            });

                        ui.label(RichText::new("(past workouts)").weak().small());
                    });

                    ui.add_space(12.0);

                    // Sync status display
                    self.render_workout_sync_status(ui);

                    ui.add_space(8.0);

                    // Workout sync statistics
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Synced workouts:").weak());
                        ui.label(RichText::new(format!("{}", self.synced_workouts_count)).weak());
                    });

                    if let Some(ref last_workout_sync) = self.last_workout_sync {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Last sync:").weak());
                            ui.label(RichText::new(last_workout_sync).weak());
                        });
                    }

                    ui.add_space(8.0);

                    // Manual sync button
                    ui.horizontal(|ui| {
                        let is_syncing = self.is_syncing_workouts();

                        let sync_button = egui::Button::new(
                            if is_syncing {
                                RichText::new("Syncing...").size(13.0)
                            } else {
                                RichText::new("Sync Now").size(13.0).color(Color32::WHITE)
                            }
                        )
                        .fill(if is_syncing { Color32::GRAY } else { TRAININGPEAKS_TEAL })
                        .min_size(Vec2::new(100.0, 32.0));

                        if ui.add_enabled(!is_syncing, sync_button).clicked() {
                            action = Some(TrainingPeaksSettingsAction::SyncWorkoutsNow);
                        }

                        if is_syncing {
                            ui.spinner();
                        }
                    });

                    ui.add_space(4.0);

                    // Cycling only filter hint
                    if self.tp_config.cycling_only {
                        ui.label(
                            RichText::new("Only cycling workouts are synced")
                                .weak()
                                .small()
                                .italics(),
                        );
                    }
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

    /// Render the workout sync status indicator.
    fn render_workout_sync_status(&self, ui: &mut Ui) {
        match &self.workout_sync_status {
            WorkoutSyncStatus::Idle => {
                // No special display for idle state
            }
            WorkoutSyncStatus::Syncing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Syncing workouts...").color(TRAININGPEAKS_TEAL));
                });
            }
            WorkoutSyncStatus::Success { new_workouts, timestamp } => {
                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 50, 30))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✓").color(Color32::from_rgb(52, 168, 83)));
                            if *new_workouts > 0 {
                                ui.label(
                                    RichText::new(format!("Synced {} new workout{}", new_workouts, if *new_workouts == 1 { "" } else { "s" }))
                                        .color(Color32::from_rgb(52, 168, 83)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Sync complete - no new workouts")
                                        .color(Color32::from_rgb(52, 168, 83)),
                                );
                            }
                            ui.label(RichText::new(format!("at {}", timestamp)).weak().small());
                        });
                    });
            }
            WorkoutSyncStatus::Error(error) => {
                egui::Frame::new()
                    .fill(Color32::from_rgb(50, 30, 30))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✗").color(Color32::from_rgb(234, 67, 53)));
                            ui.label(
                                RichText::new(format!("Sync failed: {}", error))
                                    .color(Color32::from_rgb(234, 67, 53)),
                            );
                        });
                    });
            }
        }
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
            TrainingPeaksSettingsAction::ToggleAutoSyncRides(true),
            TrainingPeaksSettingsAction::ToggleAutoSyncRides(true)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::ToggleAutoSyncRides(true),
            TrainingPeaksSettingsAction::ToggleAutoSyncRides(false)
        );
    }

    #[test]
    fn test_settings_action_sync_workout_plans() {
        assert_eq!(
            TrainingPeaksSettingsAction::ToggleSyncWorkoutPlans(true),
            TrainingPeaksSettingsAction::ToggleSyncWorkoutPlans(true)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::ToggleSyncWorkoutPlans(true),
            TrainingPeaksSettingsAction::ToggleSyncWorkoutPlans(false)
        );
    }

    #[test]
    fn test_settings_action_sync_frequency() {
        assert_eq!(
            TrainingPeaksSettingsAction::SetSyncFrequency(6),
            TrainingPeaksSettingsAction::SetSyncFrequency(6)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::SetSyncFrequency(6),
            TrainingPeaksSettingsAction::SetSyncFrequency(12)
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
    fn test_last_workout_sync_timestamp() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        assert!(screen.last_workout_sync.is_none());

        screen.set_last_workout_sync(Some("2024-01-15 11:00 AM".to_string()));
        assert_eq!(screen.last_workout_sync, Some("2024-01-15 11:00 AM".to_string()));

        screen.set_last_workout_sync(None);
        assert!(screen.last_workout_sync.is_none());
    }

    #[test]
    fn test_synced_workouts_count() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        assert_eq!(screen.synced_workouts_count, 0);

        screen.set_synced_workouts_count(15);
        assert_eq!(screen.synced_workouts_count, 15);
    }

    #[test]
    fn test_tp_config() {
        let mut screen = TrainingPeaksSettingsScreen::new();

        // Check defaults
        assert!(screen.tp_config.auto_sync_rides);
        assert!(screen.tp_config.sync_workout_plans);
        assert_eq!(screen.tp_config.sync_frequency_hours, 6);

        // Update config
        let mut config = TrainingPeaksPlatformConfig::default();
        config.auto_sync_rides = false;
        config.sync_workout_plans = false;
        config.sync_frequency_hours = 12;

        screen.set_tp_config(config);

        assert!(!screen.tp_config.auto_sync_rides);
        assert!(!screen.tp_config.sync_workout_plans);
        assert_eq!(screen.tp_config.sync_frequency_hours, 12);
    }

    #[test]
    fn test_disconnecting_state() {
        let mut screen = TrainingPeaksSettingsScreen::new();
        screen.set_connection_state(TrainingPeaksConnectionState::Disconnecting);

        assert!(!screen.is_connected());
        assert_eq!(screen.connection_state, TrainingPeaksConnectionState::Disconnecting);
    }

    #[test]
    fn test_update_config_action() {
        let config = TrainingPeaksPlatformConfig {
            enabled: true,
            auto_sync_rides: true,
            sync_workout_plans: true,
            sync_frequency_hours: 12,
            lookahead_days: 14,
            lookback_days: 7,
            cycling_only: true,
        };

        let action = TrainingPeaksSettingsAction::UpdateConfig(config.clone());

        if let TrainingPeaksSettingsAction::UpdateConfig(c) = action {
            assert!(c.enabled);
            assert!(c.auto_sync_rides);
            assert!(c.sync_workout_plans);
            assert_eq!(c.sync_frequency_hours, 12);
        } else {
            panic!("Expected UpdateConfig action");
        }
    }

    #[test]
    fn test_workout_sync_status_default() {
        let status = WorkoutSyncStatus::default();
        assert_eq!(status, WorkoutSyncStatus::Idle);
    }

    #[test]
    fn test_workout_sync_status_syncing() {
        let status = WorkoutSyncStatus::Syncing;
        assert_eq!(status, WorkoutSyncStatus::Syncing);
    }

    #[test]
    fn test_workout_sync_status_success() {
        let status = WorkoutSyncStatus::Success {
            new_workouts: 5,
            timestamp: "10:30 AM".to_string(),
        };

        if let WorkoutSyncStatus::Success { new_workouts, timestamp } = status {
            assert_eq!(new_workouts, 5);
            assert_eq!(timestamp, "10:30 AM");
        } else {
            panic!("Expected Success status");
        }
    }

    #[test]
    fn test_workout_sync_status_error() {
        let error_msg = "Network timeout".to_string();
        let status = WorkoutSyncStatus::Error(error_msg.clone());

        if let WorkoutSyncStatus::Error(err) = status {
            assert_eq!(err, error_msg);
        } else {
            panic!("Expected Error status");
        }
    }

    #[test]
    fn test_set_workout_sync_status() {
        let mut screen = TrainingPeaksSettingsScreen::new();

        // Default is Idle
        assert_eq!(screen.workout_sync_status, WorkoutSyncStatus::Idle);

        // Set to Syncing
        screen.set_workout_sync_status(WorkoutSyncStatus::Syncing);
        assert_eq!(screen.workout_sync_status, WorkoutSyncStatus::Syncing);

        // Set to Success
        screen.set_workout_sync_status(WorkoutSyncStatus::Success {
            new_workouts: 3,
            timestamp: "11:00 AM".to_string(),
        });
        if let WorkoutSyncStatus::Success { new_workouts, .. } = &screen.workout_sync_status {
            assert_eq!(*new_workouts, 3);
        } else {
            panic!("Expected Success status");
        }

        // Set to Error
        screen.set_workout_sync_status(WorkoutSyncStatus::Error("API error".to_string()));
        if let WorkoutSyncStatus::Error(err) = &screen.workout_sync_status {
            assert_eq!(err, "API error");
        } else {
            panic!("Expected Error status");
        }
    }

    #[test]
    fn test_is_syncing_workouts() {
        let mut screen = TrainingPeaksSettingsScreen::new();

        // Not syncing by default
        assert!(!screen.is_syncing_workouts());

        // Set to syncing
        screen.set_workout_sync_status(WorkoutSyncStatus::Syncing);
        assert!(screen.is_syncing_workouts());

        // Set to success - not syncing
        screen.set_workout_sync_status(WorkoutSyncStatus::Success {
            new_workouts: 0,
            timestamp: "12:00 PM".to_string(),
        });
        assert!(!screen.is_syncing_workouts());

        // Set to error - not syncing
        screen.set_workout_sync_status(WorkoutSyncStatus::Error("Test".to_string()));
        assert!(!screen.is_syncing_workouts());
    }

    #[test]
    fn test_sync_workouts_now_action() {
        assert_eq!(
            TrainingPeaksSettingsAction::SyncWorkoutsNow,
            TrainingPeaksSettingsAction::SyncWorkoutsNow
        );
    }

    #[test]
    fn test_set_lookahead_days_action() {
        assert_eq!(
            TrainingPeaksSettingsAction::SetLookaheadDays(14),
            TrainingPeaksSettingsAction::SetLookaheadDays(14)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::SetLookaheadDays(7),
            TrainingPeaksSettingsAction::SetLookaheadDays(14)
        );
    }

    #[test]
    fn test_set_lookback_days_action() {
        assert_eq!(
            TrainingPeaksSettingsAction::SetLookbackDays(7),
            TrainingPeaksSettingsAction::SetLookbackDays(7)
        );
        assert_ne!(
            TrainingPeaksSettingsAction::SetLookbackDays(3),
            TrainingPeaksSettingsAction::SetLookbackDays(7)
        );
    }

    #[test]
    fn test_lookahead_options() {
        // Verify lookahead options are reasonable values
        assert!(LOOKAHEAD_OPTIONS.contains(&7));
        assert!(LOOKAHEAD_OPTIONS.contains(&14));
        assert!(LOOKAHEAD_OPTIONS.contains(&28));
        assert!(!LOOKAHEAD_OPTIONS.is_empty());
    }

    #[test]
    fn test_lookback_options() {
        // Verify lookback options include 0 (no lookback) and reasonable values
        assert!(LOOKBACK_OPTIONS.contains(&0));
        assert!(LOOKBACK_OPTIONS.contains(&7));
        assert!(LOOKBACK_OPTIONS.contains(&14));
        assert!(!LOOKBACK_OPTIONS.is_empty());
    }

    #[test]
    fn test_date_range_config() {
        let mut screen = TrainingPeaksSettingsScreen::new();

        // Check default lookahead/lookback values
        assert_eq!(screen.tp_config.lookahead_days, 14);
        assert_eq!(screen.tp_config.lookback_days, 7);

        // Update config with new date range
        let mut config = TrainingPeaksPlatformConfig::default();
        config.lookahead_days = 28;
        config.lookback_days = 3;
        screen.set_tp_config(config);

        assert_eq!(screen.tp_config.lookahead_days, 28);
        assert_eq!(screen.tp_config.lookback_days, 3);
    }
}
