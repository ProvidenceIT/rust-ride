//! Request handlers for companion server commands.
//!
//! Routes incoming WebSocket requests to appropriate handlers and
//! returns responses. Integrates with the daemon state and workout
//! engine to execute commands.
//!
//! ## Handler Categories
//!
//! - **Authentication**: PIN verification and session management
//! - **Session Status**: Current workout/ride state queries
//! - **Workout Control**: Pause, resume, skip, stop commands
//! - **Metrics**: Real-time metrics subscription management
//! - **Ride History**: Past ride queries and statistics (T007)

use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::types::{
    CompanionErrorCode, CompanionEvent, CompanionRequest, CompanionResponse, RideDetailInfo,
    RideSummary, SessionState, SessionStatusInfo,
};
use crate::daemon::state::{DaemonState, SessionType};
use crate::recording::types::Ride;
use crate::storage::Database;

/// Handle an incoming companion request.
///
/// This function routes the request to the appropriate handler based
/// on the request type and returns a response.
///
/// # Arguments
///
/// * `request` - The companion request to handle
/// * `session_id` - The session ID of the requesting client
/// * `is_authenticated` - Whether the client is authenticated
/// * `daemon_state` - Optional daemon state for workout control commands
/// * `database` - Optional database for ride history queries (T007)
/// * `user_id` - Optional user ID for ride history queries (T007)
pub async fn handle_request(
    request: CompanionRequest,
    session_id: Uuid,
    is_authenticated: bool,
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
    database: Option<Arc<Mutex<Database>>>,
    user_id: Option<Uuid>,
) -> CompanionResponse {
    debug!("Handling companion request from session {}", session_id);

    // Most commands require authentication
    if !is_authenticated {
        match &request {
            CompanionRequest::Auth { .. } | CompanionRequest::Ping => {
                // These don't require auth
            }
            _ => {
                return CompanionResponse::Error {
                    code: CompanionErrorCode::AuthRequired,
                    message: "Authentication required".to_string(),
                };
            }
        }
    }

    match request {
        CompanionRequest::Auth { pin } => handle_auth(pin).await,
        CompanionRequest::GetSessionStatus => handle_get_session_status(daemon_state).await,
        CompanionRequest::SubscribeMetrics => handle_subscribe_metrics(session_id).await,
        CompanionRequest::UnsubscribeMetrics => handle_unsubscribe_metrics(session_id).await,
        CompanionRequest::WorkoutPause => handle_workout_pause(daemon_state).await,
        CompanionRequest::WorkoutResume => handle_workout_resume(daemon_state).await,
        CompanionRequest::WorkoutSkip => handle_workout_skip(daemon_state).await,
        CompanionRequest::WorkoutStop => handle_workout_stop(daemon_state).await,
        CompanionRequest::AdjustResistance { delta } => handle_adjust_resistance(delta).await,
        CompanionRequest::GetRideHistory { limit, offset } => {
            handle_get_ride_history(database, user_id, limit, offset).await
        }
        CompanionRequest::GetRideDetails { ride_id } => {
            handle_get_ride_details(database, ride_id).await
        }
        CompanionRequest::Ping => CompanionResponse::Pong,
    }
}

/// Handle authentication request.
async fn handle_auth(pin: String) -> CompanionResponse {
    // TODO: T003 - Validate PIN against server's current PIN
    debug!("Auth request with PIN: {}", pin);

    // Placeholder: Accept any 6-digit PIN for now
    if pin.len() == 6 && pin.chars().all(|c| c.is_ascii_digit()) {
        CompanionResponse::AuthOk {
            session_id: Uuid::new_v4(),
        }
    } else {
        CompanionResponse::AuthFailed {
            reason: "Invalid PIN format".to_string(),
        }
    }
}

/// Handle get session status request.
///
/// Returns the current session status from the daemon state if available.
async fn handle_get_session_status(
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
) -> CompanionResponse {
    debug!("Session status request");

    // If no daemon state available, return no active session
    let state = match daemon_state {
        Some(s) => s,
        None => {
            return CompanionResponse::SessionStatus {
                active: false,
                session: None,
            };
        }
    };

    let state = state.read().await;

    match &state.active_session {
        Some(session) => {
            let session_type = match &session.session_type {
                SessionType::FreeRide => "free_ride".to_string(),
                SessionType::Workout { .. } => "workout".to_string(),
            };

            let session_info = SessionStatusInfo {
                session_id: session.session_id,
                session_type,
                workout_name: session.workout_info.as_ref().map(|w| w.name.clone()),
                workout_path: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.file_path.clone()),
                is_paused: session.is_paused,
                elapsed_secs: session.elapsed_seconds() as u32,
                current_interval_index: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.current_interval_index),
                total_intervals: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.total_intervals),
                current_interval_name: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.current_interval_name.clone()),
                target_power_watts: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.target_power_watts),
                interval_remaining_secs: session
                    .workout_info
                    .as_ref()
                    .map(|w| w.interval_remaining_seconds as u32),
            };

            CompanionResponse::SessionStatus {
                active: true,
                session: Some(session_info),
            }
        }
        None => CompanionResponse::SessionStatus {
            active: false,
            session: None,
        },
    }
}

/// Handle metrics subscription request.
async fn handle_subscribe_metrics(session_id: Uuid) -> CompanionResponse {
    // TODO: T005 - Add client to metrics broadcast list
    debug!("Subscribe metrics request from session {}", session_id);

    CompanionResponse::SubscribedMetrics
}

