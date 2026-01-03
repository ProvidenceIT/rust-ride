//! Connection state machine for sensor lifecycle management.
//!
//! This module provides a proper state machine for connection lifecycle:
//! Disconnected -> Connecting -> Connected -> Reconnecting.
//!
//! The state machine enforces valid transitions and handles edge cases cleanly.

use std::time::{Duration, Instant};

/// Connection states for the state machine.
///
/// States:
/// - Disconnected: Not connected, no active connection attempt
/// - Connecting: Initial connection attempt in progress
/// - Connected: Active connection with data flow
/// - Reconnecting: Automatic reconnection attempt after disconnect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionLifecycleState {
    /// Not connected, no active connection attempt.
    Disconnected,
    /// Initial connection attempt in progress.
    Connecting,
    /// Active connection with data flow.
    Connected,
    /// Automatic reconnection attempt after disconnect.
    Reconnecting,
}

impl Default for ConnectionLifecycleState {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl std::fmt::Display for ConnectionLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionLifecycleState::Disconnected => write!(f, "Disconnected"),
            ConnectionLifecycleState::Connecting => write!(f, "Connecting..."),
            ConnectionLifecycleState::Connected => write!(f, "Connected"),
            ConnectionLifecycleState::Reconnecting => write!(f, "Reconnecting..."),
        }
    }
}

/// Transitions that can occur in the connection state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransition {
    /// User or system requests to connect.
    Connect,
    /// Connection established successfully.
    ConnectionSuccess,
    /// Connection attempt failed.
    ConnectionFailed,
    /// Connection lost unexpectedly.
    ConnectionLost,
    /// User requests disconnect.
    Disconnect,
    /// System triggers reconnection attempt.
    Reconnect,
    /// Reconnection attempt succeeded.
    ReconnectionSuccess,
    /// Reconnection attempt failed (may retry or give up).
    ReconnectionFailed,
    /// Max reconnection attempts exhausted.
    ReconnectionExhausted,
    /// Reset state machine to initial state.
    Reset,
}

impl std::fmt::Display for StateTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateTransition::Connect => write!(f, "Connect"),
            StateTransition::ConnectionSuccess => write!(f, "Connection Success"),
            StateTransition::ConnectionFailed => write!(f, "Connection Failed"),
            StateTransition::ConnectionLost => write!(f, "Connection Lost"),
            StateTransition::Disconnect => write!(f, "Disconnect"),
            StateTransition::Reconnect => write!(f, "Reconnect"),
            StateTransition::ReconnectionSuccess => write!(f, "Reconnection Success"),
            StateTransition::ReconnectionFailed => write!(f, "Reconnection Failed"),
            StateTransition::ReconnectionExhausted => write!(f, "Reconnection Exhausted"),
            StateTransition::Reset => write!(f, "Reset"),
        }
    }
}

/// Error returned when an invalid transition is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTransitionError {
    /// The current state when transition was attempted.
    pub from_state: ConnectionLifecycleState,
    /// The transition that was attempted.
    pub transition: StateTransition,
}

impl std::fmt::Display for InvalidTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid transition '{}' from state '{}'",
            self.transition, self.from_state
        )
    }
}

impl std::error::Error for InvalidTransitionError {}

/// Result type for state transitions.
pub type TransitionResult = Result<ConnectionLifecycleState, InvalidTransitionError>;

/// Configuration for the connection state machine.
#[derive(Debug, Clone)]
pub struct ConnectionStateMachineConfig {
    /// Whether automatic reconnection is enabled.
    pub auto_reconnect: bool,
    /// Maximum number of reconnection attempts (0 = unlimited).
    pub max_reconnect_attempts: u32,
    /// Initial delay before first reconnection attempt.
    pub initial_reconnect_delay: Duration,
    /// Maximum delay between reconnection attempts.
    pub max_reconnect_delay: Duration,
    /// Timeout for connection attempts.
    pub connection_timeout: Duration,
}

impl Default for ConnectionStateMachineConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            initial_reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
        }
    }
}

impl ConnectionStateMachineConfig {
    /// Create a configuration with no automatic reconnection.
    pub fn no_reconnect() -> Self {
        Self {
            auto_reconnect: false,
            ..Self::default()
        }
    }

    /// Create a configuration for aggressive reconnection (more attempts, shorter delays).
    pub fn aggressive() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 10,
            initial_reconnect_delay: Duration::from_millis(500),
            max_reconnect_delay: Duration::from_secs(15),
            connection_timeout: Duration::from_secs(5),
        }
    }

    /// Create a configuration for conservative reconnection (fewer attempts, longer delays).
    pub fn conservative() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            initial_reconnect_delay: Duration::from_secs(2),
            max_reconnect_delay: Duration::from_secs(60),
            connection_timeout: Duration::from_secs(15),
        }
    }
}

