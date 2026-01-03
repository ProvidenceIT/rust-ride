/**
 * Session Store
 *
 * Manages workout/ride session state received from the RustRide desktop app.
 * Tracks session type, status, intervals, and elapsed time.
 */

import { create } from 'zustand';
import type { SessionStatusInfo, SessionType, SessionState } from '@/types';

/**
 * Interval information for structured workouts
 */
interface IntervalInfo {
  index: number;
  total: number;
  name: string | null;
  remainingSecs: number | null;
}

/**
 * Session store state
 */
interface SessionStoreState {
  // Session status
  isActive: boolean;
  sessionId: string | null;
  sessionType: SessionType | null;
  sessionState: SessionState;

  // Workout details
  workoutName: string | null;
  workoutPath: string | null;

  // Timing
  elapsedSecs: number;
  isPaused: boolean;

  // Interval tracking
  currentInterval: IntervalInfo | null;
  targetPowerWatts: number | null;

  // Resistance/Grade control (for free rides)
  resistanceLevel: number;

  // Last update
  lastStatusUpdate: number | null;
}

/**
 * Session store actions
 */
interface SessionActions {
  // Session lifecycle
  startSession: (info: SessionStatusInfo) => void;
  endSession: () => void;

  // Session updates
  updateStatus: (info: SessionStatusInfo) => void;
  setSessionState: (state: SessionState) => void;
  setPaused: (isPaused: boolean) => void;

  // Interval updates
  updateInterval: (interval: IntervalInfo | null) => void;
  setTargetPower: (watts: number | null) => void;

  // Time tracking
  updateElapsedTime: (secs: number) => void;

  // Resistance control (for free rides)
  setResistanceLevel: (level: number) => void;
  adjustResistanceLevel: (delta: number) => void;

  // Reset store
  reset: () => void;
}

/**
 * Initial session state
 */
const initialState: SessionStoreState = {
  isActive: false,
  sessionId: null,
  sessionType: null,
  sessionState: 'idle',
  workoutName: null,
  workoutPath: null,
  elapsedSecs: 0,
  isPaused: false,
  currentInterval: null,
  targetPowerWatts: null,
  resistanceLevel: 0,
  lastStatusUpdate: null,
};

/**
 * Session store
 *
 * Manages workout and free ride session state with interval tracking.
 */
export const useSessionStore = create<SessionStoreState & SessionActions>()(set => ({
  ...initialState,

  // Session lifecycle
  startSession: (info: SessionStatusInfo) => {
    set({
      isActive: true,
      sessionId: info.session_id,
      sessionType: info.session_type,
      sessionState: 'active',
      workoutName: info.workout_name ?? null,
      workoutPath: info.workout_path ?? null,
      elapsedSecs: info.elapsed_secs,
      isPaused: info.is_paused,
      currentInterval:
        info.current_interval_index !== undefined
          ? {
              index: info.current_interval_index,
              total: info.total_intervals ?? 0,
              name: info.current_interval_name ?? null,
              remainingSecs: info.interval_remaining_secs ?? null,
            }
          : null,
      targetPowerWatts: info.target_power_watts ?? null,
      lastStatusUpdate: Date.now(),
    });
  },

  endSession: () => {
    set({
      ...initialState,
      sessionState: 'completed',
    });
  },

  // Session updates
  updateStatus: (info: SessionStatusInfo) => {
    set({
      isActive: true,
      sessionId: info.session_id,
      sessionType: info.session_type,
      sessionState: info.is_paused ? 'paused' : 'active',
      workoutName: info.workout_name ?? null,
      workoutPath: info.workout_path ?? null,
      elapsedSecs: info.elapsed_secs,
      isPaused: info.is_paused,
      currentInterval:
        info.current_interval_index !== undefined
          ? {
              index: info.current_interval_index,
              total: info.total_intervals ?? 0,
              name: info.current_interval_name ?? null,
              remainingSecs: info.interval_remaining_secs ?? null,
            }
          : null,
      targetPowerWatts: info.target_power_watts ?? null,
      lastStatusUpdate: Date.now(),
    });
  },

  setSessionState: (state: SessionState) => {
    set({ sessionState: state });
  },

  setPaused: (isPaused: boolean) => {
    set({
      isPaused,
      sessionState: isPaused ? 'paused' : 'active',
    });
  },

  // Interval updates
  updateInterval: (interval: IntervalInfo | null) => {
    set({ currentInterval: interval });
  },

  setTargetPower: (watts: number | null) => {
    set({ targetPowerWatts: watts });
  },

  // Time tracking
  updateElapsedTime: (secs: number) => {
    set({ elapsedSecs: secs });
  },

  // Resistance control (for free rides)
  setResistanceLevel: (level: number) => {
    set({ resistanceLevel: level });
  },

  adjustResistanceLevel: (delta: number) => {
    set(state => ({
      resistanceLevel: Math.max(-100, Math.min(100, state.resistanceLevel + delta)),
    }));
  },

  // Reset store
  reset: () => {
    set(initialState);
  },
}));

// Selectors for optimized component subscriptions
export const selectIsSessionActive = (state: SessionStoreState & SessionActions) => state.isActive;

export const selectSessionId = (state: SessionStoreState & SessionActions) => state.sessionId;

export const selectSessionType = (state: SessionStoreState & SessionActions) => state.sessionType;

export const selectSessionState = (state: SessionStoreState & SessionActions) => state.sessionState;

export const selectWorkoutName = (state: SessionStoreState & SessionActions) => state.workoutName;

export const selectIsPaused = (state: SessionStoreState & SessionActions) => state.isPaused;

export const selectElapsedSecs = (state: SessionStoreState & SessionActions) => state.elapsedSecs;

export const selectCurrentInterval = (state: SessionStoreState & SessionActions) =>
  state.currentInterval;

export const selectTargetPower = (state: SessionStoreState & SessionActions) =>
  state.targetPowerWatts;

export const selectIsWorkout = (state: SessionStoreState & SessionActions) =>
  state.sessionType === 'workout';

export const selectIsFreeRide = (state: SessionStoreState & SessionActions) =>
  state.sessionType === 'free_ride';

export const selectIntervalProgress = (
  state: SessionStoreState & SessionActions,
): number | null => {
  const interval = state.currentInterval;
  if (!interval || interval.total === 0) {
    return null;
  }
  return (interval.index + 1) / interval.total;
};

export const selectCanPause = (state: SessionStoreState & SessionActions) =>
  state.isActive && !state.isPaused;

export const selectCanResume = (state: SessionStoreState & SessionActions) =>
  state.isActive && state.isPaused;

export const selectCanSkip = (state: SessionStoreState & SessionActions): boolean => {
  const interval = state.currentInterval;
  if (!interval || !state.isActive) {
    return false;
  }
  // Can skip if not on the last interval
  return interval.index < interval.total - 1;
};

export const selectCanStop = (state: SessionStoreState & SessionActions) => state.isActive;

export const selectResistanceLevel = (state: SessionStoreState & SessionActions) =>
  state.resistanceLevel;
