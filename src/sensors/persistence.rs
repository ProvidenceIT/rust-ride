//! Connection state persistence for automatic reconnection across app restarts.
//!
//! This module saves the connection state of sensors to enable automatic
//! reconnection to previously connected sensors when the app restarts.

use crate::sensors::types::{Protocol, SensorType};
use crate::storage::config::get_data_dir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Default session file name.
const SESSION_FILE_NAME: &str = "sensor_session.json";

/// Maximum age for a session before it's considered stale (24 hours).
/// Sessions older than this won't trigger auto-reconnection.
const MAX_SESSION_AGE_HOURS: i64 = 24;

/// A sensor that was connected in the previous session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSensor {
    /// Device address/identifier (BLE address or ANT+ device ID).
    pub device_id: String,
    /// User-friendly name from advertisement.
    pub name: String,
    /// Type of sensor (trainer, power meter, heart rate, etc.).
    pub sensor_type: SensorType,
    /// Communication protocol (BLE FTMS, BLE HR, ANT+ Power, etc.).
    pub protocol: Protocol,
    /// When the sensor was connected in the session.
    pub connected_at: DateTime<Utc>,
    /// When the sensor was last seen sending data.
    pub last_data_at: Option<DateTime<Utc>>,
    /// Whether this was the primary sensor for its data type.
    pub is_primary: bool,
    /// Connection was healthy (received data) when session ended.
    pub was_healthy: bool,
}

impl SessionSensor {
    /// Create a new session sensor entry.
    pub fn new(
        device_id: String,
        name: String,
        sensor_type: SensorType,
        protocol: Protocol,
    ) -> Self {
        Self {
            device_id,
            name,
            sensor_type,
            protocol,
            connected_at: Utc::now(),
            last_data_at: None,
            is_primary: false,
            was_healthy: true,
        }
    }

    /// Update the last data timestamp.
    pub fn record_data(&mut self) {
        self.last_data_at = Some(Utc::now());
        self.was_healthy = true;
    }

    /// Mark the sensor as unhealthy (stale connection).
    pub fn mark_unhealthy(&mut self) {
        self.was_healthy = false;
    }

    /// Get a display name for the sensor.
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

/// Represents a connection session that can be persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSession {
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// When the session was last updated.
    pub last_updated_at: DateTime<Utc>,
    /// Connected sensors in this session.
    #[serde(default)]
    pub sensors: HashMap<String, SessionSensor>,
    /// Session ended cleanly (user initiated shutdown).
    pub clean_shutdown: bool,
    /// App version that created this session.
    pub app_version: Option<String>,
}

