//! Sensor conflict detection and resolution.
//!
//! Detects when multiple sensors provide the same data type (e.g., two power meters)
//! and provides mechanisms to alert the user and select a primary sensor.
//!
//! Conflicts can occur when:
//! - Two power meters are connected (e.g., pedals + trainer power)
//! - A trainer and standalone power meter both provide power data
//! - Multiple heart rate monitors are connected
//! - Multiple cadence sources are available

use crate::sensors::types::{DiscoveredSensor, Protocol, SensorProtocol, SensorState, SensorType};
use crate::storage::config::get_data_dir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use thiserror::Error;

/// Types of data that sensors can provide.
///
/// This represents the actual data output rather than the sensor type,
/// since some sensors (like trainers) can provide multiple data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    /// Power output in watts
    Power,
    /// Heart rate in BPM
    HeartRate,
    /// Cadence in RPM
    Cadence,
    /// Speed in km/h
    Speed,
    /// Controllable trainer resistance
    TrainerControl,
}

impl DataType {
    /// Get all data types that a sensor type can provide.
    pub fn from_sensor_type(sensor_type: SensorType) -> Vec<DataType> {
        match sensor_type {
            SensorType::Trainer | SensorType::SmartTrainer => {
                vec![DataType::Power, DataType::Cadence, DataType::Speed, DataType::TrainerControl]
            }
            SensorType::PowerMeter => vec![DataType::Power, DataType::Cadence],
            SensorType::HeartRate => vec![DataType::HeartRate],
            SensorType::Cadence | SensorType::CadenceSensor => vec![DataType::Cadence],
            SensorType::Speed => vec![DataType::Speed],
            SensorType::SpeedCadence => vec![DataType::Speed, DataType::Cadence],
            SensorType::SmO2 => vec![], // SmO2 doesn't conflict with other data
            SensorType::Imu => vec![],  // IMU doesn't conflict
        }
    }

    /// Check if this data type is critical for cycling performance.
    ///
    /// Critical data types should always have a primary source selected.
    pub fn is_critical(&self) -> bool {
        matches!(self, DataType::Power | DataType::HeartRate | DataType::TrainerControl)
    }

