/**
 * WorkoutIntervalDisplay Component
 *
 * Displays workout interval information including current interval name,
 * time remaining, progress bar, and next interval preview.
 * Only shown when a structured workout is active.
 */

import React, { useMemo } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
  AccessibilityProps,
} from 'react-native';
import { useTheme } from '@/theme';
import { zoneColors } from '@/theme/colors';

/**
 * Interval information
 */
export interface IntervalInfo {
  /** Current interval index (0-based) */
  index: number;
  /** Total number of intervals */
  total: number;
  /** Current interval name/description */
  name: string | null;
  /** Seconds remaining in current interval */
  remainingSecs: number | null;
}

/**
 * Next interval preview info
 */
export interface NextIntervalInfo {
  /** Name of the next interval */
  name: string;
  /** Target power in watts for next interval */
  targetPower?: number;
  /** Duration of next interval in seconds */
  durationSecs?: number;
}

/**
 * WorkoutIntervalDisplay props
 */
export interface WorkoutIntervalDisplayProps extends AccessibilityProps {
  /** Current interval information */
  currentInterval: IntervalInfo;
  /** Next interval preview (optional) */
  nextInterval?: NextIntervalInfo | null;
  /** Current target power in watts (used for color coding) */
  targetPower?: number | null;
  /** User's FTP for zone calculation */
  ftp?: number;
  /** Whether the workout is paused */
  isPaused?: boolean;
  /** Whether to show the component (hides when no workout active) */
  showMetrics?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * Format seconds into MM:SS or HH:MM:SS format
 */
function formatTime(totalSeconds: number | null): string {
  if (totalSeconds === null || totalSeconds < 0) {
    return '--:--';
  }

  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

/**
 * Get zone color based on target power and FTP
 */
function getZoneColor(targetPower: number | null | undefined, ftp: number): string {
  if (targetPower == null || ftp <= 0) {
    return zoneColors.z2Endurance;
  }

  const percentFtp = (targetPower / ftp) * 100;

  if (percentFtp < 56) {
    return zoneColors.z1Recovery;
  }
  if (percentFtp < 76) {
    return zoneColors.z2Endurance;
  }
  if (percentFtp < 91) {
    return zoneColors.z3Tempo;
  }
  if (percentFtp < 106) {
    return zoneColors.z4Threshold;
  }
  if (percentFtp < 121) {
    return zoneColors.z5Vo2max;
  }
  if (percentFtp < 151) {
    return zoneColors.z6Anaerobic;
  }
  return zoneColors.z7Neuromuscular;
}

/**
 * WorkoutIntervalDisplay Component
 *
 * Shows current workout interval information with visual progress tracking.
 * Features:
 * - Current interval name and number (e.g., "Interval 3 of 8")
 * - Time remaining countdown
 * - Visual progress bar for interval completion
 * - Next interval preview with name and target power
 * - Color coding based on intensity zone
 *
 * @example
 * ```tsx
 * <WorkoutIntervalDisplay
 *   currentInterval={{
 *     index: 2,
 *     total: 8,
 *     name: "Threshold Effort",
 *     remainingSecs: 180
 *   }}
 *   nextInterval={{
 *     name: "Recovery",
 *     targetPower: 120
 *   }}
 *   targetPower={275}
 *   ftp={250}
 *   showMetrics={true}
 * />
 * ```
 */
export function WorkoutIntervalDisplay({
  currentInterval,
  nextInterval,
  targetPower,
  ftp = 200,
  isPaused = false,
  showMetrics = true,
  style,
  ...accessibilityProps
}: WorkoutIntervalDisplayProps): React.JSX.Element | null {
  const { colors, spacing, typography, borderRadius } = useTheme();

  // Get zone color for the current interval
  const zoneColor = useMemo(
    () => getZoneColor(targetPower, ftp),
    [targetPower, ftp]
  );

  // Calculate progress percentage (estimate based on typical interval duration)
  // If we had total interval duration, we could calculate exact progress
  // For now, show progress as interval index / total
  const overallProgress = useMemo(() => {
    if (currentInterval.total === 0) {
      return 0;
    }
    return ((currentInterval.index + 1) / currentInterval.total) * 100;
  }, [currentInterval.index, currentInterval.total]);

  // Format time remaining
  const timeRemaining = useMemo(
    () => formatTime(currentInterval.remainingSecs),
    [currentInterval.remainingSecs]
  );

  // Interval display text
  const intervalText = useMemo(() => {
    return `Interval ${currentInterval.index + 1} of ${currentInterval.total}`;
  }, [currentInterval.index, currentInterval.total]);

  // Interval name with fallback
  const intervalName = currentInterval.name || 'Interval';

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `${intervalName}, ${intervalText}, ` +
    `${currentInterval.remainingSecs !== null ? `${timeRemaining} remaining` : 'time not available'}` +
    (nextInterval ? `, Next: ${nextInterval.name}` : '') +
    (isPaused ? ', Workout paused' : '');

  // Don't render if no metrics to show
  if (!showMetrics) {
    return null;
  }

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.lg,
          padding: spacing.md,
          borderLeftWidth: 5,
          borderLeftColor: zoneColor,
        },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Header with interval counter */}
      <View style={styles.header}>
        <Text
          style={[
            styles.intervalCounter,
            typography.textStyles.metricLabel,
            { color: colors.textMuted },
          ]}
        >
          {intervalText.toUpperCase()}
        </Text>
        {isPaused && (
          <View
            style={[
              styles.pausedBadge,
              { backgroundColor: colors.warning },
            ]}
          >
            <Text style={styles.pausedBadgeText}>PAUSED</Text>
          </View>
        )}
      </View>

