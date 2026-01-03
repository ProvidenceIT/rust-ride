//! Smart Fan Controller
//!
//! Controls smart fans via MQTT based on training zones.

use super::{MqttClient, MqttError, QoS};
use crate::storage::hardware_store::StoredFanProfile;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Fan profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanProfile {
    /// Unique ID for this profile
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// MQTT topic for controlling this fan
    pub mqtt_topic: String,
    /// Whether to include /set suffix for commands
    pub use_set_suffix: bool,
    /// Payload format for speed commands
    pub payload_format: PayloadFormat,
    /// Zone to speed mapping (zone 1-7 -> speed 0-100)
    pub zone_speeds: [u8; 7],
    /// Use power zones (true) or HR zones (false)
    pub use_power_zones: bool,
    /// Delay before changing speed (to prevent rapid changes)
    pub change_delay_secs: u8,
}

/// Payload format for MQTT messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadFormat {
    /// Just the speed number (e.g., "75")
    SpeedOnly,
    /// JSON format (e.g., {"speed": 75})
    JsonSpeed,
    /// JSON with on/off (e.g., {"speed": 75, "on": true})
    JsonSpeedOnOff,
    /// Percentage format (e.g., "75%")
    Percentage,
}

impl Default for FanProfile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Default Fan".to_string(),
            mqtt_topic: "home/fan/living_room".to_string(),
            use_set_suffix: true,
            payload_format: PayloadFormat::JsonSpeed,
            // Zone 1 = 0%, Zone 2 = 20%, Zone 3 = 40%, etc.
            zone_speeds: [0, 20, 40, 60, 80, 90, 100],
            use_power_zones: true,
            change_delay_secs: 3,
        }
    }
}

impl FanProfile {
    /// Get the MQTT topic for commands
    pub fn command_topic(&self) -> String {
        if self.use_set_suffix {
            format!("{}/set", self.mqtt_topic)
        } else {
            self.mqtt_topic.clone()
        }
    }

