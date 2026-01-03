/**
 * History Screen
 *
 * Displays a paginated list of past rides with summary information.
 * Features:
 * - Paginated list (20 rides per page)
 * - Pull to refresh
 * - Infinite scroll pagination
 * - Date, duration, distance, and avg power display
 * - Unit preference support (metric/imperial)
 * - Tapping a ride navigates to the detail screen
 */

import React, { useEffect, useCallback, useRef } from 'react';
import {
  StyleSheet,
  Text,
  View,
  FlatList,
  TouchableOpacity,
  RefreshControl,
  ActivityIndicator,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import Icon from 'react-native-vector-icons/Ionicons';
import type { MainTabScreenProps, RootStackNavigationProp } from '@/navigation/types';
import type { RideSummary } from '@/types';
import { useTheme } from '@/theme';
import { ConnectionStatus, LoadingSpinner } from '@/components';
import {
  useHistoryStore,
  selectRides,
  selectIsLoading,
  selectIsLoadingMore,
  selectError,
  selectHasMore,
  selectIsEmpty,
  selectTotal,
  selectPagination,
} from '@/stores/historyStore';
import {
  useConnectionStore,
  selectConnectionStatus,
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

/**
 * Page size for ride history
 */
const PAGE_SIZE = 20;

type Props = MainTabScreenProps<'History'>;

export function HistoryScreen(_props: Props): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();
  const navigation = useNavigation<RootStackNavigationProp>();

  // Track if initial load has been attempted
  const hasLoadedRef = useRef(false);

  // Connection state
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const isConnected = useConnectionStore(selectIsConnected);

  // History state
  const rides = useHistoryStore(selectRides);
  const isLoading = useHistoryStore(selectIsLoading);
  const isLoadingMore = useHistoryStore(selectIsLoadingMore);
  const error = useHistoryStore(selectError);
  const hasMore = useHistoryStore(selectHasMore);
  const isEmpty = useHistoryStore(selectIsEmpty);
  const total = useHistoryStore(selectTotal);
  const pagination = useHistoryStore(selectPagination);

  // Settings state
  const units = useSettingsStore(selectUnits);
  const loadSettings = useSettingsStore(state => state.loadSettings);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Load initial rides when connected
  useEffect(() => {
    if (isConnected && !hasLoadedRef.current && rides.length === 0) {
      hasLoadedRef.current = true;
      loadRides();
    }
  }, [isConnected, rides.length, loadRides]);

  // Reset load flag when disconnected
  useEffect(() => {
    if (!isConnected) {
      hasLoadedRef.current = false;
    }
  }, [isConnected]);

  /**
   * Load initial rides (first page)
   */
  const loadRides = useCallback(async () => {
    if (!isConnected) return;

    const historyStore = useHistoryStore.getState();
    historyStore.resetPagination();
    historyStore.clearRides();

    const connectionService = getConnectionService();
    await connectionService.fetchRideHistory(PAGE_SIZE, 0);
  }, [isConnected]);

  /**
   * Handle pull to refresh
   */
  const handleRefresh = useCallback(async () => {
    hasLoadedRef.current = true;
    await loadRides();
  }, [loadRides]);

  /**
   * Load more rides (next page) for infinite scroll
   */
  const handleLoadMore = useCallback(async () => {
    if (!hasMore || isLoadingMore || !isConnected) return;

    const connectionService = getConnectionService();
    await connectionService.fetchRideHistory(PAGE_SIZE, pagination.offset);
  }, [hasMore, isLoadingMore, isConnected, pagination.offset]);

  /**
   * Handle ride press - navigate to detail screen
   */
  const handleRidePress = useCallback(
    (rideId: string) => {
      navigation.navigate('RideDetail', { rideId });
    },
    [navigation]
  );

  /**
   * Navigate to connection screen
   */
  const handleConnectPress = useCallback(() => {
    navigation.navigate('Connection');
  }, [navigation]);

  /**
   * Format date for display
   */
  const formatDate = (dateString: string): string => {
    try {
      const date = new Date(dateString);
      const today = new Date();
      const yesterday = new Date(today);
      yesterday.setDate(yesterday.getDate() - 1);

      // Check if today
      if (date.toDateString() === today.toDateString()) {
        return `Today, ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
      }

      // Check if yesterday
      if (date.toDateString() === yesterday.toDateString()) {
        return `Yesterday, ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
      }

      // Otherwise show full date
      return date.toLocaleDateString([], {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
      });
    } catch {
      return dateString;
    }
  };

  /**
   * Render a single ride item
   */
  const renderRideItem = ({ item }: { item: RideSummary }) => {
    const distanceFormatted = formatDistance(item.distance_km, units);
    const distanceUnit = getDistanceUnit(units);
    const durationFormatted = formatElapsedTime(item.duration_secs);

    return (
      <TouchableOpacity
        style={[styles.rideCard, { backgroundColor: colors.surface }]}
        onPress={() => handleRidePress(item.id)}
        activeOpacity={0.7}
        accessibilityRole="button"
        accessibilityLabel={`Ride on ${formatDate(item.date)}, ${durationFormatted}, ${distanceFormatted} ${distanceUnit}, ${item.avg_power_watts} watts average`}
        accessibilityHint="Double tap to view ride details"
      >
        {/* Header with date and workout badge */}
        <View style={styles.rideHeader}>
          <View style={styles.rideDateContainer}>
            <Icon
              name={item.is_workout ? 'barbell-outline' : 'bicycle-outline'}
              size={16}
              color={colors.textSecondary}
              style={styles.rideIcon}
            />
            <Text style={[styles.rideDate, { color: colors.textPrimary }]}>
              {formatDate(item.date)}
            </Text>
          </View>
          {item.workout_name && (
            <View style={[styles.workoutBadge, { backgroundColor: colors.accent }]}>
              <Text
                style={[styles.workoutBadgeText, { color: colors.textInverse }]}
                numberOfLines={1}
              >
                {item.workout_name}
              </Text>
            </View>
          )}
        </View>

        {/* Stats row */}
        <View style={styles.rideStats}>
          <View style={styles.rideStat}>
            <Text style={[styles.rideStatValue, { color: colors.textPrimary }]}>
              {durationFormatted}
            </Text>
            <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>Duration</Text>
          </View>
          <View style={styles.rideStat}>
            <Text style={[styles.rideStatValue, { color: colors.textPrimary }]}>
              {distanceFormatted}
            </Text>
            <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>
              {distanceUnit}
            </Text>
          </View>
          <View style={styles.rideStat}>
            <Text style={[styles.rideStatValue, { color: colors.textPrimary }]}>
              {item.avg_power_watts}
            </Text>
            <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>Avg W</Text>
          </View>
        </View>

        {/* Chevron indicator */}
        <View style={styles.chevronContainer}>
          <Icon name="chevron-forward" size={20} color={colors.textSecondary} />
        </View>
      </TouchableOpacity>
    );
  };

  /**
   * Render loading indicator at bottom for infinite scroll
   */
  const renderFooter = () => {
    if (!isLoadingMore) return null;

    return (
      <View style={styles.footerLoader}>
        <ActivityIndicator size="small" color={colors.accent} />
        <Text style={[styles.footerText, { color: colors.textSecondary }]}>Loading more...</Text>
      </View>
    );
  };

  /**
   * Render empty state
   */
  const renderEmptyState = () => {
    // Don't show empty state while loading
    if (isLoading) {
      return (
        <View style={styles.centerContainer}>
          <LoadingSpinner size="large" message="Loading rides..." />
        </View>
      );
    }

    // Show error state
    if (error) {
      return (
        <View style={[styles.emptyState, { backgroundColor: colors.surface }]}>
          <Icon name="warning-outline" size={48} color={colors.error} style={styles.emptyIcon} />
          <Text style={[styles.emptyStateTitle, { color: colors.textPrimary }]}>
            Failed to Load Rides
          </Text>
          <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>{error}</Text>
          <TouchableOpacity
            style={[styles.retryButton, { backgroundColor: colors.accent }]}
            onPress={handleRefresh}
            accessibilityRole="button"
            accessibilityLabel="Retry loading rides"
          >
            <Text style={[styles.retryButtonText, { color: colors.textInverse }]}>Try Again</Text>
          </TouchableOpacity>
        </View>
      );
    }

    // Show disconnected state
    if (!isConnected) {
      return (
        <View style={[styles.emptyState, { backgroundColor: colors.surface }]}>
          <Icon
            name="cloud-offline-outline"
            size={48}
            color={colors.textSecondary}
            style={styles.emptyIcon}
          />
          <Text style={[styles.emptyStateTitle, { color: colors.textPrimary }]}>Not Connected</Text>
          <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>
            Connect to your desktop app to view ride history
          </Text>
          <TouchableOpacity
            style={[styles.retryButton, { backgroundColor: colors.accent }]}
            onPress={handleConnectPress}
            accessibilityRole="button"
            accessibilityLabel="Connect to desktop app"
          >
            <Text style={[styles.retryButtonText, { color: colors.textInverse }]}>Connect</Text>
          </TouchableOpacity>
        </View>
      );
    }

    // Show empty rides state
    return (
      <View style={[styles.emptyState, { backgroundColor: colors.surface }]}>
        <Icon
          name="bicycle-outline"
          size={48}
          color={colors.textSecondary}
          style={styles.emptyIcon}
        />
        <Text style={[styles.emptyStateTitle, { color: colors.textPrimary }]}>No Rides Yet</Text>
        <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>
          Complete a ride on your desktop app and it will appear here
        </Text>
      </View>
    );
  };

  /**
   * Render list header with ride count
   */
  const renderListHeader = () => {
    if (rides.length === 0) return null;

    return (
      <View style={styles.listHeader}>
        <Text style={[styles.listHeaderText, { color: colors.textSecondary }]}>
          {total > 0 ? `${total} rides` : `${rides.length} rides`}
        </Text>
      </View>
    );
  };

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['top']}
    >
      {/* Header with title and connection status */}
      <View style={[styles.header, { paddingHorizontal: spacing.md }]}>
        <Text
          style={[styles.title, typography.textStyles.screenTitle, { color: colors.textPrimary }]}
        >
          History
        </Text>
        <ConnectionStatus status={connectionStatus} variant="badge" animated />
      </View>

      {/* Ride list */}
      <FlatList
        data={rides}
        keyExtractor={item => item.id}
        renderItem={renderRideItem}
        contentContainerStyle={[
          styles.listContent,
          { paddingHorizontal: spacing.md, paddingBottom: spacing.xl },
          isEmpty && styles.listContentEmpty,
        ]}
        ListHeaderComponent={renderListHeader}
        ListEmptyComponent={renderEmptyState}
        ListFooterComponent={renderFooter}
        showsVerticalScrollIndicator={false}
        onEndReached={handleLoadMore}
        onEndReachedThreshold={0.5}
        refreshControl={
          <RefreshControl
            refreshing={isLoading && rides.length > 0}
            onRefresh={handleRefresh}
            tintColor={colors.accent}
            colors={[colors.accent]}
            enabled={isConnected}
          />
        }
        // Performance optimizations
        removeClippedSubviews={true}
        maxToRenderPerBatch={10}
        windowSize={10}
        initialNumToRender={10}
        getItemLayout={(_, index) => ({
          length: 110, // Approximate item height
          offset: 110 * index,
          index,
        })}
      />
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 12,
  },
  title: {
    // Typography from theme
  },
  listContent: {
    gap: 12,
    flexGrow: 1,
  },
  listContentEmpty: {
    flex: 1,
    justifyContent: 'center',
  },
  listHeader: {
    paddingVertical: 8,
  },
  listHeaderText: {
    fontSize: 14,
    fontWeight: '500',
  },
  rideCard: {
    padding: 16,
    borderRadius: 12,
    position: 'relative',
  },
  rideHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
    paddingRight: 24,
  },
  rideDateContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
  },
  rideIcon: {
    marginRight: 8,
  },
  rideDate: {
    fontSize: 16,
    fontWeight: '600',
  },
  workoutBadge: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
    marginLeft: 8,
    maxWidth: 120,
  },
  workoutBadgeText: {
    fontSize: 12,
    fontWeight: '500',
  },
  rideStats: {
    flexDirection: 'row',
    justifyContent: 'space-around',
    paddingRight: 24,
  },
  rideStat: {
    alignItems: 'center',
    minWidth: 70,
  },
  rideStatValue: {
    fontSize: 18,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  rideStatLabel: {
    fontSize: 12,
    marginTop: 4,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  chevronContainer: {
    position: 'absolute',
    right: 12,
    top: 0,
    bottom: 0,
    justifyContent: 'center',
  },
  centerContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: 48,
  },
  emptyState: {
    justifyContent: 'center',
    alignItems: 'center',
    padding: 32,
    borderRadius: 12,
    marginTop: 24,
  },
  emptyIcon: {
    marginBottom: 16,
  },
  emptyStateTitle: {
    fontSize: 18,
    fontWeight: '600',
    marginBottom: 8,
    textAlign: 'center',
  },
  emptyStateText: {
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
  footerLoader: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: 16,
    gap: 8,
  },
  footerText: {
    fontSize: 14,
  },
});
