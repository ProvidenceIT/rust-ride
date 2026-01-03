/**
 * ResistanceControl Component
 *
 * Displays +/- buttons to adjust trainer resistance or simulated grade for free rides.
 * Shows the current resistance level as a percentage.
 *
 * Features:
 * - Increase/decrease buttons with configurable step size
 * - Current resistance level display (-100% to +100%)
 * - Only shown during free rides (not structured workouts)
 * - Haptic feedback on button press
 * - Loading state while adjustment is in progress
 * - Accessible with proper ARIA labels
 */

import React, { useCallback } from 'react';
import { View, Text, StyleSheet, ViewStyle, TextStyle } from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { IconButton } from './IconButton';
import { useHaptics } from '@/hooks/useHaptics';

/**
 * ResistanceControl props
 */
export interface ResistanceControlProps {
  /** Current resistance level (-100 to 100) */
  resistanceLevel: number;
  /** Whether resistance can be adjusted */
  canAdjust: boolean;
  /** Whether resistance can be increased (not at max) */
  canIncrease: boolean;
  /** Whether resistance can be decreased (not at min) */
  canDecrease: boolean;
  /** Whether an adjustment is currently in progress */
  isLoading?: boolean;
  /** Callback when increase button is pressed */
  onIncrease: () => void;
  /** Callback when decrease button is pressed */
  onDecrease: () => void;
  /** Step size for display purposes */
  stepSize?: number;
  /** Custom container style */
  style?: ViewStyle;
  /** Test ID for testing */
  testID?: string;
}

/**
 * Format resistance level for display
 * Shows +/- sign and percentage symbol
 */
function formatResistanceLevel(level: number): string {
  if (level === 0) {
    return '0%';
  }
  const sign = level > 0 ? '+' : '';
  return `${sign}${level}%`;
}

/**
 * Get color based on resistance level
 * Positive = incline (orange/red), Negative = decline (green), Zero = neutral
 */
function getResistanceColor(level: number, colors: ReturnType<typeof useTheme>['colors']): string {
  if (level > 0) {
    // Incline/resistance - orange to red based on intensity
    if (level >= 50) {
      return colors.error; // High resistance
    }
    return colors.warning; // Moderate resistance
  } else if (level < 0) {
    // Decline/easier - green
    return colors.success;
  }
  // Neutral
  return colors.textPrimary;
}

/**
 * Get resistance description label
 */
function getResistanceLabel(level: number): string {
  if (level > 0) {
    return 'Resistance';
  } else if (level < 0) {
    return 'Assist';
  }
  return 'Flat';
}

/**
 * ResistanceControl Component
 *
 * A control component for adjusting trainer resistance or simulated grade
 * during free ride sessions. Displays the current level with +/- buttons.
 *
 * @example
 * ```tsx
 * const {
 *   resistanceLevel,
 *   canIncrease,
 *   canDecrease,
 *   increaseResistance,
 *   decreaseResistance,
 *   isLoading,
 * } = useResistanceControl();
 *
 * <ResistanceControl
 *   resistanceLevel={resistanceLevel}
 *   canAdjust={true}
 *   canIncrease={canIncrease}
 *   canDecrease={canDecrease}
 *   isLoading={isLoading}
 *   onIncrease={increaseResistance}
 *   onDecrease={decreaseResistance}
 * />
 * ```
 */
