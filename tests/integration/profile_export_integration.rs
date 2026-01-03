//! Integration tests for profile export workflow
//!
//! T023: Test full export: create profile in DB, export to JSON, verify all fields present.
//! Uses test database.

use chrono::{TimeZone, Utc};
use rusqlite::params;
use rustride::social::{ProfileExport, ProfileExporter};
use rustride::storage::database::Database;
use rustride::storage::social_store::{Rider, SocialStore};
use rustride::world::avatar::{AvatarConfig, BikeStyle};
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test database with an in-memory SQLite connection.
fn create_test_database() -> Database {
    Database::open_in_memory().expect("Failed to create test database")
}

/// Insert a test rider into the database.
fn insert_test_rider(db: &Database, rider: &Rider) {
    let store = SocialStore::new(db.connection());
    store.insert_rider(rider).expect("Failed to insert test rider");
}

/// Insert a test FTP estimate directly into the database.
fn insert_test_ftp_estimate(
    db: &Database,
    user_id: Uuid,
    ftp_watts: u16,
    method: &str,
    confidence: &str,
    detected_at: &str,
    accepted: bool,
) {
    let id = Uuid::new_v4();
    db.connection()
        .execute(
            "INSERT INTO ftp_estimates (id, user_id, ftp_watts, method, confidence,
             supporting_data_json, detected_at, accepted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6, ?7, ?6)",
            params![
                id.to_string(),
                user_id.to_string(),
                ftp_watts,
                method,
                confidence,
                detected_at,
                if accepted { 1 } else { 0 },
            ],
        )
        .expect("Failed to insert FTP estimate");
}

/// Insert a test avatar directly into the database.
fn insert_test_avatar(db: &Database, user_id: Uuid, config: &AvatarConfig) {
    db.save_avatar(&user_id, config)
        .expect("Failed to insert test avatar");
}

// =============================================================================
// Export Workflow Integration Tests
// =============================================================================

/// Test basic export: create profile in DB, export to JSON, verify required fields.
#[test]
fn test_export_basic_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create and insert a basic rider profile
    let rider = Rider {
        id: rider_id,
        display_name: "TestRider123".to_string(),
        avatar_id: None,
        bio: Some("Integration test bio".to_string()),
        ftp: Some(265),
        total_distance_km: 1500.5,
        total_time_hours: 75.25,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create exporter and export to JSON
    let exporter = ProfileExporter::new(Arc::new(db));
    let json_result = exporter.export_json(rider_id);

    assert!(json_result.is_ok(), "Export should succeed: {:?}", json_result.err());
    let json = json_result.unwrap();

    // Parse the JSON and verify required fields
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    // Verify top-level fields
    assert!(parsed.get("export_version").is_some(), "export_version field must be present");
    assert!(parsed.get("exported_at").is_some(), "exported_at field must be present");
    assert!(parsed.get("rider_id").is_some(), "rider_id field must be present");
    assert!(parsed.get("profile").is_some(), "profile field must be present");
    assert!(parsed.get("ftp_history").is_some(), "ftp_history field must be present");

    // Verify profile data
    let profile = parsed.get("profile").unwrap();
    assert_eq!(profile.get("display_name").and_then(|v| v.as_str()), Some("TestRider123"));
    assert_eq!(profile.get("bio").and_then(|v| v.as_str()), Some("Integration test bio"));
    assert_eq!(profile.get("ftp").and_then(|v| v.as_u64()), Some(265));
    assert!(profile.get("total_distance_km").is_some());
    assert!(profile.get("total_time_hours").is_some());
    assert_eq!(profile.get("sharing_enabled").and_then(|v| v.as_bool()), Some(true));

    // Verify rider_id matches
    assert_eq!(
        parsed.get("rider_id").and_then(|v| v.as_str()),
        Some(rider_id.to_string().as_str())
    );

    // Verify export_version is current
    assert_eq!(
        parsed.get("export_version").and_then(|v| v.as_str()),
        Some(ProfileExport::CURRENT_VERSION)
    );
}

