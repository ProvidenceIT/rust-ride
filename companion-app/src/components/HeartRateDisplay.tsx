/**
 * HeartRateDisplay Component
 *
 * A specialized heart rate display for the dashboard.
 * Shows current HR in BPM, HR zone indicator with color coding,
 * max HR for session, and optional pulse animation.
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
import { hrZoneColors } from '@/theme/colors';
import type { HeartRateZone } from '@/stores/metricsStore';

/**
 * HR zone display info
 */
interface ZoneDisplayInfo {
  label: string;
  shortLabel: string;
  color: string;
}

/**
 * Get display info for a heart rate zone
 */
function getZoneDisplayInfo(zone: HeartRateZone): ZoneDisplayInfo {
  const zoneInfo: Record<HeartRateZone, ZoneDisplayInfo> = {
    zone1: {
      label: 'Recovery',
      shortLabel: 'Z1',
      color: hrZoneColors.z1,
    },
    zone2: {
      label: 'Easy',
      shortLabel: 'Z2',
      color: hrZoneColors.z2,
    },
    zone3: {
      label: 'Aerobic',
      shortLabel: 'Z3',
      color: hrZoneColors.z3,
    },
    zone4: {
      label: 'Threshold',
      shortLabel: 'Z4',
      color: hrZoneColors.z4,
    },
    zone5: {
      label: 'Max',
      shortLabel: 'Z5',
      color: hrZoneColors.z5,
    },
  };
  return zoneInfo[zone];
}

/**
 * HeartRateDisplay props
 */
export interface HeartRateDisplayProps extends AccessibilityProps {
  /** Current heart rate in BPM */
  heartRate: number | null;
  /** Current HR zone */
  hrZone: HeartRateZone | null;
  /** Maximum heart rate for the session */
  maxHeartRate: number;
  /** Whether metrics are available to display */
  showMetrics?: boolean;
  /** Whether to animate the pulse */
  showPulseAnimation?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * HeartRateDisplay Component
 *
 * A large, prominent heart rate display showing current BPM, zone indicator,
 * and max HR for the session. Features optional pulse animation.
 *
 * @example
 * ```tsx
 * <HeartRateDisplay
 *   heartRate={145}
 *   hrZone="zone4"
 *   maxHeartRate={165}
 *   showMetrics={true}
 *   showPulseAnimation={true}
 * />
 * ```
 */
export function HeartRateDisplay({
  heartRate,
  hrZone,
  maxHeartRate,
  showMetrics = true,
  showPulseAnimation = true,
  style,
  ...accessibilityProps
}: HeartRateDisplayProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  // Pulse animation value
  const pulseAnim = useRef(new Animated.Value(1)).current;

  // Get zone display info
  const zoneInfo = useMemo(
    () => (hrZone ? getZoneDisplayInfo(hrZone) : null),
    [hrZone]
  );

  // Zone color for display
  const zoneColor = useMemo(
    () => (zoneInfo ? zoneInfo.color : colors.textPrimary),
    [zoneInfo, colors.textPrimary]
  );

  // Pulse animation effect
  useEffect(() => {
    if (!showPulseAnimation || !showMetrics || !heartRate || heartRate <= 0) {
      // Reset animation when not showing
      pulseAnim.setValue(1);
      return;
    }

    // Calculate pulse duration based on heart rate
    // 60 BPM = 1000ms per beat, 180 BPM = 333ms per beat
    const pulseDuration = heartRate > 0 ? Math.round(60000 / heartRate) : 1000;

    // Create pulse animation loop
    const pulseAnimation = Animated.loop(
      Animated.sequence([
        Animated.timing(pulseAnim, {
          toValue: 1.08,
          duration: pulseDuration * 0.15,
          easing: Easing.out(Easing.ease),
          useNativeDriver: true,
        }),
        Animated.timing(pulseAnim, {
          toValue: 1,
          duration: pulseDuration * 0.85,
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
  }, [heartRate, showPulseAnimation, showMetrics, pulseAnim]);

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `Heart rate: ${showMetrics && heartRate ? heartRate : 'no data'} beats per minute` +
      (showMetrics && hrZone && zoneInfo ? `, Zone: ${zoneInfo.label}` : '') +
      (showMetrics && maxHeartRate > 0 ? `, Maximum: ${maxHeartRate}` : '');

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.lg,
          padding: spacing.lg,
          borderLeftWidth: showMetrics && heartRate && heartRate > 0 ? 5 : 0,
          borderLeftColor:
            showMetrics && heartRate && heartRate > 0 ? zoneColor : 'transparent',
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
          HEART RATE
        </Text>
        {showMetrics && heartRate && heartRate > 0 && zoneInfo && (
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

      {/* Main heart rate value with optional pulse animation */}
      <Animated.View
        style={[
          styles.mainValueRow,
          showPulseAnimation && showMetrics && heartRate && heartRate > 0
            ? { transform: [{ scale: pulseAnim }] }
            : undefined,
        ]}
      >
        <Text
          style={[
            styles.mainValue,
            typography.textStyles.metricPrimary,
            {
              color:
                showMetrics && heartRate && heartRate > 0
                  ? zoneColor
                  : colors.textPrimary,
            },
          ]}
          numberOfLines={1}
          adjustsFontSizeToFit
        >
          {showMetrics && heartRate ? heartRate : '--'}
        </Text>
        <Text
          style={[
            styles.unit,
            typography.textStyles.metricUnit,
            { color: colors.textSecondary },
          ]}
        >
          bpm
        </Text>
      </Animated.View>

      {/* Max heart rate for session */}
      <View style={styles.secondaryRow}>
        <View style={styles.maxContainer}>
          <Text
            style={[
              styles.maxValue,
              { color: colors.textSecondary },
            ]}
          >
            {showMetrics && maxHeartRate > 0 ? maxHeartRate : '--'}
          </Text>
          <Text
            style={[
              styles.maxLabel,
              { color: colors.textMuted },
            ]}
          >
            {' '}bpm (max)
          </Text>
        </View>
      </View>

      {/* Heart icon indicator - visual enhancement */}
      {showMetrics && heartRate && heartRate > 0 && (
        <Animated.View
          style={[
            styles.heartIconContainer,
            showPulseAnimation
              ? { transform: [{ scale: pulseAnim }] }
              : undefined,
          ]}
        >
          <Text style={[styles.heartIcon, { color: zoneColor }]}>
            {'\u2665'}
          </Text>
        </Animated.View>
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
  maxContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  maxValue: {
    fontSize: 18,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  maxLabel: {
    fontSize: 13,
    fontWeight: '400',
  },
  heartIconContainer: {
    position: 'absolute',
    right: 16,
    bottom: 16,
    opacity: 0.15,
  },
  heartIcon: {
    fontSize: 48,
  },
});
