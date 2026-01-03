//! Workout module for structured training sessions.

pub mod engine;
pub mod exporter_mrc;
pub mod exporter_zwo;
pub mod library;
pub mod parser_mrc;
pub mod parser_zwo;
pub mod types;

pub use engine::WorkoutEngine;
pub use library::{
    BuiltInWorkout, DifficultyTier, EnergySystem, LibraryError, SearchCriteria, WorkoutCategory,
    WorkoutLibrary,
};
pub use exporter_mrc::{export_mrc, export_mrc_to_file, export_mrc_with_ftp, generate_mrc_filename};
pub use exporter_zwo::{export_zwo, export_zwo_to_file, generate_zwo_filename};
pub use parser_mrc::{parse_mrc, parse_mrc_file};
pub use parser_zwo::{parse_zwo, parse_zwo_file};
pub use types::{
    CadenceTarget, PowerTarget, SegmentProgress, SegmentType, Workout, WorkoutError, WorkoutEvent,
    WorkoutExportError, WorkoutFormat, WorkoutParseError, WorkoutSegment, WorkoutState,
    WorkoutStatus,
};
