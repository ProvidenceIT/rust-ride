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

    // ========== CompanionConfig Tests ==========

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
    fn test_config_serialization_roundtrip() {
        let config = CompanionConfig {
            enabled: true,
            port: 8080,
            require_pin: false,
            session_timeout_secs: 7200,
            max_connections: 10,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: CompanionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, true);
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.require_pin, false);
        assert_eq!(parsed.session_timeout_secs, 7200);
        assert_eq!(parsed.max_connections, 10);
    }

    // ========== CompanionRequest Parsing Tests ==========

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
    fn test_request_deserialization_auth() {
        let json = r#"{"type":"auth","pin":"654321"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        match req {
            CompanionRequest::Auth { pin } => assert_eq!(pin, "654321"),
            _ => panic!("Expected Auth request"),
        }
    }

    #[test]
    fn test_request_deserialization_get_session_status() {
        let json = r#"{"type":"get_session_status"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::GetSessionStatus));
    }

    #[test]
    fn test_request_deserialization_subscribe_metrics() {
        let json = r#"{"type":"subscribe_metrics"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::SubscribeMetrics));
    }

    #[test]
    fn test_request_deserialization_unsubscribe_metrics() {
        let json = r#"{"type":"unsubscribe_metrics"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::UnsubscribeMetrics));
    }

    #[test]
    fn test_request_deserialization_workout_pause() {
        let json = r#"{"type":"workout_pause"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::WorkoutPause));
    }

    #[test]
    fn test_request_deserialization_workout_resume() {
        let json = r#"{"type":"workout_resume"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::WorkoutResume));
    }

    #[test]
    fn test_request_deserialization_workout_skip() {
        let json = r#"{"type":"workout_skip"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::WorkoutSkip));
    }

    #[test]
    fn test_request_deserialization_workout_stop() {
        let json = r#"{"type":"workout_stop"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::WorkoutStop));
    }

    #[test]
    fn test_request_deserialization_adjust_resistance() {
        let json = r#"{"type":"adjust_resistance","delta":5}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        match req {
            CompanionRequest::AdjustResistance { delta } => assert_eq!(delta, 5),
            _ => panic!("Expected AdjustResistance request"),
        }
    }

    #[test]
    fn test_request_deserialization_adjust_resistance_negative() {
        let json = r#"{"type":"adjust_resistance","delta":-10}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        match req {
            CompanionRequest::AdjustResistance { delta } => assert_eq!(delta, -10),
            _ => panic!("Expected AdjustResistance request"),
        }
    }

    #[test]
    fn test_request_deserialization_get_ride_history() {
        let json = r#"{"type":"get_ride_history","limit":20,"offset":10}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        match req {
            CompanionRequest::GetRideHistory { limit, offset } => {
                assert_eq!(limit, 20);
                assert_eq!(offset, 10);
            }
            _ => panic!("Expected GetRideHistory request"),
        }
    }

    #[test]
    fn test_request_deserialization_get_ride_details() {
        let json = r#"{"type":"get_ride_details","ride_id":"abc-123"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        match req {
            CompanionRequest::GetRideDetails { ride_id } => assert_eq!(ride_id, "abc-123"),
            _ => panic!("Expected GetRideDetails request"),
        }
    }

    #[test]
    fn test_request_deserialization_ping() {
        let json = r#"{"type":"ping"}"#;
        let req: CompanionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req, CompanionRequest::Ping));
    }

    #[test]
    fn test_request_deserialization_invalid_type() {
        let json = r#"{"type":"invalid_command"}"#;
        let result: Result<CompanionRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_deserialization_missing_type() {
        let json = r#"{"pin":"123456"}"#;
        let result: Result<CompanionRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_deserialization_malformed_json() {
        let json = r#"{type: auth}"#;
        let result: Result<CompanionRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ========== CompanionResponse Parsing Tests ==========

    #[test]
    fn test_response_serialization() {
        let resp = CompanionResponse::AuthOk {
            session_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"auth_ok\""));
    }

    #[test]
    fn test_response_auth_failed_serialization() {
        let resp = CompanionResponse::AuthFailed {
            reason: "Invalid PIN".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"auth_failed\""));
        assert!(json.contains("Invalid PIN"));
    }

    #[test]
    fn test_response_session_status_serialization() {
        let resp = CompanionResponse::SessionStatus {
            active: true,
            session: Some(SessionStatusInfo {
                session_id: Uuid::nil(),
                session_type: "workout".to_string(),
                workout_name: Some("Test Workout".to_string()),
                workout_path: None,
                is_paused: false,
                elapsed_secs: 1800,
                current_interval_index: Some(3),
                total_intervals: Some(10),
                current_interval_name: Some("Threshold".to_string()),
                target_power_watts: Some(250),
                interval_remaining_secs: Some(120),
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"session_status\""));
        assert!(json.contains("\"active\":true"));
        assert!(json.contains("Test Workout"));
    }

    #[test]
    fn test_response_session_status_inactive() {
        let resp = CompanionResponse::SessionStatus {
            active: false,
            session: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"active\":false"));
    }

    #[test]
    fn test_response_command_ok_serialization() {
        let resp = CompanionResponse::CommandOk {
            command: "workout_pause".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"command_ok\""));
        assert!(json.contains("workout_pause"));
    }

    #[test]
    fn test_response_command_failed_serialization() {
        let resp = CompanionResponse::CommandFailed {
            command: "workout_skip".to_string(),
            error: "Already at last interval".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"command_failed\""));
        assert!(json.contains("workout_skip"));
        assert!(json.contains("Already at last interval"));
    }

    #[test]
    fn test_response_ride_history_serialization() {
        let resp = CompanionResponse::RideHistory {
            rides: vec![RideSummary {
                ride_id: "ride-1".to_string(),
                started_at: "2024-01-15T10:00:00Z".to_string(),
                duration_secs: 3600,
                distance_km: 25.5,
                avg_power_watts: Some(200),
                is_workout: true,
                workout_name: Some("Sweet Spot".to_string()),
            }],
            total: 1,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"ride_history\""));
        assert!(json.contains("ride-1"));
        assert!(json.contains("Sweet Spot"));
    }

    #[test]
    fn test_response_pong_serialization() {
        let resp = CompanionResponse::Pong;
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"pong\""));
    }

    #[test]
    fn test_response_error_serialization() {
        let resp = CompanionResponse::Error {
            code: CompanionErrorCode::NoSession,
            message: "No active session".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("NO_SESSION"));
        assert!(json.contains("No active session"));
    }

    // ========== CompanionEvent Parsing Tests ==========

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

    #[test]
    fn test_event_metrics_with_null_values() {
        let event = CompanionEvent::Metrics {
            power_watts: None,
            heart_rate_bpm: None,
            cadence_rpm: None,
            speed_kmh: None,
            distance_km: 0.0,
            elapsed_secs: 0,
            calories: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"power_watts\":null"));
        assert!(json.contains("\"heart_rate_bpm\":null"));
    }

    #[test]
    fn test_event_session_state_changed() {
        let event = CompanionEvent::SessionStateChanged {
            state: SessionState::Active,
            session: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"session_state_changed\""));
        assert!(json.contains("\"state\":\"active\""));
    }

    #[test]
    fn test_event_interval_changed() {
        let event = CompanionEvent::IntervalChanged {
            interval_index: 2,
            total_intervals: 5,
            interval_name: "VO2max".to_string(),
            target_power_watts: 350,
            duration_secs: 180,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"interval_changed\""));
        assert!(json.contains("VO2max"));
        assert!(json.contains("350"));
    }

    #[test]
    fn test_event_disconnecting() {
        let event = CompanionEvent::Disconnecting {
            reason: "Server shutting down".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"disconnecting\""));
        assert!(json.contains("Server shutting down"));
    }

    // ========== SessionState Tests ==========

    #[test]
    fn test_session_state_serialization() {
        assert!(serde_json::to_string(&SessionState::Idle).unwrap().contains("idle"));
        assert!(serde_json::to_string(&SessionState::Starting).unwrap().contains("starting"));
        assert!(serde_json::to_string(&SessionState::Active).unwrap().contains("active"));
        assert!(serde_json::to_string(&SessionState::Paused).unwrap().contains("paused"));
        assert!(serde_json::to_string(&SessionState::Stopping).unwrap().contains("stopping"));
        assert!(serde_json::to_string(&SessionState::Completed).unwrap().contains("completed"));
    }

    // ========== CompanionErrorCode Tests ==========

    #[test]
    fn test_error_code_serialization() {
        assert!(serde_json::to_string(&CompanionErrorCode::AuthRequired).unwrap().contains("AUTH_REQUIRED"));
        assert!(serde_json::to_string(&CompanionErrorCode::InvalidPin).unwrap().contains("INVALID_PIN"));
        assert!(serde_json::to_string(&CompanionErrorCode::NoSession).unwrap().contains("NO_SESSION"));
        assert!(serde_json::to_string(&CompanionErrorCode::SessionActive).unwrap().contains("SESSION_ACTIVE"));
        assert!(serde_json::to_string(&CompanionErrorCode::UnknownCommand).unwrap().contains("UNKNOWN_COMMAND"));
        assert!(serde_json::to_string(&CompanionErrorCode::InvalidParams).unwrap().contains("INVALID_PARAMS"));
        assert!(serde_json::to_string(&CompanionErrorCode::RateLimited).unwrap().contains("RATE_LIMITED"));
        assert!(serde_json::to_string(&CompanionErrorCode::InternalError).unwrap().contains("INTERNAL_ERROR"));
    }

    // ========== CompanionClient Tests ==========

    #[test]
    fn test_companion_client_serialization() {
        let client = CompanionClient {
            session_id: Uuid::nil(),
            connected_at: "2024-01-15T10:00:00Z".to_string(),
            remote_addr: "192.168.1.100:54321".to_string(),
            is_authenticated: true,
            subscribed_to_metrics: true,
        };
        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("192.168.1.100:54321"));
        assert!(json.contains("\"is_authenticated\":true"));
        assert!(json.contains("\"subscribed_to_metrics\":true"));
    }

    // ========== RideSummary Tests ==========

    #[test]
    fn test_ride_summary_serialization() {
        let summary = RideSummary {
            ride_id: "ride-abc".to_string(),
            started_at: "2024-01-15T08:00:00Z".to_string(),
            duration_secs: 5400,
            distance_km: 40.0,
            avg_power_watts: Some(185),
            is_workout: false,
            workout_name: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("ride-abc"));
        assert!(json.contains("5400"));
        assert!(json.contains("\"is_workout\":false"));
    }

    // ========== RideDetailInfo Tests ==========

    #[test]
    fn test_ride_detail_info_serialization() {
        let detail = RideDetailInfo {
            ride_id: "ride-xyz".to_string(),
            started_at: "2024-01-15T08:00:00Z".to_string(),
            ended_at: "2024-01-15T09:30:00Z".to_string(),
            duration_secs: 5400,
            distance_km: 40.0,
            calories: 750,
            avg_power_watts: Some(185),
            max_power_watts: Some(450),
            normalized_power_watts: Some(195),
            avg_heart_rate_bpm: Some(145),
            max_heart_rate_bpm: Some(175),
            avg_cadence_rpm: Some(88),
            tss: Some(85.0),
            intensity_factor: Some(0.92),
            is_workout: true,
            workout_name: Some("Threshold Builder".to_string()),
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("ride-xyz"));
        assert!(json.contains("450")); // max power
        assert!(json.contains("Threshold Builder"));
        assert!(json.contains("0.92")); // intensity factor
    }

    #[test]
    fn test_ride_detail_info_with_nulls() {
        let detail = RideDetailInfo {
            ride_id: "ride-free".to_string(),
            started_at: "2024-01-15T08:00:00Z".to_string(),
            ended_at: "2024-01-15T08:30:00Z".to_string(),
            duration_secs: 1800,
            distance_km: 15.0,
            calories: 300,
            avg_power_watts: None,
            max_power_watts: None,
            normalized_power_watts: None,
            avg_heart_rate_bpm: None,
            max_heart_rate_bpm: None,
            avg_cadence_rpm: None,
            tss: None,
            intensity_factor: None,
            is_workout: false,
            workout_name: None,
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("ride-free"));
        assert!(json.contains("\"avg_power_watts\":null"));
        assert!(json.contains("\"is_workout\":false"));
    }

    // ========== CompanionError Tests ==========

    #[test]
    fn test_companion_error_display() {
        let err = CompanionError::BindFailed(9876, "Address already in use".to_string());
        assert!(err.to_string().contains("9876"));
        assert!(err.to_string().contains("Address already in use"));

        let err = CompanionError::AuthenticationFailed("Invalid credentials".to_string());
        assert!(err.to_string().contains("Invalid credentials"));

        let err = CompanionError::SessionNotFound(Uuid::nil());
        assert!(err.to_string().contains("Session not found"));

        let err = CompanionError::NoActiveSession;
        assert!(err.to_string().contains("No active session"));

        let err = CompanionError::WebSocketError("Connection reset".to_string());
        assert!(err.to_string().contains("Connection reset"));

        let err = CompanionError::InvalidMessage("Missing type field".to_string());
        assert!(err.to_string().contains("Missing type field"));

        let err = CompanionError::ServerNotRunning;
        assert!(err.to_string().contains("not running"));

        let err = CompanionError::MaxConnectionsReached;
        assert!(err.to_string().contains("Maximum connections"));

        let err = CompanionError::InternalError("Unexpected error".to_string());
        assert!(err.to_string().contains("Unexpected error"));
    }
}
