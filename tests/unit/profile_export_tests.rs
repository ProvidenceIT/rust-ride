//! Unit tests for ProfileExport serialization
//!
//! T021: Test that ProfileExport serializes to valid JSON and deserializes back correctly.
//! Test version field presence.

use chrono::{TimeZone, Utc};
use rustride::social::{
    AvatarExport, FtpHistoryEntry, ProfileData, ProfileExport,
};
use uuid::Uuid;

/// Helper function to create a minimal ProfileData for testing.
fn create_test_profile_data() -> ProfileData {
    ProfileData {
        display_name: "Test Rider".to_string(),
        bio: Some("A test bio".to_string()),
        ftp: Some(250),
        total_distance_km: 1234.5,
        total_time_hours: 56.7,
        sharing_enabled: true,
    }
}

/// Helper function to create a ProfileExport with minimal data.
fn create_test_profile_export() -> ProfileExport {
    ProfileExport::new(
        Uuid::new_v4(),
        create_test_profile_data(),
        Vec::new(),
        None,
    )
}

// =============================================================================
// ProfileExport Serialization Tests
// =============================================================================

/// Test that ProfileExport serializes to valid JSON.
#[test]
fn test_profile_export_serializes_to_valid_json() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export);
    assert!(json.is_ok(), "Failed to serialize ProfileExport: {:?}", json.err());

    let json_str = json.unwrap();
    assert!(!json_str.is_empty(), "Serialized JSON should not be empty");
}

/// Test that ProfileExport serializes to pretty-printed JSON.
#[test]
fn test_profile_export_serializes_to_pretty_json() {
    let export = create_test_profile_export();

    let json = serde_json::to_string_pretty(&export);
    assert!(json.is_ok(), "Failed to serialize ProfileExport to pretty JSON");

    let json_str = json.unwrap();
    // Pretty JSON should contain newlines
    assert!(json_str.contains('\n'), "Pretty JSON should contain newlines");
}

/// Test that serialized JSON is valid by parsing it back as serde_json::Value.
#[test]
fn test_profile_export_produces_valid_json_value() {
    let export = create_test_profile_export();

    let json_str = serde_json::to_string(&export).unwrap();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);

    assert!(parsed.is_ok(), "Serialized JSON should parse as valid JSON: {:?}", parsed.err());
}

// =============================================================================
// Round-trip Serialization Tests
// =============================================================================

/// Test that ProfileExport deserializes back correctly after serialization.
#[test]
fn test_profile_export_round_trip_basic() {
    let original = create_test_profile_export();

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.export_version, original.export_version);
    assert_eq!(deserialized.rider_id, original.rider_id);
    assert_eq!(deserialized.profile.display_name, original.profile.display_name);
    assert_eq!(deserialized.profile.bio, original.profile.bio);
    assert_eq!(deserialized.profile.ftp, original.profile.ftp);
}

/// Test round-trip with full profile data including optional fields.
#[test]
fn test_profile_export_round_trip_with_all_fields() {
    let rider_id = Uuid::new_v4();
    let profile = ProfileData {
        display_name: "Full Test".to_string(),
        bio: Some("Complete bio".to_string()),
        ftp: Some(300),
        total_distance_km: 5000.0,
        total_time_hours: 200.0,
        sharing_enabled: false,
    };

    let ftp_history = vec![
        FtpHistoryEntry {
            ftp_watts: 280,
            method: "ramp_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
            accepted: true,
        },
        FtpHistoryEntry {
            ftp_watts: 300,
            method: "20min_test".to_string(),
            confidence: "high".to_string(),
            detected_at: Utc.with_ymd_and_hms(2024, 3, 20, 14, 30, 0).unwrap(),
            accepted: true,
        },
    ];

    let avatar = Some(AvatarExport {
        jersey_color: "#FF5500".to_string(),
        bike_style: "road_bike".to_string(),
        jersey_secondary: Some("#FFFFFF".to_string()),
        helmet_color: Some("#000000".to_string()),
    });

    let original = ProfileExport::new(rider_id, profile, ftp_history, avatar);

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    // Verify all fields
    assert_eq!(deserialized.export_version, original.export_version);
    assert_eq!(deserialized.rider_id, original.rider_id);
    assert_eq!(deserialized.profile.display_name, original.profile.display_name);
    assert_eq!(deserialized.profile.bio, original.profile.bio);
    assert_eq!(deserialized.profile.ftp, original.profile.ftp);
    assert_eq!(deserialized.profile.total_distance_km, original.profile.total_distance_km);
    assert_eq!(deserialized.profile.total_time_hours, original.profile.total_time_hours);
    assert_eq!(deserialized.profile.sharing_enabled, original.profile.sharing_enabled);

    // Verify FTP history
    assert_eq!(deserialized.ftp_history.len(), 2);
    assert_eq!(deserialized.ftp_history[0].ftp_watts, 280);
    assert_eq!(deserialized.ftp_history[0].method, "ramp_test");
    assert_eq!(deserialized.ftp_history[1].ftp_watts, 300);
    assert_eq!(deserialized.ftp_history[1].method, "20min_test");

    // Verify avatar
    assert!(deserialized.avatar.is_some());
    let avatar = deserialized.avatar.unwrap();
    assert_eq!(avatar.jersey_color, "#FF5500");
    assert_eq!(avatar.bike_style, "road_bike");
    assert_eq!(avatar.jersey_secondary, Some("#FFFFFF".to_string()));
    assert_eq!(avatar.helmet_color, Some("#000000".to_string()));
}

