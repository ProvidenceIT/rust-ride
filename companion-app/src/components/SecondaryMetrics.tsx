/**
 * Secondary Metrics Components
 *
 * Smaller metric cards for speed, distance, elapsed time, and calories.
 * These components respect the user's unit preference (metric/imperial).
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
import {
  useSettingsStore,
  selectUnits,
  formatSpeed,
  formatDistance,
  formatElapsedTime,
  formatCalories,
  getSpeedUnit,
  getDistanceUnit,
  type UnitSystem,
} from '@/stores/settingsStore';

// ============================================================
// Base Secondary Metric Card
// ============================================================

interface SecondaryMetricCardProps extends AccessibilityProps {
  /** The metric value to display */
  value: string;
  /** The unit of measurement */
  unit?: string;
  /** The label describing the metric */
  label: string;
  /** Icon name from Ionicons */
  iconName: string;
  /** Whether to show placeholder state */
  showMetrics?: boolean;
  /** Placeholder value when metrics not available */
  placeholder?: string;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * Base component for secondary metric cards.
 * Used as foundation for specialized metric displays.
 */
function SecondaryMetricCard({
  value,
  unit,
  label,
  iconName,
  showMetrics = true,
  placeholder = '--',
  style,
  ...accessibilityProps
}: SecondaryMetricCardProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  const displayValue = showMetrics ? value : placeholder;

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `${label}: ${displayValue}${unit ? ` ${unit}` : ''}`;

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.md,
          padding: spacing.md,
        },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Header with icon and label */}
      <View style={styles.header}>
        <Icon
          name={iconName}
          size={14}
          color={colors.textMuted}
          style={styles.icon}
        />
        <Text
          style={[
            styles.label,
            typography.textStyles.metricLabel,
            { color: colors.textMuted },
          ]}
          numberOfLines={1}
        >
          {label.toUpperCase()}
        </Text>
      </View>

      {/* Value row */}
      <View style={styles.valueRow}>
        <Text
          style={[
            styles.value,
            typography.textStyles.metricTertiary,
            { color: colors.textPrimary },
          ]}
          numberOfLines={1}
          adjustsFontSizeToFit
        >
          {displayValue}
        </Text>
        {unit && (
          <Text
            style={[
              styles.unit,
              { color: colors.textSecondary },
            ]}
            numberOfLines={1}
          >
            {unit}
          </Text>
        )}
      </View>
    </View>
  );
}

// ============================================================
// Speed Display
// ============================================================

