//! Leaderboard screen.
//!
//! Displays segment leaderboards with rankings and export/import functionality.

use egui::{Color32, RichText, Ui};
use uuid::Uuid;

use crate::leaderboards::segments::{Segment, SegmentCategory};

/// Segment effort for display.
#[derive(Debug, Clone)]
pub struct SegmentEffort {
    pub id: Uuid,
    pub segment_id: Uuid,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub elapsed_time_secs: u32,
    pub avg_power_watts: Option<u16>,
}

/// Leaderboard screen actions.
#[derive(Debug, Clone)]
pub enum LeaderboardAction {
    /// View a specific segment's leaderboard.
    ViewSegment(Uuid),
    /// Export leaderboard data.
    ExportJson,
    /// Export to CSV.
    ExportCsv,
    /// Import leaderboard data.
    ImportJson,
    /// Navigate back.
    Back,
}

/// Leaderboard view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeaderboardView {
    /// List of segments.
    #[default]
    SegmentList,
    /// Specific segment's leaderboard.
    SegmentLeaderboard,
}

/// Leaderboard filter options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeaderboardFilter {
    /// Show all segments.
    #[default]
    All,
    /// Only climbs.
    Climbs,
    /// Only sprints.
    Sprints,
    /// Only flat segments.
    Flat,
}

/// Leaderboard screen state.
pub struct LeaderboardScreen {
    /// Current view mode.
    view: LeaderboardView,
    /// Current filter.
    filter: LeaderboardFilter,
    /// Selected segment ID.
    selected_segment_id: Option<Uuid>,
    /// Search text.
    search_text: String,
}

impl Default for LeaderboardScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl LeaderboardScreen {
    /// Create a new leaderboard screen.
    pub fn new() -> Self {
        Self {
            view: LeaderboardView::SegmentList,
            filter: LeaderboardFilter::All,
            selected_segment_id: None,
            search_text: String::new(),
        }
    }

