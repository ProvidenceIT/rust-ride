/**
 * RideStatisticsSummary Component
 *
 * Displays key training statistics (TSS, IF, calories) in prominent cards.
 * Used on the RideDetailScreen to highlight important training metrics.
 *
 * Features:
 * - TSS (Training Stress Score) with color-coded intensity
 * - IF (Intensity Factor) display
 * - Calories burned
 * - Descriptive labels for each metric
 * - Accessible with proper labels
 */

import React, { useMemo } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
  AccessibilityProps,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';

/**
 * Get TSS intensity level and color
 */
function getTssIntensity(tss: number): { level: string; color: string } {
  if (tss < 50) {
    return { level: 'Easy', color: '#4CAF50' }; // Green
  }
  if (tss < 100) {
    return { level: 'Moderate', color: '#FFC107' }; // Yellow
  }
  if (tss < 150) {
    return { level: 'Hard', color: '#FF9800' }; // Orange
  }
  if (tss < 200) {
    return { level: 'Very Hard', color: '#FF5722' }; // Deep Orange
  }
  return { level: 'Epic', color: '#F44336' }; // Red
}

/**
 * Get IF intensity level description
 */
function getIfDescription(ifValue: number): string {
  if (ifValue < 0.55) {
    return 'Recovery';
  }
  if (ifValue < 0.75) {
    return 'Endurance';
  }
  if (ifValue < 0.90) {
    return 'Tempo';
  }
  if (ifValue < 1.0) {
    return 'Threshold';
  }
  if (ifValue < 1.05) {
    return 'Near FTP';
  }
  return 'Above FTP';
}

/**
 * RideStatisticsSummary props
 */
export interface RideStatisticsSummaryProps extends AccessibilityProps {
  /** TSS (Training Stress Score) */
  tss: number | null;
  /** IF (Intensity Factor) */
  intensityFactor: number | null;
  /** Calories burned */
  calories: number;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * StatCard internal component
 */
interface StatCardProps {
  icon: string;
  label: string;
  value: string;
  sublabel?: string;
  valueColor?: string;
}

function StatCard({
  icon,
  label,
  value,
  sublabel,
  valueColor,
}: StatCardProps): React.JSX.Element {
  const { colors, spacing, borderRadius } = useTheme();

  return (
    <View
      style={[
        styles.statCard,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.md,
          padding: spacing.md,
        },
      ]}
      accessibilityRole="text"
      accessibilityLabel={`${label}: ${value}${sublabel ? `, ${sublabel}` : ''}`}
    >
      <View style={styles.statHeader}>
        <Icon name={icon} size={16} color={colors.textSecondary} />
        <Text style={[styles.statLabel, { color: colors.textSecondary }]}>
          {label}
        </Text>
      </View>
      <Text
        style={[
          styles.statValue,
          { color: valueColor || colors.textPrimary },
        ]}
        numberOfLines={1}
        adjustsFontSizeToFit
      >
        {value}
      </Text>
      {sublabel && (
        <Text style={[styles.statSublabel, { color: colors.textMuted }]}>
          {sublabel}
        </Text>
      )}
    </View>
  );
}

/**
 * RideStatisticsSummary Component
 *
 * Displays key training statistics in a horizontal row of cards.
 *
 * @example
 * ```tsx
 * <RideStatisticsSummary
 *   tss={85}
 *   intensityFactor={0.92}
 *   calories={650}
 * />
 * ```
 */
export function RideStatisticsSummary({
  tss,
  intensityFactor,
  calories,
  style,
  ...accessibilityProps
}: RideStatisticsSummaryProps): React.JSX.Element {
  const { colors, spacing } = useTheme();

  // Compute TSS display info
  const tssInfo = useMemo(() => {
    if (tss === null || tss === undefined) {
      return { value: '--', level: 'N/A', color: colors.textPrimary };
    }
    const intensity = getTssIntensity(tss);
    return {
      value: Math.round(tss).toString(),
      level: intensity.level,
      color: intensity.color,
    };
  }, [tss, colors.textPrimary]);

  // Compute IF display info
  const ifInfo = useMemo(() => {
    if (intensityFactor === null || intensityFactor === undefined) {
      return { value: '--', description: 'N/A' };
    }
    return {
      value: intensityFactor.toFixed(2),
      description: getIfDescription(intensityFactor),
    };
  }, [intensityFactor]);

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `Training Summary: TSS ${tssInfo.value}${tss !== null ? ` (${tssInfo.level})` : ''}, ` +
      `Intensity Factor ${ifInfo.value}${intensityFactor !== null ? ` (${ifInfo.description})` : ''}, ` +
      `${calories} calories`;

  return (
    <View
      style={[
        styles.container,
        { gap: spacing.sm },
        style,
      ]}
      accessible
      accessibilityRole="summary"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* TSS Card */}
      <StatCard
        icon="fitness-outline"
        label="TSS"
        value={tssInfo.value}
        sublabel={tss !== null ? tssInfo.level : undefined}
        valueColor={tss !== null ? tssInfo.color : colors.textMuted}
      />

      {/* Intensity Factor Card */}
      <StatCard
        icon="speedometer-outline"
        label="Intensity Factor"
        value={ifInfo.value}
        sublabel={intensityFactor !== null ? ifInfo.description : undefined}
      />

      {/* Calories Card */}
      <StatCard
        icon="flame-outline"
        label="Calories"
        value={calories > 0 ? Math.round(calories).toString() : '--'}
        sublabel={calories > 0 ? 'kcal' : undefined}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    flexWrap: 'wrap',
  },
  statCard: {
    flex: 1,
    minWidth: 100,
  },
  statHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
    marginBottom: 8,
  },
  statLabel: {
    fontSize: 11,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.3,
  },
  statValue: {
    fontSize: 28,
    fontWeight: '700',
    fontVariant: ['tabular-nums'],
    marginBottom: 2,
  },
  statSublabel: {
    fontSize: 12,
    fontWeight: '500',
  },
});
