//! Unit tests for connection state machine.
//!
//! Tests verify that:
//! - State machine correctly handles lifecycle: Disconnected -> Connecting -> Connected -> Reconnecting
//! - All state transitions are handled correctly
//! - Invalid state combinations are rejected
//! - Edge cases are handled cleanly
//! - ConnectionStateManager tracks multiple devices independently

use rustride::sensors::connection_state::{
    ConnectionLifecycleState, ConnectionStateMachine, ConnectionStateMachineConfig,
    ConnectionStateManager, ConnectionStateStats, InvalidTransitionError, StateTransition,
};
use std::time::Duration;

// ============================================================================
// ConnectionLifecycleState Tests
// ============================================================================

#[test]
fn test_default_state_is_disconnected() {
    let state = ConnectionLifecycleState::default();
    assert_eq!(state, ConnectionLifecycleState::Disconnected);
}

#[test]
fn test_state_display() {
    assert_eq!(
        ConnectionLifecycleState::Disconnected.to_string(),
        "Disconnected"
    );
    assert_eq!(
        ConnectionLifecycleState::Connecting.to_string(),
        "Connecting..."
    );
    assert_eq!(
        ConnectionLifecycleState::Connected.to_string(),
        "Connected"
    );
    assert_eq!(
        ConnectionLifecycleState::Reconnecting.to_string(),
        "Reconnecting..."
    );
}

#[test]
fn test_transition_display() {
    assert_eq!(StateTransition::Connect.to_string(), "Connect");
    assert_eq!(StateTransition::ConnectionSuccess.to_string(), "Connection Success");
    assert_eq!(StateTransition::ConnectionFailed.to_string(), "Connection Failed");
    assert_eq!(StateTransition::ConnectionLost.to_string(), "Connection Lost");
    assert_eq!(StateTransition::Disconnect.to_string(), "Disconnect");
    assert_eq!(StateTransition::Reconnect.to_string(), "Reconnect");
    assert_eq!(StateTransition::ReconnectionSuccess.to_string(), "Reconnection Success");
    assert_eq!(StateTransition::ReconnectionFailed.to_string(), "Reconnection Failed");
    assert_eq!(StateTransition::ReconnectionExhausted.to_string(), "Reconnection Exhausted");
    assert_eq!(StateTransition::Reset.to_string(), "Reset");
}

// ============================================================================
// ConnectionStateMachineConfig Tests
// ============================================================================

#[test]
fn test_default_config() {
    let config = ConnectionStateMachineConfig::default();

    assert!(config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 5);
    assert_eq!(config.initial_reconnect_delay, Duration::from_secs(1));
    assert_eq!(config.max_reconnect_delay, Duration::from_secs(30));
    assert_eq!(config.connection_timeout, Duration::from_secs(10));
}

#[test]
fn test_no_reconnect_config() {
    let config = ConnectionStateMachineConfig::no_reconnect();

    assert!(!config.auto_reconnect);
}

#[test]
fn test_aggressive_config() {
    let config = ConnectionStateMachineConfig::aggressive();

    assert!(config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 10);
    assert_eq!(config.initial_reconnect_delay, Duration::from_millis(500));
    assert_eq!(config.max_reconnect_delay, Duration::from_secs(15));
    assert_eq!(config.connection_timeout, Duration::from_secs(5));
}

#[test]
fn test_conservative_config() {
    let config = ConnectionStateMachineConfig::conservative();

    assert!(config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 3);
    assert_eq!(config.initial_reconnect_delay, Duration::from_secs(2));
    assert_eq!(config.max_reconnect_delay, Duration::from_secs(60));
    assert_eq!(config.connection_timeout, Duration::from_secs(15));
}

// ============================================================================
// ConnectionStateMachine Basic Tests
// ============================================================================

#[test]
fn test_new_state_machine() {
    let sm = ConnectionStateMachine::new("device_a".to_string());

    assert_eq!(sm.device_id(), "device_a");
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    assert!(sm.is_disconnected());
    assert!(!sm.is_connected());
    assert!(!sm.is_connecting());
    assert_eq!(sm.reconnect_attempts(), 0);
    assert_eq!(sm.total_connections(), 0);
    assert_eq!(sm.total_disconnections(), 0);
}

