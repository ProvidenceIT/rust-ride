/**
 * Ride Detail Screen
 *
 * Shows full details of a completed ride including all metrics,
 * zone distributions, and statistics.
 *
 * Features:
 * - Full ride statistics: duration, distance, calories, TSS
 * - Power stats: average, max, normalized power, intensity factor
 * - Heart rate stats: average, max
 * - Cadence stats: average
 * - Date and time display
 * - Unit preference support (metric/imperial)
 * - Loading and error states
 */

import React, { useEffect, useCallback } from 'react';
import { StyleSheet, Text, View, ScrollView, RefreshControl, TouchableOpacity } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import Icon from 'react-native-vector-icons/Ionicons';
import type { RootStackScreenProps } from '@/navigation/types';
import { useTheme } from '@/theme';
import {
  LoadingSpinner,
  RideStatisticsSummary,
  ZoneDistributionBar,
  getPowerZoneData,
  getHrZoneData,
} from '@/components';
import {
  useHistoryStore,
  selectCurrentRideDetail,
  selectIsLoadingDetail,
  selectError,
} from '@/stores/historyStore';
import {
  useConnectionStore,
  selectIsConnected,
} from '@/stores/connectionStore';
import {
  useSettingsStore,
  selectUnits,
  formatDistance,
  formatElapsedTime,
  getDistanceUnit,
} from '@/stores/settingsStore';
import { getConnectionService } from '@/services/ConnectionService';

type Props = RootStackScreenProps<'RideDetail'>;

/**
 * Format date and time from ISO string
 */
function formatDateTime(isoString: string): { date: string; time: string } {
  try {
    const dateObj = new Date(isoString);
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);

    // Format date
    let date: string;
    if (dateObj.toDateString() === today.toDateString()) {
      date = 'Today';
    } else if (dateObj.toDateString() === yesterday.toDateString()) {
      date = 'Yesterday';
    } else {
      date = dateObj.toLocaleDateString(undefined, {
        weekday: 'long',
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    }

    // Format time
    const time = dateObj.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    });

    return { date, time };
  } catch {
    return { date: isoString, time: '' };
  }
}

/**
 * Stat card component for displaying a single metric
 */
interface StatCardProps {
  label: string;
  value: string | number | null;
  unit?: string;
  icon?: string;
  accent?: boolean;
}

function StatCard({
  label,
  value,
  unit,
  icon,
  accent,
}: StatCardProps): React.JSX.Element {
  const { colors } = useTheme();

  const displayValue = value === null || value === undefined ? '--' : String(value);
  const valueColor = accent ? colors.accent : colors.textPrimary;

  return (
    <View
      style={[styles.statCard, { backgroundColor: colors.surface }]}
      accessibilityRole="text"
      accessibilityLabel={`${label}: ${displayValue}${unit ? ` ${unit}` : ''}`}
    >
      {icon && (
        <View style={styles.statIconContainer}>
          <Icon name={icon} size={16} color={colors.textSecondary} />
        </View>
      )}
      <Text style={[styles.statLabel, { color: colors.textSecondary }]}>{label}</Text>
      <View style={styles.statValueContainer}>
        <Text
          style={[styles.statValue, { color: valueColor }]}
          numberOfLines={1}
          adjustsFontSizeToFit
        >
          {displayValue}
        </Text>
        {unit && value !== null && value !== undefined && (
          <Text style={[styles.statUnit, { color: colors.textSecondary }]}>{unit}</Text>
        )}
      </View>
    </View>
  );
}

/**
 * Section header component
 */
interface SectionHeaderProps {
  title: string;
  icon?: string;
}

function SectionHeader({ title, icon }: SectionHeaderProps): React.JSX.Element {
  const { colors, spacing } = useTheme();

  return (
    <View style={[styles.sectionHeader, { marginHorizontal: spacing.md }]}>
      {icon && (
        <Icon name={icon} size={18} color={colors.accent} style={styles.sectionIcon} />
      )}
      <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>{title}</Text>
    </View>
  );
}

