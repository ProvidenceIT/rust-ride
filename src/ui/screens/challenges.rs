//! Challenges screen.
//!
//! Displays active challenges with progress and allows creating/importing challenges.

use egui::{Color32, RichText, Ui};
use uuid::Uuid;

use crate::social::types::GoalType;

/// Challenge status for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengeStatus {
    #[default]
    Active,
    Completed,
    Failed,
    Pending,
}

/// Challenge with display info.
#[derive(Debug, Clone)]
pub struct ChallengeDisplay {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub goal_type: GoalType,
    pub target_value: f64,
    pub current_value: f64,
    pub status: ChallengeStatus,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}

/// Challenges screen actions.
#[derive(Debug, Clone)]
pub enum ChallengesAction {
    /// Create a new challenge.
    Create,
    /// View challenge details.
    ViewChallenge(Uuid),
    /// Export a challenge.
    Export(Uuid),
    /// Import a challenge.
    Import,
    /// Navigate back.
    Back,
}

/// Challenges view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengesView {
    /// List of challenges.
    #[default]
    List,
    /// Create new challenge form.
    Create,
}

/// Challenges screen state.
pub struct ChallengesScreen {
    /// Current view mode.
    view: ChallengesView,
    /// New challenge name.
    new_name: String,
    /// New challenge description.
    new_description: String,
    /// New challenge goal type.
    new_goal_type: GoalType,
    /// New challenge target value.
    new_target: String,
    /// New challenge duration in days.
    new_duration_days: String,
}

impl Default for ChallengesScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengesScreen {
    /// Create a new challenges screen.
    pub fn new() -> Self {
        Self {
            view: ChallengesView::List,
            new_name: String::new(),
            new_description: String::new(),
            new_goal_type: GoalType::TotalDistanceKm,
            new_target: String::new(),
            new_duration_days: "30".to_string(),
        }
    }

