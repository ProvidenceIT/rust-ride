//! Sensor device cache for fast reconnection.
//!
//! Caches previously connected device information to enable faster reconnection
//! without requiring full discovery. Stores device addresses, names, and protocols
//! for known sensors.

use crate::sensors::types::{Protocol, SensorType};
use crate::storage::config::get_data_dir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Default cache file name.
const CACHE_FILE_NAME: &str = "sensor_cache.json";

/// Maximum age for cached sensors before they're considered stale (7 days).
const MAX_CACHE_AGE_DAYS: i64 = 7;

/// Cached sensor information for fast reconnection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSensor {
    /// Device address/identifier (BLE address or ANT+ device ID).
    pub device_id: String,
    /// User-friendly name from advertisement.
    pub name: String,
    /// Type of sensor (trainer, power meter, heart rate, etc.).
    pub sensor_type: SensorType,
    /// Communication protocol (BLE FTMS, BLE HR, ANT+ Power, etc.).
    pub protocol: Protocol,
    /// When the sensor was last successfully connected.
    pub last_connected: DateTime<Utc>,
    /// Number of successful connections (for prioritization).
    pub connection_count: u32,
    /// Optional user-assigned nickname.
    pub nickname: Option<String>,
    /// Whether this is a preferred/primary sensor.
    pub is_preferred: bool,
}

impl CachedSensor {
    /// Create a new cached sensor entry.
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
            last_connected: Utc::now(),
            connection_count: 1,
            nickname: None,
            is_preferred: false,
        }
    }

    /// Update the last connected timestamp and increment connection count.
    pub fn mark_connected(&mut self) {
        self.last_connected = Utc::now();
        self.connection_count = self.connection_count.saturating_add(1);
    }

    /// Check if this cached sensor is stale (not seen recently).
    pub fn is_stale(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.last_connected);
        age.num_days() > MAX_CACHE_AGE_DAYS
    }

    /// Get the display name (nickname if set, otherwise device name).
    pub fn display_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.name)
    }
}

/// Sensor cache manager for storing and retrieving known sensors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorCache {
    /// Cached sensors indexed by device ID.
    #[serde(default)]
    sensors: HashMap<String, CachedSensor>,
    /// Cache file path (not serialized).
    #[serde(skip)]
    cache_path: PathBuf,
    /// Whether the cache has unsaved changes.
    #[serde(skip)]
    dirty: bool,
}

impl Default for SensorCache {
    fn default() -> Self {
        Self {
            sensors: HashMap::new(),
            cache_path: get_cache_path(),
            dirty: false,
        }
    }
}