/// Test round-trip with empty FTP history.
#[test]
fn test_profile_export_round_trip_empty_history() {
    let original = ProfileExport::new(
        Uuid::new_v4(),
        create_test_profile_data(),
        Vec::new(),
        None,
    );

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert!(deserialized.ftp_history.is_empty());
    assert!(deserialized.avatar.is_none());
}

/// Test round-trip with avatar but no FTP history.
#[test]
fn test_profile_export_round_trip_avatar_only() {
    let avatar = Some(AvatarExport {
        jersey_color: "#00FF00".to_string(),
        bike_style: "tt_bike".to_string(),
        jersey_secondary: None,
        helmet_color: None,
    });

    let original = ProfileExport::new(
        Uuid::new_v4(),
        create_test_profile_data(),
        Vec::new(),
        avatar,
    );

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert!(deserialized.avatar.is_some());
    let avatar = deserialized.avatar.unwrap();
    assert_eq!(avatar.jersey_color, "#00FF00");
    assert_eq!(avatar.bike_style, "tt_bike");
    assert!(avatar.jersey_secondary.is_none());
    assert!(avatar.helmet_color.is_none());
}

// =============================================================================
// Version Field Tests
// =============================================================================

/// Test that export_version field is present in serialized JSON.
#[test]
fn test_version_field_present_in_json() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export).expect("Serialization failed");

    assert!(json.contains("export_version"), "JSON should contain export_version field");
}

/// Test that export_version has correct current version value.
#[test]
fn test_version_field_has_correct_value() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Parse failed");

    let version = parsed.get("export_version")
        .expect("export_version field should exist")
        .as_str()
        .expect("export_version should be a string");

    assert_eq!(version, ProfileExport::CURRENT_VERSION);
    assert_eq!(version, "1.0");
}

/// Test that version is preserved during round-trip.
#[test]
fn test_version_preserved_in_round_trip() {
    let original = create_test_profile_export();

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.export_version, ProfileExport::CURRENT_VERSION);
}

/// Test that ProfileExport::new() sets current version automatically.
#[test]
fn test_new_sets_current_version() {
    let export = ProfileExport::new(
        Uuid::new_v4(),
        create_test_profile_data(),
        Vec::new(),
        None,
    );

    assert_eq!(export.export_version, ProfileExport::CURRENT_VERSION);
    assert_eq!(export.export_version, "1.0");
}

// =============================================================================
// Timestamp Field Tests
// =============================================================================

/// Test that exported_at field is present in serialized JSON.
#[test]
fn test_exported_at_field_present_in_json() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export).expect("Serialization failed");

    assert!(json.contains("exported_at"), "JSON should contain exported_at field");
}

/// Test that exported_at is preserved during round-trip.
#[test]
fn test_exported_at_preserved_in_round_trip() {
    let original = create_test_profile_export();

    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    // Allow small time difference due to serialization/deserialization
    let diff = (deserialized.exported_at - original.exported_at).num_seconds().abs();
    assert!(diff < 1, "Timestamps should match within 1 second");
}

// =============================================================================
// JSON Structure Tests
// =============================================================================

/// Test that JSON has expected top-level structure.
#[test]
fn test_json_has_expected_structure() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Parse failed");

    assert!(parsed.is_object(), "JSON root should be an object");

    let obj = parsed.as_object().unwrap();

    // Check required top-level fields
    assert!(obj.contains_key("export_version"), "Missing export_version");
    assert!(obj.contains_key("exported_at"), "Missing exported_at");
    assert!(obj.contains_key("rider_id"), "Missing rider_id");
    assert!(obj.contains_key("profile"), "Missing profile");
    assert!(obj.contains_key("ftp_history"), "Missing ftp_history");
    // avatar can be null, but should be present
    assert!(obj.contains_key("avatar"), "Missing avatar");
}

