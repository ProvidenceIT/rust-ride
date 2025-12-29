//! Upcoming workouts widget.
//!
//! T068: Create upcoming workouts widget.
//!
//! Shows a compact list of upcoming scheduled workouts from the active
//! training plan with quick-start functionality.

use egui::{Align, Color32, Layout, RichText, Stroke, Ui, Vec2};

use crate::training_plans::{UpcomingWorkout, UpcomingWorkoutList, WorkoutType};

/// Configuration for upcoming workouts widget.
#[derive(Debug, Clone)]
pub struct UpcomingWorkoutsConfig {
    /// Maximum workouts to display.
    pub max_items: usize,
    /// Width of the widget.
    pub width: f32,
    /// Show quick-start button.
    pub show_start_button: bool,
    /// Compact mode (smaller text, less spacing).
    pub compact: bool,
}

impl Default for UpcomingWorkoutsConfig {
    fn default() -> Self {
        Self {
            max_items: 5,
            width: 320.0,
            show_start_button: true,
            compact: false,
        }
    }
}

/// Action from upcoming workouts widget.
#[derive(Debug, Clone)]
pub enum UpcomingWorkoutsAction {
    /// Start a specific workout.
    StartWorkout(String),
    /// View workout details.
    ViewWorkout(String),
    /// View full training plan.
    ViewPlan,
    /// Skip a workout.
    SkipWorkout(String),
}

/// Upcoming workouts widget.
pub struct UpcomingWorkoutsWidget {
    /// Display configuration.
    config: UpcomingWorkoutsConfig,
}

impl Default for UpcomingWorkoutsWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl UpcomingWorkoutsWidget {
    /// Create a new upcoming workouts widget.
    pub fn new() -> Self {
        Self {
            config: UpcomingWorkoutsConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: UpcomingWorkoutsConfig) -> Self {
        Self { config }
    }

    /// Show the upcoming workouts widget.
    pub fn show(&self, ui: &mut Ui, workouts: &UpcomingWorkoutList) -> Option<UpcomingWorkoutsAction> {
        let mut action = None;

        egui::Frame::new()
            .fill(Color32::from_rgb(30, 30, 40))
            .corner_radius(8.0)
            .stroke(Stroke::new(1.0, Color32::from_rgb(50, 50, 60)))
            .inner_margin(if self.config.compact { 8.0 } else { 12.0 })
            .show(ui, |ui| {
                ui.set_min_width(self.config.width);

                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("\u{1F4C5} Upcoming Workouts")
                            .strong()
                            .size(if self.config.compact { 13.0 } else { 14.0 }),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("View Plan").clicked() {
                            action = Some(UpcomingWorkoutsAction::ViewPlan);
                        }
                    });
                });

                ui.add_space(if self.config.compact { 6.0 } else { 10.0 });

                if workouts.is_empty() {
                    self.show_empty_state(ui);
                } else {
                    let items: Vec<_> = workouts.workouts.iter().take(self.config.max_items).collect();
                    for workout in items {
                        if let Some(a) = self.show_workout_row(ui, workout) {
                            action = Some(a);
                        }
                    }

                    // Show "more" indicator if there are more workouts
                    if workouts.len() > self.config.max_items {
                        ui.add_space(4.0);
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "+{} more workouts",
                                    workouts.len() - self.config.max_items
                                ))
                                .color(Color32::from_gray(120))
                                .size(11.0),
                            );
                        });
                    }
                }
            });

        action
    }

    /// Show empty state when no workouts are scheduled.
    fn show_empty_state(&self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("\u{1F3C6}")
                    .size(24.0),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("No upcoming workouts")
                    .color(Color32::from_gray(150))
                    .size(12.0),
            );
            ui.label(
                RichText::new("Start a training plan to see scheduled workouts")
                    .color(Color32::from_gray(120))
                    .size(11.0),
            );
            ui.add_space(12.0);
        });
    }

    /// Show a single workout row.
    fn show_workout_row(&self, ui: &mut Ui, workout: &UpcomingWorkout) -> Option<UpcomingWorkoutsAction> {
        let mut action = None;
        let is_today = workout.is_today;
        let is_overdue = workout.days_until < 0;

        let _row_height = if self.config.compact { 44.0 } else { 56.0 };

        egui::Frame::new()
            .fill(if is_today {
                Color32::from_rgb(40, 60, 80)
            } else if is_overdue {
                Color32::from_rgb(60, 40, 40)
            } else {
                Color32::from_rgb(35, 35, 45)
            })
            .corner_radius(6.0)
            .inner_margin(if self.config.compact { 6.0 } else { 8.0 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left: Workout type icon and day info
                    ui.vertical(|ui| {
                        ui.set_min_width(50.0);

                        // Day label
                        let day_text = if is_today {
                            "TODAY".to_string()
                        } else if is_overdue {
                            "MISSED".to_string()
                        } else {
                            workout.date_description()
                        };

                        let day_color = if is_today {
                            Color32::from_rgb(100, 200, 100)
                        } else if is_overdue {
                            Color32::from_rgb(200, 100, 100)
                        } else {
                            Color32::from_gray(150)
                        };

                        ui.label(
                            RichText::new(day_text)
                                .color(day_color)
                                .strong()
                                .size(if self.config.compact { 9.0 } else { 10.0 }),
                        );

                        // Workout type icon
                        ui.label(
                            RichText::new(workout_type_emoji(workout.workout.workout_type))
                                .size(if self.config.compact { 16.0 } else { 20.0 }),
                        );
                    });

                    ui.add_space(8.0);

                    // Middle: Workout info
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&workout.workout.workout_name)
                                .color(Color32::WHITE)
                                .strong()
                                .size(if self.config.compact { 12.0 } else { 13.0 }),
                        );

                        ui.horizontal(|ui| {
                            // Duration
                            if workout.workout.duration_minutes > 0 {
                                ui.label(
                                    RichText::new(format!("{}min", workout.workout.duration_minutes))
                                        .color(Color32::from_gray(150))
                                        .size(if self.config.compact { 10.0 } else { 11.0 }),
                                );
                            }

                            // Workout type name
                            ui.label(
                                RichText::new(workout.workout.workout_type.display_name())
                                    .color(workout_type_color(workout.workout.workout_type))
                                    .size(if self.config.compact { 10.0 } else { 11.0 }),
                            );

                            // Week info
                            ui.label(
                                RichText::new(format!("Week {}", workout.workout.week_number))
                                    .color(Color32::from_gray(120))
                                    .size(if self.config.compact { 9.0 } else { 10.0 }),
                            );
                        });
                    });

                    // Right: Action buttons
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.config.show_start_button
                            && ui
                                .add(egui::Button::new(
                                    RichText::new(if is_overdue { "Make Up" } else { "Start" })
                                        .color(Color32::WHITE)
                                        .size(11.0),
                                ).min_size(Vec2::new(50.0, 24.0)))
                                .clicked()
                            {
                                action = Some(UpcomingWorkoutsAction::StartWorkout(
                                    workout.workout.id.to_string(),
                                ));
                            }

                        // Skip button (only for overdue)
                        if is_overdue
                            && ui.small_button("Skip").clicked() {
                                action = Some(UpcomingWorkoutsAction::SkipWorkout(
                                    workout.workout.id.to_string(),
                                ));
                            }
                    });
                });
            });

        ui.add_space(if self.config.compact { 3.0 } else { 6.0 });

        action
    }
}

