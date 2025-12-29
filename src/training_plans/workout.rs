//! Scheduled workout definitions.
//!
//! T060: Create ScheduledWorkout and UpcomingWorkout structs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::plan::{TrainingPhase, WorkoutType};

/// A workout that has been scheduled for a specific date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledWorkout {
    /// Unique identifier for this scheduled instance.
    pub id: Uuid,
    /// Reference to the plan assignment.
    pub plan_id: Uuid,
    /// Week number within the plan.
    pub week_number: u8,
    /// Day of the week (1-7).
    pub day_of_week: u8,
    /// The scheduled date.
    pub scheduled_date: NaiveDate,
    /// Reference to the workout definition.
    pub workout_id: Option<Uuid>,
    /// Workout name.
    pub workout_name: String,
    /// Workout description.
    pub description: String,
    /// Duration in minutes.
    pub duration_minutes: u16,
    /// Estimated TSS.
    pub estimated_tss: f32,
    /// Workout type.
    pub workout_type: WorkoutType,
    /// Whether this is optional.
    pub is_optional: bool,
    /// Completion status.
    pub status: WorkoutStatus,
    /// When the workout was completed (if completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Actual ride ID if completed.
    pub ride_id: Option<Uuid>,
    /// Notes from the user.
    pub notes: Option<String>,
}

impl ScheduledWorkout {
    /// Create a new scheduled workout.
    pub fn new(
        plan_id: Uuid,
        week_number: u8,
        day_of_week: u8,
        scheduled_date: NaiveDate,
        workout_name: impl Into<String>,
        duration_minutes: u16,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            plan_id,
            week_number,
            day_of_week,
            scheduled_date,
            workout_id: None,
            workout_name: workout_name.into(),
            description: String::new(),
            duration_minutes,
            estimated_tss: 0.0,
            workout_type: WorkoutType::Endurance,
            is_optional: false,
            status: WorkoutStatus::Pending,
            completed_at: None,
            ride_id: None,
            notes: None,
        }
    }

    /// Set workout ID.
    pub fn with_workout_id(mut self, id: Uuid) -> Self {
        self.workout_id = Some(id);
        self
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set TSS.
    pub fn with_tss(mut self, tss: f32) -> Self {
        self.estimated_tss = tss;
        self
    }

    /// Set workout type.
    pub fn with_type(mut self, workout_type: WorkoutType) -> Self {
        self.workout_type = workout_type;
        self
    }

    /// Mark as optional.
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Mark the workout as completed.
    pub fn complete(&mut self, ride_id: Option<Uuid>) {
        self.status = WorkoutStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.ride_id = ride_id;
    }

    /// Mark the workout as skipped.
    pub fn skip(&mut self, reason: Option<String>) {
        self.status = WorkoutStatus::Skipped;
        self.notes = reason;
    }

    /// Check if the workout is completed.
    pub fn is_completed(&self) -> bool {
        matches!(self.status, WorkoutStatus::Completed)
    }

    /// Check if the workout is skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self.status, WorkoutStatus::Skipped)
    }

    /// Check if the workout is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, WorkoutStatus::Pending)
    }

    /// Check if the workout is overdue.
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        self.is_pending() && self.scheduled_date < today
    }

    /// Get days until/since the workout.
    pub fn days_relative(&self, today: NaiveDate) -> i64 {
        (self.scheduled_date - today).num_days()
    }
}

/// Status of a scheduled workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorkoutStatus {
    /// Not yet done.
    #[default]
    Pending,
    /// Successfully completed.
    Completed,
    /// Intentionally skipped.
    Skipped,
    /// Missed (overdue and not done).
    Missed,
    /// In progress.
    InProgress,
}

impl WorkoutStatus {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Completed => "Completed",
            Self::Skipped => "Skipped",
            Self::Missed => "Missed",
            Self::InProgress => "In Progress",
        }
    }

    /// Get status icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Completed => "✓",
            Self::Skipped => "⊘",
            Self::Missed => "✗",
            Self::InProgress => "▶",
        }
    }
}

/// An upcoming workout with computed context.
#[derive(Debug, Clone)]
pub struct UpcomingWorkout {
    /// The scheduled workout.
    pub workout: ScheduledWorkout,
    /// Plan name for display.
    pub plan_name: String,
    /// Phase of training.
    pub phase: TrainingPhase,
    /// Days until this workout (negative if overdue).
    pub days_until: i64,
    /// Whether this is today's workout.
    pub is_today: bool,
    /// Whether this is tomorrow's workout.
    pub is_tomorrow: bool,
    /// Week title.
    pub week_title: String,
    /// Compliance warning (if applicable).
    pub compliance_warning: Option<String>,
}

impl UpcomingWorkout {
    /// Create from a scheduled workout.
    pub fn from_scheduled(
        workout: ScheduledWorkout,
        plan_name: impl Into<String>,
        phase: TrainingPhase,
        week_title: impl Into<String>,
        today: NaiveDate,
    ) -> Self {
        let days_until = workout.days_relative(today);
        Self {
            is_today: days_until == 0,
            is_tomorrow: days_until == 1,
            compliance_warning: if days_until < 0 {
                Some("Overdue".to_string())
            } else {
                None
            },
            workout,
            plan_name: plan_name.into(),
            phase,
            days_until,
            week_title: week_title.into(),
        }
    }