/// Test export with FTP history: multiple FTP entries should be exported.
#[test]
fn test_export_with_ftp_history() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider
    let rider = Rider {
        id: rider_id,
        display_name: "FTPRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(280),
        total_distance_km: 2000.0,
        total_time_hours: 100.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Insert FTP history entries
    let date1 = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap().to_rfc3339();
    let date2 = Utc.with_ymd_and_hms(2024, 3, 20, 14, 30, 0).unwrap().to_rfc3339();
    let date3 = Utc.with_ymd_and_hms(2024, 6, 10, 8, 0, 0).unwrap().to_rfc3339();

    insert_test_ftp_estimate(&db, rider_id, 250, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(&db, rider_id, 265, "20min_test", "high", &date2, true);
    insert_test_ftp_estimate(&db, rider_id, 280, "manual", "medium", &date3, false);

    // Export and verify
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Verify FTP history
    let ftp_history = parsed.get("ftp_history").and_then(|v| v.as_array());
    assert!(ftp_history.is_some(), "ftp_history should be an array");
    let history = ftp_history.unwrap();
    assert_eq!(history.len(), 3, "Should have 3 FTP entries");

    // Entries should be ordered by detected_at DESC (most recent first)
    let first_entry = &history[0];
    assert_eq!(first_entry.get("ftp_watts").and_then(|v| v.as_u64()), Some(280));
    assert_eq!(first_entry.get("method").and_then(|v| v.as_str()), Some("manual"));
    assert_eq!(first_entry.get("confidence").and_then(|v| v.as_str()), Some("medium"));
    assert_eq!(first_entry.get("accepted").and_then(|v| v.as_bool()), Some(false));

    let last_entry = &history[2];
    assert_eq!(last_entry.get("ftp_watts").and_then(|v| v.as_u64()), Some(250));
    assert_eq!(last_entry.get("method").and_then(|v| v.as_str()), Some("ramp_test"));
    assert_eq!(last_entry.get("accepted").and_then(|v| v.as_bool()), Some(true));

    // Verify all entries have required fields
    for entry in history {
        assert!(entry.get("ftp_watts").is_some(), "ftp_watts required");
        assert!(entry.get("method").is_some(), "method required");
        assert!(entry.get("confidence").is_some(), "confidence required");
        assert!(entry.get("detected_at").is_some(), "detected_at required");
        assert!(entry.get("accepted").is_some(), "accepted required");
    }
}

/// Test export with avatar configuration.
#[test]
fn test_export_with_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider
    let rider = Rider {
        id: rider_id,
        display_name: "AvatarRider".to_string(),
        avatar_id: Some("avatar_123".to_string()),
        bio: None,
        ftp: None,
        total_distance_km: 500.0,
        total_time_hours: 25.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Insert avatar with full configuration
    let avatar_config = AvatarConfig {
        jersey_color: [255, 128, 0], // Orange
        bike_style: BikeStyle::Road,
        jersey_secondary: Some([0, 0, 255]), // Blue
        helmet_color: Some([0, 0, 0]), // Black
    };
    insert_test_avatar(&db, rider_id, &avatar_config);

    // Export and verify
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Verify avatar is present
    let avatar = parsed.get("avatar");
    assert!(avatar.is_some(), "avatar field should be present");
    let avatar = avatar.unwrap();

    // Verify avatar fields (colors stored as hex strings)
    assert!(avatar.get("jersey_color").is_some(), "jersey_color required");
    assert!(avatar.get("bike_style").is_some(), "bike_style required");
    assert!(avatar.get("jersey_secondary").is_some(), "jersey_secondary required");
    assert!(avatar.get("helmet_color").is_some(), "helmet_color required");

    // Verify color format (hex string like "#FF8000")
    let jersey_color = avatar.get("jersey_color").and_then(|v| v.as_str());
    assert!(jersey_color.is_some());
    assert!(
        jersey_color.unwrap().starts_with('#'),
        "jersey_color should be hex format"
    );
    assert_eq!(
        jersey_color.unwrap().len(),
        7,
        "jersey_color should be #RRGGBB format"
    );
}

/// Test export with all fields populated (complete profile).
#[test]
fn test_export_complete_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create a complete rider profile
    let rider = Rider {
        id: rider_id,
        display_name: "CompleteRider".to_string(),
        avatar_id: Some("complete_avatar".to_string()),
        bio: Some("A complete rider profile with all fields populated for testing.".to_string()),
        ftp: Some(300),
        total_distance_km: 10000.0,
        total_time_hours: 500.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Add FTP history
    let date1 = Utc.with_ymd_and_hms(2023, 6, 1, 12, 0, 0).unwrap().to_rfc3339();
    let date2 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap().to_rfc3339();
    insert_test_ftp_estimate(&db, rider_id, 280, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(&db, rider_id, 300, "20min_test", "high", &date2, true);

    // Add avatar
    let avatar_config = AvatarConfig {
        jersey_color: [255, 0, 0], // Red
        bike_style: BikeStyle::TT,
        jersey_secondary: Some([255, 255, 255]), // White
        helmet_color: Some([128, 128, 128]), // Gray
    };
    insert_test_avatar(&db, rider_id, &avatar_config);

    // Export
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Verify all top-level fields
    assert!(parsed.get("export_version").is_some());
    assert!(parsed.get("exported_at").is_some());
    assert!(parsed.get("rider_id").is_some());
    assert!(parsed.get("profile").is_some());
    assert!(parsed.get("ftp_history").is_some());
    assert!(parsed.get("avatar").is_some());

    // Verify profile
    let profile = parsed.get("profile").unwrap();
    assert_eq!(profile.get("display_name").and_then(|v| v.as_str()), Some("CompleteRider"));
    assert!(profile.get("bio").and_then(|v| v.as_str()).is_some());
    assert_eq!(profile.get("ftp").and_then(|v| v.as_u64()), Some(300));
    assert_eq!(profile.get("total_distance_km").and_then(|v| v.as_f64()), Some(10000.0));
    assert_eq!(profile.get("total_time_hours").and_then(|v| v.as_f64()), Some(500.0));
    assert_eq!(profile.get("sharing_enabled").and_then(|v| v.as_bool()), Some(true));

    // Verify FTP history
    let ftp_history = parsed.get("ftp_history").and_then(|v| v.as_array()).unwrap();
    assert_eq!(ftp_history.len(), 2);

    // Verify avatar
    let avatar = parsed.get("avatar").unwrap();
    assert!(avatar.get("jersey_color").is_some());
    assert!(avatar.get("bike_style").is_some());
    assert!(avatar.get("jersey_secondary").is_some());
    assert!(avatar.get("helmet_color").is_some());
}

/// Test export without avatar (avatar should be null in JSON).
#[test]
fn test_export_without_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider without avatar
    let rider = Rider {
        id: rider_id,
        display_name: "NoAvatarRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(200),
        total_distance_km: 100.0,
        total_time_hours: 5.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Export without inserting any avatar
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Avatar should be null
    let avatar = parsed.get("avatar");
    assert!(avatar.is_some(), "avatar field should exist");
    assert!(avatar.unwrap().is_null(), "avatar should be null when not set");
}

/// Test export with empty FTP history.
#[test]
fn test_export_with_empty_ftp_history() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider without any FTP history
    let rider = Rider {
        id: rider_id,
        display_name: "NoFTPRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: None,
        total_distance_km: 50.0,
        total_time_hours: 2.5,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Export
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // FTP history should be an empty array
    let ftp_history = parsed.get("ftp_history").and_then(|v| v.as_array());
    assert!(ftp_history.is_some(), "ftp_history should be present");
    assert!(ftp_history.unwrap().is_empty(), "ftp_history should be empty");

    // Profile FTP should be null
    let profile = parsed.get("profile").unwrap();
    assert!(profile.get("ftp").unwrap().is_null(), "ftp should be null when not set");
}

/// Test export with minimal profile (no optional fields).
#[test]
fn test_export_minimal_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create minimal rider profile
    let rider = Rider {
        id: rider_id,
        display_name: "MinimalRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: None,
        total_distance_km: 0.0,
        total_time_hours: 0.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Export
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // All required fields should be present
    assert!(parsed.get("export_version").is_some());
    assert!(parsed.get("exported_at").is_some());
    assert!(parsed.get("rider_id").is_some());
    assert!(parsed.get("profile").is_some());
    assert!(parsed.get("ftp_history").is_some());
    // avatar may or may not be present as null

    let profile = parsed.get("profile").unwrap();
    assert_eq!(profile.get("display_name").and_then(|v| v.as_str()), Some("MinimalRider"));
    assert!(profile.get("bio").unwrap().is_null());
    assert!(profile.get("ftp").unwrap().is_null());
    assert_eq!(profile.get("total_distance_km").and_then(|v| v.as_f64()), Some(0.0));
    assert_eq!(profile.get("total_time_hours").and_then(|v| v.as_f64()), Some(0.0));
    assert_eq!(profile.get("sharing_enabled").and_then(|v| v.as_bool()), Some(false));
}

/// Test export returns error for non-existent profile.
#[test]
fn test_export_profile_not_found() {
    let db = create_test_database();
    let non_existent_id = Uuid::new_v4();

    let exporter = ProfileExporter::new(Arc::new(db));
    let result = exporter.export_json(non_existent_id);

    assert!(result.is_err(), "Export should fail for non-existent profile");
    let error = result.unwrap_err();
    assert!(
        format!("{:?}", error).contains("ProfileNotFound"),
        "Error should be ProfileNotFound"
    );
}

/// Test that exported JSON is pretty-printed with newlines and indentation.
#[test]
fn test_export_json_is_pretty_printed() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "PrettyRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: None,
        total_distance_km: 0.0,
        total_time_hours: 0.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");

    // Pretty-printed JSON should contain newlines and indentation
    assert!(json.contains('\n'), "Pretty JSON should have newlines");
    assert!(json.contains("  "), "Pretty JSON should have indentation");
}

/// Test exported_at timestamp is in ISO8601/RFC3339 format.
#[test]
fn test_export_timestamp_format() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "TimestampRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: None,
        total_distance_km: 0.0,
        total_time_hours: 0.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    let exported_at = parsed.get("exported_at").and_then(|v| v.as_str());
    assert!(exported_at.is_some(), "exported_at should be a string");

    // Should be parseable as RFC3339
    let timestamp_str = exported_at.unwrap();
    let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str);
    assert!(
        parsed_timestamp.is_ok(),
        "exported_at should be valid RFC3339: {}",
        timestamp_str
    );
}

