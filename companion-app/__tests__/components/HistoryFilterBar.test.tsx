/**
 * HistoryFilterBar Tests
 *
 * Tests for the history filter bar component with date range and workout type filters.
 */

import React from 'react';
import { render, fireEvent, act } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { HistoryFilterBar } from '../../src/components';
import { useHistoryStore } from '../../src/stores/historyStore';

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('HistoryFilterBar', () => {
  const mockOnCustomDatePress = jest.fn();
  const mockOnFilterChange = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    // Reset history store to initial state
    act(() => {
      useHistoryStore.getState().clearFilters();
    });
  });

  describe('Rendering', () => {
    it('renders date range filter chips', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      expect(getByTestId('date-filter-all')).toBeTruthy();
      expect(getByTestId('date-filter-week')).toBeTruthy();
      expect(getByTestId('date-filter-month')).toBeTruthy();
      expect(getByTestId('date-filter-year')).toBeTruthy();
      expect(getByTestId('date-filter-custom')).toBeTruthy();
    });

    it('renders ride type filter chips', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      expect(getByTestId('type-filter-all')).toBeTruthy();
      expect(getByTestId('type-filter-workout')).toBeTruthy();
      expect(getByTestId('type-filter-free_ride')).toBeTruthy();
    });

    it('shows "All Time" as selected by default', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      const allTimeChip = getByTestId('date-filter-all');
      expect(allTimeChip.props.accessibilityState.selected).toBe(true);
    });

    it('shows "All Rides" as selected by default for type filter', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      const allRidesChip = getByTestId('type-filter-all');
      expect(allRidesChip.props.accessibilityState.selected).toBe(true);
    });

    it('does not show clear button when no filters applied', () => {
      const { queryByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      expect(queryByTestId('clear-filters-button')).toBeNull();
    });
  });

  describe('Date Range Filter', () => {
    it('updates store when week filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('date-filter-week'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.dateRange).toBe('week');
    });

    it('updates store when month filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('date-filter-month'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.dateRange).toBe('month');
    });

    it('updates store when year filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('date-filter-year'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.dateRange).toBe('year');
    });

    it('calls onCustomDatePress when custom filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('date-filter-custom'));

      expect(mockOnCustomDatePress).toHaveBeenCalled();
    });

    it('calls onFilterChange when date filter changes', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('date-filter-month'));

      expect(mockOnFilterChange).toHaveBeenCalled();
    });
  });

  describe('Ride Type Filter', () => {
    it('updates store when workout filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('type-filter-workout'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.rideType).toBe('workout');
    });

    it('updates store when free ride filter is selected', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('type-filter-free_ride'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.rideType).toBe('free_ride');
    });

    it('calls onFilterChange when type filter changes', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('type-filter-workout'));

      expect(mockOnFilterChange).toHaveBeenCalled();
    });
  });

  describe('Clear Filters', () => {
    it('shows clear button when filters are applied', () => {
      // Apply a filter first
      act(() => {
        useHistoryStore.getState().setDateRangeFilter('week');
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      expect(getByTestId('clear-filters-button')).toBeTruthy();
    });

    it('clears all filters when clear button is pressed', () => {
      // Apply filters first
      act(() => {
        useHistoryStore.getState().setDateRangeFilter('month');
        useHistoryStore.getState().setRideTypeFilter('workout');
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('clear-filters-button'));

      const filters = useHistoryStore.getState().filters;
      expect(filters.dateRange).toBe('all');
      expect(filters.rideType).toBe('all');
    });

    it('calls onFilterChange when filters are cleared', () => {
      // Apply a filter first
      act(() => {
        useHistoryStore.getState().setDateRangeFilter('week');
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      fireEvent.press(getByTestId('clear-filters-button'));

      expect(mockOnFilterChange).toHaveBeenCalled();
    });
  });

  describe('Accessibility', () => {
    it('has proper accessibility role for filter chips', () => {
      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      const weekChip = getByTestId('date-filter-week');
      expect(weekChip.props.accessibilityRole).toBe('button');
    });

    it('has proper accessibility state for selected chips', () => {
      act(() => {
        useHistoryStore.getState().setDateRangeFilter('month');
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      const monthChip = getByTestId('date-filter-month');
      expect(monthChip.props.accessibilityState.selected).toBe(true);

      const allChip = getByTestId('date-filter-all');
      expect(allChip.props.accessibilityState.selected).toBe(false);
    });

    it('has accessible label for clear button', () => {
      act(() => {
        useHistoryStore.getState().setDateRangeFilter('week');
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      const clearButton = getByTestId('clear-filters-button');
      expect(clearButton.props.accessibilityLabel).toBe('Clear all filters');
    });
  });

  describe('Custom Date Display', () => {
    it('shows custom date range chip as selected when set', () => {
      // Set custom date range
      act(() => {
        useHistoryStore.getState().setDateRangeFilter(
          'custom',
          '2024-01-15T12:00:00.000Z',
          '2024-01-30T12:00:00.000Z',
        );
      });

      const { getByTestId } = renderWithTheme(
        <HistoryFilterBar
          onCustomDatePress={mockOnCustomDatePress}
          onFilterChange={mockOnFilterChange}
        />,
      );

      // Custom filter chip should be selected
      const customChip = getByTestId('date-filter-custom');
      expect(customChip.props.accessibilityState.selected).toBe(true);
    });
  });
});
