//! Group ride screen.
//!
//! Provides LAN peer discovery, session hosting/joining, and participant display.

use egui::{Color32, RichText, Ui, Vec2};
use uuid::Uuid;

use crate::networking::{DiscoveryService, PeerInfo, SessionManager, SessionState};
use crate::networking::protocol::RiderMetrics;

/// Group ride screen actions.
#[derive(Debug, Clone)]
pub enum GroupRideAction {
    /// Start hosting a new session.
    HostSession { name: Option<String>, world_id: String },
    /// Join an existing session.
    JoinSession { peer: PeerInfo, session_id: Uuid },
    /// Leave the current session.
    LeaveSession,
    /// Refresh peer discovery.
    RefreshPeers,
    /// Navigate back.
    Back,
}

/// Group ride screen state.
pub struct GroupRideScreen {
    /// Session name input for hosting.
    session_name: String,
    /// Selected world ID.
    world_id: String,
    /// Selected peer index for joining.
    selected_peer_idx: Option<usize>,
    /// Error message to display.
    error_message: Option<String>,
}

impl Default for GroupRideScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupRideScreen {
    /// Create a new group ride screen.
    pub fn new() -> Self {
        Self {
            session_name: String::new(),
            world_id: "watopia".to_string(),
            selected_peer_idx: None,
            error_message: None,
        }
    }

    /// Set error message.
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Clear error message.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Render the group ride screen.
    #[allow(unused_assignments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        session_manager: &SessionManager,
        discovery_service: Option<&DiscoveryService>,
        peer_metrics: &std::collections::HashMap<Uuid, RiderMetrics>,
    ) -> Option<GroupRideAction> {
        let mut action = None;

        ui.heading("Group Ride");
        ui.add_space(10.0);

        // Show error if any
        if let Some(ref error) = self.error_message {
            ui.colored_label(Color32::RED, error);
            ui.add_space(5.0);
        }

        let state = session_manager.state();

        match state {
            SessionState::Idle => {
                action = self.show_idle_state(ui, discovery_service);
            }
            SessionState::Hosting | SessionState::Joined => {
                action = self.show_active_session(ui, session_manager, peer_metrics);
            }
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            action = Some(GroupRideAction::Back);
        }

        action
    }

    /// Show idle state - host or join options.
    fn show_idle_state(
        &mut self,
        ui: &mut Ui,
        discovery_service: Option<&DiscoveryService>,
    ) -> Option<GroupRideAction> {
        let mut action = None;

        // Host section
        ui.group(|ui| {
            ui.heading("Host a Group Ride");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Session Name:");
                ui.text_edit_singleline(&mut self.session_name);
            });

            ui.horizontal(|ui| {
                ui.label("World:");
                egui::ComboBox::from_id_salt("world_select")
                    .selected_text(&self.world_id)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.world_id, "watopia".to_string(), "Watopia");
                        ui.selectable_value(&mut self.world_id, "london".to_string(), "London");
                        ui.selectable_value(&mut self.world_id, "richmond".to_string(), "Richmond");
                        ui.selectable_value(&mut self.world_id, "innsbruck".to_string(), "Innsbruck");
                        ui.selectable_value(&mut self.world_id, "yorkshire".to_string(), "Yorkshire");
                    });
            });

            ui.add_space(5.0);

            if ui.button("Host Session").clicked() {
                let name = if self.session_name.is_empty() {
                    None
                } else {
                    Some(self.session_name.clone())
                };
                action = Some(GroupRideAction::HostSession {
                    name,
                    world_id: self.world_id.clone(),
                });
            }
        });

        ui.add_space(15.0);

        // Join section
        ui.group(|ui| {
            ui.heading("Join a Group Ride");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Available Sessions:");
                if ui.button("Refresh").clicked() {
                    action = Some(GroupRideAction::RefreshPeers);
                }
            });

            ui.add_space(5.0);

            if let Some(discovery) = discovery_service {
                let peers = discovery.peers();

                if peers.is_empty() {
                    ui.label(RichText::new("No sessions found on local network").italics());
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (idx, peer) in peers.iter().enumerate() {
                                let is_selected = self.selected_peer_idx == Some(idx);
                                let text = format!(
                                    "{} - {}",
                                    peer.rider_name,
                                    peer.world_id.as_deref().unwrap_or("Unknown World")
                                );

                                if ui.selectable_label(is_selected, &text).clicked() {
                                    self.selected_peer_idx = Some(idx);
                                }
                            }
                        });

                    ui.add_space(5.0);

                    if let Some(idx) = self.selected_peer_idx {
                        if idx < peers.len() {
                            let peer = &peers[idx];
                            if ui.button("Join Selected Session").clicked() {
                                if let Some(session_id) = peer.session_id {
                                    action = Some(GroupRideAction::JoinSession {
                                        peer: peer.clone(),
                                        session_id,
                                    });
                                }
                            }
                        }
                    }
                }
            } else {
                ui.label(RichText::new("Discovery service not available").italics());
            }
        });

        action
    }

    /// Show active session state.
    fn show_active_session(
        &mut self,
        ui: &mut Ui,
        session_manager: &SessionManager,
        peer_metrics: &std::collections::HashMap<Uuid, RiderMetrics>,
    ) -> Option<GroupRideAction> {
        let mut action = None;

        let state = session_manager.state();
        let session = session_manager.current_session();
        let participants = session_manager.participants();

        ui.group(|ui| {
            // Session header
            let status = match state {
                SessionState::Hosting => "Hosting",
                SessionState::Joined => "Joined",
                SessionState::Idle => "Idle",
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Status: {}", status)).strong());

                if let Some(ref s) = session {
                    ui.label(format!("| World: {}", s.world_id));
                    if let Some(ref name) = s.name {
                        ui.label(format!("| Session: {}", name));
                    }
                }
            });

            ui.add_space(10.0);

            // Participants list
            ui.heading(format!("Participants ({})", participants.len()));
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for participant in &participants {
                        ui.group(|ui| {
                            ui.set_min_size(Vec2::new(ui.available_width(), 0.0));

                            ui.horizontal(|ui| {
                                // Participant name
                                let name_text = if participant.is_host {
                                    format!("{} (Host)", participant.rider_name)
                                } else {
                                    participant.rider_name.clone()
                                };
                                ui.label(RichText::new(&name_text).strong());

                                ui.add_space(20.0);

                                // Show metrics if available
                                if let Some(metrics) = peer_metrics.get(&participant.rider_id) {
                                    ui.label(format!("{}W", metrics.power_watts));
                                    if let Some(cadence) = metrics.cadence_rpm {
                                        ui.label(format!("{}rpm", cadence));
                                    }
                                    if let Some(hr) = metrics.heart_rate_bpm {
                                        ui.label(format!("{}bpm", hr));
                                    }
                                    ui.label(format!("{:.1}km/h", metrics.speed_kmh));
                                }
                            });
                        });
                        ui.add_space(2.0);
                    }
                });
        });

        ui.add_space(10.0);

        if ui.button("Leave Session").clicked() {
            action = Some(GroupRideAction::LeaveSession);
        }

        action
    }
}
