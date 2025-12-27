//! RustRide - Indoor Cycling Training Application
//!
//! An open-source, self-hosted indoor cycling training application built in Rust.
//! Provides BLE sensor connectivity, structured workout execution with ERG mode,
//! real-time metrics display, and ride recording with export capabilities.

// Core modules
pub mod goals;
pub mod leaderboards;
pub mod metrics;
pub mod ml;
pub mod networking;
pub mod racing;
pub mod recording;
pub mod sensors;
pub mod social;
pub mod storage;
pub mod ui;
pub mod workouts;
pub mod world;

// Hardware integration modules (Feature 007)
pub mod audio;
pub mod hid;
pub mod integrations;
pub mod video;

// UX & Accessibility modules (Feature 008)
pub mod accessibility;
pub mod i18n;
pub mod input;
pub mod onboarding;

// Re-export commonly used types
pub use metrics::calculator::MetricsCalculator;
pub use recording::recorder::RideRecorder;
pub use sensors::manager::SensorManager;
pub use storage::config::UserProfile;
pub use workouts::engine::WorkoutEngine;
