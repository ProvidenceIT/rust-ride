//! Power meter calibration tracking and reminder system.
//!
//! Tracks last calibration date for power meters and provides reminders
//! when calibration is due. Calibration is important for accurate power
//! readings, especially after temperature changes.
//!
//! Key features:
//! - Tracks last calibration date per power meter
//! - Configurable reminder period (default 7 days)
//! - Persists calibration history to JSON
//! - Generates user-friendly reminder messages
//! - Supports multiple power meters

use crate::sensors::types::{Protocol, SensorType};
use crate::storage::config::get_data_dir;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Default calibration reminder period in days.
pub const DEFAULT_CALIBRATION_REMINDER_DAYS: i64 = 7;

/// Minimum calibration reminder period in days.
pub const MIN_CALIBRATION_REMINDER_DAYS: i64 = 1;

/// Maximum calibration reminder period in days.
pub const MAX_CALIBRATION_REMINDER_DAYS: i64 = 90;

/// Default calibration file name.
const CALIBRATION_FILE_NAME: &str = "power_meter_calibration.json";

/// Calibration record for a single power meter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationRecord {
    /// Device ID of the power meter.
    pub device_id: String,
    /// Name of the power meter.
    pub name: String,
    /// Protocol (BLE or ANT+).
    pub protocol: Protocol,
    /// When the power meter was last calibrated.
    pub last_calibrated_at: DateTime<Utc>,
    /// Calibration offset value (if available from the device).
    pub offset_value: Option<i32>,
    /// Whether the last calibration was successful.
    pub was_successful: bool,
    /// Number of times this power meter has been calibrated.
    pub calibration_count: u32,
    /// User notes about the calibration (e.g., "Cold garage").
    pub notes: Option<String>,
}

impl CalibrationRecord {
    /// Create a new calibration record.
    pub fn new(device_id: String, name: String, protocol: Protocol) -> Self {
        Self {
            device_id,
            name,
            protocol,
            last_calibrated_at: Utc::now(),
            offset_value: None,
            was_successful: true,
            calibration_count: 1,
            notes: None,
        }
    }

    /// Record a new calibration event.
    pub fn record_calibration(&mut self, offset: Option<i32>, successful: bool) {
        self.last_calibrated_at = Utc::now();
        self.offset_value = offset;
        self.was_successful = successful;
        self.calibration_count = self.calibration_count.saturating_add(1);
    }

    /// Record a new calibration with notes.
    pub fn record_calibration_with_notes(
        &mut self,
        offset: Option<i32>,
        successful: bool,
        notes: Option<String>,
    ) {
        self.record_calibration(offset, successful);
        self.notes = notes;
    }

    /// Get the time since last calibration.
    pub fn time_since_calibration(&self) -> Duration {
        Utc::now().signed_duration_since(self.last_calibrated_at)
    }

    /// Get days since last calibration.
    pub fn days_since_calibration(&self) -> i64 {
        self.time_since_calibration().num_days()
    }

    /// Check if calibration is due based on the reminder period.
    pub fn is_calibration_due(&self, reminder_days: i64) -> bool {
        self.days_since_calibration() >= reminder_days
    }

    /// Get a formatted string for the last calibration time.
    pub fn last_calibrated_display(&self) -> String {
        let days = self.days_since_calibration();
        if days == 0 {
            "Today".to_string()
        } else if days == 1 {
            "Yesterday".to_string()
        } else if days < 7 {
            format!("{} days ago", days)
        } else if days < 30 {
            let weeks = days / 7;
            if weeks == 1 {
                "1 week ago".to_string()
            } else {
                format!("{} weeks ago", weeks)
            }
        } else {
            let months = days / 30;
            if months == 1 {
                "1 month ago".to_string()
            } else {
                format!("{} months ago", months)
            }
        }
    }
}

/// Calibration reminder for a power meter.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationReminder {
    /// Device ID of the power meter.
    pub device_id: String,
    /// Name of the power meter.
    pub name: String,
    /// Days since last calibration.
    pub days_since_calibration: i64,
    /// Type of reminder.
    pub reminder_type: CalibrationReminderType,
    /// Whether this reminder has been shown to the user.
    pub shown: bool,
    /// Whether the user has dismissed this reminder.
    pub dismissed: bool,
}

