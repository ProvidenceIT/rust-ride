//! Plan detail view widget.
//!
//! T067: Create plan detail view with weekly overview.
//!
//! Shows a detailed view of a training plan including weekly breakdown,
//! workout types, and plan actions (start, resume, abandon).

use egui::{Align, Color32, Layout, RichText, ScrollArea, Stroke, Ui};

use crate::training_plans::{
    DifficultyLevel, Discipline, PlanWeek, TrainingPhase, TrainingPlan, WorkoutType,
};

/// Configuration for plan detail display.
#[derive(Debug, Clone)]
pub struct PlanDetailConfig {
    /// Width of the detail panel.
    pub panel_width: f32,
    /// Whether to show start button.
    pub show_start_button: bool,
    /// Whether to show close button.
    pub show_close_button: bool,
}

impl Default for PlanDetailConfig {
    fn default() -> Self {
        Self {
            panel_width: 400.0,
            show_start_button: true,
            show_close_button: true,
        }
    }
}

/// Action from plan detail widget.
#[derive(Debug, Clone)]
pub enum PlanDetailAction {
    /// Start this plan.
    StartPlan(String),
    /// Close the detail view.
    Close,
}

/// Plan detail view widget.
pub struct PlanDetailWidget {
    /// Display configuration.
    config: PlanDetailConfig,
    /// Currently expanded week (for accordion view).
    expanded_week: Option<u8>,
}

impl Default for PlanDetailWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanDetailWidget {
    /// Create a new plan detail widget.
    pub fn new() -> Self {
        Self {
            config: PlanDetailConfig::default(),
            expanded_week: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: PlanDetailConfig) -> Self {
        Self {
            config,
            expanded_week: None,
        }
    }

    /// Show the plan detail view.
    pub fn show(&mut self, ui: &mut Ui, plan: &TrainingPlan) -> Option<PlanDetailAction> {
        let mut action = None;

        egui::Frame::new()
            .fill(Color32::from_rgb(30, 30, 40))
            .corner_radius(12.0)
            .stroke(Stroke::new(1.0, Color32::from_rgb(60, 60, 70)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_width(self.config.panel_width);

                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&plan.name)
                            .strong()
                            .size(20.0)
                            .color(Color32::WHITE),
                    );

                    if self.config.show_close_button {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("X").clicked() {
                                action = Some(PlanDetailAction::Close);
                            }
                        });
                    }
                });

                ui.add_space(8.0);

                // Metadata badges
                ui.horizontal(|ui| {
                    // Discipline
                    self.show_badge(
                        ui,
                        discipline_emoji(plan.discipline),
                        plan.discipline.display_name(),
                    );

                    // Difficulty
                    self.show_difficulty_badge(ui, plan.difficulty);

                    // Duration
                    self.show_badge(ui, "\u{1F4C5}", &format!("{} weeks", plan.duration_weeks));

                    // Workouts per week
                    self.show_badge(ui, "\u{1F4AA}", &format!("{}/week", plan.workouts_per_week));
                });

                ui.add_space(12.0);

                // Description
                ui.label(
                    RichText::new(&plan.description)
                        .color(Color32::from_gray(180))
                        .size(13.0),
                );

                ui.add_space(12.0);

                // Tags
                if !plan.tags.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for tag in &plan.tags {
                            egui::Frame::new()
                                .fill(Color32::from_rgb(50, 50, 70))
                                .corner_radius(4.0)
                                .inner_margin(egui::vec2(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("#{}", tag))
                                            .color(Color32::from_rgb(100, 149, 237))
                                            .size(11.0),
                                    );
                                });
                        }
                    });

                    ui.add_space(12.0);
                }

                ui.separator();
                ui.add_space(8.0);

                // Weekly breakdown
                ui.label(RichText::new("Weekly Breakdown").strong().size(14.0));

                ui.add_space(8.0);

                ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (idx, week) in plan.weeks.iter().enumerate() {
                        self.show_week_row(ui, week, idx as u8 + 1);
                    }
                });

                ui.add_space(12.0);

                // Action buttons
                if self.config.show_start_button {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Start Plan").strong().color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(0, 100, 180)),
                            )
                            .clicked()
                        {
                            action = Some(PlanDetailAction::StartPlan(plan.id.to_string()));
                        }
                    });
                }
            });

        action
    }

    /// Show a metadata badge.
    fn show_badge(&self, ui: &mut Ui, emoji: &str, text: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(45, 45, 55))
            .corner_radius(4.0)
            .inner_margin(egui::vec2(6.0, 3.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(emoji).size(12.0));
                    ui.label(
                        RichText::new(text)
                            .color(Color32::from_gray(200))
                            .size(11.0),
                    );
                });
            });
    }

    /// Show difficulty badge with color.
    fn show_difficulty_badge(&self, ui: &mut Ui, difficulty: DifficultyLevel) {
        let color = difficulty_color(difficulty);

        egui::Frame::new()
            .fill(color)
            .corner_radius(4.0)
            .inner_margin(egui::vec2(6.0, 3.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(difficulty.display_name())
                        .color(Color32::WHITE)
                        .size(11.0),
                );
            });
    }

    /// Show a week row in the breakdown.
    fn show_week_row(&mut self, ui: &mut Ui, week: &PlanWeek, week_num: u8) {
        let is_expanded = self.expanded_week == Some(week_num);
        let phase_color = phase_color(week.phase);

        egui::Frame::new()
            .fill(if is_expanded {
                Color32::from_rgb(45, 45, 55)
            } else {
                Color32::from_rgb(35, 35, 45)
            })
            .corner_radius(6.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                // Header row (clickable to expand)
                let response = ui
                    .horizontal(|ui| {
                        // Week number with phase indicator
                        egui::Frame::new()
                            .fill(phase_color)
                            .corner_radius(4.0)
                            .inner_margin(egui::vec2(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("Week {}", week_num))
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(11.0),
                                );
                            });

                        ui.add_space(8.0);

                        // Week name
                        ui.label(RichText::new(&week.title).color(Color32::WHITE).size(12.0));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Expand indicator
                            ui.label(
                                RichText::new(if is_expanded { "\u{25BC}" } else { "\u{25B6}" })
                                    .color(Color32::from_gray(150))
                                    .size(10.0),
                            );

                            // Workout count
                            ui.label(
                                RichText::new(format!("{} workouts", week.workouts.len()))
                                    .color(Color32::from_gray(150))
                                    .size(11.0),
                            );

                            // Phase name
                            ui.label(
                                RichText::new(week.phase.display_name())
                                    .color(phase_color)
                                    .size(10.0),
                            );
                        });
                    })
                    .response;

                if response.interact(egui::Sense::click()).clicked() {
                    if is_expanded {
                        self.expanded_week = None;
                    } else {
                        self.expanded_week = Some(week_num);
                    }
                }

                // Expanded content
                if is_expanded {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Focus and description
                    if !week.description.is_empty() {
                        ui.label(
                            RichText::new(&week.description)
                                .color(Color32::from_gray(180))
                                .italics()
                                .size(11.0),
                        );
                        ui.add_space(4.0);
                    }

                    // Workout list
                    for workout in &week.workouts {
                        ui.horizontal(|ui| {
                            // Day indicator
                            ui.label(
                                RichText::new(format!("Day {}:", workout.day_of_week))
                                    .color(Color32::from_gray(120))
                                    .size(10.0),
                            );

                            // Workout type icon
                            ui.label(
                                RichText::new(workout_type_emoji(workout.workout_type)).size(12.0),
                            );

                            // Workout name
                            ui.label(
                                RichText::new(&workout.workout_name)
                                    .color(Color32::WHITE)
                                    .size(11.0),
                            );

                            // Duration
                            if workout.duration_minutes > 0 {
                                ui.label(
                                    RichText::new(format!("{}min", workout.duration_minutes))
                                        .color(Color32::from_gray(150))
                                        .size(10.0),
                                );
                            }
                        });
                    }
                }
            });

        ui.add_space(4.0);
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

