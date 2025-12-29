//! Training plan manager implementation.
//!
//! T062: Create TrainingPlanManager trait implementation.

use chrono::Utc;
use rusqlite::Connection;
use uuid::Uuid;

use super::assignment::{PlanAssignment, PlanProgress, PlanStatus};
use super::library::{get_plan_by_id, all_plans};
use super::plan::TrainingPlan;
use super::progress::ProgressTracker;
use super::scheduler::{PlanScheduler, ScheduleConfig};
use super::workout::{ScheduledWorkout, UpcomingWorkout, UpcomingWorkoutList};
use crate::storage::plan_store::PlanStore;

/// Error type for training plan operations.
#[derive(Debug)]
pub enum PlanError {
    /// Plan not found.
    PlanNotFound(Uuid),
    /// No active plan.
    NoActivePlan,
    /// Plan already active.
    PlanAlreadyActive,
    /// Database error.
    Database(String),
    /// Invalid operation.
    InvalidOperation(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanNotFound(id) => write!(f, "Plan not found: {}", id),
            Self::NoActivePlan => write!(f, "No active plan"),
            Self::PlanAlreadyActive => write!(f, "A plan is already active"),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for PlanError {}

impl From<rusqlite::Error> for PlanError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}

/// Result type for plan operations.
pub type PlanResult<T> = Result<T, PlanError>;

/// Manager for training plan operations.
#[derive(Debug)]
pub struct TrainingPlanManager {
    /// Current user ID.
    user_id: i64,
    /// Currently active assignment (cached).
    current_assignment: Option<PlanAssignment>,
    /// Currently active plan (cached).
    current_plan: Option<TrainingPlan>,
    /// Scheduled workouts for the current plan.
    scheduled_workouts: Vec<ScheduledWorkout>,
    /// Progress tracker.
    #[allow(dead_code)]
    progress_tracker: ProgressTracker,
}

impl TrainingPlanManager {
    /// Create a new training plan manager.
    pub fn new(user_id: i64) -> Self {
        Self {
            user_id,
            current_assignment: None,
            current_plan: None,
            scheduled_workouts: Vec::new(),
            progress_tracker: ProgressTracker::new(),
        }
    }

    /// Create a builder for the manager.
    pub fn builder() -> TrainingPlanManagerBuilder {
        TrainingPlanManagerBuilder::default()
    }

    /// Load current assignment from database.
    pub fn load_current(&mut self, conn: &Connection) -> PlanResult<bool> {
        let record = PlanStore::get_current(conn, self.user_id)?;

        if let Some(record) = record {
            let assignment = PlanAssignment {
                user_id: record.user_id,
                plan_id: record.plan_id,
                started_at: record.started_at,
                current_week: record.current_week,
                completed_workouts: record.completed_workouts,
                skipped_workouts: record.skipped_workouts,
                status: match record.status.as_str() {
                    "active" => PlanStatus::Active,
                    "paused" => PlanStatus::Paused,
                    "completed" => PlanStatus::Completed,
                    "abandoned" => PlanStatus::Abandoned,
                    _ => PlanStatus::Active,
                },
                available_days: record.available_days,
                ends_at: None,
                last_activity_at: None,
            };

            // Load the plan definition
            if let Some(plan) = get_plan_by_id(assignment.plan_id) {
                self.current_plan = Some(plan.clone());
                self.current_assignment = Some(assignment);

                // Generate schedule
                self.regenerate_schedule();

                return Ok(true);
            }
        }

        self.current_assignment = None;
        self.current_plan = None;
        self.scheduled_workouts.clear();
        Ok(false)
    }

    /// Assign a plan to the user.
    pub fn assign_plan(
        &mut self,
        conn: &Connection,
        plan_id: Uuid,
        config: ScheduleConfig,
    ) -> PlanResult<()> {
        // Check if there's already an active plan
        if let Some(ref assignment) = self.current_assignment {
            if assignment.is_active() {
                return Err(PlanError::PlanAlreadyActive);
            }
        }

        // Find the plan
        let plan = get_plan_by_id(plan_id)
            .ok_or(PlanError::PlanNotFound(plan_id))?;

        // Create assignment
        let assignment = PlanAssignment::new(self.user_id, plan_id)
            .with_available_days(config.available_days)
            .with_end_date(plan.duration_weeks);

        // Store in database
        PlanStore::assign(conn, self.user_id, plan_id, config.available_days)?;

        // Update local state
        self.current_plan = Some(plan);
        self.current_assignment = Some(assignment);
        self.regenerate_schedule();

        Ok(())
    }

