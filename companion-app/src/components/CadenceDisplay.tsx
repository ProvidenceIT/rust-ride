/**
 * CadenceDisplay Component
 *
 * A specialized cadence display for the dashboard.
 * Shows current cadence in RPM with target cadence during workouts.
 * Visual indicator when outside target range.
 */

import React, { useMemo, useEffect, useRef } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
  AccessibilityProps,
  Animated,
  Easing,
} from 'react-native';
import { useTheme } from '@/theme';

/**
 * Cadence status based on target range
 */
type CadenceStatus = 'low' | 'on_target' | 'high' | 'no_target';

/**
 * CadenceDisplay props
 */
export interface CadenceDisplayProps extends AccessibilityProps {
  /** Current cadence in RPM */
  cadence: number | null;
  /** Target cadence for structured workouts */
  targetCadence?: number | null;
  /** Tolerance range in RPM (default: 10) */
  tolerance?: number;
  /** Whether metrics are available to display */
  showMetrics?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * Get status color based on cadence status
 */
function getStatusColor(
  status: CadenceStatus,
  colors: { success: string; warning: string; error: string; accent: string }
): string {
  switch (status) {
    case 'on_target':
      return colors.success;
    case 'low':
      return colors.error;
    case 'high':
      return colors.warning;
    case 'no_target':
    default:
      return colors.accent;
  }
}

/**
 * Get status label text
 */
function getStatusLabel(status: CadenceStatus): string {
  switch (status) {
    case 'on_target':
      return 'On Target';
    case 'low':
      return 'Spin Faster';
    case 'high':
      return 'Slow Down';
    case 'no_target':
    default:
      return '';
  }
}

/**
 * CadenceDisplay Component
 *
 * A prominent cadence display showing current RPM and target cadence.
 * Displays visual indicator when outside acceptable range of target.
 *
 * @example
 * ```tsx
 * <CadenceDisplay
 *   cadence={90}
 *   targetCadence={85}
 *   showMetrics={true}
 * />
 * ```
 */
export function CadenceDisplay({
  cadence,
  targetCadence,
  tolerance = 10,
  showMetrics = true,
  style,
  ...accessibilityProps
}: CadenceDisplayProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  // Animation for warning indicator
  const pulseAnim = useRef(new Animated.Value(1)).current;

  // Calculate cadence status relative to target
  const cadenceStatus = useMemo((): CadenceStatus => {
    if (!showMetrics || cadence == null || targetCadence == null) {
      return 'no_target';
    }
    const diff = cadence - targetCadence;
    if (Math.abs(diff) <= tolerance) {
      return 'on_target';
    }
    return diff < 0 ? 'low' : 'high';
  }, [cadence, targetCadence, tolerance, showMetrics]);

  // Calculate difference from target
  const cadenceDiff = useMemo(() => {
    if (cadence == null || targetCadence == null || !showMetrics) {
      return null;
    }
    return cadence - targetCadence;
  }, [cadence, targetCadence, showMetrics]);

  // Status color based on target adherence
  const statusColor = useMemo(
    () => getStatusColor(cadenceStatus, colors),
    [cadenceStatus, colors]
  );

  // Animate warning indicator when out of range
  useEffect(() => {
    if (cadenceStatus === 'low' || cadenceStatus === 'high') {
      const pulseAnimation = Animated.loop(
        Animated.sequence([
          Animated.timing(pulseAnim, {
            toValue: 1.1,
            duration: 400,
            easing: Easing.out(Easing.ease),
            useNativeDriver: true,
          }),
          Animated.timing(pulseAnim, {
            toValue: 1,
            duration: 400,
            easing: Easing.in(Easing.ease),
            useNativeDriver: true,
          }),
        ])
      );
      pulseAnimation.start();
      return () => {
        pulseAnimation.stop();
        pulseAnim.setValue(1);
      };
    } else {
      pulseAnim.setValue(1);
      return undefined;
    }
  }, [cadenceStatus, pulseAnim]);

  // Determine border color
  const borderColor = useMemo(() => {
    if (!showMetrics || cadence == null || cadence === 0) {
      return 'transparent';
    }
    if (targetCadence != null && cadenceStatus !== 'no_target') {
      return statusColor;
    }
    return colors.accent;
  }, [showMetrics, cadence, targetCadence, cadenceStatus, statusColor, colors.accent]);

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `Cadence: ${showMetrics && cadence != null ? cadence : 'no data'} revolutions per minute` +
      (targetCadence != null ? `, Target: ${targetCadence} RPM` : '') +
      (cadenceStatus !== 'no_target' ? `, Status: ${getStatusLabel(cadenceStatus)}` : '');

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.lg,
          padding: spacing.lg,
          borderLeftWidth: showMetrics && cadence != null && cadence > 0 ? 5 : 0,
          borderLeftColor: borderColor,
        },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Header with label */}
      <View style={styles.header}>
        <Text
          style={[
            styles.label,
            typography.textStyles.metricLabel,
            { color: colors.textMuted },
          ]}
        >
          CADENCE
        </Text>
        {/* Status badge when target is set and out of range */}
        {showMetrics &&
          cadence != null &&
          cadence > 0 &&
          targetCadence != null &&
          cadenceStatus !== 'on_target' &&
          cadenceStatus !== 'no_target' && (
            <Animated.View
              style={[
                styles.statusBadge,
                {
                  backgroundColor: statusColor,
                  transform: [{ scale: pulseAnim }],
                },
              ]}
            >
              <Text style={[styles.statusBadgeText, { color: '#FFFFFF' }]}>
                {getStatusLabel(cadenceStatus)}
              </Text>
            </Animated.View>
          )}
        {/* On target badge */}
        {showMetrics &&
          cadence != null &&
          cadence > 0 &&
          cadenceStatus === 'on_target' && (
            <View
              style={[
                styles.statusBadge,
                { backgroundColor: colors.success },
              ]}
            >
              <Text style={[styles.statusBadgeText, { color: '#FFFFFF' }]}>
                On Target
              </Text>
            </View>
          )}
      </View>

      {/* Main cadence value */}
      <View style={styles.mainValueRow}>
        <Text
          style={[
            styles.mainValue,
            typography.textStyles.metricPrimary,
            {
              color:
                showMetrics && cadence != null && cadence > 0
                  ? targetCadence != null
                    ? statusColor
                    : colors.accent
                  : colors.textPrimary,
            },
          ]}
          numberOfLines={1}
          adjustsFontSizeToFit
        >
          {showMetrics && cadence != null ? cadence : '--'}
        </Text>
        <Text
          style={[
            styles.unit,
            typography.textStyles.metricUnit,
            { color: colors.textSecondary },
          ]}
        >
          rpm
        </Text>
      </View>

      {/* Target cadence section */}
      {targetCadence != null && showMetrics && (
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
                    color:
                      cadenceStatus === 'on_target'
                        ? colors.success
                        : colors.textSecondary,
                  },
                ]}
              >
                {targetCadence}
              </Text>
              <Text
                style={[
                  styles.targetUnit,
                  { color: colors.textMuted },
                ]}
              >
                {' '}rpm
              </Text>
            </View>
          </View>

          {/* Cadence difference indicator */}
          {cadenceDiff !== null && cadence != null && cadence > 0 && (
            <View style={styles.diffRow}>
              <View
                style={[
                  styles.diffBadge,
                  { backgroundColor: statusColor },
                ]}
              >
                <Text style={styles.diffText}>
                  {cadenceDiff >= 0 ? '+' : ''}{cadenceDiff} rpm
                </Text>
              </View>
              <Text
                style={[
                  styles.diffHint,
                  { color: colors.textMuted },
                ]}
              >
                {getStatusLabel(cadenceStatus)}
              </Text>
            </View>
          )}
        </View>
      )}

      {/* No target indicator - just show range hint */}
      {targetCadence == null && showMetrics && cadence != null && cadence > 0 && (
        <View style={styles.secondaryRow}>
          <Text
            style={[
              styles.hintText,
              { color: colors.textMuted },
            ]}
          >
            No target set
          </Text>
        </View>
      )}

      {/* Decorative cadence icon indicator */}
      {showMetrics && cadence != null && cadence > 0 && (
        <View style={styles.iconContainer}>
          <Text style={[styles.icon, { color: statusColor }]}>
            {'\u21BB'}
          </Text>
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    minHeight: 140,
    position: 'relative',
    overflow: 'hidden',
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
  statusBadge: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
  },
  statusBadgeText: {
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
    marginTop: 8,
  },
  hintText: {
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
  iconContainer: {
    position: 'absolute',
    right: 16,
    bottom: 16,
    opacity: 0.15,
  },
  icon: {
    fontSize: 48,
  },
});
