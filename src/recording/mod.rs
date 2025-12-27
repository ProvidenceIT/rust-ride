//! Recording module for ride data capture and export.

pub mod exporter_csv;
pub mod exporter_fit;
pub mod exporter_tcx;
pub mod recorder;
pub mod types;

pub use exporter_csv::{export_csv, export_csv_to_file, export_summary_csv, generate_csv_filename};
pub use exporter_fit::{export_fit, export_fit_to_file, generate_fit_filename};
pub use exporter_tcx::{export_tcx, export_tcx_to_file, generate_tcx_filename};
pub use recorder::{RecoverableRide, RideRecorder, SmO2Sample, StorageStatus};
pub use types::{
    ExportError, ExportFormat, LiveRideSummary, RecorderConfig, RecorderError, RecordingStatus,
    Ride, RideSample,
};
