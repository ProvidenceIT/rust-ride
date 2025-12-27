//! Race lobby screen.
//!
//! Displays race creation, discovery, countdown, and results.

use egui::{Color32, RichText, Ui};
use uuid::Uuid;

use crate::networking::protocol::RiderMetrics;
use crate::racing::events::{RaceEvent, RaceStatus, RacerInfo};

/// Race lobby screen actions.
#[derive(Debug, Clone)]
pub enum RaceLobbyAction {
    /// Create a new race.
    CreateRace {
        name: String,
        route_id: String,
        distance_km: f64,
    },
    /// Join a race.
    JoinRace(Uuid),
    /// Leave a race.
    LeaveRace(Uuid),
    /// Start the race (host only).
    StartRace(Uuid),
    /// Navigate back.
    Back,
}

/// Race lobby view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RaceLobbyView {
    /// List of available races.
    #[default]
    List,
    /// Create race form.
    Create,
    /// Race waiting room.
    Lobby,
    /// Race in progress.
    Racing,
    /// Race results.
    Results,
}

/// Race lobby screen state.
pub struct RaceLobbyScreen {
    /// Current view mode.
    view: RaceLobbyView,
    /// Selected race ID.
    selected_race_id: Option<Uuid>,
    /// New race name.
    new_race_name: String,
    /// New race route.
    new_route_id: String,
    /// New race distance.
    new_distance: String,
    /// Countdown seconds remaining.
    countdown_seconds: u8,
}

impl Default for RaceLobbyScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RaceLobbyScreen {
    /// Create a new race lobby screen.
    pub fn new() -> Self {
        Self {
            view: RaceLobbyView::List,
            selected_race_id: None,
            new_race_name: String::new(),
            new_route_id: "watopia_flat".to_string(),
            new_distance: "10".to_string(),
            countdown_seconds: 0,
        }
    }

    /// Set countdown seconds.
    pub fn set_countdown(&mut self, seconds: u8) {
        self.countdown_seconds = seconds;
        if seconds > 0 {
            self.view = RaceLobbyView::Lobby;
        }
    }

    /// Transition to racing view.
    pub fn start_racing(&mut self) {
        self.view = RaceLobbyView::Racing;
    }

    /// Transition to results view.
    pub fn show_results(&mut self) {
        self.view = RaceLobbyView::Results;
    }

