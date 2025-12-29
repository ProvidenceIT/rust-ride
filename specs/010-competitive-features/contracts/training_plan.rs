//! Training Plan Contract
//!
//! Defines the interface for multi-discipline training plans.

use chrono::{DateTime, NaiveDate, Utc, Weekday};
use uuid::Uuid;

/// Cycling discipline for plan specialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discipline {
    /// Road racing and criteriums
    Road,
    /// Gravel and adventure riding
    Gravel,
    /// Triathlon (bike-focused)
    Triathlon,
    /// Mountain biking
    MTB,
    /// General fitness and base building
    GeneralFitness,
}

impl Discipline {
    /// Get all discipline variants.
    pub fn all() -> &'static [Discipline] {
        &[
            Discipline::Road,
            Discipline::Gravel,
            Discipline::Triathlon,
            Discipline::MTB,
            Discipline::GeneralFitness,
        ]
    }

    /// Display name for the discipline.
    pub fn display_name(&self) -> &'static str {
        match self {
            Discipline::Road => "Road Racing",
            Discipline::Gravel => "Gravel",
            Discipline::Triathlon => "Triathlon",
            Discipline::MTB => "Mountain Bike",
            Discipline::GeneralFitness => "General Fitness",
        }
    }
}

/// Plan difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyLevel {
    /// 3-4 hours/week, lower intensity
    Beginner,
    /// 5-8 hours/week, mixed intensity
    Intermediate,
    /// 8-12+ hours/week, high intensity
    Advanced,
}

/// A training plan definition.
#[derive(Debug, Clone)]
pub struct TrainingPlan {
    /// Unique identifier
    pub id: Uuid,
    /// Plan name
    pub name: String,
    /// Target discipline
    pub discipline: Discipline,
    /// Duration in weeks
    pub duration_weeks: u8,
    /// Recommended workouts per week
    pub workouts_per_week: u8,
    /// Estimated weekly hours
    pub weekly_hours: f32,
    /// Description
    pub description: String,
    /// Difficulty level
    pub difficulty: DifficultyLevel,
    /// Weekly workout schedule
    pub weeks: Vec<PlanWeek>,
}

/// A week within a training plan.
#[derive(Debug, Clone)]
pub struct PlanWeek {
    /// Week number (1-based)
    pub week_number: u8,
    /// Week focus (e.g., "Base Building", "VO2max", "Recovery")
    pub focus: String,
    /// Total scheduled TSS for the week
    pub target_tss: Option<u32>,
    /// Workouts for this week
    pub workouts: Vec<ScheduledWorkout>,
}

/// A workout scheduled within a plan.
#[derive(Debug, Clone)]
pub struct ScheduledWorkout {
    /// Day of week (1=Monday, 7=Sunday)
    pub day_of_week: u8,
    /// Reference to workout definition
    pub workout_id: Uuid,
    /// Workout name (for display)
    pub workout_name: String,
    /// Workout duration (minutes)
    pub duration_minutes: u16,
    /// Expected TSS
    pub expected_tss: Option<u16>,
    /// Whether this workout can be skipped
    pub is_optional: bool,
    /// Alternative workout ID
    pub alternative_id: Option<Uuid>,
}

/// User's active plan assignment.
#[derive(Debug, Clone)]
pub struct PlanAssignment {
    /// Assigned plan
    pub plan: TrainingPlan,
    /// Start date
    pub started_at: NaiveDate,
    /// Current week number
    pub current_week: u8,
    /// Workouts completed
    pub completed_count: u32,
    /// Workouts skipped
    pub skipped_count: u32,
    /// Assignment status
    pub status: PlanStatus,
    /// User's available training days
    pub available_days: Vec<Weekday>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    /// Currently following plan
    Active,
    /// Temporarily paused
    Paused,
    /// Successfully finished
    Completed,
    /// User quit before completion
    Abandoned,
}

/// A scheduled workout instance with date.
#[derive(Debug, Clone)]
pub struct UpcomingWorkout {
    /// Scheduled date
    pub date: NaiveDate,
    /// Workout details
    pub workout: ScheduledWorkout,
    /// Week number this belongs to
    pub week_number: u8,
    /// Whether already completed
    pub is_completed: bool,
}

/// Manages training plan lifecycle.
pub trait TrainingPlanManager: Send + Sync {
    /// Get all available plans.
    fn get_all_plans(&self) -> Vec<TrainingPlan>;

    /// Get plans filtered by discipline.
    fn get_plans_by_discipline(&self, discipline: Discipline) -> Vec<TrainingPlan>;

    /// Get plans filtered by difficulty.
    fn get_plans_by_difficulty(&self, difficulty: DifficultyLevel) -> Vec<TrainingPlan>;

    /// Get a specific plan by ID.
    fn get_plan(&self, id: Uuid) -> Option<TrainingPlan>;

    /// Assign a plan to the user.
    ///
    /// # Arguments
    /// * `plan_id` - Plan to assign
    /// * `start_date` - When to start the plan
    /// * `available_days` - Days user can train
    fn assign_plan(
        &mut self,
        plan_id: Uuid,
        start_date: NaiveDate,
        available_days: Vec<Weekday>,
    ) -> Result<PlanAssignment, PlanError>;

    /// Get current plan assignment (if any).
    fn get_current_assignment(&self) -> Option<PlanAssignment>;

    /// Mark a workout as completed.
    fn complete_workout(&mut self, date: NaiveDate, workout_id: Uuid) -> Result<(), PlanError>;

    /// Skip a workout.
    fn skip_workout(&mut self, date: NaiveDate, workout_id: Uuid) -> Result<(), PlanError>;

    /// Swap a workout for its alternative.
    fn swap_workout(&mut self, date: NaiveDate, workout_id: Uuid) -> Result<(), PlanError>;

    /// Get upcoming workouts for the next N days.
    fn get_upcoming_workouts(&self, days: u8) -> Vec<UpcomingWorkout>;

    /// Get today's scheduled workout (if any).
    fn get_todays_workout(&self) -> Option<UpcomingWorkout>;

    /// Pause current plan.
    fn pause_plan(&mut self) -> Result<(), PlanError>;

    /// Resume paused plan.
    fn resume_plan(&mut self) -> Result<(), PlanError>;

    /// Abandon current plan.
    fn abandon_plan(&mut self) -> Result<(), PlanError>;

    /// Reschedule plan start date.
    fn reschedule_plan(&mut self, new_start_date: NaiveDate) -> Result<(), PlanError>;
}

/// Errors from plan operations.
#[derive(Debug, Clone)]
pub enum PlanError {
    /// Plan not found
    PlanNotFound(Uuid),
    /// No active plan assignment
    NoPlanAssigned,
    /// Plan already assigned
    PlanAlreadyAssigned,
    /// Workout not found in plan
    WorkoutNotFound(Uuid),
    /// Invalid date
    InvalidDate(String),
    /// Storage error
    StorageError(String),
}