#[test]
fn test_state_machine_with_custom_config() {
    let config = ConnectionStateMachineConfig::aggressive();
    let sm = ConnectionStateMachine::with_config("device_a".to_string(), config.clone());

    assert_eq!(sm.config().max_reconnect_attempts, 10);
}

// ============================================================================
// Valid Transition Tests: Disconnected -> Connecting -> Connected
// ============================================================================

#[test]
fn test_disconnected_to_connecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    let result = sm.transition(StateTransition::Connect);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Connecting);
    assert!(sm.is_connecting());
}

#[test]
fn test_connecting_to_connected() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();

    let result = sm.transition(StateTransition::ConnectionSuccess);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Connected);
    assert!(sm.is_connected());
    assert_eq!(sm.total_connections(), 1);
    assert!(sm.connected_at().is_some());
}

#[test]
fn test_connecting_to_disconnected_on_failure() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();

    let result = sm.transition(StateTransition::ConnectionFailed);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    assert_eq!(sm.total_connections(), 0);
}

#[test]
fn test_complete_connect_flow() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    // Disconnected -> Connecting
    assert!(sm.can_transition(StateTransition::Connect));
    sm.transition(StateTransition::Connect).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Connecting);

    // Connecting -> Connected
    assert!(sm.can_transition(StateTransition::ConnectionSuccess));
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Connected);
    assert!(sm.is_connected());
}

// ============================================================================
// Reconnection Flow Tests: Connected -> Reconnecting -> Connected/Disconnected
// ============================================================================

#[test]
fn test_connection_lost_triggers_reconnecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    let result = sm.transition(StateTransition::ConnectionLost);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
    assert!(sm.is_connecting());
    assert!(!sm.is_connected());
    assert_eq!(sm.total_disconnections(), 1);
    assert!(!sm.was_last_disconnect_intentional());
}

#[test]
fn test_connection_lost_without_auto_reconnect() {
    let config = ConnectionStateMachineConfig::no_reconnect();
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    let result = sm.transition(StateTransition::ConnectionLost);

    assert!(result.is_ok());
    // Without auto_reconnect, goes directly to Disconnected
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
}

#[test]
fn test_reconnection_success() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    let result = sm.transition(StateTransition::ReconnectionSuccess);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Connected);
    assert!(sm.is_connected());
    assert_eq!(sm.reconnect_attempts(), 0); // Reset on success
    assert_eq!(sm.total_connections(), 2);
}

#[test]
fn test_reconnection_failure_stays_reconnecting() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 3,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // First failure - still reconnecting
    let result = sm.transition(StateTransition::ReconnectionFailed);
    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
    assert_eq!(sm.reconnect_attempts(), 1);
    assert!(!sm.is_reconnection_exhausted());

    // Second failure - still reconnecting
    let result = sm.transition(StateTransition::ReconnectionFailed);
    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
    assert_eq!(sm.reconnect_attempts(), 2);
    assert!(!sm.is_reconnection_exhausted());
}

#[test]
fn test_reconnection_exhausted() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 2,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // First failure
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
    assert_eq!(sm.remaining_reconnect_attempts(), Some(1));

    // Second failure - exhausted, goes to Disconnected
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    assert!(sm.is_reconnection_exhausted());
    assert_eq!(sm.remaining_reconnect_attempts(), Some(0));
}

#[test]
fn test_reconnection_exhausted_explicit() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    let result = sm.transition(StateTransition::ReconnectionExhausted);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
}

#[test]
fn test_unlimited_reconnect_attempts() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 0, // Unlimited
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // Many failures - never exhausted
    for i in 0..100 {
        sm.transition(StateTransition::ReconnectionFailed).unwrap();
        assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
        assert_eq!(sm.reconnect_attempts(), i + 1);
        assert!(!sm.is_reconnection_exhausted());
        assert_eq!(sm.remaining_reconnect_attempts(), None);
    }
}

