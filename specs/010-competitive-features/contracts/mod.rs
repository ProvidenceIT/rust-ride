//! Competitive Features Contracts
//!
//! This module defines the public interfaces for the competitive feature gaps:
//!
//! 1. **Gradient Controller** - Gradient-responsive resistance for GPX routes
//! 2. **Achievement Tracker** - Achievement badges and XP system
//! 3. **Power Profile** - 4D power profiling with rolling windows
//! 4. **Training Plan** - Multi-discipline training plans
//! 5. **Career Levels** - Career progression with cosmetic rewards
//!
//! These contracts define the trait interfaces that implementations must satisfy.
//! They are designed to be testable, with clear input/output boundaries.

pub mod achievement_tracker;
pub mod career_levels;
pub mod gradient_controller;
pub mod power_profile;
pub mod training_plan;

pub use achievement_tracker::{
    Achievement, AchievementCategory, AchievementNotification, AchievementTier, AchievementTracker,
    EarnedAchievement, RideMetrics, UserLevel,
};
pub use career_levels::{
    CareerError, CareerManager, CareerStatus, LevelUpEvent, Reward, RewardType, UnlockedReward,
    MAX_LEVEL, MILESTONE_LEVELS,
};
pub use gradient_controller::{
    GradientController, GradientError, GradientPoint, GradientResult, GradientSettings,
};
pub use power_profile::{
    DurationStrength, PowerPoint, PowerProfile, PowerProfileManager, ProfileAnalysis, ProfileError,
    ProfileType, RiderType, PROFILE_DURATIONS,
};
pub use training_plan::{
    Discipline, DifficultyLevel, PlanAssignment, PlanError, PlanStatus, PlanWeek, ScheduledWorkout,
    TrainingPlan, TrainingPlanManager, UpcomingWorkout,
};
