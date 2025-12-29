//! Training plans browse and filter screen.
//!
//! T066: Create training plans browse/filter screen.
//!
//! Displays available training plans with filtering by discipline,
//! difficulty, and duration. Allows starting a new training plan.

use egui::{Align, Color32, Layout, RichText, ScrollArea, Stroke, StrokeKind, Ui, Vec2};

use crate::training_plans::{
    all_plans, DifficultyLevel, Discipline, TrainingPlan, TrainingPlanManager,
};

/// Training plans screen state.
#[derive(Default)]
pub struct TrainingPlansScreen {
    /// Selected discipline filter.
    pub discipline_filter: Option<Discipline>,
    /// Selected difficulty filter.
    pub difficulty_filter: Option<DifficultyLevel>,
    /// Selected plan for detail view.
    pub selected_plan_id: Option<String>,
    /// Search query for plan names.
    pub search_query: String,
    /// Show only featured plans.
    pub show_featured_only: bool,
    /// Sort order.
    pub sort_order: PlanSortOrder,
}

/// Sort order for training plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanSortOrder {
    /// Sort by name alphabetically.
    #[default]
    Name,
    /// Sort by duration (shortest first).
    DurationAsc,
    /// Sort by duration (longest first).
    DurationDesc,
    /// Sort by difficulty (easiest first).
    DifficultyAsc,
    /// Sort by difficulty (hardest first).
    DifficultyDesc,
}

/// Action from training plans screen.
#[derive(Debug, Clone)]
pub enum TrainingPlansAction {
    /// View plan details.
    ViewPlanDetails(String),
    /// Start a training plan.
    StartPlan(String),
    /// Navigate back.
    Back,
}

