/**
 * useWorkoutControls Hook
 *
 * Provides workout control actions (pause, resume, skip, stop) with:
 * - WebSocket command sending via ConnectionService
 * - Optimistic state updates for responsive UI
 * - Error handling with state rollback
 * - Loading states for each action
 */

import { useState, useCallback } from 'react';
import { getConnectionService } from '@/services/ConnectionService';
import { useSessionStore } from '@/stores/sessionStore';

/**
 * State for each workout control action
 */
interface ActionState {
  /** Whether the action is currently in progress */
  isLoading: boolean;
  /** Error message if the action failed */
  error: string | null;
}

/**
 * Return type for the useWorkoutControls hook
 */
export interface UseWorkoutControlsReturn {
  /** State for pause action */
  pauseState: ActionState;
  /** State for resume action */
  resumeState: ActionState;
  /** State for skip action */
  skipState: ActionState;
  /** State for stop action */
  stopState: ActionState;

  /** Pause the current workout/session */
  pause: () => Promise<void>;
  /** Resume the current workout/session */
  resume: () => Promise<void>;
  /** Skip to the next interval */
  skip: () => Promise<void>;
  /** Stop the current workout/session */
  stop: () => Promise<void>;

  /** Clear any error state */
  clearError: () => void;

  /** Combined loading state for pause/resume */
  isPauseResumeLoading: boolean;
  /** Combined loading state for skip */
  isSkipLoading: boolean;
  /** Combined loading state for stop */
  isStopLoading: boolean;
}

/**
 * Initial action state
 */
const initialActionState: ActionState = {
  isLoading: false,
  error: null,
};

/**
 * useWorkoutControls Hook
 *
 * Provides workout control actions with optimistic updates and error handling.
 *
 * @example
 * ```tsx
 * function WorkoutControls() {
 *   const {
 *     pause,
 *     resume,
 *     isPauseResumeLoading,
 *     pauseState,
 *   } = useWorkoutControls();
 *
 *   const handleToggle = async () => {
 *     if (isPaused) {
 *       await resume();
 *     } else {
 *       await pause();
 *     }
 *   };
 *
 *   return (
 *     <Button
 *       onPress={handleToggle}
 *       loading={isPauseResumeLoading}
 *       disabled={isPauseResumeLoading}
 *     />
 *   );
 * }
 * ```
 */
export function useWorkoutControls(): UseWorkoutControlsReturn {
  const [pauseState, setPauseState] = useState<ActionState>(initialActionState);
  const [resumeState, setResumeState] = useState<ActionState>(initialActionState);
  const [skipState, setSkipState] = useState<ActionState>(initialActionState);
  const [stopState, setStopState] = useState<ActionState>(initialActionState);

  /**
   * Pause the current workout/session
   *
   * Performs an optimistic update to show paused state immediately,
   * then sends the command to the server. If the command fails,
   * the state is rolled back.
   */
  const pause = useCallback(async (): Promise<void> => {
    const connectionService = getConnectionService();
    const sessionStore = useSessionStore.getState();

    // Don't allow pause if already paused or not active
    if (sessionStore.isPaused || !sessionStore.isActive) {
      return;
    }

    // Set loading state
    setPauseState({ isLoading: true, error: null });

    // Optimistically update local state
    sessionStore.setPaused(true);

    try {
      // Send command to server
      await connectionService.pauseWorkout();

      // Success - clear loading state
      setPauseState({ isLoading: false, error: null });
    } catch (error) {
      // Rollback optimistic update
      sessionStore.setPaused(false);

      // Set error state
      const errorMessage = error instanceof Error ? error.message : 'Failed to pause workout';
      setPauseState({ isLoading: false, error: errorMessage });
    }
  }, []);

  /**
   * Resume the current workout/session
   *
   * Performs an optimistic update to show active state immediately,
   * then sends the command to the server. If the command fails,
   * the state is rolled back.
   */
  const resume = useCallback(async (): Promise<void> => {
    const connectionService = getConnectionService();
    const sessionStore = useSessionStore.getState();

    // Don't allow resume if not paused or not active
    if (!sessionStore.isPaused || !sessionStore.isActive) {
      return;
    }

    // Set loading state
    setResumeState({ isLoading: true, error: null });

    // Optimistically update local state
    sessionStore.setPaused(false);

    try {
      // Send command to server
      await connectionService.resumeWorkout();

      // Success - clear loading state
      setResumeState({ isLoading: false, error: null });
    } catch (error) {
      // Rollback optimistic update
      sessionStore.setPaused(true);

      // Set error state
      const errorMessage = error instanceof Error ? error.message : 'Failed to resume workout';
      setResumeState({ isLoading: false, error: errorMessage });
    }
  }, []);

  /**
   * Skip to the next interval
   *
   * Note: Skip does not have an optimistic update since the interval
   * information comes from the server.
   */
  const skip = useCallback(async (): Promise<void> => {
    const connectionService = getConnectionService();
    const sessionStore = useSessionStore.getState();

    // Don't allow skip if no active session or no interval info
    if (!sessionStore.isActive || !sessionStore.currentInterval) {
      return;
    }

    // Set loading state
    setSkipState({ isLoading: true, error: null });

    try {
      // Send command to server
      await connectionService.skipInterval();

      // Success - clear loading state (server will push interval_changed event)
      setSkipState({ isLoading: false, error: null });
    } catch (error) {
      // Set error state
      const errorMessage = error instanceof Error ? error.message : 'Failed to skip interval';
      setSkipState({ isLoading: false, error: errorMessage });
    }
  }, []);

  /**
   * Stop the current workout/session
   *
   * Note: Stop does not have an optimistic update since stopping a session
   * should wait for server confirmation.
   */
  const stop = useCallback(async (): Promise<void> => {
    const connectionService = getConnectionService();
    const sessionStore = useSessionStore.getState();

    // Don't allow stop if no active session
    if (!sessionStore.isActive) {
      return;
    }

    // Set loading state
    setStopState({ isLoading: true, error: null });

    try {
      // Send command to server
      await connectionService.stopWorkout();

      // Success - clear loading state (server will push session_state_changed event)
      setStopState({ isLoading: false, error: null });
    } catch (error) {
      // Set error state
      const errorMessage = error instanceof Error ? error.message : 'Failed to stop workout';
      setStopState({ isLoading: false, error: errorMessage });
    }
  }, []);

  /**
   * Clear all error states
   */
  const clearError = useCallback((): void => {
    setPauseState(prev => ({ ...prev, error: null }));
    setResumeState(prev => ({ ...prev, error: null }));
    setSkipState(prev => ({ ...prev, error: null }));
    setStopState(prev => ({ ...prev, error: null }));
  }, []);

  return {
    pauseState,
    resumeState,
    skipState,
    stopState,
    pause,
    resume,
    skip,
    stop,
    clearError,
    isPauseResumeLoading: pauseState.isLoading || resumeState.isLoading,
    isSkipLoading: skipState.isLoading,
    isStopLoading: stopState.isLoading,
  };
}
