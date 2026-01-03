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