impl CalibrationReminder {
    /// Create a new calibration reminder.
    pub fn new(
        device_id: String,
        name: String,
        days_since_calibration: i64,
        reminder_type: CalibrationReminderType,
    ) -> Self {
        Self {
            device_id,
            name,
            days_since_calibration,
            reminder_type,
            shown: false,
            dismissed: false,
        }
    }

    /// Get a user-friendly message for this reminder.
    pub fn message(&self) -> String {
        match self.reminder_type {
            CalibrationReminderType::Due => {
                format!(
                    "{} hasn't been calibrated in {} days. Consider calibrating for accurate power readings.",
                    self.name, self.days_since_calibration
                )
            }
            CalibrationReminderType::Overdue => {
                format!(
                    "{} is overdue for calibration ({} days). Calibrate now for accurate power data.",
                    self.name, self.days_since_calibration
                )
            }
            CalibrationReminderType::NeverCalibrated => {
                format!(
                    "{} has never been calibrated. Calibrate before your workout for best accuracy.",
                    self.name
                )
            }
            CalibrationReminderType::RecentFailure => {
                format!(
                    "Last calibration for {} failed. Try calibrating again.",
                    self.name
                )
            }
        }
    }

    /// Get a short message for compact display.
    pub fn short_message(&self) -> String {
        match self.reminder_type {
            CalibrationReminderType::Due => {
                format!("Calibrate {} ({} days)", self.name, self.days_since_calibration)
            }
            CalibrationReminderType::Overdue => {
                format!("{} overdue", self.name)
            }
            CalibrationReminderType::NeverCalibrated => {
                format!("Calibrate {}", self.name)
            }
            CalibrationReminderType::RecentFailure => {
                format!("Retry {}", self.name)
            }
        }
    }

    /// Mark this reminder as shown.
    pub fn mark_shown(&mut self) {
        self.shown = true;
    }

    /// Dismiss this reminder.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }
}

/// Type of calibration reminder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationReminderType {
    /// Calibration is due (past the configured reminder period).
    Due,
    /// Calibration is significantly overdue (2x the reminder period).
    Overdue,
    /// Power meter has never been calibrated.
    NeverCalibrated,
    /// Last calibration attempt failed.
    RecentFailure,
}

impl std::fmt::Display for CalibrationReminderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalibrationReminderType::Due => write!(f, "Due"),
            CalibrationReminderType::Overdue => write!(f, "Overdue"),
            CalibrationReminderType::NeverCalibrated => write!(f, "Never Calibrated"),
            CalibrationReminderType::RecentFailure => write!(f, "Failed"),
        }
    }
}

/// Configuration for calibration reminders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationReminderConfig {
    /// Number of days before showing a calibration reminder (default: 7).
    pub reminder_days: i64,
    /// Whether calibration reminders are enabled (default: true).
    pub enabled: bool,
    /// Show reminder on every app start (default: false).
    /// If false, only shows once per session for each due device.
    pub show_on_startup: bool,
    /// Remind for devices that have never been calibrated (default: true).
    pub remind_never_calibrated: bool,
}

impl Default for CalibrationReminderConfig {
    fn default() -> Self {
        Self {
            reminder_days: DEFAULT_CALIBRATION_REMINDER_DAYS,
            enabled: true,
            show_on_startup: false,
            remind_never_calibrated: true,
        }
    }
}

impl CalibrationReminderConfig {
    /// Create a strict configuration with shorter reminder period.
    pub fn strict() -> Self {
        Self {
            reminder_days: 3,
            enabled: true,
            show_on_startup: true,
            remind_never_calibrated: true,
        }
    }

    /// Create a relaxed configuration with longer reminder period.
    pub fn relaxed() -> Self {
        Self {
            reminder_days: 14,
            enabled: true,
            show_on_startup: false,
            remind_never_calibrated: true,
        }
    }

    /// Create a disabled configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set the reminder period in days, clamped to valid range.
    pub fn set_reminder_days(&mut self, days: i64) {
        self.reminder_days = days.clamp(MIN_CALIBRATION_REMINDER_DAYS, MAX_CALIBRATION_REMINDER_DAYS);
    }

    /// Validate and fix any invalid configuration values.
    pub fn validate(&mut self) {
        self.reminder_days = self.reminder_days.clamp(
            MIN_CALIBRATION_REMINDER_DAYS,
            MAX_CALIBRATION_REMINDER_DAYS,
        );
    }
}