/// Connection state machine for a single sensor.
///
/// Manages the connection lifecycle with proper state transitions,
/// tracking of timing information, and reconnection attempt counting.
#[derive(Debug, Clone)]
pub struct ConnectionStateMachine {
    /// Current state of the connection.
    state: ConnectionLifecycleState,
    /// Configuration for the state machine.
    config: ConnectionStateMachineConfig,
    /// Device identifier.
    device_id: String,
    /// When the current state was entered.
    state_entered_at: Instant,
    /// When the connection was first established (in current session).
    connected_at: Option<Instant>,
    /// When the last disconnect occurred.
    disconnected_at: Option<Instant>,
    /// Number of reconnection attempts in current reconnection cycle.
    reconnect_attempts: u32,
    /// Total number of successful connections (lifetime).
    total_connections: u32,
    /// Total number of disconnections (lifetime).
    total_disconnections: u32,
    /// Whether the last disconnect was intentional (user-initiated).
    last_disconnect_intentional: bool,
}

impl ConnectionStateMachine {
    /// Create a new connection state machine for a device.
    pub fn new(device_id: String) -> Self {
        Self::with_config(device_id, ConnectionStateMachineConfig::default())
    }

    /// Create a new connection state machine with custom configuration.
    pub fn with_config(device_id: String, config: ConnectionStateMachineConfig) -> Self {
        Self {
            state: ConnectionLifecycleState::Disconnected,
            config,
            device_id,
            state_entered_at: Instant::now(),
            connected_at: None,
            disconnected_at: None,
            reconnect_attempts: 0,
            total_connections: 0,
            total_disconnections: 0,
            last_disconnect_intentional: false,
        }
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionLifecycleState {
        self.state
    }

    /// Get how long the machine has been in the current state.
    pub fn time_in_state(&self) -> Duration {
        self.state_entered_at.elapsed()
    }

    /// Get when the current connection was established.
    pub fn connected_at(&self) -> Option<Instant> {
        self.connected_at
    }

    /// Get how long the current connection has been active.
    pub fn connection_duration(&self) -> Option<Duration> {
        self.connected_at.map(|t| t.elapsed())
    }

    /// Get the last disconnect time.
    pub fn disconnected_at(&self) -> Option<Instant> {
        self.disconnected_at
    }

    /// Get the number of reconnection attempts in the current cycle.
    pub fn reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts
    }

    /// Get the total number of successful connections.
    pub fn total_connections(&self) -> u32 {
        self.total_connections
    }

    /// Get the total number of disconnections.
    pub fn total_disconnections(&self) -> u32 {
        self.total_disconnections
    }

    /// Check if the last disconnect was intentional (user-initiated).
    pub fn was_last_disconnect_intentional(&self) -> bool {
        self.last_disconnect_intentional
    }