impl TrainingPlansScreen {
    /// Create a new training plans screen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the training plans screen.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        manager: Option<&TrainingPlanManager>,
    ) -> Option<TrainingPlansAction> {
        let mut action = None;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button("< Back").clicked() {
                    action = Some(TrainingPlansAction::Back);
                }
                ui.add_space(16.0);
                ui.heading("Training Plans");
            });

            ui.add_space(16.0);

            // Current plan status (if any)
            if let Some(mgr) = manager {
                if mgr.has_active_plan() {
                    self.show_current_plan_banner(ui, mgr);
                    ui.add_space(16.0);
                }
            }

            // Filter bar
            self.show_filter_bar(ui);
            ui.add_space(12.0);

            ui.separator();
            ui.add_space(8.0);

            // Plan grid
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(a) = self.show_plan_grid(ui) {
                        action = Some(a);
                    }
                });
        });

        action
    }

    /// Show current plan banner if one is active.
    fn show_current_plan_banner(&self, ui: &mut Ui, manager: &TrainingPlanManager) {
        egui::Frame::new()
            .fill(Color32::from_rgb(30, 60, 100))
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Active Plan")
                            .color(Color32::from_rgb(100, 149, 237))
                            .strong(),
                    );

                    if let Some(plan) = manager.current_plan() {
                        ui.label(RichText::new(&plan.name).color(Color32::WHITE).strong());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(progress) = manager.progress() {
                                ui.label(
                                    RichText::new(format!(
                                        "Week {} of {}",
                                        progress.current_week, progress.total_weeks
                                    ))
                                    .color(Color32::from_gray(180)),
                                );
                            }
                        });
                    }
                });
            });
    }

    /// Show filter bar with discipline, difficulty, and search.
    fn show_filter_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Search box
            ui.label("Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Plan name...")
                    .desired_width(150.0),
            );

            ui.add_space(16.0);

            // Discipline filter
            ui.label("Discipline:");
            egui::ComboBox::from_id_salt("discipline_filter")
                .selected_text(
                    self.discipline_filter
                        .map(|d| d.display_name())
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.discipline_filter.is_none(), "All")
                        .clicked()
                    {
                        self.discipline_filter = None;
                    }
                    for discipline in Discipline::all() {
                        if ui
                            .selectable_label(
                                self.discipline_filter == Some(*discipline),
                                discipline.display_name(),
                            )
                            .clicked()
                        {
                            self.discipline_filter = Some(*discipline);
                        }
                    }
                });

            ui.add_space(8.0);

            // Difficulty filter
            ui.label("Difficulty:");
            egui::ComboBox::from_id_salt("difficulty_filter")
                .selected_text(
                    self.difficulty_filter
                        .map(|d| d.display_name())
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.difficulty_filter.is_none(), "All")
                        .clicked()
                    {
                        self.difficulty_filter = None;
                    }
                    for difficulty in DifficultyLevel::all() {
                        if ui
                            .selectable_label(
                                self.difficulty_filter == Some(*difficulty),
                                difficulty.display_name(),
                            )
                            .clicked()
                        {
                            self.difficulty_filter = Some(*difficulty);
                        }
                    }
                });

            ui.add_space(8.0);

            // Featured only toggle
            ui.checkbox(&mut self.show_featured_only, "Featured only");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Sort order
                ui.label("Sort:");
                egui::ComboBox::from_id_salt("sort_order")
                    .selected_text(self.sort_order.display_name())
                    .show_ui(ui, |ui| {
                        for order in [
                            PlanSortOrder::Name,
                            PlanSortOrder::DurationAsc,
                            PlanSortOrder::DurationDesc,
                            PlanSortOrder::DifficultyAsc,
                            PlanSortOrder::DifficultyDesc,
                        ] {
                            if ui
                                .selectable_label(self.sort_order == order, order.display_name())
                                .clicked()
                            {
                                self.sort_order = order;
                            }
                        }
                    });
            });
        });
    }

    /// Show the plan grid.
    fn show_plan_grid(&mut self, ui: &mut Ui) -> Option<TrainingPlansAction> {
        let mut action = None;
        let plans = self.get_filtered_plans();

        if plans.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("No training plans match your filters")
                        .color(Color32::from_gray(150))
                        .size(16.0),
                );
            });
            return None;
        }

        // Group by discipline
        let mut by_discipline: std::collections::HashMap<Discipline, Vec<&TrainingPlan>> =
            std::collections::HashMap::new();

        for plan in &plans {
            by_discipline.entry(plan.discipline).or_default().push(plan);
        }

        // Sort disciplines for consistent display
        let mut disciplines: Vec<_> = by_discipline.keys().copied().collect();
        disciplines.sort_by_key(|d| d.display_name());

        for discipline in disciplines {
            if let Some(plans) = by_discipline.get(&discipline) {
                ui.add_space(8.0);

                // Discipline header
                ui.horizontal(|ui| {
                    ui.label(RichText::new(discipline_emoji(discipline)).size(18.0));
                    ui.label(RichText::new(discipline.display_name()).strong().size(16.0));
                    ui.label(
                        RichText::new(format!("({} plans)", plans.len()))
                            .color(Color32::from_gray(150))
                            .size(12.0),
                    );
                });

                ui.add_space(4.0);

                // Plan cards in horizontal layout
                ui.horizontal_wrapped(|ui| {
                    for plan in plans {
                        if let Some(a) = self.show_plan_card(ui, plan) {
                            action = Some(a);
                        }
                    }
                });

                ui.add_space(8.0);
            }
        }

        action
    }

    /// Show a single plan card.
    fn show_plan_card(&mut self, ui: &mut Ui, plan: &TrainingPlan) -> Option<TrainingPlansAction> {
        let mut action = None;
        let is_selected = self.selected_plan_id.as_ref() == Some(&plan.id.to_string());

        let card_width = 280.0;
        let card_height = 160.0;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(card_width, card_height), egui::Sense::click());

        if response.clicked() {
            action = Some(TrainingPlansAction::ViewPlanDetails(plan.id.to_string()));
        }

        // Background
        let bg_color = if is_selected {
            Color32::from_rgb(40, 70, 120)
        } else if response.hovered() {
            Color32::from_rgb(50, 50, 60)
        } else {
            Color32::from_rgb(35, 35, 45)
        };

        ui.painter().rect_filled(rect, 8.0, bg_color);

        // Border
        let border_color = if plan.is_featured {
            Color32::from_rgb(255, 215, 0) // Gold for featured
        } else {
            Color32::from_rgb(60, 60, 70)
        };
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(1.0, border_color),
            StrokeKind::Inside,
        );

        // Content
        let content_rect = rect.shrink(12.0);
        let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        ui_child.vertical(|ui| {
            // Header with name and featured badge
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&plan.name)
                        .strong()
                        .size(14.0)
                        .color(Color32::WHITE),
                );

                if plan.is_featured {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Featured")
                                .color(Color32::from_rgb(255, 215, 0))
                                .size(10.0),
                        );
                    });
                }
            });

            ui.add_space(4.0);

            // Difficulty badge
            let diff_color = difficulty_color(plan.difficulty);
            ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(diff_color)
                    .corner_radius(4.0)
                    .inner_margin(egui::vec2(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(plan.difficulty.display_name())
                                .color(Color32::WHITE)
                                .size(10.0),
                        );
                    });

                ui.add_space(4.0);

                ui.label(
                    RichText::new(format!("{} weeks", plan.duration_weeks))
                        .color(Color32::from_gray(180))
                        .size(11.0),
                );

                ui.label(
                    RichText::new(format!("{}/week", plan.workouts_per_week))
                        .color(Color32::from_gray(180))
                        .size(11.0),
                );
            });

            ui.add_space(6.0);

            // Description (truncated)
            let desc = if plan.description.len() > 100 {
                format!("{}...", &plan.description[..97])
            } else {
                plan.description.clone()
            };
            ui.label(
                RichText::new(desc)
                    .color(Color32::from_gray(160))
                    .size(11.0),
            );

            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                // Tags
                ui.horizontal(|ui| {
                    for tag in plan.tags.iter().take(3) {
                        ui.label(
                            RichText::new(format!("#{}", tag))
                                .color(Color32::from_rgb(100, 149, 237))
                                .size(10.0),
                        );
                    }
                });
            });
        });

        action
    }

    /// Get filtered and sorted plans.
    fn get_filtered_plans(&self) -> Vec<TrainingPlan> {
        let all = all_plans();
        let mut filtered: Vec<_> = all
            .into_iter()
            .filter(|p| {
                // Discipline filter
                if let Some(d) = self.discipline_filter {
                    if p.discipline != d {
                        return false;
                    }
                }

                // Difficulty filter
                if let Some(d) = self.difficulty_filter {
                    if p.difficulty != d {
                        return false;
                    }
                }

                // Featured filter
                if self.show_featured_only && !p.is_featured {
                    return false;
                }

                // Search filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !p.name.to_lowercase().contains(&query)
                        && !p.description.to_lowercase().contains(&query)
                    {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort
        match self.sort_order {
            PlanSortOrder::Name => filtered.sort_by(|a, b| a.name.cmp(&b.name)),
            PlanSortOrder::DurationAsc => filtered.sort_by_key(|p| p.duration_weeks),
            PlanSortOrder::DurationDesc => {
                filtered.sort_by_key(|p| std::cmp::Reverse(p.duration_weeks))
            }
            PlanSortOrder::DifficultyAsc => filtered.sort_by_key(|p| p.difficulty as u8),
            PlanSortOrder::DifficultyDesc => {
                filtered.sort_by_key(|p| std::cmp::Reverse(p.difficulty as u8))
            }
        }

        filtered
    }

    /// Clear all filters.
    pub fn clear_filters(&mut self) {
        self.discipline_filter = None;
        self.difficulty_filter = None;
        self.search_query.clear();
        self.show_featured_only = false;
    }

    /// Select a plan by ID.
    pub fn select_plan(&mut self, plan_id: &str) {
        self.selected_plan_id = Some(plan_id.to_string());
    }
}

impl PlanSortOrder {
    /// Get display name for sort order.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::DurationAsc => "Duration (short first)",
            Self::DurationDesc => "Duration (long first)",
            Self::DifficultyAsc => "Difficulty (easy first)",
            Self::DifficultyDesc => "Difficulty (hard first)",
        }
    }
}

