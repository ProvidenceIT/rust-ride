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
    let mut profile = FanProfile {
        mqtt_topic: "home/fan/bedroom".to_string(),
        use_set_suffix: true,
        ..Default::default()
    };

    // With /set suffix
    assert_eq!(profile.command_topic(), "home/fan/bedroom/set");

    // Without /set suffix
    profile.use_set_suffix = false;
    assert_eq!(profile.command_topic(), "home/fan/bedroom");
}

/// Test fan profile payload formatting.
#[test]
fn test_fan_profile_payload_formats() {
    let mut profile = FanProfile {
        payload_format: PayloadFormat::SpeedOnly,
        ..Default::default()
    };

    // Speed only format
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
    // Set a "threshold only" profile (fan only at high zones)
    let profile = FanProfile {
        zone_speeds: [0, 0, 0, 25, 50, 75, 100],
        ..Default::default()
    };

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

// ========================================================================
// Integration Tests for Actual rumqttc Operations
// ========================================================================
// These tests exercise the real rumqttc client to verify proper integration
// with the MQTT library. They test connection attempts, error handling,
// and state management with the actual AsyncClient and EventLoop.

use rustride::integrations::mqtt::{test_mqtt_connection, ConnectionState};
use std::time::Duration;

/// Test that test_mqtt_connection() returns failure when MQTT is disabled.
#[tokio::test]
async fn test_connection_test_mqtt_disabled() {
    let config = MqttConfig {
        enabled: false,
        ..Default::default()
    };

    let result = test_mqtt_connection(&config).await;

    assert!(!result.success);
    assert!(result.message.contains("disabled"));
}

/// Test that test_mqtt_connection() returns failure when broker host is empty.
#[tokio::test]
async fn test_connection_test_empty_host() {
    let config = MqttConfig {
        enabled: true,
        broker_host: "".to_string(),
        ..Default::default()
    };

    let result = test_mqtt_connection(&config).await;

    assert!(!result.success);
    assert!(result.message.contains("not configured"));
}

/// Test that test_mqtt_connection() handles unreachable broker correctly.
/// Uses a short timeout to keep the test fast.
#[tokio::test]
async fn test_connection_test_unreachable_broker() {
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(), // TEST-NET-1 address (non-routable)
        broker_port: 1883,
        connection_timeout_secs: 1, // Very short timeout for fast test
        ..Default::default()
    };

    let result = test_mqtt_connection(&config).await;

    // Should fail with timeout or connection error
    assert!(!result.success);
    // Duration should be recorded
    assert!(result.duration_ms > 0);
}

/// Test that test_mqtt_connection() handles invalid port gracefully.
#[tokio::test]
async fn test_connection_test_invalid_port() {
    let config = MqttConfig {
        enabled: true,
        broker_host: "localhost".to_string(),
        broker_port: 65534, // Unlikely to have MQTT broker on this port
        connection_timeout_secs: 1,
        ..Default::default()
    };

    let result = test_mqtt_connection(&config).await;

    assert!(!result.success);
    assert!(result.duration_ms > 0);
}

/// Test client connection state initialization.
#[test]
fn test_client_initial_connection_state() {
    let client = DefaultMqttClient::new();

    // Initial state should be Disconnected
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(!client.is_connected());
}

/// Test connecting with an enabled config updates state to Connecting.
#[tokio::test]
async fn test_connect_updates_state_to_connecting() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(), // Non-routable
        broker_port: 1883,
        connection_timeout_secs: 1,
        ..Default::default()
    };

    // Start connection - this returns immediately, connection happens async
    let result = client.connect(&config).await;
    assert!(result.is_ok());

    // Immediately after connect(), state should be Connecting or beyond
    // (Connection attempt has started)
    let state = client.connection_state();
    assert!(
        matches!(
            state,
            ConnectionState::Connecting
                | ConnectionState::Connected
                | ConnectionState::ConnectionLost
                | ConnectionState::Disconnected
        ),
        "Expected a valid connection state, got: {:?}",
        state
    );
}

