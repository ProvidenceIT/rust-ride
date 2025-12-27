//! T093: Integration tests for HID button control.
//!
//! Tests the HID device manager, button mapping, and action execution.

use rustride::hid::{
    ButtonAction, ButtonInputHandler, ButtonMapping, DefaultButtonInputHandler,
    DefaultHidDeviceManager, HidConfig, HidDevice, HidDeviceManager, HidDeviceStatus,
    KNOWN_DEVICES,
};
use uuid::Uuid;

/// Test HID config defaults.
#[test]
fn test_hid_config_defaults() {
    let config = HidConfig::default();
    assert!(config.enabled);
    assert!(config.devices.is_empty());
}

/// Test known devices list.
#[test]
fn test_known_devices() {
    assert!(!KNOWN_DEVICES.is_empty());

    // Stream Deck should be in the list
    let stream_deck = KNOWN_DEVICES
        .iter()
        .find(|d| d.name.contains("Stream Deck"));
    assert!(stream_deck.is_some());

    // Elgato vendor ID
    assert!(KNOWN_DEVICES.iter().any(|d| d.vendor_id == 0x0FD9));
}

/// Test HID device creation.
#[test]
fn test_hid_device_creation() {
    let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

    assert!(device.is_known);
    assert_eq!(device.vendor_id, 0x0FD9);
    assert_eq!(device.product_id, 0x0060);
    assert_eq!(device.button_count, Some(15));
    assert_eq!(device.status, HidDeviceStatus::Detected);
    assert!(!device.is_open());
}

/// Test unknown device detection.
#[test]
fn test_unknown_device() {
    let device = HidDevice::new(0x1234, 0x5678, "Unknown USB".to_string());

    assert!(!device.is_known);
    assert!(device.button_count.is_none());
}

/// Test device display path.
#[test]
fn test_device_display_path() {
    let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
    assert_eq!(device.display_path(), "0FD9:0060");
}

/// Test HID device manager creation.
#[test]
fn test_device_manager_creation() {
    let config = HidConfig::default();
    let manager = DefaultHidDeviceManager::new(config);

    // Should be able to scan (returns empty without actual devices)
    let devices = manager.scan_devices();
    assert!(devices.is_empty());
}

/// Test button action display names.
#[test]
fn test_button_action_display_names() {
    assert_eq!(ButtonAction::AddLapMarker.display_name(), "Add Lap Marker");
    assert_eq!(ButtonAction::PauseResume.display_name(), "Pause/Resume");
    assert_eq!(ButtonAction::EndRide.display_name(), "End Ride");
    assert_eq!(ButtonAction::SkipInterval.display_name(), "Skip Interval");
    assert_eq!(ButtonAction::VolumeUp.display_name(), "Volume Up");
    assert_eq!(ButtonAction::FanSpeedUp.display_name(), "Fan Speed Up");
}

/// Test button action categories.
#[test]
fn test_button_action_categories() {
    use rustride::hid::actions::ActionCategory;

    assert_eq!(
        ButtonAction::AddLapMarker.category(),
        ActionCategory::RideControl
    );
    assert_eq!(
        ButtonAction::SkipInterval.category(),
        ActionCategory::WorkoutControl
    );
    assert_eq!(ButtonAction::VolumeUp.category(), ActionCategory::Audio);
    assert_eq!(ButtonAction::FanSpeedUp.category(), ActionCategory::Fan);
    assert_eq!(
        ButtonAction::ShowMetrics.category(),
        ActionCategory::Navigation
    );
    assert_eq!(
        ButtonAction::CameraZoomIn.category(),
        ActionCategory::Camera
    );
}

/// Test all available actions.
#[test]
fn test_all_button_actions() {
    let actions = ButtonAction::all_actions();
    assert!(!actions.is_empty());

    // Should include common actions
    assert!(actions.contains(&ButtonAction::AddLapMarker));
    assert!(actions.contains(&ButtonAction::PauseResume));
    assert!(actions.contains(&ButtonAction::SkipInterval));
}

/// Test button mapping creation.
#[test]
fn test_button_mapping_creation() {
    let device_id = Uuid::new_v4();
    let mapping = ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker);

    assert_eq!(mapping.device_id, device_id);
    assert_eq!(mapping.button_code, 1);
    assert_eq!(mapping.action, ButtonAction::AddLapMarker);
    assert!(mapping.enabled);
    assert!(mapping.label.is_none());
}

/// Test button mapping with label.
#[test]
fn test_button_mapping_with_label() {
    let device_id = Uuid::new_v4();
    let mapping =
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker).with_label("Lap Button");

    assert_eq!(mapping.label, Some("Lap Button".to_string()));
}

