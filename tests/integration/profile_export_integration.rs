//! Integration tests for profile export and import workflow
//!
//! T023: Test full export: create profile in DB, export to JSON, verify all fields present.
//! T024: Test import scenarios: empty DB import, conflict detection, merge strategy, replace strategy.
//! Uses test database.

use chrono::{TimeZone, Utc};
use rusqlite::params;
use rustride::social::{
    ConflictResolution, ProfileConflict, ProfileData, ProfileExport, ProfileExporter,
};
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
    store
        .insert_rider(rider)
        .expect("Failed to insert test rider");
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

    assert!(
        json_result.is_ok(),
        "Export should succeed: {:?}",
        json_result.err()
    );
    let json = json_result.unwrap();

    // Parse the JSON and verify required fields
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    // Verify top-level fields
    assert!(
        parsed.get("export_version").is_some(),
        "export_version field must be present"
    );
    assert!(
        parsed.get("exported_at").is_some(),
        "exported_at field must be present"
    );
    assert!(
        parsed.get("rider_id").is_some(),
        "rider_id field must be present"
    );
    assert!(
        parsed.get("profile").is_some(),
        "profile field must be present"
    );
    assert!(
        parsed.get("ftp_history").is_some(),
        "ftp_history field must be present"
    );

    // Verify profile data
    let profile = parsed.get("profile").unwrap();
    assert_eq!(
        profile.get("display_name").and_then(|v| v.as_str()),
        Some("TestRider123")
    );
    assert_eq!(
        profile.get("bio").and_then(|v| v.as_str()),
        Some("Integration test bio")
    );
    assert_eq!(profile.get("ftp").and_then(|v| v.as_u64()), Some(265));
    assert!(profile.get("total_distance_km").is_some());
    assert!(profile.get("total_time_hours").is_some());
    assert_eq!(
        profile.get("sharing_enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

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
    let date1 = Utc
        .with_ymd_and_hms(2024, 1, 15, 10, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 3, 20, 14, 30, 0)
        .unwrap()
        .to_rfc3339();
    let date3 = Utc
        .with_ymd_and_hms(2024, 6, 10, 8, 0, 0)
        .unwrap()
        .to_rfc3339();

    insert_test_ftp_estimate(&db, rider_id, 250, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(&db, rider_id, 265, "20min_test", "high", &date2, true);
    insert_test_ftp_estimate(&db, rider_id, 280, "manual", "medium", &date3, false);

    // Export and verify
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Verify FTP history
    let ftp_history = parsed.get("ftp_history").and_then(|v| v.as_array());
    assert!(ftp_history.is_some(), "ftp_history should be an array");
    let history = ftp_history.unwrap();
    assert_eq!(history.len(), 3, "Should have 3 FTP entries");

    // Entries should be ordered by detected_at DESC (most recent first)
    let first_entry = &history[0];
    assert_eq!(
        first_entry.get("ftp_watts").and_then(|v| v.as_u64()),
        Some(280)
    );
    assert_eq!(
        first_entry.get("method").and_then(|v| v.as_str()),
        Some("manual")
    );
    assert_eq!(
        first_entry.get("confidence").and_then(|v| v.as_str()),
        Some("medium")
    );
    assert_eq!(
        first_entry.get("accepted").and_then(|v| v.as_bool()),
        Some(false)
    );

    let last_entry = &history[2];
    assert_eq!(
        last_entry.get("ftp_watts").and_then(|v| v.as_u64()),
        Some(250)
    );
    assert_eq!(
        last_entry.get("method").and_then(|v| v.as_str()),
        Some("ramp_test")
    );
    assert_eq!(
        last_entry.get("accepted").and_then(|v| v.as_bool()),
        Some(true)
    );

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
        helmet_color: Some([0, 0, 0]),       // Black
    };
    insert_test_avatar(&db, rider_id, &avatar_config);

    // Export and verify
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Verify avatar is present
    let avatar = parsed.get("avatar");
    assert!(avatar.is_some(), "avatar field should be present");
    let avatar = avatar.unwrap();

    // Verify avatar fields (colors stored as hex strings)
    assert!(
        avatar.get("jersey_color").is_some(),
        "jersey_color required"
    );
    assert!(avatar.get("bike_style").is_some(), "bike_style required");
    assert!(
        avatar.get("jersey_secondary").is_some(),
        "jersey_secondary required"
    );
    assert!(
        avatar.get("helmet_color").is_some(),
        "helmet_color required"
    );

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
    let date1 = Utc
        .with_ymd_and_hms(2023, 6, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    insert_test_ftp_estimate(&db, rider_id, 280, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(&db, rider_id, 300, "20min_test", "high", &date2, true);

    // Add avatar
    let avatar_config = AvatarConfig {
        jersey_color: [255, 0, 0], // Red
        bike_style: BikeStyle::TT,
        jersey_secondary: Some([255, 255, 255]), // White
        helmet_color: Some([128, 128, 128]),     // Gray
    };
    insert_test_avatar(&db, rider_id, &avatar_config);

    // Export
    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
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
    assert_eq!(
        profile.get("display_name").and_then(|v| v.as_str()),
        Some("CompleteRider")
    );
    assert!(profile.get("bio").and_then(|v| v.as_str()).is_some());
    assert_eq!(profile.get("ftp").and_then(|v| v.as_u64()), Some(300));
    assert_eq!(
        profile.get("total_distance_km").and_then(|v| v.as_f64()),
        Some(10000.0)
    );
    assert_eq!(
        profile.get("total_time_hours").and_then(|v| v.as_f64()),
        Some(500.0)
    );
    assert_eq!(
        profile.get("sharing_enabled").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify FTP history
    let ftp_history = parsed
        .get("ftp_history")
        .and_then(|v| v.as_array())
        .unwrap();
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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // Avatar should be null
    let avatar = parsed.get("avatar");
    assert!(avatar.is_some(), "avatar field should exist");
    assert!(
        avatar.unwrap().is_null(),
        "avatar should be null when not set"
    );
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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // FTP history should be an empty array
    let ftp_history = parsed.get("ftp_history").and_then(|v| v.as_array());
    assert!(ftp_history.is_some(), "ftp_history should be present");
    assert!(
        ftp_history.unwrap().is_empty(),
        "ftp_history should be empty"
    );

    // Profile FTP should be null
    let profile = parsed.get("profile").unwrap();
    assert!(
        profile.get("ftp").unwrap().is_null(),
        "ftp should be null when not set"
    );
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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    // All required fields should be present
    assert!(parsed.get("export_version").is_some());
    assert!(parsed.get("exported_at").is_some());
    assert!(parsed.get("rider_id").is_some());
    assert!(parsed.get("profile").is_some());
    assert!(parsed.get("ftp_history").is_some());
    // avatar may or may not be present as null

    let profile = parsed.get("profile").unwrap();
    assert_eq!(
        profile.get("display_name").and_then(|v| v.as_str()),
        Some("MinimalRider")
    );
    assert!(profile.get("bio").unwrap().is_null());
    assert!(profile.get("ftp").unwrap().is_null());
    assert_eq!(
        profile.get("total_distance_km").and_then(|v| v.as_f64()),
        Some(0.0)
    );
    assert_eq!(
        profile.get("total_time_hours").and_then(|v| v.as_f64()),
        Some(0.0)
    );
    assert_eq!(
        profile.get("sharing_enabled").and_then(|v| v.as_bool()),
        Some(false)
    );
}

/// Test export returns error for non-existent profile.
#[test]
fn test_export_profile_not_found() {
    let db = create_test_database();
    let non_existent_id = Uuid::new_v4();

    let exporter = ProfileExporter::new(Arc::new(db));
    let result = exporter.export_json(non_existent_id);

    assert!(
        result.is_err(),
        "Export should fail for non-existent profile"
    );
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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");

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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");
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
        bio: Some(
            "Bio with\nnewline\tand\ttabs \"quotes\" 'apostrophes' <tags> &ampersands;".to_string(),
        ),
        ftp: Some(250),
        total_distance_km: 100.0,
        total_time_hours: 5.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    let exporter = ProfileExporter::new(Arc::new(db));
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");

    // Should be valid JSON even with special characters
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    let profile = parsed.get("profile").unwrap();
    assert_eq!(
        profile.get("display_name").and_then(|v| v.as_str()),
        Some("Rider\"Quote'Apos<Tag>&Amp")
    );
    assert!(profile
        .get("bio")
        .and_then(|v| v.as_str())
        .unwrap()
        .contains("newline"));
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
    let json = exporter
        .export_json(rider_id)
        .expect("Export should succeed");

    // Should handle unicode correctly
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");

    let profile = parsed.get("profile").unwrap();
    let display_name = profile
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap();
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

    assert!(
        result.is_ok(),
        "Export to file should succeed: {:?}",
        result.err()
    );
    assert!(export_path.exists(), "Export file should exist");

    // Read and verify file content
    let content = std::fs::read_to_string(&export_path).expect("Read file");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Valid JSON");
    assert!(parsed.get("export_version").is_some());
    assert_eq!(
        parsed
            .get("profile")
            .unwrap()
            .get("display_name")
            .and_then(|v| v.as_str()),
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
    let export = exporter
        .build_export(rider_id)
        .expect("Build export should succeed");

    // Verify the ProfileExport struct
    assert_eq!(export.export_version, ProfileExport::CURRENT_VERSION);
    assert_eq!(export.rider_id, rider_id);
    assert_eq!(export.profile.display_name, "BuildExportRider");
    assert_eq!(
        export.profile.bio,
        Some("Testing build_export method".to_string())
    );
    assert_eq!(export.profile.ftp, Some(270));
    assert!((export.profile.total_distance_km - 1234.5).abs() < 0.01);
    assert!((export.profile.total_time_hours - 67.89).abs() < 0.01);
    assert!(!export.profile.sharing_enabled);
    assert!(export.ftp_history.is_empty());
    assert!(export.avatar.is_none());
}

// =============================================================================
// Import Workflow Integration Tests (T024)
// =============================================================================

/// Helper to create an export JSON string for testing imports.
fn create_test_export_json(
    rider_id: Uuid,
    display_name: &str,
    ftp: Option<u16>,
    ftp_history: Vec<(&str, u16, &str, &str, bool)>, // (date, watts, method, confidence, accepted)
    avatar: Option<(&str, &str, Option<&str>, Option<&str>)>, // (jersey, bike_style, secondary, helmet)
) -> String {
    let ftp_entries: String = ftp_history
        .iter()
        .map(|(date, watts, method, confidence, accepted)| {
            format!(
                r#"{{"ftp_watts":{},"method":"{}","confidence":"{}","detected_at":"{}","accepted":{}}}"#,
                watts, method, confidence, date, accepted
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let avatar_json = match avatar {
        Some((jersey, bike_style, secondary, helmet)) => {
            let sec = match secondary {
                Some(s) => format!("\"{}\"", s),
                None => "null".to_string(),
            };
            let helm = match helmet {
                Some(h) => format!("\"{}\"", h),
                None => "null".to_string(),
            };
            format!(
                r#"{{"jersey_color":"{}","bike_style":"{}","jersey_secondary":{},"helmet_color":{}}}"#,
                jersey, bike_style, sec, helm
            )
        }
        None => "null".to_string(),
    };

    let ftp_json = match ftp {
        Some(f) => f.to_string(),
        None => "null".to_string(),
    };

    format!(
        r#"{{
  "export_version": "1.0",
  "exported_at": "2024-06-15T12:00:00Z",
  "rider_id": "{}",
  "profile": {{
    "display_name": "{}",
    "bio": null,
    "ftp": {},
    "total_distance_km": 500.0,
    "total_time_hours": 25.0,
    "sharing_enabled": true
  }},
  "ftp_history": [{}],
  "avatar": {}
}}"#,
        rider_id, display_name, ftp_json, ftp_entries, avatar_json
    )
}

/// Helper to query profile from database.
fn query_profile_from_db(db: &Database, rider_id: Uuid) -> Option<(String, Option<u16>, bool)> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare("SELECT display_name, ftp, sharing_enabled FROM riders WHERE id = ?1")
        .ok()?;

    stmt.query_row([rider_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<u16>>(1)?,
            row.get::<_, bool>(2)?,
        ))
    })
    .ok()
}

/// Helper to count FTP history entries in database.
fn count_ftp_entries(db: &Database, rider_id: Uuid) -> usize {
    let conn = db.connection();
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM ftp_estimates WHERE user_id = ?1")
        .expect("Prepare statement");
    stmt.query_row([rider_id.to_string()], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as usize
}

/// Helper to query avatar from database.
fn query_avatar_from_db(db: &Database, rider_id: Uuid) -> Option<(String, String)> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare("SELECT jersey_color, bike_style FROM avatars WHERE user_id = ?1")
        .ok()?;

    stmt.query_row([rider_id.to_string()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .ok()
}

// =============================================================================
// Empty DB Import Tests
// =============================================================================

/// Test import to empty database: profile is created successfully.
#[test]
fn test_import_to_empty_db_creates_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    let json = create_test_export_json(rider_id, "NewRider", Some(275), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success, "Import should succeed");
    assert!(result.profile_updated, "Profile should be created");

    // Verify profile was inserted
    let profile = query_profile_from_db(&db, rider_id);
    assert!(profile.is_some(), "Profile should exist in database");
    let (name, ftp, sharing) = profile.unwrap();
    assert_eq!(name, "NewRider");
    assert_eq!(ftp, Some(275));
    assert!(sharing);
}

/// Test import to empty database with FTP history: all entries imported.
#[test]
fn test_import_to_empty_db_with_ftp_history() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    let ftp_history = vec![
        ("2024-01-15T10:00:00Z", 250u16, "ramp_test", "high", true),
        ("2024-03-20T14:30:00Z", 265, "20min_test", "high", true),
        ("2024-06-10T08:00:00Z", 280, "manual", "medium", false),
    ];

    let json = create_test_export_json(rider_id, "FTPRider", Some(280), ftp_history, None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert_eq!(
        result.ftp_entries_imported, 3,
        "All 3 FTP entries should be imported"
    );
    assert_eq!(
        result.ftp_entries_skipped, 0,
        "No entries should be skipped"
    );

    // Verify FTP entries in database
    let count = count_ftp_entries(&db, rider_id);
    assert_eq!(count, 3, "Database should have 3 FTP entries");
}

/// Test import to empty database with avatar: avatar is created.
#[test]
fn test_import_to_empty_db_with_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    let avatar = ("#FF0000", "road_bike", Some("#FFFFFF"), Some("#000000"));
    let json = create_test_export_json(rider_id, "AvatarRider", None, vec![], Some(avatar));

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.avatar_updated, "Avatar should be created");

    // Verify avatar in database
    let avatar_data = query_avatar_from_db(&db, rider_id);
    assert!(avatar_data.is_some(), "Avatar should exist in database");
    let (jersey, bike) = avatar_data.unwrap();
    assert_eq!(jersey, "#FF0000");
    assert_eq!(bike, "road_bike");
}

/// Test import to empty database with complete data: all data imported.
#[test]
fn test_import_to_empty_db_complete_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    let ftp_history = vec![
        ("2024-01-01T12:00:00Z", 260u16, "ramp_test", "high", true),
        ("2024-06-01T12:00:00Z", 280, "20min_test", "high", true),
    ];
    let avatar = ("#00FF00", "tt_bike", None, Some("#888888"));
    let json = create_test_export_json(
        rider_id,
        "CompleteImport",
        Some(280),
        ftp_history,
        Some(avatar),
    );

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert!(result.avatar_updated);
    assert_eq!(result.ftp_entries_imported, 2);

    // Verify all data
    let profile = query_profile_from_db(&db, rider_id);
    assert!(profile.is_some());
    let avatar_data = query_avatar_from_db(&db, rider_id);
    assert!(avatar_data.is_some());
    let ftp_count = count_ftp_entries(&db, rider_id);
    assert_eq!(ftp_count, 2);
}

// =============================================================================
// Conflict Detection Tests
// =============================================================================

/// Test conflict detection: existing profile triggers ExistingProfile conflict.
#[test]
fn test_detect_conflicts_existing_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "ExistingRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create import with same rider_id but different name
    let json = create_test_export_json(rider_id, "DifferentName", Some(275), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    // Should detect ExistingProfile conflict
    assert!(!conflicts.is_empty(), "Should detect conflicts");
    let has_existing = conflicts
        .iter()
        .any(|c| matches!(c, ProfileConflict::ExistingProfile { .. }));
    assert!(has_existing, "Should have ExistingProfile conflict");
}

/// Test conflict detection: display name mismatch detected.
#[test]
fn test_detect_conflicts_display_name_mismatch() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "OldName".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create import with different display name
    let json = create_test_export_json(rider_id, "NewName", Some(250), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    // Should detect DisplayNameMismatch
    let has_name_mismatch = conflicts.iter().any(|c| {
        matches!(c, ProfileConflict::DisplayNameMismatch {
            imported_name,
            existing_name
        } if imported_name == "NewName" && existing_name == "OldName")
    });
    assert!(
        has_name_mismatch,
        "Should detect DisplayNameMismatch conflict"
    );
}

/// Test conflict detection: FTP mismatch detected.
#[test]
fn test_detect_conflicts_ftp_mismatch() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile with FTP 250
    let rider = Rider {
        id: rider_id,
        display_name: "FTPRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create import with different FTP (300)
    let json = create_test_export_json(rider_id, "FTPRider", Some(300), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    // Should detect FtpMismatch
    let has_ftp_mismatch = conflicts.iter().any(|c| {
        matches!(
            c,
            ProfileConflict::FtpMismatch {
                imported_ftp: Some(300),
                existing_ftp: Some(250)
            }
        )
    });
    assert!(has_ftp_mismatch, "Should detect FtpMismatch conflict");
}

/// Test conflict detection: avatar mismatch when import has avatar but existing doesn't.
#[test]
fn test_detect_conflicts_avatar_mismatch() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile without avatar
    let rider = Rider {
        id: rider_id,
        display_name: "NoAvatarRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create import WITH avatar
    let avatar = ("#FF0000", "road_bike", None, None);
    let json = create_test_export_json(rider_id, "NoAvatarRider", Some(250), vec![], Some(avatar));

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    // Should detect AvatarMismatch
    let has_avatar_mismatch = conflicts.iter().any(|c| {
        matches!(
            c,
            ProfileConflict::AvatarMismatch {
                import_has_avatar: true,
                existing_has_avatar: false
            }
        )
    });
    assert!(has_avatar_mismatch, "Should detect AvatarMismatch conflict");
}

/// Test conflict detection: no conflicts when values match.
#[test]
fn test_detect_conflicts_no_conflict_when_matching() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "MatchingRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(275),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create import with matching name and FTP (no avatar on either side)
    let json = create_test_export_json(rider_id, "MatchingRider", Some(275), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    // Should still have ExistingProfile conflict (profile exists)
    let has_existing = conflicts
        .iter()
        .any(|c| matches!(c, ProfileConflict::ExistingProfile { .. }));
    assert!(has_existing, "Should have ExistingProfile conflict");

    // But should NOT have name/FTP/avatar mismatches
    let has_name_mismatch = conflicts
        .iter()
        .any(|c| matches!(c, ProfileConflict::DisplayNameMismatch { .. }));
    let has_ftp_mismatch = conflicts
        .iter()
        .any(|c| matches!(c, ProfileConflict::FtpMismatch { .. }));
    let has_avatar_mismatch = conflicts
        .iter()
        .any(|c| matches!(c, ProfileConflict::AvatarMismatch { .. }));

    assert!(!has_name_mismatch, "Should not have DisplayNameMismatch");
    assert!(!has_ftp_mismatch, "Should not have FtpMismatch");
    assert!(!has_avatar_mismatch, "Should not have AvatarMismatch");
}

/// Test conflict detection: no conflicts for new profile (empty DB).
#[test]
fn test_detect_conflicts_no_conflict_for_new_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    // No existing profile - import to empty DB
    let json = create_test_export_json(rider_id, "BrandNewRider", Some(280), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let conflicts = exporter
        .detect_conflicts(&export)
        .expect("Conflict detection should succeed");

    assert!(
        conflicts.is_empty(),
        "Should have no conflicts for new profile"
    );
}

// =============================================================================
// Merge Strategy Tests
// =============================================================================

/// Test merge strategy: updates existing profile data.
#[test]
fn test_merge_strategy_updates_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "OldName".to_string(),
        avatar_id: None,
        bio: Some("Old bio".to_string()),
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Import with Merge strategy - updates profile
    let json = create_test_export_json(rider_id, "NewName", Some(300), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);

    // Verify profile was updated
    let profile = query_profile_from_db(&db, rider_id);
    let (name, ftp, sharing) = profile.unwrap();
    assert_eq!(name, "NewName", "Name should be updated");
    assert_eq!(ftp, Some(300), "FTP should be updated");
    assert!(sharing, "Sharing should be updated to true");
}

/// Test merge strategy: combines FTP history without duplicates.
#[test]
fn test_merge_strategy_combines_ftp_history() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "FTPRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(265),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Add existing FTP history (2 entries)
    insert_test_ftp_estimate(
        &db,
        rider_id,
        250,
        "ramp_test",
        "high",
        "2024-01-15T10:00:00Z",
        true,
    );
    insert_test_ftp_estimate(
        &db,
        rider_id,
        265,
        "20min_test",
        "high",
        "2024-03-20T14:30:00Z",
        true,
    );

    // Import with additional FTP history (1 new, 1 duplicate timestamp)
    let ftp_history = vec![
        ("2024-03-20T14:30:00Z", 265u16, "20min_test", "high", true), // Duplicate
        ("2024-06-10T08:00:00Z", 280, "manual", "medium", false),     // New
    ];
    let json = create_test_export_json(rider_id, "FTPRider", Some(280), ftp_history, None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert_eq!(result.ftp_entries_imported, 1, "Should import 1 new entry");
    assert_eq!(result.ftp_entries_skipped, 1, "Should skip 1 duplicate");

    // Verify total FTP entries (2 existing + 1 new = 3)
    let count = count_ftp_entries(&db, rider_id);
    assert_eq!(count, 3, "Should have 3 total FTP entries after merge");
}

/// Test merge strategy: adds avatar to profile without one.
#[test]
fn test_merge_strategy_adds_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile without avatar
    let rider = Rider {
        id: rider_id,
        display_name: "NoAvatarRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(260),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Verify no avatar exists
    assert!(query_avatar_from_db(&db, rider_id).is_none());

    // Import with avatar using Merge
    let avatar = ("#0000FF", "gravel", Some("#FFFF00"), None);
    let json = create_test_export_json(rider_id, "NoAvatarRider", Some(260), vec![], Some(avatar));

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.avatar_updated, "Avatar should be added");

    // Verify avatar was created
    let avatar_data = query_avatar_from_db(&db, rider_id);
    assert!(avatar_data.is_some(), "Avatar should exist now");
    let (jersey, bike) = avatar_data.unwrap();
    assert_eq!(jersey, "#0000FF");
    assert_eq!(bike, "gravel");
}