    /// Get a friendly date description.
    pub fn date_description(&self) -> String {
        if self.is_today {
            "Today".to_string()
        } else if self.is_tomorrow {
            "Tomorrow".to_string()
        } else if self.days_until < 0 {
            format!("{} days ago", -self.days_until)
        } else if self.days_until < 7 {
            self.workout.scheduled_date.format("%A").to_string()
        } else {
            self.workout.scheduled_date.format("%b %d").to_string()
        }
    }

    /// Get priority score for sorting.
    pub fn priority_score(&self) -> i32 {
        let mut score = 0;

        // Today's workouts are highest priority
        if self.is_today {
            score += 1000;
        }

        // Overdue workouts also high priority
        if self.days_until < 0 {
            score += 500;
        }

        // Tomorrow is next
        if self.is_tomorrow {
            score += 100;
        }

        // Non-optional workouts higher priority
        if !self.workout.is_optional {
            score += 50;
        }

        score
    }
}

/// Collection of upcoming workouts for display.
#[derive(Debug, Default)]
pub struct UpcomingWorkoutList {
    /// All upcoming workouts sorted by date.
    pub workouts: Vec<UpcomingWorkout>,
}

impl UpcomingWorkoutList {
    /// Create a new empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a workout to the list.
    pub fn add(&mut self, workout: UpcomingWorkout) {
        self.workouts.push(workout);
    }

    /// Sort workouts by scheduled date.
    pub fn sort_by_date(&mut self) {
        self.workouts.sort_by(|a, b| {
            a.workout.scheduled_date.cmp(&b.workout.scheduled_date)
        });
    }

    /// Sort workouts by priority.
    pub fn sort_by_priority(&mut self) {
        self.workouts.sort_by(|a, b| {
            b.priority_score().cmp(&a.priority_score())
        });
    }

    /// Get today's workouts.
    pub fn today(&self) -> Vec<&UpcomingWorkout> {
        self.workouts.iter().filter(|w| w.is_today).collect()
    }

    /// Get overdue workouts.
    pub fn overdue(&self) -> Vec<&UpcomingWorkout> {
        self.workouts.iter().filter(|w| w.days_until < 0).collect()
    }

    /// Get this week's workouts.
    pub fn this_week(&self) -> Vec<&UpcomingWorkout> {
        self.workouts
            .iter()
            .filter(|w| w.days_until >= 0 && w.days_until < 7)
            .collect()
    }

    /// Check if there are any workouts.
    pub fn is_empty(&self) -> bool {
        self.workouts.is_empty()
    }

    /// Get the count.
    pub fn len(&self) -> usize {
        self.workouts.len()
    }

    /// Get pending workout count.
    pub fn pending_count(&self) -> usize {
        self.workouts
            .iter()
            .filter(|w| w.workout.is_pending())
            .count()
    }

    /// Get completed count.
    pub fn completed_count(&self) -> usize {
        self.workouts
            .iter()
            .filter(|w| w.workout.is_completed())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_scheduled_workout() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let mut workout = ScheduledWorkout::new(
            Uuid::new_v4(),
            1,
            3, // Wednesday
            today,
            "Endurance Ride",
            60,
        );

        assert!(workout.is_pending());
        assert!(!workout.is_completed());

        workout.complete(Some(Uuid::new_v4()));
        assert!(workout.is_completed());
        assert!(workout.completed_at.is_some());
    }

    #[test]
    fn test_workout_is_overdue() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let workout = ScheduledWorkout::new(
            Uuid::new_v4(),
            1,
            1,
            yesterday,
            "Old Workout",
            60,
        );

        assert!(workout.is_overdue(today));
        assert_eq!(workout.days_relative(today), -1);
    }

    #[test]
    fn test_upcoming_workout() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let workout = ScheduledWorkout::new(
            Uuid::new_v4(),
            1,
            3,
            today,
            "Today's Workout",
            60,
        );

        let upcoming = UpcomingWorkout::from_scheduled(
            workout,
            "Test Plan",
            TrainingPhase::Base,
            "Week 1",
            today,
        );

        assert!(upcoming.is_today);
        assert!(!upcoming.is_tomorrow);
        assert_eq!(upcoming.date_description(), "Today");
    }

    #[test]
    fn test_workout_list() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let plan_id = Uuid::new_v4();
        let mut list = UpcomingWorkoutList::new();

        // Add today's workout
        let today_workout = UpcomingWorkout::from_scheduled(
            ScheduledWorkout::new(plan_id, 1, 3, today, "Today", 60),
            "Plan",
            TrainingPhase::Base,
            "Week 1",
            today,
        );
        list.add(today_workout);

        // Add tomorrow's workout
        let tomorrow = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();
        let tomorrow_workout = UpcomingWorkout::from_scheduled(
            ScheduledWorkout::new(plan_id, 1, 4, tomorrow, "Tomorrow", 45),
            "Plan",
            TrainingPhase::Base,
            "Week 1",
            today,
        );
        list.add(tomorrow_workout);

        assert_eq!(list.len(), 2);
        assert_eq!(list.today().len(), 1);
        assert!(list.overdue().is_empty());
    }

    #[test]
    fn test_workout_status() {
        assert_eq!(WorkoutStatus::Pending.display_name(), "Pending");
        assert_eq!(WorkoutStatus::Completed.icon(), "✓");
    }
}
