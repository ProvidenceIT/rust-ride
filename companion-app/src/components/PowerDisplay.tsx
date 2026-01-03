/**
 * PowerDisplay Component
 *
 * A specialized large power display for the dashboard.
 * Shows current power, 3-second average, power zone indicator,
 * and target power during structured workouts.
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
import type { PowerZone } from '@/stores/metricsStore';

/**
 * Power zone display info
 */
interface ZoneDisplayInfo {
  label: string;
  shortLabel: string;
  color: string;
}

/**
 * Get display info for a power zone
 */
function getZoneDisplayInfo(zone: PowerZone): ZoneDisplayInfo {
  const zoneInfo: Record<PowerZone, ZoneDisplayInfo> = {
    recovery: {
      label: 'Recovery',
      shortLabel: 'Z1',
      color: zoneColors.z1Recovery,
    },
    endurance: {
      label: 'Endurance',
      shortLabel: 'Z2',
      color: zoneColors.z2Endurance,
    },
    tempo: {
      label: 'Tempo',
      shortLabel: 'Z3',
      color: zoneColors.z3Tempo,
    },
    threshold: {
      label: 'Threshold',
      shortLabel: 'Z4',
      color: zoneColors.z4Threshold,
    },
    vo2max: {
      label: 'VO2max',
      shortLabel: 'Z5',
      color: zoneColors.z5Vo2max,
    },
    anaerobic: {
      label: 'Anaerobic',
      shortLabel: 'Z6',
      color: zoneColors.z6Anaerobic,
    },
    neuromuscular: {
      label: 'Neuromuscular',
      shortLabel: 'Z7',
      color: zoneColors.z7Neuromuscular,
    },
  };
  return zoneInfo[zone];
}

/**
 * PowerDisplay props
 */
