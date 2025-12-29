//! Career progress screen.
//!
//! T078: Create career progress screen with level display.
//!
//! Displays career level, XP progress, milestone progress, and unlocked rewards.

use egui::{Align, Color32, Layout, Rect, RichText, ScrollArea, Stroke, StrokeKind, Ui, Vec2};

use crate::career::{
    CareerManager, CareerStatus, CosmeticType, MilestoneProgress, Reward, RewardType,
    all_level_rewards, is_milestone_level, level_title, next_milestone,
};

/// Career screen state.
pub struct CareerScreen {
    /// Currently selected tab.
    pub selected_tab: CareerTab,
    /// Selected reward for detail view.
    pub selected_reward: Option<String>,
    /// Filter for rewards display.
    pub reward_filter: RewardFilter,
}

/// Tabs in the career screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CareerTab {
    /// Overview with level and progress.
    #[default]
    Overview,
    /// All rewards gallery.
    Rewards,
    /// Level roadmap.
    Roadmap,
}

impl CareerTab {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Rewards => "Rewards",
            Self::Roadmap => "Roadmap",
        }
    }
}

/// Filter options for rewards display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RewardFilter {
    /// Show all rewards.
    #[default]
    All,
    /// Show unlocked rewards only.
    Unlocked,
    /// Show locked rewards only.
    Locked,
    /// Filter by type.
    Type(RewardType),
}

impl RewardFilter {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Unlocked => "Unlocked",
            Self::Locked => "Locked",
            Self::Type(t) => t.display_name(),
        }
    }
}

impl Default for CareerScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl CareerScreen {
    /// Create a new career screen.
    pub fn new() -> Self {
        Self {
            selected_tab: CareerTab::Overview,
            selected_reward: None,
            reward_filter: RewardFilter::All,
        }
    }

    /// Show the career screen.
    pub fn show(&mut self, ui: &mut Ui, manager: &CareerManager) {
        ui.vertical(|ui| {
            self.show_header(ui, manager.status());
            ui.add_space(8.0);
            self.show_tabs(ui);
            ui.add_space(16.0);

            match self.selected_tab {
                CareerTab::Overview => self.show_overview(ui, manager),
                CareerTab::Rewards => self.show_rewards(ui, manager),
                CareerTab::Roadmap => self.show_roadmap(ui, manager.status()),
            }
        });
    }

    /// Show header with level display.
    fn show_header(&self, ui: &mut Ui, status: &CareerStatus) {
        let is_milestone = is_milestone_level(status.current_level);

        ui.horizontal(|ui| {
            // Large level display
            let level_color = if is_milestone {
                Color32::from_rgb(255, 215, 0) // Gold
            } else {
                Color32::from_rgb(100, 149, 237) // Cornflower blue
            };

            // Level circle
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(80.0), egui::Sense::hover());
            let center = rect.center();

            // Background circle
            let bg_color = if is_milestone {
                Color32::from_rgb(139, 117, 0)
            } else {
                Color32::from_rgb(30, 60, 100)
            };
            ui.painter().circle_filled(center, 38.0, bg_color);
            ui.painter().circle_stroke(center, 38.0, Stroke::new(3.0, level_color));

            // Level number
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                status.current_level.to_string(),
                egui::FontId::proportional(32.0),
                level_color,
            );

            ui.add_space(16.0);

            // Level info
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(level_title(status.current_level))
                        .strong()
                        .size(24.0),
                );

                ui.label(
                    RichText::new(format!("Total XP: {}", format_xp(status.total_xp)))
                        .weak()
                        .size(14.0),
                );

