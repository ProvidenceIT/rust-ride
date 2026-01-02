//! Rider profile export and import.
//!
//! Provides JSON export for rider profiles including settings, FTP history,
//! and avatar configuration. Enables profile backup and transfer between installations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
