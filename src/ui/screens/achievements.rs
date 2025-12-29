//! Achievements gallery screen.
//!
//! T042: Create achievements gallery screen.
//!
//! Displays all available achievements, earned/unearned status, and progress.

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui};
use uuid::Uuid;

use crate::achievements::{
    Achievement, AchievementCategory, AchievementProgress, AchievementTier, AchievementTracker,
    DefaultAchievementTracker, XpStatus,
};

/// Filter options for achievements display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AchievementFilter {
    /// Show all achievements
    #[default]
    All,
    /// Show only earned achievements
    Earned,
    /// Show only unearned achievements
    Unearned,
    /// Filter by category
    Category(AchievementCategory),
    /// Filter by tier
    Tier(AchievementTier),
}

impl AchievementFilter {
    /// Get display label.
    pub fn label(&self) -> String {
        match self {
            Self::All => "All".to_string(),
            Self::Earned => "Earned".to_string(),
            Self::Unearned => "Locked".to_string(),
            Self::Category(cat) => cat.display_name().to_string(),
            Self::Tier(tier) => tier.display_name().to_string(),
        }
    }
}

/// Sort options for achievements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AchievementSort {
    /// Sort by category then tier
    #[default]
    Category,
    /// Sort by tier (hardest first)
    TierDesc,
    /// Sort by XP value (highest first)
    XpDesc,
    /// Sort by earned date (most recent first)
    RecentlyEarned,
    /// Sort by progress (closest to completion first)
    NearCompletion,
}

impl AchievementSort {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Category => "By Category",
            Self::TierDesc => "By Difficulty",
            Self::XpDesc => "By XP Value",
            Self::RecentlyEarned => "Recently Earned",
            Self::NearCompletion => "Near Completion",
        }
    }
}

/// Achievements screen state.
pub struct AchievementsScreen {
    /// All achievement definitions.
    pub achievements: Vec<Achievement>,
    /// Set of earned achievement IDs.
    pub earned_ids: std::collections::HashSet<Uuid>,
    /// Progress for in-progress achievements.
    pub progress: std::collections::HashMap<Uuid, AchievementProgress>,
    /// Current XP status.
    pub xp_status: XpStatus,
    /// Current filter.
    pub filter: AchievementFilter,
    /// Current sort order.
    pub sort: AchievementSort,
    /// Selected achievement for detail view.
    pub selected: Option<Uuid>,
    /// Category filter (None = all categories).
    pub category_filter: Option<AchievementCategory>,
    /// Show secret achievements.
    pub show_secrets: bool,
}

impl Default for AchievementsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl AchievementsScreen {
    /// Create a new achievements screen.
    pub fn new() -> Self {
        Self {
            achievements: crate::achievements::all_achievements(),
            earned_ids: std::collections::HashSet::new(),
            progress: std::collections::HashMap::new(),
            xp_status: XpStatus::default(),
            filter: AchievementFilter::All,
            sort: AchievementSort::Category,
            selected: None,
            category_filter: None,
            show_secrets: false,
        }
    }

    /// Load state from tracker.
    pub fn from_tracker(tracker: &DefaultAchievementTracker) -> Self {
        let mut screen = Self::new();
        screen.xp_status = tracker.xp_status();

        for earned in tracker.earned_achievements() {
            screen.earned_ids.insert(earned.achievement_id);
        }

        screen
    }

