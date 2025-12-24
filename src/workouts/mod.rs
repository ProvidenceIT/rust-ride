//! Workout module for structured training sessions.

pub mod engine;
pub mod parser_mrc;
pub mod parser_zwo;
pub mod types;

pub use engine::WorkoutEngine;
pub use parser_mrc::{parse_mrc, parse_mrc_file};
pub use parser_zwo::{parse_zwo, parse_zwo_file};
pub use types::{
    CadenceTarget, PowerTarget, SegmentProgress, SegmentType, Workout, WorkoutError, WorkoutFormat,
    WorkoutParseError, WorkoutSegment, WorkoutState, WorkoutStatus,
};