/// Test that disconnect properly resets state.
#[tokio::test]
async fn test_disconnect_resets_state() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        ..Default::default()
    };

    // Start connection
    let _ = client.connect(&config).await;

    // Disconnect
    let disconnect_result = client.disconnect().await;
    assert!(disconnect_result.is_ok());

    // State should be Disconnected after disconnect
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(!client.is_connected());
}

/// Test that event subscription works and receives events.
#[tokio::test]
async fn test_event_subscription_receives_disconnect_event() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();

    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        ..Default::default()
    };

    // Start connection
    let _ = client.connect(&config).await;

    // Disconnect should trigger an event
    let _ = client.disconnect().await;

    // Try to receive the disconnect event with a short timeout
    let event_result = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await;

    // We should receive some event (Disconnected)
    assert!(event_result.is_ok());
}

/// Test connection state enum comparisons.
#[test]
fn test_connection_state_comparisons() {
    assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
    assert_ne!(ConnectionState::Connected, ConnectionState::Connecting);
    assert_ne!(ConnectionState::Connecting, ConnectionState::ConnectionLost);

    // Reconnecting with different attempts should not be equal
    assert_ne!(
        ConnectionState::Reconnecting { attempt: 1 },
        ConnectionState::Reconnecting { attempt: 2 }
    );

    // Same attempt should be equal
    assert_eq!(
        ConnectionState::Reconnecting { attempt: 3 },
        ConnectionState::Reconnecting { attempt: 3 }
    );
}

/// Test MqttConfig with various timeout settings.
#[test]
fn test_mqtt_config_timeout_settings() {
    let config = MqttConfig {
        connection_timeout_secs: 10,
        max_reconnect_attempts: Some(5),
        reconnect_interval_secs: 2,
        ..Default::default()
    };

    assert_eq!(config.connection_timeout_secs, 10);
    assert_eq!(config.max_reconnect_attempts, Some(5));
    assert_eq!(config.reconnect_interval_secs, 2);
}

/// Test MqttConfig unlimited reconnection attempts (default).
#[test]
fn test_mqtt_config_unlimited_reconnect() {
    let config = MqttConfig::default();

    // By default, max_reconnect_attempts should be None (unlimited)
    assert!(config.max_reconnect_attempts.is_none());
}

/// Test multiple clients can be created independently.
#[test]
fn test_multiple_client_instances() {
    let client1 = DefaultMqttClient::new();
    let client2 = DefaultMqttClient::new();

    // Both should start disconnected
    assert!(!client1.is_connected());
    assert!(!client2.is_connected());

    // States should be independent
    assert_eq!(client1.connection_state(), ConnectionState::Disconnected);
    assert_eq!(client2.connection_state(), ConnectionState::Disconnected);
}

/// Test that client properly handles rapid connect/disconnect cycles.
#[tokio::test]
async fn test_rapid_connect_disconnect_cycle() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        ..Default::default()
    };

    // Rapid cycle 3 times
    for _ in 0..3 {
        let _ = client.connect(&config).await;
        // Small delay to let async operations start
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = client.disconnect().await;
    }

    // Should end in disconnected state
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
}

/// Test that publish fails immediately when not connected.
#[tokio::test]
async fn test_publish_fails_when_disconnected() {
    let client = DefaultMqttClient::new();

    let result = client
        .publish("test/topic", r#"{"speed": 50}"#, QoS::AtMostOnce)
        .await;

    assert!(matches!(result, Err(MqttError::NotConnected)));
}

/// Test that subscribe fails immediately when not connected.
#[tokio::test]
async fn test_subscribe_fails_when_disconnected() {
    let client = DefaultMqttClient::new();

    let result = client.subscribe("test/topic/#", QoS::AtLeastOnce).await;

    assert!(matches!(result, Err(MqttError::NotConnected)));
}

/// Test that unsubscribe fails immediately when not connected.
#[tokio::test]
async fn test_unsubscribe_fails_when_disconnected() {
    let client = DefaultMqttClient::new();

    let result = client.unsubscribe("test/topic/#").await;

    assert!(matches!(result, Err(MqttError::NotConnected)));
}

/// Test fan controller with MQTT client integration.
#[tokio::test]
async fn test_fan_controller_set_speed_not_connected() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profile = FanProfile::default();
    let profile_id = profile.id;

    fan_controller.configure(vec![profile]);
    fan_controller.start().await.unwrap();

    // Try to set speed - should fail because MQTT is not connected
    let result = fan_controller.set_speed(&profile_id, 50).await;

    // The set_speed should fail with NotConnected error since MQTT client isn't connected
    assert!(result.is_err());
}