export function ResistanceControl({
  resistanceLevel,
  canAdjust,
  canIncrease,
  canDecrease,
  isLoading = false,
  onIncrease,
  onDecrease,
  stepSize = 5,
  style,
  testID,
}: ResistanceControlProps): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();
  const { impactHaptic } = useHaptics();

  // Get colors based on current level
  const levelColor = getResistanceColor(resistanceLevel, colors);
  const levelLabel = getResistanceLabel(resistanceLevel);

  // Handle button presses with haptic feedback
  const handleIncrease = useCallback(() => {
    if (canIncrease && !isLoading) {
      impactHaptic();
      onIncrease();
    }
  }, [canIncrease, isLoading, impactHaptic, onIncrease]);

  const handleDecrease = useCallback(() => {
    if (canDecrease && !isLoading) {
      impactHaptic();
      onDecrease();
    }
  }, [canDecrease, isLoading, impactHaptic, onDecrease]);

  // Dynamic styles
  const containerStyle: ViewStyle = {
    backgroundColor: colors.surface,
    padding: spacing.md,
    borderRadius: 12,
  };

  const labelStyle: TextStyle = {
    ...typography.textStyles.label,
    color: colors.textSecondary,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: spacing.sm,
    textAlign: 'center',
  };

  const levelTextStyle: TextStyle = {
    fontSize: 48,
    fontWeight: '600',
    color: levelColor,
    fontVariant: ['tabular-nums'],
    textAlign: 'center',
  };

  const sublabelStyle: TextStyle = {
    ...typography.textStyles.caption,
    color: colors.textMuted,
    textAlign: 'center',
    marginTop: spacing.xs,
  };

  const stepHintStyle: TextStyle = {
    ...typography.textStyles.caption,
    color: colors.textMuted,
    textAlign: 'center',
    marginTop: spacing.sm,
  };

  return (
    <View
      style={[containerStyle, style]}
      testID={testID}
      accessibilityRole="adjustable"
      accessibilityLabel={`Resistance level: ${formatResistanceLevel(resistanceLevel)}. ${levelLabel}`}
      accessibilityHint={
        canAdjust
          ? `Use plus and minus buttons to adjust resistance by ${stepSize}%`
          : 'Resistance adjustment is not available'
      }
      accessibilityValue={{
        min: -100,
        max: 100,
        now: resistanceLevel,
        text: formatResistanceLevel(resistanceLevel),
      }}
    >
      {/* Header label */}
      <Text style={labelStyle}>Resistance / Grade</Text>

      {/* Control row with buttons and value */}
      <View style={styles.controlRow}>
        {/* Decrease button */}
        <IconButton
          icon={<Icon name="remove" />}
          variant="default"
          size="large"
          circular
          onPress={handleDecrease}
          disabled={!canDecrease || isLoading}
          accessibilityLabel={`Decrease resistance by ${stepSize}%`}
          accessibilityHint={
            canDecrease
              ? `Current level is ${formatResistanceLevel(resistanceLevel)}`
              : 'Cannot decrease further. At minimum resistance.'
          }
          testID={`${testID}-decrease`}
        />

        {/* Current level display */}
        <View style={styles.levelContainer}>
          <Text
            style={levelTextStyle}
            accessibilityElementsHidden
            importantForAccessibility="no-hide-descendants"
          >
            {formatResistanceLevel(resistanceLevel)}
          </Text>
          <Text
            style={sublabelStyle}
            accessibilityElementsHidden
            importantForAccessibility="no-hide-descendants"
          >
            {levelLabel}
          </Text>
        </View>

        {/* Increase button */}
        <IconButton
          icon={<Icon name="add" />}
          variant="default"
          size="large"
          circular
          onPress={handleIncrease}
          disabled={!canIncrease || isLoading}
          accessibilityLabel={`Increase resistance by ${stepSize}%`}
          accessibilityHint={
            canIncrease
              ? `Current level is ${formatResistanceLevel(resistanceLevel)}`
              : 'Cannot increase further. At maximum resistance.'
          }
          testID={`${testID}-increase`}
        />
      </View>

      {/* Step size hint */}
      <Text
        style={stepHintStyle}
        accessibilityElementsHidden
        importantForAccessibility="no-hide-descendants"
      >
        Adjust by {stepSize}% per tap
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  controlRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 24,
  },
  levelContainer: {
    minWidth: 120,
    alignItems: 'center',
  },
});
