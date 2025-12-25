//! Storage module for database and configuration.

pub mod analytics_store;
pub mod config;
pub mod database;
pub mod ml_store;
pub mod schema;

pub use analytics_store::AnalyticsStore;
pub use config::{AppConfig, DashboardLayout, MetricType, Theme, UiSettings, Units, UserProfile};
pub use database::{Database, DatabaseError};
pub use ml_store::{CachedPrediction, FatigueStateRecord, MlStore, WorkoutRecommendationRecord};
