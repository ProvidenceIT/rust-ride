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
pub use library::{get_plan_by_id, get_plans_for_discipline, all_plans, PlanLibrary};
pub use manager::{TrainingPlanManager, TrainingPlanManagerBuilder};
pub use plan::{PlanWeek, PlanWorkout, TrainingPlan, TrainingPhase, WorkoutType};
pub use progress::{ProgressTracker, WeekSummary, PlanComplianceReport};
pub use scheduler::{PlanScheduler, ScheduleConfig};
pub use workout::{ScheduledWorkout, UpcomingWorkout, UpcomingWorkoutList, WorkoutStatus};
pub use workout_loader::WorkoutLoader;