/// Persistent calibration data for all power meters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationData {
    /// Calibration records indexed by device ID.
    #[serde(default)]
    pub records: HashMap<String, CalibrationRecord>,
    /// Configuration for reminders.
    #[serde(default)]
    pub config: CalibrationReminderConfig,
    /// When this data was last updated.
    #[serde(default = "Utc::now")]
    pub last_updated_at: DateTime<Utc>,
}

impl CalibrationData {
    /// Create new empty calibration data.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            config: CalibrationReminderConfig::default(),
            last_updated_at: Utc::now(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: CalibrationReminderConfig) -> Self {
        Self {
            records: HashMap::new(),
            config,
            last_updated_at: Utc::now(),
        }
    }
}

/// Manages calibration tracking and reminders for power meters.
#[derive(Debug)]
pub struct CalibrationManager {
    /// Calibration data.
    data: CalibrationData,
    /// Path to the calibration file.
    file_path: PathBuf,
    /// Whether data has been modified since last save.
    dirty: bool,
    /// Whether to auto-save on changes.
    auto_save: bool,
    /// Dismissed reminders for this session (device_id).
    session_dismissed: std::collections::HashSet<String>,
    /// Shown reminders for this session (device_id).
    session_shown: std::collections::HashSet<String>,
}

impl Default for CalibrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationManager {
    /// Create a new calibration manager with default path.
    pub fn new() -> Self {
        Self::with_path(get_calibration_path())
    }

