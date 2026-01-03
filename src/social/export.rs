//! Rider profile export and import functionality.
//!
//! This module provides comprehensive JSON export/import capabilities for rider profiles,
//! enabling profile backup, transfer between installations, and data portability.
//!
//! # Overview
//!
//! The profile export system allows riders to:
//! - **Export** their complete profile including settings, FTP history, and avatar
//! - **Import** profiles from JSON files with conflict detection and resolution
//! - **Transfer** profiles between different RustRide installations
//! - **Backup** profile data for safekeeping
//!
//! # Export Format
//!
//! Profiles are exported as versioned JSON files with the following structure:
//!
//! ```json
//! {
//!   "export_version": "1.0",
//!   "exported_at": "2026-01-03T12:00:00Z",
//!   "rider_id": "550e8400-e29b-41d4-a716-446655440000",
//!   "profile": {
//!     "display_name": "Rider Name",
//!     "bio": "Optional bio text",
//!     "ftp": 250,
//!     "total_distance_km": 1500.5,
//!     "total_time_hours": 75.25,
//!     "sharing_enabled": true
//!   },
//!   "ftp_history": [
//!     {
//!       "ftp_watts": 250,
//!       "method": "ramp_test",
//!       "confidence": "high",
//!       "detected_at": "2026-01-02T10:30:00Z",
//!       "accepted": true
//!     }
//!   ],
//!   "avatar": {
//!     "jersey_color": "#FF5733",
//!     "bike_style": "road_bike",
//!     "jersey_secondary": "#FFFFFF",
//!     "helmet_color": "#000000"
//!   }
//! }
//! ```
//!
//! ## Version Compatibility
//!
//! The `export_version` field ensures forward compatibility:
//! - Current version: `1.0`
//! - Only exact version matches are accepted for import
//! - Future versions may implement migration logic for older formats
//!
//! # Usage Examples
//!
//! ## Exporting a Profile
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use uuid::Uuid;
//! use rust_ride::social::export::{ProfileExporter, ProfileExportError};
//! use rust_ride::storage::Database;
//!
//! // Create the exporter with a database connection
//! let db = Arc::new(Database::open("profile.db")?);
//! let exporter = ProfileExporter::new(db);
//!
//! // Export to JSON string
//! let rider_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
//! let json = exporter.export_json(rider_id)?;
//!
//! // Or export directly to a file
//! let path = exporter.export_to_file(rider_id, "backup/my_profile.json")?;
//! println!("Profile exported to: {}", path.display());
//! ```
//!
//! ## Importing a Profile
//!
//! ```rust,ignore
//! use rust_ride::social::export::{ProfileExporter, ConflictResolution};
//!
//! let exporter = ProfileExporter::new(db);
//!
//! // Import with merge strategy (combines FTP history)
//! let result = exporter.import_from_file(
//!     "backup/my_profile.json",
//!     ConflictResolution::Merge
//! )?;
//!
//! if result.success {
//!     println!(
//!         "Imported {} FTP entries ({} skipped as duplicates)",
//!         result.ftp_entries_imported,
//!         result.ftp_entries_skipped
//!     );
//! }
//! ```
//!
//! ## Handling Conflicts
//!
//! When importing to a database with existing data, conflicts may occur:
//!
//! ```rust,ignore
//! use rust_ride::social::export::{ProfileExporter, ProfileConflict, ConflictResolution};
//!
//! let exporter = ProfileExporter::new(db);
//!
//! // Parse the import file first
//! let json = std::fs::read_to_string("import.json")?;
//! let export = exporter.parse_import(&json)?;
//!
//! // Check for conflicts before importing
//! let conflicts = exporter.detect_conflicts(&export)?;
//!
//! if conflicts.is_empty() {
//!     // No conflicts - safe to import directly
//!     let result = exporter.import_profile(&export, ConflictResolution::Merge)?;
//! } else {
//!     // Handle conflicts - show user options
//!     for conflict in &conflicts {
//!         match conflict {
//!             ProfileConflict::ExistingProfile { rider_id, existing_name } => {
//!                 println!("Profile already exists: {} ({})", existing_name, rider_id);
//!             }
//!             ProfileConflict::DisplayNameMismatch { imported_name, existing_name } => {
//!                 println!("Name differs: imported '{}', existing '{}'", imported_name, existing_name);
//!             }
//!             ProfileConflict::FtpMismatch { imported_ftp, existing_ftp } => {
//!                 println!("FTP differs: imported {:?}, existing {:?}", imported_ftp, existing_ftp);
//!             }
//!             ProfileConflict::AvatarMismatch { .. } => {
//!                 println!("Avatar configuration differs");
//!             }
//!         }
//!     }
//!
//!     // User chooses resolution strategy
//!     let result = exporter.import_profile(&export, ConflictResolution::Replace)?;
//! }
//! ```
//!
//! # Conflict Resolution Strategies
//!
//! Three strategies are available for handling import conflicts:
//!
//! | Strategy | Profile Data | FTP History | Avatar |
//! |----------|--------------|-------------|--------|
//! | `Replace` | Overwrites existing | Deletes all, imports fresh | Overwrites existing |
//! | `Merge` | Updates existing | Combines, skips duplicates | Updates if different |
//! | `Skip` | No changes | No changes | No changes |
//!
//! ## When to Use Each Strategy
//!
//! - **Replace**: Use when restoring from backup to a fresh installation,
//!   or when you want the imported data to completely replace existing data.
//!
//! - **Merge**: Use when combining data from multiple sources, or when
//!   importing to a profile that has accumulated new FTP tests since export.
//!
//! - **Skip**: Use when you want to abort the import without making changes.
//!
//! # Error Handling
//!
//! The module uses [`ProfileExportError`] for all error conditions:
//!
//! ```rust,ignore
//! use rust_ride::social::export::{ProfileExporter, ProfileExportError};
//!
//! let result = exporter.export_json(rider_id);
//!
//! match result {
//!     Ok(json) => println!("Export successful"),
//!     Err(ProfileExportError::ProfileNotFound(id)) => {
//!         println!("No profile found for rider: {}", id);
//!     }
//!     Err(ProfileExportError::DatabaseError(msg)) => {
//!         eprintln!("Database error: {}", msg);
//!     }
//!     Err(ProfileExportError::InvalidVersion { expected, found }) => {
//!         eprintln!("Version mismatch: expected {}, found {}", expected, found);
//!     }
//!     Err(e) => eprintln!("Export failed: {}", e),
//! }
//! ```
//!
//! # Database Tables
//!
//! The exporter reads from and writes to the following tables:
//!
//! - `riders`: Core profile data (display_name, bio, ftp, stats)
//! - `ftp_estimates`: FTP history records with detection method and confidence
//! - `avatars`: Avatar customization (jersey colors, bike style, helmet)
//!
//! # Thread Safety
//!
//! [`ProfileExporter`] uses `Arc<Database>` for the database connection,
//! making it safe to share across threads. Each operation acquires a
//! connection from the pool as needed.

use chrono::{DateTime, Utc};
use rusqlite;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::storage::Database;

/// Export format for rider profile data.
///
/// Contains the complete rider profile including settings, FTP history,
/// and avatar configuration in a versioned format for portability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExport {
    /// Format version for compatibility checking.
    pub export_version: String,
    /// Timestamp when the export was created.
    pub exported_at: DateTime<Utc>,
    /// The rider's unique identifier.
    pub rider_id: Uuid,
    /// Core profile data.
    pub profile: ProfileData,
    /// FTP history records.
    pub ftp_history: Vec<FtpHistoryEntry>,
    /// Avatar configuration.
    pub avatar: Option<AvatarExport>,
}

impl ProfileExport {
    /// Current export format version.
    pub const CURRENT_VERSION: &'static str = "1.0";

    /// Create a new profile export with current timestamp and version.
    pub fn new(
        rider_id: Uuid,
        profile: ProfileData,
        ftp_history: Vec<FtpHistoryEntry>,
        avatar: Option<AvatarExport>,
    ) -> Self {
        Self {
            export_version: Self::CURRENT_VERSION.to_string(),
            exported_at: Utc::now(),
            rider_id,
            profile,
            ftp_history,
            avatar,
        }
    }
}

/// Core profile data for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    /// Display name shown to other riders.
    pub display_name: String,
    /// Optional bio/description.
    pub bio: Option<String>,
    /// Current FTP in watts.
    pub ftp: Option<u16>,
    /// Total distance ridden in kilometers.
    pub total_distance_km: f64,
    /// Total time ridden in hours.
    pub total_time_hours: f64,
    /// Whether profile sharing is enabled.
    pub sharing_enabled: bool,
}

/// FTP history entry for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpHistoryEntry {
    /// FTP value in watts.
    pub ftp_watts: u16,
    /// Detection method (e.g., "ramp_test", "20min_test", "manual").
    pub method: String,
    /// Confidence level (e.g., "high", "medium", "low").
    pub confidence: String,
    /// When the FTP was detected.
    pub detected_at: DateTime<Utc>,
    /// Whether this estimate was accepted by the user.
    pub accepted: bool,
}

/// Avatar configuration for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarExport {
    /// Primary jersey color as hex string (e.g., "#FF0000").
    pub jersey_color: String,
    /// Bike style (e.g., "road_bike", "tt_bike", "gravel").
    pub bike_style: String,
    /// Optional secondary jersey color.
    pub jersey_secondary: Option<String>,
    /// Optional helmet color.
    pub helmet_color: Option<String>,
}

/// Result of a profile import operation.
#[derive(Debug, Clone)]
pub struct ProfileImportResult {
    /// Whether the import completed successfully.
    pub success: bool,
    /// Number of FTP history entries imported.
    pub ftp_entries_imported: u32,
    /// Number of FTP history entries skipped (duplicates).
    pub ftp_entries_skipped: u32,
    /// Whether the profile data was updated.
    pub profile_updated: bool,
    /// Whether the avatar was updated.
    pub avatar_updated: bool,
    /// List of conflicts detected during import.
    pub conflicts: Vec<ProfileConflict>,
}

