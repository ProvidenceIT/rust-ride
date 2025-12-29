//! Progress tracking for training plans.
//!
//! T065: Implement workout completion/skip tracking.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::assignment::PlanAssignment;
use super::plan::TrainingPlan;
use super::workout::ScheduledWorkout;

/// Tracker for plan progress and compliance.
#[derive(Debug, Default)]
pub struct ProgressTracker {
    /// Weekly summaries.
    weekly_summaries: Vec<WeekSummary>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate weekly summaries from scheduled workouts.
    pub fn generate_summaries(&mut self, plan: &TrainingPlan, workouts: &[ScheduledWorkout]) {
        self.weekly_summaries.clear();

        for week in &plan.weeks {
            let week_workouts: Vec<_> = workouts
                .iter()
                .filter(|w| w.week_number == week.week_number)
                .collect();

            let summary = WeekSummary::from_workouts(
                week.week_number,
                &week.title,
                week.phase,
                &week_workouts,
            );

            self.weekly_summaries.push(summary);
        }
    }

    /// Get all weekly summaries.
    pub fn weekly_summaries(&self) -> &[WeekSummary] {
        &self.weekly_summaries
    }

    /// Get summary for a specific week.
    pub fn week_summary(&self, week_number: u8) -> Option<&WeekSummary> {
        self.weekly_summaries
            .iter()
            .find(|s| s.week_number == week_number)
    }

    /// Generate a compliance report.
    pub fn compliance_report(
        &self,
        assignment: &PlanAssignment,
        plan: &TrainingPlan,
        workouts: &[ScheduledWorkout],
    ) -> PlanComplianceReport {
        let total_workouts = workouts.len() as u32;
        let completed = workouts.iter().filter(|w| w.is_completed()).count() as u32;
        let skipped = workouts.iter().filter(|w| w.is_skipped()).count() as u32;
        let pending = workouts.iter().filter(|w| w.is_pending()).count() as u32;

        let today = Utc::now().date_naive();
        let overdue = workouts
            .iter()
            .filter(|w| w.is_pending() && w.scheduled_date < today)
            .count() as u32;

        let total_tss_completed: f32 = workouts
            .iter()
            .filter(|w| w.is_completed())
            .map(|w| w.estimated_tss)
            .sum();

        let total_tss_planned: f32 = workouts.iter().map(|w| w.estimated_tss).sum();

        let total_hours_completed: f32 = workouts
            .iter()
            .filter(|w| w.is_completed())
            .map(|w| w.duration_minutes as f32 / 60.0)
            .sum();

        let completion_rate = if total_workouts > 0 {
            (completed as f32 / total_workouts as f32) * 100.0
        } else {
            0.0
        };

        let streak = self.calculate_streak(workouts);

        PlanComplianceReport {
            plan_name: plan.name.clone(),
            current_week: assignment.current_week,
            total_weeks: plan.duration_weeks,
            total_workouts,
            completed_workouts: completed,
            skipped_workouts: skipped,
            pending_workouts: pending,
            overdue_workouts: overdue,
            completion_rate,
            total_tss_completed,
            total_tss_planned,
            total_hours_completed,
            current_streak: streak,
            weekly_summaries: self.weekly_summaries.clone(),
        }
    }

    /// Calculate the current workout streak.
    fn calculate_streak(&self, workouts: &[ScheduledWorkout]) -> u32 {
        // Sort by date descending
        let mut sorted: Vec<_> = workouts.iter().filter(|w| !w.is_pending()).collect();
        sorted.sort_by(|a, b| b.scheduled_date.cmp(&a.scheduled_date));

        let mut streak = 0u32;
        for workout in sorted {
            if workout.is_completed() {
                streak += 1;
            } else {
                break;
            }
        }
        streak
    }
}

/// Summary for a single week.
#[derive(Debug, Clone)]
pub struct WeekSummary {
    /// Week number.
    pub week_number: u8,
    /// Week title.
    pub title: String,
    /// Training phase.
    pub phase: super::plan::TrainingPhase,
    /// Total workouts in the week.
    pub total_workouts: usize,
    /// Completed workouts.
    pub completed: usize,
    /// Skipped workouts.
    pub skipped: usize,
    /// Pending workouts.
    pub pending: usize,
    /// Total planned TSS.
    pub planned_tss: f32,
    /// Completed TSS.
    pub completed_tss: f32,
    /// Total planned hours.
    pub planned_hours: f32,
    /// Completed hours.
    pub completed_hours: f32,
    /// Compliance percentage.
    pub compliance_percent: f32,
}