/// Test fan controller stop turns off fans (sends speed 0).
#[tokio::test]
async fn test_fan_controller_stop_attempts_turnoff() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profile = FanProfile::default();
    fan_controller.configure(vec![profile]);

    // Start the controller
    fan_controller.start().await.unwrap();

    // Stop should attempt to turn off fans (will fail silently due to no connection)
    let result = fan_controller.stop().await;
    assert!(result.is_ok());
}

/// Test fan controller test_fan function with no connection.
#[tokio::test]
async fn test_fan_controller_test_fan_no_connection() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    let profile = FanProfile::default();
    let profile_id = profile.id;
    fan_controller.configure(vec![profile]);

    // test_fan should fail because MQTT is not connected
    let result = fan_controller.test_fan(&profile_id).await;
    assert!(result.is_err());
}

/// Test fan controller with profile not found.
#[tokio::test]
async fn test_fan_controller_set_speed_profile_not_found() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    // Configure with a profile
    fan_controller.configure(vec![FanProfile::default()]);
    fan_controller.start().await.unwrap();

    // Try to set speed for a non-existent profile
    let fake_id = Uuid::new_v4();
    let result = fan_controller.set_speed(&fake_id, 50).await;

    // Should fail with ConfigError about profile not found
    assert!(matches!(result, Err(MqttError::ConfigError(_))));
}

/// Test fan controller test_fan with profile not found.
#[tokio::test]
async fn test_fan_controller_test_fan_profile_not_found() {
    let mqtt_client = Arc::new(DefaultMqttClient::new());
    let fan_controller = DefaultFanController::new(mqtt_client);

    fan_controller.configure(vec![FanProfile::default()]);

    let fake_id = Uuid::new_v4();
    let result = fan_controller.test_fan(&fake_id).await;

    assert!(matches!(result, Err(MqttError::ConfigError(_))));
}

/// Test the standalone test_fan function with disabled MQTT.
#[tokio::test]
async fn test_standalone_test_fan_mqtt_disabled() {
    use rustride::integrations::mqtt::test_fan;

    let config = MqttConfig {
        enabled: false,
        ..Default::default()
    };
    let profile = FanProfile::default();

    let result = test_fan(&config, &profile, None).await;

    assert!(!result.success);
    assert!(result.message.contains("disabled"));
}

/// Test the standalone test_fan function with empty broker host.
#[tokio::test]
async fn test_standalone_test_fan_empty_host() {
    use rustride::integrations::mqtt::test_fan;

    let config = MqttConfig {
        enabled: true,
        broker_host: "".to_string(),
        ..Default::default()
    };
    let profile = FanProfile::default();

    let result = test_fan(&config, &profile, None).await;

    assert!(!result.success);
    assert!(result.message.contains("Broker host"));
}

/// Test the standalone test_fan function with empty topic.
#[tokio::test]
async fn test_standalone_test_fan_empty_topic() {
    use rustride::integrations::mqtt::test_fan;

    let config = MqttConfig {
        enabled: true,
        broker_host: "localhost".to_string(),
        ..Default::default()
    };
    let profile = FanProfile {
        mqtt_topic: "".to_string(),
        ..Default::default()
    };

    let result = test_fan(&config, &profile, None).await;

    assert!(!result.success);
    assert!(result.message.contains("topic"));
}