/// Test merge strategy: updates existing avatar.
#[test]
fn test_merge_strategy_updates_existing_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile with avatar
    let rider = Rider {
        id: rider_id,
        display_name: "AvatarRider".to_string(),
        avatar_id: Some("avatar_id".to_string()),
        bio: None,
        ftp: Some(270),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create existing avatar
    let existing_avatar = AvatarConfig {
        jersey_color: [255, 0, 0], // Red
        bike_style: BikeStyle::Road,
        jersey_secondary: None,
        helmet_color: None,
    };
    insert_test_avatar(&db, rider_id, &existing_avatar);

    // Verify existing avatar
    let (jersey, _) = query_avatar_from_db(&db, rider_id).unwrap();
    assert!(
        jersey.contains("FF") || jersey.contains("ff"),
        "Should have red jersey"
    );

    // Import with different avatar using Merge
    let new_avatar = ("#00FF00", "tt_bike", Some("#000000"), Some("#FFFFFF")); // Green TT bike
    let json =
        create_test_export_json(rider_id, "AvatarRider", Some(270), vec![], Some(new_avatar));

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.avatar_updated, "Avatar should be updated");

    // Verify avatar was updated to green
    let (jersey, bike) = query_avatar_from_db(&db, rider_id).unwrap();
    assert_eq!(jersey, "#00FF00", "Jersey should be updated to green");
    assert_eq!(bike, "tt_bike", "Bike style should be updated");
}