      {/* Current interval name */}
      <Text
        style={[
          styles.intervalName,
          typography.textStyles.sectionTitle,
          { color: colors.textPrimary },
        ]}
        numberOfLines={1}
        adjustsFontSizeToFit
      >
        {intervalName}
      </Text>

      {/* Time remaining */}
      <View style={styles.timeContainer}>
        <Text
          style={[
            styles.timeRemaining,
            { color: zoneColor },
          ]}
        >
          {timeRemaining}
        </Text>
        <Text
          style={[
            styles.timeLabel,
            { color: colors.textMuted },
          ]}
        >
          {' '}remaining
        </Text>
      </View>

      {/* Progress bar */}
      <View
        style={[
          styles.progressBarContainer,
          {
            backgroundColor: colors.border,
            borderRadius: borderRadius.sm,
            marginTop: spacing.sm,
          },
        ]}
      >
        <View
          style={[
            styles.progressBarFill,
            {
              backgroundColor: zoneColor,
              borderRadius: borderRadius.sm,
              width: `${overallProgress}%`,
            },
          ]}
        />
      </View>

      {/* Next interval preview */}
      {nextInterval && (
        <View
          style={[
            styles.nextIntervalContainer,
            {
              borderTopWidth: StyleSheet.hairlineWidth,
              borderTopColor: colors.border,
              marginTop: spacing.md,
              paddingTop: spacing.md,
            },
          ]}
        >
          <View style={styles.nextIntervalRow}>
            <View style={styles.nextLabelContainer}>
              <Text
                style={[
                  styles.nextLabel,
                  typography.textStyles.metricLabel,
                  { color: colors.textMuted },
                ]}
              >
                NEXT
              </Text>
            </View>
            <View style={styles.nextInfoContainer}>
              <Text
                style={[
                  styles.nextIntervalName,
                  { color: colors.textSecondary },
                ]}
                numberOfLines={1}
              >
                {nextInterval.name}
              </Text>
              {nextInterval.targetPower != null && (
                <View
                  style={[
                    styles.nextPowerBadge,
                    { backgroundColor: getZoneColor(nextInterval.targetPower, ftp) },
                  ]}
                >
                  <Text style={styles.nextPowerText}>
                    {nextInterval.targetPower}W
                  </Text>
                </View>
              )}
            </View>
          </View>
          {nextInterval.durationSecs != null && (
            <Text
              style={[
                styles.nextDuration,
                { color: colors.textMuted },
              ]}
            >
              {formatTime(nextInterval.durationSecs)}
            </Text>
          )}
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    minHeight: 120,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 4,
  },
  intervalCounter: {
    letterSpacing: 0.5,
  },
  pausedBadge: {
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 8,
  },
  pausedBadgeText: {
    fontSize: 10,
    fontWeight: '700',
    color: '#000000',
    letterSpacing: 0.5,
  },
  intervalName: {
    marginBottom: 8,
  },
  timeContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  timeRemaining: {
    fontSize: 36,
    fontWeight: '700',
    fontVariant: ['tabular-nums'],
    includeFontPadding: false,
  },
  timeLabel: {
    fontSize: 14,
    fontWeight: '400',
  },
  progressBarContainer: {
    height: 6,
    overflow: 'hidden',
  },
  progressBarFill: {
    height: '100%',
  },
  nextIntervalContainer: {
    // Dynamic styling applied inline
  },
  nextIntervalRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  nextLabelContainer: {
    // Layout handled by flex
  },
  nextLabel: {
    // Typography from theme
  },
  nextInfoContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
    justifyContent: 'flex-end',
    gap: 8,
  },
  nextIntervalName: {
    fontSize: 14,
    fontWeight: '500',
  },
  nextPowerBadge: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 8,
  },
  nextPowerText: {
    fontSize: 12,
    fontWeight: '700',
    color: '#FFFFFF',
    fontVariant: ['tabular-nums'],
  },
  nextDuration: {
    fontSize: 12,
    fontWeight: '400',
    marginTop: 4,
    textAlign: 'right',
  },
});
