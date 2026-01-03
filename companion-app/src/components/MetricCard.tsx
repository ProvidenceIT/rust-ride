/**
 * MetricCard Component
 *
 * Displays a metric value with unit and label.
 * Supports different sizes and optional zone-based color coding.
 */

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
  TextStyle,
  AccessibilityProps,
} from 'react-native';
import { useTheme } from '@/theme';

/**
 * MetricCard size variants
 */
export type MetricCardSize = 'small' | 'medium' | 'large';

/**
 * MetricCard props
 */
export interface MetricCardProps extends AccessibilityProps {
  /** The metric value to display */
  value: string | number;
  /** The unit of measurement (e.g., 'W', 'bpm', 'rpm') */
  unit?: string;
  /** The label describing the metric (e.g., 'Power', 'Heart Rate') */
  label: string;
  /** Size variant */
  size?: MetricCardSize;
  /** Optional accent/zone color for the value */
  accentColor?: string;
  /** Optional target value to display */
  targetValue?: string | number;
  /** Optional target label */
  targetLabel?: string;
  /** Optional secondary value (e.g., 3s average) */
  secondaryValue?: string | number;
  /** Optional secondary label */
  secondaryLabel?: string;
  /** Custom container style */
  style?: ViewStyle;
  /** Whether to show a border when accent color is set */
  showAccentBorder?: boolean;
}

/**
 * MetricCard Component
 *
 * A card component for displaying workout metrics like power, heart rate, cadence, etc.
 *
 * @example
 * ```tsx
 * <MetricCard
 *   value={250}
 *   unit="W"
 *   label="Power"
 *   size="large"
 *   accentColor={getPowerZoneColor(4)}
 * />
 * ```
 */
export function MetricCard({
  value,
  unit,
  label,
  size = 'medium',
  accentColor,
  targetValue,
  targetLabel,
  secondaryValue,
  secondaryLabel,
  style,
  showAccentBorder = false,
  ...accessibilityProps
}: MetricCardProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius: themeRadius } = useTheme();

  // Format value for display
  const displayValue = typeof value === 'number' ? value.toString() : value;

  // Get size-specific styles
  const sizeStyles = getSizeStyles(size, typography);

  // Build dynamic styles
  const cardStyle: ViewStyle = {
    backgroundColor: colors.card,
    borderRadius: themeRadius.md,
    padding: spacing.lg,
    minHeight: size === 'large' ? 160 : size === 'medium' ? 120 : 80,
    ...(showAccentBorder && accentColor
      ? {
          borderLeftWidth: 4,
          borderLeftColor: accentColor,
        }
      : {}),
  };

  const valueStyle: TextStyle = {
    ...sizeStyles.value,
    color: accentColor || colors.textPrimary,
  };

  const unitStyle: TextStyle = {
    ...sizeStyles.unit,
    color: colors.textSecondary,
  };

  const labelStyle: TextStyle = {
    ...sizeStyles.label,
    color: colors.textMuted,
  };

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `${label}: ${displayValue}${unit ? ` ${unit}` : ''}`;

  return (
    <View
      style={[styles.container, cardStyle, style]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Label at top */}
      <Text style={[styles.label, labelStyle]} numberOfLines={1}>
        {label.toUpperCase()}
      </Text>

      {/* Main value row */}
      <View style={styles.valueRow}>
        <Text style={[styles.value, valueStyle]} numberOfLines={1} adjustsFontSizeToFit>
          {displayValue}
        </Text>
        {unit && (
          <Text style={[styles.unit, unitStyle]} numberOfLines={1}>
            {unit}
          </Text>
        )}
      </View>

      {/* Secondary value (e.g., 3s average) */}
      {secondaryValue !== undefined && (
        <View style={styles.secondaryRow}>
          <Text style={[styles.secondaryValue, { color: colors.textSecondary }]}>
            {secondaryValue}
          </Text>
          {secondaryLabel && (
            <Text style={[styles.secondaryLabel, { color: colors.textMuted }]}>
              {secondaryLabel}
            </Text>
          )}
        </View>
      )}

      {/* Target value (for workouts) */}
      {targetValue !== undefined && (
        <View style={styles.targetRow}>
          <Text style={[styles.targetLabel, { color: colors.textMuted }]}>
            {targetLabel || 'Target'}:
          </Text>
          <Text style={[styles.targetValue, { color: colors.textSecondary }]}>
            {targetValue}
            {unit && ` ${unit}`}
          </Text>
        </View>
      )}
    </View>
  );
}

/**
 * Get size-specific text styles
 */
function getSizeStyles(
  size: MetricCardSize,
  typography: ReturnType<typeof useTheme>['typography'],
) {
  const { textStyles } = typography;
  switch (size) {
    case 'large':
      return {
        value: textStyles.metricPrimary,
        unit: textStyles.metricUnit,
        label: textStyles.metricLabel,
      };
    case 'medium':
      return {
        value: textStyles.metricSecondary,
        unit: textStyles.metricUnit,
        label: textStyles.metricLabel,
      };
    case 'small':
      return {
        value: textStyles.metricTertiary,
        unit: textStyles.metricUnit,
        label: textStyles.metricLabel,
      };
  }
}

const styles = StyleSheet.create({
  container: {
    justifyContent: 'center',
    alignItems: 'flex-start',
  },
  label: {
    marginBottom: 4,
  },
  valueRow: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  value: {
    includeFontPadding: false,
  },
  unit: {
    marginLeft: 4,
    alignSelf: 'flex-end',
    marginBottom: 4,
  },
  secondaryRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 4,
  },
  secondaryValue: {
    fontSize: 14,
    fontWeight: '500',
  },
  secondaryLabel: {
    fontSize: 12,
    marginLeft: 4,
  },
  targetRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 8,
    paddingTop: 8,
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: 'rgba(255,255,255,0.1)',
    alignSelf: 'stretch',
  },
  targetLabel: {
    fontSize: 12,
    fontWeight: '500',
  },
  targetValue: {
    fontSize: 14,
    fontWeight: '600',
    marginLeft: 4,
  },
});
