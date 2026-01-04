//! Integration tests for analytics data export.
//!
//! P4.2: Integration test for analytics export functionality.
//!
//! Tests end-to-end export with real database:
//! - Create user with PDC, training load, CP model data
//! - Export to JSON and verify structure
//! - Export to CSV and verify format
//! - Import/export round-trip preserves data

use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use rustride::metrics::analytics::vo2max::{FitnessLevel, Vo2maxMethod, Vo2maxResult};
use rustride::metrics::analytics::{
    AnalyticsExport, AnalyticsExporter, CpModel, DailyLoad, ExportOptions, FtpConfidence,
    FtpEstimate, FtpMethod, PdcPoint, PowerProfile, RiderType,
};
use rustride::storage::analytics_store::AnalyticsStore;
use rustride::storage::Database;

/// Helper to create test database with analytics data.
fn setup_test_database() -> (Arc<Database>, Uuid) {
    let db = Database::open_in_memory().expect("Failed to create database");
    let user_id = Uuid::new_v4();
    let db = Arc::new(db);

    // Save PDC data
    let pdc_points = vec![
        PdcPoint {
            duration_secs: 5,
            power_watts: 900,
        },
        PdcPoint {
            duration_secs: 60,
            power_watts: 450,
        },
        PdcPoint {
            duration_secs: 300,
            power_watts: 350,
        },
        PdcPoint {
            duration_secs: 1200,
            power_watts: 280,
        },
        PdcPoint {
            duration_secs: 3600,
            power_watts: 240,
        },
    ];

    {
        let store = AnalyticsStore::new(db.connection());
        store
            .save_pdc_points(&user_id, &pdc_points, None)
            .expect("Failed to save PDC points");
    }

    // Save CP model
    let cp_model = CpModel {
        cp: 250,
        w_prime: 20000,
        r_squared: 0.98,
    };

    {
        let store = AnalyticsStore::new(db.connection());
        store
            .save_cp_model(&user_id, &cp_model)
            .expect("Failed to save CP model");
    }

    // Save training load history (last 7 days)
    let today = Utc::now().date_naive();
    let daily_loads = [
        (today - chrono::Duration::days(6), 60.0, 45.0, 50.0, 5.0),
        (today - chrono::Duration::days(5), 80.0, 52.0, 51.0, -1.0),
        (today - chrono::Duration::days(4), 45.0, 50.0, 51.0, 1.0),
        (today - chrono::Duration::days(3), 90.0, 58.0, 52.0, -6.0),
        (today - chrono::Duration::days(2), 0.0, 50.0, 52.0, 2.0),
        (today - chrono::Duration::days(1), 70.0, 55.0, 52.0, -3.0),
        (today, 85.0, 62.0, 53.0, -9.0),
    ];

    {
        let store = AnalyticsStore::new(db.connection());
        for (date, tss, atl, ctl, tsb) in daily_loads {
            let load = DailyLoad { tss, atl, ctl, tsb };
            store
                .save_daily_load(&user_id, date, &load)
                .expect("Failed to save daily load");
        }
    }

    // Save FTP estimate and accept it
    let ftp_estimate = FtpEstimate {
        ftp_watts: 260,
        method: FtpMethod::TwentyMinute,
        confidence: FtpConfidence::High,
        supporting_data: vec![(1200, 275)], // 20-min power used for estimate
    };

    {
        let store = AnalyticsStore::new(db.connection());
        let estimate_id = store
            .save_ftp_estimate(&user_id, &ftp_estimate)
            .expect("Failed to save FTP estimate");
        store
            .accept_ftp_estimate(&estimate_id)
            .expect("Failed to accept FTP estimate");
    }

    // Save VO2max result
    let vo2max_result = Vo2maxResult {
        vo2max: 52.5,
        method: Vo2maxMethod::FiveMinutePower,
        classification: FitnessLevel::Trained,
    };

    {
        let store = AnalyticsStore::new(db.connection());
        store
            .save_vo2max(&user_id, &vo2max_result)
            .expect("Failed to save VO2max");
    }

    // Save rider profile
    let power_profile = PowerProfile {
        neuromuscular: 0.85,
        anaerobic: 0.78,
        vo2max: 0.82,
        threshold: 0.80,
    };

    {
        let store = AnalyticsStore::new(db.connection());
        store
            .save_rider_profile(&user_id, RiderType::AllRounder, &power_profile)
            .expect("Failed to save rider profile");
    }

    (db, user_id)
}