/// Test button input handler creation.
#[test]
fn test_button_input_handler_creation() {
    let handler = DefaultButtonInputHandler::new();
    assert!(!handler.is_learning());
    assert!(handler.get_learned_button().is_none());
}

/// Test registering button mappings.
#[test]
fn test_register_mappings() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mappings = vec![
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
        ButtonMapping::new(device_id, 2, ButtonAction::PauseResume),
        ButtonMapping::new(device_id, 3, ButtonAction::EndRide),
    ];

    handler.register_mappings(&device_id, mappings);

    let retrieved = handler.get_mappings(&device_id);
    assert_eq!(retrieved.len(), 3);
}

/// Test adding individual mapping.
#[test]
fn test_add_mapping() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mapping = ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker);
    handler.add_mapping(&device_id, mapping);

    let mappings = handler.get_mappings(&device_id);
    assert_eq!(mappings.len(), 1);
}

/// Test clearing mappings.
#[test]
fn test_clear_mappings() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mappings = vec![
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
        ButtonMapping::new(device_id, 2, ButtonAction::PauseResume),
    ];

    handler.register_mappings(&device_id, mappings);
    assert_eq!(handler.get_mappings(&device_id).len(), 2);

    handler.clear_mappings(&device_id);
    assert_eq!(handler.get_mappings(&device_id).len(), 0);
}

/// Test learning mode.
#[test]
fn test_learning_mode() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    assert!(!handler.is_learning());

    handler.start_learning_mode(&device_id);
    assert!(handler.is_learning());

    handler.stop_learning_mode();
    assert!(!handler.is_learning());
}

/// Test event subscription.
#[test]
fn test_event_subscription() {
    let handler = DefaultButtonInputHandler::new();

    let _action_rx = handler.subscribe_actions();
    let _raw_rx = handler.subscribe_raw();
    // Should be able to subscribe without panic
}

/// Test device manager event subscription.
#[test]
fn test_device_manager_events() {
    let config = HidConfig::default();
    let manager = DefaultHidDeviceManager::new(config);

    let _rx = manager.subscribe_events();
    // Should be able to subscribe without panic
}

/// Test device status transitions.
#[test]
fn test_device_status() {
    let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

    assert_eq!(device.status, HidDeviceStatus::Detected);
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Opening;
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Open;
    assert!(device.is_open());

    device.status = HidDeviceStatus::Error("test error".to_string());
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Disconnected;
    assert!(!device.is_open());
}

/// Test known device lookup.
#[test]
fn test_find_known_device() {
    use rustride::hid::find_known_device;

    // Stream Deck should be found
    let device = find_known_device(0x0FD9, 0x0060);
    assert!(device.is_some());
    assert_eq!(device.unwrap().name, "Elgato Stream Deck");

    // Unknown device should return None
    let unknown = find_known_device(0x1234, 0x5678);
    assert!(unknown.is_none());
}

/// Test action info creation.
#[test]
fn test_action_info() {
    use rustride::hid::actions::{ActionContext, ActionInfo};

    let info = ActionInfo::new(ButtonAction::AddLapMarker);
    assert_eq!(info.name, "Add Lap Marker");
    assert_eq!(info.available_during, ActionContext::DuringRide);

    let volume_info = ActionInfo::new(ButtonAction::VolumeUp);
    assert_eq!(volume_info.available_during, ActionContext::Always);

    let skip_info = ActionInfo::new(ButtonAction::SkipInterval);
    assert_eq!(skip_info.available_during, ActionContext::DuringWorkout);
}

/// Test device with serial number.
#[test]
fn test_device_with_serial() {
    let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
    device.serial_number = Some("ABC123456".to_string());

    assert_eq!(device.serial_number, Some("ABC123456".to_string()));
}

/// Test multiple devices.
#[test]
fn test_multiple_device_mappings() {
    let handler = DefaultButtonInputHandler::new();

    let device1 = Uuid::new_v4();
    let device2 = Uuid::new_v4();

    handler.register_mappings(
        &device1,
        vec![ButtonMapping::new(device1, 1, ButtonAction::AddLapMarker)],
    );
    handler.register_mappings(
        &device2,
        vec![ButtonMapping::new(device2, 1, ButtonAction::PauseResume)],
    );

    assert_eq!(handler.get_mappings(&device1).len(), 1);
    assert_eq!(handler.get_mappings(&device2).len(), 1);

    // Different actions for same button code on different devices
    let m1 = handler.get_mappings(&device1);
    let m2 = handler.get_mappings(&device2);
    assert_eq!(m1[0].action, ButtonAction::AddLapMarker);
    assert_eq!(m2[0].action, ButtonAction::PauseResume);
}