/// Handle metrics unsubscription request.
async fn handle_unsubscribe_metrics(session_id: Uuid) -> CompanionResponse {
    // TODO: T005 - Remove client from metrics broadcast list
    debug!("Unsubscribe metrics request from session {}", session_id);

    CompanionResponse::UnsubscribedMetrics
}

/// Handle workout pause request.
///
/// Pauses the active workout session. Requires an active workout (not free ride).
async fn handle_workout_pause(
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
) -> CompanionResponse {
    debug!("Workout pause request");

    // Get daemon state or return error if not available
    let state = match daemon_state {
        Some(s) => s,
        None => {
            return CompanionResponse::CommandFailed {
                command: "workout_pause".to_string(),
                error: "Daemon state not available".to_string(),
            };
        }
    };

    let mut state = state.write().await;

    // Check if session exists
    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::NoSession,
                message: "No active session".to_string(),
            };
        }
    };

    // Check if it's a workout session
    if session.workout_info.is_none() {
        return CompanionResponse::CommandFailed {
            command: "workout_pause".to_string(),
            error: "Active session is not a workout".to_string(),
        };
    }

    // Check if already paused
    if session.is_paused {
        return CompanionResponse::CommandFailed {
            command: "workout_pause".to_string(),
            error: "Workout is already paused".to_string(),
        };
    }

    session.is_paused = true;
    info!("Paused workout session: {} (via companion)", session.session_id);

    CompanionResponse::CommandOk {
        command: "workout_pause".to_string(),
    }
}

/// Handle workout resume request.
///
/// Resumes a paused workout session.
async fn handle_workout_resume(
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
) -> CompanionResponse {
    debug!("Workout resume request");

    // Get daemon state or return error if not available
    let state = match daemon_state {
        Some(s) => s,
        None => {
            return CompanionResponse::CommandFailed {
                command: "workout_resume".to_string(),
                error: "Daemon state not available".to_string(),
            };
        }
    };

    let mut state = state.write().await;

    // Check if session exists
    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::NoSession,
                message: "No active session".to_string(),
            };
        }
    };

    // Check if it's a workout session
    if session.workout_info.is_none() {
        return CompanionResponse::CommandFailed {
            command: "workout_resume".to_string(),
            error: "Active session is not a workout".to_string(),
        };
    }

    // Check if workout is paused
    if !session.is_paused {
        return CompanionResponse::CommandFailed {
            command: "workout_resume".to_string(),
            error: "Workout is not paused".to_string(),
        };
    }

    session.is_paused = false;
    info!("Resumed workout session: {} (via companion)", session.session_id);

    CompanionResponse::CommandOk {
        command: "workout_resume".to_string(),
    }
}

/// Handle workout skip request.
///
/// Skips to the next interval in the workout.
async fn handle_workout_skip(
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
) -> CompanionResponse {
    debug!("Workout skip request");

    // Get daemon state or return error if not available
    let state = match daemon_state {
        Some(s) => s,
        None => {
            return CompanionResponse::CommandFailed {
                command: "workout_skip".to_string(),
                error: "Daemon state not available".to_string(),
            };
        }
    };

    let mut state = state.write().await;

    // Check if session exists
    let session = match &mut state.active_session {
        Some(s) => s,
        None => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::NoSession,
                message: "No active session".to_string(),
            };
        }
    };

    // Check if it's a workout session and get workout info
    let workout_info = match &mut session.workout_info {
        Some(w) => w,
        None => {
            return CompanionResponse::CommandFailed {
                command: "workout_skip".to_string(),
                error: "Active session is not a workout".to_string(),
            };
        }
    };

    // Check if there's a next interval
    if workout_info.current_interval_index + 1 >= workout_info.total_intervals {
        return CompanionResponse::CommandFailed {
            command: "workout_skip".to_string(),
            error: "Already at last interval".to_string(),
        };
    }

    let previous_index = workout_info.current_interval_index;
    workout_info.current_interval_index += 1;
    workout_info.interval_elapsed_seconds = 0;

    info!(
        "Skipped to interval {} (from {}) in session {} (via companion)",
        workout_info.current_interval_index,
        previous_index,
        session.session_id
    );

    CompanionResponse::CommandOk {
        command: "workout_skip".to_string(),
    }
}

/// Handle workout stop request.
///
/// Stops the active session (workout or free ride).
async fn handle_workout_stop(
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
) -> CompanionResponse {
    debug!("Workout stop request");

    // Get daemon state or return error if not available
    let state = match daemon_state {
        Some(s) => s,
        None => {
            return CompanionResponse::CommandFailed {
                command: "workout_stop".to_string(),
                error: "Daemon state not available".to_string(),
            };
        }
    };

    let mut state = state.write().await;

    // Check if session exists and take it
    let session = match state.active_session.take() {
        Some(s) => s,
        None => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::NoSession,
                message: "No active session to stop".to_string(),
            };
        }
    };

    let elapsed = session.elapsed_seconds();
    let session_type = match &session.session_type {
        SessionType::FreeRide => "free_ride",
        SessionType::Workout { .. } => "workout",
    };

    info!(
        "Stopped {} session: {} ({}s) (via companion)",
        session_type,
        session.session_id,
        elapsed
    );

    CompanionResponse::CommandOk {
        command: "workout_stop".to_string(),
    }
}

/// Handle resistance adjustment request.
async fn handle_adjust_resistance(delta: i8) -> CompanionResponse {
    // TODO: T034 (Mobile App Phase) - Integrate with trainer control
    debug!("Adjust resistance request: delta={}", delta);

    CompanionResponse::CommandOk {
        command: "adjust_resistance".to_string(),
    }
}