// =============================================================================
// Replace Strategy Tests
// =============================================================================

/// Test replace strategy: overwrites existing profile completely.
#[test]
fn test_replace_strategy_overwrites_profile() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "OldProfile".to_string(),
        avatar_id: None,
        bio: Some("Old bio that should be replaced".to_string()),
        ftp: Some(220),
        total_distance_km: 5000.0,
        total_time_hours: 250.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Import with Replace strategy
    let json = create_test_export_json(rider_id, "NewProfile", Some(310), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Replace)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);

    // Verify profile was completely replaced
    let profile = query_profile_from_db(&db, rider_id);
    let (name, ftp, sharing) = profile.unwrap();
    assert_eq!(name, "NewProfile", "Name should be replaced");
    assert_eq!(ftp, Some(310), "FTP should be replaced");
    assert!(sharing, "Sharing should be replaced (now true)");
}

/// Test replace strategy: deletes all existing FTP history and imports new.
#[test]
fn test_replace_strategy_replaces_ftp_history() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "FTPRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Add existing FTP history (3 entries)
    insert_test_ftp_estimate(
        &db,
        rider_id,
        230,
        "ramp_test",
        "high",
        "2023-06-01T12:00:00Z",
        true,
    );
    insert_test_ftp_estimate(
        &db,
        rider_id,
        240,
        "20min_test",
        "high",
        "2023-09-15T12:00:00Z",
        true,
    );
    insert_test_ftp_estimate(
        &db,
        rider_id,
        250,
        "ramp_test",
        "high",
        "2024-01-10T12:00:00Z",
        true,
    );

    // Verify existing count
    assert_eq!(count_ftp_entries(&db, rider_id), 3);

    // Import with Replace strategy (only 2 new entries)
    let ftp_history = vec![
        ("2024-06-01T12:00:00Z", 290u16, "20min_test", "high", true),
        ("2024-07-15T12:00:00Z", 300, "manual", "medium", false),
    ];
    let json = create_test_export_json(rider_id, "FTPRider", Some(300), ftp_history, None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Replace)
        .expect("Import should succeed");

    assert!(result.success);
    assert_eq!(
        result.ftp_entries_imported, 2,
        "Should import 2 new entries"
    );
    assert_eq!(
        result.ftp_entries_skipped, 0,
        "No entries skipped in Replace"
    );

    // Verify only new FTP entries exist (old ones deleted)
    let count = count_ftp_entries(&db, rider_id);
    assert_eq!(count, 2, "Should have only 2 FTP entries after replace");
}

