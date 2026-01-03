/**
 * useIntervalChangeHaptics Hook
 *
 * Monitors workout interval changes and triggers haptic feedback when
 * the user transitions to a new interval. This provides tactile feedback
 * during structured workouts to alert the user of interval changes.
 *
 * Features:
 * - Detects interval index changes
 * - Triggers notification haptic on interval change
 * - Respects user's haptic feedback settings
 * - Only triggers during active workout sessions
 */

import { useEffect, useRef } from 'react';
import { useSessionStore, selectCurrentInterval, selectIsSessionActive, selectIsWorkout } from '@/stores/sessionStore';
import { useHaptics } from './useHaptics';

/**
 * Options for the interval change haptics hook
 */
export interface UseIntervalChangeHapticsOptions {
  /** Whether the hook is enabled (defaults to true) */
  enabled?: boolean;
}

/**
 * Return type for the hook
 */
export interface UseIntervalChangeHapticsReturn {
  /** Current interval index (null if no workout active) */
  currentIntervalIndex: number | null;
  /** Whether interval change haptics are enabled */
  isEnabled: boolean;
}

/**
 * useIntervalChangeHaptics
 *
 * Monitors workout intervals and triggers haptic feedback when the interval
 * changes. This helps users stay aware of their workout progress without
 * looking at the screen.
 *
 * @param options Configuration options
 * @returns Current interval state and enabled status
 *
 * @example
 * ```tsx
 * function WorkoutScreen() {
 *   // Enable interval change haptics on this screen
 *   useIntervalChangeHaptics();
 *
 *   return <View>...</View>;
 * }
 * ```
 */
export function useIntervalChangeHaptics(
  options: UseIntervalChangeHapticsOptions = {}
): UseIntervalChangeHapticsReturn {
  const { enabled = true } = options;

  // Get session state
  const currentInterval = useSessionStore(selectCurrentInterval);
  const isSessionActive = useSessionStore(selectIsSessionActive);
  const isWorkout = useSessionStore(selectIsWorkout);

  // Get haptic feedback functions
  const { triggerHaptic, isHapticEnabled } = useHaptics();

  // Track previous interval index to detect changes
  const previousIntervalRef = useRef<number | null>(null);
  const isInitializedRef = useRef(false);

  // Determine if we should track interval changes
  const shouldTrack = enabled && isSessionActive && isWorkout && isHapticEnabled;
  const currentIntervalIndex = currentInterval?.index ?? null;

  useEffect(() => {
    // Skip if not tracking
    if (!shouldTrack) {
      // Reset initialization when session ends
      if (!isSessionActive) {
        isInitializedRef.current = false;
        previousIntervalRef.current = null;
      }
      return;
    }

    // Skip if interval index is null
    if (currentIntervalIndex === null) {
      return;
    }

    // On first mount/initialization, don't trigger haptic
    // Just record the initial interval
    if (!isInitializedRef.current) {
      previousIntervalRef.current = currentIntervalIndex;
      isInitializedRef.current = true;
      return;
    }

    // Check if interval has changed
    const previousIndex = previousIntervalRef.current;
    if (previousIndex !== null && previousIndex !== currentIntervalIndex) {
      // Interval changed! Trigger haptic feedback
      triggerHaptic('notification');
    }

    // Update ref for next comparison
    previousIntervalRef.current = currentIntervalIndex;
  }, [shouldTrack, currentIntervalIndex, triggerHaptic, isSessionActive]);

  return {
    currentIntervalIndex,
    isEnabled: shouldTrack,
  };
}
