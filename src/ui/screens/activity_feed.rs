//! Activity feed screen.
//!
//! Displays ride summaries from LAN peers.

use egui::{Color32, RichText, Ui};
use uuid::Uuid;

use crate::storage::social_store::ActivitySummary;

/// Activity feed screen actions.
#[derive(Debug, Clone)]
pub enum ActivityFeedAction {
    /// View activity details.
    ViewActivity(Uuid),
    /// Refresh feed.
    Refresh,
    /// Navigate back.
    Back,
}

/// Activity feed screen state.
pub struct ActivityFeedScreen {
    /// Selected activity for detail view.
    #[allow(dead_code)]
    selected_activity: Option<Uuid>,
    /// Show only peer activities.
    peers_only: bool,
}

impl Default for ActivityFeedScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityFeedScreen {
    /// Create a new activity feed screen.
    pub fn new() -> Self {
        Self {
            selected_activity: None,
            peers_only: false,
        }
    }

    /// Render the activity feed screen.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        activities: &[ActivitySummary],
        local_rider_id: Uuid,
    ) -> Option<ActivityFeedAction> {
        let mut action = None;

        ui.heading("Activity Feed");
        ui.add_space(10.0);

        // Controls
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                action = Some(ActivityFeedAction::Refresh);
            }
            ui.checkbox(&mut self.peers_only, "Peers only");
        });

        ui.add_space(15.0);

        // Filter activities
        let filtered: Vec<_> = activities
            .iter()
            .filter(|a| {
                if self.peers_only {
                    a.rider_id != local_rider_id
                } else {
                    true
                }
            })
            .collect();

        if filtered.is_empty() {
            ui.label(RichText::new("No activities to show").italics());
            ui.label("Complete a ride or connect with peers on your network.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(500.0)
                .show(ui, |ui| {
                    for activity in filtered {
                        if self.show_activity_card(
                            ui,
                            activity,
                            activity.rider_id == local_rider_id,
                        ) {
                            action = Some(ActivityFeedAction::ViewActivity(activity.id));
                        }
                    }
                });
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            action = Some(ActivityFeedAction::Back);
        }

        action
    }

    /// Show an activity card.
    fn show_activity_card(&self, ui: &mut Ui, activity: &ActivitySummary, is_local: bool) -> bool {
        let mut clicked = false;

        let bg_color = if is_local {
            Color32::from_rgb(40, 50, 60)
        } else {
            Color32::from_rgb(35, 35, 40)
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(12.0)
            .outer_margin(3.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Avatar placeholder
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(40.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(rect.center(), 20.0, Color32::from_rgb(80, 80, 100));

                    ui.add_space(12.0);

                    ui.vertical(|ui| {
                        // Rider name and time
                        ui.horizontal(|ui| {
                            let name_text = if is_local {
                                RichText::new(&activity.rider_name).strong()
                            } else {
                                RichText::new(&activity.rider_name)
                            };
                            ui.label(name_text);

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(
                                            activity.recorded_at.format("%b %d, %H:%M").to_string(),
                                        )
                                        .small()
                                        .weak(),
                                    );
                                },
                            );
                        });

                        ui.add_space(5.0);

                        // Activity stats
                        ui.horizontal(|ui| {
                            // Distance
                            ui.label(format!("{:.1} km", activity.distance_km));
                            ui.label("|");

                            // Duration
                            let hours = activity.duration_minutes / 60;
                            let mins = activity.duration_minutes % 60;
                            if hours > 0 {
                                ui.label(format!("{}h {}m", hours, mins));
                            } else {
                                ui.label(format!("{}m", mins));
                            }
                            ui.label("|");

                            // Power if available
                            if let Some(power) = activity.avg_power_watts {
                                ui.label(format!("{}W avg", power));
                                ui.label("|");
                            }

                            // Elevation
                            if activity.elevation_gain_m > 0.0 {
                                ui.label(format!("{:.0}m", activity.elevation_gain_m));
                            }
                        });

                        // World info
                        if let Some(ref world_id) = activity.world_id {
                            ui.label(RichText::new(format!("World: {}", world_id)).small().weak());
                        }
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
}
