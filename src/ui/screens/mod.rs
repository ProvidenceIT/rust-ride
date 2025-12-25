//! UI screens for the application.

pub mod analytics;
pub mod avatar;
pub mod home;
pub mod ride;
pub mod ride_detail;
pub mod ride_history;
pub mod ride_summary;
pub mod sensor_setup;
pub mod settings;
pub mod workout_library;
pub mod world_select;

pub use crate::recording::types::ExportFormat;
pub use analytics::{AnalyticsScreen, AnalyticsTab};
pub use avatar::AvatarScreen;
pub use home::HomeScreen;
pub use ride::RideScreen;
pub use ride_detail::{ExportFormat as DetailExportFormat, RideDetailAction, RideDetailScreen};
pub use ride_history::{DateFilter, RideHistoryScreen, SortOrder};
pub use ride_summary::{RideSummaryAction, RideSummaryScreen};
pub use sensor_setup::SensorSetupScreen;
pub use settings::{SettingsAction, SettingsScreen};
pub use workout_library::{WorkoutImportError, WorkoutLibraryScreen};
pub use world_select::{WorldRouteSelection, WorldSelectScreen};

/// Screen navigation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// Home screen
    #[default]
    Home,
    /// Sensor setup screen
    SensorSetup,
    /// Workout library screen
    WorkoutLibrary,
    /// Active ride screen
    Ride,
    /// Ride summary screen
    RideSummary,
    /// Ride history screen
    RideHistory,
    /// Ride detail screen
    RideDetail,
    /// Settings screen
    Settings,
    /// World selection screen
    WorldSelect,
    /// Avatar customization screen
    Avatar,
    /// Analytics screen (training science)
    Analytics,
}