    /// Get filtered and sorted achievements.
    pub fn filtered_achievements(&self) -> Vec<&Achievement> {
        let mut achievements: Vec<_> = self
            .achievements
            .iter()
            .filter(|a| self.matches_filter(a))
            .collect();

        // Sort
        match self.sort {
            AchievementSort::Category => {
                achievements.sort_by(|a, b| {
                    a.category
                        .display_name()
                        .cmp(b.category.display_name())
                        .then_with(|| a.tier.cmp(&b.tier))
                });
            }
            AchievementSort::TierDesc => {
                achievements.sort_by(|a, b| b.tier.cmp(&a.tier));
            }
            AchievementSort::XpDesc => {
                achievements.sort_by_key(|a| std::cmp::Reverse(a.effective_xp()));
            }
            AchievementSort::RecentlyEarned => {
                // Earned first, then by tier
                achievements.sort_by(|a, b| {
                    let a_earned = self.earned_ids.contains(&a.id);
                    let b_earned = self.earned_ids.contains(&b.id);
                    b_earned.cmp(&a_earned).then_with(|| b.tier.cmp(&a.tier))
                });
            }
            AchievementSort::NearCompletion => {
                achievements.sort_by(|a, b| {
                    let a_progress = self.get_progress_percent(&a.id);
                    let b_progress = self.get_progress_percent(&b.id);
                    // Higher progress first (but not 100%, which means earned)
                    b_progress
                        .partial_cmp(&a_progress)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        achievements
    }

    /// Check if achievement matches current filter.
    fn matches_filter(&self, achievement: &Achievement) -> bool {
        // Hide secrets unless earned or show_secrets enabled
        if achievement.is_secret && !self.show_secrets && !self.earned_ids.contains(&achievement.id)
        {
            return false;
        }

        // Category filter
        if let Some(cat) = self.category_filter {
            if achievement.category != cat {
                return false;
            }
        }

        // Main filter
        match self.filter {
            AchievementFilter::All => true,
            AchievementFilter::Earned => self.earned_ids.contains(&achievement.id),
            AchievementFilter::Unearned => !self.earned_ids.contains(&achievement.id),
            AchievementFilter::Category(cat) => achievement.category == cat,
            AchievementFilter::Tier(tier) => achievement.tier == tier,
        }
    }

    /// Get progress percent for achievement.
    fn get_progress_percent(&self, id: &Uuid) -> f32 {
        if self.earned_ids.contains(id) {
            1.0
        } else {
            self.progress
                .get(id)
                .map(|p| p.progress_percent)
                .unwrap_or(0.0)
        }
    }

    /// Count achievements by status.
    pub fn counts(&self) -> (usize, usize) {
        let total = self.achievements.len();
        let earned = self.earned_ids.len();
        (earned, total)
    }

    /// Show the achievements screen.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            self.show_header(ui);
            ui.add_space(16.0);
            self.show_filters(ui);
            ui.add_space(8.0);
            self.show_achievements_grid(ui);
        });
    }

    /// Show header with XP status.
    fn show_header(&self, ui: &mut Ui) {
        let (earned, total) = self.counts();

        ui.horizontal(|ui| {
            ui.heading("Achievements");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // XP display
                ui.label(
                    RichText::new(format!("Level {}", self.xp_status.level))
                        .strong()
                        .size(18.0),
                );

                ui.separator();

                ui.label(
                    RichText::new(format!("{} / {} earned", earned, total))
                        .weak()
                        .size(14.0),
                );

                // XP progress bar
                let progress = self.xp_status.level_progress;
                let xp_text = format!(
                    "{} / {} XP",
                    self.xp_status.xp_into_level, self.xp_status.xp_for_next
                );

                ui.add(
                    egui::ProgressBar::new(progress)
                        .text(xp_text)
                        .desired_width(150.0),
                );
            });
        });
    }

    /// Show filter controls.
    fn show_filters(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Category filter
            egui::ComboBox::from_label("Category")
                .selected_text(
                    self.category_filter
                        .map(|c| c.display_name())
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.category_filter.is_none(), "All")
                        .clicked()
                    {
                        self.category_filter = None;
                    }
                    for cat in AchievementCategory::all() {
                        if ui
                            .selectable_label(
                                self.category_filter == Some(*cat),
                                cat.display_name(),
                            )
                            .clicked()
                        {
                            self.category_filter = Some(*cat);
                        }
                    }
                });

            ui.separator();

            // Status filter
            egui::ComboBox::from_label("Status")
                .selected_text(self.filter.label())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter == AchievementFilter::All, "All")
                        .clicked()
                    {
                        self.filter = AchievementFilter::All;
                    }
                    if ui
                        .selectable_label(self.filter == AchievementFilter::Earned, "Earned")
                        .clicked()
                    {
                        self.filter = AchievementFilter::Earned;
                    }
                    if ui
                        .selectable_label(self.filter == AchievementFilter::Unearned, "Locked")
                        .clicked()
                    {
                        self.filter = AchievementFilter::Unearned;
                    }
                });

            ui.separator();

            // Sort
            egui::ComboBox::from_label("Sort")
                .selected_text(self.sort.label())
                .show_ui(ui, |ui| {
                    for sort in [
                        AchievementSort::Category,
                        AchievementSort::TierDesc,
                        AchievementSort::XpDesc,
                        AchievementSort::RecentlyEarned,
                        AchievementSort::NearCompletion,
                    ] {
                        if ui
                            .selectable_label(self.sort == sort, sort.label())
                            .clicked()
                        {
                            self.sort = sort;
                        }
                    }
                });

            ui.separator();

            // Show secrets toggle
            ui.checkbox(&mut self.show_secrets, "Show Secrets");
        });
    }

    /// Show achievements grid.
    fn show_achievements_grid(&mut self, ui: &mut Ui) {
        let achievements = self.filtered_achievements();
        let mut clicked_id = None;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for achievement in achievements {
                        let is_earned = self.earned_ids.contains(&achievement.id);
                        let progress = self.get_progress_percent(&achievement.id);

                        if self.show_achievement_card(ui, achievement, is_earned, progress) {
                            clicked_id = Some(achievement.id);
                        }
                    }
                });
            });

        if let Some(id) = clicked_id {
            self.selected = Some(id);
        }
    }

    /// Show a single achievement card. Returns true if clicked.
    fn show_achievement_card(
        &self,
        ui: &mut Ui,
        achievement: &Achievement,
        earned: bool,
        progress: f32,
    ) -> bool {
        let mut clicked = false;

        let card_width = 200.0;
        let card_height = 120.0;

        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2::new(card_width, card_height),
            egui::Sense::click(),
        );

        if response.clicked() {
            clicked = true;
        }

        // Background color based on state
        let bg_color = if earned {
            tier_bg_color(achievement.tier)
        } else {
            Color32::from_gray(40)
        };

        let border_color = if earned {
            tier_accent_color(achievement.tier)
        } else {
            Color32::from_gray(60)
        };

        // Draw card background
        ui.painter().rect_filled(rect, 8.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, border_color),
            egui::StrokeKind::Middle,
        );

        // Draw content inside card
        let content_rect = rect.shrink(10.0);
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        child_ui.vertical(|ui| {
            // Tier badge
            ui.horizontal(|ui| {
                let tier_color = if earned {
                    tier_accent_color(achievement.tier)
                } else {
                    Color32::from_gray(100)
                };
                ui.label(
                    RichText::new(achievement.tier.display_name().to_uppercase())
                        .color(tier_color)
                        .size(10.0)
                        .strong(),
                );

                if achievement.is_secret {
                    ui.label(
                        RichText::new("SECRET")
                            .color(Color32::from_rgb(186, 85, 211))
                            .size(10.0),
                    );
                }
            });

            ui.add_space(4.0);

            // Title
            let title_color = if earned {
                Color32::WHITE
            } else {
                Color32::from_gray(150)
            };
            ui.label(
                RichText::new(&achievement.title)
                    .color(title_color)
                    .strong()
                    .size(14.0),
            );

            // Description (truncated)
            let desc = if achievement.description.len() > 50 && !earned {
                format!("{}...", &achievement.description[..47])
            } else {
                achievement.description.clone()
            };
            ui.label(
                RichText::new(desc)
                    .color(Color32::from_gray(if earned { 200 } else { 100 }))
                    .size(11.0),
            );

            ui.add_space(4.0);

            // XP or progress bar
            if earned {
                ui.label(
                    RichText::new(format!("+{} XP", achievement.effective_xp()))
                        .color(Color32::from_rgb(255, 215, 0))
                        .strong()
                        .size(12.0),
                );
            } else if progress > 0.0 && achievement.threshold.is_some() {
                ui.add(
                    egui::ProgressBar::new(progress)
                        .text(format!("{:.0}%", progress * 100.0))
                        .desired_width(card_width - 20.0),
                );
            } else {
                ui.label(
                    RichText::new(format!("{} XP", achievement.effective_xp()))
                        .color(Color32::from_gray(80))
                        .size(11.0),
                );
            }
        });

        clicked
    }
}