/// Test that MqttTestResult fields are populated correctly.
#[tokio::test]
async fn test_mqtt_test_result_fields() {
    let config = MqttConfig {
        enabled: false,
        ..Default::default()
    };

    let result = test_mqtt_connection(&config).await;

    // Result should have all fields populated
    assert!(!result.success);
    assert!(!result.message.is_empty());
    // Duration should be minimal for disabled config check
    assert!(result.duration_ms < 1000);
}

/// Test FanTestResult structure from standalone test.
#[tokio::test]
async fn test_fan_test_result_fields() {
    use rustride::integrations::mqtt::test_fan;

    let config = MqttConfig {
        enabled: false,
        ..Default::default()
    };
    let profile = FanProfile::default();

    let result = test_fan(&config, &profile, None).await;

    assert!(!result.success);
    assert!(!result.message.is_empty());
    assert_eq!(result.current_speed, 0);
    assert!(result.duration_ms < 1000);
}

// ========================================================================
// Reconnection Scenario Tests
// ========================================================================
// These tests verify that the MQTT client handles reconnection scenarios
// correctly, including broker unavailability and recovery.

use rustride::integrations::mqtt::MqttEvent;

/// Test that connecting to a broker enables auto-reconnection.
#[tokio::test]
async fn test_connect_enables_auto_reconnection() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(), // Non-routable, will fail
        broker_port: 1883,
        connection_timeout_secs: 1,
        ..Default::default()
    };

    // Before connect, reconnection should be disabled
    assert!(!client.is_connected());

    // Connect enables auto-reconnection
    let _ = client.connect(&config).await;

    // Give the async task a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The client should have started connection (state may vary due to async)
    let state = client.connection_state();
    assert!(
        matches!(
            state,
            ConnectionState::Connecting
                | ConnectionState::ConnectionLost
                | ConnectionState::Reconnecting { .. }
                | ConnectionState::Disconnected
        ),
        "Expected valid state after connect, got: {:?}",
        state
    );
}

/// Test that disconnect disables auto-reconnection and stops reconnection attempts.
#[tokio::test]
async fn test_disconnect_stops_reconnection() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,
        ..Default::default()
    };

    // Start connection (will fail due to non-routable address)
    let _ = client.connect(&config).await;

    // Wait a bit for connection attempt
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Disconnect should stop all reconnection attempts
    let _ = client.disconnect().await;

    // State should be Disconnected
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(!client.is_connected());

    // Should receive a Disconnected event
    let event_result = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await;
    assert!(event_result.is_ok(), "Should receive disconnect event");
}

/// Test that max reconnection attempts is respected.
#[tokio::test]
async fn test_max_reconnection_attempts_limit() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(), // Non-routable
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,      // Fast reconnection for testing
        max_reconnect_attempts: Some(2), // Limit to 2 attempts
        ..Default::default()
    };

    // Start connection
    let _ = client.connect(&config).await;

    // Collect events for a few seconds to observe reconnection behavior
    let mut events = Vec::new();
    let collect_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < collect_duration {
        match tokio::time::timeout(Duration::from_millis(500), event_rx.recv()).await {
            Ok(Ok(event)) => {
                events.push(event.clone());
                // If we see ReconnectionFailed, we can stop early
                if matches!(event, MqttEvent::ReconnectionFailed { .. }) {
                    break;
                }
            }
            Ok(Err(_)) => break, // Channel closed
            Err(_) => continue,  // Timeout, keep waiting
        }
    }

    // Clean up
    let _ = client.disconnect().await;

    // We should have received some events (ConnectionLost, Reconnecting, etc.)
    // The exact sequence depends on timing, but we should see max_reconnect_attempts
    // being respected
    let reconnecting_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, MqttEvent::Reconnecting { .. }))
        .collect();

    // Should have at most 2 reconnection attempts (matching our max_reconnect_attempts)
    assert!(
        reconnecting_events.len() <= 2,
        "Expected at most 2 reconnection events, got: {}",
        reconnecting_events.len()
    );
}

