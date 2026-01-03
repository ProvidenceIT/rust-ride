//! HID Configuration Persistence
//!
//! Conversion functions between HID runtime types and database storage types.
//! Enables loading and saving HID device configurations and button mappings
//! to the SQLite database.

use chrono::Utc;
use uuid::Uuid;

use super::actions::ButtonAction;
use super::device::HidDevice;
use super::{get_default_mappings, ButtonMappingConfig, HidConfig, HidDeviceConfig};
use crate::storage::hardware_store::{StoredButtonMapping, StoredHidDevice};

/// Convert a StoredHidDevice to an HidDeviceConfig.
///
/// Note: This creates an HidDeviceConfig with empty mappings.
/// Mappings should be loaded separately and added via `add_mappings_to_config`.
pub fn stored_device_to_config(stored: &StoredHidDevice) -> HidDeviceConfig {
    HidDeviceConfig {
        device_id: stored.id,
        vendor_id: stored.vendor_id,
        product_id: stored.product_id,
        name: stored.name.clone(),
        enabled: stored.is_enabled,
        mappings: Vec::new(),
    }
}

/// Create a new HidDeviceConfig from a detected HidDevice.
///
/// This is used when a new device is connected for the first time.
/// Default mappings are automatically applied for known devices.
pub fn new_device_config(device: &HidDevice) -> HidDeviceConfig {
    let defaults = get_default_mappings(device.vendor_id, device.product_id);

    if !defaults.is_empty() {
        tracing::info!(
            "Creating config for {} with {} default mappings",
            device.name,
            defaults.len()
        );
    }

    HidDeviceConfig {
        device_id: device.id,
        vendor_id: device.vendor_id,
        product_id: device.product_id,
        name: device.name.clone(),
        enabled: true, // Newly connected devices are enabled by default
        mappings: defaults,
    }
}

/// Create a new HidDeviceConfig from a detected HidDevice without default mappings.
///
/// Use this when you explicitly want an empty mapping configuration,
/// for example if the user has cleared all mappings.
pub fn new_device_config_without_defaults(device: &HidDevice) -> HidDeviceConfig {
    HidDeviceConfig {
        device_id: device.id,
        vendor_id: device.vendor_id,
        product_id: device.product_id,
        name: device.name.clone(),
        enabled: true,
        mappings: Vec::new(),
    }
}

/// Convert an HidDeviceConfig to a StoredHidDevice.
pub fn config_to_stored_device(config: &HidDeviceConfig, user_id: Uuid) -> StoredHidDevice {
    StoredHidDevice {
        id: config.device_id,
        user_id,
        vendor_id: config.vendor_id,
        product_id: config.product_id,
        serial_number: None, // Not stored in config
        device_type: "unknown".to_string(), // Could be derived from known devices
        name: config.name.clone(),
        button_count: 0, // Could be derived from known devices
        has_display: false, // Could be derived from known devices
        is_enabled: config.enabled,
        created_at: Utc::now().to_rfc3339(),
    }
}

/// Convert a StoredButtonMapping to a ButtonMappingConfig.
///
/// Returns None if the action_type cannot be parsed.
pub fn stored_mapping_to_config(stored: &StoredButtonMapping) -> Option<ButtonMappingConfig> {
    let action = parse_button_action(&stored.action_type, stored.action_params_json.as_deref())?;

    Some(ButtonMappingConfig {
        button_code: stored.button_index,
        action,
        label: stored.label.clone(),
    })
}

/// Convert a ButtonMappingConfig to a StoredButtonMapping.
pub fn config_to_stored_mapping(
    config: &ButtonMappingConfig,
    hid_device_id: Uuid,
) -> StoredButtonMapping {
    let (action_type, action_params_json) = serialize_button_action(&config.action);
    let now = Utc::now().to_rfc3339();

    StoredButtonMapping {
        id: Uuid::new_v4(),
        hid_device_id,
        button_index: config.button_code,
        action_type,
        action_params_json,
        hold_action_type: None,
        hold_action_params_json: None,
        hold_threshold_ms: 500, // Default hold threshold
        icon_path: None,
        label: config.label.clone(),
        created_at: now.clone(),
        updated_at: now,
    }
}