    /// Check if the connection is currently active.
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionLifecycleState::Connected
    }

    /// Check if a connection attempt is in progress.
    pub fn is_connecting(&self) -> bool {
        matches!(
            self.state,
            ConnectionLifecycleState::Connecting | ConnectionLifecycleState::Reconnecting
        )
    }

    /// Check if the connection is disconnected.
    pub fn is_disconnected(&self) -> bool {
        self.state == ConnectionLifecycleState::Disconnected
    }

    /// Check if reconnection attempts have been exhausted.
    pub fn is_reconnection_exhausted(&self) -> bool {
        if self.config.max_reconnect_attempts == 0 {
            false // Unlimited
        } else {
            self.reconnect_attempts >= self.config.max_reconnect_attempts
        }
    }

    /// Get remaining reconnection attempts.
    pub fn remaining_reconnect_attempts(&self) -> Option<u32> {
        if self.config.max_reconnect_attempts == 0 {
            None // Unlimited
        } else {
            Some(self.config.max_reconnect_attempts.saturating_sub(self.reconnect_attempts))
        }
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ConnectionStateMachineConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: ConnectionStateMachineConfig) {
        self.config = config;
    }

    /// Apply a transition to the state machine.
    ///
    /// Returns the new state if the transition is valid, or an error if invalid.
    pub fn transition(&mut self, transition: StateTransition) -> TransitionResult {
        let new_state = self.validate_transition(transition)?;
        self.apply_transition(transition, new_state);
        Ok(new_state)
    }

    /// Check if a transition is valid without applying it.
    pub fn can_transition(&self, transition: StateTransition) -> bool {
        self.validate_transition(transition).is_ok()
    }

    /// Get all valid transitions from the current state.
    pub fn valid_transitions(&self) -> Vec<StateTransition> {
        let all_transitions = [
            StateTransition::Connect,
            StateTransition::ConnectionSuccess,
            StateTransition::ConnectionFailed,
            StateTransition::ConnectionLost,
            StateTransition::Disconnect,
            StateTransition::Reconnect,
            StateTransition::ReconnectionSuccess,
            StateTransition::ReconnectionFailed,
            StateTransition::ReconnectionExhausted,
            StateTransition::Reset,
        ];

        all_transitions
            .iter()
            .filter(|&&t| self.can_transition(t))
            .copied()
            .collect()
    }

    /// Validate a transition and return the resulting state.
    fn validate_transition(&self, transition: StateTransition) -> TransitionResult {
        use ConnectionLifecycleState::*;
        use StateTransition::*;

        let new_state = match (self.state, transition) {
            // From Disconnected
            (Disconnected, Connect) => Connecting,
            (Disconnected, Reset) => Disconnected,

            // From Connecting
            (Connecting, ConnectionSuccess) => Connected,
            (Connecting, ConnectionFailed) => Disconnected,
            (Connecting, Disconnect) => Disconnected,
            (Connecting, Reset) => Disconnected,

            // From Connected
            (Connected, ConnectionLost) => {
                if self.config.auto_reconnect {
                    Reconnecting
                } else {
                    Disconnected
                }
            }
            (Connected, Disconnect) => Disconnected,
            (Connected, Reset) => Disconnected,

            // From Reconnecting
            (Reconnecting, ReconnectionSuccess) => Connected,
            (Reconnecting, ReconnectionFailed) => {
                // Stay in Reconnecting if more attempts allowed
                if self.reconnect_attempts + 1 < self.config.max_reconnect_attempts
                    || self.config.max_reconnect_attempts == 0
                {
                    Reconnecting
                } else {
                    Disconnected
                }
            }
            (Reconnecting, ReconnectionExhausted) => Disconnected,
            (Reconnecting, Disconnect) => Disconnected,
            (Reconnecting, Reset) => Disconnected,

            // Invalid transitions
            _ => {
                return Err(InvalidTransitionError {
                    from_state: self.state,
                    transition,
                });
            }
        };

        Ok(new_state)
    }

    /// Apply a validated transition to the state machine.
    fn apply_transition(&mut self, transition: StateTransition, new_state: ConnectionLifecycleState) {
        let old_state = self.state;
        let now = Instant::now();

        // Update state
        self.state = new_state;
        self.state_entered_at = now;

        // Handle state-specific logic
        match transition {
            StateTransition::ConnectionSuccess | StateTransition::ReconnectionSuccess => {
                self.connected_at = Some(now);
                self.total_connections += 1;
                self.reconnect_attempts = 0;
                self.last_disconnect_intentional = false;
            }
            StateTransition::ConnectionFailed => {
                // Initial connection failed - no reconnection tracking
            }
            StateTransition::ConnectionLost => {
                self.disconnected_at = Some(now);
                self.total_disconnections += 1;
                self.last_disconnect_intentional = false;
                self.reconnect_attempts = 0;
            }
            StateTransition::Disconnect => {
                self.disconnected_at = Some(now);
                self.total_disconnections += 1;
                self.last_disconnect_intentional = true;
                self.reconnect_attempts = 0;
            }
            StateTransition::ReconnectionFailed => {
                self.reconnect_attempts += 1;
            }
            StateTransition::ReconnectionExhausted => {
                // Already at max attempts
            }
            StateTransition::Reset => {
                self.reconnect_attempts = 0;
                self.connected_at = None;
            }
            StateTransition::Connect | StateTransition::Reconnect => {
                // State transition only
            }
        }

        // Log state change
        if old_state != new_state {
            tracing::debug!(
                "Sensor {} state transition: {} -> {} ({})",
                self.device_id,
                old_state,
                new_state,
                transition
            );
        }
    }

    /// Reset the state machine to initial disconnected state.
    pub fn reset(&mut self) {
        let _ = self.transition(StateTransition::Reset);
    }

    /// Get statistics about the connection state machine.
    pub fn stats(&self) -> ConnectionStateStats {
        ConnectionStateStats {
            device_id: self.device_id.clone(),
            current_state: self.state,
            time_in_state: self.time_in_state(),
            connection_duration: self.connection_duration(),
            reconnect_attempts: self.reconnect_attempts,
            remaining_attempts: self.remaining_reconnect_attempts(),
            total_connections: self.total_connections,
            total_disconnections: self.total_disconnections,
            is_reconnection_exhausted: self.is_reconnection_exhausted(),
        }
    }
}

