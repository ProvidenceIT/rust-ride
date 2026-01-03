//! Unit tests for power meter calibration reminder system.
//!
//! Tests the calibration tracking, reminder generation, and persistence
//! functionality for power meters.

use rust_ride::sensors::calibration::{
    CalibrationManager, CalibrationRecord, CalibrationReminder, CalibrationReminderConfig,
    CalibrationReminderType, get_calibration_path, is_calibratable_sensor,
    DEFAULT_CALIBRATION_REMINDER_DAYS, MAX_CALIBRATION_REMINDER_DAYS, MIN_CALIBRATION_REMINDER_DAYS,
};
use rust_ride::sensors::types::{Protocol, SensorType};
use std::path::PathBuf;

// ============================================================================
// CalibrationRecord tests
// ============================================================================

#[test]
fn test_calibration_record_new() {
    let record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(record.device_id, "device1");
    assert_eq!(record.name, "Stages Power L");
    assert_eq!(record.protocol, Protocol::BleCyclingPower);
    assert!(record.was_successful);
    assert_eq!(record.calibration_count, 1);
    assert!(record.offset_value.is_none());
    assert!(record.notes.is_none());
}

#[test]
fn test_calibration_record_record_calibration() {
    let mut record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(record.calibration_count, 1);
    assert!(record.offset_value.is_none());

    record.record_calibration(Some(500), true);

    assert_eq!(record.calibration_count, 2);
    assert_eq!(record.offset_value, Some(500));
    assert!(record.was_successful);

    record.record_calibration(Some(-100), false);

    assert_eq!(record.calibration_count, 3);
    assert_eq!(record.offset_value, Some(-100));
    assert!(!record.was_successful);
}

#[test]
fn test_calibration_record_with_notes() {
    let mut record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    record.record_calibration_with_notes(
        Some(100),
        true,
        Some("Cold garage calibration".to_string()),
    );

    assert_eq!(record.notes, Some("Cold garage calibration".to_string()));
    assert_eq!(record.offset_value, Some(100));
}

#[test]
fn test_calibration_record_days_since_calibration() {
    let record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    // Just created, should be 0 days
    assert_eq!(record.days_since_calibration(), 0);
}

#[test]
fn test_calibration_record_is_calibration_due() {
    let record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    // Just created, should not be due
    assert!(!record.is_calibration_due(7));
    assert!(!record.is_calibration_due(1));

    // Edge case: 0 days
    assert!(record.is_calibration_due(0));
}

#[test]
fn test_calibration_record_display_today() {
    let record = CalibrationRecord::new(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
    );

    assert_eq!(record.last_calibrated_display(), "Today");
}

// ============================================================================
// CalibrationReminder tests
// ============================================================================

#[test]
fn test_calibration_reminder_new() {
    let reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Stages Power L".to_string(),
        10,
        CalibrationReminderType::Due,
    );

    assert_eq!(reminder.device_id, "device1");
    assert_eq!(reminder.name, "Stages Power L");
    assert_eq!(reminder.days_since_calibration, 10);
    assert_eq!(reminder.reminder_type, CalibrationReminderType::Due);
    assert!(!reminder.shown);
    assert!(!reminder.dismissed);
}

#[test]
fn test_calibration_reminder_message_due() {
    let reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Stages Power".to_string(),
        7,
        CalibrationReminderType::Due,
    );

    let message = reminder.message();
    assert!(message.contains("Stages Power"));
    assert!(message.contains("7 days"));
    assert!(message.to_lowercase().contains("calibrat"));
}

#[test]
fn test_calibration_reminder_message_overdue() {
    let reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Quarq DZero".to_string(),
        21,
        CalibrationReminderType::Overdue,
    );

    let message = reminder.message();
    assert!(message.contains("Quarq DZero"));
    assert!(message.to_lowercase().contains("overdue"));
}

#[test]
fn test_calibration_reminder_message_never_calibrated() {
    let reminder = CalibrationReminder::new(
        "device1".to_string(),
        "4iiii Precision".to_string(),
        0,
        CalibrationReminderType::NeverCalibrated,
    );

    let message = reminder.message();
    assert!(message.contains("4iiii Precision"));
    assert!(message.to_lowercase().contains("never"));
}