    /// Render the challenges screen.
    #[allow(unused_assignments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        challenges: &[ChallengeDisplay],
    ) -> Option<ChallengesAction> {
        let mut action = None;

        ui.heading("Challenges");
        ui.add_space(10.0);

        match self.view {
            ChallengesView::List => {
                action = self.show_list(ui, challenges);
            }
            ChallengesView::Create => {
                action = self.show_create_form(ui);
            }
        }

        ui.add_space(20.0);

        if ui.button("Back").clicked() {
            if self.view == ChallengesView::Create {
                self.view = ChallengesView::List;
            } else {
                action = Some(ChallengesAction::Back);
            }
        }

        action
    }

    /// Show the challenges list.
    fn show_list(
        &mut self,
        ui: &mut Ui,
        challenges: &[ChallengeDisplay],
    ) -> Option<ChallengesAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            if ui.button("Create Challenge").clicked() {
                self.view = ChallengesView::Create;
            }
            if ui.button("Import").clicked() {
                action = Some(ChallengesAction::Import);
            }
        });

        ui.add_space(15.0);

        // Active challenges
        let active: Vec<_> = challenges
            .iter()
            .filter(|c| c.status == ChallengeStatus::Active)
            .collect();

        if !active.is_empty() {
            ui.label(RichText::new("Active Challenges").strong());
            ui.add_space(5.0);

            for challenge in active {
                self.show_challenge_card(ui, challenge, &mut action);
            }
        }

        ui.add_space(15.0);

        // Completed challenges
        let completed: Vec<_> = challenges
            .iter()
            .filter(|c| c.status == ChallengeStatus::Completed)
            .collect();

        if !completed.is_empty() {
            ui.label(RichText::new("Completed Challenges").strong());
            ui.add_space(5.0);

            for challenge in completed {
                self.show_challenge_card(ui, challenge, &mut action);
            }
        }

        if challenges.is_empty() {
            ui.label(RichText::new("No challenges yet").italics());
            ui.label("Create a challenge or import one to get started!");
        }

        action
    }

    /// Show a challenge card.
    fn show_challenge_card(
        &self,
        ui: &mut Ui,
        challenge: &ChallengeDisplay,
        action: &mut Option<ChallengesAction>,
    ) {
        let bg_color = match challenge.status {
            ChallengeStatus::Active => Color32::from_rgb(40, 50, 60),
            ChallengeStatus::Completed => Color32::from_rgb(30, 50, 30),
            ChallengeStatus::Failed => Color32::from_rgb(50, 30, 30),
            ChallengeStatus::Pending => Color32::from_rgb(40, 40, 40),
        };

        egui::Frame::new()
            .fill(bg_color)
            .inner_margin(12.0)
            .outer_margin(2.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&challenge.name).strong().size(16.0));

                        if let Some(ref desc) = challenge.description {
                            ui.label(RichText::new(desc).weak());
                        }

                        ui.add_space(5.0);

                        // Goal info
                        let goal_text = format_goal(&challenge.goal_type, challenge.target_value);
                        ui.label(&goal_text);

                        // Progress bar
                        let progress = challenge.current_value / challenge.target_value;
                        let progress_clamped = progress.min(1.0);

                        ui.add_space(5.0);

                        let progress_bar = egui::ProgressBar::new(progress_clamped as f32)
                            .text(format!(
                                "{:.1}% ({:.1} / {:.1})",
                                progress * 100.0,
                                challenge.current_value,
                                challenge.target_value
                            ));
                        ui.add(progress_bar);

                        // Dates
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!(
                                "Started: {}",
                                challenge.start_date.format("%Y-%m-%d")
                            )).small().weak());
                            ui.label(RichText::new(format!(
                                "Ends: {}",
                                challenge.end_date.format("%Y-%m-%d")
                            )).small().weak());
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Export").clicked() {
                            *action = Some(ChallengesAction::Export(challenge.id));
                        }
                    });
                });
            });
    }

    /// Show the create challenge form.
    fn show_create_form(&mut self, ui: &mut Ui) -> Option<ChallengesAction> {
        let mut action = None;

        ui.label(RichText::new("Create New Challenge").strong().size(18.0));
        ui.add_space(15.0);

        egui::Grid::new("challenge_form")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_name);
                ui.end_row();

                ui.label("Description:");
                ui.text_edit_multiline(&mut self.new_description);
                ui.end_row();

                ui.label("Goal Type:");
                egui::ComboBox::from_id_salt("goal_type")
                    .selected_text(format!("{:?}", self.new_goal_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.new_goal_type,
                            GoalType::TotalDistanceKm,
                            "Total Distance (km)",
                        );
                        ui.selectable_value(
                            &mut self.new_goal_type,
                            GoalType::TotalTimeHours,
                            "Total Time (hours)",
                        );
                        ui.selectable_value(
                            &mut self.new_goal_type,
                            GoalType::WorkoutCount,
                            "Workout Count",
                        );
                        ui.selectable_value(
                            &mut self.new_goal_type,
                            GoalType::TotalTss,
                            "Total TSS",
                        );
                    });
                ui.end_row();

                ui.label("Target:");
                ui.text_edit_singleline(&mut self.new_target);
                ui.end_row();

                ui.label("Duration (days):");
                ui.text_edit_singleline(&mut self.new_duration_days);
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("Create").clicked() {
                if !self.new_name.is_empty() && !self.new_target.is_empty() {
                    action = Some(ChallengesAction::Create);
                    // Reset form
                    self.view = ChallengesView::List;
                    self.new_name.clear();
                    self.new_description.clear();
                    self.new_target.clear();
                }
            }
            if ui.button("Cancel").clicked() {
                self.view = ChallengesView::List;
            }
        });

        action
    }

    /// Get new challenge data.
    pub fn get_new_challenge_data(&self) -> Option<(String, Option<String>, GoalType, f64, u32)> {
        let target = self.new_target.parse::<f64>().ok()?;
        let duration = self.new_duration_days.parse::<u32>().ok()?;
        let description = if self.new_description.is_empty() {
            None
        } else {
            Some(self.new_description.clone())
        };
        Some((self.new_name.clone(), description, self.new_goal_type, target, duration))
    }
}

/// Format goal description.
fn format_goal(goal_type: &GoalType, target: f64) -> String {
    match goal_type {
        GoalType::TotalDistanceKm => format!("Ride {:.0} km", target),
        GoalType::TotalTimeHours => format!("Ride for {:.1} hours", target),
        GoalType::WorkoutCount => format!("Complete {:.0} workouts", target),
        GoalType::TotalTss => format!("Accumulate {:.0} TSS", target),
        GoalType::WorkoutTypeCount => format!("Complete {:.0} workouts of a specific type", target),
    }
}
