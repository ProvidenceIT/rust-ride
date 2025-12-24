//! UI screens for the application.

pub mod home;
pub mod ride;
pub mod sensor_setup;
pub mod workout_library;

pub use home::HomeScreen;
pub use ride::RideScreen;
pub use sensor_setup::SensorSetupScreen;
pub use workout_library::WorkoutLibraryScreen;

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
}
