//! Clubs screen.
//!
//! Displays club management: creation, joining, and member roster.

use egui::{Color32, RichText, Ui};
use uuid::Uuid;

use crate::social::clubs::{ClubInfo, ClubMember};

/// Clubs screen actions.
#[derive(Debug, Clone)]
pub enum ClubsAction {
    /// Create a new club.
    CreateClub { name: String, description: Option<String> },
    /// Join a club by code.
    JoinClub { join_code: String },
    /// Leave a club.
    LeaveClub(Uuid),
    /// View club details.
    ViewClub(Uuid),
    /// Navigate back.
    Back,
}

/// Clubs view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClubsView {
    /// List of clubs.
    #[default]
    List,
    /// Create new club form.
    Create,
    /// Join club form.
    Join,
    /// Club detail view.
    Detail,
}

/// Clubs screen state.
pub struct ClubsScreen {
    /// Current view mode.
    view: ClubsView,
    /// New club name.
    new_club_name: String,
    /// New club description.
    new_club_description: String,
    /// Join code input.
    join_code: String,
    /// Selected club for detail view.
    selected_club_id: Option<Uuid>,
    /// Error message.
    error_message: Option<String>,
}

impl Default for ClubsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ClubsScreen {
    /// Create a new clubs screen.
    pub fn new() -> Self {
        Self {
            view: ClubsView::List,
            new_club_name: String::new(),
            new_club_description: String::new(),
            join_code: String::new(),
            selected_club_id: None,
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

    /// Render the clubs screen.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        clubs: &[ClubInfo],
        selected_club_members: Option<&[ClubMember]>,
        rider_id: Uuid,
    ) -> Option<ClubsAction> {
        let mut action = None;

        ui.heading("Clubs");
        ui.add_space(10.0);

        // Show error if any
        if let Some(ref error) = self.error_message {
            ui.colored_label(Color32::RED, error);
            ui.add_space(5.0);
        }

        match self.view {
            ClubsView::List => {
                action = self.show_club_list(ui, clubs, rider_id);
            }
            ClubsView::Create => {
                action = self.show_create_form(ui);
            }
            ClubsView::Join => {
                action = self.show_join_form(ui);
            }
            ClubsView::Detail => {
                if let Some(club_id) = self.selected_club_id {
                    let club = clubs.iter().find(|c| c.id == club_id);
                    action = self.show_club_detail(ui, club, selected_club_members, rider_id);
                } else {
                    self.view = ClubsView::List;
                }
            }
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            match self.view {
                ClubsView::List => action = Some(ClubsAction::Back),
                _ => {
                    self.view = ClubsView::List;
                    self.selected_club_id = None;
                }
            }
        }

        action
    }

    /// Show the club list.
    fn show_club_list(
        &mut self,
        ui: &mut Ui,
        clubs: &[ClubInfo],
        rider_id: Uuid,
    ) -> Option<ClubsAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            if ui.button("Create Club").clicked() {
                self.view = ClubsView::Create;
            }
            if ui.button("Join Club").clicked() {
                self.view = ClubsView::Join;
            }
        });

        ui.add_space(15.0);

        if clubs.is_empty() {
            ui.label(RichText::new("You haven't joined any clubs yet").italics());
            ui.label("Create a club or join one with a code!");
        } else {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for club in clubs {
                        let is_admin = club.admin_rider_id == rider_id;
                        if self.show_club_card(ui, club, is_admin) {
                            self.selected_club_id = Some(club.id);
                            self.view = ClubsView::Detail;
                            action = Some(ClubsAction::ViewClub(club.id));
                        }
                    }
                });
        }

        action
    }

    /// Show a club card.
    fn show_club_card(&self, ui: &mut Ui, club: &ClubInfo, is_admin: bool) -> bool {
        let mut clicked = false;

        egui::Frame::new()
            .fill(Color32::from_rgb(40, 45, 50))
            .inner_margin(12.0)
            .outer_margin(3.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&club.name).strong().size(16.0));
                            if is_admin {
                                ui.label(RichText::new("Admin").small().color(Color32::GOLD));
                            }
                        });

                        if let Some(ref desc) = club.description {
                            ui.label(RichText::new(desc).weak());
                        }

                        ui.add_space(5.0);

                        // Stats
                        ui.horizontal(|ui| {
                            ui.label(format!("{} members", club.member_count));
                            ui.label("|");
                            ui.label(format!("{:.0} km total", club.total_distance_km));
                            ui.label("|");
                            ui.label(format!("{:.0} hours", club.total_time_hours));
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("View").clicked() {
                            clicked = true;
                        }
                    });
                });
            });

        clicked
    }

    /// Show create club form.
    fn show_create_form(&mut self, ui: &mut Ui) -> Option<ClubsAction> {
        let mut action = None;

        ui.label(RichText::new("Create a New Club").strong().size(18.0));
        ui.add_space(15.0);

        egui::Grid::new("create_club_form")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Club Name:");
                ui.text_edit_singleline(&mut self.new_club_name);
                ui.end_row();

                ui.label("Description:");
                ui.text_edit_multiline(&mut self.new_club_description);
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Create").clicked() {
                if !self.new_club_name.is_empty() {
                    let description = if self.new_club_description.is_empty() {
                        None
                    } else {
                        Some(self.new_club_description.clone())
                    };
                    action = Some(ClubsAction::CreateClub {
                        name: self.new_club_name.clone(),
                        description,
                    });
                    self.view = ClubsView::List;
                    self.new_club_name.clear();
                    self.new_club_description.clear();
                }
            }
            if ui.button("Cancel").clicked() {
                self.view = ClubsView::List;
            }
        });

        action
    }

    /// Show join club form.
    fn show_join_form(&mut self, ui: &mut Ui) -> Option<ClubsAction> {
        let mut action = None;

        ui.label(RichText::new("Join a Club").strong().size(18.0));
        ui.add_space(15.0);

        ui.label("Enter the club's join code:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.join_code)
                    .desired_width(200.0)
                    .hint_text("e.g., ABCD1234")
            );

            // Auto-uppercase
            if response.changed() {
                self.join_code = self.join_code.to_uppercase();
            }
        });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Join").clicked() {
                if !self.join_code.is_empty() {
                    action = Some(ClubsAction::JoinClub {
                        join_code: self.join_code.clone(),
                    });
                    self.view = ClubsView::List;
                    self.join_code.clear();
                }
            }
            if ui.button("Cancel").clicked() {
                self.view = ClubsView::List;
            }
        });

        action
    }

    /// Show club detail view.
    fn show_club_detail(
        &mut self,
        ui: &mut Ui,
        club: Option<&ClubInfo>,
        members: Option<&[ClubMember]>,
        rider_id: Uuid,
    ) -> Option<ClubsAction> {
        let mut action = None;

        if let Some(club) = club {
            let is_admin = club.admin_rider_id == rider_id;

            ui.horizontal(|ui| {
                ui.label(RichText::new(&club.name).strong().size(20.0));
                if is_admin {
                    ui.label(RichText::new("(Admin)").color(Color32::GOLD));
                }
            });

            if let Some(ref desc) = club.description {
                ui.label(desc);
            }

            ui.add_space(10.0);

            // Join code (admin only)
            if is_admin {
                ui.horizontal(|ui| {
                    ui.label("Join Code:");
                    ui.label(RichText::new(&club.join_code).strong().monospace());
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(club.join_code.clone());
                    }
                });
            }

            ui.add_space(15.0);

            // Club stats
            ui.label(RichText::new("Club Stats").strong());
            egui::Grid::new("club_stats")
                .num_columns(2)
                .spacing([40.0, 5.0])
                .show(ui, |ui| {
                    ui.label("Members:");
                    ui.label(format!("{}", club.member_count));
                    ui.end_row();

                    ui.label("Total Distance:");
                    ui.label(format!("{:.1} km", club.total_distance_km));
                    ui.end_row();

                    ui.label("Total Time:");
                    ui.label(format!("{:.1} hours", club.total_time_hours));
                    ui.end_row();
                });

            ui.add_space(15.0);

            // Member roster
            ui.label(RichText::new("Members").strong());
            ui.add_space(5.0);

            if let Some(members) = members {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for member in members {
                            self.show_member_row(ui, member, member.rider_id == rider_id);
                        }
                    });
            } else {
                ui.label(RichText::new("Loading members...").italics());
            }

            ui.add_space(20.0);

            // Leave button (non-admin only)
            if !is_admin {
                if ui.button("Leave Club").clicked() {
                    action = Some(ClubsAction::LeaveClub(club.id));
                    self.view = ClubsView::List;
                    self.selected_club_id = None;
                }
            }
        } else {
            ui.label("Club not found");
        }

        action
    }

    /// Show a member row.
    fn show_member_row(&self, ui: &mut Ui, member: &ClubMember, is_self: bool) {
        egui::Frame::new()
            .fill(if is_self {
                Color32::from_rgb(40, 50, 60)
            } else {
                Color32::TRANSPARENT
            })
            .inner_margin(5.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Avatar placeholder
                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(24.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        rect.center(),
                        12.0,
                        Color32::from_rgb(80, 80, 100),
                    );

                    ui.add_space(8.0);

                    ui.label(&member.display_name);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1} km", member.distance_km)).small());
                        ui.label("|");
                        ui.label(RichText::new(format!("{:.1} h", member.time_hours)).small());
                    });
                });
            });
    }
}