impl SensorCache {
    /// Create a new empty sensor cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sensor cache with a custom path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            sensors: HashMap::new(),
            cache_path: path,
            dirty: false,
        }
    }

    /// Load the sensor cache from disk.
    ///
    /// If the cache file doesn't exist or is invalid, returns an empty cache.
    pub fn load() -> Self {
        Self::load_from_path(get_cache_path())
    }

    /// Load the sensor cache from a specific path.
    pub fn load_from_path(path: PathBuf) -> Self {
        if !path.exists() {
            tracing::debug!("Sensor cache file not found, using empty cache");
            return Self::with_path(path);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<SensorCache>(&content) {
                Ok(mut cache) => {
                    cache.cache_path = path;
                    cache.dirty = false;
                    tracing::info!("Loaded {} sensors from cache", cache.sensors.len());
                    cache
                }
                Err(e) => {
                    tracing::warn!("Failed to parse sensor cache: {}", e);
                    Self::with_path(path)
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read sensor cache: {}", e);
                Self::with_path(path)
            }
        }
    }

    /// Save the sensor cache to disk.
    pub fn save(&mut self) -> Result<(), CacheError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CacheError::IoError(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CacheError::SerializeError(e.to_string()))?;

        std::fs::write(&self.cache_path, content)
            .map_err(|e| CacheError::IoError(e.to_string()))?;

        self.dirty = false;
        tracing::debug!("Saved {} sensors to cache", self.sensors.len());

        Ok(())
    }

    /// Add or update a sensor in the cache.
    pub fn cache_sensor(
        &mut self,
        device_id: String,
        name: String,
        sensor_type: SensorType,
        protocol: Protocol,
    ) {
        if let Some(existing) = self.sensors.get_mut(&device_id) {
            // Update existing entry
            existing.mark_connected();
            if existing.name != name {
                existing.name = name;
            }
            tracing::debug!(
                "Updated cached sensor: {} ({})",
                existing.display_name(),
                device_id
            );
        } else {
            // Add new entry
            let sensor = CachedSensor::new(device_id.clone(), name.clone(), sensor_type, protocol);
            tracing::info!("Caching new sensor: {} ({})", name, device_id);
            self.sensors.insert(device_id, sensor);
        }
        self.dirty = true;
    }

    /// Get a cached sensor by device ID.
    pub fn get(&self, device_id: &str) -> Option<&CachedSensor> {
        self.sensors.get(device_id)
    }

    /// Get a mutable reference to a cached sensor.
    pub fn get_mut(&mut self, device_id: &str) -> Option<&mut CachedSensor> {
        self.dirty = true;
        self.sensors.get_mut(device_id)
    }

    /// Check if a device is in the cache.
    pub fn contains(&self, device_id: &str) -> bool {
        self.sensors.contains_key(device_id)
    }

    /// Remove a sensor from the cache.
    pub fn remove(&mut self, device_id: &str) -> Option<CachedSensor> {
        let removed = self.sensors.remove(device_id);
        if removed.is_some() {
            self.dirty = true;
        }
        removed
    }

    /// Get all cached sensors.
    pub fn all_sensors(&self) -> impl Iterator<Item = &CachedSensor> {
        self.sensors.values()
    }

    /// Get cached sensors of a specific type.
    pub fn sensors_of_type(&self, sensor_type: SensorType) -> Vec<&CachedSensor> {
        self.sensors
            .values()
            .filter(|s| s.sensor_type == sensor_type)
            .collect()
    }

    /// Get cached sensors sorted by last connected (most recent first).
    pub fn recent_sensors(&self) -> Vec<&CachedSensor> {
        let mut sensors: Vec<_> = self.sensors.values().collect();
        sensors.sort_by(|a, b| b.last_connected.cmp(&a.last_connected));
        sensors
    }

    /// Get preferred sensors (marked as primary).
    pub fn preferred_sensors(&self) -> Vec<&CachedSensor> {
        self.sensors.values().filter(|s| s.is_preferred).collect()
    }

    /// Get sensors that should be prioritized for reconnection.
    ///
    /// Returns sensors sorted by priority:
    /// 1. Preferred sensors first
    /// 2. Then by connection count (most used)
    /// 3. Then by last connected (most recent)
    pub fn reconnection_priority(&self) -> Vec<&CachedSensor> {
        let mut sensors: Vec<_> = self.sensors.values().filter(|s| !s.is_stale()).collect();
        sensors.sort_by(|a, b| {
            // Preferred sensors first
            match (a.is_preferred, b.is_preferred) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Then by connection count (descending)
                    match b.connection_count.cmp(&a.connection_count) {
                        std::cmp::Ordering::Equal => {
                            // Then by last connected (descending)
                            b.last_connected.cmp(&a.last_connected)
                        }
                        other => other,
                    }
                }
            }
        });
        sensors
    }

    /// Get the number of cached sensors.
    pub fn len(&self) -> usize {
        self.sensors.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }

    /// Remove stale sensors from the cache.
    ///
    /// Returns the number of sensors removed.
    pub fn prune_stale(&mut self) -> usize {
        let stale_ids: Vec<_> = self
            .sensors
            .iter()
            .filter(|(_, s)| s.is_stale())
            .map(|(id, _)| id.clone())
            .collect();

        let count = stale_ids.len();
        for id in stale_ids {
            self.sensors.remove(&id);
        }

        if count > 0 {
            self.dirty = true;
            tracing::info!("Pruned {} stale sensors from cache", count);
        }

        count
    }

    /// Clear all cached sensors.
    pub fn clear(&mut self) {
        if !self.sensors.is_empty() {
            self.sensors.clear();
            self.dirty = true;
        }
    }

    /// Set a sensor as preferred.
    pub fn set_preferred(&mut self, device_id: &str, preferred: bool) -> bool {
        if let Some(sensor) = self.sensors.get_mut(device_id) {
            sensor.is_preferred = preferred;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Set a nickname for a sensor.
    pub fn set_nickname(&mut self, device_id: &str, nickname: Option<String>) -> bool {
        if let Some(sensor) = self.sensors.get_mut(device_id) {
            sensor.nickname = nickname;
            self.dirty = true;
            true
        } else {
            false
        }
    }
}

/// Get the default cache file path.
pub fn get_cache_path() -> PathBuf {
    get_data_dir().join(CACHE_FILE_NAME)
}

/// Errors that can occur with the sensor cache.
#[derive(Debug, Error)]
pub enum CacheError {
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
    fn test_cached_sensor_new() {
        let sensor = CachedSensor::new(
            "00:11:22:33:44:55".to_string(),
            "My Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert_eq!(sensor.device_id, "00:11:22:33:44:55");
        assert_eq!(sensor.name, "My Trainer");
        assert_eq!(sensor.sensor_type, SensorType::Trainer);
        assert_eq!(sensor.protocol, Protocol::BleFtms);
        assert_eq!(sensor.connection_count, 1);
        assert!(!sensor.is_preferred);
        assert!(sensor.nickname.is_none());
    }

    #[test]
    fn test_cached_sensor_mark_connected() {
        let mut sensor = CachedSensor::new(
            "00:11:22:33:44:55".to_string(),
            "My Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        let original_time = sensor.last_connected;
        sensor.mark_connected();

        assert_eq!(sensor.connection_count, 2);
        assert!(sensor.last_connected >= original_time);
    }

    #[test]
    fn test_cached_sensor_display_name() {
        let mut sensor = CachedSensor::new(
            "00:11:22:33:44:55".to_string(),
            "KICKR CORE 1234".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        // Without nickname, uses device name
        assert_eq!(sensor.display_name(), "KICKR CORE 1234");

        // With nickname, uses nickname
        sensor.nickname = Some("Living Room Trainer".to_string());
        assert_eq!(sensor.display_name(), "Living Room Trainer");
    }

    #[test]
    fn test_sensor_cache_basic_operations() {
        let mut cache = SensorCache::new();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer 1".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("device1"));
        assert!(!cache.contains("device2"));

        let sensor = cache.get("device1").unwrap();
        assert_eq!(sensor.name, "Trainer 1");
    }

    #[test]
    fn test_sensor_cache_update_existing() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer 1".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        let count_before = cache.get("device1").unwrap().connection_count;

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer 1 Updated".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        let sensor = cache.get("device1").unwrap();
        assert_eq!(sensor.name, "Trainer 1 Updated");
        assert_eq!(sensor.connection_count, count_before + 1);
    }

    #[test]
    fn test_sensor_cache_remove() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer 1".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert!(cache.contains("device1"));

        let removed = cache.remove("device1");
        assert!(removed.is_some());
        assert!(!cache.contains("device1"));
        assert!(cache.is_empty());
    }

    #[test]
    fn test_sensor_cache_sensors_of_type() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "trainer1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        cache.cache_sensor(
            "hr1".to_string(),
            "HR Monitor".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );
        cache.cache_sensor(
            "power1".to_string(),
            "Power Meter".to_string(),
            SensorType::PowerMeter,
            Protocol::BleCyclingPower,
        );

        let trainers = cache.sensors_of_type(SensorType::Trainer);
        assert_eq!(trainers.len(), 1);
        assert_eq!(trainers[0].device_id, "trainer1");

        let hr_sensors = cache.sensors_of_type(SensorType::HeartRate);
        assert_eq!(hr_sensors.len(), 1);
    }

    #[test]
    fn test_sensor_cache_preferred() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer 1".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        cache.cache_sensor(
            "device2".to_string(),
            "Trainer 2".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert!(cache.set_preferred("device1", true));
        assert!(cache.get("device1").unwrap().is_preferred);
        assert!(!cache.get("device2").unwrap().is_preferred);

        let preferred = cache.preferred_sensors();
        assert_eq!(preferred.len(), 1);
        assert_eq!(preferred[0].device_id, "device1");
    }

    #[test]
    fn test_sensor_cache_nickname() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "device1".to_string(),
            "KICKR CORE 1234".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );

        assert!(cache.set_nickname("device1", Some("Pain Cave".to_string())));

        let sensor = cache.get("device1").unwrap();
        assert_eq!(sensor.nickname, Some("Pain Cave".to_string()));
        assert_eq!(sensor.display_name(), "Pain Cave");
    }

    #[test]
    fn test_sensor_cache_clear() {
        let mut cache = SensorCache::new();

        cache.cache_sensor(
            "device1".to_string(),
            "Trainer".to_string(),
            SensorType::Trainer,
            Protocol::BleFtms,
        );
        cache.cache_sensor(
            "device2".to_string(),
            "HR".to_string(),
            SensorType::HeartRate,
            Protocol::BleHeartRate,
        );

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }
}