    /// Get a human-readable name for the data type.
    pub fn display_name(&self) -> &'static str {
        match self {
            DataType::Power => "Power",
            DataType::HeartRate => "Heart Rate",
            DataType::Cadence => "Cadence",
            DataType::Speed => "Speed",
            DataType::TrainerControl => "Trainer Control",
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A sensor that can provide a specific data type.
#[derive(Debug, Clone)]
pub struct DataSource {
    /// The device ID of the sensor.
    pub device_id: String,
    /// The sensor's display name.
    pub name: String,
    /// The sensor type.
    pub sensor_type: SensorType,
    /// The protocol used (BLE or ANT+).
    pub protocol: SensorProtocol,
    /// Whether this sensor is currently connected.
    pub is_connected: bool,
    /// The data type this source provides.
    pub data_type: DataType,
    /// Whether this is the primary source for this data type.
    pub is_primary: bool,
}

impl DataSource {
    /// Create from a discovered sensor for a specific data type.
    pub fn from_discovered(sensor: &DiscoveredSensor, data_type: DataType) -> Self {
        Self {
            device_id: sensor.device_id.clone(),
            name: sensor.name.clone(),
            sensor_type: sensor.sensor_type,
            protocol: sensor.protocol.sensor_protocol(),
            is_connected: false,
            data_type,
            is_primary: false,
        }
    }

    /// Create from a sensor state for a specific data type.
    pub fn from_state(state: &SensorState, data_type: DataType) -> Self {
        Self {
            device_id: state.device_id.clone(),
            name: state.name.clone(),
            sensor_type: state.sensor_type,
            protocol: state.protocol.sensor_protocol(),
            is_connected: matches!(
                state.connection_state,
                crate::sensors::types::ConnectionState::Connected
            ),
            data_type,
            is_primary: state.is_primary,
        }
    }

    /// Get a display string for this source.
    pub fn display(&self) -> String {
        let protocol = match self.protocol {
            SensorProtocol::Ble => "BLE",
            SensorProtocol::AntPlus => "ANT+",
        };
        format!("{} ({}, {})", self.name, self.sensor_type, protocol)
    }
}

/// A conflict between multiple sensors providing the same data type.
#[derive(Debug, Clone)]
pub struct SensorConflict {
    /// The data type in conflict.
    pub data_type: DataType,
    /// The sensors providing this data type.
    pub sources: Vec<DataSource>,
    /// The currently selected primary source (if any).
    pub primary_device_id: Option<String>,
    /// When the conflict was detected.
    pub detected_at: std::time::Instant,
    /// Whether the user has been notified of this conflict.
    pub user_notified: bool,
    /// Whether this conflict has been resolved.
    pub is_resolved: bool,
}

impl SensorConflict {
    /// Create a new conflict.
    pub fn new(data_type: DataType, sources: Vec<DataSource>) -> Self {
        Self {
            data_type,
            sources,
            primary_device_id: None,
            detected_at: std::time::Instant::now(),
            user_notified: false,
            is_resolved: false,
        }
    }

    /// Get the number of sensors in this conflict.
    pub fn sensor_count(&self) -> usize {
        self.sources.len()
    }

    /// Check if this conflict needs user attention.
    pub fn needs_attention(&self) -> bool {
        !self.is_resolved && !self.user_notified
    }

    /// Get the primary source if set.
    pub fn primary_source(&self) -> Option<&DataSource> {
        self.primary_device_id.as_ref().and_then(|id| {
            self.sources.iter().find(|s| &s.device_id == id)
        })
    }

    /// Get the non-primary sources.
    pub fn secondary_sources(&self) -> Vec<&DataSource> {
        self.sources
            .iter()
            .filter(|s| Some(&s.device_id) != self.primary_device_id.as_ref())
            .collect()
    }

    /// Set the primary sensor for this conflict.
    pub fn set_primary(&mut self, device_id: &str) -> bool {
        if self.sources.iter().any(|s| s.device_id == device_id) {
            self.primary_device_id = Some(device_id.to_string());
            self.is_resolved = true;

            // Update source primary flags
            for source in &mut self.sources {
                source.is_primary = source.device_id == device_id;
            }

            true
        } else {
            false
        }
    }

    /// Clear the primary selection.
    pub fn clear_primary(&mut self) {
        self.primary_device_id = None;
        self.is_resolved = false;
        for source in &mut self.sources {
            source.is_primary = false;
        }
    }

    /// Add a source to this conflict.
    pub fn add_source(&mut self, source: DataSource) {
        if !self.sources.iter().any(|s| s.device_id == source.device_id) {
            self.sources.push(source);
            // New source means conflict may need re-resolution
            if self.sources.len() > 2 {
                self.user_notified = false;
            }
        }
    }

    /// Remove a source from this conflict.
    pub fn remove_source(&mut self, device_id: &str) -> bool {
        let initial_len = self.sources.len();
        self.sources.retain(|s| s.device_id != device_id);

        if self.sources.len() < initial_len {
            // If the primary was removed, clear resolution
            if self.primary_device_id.as_deref() == Some(device_id) {
                self.primary_device_id = None;
                self.is_resolved = false;
            }
            true
        } else {
            false
        }
    }

    /// Check if this is still a conflict (2+ sources).
    pub fn is_active(&self) -> bool {
        self.sources.len() >= 2
    }

    /// Mark the user as notified.
    pub fn mark_notified(&mut self) {
        self.user_notified = true;
    }

    /// Get a summary of this conflict.
    pub fn summary(&self) -> String {
        let sensor_names: Vec<&str> = self.sources.iter().map(|s| s.name.as_str()).collect();
        format!(
            "{} conflict: {} sensors ({})",
            self.data_type,
            self.sources.len(),
            sensor_names.join(", ")
        )
    }
}

/// Resolution strategy for conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    /// User manually selects the primary sensor.
    UserSelection,
    /// Automatically select based on sensor type priority.
    AutoPriority,
    /// Automatically select the first connected sensor.
    FirstConnected,
    /// Use the most recently connected sensor.
    MostRecent,
}

impl Default for ResolutionStrategy {
    fn default() -> Self {
        Self::UserSelection
    }
}

/// Priority order for automatic conflict resolution.
///
/// Lower numbers = higher priority.
fn sensor_type_priority(sensor_type: SensorType) -> u8 {
    match sensor_type {
        // Dedicated power meters are most accurate for power
        SensorType::PowerMeter => 1,
        // Smart trainers have good power accuracy
        SensorType::Trainer | SensorType::SmartTrainer => 2,
        // Dedicated HR monitors are most reliable
        SensorType::HeartRate => 1,
        // Dedicated cadence/speed sensors
        SensorType::Cadence | SensorType::CadenceSensor => 1,
        SensorType::Speed => 1,
        SensorType::SpeedCadence => 2,
        // Others
        SensorType::SmO2 => 5,
        SensorType::Imu => 5,
    }
}

/// Result of a primary sensor failover operation.
///
/// When the primary sensor for a data type disconnects and a secondary
/// sensor is available, the secondary is automatically promoted to primary.
/// This struct captures the details of that failover for user notification.
#[derive(Debug, Clone)]
pub struct FailoverResult {
    /// The data type that experienced failover.
    pub data_type: DataType,
    /// The device ID of the sensor that disconnected (former primary).
    pub from_device_id: String,
    /// The name of the sensor that disconnected.
    pub from_sensor_name: String,
    /// The device ID of the sensor that was promoted to primary.
    pub to_device_id: String,
    /// The name of the sensor that was promoted to primary.
    pub to_sensor_name: String,
}

impl FailoverResult {
    /// Get a human-readable message describing the failover.
    pub fn message(&self) -> String {
        format!(
            "{} source switched from {} to {}",
            self.data_type.display_name(),
            self.from_sensor_name,
            self.to_sensor_name
        )
    }
}

/// Configuration for the conflict detector.
#[derive(Debug, Clone)]
pub struct ConflictDetectorConfig {
    /// Resolution strategy to use.
    pub strategy: ResolutionStrategy,
    /// Whether to auto-resolve non-critical data types.
    pub auto_resolve_non_critical: bool,
    /// Whether to persist conflict resolutions.
    pub persist_resolutions: bool,
}

impl Default for ConflictDetectorConfig {
    fn default() -> Self {
        Self {
            strategy: ResolutionStrategy::UserSelection,
            auto_resolve_non_critical: true,
            persist_resolutions: true,
        }
    }
}

/// Detector for sensor data conflicts.
///
/// Tracks which sensors provide which data types and detects when
/// multiple sensors provide the same data type.
#[derive(Debug)]
pub struct ConflictDetector {
    /// Configuration.
    config: ConflictDetectorConfig,
    /// Active conflicts by data type.
    conflicts: HashMap<DataType, SensorConflict>,
    /// All registered sources by device ID.
    sources_by_device: HashMap<String, Vec<DataType>>,
    /// Persisted conflict preferences.
    preferences: ConflictPreferenceManager,
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictDetector {
    /// Create a new conflict detector with default configuration.
    pub fn new() -> Self {
        Self::with_config(ConflictDetectorConfig::default())
    }

    /// Create a conflict detector with custom configuration.
    pub fn with_config(config: ConflictDetectorConfig) -> Self {
        let preferences = if config.persist_resolutions {
            ConflictPreferenceManager::load()
        } else {
            ConflictPreferenceManager::new()
        };

        Self {
            config,
            conflicts: HashMap::new(),
            sources_by_device: HashMap::new(),
            preferences,
        }
    }

    /// Register a discovered sensor.
    ///
    /// Returns a list of new conflicts created by this sensor.
    pub fn register_sensor(&mut self, sensor: &DiscoveredSensor) -> Vec<DataType> {
        let data_types = DataType::from_sensor_type(sensor.sensor_type);
        let mut new_conflicts = Vec::new();

        for data_type in &data_types {
            let source = DataSource::from_discovered(sensor, *data_type);
            if self.add_source(source) {
                new_conflicts.push(*data_type);
            }
        }

        // Track which data types this device provides
        self.sources_by_device.insert(sensor.device_id.clone(), data_types);

        new_conflicts
    }

    /// Register a connected sensor from its state.
    ///
    /// Returns a list of new conflicts created by this sensor.
    pub fn register_sensor_state(&mut self, state: &SensorState) -> Vec<DataType> {
        let data_types = DataType::from_sensor_type(state.sensor_type);
        let mut new_conflicts = Vec::new();

        for data_type in &data_types {
            let source = DataSource::from_state(state, *data_type);
            if self.add_source(source) {
                new_conflicts.push(*data_type);
            }
        }

        // Track which data types this device provides
        self.sources_by_device.insert(state.device_id.clone(), data_types);

        new_conflicts
    }

    /// Add a source for a data type.
    ///
    /// Returns true if this created a new conflict.
    fn add_source(&mut self, source: DataSource) -> bool {
        let data_type = source.data_type;
        let device_id = source.device_id.clone();

        if let Some(conflict) = self.conflicts.get_mut(&data_type) {
            // Check if already in conflict
            if conflict.sources.iter().any(|s| s.device_id == device_id) {
                return false;
            }
            conflict.add_source(source);
            false // Not a NEW conflict
        } else {
            // First source for this data type - check for saved preference
            let mut conflict = SensorConflict::new(data_type, vec![source]);

            // Apply saved preference if available
            if let Some(pref) = self.preferences.get_preference(data_type) {
                if pref.primary_device_id == device_id {
                    conflict.set_primary(&device_id);
                }
            }

            self.conflicts.insert(data_type, conflict);
            false // Single source is not a conflict
        }
    }

    /// Unregister a sensor.
    ///
    /// Returns the data types that are no longer in conflict.
    pub fn unregister_sensor(&mut self, device_id: &str) -> Vec<DataType> {
        let mut resolved = Vec::new();

        // Get data types this device was providing
        if let Some(data_types) = self.sources_by_device.remove(device_id) {
            for data_type in data_types {
                if let Some(conflict) = self.conflicts.get_mut(&data_type) {
                    conflict.remove_source(device_id);

                    if !conflict.is_active() {
                        resolved.push(data_type);
                    }
                }
            }
        }

        // Clean up empty conflicts
        self.conflicts.retain(|_, c| c.sources.len() > 0);

        resolved
    }

    /// Check if there are any active conflicts.
    pub fn has_conflicts(&self) -> bool {
        self.conflicts.values().any(|c| c.is_active())
    }

    /// Check if a specific data type has a conflict.
    pub fn has_conflict(&self, data_type: DataType) -> bool {
        self.conflicts.get(&data_type).map_or(false, |c| c.is_active())
    }

    /// Get all active conflicts.
    pub fn active_conflicts(&self) -> Vec<&SensorConflict> {
        self.conflicts.values().filter(|c| c.is_active()).collect()
    }

    /// Get conflicts that need user attention.
    pub fn conflicts_needing_attention(&self) -> Vec<&SensorConflict> {
        self.conflicts
            .values()
            .filter(|c| c.is_active() && c.needs_attention())
            .collect()
    }

    /// Get a specific conflict.
    pub fn get_conflict(&self, data_type: DataType) -> Option<&SensorConflict> {
        self.conflicts.get(&data_type).filter(|c| c.is_active())
    }

    /// Get a mutable reference to a specific conflict.
    pub fn get_conflict_mut(&mut self, data_type: DataType) -> Option<&mut SensorConflict> {
        self.conflicts.get_mut(&data_type).filter(|c| c.is_active())
    }

    /// Set the primary sensor for a data type.
    ///
    /// Returns true if successful.
    pub fn set_primary(&mut self, data_type: DataType, device_id: &str) -> bool {
        if let Some(conflict) = self.conflicts.get_mut(&data_type) {
            if conflict.set_primary(device_id) {
                // Save preference
                if self.config.persist_resolutions {
                    if let Some(source) = conflict.primary_source() {
                        self.preferences.set_preference(ConflictPreference {
                            data_type,
                            primary_device_id: device_id.to_string(),
                            primary_sensor_name: source.name.clone(),
                            updated_at: Utc::now(),
                            user_set: true,
                        });
                    }
                }
                return true;
            }
        }
        false
    }

    /// Clear the primary selection for a data type.
    pub fn clear_primary(&mut self, data_type: DataType) {
        if let Some(conflict) = self.conflicts.get_mut(&data_type) {
            conflict.clear_primary();
        }
        self.preferences.remove_preference(data_type);
    }

    /// Get the primary device ID for a data type.
    pub fn get_primary(&self, data_type: DataType) -> Option<&str> {
        self.conflicts
            .get(&data_type)
            .and_then(|c| c.primary_device_id.as_deref())
    }

    /// Check if a device is the primary source for any data type.
    pub fn is_primary(&self, device_id: &str) -> bool {
        self.conflicts
            .values()
            .any(|c| c.primary_device_id.as_deref() == Some(device_id))
    }

    /// Get all data types a device is primary for.
    pub fn primary_for(&self, device_id: &str) -> Vec<DataType> {
        self.conflicts
            .iter()
            .filter(|(_, c)| c.primary_device_id.as_deref() == Some(device_id))
            .map(|(dt, _)| *dt)
            .collect()
    }

    /// Auto-resolve conflicts based on the configured strategy.
    ///
    /// Returns the data types that were auto-resolved.
    pub fn auto_resolve(&mut self) -> Vec<DataType> {
        let mut resolved = Vec::new();

        // Collect conflicts to resolve (to avoid borrow issues)
        let to_resolve: Vec<_> = self.conflicts
            .iter()
            .filter(|(_, c)| c.is_active() && !c.is_resolved)
            .filter(|(dt, _)| !dt.is_critical() || self.config.strategy != ResolutionStrategy::UserSelection)
            .map(|(dt, _)| *dt)
            .collect();

        for data_type in to_resolve {
            if let Some(conflict) = self.conflicts.get(&data_type) {
                let device_id = match self.config.strategy {
                    ResolutionStrategy::AutoPriority => {
                        // Select by sensor type priority
                        conflict.sources
                            .iter()
                            .min_by_key(|s| sensor_type_priority(s.sensor_type))
                            .map(|s| s.device_id.clone())
                    }
                    ResolutionStrategy::FirstConnected => {
                        // Select first connected or first in list
                        conflict.sources
                            .iter()
                            .find(|s| s.is_connected)
                            .or_else(|| conflict.sources.first())
                            .map(|s| s.device_id.clone())
                    }
                    ResolutionStrategy::MostRecent => {
                        // Select last in list (most recently added)
                        conflict.sources.last().map(|s| s.device_id.clone())
                    }
                    ResolutionStrategy::UserSelection => None,
                };

                if let Some(id) = device_id {
                    if self.set_primary(data_type, &id) {
                        resolved.push(data_type);
                    }
                }
            }
        }

        resolved
    }

    /// Apply saved preferences to current conflicts.
    pub fn apply_saved_preferences(&mut self) {
        for (data_type, conflict) in &mut self.conflicts {
            if !conflict.is_resolved {
                if let Some(pref) = self.preferences.get_preference(*data_type) {
                    // Check if the saved primary is still in our sources
                    if conflict.sources.iter().any(|s| s.device_id == pref.primary_device_id) {
                        conflict.set_primary(&pref.primary_device_id);
                    }
                }
            }
        }
    }

    /// Mark a conflict as notified.
    pub fn mark_notified(&mut self, data_type: DataType) {
        if let Some(conflict) = self.conflicts.get_mut(&data_type) {
            conflict.mark_notified();
        }
    }

    /// Mark all conflicts as notified.
    pub fn mark_all_notified(&mut self) {
        for conflict in self.conflicts.values_mut() {
            conflict.mark_notified();
        }
    }

    /// Get the number of active conflicts.
    pub fn conflict_count(&self) -> usize {
        self.conflicts.values().filter(|c| c.is_active()).count()
    }

    /// Get the number of unresolved conflicts.
    pub fn unresolved_count(&self) -> usize {
        self.conflicts
            .values()
            .filter(|c| c.is_active() && !c.is_resolved)
            .count()
    }

    /// Clear all conflicts.
    pub fn clear(&mut self) {
        self.conflicts.clear();
        self.sources_by_device.clear();
    }

    /// Save preferences to disk.
    pub fn save_preferences(&mut self) -> Result<(), ConflictError> {
        self.preferences.save()
    }

    // =========================================================================
    // Failover Methods
    // =========================================================================

    /// Update the connection status of a sensor in all conflicts.
    ///
    /// Call this when a sensor's connection state changes to keep
    /// conflict sources accurate.
    pub fn update_connection_status(&mut self, device_id: &str, is_connected: bool) {
        for conflict in self.conflicts.values_mut() {
            for source in &mut conflict.sources {
                if source.device_id == device_id {
                    source.is_connected = is_connected;
                }
            }
        }
    }

    /// Handle a primary sensor disconnection with automatic failover.
    ///
    /// When the primary sensor for a data type disconnects, this method
    /// attempts to promote a connected secondary sensor to primary.
    /// If successful, returns failover details for user notification.
    ///
    /// # Arguments
    /// * `device_id` - The device ID of the sensor that disconnected
    ///
    /// # Returns
    /// A list of failover results for each data type that was affected.
    /// Empty if no failovers occurred (no connected secondary available).
    pub fn handle_primary_disconnect(&mut self, device_id: &str) -> Vec<FailoverResult> {
        let mut failovers = Vec::new();

        // First, update the connection status
        self.update_connection_status(device_id, false);

        // Find all data types where this device was primary
        let primary_data_types: Vec<DataType> = self.primary_for(device_id);

        if primary_data_types.is_empty() {
            return failovers;
        }

        tracing::info!(
            "Primary sensor {} disconnected, checking failover for {:?}",
            device_id,
            primary_data_types
        );

        // For each data type, try to find a connected secondary to promote
        for data_type in primary_data_types {
            if let Some(failover) = self.try_failover(data_type, device_id) {
                failovers.push(failover);
            }
        }

        failovers
    }

    /// Try to perform a failover for a specific data type.
    ///
    /// Looks for connected secondary sensors that can be promoted to primary.
    fn try_failover(&mut self, data_type: DataType, from_device_id: &str) -> Option<FailoverResult> {
        let conflict = self.conflicts.get_mut(&data_type)?;

        // Get info about the disconnected primary before we clear it
        let from_sensor_name = conflict
            .primary_source()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Find a connected secondary sensor to promote
        // Prioritize by sensor type priority, then by signal quality
        let secondary = conflict
            .sources
            .iter()
            .filter(|s| s.device_id != from_device_id && s.is_connected)
            .min_by_key(|s| sensor_type_priority(s.sensor_type));

        let secondary = match secondary {
            Some(s) => s.clone(),
            None => {
                tracing::info!(
                    "No connected secondary available for {} failover",
                    data_type.display_name()
                );
                // Clear the primary since it's no longer connected
                conflict.clear_primary();
                return None;
            }
        };

        tracing::info!(
            "Failover: {} switching from {} to {}",
            data_type.display_name(),
            from_sensor_name,
            secondary.name
        );

        // Promote the secondary to primary
        let new_primary_id = secondary.device_id.clone();
        let new_primary_name = secondary.name.clone();

        conflict.set_primary(&new_primary_id);

        // Save the new preference
        if self.config.persist_resolutions {
            self.preferences.set_preference(ConflictPreference {
                data_type,
                primary_device_id: new_primary_id.clone(),
                primary_sensor_name: new_primary_name.clone(),
                updated_at: Utc::now(),
                user_set: false, // This was an automatic failover
            });
        }

        Some(FailoverResult {
            data_type,
            from_device_id: from_device_id.to_string(),
            from_sensor_name,
            to_device_id: new_primary_id,
            to_sensor_name: new_primary_name,
        })
    }

    /// Get available failover targets for a data type.
    ///
    /// Returns a list of connected sensors that could take over if the
    /// current primary disconnects, sorted by priority.
    pub fn get_failover_targets(&self, data_type: DataType) -> Vec<&DataSource> {
        if let Some(conflict) = self.conflicts.get(&data_type) {
            let current_primary = conflict.primary_device_id.as_deref();

            let mut targets: Vec<_> = conflict
                .sources
                .iter()
                .filter(|s| {
                    s.is_connected && Some(s.device_id.as_str()) != current_primary
                })
                .collect();

            // Sort by priority (lower = higher priority)
            targets.sort_by_key(|s| sensor_type_priority(s.sensor_type));
            targets
        } else {
            Vec::new()
        }
    }

    /// Check if failover is available for a data type.
    ///
    /// Returns true if there's at least one connected secondary sensor
    /// that could take over if the primary disconnects.
    pub fn has_failover_available(&self, data_type: DataType) -> bool {
        !self.get_failover_targets(data_type).is_empty()
    }

    /// Get data types that have failover protection.
    ///
    /// Returns a list of data types where at least one secondary sensor
    /// is connected and could take over if the primary fails.
    pub fn get_protected_data_types(&self) -> Vec<DataType> {
        self.conflicts
            .keys()
            .filter(|dt| self.has_failover_available(**dt))
            .copied()
            .collect()
    }

    /// Get data types that are at risk (primary connected, no failover).
    ///
    /// These are data types where losing the primary would result in
    /// no data for that type.
    pub fn get_at_risk_data_types(&self) -> Vec<DataType> {
        self.conflicts
            .iter()
            .filter_map(|(data_type, conflict)| {
                // Has a connected primary but no failover targets
                let has_primary = conflict.primary_device_id.as_ref()
                    .and_then(|id| conflict.sources.iter().find(|s| &s.device_id == id))
                    .map_or(false, |s| s.is_connected);

                if has_primary && !self.has_failover_available(*data_type) {
                    Some(*data_type)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get conflict summary for display.
    pub fn summary(&self) -> ConflictSummary {
        let active: Vec<_> = self.active_conflicts().iter().map(|c| {
            ConflictInfo {
                data_type: c.data_type,
                sensor_count: c.sensor_count(),
                is_resolved: c.is_resolved,
                primary_name: c.primary_source().map(|s| s.name.clone()),
            }
        }).collect();

        ConflictSummary {
            total_conflicts: active.len(),
            unresolved_count: active.iter().filter(|c| !c.is_resolved).count(),
            conflicts: active,
        }
    }
}

/// Summary of conflict status.
#[derive(Debug, Clone)]
pub struct ConflictSummary {
    /// Total number of active conflicts.
    pub total_conflicts: usize,
    /// Number of unresolved conflicts.
    pub unresolved_count: usize,
    /// Details of each conflict.
    pub conflicts: Vec<ConflictInfo>,
}

impl ConflictSummary {
    /// Check if there are any conflicts needing attention.
    pub fn needs_attention(&self) -> bool {
        self.unresolved_count > 0
    }
}

/// Information about a single conflict.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// The data type in conflict.
    pub data_type: DataType,
    /// Number of sensors providing this data.
    pub sensor_count: usize,
    /// Whether a primary has been selected.
    pub is_resolved: bool,
    /// Name of the primary sensor if selected.
    pub primary_name: Option<String>,
}

// ============================================================================
// Conflict Preference Persistence
// ============================================================================

const CONFLICT_PREFERENCE_FILE: &str = "conflict_preferences.json";

/// A stored conflict preference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConflictPreference {
    /// The data type this preference applies to.
    pub data_type: DataType,
    /// The preferred primary device ID.
    pub primary_device_id: String,
    /// The primary sensor's name (for display).
    pub primary_sensor_name: String,
    /// When this preference was last updated.
    pub updated_at: DateTime<Utc>,
    /// Whether this was explicitly set by the user.
    pub user_set: bool,
}

/// Persisted preferences data structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConflictPreferenceData {
    /// Version of the preference file format.
    pub version: u32,
    /// When the preferences were last saved.
    pub last_saved_at: Option<DateTime<Utc>>,
    /// Map of data type to preference.
    #[serde(default)]
    pub preferences: HashMap<DataType, ConflictPreference>,
}

/// Manager for conflict preference persistence.
#[derive(Debug)]
pub struct ConflictPreferenceManager {
    /// Path to the preferences file.
    file_path: PathBuf,
    /// Current preferences data.
    data: ConflictPreferenceData,
    /// Whether there are unsaved changes.
    dirty: bool,
    /// Whether to auto-save on changes.
    auto_save: bool,
}

impl Default for ConflictPreferenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictPreferenceManager {
    /// Create a new preference manager with default path.
    pub fn new() -> Self {
        Self::with_path(get_conflict_preference_path())
    }

    /// Create a preference manager with a custom path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            file_path: path,
            data: ConflictPreferenceData {
                version: 1,
                last_saved_at: None,
                preferences: HashMap::new(),
            },
            dirty: false,
            auto_save: true,
        }
    }