    /// Render the leaderboard screen.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        segments: &[Segment],
        efforts: &[SegmentEffort],
        rider_id: Uuid,
    ) -> Option<LeaderboardAction> {
        let mut action = None;

        ui.heading("Leaderboards");
        ui.add_space(10.0);

        match self.view {
            LeaderboardView::SegmentList => {
                action = self.show_segment_list(ui, segments);
            }
            LeaderboardView::SegmentLeaderboard => {
                if let Some(segment_id) = self.selected_segment_id {
                    let segment = segments.iter().find(|s| s.id == segment_id);
                    let segment_efforts: Vec<_> = efforts
                        .iter()
                        .filter(|e| e.segment_id == segment_id)
                        .collect();

                    action = self.show_segment_leaderboard(ui, segment, &segment_efforts, rider_id);
                } else {
                    self.view = LeaderboardView::SegmentList;
                }
            }
        }

        ui.add_space(20.0);

        // Export/Import buttons
        ui.horizontal(|ui| {
            if ui.button("Export JSON").clicked() {
                action = Some(LeaderboardAction::ExportJson);
            }
            if ui.button("Export CSV").clicked() {
                action = Some(LeaderboardAction::ExportCsv);
            }
            if ui.button("Import").clicked() {
                action = Some(LeaderboardAction::ImportJson);
            }
        });

        ui.add_space(10.0);

        if ui.button("Back").clicked() {
            if self.view == LeaderboardView::SegmentLeaderboard {
                self.view = LeaderboardView::SegmentList;
                self.selected_segment_id = None;
            } else {
                action = Some(LeaderboardAction::Back);
            }
        }

        action
    }

    /// Show the list of segments.
    fn show_segment_list(
        &mut self,
        ui: &mut Ui,
        segments: &[Segment],
    ) -> Option<LeaderboardAction> {
        let mut action = None;

        // Filter controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.selectable_value(&mut self.filter, LeaderboardFilter::All, "All");
            ui.selectable_value(&mut self.filter, LeaderboardFilter::Climbs, "Climbs");
            ui.selectable_value(&mut self.filter, LeaderboardFilter::Sprints, "Sprints");
            ui.selectable_value(&mut self.filter, LeaderboardFilter::Flat, "Flat");
        });

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_text);
        });

        ui.add_space(10.0);

        // Filtered segments
        let filtered: Vec<_> = segments
            .iter()
            .filter(|s| {
                let matches_filter = match self.filter {
                    LeaderboardFilter::All => true,
                    LeaderboardFilter::Climbs => s.category == SegmentCategory::Climb,
                    LeaderboardFilter::Sprints => s.category == SegmentCategory::Sprint,
                    LeaderboardFilter::Flat => s.category == SegmentCategory::Mixed,
                };

                let matches_search = self.search_text.is_empty()
                    || s.name.to_lowercase().contains(&self.search_text.to_lowercase());

                matches_filter && matches_search
            })
            .collect();

        if filtered.is_empty() {
            ui.label(RichText::new("No segments found").italics());
        } else {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for segment in filtered {
                        if self.show_segment_card(ui, segment) {
                            self.selected_segment_id = Some(segment.id);
                            self.view = LeaderboardView::SegmentLeaderboard;
                            action = Some(LeaderboardAction::ViewSegment(segment.id));
                        }
                    }
                });
        }

        action
    }

    /// Show a segment card.
    fn show_segment_card(&self, ui: &mut Ui, segment: &Segment) -> bool {
        let mut clicked = false;

        egui::Frame::new()
            .fill(Color32::from_rgb(40, 40, 40))
            .inner_margin(10.0)
            .outer_margin(2.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Category icon
                    let icon = match segment.category {
                        SegmentCategory::Climb => "⛰",
                        SegmentCategory::Sprint => "⚡",
                        SegmentCategory::Mixed => "➡",
                    };
                    ui.label(RichText::new(icon).size(24.0));

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(&segment.name).strong());
                        ui.horizontal(|ui| {
                            ui.label(format!("{:.1} km", segment.length_m() / 1000.0));
                            ui.label("|");
                            ui.label(format!("{:.0}m elev", segment.elevation_gain_m));
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

    /// Show a segment's leaderboard.
    fn show_segment_leaderboard(
        &mut self,
        ui: &mut Ui,
        segment: Option<&Segment>,
        efforts: &[&SegmentEffort],
        rider_id: Uuid,
    ) -> Option<LeaderboardAction> {
        if let Some(segment) = segment {
            ui.label(RichText::new(&segment.name).size(20.0).strong());
            ui.horizontal(|ui| {
                ui.label(format!("{:.1} km", segment.length_m() / 1000.0));
                ui.label("|");
                ui.label(format!("{:.0}m elevation", segment.elevation_gain_m));
            });

            ui.add_space(15.0);

            // Leaderboard table
            ui.label(RichText::new("Leaderboard").strong());

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    // Header
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Rank").strong());
                        ui.add_space(30.0);
                        ui.label(RichText::new("Rider").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Power").strong());
                            ui.add_space(20.0);
                            ui.label(RichText::new("Time").strong());
                        });
                    });
                    ui.separator();

                    if efforts.is_empty() {
                        ui.label(RichText::new("No efforts recorded yet").italics());
                    } else {
                        for (rank, effort) in efforts.iter().enumerate() {
                            let is_user = effort.rider_id == rider_id;
                            let bg = if is_user {
                                Color32::from_rgb(40, 60, 80)
                            } else {
                                Color32::TRANSPARENT
                            };

                            egui::Frame::new().fill(bg).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Rank
                                    let rank_text = format!("#{}", rank + 1);
                                    let rank_color = match rank {
                                        0 => Color32::GOLD,
                                        1 => Color32::from_rgb(192, 192, 192),
                                        2 => Color32::from_rgb(205, 127, 50),
                                        _ => Color32::WHITE,
                                    };
                                    ui.label(RichText::new(&rank_text).color(rank_color).strong());

                                    ui.add_space(30.0);

                                    // Rider name
                                    let name = &effort.rider_name;
                                    if is_user {
                                        ui.label(RichText::new(name).strong());
                                    } else {
                                        ui.label(name);
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Power
                                            if let Some(power) = effort.avg_power_watts {
                                                ui.label(format!("{}W", power));
                                            } else {
                                                ui.label("--");
                                            }
                                            ui.add_space(20.0);

                                            // Time
                                            let mins = effort.elapsed_time_secs / 60;
                                            let secs = effort.elapsed_time_secs % 60;
                                            ui.label(format!("{}:{:02}", mins, secs));
                                        },
                                    );
                                });
                            });
                        }
                    }
                });
        } else {
            ui.label("Segment not found");
        }

        None
    }
}