#[test]
fn test_export_json_full_structure() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let json = exporter
        .export_json(user_id)
        .expect("Failed to export JSON");

    // Parse and verify structure
    let export: AnalyticsExport =
        serde_json::from_str(&json).expect("Failed to parse exported JSON");

    // Verify metadata
    assert_eq!(export.export_version, "1.0");
    assert_eq!(export.user_id, user_id.to_string());

    // Verify PDC data
    let pdc = export.pdc.expect("PDC should be present");
    assert_eq!(pdc.points.len(), 5);
    assert_eq!(pdc.points[0].duration_secs, 5);
    assert_eq!(pdc.points[0].power_watts, 900);
    assert_eq!(pdc.points[4].duration_secs, 3600);
    assert_eq!(pdc.points[4].power_watts, 240);

    // Verify training load data
    let training_load = export
        .training_load
        .expect("Training load should be present");
    assert_eq!(training_load.days.len(), 7);

    // Verify CP model
    let cp_model = export.cp_model.expect("CP model should be present");
    assert_eq!(cp_model.cp_watts, 250);
    assert_eq!(cp_model.w_prime_joules, 20000);
    assert!((cp_model.r_squared - 0.98).abs() < 0.01);

    // Verify fitness profile
    let fitness = export
        .fitness_profile
        .expect("Fitness profile should be present");
    assert_eq!(fitness.ftp_watts, Some(260));
    assert!(fitness.vo2max.is_some());
    assert!(fitness.rider_type.is_some());
    assert!(fitness.power_profile.is_some());

    let vo2max = fitness.vo2max.unwrap();
    assert!((vo2max.vo2max - 52.5).abs() < 0.1);
    assert_eq!(vo2max.classification, "Trained");
    assert_eq!(vo2max.method, "Five-Minute Power");

    let rider_type = fitness.rider_type.unwrap();
    assert_eq!(rider_type, "All-Rounder");

    let power_profile = fitness.power_profile.unwrap();
    assert!((power_profile.neuromuscular_pct - 0.85).abs() < 0.01);
}

#[test]
fn test_export_pdc_csv_format() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let csv = exporter
        .export_pdc_csv(user_id)
        .expect("Failed to export PDC CSV");

    // Verify headers
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "duration_secs,power_watts,achieved_at");

    // Verify we have data rows (header + 5 data points)
    assert_eq!(lines.len(), 6);

    // Verify first data row (5 second power)
    let first_data: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(first_data[0], "5");
    assert_eq!(first_data[1], "900");

    // Verify last data row (60 minute power)
    let last_data: Vec<&str> = lines[5].split(',').collect();
    assert_eq!(last_data[0], "3600");
    assert_eq!(last_data[1], "240");

    // Verify data is sorted by duration
    let mut prev_duration = 0u32;
    for line in lines.iter().skip(1) {
        let duration: u32 = line.split(',').next().unwrap().parse().unwrap();
        assert!(duration > prev_duration, "PDC should be sorted by duration");
        prev_duration = duration;
    }
}

#[test]
fn test_export_training_load_csv_format() {
    let (db, user_id) = setup_test_database();
    let today = Utc::now().date_naive();

    let exporter = AnalyticsExporter::new(db);
    let csv = exporter
        .export_training_load_csv(user_id, today - chrono::Duration::days(6), today)
        .expect("Failed to export training load CSV");

    // Verify headers
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "date,tss,atl,ctl,tsb,acwr");

    // Verify we have data rows (header + 7 days)
    assert_eq!(lines.len(), 8);

    // Verify data is chronologically ordered
    let mut prev_date: Option<NaiveDate> = None;
    for line in lines.iter().skip(1) {
        let date_str = line.split(',').next().unwrap();
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
        if let Some(prev) = prev_date {
            assert!(
                date > prev,
                "Training load should be chronologically ordered"
            );
        }
        prev_date = Some(date);
    }

    // Verify decimal precision in TSS column
    let last_line: Vec<&str> = lines[7].split(',').collect();
    assert_eq!(last_line[1], "85.00"); // TSS with 2 decimal places
}

