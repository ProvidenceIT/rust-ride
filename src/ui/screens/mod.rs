//! UI screens for the application.

pub mod activity_feed;
pub mod analytics;
pub mod avatar;
pub mod challenges;
pub mod clubs;
pub mod group_ride;
pub mod home;
pub mod leaderboard;
pub mod race_lobby;
pub mod ride;
pub mod ride_detail;
pub mod ride_history;
pub mod ride_summary;
pub mod rider_profile;
pub mod route_browser;
pub mod route_import;
pub mod sensor_setup;
pub mod settings;
pub mod streaming;
pub mod workout_library;
pub mod world_select;

pub use crate::recording::types::ExportFormat;
pub use activity_feed::{ActivityFeedAction, ActivityFeedScreen};
pub use analytics::{AnalyticsScreen, AnalyticsTab};
pub use avatar::AvatarScreen;
pub use challenges::{ChallengesAction, ChallengesScreen};
pub use clubs::{ClubsAction, ClubsScreen};
pub use group_ride::{GroupRideAction, GroupRideScreen};
pub use home::HomeScreen;
pub use leaderboard::{LeaderboardAction, LeaderboardScreen};
pub use race_lobby::{RaceLobbyAction, RaceLobbyScreen};
pub use ride::RideScreen;
pub use ride_detail::{ExportFormat as DetailExportFormat, RideDetailAction, RideDetailScreen};
pub use ride_history::{DateFilter, RideHistoryScreen, SortOrder};
pub use ride_summary::{RideSummaryAction, RideSummaryScreen};
pub use rider_profile::{RiderProfileAction, RiderProfileScreen};
pub use route_browser::{RouteBrowserAction, RouteBrowserScreen, RouteSortOrder};
pub use route_import::{RouteImportAction, RouteImportScreen};
pub use sensor_setup::SensorSetupScreen;
pub use settings::{SettingsAction, SettingsScreen};
pub use streaming::{StreamingAction, StreamingScreen};
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
    /// Route import screen
    RouteImport,
    /// Route browser screen
    RouteBrowser,
    /// Group ride screen
    GroupRide,
    /// Leaderboard screen
    Leaderboard,
    /// Challenges screen
    Challenges,
    /// Activity feed screen
    ActivityFeed,
    /// Clubs screen
    Clubs,
    /// Race lobby screen
    RaceLobby,
    /// Rider profile screen
    RiderProfile,
    /// Streaming screen (external displays)
    Streaming,
}