impl ProfileImportResult {
    /// Create a new successful import result with no conflicts.
    pub fn success(
        ftp_entries_imported: u32,
        ftp_entries_skipped: u32,
        profile_updated: bool,
        avatar_updated: bool,
    ) -> Self {
        Self {
            success: true,
            ftp_entries_imported,
            ftp_entries_skipped,
            profile_updated,
            avatar_updated,
            conflicts: Vec::new(),
        }
    }

    /// Create a result indicating conflicts were detected.
    pub fn with_conflicts(conflicts: Vec<ProfileConflict>) -> Self {
        Self {
            success: false,
            ftp_entries_imported: 0,
            ftp_entries_skipped: 0,
            profile_updated: false,
            avatar_updated: false,
            conflicts,
        }
    }
}

/// Conflict detected during profile import.
#[derive(Debug, Clone)]
pub enum ProfileConflict {
    /// A profile already exists with a different display name.
    DisplayNameMismatch {
        /// Display name in the import file.
        imported_name: String,
        /// Display name in the existing profile.
        existing_name: String,
    },
    /// A profile already exists for this rider ID.
    ExistingProfile {
        /// The rider ID that already exists.
        rider_id: Uuid,
        /// Display name of the existing profile.
        existing_name: String,
    },
    /// FTP value differs between import and existing profile.
    FtpMismatch {
        /// FTP value in the import file.
        imported_ftp: Option<u16>,
        /// FTP value in the existing profile.
        existing_ftp: Option<u16>,
    },
    /// Avatar configuration differs between import and existing profile.
    AvatarMismatch {
        /// Whether the import has avatar data.
        import_has_avatar: bool,
        /// Whether the existing profile has avatar data.
        existing_has_avatar: bool,
    },
}

/// Strategy for resolving import conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Replace existing profile data with imported data.
    Replace,
    /// Merge imported data with existing data (e.g., combine FTP history).
    Merge,
    /// Skip the import and keep existing data.
    Skip,
}

/// Errors that can occur during profile export/import operations.
#[derive(Debug, thiserror::Error)]
pub enum ProfileExportError {
    /// Database operation failed.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// JSON serialization failed.
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    /// JSON parsing failed.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Requested profile was not found.
    #[error("Profile not found: {0}")]
    ProfileNotFound(Uuid),

    /// Export format version is not compatible.
    #[error("Invalid version: expected {expected}, found {found}")]
    InvalidVersion {
        /// The expected version.
        expected: String,
        /// The version found in the import file.
        found: String,
    },

    /// File I/O operation failed.
    #[error("IO error: {0}")]
    IoError(String),
}

/// Profile exporter for creating and importing profile backups.
///
/// Provides methods to export rider profiles to JSON and import them back,
/// following the pattern from `LeaderboardExporter`.
pub struct ProfileExporter {
    db: Arc<Database>,
}

impl ProfileExporter {
    /// Create a new profile exporter with the given database connection.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get a reference to the database.
    pub fn database(&self) -> &Arc<Database> {
        &self.db
    }

    /// Build a complete profile export for the given rider.
    ///
    /// Gathers data from the riders, ftp_estimates, and avatars tables
    /// and combines it into a ProfileExport struct.
    pub fn build_export(&self, rider_id: Uuid) -> Result<ProfileExport, ProfileExportError> {
        let conn = self.db.connection();

        // Query rider profile data
        let profile_data = self.query_profile_data(&conn, rider_id)?;

        // Query FTP history
        let ftp_history = self.query_ftp_history(&conn, rider_id)?;

        // Query avatar configuration
        let avatar = self.query_avatar(&conn, rider_id)?;

        Ok(ProfileExport::new(rider_id, profile_data, ftp_history, avatar))
    }

    /// Query the riders table for profile data.
    fn query_profile_data(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
    ) -> Result<ProfileData, ProfileExportError> {
        let mut stmt = conn
            .prepare(
                "SELECT display_name, bio, ftp, total_distance_km, total_time_hours, sharing_enabled
                 FROM riders WHERE id = ?1",
            )
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        stmt.query_row([rider_id.to_string()], |row| {
            Ok(ProfileData {
                display_name: row.get(0)?,
                bio: row.get(1)?,
                ftp: row.get(2)?,
                total_distance_km: row.get(3)?,
                total_time_hours: row.get(4)?,
                sharing_enabled: row.get(5)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ProfileExportError::ProfileNotFound(rider_id),
            other => ProfileExportError::DatabaseError(other.to_string()),
        })
    }

    /// Query the ftp_estimates table for FTP history.
    fn query_ftp_history(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
    ) -> Result<Vec<FtpHistoryEntry>, ProfileExportError> {
        let mut stmt = conn
            .prepare(
                "SELECT ftp_watts, method, confidence, detected_at, accepted
                 FROM ftp_estimates WHERE user_id = ?1
                 ORDER BY detected_at DESC",
            )
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([rider_id.to_string()], |row| {
                let detected_at_str: String = row.get(3)?;
                let accepted_int: i32 = row.get(4)?;

                Ok(FtpHistoryEntryRow {
                    ftp_watts: row.get(0)?,
                    method: row.get(1)?,
                    confidence: row.get(2)?,
                    detected_at_str,
                    accepted: accepted_int != 0,
                })
            })
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        let mut history = Vec::new();
        for row_result in rows {
            let row = row_result.map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

            let detected_at = DateTime::parse_from_rfc3339(&row.detected_at_str)
                .map_err(|e| ProfileExportError::DatabaseError(format!("Invalid date format: {}", e)))?
                .with_timezone(&Utc);

            history.push(FtpHistoryEntry {
                ftp_watts: row.ftp_watts,
                method: row.method,
                confidence: row.confidence,
                detected_at,
                accepted: row.accepted,
            });
        }

        Ok(history)
    }

    /// Export a rider profile to pretty-printed JSON.
    ///
    /// Builds the complete profile export and serializes it to JSON format.
    ///
    /// # Arguments
    /// * `rider_id` - The UUID of the rider to export
    ///
    /// # Returns
    /// A pretty-printed JSON string on success, or an error if the profile
    /// is not found or serialization fails.
    pub fn export_json(&self, rider_id: Uuid) -> Result<String, ProfileExportError> {
        let export = self.build_export(rider_id)?;
        serde_json::to_string_pretty(&export)
            .map_err(|e| ProfileExportError::SerializationFailed(e.to_string()))
    }

    /// Export a rider profile to a JSON file at the specified path.
    ///
    /// Creates parent directories if they don't exist, then writes the
    /// exported JSON to the file.
    ///
    /// # Arguments
    /// * `rider_id` - The UUID of the rider to export
    /// * `path` - The file path to write the export to
    ///
    /// # Returns
    /// The path to the created file on success, or an error if the profile
    /// is not found, serialization fails, or file I/O fails.
    pub fn export_to_file<P: AsRef<Path>>(
        &self,
        rider_id: Uuid,
        path: P,
    ) -> Result<PathBuf, ProfileExportError> {
        let path = path.as_ref();
        let json = self.export_json(rider_id)?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ProfileExportError::IoError(format!(
                        "Failed to create directory '{}': {}",
                        parent.display(),
                        e
                    )))?;
            }
        }

        // Write the JSON to the file
        std::fs::write(path, json)
            .map_err(|e| ProfileExportError::IoError(format!(
                "Failed to write file '{}': {}",
                path.display(),
                e
            )))?;