#[test]
fn test_export_json_roundtrip() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);

    // Export to JSON
    let json = exporter
        .export_json(user_id)
        .expect("Failed to export JSON");

    // Parse back to struct
    let export: AnalyticsExport =
        serde_json::from_str(&json).expect("Failed to parse exported JSON");

    // Re-export and compare
    let re_exported_json = export.export_json().expect("Failed to re-export JSON");
    let re_export: AnalyticsExport =
        serde_json::from_str(&re_exported_json).expect("Failed to parse re-exported JSON");

    // Verify data is preserved through round-trip
    assert_eq!(export.user_id, re_export.user_id);
    assert_eq!(export.export_version, re_export.export_version);

    // PDC round-trip
    let original_pdc = export.pdc.as_ref().unwrap();
    let roundtrip_pdc = re_export.pdc.as_ref().unwrap();
    assert_eq!(original_pdc.points.len(), roundtrip_pdc.points.len());
    for (orig, rt) in original_pdc.points.iter().zip(roundtrip_pdc.points.iter()) {
        assert_eq!(orig.duration_secs, rt.duration_secs);
        assert_eq!(orig.power_watts, rt.power_watts);
    }

    // Training load round-trip
    let original_load = export.training_load.as_ref().unwrap();
    let roundtrip_load = re_export.training_load.as_ref().unwrap();
    assert_eq!(original_load.days.len(), roundtrip_load.days.len());
    for (orig, rt) in original_load.days.iter().zip(roundtrip_load.days.iter()) {
        assert_eq!(orig.date, rt.date);
        assert!((orig.tss - rt.tss).abs() < 0.01);
        assert!((orig.atl - rt.atl).abs() < 0.01);
        assert!((orig.ctl - rt.ctl).abs() < 0.01);
        assert!((orig.tsb - rt.tsb).abs() < 0.01);
    }

    // CP model round-trip
    let original_cp = export.cp_model.as_ref().unwrap();
    let roundtrip_cp = re_export.cp_model.as_ref().unwrap();
    assert_eq!(original_cp.cp_watts, roundtrip_cp.cp_watts);
    assert_eq!(original_cp.w_prime_joules, roundtrip_cp.w_prime_joules);
    assert!((original_cp.r_squared - roundtrip_cp.r_squared).abs() < 0.001);

    // Fitness profile round-trip
    let original_fitness = export.fitness_profile.as_ref().unwrap();
    let roundtrip_fitness = re_export.fitness_profile.as_ref().unwrap();
    assert_eq!(original_fitness.ftp_watts, roundtrip_fitness.ftp_watts);
    assert_eq!(original_fitness.rider_type, roundtrip_fitness.rider_type);
}

#[test]
fn test_export_with_options_pdc_only() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let options = ExportOptions::pdc_only();

    let export = exporter
        .build_export_with_options(user_id, &options)
        .expect("Failed to build export with options");

    // Only PDC should be present
    assert!(export.pdc.is_some(), "PDC should be included");
    assert!(
        export.training_load.is_none(),
        "Training load should be excluded"
    );
    assert!(export.cp_model.is_none(), "CP model should be excluded");
    assert!(
        export.fitness_profile.is_none(),
        "Fitness profile should be excluded"
    );
}

#[test]
fn test_export_with_options_training_load_only() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let options = ExportOptions::training_load_only();

    let export = exporter
        .build_export_with_options(user_id, &options)
        .expect("Failed to build export with options");

    // Only training load should be present
    assert!(export.pdc.is_none(), "PDC should be excluded");
    assert!(
        export.training_load.is_some(),
        "Training load should be included"
    );
    assert!(export.cp_model.is_none(), "CP model should be excluded");
    assert!(
        export.fitness_profile.is_none(),
        "Fitness profile should be excluded"
    );
}

