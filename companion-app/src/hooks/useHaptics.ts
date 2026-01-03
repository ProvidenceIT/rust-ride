/**
 * useHaptics Hook
 *
 * Provides haptic feedback functionality based on user settings.
 * Uses the React Native Vibration API with intensity-based patterns.
 */

import { useCallback } from 'react';
import { Vibration, Platform } from 'react-native';
import { useSettingsStore, selectHapticFeedback } from '@/stores/settingsStore';
import type { HapticIntensity } from '@/stores/settingsStore';

/**
 * Haptic feedback types for different interactions
 */
export type HapticFeedbackType =
  | 'selection' // Light tap for selection/toggle
  | 'impact' // Medium impact for button press
  | 'notification' // Notification feedback
  | 'success' // Success confirmation
  | 'warning' // Warning feedback
  | 'error'; // Error feedback

/**
 * Vibration patterns for different feedback types and intensities
 * On iOS, duration values are ignored; on Android, they define vibration length in ms
 */
const VIBRATION_PATTERNS: Record<HapticFeedbackType, Record<Exclude<HapticIntensity, 'off'>, number>> = {
  selection: {
    light: 5,
    medium: 10,
    strong: 20,
  },
  impact: {
    light: 10,
    medium: 25,
    strong: 50,
  },
  notification: {
    light: 15,
    medium: 30,
    strong: 60,
  },
  success: {
    light: 10,
    medium: 20,
    strong: 40,
  },
  warning: {
    light: 20,
    medium: 40,
    strong: 80,
  },
  error: {
    light: 30,
    medium: 60,
    strong: 100,
  },
};

/**
 * useHaptics hook
 *
 * Returns functions for triggering haptic feedback based on user's intensity setting.
 * Respects the haptic feedback preference from settings store.
 *
 * @example
 * ```tsx
 * const { triggerHaptic, selectionHaptic, impactHaptic } = useHaptics();
 *
 * // Trigger with specific type
 * onPress={() => {
 *   impactHaptic();
 *   handleAction();
 * }}
 * ```
 */
export function useHaptics() {
  const hapticIntensity = useSettingsStore(selectHapticFeedback);

  /**
   * Trigger haptic feedback with a specific type
   */
  const triggerHaptic = useCallback(
    (type: HapticFeedbackType = 'impact') => {
      if (hapticIntensity === 'off') {
        return;
      }

      const duration = VIBRATION_PATTERNS[type][hapticIntensity];

      // Use Vibration API
      // On iOS with pattern, this will use the haptic engine
      // On Android, this uses the vibration motor
      if (Platform.OS === 'ios') {
        // iOS: Use selection feedback for light interactions
        // For now, use Vibration which works on both platforms
        Vibration.vibrate(duration);
      } else {
        Vibration.vibrate(duration);
      }
    },
    [hapticIntensity],
  );

  /**
   * Light tap feedback for selection/toggle actions
   */
  const selectionHaptic = useCallback(() => {
    triggerHaptic('selection');
  }, [triggerHaptic]);

  /**
   * Medium impact feedback for button presses
   */
  const impactHaptic = useCallback(() => {
    triggerHaptic('impact');
  }, [triggerHaptic]);

  /**
   * Success feedback for completed actions
   */
  const successHaptic = useCallback(() => {
    triggerHaptic('success');
  }, [triggerHaptic]);

  /**
   * Warning feedback for attention-required actions
   */
  const warningHaptic = useCallback(() => {
    triggerHaptic('warning');
  }, [triggerHaptic]);

  /**
   * Error feedback for failed actions
   */
  const errorHaptic = useCallback(() => {
    triggerHaptic('error');
  }, [triggerHaptic]);

  /**
   * Check if haptic feedback is enabled
   */
  const isHapticEnabled = hapticIntensity !== 'off';

  return {
    triggerHaptic,
    selectionHaptic,
    impactHaptic,
    successHaptic,
    warningHaptic,
    errorHaptic,
    isHapticEnabled,
    hapticIntensity,
  };
}

/**
 * Trigger a one-off haptic feedback without using the hook
 * Useful in callbacks where you can't use hooks
 *
 * @param type The type of haptic feedback
 * @param intensity The intensity level (defaults to 'medium' if not provided)
 */
export function triggerHapticFeedback(
  type: HapticFeedbackType = 'impact',
  intensity: HapticIntensity = 'medium',
): void {
  if (intensity === 'off') {
    return;
  }

  const duration = VIBRATION_PATTERNS[type][intensity];
  Vibration.vibrate(duration);
}