impl WeekSummary {
    /// Create a summary from workouts.
    pub fn from_workouts(
        week_number: u8,
        title: &str,
        phase: super::plan::TrainingPhase,
        workouts: &[&ScheduledWorkout],
    ) -> Self {
        let total_workouts = workouts.len();
        let completed = workouts.iter().filter(|w| w.is_completed()).count();
        let skipped = workouts.iter().filter(|w| w.is_skipped()).count();
        let pending = workouts.iter().filter(|w| w.is_pending()).count();

        let planned_tss: f32 = workouts.iter().map(|w| w.estimated_tss).sum();
        let completed_tss: f32 = workouts
            .iter()
            .filter(|w| w.is_completed())
            .map(|w| w.estimated_tss)
            .sum();

        let planned_hours: f32 = workouts
            .iter()
            .map(|w| w.duration_minutes as f32 / 60.0)
            .sum();
        let completed_hours: f32 = workouts
            .iter()
            .filter(|w| w.is_completed())
            .map(|w| w.duration_minutes as f32 / 60.0)
            .sum();

        let compliance_percent = if total_workouts > 0 {
            (completed as f32 / total_workouts as f32) * 100.0
        } else {
            0.0
        };

        Self {
            week_number,
            title: title.to_string(),
            phase,
            total_workouts,
            completed,
            skipped,
            pending,
            planned_tss,
            completed_tss,
            planned_hours,
            completed_hours,
            compliance_percent,
        }
    }

    /// Check if the week is fully completed.
    pub fn is_complete(&self) -> bool {
        self.pending == 0
    }

    /// Get the week's status.
    pub fn status(&self) -> WeekStatus {
        if self.is_complete() {
            if self.compliance_percent >= 80.0 {
                WeekStatus::CompletedGood
            } else {
                WeekStatus::CompletedPoor
            }
        } else if self.completed > 0 || self.skipped > 0 {
            WeekStatus::InProgress
        } else {
            WeekStatus::NotStarted
        }
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "{}: {} of {} workouts ({:.0}%)",
            self.title, self.completed, self.total_workouts, self.compliance_percent
        )
    }
}

/// Status of a week's progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekStatus {
    /// Week hasn't started yet.
    NotStarted,
    /// Week is in progress.
    InProgress,
    /// Completed with good compliance.
    CompletedGood,
    /// Completed with poor compliance.
    CompletedPoor,
}

impl WeekStatus {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NotStarted => "Not Started",
            Self::InProgress => "In Progress",
            Self::CompletedGood => "Completed",
            Self::CompletedPoor => "Needs Improvement",
        }
    }

    /// Get status icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::NotStarted => "○",
            Self::InProgress => "◐",
            Self::CompletedGood => "●",
            Self::CompletedPoor => "◑",
        }
    }
}

/// Comprehensive compliance report for a plan.
#[derive(Debug, Clone)]
pub struct PlanComplianceReport {
    /// Plan name.
    pub plan_name: String,
    /// Current week.
    pub current_week: u8,
    /// Total weeks.
    pub total_weeks: u8,
    /// Total workouts in the plan.
    pub total_workouts: u32,
    /// Completed workouts.
    pub completed_workouts: u32,
    /// Skipped workouts.
    pub skipped_workouts: u32,
    /// Pending workouts.
    pub pending_workouts: u32,
    /// Overdue workouts.
    pub overdue_workouts: u32,
    /// Overall completion rate percentage.
    pub completion_rate: f32,
    /// Total TSS completed.
    pub total_tss_completed: f32,
    /// Total TSS planned.
    pub total_tss_planned: f32,
    /// Total hours completed.
    pub total_hours_completed: f32,
    /// Current workout streak.
    pub current_streak: u32,
    /// Weekly summaries.
    pub weekly_summaries: Vec<WeekSummary>,
}

impl PlanComplianceReport {
    /// Get a brief summary string.
    pub fn summary(&self) -> String {
        format!(
            "{}: Week {}/{}, {:.0}% complete ({} workouts)",
            self.plan_name,
            self.current_week,
            self.total_weeks,
            self.completion_rate,
            self.completed_workouts
        )
    }

    /// Get compliance grade (A, B, C, D, F).
    pub fn grade(&self) -> char {
        if self.completion_rate >= 90.0 {
            'A'
        } else if self.completion_rate >= 80.0 {
            'B'
        } else if self.completion_rate >= 70.0 {
            'C'
        } else if self.completion_rate >= 60.0 {
            'D'
        } else {
            'F'
        }
    }

    /// Check if there are overdue workouts.
    pub fn has_overdue(&self) -> bool {
        self.overdue_workouts > 0
    }

    /// Get TSS completion percentage.
    pub fn tss_completion_percent(&self) -> f32 {
        if self.total_tss_planned > 0.0 {
            (self.total_tss_completed / self.total_tss_planned) * 100.0
        } else {
            0.0
        }
    }

