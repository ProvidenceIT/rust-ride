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

// =============================================================================
// Import Parsing Tests (T022)
// =============================================================================

// -----------------------------------------------------------------------------
// Valid Export Parsing Tests
// -----------------------------------------------------------------------------

/// Test parsing a valid export with minimal data.
#[test]
fn test_parse_valid_export_minimal() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Minimal Rider",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse valid minimal export");

    assert_eq!(parsed.export_version, "1.0");
    assert_eq!(parsed.profile.display_name, "Minimal Rider");
    assert!(parsed.profile.bio.is_none());
    assert!(parsed.profile.ftp.is_none());
    assert!(parsed.ftp_history.is_empty());
    assert!(parsed.avatar.is_none());
}

/// Test parsing a valid export with all optional fields present.
#[test]
fn test_parse_valid_export_complete() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-06-15T10:30:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Complete Rider",
            "bio": "Professional cyclist with 10 years experience",
            "ftp": 320,
            "total_distance_km": 50000.5,
            "total_time_hours": 1500.75,
            "sharing_enabled": true
        },
        "ftp_history": [
            {
                "ftp_watts": 280,
                "method": "ramp_test",
                "confidence": "high",
                "detected_at": "2024-01-15T08:00:00Z",
                "accepted": true
            },
            {
                "ftp_watts": 300,
                "method": "20min_test",
                "confidence": "high",
                "detected_at": "2024-03-20T09:30:00Z",
                "accepted": true
            },
            {
                "ftp_watts": 320,
                "method": "ramp_test",
                "confidence": "medium",
                "detected_at": "2024-06-01T07:45:00Z",
                "accepted": true
            }
        ],
        "avatar": {
            "jersey_color": "#FF5500",
            "bike_style": "tt_bike",
            "jersey_secondary": "#FFFFFF",
            "helmet_color": "#000000"
        }
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse valid complete export");

    // Verify profile data
    assert_eq!(parsed.export_version, "1.0");
    assert_eq!(parsed.profile.display_name, "Complete Rider");
    assert_eq!(
        parsed.profile.bio,
        Some("Professional cyclist with 10 years experience".to_string())
    );
    assert_eq!(parsed.profile.ftp, Some(320));
    assert!((parsed.profile.total_distance_km - 50000.5).abs() < f64::EPSILON);
    assert!((parsed.profile.total_time_hours - 1500.75).abs() < f64::EPSILON);
    assert!(parsed.profile.sharing_enabled);

    // Verify FTP history
    assert_eq!(parsed.ftp_history.len(), 3);
    assert_eq!(parsed.ftp_history[0].ftp_watts, 280);
    assert_eq!(parsed.ftp_history[0].method, "ramp_test");
    assert!(parsed.ftp_history[0].accepted);
    assert_eq!(parsed.ftp_history[1].ftp_watts, 300);
    assert_eq!(parsed.ftp_history[2].ftp_watts, 320);

    // Verify avatar
    let avatar = parsed.avatar.expect("Avatar should be present");
    assert_eq!(avatar.jersey_color, "#FF5500");
    assert_eq!(avatar.bike_style, "tt_bike");
    assert_eq!(avatar.jersey_secondary, Some("#FFFFFF".to_string()));
    assert_eq!(avatar.helmet_color, Some("#000000".to_string()));
}

/// Test parsing valid export with avatar having minimal required fields.
#[test]
fn test_parse_valid_export_avatar_minimal() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Rider",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": {
            "jersey_color": "#0000FF",
            "bike_style": "road_bike",
            "jersey_secondary": null,
            "helmet_color": null
        }
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse export with minimal avatar");

    let avatar = parsed.avatar.expect("Avatar should be present");
    assert_eq!(avatar.jersey_color, "#0000FF");
    assert_eq!(avatar.bike_style, "road_bike");
    assert!(avatar.jersey_secondary.is_none());
    assert!(avatar.helmet_color.is_none());
}

/// Test parsing valid export with FTP history that has been rejected.
#[test]
fn test_parse_valid_export_with_rejected_ftp() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Rider",
            "bio": null,
            "ftp": 250,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [
            {
                "ftp_watts": 300,
                "method": "auto_detect",
                "confidence": "low",
                "detected_at": "2024-05-01T10:00:00Z",
                "accepted": false
            },
            {
                "ftp_watts": 250,
                "method": "ramp_test",
                "confidence": "high",
                "detected_at": "2024-06-01T10:00:00Z",
                "accepted": true
            }
        ],
        "avatar": null
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse export with rejected FTP");

    assert_eq!(parsed.ftp_history.len(), 2);
    assert!(!parsed.ftp_history[0].accepted);
    assert_eq!(parsed.ftp_history[0].confidence, "low");
    assert!(parsed.ftp_history[1].accepted);
}

// -----------------------------------------------------------------------------
// Invalid JSON Parsing Tests
// -----------------------------------------------------------------------------

/// Test parsing completely invalid JSON (not even close to JSON format).
#[test]
fn test_parse_invalid_json_garbage() {
    let invalid_json = "this is not json at all";

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse garbage input");

    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(
        error_string.contains("expected") || error_string.contains("EOF") || error_string.contains("value"),
        "Error should indicate parse failure: {}",
        error_string
    );
}

