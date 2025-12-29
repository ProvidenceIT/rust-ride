//! Command handler for daemon IPC commands.
//!
//! Routes incoming IPC requests to appropriate handlers and returns responses.
//!
//! Phase 4 (T036-T048): Ride and workout command handlers
//!
//! ## Integration Status
//!
//! The IPC command handlers are fully implemented for session management:
//! - Session creation, tracking, and cleanup
//! - State machine transitions (start/pause/resume/stop)
//! - JSON responses for CLI consumption
//!
//! ### T047/T048: Deep Engine/Recorder Integration (Future Work)
//!
//! Full integration with WorkoutEngine and RideRecorder requires:
//! 1. A `DaemonContext` struct to hold non-serializable resources
//! 2. Background tick loop for workout time progression
//! 3. Sensor data routing to update LiveMetrics
//! 4. Database persistence for ride samples
//!
//! Current implementation provides the complete IPC interface; deep integration
//! would connect the session state to actual workout execution and recording.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ipc::messages::{ErrorCode, IpcError, IpcRequest, IpcResponse};

use super::state::{DaemonState, DaemonStatus, LiveMetrics, SessionInfo, SessionType, WorkoutInfo};
use chrono::Utc;
use uuid::Uuid;

/// Handle an incoming IPC request
pub async fn handle_request(request: IpcRequest, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    debug!("Handling command: {}", request.command);

    match request.command.as_str() {
        // Daemon commands
        "daemon.status" | "DaemonStatus" => handle_daemon_status(request.id, state).await,
        "daemon.shutdown" | "DaemonShutdown" => handle_daemon_shutdown(request.id, state).await,

        // Ride commands (T036-T037)
        "ride.start" | "RideStart" => handle_ride_start(request.id, request.params, state).await,
        "ride.stop" | "RideStop" => handle_ride_stop(request.id, state).await,

        // Workout commands (T038-T042)
        "workout.start" | "WorkoutStart" => {
            handle_workout_start(request.id, request.params, state).await
        }
        "workout.pause" | "WorkoutPause" => handle_workout_pause(request.id, state).await,
        "workout.resume" | "WorkoutResume" => handle_workout_resume(request.id, state).await,
        "workout.skip" | "WorkoutSkip" => handle_workout_skip(request.id, state).await,
        "workout.stop" | "WorkoutStop" => handle_workout_stop(request.id, state).await,

        // Status commands (T043)
        "status.live" | "StatusLive" => handle_status_live(request.id, state).await,

        // Ride export commands (T051)
        "ride.export" | "RideExport" => handle_ride_export(request.id, request.params, state).await,
        "rides.list" | "RidesList" => handle_rides_list(request.id, request.params, state).await,

        // Sensor commands (T057-T059)
        "sensors.list" | "SensorsList" => handle_sensors_list(request.id, state).await,
        "sensor.connect" | "SensorConnect" => {
            handle_sensor_connect(request.id, request.params, state).await
        }
        "sensor.disconnect" | "SensorDisconnect" => {
            handle_sensor_disconnect(request.id, request.params, state).await
        }

        // Ride recovery commands (T071)
        "ride.recover" | "RideRecover" => {
            handle_ride_recover(request.id, request.params, state).await
        }
        "rides.incomplete" | "RidesIncomplete" => handle_rides_incomplete(request.id, state).await,

        _ => {
            warn!("Unknown command: {}", request.command);
            IpcResponse::error(
                request.id,
                IpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Unknown command: {}", request.command),
                },
            )
        }
    }
}

/// Handle DaemonStatus command
async fn handle_daemon_status(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let state = state.read().await;

    let result = serde_json::json!({
        "pid": state.pid,
        "status": state.status,
        "uptime_seconds": state.uptime_seconds(),
        "started_at": state.started_at,
        "ble_adapter_available": state.ble_adapter_available,
        "active_session": state.active_session,
        "connected_sensors": state.connected_sensors,
        "version": env!("CARGO_PKG_VERSION"),
    });

    IpcResponse::success(id, result)
}

