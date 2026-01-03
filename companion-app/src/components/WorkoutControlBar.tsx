/**
 * WorkoutControlBar Component
 *
 * A fixed bottom bar with workout control buttons: play/pause, skip interval, and stop.
 * Provides haptic feedback on button press and disabled states when no session is active.
 *
 * Features:
 * - Play/Pause toggle button based on current session state
 * - Skip interval button (only enabled during workouts with intervals)
 * - Stop button with confirmation (see T033 for confirmation implementation)
 * - Disabled state when no active session
 * - Haptic feedback on press based on user settings
 * - Accessible with proper ARIA labels
 */

import React, { useCallback } from 'react';
import { View, StyleSheet, ViewStyle } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { shadows } from '@/theme/spacing';
import { IconButton, type IconButtonVariant } from './IconButton';
import { useHaptics } from '@/hooks/useHaptics';
import {
  useSessionStore,
  selectIsSessionActive,
  selectIsPaused,
  selectCanPause,
  selectCanResume,
  selectCanSkip,
  selectCanStop,
  selectIsWorkout,
} from '@/stores/sessionStore';

/**
 * WorkoutControlBar props
 */
export interface WorkoutControlBarProps {
  /** Callback when pause is pressed */
  onPause?: () => void;
  /** Callback when resume is pressed */
  onResume?: () => void;
  /** Callback when skip is pressed */
  onSkip?: () => void;
  /** Callback when stop is pressed */
  onStop?: () => void;
  /** Whether the pause action is loading */
  isPauseLoading?: boolean;
  /** Whether the skip action is loading */
  isSkipLoading?: boolean;
  /** Whether the stop action is loading */
  isStopLoading?: boolean;
  /** Custom style for the container */
  style?: ViewStyle;
  /** Test ID for testing */
  testID?: string;
}

/**
 * WorkoutControlBar Component
 *
 * A fixed bottom control bar for workout actions. The bar appears at the bottom
 * of the screen with buttons for pause/resume, skip interval, and stop.
 *
 * The buttons are automatically enabled/disabled based on the current session state:
 * - Pause: Enabled when session is active and not paused
 * - Resume: Enabled when session is active and paused
 * - Skip: Enabled during workouts when not on the last interval
 * - Stop: Enabled when any session is active
 *
 * @example
 * ```tsx
 * <WorkoutControlBar
 *   onPause={handlePause}
 *   onResume={handleResume}
 *   onSkip={handleSkip}
 *   onStop={handleStop}
 * />
 * ```
 */
export function WorkoutControlBar({
  onPause,
  onResume,
  onSkip,
  onStop,
  isPauseLoading = false,
  isSkipLoading = false,
  isStopLoading = false,
  style,
  testID,
}: WorkoutControlBarProps): React.JSX.Element {
  const { colors, spacing } = useTheme();
  const insets = useSafeAreaInsets();
  const { impactHaptic, warningHaptic } = useHaptics();

  // Session state selectors
  const isSessionActive = useSessionStore(selectIsSessionActive);
  const isPaused = useSessionStore(selectIsPaused);
  const canPause = useSessionStore(selectCanPause);
  const canResume = useSessionStore(selectCanResume);
  const canSkip = useSessionStore(selectCanSkip);
  const canStop = useSessionStore(selectCanStop);
  const isWorkout = useSessionStore(selectIsWorkout);

  // Determine if controls should be globally disabled (no active session)
  const isDisabled = !isSessionActive;

  // Handle play/pause button press
  const handlePlayPausePress = useCallback(() => {
    impactHaptic();

    if (isPaused && onResume) {
      onResume();
    } else if (!isPaused && onPause) {
      onPause();
    }
  }, [isPaused, onPause, onResume, impactHaptic]);

  // Handle skip button press
  const handleSkipPress = useCallback(() => {
    impactHaptic();
    onSkip?.();
  }, [onSkip, impactHaptic]);

  // Handle stop button press
  const handleStopPress = useCallback(() => {
    warningHaptic();
    onStop?.();
  }, [onStop, warningHaptic]);

  // Determine play/pause button state
  const playPauseIcon = isPaused ? 'play' : 'pause';
  const playPauseLabel = isPaused ? 'Resume workout' : 'Pause workout';
  const playPauseVariant: IconButtonVariant = 'primary';
  const isPlayPauseDisabled = isDisabled || (isPaused ? !canResume : !canPause);

  // Skip button is only enabled during workouts with remaining intervals
  const isSkipDisabled = isDisabled || !isWorkout || !canSkip;
  const skipVariant: IconButtonVariant = 'default';

  // Stop button is enabled whenever there's an active session
  const isStopDisabled = isDisabled || !canStop;
  const stopVariant: IconButtonVariant = 'danger';

  // Container style with safe area padding
  const containerStyle: ViewStyle = {
    paddingBottom: Math.max(insets.bottom, spacing.md),
  };

  return (
    <View
      style={[
        styles.container,
        { backgroundColor: colors.surface, borderTopColor: colors.border },
        shadows.md,
        containerStyle,
        style,
      ]}
      testID={testID}
      accessibilityRole="toolbar"
      accessibilityLabel="Workout controls"
    >
      <View style={[styles.buttonContainer, { gap: spacing.lg }]}>
        {/* Play/Pause Button */}
        <IconButton
          icon={<Icon name={playPauseIcon} />}
          variant={playPauseVariant}
          size="large"
          circular
          onPress={handlePlayPausePress}
          disabled={isPlayPauseDisabled}
          loading={isPauseLoading}
          accessibilityLabel={playPauseLabel}
          accessibilityHint={
            isPlayPauseDisabled
              ? 'Button is disabled. Start a session from the desktop app.'
              : undefined
          }
          testID={`${testID}-play-pause`}
        />

        {/* Skip Interval Button */}
        <IconButton
          icon={<Icon name="play-skip-forward" />}
          variant={skipVariant}
          size="large"
          circular
          onPress={handleSkipPress}
          disabled={isSkipDisabled}
          loading={isSkipLoading}
          accessibilityLabel="Skip to next interval"
          accessibilityHint={
            isSkipDisabled
              ? isWorkout
                ? 'Cannot skip. You are on the last interval.'
                : 'Skip is only available during structured workouts.'
              : 'Skip to the next interval in the workout.'
          }
          testID={`${testID}-skip`}
        />

        {/* Stop Button */}
        <IconButton
          icon={<Icon name="stop" />}
          variant={stopVariant}
          size="large"
          circular
          onPress={handleStopPress}
          disabled={isStopDisabled}
          loading={isStopLoading}
          accessibilityLabel="Stop session"
          accessibilityHint={
            isStopDisabled
              ? 'No active session to stop.'
              : 'Stop the current workout or free ride session.'
          }
          testID={`${testID}-stop`}
        />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    borderTopWidth: 1,
    paddingTop: 16,
    paddingHorizontal: 24,
  },
  buttonContainer: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
  },
});
