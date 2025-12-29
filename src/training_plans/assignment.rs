//! Plan assignment and status tracking.
//!
//! T061: Create PlanAssignment and PlanStatus structs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::plan::TrainingPlan;

/// Tracks a user's assignment to a training plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAssignment {
    /// User identifier.
    pub user_id: i64,
    /// The assigned plan ID.
    pub plan_id: Uuid,
    /// When the plan was started.
    pub started_at: DateTime<Utc>,
    /// Current week within the plan (1-indexed).
    pub current_week: u8,
    /// Number of completed workouts.
    pub completed_workouts: u32,
    /// Number of skipped workouts.
    pub skipped_workouts: u32,
    /// Assignment status.
    pub status: PlanStatus,
    /// Available training days (bitmask: Mon=1, Tue=2, Wed=4, etc.).
    pub available_days: u8,
    /// When the plan ends (calculated from start + duration).
    pub ends_at: Option<DateTime<Utc>>,
    /// Last activity on the plan.
    pub last_activity_at: Option<DateTime<Utc>>,
}

impl PlanAssignment {
    /// Create a new plan assignment.
    pub fn new(user_id: i64, plan_id: Uuid) -> Self {
        Self {
            user_id,
            plan_id,
            started_at: Utc::now(),
            current_week: 1,
            completed_workouts: 0,
            skipped_workouts: 0,
            status: PlanStatus::Active,
            available_days: 0b1111111, // All days by default
            ends_at: None,
            last_activity_at: None,
        }
    }

    /// Set available training days.
    pub fn with_available_days(mut self, days: u8) -> Self {
        self.available_days = days;
        self
    }

    /// Set end date from plan duration.
    pub fn with_end_date(mut self, plan_duration_weeks: u8) -> Self {
        let days = plan_duration_weeks as i64 * 7;
        self.ends_at = Some(self.started_at + chrono::Duration::days(days));
        self
    }

    /// Check if a day is available for training.
    pub fn is_day_available(&self, day_of_week: u8) -> bool {
        if !(1..=7).contains(&day_of_week) {
            return false;
        }
        let mask = 1 << (day_of_week - 1);
        (self.available_days & mask) != 0
    }

    /// Get available day names.
    pub fn available_day_names(&self) -> Vec<&'static str> {
        const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let mut result = Vec::new();
        for (i, &day) in DAYS.iter().enumerate() {
            if (self.available_days & (1 << i)) != 0 {
                result.push(day);
            }
        }
        result
    }

    /// Get the count of available days per week.
    pub fn days_per_week(&self) -> u8 {
        self.available_days.count_ones() as u8
    }

    /// Record a completed workout.
    pub fn record_completed(&mut self) {
        self.completed_workouts += 1;
        self.last_activity_at = Some(Utc::now());
    }

    /// Record a skipped workout.
    pub fn record_skipped(&mut self) {
        self.skipped_workouts += 1;
        self.last_activity_at = Some(Utc::now());
    }

    /// Advance to the next week.
    pub fn advance_week(&mut self) {
        self.current_week += 1;
    }

    /// Pause the plan.
    pub fn pause(&mut self) {
        self.status = PlanStatus::Paused;
        self.last_activity_at = Some(Utc::now());
    }

    /// Resume the plan.
    pub fn resume(&mut self) {
        self.status = PlanStatus::Active;
        self.last_activity_at = Some(Utc::now());
    }

    /// Mark as completed.
    pub fn complete(&mut self) {
        self.status = PlanStatus::Completed;
        self.last_activity_at = Some(Utc::now());
    }

    /// Abandon the plan.
    pub fn abandon(&mut self) {
        self.status = PlanStatus::Abandoned;
        self.last_activity_at = Some(Utc::now());
    }

    /// Check if the plan is active.
    pub fn is_active(&self) -> bool {
        matches!(self.status, PlanStatus::Active)
    }

    /// Check if the plan is paused.
    pub fn is_paused(&self) -> bool {
        matches!(self.status, PlanStatus::Paused)
    }

    /// Check if the plan is finished (completed or abandoned).
    pub fn is_finished(&self) -> bool {
        matches!(self.status, PlanStatus::Completed | PlanStatus::Abandoned)
    }

    /// Get the total workouts done (completed + skipped).
    pub fn total_workouts_done(&self) -> u32 {
        self.completed_workouts + self.skipped_workouts
    }

    /// Get the completion rate as a percentage.
    pub fn completion_rate(&self) -> f32 {
        let total = self.total_workouts_done();
        if total == 0 {
            return 0.0;
        }
        (self.completed_workouts as f32 / total as f32) * 100.0
    }

    /// Get the compliance rate (completed / expected).
    pub fn compliance_rate(&self, expected_total: u32) -> f32 {
        if expected_total == 0 {
            return 0.0;
        }
        (self.completed_workouts as f32 / expected_total as f32) * 100.0
    }

    /// Calculate progress through the plan.
    pub fn progress(&self, plan: &TrainingPlan) -> PlanProgress {
        let expected_workouts = plan
            .weeks
            .iter()
            .take(self.current_week as usize)
            .map(|w| w.workouts.len() as u32)
            .sum::<u32>();

        let total_plan_workouts = plan.total_workouts() as u32;

        let week_progress = if plan.duration_weeks == 0 {
            0.0
        } else {
            (self.current_week as f32 / plan.duration_weeks as f32) * 100.0
        };

        let workout_progress = if total_plan_workouts == 0 {
            0.0
        } else {
            (self.completed_workouts as f32 / total_plan_workouts as f32) * 100.0
        };

        PlanProgress {
            current_week: self.current_week,
            total_weeks: plan.duration_weeks,
            completed_workouts: self.completed_workouts,
            expected_workouts_so_far: expected_workouts,
            total_plan_workouts,
            skipped_workouts: self.skipped_workouts,
            week_progress_percent: week_progress,
            workout_progress_percent: workout_progress,
            compliance_percent: self.compliance_rate(expected_workouts),
            is_on_track: self.completed_workouts >= expected_workouts.saturating_sub(1),
        }
    }

    /// Get the start date as a NaiveDate.
    pub fn start_date(&self) -> NaiveDate {
        self.started_at.date_naive()
    }

    /// Get the current week start date.
    pub fn current_week_start(&self) -> NaiveDate {
        let days_offset = (self.current_week as i64 - 1) * 7;
        self.start_date() + chrono::Duration::days(days_offset)
    }
}