/// Handle DaemonShutdown command
async fn handle_daemon_shutdown(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    {
        let mut state = state.write().await;
        state.initiate_shutdown();
    }

    IpcResponse::success(
        id,
        serde_json::json!({
            "message": "Shutdown initiated"
        }),
    )
}

// =============================================================================
// T036: RideStart command handler
// =============================================================================

/// Handle RideStart command - starts a free ride session
async fn handle_ride_start(
    id: String,
    _params: serde_json::Value,
    state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let mut state = state.write().await;

    // Check if session already active
    if state.active_session.is_some() {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::SessionActive,
                message: "A session is already active. Stop it first.".to_string(),
            },
        );
    }

    // Check daemon status
    if state.status != DaemonStatus::Running && state.status != DaemonStatus::Degraded {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::InternalError,
                message: format!("Daemon is not ready: {:?}", state.status),
            },
        );
    }

    // Create new free ride session
    let session_id = Uuid::new_v4();
    let session = SessionInfo {
        session_id,
        session_type: SessionType::FreeRide,
        started_at: Utc::now(),
        workout_info: None,
        current_metrics: LiveMetrics::default(),
        is_paused: false,
    };

    state.active_session = Some(session.clone());
    info!("Started free ride session: {}", session_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session_id.to_string(),
            "session_type": "free_ride",
            "started_at": session.started_at,
        }),
    )
}

// =============================================================================
// T037: RideStop command handler
// =============================================================================

/// Handle RideStop command - stops the current ride session
async fn handle_ride_stop(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let mut state = state.write().await;

    // Check if session exists
    let session = match state.active_session.take() {
        Some(s) => s,
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::NoSession,
                    message: "No active session to stop".to_string(),
                },
            );
        }
    };

    let elapsed = session.elapsed_seconds();
    info!(
        "Stopped ride session: {} ({}s)",
        session.session_id, elapsed
    );

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session.session_id.to_string(),
            "elapsed_seconds": elapsed,
            "metrics": session.current_metrics,
        }),
    )
}

// =============================================================================
// T038: WorkoutStart command handler
// =============================================================================

/// Handle WorkoutStart command - starts a structured workout
async fn handle_workout_start(
    id: String,
    params: serde_json::Value,
    state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let mut state = state.write().await;

    // Check if session already active
    if state.active_session.is_some() {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::SessionActive,
                message: "A session is already active. Stop it first.".to_string(),
            },
        );
    }

    // Extract workout path from params
    let workout_path = match params.get("path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::WorkoutNotFound,
                    message: "Missing 'path' parameter for workout".to_string(),
                },
            );
        }
    };

    // Check workout file exists
    if !workout_path.exists() {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::WorkoutNotFound,
                message: format!("Workout file not found: {}", workout_path.display()),
            },
        );
    }

    // Create workout session
    let session_id = Uuid::new_v4();
    let workout_name = workout_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let workout_info = WorkoutInfo {
        name: workout_name.clone(),
        file_path: workout_path.clone(),
        total_duration_seconds: 0, // Will be updated when workout is loaded
        current_interval_index: 0,
        total_intervals: 0,
        current_interval_name: "Starting".to_string(),
        interval_elapsed_seconds: 0,
        interval_remaining_seconds: 0,
        target_power_watts: 0,
        target_power_percent_ftp: 0.0,
    };

    let session = SessionInfo {
        session_id,
        session_type: SessionType::Workout {
            path: workout_path.clone(),
        },
        started_at: Utc::now(),
        workout_info: Some(workout_info),
        current_metrics: LiveMetrics::default(),
        is_paused: false,
    };

    state.active_session = Some(session.clone());
    info!("Started workout session: {} ({})", session_id, workout_name);

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session_id.to_string(),
            "session_type": "workout",
            "workout_name": workout_name,
            "workout_path": workout_path.display().to_string(),
            "started_at": session.started_at,
        }),
    )
}

// =============================================================================
// T039: WorkoutPause command handler
// =============================================================================