/// Compact upcoming workouts widget for dashboard/home screen.
pub struct UpcomingWorkoutsCompact;

impl UpcomingWorkoutsCompact {
    /// Show a compact version with just the next workout.
    pub fn show(ui: &mut Ui, workouts: &UpcomingWorkoutList) -> Option<UpcomingWorkoutsAction> {
        let config = UpcomingWorkoutsConfig {
            max_items: 1,
            width: 280.0,
            show_start_button: true,
            compact: true,
        };

        UpcomingWorkoutsWidget::with_config(config).show(ui, workouts)
    }

    /// Show a single-line indicator for header bars.
    pub fn show_inline(ui: &mut Ui, workouts: &UpcomingWorkoutList) -> Option<UpcomingWorkoutsAction> {
        let mut action = None;

        if let Some(next) = workouts.workouts.first() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(workout_type_emoji(next.workout.workout_type))
                        .size(14.0),
                );

                let label = if next.is_today {
                    format!("Today: {}", next.workout.workout_name)
                } else {
                    format!("{}: {}", next.date_description(), next.workout.workout_name)
                };

                if ui
                    .add(egui::Label::new(
                        RichText::new(label)
                            .color(Color32::from_gray(200))
                            .size(12.0),
                    ).sense(egui::Sense::click()))
                    .clicked()
                {
                    action = Some(UpcomingWorkoutsAction::ViewWorkout(next.workout.id.to_string()));
                }
            });
        }

        action
    }
}

/// Get emoji for workout type.
fn workout_type_emoji(workout_type: WorkoutType) -> &'static str {
    match workout_type {
        WorkoutType::Endurance => "\u{1F6B4}",       // Biking
        WorkoutType::Tempo => "\u{23F1}",            // Stopwatch
        WorkoutType::Threshold => "\u{1F525}",       // Fire
        WorkoutType::Vo2Max => "\u{1F4A8}",          // Dash
        WorkoutType::Sprint => "\u{26A1}",           // Lightning
        WorkoutType::Recovery => "\u{1F49A}",        // Green heart
        WorkoutType::Anaerobic => "\u{1F4A5}",       // Collision
        WorkoutType::RaceSimulation => "\u{1F3C1}", // Checkered flag
        WorkoutType::Test => "\u{1F4CA}",            // Bar chart
        WorkoutType::Mixed => "\u{1F500}",           // Shuffle
    }
}

/// Get color for workout type.
fn workout_type_color(workout_type: WorkoutType) -> Color32 {
    match workout_type {
        WorkoutType::Endurance => Color32::from_rgb(80, 140, 200),
        WorkoutType::Tempo => Color32::from_rgb(200, 160, 80),
        WorkoutType::Threshold => Color32::from_rgb(220, 120, 60),
        WorkoutType::Vo2Max => Color32::from_rgb(220, 80, 80),
        WorkoutType::Sprint => Color32::from_rgb(180, 80, 180),
        WorkoutType::Recovery => Color32::from_rgb(80, 180, 100),
        WorkoutType::Anaerobic => Color32::from_rgb(200, 60, 120),
        WorkoutType::RaceSimulation => Color32::from_rgb(160, 100, 180),
        WorkoutType::Test => Color32::from_rgb(100, 100, 100),
        WorkoutType::Mixed => Color32::from_rgb(140, 140, 160),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let widget = UpcomingWorkoutsWidget::new();
        assert_eq!(widget.config.max_items, 5);
    }

    #[test]
    fn test_config_defaults() {
        let config = UpcomingWorkoutsConfig::default();
        assert!(config.max_items > 0);
        assert!(config.width > 0.0);
        assert!(config.show_start_button);
        assert!(!config.compact);
    }

    #[test]
    fn test_workout_type_colors() {
        // Ensure different workout types have different colors
        let endurance = workout_type_color(WorkoutType::Endurance);
        let vo2max = workout_type_color(WorkoutType::Vo2Max);
        let recovery = workout_type_color(WorkoutType::Recovery);

        assert_ne!(endurance, vo2max);
        assert_ne!(vo2max, recovery);
    }
}