/// Get background color for earned achievement tier.
fn tier_bg_color(tier: AchievementTier) -> Color32 {
    match tier {
        AchievementTier::Bronze => Color32::from_rgb(80, 50, 30),
        AchievementTier::Silver => Color32::from_rgb(70, 70, 75),
        AchievementTier::Gold => Color32::from_rgb(80, 65, 10),
        AchievementTier::Diamond => Color32::from_rgb(40, 70, 90),
        AchievementTier::Legendary => Color32::from_rgb(70, 30, 70),
    }
}

/// Get accent color for tier.
fn tier_accent_color(tier: AchievementTier) -> Color32 {
    match tier {
        AchievementTier::Bronze => Color32::from_rgb(205, 127, 50),
        AchievementTier::Silver => Color32::from_rgb(192, 192, 192),
        AchievementTier::Gold => Color32::from_rgb(255, 215, 0),
        AchievementTier::Diamond => Color32::from_rgb(185, 242, 255),
        AchievementTier::Legendary => Color32::from_rgb(186, 85, 211),
    }
}

/// Action from achievements screen.
#[derive(Debug, Clone)]
pub enum AchievementsAction {
    /// Go back to previous screen.
    Back,
    /// View achievement detail.
    ViewDetail(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = AchievementsScreen::new();
        assert!(!screen.achievements.is_empty());
        assert!(screen.earned_ids.is_empty());
    }

    #[test]
    fn test_filter_all() {
        let screen = AchievementsScreen::new();
        let filtered = screen.filtered_achievements();
        // Should have achievements (secrets hidden unless earned)
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_counts() {
        let mut screen = AchievementsScreen::new();
        let (earned, total) = screen.counts();
        assert_eq!(earned, 0);
        assert!(total > 50); // We defined 62 achievements

        // Mark one as earned
        if let Some(a) = screen.achievements.first() {
            screen.earned_ids.insert(a.id);
        }
        let (earned, _) = screen.counts();
        assert_eq!(earned, 1);
    }

    #[test]
    fn test_filter_earned() {
        let mut screen = AchievementsScreen::new();

        // Mark first achievement as earned
        if let Some(a) = screen.achievements.first() {
            screen.earned_ids.insert(a.id);
        }

        screen.filter = AchievementFilter::Earned;
        let filtered = screen.filtered_achievements();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_tier_colors() {
        for tier in AchievementTier::all() {
            let bg = tier_bg_color(*tier);
            let accent = tier_accent_color(*tier);
            // Colors should be reasonable values
            assert!(bg.r() > 0 || bg.g() > 0 || bg.b() > 0);
            assert!(accent.r() > 0 || accent.g() > 0 || accent.b() > 0);
        }
    }
}