    /// Record a completed workout.
    pub fn record_workout_completed(
        &mut self,
        conn: &Connection,
        workout_id: Uuid,
        ride_id: Option<Uuid>,
    ) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        // Find and update the scheduled workout
        if let Some(workout) = self.scheduled_workouts.iter_mut().find(|w| w.id == workout_id) {
            workout.complete(ride_id);
        }

        // Update assignment
        assignment.record_completed();

        // Persist
        PlanStore::record_workout_completed(conn, self.user_id)?;

        Ok(())
    }

    /// Record a skipped workout.
    pub fn record_workout_skipped(
        &mut self,
        conn: &Connection,
        workout_id: Uuid,
        reason: Option<String>,
    ) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        // Find and update the scheduled workout
        if let Some(workout) = self.scheduled_workouts.iter_mut().find(|w| w.id == workout_id) {
            workout.skip(reason);
        }

        // Update assignment
        assignment.record_skipped();

        // Persist
        PlanStore::record_workout_skipped(conn, self.user_id)?;

        Ok(())
    }

    /// Advance to the next week.
    pub fn advance_week(&mut self, conn: &Connection) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        let plan = self.current_plan.as_ref()
            .ok_or(PlanError::NoActivePlan)?;

        if assignment.current_week >= plan.duration_weeks {
            // Plan is complete
            assignment.complete();
            PlanStore::update_status(conn, self.user_id, "completed")?;
        } else {
            assignment.advance_week();
            PlanStore::advance_week(conn, self.user_id)?;
        }

        Ok(())
    }

    /// Pause the current plan.
    pub fn pause_plan(&mut self, conn: &Connection) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        if !assignment.is_active() {
            return Err(PlanError::InvalidOperation("Plan is not active".to_string()));
        }

        assignment.pause();
        PlanStore::update_status(conn, self.user_id, "paused")?;

        Ok(())
    }

    /// Resume the current plan.
    pub fn resume_plan(&mut self, conn: &Connection) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        if !assignment.is_paused() {
            return Err(PlanError::InvalidOperation("Plan is not paused".to_string()));
        }

        assignment.resume();
        PlanStore::update_status(conn, self.user_id, "active")?;

        Ok(())
    }

    /// Abandon the current plan.
    pub fn abandon_plan(&mut self, conn: &Connection) -> PlanResult<()> {
        let assignment = self.current_assignment.as_mut()
            .ok_or(PlanError::NoActivePlan)?;

        assignment.abandon();
        PlanStore::update_status(conn, self.user_id, "abandoned")?;

        Ok(())
    }

    /// Get the current plan.
    pub fn current_plan(&self) -> Option<&TrainingPlan> {
        self.current_plan.as_ref()
    }

    /// Get the current assignment.
    pub fn current_assignment(&self) -> Option<&PlanAssignment> {
        self.current_assignment.as_ref()
    }

    /// Check if there's an active plan.
    pub fn has_active_plan(&self) -> bool {
        self.current_assignment
            .as_ref()
            .map(|a| a.is_active())
            .unwrap_or(false)
    }

    /// Get upcoming workouts.
    pub fn upcoming_workouts(&self, limit: usize) -> UpcomingWorkoutList {
        let today = Utc::now().date_naive();
        let mut list = UpcomingWorkoutList::new();

        let Some(plan) = self.current_plan.as_ref() else {
            return list;
        };

        if self.current_assignment.is_none() {
            return list;
        };

        // Get pending workouts
        for workout in self.scheduled_workouts.iter().filter(|w| w.is_pending()) {
            if list.len() >= limit {
                break;
            }

            let week = plan.get_week(workout.week_number);
            let phase = week.map(|w| w.phase).unwrap_or(super::plan::TrainingPhase::Base);
            let week_title = week.map(|w| w.title.as_str()).unwrap_or("Week");

            let upcoming = UpcomingWorkout::from_scheduled(
                workout.clone(),
                &plan.name,
                phase,
                week_title,
                today,
            );
            list.add(upcoming);
        }

        list.sort_by_date();
        list
    }

    /// Get today's workouts.
    pub fn todays_workouts(&self) -> Vec<&ScheduledWorkout> {
        let today = Utc::now().date_naive();
        self.scheduled_workouts
            .iter()
            .filter(|w| w.scheduled_date == today && w.is_pending())
            .collect()
    }

    /// Get progress for the current plan.
    pub fn progress(&self) -> Option<PlanProgress> {
        let assignment = self.current_assignment.as_ref()?;
        let plan = self.current_plan.as_ref()?;
        Some(assignment.progress(plan))
    }

    /// Get all available plans.
    pub fn available_plans(&self) -> Vec<TrainingPlan> {
        all_plans()
    }

    /// Regenerate the workout schedule.
    fn regenerate_schedule(&mut self) {
        self.scheduled_workouts.clear();

        let Some(plan) = self.current_plan.as_ref() else {
            return;
        };

        let Some(assignment) = self.current_assignment.as_ref() else {
            return;
        };

        let scheduler = PlanScheduler::new(ScheduleConfig {
            start_date: assignment.start_date(),
            available_days: assignment.available_days,
        });

        self.scheduled_workouts = scheduler.schedule_plan(plan);
    }

    /// Get a specific workout by ID.
    pub fn get_workout(&self, workout_id: Uuid) -> Option<&ScheduledWorkout> {
        self.scheduled_workouts.iter().find(|w| w.id == workout_id)
    }

    /// Get workouts for a specific week.
    pub fn workouts_for_week(&self, week_number: u8) -> Vec<&ScheduledWorkout> {
        self.scheduled_workouts
            .iter()
            .filter(|w| w.week_number == week_number)
            .collect()
    }
}

