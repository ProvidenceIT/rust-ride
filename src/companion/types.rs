//! Type definitions for the companion server API.
//!
//! Defines request, response, and event types for WebSocket communication
//! between the desktop app and mobile companion apps.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Configuration for the companion server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionConfig {
    /// Whether the companion server is enabled.
    pub enabled: bool,
    /// Port to listen on (default: 9876).
    pub port: u16,
    /// Whether to require PIN authentication.
    pub require_pin: bool,
    /// Session timeout in seconds (0 = no timeout).
    pub session_timeout_secs: u32,
    /// Maximum number of concurrent connections.
    pub max_connections: u8,
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9876,
            require_pin: true,
            session_timeout_secs: 3600, // 1 hour
            max_connections: 5,
        }
    }
}

/// Error types for companion server operations.
#[derive(Debug, Error)]
pub enum CompanionError {
    /// Failed to bind to the specified port.
    #[error("Failed to bind to port {0}: {1}")]
    BindFailed(u16, String),

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Session not found or expired.
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    /// No active workout or ride session.
    #[error("No active session")]
    NoActiveSession,

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Invalid message format.
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Server is not running.
    #[error("Server is not running")]
    ServerNotRunning,

    /// Maximum connections reached.
    #[error("Maximum connections reached")]
    MaxConnectionsReached,

    /// Internal server error.
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Request messages from mobile companion to desktop app.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompanionRequest {
    /// Authenticate with PIN.
    Auth {
        /// The PIN code entered by the user.
        pin: String,
    },

    /// Request current session status.
    GetSessionStatus,

    /// Subscribe to real-time metrics updates.
    SubscribeMetrics,

    /// Unsubscribe from metrics updates.
    UnsubscribeMetrics,

    /// Pause the active workout.
    WorkoutPause,

    /// Resume a paused workout.
    WorkoutResume,

    /// Skip to the next interval.
    WorkoutSkip,

    /// Stop the active workout or ride.
    WorkoutStop,

    /// Adjust resistance/grade (for free rides).
    AdjustResistance {
        /// Delta to apply to current resistance (-100 to 100).
        delta: i8,
    },

    /// Request ride history list.
    GetRideHistory {
        /// Maximum number of rides to return.
        limit: u32,
        /// Offset for pagination.
        offset: u32,
    },

    /// Request details for a specific ride.
    GetRideDetails {
        /// The ride ID to retrieve.
        ride_id: String,
    },

    /// Ping to keep connection alive.
    Ping,
}

/// Response messages from desktop app to mobile companion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompanionResponse {
    /// Authentication successful.
    AuthOk {
        /// Session ID for this connection.
        session_id: Uuid,
    },

    /// Authentication failed.
    AuthFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Session status response.
    SessionStatus {
        /// Whether there's an active session.
        active: bool,
        /// Session details if active.
        session: Option<SessionStatusInfo>,
    },

    /// Metrics subscription confirmed.
    SubscribedMetrics,

    /// Metrics unsubscription confirmed.
    UnsubscribedMetrics,

    /// Command executed successfully.
    CommandOk {
        /// The command that was executed.
        command: String,
    },

    /// Command execution failed.
    CommandFailed {
        /// The command that failed.
        command: String,
        /// Error message.
        error: String,
    },

    /// Ride history list.
    RideHistory {
        /// List of rides.
        rides: Vec<RideSummary>,
        /// Total number of rides.
        total: u32,
    },

    /// Ride details.
    RideDetails {
        /// The requested ride details.
        ride: RideDetailInfo,
    },

    /// Pong response to ping.
    Pong,

    /// Error response.
    Error {
        /// Error code.
        code: CompanionErrorCode,
        /// Error message.
        message: String,
    },
}

/// Event messages pushed from desktop to mobile companion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompanionEvent {
    /// Real-time metrics update (pushed at 1Hz).
    Metrics {
        /// Current power in watts.
        power_watts: Option<u16>,
        /// Current heart rate in BPM.
        heart_rate_bpm: Option<u8>,
        /// Current cadence in RPM.
        cadence_rpm: Option<u8>,
        /// Current speed in km/h.
        speed_kmh: Option<f32>,
        /// Total distance in km.
        distance_km: f32,
        /// Elapsed time in seconds.
        elapsed_secs: u32,
        /// Calories burned.
        calories: u32,
    },

    /// Session state changed.
    SessionStateChanged {
        /// New session state.
        state: SessionState,
        /// Session details if applicable.
        session: Option<SessionStatusInfo>,
    },

    /// Workout interval changed.
    IntervalChanged {
        /// Current interval index.
        interval_index: usize,
        /// Total intervals.
        total_intervals: usize,
        /// Interval name.
        interval_name: String,
        /// Target power in watts.
        target_power_watts: u16,
        /// Duration of this interval in seconds.
        duration_secs: u32,
    },

    /// Connection will be terminated.
    Disconnecting {
        /// Reason for disconnection.
        reason: String,
    },
}