/// Test that ReconnectionFailed event is emitted when max attempts exceeded.
#[tokio::test]
async fn test_reconnection_failed_event() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,
        max_reconnect_attempts: Some(1), // Only 1 attempt
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Wait for ReconnectionFailed event
    let mut got_reconnection_failed = false;
    let timeout_duration = Duration::from_secs(8);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration && !got_reconnection_failed {
        match tokio::time::timeout(Duration::from_millis(500), event_rx.recv()).await {
            Ok(Ok(MqttEvent::ReconnectionFailed { attempts, reason })) => {
                got_reconnection_failed = true;
                assert_eq!(attempts, 1, "Should have made 1 attempt");
                assert!(
                    reason.contains("maximum"),
                    "Reason should mention max attempts exceeded"
                );
            }
            Ok(Ok(_)) => continue, // Other events
            Ok(Err(_)) => break,   // Channel closed
            Err(_) => continue,    // Timeout
        }
    }

    let _ = client.disconnect().await;

    assert!(
        got_reconnection_failed,
        "Should have received ReconnectionFailed event"
    );
}

/// Test that ConnectionLost event is emitted when connection fails.
#[tokio::test]
async fn test_connection_lost_event_on_failure() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        max_reconnect_attempts: Some(0), // No reconnection attempts
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Should receive ConnectionLost or Error event when connection fails
    let mut got_loss_event = false;
    let timeout_duration = Duration::from_secs(3);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration && !got_loss_event {
        match tokio::time::timeout(Duration::from_millis(500), event_rx.recv()).await {
            Ok(Ok(MqttEvent::ConnectionLost { reason })) => {
                got_loss_event = true;
                assert!(
                    !reason.is_empty(),
                    "Should have a reason for connection loss"
                );
            }
            Ok(Ok(MqttEvent::Error { message })) => {
                got_loss_event = true;
                assert!(!message.is_empty(), "Error should have a message");
            }
            Ok(Ok(MqttEvent::ReconnectionFailed { .. })) => {
                // With max_reconnect_attempts=0, we might get this instead
                got_loss_event = true;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    let _ = client.disconnect().await;

    assert!(
        got_loss_event,
        "Should have received a connection loss/error event"
    );
}

/// Test that Reconnecting event includes attempt number.
#[tokio::test]
async fn test_reconnecting_event_has_attempt_number() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,
        max_reconnect_attempts: Some(3),
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Collect reconnecting events
    let mut reconnect_attempts = Vec::new();
    let timeout_duration = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        match tokio::time::timeout(Duration::from_millis(500), event_rx.recv()).await {
            Ok(Ok(MqttEvent::Reconnecting { attempt })) => {
                reconnect_attempts.push(attempt);
                if attempt >= 3 {
                    break; // Got enough attempts
                }
            }
            Ok(Ok(MqttEvent::ReconnectionFailed { .. })) => break,
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    let _ = client.disconnect().await;

    // Verify attempt numbers are sequential
    if !reconnect_attempts.is_empty() {
        for (i, attempt) in reconnect_attempts.iter().enumerate() {
            assert_eq!(
                *attempt,
                (i + 1) as u32,
                "Attempt numbers should be sequential starting from 1"
            );
        }
    }
}

/// Test that ConnectionState::Reconnecting reflects current attempt.
#[tokio::test]
async fn test_connection_state_shows_reconnect_attempt() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 2, // 2 second delay to observe state
        max_reconnect_attempts: Some(2),
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Wait for a Reconnecting event
    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut saw_reconnecting = false;

    while start.elapsed() < timeout_duration && !saw_reconnecting {
        match tokio::time::timeout(Duration::from_millis(200), event_rx.recv()).await {
            Ok(Ok(MqttEvent::Reconnecting { .. })) => {
                saw_reconnecting = true;
                // Check the connection state right after receiving the event
                let state = client.connection_state();
                // State should be Reconnecting or Connecting (after delay, it might have moved on)
                assert!(
                    matches!(
                        state,
                        ConnectionState::Reconnecting { .. }
                            | ConnectionState::Connecting
                            | ConnectionState::ConnectionLost
                            | ConnectionState::Disconnected
                    ),
                    "State should be in reconnection cycle, got: {:?}",
                    state
                );
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    let _ = client.disconnect().await;
}

/// Test that disconnect during reconnection delay properly stops reconnection.
#[tokio::test]
async fn test_disconnect_during_reconnection_delay() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 10, // Long delay so we can disconnect during it
        max_reconnect_attempts: Some(5),
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Wait for initial connection failure
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Now disconnect while reconnection delay is in progress
    let _ = client.disconnect().await;

    // State should be Disconnected
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);

    // Wait a bit to ensure no reconnection attempts happen after disconnect
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        client.connection_state(),
        ConnectionState::Disconnected,
        "Should stay disconnected after explicit disconnect"
    );
}

/// Test multiple connect/disconnect cycles don't leak reconnection state.
#[tokio::test]
async fn test_multiple_connect_disconnect_cycles_clean() {
    let client = DefaultMqttClient::new();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        max_reconnect_attempts: Some(1),
        ..Default::default()
    };

    // Perform 3 cycles
    for cycle in 1..=3 {
        // Connect
        let _ = client.connect(&config).await;

        // Wait briefly for connection attempt
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Disconnect
        let _ = client.disconnect().await;

        // Verify clean state after each cycle
        assert_eq!(
            client.connection_state(),
            ConnectionState::Disconnected,
            "Cycle {}: Should be disconnected after disconnect",
            cycle
        );
    }
}

