//! T074: Integration tests for MQTT fan control.
//!
//! Tests the MQTT client and fan controller integration with zone-based speed control.

use rustride::integrations::mqtt::{
    DefaultFanController, DefaultMqttClient, FanController, FanProfile, MqttClient, MqttConfig,
    MqttError, PayloadFormat, QoS,
};
use std::sync::Arc;
use uuid::Uuid;

/// Test MQTT client creation and basic state.
#[test]
fn test_mqtt_client_creation() {
    let client = DefaultMqttClient::new();
    assert!(!client.is_connected());
}

/// Test MQTT config defaults.
#[test]
fn test_mqtt_config_defaults() {
    let config = MqttConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.broker_host, "localhost");
    assert_eq!(config.broker_port, 1883);
    assert!(!config.use_tls);
    assert!(config.username.is_none());
    assert_eq!(config.keep_alive_secs, 60);
    assert_eq!(config.reconnect_interval_secs, 5);
}

/// Test fan profile defaults.
#[test]
fn test_fan_profile_defaults() {
    let profile = FanProfile::default();
    assert!(!profile.name.is_empty());
    assert!(!profile.mqtt_topic.is_empty());
    assert!(profile.use_set_suffix);
    assert_eq!(profile.payload_format, PayloadFormat::JsonSpeed);
    assert!(profile.use_power_zones);
    assert_eq!(profile.change_delay_secs, 3);

    // Zone speeds should have 7 entries
    assert_eq!(profile.zone_speeds.len(), 7);
    // Zone 1 should be low/off
    assert_eq!(profile.zone_speeds[0], 0);
    // Zone 7 should be max
    assert_eq!(profile.zone_speeds[6], 100);
}

/// Test fan profile command topic generation.
#[test]
fn test_fan_profile_command_topic() {
    let mut profile = FanProfile::default();
    profile.mqtt_topic = "home/fan/bedroom".to_string();

    // With /set suffix
    profile.use_set_suffix = true;
    assert_eq!(profile.command_topic(), "home/fan/bedroom/set");

    // Without /set suffix
    profile.use_set_suffix = false;
    assert_eq!(profile.command_topic(), "home/fan/bedroom");
}

/// Test fan profile payload formatting.
#[test]
fn test_fan_profile_payload_formats() {
    let mut profile = FanProfile::default();

    // Speed only format
    profile.payload_format = PayloadFormat::SpeedOnly;
    assert_eq!(profile.format_payload(75, true), "75");
    assert_eq!(profile.format_payload(0, false), "0");

    // JSON speed format
    profile.payload_format = PayloadFormat::JsonSpeed;
    assert_eq!(profile.format_payload(75, true), r#"{"speed": 75}"#);

    // JSON speed + on/off format
    profile.payload_format = PayloadFormat::JsonSpeedOnOff;
    assert_eq!(
        profile.format_payload(75, true),
        r#"{"speed": 75, "on": true}"#
    );
    assert_eq!(
        profile.format_payload(0, false),
        r#"{"speed": 0, "on": false}"#
    );

    // Percentage format
    profile.payload_format = PayloadFormat::Percentage;
    assert_eq!(profile.format_payload(75, true), "75%");
}

/// Test fan profile zone-to-speed mapping.
#[test]
fn test_fan_profile_zone_speed_mapping() {
    let profile = FanProfile::default();

    // Zone 1 should be 0%
    assert_eq!(profile.speed_for_zone(1), 0);

    // Zone 3 should be moderate
    assert_eq!(profile.speed_for_zone(3), 40);

    // Zone 7 should be max
    assert_eq!(profile.speed_for_zone(7), 100);

    // Zone 0 (invalid) should clamp to zone 1
    assert_eq!(profile.speed_for_zone(0), 0);

    // Zone 10 (invalid) should clamp to zone 7
    assert_eq!(profile.speed_for_zone(10), 100);
}

/// Test custom zone speed mapping.
#[test]
fn test_fan_profile_custom_zone_speeds() {
    let mut profile = FanProfile::default();

    // Set a "threshold only" profile (fan only at high zones)
    profile.zone_speeds = [0, 0, 0, 25, 50, 75, 100];

    assert_eq!(profile.speed_for_zone(1), 0);
    assert_eq!(profile.speed_for_zone(2), 0);
    assert_eq!(profile.speed_for_zone(3), 0);
    assert_eq!(profile.speed_for_zone(4), 25);
    assert_eq!(profile.speed_for_zone(5), 50);
    assert_eq!(profile.speed_for_zone(6), 75);
    assert_eq!(profile.speed_for_zone(7), 100);
}

/// Test fan controller creation and configuration.
#[test]
fn test_fan_controller_creation() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    // Configure with profiles
    let profiles = vec![
        FanProfile {
            id: Uuid::new_v4(),
            name: "Living Room".to_string(),
            mqtt_topic: "home/fan/living_room".to_string(),
            ..FanProfile::default()
        },
        FanProfile {
            id: Uuid::new_v4(),
            name: "Workout Room".to_string(),
            mqtt_topic: "home/fan/workout".to_string(),
            ..FanProfile::default()
        },
    ];

    fan_controller.configure(profiles);

    // States should be empty until started
    let states = fan_controller.get_states();
    assert!(states.is_empty());
}