/// Statistics about a connection state machine.
#[derive(Debug, Clone)]
pub struct ConnectionStateStats {
    /// Device identifier.
    pub device_id: String,
    /// Current connection state.
    pub current_state: ConnectionLifecycleState,
    /// Time in current state.
    pub time_in_state: Duration,
    /// Duration of current connection (if connected).
    pub connection_duration: Option<Duration>,
    /// Number of reconnection attempts in current cycle.
    pub reconnect_attempts: u32,
    /// Remaining reconnection attempts (None if unlimited).
    pub remaining_attempts: Option<u32>,
    /// Total number of successful connections.
    pub total_connections: u32,
    /// Total number of disconnections.
    pub total_disconnections: u32,
    /// Whether reconnection attempts have been exhausted.
    pub is_reconnection_exhausted: bool,
}

impl ConnectionStateStats {
    /// Get a human-readable status text.
    pub fn status_text(&self) -> String {
        match self.current_state {
            ConnectionLifecycleState::Disconnected => {
                if self.is_reconnection_exhausted {
                    format!("Disconnected (gave up after {} attempts)", self.reconnect_attempts)
                } else {
                    "Disconnected".to_string()
                }
            }
            ConnectionLifecycleState::Connecting => {
                format!("Connecting ({:.1}s)", self.time_in_state.as_secs_f32())
            }
            ConnectionLifecycleState::Connected => {
                if let Some(duration) = self.connection_duration {
                    format!("Connected ({:.1}s)", duration.as_secs_f32())
                } else {
                    "Connected".to_string()
                }
            }
            ConnectionLifecycleState::Reconnecting => {
                match self.remaining_attempts {
                    Some(remaining) => format!(
                        "Reconnecting (attempt {}, {} remaining)",
                        self.reconnect_attempts + 1,
                        remaining
                    ),
                    None => format!(
                        "Reconnecting (attempt {})",
                        self.reconnect_attempts + 1
                    ),
                }
            }
        }
    }
}

/// Manager for tracking connection state machines for multiple sensors.
#[derive(Debug, Default)]
pub struct ConnectionStateManager {
    /// Per-device state machines.
    machines: std::collections::HashMap<String, ConnectionStateMachine>,
    /// Default configuration for new state machines.
    default_config: ConnectionStateMachineConfig,
}

impl ConnectionStateManager {
    /// Create a new connection state manager.
    pub fn new() -> Self {
        Self {
            machines: std::collections::HashMap::new(),
            default_config: ConnectionStateMachineConfig::default(),
        }
    }

    /// Create a new connection state manager with custom default configuration.
    pub fn with_config(config: ConnectionStateMachineConfig) -> Self {
        Self {
            machines: std::collections::HashMap::new(),
            default_config: config,
        }
    }

    /// Get or create a state machine for a device.
    pub fn get_or_create(&mut self, device_id: &str) -> &mut ConnectionStateMachine {
        self.machines
            .entry(device_id.to_string())
            .or_insert_with(|| ConnectionStateMachine::with_config(
                device_id.to_string(),
                self.default_config.clone(),
            ))
    }

    /// Get a state machine for a device if it exists.
    pub fn get(&self, device_id: &str) -> Option<&ConnectionStateMachine> {
        self.machines.get(device_id)
    }

    /// Get a mutable state machine for a device if it exists.
    pub fn get_mut(&mut self, device_id: &str) -> Option<&mut ConnectionStateMachine> {
        self.machines.get_mut(device_id)
    }

    /// Apply a transition to a device's state machine.
    pub fn transition(
        &mut self,
        device_id: &str,
        transition: StateTransition,
    ) -> TransitionResult {
        self.get_or_create(device_id).transition(transition)
    }

    /// Get the current state of a device.
    pub fn get_state(&self, device_id: &str) -> Option<ConnectionLifecycleState> {
        self.machines.get(device_id).map(|m| m.state())
    }

    /// Check if a device is connected.
    pub fn is_connected(&self, device_id: &str) -> bool {
        self.machines
            .get(device_id)
            .map_or(false, |m| m.is_connected())
    }