/// Handle get ride history request.
///
/// T007: Query rides from the database and return paginated results.
async fn handle_get_ride_history(
    database: Option<Arc<Mutex<Database>>>,
    user_id: Option<Uuid>,
    limit: u32,
    offset: u32,
) -> CompanionResponse {
    debug!(
        "Get ride history request: limit={}, offset={}, user_id={:?}",
        limit, offset, user_id
    );

    // Validate parameters
    let limit = limit.min(100).max(1); // Clamp to 1-100
    let offset = offset;

    // Get database or return empty list if not available
    let db = match database {
        Some(db) => db,
        None => {
            warn!("Ride history requested but database not available");
            return CompanionResponse::RideHistory {
                rides: Vec::new(),
                total: 0,
            };
        }
    };

    // Get user ID or return empty list if not available
    let user_id = match user_id {
        Some(id) => id,
        None => {
            warn!("Ride history requested but user_id not available");
            return CompanionResponse::RideHistory {
                rides: Vec::new(),
                total: 0,
            };
        }
    };

    // Lock database and query rides
    let db_guard = match db.lock() {
        Ok(guard) => guard,
        Err(e) => {
            warn!("Failed to lock database: {}", e);
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Database access error".to_string(),
            };
        }
    };

    // Get total count for pagination
    let total = match db_guard.count_rides(&user_id) {
        Ok(count) => count as u32,
        Err(e) => {
            warn!("Failed to count rides: {}", e);
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Failed to query ride count".to_string(),
            };
        }
    };

    // Query rides with pagination
    let rides = match db_guard.list_rides(&user_id, Some(limit), Some(offset)) {
        Ok(rides) => rides,
        Err(e) => {
            warn!("Failed to list rides: {}", e);
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Failed to query rides".to_string(),
            };
        }
    };

    // Get workout names for rides that have workout_ids
    let ride_summaries: Vec<RideSummary> = rides
        .into_iter()
        .map(|ride| {
            // Try to get workout name if ride has a workout_id
            let workout_name = ride.workout_id.and_then(|workout_id| {
                db_guard
                    .get_workout(&workout_id)
                    .ok()
                    .flatten()
                    .map(|w| w.name)
            });

            ride_to_summary(&ride, workout_name)
        })
        .collect();

    info!(
        "Returning {} rides (total: {}, offset: {})",
        ride_summaries.len(),
        total,
        offset
    );

    CompanionResponse::RideHistory {
        rides: ride_summaries,
        total,
    }
}

/// Handle get ride details request.
///
/// T007: Query a specific ride from the database and return full details.
async fn handle_get_ride_details(
    database: Option<Arc<Mutex<Database>>>,
    ride_id: String,
) -> CompanionResponse {
    debug!("Get ride details request: ride_id={}", ride_id);

    // Parse ride_id as UUID
    let ride_uuid = match Uuid::parse_str(&ride_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::InvalidParams,
                message: format!("Invalid ride ID format: {}", ride_id),
            };
        }
    };

    // Get database or return error if not available
    let db = match database {
        Some(db) => db,
        None => {
            warn!("Ride details requested but database not available");
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Database not available".to_string(),
            };
        }
    };

    // Lock database and query ride
    let db_guard = match db.lock() {
        Ok(guard) => guard,
        Err(e) => {
            warn!("Failed to lock database: {}", e);
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Database access error".to_string(),
            };
        }
    };

    // Get ride from database
    let ride = match db_guard.get_ride(&ride_uuid) {
        Ok(Some(ride)) => ride,
        Ok(None) => {
            return CompanionResponse::Error {
                code: CompanionErrorCode::InvalidParams,
                message: format!("Ride not found: {}", ride_id),
            };
        }
        Err(e) => {
            warn!("Failed to get ride: {}", e);
            return CompanionResponse::Error {
                code: CompanionErrorCode::InternalError,
                message: "Failed to query ride".to_string(),
            };
        }
    };

    // Get workout name if ride has a workout_id
    let workout_name = ride.workout_id.and_then(|workout_id| {
        db_guard
            .get_workout(&workout_id)
            .ok()
            .flatten()
            .map(|w| w.name)
    });

    let ride_detail = ride_to_detail(&ride, workout_name);

    info!("Returning ride details for {}", ride_id);

    CompanionResponse::RideDetails { ride: ride_detail }
}

// ========== Ride Conversion Helpers (T007) ==========

/// Convert a Ride to a RideSummary for list responses.
fn ride_to_summary(ride: &Ride, workout_name: Option<String>) -> RideSummary {
    RideSummary {
        ride_id: ride.id.to_string(),
        started_at: ride.started_at.to_rfc3339(),
        duration_secs: ride.duration_seconds,
        distance_km: (ride.distance_meters / 1000.0) as f32,
        avg_power_watts: ride.avg_power,
        is_workout: ride.workout_id.is_some(),
        workout_name,
    }
}

/// Convert a Ride to a RideDetailInfo for detail responses.
fn ride_to_detail(ride: &Ride, workout_name: Option<String>) -> RideDetailInfo {
    RideDetailInfo {
        ride_id: ride.id.to_string(),
        started_at: ride.started_at.to_rfc3339(),
        ended_at: ride
            .ended_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| ride.started_at.to_rfc3339()),
        duration_secs: ride.duration_seconds,
        distance_km: (ride.distance_meters / 1000.0) as f32,
        calories: ride.calories,
        avg_power_watts: ride.avg_power,
        max_power_watts: ride.max_power,
        normalized_power_watts: ride.normalized_power,
        avg_heart_rate_bpm: ride.avg_hr,
        max_heart_rate_bpm: ride.max_hr,
        avg_cadence_rpm: ride.avg_cadence,
        tss: ride.tss,
        intensity_factor: ride.intensity_factor,
        is_workout: ride.workout_id.is_some(),
        workout_name,
    }
}