                // XP progress bar
                if !status.is_max_level() {
                    ui.add_space(8.0);
                    self.show_xp_progress(ui, status);
                } else {
                    ui.label(
                        RichText::new("MAX LEVEL")
                            .color(Color32::from_rgb(255, 215, 0))
                            .strong(),
                    );
                }
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Stats summary
                let unlocked_count = status.unlocked_rewards.len();
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{} rewards", unlocked_count))
                            .size(14.0),
                    );
                    if let Some(milestone_progress) = next_milestone(status.current_level)
                        .map(|target| {
                            let remaining = target.saturating_sub(status.current_level);
                            format!("{} levels to milestone", remaining)
                        })
                    {
                        ui.label(RichText::new(milestone_progress).weak().size(12.0));
                    }
                });
            });
        });
    }

    /// Show XP progress bar.
    fn show_xp_progress(&self, ui: &mut Ui, status: &CareerStatus) {
        let progress = status.progress;
        let xp_into = status.xp_into_level;
        let xp_needed = status.xp_to_next + xp_into;

        ui.horizontal(|ui| {
            ui.label(RichText::new("XP:").weak().size(12.0));

            let available_width = ui.available_width().min(300.0);
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(available_width - 100.0, 12.0),
                egui::Sense::hover(),
            );

            // Background
            ui.painter().rect_filled(rect, 6.0, Color32::from_gray(60));

            // Fill
            let filled_width = rect.width() * progress;
            let filled_rect = Rect::from_min_size(rect.min, Vec2::new(filled_width, rect.height()));
            ui.painter().rect_filled(filled_rect, 6.0, Color32::from_rgb(100, 149, 237));

            ui.label(
                RichText::new(format!("{} / {}", xp_into, xp_needed))
                    .size(12.0),
            );
        });
    }

    /// Show tab selector.
    fn show_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for tab in [CareerTab::Overview, CareerTab::Rewards, CareerTab::Roadmap] {
                let selected = self.selected_tab == tab;
                let response = ui.selectable_label(selected, tab.label());
                if response.clicked() {
                    self.selected_tab = tab;
                }
            }
        });
        ui.separator();
    }

    /// Show overview tab.
    fn show_overview(&self, ui: &mut Ui, manager: &CareerManager) {
        let status = manager.status();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Milestone progress
                if let Some(progress) = manager.milestone_progress() {
                    self.show_milestone_progress(ui, &progress);
                    ui.add_space(16.0);
                }

                // Next level rewards
                let next_rewards = manager.next_level_rewards();
                if !next_rewards.is_empty() {
                    ui.heading("Next Level Rewards");
                    ui.add_space(8.0);
                    self.show_reward_cards(ui, &next_rewards, false);
                    ui.add_space(16.0);
                }

                // Recent unlocks
                let recent: Vec<_> = status.unlocked_rewards.iter().take(5).collect();
                if !recent.is_empty() {
                    ui.heading("Recent Unlocks");
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        for reward_id in recent {
                            self.show_unlocked_badge(ui, reward_id);
                        }
                    });
                }
            });
    }

    /// Show milestone progress section.
    fn show_milestone_progress(&self, ui: &mut Ui, progress: &MilestoneProgress) {
        egui::Frame::new()
            .fill(Color32::from_rgb(30, 30, 40))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Next Milestone: Level {}", progress.target_level))
                            .strong()
                            .size(16.0),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} levels away", progress.levels_remaining))
                                .weak(),
                        );
                    });
                });

                ui.add_space(8.0);

                // Progress bar
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 16.0),
                    egui::Sense::hover(),
                );

                ui.painter().rect_filled(rect, 8.0, Color32::from_gray(50));

                let filled_width = rect.width() * progress.progress;
                let filled_rect = Rect::from_min_size(rect.min, Vec2::new(filled_width, rect.height()));
                ui.painter().rect_filled(filled_rect, 8.0, Color32::from_rgb(255, 215, 0));

                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("{:.0}% complete", progress.progress * 100.0))
                        .size(12.0)
                        .weak(),
                );
            });
    }

    /// Show rewards tab.
    fn show_rewards(&mut self, ui: &mut Ui, manager: &CareerManager) {
        // Filter controls
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Filter")
                .selected_text(self.reward_filter.label())
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.reward_filter == RewardFilter::All, "All").clicked() {
                        self.reward_filter = RewardFilter::All;
                    }
                    if ui.selectable_label(self.reward_filter == RewardFilter::Unlocked, "Unlocked").clicked() {
                        self.reward_filter = RewardFilter::Unlocked;
                    }
                    if ui.selectable_label(self.reward_filter == RewardFilter::Locked, "Locked").clicked() {
                        self.reward_filter = RewardFilter::Locked;
                    }
                    ui.separator();
                    for reward_type in [
                        RewardType::JerseyColor,
                        RewardType::BikeFrame,
                        RewardType::WheelStyle,
                        RewardType::HelmetStyle,
                        RewardType::UiTheme,
                        RewardType::AccentColor,
                        RewardType::ProfileBadge,
                    ] {
                        if ui.selectable_label(
                            self.reward_filter == RewardFilter::Type(reward_type),
                            reward_type.display_name(),
                        ).clicked() {
                            self.reward_filter = RewardFilter::Type(reward_type);
                        }
                    }
                });
        });

        ui.add_space(8.0);

        // Get all rewards and filter
        let all_rewards = crate::career::all_rewards();
        let inventory = manager.inventory();

        let filtered: Vec<_> = all_rewards
            .iter()
            .filter(|r| match self.reward_filter {
                RewardFilter::All => true,
                RewardFilter::Unlocked => inventory.is_unlocked(&r.id),
                RewardFilter::Locked => !inventory.is_unlocked(&r.id),
                RewardFilter::Type(t) => r.reward_type == t,
            })
            .collect();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for reward in filtered {
                        let unlocked = inventory.is_unlocked(&reward.id);
                        self.show_reward_card(ui, reward, unlocked);
                    }
                });
            });
    }

    /// Show reward cards.
    fn show_reward_cards(&self, ui: &mut Ui, rewards: &[Reward], unlocked: bool) {
        ui.horizontal_wrapped(|ui| {
            for reward in rewards {
                self.show_reward_card(ui, reward, unlocked);
            }
        });
    }

    /// Show a single reward card.
    fn show_reward_card(&self, ui: &mut Ui, reward: &Reward, unlocked: bool) {
        let card_size = Vec2::new(140.0, 100.0);
        let (rect, response) = ui.allocate_exact_size(card_size, egui::Sense::click());

        let bg_color = if unlocked {
            reward_type_bg_color(reward.reward_type)
        } else {
            Color32::from_gray(35)
        };

        let border_color = if unlocked {
            reward_type_accent_color(reward.reward_type)
        } else {
            Color32::from_gray(50)
        };

        // Background
        ui.painter().rect_filled(rect, 6.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            6.0,
            Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, border_color),
            StrokeKind::Middle,
        );

        // Content
        let content_rect = rect.shrink(8.0);
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        child_ui.vertical(|ui| {
            // Type icon/label
            let type_text = reward_type_emoji(reward.reward_type);
            ui.label(
                RichText::new(type_text)
                    .size(20.0),
            );

            // Name
            let name_color = if unlocked {
                Color32::WHITE
            } else {
                Color32::from_gray(100)
            };
            ui.label(
                RichText::new(&reward.name)
                    .color(name_color)
                    .strong()
                    .size(11.0),
            );

            // Description (short)
            let desc = if reward.description.len() > 30 {
                format!("{}...", &reward.description[..27])
            } else {
                reward.description.clone()
            };
            ui.label(
                RichText::new(desc)
                    .color(Color32::from_gray(if unlocked { 180 } else { 80 }))
                    .size(9.0),
            );

            // Level requirement if locked
            if !unlocked {
                ui.label(
                    RichText::new(format!("Level {}", reward.unlock_level))
                        .color(Color32::from_gray(120))
                        .size(10.0),
                );
            }
        });
    }

    /// Show unlocked badge (compact).
    fn show_unlocked_badge(&self, ui: &mut Ui, reward_id: &str) {
        egui::Frame::new()
            .fill(Color32::from_gray(50))
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.label(RichText::new(reward_id).size(10.0));
            });
    }

    /// Show roadmap tab.
    fn show_roadmap(&self, ui: &mut Ui, status: &CareerStatus) {
        let current_level = status.current_level;
        let all_levels = all_level_rewards();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for level_def in all_levels {
                    let is_current = level_def.level == current_level;
                    let is_completed = level_def.level < current_level;
                    let is_milestone = is_milestone_level(level_def.level);

                    self.show_level_row(
                        ui,
                        level_def.level,
                        &level_def.title,
                        &level_def.rewards,
                        is_current,
                        is_completed,
                        is_milestone,
                    );
                }
            });
    }

    /// Show a level row in the roadmap.
    #[allow(clippy::too_many_arguments)]
    fn show_level_row(
        &self,
        ui: &mut Ui,
        level: u32,
        title: &str,
        rewards: &[Reward],
        is_current: bool,
        is_completed: bool,
        is_milestone: bool,
    ) {
        let bg_color = if is_current {
            Color32::from_rgb(30, 60, 100)
        } else if is_completed {
            Color32::from_rgb(25, 40, 25)
        } else {
            Color32::from_gray(25)
        };

        let border_color = if is_milestone {
            Color32::from_rgb(255, 215, 0)
        } else if is_current {
            Color32::from_rgb(100, 149, 237)
        } else {
            Color32::TRANSPARENT
        };

        egui::Frame::new()
            .fill(bg_color)
            .stroke(Stroke::new(if is_milestone { 2.0 } else { 0.0 }, border_color))
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Level indicator
                    let level_color = if is_milestone {
                        Color32::from_rgb(255, 215, 0)
                    } else if is_completed {
                        Color32::from_rgb(100, 200, 100)
                    } else if is_current {
                        Color32::from_rgb(100, 149, 237)
                    } else {
                        Color32::from_gray(120)
                    };

                    // Level badge
                    let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                    let badge_bg = if is_milestone {
                        Color32::from_rgb(139, 117, 0)
                    } else {
                        Color32::from_gray(40)
                    };
                    ui.painter().circle_filled(badge_rect.center(), 14.0, badge_bg);
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        level.to_string(),
                        egui::FontId::proportional(12.0),
                        level_color,
                    );

                    ui.add_space(8.0);

                    // Title and status
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(title)
                                    .strong()
                                    .color(if is_completed || is_current {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(150)
                                    }),
                            );

                            if is_current {
                                ui.label(
                                    RichText::new("CURRENT")
                                        .color(Color32::from_rgb(100, 149, 237))
                                        .size(10.0),
                                );
                            } else if is_completed {
                                ui.label(
                                    RichText::new("COMPLETE")
                                        .color(Color32::from_rgb(100, 200, 100))
                                        .size(10.0),
                                );
                            }

                            if is_milestone {
                                ui.label(
                                    RichText::new("MILESTONE")
                                        .color(Color32::from_rgb(255, 215, 0))
                                        .size(10.0),
                                );
                            }
                        });

                        // Rewards
                        if !rewards.is_empty() {
                            ui.horizontal(|ui| {
                                for reward in rewards {
                                    let emoji = reward_type_emoji(reward.reward_type);
                                    let color = if is_completed {
                                        reward_type_accent_color(reward.reward_type)
                                    } else {
                                        Color32::from_gray(100)
                                    };
                                    ui.label(
                                        RichText::new(format!("{} {}", emoji, &reward.name))
                                            .size(11.0)
                                            .color(color),
                                    );
                                }
                            });
                        }
                    });
                });
            });

        ui.add_space(2.0);
    }
}