#[test]
fn test_export_with_date_range_filter() {
    let (db, user_id) = setup_test_database();
    let today = Utc::now().date_naive();

    let exporter = AnalyticsExporter::new(db);

    // Only export last 3 days of training load
    let options = ExportOptions::new()
        .with_pdc(false)
        .with_training_load(true)
        .with_cp_model(false)
        .with_fitness_profile(false)
        .with_start_date(today - chrono::Duration::days(2))
        .with_end_date(today);

    let export = exporter
        .build_export_with_options(user_id, &options)
        .expect("Failed to build export with date filter");

    let training_load = export
        .training_load
        .expect("Training load should be present");
    assert_eq!(
        training_load.days.len(),
        3,
        "Should only have 3 days of data"
    );

    // Verify dates are in range
    for day in &training_load.days {
        assert!(day.date >= today - chrono::Duration::days(2));
        assert!(day.date <= today);
    }
}

#[test]
fn test_export_empty_user_no_data() {
    let db = Database::open_in_memory().expect("Failed to create database");
    let user_id = Uuid::new_v4();
    let db = Arc::new(db);

    let exporter = AnalyticsExporter::new(db);

    // Should succeed with empty data (no error for missing data in build_export)
    let export = exporter
        .build_export(user_id)
        .expect("Should succeed even with no data");

    // All optional fields should be None
    assert!(export.pdc.is_none());
    assert!(export.training_load.is_none());
    assert!(export.cp_model.is_none());
    assert!(export.fitness_profile.is_none());

    // Metadata should still be present
    assert_eq!(export.user_id, user_id.to_string());
    assert_eq!(export.export_version, "1.0");
}

#[test]
fn test_export_pdc_csv_insufficient_data_error() {
    let db = Database::open_in_memory().expect("Failed to create database");
    let user_id = Uuid::new_v4();
    let db = Arc::new(db);

    let exporter = AnalyticsExporter::new(db);

    // Should return InsufficientData error for PDC CSV when no data exists
    let result = exporter.export_pdc_csv(user_id);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No PDC data available"),
        "Error should mention missing PDC data"
    );
}

#[test]
fn test_export_training_load_csv_insufficient_data_error() {
    let db = Database::open_in_memory().expect("Failed to create database");
    let user_id = Uuid::new_v4();
    let db = Arc::new(db);
    let today = Utc::now().date_naive();

    let exporter = AnalyticsExporter::new(db);

    // Should return InsufficientData error for training load CSV when no data exists
    let result =
        exporter.export_training_load_csv(user_id, today - chrono::Duration::days(30), today);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No training load data available"),
        "Error should mention missing training load data"
    );
}

#[test]
fn test_json_export_pretty_printed() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let json = exporter
        .export_json(user_id)
        .expect("Failed to export JSON");

    // Pretty-printed JSON should have newlines and indentation
    assert!(
        json.contains('\n'),
        "JSON should be pretty-printed with newlines"
    );
    assert!(
        json.contains("  ") || json.contains("\t"),
        "JSON should have indentation"
    );

    // Verify it's valid JSON that can be parsed
    let _: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");
}

#[test]
fn test_export_preserves_data_precision() {
    let (db, user_id) = setup_test_database();

    let exporter = AnalyticsExporter::new(db);
    let export = exporter
        .build_export(user_id)
        .expect("Failed to build export");

    // Check r_squared precision is preserved
    let cp_model = export.cp_model.unwrap();
    assert!(
        (cp_model.r_squared - 0.98).abs() < 0.001,
        "R-squared precision should be preserved"
    );

    // Check VO2max precision is preserved
    let fitness = export.fitness_profile.unwrap();
    let vo2max = fitness.vo2max.unwrap();
    assert!(
        (vo2max.vo2max - 52.5).abs() < 0.1,
        "VO2max precision should be preserved"
    );

    // Check power profile percentages are preserved
    let power_profile = fitness.power_profile.unwrap();
    assert!((power_profile.neuromuscular_pct - 0.85).abs() < 0.01);
    assert!((power_profile.anaerobic_pct - 0.78).abs() < 0.01);
    assert!((power_profile.vo2max_pct - 0.82).abs() < 0.01);
    assert!((power_profile.threshold_pct - 0.80).abs() < 0.01);
}