// ============================================================================
// Disconnect Tests
// ============================================================================

#[test]
fn test_intentional_disconnect_from_connected() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    let result = sm.transition(StateTransition::Disconnect);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    assert_eq!(sm.total_disconnections(), 1);
    assert!(sm.was_last_disconnect_intentional());
}

#[test]
fn test_disconnect_from_connecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();

    let result = sm.transition(StateTransition::Disconnect);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
}

#[test]
fn test_disconnect_from_reconnecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    let result = sm.transition(StateTransition::Disconnect);

    assert!(result.is_ok());
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
}

// ============================================================================
// Reset Tests
// ============================================================================

#[test]
fn test_reset_from_any_state() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    // Reset from Disconnected
    assert!(sm.can_transition(StateTransition::Reset));
    sm.transition(StateTransition::Reset).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);

    // Reset from Connecting
    sm.transition(StateTransition::Connect).unwrap();
    assert!(sm.can_transition(StateTransition::Reset));
    sm.transition(StateTransition::Reset).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);

    // Reset from Connected
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    assert!(sm.can_transition(StateTransition::Reset));
    sm.transition(StateTransition::Reset).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);

    // Reset from Reconnecting
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();
    assert!(sm.can_transition(StateTransition::Reset));
    sm.transition(StateTransition::Reset).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
}

#[test]
fn test_reset_clears_reconnect_attempts() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 5,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();

    assert_eq!(sm.reconnect_attempts(), 2);

    sm.reset();

    assert_eq!(sm.reconnect_attempts(), 0);
    assert!(!sm.is_reconnection_exhausted());
}

// ============================================================================
// Invalid Transition Tests
// ============================================================================

#[test]
fn test_invalid_transition_from_disconnected() {
    let sm = ConnectionStateMachine::new("device_a".to_string());

    // Cannot succeed, fail, or lose connection when disconnected
    assert!(!sm.can_transition(StateTransition::ConnectionSuccess));
    assert!(!sm.can_transition(StateTransition::ConnectionFailed));
    assert!(!sm.can_transition(StateTransition::ConnectionLost));
    assert!(!sm.can_transition(StateTransition::ReconnectionSuccess));
    assert!(!sm.can_transition(StateTransition::ReconnectionFailed));
}

#[test]
fn test_invalid_transition_from_connecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();

    // Cannot connect again or lose connection
    assert!(!sm.can_transition(StateTransition::Connect));
    assert!(!sm.can_transition(StateTransition::ConnectionLost));
    assert!(!sm.can_transition(StateTransition::ReconnectionSuccess));
}

#[test]
fn test_invalid_transition_from_connected() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    // Cannot connect again or succeed/fail connection
    assert!(!sm.can_transition(StateTransition::Connect));
    assert!(!sm.can_transition(StateTransition::ConnectionSuccess));
    assert!(!sm.can_transition(StateTransition::ConnectionFailed));
}

#[test]
fn test_invalid_transition_from_reconnecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // Cannot connect normally or succeed initial connection
    assert!(!sm.can_transition(StateTransition::Connect));
    assert!(!sm.can_transition(StateTransition::ConnectionSuccess));
}

#[test]
fn test_invalid_transition_error() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    let result = sm.transition(StateTransition::ConnectionSuccess);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.from_state, ConnectionLifecycleState::Disconnected);
    assert_eq!(error.transition, StateTransition::ConnectionSuccess);
    assert!(error.to_string().contains("Invalid transition"));
}

// ============================================================================
// Valid Transitions List Tests
// ============================================================================

#[test]
fn test_valid_transitions_from_disconnected() {
    let sm = ConnectionStateMachine::new("device_a".to_string());

    let valid = sm.valid_transitions();

    assert!(valid.contains(&StateTransition::Connect));
    assert!(valid.contains(&StateTransition::Reset));
    assert!(!valid.contains(&StateTransition::ConnectionSuccess));
}

