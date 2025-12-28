//! CLI module for headless daemon control.
//!
//! This module provides the command-line interface for controlling the
//! RustRide daemon when running in headless mode.

pub mod client;
pub mod commands;

/// Exit codes for CLI commands (per IPC protocol contract)
pub mod exit_codes {
    /// Command succeeded
    pub const SUCCESS: i32 = 0;
    /// General error
    pub const GENERAL_ERROR: i32 = 1;
    /// Invalid arguments
    pub const INVALID_ARGS: i32 = 2;
    /// Daemon not running
    pub const DAEMON_NOT_RUNNING: i32 = 3;
    /// Connection to daemon failed
    pub const CONNECTION_FAILED: i32 = 4;
    /// Command rejected (e.g., session already active)
    pub const COMMAND_REJECTED: i32 = 5;
    /// Resource not found
    pub const RESOURCE_NOT_FOUND: i32 = 6;
    /// Operation timed out
    pub const OPERATION_TIMEOUT: i32 = 7;
}

/// Global JSON output mode flag
static JSON_OUTPUT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set JSON output mode
pub fn set_json_output(enabled: bool) {
    JSON_OUTPUT.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Check if JSON output mode is enabled
pub fn is_json_output() -> bool {
    JSON_OUTPUT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Format output as JSON or human-readable based on current mode
pub fn format_output<T: serde::Serialize>(data: &T) -> String {
    if is_json_output() {
        serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string())
    } else {
        // For human-readable, use Debug formatting as fallback
        format!("{:?}", serde_json::to_value(data).unwrap_or_default())
    }
}
