//! Workout module for structured training sessions.

pub mod engine;
pub mod types;

pub use engine::WorkoutEngine;
pub use types::{
    CadenceTarget, PowerTarget, SegmentProgress, SegmentType, Workout, WorkoutError,
    WorkoutFormat, WorkoutParseError, WorkoutSegment, WorkoutState, WorkoutStatus,
};