#[test]
fn test_calibration_reminder_message_failure() {
    let reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Favero Assioma".to_string(),
        1,
        CalibrationReminderType::RecentFailure,
    );

    let message = reminder.message();
    assert!(message.contains("Favero Assioma"));
    assert!(message.to_lowercase().contains("failed") || message.to_lowercase().contains("try"));
}

#[test]
fn test_calibration_reminder_short_messages() {
    let due = CalibrationReminder::new(
        "d1".to_string(),
        "Stages".to_string(),
        7,
        CalibrationReminderType::Due,
    );
    assert!(due.short_message().contains("7 days"));

    let overdue = CalibrationReminder::new(
        "d2".to_string(),
        "Quarq".to_string(),
        14,
        CalibrationReminderType::Overdue,
    );
    assert!(overdue.short_message().to_lowercase().contains("overdue"));

    let never = CalibrationReminder::new(
        "d3".to_string(),
        "4iiii".to_string(),
        0,
        CalibrationReminderType::NeverCalibrated,
    );
    assert!(never.short_message().to_lowercase().contains("calibrate"));

    let failed = CalibrationReminder::new(
        "d4".to_string(),
        "Favero".to_string(),
        1,
        CalibrationReminderType::RecentFailure,
    );
    assert!(failed.short_message().to_lowercase().contains("retry"));
}

#[test]
fn test_calibration_reminder_mark_shown() {
    let mut reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Stages".to_string(),
        7,
        CalibrationReminderType::Due,
    );

    assert!(!reminder.shown);
    reminder.mark_shown();
    assert!(reminder.shown);
}

#[test]
fn test_calibration_reminder_dismiss() {
    let mut reminder = CalibrationReminder::new(
        "device1".to_string(),
        "Stages".to_string(),
        7,
        CalibrationReminderType::Due,
    );

    assert!(!reminder.dismissed);
    reminder.dismiss();
    assert!(reminder.dismissed);
}

#[test]
fn test_calibration_reminder_type_display() {
    assert_eq!(format!("{}", CalibrationReminderType::Due), "Due");
    assert_eq!(format!("{}", CalibrationReminderType::Overdue), "Overdue");
    assert_eq!(
        format!("{}", CalibrationReminderType::NeverCalibrated),
        "Never Calibrated"
    );
    assert_eq!(format!("{}", CalibrationReminderType::RecentFailure), "Failed");
}

// ============================================================================
// CalibrationReminderConfig tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = CalibrationReminderConfig::default();

    assert_eq!(config.reminder_days, DEFAULT_CALIBRATION_REMINDER_DAYS);
    assert!(config.enabled);
    assert!(!config.show_on_startup);
    assert!(config.remind_never_calibrated);
}

#[test]
fn test_config_strict() {
    let strict = CalibrationReminderConfig::strict();

    assert!(strict.reminder_days < DEFAULT_CALIBRATION_REMINDER_DAYS);
    assert!(strict.enabled);
    assert!(strict.show_on_startup);
    assert!(strict.remind_never_calibrated);
}

#[test]
fn test_config_relaxed() {
    let relaxed = CalibrationReminderConfig::relaxed();

    assert!(relaxed.reminder_days > DEFAULT_CALIBRATION_REMINDER_DAYS);
    assert!(relaxed.enabled);
    assert!(!relaxed.show_on_startup);
    assert!(relaxed.remind_never_calibrated);
}

#[test]
fn test_config_disabled() {
    let disabled = CalibrationReminderConfig::disabled();

    assert!(!disabled.enabled);
}

#[test]
fn test_config_set_reminder_days_clamps_min() {
    let mut config = CalibrationReminderConfig::default();

    config.set_reminder_days(0);
    assert_eq!(config.reminder_days, MIN_CALIBRATION_REMINDER_DAYS);

    config.set_reminder_days(-10);
    assert_eq!(config.reminder_days, MIN_CALIBRATION_REMINDER_DAYS);
}