    /// Get progress through the plan as a percentage.
    pub fn progress_percent(&self) -> f32 {
        if self.total_weeks > 0 {
            ((self.current_week - 1) as f32 / self.total_weeks as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Track a workout completion event.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkoutCompletionEvent {
    /// Workout ID.
    pub workout_id: Uuid,
    /// Ride ID (if completed with a ride).
    pub ride_id: Option<Uuid>,
    /// When completed.
    pub completed_at: DateTime<Utc>,
    /// Actual duration in minutes.
    pub actual_duration_minutes: Option<u16>,
    /// Actual TSS (if known).
    pub actual_tss: Option<f32>,
    /// Was the workout completed as planned?
    pub completed_as_planned: bool,
    /// Notes.
    pub notes: Option<String>,
}

#[allow(dead_code)]
impl WorkoutCompletionEvent {
    /// Create a new completion event.
    pub fn new(workout_id: Uuid) -> Self {
        Self {
            workout_id,
            ride_id: None,
            completed_at: Utc::now(),
            actual_duration_minutes: None,
            actual_tss: None,
            completed_as_planned: true,
            notes: None,
        }
    }

    /// Set ride ID.
    pub fn with_ride(mut self, ride_id: Uuid) -> Self {
        self.ride_id = Some(ride_id);
        self
    }

    /// Set actual duration.
    pub fn with_duration(mut self, minutes: u16) -> Self {
        self.actual_duration_minutes = Some(minutes);
        self
    }

    /// Set actual TSS.
    pub fn with_tss(mut self, tss: f32) -> Self {
        self.actual_tss = Some(tss);
        self
    }

    /// Set notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Mark as not completed as planned.
    pub fn not_as_planned(mut self) -> Self {
        self.completed_as_planned = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training_plans::plan::{PlanWeek, PlanWorkout, TrainingPhase, WorkoutType};
    use crate::training_plans::{DifficultyLevel, Discipline};

    fn create_test_data() -> (TrainingPlan, Vec<ScheduledWorkout>, PlanAssignment) {
        let plan_id = Uuid::new_v4();
        let mut plan = TrainingPlan::new(
            plan_id,
            "Test Plan",
            Discipline::Road,
            DifficultyLevel::Beginner,
            "Test",
        );

        let week1 = PlanWeek::new(1, "Week 1", TrainingPhase::Base).with_workouts(vec![
            PlanWorkout::new(1, "Workout 1", 60),
            PlanWorkout::new(3, "Workout 2", 60),
        ]);
        plan.add_week(week1);

        let today = Utc::now().date_naive();

        let mut workouts = vec![
            ScheduledWorkout::new(plan_id, 1, 1, today, "Workout 1", 60).with_tss(50.0),
            ScheduledWorkout::new(plan_id, 1, 3, today, "Workout 2", 60).with_tss(50.0),
        ];

        // Complete first workout
        workouts[0].complete(None);

        let assignment = PlanAssignment::new(1, plan_id);

        (plan, workouts, assignment)
    }

    #[test]
    fn test_week_summary() {
        let (plan, workouts, _) = create_test_data();
        let workout_refs: Vec<_> = workouts.iter().collect();

        let summary = WeekSummary::from_workouts(1, "Week 1", TrainingPhase::Base, &workout_refs);

        assert_eq!(summary.total_workouts, 2);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.pending, 1);
        assert!((summary.compliance_percent - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_progress_tracker() {
        let (plan, workouts, _) = create_test_data();
        let mut tracker = ProgressTracker::new();
        tracker.generate_summaries(&plan, &workouts);

        let summaries = tracker.weekly_summaries();
        assert_eq!(summaries.len(), 1);
    }

    #[test]
    fn test_compliance_report() {
        let (plan, workouts, assignment) = create_test_data();
        let mut tracker = ProgressTracker::new();
        tracker.generate_summaries(&plan, &workouts);

        let report = tracker.compliance_report(&assignment, &plan, &workouts);

        assert_eq!(report.total_workouts, 2);
        assert_eq!(report.completed_workouts, 1);
        assert!((report.completion_rate - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_compliance_grade() {
        let report = PlanComplianceReport {
            plan_name: "Test".to_string(),
            current_week: 1,
            total_weeks: 4,
            total_workouts: 10,
            completed_workouts: 9,
            skipped_workouts: 0,
            pending_workouts: 1,
            overdue_workouts: 0,
            completion_rate: 90.0,
            total_tss_completed: 400.0,
            total_tss_planned: 500.0,
            total_hours_completed: 8.0,
            current_streak: 5,
            weekly_summaries: Vec::new(),
        };

        assert_eq!(report.grade(), 'A');
    }

    #[test]
    fn test_workout_completion_event() {
        let event = WorkoutCompletionEvent::new(Uuid::new_v4())
            .with_ride(Uuid::new_v4())
            .with_duration(65)
            .with_tss(55.0)
            .with_notes("Felt great!");

        assert!(event.ride_id.is_some());
        assert_eq!(event.actual_duration_minutes, Some(65));
        assert!(event.completed_as_planned);
    }
}