#[test]
fn test_valid_transitions_from_connecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();

    let valid = sm.valid_transitions();

    assert!(valid.contains(&StateTransition::ConnectionSuccess));
    assert!(valid.contains(&StateTransition::ConnectionFailed));
    assert!(valid.contains(&StateTransition::Disconnect));
    assert!(valid.contains(&StateTransition::Reset));
    assert!(!valid.contains(&StateTransition::Connect));
}

#[test]
fn test_valid_transitions_from_connected() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    let valid = sm.valid_transitions();

    assert!(valid.contains(&StateTransition::ConnectionLost));
    assert!(valid.contains(&StateTransition::Disconnect));
    assert!(valid.contains(&StateTransition::Reset));
    assert!(!valid.contains(&StateTransition::Connect));
    assert!(!valid.contains(&StateTransition::ConnectionSuccess));
}

#[test]
fn test_valid_transitions_from_reconnecting() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    let valid = sm.valid_transitions();

    assert!(valid.contains(&StateTransition::ReconnectionSuccess));
    assert!(valid.contains(&StateTransition::ReconnectionFailed));
    assert!(valid.contains(&StateTransition::ReconnectionExhausted));
    assert!(valid.contains(&StateTransition::Disconnect));
    assert!(valid.contains(&StateTransition::Reset));
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[test]
fn test_stats_initial_state() {
    let sm = ConnectionStateMachine::new("device_a".to_string());

    let stats = sm.stats();

    assert_eq!(stats.device_id, "device_a");
    assert_eq!(stats.current_state, ConnectionLifecycleState::Disconnected);
    assert_eq!(stats.reconnect_attempts, 0);
    assert_eq!(stats.total_connections, 0);
    assert_eq!(stats.total_disconnections, 0);
    assert!(!stats.is_reconnection_exhausted);
    assert!(stats.connection_duration.is_none());
}

#[test]
fn test_stats_after_connection() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    let stats = sm.stats();

    assert_eq!(stats.current_state, ConnectionLifecycleState::Connected);
    assert_eq!(stats.total_connections, 1);
    assert!(stats.connection_duration.is_some());
}

#[test]
fn test_stats_after_reconnection_attempts() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 5,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();

    let stats = sm.stats();

    assert_eq!(stats.current_state, ConnectionLifecycleState::Reconnecting);
    assert_eq!(stats.reconnect_attempts, 2);
    assert_eq!(stats.remaining_attempts, Some(3));
    assert_eq!(stats.total_connections, 1);
    assert_eq!(stats.total_disconnections, 1);
}

#[test]
fn test_stats_status_text() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    // Disconnected
    assert_eq!(sm.stats().status_text(), "Disconnected");

    // Connecting
    sm.transition(StateTransition::Connect).unwrap();
    assert!(sm.stats().status_text().contains("Connecting"));

    // Connected
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    assert!(sm.stats().status_text().contains("Connected"));

    // Reconnecting
    sm.transition(StateTransition::ConnectionLost).unwrap();
    assert!(sm.stats().status_text().contains("Reconnecting"));
}

// ============================================================================
// ConnectionStateManager Tests
// ============================================================================

#[test]
fn test_manager_new_is_empty() {
    let manager = ConnectionStateManager::new();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_manager_get_or_create() {
    let mut manager = ConnectionStateManager::new();

    let sm = manager.get_or_create("device_a");
    assert_eq!(sm.device_id(), "device_a");
    assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);

    assert_eq!(manager.len(), 1);
}

#[test]
fn test_manager_transition() {
    let mut manager = ConnectionStateManager::new();

    let result = manager.transition("device_a", StateTransition::Connect);
    assert!(result.is_ok());
    assert_eq!(
        manager.get_state("device_a"),
        Some(ConnectionLifecycleState::Connecting)
    );
}

#[test]
fn test_manager_tracks_multiple_devices() {
    let mut manager = ConnectionStateManager::new();

    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();

    manager.transition("device_b", StateTransition::Connect).unwrap();

    assert_eq!(manager.len(), 2);
    assert!(manager.is_connected("device_a"));
    assert!(!manager.is_connected("device_b"));
}

