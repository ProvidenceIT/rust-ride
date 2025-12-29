//! Training plans module for multi-discipline structured training.
//!
//! Provides pre-built training plans for road, gravel, triathlon, MTB, and
//! general fitness, with scheduling, progress tracking, and customization.

mod assignment;
mod disciplines;
mod library;
mod manager;
mod plan;
mod progress;
mod scheduler;
mod workout;
mod workout_loader;

pub use assignment::{days, PlanAssignment, PlanProgress, PlanStatus};
pub use disciplines::{DifficultyLevel, Discipline};
pub use library::{all_plans, get_plan_by_id, get_plans_for_discipline, PlanLibrary};
pub use manager::{TrainingPlanManager, TrainingPlanManagerBuilder};
pub use plan::{PlanWeek, PlanWorkout, TrainingPhase, TrainingPlan, WorkoutType};
pub use progress::{PlanComplianceReport, ProgressTracker, WeekSummary};
pub use scheduler::{PlanScheduler, ScheduleConfig};
pub use workout::{ScheduledWorkout, UpcomingWorkout, UpcomingWorkoutList, WorkoutStatus};
pub use workout_loader::WorkoutLoader;