#[test]
fn test_config_set_reminder_days_clamps_max() {
    let mut config = CalibrationReminderConfig::default();

    config.set_reminder_days(100);
    assert_eq!(config.reminder_days, MAX_CALIBRATION_REMINDER_DAYS);

    config.set_reminder_days(365);
    assert_eq!(config.reminder_days, MAX_CALIBRATION_REMINDER_DAYS);
}

#[test]
fn test_config_set_reminder_days_valid_range() {
    let mut config = CalibrationReminderConfig::default();

    config.set_reminder_days(14);
    assert_eq!(config.reminder_days, 14);

    config.set_reminder_days(30);
    assert_eq!(config.reminder_days, 30);
}

#[test]
fn test_config_validate() {
    let mut config = CalibrationReminderConfig {
        reminder_days: 1000, // Invalid - too high
        enabled: true,
        show_on_startup: false,
        remind_never_calibrated: true,
    };

    config.validate();

    assert_eq!(config.reminder_days, MAX_CALIBRATION_REMINDER_DAYS);
}

// ============================================================================
// CalibrationManager basic tests
// ============================================================================

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("test_cal_{}.json", name))
}

#[test]
fn test_manager_new() {
    let path = temp_path("new");
    let manager = CalibrationManager::with_path(path.clone());

    assert_eq!(manager.record_count(), 0);
    assert!(manager.config().enabled);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_record_calibration() {
    let path = temp_path("record");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        Some(100),
        true,
    );

    assert_eq!(manager.record_count(), 1);
    assert!(manager.has_been_calibrated("device1"));

    let record = manager.get_record("device1").unwrap();
    assert_eq!(record.name, "Stages Power L");
    assert_eq!(record.offset_value, Some(100));
    assert!(record.was_successful);
    assert_eq!(record.calibration_count, 1);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_record_calibration_updates_existing() {
    let path = temp_path("update");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // First calibration
    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        Some(100),
        true,
    );

    // Second calibration
    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        Some(150),
        true,
    );

    assert_eq!(manager.record_count(), 1);

    let record = manager.get_record("device1").unwrap();
    assert_eq!(record.calibration_count, 2);
    assert_eq!(record.offset_value, Some(150));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_record_calibration_with_notes() {
    let path = temp_path("notes");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration_with_notes(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        Some(200),
        true,
        Some("Indoor warmup calibration".to_string()),
    );

    let record = manager.get_record("device1").unwrap();
    assert_eq!(record.notes, Some("Indoor warmup calibration".to_string()));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_has_been_calibrated() {
    let path = temp_path("has");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    assert!(!manager.has_been_calibrated("device1"));

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    assert!(manager.has_been_calibrated("device1"));
    assert!(!manager.has_been_calibrated("device2"));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_is_calibration_due_never_calibrated() {
    let path = temp_path("due_never");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // With remind_never_calibrated = true (default)
    assert!(manager.is_calibration_due("unknown_device"));

    // With remind_never_calibrated = false
    let mut config = manager.config().clone();
    config.remind_never_calibrated = false;
    manager.set_config(config);
    assert!(!manager.is_calibration_due("unknown_device"));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_is_calibration_due_recently_calibrated() {
    let path = temp_path("due_recent");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    // Just calibrated, should not be due
    assert!(!manager.is_calibration_due("device1"));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_is_calibration_due_disabled() {
    let path = temp_path("due_disabled");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.set_config(CalibrationReminderConfig::disabled());

    // Even for unknown device, should return false when disabled
    assert!(!manager.is_calibration_due("unknown_device"));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_days_since_calibration() {
    let path = temp_path("days");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    assert!(manager.days_since_calibration("device1").is_none());

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    assert_eq!(manager.days_since_calibration("device1"), Some(0));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_all_records() {
    let path = temp_path("all");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );
    manager.record_calibration(
        "device2".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
        None,
        true,
    );

    let records: Vec<_> = manager.all_records().collect();
    assert_eq!(records.len(), 2);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// CalibrationManager reminder tests
// ============================================================================

#[test]
fn test_manager_check_for_reminders_never_calibrated() {
    let path = temp_path("remind_never");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    let reminders = manager.check_for_reminders(&power_meters);

    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].device_id, "device1");
    assert_eq!(reminders[0].reminder_type, CalibrationReminderType::NeverCalibrated);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_check_for_reminders_recently_calibrated() {
    let path = temp_path("remind_recent");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    let reminders = manager.check_for_reminders(&power_meters);

    // Just calibrated, no reminder
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_check_for_reminders_failed_calibration() {
    let path = temp_path("remind_failed");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // Record a failed calibration
    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        false, // Failed
    );

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    let reminders = manager.check_for_reminders(&power_meters);

    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].reminder_type, CalibrationReminderType::RecentFailure);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_check_for_reminders_disabled() {
    let path = temp_path("remind_disabled");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);
    manager.set_config(CalibrationReminderConfig::disabled());

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    let reminders = manager.check_for_reminders(&power_meters);

    // No reminders when disabled
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_dismiss_reminder() {
    let path = temp_path("dismiss");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    // First check - should get reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert_eq!(reminders.len(), 1);

    // Dismiss it
    manager.dismiss_reminder("device1");

    // Second check - should not get reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_mark_reminder_shown() {
    let path = temp_path("shown");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    // First check
    let reminders = manager.check_for_reminders(&power_meters);
    assert_eq!(reminders.len(), 1);

    // Mark as shown
    manager.mark_reminder_shown("device1");

    // Second check - should not get reminder (already shown this session)
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_dismiss_all_reminders() {
    let path = temp_path("dismiss_all");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // Record calibration for device1 (so it's tracked)
    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        false, // Failed, so will generate reminder
    );

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    // First check - should get reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(!reminders.is_empty());

    // Dismiss all
    manager.dismiss_all_reminders();

    // Second check - should not get reminders
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_clear_session_state() {
    let path = temp_path("clear_session");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let power_meters = vec![(
        "device1".to_string(),
        "Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    // Get and dismiss reminder
    manager.check_for_reminders(&power_meters);
    manager.dismiss_reminder("device1");

    // Should not get reminder
    assert!(manager.check_for_reminders(&power_meters).is_empty());

    // Clear session state
    manager.clear_session_state();

    // Should get reminder again
    assert!(!manager.check_for_reminders(&power_meters).is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// CalibrationManager removal and clearing tests
// ============================================================================

#[test]
fn test_manager_remove_record() {
    let path = temp_path("remove");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    assert!(manager.has_been_calibrated("device1"));

    let removed = manager.remove_record("device1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().device_id, "device1");

    assert!(!manager.has_been_calibrated("device1"));
    assert_eq!(manager.record_count(), 0);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_remove_nonexistent_record() {
    let path = temp_path("remove_nonexistent");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let removed = manager.remove_record("nonexistent");
    assert!(removed.is_none());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_clear() {
    let path = temp_path("clear");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.record_calibration(
        "device1".to_string(),
        "Stages".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );
    manager.record_calibration(
        "device2".to_string(),
        "Quarq".to_string(),
        Protocol::AntPower,
        None,
        true,
    );

    assert_eq!(manager.record_count(), 2);

    manager.clear();

    assert_eq!(manager.record_count(), 0);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// CalibrationManager configuration tests
// ============================================================================

#[test]
fn test_manager_set_config() {
    let path = temp_path("set_config");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let strict = CalibrationReminderConfig::strict();
    manager.set_config(strict.clone());

    assert_eq!(manager.config().reminder_days, strict.reminder_days);
    assert!(manager.config().show_on_startup);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_set_reminder_days() {
    let path = temp_path("set_days");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    manager.set_reminder_days(14);
    assert_eq!(manager.config().reminder_days, 14);

    manager.set_reminder_days(30);
    assert_eq!(manager.config().reminder_days, 30);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// Helper function tests
// ============================================================================

#[test]
fn test_is_calibratable_sensor() {
    // Sensors that can be calibrated
    assert!(is_calibratable_sensor(SensorType::PowerMeter));
    assert!(is_calibratable_sensor(SensorType::Trainer));
    assert!(is_calibratable_sensor(SensorType::SmartTrainer));

    // Sensors that cannot be calibrated
    assert!(!is_calibratable_sensor(SensorType::HeartRate));
    assert!(!is_calibratable_sensor(SensorType::Cadence));
    assert!(!is_calibratable_sensor(SensorType::Speed));
    assert!(!is_calibratable_sensor(SensorType::SpeedCadence));
    assert!(!is_calibratable_sensor(SensorType::SmO2));
}

#[test]
fn test_get_calibration_path() {
    let path = get_calibration_path();

    // Should end with the expected filename
    assert!(path.to_string_lossy().ends_with("power_meter_calibration.json"));
}

// ============================================================================
// Multiple power meter tests
// ============================================================================

#[test]
fn test_manager_multiple_power_meters() {
    let path = temp_path("multiple");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // Calibrate first power meter
    manager.record_calibration(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        Some(100),
        true,
    );

    // Calibrate second power meter
    manager.record_calibration(
        "quarq_001".to_string(),
        "Quarq DZero".to_string(),
        Protocol::AntPower,
        Some(-50),
        true,
    );

    assert_eq!(manager.record_count(), 2);
    assert!(manager.has_been_calibrated("stages_001"));
    assert!(manager.has_been_calibrated("quarq_001"));

    let stages = manager.get_record("stages_001").unwrap();
    assert_eq!(stages.offset_value, Some(100));

    let quarq = manager.get_record("quarq_001").unwrap();
    assert_eq!(quarq.offset_value, Some(-50));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_manager_check_reminders_multiple_power_meters() {
    let path = temp_path("remind_multi");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // Calibrate only one power meter
    manager.record_calibration(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        None,
        true,
    );

    let power_meters = vec![
        (
            "stages_001".to_string(),
            "Stages Power L".to_string(),
            Protocol::BleCyclingPower,
        ),
        (
            "quarq_001".to_string(),
            "Quarq DZero".to_string(),
            Protocol::AntPower,
        ),
    ];

    let reminders = manager.check_for_reminders(&power_meters);

    // Should only get reminder for Quarq (never calibrated)
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].device_id, "quarq_001");
    assert_eq!(reminders[0].reminder_type, CalibrationReminderType::NeverCalibrated);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ============================================================================
// Real-world scenario tests
// ============================================================================

#[test]
fn test_scenario_new_power_meter_first_ride() {
    let path = temp_path("scenario_new");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // User connects new power meter for the first time
    let power_meters = vec![(
        "stages_new".to_string(),
        "New Stages Power".to_string(),
        Protocol::BleCyclingPower,
    )];

    // Should get "never calibrated" reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].reminder_type, CalibrationReminderType::NeverCalibrated);

    // User performs calibration
    manager.record_calibration(
        "stages_new".to_string(),
        "New Stages Power".to_string(),
        Protocol::BleCyclingPower,
        Some(125),
        true,
    );

    // No more reminders
    manager.clear_session_state();
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_scenario_calibration_failure_retry() {
    let path = temp_path("scenario_retry");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    // First calibration attempt fails
    manager.record_calibration(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        None,
        false, // Failed
    );

    let power_meters = vec![(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    )];

    // Should get failure reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].reminder_type, CalibrationReminderType::RecentFailure);

    // User retries and succeeds
    manager.record_calibration(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        Some(100),
        true,
    );

    // No more reminders
    manager.clear_session_state();
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_scenario_user_dismisses_reminder_for_session() {
    let path = temp_path("scenario_dismiss");
    let mut manager = CalibrationManager::with_path(path.clone());
    manager.set_auto_save(false);

    let power_meters = vec![(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
    )];

    // User sees reminder
    let reminders = manager.check_for_reminders(&power_meters);
    assert_eq!(reminders.len(), 1);

    // User dismisses reminder (not calibrating now)
    manager.dismiss_reminder("stages_001");

    // No more reminders this session
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // But if recording a calibration later, the dismiss is cleared
    manager.record_calibration(
        "stages_001".to_string(),
        "Stages Power L".to_string(),
        Protocol::BleCyclingPower,
        Some(100),
        true,
    );

    // Still no reminder (now calibrated)
    manager.clear_session_state();
    let reminders = manager.check_for_reminders(&power_meters);
    assert!(reminders.is_empty());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}