/// Builder for TrainingPlanManager.
#[derive(Default)]
pub struct TrainingPlanManagerBuilder {
    user_id: Option<i64>,
}

impl TrainingPlanManagerBuilder {
    /// Set the user ID.
    pub fn user_id(mut self, id: i64) -> Self {
        self.user_id = Some(id);
        self
    }

    /// Build the manager.
    pub fn build(self) -> TrainingPlanManager {
        TrainingPlanManager::new(self.user_id.unwrap_or(1))
    }

    /// Build and load from database.
    pub fn build_and_load(self, conn: &Connection) -> PlanResult<TrainingPlanManager> {
        let mut manager = self.build();
        manager.load_current(conn)?;
        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::training_plans::{DifficultyLevel, Discipline};

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_assignments (
                user_id INTEGER PRIMARY KEY,
                plan_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                current_week INTEGER NOT NULL DEFAULT 1,
                completed_workouts INTEGER NOT NULL DEFAULT 0,
                skipped_workouts INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active',
                available_days INTEGER NOT NULL DEFAULT 127
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_manager_creation() {
        let manager = TrainingPlanManager::new(1);
        assert!(!manager.has_active_plan());
        assert!(manager.current_plan().is_none());
    }

    #[test]
    fn test_builder() {
        let manager = TrainingPlanManager::builder()
            .user_id(42)
            .build();

        assert_eq!(manager.user_id, 42);
    }

    #[test]
    fn test_available_plans() {
        let manager = TrainingPlanManager::new(1);
        let plans = manager.available_plans();

        // Should have plans from the library
        assert!(!plans.is_empty());
    }

    #[test]
    fn test_assign_plan() {
        let conn = setup_test_db();
        let mut manager = TrainingPlanManager::new(1);

        let plans = manager.available_plans();
        let plan = &plans[0];

        let config = ScheduleConfig {
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            available_days: 0b1111111,
        };

        let result = manager.assign_plan(&conn, plan.id, config);
        assert!(result.is_ok());
        assert!(manager.has_active_plan());
    }

    #[test]
    fn test_load_from_empty_db() {
        let conn = setup_test_db();
        let mut manager = TrainingPlanManager::new(1);

        let result = manager.load_current(&conn);
        assert!(result.is_ok());
        assert!(!manager.has_active_plan());
    }
}
