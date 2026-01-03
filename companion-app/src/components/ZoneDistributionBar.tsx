/**
 * ZoneDistributionBar Component
 *
 * Displays a horizontal bar chart showing time spent in each zone.
 * Used for both power and heart rate zone distribution visualization.
 *
 * Features:
 * - Stacked horizontal bar showing zone percentages
 * - Zone labels with time values
 * - Power zones (7 zones) and HR zones (5 zones)
 * - Color-coded by zone
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
import { useTheme } from '@/theme';
import { zoneColors, hrZoneColors } from '@/theme/colors';

/**
 * Zone data for visualization
 */
export interface ZoneData {
  /** Zone identifier */
  zone: string;
  /** Zone label (e.g., "Z1 Recovery") */
  label: string;
  /** Short label (e.g., "Z1") */
  shortLabel: string;
  /** Time in seconds */
  seconds: number;
  /** Zone color */
  color: string;
}

/**
 * ZoneDistributionBar props
 */
export interface ZoneDistributionBarProps extends AccessibilityProps {
  /** Zone distribution data */
  zones: ZoneData[];
  /** Title for the distribution section */
  title: string;
  /** Whether to show the legend */
  showLegend?: boolean;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * Format seconds to MM:SS or HH:MM:SS
 */
function formatTime(seconds: number): string {
  if (seconds <= 0) {
    return '0:00';
  }

  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

/**
 * ZoneDistributionBar Component
 *
 * Displays a stacked horizontal bar chart showing the distribution
 * of time spent in each training zone.
 *
 * @example
 * ```tsx
 * const powerZones = getPowerZoneData(distribution);
 * <ZoneDistributionBar
 *   title="Power Zones"
 *   zones={powerZones}
 *   showLegend
 * />
 * ```
 */
export function ZoneDistributionBar({
  zones,
  title,
  showLegend = true,
  style,
  ...accessibilityProps
}: ZoneDistributionBarProps): React.JSX.Element {
  const { colors, spacing, borderRadius } = useTheme();

  // Calculate total time and percentages
  const { totalSeconds, zonePercentages } = useMemo(() => {
    const total = zones.reduce((sum, z) => sum + z.seconds, 0);
    const percentages = zones.map(z => ({
      ...z,
      percentage: total > 0 ? (z.seconds / total) * 100 : 0,
    }));
    return { totalSeconds: total, zonePercentages: percentages };
  }, [zones]);

  // Build accessibility label
  const accessibilityLabel =
    accessibilityProps.accessibilityLabel ||
    `${title}: ${zones
      .filter(z => z.seconds > 0)
      .map(z => `${z.label}: ${formatTime(z.seconds)}`)
      .join(', ')}`;

  // Filter zones with non-zero time for the bar
  const activeZones = zonePercentages.filter(z => z.percentage > 0);

  return (
    <View
      style={[
        styles.container,
        { backgroundColor: colors.surface },
        style,
      ]}
      accessible
      accessibilityRole="summary"
      accessibilityLabel={accessibilityLabel}
      {...accessibilityProps}
    >
      {/* Title */}
      <Text style={[styles.title, { color: colors.textPrimary }]}>
        {title}
      </Text>

      {/* Total time */}
      <Text style={[styles.totalTime, { color: colors.textSecondary }]}>
        Total: {formatTime(totalSeconds)}
      </Text>

      {/* Stacked bar chart */}
      <View
        style={[
          styles.barContainer,
          {
            borderRadius: borderRadius.sm,
            backgroundColor: colors.card,
          },
        ]}
      >
        {activeZones.length > 0 ? (
          activeZones.map((zone, index) => (
            <View
              key={zone.zone}
              style={[
                styles.barSegment,
                {
                  width: `${zone.percentage}%`,
                  backgroundColor: zone.color,
                  borderTopLeftRadius: index === 0 ? borderRadius.sm : 0,
                  borderBottomLeftRadius: index === 0 ? borderRadius.sm : 0,
                  borderTopRightRadius:
                    index === activeZones.length - 1 ? borderRadius.sm : 0,
                  borderBottomRightRadius:
                    index === activeZones.length - 1 ? borderRadius.sm : 0,
                },
              ]}
            >
              {/* Show label if segment is wide enough */}
              {zone.percentage >= 10 && (
                <Text style={styles.barLabel}>{zone.shortLabel}</Text>
              )}
            </View>
          ))
        ) : (
          <View style={styles.noDataBar}>
            <Text style={[styles.noDataText, { color: colors.textMuted }]}>
              No zone data available
            </Text>
          </View>
        )}
      </View>

      {/* Legend */}
      {showLegend && (
        <View style={[styles.legend, { gap: spacing.sm }]}>
          {zonePercentages.map(zone => (
            <View
              key={zone.zone}
              style={[styles.legendItem, { gap: spacing.xs }]}
            >
              <View
                style={[
                  styles.legendColor,
                  {
                    backgroundColor: zone.color,
                    borderRadius: 2,
                    opacity: zone.seconds > 0 ? 1 : 0.3,
                  },
                ]}
              />
              <View style={styles.legendTextContainer}>
                <Text
                  style={[
                    styles.legendLabel,
                    {
                      color: zone.seconds > 0 ? colors.textSecondary : colors.textMuted,
                    },
                  ]}
                >
                  {zone.shortLabel}
                </Text>
                <Text
                  style={[
                    styles.legendTime,
                    {
                      color: zone.seconds > 0 ? colors.textPrimary : colors.textMuted,
                    },
                  ]}
                >
                  {formatTime(zone.seconds)}
                </Text>
              </View>
            </View>
          ))}
        </View>
      )}
    </View>
  );
}

/**
 * Helper function to create power zone data from distribution
 */
export function getPowerZoneData(distribution: {
  z1_recovery: number;
  z2_endurance: number;
  z3_tempo: number;
  z4_threshold: number;
  z5_vo2max: number;
  z6_anaerobic: number;
  z7_neuromuscular: number;
} | null | undefined): ZoneData[] {
  const d = distribution || {
    z1_recovery: 0,
    z2_endurance: 0,
    z3_tempo: 0,
    z4_threshold: 0,
    z5_vo2max: 0,
    z6_anaerobic: 0,
    z7_neuromuscular: 0,
  };

  return [
    {
      zone: 'z1',
      label: 'Z1 Recovery',
      shortLabel: 'Z1',
      seconds: d.z1_recovery,
      color: zoneColors.z1Recovery,
    },
    {
      zone: 'z2',
      label: 'Z2 Endurance',
      shortLabel: 'Z2',
      seconds: d.z2_endurance,
      color: zoneColors.z2Endurance,
    },
    {
      zone: 'z3',
      label: 'Z3 Tempo',
      shortLabel: 'Z3',
      seconds: d.z3_tempo,
      color: zoneColors.z3Tempo,
    },
    {
      zone: 'z4',
      label: 'Z4 Threshold',
      shortLabel: 'Z4',
      seconds: d.z4_threshold,
      color: zoneColors.z4Threshold,
    },
    {
      zone: 'z5',
      label: 'Z5 VO2max',
      shortLabel: 'Z5',
      seconds: d.z5_vo2max,
      color: zoneColors.z5Vo2max,
    },
    {
      zone: 'z6',
      label: 'Z6 Anaerobic',
      shortLabel: 'Z6',
      seconds: d.z6_anaerobic,
      color: zoneColors.z6Anaerobic,
    },
    {
      zone: 'z7',
      label: 'Z7 Neuromuscular',
      shortLabel: 'Z7',
      seconds: d.z7_neuromuscular,
      color: zoneColors.z7Neuromuscular,
    },
  ];
}

/**
 * Helper function to create HR zone data from distribution
 */
export function getHrZoneData(distribution: {
  z1: number;
  z2: number;
  z3: number;
  z4: number;
  z5: number;
} | null | undefined): ZoneData[] {
  const d = distribution || { z1: 0, z2: 0, z3: 0, z4: 0, z5: 0 };

  return [
    {
      zone: 'z1',
      label: 'Z1 Recovery',
      shortLabel: 'Z1',
      seconds: d.z1,
      color: hrZoneColors.z1,
    },
    {
      zone: 'z2',
      label: 'Z2 Easy',
      shortLabel: 'Z2',
      seconds: d.z2,
      color: hrZoneColors.z2,
    },
    {
      zone: 'z3',
      label: 'Z3 Aerobic',
      shortLabel: 'Z3',
      seconds: d.z3,
      color: hrZoneColors.z3,
    },
    {
      zone: 'z4',
      label: 'Z4 Threshold',
      shortLabel: 'Z4',
      seconds: d.z4,
      color: hrZoneColors.z4,
    },
    {
      zone: 'z5',
      label: 'Z5 Max',
      shortLabel: 'Z5',
      seconds: d.z5,
      color: hrZoneColors.z5,
    },
  ];
}

const styles = StyleSheet.create({
  container: {
    padding: 16,
    borderRadius: 12,
  },
  title: {
    fontSize: 16,
    fontWeight: '600',
    marginBottom: 4,
  },
  totalTime: {
    fontSize: 13,
    marginBottom: 12,
  },
  barContainer: {
    height: 24,
    flexDirection: 'row',
    overflow: 'hidden',
    marginBottom: 12,
  },
  barSegment: {
    height: '100%',
    justifyContent: 'center',
    alignItems: 'center',
    minWidth: 2,
  },
  barLabel: {
    fontSize: 10,
    fontWeight: '700',
    color: '#FFFFFF',
    textShadowColor: 'rgba(0, 0, 0, 0.3)',
    textShadowOffset: { width: 0, height: 1 },
    textShadowRadius: 2,
  },
  noDataBar: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  noDataText: {
    fontSize: 12,
  },
  legend: {
    flexDirection: 'row',
    flexWrap: 'wrap',
  },
  legendItem: {
    flexDirection: 'row',
    alignItems: 'center',
    minWidth: 70,
    marginBottom: 4,
  },
  legendColor: {
    width: 12,
    height: 12,
  },
  legendTextContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
    gap: 4,
  },
  legendLabel: {
    fontSize: 11,
    fontWeight: '500',
  },
  legendTime: {
    fontSize: 12,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
});