/// Test replace strategy: replaces avatar completely.
#[test]
fn test_replace_strategy_replaces_avatar() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile with avatar
    let rider = Rider {
        id: rider_id,
        display_name: "AvatarRider".to_string(),
        avatar_id: Some("old_avatar".to_string()),
        bio: None,
        ftp: Some(270),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create existing avatar (red road bike)
    let existing_avatar = AvatarConfig {
        jersey_color: [255, 0, 0],
        bike_style: BikeStyle::Road,
        jersey_secondary: Some([255, 255, 255]),
        helmet_color: Some([0, 0, 0]),
    };
    insert_test_avatar(&db, rider_id, &existing_avatar);

    // Import with Replace strategy and different avatar
    let new_avatar = ("#00FF00", "tt_bike", None, Some("#888888")); // Green TT bike
    let json =
        create_test_export_json(rider_id, "AvatarRider", Some(270), vec![], Some(new_avatar));

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Replace)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.avatar_updated);

    // Verify avatar was replaced
    let (jersey, bike) = query_avatar_from_db(&db, rider_id).unwrap();
    assert_eq!(
        jersey, "#00FF00",
        "Avatar jersey should be replaced to green"
    );
    assert_eq!(bike, "tt_bike", "Avatar bike style should be replaced");
}

