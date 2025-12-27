//! T081: Streaming screen for external display management.
//!
//! Provides UI for starting/stopping the streaming server, displaying
//! PIN codes, QR codes, and managing connected sessions.

use egui::{Align, Color32, Layout, RichText, Ui};

use crate::integrations::streaming::{
    QrCodeData, StreamingConfig, StreamingServer, StreamingSession,
};

/// Streaming screen state.
pub struct StreamingScreen {
    /// Whether streaming is enabled in settings
    pub streaming_enabled: bool,
    /// Current server URL if running
    pub server_url: Option<String>,
    /// Current PIN if set
    pub current_pin: Option<String>,
    /// QR code data if available
    pub qr_code: Option<QrCodeData>,
    /// Connected sessions
    pub sessions: Vec<StreamingSession>,
    /// Error message if any
    pub error_message: Option<String>,
    /// Info message
    pub info_message: Option<String>,
}

/// Actions from streaming screen.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingAction {
    /// No action
    None,
    /// Go back to home
    Back,
    /// Start the streaming server
    StartServer,
    /// Stop the streaming server
    StopServer,
    /// Regenerate PIN
    RegeneratePin,
    /// Disconnect a session
    DisconnectSession { session_id: uuid::Uuid },
    /// Open settings
    OpenSettings,
}

impl StreamingScreen {
    /// Create a new streaming screen.
    pub fn new() -> Self {
        Self {
            streaming_enabled: false,
            server_url: None,
            current_pin: None,
            qr_code: None,
            sessions: Vec::new(),
            error_message: None,
            info_message: None,
        }
    }

    /// Update with current server state.
    pub fn update_from_server<S: StreamingServer>(&mut self, server: &S, config: &StreamingConfig) {
        self.streaming_enabled = config.enabled;
        self.server_url = server.get_url();
        self.current_pin = server.get_pin();
        self.qr_code = server.get_qr_code();
        self.sessions = server.get_sessions();
    }

    /// Set error message.
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error_message = Some(error.into());
        self.info_message = None;
    }

    /// Set info message.
    pub fn set_info(&mut self, info: impl Into<String>) {
        self.info_message = Some(info.into());
        self.error_message = None;
    }

    /// Clear messages.
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.info_message = None;
    }

    /// Render the streaming screen.
    pub fn show(&mut self, ui: &mut Ui) -> StreamingAction {
        let mut action = StreamingAction::None;

        // Header
        ui.horizontal(|ui| {
            if ui.button("< Back").clicked() {
                action = StreamingAction::Back;
            }
            ui.heading("External Display Streaming");
        });

        ui.separator();

        // Error message
        if let Some(ref error) = self.error_message {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Error: {}", error))
                        .color(Color32::from_rgb(234, 67, 53)),
                );
            });
            ui.add_space(8.0);
        }

        // Info message
        if let Some(ref info) = self.info_message {
            ui.horizontal(|ui| {
                ui.label(RichText::new(info).color(Color32::from_rgb(52, 168, 83)));
            });
            ui.add_space(8.0);
        }

        // Main content
        ui.add_space(16.0);

        if !self.streaming_enabled {
            // Streaming not enabled
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("Streaming is disabled")
                        .size(20.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(8.0);
                ui.label("Enable streaming in Settings to broadcast metrics to secondary devices.");
                ui.add_space(16.0);
                if ui.button("Open Settings").clicked() {
                    action = StreamingAction::OpenSettings;
                }
            });
        } else if self.server_url.is_some() {
            // Server is running
            action = self.render_server_running(ui);
        } else {
            // Server is stopped
            action = self.render_server_stopped(ui);
        }

        action
    }

    /// Render when server is running.
    fn render_server_running(&mut self, ui: &mut Ui) -> StreamingAction {
        let mut action = StreamingAction::None;

        // Server status
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Server Running")
                    .size(18.0)
                    .color(Color32::from_rgb(52, 168, 83))
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(RichText::new("Stop Server").color(Color32::from_rgb(234, 67, 53)))
                    .clicked()
                {
                    action = StreamingAction::StopServer;
                }
            });
        });

        ui.add_space(16.0);

        // Connection info
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.label(RichText::new("Connection Info").size(16.0).strong());
            ui.add_space(8.0);

            // URL
            if let Some(ref url) = self.server_url {
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.label(RichText::new(url).monospace().strong());
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(url.clone());
                    }
                });
            }

            // PIN
            if let Some(ref pin) = self.current_pin {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("PIN:");
                    ui.label(
                        RichText::new(pin)
                            .monospace()
                            .size(24.0)
                            .color(Color32::from_rgb(66, 133, 244))
                            .strong(),
                    );
                    if ui.button("Regenerate").clicked() {
                        action = StreamingAction::RegeneratePin;
                    }
                });
            }

            // QR code
            ui.add_space(16.0);
            ui.label(RichText::new("Scan QR Code:").small());
            ui.add_space(4.0);
            if let Some(ref qr) = self.qr_code {
                // Display ASCII QR code for now
                ui.label(RichText::new(&qr.ascii).monospace().small());
            } else {
                ui.label(RichText::new("[QR Code unavailable]").weak().italics());
            }
        });

        ui.add_space(16.0);

        // Connected sessions
        self.render_sessions(ui, &mut action);

        action
    }

    /// Render when server is stopped.
    fn render_server_stopped(&self, ui: &mut Ui) -> StreamingAction {
        let mut action = StreamingAction::None;

        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            ui.label(
                RichText::new("Server Stopped")
                    .size(20.0)
                    .color(Color32::GRAY),
            );

            ui.add_space(16.0);

            ui.label("Start the streaming server to broadcast metrics to secondary devices.");
            ui.add_space(8.0);
            ui.label(
                RichText::new("Devices on your local network can connect via web browser.").weak(),
            );

            ui.add_space(24.0);

            if ui
                .button(RichText::new("Start Server").size(16.0))
                .clicked()
            {
                action = StreamingAction::StartServer;
            }
        });

        action
    }

    /// Render connected sessions.
    fn render_sessions(&mut self, ui: &mut Ui, action: &mut StreamingAction) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Connected Devices").size(16.0).strong());
                ui.label(RichText::new(format!("({})", self.sessions.len())).weak());
            });

            ui.add_space(8.0);

            if self.sessions.is_empty() {
                ui.label(RichText::new("No devices connected").weak().italics());
            } else {
                for session in &self.sessions {
                    ui.horizontal(|ui| {
                        // Connection status indicator
                        let status_color = if session.authenticated {
                            Color32::from_rgb(52, 168, 83)
                        } else {
                            Color32::from_rgb(255, 165, 0)
                        };
                        ui.colored_label(status_color, "●");

                        // Client info
                        ui.label(&session.client_ip);
                        if let Some(ref ua) = session.user_agent {
                            ui.label(RichText::new(ua).weak().small());
                        }

                        // Connected time
                        let connected_secs = session.connected_at.elapsed().as_secs();
                        let time_str = if connected_secs < 60 {
                            format!("{}s", connected_secs)
                        } else if connected_secs < 3600 {
                            format!("{}m", connected_secs / 60)
                        } else {
                            format!("{}h", connected_secs / 3600)
                        };
                        ui.label(RichText::new(format!("({})", time_str)).weak());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Disconnect").clicked() {
                                *action = StreamingAction::DisconnectSession {
                                    session_id: session.id,
                                };
                            }
                        });
                    });
                    ui.separator();
                }
            }
        });
    }
}

impl Default for StreamingScreen {
    fn default() -> Self {
        Self::new()
    }
}