/// Handle WorkoutPause command - pauses the current workout
async fn handle_workout_pause(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let mut state = state.write().await;

    // Check if session exists and is a workout
    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::NoSession,
                    message: "No active session".to_string(),
                },
            );
        }
    };

    if session.workout_info.is_none() {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::NoSession,
                message: "Active session is not a workout".to_string(),
            },
        );
    }

    if session.is_paused {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::InternalError,
                message: "Workout is already paused".to_string(),
            },
        );
    }

    session.is_paused = true;
    info!("Paused workout session: {}", session.session_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session.session_id.to_string(),
            "is_paused": true,
        }),
    )
}

// =============================================================================
// T040: WorkoutResume command handler
// =============================================================================

/// Handle WorkoutResume command - resumes a paused workout
async fn handle_workout_resume(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let mut state = state.write().await;

    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::NoSession,
                    message: "No active session".to_string(),
                },
            );
        }
    };

    if session.workout_info.is_none() {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::NoSession,
                message: "Active session is not a workout".to_string(),
            },
        );
    }

    if !session.is_paused {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::InternalError,
                message: "Workout is not paused".to_string(),
            },
        );
    }

    session.is_paused = false;
    info!("Resumed workout session: {}", session.session_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session.session_id.to_string(),
            "is_paused": false,
        }),
    )
}

// =============================================================================
// T041: WorkoutSkip command handler
// =============================================================================

/// Handle WorkoutSkip command - skips to the next interval
async fn handle_workout_skip(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let mut state = state.write().await;

    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::NoSession,
                    message: "No active session".to_string(),
                },
            );
        }
    };

    let workout_info = match &mut session.workout_info {
        Some(w) => w,
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::NoSession,
                    message: "Active session is not a workout".to_string(),
                },
            );
        }
    };

    // Check if there's a next interval
    if workout_info.current_interval_index + 1 >= workout_info.total_intervals {
        return IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::InternalError,
                message: "Already at last interval".to_string(),
            },
        );
    }

    workout_info.current_interval_index += 1;
    workout_info.interval_elapsed_seconds = 0;
    info!(
        "Skipped to interval {} in session {}",
        workout_info.current_interval_index, session.session_id
    );

    IpcResponse::success(
        id,
        serde_json::json!({
            "session_id": session.session_id.to_string(),
            "current_interval_index": workout_info.current_interval_index,
            "total_intervals": workout_info.total_intervals,
        }),
    )
}

// =============================================================================
// T042: WorkoutStop command handler
// =============================================================================

/// Handle WorkoutStop command - stops the workout session
async fn handle_workout_stop(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    // Reuse RideStop logic - both end the session
    handle_ride_stop(id, state).await
}

// =============================================================================
// T043: StatusLive command handler
// =============================================================================

/// Handle StatusLive command - returns current live metrics
async fn handle_status_live(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let state = state.read().await;

    match &state.active_session {
        Some(session) => {
            let result = serde_json::json!({
                "session_id": session.session_id.to_string(),
                "session_type": match &session.session_type {
                    SessionType::FreeRide => "free_ride",
                    SessionType::Workout { .. } => "workout",
                },
                "elapsed_seconds": session.elapsed_seconds(),
                "is_paused": session.is_paused,
                "metrics": session.current_metrics,
                "workout_info": session.workout_info,
            });

            IpcResponse::success(id, result)
        }
        None => IpcResponse::error(
            id,
            IpcError {
                code: ErrorCode::NoSession,
                message: "No active session".to_string(),
            },
        ),
    }
}

// =============================================================================
// T051: RideExport command handler
// =============================================================================