/// Status of a plan assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlanStatus {
    /// Currently following the plan.
    #[default]
    Active,
    /// Temporarily paused.
    Paused,
    /// Successfully completed.
    Completed,
    /// Gave up or cancelled.
    Abandoned,
}

impl PlanStatus {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::Completed => "Completed",
            Self::Abandoned => "Abandoned",
        }
    }

    /// Get status icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Active => "▶",
            Self::Paused => "⏸",
            Self::Completed => "✓",
            Self::Abandoned => "✗",
        }
    }

    /// Check if the plan can be modified.
    pub fn is_modifiable(&self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }
}

/// Progress summary for a plan assignment.
#[derive(Debug, Clone)]
pub struct PlanProgress {
    /// Current week (1-indexed).
    pub current_week: u8,
    /// Total weeks in the plan.
    pub total_weeks: u8,
    /// Completed workouts so far.
    pub completed_workouts: u32,
    /// Expected workouts by now.
    pub expected_workouts_so_far: u32,
    /// Total workouts in the plan.
    pub total_plan_workouts: u32,
    /// Skipped workouts.
    pub skipped_workouts: u32,
    /// Week progress percentage.
    pub week_progress_percent: f32,
    /// Workout progress percentage.
    pub workout_progress_percent: f32,
    /// Compliance percentage.
    pub compliance_percent: f32,
    /// Whether the user is on track.
    pub is_on_track: bool,
}

impl PlanProgress {
    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "Week {}/{} • {} of {} workouts ({}%)",
            self.current_week,
            self.total_weeks,
            self.completed_workouts,
            self.total_plan_workouts,
            self.workout_progress_percent.round() as i32
        )
    }

    /// Get remaining workouts.
    pub fn remaining_workouts(&self) -> u32 {
        self.total_plan_workouts.saturating_sub(self.completed_workouts)
    }

    /// Get remaining weeks.
    pub fn remaining_weeks(&self) -> u8 {
        self.total_weeks.saturating_sub(self.current_week)
    }
}

