//! Hardware integration data storage operations.
//!
//! Provides persistence for:
//! - ANT+ dongles
//! - Dual protocol bindings
//! - Fan profiles (MQTT)
//! - HID devices and button mappings
//! - Platform syncs (Strava, TrainingPeaks, etc.)
//! - Video syncs
//! - Audio settings

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::database::DatabaseError;

/// Hardware store for persisting hardware integration data.
pub struct HardwareStore<'a> {
    conn: &'a Connection,
}

// ========== ANT+ Dongle Types ==========

/// Stored ANT+ dongle record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAntDongle {
    pub id: Uuid,
    pub user_id: Uuid,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub name: String,
    pub firmware_version: Option<String>,
    pub status: String,
    pub last_connected_at: Option<String>,
    pub created_at: String,
}

// ========== Dual Protocol Binding Types ==========

/// Stored dual protocol binding record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDualProtocolBinding {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ble_device_id: String,
    pub ant_device_id: String,
    pub sensor_type: String,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub preferred_protocol: String,
    pub auto_detected: bool,
    pub created_at: String,
}

// ========== Fan Profile Types ==========

/// Stored fan profile record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFanProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub zone_settings_json: String,
    pub hr_thresholds_json: Option<String>,
    pub min_speed_pct: u8,
    pub max_speed_pct: u8,
    pub ramp_up_seconds: u32,
    pub ramp_down_seconds: u32,
    pub created_at: String,
    pub updated_at: String,
}

// ========== HID Device Types ==========

/// Stored HID device record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredHidDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub device_type: String,
    pub name: String,
    pub button_count: u8,
    pub has_display: bool,
    pub is_enabled: bool,
    pub created_at: String,
}

/// Stored button mapping record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredButtonMapping {
    pub id: Uuid,
    pub hid_device_id: Uuid,
    pub button_index: u8,
    pub action_type: String,
    pub action_params_json: Option<String>,
    pub hold_action_type: Option<String>,
    pub hold_action_params_json: Option<String>,
    pub hold_threshold_ms: u32,
    pub icon_path: Option<String>,
    pub label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ========== Platform Sync Types ==========

/// Stored platform sync record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlatformSync {
    pub id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub is_enabled: bool,
    pub auto_upload: bool,
    pub access_token_encrypted: Option<String>,
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<String>,
    pub scopes_json: Option<String>,
    pub athlete_id: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Stored sync record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSyncRecord {
    pub id: Uuid,
    pub platform_sync_id: Uuid,
    pub ride_id: Uuid,
    pub external_activity_id: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub uploaded_at: Option<String>,
    pub created_at: String,
}

// ========== Video Sync Types ==========

/// Stored video sync record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVideoSync {
    pub id: Uuid,
    pub route_id: String,
    pub video_path: String,
    pub total_route_distance_m: f32,
    pub duration_seconds: f32,
    pub recording_speed_kmh: f32,
    pub min_playback_speed: f32,
    pub max_playback_speed: f32,
    pub sync_points_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ========== Audio Settings Types ==========