    /// Load preferences from disk.
    pub fn load() -> Self {
        Self::load_from_path(get_conflict_preference_path())
    }

    /// Load preferences from a specific path.
    pub fn load_from_path(path: PathBuf) -> Self {
        if !path.exists() {
            tracing::debug!("No conflict preference file found at {:?}, starting fresh", path);
            return Self::with_path(path);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<ConflictPreferenceData>(&content) {
                Ok(data) => {
                    tracing::info!(
                        "Loaded {} conflict preferences from {:?}",
                        data.preferences.len(),
                        path
                    );
                    Self {
                        file_path: path,
                        data,
                        dirty: false,
                        auto_save: true,
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse conflict preference file: {}", e);
                    Self::with_path(path)
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read conflict preference file: {}", e);
                Self::with_path(path)
            }
        }
    }

    /// Save preferences to disk.
    pub fn save(&mut self) -> Result<(), ConflictError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConflictError::IoError(e.to_string()))?;
        }

        self.data.last_saved_at = Some(Utc::now());

        let content = serde_json::to_string_pretty(&self.data)
            .map_err(|e| ConflictError::SerializeError(e.to_string()))?;

        std::fs::write(&self.file_path, content)
            .map_err(|e| ConflictError::IoError(e.to_string()))?;

        self.dirty = false;
        tracing::debug!("Saved {} conflict preferences", self.data.preferences.len());