export function RideDetailScreen({ route, navigation }: Props): React.JSX.Element {
  const { colors, spacing } = useTheme();
  const { rideId } = route.params;

  // Store selectors
  const rideDetail = useHistoryStore(selectCurrentRideDetail);
  const isLoadingDetail = useHistoryStore(selectIsLoadingDetail);
  const error = useHistoryStore(selectError);
  const isConnected = useConnectionStore(selectIsConnected);
  const units = useSettingsStore(selectUnits);
  const loadSettings = useSettingsStore(state => state.loadSettings);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Fetch ride details when screen loads
  useEffect(() => {
    if (isConnected) {
      const connectionService = getConnectionService();
      connectionService.fetchRideDetails(rideId);
    }
  }, [rideId, isConnected]);

  // Handle refresh
  const handleRefresh = useCallback(async () => {
    if (!isConnected) return;

    // Clear current detail to force re-fetch
    const historyStore = useHistoryStore.getState();
    historyStore.setCurrentRideDetail(null);

    const connectionService = getConnectionService();
    await connectionService.fetchRideDetails(rideId);
  }, [rideId, isConnected]);

  // Render loading state
  if (isLoadingDetail && !rideDetail) {
    return (
      <SafeAreaView
        style={[styles.container, { backgroundColor: colors.background }]}
        edges={['bottom']}
      >
        <View style={styles.loadingContainer}>
          <LoadingSpinner size="large" message="Loading ride details..." />
        </View>
      </SafeAreaView>
    );
  }

  // Render error state
  if (error && !rideDetail) {
    return (
      <SafeAreaView
        style={[styles.container, { backgroundColor: colors.background }]}
        edges={['bottom']}
      >
        <View style={styles.errorContainer}>
          <Icon name="warning-outline" size={48} color={colors.error} style={styles.errorIcon} />
          <Text style={[styles.errorTitle, { color: colors.textPrimary }]}>
            Failed to Load Ride
          </Text>
          <Text style={[styles.errorText, { color: colors.textSecondary }]}>{error}</Text>
          <TouchableOpacity
            style={[styles.retryButton, { backgroundColor: colors.accent }]}
            onPress={handleRefresh}
            accessibilityRole="button"
            accessibilityLabel="Retry loading ride details"
          >
            <Text style={[styles.retryButtonText, { color: colors.textInverse }]}>Try Again</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  // Render not connected state
  if (!isConnected && !rideDetail) {
    return (
      <SafeAreaView
        style={[styles.container, { backgroundColor: colors.background }]}
        edges={['bottom']}
      >
        <View style={styles.errorContainer}>
          <Icon
            name="cloud-offline-outline"
            size={48}
            color={colors.textSecondary}
            style={styles.errorIcon}
          />
          <Text style={[styles.errorTitle, { color: colors.textPrimary }]}>Not Connected</Text>
          <Text style={[styles.errorText, { color: colors.textSecondary }]}>
            Connect to your desktop app to view ride details
          </Text>
          <TouchableOpacity
            style={[styles.retryButton, { backgroundColor: colors.accent }]}
            onPress={() => navigation.navigate('Connection')}
            accessibilityRole="button"
            accessibilityLabel="Connect to desktop app"
          >
            <Text style={[styles.retryButtonText, { color: colors.textInverse }]}>Connect</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  // Use ride detail or show empty state
  if (!rideDetail) {
    return (
      <SafeAreaView
        style={[styles.container, { backgroundColor: colors.background }]}
        edges={['bottom']}
      >
        <View style={styles.errorContainer}>
          <Icon name="bicycle-outline" size={48} color={colors.textSecondary} />
          <Text style={[styles.errorTitle, { color: colors.textPrimary }]}>Ride Not Found</Text>
          <Text style={[styles.errorText, { color: colors.textSecondary }]}>
            The requested ride could not be found
          </Text>
        </View>
      </SafeAreaView>
    );
  }

  // Format ride data
  const { date, time } = formatDateTime(rideDetail.started_at);
  const distanceValue = formatDistance(rideDetail.distance_km, units);
  const distanceUnit = getDistanceUnit(units);
  const durationFormatted = formatElapsedTime(rideDetail.duration_secs);

  // Format nullable values
  const formatNullableNumber = (value: number | null, decimals = 0): string | null => {
    if (value === null || value === undefined) return null;
    return decimals > 0 ? value.toFixed(decimals) : String(Math.round(value));
  };

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['bottom']}
    >
      <ScrollView
        style={styles.scrollView}
        showsVerticalScrollIndicator={false}
        refreshControl={
          <RefreshControl
            refreshing={isLoadingDetail}
            onRefresh={handleRefresh}
            tintColor={colors.accent}
            colors={[colors.accent]}
            enabled={isConnected}
          />
        }
      >
        {/* Header with date and time */}
        <View style={[styles.header, { backgroundColor: colors.surface }]}>
          <View style={styles.headerDateContainer}>
            <Icon name="calendar-outline" size={20} color={colors.accent} style={styles.headerIcon} />
            <Text style={[styles.date, { color: colors.textPrimary }]}>{date}</Text>
          </View>
          <Text style={[styles.time, { color: colors.textSecondary }]}>{time}</Text>
          {rideDetail.workout_name && (
            <View style={[styles.workoutBadge, { backgroundColor: colors.accent }]}>
              <Icon name="barbell-outline" size={14} color={colors.textInverse} style={styles.badgeIcon} />
              <Text style={[styles.workoutBadgeText, { color: colors.textInverse }]}>
                {rideDetail.workout_name}
              </Text>
            </View>
          )}
          {!rideDetail.is_workout && (
            <View style={[styles.freeRideBadge, { borderColor: colors.textSecondary }]}>
              <Icon name="bicycle-outline" size={14} color={colors.textSecondary} style={styles.badgeIcon} />
              <Text style={[styles.freeRideBadgeText, { color: colors.textSecondary }]}>
                Free Ride
              </Text>
            </View>
          )}
        </View>

        {/* Training Summary Section - Key Stats */}
        <SectionHeader title="Training Summary" icon="stats-chart-outline" />
        <View style={{ paddingHorizontal: spacing.md, marginBottom: spacing.md }}>
          <RideStatisticsSummary
            tss={rideDetail.tss}
            intensityFactor={rideDetail.intensity_factor}
            calories={rideDetail.calories}
          />
        </View>

        {/* Summary Section */}
        <SectionHeader title="Ride Overview" icon="bicycle-outline" />
        <View style={[styles.statsGrid, { paddingHorizontal: spacing.md }]}>
          <StatCard
            label="Duration"
            value={durationFormatted}
            icon="time-outline"
            accent
          />
          <StatCard
            label="Distance"
            value={distanceValue}
            unit={distanceUnit}
            icon="navigate-outline"
          />
        </View>

        {/* Power Section */}
        <SectionHeader title="Power" icon="flash-outline" />
        <View style={[styles.statsGrid, { paddingHorizontal: spacing.md }]}>
          <StatCard
            label="Average"
            value={formatNullableNumber(rideDetail.avg_power_watts)}
            unit="W"
          />
          <StatCard
            label="Maximum"
            value={formatNullableNumber(rideDetail.max_power_watts)}
            unit="W"
          />
          <StatCard
            label="Normalized (NP)"
            value={formatNullableNumber(rideDetail.normalized_power_watts)}
            unit="W"
          />
        </View>

        {/* Heart Rate Section */}
        <SectionHeader title="Heart Rate" icon="heart-outline" />
        <View style={[styles.statsGrid, { paddingHorizontal: spacing.md }]}>
          <StatCard
            label="Average"
            value={formatNullableNumber(rideDetail.avg_heart_rate_bpm)}
            unit="bpm"
          />
          <StatCard
            label="Maximum"
            value={formatNullableNumber(rideDetail.max_heart_rate_bpm)}
            unit="bpm"
          />
        </View>

        {/* Cadence Section */}
        <SectionHeader title="Cadence" icon="sync-outline" />
        <View style={[styles.statsGrid, { paddingHorizontal: spacing.md }]}>
          <StatCard
            label="Average"
            value={formatNullableNumber(rideDetail.avg_cadence_rpm)}
            unit="rpm"
          />
        </View>

        {/* Power Zone Distribution */}
        <SectionHeader title="Time in Power Zones" icon="flash-outline" />
        <View style={{ paddingHorizontal: spacing.md, marginBottom: spacing.md }}>
          <ZoneDistributionBar
            title="Power Zone Distribution"
            zones={getPowerZoneData(rideDetail.power_zone_distribution)}
            showLegend
          />
        </View>

        {/* HR Zone Distribution */}
        <SectionHeader title="Time in HR Zones" icon="heart-outline" />
        <View style={{ paddingHorizontal: spacing.md, marginBottom: spacing.md }}>
          <ZoneDistributionBar
            title="Heart Rate Zone Distribution"
            zones={getHrZoneData(rideDetail.hr_zone_distribution)}
            showLegend
          />
        </View>

        {/* Footer spacing */}
        <View style={styles.footer} />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  scrollView: {
    flex: 1,
  },
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  errorContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 32,
  },
  errorIcon: {
    marginBottom: 16,
  },
  errorTitle: {
    fontSize: 18,
    fontWeight: '600',
    marginBottom: 8,
    textAlign: 'center',
  },
  errorText: {
    fontSize: 14,
    textAlign: 'center',
    lineHeight: 20,
    marginBottom: 16,
  },
  retryButton: {
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 8,
    marginTop: 8,
  },
  retryButtonText: {
    fontSize: 16,
    fontWeight: '600',
  },
  header: {
    padding: 20,
    alignItems: 'center',
    marginBottom: 16,
  },
  headerDateContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 4,
  },
  headerIcon: {
    marginRight: 8,
  },
  date: {
    fontSize: 20,
    fontWeight: '600',
  },
  time: {
    fontSize: 16,
    marginBottom: 12,
  },
  workoutBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 20,
    marginTop: 8,
  },
  freeRideBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 20,
    marginTop: 8,
    borderWidth: 1,
  },
  badgeIcon: {
    marginRight: 6,
  },
  workoutBadgeText: {
    fontSize: 14,
    fontWeight: '500',
  },
  freeRideBadgeText: {
    fontSize: 14,
    fontWeight: '500',
  },
  sectionHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 12,
    marginTop: 8,
  },
  sectionIcon: {
    marginRight: 8,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  statsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
    marginBottom: 16,
  },
  statCard: {
    width: '48%',
    flexGrow: 1,
    minWidth: 150,
    padding: 16,
    borderRadius: 12,
  },
  statIconContainer: {
    marginBottom: 4,
  },
  statLabel: {
    fontSize: 12,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 4,
  },
  statValueContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
    gap: 4,
  },
  statValue: {
    fontSize: 24,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  statUnit: {
    fontSize: 14,
  },
  footer: {
    height: 40,
  },
});