/// Test parsing JSON with unclosed braces.
#[test]
fn test_parse_invalid_json_unclosed_brace() {
    let invalid_json = r#"{ "export_version": "1.0""#;

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse unclosed JSON");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("EOF") || error.to_string().contains("expected"),
        "Error should mention EOF or expected token"
    );
}

/// Test parsing JSON with trailing comma.
#[test]
fn test_parse_invalid_json_trailing_comma() {
    let invalid_json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse JSON with trailing comma");
}

/// Test parsing JSON with unquoted string values.
#[test]
fn test_parse_invalid_json_unquoted_string() {
    let invalid_json = r#"{
        "export_version": 1.0,
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000"
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail when version is number instead of string");
}

/// Test parsing empty JSON object.
#[test]
fn test_parse_invalid_json_empty_object() {
    let invalid_json = "{}";

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse empty object");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing fields"
    );
}

/// Test parsing empty string.
#[test]
fn test_parse_invalid_json_empty_string() {
    let invalid_json = "";

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail to parse empty string");
}

/// Test parsing JSON array instead of object.
#[test]
fn test_parse_invalid_json_array_root() {
    let invalid_json = r#"[{"export_version": "1.0"}]"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail when root is array instead of object");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("expected"),
        "Error should indicate type mismatch: {}",
        error.to_string()
    );
}

/// Test parsing JSON with invalid escape sequences.
#[test]
fn test_parse_invalid_json_bad_escape() {
    let invalid_json = r#"{"export_version": "\x00"}"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "Should fail with invalid escape sequence");
}

// -----------------------------------------------------------------------------
// Incompatible Version Tests
// -----------------------------------------------------------------------------

/// Test that parsing succeeds for wrong version (validation happens separately).
/// This tests the raw parsing behavior - version validation is a separate step.
#[test]
fn test_parse_incompatible_version_future() {
    let json = r#"{
        "export_version": "2.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Future Rider",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    // Raw parsing should succeed - version validation is done by ProfileExporter
    let parsed: ProfileExport = serde_json::from_str(json).expect("Raw parsing should succeed");
    assert_eq!(parsed.export_version, "2.0");

    // Verify this is not the current version
    assert_ne!(parsed.export_version, ProfileExport::CURRENT_VERSION);
}

/// Test parsing with old version format.
#[test]
fn test_parse_incompatible_version_old() {
    let json = r#"{
        "export_version": "0.5",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Old Rider",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Raw parsing should succeed");
    assert_eq!(parsed.export_version, "0.5");
    assert_ne!(parsed.export_version, ProfileExport::CURRENT_VERSION);
}

/// Test parsing with non-semver version format.
#[test]
fn test_parse_incompatible_version_non_semver() {
    let json = r#"{
        "export_version": "version1",
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

    // Parsing succeeds with any string version
    let parsed: ProfileExport = serde_json::from_str(json).expect("Raw parsing should succeed");
    assert_eq!(parsed.export_version, "version1");
}

/// Test parsing with empty version string.
#[test]
fn test_parse_incompatible_version_empty() {
    let json = r#"{
        "export_version": "",
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

    // Parsing succeeds with empty version (validation happens later)
    let parsed: ProfileExport = serde_json::from_str(json).expect("Raw parsing should succeed");
    assert_eq!(parsed.export_version, "");
}

/// Test that CURRENT_VERSION constant is correct.
#[test]
fn test_current_version_value() {
    assert_eq!(ProfileExport::CURRENT_VERSION, "1.0");
}

// -----------------------------------------------------------------------------
// Missing Required Fields Tests
// -----------------------------------------------------------------------------

/// Test parsing JSON missing export_version.
#[test]
fn test_parse_missing_export_version() {
    let json = r#"{
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when export_version is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field") && error.to_string().contains("export_version"),
        "Error should mention missing export_version: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing exported_at.
#[test]
fn test_parse_missing_exported_at() {
    let json = r#"{
        "export_version": "1.0",
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when exported_at is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing field: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing rider_id.
#[test]
fn test_parse_missing_rider_id() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when rider_id is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field") && error.to_string().contains("rider_id"),
        "Error should mention missing rider_id: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing profile.
#[test]
fn test_parse_missing_profile() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when profile is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field") && error.to_string().contains("profile"),
        "Error should mention missing profile: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing ftp_history.
#[test]
fn test_parse_missing_ftp_history() {
    let json = r#"{
        "export_version": "1.0",
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
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when ftp_history is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing field: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing display_name in profile.
#[test]
fn test_parse_missing_profile_display_name() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when display_name is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field") && error.to_string().contains("display_name"),
        "Error should mention missing display_name: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing sharing_enabled in profile.
#[test]
fn test_parse_missing_profile_sharing_enabled() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when sharing_enabled is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing field: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing required avatar fields when avatar is present.
#[test]
fn test_parse_missing_avatar_jersey_color() {
    let json = r#"{
        "export_version": "1.0",
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
        "avatar": {
            "bike_style": "road_bike",
            "jersey_secondary": null,
            "helmet_color": null
        }
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when avatar jersey_color is missing");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing field: {}",
        error.to_string()
    );
}

/// Test parsing JSON missing required FTP history fields.
#[test]
fn test_parse_missing_ftp_history_fields() {
    let json = r#"{
        "export_version": "1.0",
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
        "ftp_history": [
            {
                "ftp_watts": 250,
                "method": "ramp_test"
            }
        ],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when FTP history entry has missing fields");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("missing field"),
        "Error should indicate missing field: {}",
        error.to_string()
    );
}

// -----------------------------------------------------------------------------
// Type Mismatch Tests
// -----------------------------------------------------------------------------

/// Test parsing with wrong type for ftp_watts (string instead of number).
#[test]
fn test_parse_wrong_type_ftp_watts() {
    let json = r#"{
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when ftp is wrong type");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("integer"),
        "Error should indicate type mismatch: {}",
        error.to_string()
    );
}

