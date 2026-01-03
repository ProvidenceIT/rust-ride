/**
 * HistoryFilterBar Component
 *
 * A horizontal scrollable bar with filter chips for filtering ride history.
 * Supports date range (week, month, year, custom) and workout type filters.
 */

import React, { useCallback } from 'react';
import {
  View,
  Text,
  ScrollView,
  TouchableOpacity,
  StyleSheet,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import {
  useHistoryStore,
  selectFilters,
  selectHasFiltersApplied,
  type DateRangeFilter,
  type RideTypeFilter,
} from '@/stores/historyStore';

/**
 * HistoryFilterBar props
 */
export interface HistoryFilterBarProps {
  /** Called when date range filter is pressed (for custom date picker) */
  onCustomDatePress?: () => void;
  /** Called when any filter changes */
  onFilterChange?: () => void;
}

/**
 * Date range filter options with labels
 */
const DATE_RANGE_OPTIONS: { value: DateRangeFilter; label: string }[] = [
  { value: 'all', label: 'All Time' },
  { value: 'week', label: 'This Week' },
  { value: 'month', label: 'This Month' },
  { value: 'year', label: 'This Year' },
  { value: 'custom', label: 'Custom' },
];

/**
 * Ride type filter options with labels and icons
 */
const RIDE_TYPE_OPTIONS: { value: RideTypeFilter; label: string; icon: string }[] = [
  { value: 'all', label: 'All Rides', icon: 'bicycle-outline' },
  { value: 'workout', label: 'Workouts', icon: 'barbell-outline' },
  { value: 'free_ride', label: 'Free Rides', icon: 'trail-sign-outline' },
];

/**
 * FilterChip - A single selectable filter chip
 */
interface FilterChipProps {
  label: string;
  selected: boolean;
  onPress: () => void;
  icon?: string;
  testID?: string;
}

function FilterChip({
  label,
  selected,
  onPress,
  icon,
  testID,
}: FilterChipProps): React.JSX.Element {
  const { colors, spacing, borderRadius } = useTheme();

  return (
    <TouchableOpacity
      style={[
        styles.chip,
        {
          backgroundColor: selected ? colors.accent : colors.surface,
          borderColor: selected ? colors.accent : colors.border,
          borderRadius: borderRadius.full,
          paddingHorizontal: spacing.md,
          paddingVertical: spacing.xs,
        },
      ]}
      onPress={onPress}
      activeOpacity={0.7}
      accessibilityRole="button"
      accessibilityState={{ selected }}
      accessibilityLabel={`${label}${selected ? ', selected' : ''}`}
      testID={testID}
    >
      {icon && (
        <Icon
          name={icon}
          size={14}
          color={selected ? colors.textInverse : colors.textSecondary}
          style={styles.chipIcon}
        />
      )}
      <Text
        style={[
          styles.chipText,
          {
            color: selected ? colors.textInverse : colors.textPrimary,
          },
        ]}
      >
        {label}
      </Text>
    </TouchableOpacity>
  );
}

/**
 * HistoryFilterBar Component
 *
 * Displays filter chips for date range and workout type.
 * Persists filter state in the history store.
 *
 * @example
 * ```tsx
 * <HistoryFilterBar
 *   onCustomDatePress={() => setShowDatePicker(true)}
 *   onFilterChange={() => refreshRides()}
 * />
 * ```
 */
export function HistoryFilterBar({
  onCustomDatePress,
  onFilterChange,
}: HistoryFilterBarProps): React.JSX.Element {
  const { colors, spacing } = useTheme();

  // Get filter state from store
  const filters = useHistoryStore(selectFilters);
  const hasFiltersApplied = useHistoryStore(selectHasFiltersApplied);
  const setDateRangeFilter = useHistoryStore(state => state.setDateRangeFilter);
  const setRideTypeFilter = useHistoryStore(state => state.setRideTypeFilter);
  const clearFilters = useHistoryStore(state => state.clearFilters);

  /**
   * Handle date range filter selection
   */
  const handleDateRangeSelect = useCallback(
    (range: DateRangeFilter) => {
      if (range === 'custom') {
        // Open custom date picker modal
        onCustomDatePress?.();
      } else {
        setDateRangeFilter(range);
        onFilterChange?.();
      }
    },
    [setDateRangeFilter, onCustomDatePress, onFilterChange]
  );

  /**
   * Handle ride type filter selection
   */
  const handleRideTypeSelect = useCallback(
    (type: RideTypeFilter) => {
      setRideTypeFilter(type);
      onFilterChange?.();
    },
    [setRideTypeFilter, onFilterChange]
  );

  /**
   * Handle clear all filters
   */
  const handleClearFilters = useCallback(() => {
    clearFilters();
    onFilterChange?.();
  }, [clearFilters, onFilterChange]);

  /**
   * Get date range label (with custom date display)
   */
  const getDateRangeLabel = (option: typeof DATE_RANGE_OPTIONS[0]): string => {
    if (
      option.value === 'custom' &&
      filters.dateRange === 'custom' &&
      filters.customStartDate &&
      filters.customEndDate
    ) {
      // Format custom dates for display
      const startDate = new Date(filters.customStartDate);
      const endDate = new Date(filters.customEndDate);
      const formatDate = (date: Date) =>
        date.toLocaleDateString([], { month: 'short', day: 'numeric' });
      return `${formatDate(startDate)} - ${formatDate(endDate)}`;
    }
    return option.label;
  };

  return (
    <View style={styles.container}>
      {/* Filter sections */}
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        contentContainerStyle={[
          styles.scrollContent,
          { paddingHorizontal: spacing.md },
        ]}
      >
        {/* Date range filters */}
        <View style={styles.section}>
          <Text style={[styles.sectionLabel, { color: colors.textSecondary }]}>
            Date
          </Text>
          <View style={styles.chipRow}>
            {DATE_RANGE_OPTIONS.map(option => (
              <FilterChip
                key={option.value}
                label={getDateRangeLabel(option)}
                selected={filters.dateRange === option.value}
                onPress={() => handleDateRangeSelect(option.value)}
                testID={`date-filter-${option.value}`}
              />
            ))}
          </View>
        </View>

        {/* Separator */}
        <View
          style={[
            styles.separator,
            { backgroundColor: colors.border, marginHorizontal: spacing.md },
          ]}
        />

        {/* Ride type filters */}
        <View style={styles.section}>
          <Text style={[styles.sectionLabel, { color: colors.textSecondary }]}>
            Type
          </Text>
          <View style={styles.chipRow}>
            {RIDE_TYPE_OPTIONS.map(option => (
              <FilterChip
                key={option.value}
                label={option.label}
                selected={filters.rideType === option.value}
                onPress={() => handleRideTypeSelect(option.value)}
                icon={option.icon}
                testID={`type-filter-${option.value}`}
              />
            ))}
          </View>
        </View>

        {/* Clear filters button (only shown when filters are applied) */}
        {hasFiltersApplied && (
          <>
            <View
              style={[
                styles.separator,
                { backgroundColor: colors.border, marginHorizontal: spacing.md },
              ]}
            />
            <TouchableOpacity
              style={[
                styles.clearButton,
                {
                  backgroundColor: colors.error + '20',
                  borderRadius: 16,
                  paddingHorizontal: spacing.md,
                  paddingVertical: spacing.xs,
                },
              ]}
              onPress={handleClearFilters}
              accessibilityRole="button"
              accessibilityLabel="Clear all filters"
              testID="clear-filters-button"
            >
              <Icon name="close-circle-outline" size={14} color={colors.error} />
              <Text style={[styles.clearButtonText, { color: colors.error }]}>
                Clear
              </Text>
            </TouchableOpacity>
          </>
        )}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    // Container for the filter bar
  },
  scrollContent: {
    paddingVertical: 12,
    alignItems: 'center',
  },
  section: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  sectionLabel: {
    fontSize: 12,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginRight: 8,
  },
  chipRow: {
    flexDirection: 'row',
    gap: 6,
  },
  chip: {
    flexDirection: 'row',
    alignItems: 'center',
    borderWidth: 1,
  },
  chipIcon: {
    marginRight: 4,
  },
  chipText: {
    fontSize: 13,
    fontWeight: '500',
  },
  separator: {
    width: 1,
    height: 24,
  },
  clearButton: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
  },
  clearButtonText: {
    fontSize: 13,
    fontWeight: '500',
  },
});