/// Create a metrics event from current sensor data.
///
/// This helper function constructs a metrics event that can be
/// broadcast to subscribed clients.
pub fn create_metrics_event(
    power_watts: Option<u16>,
    heart_rate_bpm: Option<u8>,
    cadence_rpm: Option<u8>,
    speed_kmh: Option<f32>,
    distance_km: f32,
    elapsed_secs: u32,
    calories: u32,
) -> CompanionEvent {
    CompanionEvent::Metrics {
        power_watts,
        heart_rate_bpm,
        cadence_rpm,
        speed_kmh,
        distance_km,
        elapsed_secs,
        calories,
    }
}

/// Create a session state changed event.
pub fn create_session_state_event(
    state: SessionState,
    session: Option<SessionStatusInfo>,
) -> CompanionEvent {
    CompanionEvent::SessionStateChanged { state, session }
}

/// Create an interval changed event.
pub fn create_interval_changed_event(
    interval_index: usize,
    total_intervals: usize,
    interval_name: String,
    target_power_watts: u16,
    duration_secs: u32,
) -> CompanionEvent {
    CompanionEvent::IntervalChanged {
        interval_index,
        total_intervals,
        interval_name,
        target_power_watts,
        duration_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::state::{LiveMetrics, SessionInfo, WorkoutInfo};
    use chrono::Utc;
    use std::path::PathBuf;

    /// Helper to create a test daemon state with an active workout session
    fn create_test_daemon_state_with_workout() -> Arc<RwLock<DaemonState>> {
        let mut state = DaemonState::default();
        state.active_session = Some(SessionInfo {
            session_id: Uuid::new_v4(),
            session_type: SessionType::Workout {
                path: PathBuf::from("/test/workout.zwo"),
            },
            started_at: Utc::now(),
            workout_info: Some(WorkoutInfo {
                name: "Test Workout".to_string(),
                file_path: PathBuf::from("/test/workout.zwo"),
                total_duration_seconds: 3600,
                current_interval_index: 0,
                total_intervals: 5,
                current_interval_name: "Warmup".to_string(),
                interval_elapsed_seconds: 0,
                interval_remaining_seconds: 300,
                target_power_watts: 150,
                target_power_percent_ftp: 0.60,
            }),
            current_metrics: LiveMetrics::default(),
            is_paused: false,
        });
        Arc::new(RwLock::new(state))
    }

    /// Helper to create a test database with rides
    fn create_test_database_with_rides(user_id: Uuid) -> Arc<Mutex<Database>> {
        let db = Database::open_in_memory().expect("Failed to create test database");

        // Insert some test rides
        for i in 0..5 {
            let mut ride = Ride::new(user_id, 200);
            ride.duration_seconds = 3600 + (i * 600);
            ride.distance_meters = 30000.0 + (i as f64 * 5000.0);
            ride.avg_power = Some(180 + (i as u16 * 10));
            ride.max_power = Some(300 + (i as u16 * 20));
            ride.calories = 500 + (i * 100);
            ride.ended_at = Some(ride.started_at + chrono::Duration::seconds(ride.duration_seconds as i64));
            db.insert_ride(&ride).expect("Failed to insert ride");
        }

        Arc::new(Mutex::new(db))
    }

    #[tokio::test]
    async fn test_ping_handler() {
        let response =
            handle_request(CompanionRequest::Ping, Uuid::new_v4(), false, None, None, None).await;
        assert!(matches!(response, CompanionResponse::Pong));
    }

    #[tokio::test]
    async fn test_auth_required() {
        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            false, // Not authenticated
            None,
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::Error { code, .. } => {
                assert_eq!(code, CompanionErrorCode::AuthRequired);
            }
            _ => panic!("Expected auth required error"),
        }
    }

    #[tokio::test]
    async fn test_auth_valid_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "123456".to_string(),
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthOk { .. }));
    }

    #[tokio::test]
    async fn test_auth_invalid_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "abc".to_string(),
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn test_workout_pause() {
        let daemon_state = create_test_daemon_state_with_workout();

        let response = handle_request(
            CompanionRequest::WorkoutPause,
            Uuid::new_v4(),
            true, // Authenticated
            Some(daemon_state.clone()),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "workout_pause"
        ));

        // Verify session is paused
        let state = daemon_state.read().await;
        assert!(state.active_session.as_ref().unwrap().is_paused);
    }

    #[tokio::test]
    async fn test_workout_pause_already_paused() {
        let daemon_state = create_test_daemon_state_with_workout();

        // Pause it first
        {
            let mut state = daemon_state.write().await;
            state.active_session.as_mut().unwrap().is_paused = true;
        }

        let response = handle_request(
            CompanionRequest::WorkoutPause,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, error }
            if command == "workout_pause" && error.contains("already paused")
        ));
    }

    #[tokio::test]
    async fn test_workout_resume() {
        let daemon_state = create_test_daemon_state_with_workout();

        // Pause it first
        {
            let mut state = daemon_state.write().await;
            state.active_session.as_mut().unwrap().is_paused = true;
        }

        let response = handle_request(
            CompanionRequest::WorkoutResume,
            Uuid::new_v4(),
            true,
            Some(daemon_state.clone()),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "workout_resume"
        ));

        // Verify session is resumed
        let state = daemon_state.read().await;
        assert!(!state.active_session.as_ref().unwrap().is_paused);
    }

    #[tokio::test]
    async fn test_workout_skip() {
        let daemon_state = create_test_daemon_state_with_workout();

        let response = handle_request(
            CompanionRequest::WorkoutSkip,
            Uuid::new_v4(),
            true,
            Some(daemon_state.clone()),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "workout_skip"
        ));

        // Verify interval index advanced
        let state = daemon_state.read().await;
        let workout = state
            .active_session
            .as_ref()
            .unwrap()
            .workout_info
            .as_ref()
            .unwrap();
        assert_eq!(workout.current_interval_index, 1);
    }

    #[tokio::test]
    async fn test_workout_skip_last_interval() {
        let daemon_state = create_test_daemon_state_with_workout();

        // Set to last interval
        {
            let mut state = daemon_state.write().await;
            let workout = state
                .active_session
                .as_mut()
                .unwrap()
                .workout_info
                .as_mut()
                .unwrap();
            workout.current_interval_index = 4; // Last interval (5 total, 0-indexed)
        }

        let response = handle_request(
            CompanionRequest::WorkoutSkip,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, error }
            if command == "workout_skip" && error.contains("last interval")
        ));
    }

    #[tokio::test]
    async fn test_workout_stop() {
        let daemon_state = create_test_daemon_state_with_workout();

        let response = handle_request(
            CompanionRequest::WorkoutStop,
            Uuid::new_v4(),
            true,
            Some(daemon_state.clone()),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "workout_stop"
        ));

        // Verify session is cleared
        let state = daemon_state.read().await;
        assert!(state.active_session.is_none());
    }

    #[tokio::test]
    async fn test_workout_command_no_session() {
        let daemon_state = Arc::new(RwLock::new(DaemonState::default()));

        let response = handle_request(
            CompanionRequest::WorkoutPause,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::Error { code: CompanionErrorCode::NoSession, .. }
        ));
    }

    #[tokio::test]
    async fn test_workout_command_no_daemon_state() {
        let response = handle_request(
            CompanionRequest::WorkoutPause,
            Uuid::new_v4(),
            true,
            None, // No daemon state
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, .. } if command == "workout_pause"
        ));
    }

    #[tokio::test]
    async fn test_get_session_status_with_active_workout() {
        let daemon_state = create_test_daemon_state_with_workout();

        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::SessionStatus { active, session } => {
                assert!(active);
                let session = session.unwrap();
                assert_eq!(session.session_type, "workout");
                assert_eq!(session.workout_name, Some("Test Workout".to_string()));
                assert_eq!(session.total_intervals, Some(5));
            }
            _ => panic!("Expected SessionStatus response"),
        }
    }

    #[tokio::test]
    async fn test_get_session_status_no_session() {
        let daemon_state = Arc::new(RwLock::new(DaemonState::default()));

        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::SessionStatus { active, session } => {
                assert!(!active);
                assert!(session.is_none());
            }
            _ => panic!("Expected SessionStatus response"),
        }
    }

    #[test]
    fn test_create_metrics_event() {
        let event = create_metrics_event(Some(200), Some(140), Some(90), Some(30.0), 10.5, 1800, 350);

        match event {
            CompanionEvent::Metrics {
                power_watts,
                heart_rate_bpm,
                ..
            } => {
                assert_eq!(power_watts, Some(200));
                assert_eq!(heart_rate_bpm, Some(140));
            }
            _ => panic!("Expected metrics event"),
        }
    }

    // ========== Ride History Tests (T007) ==========

    #[tokio::test]
    async fn test_get_ride_history_with_rides() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 10,
                offset: 0,
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, total } => {
                assert_eq!(total, 5);
                assert_eq!(rides.len(), 5);
                // Verify first ride has expected data
                assert!(rides[0].duration_secs >= 3600);
                assert!(rides[0].distance_km > 0.0);
                assert!(!rides[0].is_workout);
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_history_pagination() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 2,
                offset: 1,
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, total } => {
                assert_eq!(total, 5);
                assert_eq!(rides.len(), 2);
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_history_no_database() {
        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 10,
                offset: 0,
            },
            Uuid::new_v4(),
            true,
            None,
            None, // No database
            Some(Uuid::new_v4()),
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, total } => {
                assert_eq!(total, 0);
                assert!(rides.is_empty());
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_history_no_user_id() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 10,
                offset: 0,
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            None, // No user_id
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, total } => {
                assert_eq!(total, 0);
                assert!(rides.is_empty());
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_details_success() {
        let user_id = Uuid::new_v4();
        let db = Database::open_in_memory().expect("Failed to create test database");

        // Create a test ride
        let mut ride = Ride::new(user_id, 200);
        ride.duration_seconds = 3600;
        ride.distance_meters = 30000.0;
        ride.avg_power = Some(180);
        ride.max_power = Some(350);
        ride.normalized_power = Some(190);
        ride.avg_hr = Some(145);
        ride.max_hr = Some(175);
        ride.avg_cadence = Some(85);
        ride.calories = 650;
        ride.tss = Some(75.0);
        ride.intensity_factor = Some(0.95);
        ride.ended_at = Some(ride.started_at + chrono::Duration::seconds(3600));
        let ride_id = ride.id.to_string();
        db.insert_ride(&ride).expect("Failed to insert ride");

        let database = Arc::new(Mutex::new(db));

        let response = handle_request(
            CompanionRequest::GetRideDetails { ride_id: ride_id.clone() },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::RideDetails { ride: ride_detail } => {
                assert_eq!(ride_detail.ride_id, ride_id);
                assert_eq!(ride_detail.duration_secs, 3600);
                assert_eq!(ride_detail.distance_km, 30.0);
                assert_eq!(ride_detail.avg_power_watts, Some(180));
                assert_eq!(ride_detail.max_power_watts, Some(350));
                assert_eq!(ride_detail.normalized_power_watts, Some(190));
                assert_eq!(ride_detail.avg_heart_rate_bpm, Some(145));
                assert_eq!(ride_detail.max_heart_rate_bpm, Some(175));
                assert_eq!(ride_detail.avg_cadence_rpm, Some(85));
                assert_eq!(ride_detail.calories, 650);
                assert!(!ride_detail.is_workout);
            }
            _ => panic!("Expected RideDetails response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_details_not_found() {
        let user_id = Uuid::new_v4();
        let db = Database::open_in_memory().expect("Failed to create test database");
        let database = Arc::new(Mutex::new(db));
        let fake_ride_id = Uuid::new_v4().to_string();

        let response = handle_request(
            CompanionRequest::GetRideDetails { ride_id: fake_ride_id.clone() },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::Error { code, message } => {
                assert_eq!(code, CompanionErrorCode::InvalidParams);
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_details_invalid_id() {
        let user_id = Uuid::new_v4();
        let db = Database::open_in_memory().expect("Failed to create test database");
        let database = Arc::new(Mutex::new(db));

        let response = handle_request(
            CompanionRequest::GetRideDetails { ride_id: "not-a-uuid".to_string() },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::Error { code, message } => {
                assert_eq!(code, CompanionErrorCode::InvalidParams);
                assert!(message.contains("Invalid ride ID format"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_details_no_database() {
        let response = handle_request(
            CompanionRequest::GetRideDetails { ride_id: Uuid::new_v4().to_string() },
            Uuid::new_v4(),
            true,
            None,
            None, // No database
            Some(Uuid::new_v4()),
        )
        .await;

        match response {
            CompanionResponse::Error { code, message } => {
                assert_eq!(code, CompanionErrorCode::InternalError);
                assert!(message.contains("not available"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_ride_to_summary() {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.duration_seconds = 3600;
        ride.distance_meters = 40000.0;
        ride.avg_power = Some(200);

        let summary = ride_to_summary(&ride, None);
        assert_eq!(summary.ride_id, ride.id.to_string());
        assert_eq!(summary.duration_secs, 3600);
        assert_eq!(summary.distance_km, 40.0);
        assert_eq!(summary.avg_power_watts, Some(200));
        assert!(!summary.is_workout);
        assert!(summary.workout_name.is_none());
    }

    #[test]
    fn test_ride_to_summary_with_workout() {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.workout_id = Some(Uuid::new_v4());
        ride.duration_seconds = 3600;
        ride.distance_meters = 40000.0;

        let summary = ride_to_summary(&ride, Some("Sweet Spot".to_string()));
        assert!(summary.is_workout);
        assert_eq!(summary.workout_name, Some("Sweet Spot".to_string()));
    }

    #[test]
    fn test_ride_to_detail() {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.duration_seconds = 3600;
        ride.distance_meters = 40000.0;
        ride.avg_power = Some(200);
        ride.max_power = Some(350);
        ride.normalized_power = Some(210);
        ride.avg_hr = Some(145);
        ride.max_hr = Some(175);
        ride.avg_cadence = Some(90);
        ride.calories = 700;
        ride.tss = Some(80.0);
        ride.intensity_factor = Some(1.05);
        ride.ended_at = Some(ride.started_at + chrono::Duration::seconds(3600));

        let detail = ride_to_detail(&ride, None);
        assert_eq!(detail.ride_id, ride.id.to_string());
        assert_eq!(detail.duration_secs, 3600);
        assert_eq!(detail.distance_km, 40.0);
        assert_eq!(detail.calories, 700);
        assert_eq!(detail.avg_power_watts, Some(200));
        assert_eq!(detail.max_power_watts, Some(350));
        assert_eq!(detail.normalized_power_watts, Some(210));
        assert_eq!(detail.avg_heart_rate_bpm, Some(145));
        assert_eq!(detail.max_heart_rate_bpm, Some(175));
        assert_eq!(detail.avg_cadence_rpm, Some(90));
        assert_eq!(detail.tss, Some(80.0));
        assert_eq!(detail.intensity_factor, Some(1.05));
        assert!(!detail.is_workout);
    }

    // ========== Additional Authentication Tests ==========

    #[tokio::test]
    async fn test_auth_short_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "123".to_string(), // Too short
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn test_auth_long_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "12345678".to_string(), // Too long
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn test_auth_non_numeric_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "abc123".to_string(),
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn test_auth_empty_pin() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "".to_string(),
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
    }

    #[tokio::test]
    async fn test_auth_all_zeros_valid() {
        let response = handle_request(
            CompanionRequest::Auth {
                pin: "000000".to_string(),
            },
            Uuid::new_v4(),
            false,
            None,
            None,
            None,
        )
        .await;

        // All zeros is a valid 6-digit PIN format
        assert!(matches!(response, CompanionResponse::AuthOk { .. }));
    }

    #[tokio::test]
    async fn test_auth_required_for_get_ride_history() {
        let response = handle_request(
            CompanionRequest::GetRideHistory { limit: 10, offset: 0 },
            Uuid::new_v4(),
            false, // Not authenticated
            None,
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::Error { code, .. } => {
                assert_eq!(code, CompanionErrorCode::AuthRequired);
            }
            _ => panic!("Expected auth required error"),
        }
    }

    #[tokio::test]
    async fn test_auth_required_for_get_ride_details() {
        let response = handle_request(
            CompanionRequest::GetRideDetails { ride_id: "abc".to_string() },
            Uuid::new_v4(),
            false, // Not authenticated
            None,
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::Error { code, .. } => {
                assert_eq!(code, CompanionErrorCode::AuthRequired);
            }
            _ => panic!("Expected auth required error"),
        }
    }

    #[tokio::test]
    async fn test_auth_required_for_workout_commands() {
        // Test all workout control commands require auth
        let commands = vec![
            CompanionRequest::WorkoutPause,
            CompanionRequest::WorkoutResume,
            CompanionRequest::WorkoutSkip,
            CompanionRequest::WorkoutStop,
            CompanionRequest::AdjustResistance { delta: 5 },
        ];

        for cmd in commands {
            let response = handle_request(
                cmd,
                Uuid::new_v4(),
                false, // Not authenticated
                None,
                None,
                None,
            )
            .await;

            match response {
                CompanionResponse::Error { code, .. } => {
                    assert_eq!(code, CompanionErrorCode::AuthRequired);
                }
                _ => panic!("Expected auth required error for workout command"),
            }
        }
    }

    #[tokio::test]
    async fn test_auth_required_for_subscribe_metrics() {
        let response = handle_request(
            CompanionRequest::SubscribeMetrics,
            Uuid::new_v4(),
            false, // Not authenticated
            None,
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::Error { code, .. } => {
                assert_eq!(code, CompanionErrorCode::AuthRequired);
            }
            _ => panic!("Expected auth required error"),
        }
    }

    // ========== Additional Handler Tests ==========

    #[tokio::test]
    async fn test_subscribe_metrics_authenticated() {
        let response = handle_request(
            CompanionRequest::SubscribeMetrics,
            Uuid::new_v4(),
            true, // Authenticated
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::SubscribedMetrics));
    }

    #[tokio::test]
    async fn test_unsubscribe_metrics_authenticated() {
        let response = handle_request(
            CompanionRequest::UnsubscribeMetrics,
            Uuid::new_v4(),
            true, // Authenticated
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(response, CompanionResponse::UnsubscribedMetrics));
    }

    #[tokio::test]
    async fn test_adjust_resistance_authenticated() {
        let response = handle_request(
            CompanionRequest::AdjustResistance { delta: 10 },
            Uuid::new_v4(),
            true, // Authenticated
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "adjust_resistance"
        ));
    }

    #[tokio::test]
    async fn test_adjust_resistance_negative_delta() {
        let response = handle_request(
            CompanionRequest::AdjustResistance { delta: -15 },
            Uuid::new_v4(),
            true,
            None,
            None,
            None,
        )
        .await;

        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "adjust_resistance"
        ));
    }

    /// Helper to create a test daemon state with a free ride session
    fn create_test_daemon_state_with_free_ride() -> Arc<RwLock<DaemonState>> {
        let mut state = DaemonState::default();
        state.active_session = Some(SessionInfo {
            session_id: Uuid::new_v4(),
            session_type: SessionType::FreeRide,
            started_at: Utc::now(),
            workout_info: None, // No workout info for free ride
            current_metrics: LiveMetrics::default(),
            is_paused: false,
        });
        Arc::new(RwLock::new(state))
    }

    #[tokio::test]
    async fn test_workout_pause_on_free_ride() {
        let daemon_state = create_test_daemon_state_with_free_ride();

        let response = handle_request(
            CompanionRequest::WorkoutPause,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        // Should fail because free rides don't have workout info
        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, error }
            if command == "workout_pause" && error.contains("not a workout")
        ));
    }

    #[tokio::test]
    async fn test_workout_resume_not_paused() {
        let daemon_state = create_test_daemon_state_with_workout();

        let response = handle_request(
            CompanionRequest::WorkoutResume,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        // Should fail because workout is not paused
        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, error }
            if command == "workout_resume" && error.contains("not paused")
        ));
    }

    #[tokio::test]
    async fn test_workout_skip_on_free_ride() {
        let daemon_state = create_test_daemon_state_with_free_ride();

        let response = handle_request(
            CompanionRequest::WorkoutSkip,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        // Should fail because free rides don't have intervals
        assert!(matches!(
            response,
            CompanionResponse::CommandFailed { command, error }
            if command == "workout_skip" && error.contains("not a workout")
        ));
    }

    #[tokio::test]
    async fn test_workout_stop_free_ride() {
        let daemon_state = create_test_daemon_state_with_free_ride();

        let response = handle_request(
            CompanionRequest::WorkoutStop,
            Uuid::new_v4(),
            true,
            Some(daemon_state.clone()),
            None,
            None,
        )
        .await;

        // Should succeed - stop works for both workouts and free rides
        assert!(matches!(
            response,
            CompanionResponse::CommandOk { command } if command == "workout_stop"
        ));

        // Verify session is cleared
        let state = daemon_state.read().await;
        assert!(state.active_session.is_none());
    }

    #[tokio::test]
    async fn test_get_session_status_no_daemon_state() {
        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            true,
            None, // No daemon state
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::SessionStatus { active, session } => {
                assert!(!active);
                assert!(session.is_none());
            }
            _ => panic!("Expected SessionStatus response"),
        }
    }

    #[tokio::test]
    async fn test_get_session_status_free_ride() {
        let daemon_state = create_test_daemon_state_with_free_ride();

        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            true,
            Some(daemon_state),
            None,
            None,
        )
        .await;

        match response {
            CompanionResponse::SessionStatus { active, session } => {
                assert!(active);
                let session = session.unwrap();
                assert_eq!(session.session_type, "free_ride");
                assert!(session.workout_name.is_none());
                assert!(session.total_intervals.is_none());
            }
            _ => panic!("Expected SessionStatus response"),
        }
    }

    // ========== Event Helper Tests ==========

    #[test]
    fn test_create_session_state_event() {
        let event = create_session_state_event(SessionState::Paused, None);

        match event {
            CompanionEvent::SessionStateChanged { state, session } => {
                assert_eq!(state, SessionState::Paused);
                assert!(session.is_none());
            }
            _ => panic!("Expected SessionStateChanged event"),
        }
    }

    #[test]
    fn test_create_session_state_event_with_session() {
        let session_info = SessionStatusInfo {
            session_id: Uuid::nil(),
            session_type: "workout".to_string(),
            workout_name: Some("Test".to_string()),
            workout_path: None,
            is_paused: false,
            elapsed_secs: 100,
            current_interval_index: Some(0),
            total_intervals: Some(5),
            current_interval_name: Some("Warmup".to_string()),
            target_power_watts: Some(150),
            interval_remaining_secs: Some(300),
        };

        let event = create_session_state_event(SessionState::Active, Some(session_info));

        match event {
            CompanionEvent::SessionStateChanged { state, session } => {
                assert_eq!(state, SessionState::Active);
                assert!(session.is_some());
            }
            _ => panic!("Expected SessionStateChanged event"),
        }
    }

    #[test]
    fn test_create_interval_changed_event() {
        let event = create_interval_changed_event(
            3,
            10,
            "Threshold".to_string(),
            280,
            300,
        );

        match event {
            CompanionEvent::IntervalChanged {
                interval_index,
                total_intervals,
                interval_name,
                target_power_watts,
                duration_secs,
            } => {
                assert_eq!(interval_index, 3);
                assert_eq!(total_intervals, 10);
                assert_eq!(interval_name, "Threshold");
                assert_eq!(target_power_watts, 280);
                assert_eq!(duration_secs, 300);
            }
            _ => panic!("Expected IntervalChanged event"),
        }
    }

    #[test]
    fn test_create_metrics_event_all_values() {
        let event = create_metrics_event(
            Some(250),
            Some(150),
            Some(95),
            Some(35.5),
            42.5,
            5400,
            750,
        );

        match event {
            CompanionEvent::Metrics {
                power_watts,
                heart_rate_bpm,
                cadence_rpm,
                speed_kmh,
                distance_km,
                elapsed_secs,
                calories,
            } => {
                assert_eq!(power_watts, Some(250));
                assert_eq!(heart_rate_bpm, Some(150));
                assert_eq!(cadence_rpm, Some(95));
                assert_eq!(speed_kmh, Some(35.5));
                assert!((distance_km - 42.5).abs() < 0.001);
                assert_eq!(elapsed_secs, 5400);
                assert_eq!(calories, 750);
            }
            _ => panic!("Expected Metrics event"),
        }
    }

    #[test]
    fn test_create_metrics_event_partial_values() {
        let event = create_metrics_event(
            Some(200),
            None, // No HR sensor
            None, // No cadence sensor
            Some(30.0),
            10.0,
            1800,
            250,
        );

        match event {
            CompanionEvent::Metrics {
                power_watts,
                heart_rate_bpm,
                cadence_rpm,
                ..
            } => {
                assert_eq!(power_watts, Some(200));
                assert_eq!(heart_rate_bpm, None);
                assert_eq!(cadence_rpm, None);
            }
            _ => panic!("Expected Metrics event"),
        }
    }

    // ========== Ride History Edge Cases ==========

    #[tokio::test]
    async fn test_get_ride_history_limit_clamping_high() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 1000, // Should be clamped to 100
                offset: 0,
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        // Should still work (clamped limit)
        assert!(matches!(response, CompanionResponse::RideHistory { .. }));
    }

    #[tokio::test]
    async fn test_get_ride_history_limit_clamping_zero() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 0, // Should be clamped to 1
                offset: 0,
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, .. } => {
                // Clamped to at least 1
                assert!(rides.len() <= 1);
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[tokio::test]
    async fn test_get_ride_history_high_offset() {
        let user_id = Uuid::new_v4();
        let database = create_test_database_with_rides(user_id);

        let response = handle_request(
            CompanionRequest::GetRideHistory {
                limit: 10,
                offset: 1000, // Beyond all rides
            },
            Uuid::new_v4(),
            true,
            None,
            Some(database),
            Some(user_id),
        )
        .await;

        match response {
            CompanionResponse::RideHistory { rides, total } => {
                assert_eq!(total, 5); // Total is still 5
                assert!(rides.is_empty()); // But no rides returned at this offset
            }
            _ => panic!("Expected RideHistory response"),
        }
    }

    #[test]
    fn test_ride_to_detail_with_workout() {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.workout_id = Some(Uuid::new_v4());
        ride.duration_seconds = 3600;
        ride.distance_meters = 30000.0;
        ride.ended_at = Some(ride.started_at + chrono::Duration::seconds(3600));

        let detail = ride_to_detail(&ride, Some("VO2max Intervals".to_string()));
        assert!(detail.is_workout);
        assert_eq!(detail.workout_name, Some("VO2max Intervals".to_string()));
    }

    #[test]
    fn test_ride_to_summary_without_power() {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.duration_seconds = 1800;
        ride.distance_meters = 15000.0;
        ride.avg_power = None; // No power data

        let summary = ride_to_summary(&ride, None);
        assert!(summary.avg_power_watts.is_none());
    }
}