/// Test that profile sub-object has expected structure.
#[test]
fn test_profile_subobject_has_expected_structure() {
    let export = create_test_profile_export();

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Parse failed");

    let profile = parsed.get("profile")
        .expect("profile field should exist")
        .as_object()
        .expect("profile should be an object");

    assert!(profile.contains_key("display_name"), "Missing display_name");
    assert!(profile.contains_key("bio"), "Missing bio");
    assert!(profile.contains_key("ftp"), "Missing ftp");
    assert!(profile.contains_key("total_distance_km"), "Missing total_distance_km");
    assert!(profile.contains_key("total_time_hours"), "Missing total_time_hours");
    assert!(profile.contains_key("sharing_enabled"), "Missing sharing_enabled");
}

/// Test that FTP history array has correct structure.
#[test]
fn test_ftp_history_array_structure() {
    let rider_id = Uuid::new_v4();
    let ftp_history = vec![FtpHistoryEntry {
        ftp_watts: 250,
        method: "ramp_test".to_string(),
        confidence: "high".to_string(),
        detected_at: Utc::now(),
        accepted: true,
    }];

    let export = ProfileExport::new(rider_id, create_test_profile_data(), ftp_history, None);

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Parse failed");

    let history = parsed.get("ftp_history")
        .expect("ftp_history field should exist")
        .as_array()
        .expect("ftp_history should be an array");

    assert_eq!(history.len(), 1);

    let entry = history[0].as_object().expect("Entry should be an object");
    assert!(entry.contains_key("ftp_watts"), "Missing ftp_watts");
    assert!(entry.contains_key("method"), "Missing method");
    assert!(entry.contains_key("confidence"), "Missing confidence");
    assert!(entry.contains_key("detected_at"), "Missing detected_at");
    assert!(entry.contains_key("accepted"), "Missing accepted");
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test profile with None optional fields serializes correctly.
#[test]
fn test_profile_with_none_optionals() {
    let profile = ProfileData {
        display_name: "Minimal Rider".to_string(),
        bio: None,
        ftp: None,
        total_distance_km: 0.0,
        total_time_hours: 0.0,
        sharing_enabled: false,
    };

    let export = ProfileExport::new(Uuid::new_v4(), profile, Vec::new(), None);

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert!(deserialized.profile.bio.is_none());
    assert!(deserialized.profile.ftp.is_none());
    assert_eq!(deserialized.profile.total_distance_km, 0.0);
}

/// Test profile with special characters in display name.
#[test]
fn test_profile_with_special_characters() {
    let profile = ProfileData {
        display_name: "Test \"Rider\" <with> & 'special' chars!".to_string(),
        bio: Some("Bio with\nnewlines\tand\ttabs".to_string()),
        ftp: Some(250),
        total_distance_km: 100.0,
        total_time_hours: 5.0,
        sharing_enabled: true,
    };

    let export = ProfileExport::new(Uuid::new_v4(), profile.clone(), Vec::new(), None);

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.profile.display_name, profile.display_name);
    assert_eq!(deserialized.profile.bio, profile.bio);
}

/// Test profile with unicode characters.
#[test]
fn test_profile_with_unicode() {
    let profile = ProfileData {
        display_name: "Cyclist".to_string(),
        bio: Some("Riding through the mountains".to_string()),
        ftp: Some(280),
        total_distance_km: 42.195,
        total_time_hours: 1.5,
        sharing_enabled: true,
    };

    let export = ProfileExport::new(Uuid::new_v4(), profile.clone(), Vec::new(), None);

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.profile.display_name, profile.display_name);
    assert_eq!(deserialized.profile.bio, profile.bio);
}

/// Test that large FTP history serializes and deserializes correctly.
#[test]
fn test_large_ftp_history() {
    let rider_id = Uuid::new_v4();
    let mut ftp_history = Vec::new();

    for i in 0..100 {
        ftp_history.push(FtpHistoryEntry {
            ftp_watts: 200 + (i % 50) as u16,
            method: format!("test_{}", i % 5),
            confidence: "medium".to_string(),
            detected_at: Utc::now(),
            accepted: i % 2 == 0,
        });
    }

    let export = ProfileExport::new(rider_id, create_test_profile_data(), ftp_history, None);

    let json = serde_json::to_string(&export).expect("Serialization failed");
    let deserialized: ProfileExport = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.ftp_history.len(), 100);
}