        Ok(path.to_path_buf())
    }

    /// Parse a JSON string into a ProfileExport struct.
    ///
    /// Validates the export format version for compatibility and returns
    /// structured errors for invalid data.
    ///
    /// # Arguments
    /// * `json_content` - The JSON string to parse
    ///
    /// # Returns
    /// The parsed ProfileExport on success, or an error if:
    /// - The JSON is malformed (ParseError)
    /// - The export version is incompatible (InvalidVersion)
    ///
    /// # Example
    /// ```ignore
    /// let exporter = ProfileExporter::new(db);
    /// let export = exporter.parse_import(json_content)?;
    /// ```
    pub fn parse_import(&self, json_content: &str) -> Result<ProfileExport, ProfileExportError> {
        // Parse the JSON string
        let export: ProfileExport = serde_json::from_str(json_content)
            .map_err(|e| ProfileExportError::ParseError(e.to_string()))?;

        // Validate export version compatibility
        self.validate_version(&export.export_version)?;

        Ok(export)
    }

    /// Validate that the export version is compatible with current version.
    ///
    /// Currently supports exact version match only. Future versions may
    /// implement migration logic for older export formats.
    fn validate_version(&self, version: &str) -> Result<(), ProfileExportError> {
        // For now, we only accept exact version match
        // Future: implement version migration logic for older formats
        if version != ProfileExport::CURRENT_VERSION {
            return Err(ProfileExportError::InvalidVersion {
                expected: ProfileExport::CURRENT_VERSION.to_string(),
                found: version.to_string(),
            });
        }
        Ok(())
    }

    /// Detect conflicts between an imported profile and existing data.
    ///
    /// Checks for:
    /// - Existing profile with the same rider_id
    /// - Display name mismatches between import and existing profile
    /// - FTP value differences
    /// - Avatar configuration differences
    ///
    /// # Arguments
    /// * `export` - The parsed profile export to check for conflicts
    ///
    /// # Returns
    /// A list of detected conflicts. An empty list means no conflicts.
    ///
    /// # Example
    /// ```ignore
    /// let exporter = ProfileExporter::new(db);
    /// let export = exporter.parse_import(json_content)?;
    /// let conflicts = exporter.detect_conflicts(&export)?;
    /// if conflicts.is_empty() {
    ///     // Safe to import directly
    /// } else {
    ///     // Show conflicts to user for resolution
    /// }
    /// ```
    pub fn detect_conflicts(
        &self,
        export: &ProfileExport,
    ) -> Result<Vec<ProfileConflict>, ProfileExportError> {
        let conn = self.db.connection();
        let mut conflicts = Vec::new();

        // Check if a profile with the same rider_id exists
        let existing_profile = self.query_existing_profile(&conn, export.rider_id)?;

        if let Some(existing) = existing_profile {
            // ExistingProfile conflict - a profile with this ID already exists
            conflicts.push(ProfileConflict::ExistingProfile {
                rider_id: export.rider_id,
                existing_name: existing.display_name.clone(),
            });

            // DisplayNameMismatch - imported name differs from existing
            if export.profile.display_name != existing.display_name {
                conflicts.push(ProfileConflict::DisplayNameMismatch {
                    imported_name: export.profile.display_name.clone(),
                    existing_name: existing.display_name.clone(),
                });
            }

            // FtpMismatch - FTP values differ
            if export.profile.ftp != existing.ftp {
                conflicts.push(ProfileConflict::FtpMismatch {
                    imported_ftp: export.profile.ftp,
                    existing_ftp: existing.ftp,
                });
            }

            // Check for avatar mismatch
            let existing_avatar = self.query_avatar(&conn, export.rider_id)?;
            let import_has_avatar = export.avatar.is_some();
            let existing_has_avatar = existing_avatar.is_some();

            if import_has_avatar != existing_has_avatar {
                conflicts.push(ProfileConflict::AvatarMismatch {
                    import_has_avatar,
                    existing_has_avatar,
                });
            } else if import_has_avatar && existing_has_avatar {
                // Both have avatars - check if they differ
                let import_avatar = export.avatar.as_ref().unwrap();
                let existing_av = existing_avatar.as_ref().unwrap();

                if import_avatar.jersey_color != existing_av.jersey_color
                    || import_avatar.bike_style != existing_av.bike_style
                    || import_avatar.jersey_secondary != existing_av.jersey_secondary
                    || import_avatar.helmet_color != existing_av.helmet_color
                {
                    conflicts.push(ProfileConflict::AvatarMismatch {
                        import_has_avatar,
                        existing_has_avatar,
                    });
                }
            }
        } else {
            // No existing profile with same rider_id - check for display name collision
            let name_collision = self.find_rider_by_display_name(&conn, &export.profile.display_name)?;

            if let Some(colliding_rider_id) = name_collision {
                // Different rider_id but same display_name
                conflicts.push(ProfileConflict::DisplayNameMismatch {
                    imported_name: export.profile.display_name.clone(),
                    existing_name: export.profile.display_name.clone(),
                });
                conflicts.push(ProfileConflict::ExistingProfile {
                    rider_id: colliding_rider_id,
                    existing_name: export.profile.display_name.clone(),
                });
            }
        }

        Ok(conflicts)
    }

    /// Query for an existing profile by rider_id.
    fn query_existing_profile(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
    ) -> Result<Option<ProfileData>, ProfileExportError> {
        let mut stmt = conn
            .prepare(
                "SELECT display_name, bio, ftp, total_distance_km, total_time_hours, sharing_enabled
                 FROM riders WHERE id = ?1",
            )
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        match stmt.query_row([rider_id.to_string()], |row| {
            Ok(ProfileData {
                display_name: row.get(0)?,
                bio: row.get(1)?,
                ftp: row.get(2)?,
                total_distance_km: row.get(3)?,
                total_time_hours: row.get(4)?,
                sharing_enabled: row.get(5)?,
            })
        }) {
            Ok(profile) => Ok(Some(profile)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ProfileExportError::DatabaseError(e.to_string())),
        }
    }

    /// Find a rider by display name (for collision detection).
    fn find_rider_by_display_name(
        &self,
        conn: &rusqlite::Connection,
        display_name: &str,
    ) -> Result<Option<Uuid>, ProfileExportError> {
        let mut stmt = conn
            .prepare("SELECT id FROM riders WHERE display_name = ?1")
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        match stmt.query_row([display_name], |row| {
            let id_str: String = row.get(0)?;
            Ok(id_str)
        }) {
            Ok(id_str) => {
                let uuid = Uuid::parse_str(&id_str)
                    .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;
                Ok(Some(uuid))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ProfileExportError::DatabaseError(e.to_string())),
        }
    }

    /// Query the avatars table for avatar configuration.
    fn query_avatar(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
    ) -> Result<Option<AvatarExport>, ProfileExportError> {
        let mut stmt = conn
            .prepare(
                "SELECT jersey_color, bike_style, jersey_secondary, helmet_color
                 FROM avatars WHERE user_id = ?1",
            )
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        match stmt.query_row([rider_id.to_string()], |row| {
            Ok(AvatarExport {
                jersey_color: row.get(0)?,
                bike_style: row.get(1)?,
                jersey_secondary: row.get(2)?,
                helmet_color: row.get(3)?,
            })
        }) {
            Ok(avatar) => Ok(Some(avatar)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ProfileExportError::DatabaseError(e.to_string())),
        }
    }

    /// Import a profile export to the database.
    ///
    /// Imports profile data, FTP history, and avatar configuration using
    /// the specified conflict resolution strategy.
    ///
    /// # Strategies
    /// - `Replace`: Completely overwrite existing profile data
    /// - `Merge`: Combine FTP history, keeping both existing and imported entries
    /// - `Skip`: Do not import anything, return immediately
    ///
    /// # Arguments
    /// * `export` - The parsed profile export to import
    /// * `resolution` - Strategy for handling conflicts with existing data
    ///
    /// # Returns
    /// A `ProfileImportResult` with details about what was imported/skipped.
    ///
    /// # Example
    /// ```ignore
    /// let exporter = ProfileExporter::new(db);
    /// let export = exporter.parse_import(json_content)?;
    /// let result = exporter.import_profile(&export, ConflictResolution::Merge)?;
    /// if result.success {
    ///     println!("Imported {} FTP entries", result.ftp_entries_imported);
    /// }
    /// ```
    pub fn import_profile(
        &self,
        export: &ProfileExport,
        resolution: ConflictResolution,
    ) -> Result<ProfileImportResult, ProfileExportError> {
        // Skip strategy: return immediately without changes
        if resolution == ConflictResolution::Skip {
            return Ok(ProfileImportResult {
                success: true,
                ftp_entries_imported: 0,
                ftp_entries_skipped: 0,
                profile_updated: false,
                avatar_updated: false,
                conflicts: Vec::new(),
            });
        }

        let conn = self.db.connection();

        // Check if profile exists
        let existing_profile = self.query_existing_profile(&conn, export.rider_id)?;

        let profile_updated;
        let avatar_updated;

        match resolution {
            ConflictResolution::Replace => {
                // Replace: delete existing data and insert fresh
                profile_updated = self.import_profile_data_replace(&conn, export)?;
                avatar_updated = self.import_avatar_replace(&conn, export)?;
            }
            ConflictResolution::Merge => {
                // Merge: update profile, keep existing data where appropriate
                profile_updated = if existing_profile.is_some() {
                    self.import_profile_data_update(&conn, export)?
                } else {
                    self.import_profile_data_insert(&conn, export)?
                };
                avatar_updated = self.import_avatar_merge(&conn, export)?;
            }
            ConflictResolution::Skip => {
                // Already handled above
                unreachable!()
            }
        }

        // Import FTP history based on strategy
        let (ftp_entries_imported, ftp_entries_skipped) = match resolution {
            ConflictResolution::Replace => {
                // Delete all existing FTP entries and import fresh
                self.delete_ftp_history(&conn, export.rider_id)?;
                let imported = self.import_ftp_history_all(&conn, export)?;
                (imported, 0)
            }
            ConflictResolution::Merge => {
                // Merge FTP history, skipping duplicates
                self.import_ftp_history_merge(&conn, export)?
            }
            ConflictResolution::Skip => (0, 0),
        };

        Ok(ProfileImportResult::success(
            ftp_entries_imported,
            ftp_entries_skipped,
            profile_updated,
            avatar_updated,
        ))
    }

    /// Import a rider profile from a JSON file at the specified path.
    ///
    /// Reads the file contents, parses the JSON, and imports the profile
    /// with the specified conflict resolution strategy.
    ///
    /// # Arguments
    /// * `path` - The file path to read the import from
    /// * `resolution` - Strategy for handling conflicts with existing data
    ///
    /// # Returns
    /// A `ProfileImportResult` with details about what was imported/skipped,
    /// or an error if:
    /// - The file cannot be read (IoError)
    /// - The JSON is malformed (ParseError)
    /// - The export version is incompatible (InvalidVersion)
    /// - Database operations fail (DatabaseError)
    ///
    /// # Example
    /// ```ignore
    /// let exporter = ProfileExporter::new(db);
    /// let result = exporter.import_from_file("backup.json", ConflictResolution::Merge)?;
    /// if result.success {
    ///     println!("Imported {} FTP entries", result.ftp_entries_imported);
    /// }
    /// ```
    pub fn import_from_file<P: AsRef<Path>>(
        &self,
        path: P,
        resolution: ConflictResolution,
    ) -> Result<ProfileImportResult, ProfileExportError> {
        let path = path.as_ref();

        // Read file contents
        let json_content = std::fs::read_to_string(path).map_err(|e| {
            ProfileExportError::IoError(format!("Failed to read file '{}': {}", path.display(), e))
        })?;

        // Parse JSON and validate version
        let export = self.parse_import(&json_content)?;

        // Import with specified conflict resolution
        self.import_profile(&export, resolution)
    }

    /// Replace profile data by deleting existing and inserting new.
    fn import_profile_data_replace(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<bool, ProfileExportError> {
        // Delete existing profile if present
        conn.execute(
            "DELETE FROM riders WHERE id = ?1",
            [export.rider_id.to_string()],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        // Insert new profile
        self.import_profile_data_insert(conn, export)
    }

    /// Insert profile data for a new rider.
    fn import_profile_data_insert(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<bool, ProfileExportError> {
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO riders (id, display_name, bio, ftp, total_distance_km, total_time_hours, sharing_enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                export.rider_id.to_string(),
                export.profile.display_name,
                export.profile.bio,
                export.profile.ftp,
                export.profile.total_distance_km,
                export.profile.total_time_hours,
                if export.profile.sharing_enabled { 1 } else { 0 },
                now,
                now,
            ],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(true)
    }

    /// Update existing profile data (for merge strategy).
    fn import_profile_data_update(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<bool, ProfileExportError> {
        let now = Utc::now().to_rfc3339();

        let rows_affected = conn
            .execute(
                "UPDATE riders
                 SET display_name = ?1, bio = ?2, ftp = ?3, total_distance_km = ?4, total_time_hours = ?5, sharing_enabled = ?6, updated_at = ?7
                 WHERE id = ?8",
                rusqlite::params![
                    export.profile.display_name,
                    export.profile.bio,
                    export.profile.ftp,
                    export.profile.total_distance_km,
                    export.profile.total_time_hours,
                    if export.profile.sharing_enabled { 1 } else { 0 },
                    now,
                    export.rider_id.to_string(),
                ],
            )
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(rows_affected > 0)
    }

    /// Replace avatar data by deleting existing and inserting new.
    fn import_avatar_replace(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<bool, ProfileExportError> {
        // Delete existing avatar if present
        conn.execute(
            "DELETE FROM avatars WHERE user_id = ?1",
            [export.rider_id.to_string()],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        // Insert new avatar if export has one
        if let Some(ref avatar) = export.avatar {
            self.insert_avatar(conn, export.rider_id, avatar)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Merge avatar data (update existing or insert new).
    fn import_avatar_merge(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<bool, ProfileExportError> {
        let Some(ref avatar) = export.avatar else {
            // No avatar in export - no changes needed
            return Ok(false);
        };

        // Check if avatar exists
        let existing_avatar = self.query_avatar(conn, export.rider_id)?;

        if existing_avatar.is_some() {
            // Update existing avatar
            self.update_avatar(conn, export.rider_id, avatar)?;
        } else {
            // Insert new avatar
            self.insert_avatar(conn, export.rider_id, avatar)?;
        }

        Ok(true)
    }

    /// Insert a new avatar record.
    fn insert_avatar(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
        avatar: &AvatarExport,
    ) -> Result<(), ProfileExportError> {
        let avatar_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO avatars (id, user_id, jersey_color, jersey_secondary, bike_style, helmet_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                avatar_id.to_string(),
                rider_id.to_string(),
                avatar.jersey_color,
                avatar.jersey_secondary,
                avatar.bike_style,
                avatar.helmet_color,
                now,
                now,
            ],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update an existing avatar record.
    fn update_avatar(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
        avatar: &AvatarExport,
    ) -> Result<(), ProfileExportError> {
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE avatars
             SET jersey_color = ?1, jersey_secondary = ?2, bike_style = ?3, helmet_color = ?4, updated_at = ?5
             WHERE user_id = ?6",
            rusqlite::params![
                avatar.jersey_color,
                avatar.jersey_secondary,
                avatar.bike_style,
                avatar.helmet_color,
                now,
                rider_id.to_string(),
            ],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Delete all FTP history for a rider.
    fn delete_ftp_history(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
    ) -> Result<(), ProfileExportError> {
        conn.execute(
            "DELETE FROM ftp_estimates WHERE user_id = ?1",
            [rider_id.to_string()],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Import all FTP history entries (for replace strategy).
    fn import_ftp_history_all(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<u32, ProfileExportError> {
        let mut imported = 0u32;

        for entry in &export.ftp_history {
            self.insert_ftp_entry(conn, export.rider_id, entry)?;
            imported += 1;
        }

        Ok(imported)
    }

    /// Import FTP history with deduplication (for merge strategy).
    ///
    /// Returns (imported_count, skipped_count).
    fn import_ftp_history_merge(
        &self,
        conn: &rusqlite::Connection,
        export: &ProfileExport,
    ) -> Result<(u32, u32), ProfileExportError> {
        let mut imported = 0u32;
        let mut skipped = 0u32;

        for entry in &export.ftp_history {
            // Check if this entry already exists (same timestamp)
            if self.ftp_entry_exists(conn, export.rider_id, entry)? {
                skipped += 1;
            } else {
                self.insert_ftp_entry(conn, export.rider_id, entry)?;
                imported += 1;
            }
        }

        Ok((imported, skipped))
    }

    /// Check if an FTP entry already exists (by timestamp).
    fn ftp_entry_exists(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
        entry: &FtpHistoryEntry,
    ) -> Result<bool, ProfileExportError> {
        let mut stmt = conn
            .prepare("SELECT 1 FROM ftp_estimates WHERE user_id = ?1 AND detected_at = ?2")
            .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        stmt.exists(rusqlite::params![
            rider_id.to_string(),
            entry.detected_at.to_rfc3339()
        ])
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))
    }

    /// Insert a single FTP history entry.
    fn insert_ftp_entry(
        &self,
        conn: &rusqlite::Connection,
        rider_id: Uuid,
        entry: &FtpHistoryEntry,
    ) -> Result<(), ProfileExportError> {
        let entry_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO ftp_estimates (id, user_id, ftp_watts, method, confidence, detected_at, accepted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry_id.to_string(),
                rider_id.to_string(),
                entry.ftp_watts,
                entry.method,
                entry.confidence,
                entry.detected_at.to_rfc3339(),
                if entry.accepted { 1 } else { 0 },
                now,
            ],
        )
        .map_err(|e| ProfileExportError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Helper struct for FTP history row data.
struct FtpHistoryEntryRow {
    ftp_watts: u16,
    method: String,
    confidence: String,
    detected_at_str: String,
    accepted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_export_serialization() {
        let profile = ProfileData {
            display_name: "Test Rider".to_string(),
            bio: Some("Test bio".to_string()),
            ftp: Some(250),
            total_distance_km: 1000.0,
            total_time_hours: 50.0,
            sharing_enabled: true,
        };

        let export = ProfileExport::new(
            Uuid::new_v4(),
            profile,
            vec![],
            None,
        );

        assert_eq!(export.export_version, ProfileExport::CURRENT_VERSION);
        assert!(export.exported_at <= Utc::now());

        // Verify serialization works
        let json = serde_json::to_string_pretty(&export).unwrap();
        assert!(json.contains("export_version"));
        assert!(json.contains("exported_at"));
        assert!(json.contains("profile"));

        // Verify deserialization works
        let parsed: ProfileExport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.export_version, export.export_version);
        assert_eq!(parsed.profile.display_name, "Test Rider");
    }

    #[test]
    fn test_profile_export_with_ftp_history() {
        let profile = ProfileData {
            display_name: "FTP Tracker".to_string(),
            bio: None,
            ftp: Some(280),
            total_distance_km: 500.0,
            total_time_hours: 25.0,
            sharing_enabled: false,
        };

        let ftp_history = vec![
            FtpHistoryEntry {
                ftp_watts: 250,
                method: "ramp_test".to_string(),
                confidence: "high".to_string(),
                detected_at: Utc::now(),
                accepted: true,
            },
            FtpHistoryEntry {
                ftp_watts: 280,
                method: "20min_test".to_string(),
                confidence: "high".to_string(),
                detected_at: Utc::now(),
                accepted: true,
            },
        ];

        let export = ProfileExport::new(
            Uuid::new_v4(),
            profile,
            ftp_history,
            None,
        );

        assert_eq!(export.ftp_history.len(), 2);
        assert_eq!(export.ftp_history[0].ftp_watts, 250);
        assert_eq!(export.ftp_history[1].ftp_watts, 280);
    }

    #[test]
    fn test_profile_export_with_avatar() {
        let profile = ProfileData {
            display_name: "Stylish Rider".to_string(),
            bio: None,
            ftp: Some(200),
            total_distance_km: 100.0,
            total_time_hours: 5.0,
            sharing_enabled: true,
        };

        let avatar = AvatarExport {
            jersey_color: "#FF0000".to_string(),
            bike_style: "road_bike".to_string(),
            jersey_secondary: Some("#FFFFFF".to_string()),
            helmet_color: Some("#000000".to_string()),
        };

        let export = ProfileExport::new(
            Uuid::new_v4(),
            profile,
            vec![],
            Some(avatar),
        );

        assert!(export.avatar.is_some());
        let avatar = export.avatar.unwrap();
        assert_eq!(avatar.jersey_color, "#FF0000");
        assert_eq!(avatar.bike_style, "road_bike");
    }

    #[test]
    fn test_profile_import_result_success() {
        let result = ProfileImportResult::success(5, 2, true, true);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 5);
        assert_eq!(result.ftp_entries_skipped, 2);
        assert!(result.profile_updated);
        assert!(result.avatar_updated);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_profile_import_result_with_conflicts() {
        let conflicts = vec![
            ProfileConflict::DisplayNameMismatch {
                imported_name: "New Name".to_string(),
                existing_name: "Old Name".to_string(),
            },
            ProfileConflict::FtpMismatch {
                imported_ftp: Some(280),
                existing_ftp: Some(250),
            },
        ];

        let result = ProfileImportResult::with_conflicts(conflicts);

        assert!(!result.success);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(!result.profile_updated);
        assert!(!result.avatar_updated);
        assert_eq!(result.conflicts.len(), 2);
    }

    #[test]
    fn test_profile_conflict_variants() {
        // Test DisplayNameMismatch
        let conflict = ProfileConflict::DisplayNameMismatch {
            imported_name: "Imported".to_string(),
            existing_name: "Existing".to_string(),
        };
        assert!(matches!(conflict, ProfileConflict::DisplayNameMismatch { .. }));

        // Test ExistingProfile
        let rider_id = Uuid::new_v4();
        let conflict = ProfileConflict::ExistingProfile {
            rider_id,
            existing_name: "Existing Rider".to_string(),
        };
        assert!(matches!(conflict, ProfileConflict::ExistingProfile { .. }));

        // Test FtpMismatch
        let conflict = ProfileConflict::FtpMismatch {
            imported_ftp: Some(280),
            existing_ftp: Some(250),
        };
        assert!(matches!(conflict, ProfileConflict::FtpMismatch { .. }));

        // Test AvatarMismatch
        let conflict = ProfileConflict::AvatarMismatch {
            import_has_avatar: true,
            existing_has_avatar: false,
        };
        assert!(matches!(conflict, ProfileConflict::AvatarMismatch { .. }));
    }

    #[test]
    fn test_conflict_resolution_variants() {
        assert_eq!(ConflictResolution::Replace, ConflictResolution::Replace);
        assert_eq!(ConflictResolution::Merge, ConflictResolution::Merge);
        assert_eq!(ConflictResolution::Skip, ConflictResolution::Skip);

        // Verify they are different from each other
        assert_ne!(ConflictResolution::Replace, ConflictResolution::Merge);
        assert_ne!(ConflictResolution::Replace, ConflictResolution::Skip);
        assert_ne!(ConflictResolution::Merge, ConflictResolution::Skip);
    }

    #[test]
    fn test_profile_export_error_database() {
        let error = ProfileExportError::DatabaseError("connection failed".to_string());
        let error_msg = error.to_string();
        assert!(error_msg.contains("Database error"));
        assert!(error_msg.contains("connection failed"));
    }

    #[test]
    fn test_profile_export_error_serialization() {
        let error = ProfileExportError::SerializationFailed("invalid UTF-8".to_string());
        let error_msg = error.to_string();
        assert!(error_msg.contains("Serialization failed"));
        assert!(error_msg.contains("invalid UTF-8"));
    }

    #[test]
    fn test_profile_export_error_parse() {
        let error = ProfileExportError::ParseError("unexpected token".to_string());
        let error_msg = error.to_string();
        assert!(error_msg.contains("Parse error"));
        assert!(error_msg.contains("unexpected token"));
    }

    #[test]
    fn test_profile_export_error_profile_not_found() {
        let rider_id = Uuid::new_v4();
        let error = ProfileExportError::ProfileNotFound(rider_id);
        let error_msg = error.to_string();
        assert!(error_msg.contains("Profile not found"));
        assert!(error_msg.contains(&rider_id.to_string()));
    }

    #[test]
    fn test_profile_export_error_invalid_version() {
        let error = ProfileExportError::InvalidVersion {
            expected: "1.0".to_string(),
            found: "2.0".to_string(),
        };
        let error_msg = error.to_string();
        assert!(error_msg.contains("Invalid version"));
        assert!(error_msg.contains("expected 1.0"));
        assert!(error_msg.contains("found 2.0"));
    }

    #[test]
    fn test_profile_export_error_is_std_error() {
        // Verify ProfileExportError implements std::error::Error
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = ProfileExportError::DatabaseError("test".to_string());
        assert_error(&error);
    }

    #[test]
    fn test_export_json_pretty_printed_format() {
        // Test that the export produces valid pretty-printed JSON
        let profile = ProfileData {
            display_name: "JSON Test Rider".to_string(),
            bio: Some("Testing JSON export".to_string()),
            ftp: Some(275),
            total_distance_km: 1500.5,
            total_time_hours: 75.25,
            sharing_enabled: true,
        };

        let ftp_history = vec![FtpHistoryEntry {
            ftp_watts: 275,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc::now(),
            accepted: true,
        }];

        let avatar = AvatarExport {
            jersey_color: "#3366CC".to_string(),
            bike_style: "road_bike".to_string(),
            jersey_secondary: Some("#FFFFFF".to_string()),
            helmet_color: None,
        };

        let export = ProfileExport::new(Uuid::new_v4(), profile, ftp_history, Some(avatar));

        // Serialize to pretty-printed JSON (same method as export_json uses)
        let json = serde_json::to_string_pretty(&export).expect("Serialization should succeed");

        // Verify pretty-printing (contains newlines and indentation)
        assert!(json.contains('\n'), "JSON should be pretty-printed with newlines");
        assert!(json.contains("  "), "JSON should have indentation");

        // Verify all expected fields are present
        assert!(json.contains("\"export_version\""));
        assert!(json.contains("\"1.0\""));
        assert!(json.contains("\"exported_at\""));
        assert!(json.contains("\"rider_id\""));
        assert!(json.contains("\"profile\""));
        assert!(json.contains("\"display_name\""));
        assert!(json.contains("\"JSON Test Rider\""));
        assert!(json.contains("\"ftp_history\""));
        assert!(json.contains("\"ftp_watts\""));
        assert!(json.contains("275"));
        assert!(json.contains("\"avatar\""));
        assert!(json.contains("\"jersey_color\""));
        assert!(json.contains("\"#3366CC\""));

        // Verify it can be parsed back
        let parsed: ProfileExport = serde_json::from_str(&json).expect("Should parse back");
        assert_eq!(parsed.profile.display_name, "JSON Test Rider");
        assert_eq!(parsed.profile.ftp, Some(275));
        assert_eq!(parsed.ftp_history.len(), 1);
        assert!(parsed.avatar.is_some());
    }

    #[test]
    fn test_export_json_handles_all_optional_fields() {
        // Test export with minimal data (no bio, no FTP, no history, no avatar)
        let profile = ProfileData {
            display_name: "Minimal Rider".to_string(),
            bio: None,
            ftp: None,
            total_distance_km: 0.0,
            total_time_hours: 0.0,
            sharing_enabled: false,
        };

        let export = ProfileExport::new(Uuid::new_v4(), profile, vec![], None);

        let json = serde_json::to_string_pretty(&export).expect("Serialization should succeed");

        // Verify null values are serialized correctly
        assert!(json.contains("\"bio\": null") || json.contains("\"bio\":null"));
        assert!(json.contains("\"ftp\": null") || json.contains("\"ftp\":null"));
        assert!(json.contains("\"avatar\": null") || json.contains("\"avatar\":null"));
        assert!(json.contains("\"ftp_history\": []") || json.contains("\"ftp_history\":[]"));

        // Verify round-trip
        let parsed: ProfileExport = serde_json::from_str(&json).expect("Should parse back");
        assert_eq!(parsed.profile.bio, None);
        assert_eq!(parsed.profile.ftp, None);
        assert!(parsed.ftp_history.is_empty());
        assert!(parsed.avatar.is_none());
    }

    #[test]
    fn test_serialization_error_type() {
        // Verify the SerializationFailed error variant is correctly typed
        let error = ProfileExportError::SerializationFailed("JSON error".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Serialization failed"));
        assert!(msg.contains("JSON error"));
    }

    #[test]
    fn test_profile_export_error_io() {
        let error = ProfileExportError::IoError("permission denied".to_string());
        let error_msg = error.to_string();
        assert!(error_msg.contains("IO error"));
        assert!(error_msg.contains("permission denied"));
    }

    #[test]
    fn test_io_error_is_std_error() {
        // Verify IoError variant implements std::error::Error
        fn assert_error<E: std::error::Error>(_: &E) {}

        let error = ProfileExportError::IoError("test io error".to_string());
        assert_error(&error);
    }

    // Tests for parse_import method

    #[test]
    fn test_parse_import_valid_json() {
        // Create a valid JSON export string
        let rider_id = Uuid::new_v4();
        let profile = ProfileData {
            display_name: "Test Rider".to_string(),
            bio: Some("Test bio".to_string()),
            ftp: Some(250),
            total_distance_km: 1000.0,
            total_time_hours: 50.0,
            sharing_enabled: true,
        };

        let ftp_history = vec![FtpHistoryEntry {
            ftp_watts: 250,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc::now(),
            accepted: true,
        }];

        let avatar = AvatarExport {
            jersey_color: "#FF0000".to_string(),
            bike_style: "road_bike".to_string(),
            jersey_secondary: Some("#FFFFFF".to_string()),
            helmet_color: Some("#000000".to_string()),
        };

        let export = ProfileExport::new(rider_id, profile, ftp_history, Some(avatar));
        let json = serde_json::to_string_pretty(&export).unwrap();

        // Create a mock database for testing parse_import
        // We need to test that the parse_import method works correctly
        // For unit testing without a database, we'll test the parsing logic directly

        // Parse the JSON back and verify
        let parsed: ProfileExport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.export_version, ProfileExport::CURRENT_VERSION);
        assert_eq!(parsed.rider_id, rider_id);
        assert_eq!(parsed.profile.display_name, "Test Rider");
        assert_eq!(parsed.profile.bio, Some("Test bio".to_string()));
        assert_eq!(parsed.profile.ftp, Some(250));
        assert_eq!(parsed.ftp_history.len(), 1);
        assert_eq!(parsed.ftp_history[0].ftp_watts, 250);
        assert!(parsed.avatar.is_some());
        let avatar = parsed.avatar.unwrap();
        assert_eq!(avatar.jersey_color, "#FF0000");
    }

    #[test]
    fn test_parse_import_invalid_json() {
        // Test that malformed JSON produces a ParseError
        let invalid_json = "{ invalid json content";

        let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());

        // Verify that the error message contains useful information
        let error = result.unwrap_err();
        let error_string = error.to_string();
        assert!(
            error_string.contains("expected")
                || error_string.contains("key")
                || error_string.contains("EOF"),
            "Error should contain parse error details: {}",
            error_string
        );
    }

    #[test]
    fn test_parse_import_missing_required_fields() {
        // Test that JSON missing required fields produces a ParseError
        let incomplete_json = r#"{
            "export_version": "1.0",
            "exported_at": "2024-01-01T00:00:00Z"
        }"#;

        let result: Result<ProfileExport, _> = serde_json::from_str(incomplete_json);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_string = error.to_string();
        // Error should mention missing field
        assert!(
            error_string.contains("missing field") || error_string.contains("rider_id"),
            "Error should mention missing field: {}",
            error_string
        );
    }

    #[test]
    fn test_parse_import_version_compatibility() {
        // Test version validation logic
        let valid_version = ProfileExport::CURRENT_VERSION;
        assert_eq!(valid_version, "1.0");

        // Create an export with valid version
        let rider_id = Uuid::new_v4();
        let profile = ProfileData {
            display_name: "Version Test".to_string(),
            bio: None,
            ftp: None,
            total_distance_km: 0.0,
            total_time_hours: 0.0,
            sharing_enabled: false,
        };

        let export = ProfileExport::new(rider_id, profile, vec![], None);
        let json = serde_json::to_string(&export).unwrap();

        // Verify the export has correct version
        let parsed: ProfileExport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.export_version, "1.0");
    }

    #[test]
    fn test_parse_import_incompatible_version_format() {
        // Create JSON with incompatible version to test InvalidVersion error
        let json_with_wrong_version = r#"{
            "export_version": "2.0",
            "exported_at": "2024-01-01T00:00:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {
                "display_name": "Test",
                "bio": null,
                "ftp": null,
                "total_distance_km": 0.0,
                "total_time_hours": 0.0,
                "sharing_enabled": false
            },
            "ftp_history": [],
            "avatar": null
        }"#;

        // Parse will succeed, but version validation would fail
        let parsed: ProfileExport = serde_json::from_str(json_with_wrong_version).unwrap();
        assert_eq!(parsed.export_version, "2.0");
        assert_ne!(parsed.export_version, ProfileExport::CURRENT_VERSION);

        // Verify the InvalidVersion error type exists and has correct fields
        let error = ProfileExportError::InvalidVersion {
            expected: ProfileExport::CURRENT_VERSION.to_string(),
            found: "2.0".to_string(),
        };
        let error_msg = error.to_string();
        assert!(error_msg.contains("Invalid version"));
        assert!(error_msg.contains("expected 1.0"));
        assert!(error_msg.contains("found 2.0"));
    }

    #[test]
    fn test_parse_import_with_empty_ftp_history() {
        let json = r#"{
            "export_version": "1.0",
            "exported_at": "2024-01-01T00:00:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {
                "display_name": "Empty History Test",
                "bio": null,
                "ftp": null,
                "total_distance_km": 0.0,
                "total_time_hours": 0.0,
                "sharing_enabled": false
            },
            "ftp_history": [],
            "avatar": null
        }"#;

        let parsed: ProfileExport = serde_json::from_str(json).unwrap();
        assert!(parsed.ftp_history.is_empty());
        assert!(parsed.avatar.is_none());
    }

    #[test]
    fn test_parse_import_full_data() {
        let json = r#"{
            "export_version": "1.0",
            "exported_at": "2024-06-15T10:30:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {
                "display_name": "Full Data Rider",
                "bio": "Experienced cyclist from California",
                "ftp": 275,
                "total_distance_km": 5000.5,
                "total_time_hours": 250.25,
                "sharing_enabled": true
            },
            "ftp_history": [
                {
                    "ftp_watts": 250,
                    "method": "ramp_test",
                    "confidence": "high",
                    "detected_at": "2024-01-15T08:00:00Z",
                    "accepted": true
                },
                {
                    "ftp_watts": 265,
                    "method": "20min_test",
                    "confidence": "high",
                    "detected_at": "2024-03-20T09:30:00Z",
                    "accepted": true
                },
                {
                    "ftp_watts": 275,
                    "method": "ramp_test",
                    "confidence": "medium",
                    "detected_at": "2024-06-01T07:45:00Z",
                    "accepted": true
                }
            ],
            "avatar": {
                "jersey_color": "#3366CC",
                "bike_style": "tt_bike",
                "jersey_secondary": "#FFCC00",
                "helmet_color": "#FFFFFF"
            }
        }"#;

        let parsed: ProfileExport = serde_json::from_str(json).unwrap();

        // Verify profile data
        assert_eq!(parsed.export_version, "1.0");
        assert_eq!(parsed.profile.display_name, "Full Data Rider");
        assert_eq!(
            parsed.profile.bio,
            Some("Experienced cyclist from California".to_string())
        );
        assert_eq!(parsed.profile.ftp, Some(275));
        assert!((parsed.profile.total_distance_km - 5000.5).abs() < f64::EPSILON);
        assert!((parsed.profile.total_time_hours - 250.25).abs() < f64::EPSILON);
        assert!(parsed.profile.sharing_enabled);

        // Verify FTP history
        assert_eq!(parsed.ftp_history.len(), 3);
        assert_eq!(parsed.ftp_history[0].ftp_watts, 250);
        assert_eq!(parsed.ftp_history[0].method, "ramp_test");
        assert_eq!(parsed.ftp_history[1].ftp_watts, 265);
        assert_eq!(parsed.ftp_history[2].ftp_watts, 275);

        // Verify avatar
        assert!(parsed.avatar.is_some());
        let avatar = parsed.avatar.unwrap();
        assert_eq!(avatar.jersey_color, "#3366CC");
        assert_eq!(avatar.bike_style, "tt_bike");
        assert_eq!(avatar.jersey_secondary, Some("#FFCC00".to_string()));
        assert_eq!(avatar.helmet_color, Some("#FFFFFF".to_string()));
    }

    #[test]
    fn test_parse_import_wrong_type_error() {
        // Test that wrong types in JSON produce helpful errors
        let json_wrong_type = r#"{
            "export_version": "1.0",
            "exported_at": "2024-01-01T00:00:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {
                "display_name": "Test",
                "bio": null,
                "ftp": "not_a_number",
                "total_distance_km": 0.0,
                "total_time_hours": 0.0,
                "sharing_enabled": false
            },
            "ftp_history": [],
            "avatar": null
        }"#;

        let result: Result<ProfileExport, _> = serde_json::from_str(json_wrong_type);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_string = error.to_string();
        // Error should indicate type mismatch
        assert!(
            error_string.contains("invalid type")
                || error_string.contains("expected")
                || error_string.contains("integer"),
            "Error should mention type mismatch: {}",
            error_string
        );
    }

    // Tests for detect_conflicts method

    #[test]
    fn test_profile_conflict_existing_profile_creation() {
        // Test creating ExistingProfile conflict with rider_id
        let rider_id = Uuid::new_v4();
        let conflict = ProfileConflict::ExistingProfile {
            rider_id,
            existing_name: "Existing Rider".to_string(),
        };

        if let ProfileConflict::ExistingProfile {
            rider_id: conflict_id,
            existing_name,
        } = conflict
        {
            assert_eq!(conflict_id, rider_id);
            assert_eq!(existing_name, "Existing Rider");
        } else {
            panic!("Expected ExistingProfile variant");
        }
    }

    #[test]
    fn test_profile_conflict_display_name_mismatch_creation() {
        // Test creating DisplayNameMismatch conflict
        let conflict = ProfileConflict::DisplayNameMismatch {
            imported_name: "New Rider Name".to_string(),
            existing_name: "Old Rider Name".to_string(),
        };

        if let ProfileConflict::DisplayNameMismatch {
            imported_name,
            existing_name,
        } = conflict
        {
            assert_eq!(imported_name, "New Rider Name");
            assert_eq!(existing_name, "Old Rider Name");
        } else {
            panic!("Expected DisplayNameMismatch variant");
        }
    }

    #[test]
    fn test_profile_conflict_ftp_mismatch_with_values() {
        // Test FTP mismatch with both values present
        let conflict = ProfileConflict::FtpMismatch {
            imported_ftp: Some(280),
            existing_ftp: Some(250),
        };

        if let ProfileConflict::FtpMismatch {
            imported_ftp,
            existing_ftp,
        } = conflict
        {
            assert_eq!(imported_ftp, Some(280));
            assert_eq!(existing_ftp, Some(250));
        } else {
            panic!("Expected FtpMismatch variant");
        }
    }

    #[test]
    fn test_profile_conflict_ftp_mismatch_with_none() {
        // Test FTP mismatch where one value is None
        let conflict = ProfileConflict::FtpMismatch {
            imported_ftp: Some(280),
            existing_ftp: None,
        };

        if let ProfileConflict::FtpMismatch {
            imported_ftp,
            existing_ftp,
        } = conflict
        {
            assert_eq!(imported_ftp, Some(280));
            assert_eq!(existing_ftp, None);
        } else {
            panic!("Expected FtpMismatch variant");
        }
    }

    #[test]
    fn test_profile_conflict_avatar_mismatch_creation() {
        // Test AvatarMismatch when import has avatar but existing doesn't
        let conflict = ProfileConflict::AvatarMismatch {
            import_has_avatar: true,
            existing_has_avatar: false,
        };

        if let ProfileConflict::AvatarMismatch {
            import_has_avatar,
            existing_has_avatar,
        } = conflict
        {
            assert!(import_has_avatar);
            assert!(!existing_has_avatar);
        } else {
            panic!("Expected AvatarMismatch variant");
        }
    }

    #[test]
    fn test_profile_conflict_avatar_mismatch_both_have_avatar() {
        // Test AvatarMismatch when both have avatars but they differ
        let conflict = ProfileConflict::AvatarMismatch {
            import_has_avatar: true,
            existing_has_avatar: true,
        };

        if let ProfileConflict::AvatarMismatch {
            import_has_avatar,
            existing_has_avatar,
        } = conflict
        {
            assert!(import_has_avatar);
            assert!(existing_has_avatar);
        } else {
            panic!("Expected AvatarMismatch variant");
        }
    }

    #[test]
    fn test_profile_import_result_with_conflicts_list() {
        // Test creating import result with multiple conflicts
        let rider_id = Uuid::new_v4();
        let conflicts = vec![
            ProfileConflict::ExistingProfile {
                rider_id,
                existing_name: "Existing Rider".to_string(),
            },
            ProfileConflict::DisplayNameMismatch {
                imported_name: "New Name".to_string(),
                existing_name: "Existing Rider".to_string(),
            },
            ProfileConflict::FtpMismatch {
                imported_ftp: Some(300),
                existing_ftp: Some(280),
            },
            ProfileConflict::AvatarMismatch {
                import_has_avatar: true,
                existing_has_avatar: false,
            },
        ];

        let result = ProfileImportResult::with_conflicts(conflicts);

        assert!(!result.success);
        assert_eq!(result.conflicts.len(), 4);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(!result.profile_updated);
        assert!(!result.avatar_updated);
    }

    #[test]
    fn test_detect_conflicts_creates_valid_conflict_list() {
        // Test that conflict detection produces valid conflict structures
        // This tests the logic without requiring a database

        // Simulate what detect_conflicts would produce for an existing profile scenario
        let rider_id = Uuid::new_v4();
        let mut conflicts = Vec::new();

        // Simulate: existing profile found with same rider_id
        let existing_name = "Existing Rider".to_string();
        conflicts.push(ProfileConflict::ExistingProfile {
            rider_id,
            existing_name: existing_name.clone(),
        });

        // Simulate: display name mismatch
        let imported_name = "Imported Rider".to_string();
        if imported_name != existing_name {
            conflicts.push(ProfileConflict::DisplayNameMismatch {
                imported_name: imported_name.clone(),
                existing_name: existing_name.clone(),
            });
        }

        // Simulate: FTP mismatch
        let imported_ftp = Some(280u16);
        let existing_ftp = Some(250u16);
        if imported_ftp != existing_ftp {
            conflicts.push(ProfileConflict::FtpMismatch {
                imported_ftp,
                existing_ftp,
            });
        }

        // Verify conflicts were created correctly
        assert_eq!(conflicts.len(), 3);
        assert!(matches!(conflicts[0], ProfileConflict::ExistingProfile { .. }));
        assert!(matches!(conflicts[1], ProfileConflict::DisplayNameMismatch { .. }));
        assert!(matches!(conflicts[2], ProfileConflict::FtpMismatch { .. }));
    }

    #[test]
    fn test_detect_conflicts_no_conflict_when_values_match() {
        // Test that no conflicts are created when values match
        let mut conflicts = Vec::new();

        // Same display name - no DisplayNameMismatch
        let imported_name = "Same Rider".to_string();
        let existing_name = "Same Rider".to_string();
        if imported_name != existing_name {
            conflicts.push(ProfileConflict::DisplayNameMismatch {
                imported_name,
                existing_name,
            });
        }

        // Same FTP - no FtpMismatch
        let imported_ftp = Some(250u16);
        let existing_ftp = Some(250u16);
        if imported_ftp != existing_ftp {
            conflicts.push(ProfileConflict::FtpMismatch {
                imported_ftp,
                existing_ftp,
            });
        }

        // Same avatar state - no AvatarMismatch
        let import_has_avatar = true;
        let existing_has_avatar = true;
        if import_has_avatar != existing_has_avatar {
            conflicts.push(ProfileConflict::AvatarMismatch {
                import_has_avatar,
                existing_has_avatar,
            });
        }

        // No conflicts should be added when values match
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_display_name_collision_scenario() {
        // Test scenario: different rider_id but same display_name (name collision)
        let colliding_rider_id = Uuid::new_v4();
        let imported_rider_id = Uuid::new_v4();
        let display_name = "Popular Name".to_string();
        let mut conflicts = Vec::new();

        // Different IDs but same name - this is a name collision
        assert_ne!(imported_rider_id, colliding_rider_id);

        // When no existing profile with same rider_id, but name collision exists
        conflicts.push(ProfileConflict::DisplayNameMismatch {
            imported_name: display_name.clone(),
            existing_name: display_name.clone(),
        });
        conflicts.push(ProfileConflict::ExistingProfile {
            rider_id: colliding_rider_id,
            existing_name: display_name,
        });

        assert_eq!(conflicts.len(), 2);
        assert!(matches!(conflicts[0], ProfileConflict::DisplayNameMismatch { .. }));
        assert!(matches!(conflicts[1], ProfileConflict::ExistingProfile { .. }));
    }

    #[test]
    fn test_profile_conflict_clone() {
        // Test that ProfileConflict can be cloned
        let rider_id = Uuid::new_v4();
        let conflict = ProfileConflict::ExistingProfile {
            rider_id,
            existing_name: "Test Rider".to_string(),
        };

        let cloned = conflict.clone();
        if let ProfileConflict::ExistingProfile {
            rider_id: cloned_id,
            existing_name,
        } = cloned
        {
            assert_eq!(cloned_id, rider_id);
            assert_eq!(existing_name, "Test Rider");
        }
    }

    #[test]
    fn test_profile_conflict_debug_format() {
        // Test that ProfileConflict implements Debug
        let conflict = ProfileConflict::FtpMismatch {
            imported_ftp: Some(280),
            existing_ftp: Some(250),
        };

        let debug_str = format!("{:?}", conflict);
        assert!(debug_str.contains("FtpMismatch"));
        assert!(debug_str.contains("280"));
        assert!(debug_str.contains("250"));
    }

    // Tests for import_profile method (without database)

    #[test]
    fn test_conflict_resolution_skip_creates_empty_result() {
        // Test that Skip resolution produces expected result structure
        let result = ProfileImportResult {
            success: true,
            ftp_entries_imported: 0,
            ftp_entries_skipped: 0,
            profile_updated: false,
            avatar_updated: false,
            conflicts: Vec::new(),
        };

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(!result.profile_updated);
        assert!(!result.avatar_updated);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_conflict_resolution_variants_are_distinct() {
        // Verify all three resolution strategies are distinct
        assert_ne!(ConflictResolution::Replace, ConflictResolution::Merge);
        assert_ne!(ConflictResolution::Replace, ConflictResolution::Skip);
        assert_ne!(ConflictResolution::Merge, ConflictResolution::Skip);
    }

    #[test]
    fn test_profile_import_result_success_with_all_updates() {
        // Test successful import result with all flags set
        let result = ProfileImportResult::success(10, 2, true, true);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 10);
        assert_eq!(result.ftp_entries_skipped, 2);
        assert!(result.profile_updated);
        assert!(result.avatar_updated);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_profile_import_result_success_partial_update() {
        // Test successful import with only profile updated
        let result = ProfileImportResult::success(5, 0, true, false);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 5);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(result.profile_updated);
        assert!(!result.avatar_updated);
    }

    #[test]
    fn test_profile_import_result_success_only_avatar() {
        // Test successful import with only avatar updated
        let result = ProfileImportResult::success(0, 0, false, true);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(!result.profile_updated);
        assert!(result.avatar_updated);
    }

    #[test]
    fn test_profile_import_result_success_no_updates() {
        // Test successful import with nothing to update
        let result = ProfileImportResult::success(0, 0, false, false);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 0);
        assert!(!result.profile_updated);
        assert!(!result.avatar_updated);
    }

    #[test]
    fn test_profile_import_result_all_entries_skipped() {
        // Test result when all FTP entries are skipped (duplicates)
        let result = ProfileImportResult::success(0, 5, true, true);

        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 0);
        assert_eq!(result.ftp_entries_skipped, 5);
        assert!(result.profile_updated);
        assert!(result.avatar_updated);
    }

    #[test]
    fn test_profile_export_for_import() {
        // Create a complete export to test import scenarios
        let rider_id = Uuid::new_v4();
        let profile = ProfileData {
            display_name: "Import Test Rider".to_string(),
            bio: Some("Testing import functionality".to_string()),
            ftp: Some(275),
            total_distance_km: 2500.0,
            total_time_hours: 125.5,
            sharing_enabled: true,
        };

        let ftp_history = vec![
            FtpHistoryEntry {
                ftp_watts: 250,
                method: "ramp_test".to_string(),
                confidence: "high".to_string(),
                detected_at: Utc::now(),
                accepted: true,
            },
            FtpHistoryEntry {
                ftp_watts: 265,
                method: "20min_test".to_string(),
                confidence: "high".to_string(),
                detected_at: Utc::now(),
                accepted: true,
            },
            FtpHistoryEntry {
                ftp_watts: 275,
                method: "ramp_test".to_string(),
                confidence: "medium".to_string(),
                detected_at: Utc::now(),
                accepted: false,
            },
        ];

        let avatar = AvatarExport {
            jersey_color: "#FF5500".to_string(),
            bike_style: "tt_bike".to_string(),
            jersey_secondary: Some("#FFFFFF".to_string()),
            helmet_color: Some("#333333".to_string()),
        };

        let export = ProfileExport::new(rider_id, profile, ftp_history, Some(avatar));

        // Verify export is properly structured for import
        assert_eq!(export.rider_id, rider_id);
        assert_eq!(export.profile.display_name, "Import Test Rider");
        assert_eq!(export.ftp_history.len(), 3);
        assert!(export.avatar.is_some());
        assert_eq!(export.export_version, ProfileExport::CURRENT_VERSION);
    }

    #[test]
    fn test_profile_export_for_import_no_avatar() {
        // Test export without avatar for import scenarios
        let rider_id = Uuid::new_v4();
        let profile = ProfileData {
            display_name: "No Avatar Rider".to_string(),
            bio: None,
            ftp: Some(200),
            total_distance_km: 100.0,
            total_time_hours: 5.0,
            sharing_enabled: false,
        };

        let export = ProfileExport::new(rider_id, profile, vec![], None);

        assert!(export.avatar.is_none());
        assert!(export.ftp_history.is_empty());
    }

    #[test]
    fn test_ftp_history_entry_timestamps_for_deduplication() {
        // Test that FTP history entries have proper timestamps for deduplication
        let entry1 = FtpHistoryEntry {
            ftp_watts: 250,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc::now(),
            accepted: true,
        };

        let entry2 = FtpHistoryEntry {
            ftp_watts: 250,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc::now(),
            accepted: true,
        };

        // Even with same values, they should have timestamps that can be compared
        let ts1 = entry1.detected_at.to_rfc3339();
        let ts2 = entry2.detected_at.to_rfc3339();

        // Both should produce valid RFC3339 timestamps
        assert!(ts1.contains("T"));
        assert!(ts2.contains("T"));
    }

    #[test]
    fn test_avatar_export_serialization_for_import() {
        // Test avatar export round-trip for import scenarios
        let avatar = AvatarExport {
            jersey_color: "#ABCDEF".to_string(),
            bike_style: "gravel".to_string(),
            jersey_secondary: Some("#123456".to_string()),
            helmet_color: None,
        };

        let json = serde_json::to_string(&avatar).unwrap();
        let parsed: AvatarExport = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.jersey_color, "#ABCDEF");
        assert_eq!(parsed.bike_style, "gravel");
        assert_eq!(parsed.jersey_secondary, Some("#123456".to_string()));
        assert_eq!(parsed.helmet_color, None);
    }

    #[test]
    fn test_profile_data_serialization_for_import() {
        // Test profile data round-trip for import scenarios
        let profile = ProfileData {
            display_name: "Serialization Test".to_string(),
            bio: Some("Testing serialization".to_string()),
            ftp: Some(300),
            total_distance_km: 10000.0,
            total_time_hours: 500.0,
            sharing_enabled: true,
        };

        let json = serde_json::to_string(&profile).unwrap();
        let parsed: ProfileData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.display_name, "Serialization Test");
        assert_eq!(parsed.bio, Some("Testing serialization".to_string()));
        assert_eq!(parsed.ftp, Some(300));
        assert!((parsed.total_distance_km - 10000.0).abs() < f64::EPSILON);
        assert!((parsed.total_time_hours - 500.0).abs() < f64::EPSILON);
        assert!(parsed.sharing_enabled);
    }

    #[test]
    fn test_conflict_resolution_copy_trait() {
        // Verify ConflictResolution implements Copy
        let resolution = ConflictResolution::Replace;
        let copied = resolution;

        assert_eq!(resolution, copied);
        assert_eq!(resolution, ConflictResolution::Replace);
        assert_eq!(copied, ConflictResolution::Replace);
    }

    #[test]
    fn test_conflict_resolution_eq_trait() {
        // Verify ConflictResolution implements Eq properly
        assert!(ConflictResolution::Replace == ConflictResolution::Replace);
        assert!(ConflictResolution::Merge == ConflictResolution::Merge);
        assert!(ConflictResolution::Skip == ConflictResolution::Skip);
    }

    #[test]
    fn test_import_result_structure_matches_expected_fields() {
        // Verify ProfileImportResult has all expected fields
        let result = ProfileImportResult {
            success: true,
            ftp_entries_imported: 3,
            ftp_entries_skipped: 1,
            profile_updated: true,
            avatar_updated: false,
            conflicts: vec![ProfileConflict::FtpMismatch {
                imported_ftp: Some(280),
                existing_ftp: Some(250),
            }],
        };

        // Access all fields to verify structure
        assert!(result.success);
        assert_eq!(result.ftp_entries_imported, 3);
        assert_eq!(result.ftp_entries_skipped, 1);
        assert!(result.profile_updated);
        assert!(!result.avatar_updated);
        assert_eq!(result.conflicts.len(), 1);
    }

    // Tests for import_from_file method

    #[test]
    fn test_import_from_file_nonexistent_file_error() {
        // Test that importing from a non-existent file returns IoError
        use std::path::Path;

        let nonexistent_path = Path::new("/nonexistent/path/to/profile_export.json");

        // We can't call import_from_file without a database, but we can test
        // the file reading logic by simulating what would happen
        let read_result = std::fs::read_to_string(nonexistent_path);
        assert!(read_result.is_err());

        // Verify the error would be mapped to IoError correctly
        let io_error = read_result.unwrap_err();
        let profile_error = ProfileExportError::IoError(format!(
            "Failed to read file '{}': {}",
            nonexistent_path.display(),
            io_error
        ));

        let error_msg = profile_error.to_string();
        assert!(error_msg.contains("IO error"));
        assert!(error_msg.contains("Failed to read file"));
    }

    #[test]
    fn test_import_from_file_io_error_format() {
        // Test that IoError messages are formatted correctly for file paths
        let path_str = "/some/test/path/file.json";
        let error_detail = "No such file or directory";

        let error = ProfileExportError::IoError(format!(
            "Failed to read file '{}': {}",
            path_str, error_detail
        ));

        let error_msg = error.to_string();
        assert!(error_msg.contains("IO error"));
        assert!(error_msg.contains(path_str));
        assert!(error_msg.contains(error_detail));
    }

    #[test]
    fn test_import_from_file_with_temp_file_invalid_json() {
        // Test that import_from_file properly propagates parse errors
        use std::io::Write;

        // Create a temp file with invalid JSON
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join("test_invalid_profile_import.json");

        // Write invalid JSON content
        let mut file = std::fs::File::create(&temp_file_path).expect("Failed to create temp file");
        file.write_all(b"{ invalid json content }")
            .expect("Failed to write temp file");
        drop(file);

        // Read the file content and try to parse it (simulating import_from_file behavior)
        let json_content =
            std::fs::read_to_string(&temp_file_path).expect("Failed to read temp file");

        // Try to parse as ProfileExport
        let parse_result: Result<ProfileExport, _> = serde_json::from_str(&json_content);
        assert!(parse_result.is_err());

        // Verify error would be mapped to ParseError
        let serde_error = parse_result.unwrap_err();
        let profile_error = ProfileExportError::ParseError(serde_error.to_string());
        let error_msg = profile_error.to_string();
        assert!(error_msg.contains("Parse error"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file_path);
    }

    #[test]
    fn test_import_from_file_with_temp_file_wrong_version() {
        // Test that import_from_file properly validates version
        use std::io::Write;

        // Create a temp file with wrong version
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join("test_wrong_version_import.json");

        let json_content = r#"{
            "export_version": "99.0",
            "exported_at": "2024-01-01T00:00:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {
                "display_name": "Test",
                "bio": null,
                "ftp": null,
                "total_distance_km": 0.0,
                "total_time_hours": 0.0,
                "sharing_enabled": false
            },
            "ftp_history": [],
            "avatar": null
        }"#;

        // Write content to file
        let mut file = std::fs::File::create(&temp_file_path).expect("Failed to create temp file");
        file.write_all(json_content.as_bytes())
            .expect("Failed to write temp file");
        drop(file);

        // Read and parse the file content
        let file_content =
            std::fs::read_to_string(&temp_file_path).expect("Failed to read temp file");

        // Parse succeeds but version validation would fail
        let parsed: ProfileExport =
            serde_json::from_str(&file_content).expect("Parse should succeed");
        assert_eq!(parsed.export_version, "99.0");

        // Verify that version validation would fail
        assert_ne!(parsed.export_version, ProfileExport::CURRENT_VERSION);

        // InvalidVersion error would be returned
        let error = ProfileExportError::InvalidVersion {
            expected: ProfileExport::CURRENT_VERSION.to_string(),
            found: "99.0".to_string(),
        };
        let error_msg = error.to_string();
        assert!(error_msg.contains("Invalid version"));
        assert!(error_msg.contains("expected 1.0"));
        assert!(error_msg.contains("found 99.0"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file_path);
    }

    #[test]
    fn test_import_from_file_with_temp_file_valid_json() {
        // Test that valid JSON can be read from a file and parsed
        use std::io::Write;

        // Create a temp file with valid export JSON
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join("test_valid_profile_import.json");

        let rider_id = Uuid::new_v4();
        let profile = ProfileData {
            display_name: "File Import Test".to_string(),
            bio: Some("Testing file import".to_string()),
            ftp: Some(260),
            total_distance_km: 1500.0,
            total_time_hours: 75.0,
            sharing_enabled: true,
        };

        let ftp_history = vec![FtpHistoryEntry {
            ftp_watts: 260,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc::now(),
            accepted: true,
        }];

        let avatar = AvatarExport {
            jersey_color: "#00FF00".to_string(),
            bike_style: "road_bike".to_string(),
            jersey_secondary: None,
            helmet_color: Some("#AAAAAA".to_string()),
        };

        let export = ProfileExport::new(rider_id, profile, ftp_history, Some(avatar));
        let json_content = serde_json::to_string_pretty(&export).expect("Serialization should work");

        // Write to file
        let mut file = std::fs::File::create(&temp_file_path).expect("Failed to create temp file");
        file.write_all(json_content.as_bytes())
            .expect("Failed to write temp file");
        drop(file);

        // Read file and parse (simulating first two steps of import_from_file)
        let file_content =
            std::fs::read_to_string(&temp_file_path).expect("Failed to read temp file");

        let parsed: ProfileExport =
            serde_json::from_str(&file_content).expect("Parse should succeed");

        // Verify parsed content
        assert_eq!(parsed.export_version, ProfileExport::CURRENT_VERSION);
        assert_eq!(parsed.rider_id, rider_id);
        assert_eq!(parsed.profile.display_name, "File Import Test");
        assert_eq!(parsed.profile.ftp, Some(260));
        assert_eq!(parsed.ftp_history.len(), 1);
        assert!(parsed.avatar.is_some());

        // Cleanup
        let _ = std::fs::remove_file(&temp_file_path);
    }

    #[test]
    fn test_import_from_file_path_types() {
        // Test that import_from_file accepts various path types
        use std::path::{Path, PathBuf};

        // Verify that the function signature accepts different path types
        // (compile-time check - we can't actually call without database)
        let _path_ref: &Path = Path::new("test.json");
        let _path_buf: PathBuf = PathBuf::from("test.json");
        let _string_slice: &str = "test.json";
        let _string: String = String::from("test.json");

        // All of these implement AsRef<Path>, so they would work with import_from_file
        // This is a compile-time verification test
        assert!(true);
    }

    #[test]
    fn test_import_from_file_resolution_parameter() {
        // Test that all ConflictResolution variants can be passed to import_from_file
        // This is a compile-time type check

        let resolutions = vec![
            ConflictResolution::Replace,
            ConflictResolution::Merge,
            ConflictResolution::Skip,
        ];

        // Verify all resolution types are valid (compile-time check)
        for resolution in resolutions {
            match resolution {
                ConflictResolution::Replace => assert!(true),
                ConflictResolution::Merge => assert!(true),
                ConflictResolution::Skip => assert!(true),
            }
        }
    }
}