/// Get color for training phase.
fn phase_color(phase: TrainingPhase) -> Color32 {
    match phase {
        TrainingPhase::Base => Color32::from_rgb(60, 100, 140),
        TrainingPhase::Build => Color32::from_rgb(140, 100, 60),
        TrainingPhase::Peak => Color32::from_rgb(180, 60, 60),
        TrainingPhase::Recovery => Color32::from_rgb(60, 140, 80),
        TrainingPhase::Taper => Color32::from_rgb(100, 80, 140),
        TrainingPhase::Specialty => Color32::from_rgb(160, 80, 120),
        TrainingPhase::Transition => Color32::from_rgb(100, 100, 100),
    }
}

/// Get emoji for discipline.
fn discipline_emoji(discipline: Discipline) -> &'static str {
    match discipline {
        Discipline::Road => "\u{1F6B4}",
        Discipline::Gravel => "\u{26F0}",
        Discipline::Triathlon => "\u{1F3CA}",
        Discipline::MTB => "\u{1F6B5}",
        Discipline::GeneralFitness => "\u{1F4AA}",
    }
}

/// Get emoji for workout type.
fn workout_type_emoji(workout_type: WorkoutType) -> &'static str {
    match workout_type {
        WorkoutType::Endurance => "\u{1F6B4}",      // Biking
        WorkoutType::Tempo => "\u{23F1}",           // Stopwatch
        WorkoutType::Threshold => "\u{1F525}",      // Fire
        WorkoutType::Vo2Max => "\u{1F4A8}",         // Dash
        WorkoutType::Sprint => "\u{26A1}",          // Lightning
        WorkoutType::Recovery => "\u{1F49A}",       // Green heart
        WorkoutType::Anaerobic => "\u{1F4A5}",      // Collision
        WorkoutType::RaceSimulation => "\u{1F3C1}", // Checkered flag
        WorkoutType::Test => "\u{1F4CA}",           // Bar chart
        WorkoutType::Mixed => "\u{1F500}",          // Shuffle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let widget = PlanDetailWidget::new();
        assert!(widget.expanded_week.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = PlanDetailConfig::default();
        assert!(config.panel_width > 0.0);
        assert!(config.show_start_button);
        assert!(config.show_close_button);
    }

    #[test]
    fn test_phase_colors() {
        // Ensure all phases have distinct colors
        let base = phase_color(TrainingPhase::Base);
        let build = phase_color(TrainingPhase::Build);
        let peak = phase_color(TrainingPhase::Peak);

        assert_ne!(base, build);
        assert_ne!(build, peak);
        assert_ne!(base, peak);
    }
}