#[test]
fn test_manager_get_connected_devices() {
    let mut manager = ConnectionStateManager::new();

    // Connect device_a
    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();

    // Connect device_b
    manager.transition("device_b", StateTransition::Connect).unwrap();
    manager.transition("device_b", StateTransition::ConnectionSuccess).unwrap();

    // Leave device_c disconnected
    manager.get_or_create("device_c");

    let connected = manager.get_connected_devices();

    assert_eq!(connected.len(), 2);
    assert!(connected.contains(&"device_a".to_string()));
    assert!(connected.contains(&"device_b".to_string()));
    assert!(!connected.contains(&"device_c".to_string()));
}

#[test]
fn test_manager_get_devices_in_state() {
    let mut manager = ConnectionStateManager::new();

    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();

    manager.transition("device_b", StateTransition::Connect).unwrap();
    manager.transition("device_b", StateTransition::ConnectionSuccess).unwrap();
    manager.transition("device_b", StateTransition::ConnectionLost).unwrap();

    manager.get_or_create("device_c");

    let connected = manager.get_devices_in_state(ConnectionLifecycleState::Connected);
    let reconnecting = manager.get_devices_in_state(ConnectionLifecycleState::Reconnecting);
    let disconnected = manager.get_devices_in_state(ConnectionLifecycleState::Disconnected);

    assert_eq!(connected.len(), 1);
    assert!(connected.contains(&"device_a".to_string()));

    assert_eq!(reconnecting.len(), 1);
    assert!(reconnecting.contains(&"device_b".to_string()));

    assert_eq!(disconnected.len(), 1);
    assert!(disconnected.contains(&"device_c".to_string()));
}

#[test]
fn test_manager_get_reconnecting_devices() {
    let mut manager = ConnectionStateManager::new();

    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();
    manager.transition("device_a", StateTransition::ConnectionLost).unwrap();

    let reconnecting = manager.get_reconnecting_devices();

    assert_eq!(reconnecting.len(), 1);
    assert!(reconnecting.contains(&"device_a".to_string()));
}

#[test]
fn test_manager_get_exhausted_devices() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 1,
        ..Default::default()
    };
    let mut manager = ConnectionStateManager::with_config(config);

    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();
    manager.transition("device_a", StateTransition::ConnectionLost).unwrap();
    manager.transition("device_a", StateTransition::ReconnectionFailed).unwrap();

    let exhausted = manager.get_exhausted_devices();

    assert_eq!(exhausted.len(), 1);
    assert!(exhausted.contains(&"device_a".to_string()));
}

#[test]
fn test_manager_remove() {
    let mut manager = ConnectionStateManager::new();

    manager.get_or_create("device_a");
    manager.get_or_create("device_b");

    assert_eq!(manager.len(), 2);

    let removed = manager.remove("device_a");
    assert!(removed.is_some());
    assert_eq!(manager.len(), 1);
    assert!(manager.get("device_a").is_none());
    assert!(manager.get("device_b").is_some());
}

#[test]
fn test_manager_clear() {
    let mut manager = ConnectionStateManager::new();

    manager.get_or_create("device_a");
    manager.get_or_create("device_b");
    manager.get_or_create("device_c");

    assert_eq!(manager.len(), 3);

    manager.clear();

    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
}

#[test]
fn test_manager_get_all_stats() {
    let mut manager = ConnectionStateManager::new();

    manager.transition("device_a", StateTransition::Connect).unwrap();
    manager.transition("device_a", StateTransition::ConnectionSuccess).unwrap();

    manager.get_or_create("device_b");

    let stats = manager.get_all_stats();

    assert_eq!(stats.len(), 2);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_multiple_connect_disconnect_cycles() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    for i in 0..5 {
        sm.transition(StateTransition::Connect).unwrap();
        sm.transition(StateTransition::ConnectionSuccess).unwrap();
        sm.transition(StateTransition::Disconnect).unwrap();

        assert_eq!(sm.total_connections(), i + 1);
        assert_eq!(sm.total_disconnections(), i + 1);
    }
}