/// Stored audio settings record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAudioSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub master_volume: u8,
    pub voice_enabled: bool,
    pub voice_volume: u8,
    pub voice_rate: f32,
    pub sound_effects_enabled: bool,
    pub effects_volume: u8,
    pub interval_alerts: bool,
    pub zone_change_alerts: bool,
    pub milestone_alerts: bool,
    pub cadence_alerts: bool,
    pub hr_alerts: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl<'a> HardwareStore<'a> {
    /// Create a new hardware store with the given connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== ANT+ Dongle Operations ==========

    /// Save an ANT+ dongle.
    pub fn save_ant_dongle(&self, dongle: &StoredAntDongle) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO ant_dongles (id, user_id, vendor_id, product_id, serial_number, name,
                    firmware_version, status, last_connected_at, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(user_id, vendor_id, product_id) DO UPDATE SET
                    serial_number = excluded.serial_number,
                    name = excluded.name,
                    firmware_version = excluded.firmware_version,
                    status = excluded.status,
                    last_connected_at = excluded.last_connected_at
                "#,
                params![
                    dongle.id.to_string(),
                    dongle.user_id.to_string(),
                    dongle.vendor_id as i32,
                    dongle.product_id as i32,
                    dongle.serial_number,
                    dongle.name,
                    dongle.firmware_version,
                    dongle.status,
                    dongle.last_connected_at,
                    dongle.created_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all ANT+ dongles for a user.
    pub fn get_ant_dongles(&self, user_id: &Uuid) -> Result<Vec<StoredAntDongle>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, vendor_id, product_id, serial_number, name,
                        firmware_version, status, last_connected_at, created_at
                 FROM ant_dongles WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredAntDongle {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    vendor_id: row.get::<_, i32>(2)? as u16,
                    product_id: row.get::<_, i32>(3)? as u16,
                    serial_number: row.get(4)?,
                    name: row.get(5)?,
                    firmware_version: row.get(6)?,
                    status: row.get(7)?,
                    last_connected_at: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut dongles = Vec::new();
        for row in rows {
            dongles.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(dongles)
    }

    /// Delete an ANT+ dongle.
    pub fn delete_ant_dongle(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM ant_dongles WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Dual Protocol Binding Operations ==========

    /// Save a dual protocol binding.
    pub fn save_dual_protocol_binding(
        &self,
        binding: &StoredDualProtocolBinding,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO dual_protocol_bindings (id, user_id, ble_device_id, ant_device_id,
                    sensor_type, manufacturer, serial_number, preferred_protocol, auto_detected, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(user_id, ble_device_id, ant_device_id) DO UPDATE SET
                    preferred_protocol = excluded.preferred_protocol
                "#,
                params![
                    binding.id.to_string(),
                    binding.user_id.to_string(),
                    binding.ble_device_id,
                    binding.ant_device_id,
                    binding.sensor_type,
                    binding.manufacturer,
                    binding.serial_number,
                    binding.preferred_protocol,
                    binding.auto_detected as i32,
                    binding.created_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all dual protocol bindings for a user.
    pub fn get_dual_protocol_bindings(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<StoredDualProtocolBinding>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, ble_device_id, ant_device_id, sensor_type, manufacturer,
                        serial_number, preferred_protocol, auto_detected, created_at
                 FROM dual_protocol_bindings WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredDualProtocolBinding {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    ble_device_id: row.get(2)?,
                    ant_device_id: row.get(3)?,
                    sensor_type: row.get(4)?,
                    manufacturer: row.get(5)?,
                    serial_number: row.get(6)?,
                    preferred_protocol: row.get(7)?,
                    auto_detected: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut bindings = Vec::new();
        for row in rows {
            bindings.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(bindings)
    }

    /// Delete a dual protocol binding.
    pub fn delete_dual_protocol_binding(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM dual_protocol_bindings WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Fan Profile Operations ==========

    /// Save a fan profile.
    pub fn save_fan_profile(&self, profile: &StoredFanProfile) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO fan_profiles (id, user_id, name, is_active, zone_settings_json,
                    hr_thresholds_json, min_speed_pct, max_speed_pct, ramp_up_seconds,
                    ramp_down_seconds, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    is_active = excluded.is_active,
                    zone_settings_json = excluded.zone_settings_json,
                    hr_thresholds_json = excluded.hr_thresholds_json,
                    min_speed_pct = excluded.min_speed_pct,
                    max_speed_pct = excluded.max_speed_pct,
                    ramp_up_seconds = excluded.ramp_up_seconds,
                    ramp_down_seconds = excluded.ramp_down_seconds,
                    updated_at = excluded.updated_at
                "#,
                params![
                    profile.id.to_string(),
                    profile.user_id.to_string(),
                    profile.name,
                    profile.is_active as i32,
                    profile.zone_settings_json,
                    profile.hr_thresholds_json,
                    profile.min_speed_pct as i32,
                    profile.max_speed_pct as i32,
                    profile.ramp_up_seconds as i32,
                    profile.ramp_down_seconds as i32,
                    profile.created_at,
                    profile.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all fan profiles for a user.
    pub fn get_fan_profiles(&self, user_id: &Uuid) -> Result<Vec<StoredFanProfile>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, name, is_active, zone_settings_json, hr_thresholds_json,
                        min_speed_pct, max_speed_pct, ramp_up_seconds, ramp_down_seconds,
                        created_at, updated_at
                 FROM fan_profiles WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredFanProfile {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    name: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    zone_settings_json: row.get(4)?,
                    hr_thresholds_json: row.get(5)?,
                    min_speed_pct: row.get::<_, i32>(6)? as u8,
                    max_speed_pct: row.get::<_, i32>(7)? as u8,
                    ramp_up_seconds: row.get::<_, i32>(8)? as u32,
                    ramp_down_seconds: row.get::<_, i32>(9)? as u32,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(profiles)
    }

    /// Get the active fan profile for a user.
    pub fn get_active_fan_profile(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<StoredFanProfile>, DatabaseError> {
        let profiles = self.get_fan_profiles(user_id)?;
        Ok(profiles.into_iter().find(|p| p.is_active))
    }

    /// Set a fan profile as active (deactivates others).
    pub fn set_active_fan_profile(
        &self,
        user_id: &Uuid,
        profile_id: &Uuid,
    ) -> Result<(), DatabaseError> {
        // Deactivate all profiles for user
        self.conn
            .execute(
                "UPDATE fan_profiles SET is_active = 0 WHERE user_id = ?1",
                params![user_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        // Activate the specified profile
        self.conn
            .execute(
                "UPDATE fan_profiles SET is_active = 1 WHERE id = ?1",
                params![profile_id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    /// Delete a fan profile.
    pub fn delete_fan_profile(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM fan_profiles WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== HID Device Operations ==========

    /// Save an HID device.
    pub fn save_hid_device(&self, device: &StoredHidDevice) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO hid_devices (id, user_id, vendor_id, product_id, serial_number,
                    device_type, name, button_count, has_display, is_enabled, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(user_id, vendor_id, product_id, serial_number) DO UPDATE SET
                    device_type = excluded.device_type,
                    name = excluded.name,
                    button_count = excluded.button_count,
                    has_display = excluded.has_display,
                    is_enabled = excluded.is_enabled
                "#,
                params![
                    device.id.to_string(),
                    device.user_id.to_string(),
                    device.vendor_id as i32,
                    device.product_id as i32,
                    device.serial_number,
                    device.device_type,
                    device.name,
                    device.button_count as i32,
                    device.has_display as i32,
                    device.is_enabled as i32,
                    device.created_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all HID devices for a user.
    pub fn get_hid_devices(&self, user_id: &Uuid) -> Result<Vec<StoredHidDevice>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, vendor_id, product_id, serial_number, device_type,
                        name, button_count, has_display, is_enabled, created_at
                 FROM hid_devices WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredHidDevice {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    vendor_id: row.get::<_, i32>(2)? as u16,
                    product_id: row.get::<_, i32>(3)? as u16,
                    serial_number: row.get(4)?,
                    device_type: row.get(5)?,
                    name: row.get(6)?,
                    button_count: row.get::<_, i32>(7)? as u8,
                    has_display: row.get::<_, i32>(8)? != 0,
                    is_enabled: row.get::<_, i32>(9)? != 0,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(devices)
    }

    /// Delete an HID device.
    pub fn delete_hid_device(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM hid_devices WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Button Mapping Operations ==========

    /// Save a button mapping.
    pub fn save_button_mapping(&self, mapping: &StoredButtonMapping) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO button_mappings (id, hid_device_id, button_index, action_type,
                    action_params_json, hold_action_type, hold_action_params_json, hold_threshold_ms,
                    icon_path, label, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(hid_device_id, button_index) DO UPDATE SET
                    action_type = excluded.action_type,
                    action_params_json = excluded.action_params_json,
                    hold_action_type = excluded.hold_action_type,
                    hold_action_params_json = excluded.hold_action_params_json,
                    hold_threshold_ms = excluded.hold_threshold_ms,
                    icon_path = excluded.icon_path,
                    label = excluded.label,
                    updated_at = excluded.updated_at
                "#,
                params![
                    mapping.id.to_string(),
                    mapping.hid_device_id.to_string(),
                    mapping.button_index as i32,
                    mapping.action_type,
                    mapping.action_params_json,
                    mapping.hold_action_type,
                    mapping.hold_action_params_json,
                    mapping.hold_threshold_ms as i32,
                    mapping.icon_path,
                    mapping.label,
                    mapping.created_at,
                    mapping.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all button mappings for an HID device.
    pub fn get_button_mappings(
        &self,
        hid_device_id: &Uuid,
    ) -> Result<Vec<StoredButtonMapping>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, hid_device_id, button_index, action_type, action_params_json,
                        hold_action_type, hold_action_params_json, hold_threshold_ms,
                        icon_path, label, created_at, updated_at
                 FROM button_mappings WHERE hid_device_id = ?1 ORDER BY button_index",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![hid_device_id.to_string()], |row| {
                Ok(StoredButtonMapping {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    hid_device_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    button_index: row.get::<_, i32>(2)? as u8,
                    action_type: row.get(3)?,
                    action_params_json: row.get(4)?,
                    hold_action_type: row.get(5)?,
                    hold_action_params_json: row.get(6)?,
                    hold_threshold_ms: row.get::<_, i32>(7)? as u32,
                    icon_path: row.get(8)?,
                    label: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut mappings = Vec::new();
        for row in rows {
            mappings.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(mappings)
    }

    /// Delete a button mapping.
    pub fn delete_button_mapping(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM button_mappings WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Platform Sync Operations ==========

    /// Save a platform sync configuration.
    pub fn save_platform_sync(&self, sync: &StoredPlatformSync) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO platform_syncs (id, user_id, platform, is_enabled, auto_upload,
                    access_token_encrypted, refresh_token_encrypted, token_expires_at,
                    scopes_json, athlete_id, last_sync_at, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(user_id, platform) DO UPDATE SET
                    is_enabled = excluded.is_enabled,
                    auto_upload = excluded.auto_upload,
                    access_token_encrypted = excluded.access_token_encrypted,
                    refresh_token_encrypted = excluded.refresh_token_encrypted,
                    token_expires_at = excluded.token_expires_at,
                    scopes_json = excluded.scopes_json,
                    athlete_id = excluded.athlete_id,
                    last_sync_at = excluded.last_sync_at,
                    updated_at = excluded.updated_at
                "#,
                params![
                    sync.id.to_string(),
                    sync.user_id.to_string(),
                    sync.platform,
                    sync.is_enabled as i32,
                    sync.auto_upload as i32,
                    sync.access_token_encrypted,
                    sync.refresh_token_encrypted,
                    sync.token_expires_at,
                    sync.scopes_json,
                    sync.athlete_id,
                    sync.last_sync_at,
                    sync.created_at,
                    sync.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get all platform syncs for a user.
    pub fn get_platform_syncs(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<StoredPlatformSync>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, platform, is_enabled, auto_upload, access_token_encrypted,
                        refresh_token_encrypted, token_expires_at, scopes_json, athlete_id,
                        last_sync_at, created_at, updated_at
                 FROM platform_syncs WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredPlatformSync {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    platform: row.get(2)?,
                    is_enabled: row.get::<_, i32>(3)? != 0,
                    auto_upload: row.get::<_, i32>(4)? != 0,
                    access_token_encrypted: row.get(5)?,
                    refresh_token_encrypted: row.get(6)?,
                    token_expires_at: row.get(7)?,
                    scopes_json: row.get(8)?,
                    athlete_id: row.get(9)?,
                    last_sync_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut syncs = Vec::new();
        for row in rows {
            syncs.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(syncs)
    }

    /// Get platform sync by platform name.
    pub fn get_platform_sync_by_platform(
        &self,
        user_id: &Uuid,
        platform: &str,
    ) -> Result<Option<StoredPlatformSync>, DatabaseError> {
        let syncs = self.get_platform_syncs(user_id)?;
        Ok(syncs.into_iter().find(|s| s.platform == platform))
    }

    /// Update last sync time.
    pub fn update_platform_last_sync(
        &self,
        id: &Uuid,
        last_sync_at: &str,
    ) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "UPDATE platform_syncs SET last_sync_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![last_sync_at, Utc::now().to_rfc3339(), id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Delete a platform sync.
    pub fn delete_platform_sync(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM platform_syncs WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Sync Record Operations ==========

    /// Save a sync record.
    pub fn save_sync_record(&self, record: &StoredSyncRecord) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO sync_records (id, platform_sync_id, ride_id, external_activity_id,
                    status, error_message, uploaded_at, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(platform_sync_id, ride_id) DO UPDATE SET
                    external_activity_id = excluded.external_activity_id,
                    status = excluded.status,
                    error_message = excluded.error_message,
                    uploaded_at = excluded.uploaded_at
                "#,
                params![
                    record.id.to_string(),
                    record.platform_sync_id.to_string(),
                    record.ride_id.to_string(),
                    record.external_activity_id,
                    record.status,
                    record.error_message,
                    record.uploaded_at,
                    record.created_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get sync records for a platform sync.
    pub fn get_sync_records(
        &self,
        platform_sync_id: &Uuid,
    ) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, platform_sync_id, ride_id, external_activity_id, status,
                        error_message, uploaded_at, created_at
                 FROM sync_records WHERE platform_sync_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![platform_sync_id.to_string()], |row| {
                Ok(StoredSyncRecord {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    platform_sync_id: Uuid::parse_str(&row.get::<_, String>(1)?)
                        .unwrap_or_default(),
                    ride_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    external_activity_id: row.get(3)?,
                    status: row.get(4)?,
                    error_message: row.get(5)?,
                    uploaded_at: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DatabaseError::QueryFailed(e.to_string()))?);
        }
        Ok(records)
    }

    /// Get pending sync records for auto-upload.
    pub fn get_pending_sync_records(
        &self,
        platform_sync_id: &Uuid,
    ) -> Result<Vec<StoredSyncRecord>, DatabaseError> {
        let records = self.get_sync_records(platform_sync_id)?;
        Ok(records
            .into_iter()
            .filter(|r| r.status == "pending")
            .collect())
    }

    // ========== Video Sync Operations ==========

    /// Save a video sync configuration.
    pub fn save_video_sync(&self, sync: &StoredVideoSync) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO video_syncs (id, route_id, video_path, total_route_distance_m,
                    duration_seconds, recording_speed_kmh, min_playback_speed, max_playback_speed,
                    sync_points_json, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    video_path = excluded.video_path,
                    total_route_distance_m = excluded.total_route_distance_m,
                    duration_seconds = excluded.duration_seconds,
                    recording_speed_kmh = excluded.recording_speed_kmh,
                    min_playback_speed = excluded.min_playback_speed,
                    max_playback_speed = excluded.max_playback_speed,
                    sync_points_json = excluded.sync_points_json,
                    updated_at = excluded.updated_at
                "#,
                params![
                    sync.id.to_string(),
                    sync.route_id,
                    sync.video_path,
                    sync.total_route_distance_m as f64,
                    sync.duration_seconds as f64,
                    sync.recording_speed_kmh as f64,
                    sync.min_playback_speed as f64,
                    sync.max_playback_speed as f64,
                    sync.sync_points_json,
                    sync.created_at,
                    sync.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get video sync for a route.
    pub fn get_video_sync(&self, route_id: &str) -> Result<Option<StoredVideoSync>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, route_id, video_path, total_route_distance_m, duration_seconds,
                        recording_speed_kmh, min_playback_speed, max_playback_speed,
                        sync_points_json, created_at, updated_at
                 FROM video_syncs WHERE route_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![route_id], |row| {
                Ok(StoredVideoSync {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    route_id: row.get(1)?,
                    video_path: row.get(2)?,
                    total_route_distance_m: row.get::<_, f64>(3)? as f32,
                    duration_seconds: row.get::<_, f64>(4)? as f32,
                    recording_speed_kmh: row.get::<_, f64>(5)? as f32,
                    min_playback_speed: row.get::<_, f64>(6)? as f32,
                    max_playback_speed: row.get::<_, f64>(7)? as f32,
                    sync_points_json: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(sync)) => Ok(Some(sync)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }

    /// Delete a video sync.
    pub fn delete_video_sync(&self, id: &Uuid) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                "DELETE FROM video_syncs WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    // ========== Audio Settings Operations ==========

    /// Save audio settings.
    pub fn save_audio_settings(&self, settings: &StoredAudioSettings) -> Result<(), DatabaseError> {
        self.conn
            .execute(
                r#"
                INSERT INTO audio_settings (id, user_id, master_volume, voice_enabled, voice_volume,
                    voice_rate, sound_effects_enabled, effects_volume, interval_alerts,
                    zone_change_alerts, milestone_alerts, cadence_alerts, hr_alerts,
                    created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                ON CONFLICT(user_id) DO UPDATE SET
                    master_volume = excluded.master_volume,
                    voice_enabled = excluded.voice_enabled,
                    voice_volume = excluded.voice_volume,
                    voice_rate = excluded.voice_rate,
                    sound_effects_enabled = excluded.sound_effects_enabled,
                    effects_volume = excluded.effects_volume,
                    interval_alerts = excluded.interval_alerts,
                    zone_change_alerts = excluded.zone_change_alerts,
                    milestone_alerts = excluded.milestone_alerts,
                    cadence_alerts = excluded.cadence_alerts,
                    hr_alerts = excluded.hr_alerts,
                    updated_at = excluded.updated_at
                "#,
                params![
                    settings.id.to_string(),
                    settings.user_id.to_string(),
                    settings.master_volume as i32,
                    settings.voice_enabled as i32,
                    settings.voice_volume as i32,
                    settings.voice_rate as f64,
                    settings.sound_effects_enabled as i32,
                    settings.effects_volume as i32,
                    settings.interval_alerts as i32,
                    settings.zone_change_alerts as i32,
                    settings.milestone_alerts as i32,
                    settings.cadence_alerts as i32,
                    settings.hr_alerts as i32,
                    settings.created_at,
                    settings.updated_at,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    /// Get audio settings for a user.
    pub fn get_audio_settings(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<StoredAudioSettings>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, master_volume, voice_enabled, voice_volume, voice_rate,
                        sound_effects_enabled, effects_volume, interval_alerts, zone_change_alerts,
                        milestone_alerts, cadence_alerts, hr_alerts, created_at, updated_at
                 FROM audio_settings WHERE user_id = ?1",
            )
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![user_id.to_string()], |row| {
                Ok(StoredAudioSettings {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    user_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    master_volume: row.get::<_, i32>(2)? as u8,
                    voice_enabled: row.get::<_, i32>(3)? != 0,
                    voice_volume: row.get::<_, i32>(4)? as u8,
                    voice_rate: row.get::<_, f64>(5)? as f32,
                    sound_effects_enabled: row.get::<_, i32>(6)? != 0,
                    effects_volume: row.get::<_, i32>(7)? as u8,
                    interval_alerts: row.get::<_, i32>(8)? != 0,
                    zone_change_alerts: row.get::<_, i32>(9)? != 0,
                    milestone_alerts: row.get::<_, i32>(10)? != 0,
                    cadence_alerts: row.get::<_, i32>(11)? != 0,
                    hr_alerts: row.get::<_, i32>(12)? != 0,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        match rows.next() {
            Some(Ok(settings)) => Ok(Some(settings)),
            Some(Err(e)) => Err(DatabaseError::QueryFailed(e.to_string())),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::storage::schema::SCHEMA).unwrap();
        conn.execute_batch(crate::storage::schema::MIGRATION_V5_TO_V6)
            .unwrap();
        conn
    }

    #[test]
    fn test_ant_dongle_crud() {
        let conn = setup_test_db();
        let store = HardwareStore::new(&conn);

        let user_id = Uuid::new_v4();
        let dongle = StoredAntDongle {
            id: Uuid::new_v4(),
            user_id,
            vendor_id: 0x0FCF,
            product_id: 0x1008,
            serial_number: Some("12345".to_string()),
            name: "Garmin USB2 Stick".to_string(),
            firmware_version: Some("1.0".to_string()),
            status: "connected".to_string(),
            last_connected_at: Some(Utc::now().to_rfc3339()),
            created_at: Utc::now().to_rfc3339(),
        };

        // Insert a dummy user first
        conn.execute(
            "INSERT INTO users (id, name, ftp, weight_kg, power_zones_json, created_at, updated_at)
             VALUES (?1, 'Test', 200, 70.0, '{}', datetime('now'), datetime('now'))",
            params![user_id.to_string()],
        )
        .unwrap();

        store.save_ant_dongle(&dongle).unwrap();

        let dongles = store.get_ant_dongles(&user_id).unwrap();
        assert_eq!(dongles.len(), 1);
        assert_eq!(dongles[0].name, "Garmin USB2 Stick");

        store.delete_ant_dongle(&dongle.id).unwrap();
        let dongles = store.get_ant_dongles(&user_id).unwrap();
        assert!(dongles.is_empty());
    }

    #[test]
    fn test_fan_profile_crud() {
        let conn = setup_test_db();
        let store = HardwareStore::new(&conn);

        let user_id = Uuid::new_v4();
        let profile = StoredFanProfile {
            id: Uuid::new_v4(),
            user_id,
            name: "Training Profile".to_string(),
            is_active: true,
            zone_settings_json: r#"{"zone1": 20, "zone2": 40}"#.to_string(),
            hr_thresholds_json: None,
            min_speed_pct: 10,
            max_speed_pct: 100,
            ramp_up_seconds: 5,
            ramp_down_seconds: 10,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Insert a dummy user first
        conn.execute(
            "INSERT INTO users (id, name, ftp, weight_kg, power_zones_json, created_at, updated_at)
             VALUES (?1, 'Test', 200, 70.0, '{}', datetime('now'), datetime('now'))",
            params![user_id.to_string()],
        )
        .unwrap();

        store.save_fan_profile(&profile).unwrap();

        let profiles = store.get_fan_profiles(&user_id).unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].is_active);

        let active = store.get_active_fan_profile(&user_id).unwrap();
        assert!(active.is_some());
    }

    #[test]
    fn test_audio_settings_crud() {
        let conn = setup_test_db();
        let store = HardwareStore::new(&conn);

        let user_id = Uuid::new_v4();
        let settings = StoredAudioSettings {
            id: Uuid::new_v4(),
            user_id,
            master_volume: 80,
            voice_enabled: true,
            voice_volume: 75,
            voice_rate: 1.0,
            sound_effects_enabled: true,
            effects_volume: 70,
            interval_alerts: true,
            zone_change_alerts: true,
            milestone_alerts: true,
            cadence_alerts: false,
            hr_alerts: false,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Insert a dummy user first
        conn.execute(
            "INSERT INTO users (id, name, ftp, weight_kg, power_zones_json, created_at, updated_at)
             VALUES (?1, 'Test', 200, 70.0, '{}', datetime('now'), datetime('now'))",
            params![user_id.to_string()],
        )
        .unwrap();

        store.save_audio_settings(&settings).unwrap();

        let loaded = store.get_audio_settings(&user_id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.master_volume, 80);
        assert!(loaded.voice_enabled);
    }
}