    /// Create a calibration manager with a custom path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            data: CalibrationData::new(),
            file_path: path,
            dirty: false,
            auto_save: true,
            session_dismissed: std::collections::HashSet::new(),
            session_shown: std::collections::HashSet::new(),
        }
    }

    /// Load calibration data from disk.
    pub fn load() -> Self {
        Self::load_from_path(get_calibration_path())
    }

    /// Load calibration data from a specific path.
    pub fn load_from_path(path: PathBuf) -> Self {
        if !path.exists() {
            tracing::debug!("No calibration file found at {:?}, starting fresh", path);
            return Self::with_path(path);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<CalibrationData>(&content) {
                Ok(data) => {
                    tracing::info!(
                        "Loaded calibration data for {} power meters",
                        data.records.len()
                    );
                    Self {
                        data,
                        file_path: path,
                        dirty: false,
                        auto_save: true,
                        session_dismissed: std::collections::HashSet::new(),
                        session_shown: std::collections::HashSet::new(),
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse calibration file: {}", e);
                    Self::with_path(path)
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read calibration file: {}", e);
                Self::with_path(path)
            }
        }
    }

    /// Save calibration data to disk.
    pub fn save(&mut self) -> Result<(), CalibrationError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CalibrationError::IoError(e.to_string()))?;
        }

        self.data.last_updated_at = Utc::now();

        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| CalibrationError::SerializeError(e.to_string()))?;

        std::fs::write(&self.file_path, content)
            .map_err(|e| CalibrationError::IoError(e.to_string()))?;

        self.dirty = false;
        tracing::debug!(
            "Saved calibration data for {} power meters",
            self.data.records.len()
        );

        Ok(())
    }

    /// Enable or disable auto-save.
    pub fn set_auto_save(&mut self, enabled: bool) {
        self.auto_save = enabled;
    }

    /// Get the configuration.
    pub fn config(&self) -> &CalibrationReminderConfig {
        &self.data.config
    }

    /// Set the configuration.
    pub fn set_config(&mut self, config: CalibrationReminderConfig) {
        self.data.config = config;
        self.dirty = true;
        self.try_auto_save();
    }

    /// Set the reminder period in days.
    pub fn set_reminder_days(&mut self, days: i64) {
        self.data.config.set_reminder_days(days);
        self.dirty = true;
        self.try_auto_save();
    }

    /// Record a calibration event for a power meter.
    pub fn record_calibration(
        &mut self,
        device_id: String,
        name: String,
        protocol: Protocol,
        offset: Option<i32>,
        successful: bool,
    ) {
        if let Some(record) = self.data.records.get_mut(&device_id) {
            record.record_calibration(offset, successful);
            tracing::info!(
                "Recorded calibration for {} (count: {})",
                name,
                record.calibration_count
            );
        } else {
            let mut record = CalibrationRecord::new(device_id.clone(), name.clone(), protocol);
            record.offset_value = offset;
            record.was_successful = successful;
            self.data.records.insert(device_id.clone(), record);
            tracing::info!("Recorded first calibration for {}", name);
        }

        // Clear any dismissed state for this device
        self.session_dismissed.remove(&device_id);

        self.dirty = true;
        self.try_auto_save();
    }

    /// Record a calibration event with notes.
    pub fn record_calibration_with_notes(
        &mut self,
        device_id: String,
        name: String,
        protocol: Protocol,
        offset: Option<i32>,
        successful: bool,
        notes: Option<String>,
    ) {
        self.record_calibration(device_id.clone(), name, protocol, offset, successful);
        if let Some(record) = self.data.records.get_mut(&device_id) {
            record.notes = notes;
        }
    }

    /// Get the calibration record for a device.
    pub fn get_record(&self, device_id: &str) -> Option<&CalibrationRecord> {
        self.data.records.get(device_id)
    }

    /// Check if a device has ever been calibrated.
    pub fn has_been_calibrated(&self, device_id: &str) -> bool {
        self.data.records.contains_key(device_id)
    }

    /// Check if calibration is due for a device.
    pub fn is_calibration_due(&self, device_id: &str) -> bool {
        if !self.data.config.enabled {
            return false;
        }

        match self.data.records.get(device_id) {
            Some(record) => record.is_calibration_due(self.data.config.reminder_days),
            None => self.data.config.remind_never_calibrated,
        }
    }

    /// Get days since last calibration for a device.
    pub fn days_since_calibration(&self, device_id: &str) -> Option<i64> {
        self.data.records
            .get(device_id)
            .map(|r| r.days_since_calibration())
    }

    /// Get all calibration records.
    pub fn all_records(&self) -> impl Iterator<Item = &CalibrationRecord> {
        self.data.records.values()
    }

    /// Get records for devices that need calibration.
    pub fn due_records(&self) -> Vec<&CalibrationRecord> {
        if !self.data.config.enabled {
            return Vec::new();
        }

        self.data
            .records
            .values()
            .filter(|r| r.is_calibration_due(self.data.config.reminder_days))
            .collect()
    }

    /// Get the number of tracked power meters.
    pub fn record_count(&self) -> usize {
        self.data.records.len()
    }

    /// Check for calibration reminders for a list of connected power meters.
    ///
    /// Returns reminders for any power meters that are due for calibration.
    pub fn check_for_reminders(
        &mut self,
        connected_power_meters: &[(String, String, Protocol)],
    ) -> Vec<CalibrationReminder> {
        if !self.data.config.enabled {
            return Vec::new();
        }

        let mut reminders = Vec::new();

        for (device_id, name, protocol) in connected_power_meters {
            // Skip if already dismissed this session
            if self.session_dismissed.contains(device_id) {
                continue;
            }

            // Skip if already shown this session (unless show_on_startup is enabled)
            if !self.data.config.show_on_startup && self.session_shown.contains(device_id) {
                continue;
            }

            if let Some(reminder) = self.create_reminder_for_device(device_id, name, protocol) {
                reminders.push(reminder);
            }
        }

        reminders
    }

    /// Create a reminder for a specific device if needed.
    fn create_reminder_for_device(
        &self,
        device_id: &str,
        name: &str,
        _protocol: &Protocol,
    ) -> Option<CalibrationReminder> {
        match self.data.records.get(device_id) {
            Some(record) => {
                // Check if last calibration failed
                if !record.was_successful {
                    return Some(CalibrationReminder::new(
                        device_id.to_string(),
                        name.to_string(),
                        record.days_since_calibration(),
                        CalibrationReminderType::RecentFailure,
                    ));
                }

                let days = record.days_since_calibration();

                // Check if overdue (2x the reminder period)
                if days >= self.data.config.reminder_days * 2 {
                    return Some(CalibrationReminder::new(
                        device_id.to_string(),
                        name.to_string(),
                        days,
                        CalibrationReminderType::Overdue,
                    ));
                }

                // Check if due
                if days >= self.data.config.reminder_days {
                    return Some(CalibrationReminder::new(
                        device_id.to_string(),
                        name.to_string(),
                        days,
                        CalibrationReminderType::Due,
                    ));
                }

                None
            }
            None => {
                // Never calibrated
                if self.data.config.remind_never_calibrated {
                    Some(CalibrationReminder::new(
                        device_id.to_string(),
                        name.to_string(),
                        0,
                        CalibrationReminderType::NeverCalibrated,
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// Mark a reminder as shown for this session.
    pub fn mark_reminder_shown(&mut self, device_id: &str) {
        self.session_shown.insert(device_id.to_string());
    }

    /// Dismiss a reminder for this session.
    pub fn dismiss_reminder(&mut self, device_id: &str) {
        self.session_dismissed.insert(device_id.to_string());
    }

    /// Dismiss all reminders for this session.
    pub fn dismiss_all_reminders(&mut self) {
        for device_id in self.data.records.keys() {
            self.session_dismissed.insert(device_id.clone());
        }
    }

    /// Clear session state (for testing or session reset).
    pub fn clear_session_state(&mut self) {
        self.session_shown.clear();
        self.session_dismissed.clear();
    }

    /// Remove a calibration record.
    pub fn remove_record(&mut self, device_id: &str) -> Option<CalibrationRecord> {
        let removed = self.data.records.remove(device_id);
        if removed.is_some() {
            self.dirty = true;
            self.try_auto_save();
        }
        removed
    }

    /// Clear all calibration records.
    pub fn clear(&mut self) {
        self.data.records.clear();
        self.dirty = true;
        self.try_auto_save();
    }

    /// Delete the calibration file.
    pub fn delete_file(&self) -> Result<(), CalibrationError> {
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)
                .map_err(|e| CalibrationError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    /// Try to auto-save if enabled.
    fn try_auto_save(&mut self) {
        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save calibration data: {}", e);
            }
        }
    }
}

/// Get the default calibration file path.
pub fn get_calibration_path() -> PathBuf {
    get_data_dir().join(CALIBRATION_FILE_NAME)
}

/// Errors that can occur with calibration tracking.
#[derive(Debug, Error)]
pub enum CalibrationError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialize error: {0}")]
    SerializeError(String),

    #[error("Deserialize error: {0}")]
    DeserializeError(String),
}

/// Check if a sensor type is a power meter or provides power data.
pub fn is_calibratable_sensor(sensor_type: SensorType) -> bool {
    matches!(
        sensor_type,
        SensorType::PowerMeter | SensorType::Trainer | SensorType::SmartTrainer
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_record_new() {
        let record = CalibrationRecord::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert_eq!(record.device_id, "device1");
        assert_eq!(record.name, "Stages Power");
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

        record.record_calibration(Some(100), true);

        assert_eq!(record.calibration_count, 2);
        assert_eq!(record.offset_value, Some(100));
        assert!(record.was_successful);
    }

    #[test]
    fn test_calibration_record_days_since() {
        let record = CalibrationRecord::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        // Just created, should be 0 days
        assert_eq!(record.days_since_calibration(), 0);
        assert!(!record.is_calibration_due(7));
    }

    #[test]
    fn test_calibration_record_display() {
        let record = CalibrationRecord::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
        );

        assert_eq!(record.last_calibrated_display(), "Today");
    }

    #[test]
    fn test_calibration_reminder_new() {
        let reminder = CalibrationReminder::new(
            "device1".to_string(),
            "Stages Power".to_string(),
            7,
            CalibrationReminderType::Due,
        );

        assert_eq!(reminder.device_id, "device1");
        assert_eq!(reminder.name, "Stages Power");
        assert_eq!(reminder.days_since_calibration, 7);
        assert_eq!(reminder.reminder_type, CalibrationReminderType::Due);
        assert!(!reminder.shown);
        assert!(!reminder.dismissed);
    }

    #[test]
    fn test_calibration_reminder_messages() {
        let due = CalibrationReminder::new(
            "d1".to_string(),
            "Stages".to_string(),
            7,
            CalibrationReminderType::Due,
        );
        assert!(due.message().contains("7 days"));
        assert!(due.message().contains("Stages"));

        let overdue = CalibrationReminder::new(
            "d2".to_string(),
            "Quarq".to_string(),
            14,
            CalibrationReminderType::Overdue,
        );
        assert!(overdue.message().to_lowercase().contains("overdue"));

        let never = CalibrationReminder::new(
            "d3".to_string(),
            "4iiii".to_string(),
            0,
            CalibrationReminderType::NeverCalibrated,
        );
        assert!(never.message().to_lowercase().contains("never"));

        let failed = CalibrationReminder::new(
            "d4".to_string(),
            "Favero".to_string(),
            1,
            CalibrationReminderType::RecentFailure,
        );
        assert!(failed.message().to_lowercase().contains("failed"));
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

    #[test]
    fn test_config_default() {
        let config = CalibrationReminderConfig::default();

        assert_eq!(config.reminder_days, 7);
        assert!(config.enabled);
        assert!(!config.show_on_startup);
        assert!(config.remind_never_calibrated);
    }

    #[test]
    fn test_config_presets() {
        let strict = CalibrationReminderConfig::strict();
        assert!(strict.reminder_days < 7);
        assert!(strict.show_on_startup);

        let relaxed = CalibrationReminderConfig::relaxed();
        assert!(relaxed.reminder_days > 7);
        assert!(!relaxed.show_on_startup);

        let disabled = CalibrationReminderConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_config_set_reminder_days() {
        let mut config = CalibrationReminderConfig::default();

        config.set_reminder_days(0); // Below min
        assert_eq!(config.reminder_days, MIN_CALIBRATION_REMINDER_DAYS);

        config.set_reminder_days(100); // Above max
        assert_eq!(config.reminder_days, MAX_CALIBRATION_REMINDER_DAYS);

        config.set_reminder_days(14); // Valid
        assert_eq!(config.reminder_days, 14);
    }

    #[test]
    fn test_manager_new() {
        let manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));

        assert_eq!(manager.record_count(), 0);
        assert!(manager.config().enabled);
    }

    #[test]
    fn test_manager_record_calibration() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
        manager.set_auto_save(false);

        manager.record_calibration(
            "device1".to_string(),
            "Stages Power".to_string(),
            Protocol::BleCyclingPower,
            Some(100),
            true,
        );

        assert_eq!(manager.record_count(), 1);
        assert!(manager.has_been_calibrated("device1"));

        let record = manager.get_record("device1").unwrap();
        assert_eq!(record.offset_value, Some(100));
        assert!(record.was_successful);
    }

    #[test]
    fn test_manager_is_calibration_due() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
        manager.set_auto_save(false);

        // Never calibrated, should be due (if remind_never_calibrated is true)
        assert!(manager.is_calibration_due("unknown_device"));

        // Just calibrated, should not be due
        manager.record_calibration(
            "device1".to_string(),
            "Stages".to_string(),
            Protocol::BleCyclingPower,
            None,
            true,
        );
        assert!(!manager.is_calibration_due("device1"));
    }

    #[test]
    fn test_manager_check_for_reminders() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
        manager.set_auto_save(false);

        let power_meters = vec![
            (
                "device1".to_string(),
                "Stages Power".to_string(),
                Protocol::BleCyclingPower,
            ),
        ];

        // Never calibrated - should get a reminder
        let reminders = manager.check_for_reminders(&power_meters);
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].reminder_type, CalibrationReminderType::NeverCalibrated);
    }

    #[test]
    fn test_manager_dismiss_reminder() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
        manager.set_auto_save(false);

        let power_meters = vec![
            (
                "device1".to_string(),
                "Stages Power".to_string(),
                Protocol::BleCyclingPower,
            ),
        ];

        // Get initial reminder
        let reminders = manager.check_for_reminders(&power_meters);
        assert_eq!(reminders.len(), 1);

        // Dismiss it
        manager.dismiss_reminder("device1");

        // Should not get reminder again
        let reminders = manager.check_for_reminders(&power_meters);
        assert!(reminders.is_empty());
    }

    #[test]
    fn test_manager_remove_record() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
        manager.set_auto_save(false);

        manager.record_calibration(
            "device1".to_string(),
            "Stages".to_string(),
            Protocol::BleCyclingPower,
            None,
            true,
        );

        assert!(manager.has_been_calibrated("device1"));

        manager.remove_record("device1");

        assert!(!manager.has_been_calibrated("device1"));
    }

    #[test]
    fn test_manager_clear() {
        let mut manager = CalibrationManager::with_path(PathBuf::from("/tmp/test_cal.json"));
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
    }

    #[test]
    fn test_is_calibratable_sensor() {
        assert!(is_calibratable_sensor(SensorType::PowerMeter));
        assert!(is_calibratable_sensor(SensorType::Trainer));
        assert!(is_calibratable_sensor(SensorType::SmartTrainer));
        assert!(!is_calibratable_sensor(SensorType::HeartRate));
        assert!(!is_calibratable_sensor(SensorType::Cadence));
    }

    #[test]
    fn test_reminder_short_message() {
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
        assert!(overdue.short_message().contains("overdue"));
    }
}