/// Format XP for display.
fn format_xp(xp: u64) -> String {
    if xp >= 1_000_000 {
        format!("{:.1}M", xp as f64 / 1_000_000.0)
    } else if xp >= 1_000 {
        format!("{:.1}K", xp as f64 / 1_000.0)
    } else {
        xp.to_string()
    }
}

/// Get emoji for reward type.
fn reward_type_emoji(reward_type: RewardType) -> &'static str {
    match reward_type {
        RewardType::JerseyColor => "\u{1F455}",    // T-shirt
        RewardType::BikeFrame => "\u{1F6B2}",       // Bicycle
        RewardType::UiTheme => "\u{1F3A8}",         // Palette
        RewardType::AccentColor => "\u{1F308}",    // Rainbow
        RewardType::ProfileBadge => "\u{1F396}",   // Medal
        RewardType::WheelStyle => "\u{26AA}",       // Circle
        RewardType::HelmetStyle => "\u{26D1}",     // Helmet
    }
}

/// Get background color for reward type.
fn reward_type_bg_color(reward_type: RewardType) -> Color32 {
    match reward_type {
        RewardType::JerseyColor => Color32::from_rgb(50, 40, 60),
        RewardType::BikeFrame => Color32::from_rgb(40, 50, 40),
        RewardType::UiTheme => Color32::from_rgb(50, 50, 60),
        RewardType::AccentColor => Color32::from_rgb(60, 40, 50),
        RewardType::ProfileBadge => Color32::from_rgb(60, 55, 30),
        RewardType::WheelStyle => Color32::from_rgb(40, 45, 50),
        RewardType::HelmetStyle => Color32::from_rgb(45, 40, 50),
    }
}

