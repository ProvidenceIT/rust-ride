//! Rider profile screen.
//!
//! Displays and edits rider profile information, stats, and badges.

use egui::{Color32, RichText, Ui, Vec2};

use crate::social::types::{Badge, RiderProfile};

/// Rider profile screen actions.
#[derive(Debug, Clone)]
pub enum RiderProfileAction {
    /// Save profile changes.
    SaveProfile(RiderProfile),
    /// Change avatar.
    ChangeAvatar,
    /// Navigate back.
    Back,
}

/// Rider profile view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RiderProfileView {
    /// View mode.
    #[default]
    View,
    /// Edit mode.
    Edit,
}

/// Rider profile screen state.
pub struct RiderProfileScreen {
    /// Current view mode.
    view: RiderProfileView,
    /// Edited display name.
    edit_name: String,
    /// Edited bio.
    edit_bio: String,
    /// Edited sharing preference.
    edit_sharing_enabled: bool,
}

impl Default for RiderProfileScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl RiderProfileScreen {
    /// Create a new rider profile screen.
    pub fn new() -> Self {
        Self {
            view: RiderProfileView::View,
            edit_name: String::new(),
            edit_bio: String::new(),
            edit_sharing_enabled: true,
        }
    }

    /// Start editing with current profile values.
    fn start_editing(&mut self, profile: &RiderProfile) {
        self.edit_name = profile.display_name.clone();
        self.edit_bio = profile.bio.clone().unwrap_or_default();
        self.edit_sharing_enabled = profile.sharing_enabled;
        self.view = RiderProfileView::Edit;
    }

    /// Render the rider profile screen.
    #[allow(unused_assignments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
        badges: &[Badge],
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.heading("My Profile");
        ui.add_space(10.0);

        match self.view {
            RiderProfileView::View => {
                action = self.show_view_mode(ui, profile, badges);
            }
            RiderProfileView::Edit => {
                action = self.show_edit_mode(ui, profile);
            }
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            if self.view == RiderProfileView::Edit {
                self.view = RiderProfileView::View;
            } else {
                action = Some(RiderProfileAction::Back);
            }
        }

        action
    }

    /// Show view mode.
    fn show_view_mode(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
        badges: &[Badge],
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            // Avatar
            let avatar_size = 80.0;
            let (rect, response) =
                ui.allocate_exact_size(Vec2::splat(avatar_size), egui::Sense::click());
            ui.painter().circle_filled(
                rect.center(),
                avatar_size / 2.0,
                Color32::from_rgb(80, 100, 120),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &profile
                    .display_name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string(),
                egui::FontId::proportional(36.0),
                Color32::WHITE,
            );

            if response.clicked() {
                action = Some(RiderProfileAction::ChangeAvatar);
            }

            ui.add_space(20.0);

            ui.vertical(|ui| {
                ui.label(RichText::new(&profile.display_name).size(24.0).strong());

                if let Some(ref bio) = profile.bio {
                    ui.label(bio);
                }

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if profile.sharing_enabled {
                        ui.label(RichText::new("Sharing: On").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("Sharing: Off").color(Color32::GRAY));
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui.button("Edit").clicked() {
                    self.start_editing(profile);
                }
            });
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(15.0);

        // Stats section
        ui.label(RichText::new("Statistics").strong().size(18.0));
        ui.add_space(10.0);

        egui::Grid::new("profile_stats")
            .num_columns(4)
            .spacing([40.0, 10.0])
            .show(ui, |ui| {
                // Row 1
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.0}", profile.total_distance_km))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("km ridden");
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.1}", profile.total_time_hours))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("hours");
                });
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{}", profile.total_rides))
                            .size(24.0)
                            .strong(),
                    );
                    ui.label("rides");
                });
                ui.vertical(|ui| {
                    if let Some(ftp) = profile.ftp {
                        ui.label(RichText::new(format!("{}", ftp)).size(24.0).strong());
                    } else {
                        ui.label(RichText::new("--").size(24.0));
                    }
                    ui.label("FTP (W)");
                });
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(15.0);

        // Badges section
        ui.label(RichText::new("Badges").strong().size(18.0));
        ui.add_space(10.0);

        if badges.is_empty() {
            ui.label(RichText::new("No badges earned yet").italics());
            ui.label("Complete challenges and reach milestones to earn badges!");
        } else {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for badge in badges {
                        self.show_badge(ui, badge);
                    }
                });
            });
        }

        action
    }

    /// Show edit mode.
    fn show_edit_mode(
        &mut self,
        ui: &mut Ui,
        profile: &RiderProfile,
    ) -> Option<RiderProfileAction> {
        let mut action = None;

        ui.label(RichText::new("Edit Profile").strong().size(18.0));
        ui.add_space(15.0);

        egui::Grid::new("profile_edit_form")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Display Name:");
                ui.text_edit_singleline(&mut self.edit_name);
                ui.end_row();

                ui.label("Bio:");
                ui.text_edit_multiline(&mut self.edit_bio);
                ui.end_row();

                ui.label("Share Activities:");
                ui.checkbox(
                    &mut self.edit_sharing_enabled,
                    "Allow others to see my rides",
                );
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                if !self.edit_name.is_empty() {
                    let mut updated = profile.clone();
                    updated.display_name = self.edit_name.clone();
                    updated.bio = if self.edit_bio.is_empty() {
                        None
                    } else {
                        Some(self.edit_bio.clone())
                    };
                    updated.sharing_enabled = self.edit_sharing_enabled;

                    action = Some(RiderProfileAction::SaveProfile(updated));
                    self.view = RiderProfileView::View;
                }
            }
            if ui.button("Cancel").clicked() {
                self.view = RiderProfileView::View;
            }
        });

        action
    }

    /// Show a badge.
    fn show_badge(&self, ui: &mut Ui, badge: &Badge) {
        let size = 60.0;

        egui::Frame::new()
            .fill(Color32::from_rgb(50, 50, 60))
            .inner_margin(8.0)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Badge icon
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());

                    let color = if badge.earned {
                        Color32::GOLD
                    } else {
                        Color32::from_rgb(60, 60, 60)
                    };

                    ui.painter()
                        .circle_filled(rect.center(), size / 2.0 - 5.0, color);

                    // Badge icon text
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &badge.icon,
                        egui::FontId::proportional(24.0),
                        if badge.earned {
                            Color32::BLACK
                        } else {
                            Color32::GRAY
                        },
                    );

                    // Badge name
                    let name = if badge.name.len() > 10 {
                        format!("{}...", &badge.name[..10])
                    } else {
                        badge.name.clone()
                    };
                    ui.label(RichText::new(&name).small());

                    // Progress if not earned
                    if !badge.earned {
                        let progress = badge.progress / badge.target;
                        ui.add(egui::ProgressBar::new(progress as f32).desired_width(size));
                    }
                });
            });
    }
}
