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
//! - **Ride History**: Past ride queries and statistics

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::types::{
    CompanionErrorCode, CompanionEvent, CompanionRequest, CompanionResponse, RideDetailInfo,
    RideSummary, SessionState, SessionStatusInfo,
};
use crate::daemon::state::{DaemonState, SessionType};

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
pub async fn handle_request(
    request: CompanionRequest,
    session_id: Uuid,
    is_authenticated: bool,
    daemon_state: Option<Arc<RwLock<DaemonState>>>,
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
            handle_get_ride_history(limit, offset).await
        }
        CompanionRequest::GetRideDetails { ride_id } => handle_get_ride_details(ride_id).await,
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
async fn handle_get_ride_history(limit: u32, offset: u32) -> CompanionResponse {
    // TODO: T007 - Query rides from database
    debug!("Get ride history request: limit={}, offset={}", limit, offset);

    CompanionResponse::RideHistory {
        rides: Vec::new(),
        total: 0,
    }
}

/// Handle get ride details request.
async fn handle_get_ride_details(ride_id: String) -> CompanionResponse {
    // TODO: T007 - Query ride details from database
    debug!("Get ride details request: ride_id={}", ride_id);

    CompanionResponse::Error {
        code: CompanionErrorCode::NoSession,
        message: format!("Ride not found: {}", ride_id),
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

    #[tokio::test]
    async fn test_ping_handler() {
        let response = handle_request(CompanionRequest::Ping, Uuid::new_v4(), false, None).await;
        assert!(matches!(response, CompanionResponse::Pong));
    }

    #[tokio::test]
    async fn test_auth_required() {
        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            false, // Not authenticated
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
}