/// Error codes for companion responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompanionErrorCode {
    /// Authentication required.
    AuthRequired,
    /// Invalid PIN.
    InvalidPin,
    /// No active session.
    NoSession,
    /// Session already active (cannot start new).
    SessionActive,
    /// Command not recognized.
    UnknownCommand,
    /// Invalid request parameters.
    InvalidParams,
    /// Rate limit exceeded.
    RateLimited,
    /// Internal server error.
    InternalError,
}

/// Session state for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// No active session.
    Idle,
    /// Session starting.
    Starting,
    /// Session active and running.
    Active,
    /// Session paused.
    Paused,
    /// Session ending.
    Stopping,
    /// Session completed.
    Completed,
}

/// Session status information for responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusInfo {
    /// Session ID.
    pub session_id: Uuid,
    /// Session type (free_ride or workout).
    pub session_type: String,
    /// Workout name (if structured workout).
    pub workout_name: Option<String>,
    /// Workout file path (if structured workout).
    pub workout_path: Option<PathBuf>,
    /// Whether session is paused.
    pub is_paused: bool,
    /// Elapsed time in seconds.
    pub elapsed_secs: u32,
    /// Current interval index (for workouts).
    pub current_interval_index: Option<usize>,
    /// Total intervals (for workouts).
    pub total_intervals: Option<usize>,
    /// Current interval name (for workouts).
    pub current_interval_name: Option<String>,
    /// Target power in watts (for ERG mode).
    pub target_power_watts: Option<u16>,
    /// Time remaining in current interval (for workouts).
    pub interval_remaining_secs: Option<u32>,
}

/// Summary of a ride for history list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RideSummary {
    /// Ride ID.
    pub ride_id: String,
    /// Ride date/time as ISO8601 string.
    pub started_at: String,
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Distance in km.
    pub distance_km: f32,
    /// Average power in watts.
    pub avg_power_watts: Option<u16>,
    /// Whether this was a structured workout.
    pub is_workout: bool,
    /// Workout name if applicable.
    pub workout_name: Option<String>,
}

/// Detailed information about a specific ride.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RideDetailInfo {
    /// Ride ID.
    pub ride_id: String,
    /// Ride start time as ISO8601 string.
    pub started_at: String,
    /// Ride end time as ISO8601 string.
    pub ended_at: String,
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Distance in km.
    pub distance_km: f32,
    /// Calories burned.
    pub calories: u32,
    /// Average power in watts.
    pub avg_power_watts: Option<u16>,
    /// Max power in watts.
    pub max_power_watts: Option<u16>,
    /// Normalized power in watts.
    pub normalized_power_watts: Option<u16>,
    /// Average heart rate in BPM.
    pub avg_heart_rate_bpm: Option<u8>,
    /// Max heart rate in BPM.
    pub max_heart_rate_bpm: Option<u8>,
    /// Average cadence in RPM.
    pub avg_cadence_rpm: Option<u8>,
    /// TSS (Training Stress Score).
    pub tss: Option<f32>,
    /// IF (Intensity Factor).
    pub intensity_factor: Option<f32>,
    /// Whether this was a structured workout.
    pub is_workout: bool,
    /// Workout name if applicable.
    pub workout_name: Option<String>,
}

/// Information about a connected companion client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionClient {
    /// Session ID for this client.
    pub session_id: Uuid,
    /// When the client connected.
    pub connected_at: String,
    /// Client IP address.
    pub remote_addr: String,
    /// Whether the client is authenticated.
    pub is_authenticated: bool,
    /// Whether the client is subscribed to metrics.
    pub subscribed_to_metrics: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CompanionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 9876);
        assert!(config.require_pin);
        assert_eq!(config.session_timeout_secs, 3600);
        assert_eq!(config.max_connections, 5);
    }

    #[test]
    fn test_request_serialization() {
        let req = CompanionRequest::Auth {
            pin: "123456".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"type\":\"auth\""));
        assert!(json.contains("\"pin\":\"123456\""));
    }

    #[test]
    fn test_response_serialization() {
        let resp = CompanionResponse::AuthOk {
            session_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"auth_ok\""));
    }

    #[test]
    fn test_event_serialization() {
        let event = CompanionEvent::Metrics {
            power_watts: Some(200),
            heart_rate_bpm: Some(140),
            cadence_rpm: Some(90),
            speed_kmh: Some(32.5),
            distance_km: 15.2,
            elapsed_secs: 3600,
            calories: 450,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"metrics\""));
        assert!(json.contains("\"power_watts\":200"));
    }
}