/// Test replace strategy: removes avatar when import has none.
#[test]
fn test_replace_strategy_removes_avatar_when_import_has_none() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile with avatar
    let rider = Rider {
        id: rider_id,
        display_name: "AvatarRider".to_string(),
        avatar_id: Some("old_avatar".to_string()),
        bio: None,
        ftp: Some(270),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Create existing avatar
    let existing_avatar = AvatarConfig {
        jersey_color: [255, 0, 0],
        bike_style: BikeStyle::Road,
        jersey_secondary: None,
        helmet_color: None,
    };
    insert_test_avatar(&db, rider_id, &existing_avatar);

    // Verify avatar exists
    assert!(query_avatar_from_db(&db, rider_id).is_some());

    // Import with Replace strategy but NO avatar
    let json = create_test_export_json(rider_id, "AvatarRider", Some(270), vec![], None);

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Replace)
        .expect("Import should succeed");

    assert!(result.success);
    // avatar_updated is false since we didn't add a new one
    assert!(!result.avatar_updated);

    // Avatar should be deleted
    assert!(
        query_avatar_from_db(&db, rider_id).is_none(),
        "Avatar should be removed after replace with no avatar"
    );
}

// =============================================================================
// Skip Strategy Tests
// =============================================================================

/// Test skip strategy: no changes made to database.
#[test]
fn test_skip_strategy_makes_no_changes() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create existing profile
    let rider = Rider {
        id: rider_id,
        display_name: "OriginalRider".to_string(),
        avatar_id: None,
        bio: Some("Original bio".to_string()),
        ftp: Some(250),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&db, &rider);

    // Add existing FTP history
    insert_test_ftp_estimate(
        &db,
        rider_id,
        250,
        "ramp_test",
        "high",
        "2024-01-15T10:00:00Z",
        true,
    );

    // Import with Skip strategy (completely different data)
    let ftp_history = vec![("2024-06-01T12:00:00Z", 350u16, "manual", "low", false)];
    let avatar = ("#FF0000", "road_bike", None, None);
    let json = create_test_export_json(
        rider_id,
        "DifferentName",
        Some(350),
        ftp_history,
        Some(avatar),
    );

    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let export = exporter.parse_import(&json).expect("Parse should succeed");
    let result = exporter
        .import_profile(&export, ConflictResolution::Skip)
        .expect("Import should succeed");

    assert!(result.success, "Skip should succeed");
    assert!(!result.profile_updated, "Profile should not be updated");
    assert!(!result.avatar_updated, "Avatar should not be updated");
    assert_eq!(
        result.ftp_entries_imported, 0,
        "No FTP entries should be imported"
    );
    assert_eq!(
        result.ftp_entries_skipped, 0,
        "No FTP entries tracked as skipped"
    );

    // Verify original data unchanged
    let profile = query_profile_from_db(&db, rider_id);
    let (name, ftp, sharing) = profile.unwrap();
    assert_eq!(name, "OriginalRider", "Name should be unchanged");
    assert_eq!(ftp, Some(250), "FTP should be unchanged");
    assert!(!sharing, "Sharing should be unchanged");

    // Verify FTP history unchanged
    let count = count_ftp_entries(&db, rider_id);
    assert_eq!(count, 1, "FTP history should be unchanged");

    // Verify no avatar was added
    assert!(
        query_avatar_from_db(&db, rider_id).is_none(),
        "No avatar should exist"
    );
}

// =============================================================================
// Import from File Tests
// =============================================================================

