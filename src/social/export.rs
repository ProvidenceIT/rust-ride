//! Rider profile export and import.
//!
//! Provides JSON export for rider profiles including settings, FTP history,
//! and avatar configuration. Enables profile backup and transfer between installations.

use chrono::{DateTime, Utc};
use rusqlite;
use serde::{Deserialize, Serialize};
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
}
