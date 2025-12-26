//! Social features module
//!
//! Provides rider profiles, clubs, badges, challenges, and activity feed.

pub mod badges;
pub mod challenges;
pub mod clubs;
pub mod feed;
pub mod profile;
pub mod types;

// Re-export commonly used types
pub use types::*;
