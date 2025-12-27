//! Smart Fan Controller
//!
//! Controls smart fans via MQTT based on training zones.

use super::{MqttClient, MqttError, QoS};
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
}
