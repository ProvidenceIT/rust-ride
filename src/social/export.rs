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
}