        Ok(())
    }

    /// Set a preference.
    pub fn set_preference(&mut self, pref: ConflictPreference) {
        self.data.preferences.insert(pref.data_type, pref);
        self.dirty = true;

        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save conflict preferences: {}", e);
            }
        }
    }

    /// Get a preference.
    pub fn get_preference(&self, data_type: DataType) -> Option<&ConflictPreference> {
        self.data.preferences.get(&data_type)
    }

    /// Remove a preference.
    pub fn remove_preference(&mut self, data_type: DataType) -> Option<ConflictPreference> {
        let removed = self.data.preferences.remove(&data_type);
        if removed.is_some() {
            self.dirty = true;
            if self.auto_save {
                if let Err(e) = self.save() {
                    tracing::warn!("Failed to auto-save conflict preferences: {}", e);
                }
            }
        }
        removed
    }

    /// Get all preferences.
    pub fn all_preferences(&self) -> impl Iterator<Item = &ConflictPreference> {
        self.data.preferences.values()
    }

    /// Clear all preferences.
    pub fn clear(&mut self) {
        self.data.preferences.clear();
        self.dirty = true;
        if self.auto_save {
            if let Err(e) = self.save() {
                tracing::warn!("Failed to auto-save conflict preferences: {}", e);
            }
        }
    }

    /// Get the number of stored preferences.
    pub fn len(&self) -> usize {
        self.data.preferences.len()
    }

    /// Check if there are no stored preferences.
    pub fn is_empty(&self) -> bool {
        self.data.preferences.is_empty()
    }
}