/// Parse a ButtonAction from action_type string and optional JSON params.
fn parse_button_action(action_type: &str, params_json: Option<&str>) -> Option<ButtonAction> {
    match action_type {
        // Ride control
        "AddLapMarker" => Some(ButtonAction::AddLapMarker),
        "PauseResume" => Some(ButtonAction::PauseResume),
        "EndRide" => Some(ButtonAction::EndRide),

        // Workout control
        "SkipInterval" => Some(ButtonAction::SkipInterval),
        "ExtendInterval" => {
            let seconds = params_json
                .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                .and_then(|v| v.get("seconds")?.as_u64())
                .unwrap_or(30) as u32;
            Some(ButtonAction::ExtendInterval { seconds })
        }
        "RestartInterval" => Some(ButtonAction::RestartInterval),

        // Audio control
        "VolumeUp" => Some(ButtonAction::VolumeUp),
        "VolumeDown" => Some(ButtonAction::VolumeDown),
        "MuteToggle" => Some(ButtonAction::MuteToggle),

        // Fan control
        "FanSpeedUp" => Some(ButtonAction::FanSpeedUp),
        "FanSpeedDown" => Some(ButtonAction::FanSpeedDown),
        "FanToggle" => Some(ButtonAction::FanToggle),

        // UI navigation
        "ShowMetrics" => Some(ButtonAction::ShowMetrics),
        "ShowMap" => Some(ButtonAction::ShowMap),
        "ShowWorkout" => Some(ButtonAction::ShowWorkout),
        "ToggleFullscreen" => Some(ButtonAction::ToggleFullscreen),

        // Camera
        "CameraZoomIn" => Some(ButtonAction::CameraZoomIn),
        "CameraZoomOut" => Some(ButtonAction::CameraZoomOut),
        "CameraRotate" => {
            let degrees = params_json
                .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                .and_then(|v| v.get("degrees")?.as_i64())
                .unwrap_or(45) as i16;
            Some(ButtonAction::CameraRotate { degrees })
        }

        // Custom
        "Custom" => {
            let command = params_json
                .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                .and_then(|v| v.get("command")?.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            Some(ButtonAction::Custom { command })
        }

        _ => {
            tracing::warn!("Unknown button action type: {}", action_type);
            None
        }
    }
}

/// Serialize a ButtonAction to action_type string and optional JSON params.
fn serialize_button_action(action: &ButtonAction) -> (String, Option<String>) {
    match action {
        // Ride control
        ButtonAction::AddLapMarker => ("AddLapMarker".to_string(), None),
        ButtonAction::PauseResume => ("PauseResume".to_string(), None),
        ButtonAction::EndRide => ("EndRide".to_string(), None),

        // Workout control
        ButtonAction::SkipInterval => ("SkipInterval".to_string(), None),
        ButtonAction::ExtendInterval { seconds } => (
            "ExtendInterval".to_string(),
            Some(format!(r#"{{"seconds":{}}}"#, seconds)),
        ),
        ButtonAction::RestartInterval => ("RestartInterval".to_string(), None),

        // Audio control
        ButtonAction::VolumeUp => ("VolumeUp".to_string(), None),
        ButtonAction::VolumeDown => ("VolumeDown".to_string(), None),
        ButtonAction::MuteToggle => ("MuteToggle".to_string(), None),

        // Fan control
        ButtonAction::FanSpeedUp => ("FanSpeedUp".to_string(), None),
        ButtonAction::FanSpeedDown => ("FanSpeedDown".to_string(), None),
        ButtonAction::FanToggle => ("FanToggle".to_string(), None),

        // UI navigation
        ButtonAction::ShowMetrics => ("ShowMetrics".to_string(), None),
        ButtonAction::ShowMap => ("ShowMap".to_string(), None),
        ButtonAction::ShowWorkout => ("ShowWorkout".to_string(), None),
        ButtonAction::ToggleFullscreen => ("ToggleFullscreen".to_string(), None),

        // Camera
        ButtonAction::CameraZoomIn => ("CameraZoomIn".to_string(), None),
        ButtonAction::CameraZoomOut => ("CameraZoomOut".to_string(), None),
        ButtonAction::CameraRotate { degrees } => (
            "CameraRotate".to_string(),
            Some(format!(r#"{{"degrees":{}}}"#, degrees)),
        ),

        // Custom
        ButtonAction::Custom { command } => (
            "Custom".to_string(),
            Some(
                serde_json::json!({ "command": command })
                    .to_string(),
            ),
        ),
    }
}

/// Load HID configuration from the database.
///
/// This loads all HID devices for the user and their button mappings,
/// converting them to an HidConfig for use by the application.
/// If a device has no existing mappings but is a known device with defaults,
/// the default mappings will be applied automatically.
pub fn load_hid_config_from_db(
    store: &crate::storage::hardware_store::HardwareStore,
    user_id: &Uuid,
) -> HidConfig {
    // Load all devices for the user
    let stored_devices = match store.get_hid_devices(user_id) {
        Ok(devices) => devices,
        Err(e) => {
            tracing::error!("Failed to load HID devices from database: {}", e);
            return HidConfig::default();
        }
    };

    // Convert devices and load their mappings
    let mut device_configs = Vec::new();
    let mut any_enabled = false;

    for stored_device in stored_devices {
        let mut config = stored_device_to_config(&stored_device);

        // Load mappings for this device
        match store.get_button_mappings(&stored_device.id) {
            Ok(stored_mappings) => {
                for stored_mapping in stored_mappings {
                    if let Some(mapping_config) = stored_mapping_to_config(&stored_mapping) {
                        config.mappings.push(mapping_config);
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load button mappings for device {}: {}",
                    stored_device.id,
                    e
                );
            }
        }

        // If no mappings exist, apply defaults for known devices
        if config.mappings.is_empty() {
            let defaults = get_default_mappings(config.vendor_id, config.product_id);
            if !defaults.is_empty() {
                tracing::info!(
                    "Applying {} default mappings for device {} (VID:{:04X} PID:{:04X})",
                    defaults.len(),
                    config.name,
                    config.vendor_id,
                    config.product_id
                );
                config.mappings = defaults;
            }
        }

        if config.enabled {
            any_enabled = true;
        }

        device_configs.push(config);
    }

    tracing::info!(
        "Loaded {} HID device(s) from database",
        device_configs.len()
    );

    HidConfig {
        enabled: any_enabled || device_configs.is_empty(), // Enable by default if no devices
        devices: device_configs,
        reconnect_delay_ms: 1000,
        auto_reconnect: true,
    }
}

/// Save HID configuration to the database.
///
/// This saves all HID devices and their button mappings to the database.
/// Existing devices and mappings are updated, new ones are created.
pub fn save_hid_config_to_db(
    store: &crate::storage::hardware_store::HardwareStore,
    user_id: &Uuid,
    config: &HidConfig,
) -> Result<(), crate::storage::database::DatabaseError> {
    for device_config in &config.devices {
        // Save the device
        let stored_device = config_to_stored_device(device_config, *user_id);
        store.save_hid_device(&stored_device)?;

        // Delete existing mappings and save new ones
        // First get existing mappings to delete them
        if let Ok(existing_mappings) = store.get_button_mappings(&device_config.device_id) {
            for mapping in existing_mappings {
                if let Err(e) = store.delete_button_mapping(&mapping.id) {
                    tracing::warn!("Failed to delete old mapping {}: {}", mapping.id, e);
                }
            }
        }

        // Save new mappings
        for mapping_config in &device_config.mappings {
            let stored_mapping =
                config_to_stored_mapping(mapping_config, device_config.device_id);
            store.save_button_mapping(&stored_mapping)?;
        }
    }

    tracing::info!("Saved {} HID device(s) to database", config.devices.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_action() {
        assert!(matches!(
            parse_button_action("AddLapMarker", None),
            Some(ButtonAction::AddLapMarker)
        ));
        assert!(matches!(
            parse_button_action("PauseResume", None),
            Some(ButtonAction::PauseResume)
        ));
        assert!(matches!(
            parse_button_action("VolumeUp", None),
            Some(ButtonAction::VolumeUp)
        ));
    }

    #[test]
    fn test_parse_action_with_params() {
        let action = parse_button_action("ExtendInterval", Some(r#"{"seconds": 60}"#));
        assert!(matches!(
            action,
            Some(ButtonAction::ExtendInterval { seconds: 60 })
        ));

        let action = parse_button_action("CameraRotate", Some(r#"{"degrees": -90}"#));
        assert!(matches!(
            action,
            Some(ButtonAction::CameraRotate { degrees: -90 })
        ));

        let action = parse_button_action("Custom", Some(r#"{"command": "my_command"}"#));
        if let Some(ButtonAction::Custom { command }) = action {
            assert_eq!(command, "my_command");
        } else {
            panic!("Expected Custom action");
        }
    }

    #[test]
    fn test_serialize_simple_action() {
        let (action_type, params) = serialize_button_action(&ButtonAction::AddLapMarker);
        assert_eq!(action_type, "AddLapMarker");
        assert!(params.is_none());

        let (action_type, params) = serialize_button_action(&ButtonAction::VolumeDown);
        assert_eq!(action_type, "VolumeDown");
        assert!(params.is_none());
    }

    #[test]
    fn test_serialize_action_with_params() {
        let (action_type, params) =
            serialize_button_action(&ButtonAction::ExtendInterval { seconds: 45 });
        assert_eq!(action_type, "ExtendInterval");
        assert!(params.is_some());
        let params = params.unwrap();
        assert!(params.contains("45"));

        let (action_type, params) =
            serialize_button_action(&ButtonAction::CameraRotate { degrees: 90 });
        assert_eq!(action_type, "CameraRotate");
        assert!(params.is_some());
        let params = params.unwrap();
        assert!(params.contains("90"));
    }

    #[test]
    fn test_roundtrip_actions() {
        // Test that serialize -> parse returns the same action
        let actions = vec![
            ButtonAction::AddLapMarker,
            ButtonAction::PauseResume,
            ButtonAction::EndRide,
            ButtonAction::SkipInterval,
            ButtonAction::ExtendInterval { seconds: 30 },
            ButtonAction::RestartInterval,
            ButtonAction::VolumeUp,
            ButtonAction::VolumeDown,
            ButtonAction::MuteToggle,
            ButtonAction::FanSpeedUp,
            ButtonAction::FanSpeedDown,
            ButtonAction::FanToggle,
            ButtonAction::ShowMetrics,
            ButtonAction::ShowMap,
            ButtonAction::ShowWorkout,
            ButtonAction::ToggleFullscreen,
            ButtonAction::CameraZoomIn,
            ButtonAction::CameraZoomOut,
            ButtonAction::CameraRotate { degrees: -45 },
            ButtonAction::Custom {
                command: "test_cmd".to_string(),
            },
        ];

        for original in actions {
            let (action_type, params) = serialize_button_action(&original);
            let parsed = parse_button_action(&action_type, params.as_deref());
            assert_eq!(parsed, Some(original.clone()), "Roundtrip failed for {:?}", original);
        }
    }

    #[test]
    fn test_stored_device_to_config() {
        let stored = StoredHidDevice {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            vendor_id: 0x0FD9,
            product_id: 0x0060,
            serial_number: Some("ABC123".to_string()),
            device_type: "streamdeck".to_string(),
            name: "Stream Deck".to_string(),
            button_count: 15,
            has_display: true,
            is_enabled: true,
            created_at: Utc::now().to_rfc3339(),
        };

        let config = stored_device_to_config(&stored);
        assert_eq!(config.device_id, stored.id);
        assert_eq!(config.vendor_id, stored.vendor_id);
        assert_eq!(config.product_id, stored.product_id);
        assert_eq!(config.name, stored.name);
        assert!(config.enabled);
        assert!(config.mappings.is_empty());
    }

    #[test]
    fn test_config_to_stored_device() {
        let config = HidDeviceConfig {
            device_id: Uuid::new_v4(),
            vendor_id: 0x0FD9,
            product_id: 0x0060,
            name: "Stream Deck".to_string(),
            enabled: true,
            mappings: Vec::new(),
        };
        let user_id = Uuid::new_v4();

        let stored = config_to_stored_device(&config, user_id);
        assert_eq!(stored.id, config.device_id);
        assert_eq!(stored.user_id, user_id);
        assert_eq!(stored.vendor_id, config.vendor_id);
        assert_eq!(stored.product_id, config.product_id);
        assert_eq!(stored.name, config.name);
        assert!(stored.is_enabled);
    }

    #[test]
    fn test_stored_mapping_to_config() {
        let stored = StoredButtonMapping {
            id: Uuid::new_v4(),
            hid_device_id: Uuid::new_v4(),
            button_index: 5,
            action_type: "PauseResume".to_string(),
            action_params_json: None,
            hold_action_type: None,
            hold_action_params_json: None,
            hold_threshold_ms: 500,
            icon_path: None,
            label: Some("Pause".to_string()),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let config = stored_mapping_to_config(&stored);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.button_code, 5);
        assert_eq!(config.action, ButtonAction::PauseResume);
        assert_eq!(config.label, Some("Pause".to_string()));
    }

    #[test]
    fn test_config_to_stored_mapping() {
        let config = ButtonMappingConfig {
            button_code: 3,
            action: ButtonAction::ExtendInterval { seconds: 60 },
            label: Some("Extend +60s".to_string()),
        };
        let device_id = Uuid::new_v4();

        let stored = config_to_stored_mapping(&config, device_id);
        assert_eq!(stored.hid_device_id, device_id);
        assert_eq!(stored.button_index, 3);
        assert_eq!(stored.action_type, "ExtendInterval");
        assert!(stored.action_params_json.is_some());
        assert!(stored.action_params_json.as_ref().unwrap().contains("60"));
        assert_eq!(stored.label, Some("Extend +60s".to_string()));
    }

    #[test]
    fn test_new_device_config_stream_deck() {
        // Stream Deck should get default mappings
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        let config = new_device_config(&device);

        assert_eq!(config.device_id, device.id);
        assert_eq!(config.vendor_id, 0x0FD9);
        assert_eq!(config.product_id, 0x0060);
        assert!(config.enabled);

        // Should have default mappings
        assert!(!config.mappings.is_empty());
        assert_eq!(config.mappings.len(), 9); // Stream Deck has 9 default mappings

        // First mapping should be PauseResume on button 0
        let first = &config.mappings[0];
        assert_eq!(first.button_code, 0);
        assert_eq!(first.action, ButtonAction::PauseResume);
        assert_eq!(first.label, Some("Pause/Resume".to_string()));
    }

    #[test]
    fn test_new_device_config_stream_deck_pedal() {
        // Stream Deck Pedal should get foot-friendly defaults
        let device = HidDevice::new(0x0FD9, 0x0086, "Stream Deck Pedal".to_string());
        let config = new_device_config(&device);

        assert_eq!(config.mappings.len(), 3); // All 3 pedals mapped

        // Verify foot-friendly mapping (hands-free controls)
        assert_eq!(config.mappings[0].action, ButtonAction::AddLapMarker);  // Left
        assert_eq!(config.mappings[1].action, ButtonAction::PauseResume);   // Center
        assert_eq!(config.mappings[2].action, ButtonAction::SkipInterval);  // Right
    }

    #[test]
    fn test_new_device_config_unknown_device() {
        // Unknown device should have no default mappings
        let device = HidDevice::new(0x1234, 0x5678, "Unknown Device".to_string());
        let config = new_device_config(&device);

        assert_eq!(config.device_id, device.id);
        assert!(config.enabled);
        assert!(config.mappings.is_empty());
    }

    #[test]
    fn test_new_device_config_without_defaults() {
        // Even known device should have no mappings when using without_defaults
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        let config = new_device_config_without_defaults(&device);

        assert_eq!(config.device_id, device.id);
        assert!(config.enabled);
        assert!(config.mappings.is_empty());
    }
}