/// Get emoji for discipline.
fn discipline_emoji(discipline: Discipline) -> &'static str {
    match discipline {
        Discipline::Road => "\u{1F6B4}",           // Person biking
        Discipline::Gravel => "\u{26F0}",          // Mountain
        Discipline::Triathlon => "\u{1F3CA}",      // Person swimming
        Discipline::MTB => "\u{1F6B5}",            // Mountain biking
        Discipline::GeneralFitness => "\u{1F4AA}", // Flexed biceps
    }
}

/// Get color for difficulty level.
fn difficulty_color(difficulty: DifficultyLevel) -> Color32 {
    match difficulty {
        DifficultyLevel::Beginner => Color32::from_rgb(60, 120, 60),
        DifficultyLevel::Intermediate => Color32::from_rgb(140, 120, 40),
        DifficultyLevel::Advanced => Color32::from_rgb(160, 60, 40),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = TrainingPlansScreen::new();
        assert!(screen.discipline_filter.is_none());
        assert!(screen.difficulty_filter.is_none());
        assert!(screen.search_query.is_empty());
    }

    #[test]
    fn test_clear_filters() {
        let mut screen = TrainingPlansScreen::new();
        screen.discipline_filter = Some(Discipline::Road);
        screen.difficulty_filter = Some(DifficultyLevel::Advanced);
        screen.search_query = "test".to_string();
        screen.show_featured_only = true;

        screen.clear_filters();

        assert!(screen.discipline_filter.is_none());
        assert!(screen.difficulty_filter.is_none());
        assert!(screen.search_query.is_empty());
        assert!(!screen.show_featured_only);
    }

    #[test]
    fn test_sort_order_display_names() {
        assert_eq!(PlanSortOrder::Name.display_name(), "Name");
        assert_eq!(
            PlanSortOrder::DurationAsc.display_name(),
            "Duration (short first)"
        );
    }
}