/// Days of the week bitmask helpers.
pub mod days {
    /// Monday.
    pub const MON: u8 = 0b0000001;
    /// Tuesday.
    pub const TUE: u8 = 0b0000010;
    /// Wednesday.
    pub const WED: u8 = 0b0000100;
    /// Thursday.
    pub const THU: u8 = 0b0001000;
    /// Friday.
    pub const FRI: u8 = 0b0010000;
    /// Saturday.
    pub const SAT: u8 = 0b0100000;
    /// Sunday.
    pub const SUN: u8 = 0b1000000;
    /// All weekdays.
    pub const WEEKDAYS: u8 = MON | TUE | WED | THU | FRI;
    /// Weekend only.
    pub const WEEKEND: u8 = SAT | SUN;
    /// All days.
    pub const ALL: u8 = 0b1111111;

    /// Create bitmask from day numbers.
    pub fn from_days(days: &[u8]) -> u8 {
        days.iter()
            .filter(|&&d| (1..=7).contains(&d))
            .fold(0, |acc, &d| acc | (1 << (d - 1)))
    }

    /// Convert bitmask to day numbers.
    pub fn to_days(mask: u8) -> Vec<u8> {
        (1..=7).filter(|&d| (mask & (1 << (d - 1))) != 0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training_plans::{DifficultyLevel, Discipline};
    use crate::training_plans::plan::{PlanWeek, TrainingPhase, PlanWorkout};

    #[test]
    fn test_plan_assignment() {
        let assignment = PlanAssignment::new(1, Uuid::new_v4());

        assert!(assignment.is_active());
        assert!(!assignment.is_paused());
        assert!(!assignment.is_finished());
        assert_eq!(assignment.current_week, 1);
    }

    #[test]
    fn test_available_days() {
        let assignment = PlanAssignment::new(1, Uuid::new_v4())
            .with_available_days(days::MON | days::WED | days::FRI);

        assert!(assignment.is_day_available(1)); // Monday
        assert!(!assignment.is_day_available(2)); // Tuesday
        assert!(assignment.is_day_available(3)); // Wednesday
        assert_eq!(assignment.days_per_week(), 3);
    }

    #[test]
    fn test_available_day_names() {
        let assignment = PlanAssignment::new(1, Uuid::new_v4())
            .with_available_days(days::WEEKDAYS);

        let names = assignment.available_day_names();
        assert_eq!(names, vec!["Mon", "Tue", "Wed", "Thu", "Fri"]);
    }

    #[test]
    fn test_status_transitions() {
        let mut assignment = PlanAssignment::new(1, Uuid::new_v4());

        assignment.pause();
        assert!(assignment.is_paused());

        assignment.resume();
        assert!(assignment.is_active());

        assignment.complete();
        assert!(assignment.is_finished());
        assert_eq!(assignment.status, PlanStatus::Completed);
    }

    #[test]
    fn test_progress_tracking() {
        let mut assignment = PlanAssignment::new(1, Uuid::new_v4());

        assignment.record_completed();
        assignment.record_completed();
        assignment.record_skipped();

        assert_eq!(assignment.completed_workouts, 2);
        assert_eq!(assignment.skipped_workouts, 1);
        assert_eq!(assignment.total_workouts_done(), 3);
        assert!((assignment.completion_rate() - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_progress_calculation() {
        let mut plan = TrainingPlan::new(
            Uuid::new_v4(),
            "Test Plan",
            Discipline::Road,
            DifficultyLevel::Beginner,
            "Test",
        );

        let week1 = PlanWeek::new(1, "Week 1", TrainingPhase::Base)
            .with_workouts(vec![
                PlanWorkout::new(1, "W1", 60),
                PlanWorkout::new(3, "W2", 60),
            ]);
        let week2 = PlanWeek::new(2, "Week 2", TrainingPhase::Base)
            .with_workouts(vec![
                PlanWorkout::new(1, "W3", 60),
                PlanWorkout::new(3, "W4", 60),
            ]);

        plan.add_week(week1);
        plan.add_week(week2);

        let mut assignment = PlanAssignment::new(1, plan.id);
        assignment.record_completed();
        assignment.record_completed();

        let progress = assignment.progress(&plan);

        assert_eq!(progress.current_week, 1);
        assert_eq!(progress.total_weeks, 2);
        assert_eq!(progress.completed_workouts, 2);
        assert_eq!(progress.total_plan_workouts, 4);
        assert!((progress.workout_progress_percent - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_days_helpers() {
        assert_eq!(days::from_days(&[1, 3, 5]), days::MON | days::WED | days::FRI);
        assert_eq!(days::to_days(days::WEEKEND), vec![6, 7]);
    }
}
