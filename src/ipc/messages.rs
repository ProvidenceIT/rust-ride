//! IPC message types.
//!
//! Defines the request and response structures for daemon-CLI communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// IPC request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    /// UUID v4 for request correlation
    pub id: String,
    /// Command name
    pub command: String,
    /// Command-specific parameters
    #[serde(default)]
    pub params: Value,
}

impl IpcRequest {
    /// Create a new request with a generated ID
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            command: command.into(),
            params: Value::Object(serde_json::Map::new()),
        }
    }

    /// Create a new request with parameters
    pub fn with_params(command: impl Into<String>, params: Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            command: command.into(),
            params,
        }
    }
}

/// IPC response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    /// Request ID for correlation
    pub id: String,
    /// Whether the command succeeded
    pub success: bool,
    /// Result data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error information (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcResponse {
    /// Create a successful response
    pub fn success(id: String, result: Value) -> Self {
        Self {
            id,
            success: true,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: String, error: IpcError) -> Self {
        Self {
            id,
            success: false,
            result: None,
            error: Some(error),
        }
    }
}

/// IPC error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    /// Error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
}

/// Error codes per IPC protocol contract
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// No active ride/workout session
    NoSession,
    /// Cannot start, session already in progress
    SessionActive,
    /// Sensor ID not found
    SensorNotFound,
    /// Failed to connect to sensor
    SensorConnectionFailed,
    /// Workout file not found
    WorkoutNotFound,
    /// Failed to parse workout file
    WorkoutParseError,
    /// Failed to export ride
    ExportFailed,
    /// Invalid export format specified
    InvalidFormat,
    /// Ride ID not found
    RideNotFound,
    /// BLE adapter not available
    NoBleAdapter,
    /// Operation not permitted
    PermissionDenied,
    /// Unexpected internal error
    InternalError,
}

impl ErrorCode {
    /// Convert error code to CLI exit code
    pub fn to_exit_code(&self) -> i32 {
        match self {
            ErrorCode::NoSession => 5,
            ErrorCode::SessionActive => 5,
            ErrorCode::SensorNotFound => 6,
            ErrorCode::SensorConnectionFailed => 4,
            ErrorCode::WorkoutNotFound => 6,
            ErrorCode::WorkoutParseError => 1,
            ErrorCode::ExportFailed => 1,
            ErrorCode::InvalidFormat => 1,
            ErrorCode::RideNotFound => 6,
            ErrorCode::NoBleAdapter => 1,
            ErrorCode::PermissionDenied => 1,
            ErrorCode::InternalError => 1,
        }
    }
}
