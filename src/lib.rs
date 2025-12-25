//! RustRide - Indoor Cycling Training Application
//!
//! An open-source, self-hosted indoor cycling training application built in Rust.
//! Provides BLE sensor connectivity, structured workout execution with ERG mode,
//! real-time metrics display, and ride recording with export capabilities.

pub mod metrics;
pub mod recording;
pub mod sensors;
pub mod storage;
pub mod ui;
pub mod workouts;
pub mod world;

// Re-export commonly used types
pub use metrics::calculator::MetricsCalculator;
pub use recording::recorder::RideRecorder;
pub use sensors::manager::SensorManager;
pub use storage::config::UserProfile;
pub use workouts::engine::WorkoutEngine;