/// Handle RideExport command - exports a ride to file
async fn handle_ride_export(
    id: String,
    params: serde_json::Value,
    _state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let ride_id = match params.get("ride_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::RideNotFound,
                    message: "Missing 'ride_id' parameter".to_string(),
                },
            );
        }
    };

    let format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("fit")
        .to_lowercase();

    let output_path = params.get("output_path").and_then(|v| v.as_str());

    // Generate output path if not provided
    let export_path = if let Some(path) = output_path {
        PathBuf::from(path)
    } else {
        // Default to user's home directory
        let home = directories::UserDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let filename = format!("ride_{}.{}", ride_id, format);
        home.join("RustRide").join("exports").join(filename)
    };

    // Create export directory if needed
    if let Some(parent) = export_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::ExportFailed,
                    message: format!("Failed to create export directory: {}", e),
                },
            );
        }
    }

    // TODO: Actually export the ride from database
    // For now, return success with the path that would be used
    info!(
        "Export requested: ride {} to {} as {}",
        ride_id,
        export_path.display(),
        format
    );

    IpcResponse::success(
        id,
        serde_json::json!({
            "ride_id": ride_id,
            "format": format,
            "path": export_path.display().to_string(),
            "message": "Export handler ready (integration pending)"
        }),
    )
}

/// Handle RidesList command - lists recorded rides
async fn handle_rides_list(
    id: String,
    params: serde_json::Value,
    _state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    // TODO: Actually query rides from database
    // For now, return empty list
    info!("Rides list requested (limit: {})", limit);

    IpcResponse::success(
        id,
        serde_json::json!({
            "rides": [],
            "total": 0,
            "message": "Rides list handler ready (integration pending)"
        }),
    )
}

// =============================================================================
// T057: SensorsList command handler
// =============================================================================

/// Handle SensorsList command - lists discovered and connected sensors
async fn handle_sensors_list(id: String, state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    let state = state.read().await;

    IpcResponse::success(
        id,
        serde_json::json!({
            "connected": state.connected_sensors,
            "discovered": [], // Would be populated from SensorManager
            "ble_adapter_available": state.ble_adapter_available,
        }),
    )
}

// =============================================================================
// T058: SensorConnect command handler
// =============================================================================

/// Handle SensorConnect command - connects to a sensor
async fn handle_sensor_connect(
    id: String,
    params: serde_json::Value,
    _state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let sensor_id = match params.get("sensor_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::SensorNotFound,
                    message: "Missing 'sensor_id' parameter".to_string(),
                },
            );
        }
    };

    // TODO: Actually connect via SensorManager
    info!("Sensor connect requested: {}", sensor_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "sensor_id": sensor_id,
            "status": "connecting",
            "message": "Sensor connection handler ready (integration pending)"
        }),
    )
}

// =============================================================================
// T059: SensorDisconnect command handler
// =============================================================================

/// Handle SensorDisconnect command - disconnects from a sensor
async fn handle_sensor_disconnect(
    id: String,
    params: serde_json::Value,
    _state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let sensor_id = match params.get("sensor_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::SensorNotFound,
                    message: "Missing 'sensor_id' parameter".to_string(),
                },
            );
        }
    };

    // TODO: Actually disconnect via SensorManager
    info!("Sensor disconnect requested: {}", sensor_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "sensor_id": sensor_id,
            "status": "disconnected"
        }),
    )
}

// =============================================================================
// T071: RideRecover command handler
// =============================================================================

/// Handle RideRecover command - recovers an incomplete ride
async fn handle_ride_recover(
    id: String,
    params: serde_json::Value,
    _state: Arc<RwLock<DaemonState>>,
) -> IpcResponse {
    let ride_id = match params.get("ride_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return IpcResponse::error(
                id,
                IpcError {
                    code: ErrorCode::RideNotFound,
                    message: "Missing 'ride_id' parameter".to_string(),
                },
            );
        }
    };

    // TODO: Actually recover the ride from database
    info!("Ride recovery requested: {}", ride_id);

    IpcResponse::success(
        id,
        serde_json::json!({
            "ride_id": ride_id,
            "status": "recovered",
            "message": "Ride recovery handler ready (integration pending)"
        }),
    )
}

/// Handle RidesIncomplete command - lists incomplete/recoverable rides
async fn handle_rides_incomplete(id: String, _state: Arc<RwLock<DaemonState>>) -> IpcResponse {
    // TODO: Query database for incomplete rides
    info!("Incomplete rides list requested");

    IpcResponse::success(
        id,
        serde_json::json!({
            "incomplete_rides": [],
            "total": 0,
            "message": "Incomplete rides handler ready (integration pending)"
        }),
    )
}