/// Test export with special characters in display name and bio.
#[test]
fn test_export_with_special_characters() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider with special characters
    let rider = Rider {
        id: rider_id,
        display_name: "Rider\"Quote'Apos<Tag>&Amp".to_string(),
        avatar_id: None,
        bio: Some("Bio with\nnewline\tand\ttabs \"quotes\" 'apostrophes' <tags> &ampersands;".to_string()),
        ftp: Some(250),
        total_distance_km: 100.0,
        total_time_hours: 5.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");

    // Should be valid JSON even with special characters
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    let profile = parsed.get("profile").unwrap();
    assert_eq!(
        profile.get("display_name").and_then(|v| v.as_str()),
        Some("Rider\"Quote'Apos<Tag>&Amp")
    );
    assert!(profile.get("bio").and_then(|v| v.as_str()).unwrap().contains("newline"));
}

/// Test export with unicode characters in profile fields.
#[test]
fn test_export_with_unicode() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider with unicode characters
    let rider = Rider {
        id: rider_id,
        display_name: "骑手🚴‍♂️Ćÿçlîst".to_string(),
        avatar_id: None,
        bio: Some("Emoji: 💪🚵‍♀️🏔️ • Accents: àéïõü • Symbols: ∑∆©®".to_string()),
        ftp: Some(275),
        total_distance_km: 888.88,
        total_time_hours: 44.44,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter.export_json(rider_id).expect("Export should succeed");

    // Should handle unicode correctly
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    let profile = parsed.get("profile").unwrap();
    let display_name = profile.get("display_name").and_then(|v| v.as_str()).unwrap();
    assert!(display_name.contains("🚴"));
    assert!(display_name.contains("骑手"));

    let bio = profile.get("bio").and_then(|v| v.as_str()).unwrap();
    assert!(bio.contains("💪"));
    assert!(bio.contains("∆"));
}