/// Test import_from_file with valid JSON file.
#[test]
fn test_import_from_file_valid_json() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    // Create temp file with valid export JSON
    let temp_dir = std::env::temp_dir().join(format!("import_test_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Create temp dir");
    let import_path = temp_dir.join("import_test.json");

    let ftp_history = vec![("2024-06-01T12:00:00Z", 275u16, "ramp_test", "high", true)];
    let json = create_test_export_json(rider_id, "FileImportRider", Some(275), ftp_history, None);
    std::fs::write(&import_path, &json).expect("Write test file");

    // Import from file
    let exporter = ProfileExporter::new(Arc::new(db.clone()));
    let result = exporter
        .import_from_file(&import_path, ConflictResolution::Merge)
        .expect("Import from file should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert_eq!(result.ftp_entries_imported, 1);

    // Verify data was imported
    let profile = query_profile_from_db(&db, rider_id);
    assert!(profile.is_some());
    let (name, _, _) = profile.unwrap();
    assert_eq!(name, "FileImportRider");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Test import_from_file with non-existent file returns error.
#[test]
fn test_import_from_file_nonexistent() {
    let db = create_test_database();
    let exporter = ProfileExporter::new(Arc::new(db));

    let result =
        exporter.import_from_file("/nonexistent/path/import.json", ConflictResolution::Merge);

    assert!(result.is_err(), "Import from non-existent file should fail");
    let error = format!("{:?}", result.unwrap_err());
    assert!(error.contains("IoError"), "Error should be IoError");
}

/// Test import_from_file with invalid JSON returns parse error.
#[test]
fn test_import_from_file_invalid_json() {
    let db = create_test_database();

    // Create temp file with invalid JSON
    let temp_dir = std::env::temp_dir().join(format!("import_invalid_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Create temp dir");
    let import_path = temp_dir.join("invalid.json");
    std::fs::write(&import_path, "{ invalid json }").expect("Write test file");

    let exporter = ProfileExporter::new(Arc::new(db));
    let result = exporter.import_from_file(&import_path, ConflictResolution::Merge);

    assert!(result.is_err(), "Import of invalid JSON should fail");
    let error = format!("{:?}", result.unwrap_err());
    assert!(error.contains("ParseError"), "Error should be ParseError");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Test import_from_file with wrong version returns version error.
#[test]
fn test_import_from_file_wrong_version() {
    let db = create_test_database();
    let rider_id = Uuid::new_v4();

    // Create temp file with wrong version
    let temp_dir = std::env::temp_dir().join(format!("import_version_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Create temp dir");
    let import_path = temp_dir.join("wrong_version.json");

    let json = format!(
        r#"{{
  "export_version": "99.0",
  "exported_at": "2024-06-15T12:00:00Z",
  "rider_id": "{}",
  "profile": {{
    "display_name": "VersionTest",
    "bio": null,
    "ftp": null,
    "total_distance_km": 0.0,
    "total_time_hours": 0.0,
    "sharing_enabled": true
  }},
  "ftp_history": [],
  "avatar": null
}}"#,
        rider_id
    );
    std::fs::write(&import_path, &json).expect("Write test file");

    let exporter = ProfileExporter::new(Arc::new(db));
    let result = exporter.import_from_file(&import_path, ConflictResolution::Merge);

    assert!(result.is_err(), "Import with wrong version should fail");
    let error = format!("{:?}", result.unwrap_err());
    assert!(
        error.contains("InvalidVersion"),
        "Error should be InvalidVersion"
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

// =============================================================================
// Round-Trip Export/Import Tests (T025)
// =============================================================================

/// Helper to query full FTP history from database, ordered by detected_at DESC.
fn query_ftp_history_from_db(
    db: &Database,
    rider_id: Uuid,
) -> Vec<(u16, String, String, String, bool)> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare(
            "SELECT ftp_watts, method, confidence, detected_at, accepted
             FROM ftp_estimates
             WHERE user_id = ?1
             ORDER BY detected_at DESC",
        )
        .expect("Prepare FTP history query");

    let rows = stmt
        .query_map([rider_id.to_string()], |row| {
            Ok((
                row.get::<_, u16>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .expect("Query FTP history");

    rows.filter_map(|r| r.ok()).collect()
}

/// Helper to query full profile data from database.
fn query_full_profile_from_db(
    db: &Database,
    rider_id: Uuid,
) -> Option<(String, Option<String>, Option<u16>, f64, f64, bool)> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare(
            "SELECT display_name, bio, ftp, total_distance_km, total_time_hours, sharing_enabled
             FROM riders WHERE id = ?1",
        )
        .ok()?;

    stmt.query_row([rider_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<u16>>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, bool>(5)?,
        ))
    })
    .ok()
}

/// Helper to query full avatar configuration from database.
fn query_full_avatar_from_db(
    db: &Database,
    rider_id: Uuid,
) -> Option<(String, String, Option<String>, Option<String>)> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare(
            "SELECT jersey_color, bike_style, jersey_secondary, helmet_color
             FROM avatars WHERE user_id = ?1",
        )
        .ok()?;

    stmt.query_row([rider_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })
    .ok()
}

/// Test round-trip: export complete profile, import to fresh DB, verify all data matches.
#[test]
fn test_roundtrip_complete_profile() {
    // Source database with complete profile
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create complete rider profile
    let rider = Rider {
        id: rider_id,
        display_name: "RoundTripRider".to_string(),
        avatar_id: Some("avatar_123".to_string()),
        bio: Some("Complete profile for round-trip testing.".to_string()),
        ftp: Some(285),
        total_distance_km: 5432.1,
        total_time_hours: 271.5,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Add FTP history entries with different timestamps
    let date1 = Utc
        .with_ymd_and_hms(2024, 1, 15, 10, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 4, 20, 14, 30, 0)
        .unwrap()
        .to_rfc3339();
    let date3 = Utc
        .with_ymd_and_hms(2024, 7, 10, 8, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date4 = Utc
        .with_ymd_and_hms(2024, 10, 5, 16, 15, 0)
        .unwrap()
        .to_rfc3339();

    insert_test_ftp_estimate(&source_db, rider_id, 250, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(
        &source_db,
        rider_id,
        265,
        "20min_test",
        "high",
        &date2,
        true,
    );
    insert_test_ftp_estimate(
        &source_db,
        rider_id,
        275,
        "ramp_test",
        "medium",
        &date3,
        false,
    );
    insert_test_ftp_estimate(&source_db, rider_id, 285, "manual", "high", &date4, true);

    // Add avatar configuration
    let avatar_config = AvatarConfig {
        jersey_color: [0, 128, 255], // Blue
        bike_style: BikeStyle::TT,
        jersey_secondary: Some([255, 255, 0]), // Yellow
        helmet_color: Some([64, 64, 64]),      // Dark gray
    };
    insert_test_avatar(&source_db, rider_id, &avatar_config);

    // Export from source database
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    // Verify export has all expected data
    assert_eq!(export.rider_id, rider_id);
    assert_eq!(export.profile.display_name, "RoundTripRider");
    assert_eq!(
        export.profile.bio,
        Some("Complete profile for round-trip testing.".to_string())
    );
    assert_eq!(export.profile.ftp, Some(285));
    assert!((export.profile.total_distance_km - 5432.1).abs() < 0.01);
    assert!((export.profile.total_time_hours - 271.5).abs() < 0.01);
    assert!(export.profile.sharing_enabled);
    assert_eq!(export.ftp_history.len(), 4);
    assert!(export.avatar.is_some());

    // Create fresh destination database
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    // Import to destination database
    let result = dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert!(result.avatar_updated);
    assert_eq!(result.ftp_entries_imported, 4);
    assert_eq!(result.ftp_entries_skipped, 0);

    // Verify profile data matches original
    let imported_profile = query_full_profile_from_db(&dest_db, rider_id);
    assert!(
        imported_profile.is_some(),
        "Profile should exist in destination DB"
    );
    let (name, bio, ftp, distance, time, sharing) = imported_profile.unwrap();
    assert_eq!(name, "RoundTripRider");
    assert_eq!(
        bio,
        Some("Complete profile for round-trip testing.".to_string())
    );
    assert_eq!(ftp, Some(285));
    assert!((distance - 5432.1).abs() < 0.01);
    assert!((time - 271.5).abs() < 0.01);
    assert!(sharing);

    // Verify avatar matches
    let imported_avatar = query_full_avatar_from_db(&dest_db, rider_id);
    assert!(
        imported_avatar.is_some(),
        "Avatar should exist in destination DB"
    );
    let (jersey, bike_style, secondary, helmet) = imported_avatar.unwrap();
    // Colors are stored as hex strings in the export format
    assert!(!jersey.is_empty());
    assert_eq!(bike_style, "tt_bike");
    assert!(secondary.is_some());
    assert!(helmet.is_some());

    // Verify FTP history count
    let ftp_count = count_ftp_entries(&dest_db, rider_id);
    assert_eq!(ftp_count, 4, "Should have all 4 FTP entries");
}

/// Test round-trip preserves FTP history ordering (most recent first).
#[test]
fn test_roundtrip_ftp_history_ordering_preserved() {
    // Source database
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create rider profile
    let rider = Rider {
        id: rider_id,
        display_name: "OrderTestRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(300),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Add FTP history in non-chronological order (to test sorting)
    let jan = Utc
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let mar = Utc
        .with_ymd_and_hms(2024, 3, 15, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let jun = Utc
        .with_ymd_and_hms(2024, 6, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let sep = Utc
        .with_ymd_and_hms(2024, 9, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let dec = Utc
        .with_ymd_and_hms(2024, 12, 15, 12, 0, 0)
        .unwrap()
        .to_rfc3339();

    // Insert in random order
    insert_test_ftp_estimate(&source_db, rider_id, 280, "ramp_test", "high", &jun, true);
    insert_test_ftp_estimate(&source_db, rider_id, 250, "20min_test", "high", &jan, true);
    insert_test_ftp_estimate(&source_db, rider_id, 300, "manual", "medium", &dec, true);
    insert_test_ftp_estimate(&source_db, rider_id, 265, "ramp_test", "high", &mar, true);
    insert_test_ftp_estimate(&source_db, rider_id, 290, "20min_test", "high", &sep, false);

    // Export from source
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    // Verify export FTP history is ordered by detected_at DESC (most recent first)
    assert_eq!(export.ftp_history.len(), 5);
    assert_eq!(
        export.ftp_history[0].ftp_watts, 300,
        "December (most recent) should be first"
    );
    assert_eq!(
        export.ftp_history[1].ftp_watts, 290,
        "September should be second"
    );
    assert_eq!(export.ftp_history[2].ftp_watts, 280, "June should be third");
    assert_eq!(
        export.ftp_history[3].ftp_watts, 265,
        "March should be fourth"
    );
    assert_eq!(
        export.ftp_history[4].ftp_watts, 250,
        "January (oldest) should be last"
    );

    // Create fresh destination database and import
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    let result = dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert_eq!(result.ftp_entries_imported, 5);

    // Query FTP history from destination and verify ordering
    let imported_history = query_ftp_history_from_db(&dest_db, rider_id);
    assert_eq!(imported_history.len(), 5);

    // Should be ordered DESC by detected_at
    assert_eq!(
        imported_history[0].0, 300,
        "December should be first after import"
    );
    assert_eq!(
        imported_history[1].0, 290,
        "September should be second after import"
    );
    assert_eq!(
        imported_history[2].0, 280,
        "June should be third after import"
    );
    assert_eq!(
        imported_history[3].0, 265,
        "March should be fourth after import"
    );
    assert_eq!(
        imported_history[4].0, 250,
        "January should be last after import"
    );
}

/// Test round-trip with JSON serialization/deserialization (file-based).
#[test]
fn test_roundtrip_via_json_file() {
    // Source database
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "JSONRoundTrip".to_string(),
        avatar_id: Some("avatar_json".to_string()),
        bio: Some("Testing JSON file round-trip".to_string()),
        ftp: Some(275),
        total_distance_km: 2500.0,
        total_time_hours: 125.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Add FTP history
    let date1 = Utc
        .with_ymd_and_hms(2024, 2, 1, 10, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 5, 15, 14, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date3 = Utc
        .with_ymd_and_hms(2024, 8, 20, 16, 30, 0)
        .unwrap()
        .to_rfc3339();

    insert_test_ftp_estimate(&source_db, rider_id, 260, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(
        &source_db,
        rider_id,
        270,
        "20min_test",
        "medium",
        &date2,
        true,
    );
    insert_test_ftp_estimate(&source_db, rider_id, 275, "manual", "low", &date3, false);

    // Add avatar
    let avatar_config = AvatarConfig {
        jersey_color: [200, 100, 50],
        bike_style: BikeStyle::Gravel,
        jersey_secondary: None,
        helmet_color: Some([0, 0, 0]),
    };
    insert_test_avatar(&source_db, rider_id, &avatar_config);

    // Create temp directory for file-based round-trip
    let temp_dir = std::env::temp_dir().join(format!("roundtrip_json_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Create temp dir");
    let export_path = temp_dir.join("profile_export.json");

    // Export to file
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    source_exporter
        .export_to_file(rider_id, &export_path)
        .expect("Export to file should succeed");

    // Verify file exists and is valid JSON
    assert!(export_path.exists());
    let content = std::fs::read_to_string(&export_path).expect("Read export file");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Valid JSON");
    assert_eq!(
        parsed
            .get("profile")
            .unwrap()
            .get("display_name")
            .and_then(|v| v.as_str()),
        Some("JSONRoundTrip")
    );

    // Import to fresh database from file
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    let result = dest_exporter
        .import_from_file(&export_path, ConflictResolution::Merge)
        .expect("Import from file should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert!(result.avatar_updated);
    assert_eq!(result.ftp_entries_imported, 3);

    // Verify imported data
    let imported_profile = query_full_profile_from_db(&dest_db, rider_id);
    let (name, bio, ftp, distance, time, sharing) = imported_profile.unwrap();
    assert_eq!(name, "JSONRoundTrip");
    assert_eq!(bio, Some("Testing JSON file round-trip".to_string()));
    assert_eq!(ftp, Some(275));
    assert!((distance - 2500.0).abs() < 0.01);
    assert!((time - 125.0).abs() < 0.01);
    assert!(!sharing);

    // Verify avatar
    let avatar = query_full_avatar_from_db(&dest_db, rider_id);
    assert!(avatar.is_some());
    let (_, bike, _, _) = avatar.unwrap();
    assert_eq!(bike, "gravel");

    // Verify FTP history ordered correctly
    let history = query_ftp_history_from_db(&dest_db, rider_id);
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].0, 275, "Most recent should be first");
    assert_eq!(history[2].0, 260, "Oldest should be last");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Test round-trip with minimal profile (no FTP history, no avatar).
#[test]
fn test_roundtrip_minimal_profile() {
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create minimal profile
    let rider = Rider {
        id: rider_id,
        display_name: "MinimalRoundTrip".to_string(),
        avatar_id: None,
        bio: None,
        ftp: None,
        total_distance_km: 0.0,
        total_time_hours: 0.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // No FTP history, no avatar

    // Export
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    assert_eq!(export.profile.display_name, "MinimalRoundTrip");
    assert!(export.profile.bio.is_none());
    assert!(export.profile.ftp.is_none());
    assert!(export.ftp_history.is_empty());
    assert!(export.avatar.is_none());

    // Import to fresh database
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    let result = dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert!(!result.avatar_updated);
    assert_eq!(result.ftp_entries_imported, 0);

    // Verify imported data
    let profile = query_full_profile_from_db(&dest_db, rider_id);
    let (name, bio, ftp, distance, time, sharing) = profile.unwrap();
    assert_eq!(name, "MinimalRoundTrip");
    assert!(bio.is_none());
    assert!(ftp.is_none());
    assert!((distance - 0.0).abs() < 0.01);
    assert!((time - 0.0).abs() < 0.01);
    assert!(!sharing);

    // Verify no avatar or FTP history
    assert!(query_full_avatar_from_db(&dest_db, rider_id).is_none());
    assert_eq!(count_ftp_entries(&dest_db, rider_id), 0);
}

/// Test round-trip preserves FTP entry attributes (method, confidence, accepted).
#[test]
fn test_roundtrip_ftp_entry_attributes_preserved() {
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "FTPAttributeTest".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(290),
        total_distance_km: 500.0,
        total_time_hours: 25.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Add FTP entries with various attributes
    let date1 = Utc
        .with_ymd_and_hms(2024, 3, 1, 9, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 6, 1, 15, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date3 = Utc
        .with_ymd_and_hms(2024, 9, 1, 11, 30, 0)
        .unwrap()
        .to_rfc3339();

    insert_test_ftp_estimate(&source_db, rider_id, 270, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(
        &source_db,
        rider_id,
        280,
        "20min_test",
        "medium",
        &date2,
        false,
    );
    insert_test_ftp_estimate(&source_db, rider_id, 290, "manual", "low", &date3, true);

    // Export
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    // Import to fresh database
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    // Query and verify all FTP entry attributes
    let history = query_ftp_history_from_db(&dest_db, rider_id);
    assert_eq!(history.len(), 3);

    // Most recent entry (290, manual, low, accepted)
    let (watts0, method0, conf0, _, accepted0) = &history[0];
    assert_eq!(*watts0, 290);
    assert_eq!(method0, "manual");
    assert_eq!(conf0, "low");
    assert!(*accepted0);

    // Middle entry (280, 20min_test, medium, not accepted)
    let (watts1, method1, conf1, _, accepted1) = &history[1];
    assert_eq!(*watts1, 280);
    assert_eq!(method1, "20min_test");
    assert_eq!(conf1, "medium");
    assert!(!*accepted1);

    // Oldest entry (270, ramp_test, high, accepted)
    let (watts2, method2, conf2, _, accepted2) = &history[2];
    assert_eq!(*watts2, 270);
    assert_eq!(method2, "ramp_test");
    assert_eq!(conf2, "high");
    assert!(*accepted2);
}

/// Test round-trip with special characters in display name and bio.
#[test]
fn test_roundtrip_special_characters() {
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "Rider\"Quote'Test<>&".to_string(),
        avatar_id: None,
        bio: Some("Bio with\nnewlines\tand\ttabs \"quotes\" <tags> &amps;".to_string()),
        ftp: Some(260),
        total_distance_km: 100.0,
        total_time_hours: 5.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Export
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let json = source_exporter
        .export_json(rider_id)
        .expect("Export should succeed");

    // Import to fresh database from JSON string
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    let export = dest_exporter
        .parse_import(&json)
        .expect("Parse should succeed");
    dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    // Verify special characters preserved
    let profile = query_full_profile_from_db(&dest_db, rider_id);
    let (name, bio, _, _, _, _) = profile.unwrap();
    assert_eq!(name, "Rider\"Quote'Test<>&");
    assert!(bio.as_ref().unwrap().contains('\n'));
    assert!(bio.as_ref().unwrap().contains('\t'));
    assert!(bio.unwrap().contains("\"quotes\""));
}

/// Test round-trip with unicode characters.
#[test]
fn test_roundtrip_unicode_characters() {
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    let rider = Rider {
        id: rider_id,
        display_name: "骑手🚴‍♂️Çÿçlîst".to_string(),
        avatar_id: None,
        bio: Some("Emoji: 💪🏔️ • Accents: àéïõü • 日本語テスト".to_string()),
        ftp: Some(280),
        total_distance_km: 888.8,
        total_time_hours: 44.4,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    // Export
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    // Import to fresh database
    let dest_db = create_test_database();
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));

    dest_exporter
        .import_profile(&export, ConflictResolution::Merge)
        .expect("Import should succeed");

    // Verify unicode preserved
    let profile = query_full_profile_from_db(&dest_db, rider_id);
    let (name, bio, _, _, _, _) = profile.unwrap();
    assert!(name.contains("骑手"));
    assert!(name.contains("🚴"));
    assert!(name.contains("Çÿç"));
    assert!(bio.as_ref().unwrap().contains("💪"));
    assert!(bio.as_ref().unwrap().contains("日本語"));
}

/// Test round-trip with Replace strategy clears and replaces FTP history.
#[test]
fn test_roundtrip_replace_strategy() {
    let source_db = create_test_database();
    let rider_id = Uuid::new_v4();
    let now = Utc::now();

    // Create source profile with specific FTP history
    let rider = Rider {
        id: rider_id,
        display_name: "ReplaceRider".to_string(),
        avatar_id: None,
        bio: None,
        ftp: Some(280),
        total_distance_km: 1000.0,
        total_time_hours: 50.0,
        sharing_enabled: true,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&source_db, &rider);

    let date1 = Utc
        .with_ymd_and_hms(2024, 6, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let date2 = Utc
        .with_ymd_and_hms(2024, 8, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    insert_test_ftp_estimate(&source_db, rider_id, 270, "ramp_test", "high", &date1, true);
    insert_test_ftp_estimate(
        &source_db,
        rider_id,
        280,
        "20min_test",
        "high",
        &date2,
        true,
    );

    // Export
    let source_exporter = ProfileExporter::new(Arc::new(source_db));
    let export = source_exporter
        .build_export(rider_id)
        .expect("Export should succeed");

    // Create destination database with existing (different) profile
    let dest_db = create_test_database();
    let existing_rider = Rider {
        id: rider_id,
        display_name: "OldRider".to_string(),
        avatar_id: None,
        bio: Some("Old bio".to_string()),
        ftp: Some(220),
        total_distance_km: 500.0,
        total_time_hours: 25.0,
        sharing_enabled: false,
        created_at: now,
        updated_at: now,
    };
    insert_test_rider(&dest_db, &existing_rider);

    // Add different FTP history to destination
    let old_date = Utc
        .with_ymd_and_hms(2023, 1, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    insert_test_ftp_estimate(&dest_db, rider_id, 200, "manual", "low", &old_date, false);

    // Import with Replace strategy
    let dest_exporter = ProfileExporter::new(Arc::new(dest_db.clone()));
    let result = dest_exporter
        .import_profile(&export, ConflictResolution::Replace)
        .expect("Import should succeed");

    assert!(result.success);
    assert!(result.profile_updated);
    assert_eq!(result.ftp_entries_imported, 2);

    // Verify profile replaced
    let profile = query_full_profile_from_db(&dest_db, rider_id);
    let (name, _, ftp, _, _, sharing) = profile.unwrap();
    assert_eq!(name, "ReplaceRider");
    assert_eq!(ftp, Some(280));
    assert!(sharing);

    // Verify old FTP history deleted and new imported
    let history = query_ftp_history_from_db(&dest_db, rider_id);
    assert_eq!(
        history.len(),
        2,
        "Should have only imported entries, not old ones"
    );
    assert_eq!(history[0].0, 280);
    assert_eq!(history[1].0, 270);
}