export interface PowerDisplayProps extends AccessibilityProps {
  /** Current power in watts */
  power: number;
  /** 3-second average power */
  power3sAvg: number;
  /** Current power zone */
  powerZone: PowerZone;
  /** Target power for structured workouts (ERG mode) */
  targetPower?: number | null;
  /** Whether metrics are available to display */
  showMetrics?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * PowerDisplay Component
 *
 * A large, prominent power display showing current watts, 3-second average,
 * and power zone with color coding. Displays target power during workouts.
 *
 * @example
 * ```tsx
 * <PowerDisplay
 *   power={285}
 *   power3sAvg={280}
 *   powerZone="threshold"
 *   targetPower={275}
 *   showMetrics={true}
 * />
 * ```
 */
export function PowerDisplay({
  power,
  power3sAvg,
  powerZone,
  targetPower,
  showMetrics = true,
  style,
  ...accessibilityProps
}: PowerDisplayProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  // Get zone display info
  const zoneInfo = useMemo(() => getZoneDisplayInfo(powerZone), [powerZone]);

  // Calculate power difference from target
  const powerDiff = useMemo(() => {
    if (targetPower == null || !showMetrics) {
      return null;
    }
    return power - targetPower;
  }, [power, targetPower, showMetrics]);

  // Determine if power is within acceptable range of target (±5%)
  const isOnTarget = useMemo(() => {
    if (targetPower == null || !showMetrics) {
      return null;
    }
    const tolerance = targetPower * 0.05;
    return Math.abs(power - targetPower) <= tolerance;
  }, [power, targetPower, showMetrics]);

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `Power: ${showMetrics ? power : 'no data'} watts, ` +
      `3 second average: ${showMetrics ? power3sAvg : 'no data'} watts, ` +
      `Zone: ${showMetrics ? zoneInfo.label : 'none'}` +
      (targetPower != null ? `, Target: ${targetPower} watts` : '');

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.lg,
          padding: spacing.lg,
          borderLeftWidth: showMetrics && power > 0 ? 5 : 0,
          borderLeftColor: showMetrics && power > 0 ? zoneInfo.color : 'transparent',
        },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Header with label and zone indicator */}
      <View style={styles.header}>
        <Text
          style={[
            styles.label,
            typography.textStyles.metricLabel,
            { color: colors.textMuted },
          ]}
        >
          POWER
        </Text>
        {showMetrics && power > 0 && (
          <View
            style={[
              styles.zoneBadge,
              { backgroundColor: zoneInfo.color },
            ]}
          >
            <Text style={[styles.zoneBadgeText, { color: '#FFFFFF' }]}>
              {zoneInfo.shortLabel} {zoneInfo.label}
            </Text>
          </View>
        )}
      </View>

      {/* Main power value */}
      <View style={styles.mainValueRow}>
        <Text
          style={[
            styles.mainValue,
            typography.textStyles.metricPrimary,
            { color: showMetrics && power > 0 ? zoneInfo.color : colors.textPrimary },
          ]}
          numberOfLines={1}
          adjustsFontSizeToFit
        >
          {showMetrics ? power : '--'}
        </Text>
        <Text
          style={[
            styles.unit,
            typography.textStyles.metricUnit,
            { color: colors.textSecondary },
          ]}
        >
          W
        </Text>
      </View>

      {/* 3-second average */}
      <View style={styles.secondaryRow}>
        <View style={styles.avgContainer}>
          <Text
            style={[
              styles.avgValue,
              { color: colors.textSecondary },
            ]}
          >
            {showMetrics && power3sAvg > 0 ? power3sAvg : '--'}
          </Text>
          <Text
            style={[
              styles.avgLabel,
              { color: colors.textMuted },
            ]}
          >
            {' '}W (3s avg)
          </Text>
        </View>
      </View>

      {/* Target power overlay (for ERG mode workouts) */}
      {targetPower != null && showMetrics && (
        <View
          style={[
            styles.targetContainer,
            {
              borderTopWidth: StyleSheet.hairlineWidth,
              borderTopColor: colors.border,
              marginTop: spacing.md,
              paddingTop: spacing.md,
            },
          ]}
        >
          <View style={styles.targetRow}>
            <View style={styles.targetLabelContainer}>
              <Text
                style={[
                  styles.targetLabel,
                  typography.textStyles.metricLabel,
                  { color: colors.textMuted },
                ]}
              >
                TARGET
              </Text>
            </View>
            <View style={styles.targetValueContainer}>
              <Text
                style={[
                  styles.targetValue,
                  {
                    color: isOnTarget ? colors.success : colors.textSecondary,
                  },
                ]}
              >
                {targetPower}
              </Text>
              <Text
                style={[
                  styles.targetUnit,
                  { color: colors.textMuted },
                ]}
              >
                {' '}W
              </Text>
            </View>
          </View>

          {/* Power difference indicator */}
          {powerDiff !== null && (
            <View style={styles.diffRow}>
              <View
                style={[
                  styles.diffBadge,
                  {
                    backgroundColor: isOnTarget
                      ? colors.success
                      : powerDiff > 0
                        ? colors.warning
                        : colors.error,
                  },
                ]}
              >
                <Text style={styles.diffText}>
                  {powerDiff >= 0 ? '+' : ''}{powerDiff}W
                </Text>
              </View>
              <Text
                style={[
                  styles.diffHint,
                  { color: colors.textMuted },
                ]}
              >
                {isOnTarget
                  ? 'On target'
                  : powerDiff > 0
                    ? 'Reduce power'
                    : 'Increase power'}
              </Text>
            </View>
          )}
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    minHeight: 180,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 4,
  },
  label: {
    // Typography from theme
  },
  zoneBadge: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
  },
  zoneBadgeText: {
    fontSize: 11,
    fontWeight: '600',
    letterSpacing: 0.5,
    textTransform: 'uppercase',
  },
  mainValueRow: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  mainValue: {
    includeFontPadding: false,
  },
  unit: {
    marginLeft: 6,
    alignSelf: 'flex-end',
    marginBottom: 8,
  },
  secondaryRow: {
    marginTop: 4,
  },
  avgContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  avgValue: {
    fontSize: 18,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  avgLabel: {
    fontSize: 13,
    fontWeight: '400',
  },
  targetContainer: {
    // Dynamic styling applied inline
  },
  targetRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  targetLabelContainer: {
    // Layout handled by flex
  },
  targetLabel: {
    // Typography from theme
  },
  targetValueContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  targetValue: {
    fontSize: 24,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  targetUnit: {
    fontSize: 12,
    fontWeight: '400',
  },
  diffRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 8,
  },
  diffBadge: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 8,
    marginRight: 8,
  },
  diffText: {
    fontSize: 12,
    fontWeight: '700',
    color: '#FFFFFF',
    fontVariant: ['tabular-nums'],
  },
  diffHint: {
    fontSize: 12,
    fontWeight: '500',
  },
});
