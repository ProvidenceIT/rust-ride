//! Training goals module.
//!
//! Manages rider training objectives including:
//! - General fitness goals (improve endurance, lose weight, get faster)
//! - Event-focused goals with target dates
//! - Energy system goals (VO2max, threshold, sprint)

pub mod manager;
pub mod types;

// Re-exports for convenience
pub use manager::GoalManager;
pub use types::{EventType, GoalStatus, GoalType, MetricType, TargetMetric, TrainingGoal};