#[test]
fn test_reconnection_success_resets_attempts() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 5,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("device_a".to_string(), config);

    // First connection cycle
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // Some failed reconnection attempts
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    assert_eq!(sm.reconnect_attempts(), 2);

    // Successful reconnection
    sm.transition(StateTransition::ReconnectionSuccess).unwrap();
    assert_eq!(sm.reconnect_attempts(), 0); // Reset

    // Another disconnect
    sm.transition(StateTransition::ConnectionLost).unwrap();
    assert_eq!(sm.reconnect_attempts(), 0); // Fresh start
    assert_eq!(sm.remaining_reconnect_attempts(), Some(5));
}

#[test]
fn test_time_in_state() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    let initial_time = sm.time_in_state();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let after_time = sm.time_in_state();

    assert!(after_time > initial_time);

    // State change resets timer
    sm.transition(StateTransition::Connect).unwrap();
    let new_state_time = sm.time_in_state();
    assert!(new_state_time < after_time);
}

#[test]
fn test_connection_duration_tracking() {
    let mut sm = ConnectionStateMachine::new("device_a".to_string());

    // No connection duration before connecting
    assert!(sm.connection_duration().is_none());

    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    // Should have connection duration after connecting
    std::thread::sleep(std::time::Duration::from_millis(10));
    let duration = sm.connection_duration();
    assert!(duration.is_some());
    assert!(duration.unwrap().as_millis() >= 10);
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

#[test]
fn test_wahoo_kickr_connection_scenario() {
    let mut sm = ConnectionStateMachine::new("wahoo_kickr_1234".to_string());

    // User pairs trainer
    sm.transition(StateTransition::Connect).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Connecting);

    // Connection established
    sm.transition(StateTransition::ConnectionSuccess).unwrap();
    assert!(sm.is_connected());

    // Mid-workout dropout
    sm.transition(StateTransition::ConnectionLost).unwrap();
    assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);

    // Reconnection succeeds
    sm.transition(StateTransition::ReconnectionSuccess).unwrap();
    assert!(sm.is_connected());
    assert_eq!(sm.total_connections(), 2);

    // User ends workout
    sm.transition(StateTransition::Disconnect).unwrap();
    assert!(sm.is_disconnected());
    assert!(sm.was_last_disconnect_intentional());
}

#[test]
fn test_heart_rate_monitor_persistent_failure() {
    let config = ConnectionStateMachineConfig {
        max_reconnect_attempts: 3,
        ..Default::default()
    };
    let mut sm = ConnectionStateMachine::with_config("polar_h10_5678".to_string(), config);

    // Connect successfully
    sm.transition(StateTransition::Connect).unwrap();
    sm.transition(StateTransition::ConnectionSuccess).unwrap();

    // Battery dies / moves out of range
    sm.transition(StateTransition::ConnectionLost).unwrap();

    // Multiple reconnection attempts fail
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();
    sm.transition(StateTransition::ReconnectionFailed).unwrap();

    // Should be exhausted now
    assert!(sm.is_disconnected());
    assert!(sm.is_reconnection_exhausted());

    let stats = sm.stats();
    assert!(stats.status_text().contains("gave up"));
}

#[test]
fn test_multi_sensor_workout() {
    let mut manager = ConnectionStateManager::new();

    // Connect trainer
    manager.transition("trainer", StateTransition::Connect).unwrap();
    manager.transition("trainer", StateTransition::ConnectionSuccess).unwrap();

    // Connect heart rate
    manager.transition("hr", StateTransition::Connect).unwrap();
    manager.transition("hr", StateTransition::ConnectionSuccess).unwrap();

    // Connect power meter
    manager.transition("power", StateTransition::Connect).unwrap();
    manager.transition("power", StateTransition::ConnectionSuccess).unwrap();

    // All three connected
    assert_eq!(manager.get_connected_devices().len(), 3);

    // HR drops out
    manager.transition("hr", StateTransition::ConnectionLost).unwrap();
    assert_eq!(manager.get_connected_devices().len(), 2);
    assert_eq!(manager.get_reconnecting_devices().len(), 1);

    // HR reconnects
    manager.transition("hr", StateTransition::ReconnectionSuccess).unwrap();
    assert_eq!(manager.get_connected_devices().len(), 3);
}
