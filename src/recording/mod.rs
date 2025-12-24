//! Recording module for ride data capture and export.

pub mod recorder;
pub mod types;

pub use recorder::RideRecorder;
pub use types::{
    ExportError, ExportFormat, LiveRideSummary, RecorderConfig, RecorderError, RecordingStatus,
    Ride, RideSample,
};