/// Test parsing with wrong type for sharing_enabled (string instead of bool).
#[test]
fn test_parse_wrong_type_sharing_enabled() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": "yes"
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when sharing_enabled is wrong type");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("boolean"),
        "Error should indicate type mismatch: {}",
        error.to_string()
    );
}

/// Test parsing with wrong type for ftp_history (object instead of array).
#[test]
fn test_parse_wrong_type_ftp_history() {
    let json = r#"{
        "export_version": "1.0",
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
        "ftp_history": {"entry": "wrong"},
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when ftp_history is wrong type");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("sequence"),
        "Error should indicate type mismatch: {}",
        error.to_string()
    );
}

/// Test parsing with invalid UUID format for rider_id.
#[test]
fn test_parse_wrong_type_rider_id() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "not-a-valid-uuid",
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when rider_id is invalid UUID");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("UUID") || error.to_string().contains("invalid"),
        "Error should indicate UUID format error: {}",
        error.to_string()
    );
}

/// Test parsing with invalid datetime format for exported_at.
#[test]
fn test_parse_wrong_type_exported_at() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "not-a-date",
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

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when exported_at is invalid datetime");
}

/// Test parsing with integer instead of float for distances.
#[test]
fn test_parse_integer_for_float_fields() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": null,
            "total_distance_km": 1000,
            "total_time_hours": 50,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    // This should succeed - JSON integers are valid for float fields
    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse integers as floats");
    assert!((parsed.profile.total_distance_km - 1000.0).abs() < f64::EPSILON);
    assert!((parsed.profile.total_time_hours - 50.0).abs() < f64::EPSILON);
}

// -----------------------------------------------------------------------------
// Edge Cases
// -----------------------------------------------------------------------------

/// Test parsing with very long display name.
#[test]
fn test_parse_very_long_display_name() {
    let long_name = "A".repeat(1000);
    let json = format!(
        r#"{{
            "export_version": "1.0",
            "exported_at": "2024-01-01T00:00:00Z",
            "rider_id": "550e8400-e29b-41d4-a716-446655440000",
            "profile": {{
                "display_name": "{}",
                "bio": null,
                "ftp": null,
                "total_distance_km": 0.0,
                "total_time_hours": 0.0,
                "sharing_enabled": false
            }},
            "ftp_history": [],
            "avatar": null
        }}"#,
        long_name
    );

    let parsed: ProfileExport = serde_json::from_str(&json).expect("Should parse long display name");
    assert_eq!(parsed.profile.display_name.len(), 1000);
}

/// Test parsing with very large FTP value.
#[test]
fn test_parse_large_ftp_value() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": 65535,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse max u16 FTP");
    assert_eq!(parsed.profile.ftp, Some(65535));
}

/// Test parsing with FTP value exceeding u16 max.
#[test]
fn test_parse_ftp_overflow() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": 70000,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when FTP exceeds u16 max");
}

/// Test parsing with negative numeric values.
#[test]
fn test_parse_negative_distance() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": "Test",
            "bio": null,
            "ftp": null,
            "total_distance_km": -100.0,
            "total_time_hours": -10.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    // Negative values parse fine (validation is a separate concern)
    let parsed: ProfileExport = serde_json::from_str(json).expect("Should parse negative values");
    assert!(parsed.profile.total_distance_km < 0.0);
    assert!(parsed.profile.total_time_hours < 0.0);
}

/// Test parsing with null for non-optional field.
#[test]
fn test_parse_null_for_required_field() {
    let json = r#"{
        "export_version": "1.0",
        "exported_at": "2024-01-01T00:00:00Z",
        "rider_id": "550e8400-e29b-41d4-a716-446655440000",
        "profile": {
            "display_name": null,
            "bio": null,
            "ftp": null,
            "total_distance_km": 0.0,
            "total_time_hours": 0.0,
            "sharing_enabled": false
        },
        "ftp_history": [],
        "avatar": null
    }"#;

    let result: Result<ProfileExport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail when required field is null");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("invalid type") || error.to_string().contains("null"),
        "Error should indicate null is not allowed: {}",
        error.to_string()
    );
}
