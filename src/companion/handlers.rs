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

use tracing::{debug, warn};
use uuid::Uuid;

use super::types::{
    CompanionErrorCode, CompanionEvent, CompanionRequest, CompanionResponse, RideDetailInfo,
    RideSummary, SessionState, SessionStatusInfo,
};

/// Handle an incoming companion request.
///
/// This function routes the request to the appropriate handler based
/// on the request type and returns a response.
pub async fn handle_request(
    request: CompanionRequest,
    session_id: Uuid,
    is_authenticated: bool,
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
        CompanionRequest::GetSessionStatus => handle_get_session_status().await,
        CompanionRequest::SubscribeMetrics => handle_subscribe_metrics(session_id).await,
        CompanionRequest::UnsubscribeMetrics => handle_unsubscribe_metrics(session_id).await,
        CompanionRequest::WorkoutPause => handle_workout_pause().await,
        CompanionRequest::WorkoutResume => handle_workout_resume().await,
        CompanionRequest::WorkoutSkip => handle_workout_skip().await,
        CompanionRequest::WorkoutStop => handle_workout_stop().await,
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
async fn handle_get_session_status() -> CompanionResponse {
    // TODO: T010 - Query actual session state from daemon
    debug!("Session status request");

    CompanionResponse::SessionStatus {
        active: false,
        session: None,
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
async fn handle_workout_pause() -> CompanionResponse {
    // TODO: T006 - Integrate with daemon handler for workout.pause
    debug!("Workout pause request");

    CompanionResponse::CommandOk {
        command: "workout_pause".to_string(),
    }
}

/// Handle workout resume request.
async fn handle_workout_resume() -> CompanionResponse {
    // TODO: T006 - Integrate with daemon handler for workout.resume
    debug!("Workout resume request");

    CompanionResponse::CommandOk {
        command: "workout_resume".to_string(),
    }
}

/// Handle workout skip request.
async fn handle_workout_skip() -> CompanionResponse {
    // TODO: T006 - Integrate with daemon handler for workout.skip
    debug!("Workout skip request");

    CompanionResponse::CommandOk {
        command: "workout_skip".to_string(),
    }
}

/// Handle workout stop request.
async fn handle_workout_stop() -> CompanionResponse {
    // TODO: T006 - Integrate with daemon handler for workout.stop
    debug!("Workout stop request");

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

    #[tokio::test]
    async fn test_ping_handler() {
        let response = handle_request(CompanionRequest::Ping, Uuid::new_v4(), false).await;
        assert!(matches!(response, CompanionResponse::Pong));
    }

    #[tokio::test]
    async fn test_auth_required() {
        let response = handle_request(
            CompanionRequest::GetSessionStatus,
            Uuid::new_v4(),
            false, // Not authenticated
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
        )
        .await;

        assert!(matches!(response, CompanionResponse::AuthFailed { .. }));
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
