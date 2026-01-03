/**
 * RustRide Companion App - Type Definitions
 *
 * Common type definitions used throughout the app.
 */

// Connection status types
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'authenticated';

// Session types
export type SessionType = 'free_ride' | 'workout';
export type SessionState = 'idle' | 'active' | 'paused' | 'completed';

// Metrics types
export interface LiveMetrics {
  power_watts: number;
  heart_rate_bpm: number | null;
  cadence_rpm: number | null;
  speed_kph: number;
  distance_km: number;
  calories: number;
}

// Session status info from server
export interface SessionStatusInfo {
  session_id: string;
  session_type: SessionType;
  workout_name?: string;
  workout_path?: string;
  is_paused: boolean;
  elapsed_secs: number;
  current_interval_index?: number;
  total_intervals?: number;
  current_interval_name?: string;
  target_power_watts?: number;
  interval_remaining_secs?: number;
}

// Ride summary for history list
export interface RideSummary {
  id: string;
  date: string;
  duration_secs: number;
  distance_km: number;
  avg_power_watts: number;
  workout_name?: string;
  is_workout?: boolean;
}

// Detailed ride information
export interface RideDetailInfo {
  ride_id: string;
  started_at: string;
  ended_at: string;
  duration_secs: number;
  distance_km: number;
  calories: number;
  avg_power_watts: number | null;
  max_power_watts: number | null;
  normalized_power_watts: number | null;
  avg_heart_rate_bpm: number | null;
  max_heart_rate_bpm: number | null;
  avg_cadence_rpm: number | null;
  tss: number | null;
  intensity_factor: number | null;
  is_workout: boolean;
  workout_name: string | null;
}

// Server discovery result
export interface DiscoveredServer {
  name: string;
  host: string;
  port: number;
  version?: string;
}

// App settings
export interface AppSettings {
  units: 'metric' | 'imperial';
  keepScreenAwake: boolean;
  hapticFeedback: 'off' | 'light' | 'medium' | 'strong';
  theme: 'system' | 'light' | 'dark';
}

// ============================================================
// WebSocket Protocol Types
// ============================================================

/**
 * Request messages sent to the RustRide desktop app.
 * Matches CompanionRequest from src/companion/types.rs
 */
export type CompanionRequest =
  | { type: 'auth'; pin: string }
  | { type: 'get_session_status' }
  | { type: 'subscribe_metrics' }
  | { type: 'unsubscribe_metrics' }
  | { type: 'workout_pause' }
  | { type: 'workout_resume' }
  | { type: 'workout_skip' }
  | { type: 'workout_stop' }
  | { type: 'adjust_resistance'; delta: number }
  | { type: 'get_ride_history'; limit: number; offset: number }
  | { type: 'get_ride_details'; ride_id: string }
  | { type: 'ping' };

/**
 * Error codes returned by the server.
 * Matches CompanionErrorCode from src/companion/types.rs
 */
export type CompanionErrorCode =
  | 'AUTH_REQUIRED'
  | 'INVALID_PIN'
  | 'NO_SESSION'
  | 'SESSION_ACTIVE'
  | 'UNKNOWN_COMMAND'
  | 'INVALID_PARAMS'
  | 'RATE_LIMITED'
  | 'INTERNAL_ERROR';

/**
 * Response messages received from the RustRide desktop app.
 * Matches CompanionResponse from src/companion/types.rs
 */
export type CompanionResponse =
  | { type: 'auth_ok'; session_id: string }
  | { type: 'auth_failed'; reason: string }
  | { type: 'session_status'; active: boolean; session: SessionStatusInfo | null }
  | { type: 'subscribed_metrics' }
  | { type: 'unsubscribed_metrics' }
  | { type: 'command_ok'; command: string }
  | { type: 'command_failed'; command: string; error: string }
  | { type: 'ride_history'; rides: RideSummary[]; total: number }
  | { type: 'ride_details'; ride: RideDetailInfo }
  | { type: 'pong' }
  | { type: 'error'; code: CompanionErrorCode; message: string };

/**
 * Event messages pushed from the RustRide desktop app.
 * Matches CompanionEvent from src/companion/types.rs
 */
export type CompanionEvent =
  | {
      type: 'metrics';
      power_watts: number | null;
      heart_rate_bpm: number | null;
      cadence_rpm: number | null;
      speed_kmh: number | null;
      distance_km: number;
      elapsed_secs: number;
      calories: number;
    }
  | {
      type: 'session_state_changed';
      state: 'idle' | 'starting' | 'active' | 'paused' | 'stopping' | 'completed';
      session: SessionStatusInfo | null;
    }
  | {
      type: 'interval_changed';
      interval_index: number;
      total_intervals: number;
      interval_name: string;
      target_power_watts: number;
      duration_secs: number;
    }
  | {
      type: 'disconnecting';
      reason: string;
    };

/**
 * Union type for all possible WebSocket messages from the server
 */
export type ServerMessage = CompanionResponse | CompanionEvent;

/**
 * Helper to check if a message is an event (vs response)
 */
export function isCompanionEvent(message: ServerMessage): message is CompanionEvent {
  return (
    message.type === 'metrics' ||
    message.type === 'session_state_changed' ||
    message.type === 'interval_changed' ||
    message.type === 'disconnecting'
  );
}

/**
 * Helper to check if a message is a response
 */
export function isCompanionResponse(message: ServerMessage): message is CompanionResponse {
  return !isCompanionEvent(message);
}