/// Test export to file creates the file correctly.
#[test]
fn test_export_to_file() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "FileExportRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(260),
        total_distance_km: 500.0,
        total_time_hours: 25.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create temp directory for test
    let temp_dir = std::env::temp_dir().join(format!("profile_export_test_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Create temp dir");
    let export_path = temp_dir.join("test_profile.json");

    let exporter = ProfileExporter::new(Arc::new(db));
    let result = exporter.export_to_file(rider_id, &export_path);

    assert!(result.is_ok(), "Export to file should succeed: {:?}", result.err());
    assert!(export_path.exists(), "Export file should exist");

    // Read and verify file content
    let content = std::fs::read_to_string(&export_path).expect("Read file");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Valid JSON");
    assert!(parsed.get("export_version").is_some());
    assert_eq!(
        parsed.get("profile").unwrap().get("display_name").and_then(|v| v.as_str()),
        Some("FileExportRider")
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Test build_export returns ProfileExport struct directly.
#[test]
fn test_build_export_returns_struct() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "BuildExportRider".to_string(),
        avatar_id: None,
        bio: Some("Testing build_export method".to_string()),
        ftp: Some(270),
        total_distance_km: 1234.5,
        total_time_hours: 67.89,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.build_export(rider_id).expect("Build export should succeed");

    // Verify the ProfileExport struct
    assert_eq!(export.export_version, ProfileExport::CURRENT_VERSION);
    assert_eq!(export.rider_id, rider_id);
    assert_eq!(export.profile.display_name, "BuildExportRider");
    assert_eq!(export.profile.bio, Some("Testing build_export method".to_string()));
    assert_eq!(export.profile.ftp, Some(270));
    assert!((export.profile.total_distance_km - 1234.5).abs() < 0.01);
    assert!((export.profile.total_time_hours - 67.89).abs() < 0.01);
    assert!(!export.profile.sharing_enabled);
    assert!(export.ftp_history.is_empty());
    assert!(export.avatar.is_none());
}