    /// Get all connected devices.
    pub fn get_connected_devices(&self) -> Vec<String> {
        self.machines
            .iter()
            .filter(|(_, m)| m.is_connected())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all devices in a specific state.
    pub fn get_devices_in_state(&self, state: ConnectionLifecycleState) -> Vec<String> {
        self.machines
            .iter()
            .filter(|(_, m)| m.state() == state)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all devices that are currently reconnecting.
    pub fn get_reconnecting_devices(&self) -> Vec<String> {
        self.get_devices_in_state(ConnectionLifecycleState::Reconnecting)
    }

    /// Get devices with exhausted reconnection attempts.
    pub fn get_exhausted_devices(&self) -> Vec<String> {
        self.machines
            .iter()
            .filter(|(_, m)| m.is_reconnection_exhausted() && m.is_disconnected())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Remove a device from the manager.
    pub fn remove(&mut self, device_id: &str) -> Option<ConnectionStateMachine> {
        self.machines.remove(device_id)
    }

    /// Clear all state machines.
    pub fn clear(&mut self) {
        self.machines.clear();
    }

    /// Get the number of tracked devices.
    pub fn len(&self) -> usize {
        self.machines.len()
    }

    /// Check if any devices are tracked.
    pub fn is_empty(&self) -> bool {
        self.machines.is_empty()
    }

    /// Get statistics for all devices.
    pub fn get_all_stats(&self) -> Vec<ConnectionStateStats> {
        self.machines.values().map(|m| m.stats()).collect()
    }

    /// Get statistics for a specific device.
    pub fn get_stats(&self, device_id: &str) -> Option<ConnectionStateStats> {
        self.machines.get(device_id).map(|m| m.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state_is_disconnected() {
        let sm = ConnectionStateMachine::new("device_a".to_string());
        assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    }

    #[test]
    fn test_basic_connect_flow() {
        let mut sm = ConnectionStateMachine::new("device_a".to_string());

        // Disconnected -> Connecting
        let result = sm.transition(StateTransition::Connect);
        assert!(result.is_ok());
        assert_eq!(sm.state(), ConnectionLifecycleState::Connecting);

        // Connecting -> Connected
        let result = sm.transition(StateTransition::ConnectionSuccess);
        assert!(result.is_ok());
        assert_eq!(sm.state(), ConnectionLifecycleState::Connected);
        assert!(sm.is_connected());
    }

    #[test]
    fn test_connection_failed_returns_to_disconnected() {
        let mut sm = ConnectionStateMachine::new("device_a".to_string());

        sm.transition(StateTransition::Connect).unwrap();
        let result = sm.transition(StateTransition::ConnectionFailed);

        assert!(result.is_ok());
        assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
    }

    #[test]
    fn test_invalid_transition_returns_error() {
        let sm = ConnectionStateMachine::new("device_a".to_string());

        // Cannot transition to Connected from Disconnected directly
        assert!(!sm.can_transition(StateTransition::ConnectionSuccess));
    }

    #[test]
    fn test_connection_lost_triggers_reconnecting() {
        let mut sm = ConnectionStateMachine::new("device_a".to_string());

        sm.transition(StateTransition::Connect).unwrap();
        sm.transition(StateTransition::ConnectionSuccess).unwrap();

        // Connection lost should trigger reconnection
        let result = sm.transition(StateTransition::ConnectionLost);
        assert!(result.is_ok());
        assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
    }

    #[test]
    fn test_reconnection_success() {
        let mut sm = ConnectionStateMachine::new("device_a".to_string());

        sm.transition(StateTransition::Connect).unwrap();
        sm.transition(StateTransition::ConnectionSuccess).unwrap();
        sm.transition(StateTransition::ConnectionLost).unwrap();

        // Reconnection success
        let result = sm.transition(StateTransition::ReconnectionSuccess);
        assert!(result.is_ok());
        assert_eq!(sm.state(), ConnectionLifecycleState::Connected);
        assert_eq!(sm.reconnect_attempts(), 0); // Reset on success
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

        // First failure - still reconnecting
        sm.transition(StateTransition::ReconnectionFailed).unwrap();
        assert_eq!(sm.state(), ConnectionLifecycleState::Reconnecting);
        assert_eq!(sm.reconnect_attempts(), 1);

        // Second failure - exhausted
        sm.transition(StateTransition::ReconnectionFailed).unwrap();
        assert_eq!(sm.state(), ConnectionLifecycleState::Disconnected);
        assert!(sm.is_reconnection_exhausted());
    }

    #[test]
    fn test_stats() {
        let mut sm = ConnectionStateMachine::new("device_a".to_string());

        sm.transition(StateTransition::Connect).unwrap();
        sm.transition(StateTransition::ConnectionSuccess).unwrap();

        let stats = sm.stats();
        assert_eq!(stats.device_id, "device_a");
        assert_eq!(stats.current_state, ConnectionLifecycleState::Connected);
        assert_eq!(stats.total_connections, 1);
    }
}
