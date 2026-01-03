//! Social features module
//!
//! Provides rider profiles, clubs, badges, challenges, and activity feed.

pub mod badges;
pub mod challenges;
pub mod clubs;
pub mod export;
pub mod feed;
pub mod profile;
pub mod types;

// Re-export commonly used types
pub use types::*;

// Re-export profile export/import types
pub use export::{
    AvatarExport, ConflictResolution, FtpHistoryEntry, ProfileConflict, ProfileData,
    ProfileExport, ProfileExportError, ProfileExporter, ProfileImportResult,
};