    /// Render the race lobby screen.
    #[allow(unused_assignments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        races: &[RaceEvent],
        current_race_participants: Option<&[RacerInfo]>,
        peer_metrics: &std::collections::HashMap<Uuid, RiderMetrics>,
        local_rider_id: Uuid,
    ) -> Option<RaceLobbyAction> {
        let mut action = None;

        ui.heading("Virtual Racing");
        ui.add_space(10.0);

        match self.view {
            RaceLobbyView::List => {
                action = self.show_race_list(ui, races);
            }
            RaceLobbyView::Create => {
                action = self.show_create_form(ui);
            }
            RaceLobbyView::Lobby => {
                let race = self
                    .selected_race_id
                    .and_then(|id| races.iter().find(|r| r.id == id));
                action = self.show_lobby(ui, race, current_race_participants, local_rider_id);
            }
            RaceLobbyView::Racing => {
                let race = self
                    .selected_race_id
                    .and_then(|id| races.iter().find(|r| r.id == id));
                action = self.show_racing(ui, race, current_race_participants, peer_metrics);
            }
            RaceLobbyView::Results => {
                let race = self
                    .selected_race_id
                    .and_then(|id| races.iter().find(|r| r.id == id));
                action = self.show_results_view(ui, race, current_race_participants);
            }
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            match self.view {
                RaceLobbyView::List => action = Some(RaceLobbyAction::Back),
                RaceLobbyView::Results => {
                    self.view = RaceLobbyView::List;
                    self.selected_race_id = None;
                }
                _ => {
                    if let Some(race_id) = self.selected_race_id {
                        action = Some(RaceLobbyAction::LeaveRace(race_id));
                    }
                    self.view = RaceLobbyView::List;
                    self.selected_race_id = None;
                }
            }
        }

        action
    }

    /// Show race list.
    fn show_race_list(&mut self, ui: &mut Ui, races: &[RaceEvent]) -> Option<RaceLobbyAction> {
        let mut action = None;

        if ui.button("Create Race").clicked() {
            self.view = RaceLobbyView::Create;
        }

        ui.add_space(15.0);

        let pending: Vec<_> = races
            .iter()
            .filter(|r| r.status == RaceStatus::Scheduled)
            .collect();

        if pending.is_empty() {
            ui.label(RichText::new("No races available").italics());
            ui.label("Create a race or wait for someone on your network to host one.");
        } else {
            ui.label(RichText::new("Available Races").strong());
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for race in pending {
                        if self.show_race_card(ui, race) {
                            self.selected_race_id = Some(race.id);
                            self.view = RaceLobbyView::Lobby;
                            action = Some(RaceLobbyAction::JoinRace(race.id));
                        }
                    }
                });
        }

        action
    }

    /// Show race card.
    fn show_race_card(&self, ui: &mut Ui, race: &RaceEvent) -> bool {
        let mut clicked = false;

        egui::Frame::new()
            .fill(Color32::from_rgb(45, 45, 55))
            .inner_margin(12.0)
            .outer_margin(3.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&race.name).strong().size(16.0));
                        ui.horizontal(|ui| {
                            ui.label(format!("{:.1} km", race.distance_km));
                            ui.label("|");
                            ui.label(format!("{} participants", race.participant_count));
                        });
                        ui.label(
                            RichText::new(format!("Host: {}", race.organizer.rider_name))
                                .small()
                                .weak(),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Join").clicked() {
                            clicked = true;
                        }
                    });
                });
            });

        clicked
    }

    /// Show create race form.
    fn show_create_form(&mut self, ui: &mut Ui) -> Option<RaceLobbyAction> {
        let mut action = None;

        ui.label(RichText::new("Create a Race").strong().size(18.0));
        ui.add_space(15.0);

        egui::Grid::new("create_race_form")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Race Name:");
                ui.text_edit_singleline(&mut self.new_race_name);
                ui.end_row();

                ui.label("Route:");
                egui::ComboBox::from_id_salt("route_select")
                    .selected_text(&self.new_route_id)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.new_route_id,
                            "watopia_flat".to_string(),
                            "Watopia Flat",
                        );
                        ui.selectable_value(
                            &mut self.new_route_id,
                            "watopia_hilly".to_string(),
                            "Watopia Hilly",
                        );
                        ui.selectable_value(
                            &mut self.new_route_id,
                            "london_loop".to_string(),
                            "London Loop",
                        );
                        ui.selectable_value(
                            &mut self.new_route_id,
                            "richmond_flat".to_string(),
                            "Richmond Flat",
                        );
                    });
                ui.end_row();

                ui.label("Distance (km):");
                ui.text_edit_singleline(&mut self.new_distance);
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Create").clicked() && !self.new_race_name.is_empty() {
                if let Ok(distance) = self.new_distance.parse::<f64>() {
                    action = Some(RaceLobbyAction::CreateRace {
                        name: self.new_race_name.clone(),
                        route_id: self.new_route_id.clone(),
                        distance_km: distance,
                    });
                    self.view = RaceLobbyView::Lobby;
                }
            }
            if ui.button("Cancel").clicked() {
                self.view = RaceLobbyView::List;
            }
        });

        action
    }

    /// Show race lobby.
    fn show_lobby(
        &mut self,
        ui: &mut Ui,
        race: Option<&RaceEvent>,
        participants: Option<&[RacerInfo]>,
        local_rider_id: Uuid,
    ) -> Option<RaceLobbyAction> {
        let mut action = None;

        if let Some(race) = race {
            let is_host = race.organizer.rider_id == local_rider_id;

            ui.label(RichText::new(&race.name).strong().size(20.0));
            ui.label(format!("{:.1} km on {}", race.distance_km, race.route_id));

            ui.add_space(15.0);

            // Countdown display
            if self.countdown_seconds > 0 {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Race starting in").size(18.0));
                    ui.label(
                        RichText::new(format!("{}", self.countdown_seconds))
                            .size(72.0)
                            .strong()
                            .color(Color32::YELLOW),
                    );
                });
            } else {
                ui.label("Waiting for host to start race...");

                if is_host {
                    ui.add_space(10.0);
                    if ui.button("Start Race").clicked() {
                        action = Some(RaceLobbyAction::StartRace(race.id));
                    }
                }
            }

            ui.add_space(15.0);

            // Participants
            ui.label(RichText::new("Participants").strong());
            ui.add_space(5.0);

            if let Some(participants) = participants {
                for racer in participants {
                    ui.horizontal(|ui| {
                        if racer.rider_id == race.organizer.rider_id {
                            ui.label(RichText::new(&racer.rider_name).strong());
                            ui.label(RichText::new("(Host)").small().color(Color32::GOLD));
                        } else if racer.rider_id == local_rider_id {
                            ui.label(RichText::new(&racer.rider_name).strong());
                            ui.label(RichText::new("(You)").small());
                        } else {
                            ui.label(&racer.rider_name);
                        }
                    });
                }
            }
        } else {
            ui.label("Race not found");
        }

        action
    }

    /// Show racing view.
    fn show_racing(
        &mut self,
        ui: &mut Ui,
        race: Option<&RaceEvent>,
        participants: Option<&[RacerInfo]>,
        metrics: &std::collections::HashMap<Uuid, RiderMetrics>,
    ) -> Option<RaceLobbyAction> {
        if let Some(race) = race {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("RACE IN PROGRESS")
                        .size(24.0)
                        .strong()
                        .color(Color32::GREEN),
                );
            });

            ui.add_space(10.0);
            ui.label(RichText::new(&race.name).size(18.0));
            ui.label(format!("Target: {:.1} km", race.distance_km));

            ui.add_space(20.0);

            // Live positions
            ui.label(RichText::new("Positions").strong());
            ui.add_space(5.0);

            if let Some(participants) = participants {
                // Sort by distance (would need actual distance data)
                let mut sorted: Vec<_> = participants.iter().collect();
                sorted.sort_by(|a, b| {
                    let dist_a = metrics
                        .get(&a.rider_id)
                        .map(|m| m.distance_m)
                        .unwrap_or(0.0);
                    let dist_b = metrics
                        .get(&b.rider_id)
                        .map(|m| m.distance_m)
                        .unwrap_or(0.0);
                    dist_b
                        .partial_cmp(&dist_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                for (pos, racer) in sorted.iter().enumerate() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(35, 40, 45))
                        .inner_margin(8.0)
                        .outer_margin(2.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Position
                                let pos_color = match pos {
                                    0 => Color32::GOLD,
                                    1 => Color32::from_rgb(192, 192, 192),
                                    2 => Color32::from_rgb(205, 127, 50),
                                    _ => Color32::WHITE,
                                };
                                ui.label(
                                    RichText::new(format!("#{}", pos + 1))
                                        .strong()
                                        .color(pos_color),
                                );

                                ui.add_space(10.0);
                                ui.label(&racer.rider_name);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(m) = metrics.get(&racer.rider_id) {
                                            ui.label(format!("{:.2} km", m.distance_m / 1000.0));
                                            ui.label("|");
                                            ui.label(format!("{}W", m.power_watts));
                                            ui.label("|");
                                            ui.label(format!("{:.1} km/h", m.speed_kmh));
                                        }
                                    },
                                );
                            });
                        });
                }
            }
        }

        None
    }

    /// Show results view.
    fn show_results_view(
        &mut self,
        ui: &mut Ui,
        race: Option<&RaceEvent>,
        participants: Option<&[RacerInfo]>,
    ) -> Option<RaceLobbyAction> {
        if let Some(race) = race {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("RACE COMPLETE")
                        .size(24.0)
                        .strong()
                        .color(Color32::GREEN),
                );
            });

            ui.add_space(10.0);
            ui.label(RichText::new(&race.name).size(18.0));

            ui.add_space(20.0);

            // Final standings
            ui.label(RichText::new("Final Standings").strong().size(16.0));
            ui.add_space(10.0);

            if let Some(participants) = participants {
                for (pos, racer) in participants.iter().enumerate() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(40, 45, 50))
                        .inner_margin(10.0)
                        .outer_margin(2.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let pos_color = match pos {
                                    0 => Color32::GOLD,
                                    1 => Color32::from_rgb(192, 192, 192),
                                    2 => Color32::from_rgb(205, 127, 50),
                                    _ => Color32::WHITE,
                                };

                                let medal = match pos {
                                    0 => "🥇",
                                    1 => "🥈",
                                    2 => "🥉",
                                    _ => "",
                                };

                                ui.label(
                                    RichText::new(format!("#{}", pos + 1))
                                        .strong()
                                        .color(pos_color)
                                        .size(18.0),
                                );
                                ui.add_space(5.0);
                                ui.label(medal);
                                ui.add_space(10.0);
                                ui.label(RichText::new(&racer.rider_name).size(16.0));
                            });
                        });
                }
            }

            ui.add_space(20.0);

            if ui.button("Back to Races").clicked() {
                self.view = RaceLobbyView::List;
                self.selected_race_id = None;
            }
        }

        None
    }
}
