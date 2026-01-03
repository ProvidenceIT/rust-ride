/**
 * useResistanceControl Hook
 *
 * Provides resistance/grade control for free rides.
 * Sends adjust_resistance commands via WebSocket and manages local state.
 *
 * Features:
 * - Optimistic updates for responsive UI
 * - Error handling with state rollback
 * - Loading state tracking
 * - Configurable step size (default: 5%)
 */

import { useState, useCallback } from 'react';
import { getConnectionService } from '@/services/ConnectionService';
import { useSessionStore, selectResistanceLevel, selectIsFreeRide } from '@/stores/sessionStore';

/**
 * State for resistance control action
 */
interface ResistanceActionState {
  /** Whether an adjustment is in progress */
  isLoading: boolean;
  /** Error message if the adjustment failed */
  error: string | null;
}

/**
 * Return type for the useResistanceControl hook
 */
export interface UseResistanceControlReturn {
  /** Current resistance level (-100 to 100) */
  resistanceLevel: number;
  /** Whether the current session is a free ride */
  isFreeRide: boolean;
  /** Whether resistance can be adjusted */
  canAdjust: boolean;
  /** Whether an adjustment is in progress */
  isLoading: boolean;
  /** Error message if the last adjustment failed */
  error: string | null;
  /** Increase resistance by step size */
  increaseResistance: () => Promise<void>;
  /** Decrease resistance by step size */
  decreaseResistance: () => Promise<void>;
  /** Adjust resistance by a specific delta */
  adjustResistance: (delta: number) => Promise<void>;
  /** Clear any error state */
  clearError: () => void;
  /** Step size for resistance adjustments */
  stepSize: number;
  /** Whether resistance can be increased (not at max) */
  canIncrease: boolean;
  /** Whether resistance can be decreased (not at min) */
  canDecrease: boolean;
}

/**
 * Default step size for resistance adjustments (5%)
 */
const DEFAULT_STEP_SIZE = 5;

/**
 * Minimum resistance level
 */
const MIN_RESISTANCE = -100;

/**
 * Maximum resistance level
 */
const MAX_RESISTANCE = 100;

/**
 * Initial action state
 */
const initialActionState: ResistanceActionState = {
  isLoading: false,
  error: null,
};

/**
 * useResistanceControl Hook
 *
 * Provides resistance/grade adjustment controls for free rides.
 * Integrates with the ConnectionService to send commands and
 * the sessionStore to manage local state.
 *
 * @param stepSize - The amount to adjust resistance per button press (default: 5)
 *
 * @example
 * ```tsx
 * function ResistanceControls() {
 *   const {
 *     resistanceLevel,
 *     isFreeRide,
 *     increaseResistance,
 *     decreaseResistance,
 *     isLoading,
 *     canIncrease,
 *     canDecrease,
 *   } = useResistanceControl();
 *
 *   if (!isFreeRide) return null;
 *
 *   return (
 *     <View>
 *       <Text>{resistanceLevel}%</Text>
 *       <Button onPress={decreaseResistance} disabled={!canDecrease}>-</Button>
 *       <Button onPress={increaseResistance} disabled={!canIncrease}>+</Button>
 *     </View>
 *   );
 * }
 * ```
 */
export function useResistanceControl(stepSize: number = DEFAULT_STEP_SIZE): UseResistanceControlReturn {
  const [actionState, setActionState] = useState<ResistanceActionState>(initialActionState);

  // Get current resistance level and session type from store
  const resistanceLevel = useSessionStore(selectResistanceLevel);
  const isFreeRide = useSessionStore(selectIsFreeRide);
  const isSessionActive = useSessionStore(state => state.isActive);

  // Determine if resistance can be adjusted
  const canAdjust = isFreeRide && isSessionActive && !actionState.isLoading;
  const canIncrease = canAdjust && resistanceLevel < MAX_RESISTANCE;
  const canDecrease = canAdjust && resistanceLevel > MIN_RESISTANCE;

  /**
   * Adjust resistance by a specific delta value
   *
   * Performs an optimistic update to show the new level immediately,
   * then sends the command to the server. If the command fails,
   * the state is rolled back.
   */
  const adjustResistance = useCallback(
    async (delta: number): Promise<void> => {
      if (!canAdjust) {
        return;
      }

      const connectionService = getConnectionService();
      const sessionStore = useSessionStore.getState();
      const previousLevel = sessionStore.resistanceLevel;

      // Set loading state
      setActionState({ isLoading: true, error: null });

      // Optimistically update local state
      sessionStore.adjustResistanceLevel(delta);

      try {
        // Send command to server
        await connectionService.adjustResistance(delta);

        // Success - clear loading state
        setActionState({ isLoading: false, error: null });
      } catch (error) {
        // Rollback optimistic update
        sessionStore.setResistanceLevel(previousLevel);

        // Set error state
        const errorMessage = error instanceof Error ? error.message : 'Failed to adjust resistance';
        setActionState({ isLoading: false, error: errorMessage });
      }
    },
    [canAdjust],
  );

  /**
   * Increase resistance by step size
   */
  const increaseResistance = useCallback(async (): Promise<void> => {
    if (!canIncrease) {
      return;
    }
    await adjustResistance(stepSize);
  }, [canIncrease, stepSize, adjustResistance]);

  /**
   * Decrease resistance by step size
   */
  const decreaseResistance = useCallback(async (): Promise<void> => {
    if (!canDecrease) {
      return;
    }
    await adjustResistance(-stepSize);
  }, [canDecrease, stepSize, adjustResistance]);

  /**
   * Clear any error state
   */
  const clearError = useCallback((): void => {
    setActionState(prev => ({ ...prev, error: null }));
  }, []);

  return {
    resistanceLevel,
    isFreeRide,
    canAdjust,
    isLoading: actionState.isLoading,
    error: actionState.error,
    increaseResistance,
    decreaseResistance,
    adjustResistance,
    clearError,
    stepSize,
    canIncrease,
    canDecrease,
  };
}