/// Test fan controller start and state initialization.
#[tokio::test]
async fn test_fan_controller_start() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profile = FanProfile {
        id: Uuid::new_v4(),
        name: "Test Fan".to_string(),
        mqtt_topic: "home/fan/test".to_string(),
        ..FanProfile::default()
    };
    let profile_id = profile.id;

    fan_controller.configure(vec![profile]);

    // Start the controller
    let result = fan_controller.start().await;
    assert!(result.is_ok());

    // States should now be initialized
    let states = fan_controller.get_states();
    assert_eq!(states.len(), 1);

    // Check initial state
    let state = states.get(&profile_id).unwrap();
    assert_eq!(state.current_speed, 0);
    assert_eq!(state.last_zone, 1);
    assert!(state.auto_mode);
    assert!(!state.is_on);
}

/// Test fan controller stop.
#[tokio::test]
async fn test_fan_controller_stop() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    fan_controller.configure(vec![FanProfile::default()]);

    // Start then stop
    fan_controller.start().await.unwrap();
    let result = fan_controller.stop().await;
    assert!(result.is_ok());
}

/// Test fan controller auto mode toggle.
#[tokio::test]
async fn test_fan_controller_auto_mode() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profile = FanProfile::default();
    let profile_id = profile.id;

    fan_controller.configure(vec![profile]);
    fan_controller.start().await.unwrap();

    // Check initial auto mode
    let states = fan_controller.get_states();
    assert!(states.get(&profile_id).unwrap().auto_mode);

    // Disable auto mode
    fan_controller.set_auto_mode(&profile_id, false);

    let states = fan_controller.get_states();
    assert!(!states.get(&profile_id).unwrap().auto_mode);

    // Re-enable auto mode
    fan_controller.set_auto_mode(&profile_id, true);

    let states = fan_controller.get_states();
    assert!(states.get(&profile_id).unwrap().auto_mode);
}

/// Test MQTT publish without connection (should fail).
#[tokio::test]
async fn test_mqtt_publish_not_connected() {
    let client = DefaultMqttClient::new();

    let result = client
        .publish("test/topic", "payload", QoS::AtMostOnce)
        .await;
    assert!(matches!(result, Err(MqttError::NotConnected)));
}

/// Test MQTT subscribe without connection (should fail).
#[tokio::test]
async fn test_mqtt_subscribe_not_connected() {
    let client = DefaultMqttClient::new();

    let result = client.subscribe("test/topic", QoS::AtMostOnce).await;
    assert!(matches!(result, Err(MqttError::NotConnected)));
}

/// Test MQTT connect with disabled config (should fail).
#[tokio::test]
async fn test_mqtt_connect_disabled() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: false,
        ..Default::default()
    };

    let result = client.connect(&config).await;
    assert!(result.is_err());
}

/// Test MQTT event subscription.
#[test]
fn test_mqtt_event_subscription() {
    let client = DefaultMqttClient::new();
    let _receiver = client.subscribe_events();
    // Should be able to subscribe without panic
}

/// Test multiple fan profiles with different configurations.
#[tokio::test]
async fn test_multiple_fan_profiles() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profiles = vec![
        FanProfile {
            id: Uuid::new_v4(),
            name: "Power-based Fan".to_string(),
            mqtt_topic: "home/fan/power".to_string(),
            use_power_zones: true,
            zone_speeds: [0, 20, 40, 60, 80, 90, 100],
            ..FanProfile::default()
        },
        FanProfile {
            id: Uuid::new_v4(),
            name: "HR-based Fan".to_string(),
            mqtt_topic: "home/fan/hr".to_string(),
            use_power_zones: false,
            zone_speeds: [10, 30, 50, 70, 90, 95, 100],
            ..FanProfile::default()
        },
    ];

    fan_controller.configure(profiles.clone());
    fan_controller.start().await.unwrap();

    let states = fan_controller.get_states();
    assert_eq!(states.len(), 2);

    // Both fans should have initialized states
    for profile in &profiles {
        assert!(states.contains_key(&profile.id));
    }
}

/// Test fan profile with different payload formats integration.
#[test]
fn test_payload_format_integration() {
    // Test that all payload formats produce valid output
    let formats = [
        (PayloadFormat::SpeedOnly, "50"),
        (PayloadFormat::JsonSpeed, r#"{"speed": 50}"#),
        (
            PayloadFormat::JsonSpeedOnOff,
            r#"{"speed": 50, "on": true}"#,
        ),
        (PayloadFormat::Percentage, "50%"),
    ];

    for (format, expected) in formats {
        let profile = FanProfile {
            payload_format: format,
            ..FanProfile::default()
        };
        assert_eq!(profile.format_payload(50, true), expected);
    }
}

/// Test QoS levels.
#[test]
fn test_qos_levels() {
    assert_eq!(QoS::AtMostOnce as u8, 0);
    assert_eq!(QoS::AtLeastOnce as u8, 1);
    assert_eq!(QoS::ExactlyOnce as u8, 2);
}

/// Test fan profile with zero change delay (immediate response).
#[test]
fn test_fan_profile_zero_delay() {
    let profile = FanProfile {
        change_delay_secs: 0,
        ..FanProfile::default()
    };

    assert_eq!(profile.change_delay_secs, 0);
}

/// Test MQTT config with TLS settings.
#[test]
fn test_mqtt_config_tls() {
    let config = MqttConfig {
        enabled: true,
        broker_host: "mqtt.example.com".to_string(),
        broker_port: 8883,
        use_tls: true,
        username: Some("user".to_string()),
        ..Default::default()
    };

    assert!(config.enabled);
    assert!(config.use_tls);
    assert_eq!(config.broker_port, 8883);
    assert_eq!(config.username, Some("user".to_string()));
}