/// Get the default conflict preference file path.
pub fn get_conflict_preference_path() -> PathBuf {
    get_data_dir().join(CONFLICT_PREFERENCE_FILE)
}

/// Errors that can occur with conflict detection and resolution.
#[derive(Debug, Error)]
pub enum ConflictError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialize error: {0}")]
    SerializeError(String),

    #[error("Deserialize error: {0}")]
    DeserializeError(String),

    #[error("No conflict for data type: {0}")]
    NoConflict(String),

    #[error("Sensor not found in conflict: {0}")]
    SensorNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::types::Protocol;
    use std::time::Instant;

    fn make_sensor(name: &str, sensor_type: SensorType, protocol: Protocol) -> DiscoveredSensor {
        DiscoveredSensor {
            device_id: format!("{}:{}", name.to_lowercase().replace(' ', "_"), protocol),
            name: name.to_string(),
            sensor_type,
            protocol,
            signal_strength: Some(-60),
            last_seen: Instant::now(),
        }
    }

    #[test]
    fn test_data_type_from_sensor_type() {
        let power_meter_types = DataType::from_sensor_type(SensorType::PowerMeter);
        assert!(power_meter_types.contains(&DataType::Power));
        assert!(power_meter_types.contains(&DataType::Cadence));

        let trainer_types = DataType::from_sensor_type(SensorType::Trainer);
        assert!(trainer_types.contains(&DataType::Power));
        assert!(trainer_types.contains(&DataType::TrainerControl));

        let hr_types = DataType::from_sensor_type(SensorType::HeartRate);
        assert_eq!(hr_types, vec![DataType::HeartRate]);
    }

    #[test]
    fn test_data_type_is_critical() {
        assert!(DataType::Power.is_critical());
        assert!(DataType::HeartRate.is_critical());
        assert!(DataType::TrainerControl.is_critical());
        assert!(!DataType::Cadence.is_critical());
        assert!(!DataType::Speed.is_critical());
    }

    #[test]
    fn test_conflict_detector_no_conflict_single_sensor() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        detector.register_sensor(&power_meter);

        assert!(!detector.has_conflicts());
        assert_eq!(detector.conflict_count(), 0);
    }

    #[test]
    fn test_conflict_detector_power_conflict() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        // Both provide power, so there should be a conflict
        assert!(detector.has_conflict(DataType::Power));

        let conflict = detector.get_conflict(DataType::Power).unwrap();
        assert_eq!(conflict.sensor_count(), 2);
    }

    #[test]
    fn test_conflict_detector_set_primary() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        // Set power meter as primary for power
        let result = detector.set_primary(DataType::Power, &power_meter.device_id);
        assert!(result);

        let conflict = detector.get_conflict(DataType::Power).unwrap();
        assert!(conflict.is_resolved);
        assert_eq!(conflict.primary_device_id, Some(power_meter.device_id.clone()));
    }

    #[test]
    fn test_conflict_detector_unregister_sensor() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        assert!(detector.has_conflict(DataType::Power));

        // Remove the power meter
        detector.unregister_sensor(&power_meter.device_id);

        // Conflict should no longer be active (only one source)
        assert!(!detector.has_conflict(DataType::Power));
    }

    #[test]
    fn test_conflict_no_conflict_different_data_types() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let hr_monitor = make_sensor("Polar H10", SensorType::HeartRate, Protocol::BleHeartRate);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&hr_monitor);

        // No conflict - they provide different data types
        assert!(!detector.has_conflicts());
    }

    #[test]
    fn test_conflict_detector_multiple_hr_monitors() {
        let mut detector = ConflictDetector::new();

        let hr1 = make_sensor("Polar H10", SensorType::HeartRate, Protocol::BleHeartRate);
        let hr2 = make_sensor("Garmin HRM", SensorType::HeartRate, Protocol::AntHeartRate);

        detector.register_sensor(&hr1);
        detector.register_sensor(&hr2);

        assert!(detector.has_conflict(DataType::HeartRate));

        let conflict = detector.get_conflict(DataType::HeartRate).unwrap();
        assert_eq!(conflict.sensor_count(), 2);
    }

    #[test]
    fn test_conflict_summary() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        let summary = detector.summary();
        assert!(summary.total_conflicts > 0);
        assert!(summary.needs_attention());
    }

    #[test]
    fn test_auto_resolve_by_priority() {
        let mut detector = ConflictDetector::with_config(ConflictDetectorConfig {
            strategy: ResolutionStrategy::AutoPriority,
            auto_resolve_non_critical: false, // So we can test power too
            persist_resolutions: false,
        });

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        let resolved = detector.auto_resolve();

        // Power meter should be selected (higher priority than trainer for power)
        assert!(resolved.contains(&DataType::Power));

        let conflict = detector.get_conflict(DataType::Power).unwrap();
        assert!(conflict.is_resolved);
        assert_eq!(conflict.primary_device_id, Some(power_meter.device_id.clone()));
    }

    #[test]
    fn test_sensor_conflict_mark_notified() {
        let mut conflict = SensorConflict::new(DataType::Power, vec![]);

        assert!(!conflict.user_notified);
        conflict.mark_notified();
        assert!(conflict.user_notified);
    }

    #[test]
    fn test_data_type_display() {
        assert_eq!(DataType::Power.display_name(), "Power");
        assert_eq!(DataType::HeartRate.display_name(), "Heart Rate");
        assert_eq!(format!("{}", DataType::Cadence), "Cadence");
    }

    #[test]
    fn test_is_primary() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        detector.set_primary(DataType::Power, &power_meter.device_id);

        assert!(detector.is_primary(&power_meter.device_id));
        assert!(!detector.is_primary(&trainer.device_id));
    }

    #[test]
    fn test_primary_for() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        detector.set_primary(DataType::Power, &power_meter.device_id);
        detector.set_primary(DataType::Cadence, &power_meter.device_id);

        let primary_for = detector.primary_for(&power_meter.device_id);
        assert!(primary_for.contains(&DataType::Power));
        assert!(primary_for.contains(&DataType::Cadence));
    }

    #[test]
    fn test_conflict_clear_primary() {
        let mut detector = ConflictDetector::new();

        let power_meter = make_sensor("Stages Power", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter);
        detector.register_sensor(&trainer);

        detector.set_primary(DataType::Power, &power_meter.device_id);
        assert!(detector.get_conflict(DataType::Power).unwrap().is_resolved);

        detector.clear_primary(DataType::Power);

        let conflict = detector.get_conflict(DataType::Power).unwrap();
        assert!(!conflict.is_resolved);
        assert!(conflict.primary_device_id.is_none());
    }

    #[test]
    fn test_three_way_conflict() {
        let mut detector = ConflictDetector::new();

        let power_meter1 = make_sensor("Stages Left", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let power_meter2 = make_sensor("Stages Right", SensorType::PowerMeter, Protocol::BleCyclingPower);
        let trainer = make_sensor("KICKR Core", SensorType::Trainer, Protocol::BleFtms);

        detector.register_sensor(&power_meter1);
        detector.register_sensor(&power_meter2);
        detector.register_sensor(&trainer);

        let conflict = detector.get_conflict(DataType::Power).unwrap();
        assert_eq!(conflict.sensor_count(), 3);
    }
}
