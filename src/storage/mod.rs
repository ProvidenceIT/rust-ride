//! Storage module for database and configuration.

pub mod config;
pub mod database;
pub mod schema;

pub use config::{AppConfig, UserProfile};
pub use database::{Database, DatabaseError};