export interface SpeedDisplayProps extends AccessibilityProps {
  /** Speed in km/h (always metric from server) */
  speedKph: number;
  /** Whether to show metrics or placeholder */
  showMetrics?: boolean;
  /** Override unit system (uses settings by default) */
  units?: UnitSystem;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * SpeedDisplay Component
 *
 * Displays current speed with unit conversion based on user preferences.
 *
 * @example
 * ```tsx
 * <SpeedDisplay speedKph={32.5} showMetrics={true} />
 * ```
 */
export function SpeedDisplay({
  speedKph,
  showMetrics = true,
  units: overrideUnits,
  style,
  ...accessibilityProps
}: SpeedDisplayProps): React.JSX.Element {
  const settingsUnits = useSettingsStore(selectUnits);
  const units = overrideUnits ?? settingsUnits;

  const formattedValue = useMemo(
    () => formatSpeed(speedKph, units),
    [speedKph, units]
  );

  const unitLabel = useMemo(() => getSpeedUnit(units), [units]);

  const accessibilityLabel = showMetrics
    ? `Speed: ${formattedValue} ${units === 'imperial' ? 'miles per hour' : 'kilometers per hour'}`
    : 'Speed: no data';

  return (
    <SecondaryMetricCard
      value={formattedValue}
      unit={unitLabel}
      label="Speed"
      iconName="speedometer-outline"
      showMetrics={showMetrics}
      placeholder="--"
      style={style}
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    />
  );
}

// ============================================================
// Distance Display
// ============================================================

export interface DistanceDisplayProps extends AccessibilityProps {
  /** Distance in kilometers (always metric from server) */
  distanceKm: number;
  /** Whether to show metrics or placeholder */
  showMetrics?: boolean;
  /** Override unit system (uses settings by default) */
  units?: UnitSystem;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * DistanceDisplay Component
 *
 * Displays total distance with unit conversion based on user preferences.
 *
 * @example
 * ```tsx
 * <DistanceDisplay distanceKm={15.234} showMetrics={true} />
 * ```
 */
export function DistanceDisplay({
  distanceKm,
  showMetrics = true,
  units: overrideUnits,
  style,
  ...accessibilityProps
}: DistanceDisplayProps): React.JSX.Element {
  const settingsUnits = useSettingsStore(selectUnits);
  const units = overrideUnits ?? settingsUnits;

  const formattedValue = useMemo(
    () => formatDistance(distanceKm, units),
    [distanceKm, units]
  );

  const unitLabel = useMemo(() => getDistanceUnit(units), [units]);

  const accessibilityLabel = showMetrics
    ? `Distance: ${formattedValue} ${units === 'imperial' ? 'miles' : 'kilometers'}`
    : 'Distance: no data';

  return (
    <SecondaryMetricCard
      value={formattedValue}
      unit={unitLabel}
      label="Distance"
      iconName="navigate-outline"
      showMetrics={showMetrics}
      placeholder="0.00"
      style={style}
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    />
  );
}

// ============================================================
// Elapsed Time Display
// ============================================================

export interface ElapsedTimeDisplayProps extends AccessibilityProps {
  /** Elapsed time in seconds */
  elapsedSecs: number;
  /** Whether to show metrics or placeholder */
  showMetrics?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * ElapsedTimeDisplay Component
 *
 * Displays elapsed session time in HH:MM:SS or M:SS format.
 *
 * @example
 * ```tsx
 * <ElapsedTimeDisplay elapsedSecs={3725} showMetrics={true} />
 * ```
 */
export function ElapsedTimeDisplay({
  elapsedSecs,
  showMetrics = true,
  style,
  ...accessibilityProps
}: ElapsedTimeDisplayProps): React.JSX.Element {
  const formattedValue = useMemo(
    () => formatElapsedTime(elapsedSecs),
    [elapsedSecs]
  );

  // Create readable time for accessibility
  const readableTime = useMemo(() => {
    const hours = Math.floor(elapsedSecs / 3600);
    const minutes = Math.floor((elapsedSecs % 3600) / 60);
    const secs = Math.floor(elapsedSecs % 60);

    const parts: string[] = [];
    if (hours > 0) {
      parts.push(`${hours} ${hours === 1 ? 'hour' : 'hours'}`);
    }
    if (minutes > 0) {
      parts.push(`${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`);
    }
    if (secs > 0 || parts.length === 0) {
      parts.push(`${secs} ${secs === 1 ? 'second' : 'seconds'}`);
    }
    return parts.join(' ');
  }, [elapsedSecs]);

  const accessibilityLabel = showMetrics
    ? `Elapsed time: ${readableTime}`
    : 'Elapsed time: no data';

  return (
    <SecondaryMetricCard
      value={formattedValue}
      label="Time"
      iconName="time-outline"
      showMetrics={showMetrics}
      placeholder="0:00"
      style={style}
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    />
  );
}

// ============================================================
// Calories Display
// ============================================================

export interface CaloriesDisplayProps extends AccessibilityProps {
  /** Calories burned */
  calories: number;
  /** Whether to show metrics or placeholder */
  showMetrics?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * CaloriesDisplay Component
 *
 * Displays total calories burned during the session.
 *
 * @example
 * ```tsx
 * <CaloriesDisplay calories={523} showMetrics={true} />
 * ```
 */
export function CaloriesDisplay({
  calories,
  showMetrics = true,
  style,
  ...accessibilityProps
}: CaloriesDisplayProps): React.JSX.Element {
  const formattedValue = useMemo(
    () => formatCalories(calories),
    [calories]
  );

  const accessibilityLabel = showMetrics
    ? `Calories: ${Math.round(calories)} kilocalories burned`
    : 'Calories: no data';

  return (
    <SecondaryMetricCard
      value={formattedValue}
      unit="kcal"
      label="Calories"
      iconName="flame-outline"
      showMetrics={showMetrics}
      placeholder="0"
      style={style}
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    />
  );
}

// ============================================================
// Styles
// ============================================================

const styles = StyleSheet.create({
  container: {
    minHeight: 80,
    justifyContent: 'center',
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 4,
  },
  icon: {
    marginRight: 4,
  },
  label: {
    // Typography from theme
  },
  valueRow: {
    flexDirection: 'row',
    alignItems: 'baseline',
  },
  value: {
    includeFontPadding: false,
    fontVariant: ['tabular-nums'],
  },
  unit: {
    fontSize: 12,
    fontWeight: '500',
    marginLeft: 4,
    alignSelf: 'flex-end',
    marginBottom: 2,
  },
});