/// Get accent color for reward type.
fn reward_type_accent_color(reward_type: RewardType) -> Color32 {
    match reward_type {
        RewardType::JerseyColor => Color32::from_rgb(180, 120, 200),
        RewardType::BikeFrame => Color32::from_rgb(120, 180, 120),
        RewardType::UiTheme => Color32::from_rgb(150, 150, 200),
        RewardType::AccentColor => Color32::from_rgb(200, 120, 150),
        RewardType::ProfileBadge => Color32::from_rgb(255, 215, 0),
        RewardType::WheelStyle => Color32::from_rgb(150, 170, 190),
        RewardType::HelmetStyle => Color32::from_rgb(170, 150, 180),
    }
}

/// Action from career screen.
#[derive(Debug, Clone)]
pub enum CareerAction {
    /// Go back to previous screen.
    Back,
    /// View reward detail.
    ViewReward(String),
    /// Equip a cosmetic.
    Equip(CosmeticType, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = CareerScreen::new();
        assert_eq!(screen.selected_tab, CareerTab::Overview);
        assert_eq!(screen.reward_filter, RewardFilter::All);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(CareerTab::Overview.label(), "Overview");
        assert_eq!(CareerTab::Rewards.label(), "Rewards");
        assert_eq!(CareerTab::Roadmap.label(), "Roadmap");
    }

    #[test]
    fn test_reward_filter_labels() {
        assert_eq!(RewardFilter::All.label(), "All");
        assert_eq!(RewardFilter::Unlocked.label(), "Unlocked");
        assert_eq!(RewardFilter::Locked.label(), "Locked");
    }

    #[test]
    fn test_reward_type_colors() {
        for reward_type in [
            RewardType::JerseyColor,
            RewardType::BikeFrame,
            RewardType::UiTheme,
            RewardType::AccentColor,
            RewardType::ProfileBadge,
            RewardType::WheelStyle,
            RewardType::HelmetStyle,
        ] {
            let bg = reward_type_bg_color(reward_type);
            let accent = reward_type_accent_color(reward_type);
            // Colors should be non-black
            assert!(bg.r() > 0 || bg.g() > 0 || bg.b() > 0);
            assert!(accent.r() > 0 || accent.g() > 0 || accent.b() > 0);
        }
    }
}