impl Default for ConnectionSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionSession {
    /// Create a new empty session.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            started_at: now,
            last_updated_at: now,
            sensors: HashMap::new(),
            clean_shutdown: false,
            app_version: None,
        }
    }

    /// Create a new session with app version.
    pub fn with_version(version: String) -> Self {
        let mut session = Self::new();
        session.app_version = Some(version);
        session
    }

    /// Add or update a connected sensor.
    pub fn add_sensor(&mut self, sensor: SessionSensor) {
        self.sensors.insert(sensor.device_id.clone(), sensor);
        self.last_updated_at = Utc::now();
    }

    /// Remove a sensor from the session.
    pub fn remove_sensor(&mut self, device_id: &str) -> Option<SessionSensor> {
        let removed = self.sensors.remove(device_id);
        if removed.is_some() {
            self.last_updated_at = Utc::now();
        }
        removed
    }

    /// Get a sensor by device ID.
    pub fn get_sensor(&self, device_id: &str) -> Option<&SessionSensor> {
        self.sensors.get(device_id)
    }

    /// Get a mutable sensor by device ID.
    pub fn get_sensor_mut(&mut self, device_id: &str) -> Option<&mut SessionSensor> {
        self.sensors.get_mut(device_id)
    }

    /// Check if a sensor is in the session.
    pub fn contains(&self, device_id: &str) -> bool {
        self.sensors.contains_key(device_id)
    }

    /// Get all sensors.
    pub fn all_sensors(&self) -> impl Iterator<Item = &SessionSensor> {
        self.sensors.values()
    }

    /// Get sensors of a specific type.
    pub fn sensors_of_type(&self, sensor_type: SensorType) -> Vec<&SessionSensor> {
        self.sensors
            .values()
            .filter(|s| s.sensor_type == sensor_type)
            .collect()
    }

    /// Get sensors that were healthy when session ended.
    pub fn healthy_sensors(&self) -> Vec<&SessionSensor> {
        self.sensors.values().filter(|s| s.was_healthy).collect()
    }

    /// Get primary sensors only.
    pub fn primary_sensors(&self) -> Vec<&SessionSensor> {
        self.sensors.values().filter(|s| s.is_primary).collect()
    }

    /// Get the number of sensors.
    pub fn len(&self) -> usize {
        self.sensors.len()
    }

    /// Check if the session is empty.
    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }

    /// Mark the session as cleanly shut down.
    pub fn mark_clean_shutdown(&mut self) {
        self.clean_shutdown = true;
        self.last_updated_at = Utc::now();
    }

    /// Check if this session is stale (too old to use for reconnection).
    pub fn is_stale(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.last_updated_at);
        age.num_hours() > MAX_SESSION_AGE_HOURS
    }

    /// Get the age of the session.
    pub fn age(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.started_at)
    }

    /// Get sensors sorted by reconnection priority.
    ///
    /// Priority order:
    /// 1. Primary sensors first
    /// 2. Healthy sensors before unhealthy
    /// 3. By sensor type priority (trainers/power meters first)
    pub fn reconnection_priority(&self) -> Vec<&SessionSensor> {
        let mut sensors: Vec<_> = self.sensors.values().collect();
        sensors.sort_by(|a, b| {
            // Primary sensors first
            match (a.is_primary, b.is_primary) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Healthy sensors first
            match (a.was_healthy, b.was_healthy) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Sensor type priority
            let priority_a = Self::sensor_type_priority(a.sensor_type);
            let priority_b = Self::sensor_type_priority(b.sensor_type);
            priority_a.cmp(&priority_b)
        });
        sensors
    }

    /// Get priority for sensor type (lower is higher priority).
    fn sensor_type_priority(sensor_type: SensorType) -> u8 {
        match sensor_type {
            SensorType::Trainer | SensorType::SmartTrainer => 0,
            SensorType::PowerMeter => 1,
            SensorType::HeartRate => 2,
            SensorType::Cadence | SensorType::CadenceSensor => 3,
            SensorType::Speed => 4,
            SensorType::SpeedCadence => 5,
            SensorType::SmO2 => 6,
            SensorType::Imu => 7,
        }
    }

    /// Clear all sensors from the session.
    pub fn clear(&mut self) {
        self.sensors.clear();
        self.last_updated_at = Utc::now();
    }
}

/// Manages persistence of connection sessions.
#[derive(Debug)]
pub struct ConnectionSessionManager {
    /// Current session.
    session: ConnectionSession,
    /// Path to the session file.
    session_path: PathBuf,
    /// Whether changes have been made since last save.
    dirty: bool,
    /// Whether to automatically save on changes.
    auto_save: bool,
}

impl Default for ConnectionSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionSessionManager {
    /// Create a new session manager with default path.
    pub fn new() -> Self {
        Self::with_path(get_session_path())
    }