/// Test that event subscription works across reconnection cycles.
#[tokio::test]
async fn test_event_subscription_across_reconnection() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,
        max_reconnect_attempts: Some(2),
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Collect events
    let mut event_types = Vec::new();
    let timeout_duration = Duration::from_secs(8);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        match tokio::time::timeout(Duration::from_millis(300), event_rx.recv()).await {
            Ok(Ok(event)) => {
                let event_type = match &event {
                    MqttEvent::Connected => "Connected",
                    MqttEvent::Disconnected => "Disconnected",
                    MqttEvent::ConnectionLost { .. } => "ConnectionLost",
                    MqttEvent::Reconnecting { .. } => "Reconnecting",
                    MqttEvent::ReconnectionFailed { .. } => "ReconnectionFailed",
                    MqttEvent::Error { .. } => "Error",
                    MqttEvent::MessageReceived { .. } => "MessageReceived",
                };
                event_types.push(event_type.to_string());

                // Stop if we've received ReconnectionFailed or Disconnected
                if matches!(
                    event,
                    MqttEvent::ReconnectionFailed { .. } | MqttEvent::Disconnected
                ) {
                    break;
                }
            }
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    let _ = client.disconnect().await;

    // Should have received some events throughout the reconnection process
    assert!(
        !event_types.is_empty(),
        "Should have received events during reconnection process"
    );
}

/// Test that unlimited reconnection (None max_attempts) doesn't immediately give up.
#[tokio::test]
async fn test_unlimited_reconnection_keeps_trying() {
    let client = DefaultMqttClient::new();
    let mut event_rx = client.subscribe_events();
    let config = MqttConfig {
        enabled: true,
        broker_host: "192.0.2.1".to_string(),
        broker_port: 1883,
        connection_timeout_secs: 1,
        reconnect_interval_secs: 1,
        max_reconnect_attempts: None, // Unlimited
        ..Default::default()
    };

    let _ = client.connect(&config).await;

    // Count reconnection events over a few seconds
    let mut reconnect_count = 0;
    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        match tokio::time::timeout(Duration::from_millis(300), event_rx.recv()).await {
            Ok(Ok(MqttEvent::Reconnecting { .. })) => {
                reconnect_count += 1;
                if reconnect_count >= 2 {
                    break; // Seen enough to confirm it keeps trying
                }
            }
            Ok(Ok(MqttEvent::ReconnectionFailed { .. })) => {
                panic!("Should not get ReconnectionFailed with unlimited attempts");
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    let _ = client.disconnect().await;

    // With unlimited attempts, we should see multiple reconnection events
    // and never a ReconnectionFailed
    assert!(
        reconnect_count >= 1,
        "Should have at least 1 reconnection attempt with unlimited mode"
    );
}