    /// Format the speed payload
    pub fn format_payload(&self, speed: u8, on: bool) -> String {
        match self.payload_format {
            PayloadFormat::SpeedOnly => speed.to_string(),
            PayloadFormat::JsonSpeed => format!(r#"{{"speed": {}}}"#, speed),
            PayloadFormat::JsonSpeedOnOff => {
                format!(r#"{{"speed": {}, "on": {}}}"#, speed, on)
            }
            PayloadFormat::Percentage => format!("{}%", speed),
        }
    }

    /// Get speed for a zone
    pub fn speed_for_zone(&self, zone: u8) -> u8 {
        let idx = (zone.saturating_sub(1) as usize).min(6);
        self.zone_speeds[idx]
    }

    /// Convert to StoredFanProfile for database persistence.
    ///
    /// The `user_id` is required for database storage.
    /// The `is_active` flag determines if this is the currently active profile.
    pub fn to_stored(&self, user_id: Uuid, is_active: bool) -> StoredFanProfile {
        let now = Utc::now().to_rfc3339();
        let settings = FanProfileSettings {
            mqtt_topic: self.mqtt_topic.clone(),
            use_set_suffix: self.use_set_suffix,
            payload_format: self.payload_format,
            zone_speeds: self.zone_speeds,
            use_power_zones: self.use_power_zones,
            change_delay_secs: self.change_delay_secs,
        };

        StoredFanProfile {
            id: self.id,
            user_id,
            name: self.name.clone(),
            is_active,
            zone_settings_json: serde_json::to_string(&settings).unwrap_or_default(),
            hr_thresholds_json: None,
            min_speed_pct: settings.zone_speeds.iter().copied().min().unwrap_or(0),
            max_speed_pct: settings.zone_speeds.iter().copied().max().unwrap_or(100),
            ramp_up_seconds: 0, // Not used in current FanProfile
            ramp_down_seconds: 0, // Not used in current FanProfile
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Settings stored in zone_settings_json for FanProfile persistence.
///
/// This struct captures all MQTT-specific settings that need to be persisted
/// but don't have dedicated columns in the database schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanProfileSettings {
    /// MQTT topic for controlling this fan
    pub mqtt_topic: String,
    /// Whether to include /set suffix for commands
    pub use_set_suffix: bool,
    /// Payload format for speed commands
    pub payload_format: PayloadFormat,
    /// Zone to speed mapping (zone 1-7 -> speed 0-100)
    pub zone_speeds: [u8; 7],
    /// Use power zones (true) or HR zones (false)
    pub use_power_zones: bool,
    /// Delay before changing speed (to prevent rapid changes)
    pub change_delay_secs: u8,
}

impl Default for FanProfileSettings {
    fn default() -> Self {
        Self {
            mqtt_topic: "home/fan/living_room".to_string(),
            use_set_suffix: true,
            payload_format: PayloadFormat::JsonSpeed,
            zone_speeds: [0, 20, 40, 60, 80, 90, 100],
            use_power_zones: true,
            change_delay_secs: 3,
        }
    }
}

impl From<StoredFanProfile> for FanProfile {
    /// Convert a StoredFanProfile from the database to a runtime FanProfile.
    ///
    /// Parses the zone_settings_json to extract MQTT-specific settings.
    /// Falls back to defaults if JSON parsing fails.
    fn from(stored: StoredFanProfile) -> Self {
        let settings: FanProfileSettings = serde_json::from_str(&stored.zone_settings_json)
            .unwrap_or_else(|_| {
                // Fallback: try to parse as just zone speeds array for backward compatibility
                if let Ok(zone_speeds) = serde_json::from_str::<[u8; 7]>(&stored.zone_settings_json)
                {
                    FanProfileSettings {
                        zone_speeds,
                        ..Default::default()
                    }
                } else {
                    FanProfileSettings::default()
                }
            });

        FanProfile {
            id: stored.id,
            name: stored.name,
            mqtt_topic: settings.mqtt_topic,
            use_set_suffix: settings.use_set_suffix,
            payload_format: settings.payload_format,
            zone_speeds: settings.zone_speeds,
            use_power_zones: settings.use_power_zones,
            change_delay_secs: settings.change_delay_secs,
        }
    }
}

/// Current state of a fan
#[derive(Debug, Clone)]
pub struct FanState {
    /// Profile ID
    pub profile_id: Uuid,
    /// Current speed (0-100)
    pub current_speed: u8,
    /// Last zone that triggered a change
    pub last_zone: u8,
    /// Whether in automatic mode
    pub auto_mode: bool,
    /// Last time state was updated
    pub last_update: Instant,
    /// Whether fan is currently on
    pub is_on: bool,
}

/// Trait for fan controller implementations
pub trait FanController: Send + Sync {
    /// Configure with fan profiles
    fn configure(&self, profiles: Vec<FanProfile>);

    /// Start fan control for a ride
    fn start(&self) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Stop fan control
    fn stop(&self) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Update current metrics (triggers fan speed evaluation)
    fn update_metrics(&self, power: u16, hr: Option<u8>, power_zone: u8, hr_zone: Option<u8>);

    /// Manually set fan speed (overrides auto)
    fn set_speed(
        &self,
        profile_id: &Uuid,
        speed: u8,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Get current fan states
    fn get_states(&self) -> HashMap<Uuid, FanState>;

    /// Test a fan (cycle through speeds)
    fn test_fan(
        &self,
        profile_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> + Send;

    /// Enable/disable auto mode for a fan
    fn set_auto_mode(&self, profile_id: &Uuid, enabled: bool);
}

/// Default fan controller implementation
pub struct DefaultFanController<C: MqttClient> {
    mqtt_client: Arc<C>,
    profiles: Arc<RwLock<Vec<FanProfile>>>,
    states: Arc<RwLock<HashMap<Uuid, FanState>>>,
    is_running: Arc<RwLock<bool>>,
}

impl<C: MqttClient> DefaultFanController<C> {
    /// Create a new fan controller
    pub fn new(mqtt_client: Arc<C>) -> Self {
        Self {
            mqtt_client,
            profiles: Arc::new(RwLock::new(Vec::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Calculate target speed based on zone (reserved for future use)
    #[allow(dead_code)]
    fn calculate_target_speed(
        &self,
        profile: &FanProfile,
        power_zone: u8,
        hr_zone: Option<u8>,
    ) -> u8 {
        let zone = if profile.use_power_zones {
            power_zone
        } else {
            hr_zone.unwrap_or(power_zone)
        };

        profile.speed_for_zone(zone)
    }

    /// Send speed command to a fan
    async fn send_speed_command(&self, profile: &FanProfile, speed: u8) -> Result<(), MqttError> {
        let is_on = speed > 0;
        let payload = profile.format_payload(speed, is_on);
        let topic = profile.command_topic();

        self.mqtt_client
            .publish(&topic, &payload, QoS::AtLeastOnce)
            .await
    }
}

impl<C: MqttClient + 'static> FanController for DefaultFanController<C> {
    fn configure(&self, profiles: Vec<FanProfile>) {
        // Use try_write to avoid blocking in sync context
        if let Ok(mut p) = self.profiles.try_write() {
            *p = profiles;
        }
    }

    async fn start(&self) -> Result<(), MqttError> {
        *self.is_running.write().await = true;

        // Initialize states for all profiles
        let profiles = self.profiles.read().await;
        let mut states = self.states.write().await;

        for profile in profiles.iter() {
            states.insert(
                profile.id,
                FanState {
                    profile_id: profile.id,
                    current_speed: 0,
                    last_zone: 1,
                    auto_mode: true,
                    last_update: Instant::now(),
                    is_on: false,
                },
            );
        }

        tracing::info!("Fan controller started with {} profiles", profiles.len());

        Ok(())
    }

    async fn stop(&self) -> Result<(), MqttError> {
        *self.is_running.write().await = false;

        // Turn off all fans
        let profiles = self.profiles.read().await;
        for profile in profiles.iter() {
            let _ = self.send_speed_command(profile, 0).await;
        }

        tracing::info!("Fan controller stopped");

        Ok(())
    }

    fn update_metrics(&self, _power: u16, _hr: Option<u8>, power_zone: u8, hr_zone: Option<u8>) {
        // This would be called from the ride loop
        // Use spawn to avoid blocking
        let profiles = self.profiles.clone();
        let states = self.states.clone();
        let is_running = self.is_running.clone();
        let mqtt_client = self.mqtt_client.clone();

        tokio::spawn(async move {
            if !*is_running.read().await {
                return;
            }

            let profiles = profiles.read().await;
            let mut states = states.write().await;

            for profile in profiles.iter() {
                if let Some(state) = states.get_mut(&profile.id) {
                    if !state.auto_mode {
                        continue;
                    }

                    let zone = if profile.use_power_zones {
                        power_zone
                    } else {
                        hr_zone.unwrap_or(power_zone)
                    };

                    // Check if zone changed and enough time has passed
                    if zone != state.last_zone
                        && state.last_update.elapsed().as_secs() >= profile.change_delay_secs as u64
                    {
                        let target_speed = profile.speed_for_zone(zone);

                        if target_speed != state.current_speed {
                            let topic = profile.command_topic();
                            let payload = profile.format_payload(target_speed, target_speed > 0);

                            if let Err(e) = mqtt_client
                                .publish(&topic, &payload, QoS::AtLeastOnce)
                                .await
                            {
                                tracing::warn!("Failed to update fan speed: {}", e);
                            } else {
                                state.current_speed = target_speed;
                                state.is_on = target_speed > 0;
                                tracing::debug!(
                                    "Fan {} speed changed to {} (zone {})",
                                    profile.name,
                                    target_speed,
                                    zone
                                );
                            }
                        }

                        state.last_zone = zone;
                        state.last_update = Instant::now();
                    }
                }
            }
        });
    }

    async fn set_speed(&self, profile_id: &Uuid, speed: u8) -> Result<(), MqttError> {
        let profiles = self.profiles.read().await;
        let profile = profiles
            .iter()
            .find(|p| &p.id == profile_id)
            .ok_or(MqttError::ConfigError("Profile not found".to_string()))?;

        self.send_speed_command(profile, speed.min(100)).await?;

        // Update state
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(profile_id) {
            state.current_speed = speed;
            state.is_on = speed > 0;
            state.auto_mode = false; // Manual override disables auto
            state.last_update = Instant::now();
        }

        Ok(())
    }

    fn get_states(&self) -> HashMap<Uuid, FanState> {
        if let Ok(states) = self.states.try_read() {
            states.clone()
        } else {
            HashMap::new()
        }
    }

    async fn test_fan(&self, profile_id: &Uuid) -> Result<(), MqttError> {
        let profiles = self.profiles.read().await;
        let profile = profiles
            .iter()
            .find(|p| &p.id == profile_id)
            .ok_or(MqttError::ConfigError("Profile not found".to_string()))?
            .clone();
        drop(profiles);

        tracing::info!("Testing fan: {}", profile.name);

        // Cycle through speeds
        for speed in [25, 50, 75, 100, 50, 0].iter() {
            self.send_speed_command(&profile, *speed).await?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        Ok(())
    }

    fn set_auto_mode(&self, profile_id: &Uuid, enabled: bool) {
        if let Ok(mut states) = self.states.try_write() {
            if let Some(state) = states.get_mut(profile_id) {
                state.auto_mode = enabled;
            }
        }
    }
}

// ========== Database Helper Functions ==========

use crate::storage::database::DatabaseError;
use crate::storage::HardwareStore;

/// Load all fan profiles from the database for a user.
///
/// Converts StoredFanProfile records to FanProfile instances.
///
/// # Arguments
/// * `store` - The HardwareStore instance for database access
/// * `user_id` - The user ID to load profiles for
///
/// # Returns
/// A vector of FanProfile instances, or an error if the query fails.
pub fn load_fan_profiles(
    store: &HardwareStore,
    user_id: &Uuid,
) -> Result<Vec<FanProfile>, DatabaseError> {
    let stored_profiles = store.get_fan_profiles(user_id)?;
    Ok(stored_profiles.into_iter().map(FanProfile::from).collect())
}

/// Load the active fan profile from the database for a user.
///
/// Returns the currently active profile, or None if no profile is active.
///
/// # Arguments
/// * `store` - The HardwareStore instance for database access
/// * `user_id` - The user ID to load the active profile for
///
/// # Returns
/// An optional FanProfile if an active profile exists.
pub fn load_active_fan_profile(
    store: &HardwareStore,
    user_id: &Uuid,
) -> Result<Option<FanProfile>, DatabaseError> {
    let stored = store.get_active_fan_profile(user_id)?;
    Ok(stored.map(FanProfile::from))
}

/// Save a fan profile to the database.
///
/// Creates a new profile or updates an existing one based on the profile ID.
///
/// # Arguments
/// * `store` - The HardwareStore instance for database access
/// * `user_id` - The user ID to associate with the profile
/// * `profile` - The FanProfile to save
/// * `is_active` - Whether this should be the active profile
///
/// # Returns
/// An error if the save operation fails.
pub fn save_fan_profile(
    store: &HardwareStore,
    user_id: &Uuid,
    profile: &FanProfile,
    is_active: bool,
) -> Result<(), DatabaseError> {
    let stored = profile.to_stored(*user_id, is_active);
    store.save_fan_profile(&stored)?;

    // If this profile is active, deactivate others
    if is_active {
        store.set_active_fan_profile(user_id, &profile.id)?;
    }

    Ok(())
}

/// Save multiple fan profiles to the database.
///
/// # Arguments
/// * `store` - The HardwareStore instance for database access
/// * `user_id` - The user ID to associate with the profiles
/// * `profiles` - The FanProfiles to save
/// * `active_id` - Optional ID of the profile that should be active
///
/// # Returns
/// An error if any save operation fails.
pub fn save_fan_profiles(
    store: &HardwareStore,
    user_id: &Uuid,
    profiles: &[FanProfile],
    active_id: Option<Uuid>,
) -> Result<(), DatabaseError> {
    for profile in profiles {
        let is_active = Some(profile.id) == active_id;
        let stored = profile.to_stored(*user_id, is_active);
        store.save_fan_profile(&stored)?;
    }

    // Ensure only one profile is active
    if let Some(id) = active_id {
        store.set_active_fan_profile(user_id, &id)?;
    }

    Ok(())
}

/// Delete a fan profile from the database.
///
/// # Arguments
/// * `store` - The HardwareStore instance for database access
/// * `profile_id` - The ID of the profile to delete
///
/// # Returns
/// An error if the delete operation fails.
pub fn delete_fan_profile(store: &HardwareStore, profile_id: &Uuid) -> Result<(), DatabaseError> {
    store.delete_fan_profile(profile_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan_profile_default() {
        let profile = FanProfile::default();
        assert_eq!(profile.zone_speeds[0], 0); // Zone 1
        assert_eq!(profile.zone_speeds[6], 100); // Zone 7
    }

    #[test]
    fn test_speed_for_zone() {
        let profile = FanProfile::default();
        assert_eq!(profile.speed_for_zone(1), 0);
        assert_eq!(profile.speed_for_zone(3), 40);
        assert_eq!(profile.speed_for_zone(7), 100);
        assert_eq!(profile.speed_for_zone(10), 100); // Clamped
    }

    #[test]
    fn test_payload_formats() {
        let mut profile = FanProfile {
            payload_format: PayloadFormat::SpeedOnly,
            ..Default::default()
        };
        assert_eq!(profile.format_payload(75, true), "75");

        profile.payload_format = PayloadFormat::JsonSpeed;
        assert_eq!(profile.format_payload(75, true), r#"{"speed": 75}"#);

        profile.payload_format = PayloadFormat::JsonSpeedOnOff;
        assert_eq!(
            profile.format_payload(75, true),
            r#"{"speed": 75, "on": true}"#
        );

        profile.payload_format = PayloadFormat::Percentage;
        assert_eq!(profile.format_payload(75, true), "75%");
    }

    #[test]
    fn test_command_topic() {
        let mut profile = FanProfile {
            mqtt_topic: "home/fan/bedroom".to_string(),
            use_set_suffix: true,
            ..Default::default()
        };
        assert_eq!(profile.command_topic(), "home/fan/bedroom/set");

        profile.use_set_suffix = false;
        assert_eq!(profile.command_topic(), "home/fan/bedroom");
    }

    // ========== Conversion Tests ==========

    #[test]
    fn test_fan_profile_to_stored() {
        let profile = FanProfile {
            id: Uuid::new_v4(),
            name: "Test Fan".to_string(),
            mqtt_topic: "home/fan/test".to_string(),
            use_set_suffix: false,
            payload_format: PayloadFormat::Percentage,
            zone_speeds: [10, 25, 40, 55, 70, 85, 100],
            use_power_zones: false,
            change_delay_secs: 5,
        };

        let user_id = Uuid::new_v4();
        let stored = profile.to_stored(user_id, true);

        assert_eq!(stored.id, profile.id);
        assert_eq!(stored.user_id, user_id);
        assert_eq!(stored.name, "Test Fan");
        assert!(stored.is_active);
        assert_eq!(stored.min_speed_pct, 10);
        assert_eq!(stored.max_speed_pct, 100);

        // Verify the zone_settings_json contains our settings
        let settings: FanProfileSettings =
            serde_json::from_str(&stored.zone_settings_json).unwrap();
        assert_eq!(settings.mqtt_topic, "home/fan/test");
        assert!(!settings.use_set_suffix);
        assert_eq!(settings.payload_format, PayloadFormat::Percentage);
        assert_eq!(settings.zone_speeds, [10, 25, 40, 55, 70, 85, 100]);
        assert!(!settings.use_power_zones);
        assert_eq!(settings.change_delay_secs, 5);
    }

    #[test]
    fn test_stored_to_fan_profile() {
        let settings = FanProfileSettings {
            mqtt_topic: "home/office/fan".to_string(),
            use_set_suffix: true,
            payload_format: PayloadFormat::JsonSpeedOnOff,
            zone_speeds: [5, 15, 30, 45, 60, 75, 90],
            use_power_zones: true,
            change_delay_secs: 2,
        };

        let id = Uuid::new_v4();
        let stored = StoredFanProfile {
            id,
            user_id: Uuid::new_v4(),
            name: "Office Fan".to_string(),
            is_active: true,
            zone_settings_json: serde_json::to_string(&settings).unwrap(),
            hr_thresholds_json: None,
            min_speed_pct: 5,
            max_speed_pct: 90,
            ramp_up_seconds: 0,
            ramp_down_seconds: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let profile = FanProfile::from(stored);

        assert_eq!(profile.id, id);
        assert_eq!(profile.name, "Office Fan");
        assert_eq!(profile.mqtt_topic, "home/office/fan");
        assert!(profile.use_set_suffix);
        assert_eq!(profile.payload_format, PayloadFormat::JsonSpeedOnOff);
        assert_eq!(profile.zone_speeds, [5, 15, 30, 45, 60, 75, 90]);
        assert!(profile.use_power_zones);
        assert_eq!(profile.change_delay_secs, 2);
    }

    #[test]
    fn test_stored_to_fan_profile_with_invalid_json() {
        // Test fallback to defaults when JSON is invalid
        let id = Uuid::new_v4();
        let stored = StoredFanProfile {
            id,
            user_id: Uuid::new_v4(),
            name: "Broken Fan".to_string(),
            is_active: false,
            zone_settings_json: "not valid json".to_string(),
            hr_thresholds_json: None,
            min_speed_pct: 0,
            max_speed_pct: 100,
            ramp_up_seconds: 0,
            ramp_down_seconds: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let profile = FanProfile::from(stored);

        assert_eq!(profile.id, id);
        assert_eq!(profile.name, "Broken Fan");
        // Should fall back to defaults
        assert_eq!(profile.mqtt_topic, "home/fan/living_room");
        assert!(profile.use_set_suffix);
        assert_eq!(profile.payload_format, PayloadFormat::JsonSpeed);
    }

    #[test]
    fn test_stored_to_fan_profile_backward_compatible_zone_speeds() {
        // Test backward compatibility with old format (just zone speeds array)
        let id = Uuid::new_v4();
        let stored = StoredFanProfile {
            id,
            user_id: Uuid::new_v4(),
            name: "Legacy Fan".to_string(),
            is_active: false,
            zone_settings_json: "[0, 20, 40, 60, 80, 90, 100]".to_string(),
            hr_thresholds_json: None,
            min_speed_pct: 0,
            max_speed_pct: 100,
            ramp_up_seconds: 0,
            ramp_down_seconds: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let profile = FanProfile::from(stored);

        assert_eq!(profile.id, id);
        assert_eq!(profile.name, "Legacy Fan");
        assert_eq!(profile.zone_speeds, [0, 20, 40, 60, 80, 90, 100]);
        // Other settings should be defaults
        assert_eq!(profile.mqtt_topic, "home/fan/living_room");
        assert!(profile.use_set_suffix);
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Test that converting to stored and back preserves all fields
        let original = FanProfile {
            id: Uuid::new_v4(),
            name: "Roundtrip Fan".to_string(),
            mqtt_topic: "home/gym/fan".to_string(),
            use_set_suffix: false,
            payload_format: PayloadFormat::SpeedOnly,
            zone_speeds: [0, 10, 30, 50, 70, 85, 100],
            use_power_zones: false,
            change_delay_secs: 10,
        };

        let user_id = Uuid::new_v4();
        let stored = original.to_stored(user_id, true);
        let restored = FanProfile::from(stored);

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.mqtt_topic, original.mqtt_topic);
        assert_eq!(restored.use_set_suffix, original.use_set_suffix);
        assert_eq!(restored.payload_format, original.payload_format);
        assert_eq!(restored.zone_speeds, original.zone_speeds);
        assert_eq!(restored.use_power_zones, original.use_power_zones);
        assert_eq!(restored.change_delay_secs, original.change_delay_secs);
    }

    #[test]
    fn test_fan_profile_settings_default() {
        let settings = FanProfileSettings::default();
        assert_eq!(settings.mqtt_topic, "home/fan/living_room");
        assert!(settings.use_set_suffix);
        assert_eq!(settings.payload_format, PayloadFormat::JsonSpeed);
        assert_eq!(settings.zone_speeds, [0, 20, 40, 60, 80, 90, 100]);
        assert!(settings.use_power_zones);
        assert_eq!(settings.change_delay_secs, 3);
    }
}