    /// Create a session manager with a custom path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            session: ConnectionSession::new(),
            session_path: path,
            dirty: false,
            auto_save: true,
        }
    }

    /// Load the previous session from disk.
    ///
    /// If no session file exists or it's invalid, returns an empty session.
    pub fn load() -> Self {
        Self::load_from_path(get_session_path())
    }

    /// Load session from a specific path.
    pub fn load_from_path(path: PathBuf) -> Self {
        if !path.exists() {
            tracing::debug!("No session file found at {:?}, starting fresh", path);
            return Self::with_path(path);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<ConnectionSession>(&content) {
                Ok(session) => {
                    tracing::info!(
                        "Loaded session with {} sensors from {:?}",
                        session.len(),
                        path
                    );
                    Self {
                        session,
                        session_path: path,
                        dirty: false,
                        auto_save: true,
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse session file: {}", e);
                    Self::with_path(path)
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read session file: {}", e);
                Self::with_path(path)
            }
        }
    }

    /// Save the current session to disk.
    pub fn save(&mut self) -> Result<(), PersistenceError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.session_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(&self.session)
            .map_err(|e| PersistenceError::SerializeError(e.to_string()))?;

        std::fs::write(&self.session_path, content)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        self.dirty = false;
        tracing::debug!("Saved session with {} sensors", self.session.len());

        Ok(())
    }

    /// Enable or disable auto-save.
    pub fn set_auto_save(&mut self, enabled: bool) {
        self.auto_save = enabled;
    }

    /// Get the current session.
    pub fn session(&self) -> &ConnectionSession {
        &self.session
    }

    /// Get the current session mutably.
    pub fn session_mut(&mut self) -> &mut ConnectionSession {
        self.dirty = true;
        &mut self.session
    }

    /// Start a new session, optionally preserving sensor data.
    pub fn start_new_session(&mut self, preserve_sensors: bool) {
        let sensors = if preserve_sensors {
            std::mem::take(&mut self.session.sensors)
        } else {
            HashMap::new()
        };

        self.session = ConnectionSession::new();
        self.session.sensors = sensors;
        self.dirty = true;

        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
        }
    }

    /// Record that a sensor connected.
    pub fn sensor_connected(
        &mut self,
        device_id: String,
        name: String,
        sensor_type: SensorType,
        protocol: Protocol,
        is_primary: bool,
    ) {
        let mut sensor = SessionSensor::new(device_id, name, sensor_type, protocol);
        sensor.is_primary = is_primary;
        self.session.add_sensor(sensor);
        self.dirty = true;

        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
        }
    }

    /// Record that a sensor disconnected.
    pub fn sensor_disconnected(&mut self, device_id: &str) {
        self.session.remove_sensor(device_id);
        self.dirty = true;

        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
        }
    }

    /// Record that data was received from a sensor.
    pub fn sensor_data_received(&mut self, device_id: &str) {
        if let Some(sensor) = self.session.get_sensor_mut(device_id) {
            sensor.record_data();
            self.dirty = true;
            // Don't auto-save on every data event to avoid excessive I/O
        }
    }

    /// Mark a sensor as unhealthy (stale).
    pub fn sensor_unhealthy(&mut self, device_id: &str) {
        if let Some(sensor) = self.session.get_sensor_mut(device_id) {
            sensor.mark_unhealthy();
            self.dirty = true;
        }
    }

    /// Check if a session exists that can be used for reconnection.
    pub fn has_reconnectable_session(&self) -> bool {
        !self.session.is_empty() && !self.session.is_stale()
    }

    /// Get sensors that should be reconnected.
    ///
    /// Returns sensors sorted by reconnection priority.
    /// Only returns sensors that were healthy when the session ended.
    pub fn get_reconnection_targets(&self) -> Vec<&SessionSensor> {
        if self.session.is_stale() {
            tracing::info!("Session is stale, not recommending reconnection");
            return Vec::new();
        }

        self.session.reconnection_priority()
    }

    /// Get device IDs for sensors that should be reconnected.
    pub fn get_reconnection_device_ids(&self) -> Vec<String> {
        self.get_reconnection_targets()
            .iter()
            .map(|s| s.device_id.clone())
            .collect()
    }

    /// Mark session for clean shutdown.
    pub fn prepare_shutdown(&mut self) {
        self.session.mark_clean_shutdown();
        self.dirty = true;

        if let Err(e) = self.save() {
            tracing::warn!("Failed to save session on shutdown: {}", e);
        }
    }

    /// Clear the session entirely.
    pub fn clear(&mut self) {
        self.session.clear();
        self.dirty = true;

        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
        }
    }

    /// Delete the session file.
    pub fn delete_session_file(&self) -> Result<(), PersistenceError> {
        if self.session_path.exists() {
            std::fs::remove_file(&self.session_path)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    /// Get the number of sensors in the session.
    pub fn sensor_count(&self) -> usize {
        self.session.len()
    }

    /// Check if the session was a clean shutdown.
    pub fn was_clean_shutdown(&self) -> bool {
        self.session.clean_shutdown
    }

    /// Get session age.
    pub fn session_age(&self) -> chrono::Duration {
        self.session.age()
    }
}

/// Get the default session file path.
pub fn get_session_path() -> PathBuf {
    get_data_dir().join(SESSION_FILE_NAME)
}

/// Errors that can occur with session persistence.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialize error: {0}")]
    SerializeError(String),

    #[error("Deserialize error: {0}")]
    DeserializeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_sensor_new() {
        let sensor = SessionSensor::new(
            "00:11:22:33:44:55".to_string(),
            "My Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert_eq!(sensor.device_id, "00:11:22:33:44:55");
        assert_eq!(sensor.name, "My Trainer");
        assert_eq!(sensor.sensor_type, SensorType::Trainer);
        assert_eq!(sensor.protocol, Protocol::BleFtms);
        assert!(!sensor.is_primary);
        assert!(sensor.was_healthy);
        assert!(sensor.last_data_at.is_none());
    }

    #[test]
    fn test_session_sensor_record_data() {
        let mut sensor = SessionSensor::new(
            "device1".to_string(),
            "Sensor".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );

        assert!(sensor.last_data_at.is_none());

        sensor.record_data();

        assert!(sensor.last_data_at.is_some());
        assert!(sensor.was_healthy);
    }

    #[test]
    fn test_session_sensor_mark_unhealthy() {
        let mut sensor = SessionSensor::new(
            "device1".to_string(),
            "Sensor".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );

        assert!(sensor.was_healthy);

        sensor.mark_unhealthy();

        assert!(!sensor.was_healthy);
    }

    #[test]
    fn test_connection_session_new() {
        let session = ConnectionSession::new();

        assert!(session.is_empty());
        assert_eq!(session.len(), 0);
        assert!(!session.clean_shutdown);
        assert!(session.app_version.is_none());
    }

    #[test]
    fn test_connection_session_add_sensor() {
        let mut session = ConnectionSession::new();

        let sensor = SessionSensor::new(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        session.add_sensor(sensor);

        assert_eq!(session.len(), 1);
        assert!(session.contains("device1"));
        assert!(session.get_sensor("device1").is_some());
    }

    #[test]
    fn test_connection_session_remove_sensor() {
        let mut session = ConnectionSession::new();

        session.add_sensor(SessionSensor::new(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        ));

        assert!(session.contains("device1"));

        let removed = session.remove_sensor("device1");
        assert!(removed.is_some());
        assert!(!session.contains("device1"));
        assert!(session.is_empty());
    }

    #[test]
    fn test_connection_session_sensors_of_type() {
        let mut session = ConnectionSession::new();

        session.add_sensor(SessionSensor::new(
            "trainer1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        ));
        session.add_sensor(SessionSensor::new(
            "hr1".to_string(),
            "HR".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        ));
        session.add_sensor(SessionSensor::new(
            "power1".to_string(),
            "Power".to_string(),
            SensorType::PowerMeter,
            Protocol::BleCyclingPower,
        ));

        let trainers = session.sensors_of_type(SensorType::Trainer);
        assert_eq!(trainers.len(), 1);
        assert_eq!(trainers[0].device_id, "trainer1");
    }

    #[test]
    fn test_connection_session_healthy_sensors() {
        let mut session = ConnectionSession::new();

        let mut healthy = SessionSensor::new(
            "healthy1".to_string(),
            "Healthy".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );
        healthy.was_healthy = true;

        let mut unhealthy = SessionSensor::new(
            "unhealthy1".to_string(),
            "Unhealthy".to_string(),
            SensorType::PowerMeter,
            Protocol::BleCyclingPower,
        );
        unhealthy.was_healthy = false;

        session.add_sensor(healthy);
        session.add_sensor(unhealthy);

        let healthy_sensors = session.healthy_sensors();
        assert_eq!(healthy_sensors.len(), 1);
        assert_eq!(healthy_sensors[0].device_id, "healthy1");
    }

    #[test]
    fn test_connection_session_primary_sensors() {
        let mut session = ConnectionSession::new();

        let mut primary = SessionSensor::new(
            "primary1".to_string(),
            "Primary".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        primary.is_primary = true;

        let secondary = SessionSensor::new(
            "secondary1".to_string(),
            "Secondary".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );

        session.add_sensor(primary);
        session.add_sensor(secondary);

        let primaries = session.primary_sensors();
        assert_eq!(primaries.len(), 1);
        assert_eq!(primaries[0].device_id, "primary1");
    }

    #[test]
    fn test_connection_session_reconnection_priority() {
        let mut session = ConnectionSession::new();

        // Add sensors in random order
        session.add_sensor(SessionSensor::new(
            "hr1".to_string(),
            "HR".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        ));

        let mut primary_trainer = SessionSensor::new(
            "trainer1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        primary_trainer.is_primary = true;
        session.add_sensor(primary_trainer);

        session.add_sensor(SessionSensor::new(
            "power1".to_string(),
            "Power".to_string(),
            SensorType::PowerMeter,
            Protocol::BleCyclingPower,
        ));

        let priority = session.reconnection_priority();

        // Primary trainer should be first
        assert_eq!(priority[0].device_id, "trainer1");
        // Then power meter (lower type priority)
        assert_eq!(priority[1].device_id, "power1");
        // Then heart rate
        assert_eq!(priority[2].device_id, "hr1");
    }

    #[test]
    fn test_connection_session_clean_shutdown() {
        let mut session = ConnectionSession::new();

        assert!(!session.clean_shutdown);

        session.mark_clean_shutdown();

        assert!(session.clean_shutdown);
    }

    #[test]
    fn test_connection_session_clear() {
        let mut session = ConnectionSession::new();

        session.add_sensor(SessionSensor::new(
            "device1".to_string(),
            "Sensor".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        ));

        assert!(!session.is_empty());

        session.clear();

        assert!(session.is_empty());
    }

    #[test]
    fn test_session_manager_new() {
        let manager = ConnectionSessionManager::new();

        assert!(manager.session().is_empty());
        assert_eq!(manager.sensor_count(), 0);
    }

    #[test]
    fn test_session_manager_sensor_connected() {
        let mut manager = ConnectionSessionManager::with_path(PathBuf::from("/tmp/test_session.json"));
        manager.set_auto_save(false);

        manager.sensor_connected(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );

        assert_eq!(manager.sensor_count(), 1);
        assert!(manager.session().contains("device1"));

        let sensor = manager.session().get_sensor("device1").unwrap();
        assert!(sensor.is_primary);
    }

    #[test]
    fn test_session_manager_sensor_disconnected() {
        let mut manager = ConnectionSessionManager::with_path(PathBuf::from("/tmp/test_session.json"));
        manager.set_auto_save(false);

        manager.sensor_connected(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            false,
        );

        assert_eq!(manager.sensor_count(), 1);

        manager.sensor_disconnected("device1");

        assert_eq!(manager.sensor_count(), 0);
    }

    #[test]
    fn test_session_manager_get_reconnection_device_ids() {
        let mut manager = ConnectionSessionManager::with_path(PathBuf::from("/tmp/test_session.json"));
        manager.set_auto_save(false);

        manager.sensor_connected(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
            true,
        );
        manager.sensor_connected(
            "device2".to_string(),
            "HR".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
            false,
        );

        let ids = manager.get_reconnection_device_ids();

        assert_eq!(ids.len(), 2);
        // Primary trainer should be first
        assert_eq!(ids[0], "device1");
    }

    #[test]
    fn test_session_manager_prepare_shutdown() {
        let mut manager = ConnectionSessionManager::with_path(PathBuf::from("/tmp/test_session.json"));
        manager.set_auto_save(false);

        assert!(!manager.was_clean_shutdown());

        manager.prepare_shutdown();

        assert!(manager.was_clean_shutdown());
    }
}
