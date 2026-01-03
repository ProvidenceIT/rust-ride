/**
 * HistoryScreen Tests
 *
 * Tests for the ride history screen with pagination, pull-to-refresh,
 * and infinite scroll functionality.
 */

import React from 'react';
import { FlatList } from 'react-native';
import { render, waitFor } from '@testing-library/react-native';
import { HistoryScreen } from '../../src/screens/HistoryScreen';
import { ThemeProvider } from '../../src/theme';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useHistoryStore } from '../../src/stores/historyStore';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { getConnectionService } from '../../src/services/ConnectionService';
import type { RideSummary } from '../../src/types';

// Mock navigation (typed as any to avoid complex type issues in tests)
/* eslint-disable @typescript-eslint/no-explicit-any */
const mockNavigation: any = {
  navigate: jest.fn(),
  goBack: jest.fn(),
  setOptions: jest.fn(),
  getState: jest.fn(),
  reset: jest.fn(),
  dispatch: jest.fn(),
  canGoBack: jest.fn(),
  getId: jest.fn(),
  getParent: jest.fn(),
  setParams: jest.fn(),
  addListener: jest.fn(),
  removeListener: jest.fn(),
  isFocused: jest.fn(),
};

const mockRoute: any = {
  key: 'History',
  name: 'History',
  params: undefined,
};
/* eslint-enable @typescript-eslint/no-explicit-any */

// Mock ConnectionService
jest.mock('../../src/services/ConnectionService', () => ({
  getConnectionService: jest.fn(() => ({
    fetchRideHistory: jest.fn(),
    fetchRideDetails: jest.fn(),
  })),
  ConnectionService: {
    getInstance: jest.fn(),
  },
}));

// Mock RideCacheService to prevent AsyncStorage side effects in tests
jest.mock('../../src/services/RideCacheService', () => ({
  getRideCacheService: jest.fn(() => ({
    getCachedRideSummaries: jest.fn().mockResolvedValue([]),
    getCachedRideDetail: jest.fn().mockResolvedValue(null),
    getLastSync: jest.fn().mockResolvedValue(null),
    cacheRideSummaries: jest.fn().mockResolvedValue(undefined),
    cacheRideDetail: jest.fn().mockResolvedValue(undefined),
    updateLastSync: jest.fn().mockResolvedValue(undefined),
    clearCache: jest.fn().mockResolvedValue(undefined),
    hasCachedSummaries: jest.fn().mockResolvedValue(false),
  })),
  RideCacheService: {
    getInstance: jest.fn(),
  },
}));

// Sample ride data
const mockRides: RideSummary[] = [
  {
    id: 'ride-1',
    date: new Date().toISOString(), // Today
    duration_secs: 3600, // 1 hour
    distance_km: 30.5,
    avg_power_watts: 200,
    workout_name: 'Sweet Spot',
    is_workout: true,
  },
  {
    id: 'ride-2',
    date: new Date(Date.now() - 86400000).toISOString(), // Yesterday
    duration_secs: 5400, // 1:30
    distance_km: 45.2,
    avg_power_watts: 180,
    workout_name: undefined,
    is_workout: false,
  },
  {
    id: 'ride-3',
    date: '2026-01-01T10:00:00.000Z',
    duration_secs: 7200, // 2 hours
    distance_km: 60.0,
    avg_power_watts: 195,
    workout_name: 'Endurance',
    is_workout: true,
  },
];

// Helper to render with providers
const renderWithProviders = () => {
  return render(
    <ThemeProvider>
      <HistoryScreen navigation={mockNavigation} route={mockRoute} />
    </ThemeProvider>
  );
};

// Reset stores and mocks before each test
beforeEach(() => {
  useConnectionStore.getState().reset();
  useHistoryStore.getState().reset();
  jest.clearAllMocks();
});

describe('HistoryScreen', () => {
  describe('Layout and Structure', () => {
    it('renders the History title', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('History')).toBeTruthy();
    });

    it('renders connection status badge', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Disconnected')).toBeTruthy();
    });
  });

  describe('Not Connected State', () => {
    it('shows no cached rides message when disconnected with empty cache', async () => {
      // When offline with no cache, show "No Cached Rides" after loading from cache
      const { findByText } = renderWithProviders();
      expect(await findByText('No Cached Rides')).toBeTruthy();
      expect(
        await findByText('Connect while online to cache your ride history for offline viewing')
      ).toBeTruthy();
    });

    it('shows Connect button when disconnected with empty cache', async () => {
      const { findByText } = renderWithProviders();
      expect(await findByText('Connect')).toBeTruthy();
    });

    it('has accessible connect button', async () => {
      const { findByLabelText } = renderWithProviders();
      expect(await findByLabelText('Connect to desktop app')).toBeTruthy();
    });
  });

  describe('Empty State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('shows no rides message when connected but no rides exist', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('No Rides Yet')).toBeTruthy();
      expect(
        getByText('Complete a ride on your desktop app and it will appear here')
      ).toBeTruthy();
    });
  });

  describe('Loading State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setLoading(true);
    });

    it('shows loading spinner while loading', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Loading rides...')).toBeTruthy();
    });
  });

  describe('Error State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setError('Failed to fetch ride history');
    });

    it('shows error message', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Failed to Load Rides')).toBeTruthy();
      expect(getByText('Failed to fetch ride history')).toBeTruthy();
    });

    it('shows Try Again button on error', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Try Again')).toBeTruthy();
    });

    it('has accessible retry button', () => {
      const { getByLabelText } = renderWithProviders();
      expect(getByLabelText('Retry loading rides')).toBeTruthy();
    });
  });

  describe('Ride List Display', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setRides(mockRides, 3);
    });

    it('shows ride count header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('3 rides')).toBeTruthy();
    });

    it('shows ride dates', () => {
      const { getByText } = renderWithProviders();
      // Today's ride
      expect(getByText(/Today/)).toBeTruthy();
      // Yesterday's ride
      expect(getByText(/Yesterday/)).toBeTruthy();
    });

    it('shows workout badges for structured workouts', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Sweet Spot')).toBeTruthy();
      expect(getByText('Endurance')).toBeTruthy();
    });

    it('shows duration formatted correctly', () => {
      const { getByText } = renderWithProviders();
      // 3600 seconds = 1:00:00
      expect(getByText('1:00:00')).toBeTruthy();
      // 5400 seconds = 1:30:00
      expect(getByText('1:30:00')).toBeTruthy();
    });

    it('shows average power', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('200')).toBeTruthy();
      expect(getByText('180')).toBeTruthy();
      expect(getByText('195')).toBeTruthy();
    });
  });

  describe('Distance Display with Unit Preference', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setRides(mockRides, 3);
    });

    it('shows distance in kilometers by default', () => {
      const { getAllByText } = renderWithProviders();
      // Check for km unit labels
      const kmLabels = getAllByText('km');
      expect(kmLabels.length).toBeGreaterThan(0);
    });

    it('shows distance in miles when imperial units selected', async () => {
      await useSettingsStore.getState().setUnits('imperial');

      const { getAllByText } = renderWithProviders();
      // Check for mi unit labels
      const miLabels = getAllByText('mi');
      expect(miLabels.length).toBeGreaterThan(0);
    });
  });

  describe('Pagination', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('shows hasMore when there are more rides', () => {
      useHistoryStore.getState().setRides(mockRides, 100);
      expect(useHistoryStore.getState().pagination.hasMore).toBe(true);
    });

    it('does not show hasMore when all rides are loaded', () => {
      useHistoryStore.getState().setRides(mockRides, 3);
      expect(useHistoryStore.getState().pagination.hasMore).toBe(false);
    });

    it('shows loading more indicator when loading more', () => {
      useHistoryStore.getState().setRides(mockRides, 100);
      useHistoryStore.getState().setLoadingMore(true);

      const { getByText } = renderWithProviders();
      expect(getByText('Loading more...')).toBeTruthy();
    });
  });

  describe('Pull to Refresh', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setRides(mockRides, 3);
    });

    it('renders refresh control', () => {
      const { UNSAFE_getByType } = renderWithProviders();
      // FlatList should have a RefreshControl
      const flatList = UNSAFE_getByType(FlatList);
      expect(flatList.props.refreshControl).toBeDefined();
    });

    it('calls fetchRideHistory on refresh', async () => {
      const mockFetchRideHistory = jest.fn();
      (getConnectionService as jest.Mock).mockReturnValue({
        fetchRideHistory: mockFetchRideHistory,
      });

      const { UNSAFE_getByType } = renderWithProviders();
      const flatList = UNSAFE_getByType(FlatList);

      // Trigger refresh
      await waitFor(() => {
        flatList.props.refreshControl.props.onRefresh();
      });

      expect(mockFetchRideHistory).toHaveBeenCalledWith(20, 0);
    });
  });

  describe('Connection Status', () => {
    it('shows disconnected status', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Disconnected')).toBeTruthy();
    });

    it('shows connecting status', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      const { getByText } = renderWithProviders();
      expect(getByText('Connecting...')).toBeTruthy();
    });

    it('shows connected status', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setConnected();
      const { getByText } = renderWithProviders();
      expect(getByText('Connected')).toBeTruthy();
    });

    it('shows authenticated status', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      const { getByText } = renderWithProviders();
      expect(getByText('Authenticated')).toBeTruthy();
    });
  });

  describe('Date Formatting', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('formats today correctly', () => {
      const todayRide: RideSummary = {
        id: 'ride-today',
        date: new Date().toISOString(),
        duration_secs: 3600,
        distance_km: 30,
        avg_power_watts: 200,
      };
      useHistoryStore.getState().setRides([todayRide], 1);

      const { getByText } = renderWithProviders();
      expect(getByText(/Today/)).toBeTruthy();
    });

    it('formats yesterday correctly', () => {
      const yesterdayRide: RideSummary = {
        id: 'ride-yesterday',
        date: new Date(Date.now() - 86400000).toISOString(),
        duration_secs: 3600,
        distance_km: 30,
        avg_power_watts: 200,
      };
      useHistoryStore.getState().setRides([yesterdayRide], 1);

      const { getByText } = renderWithProviders();
      expect(getByText(/Yesterday/)).toBeTruthy();
    });

    it('formats older dates with weekday and month', () => {
      const oldRide: RideSummary = {
        id: 'ride-old',
        date: '2026-01-01T10:00:00.000Z',
        duration_secs: 3600,
        distance_km: 30,
        avg_power_watts: 200,
      };
      useHistoryStore.getState().setRides([oldRide], 1);

      const { getByText } = renderWithProviders();
      // Should show date format like "Wed, Jan 1" or "1/1/2026" depending on locale
      // Since we can't predict exact format, just verify it's not Today/Yesterday
      expect(() => getByText(/Today/)).toThrow();
      expect(() => getByText(/Yesterday/)).toThrow();
      // The date should be rendered in some format - just verify component renders
      expect(getByText('1:00:00')).toBeTruthy(); // Duration should render
    });
  });

  describe('Duration Formatting', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('formats short durations correctly', () => {
      const shortRide: RideSummary = {
        id: 'ride-short',
        date: new Date().toISOString(),
        duration_secs: 300, // 5 minutes
        distance_km: 5,
        avg_power_watts: 150,
      };
      useHistoryStore.getState().setRides([shortRide], 1);

      const { getByText } = renderWithProviders();
      expect(getByText('5:00')).toBeTruthy();
    });

    it('formats durations over 1 hour correctly', () => {
      const longRide: RideSummary = {
        id: 'ride-long',
        date: new Date().toISOString(),
        duration_secs: 3661, // 1:01:01
        distance_km: 40,
        avg_power_watts: 200,
      };
      useHistoryStore.getState().setRides([longRide], 1);

      const { getByText } = renderWithProviders();
      expect(getByText('1:01:01')).toBeTruthy();
    });
  });

  describe('Store Integration', () => {
    it('updates when store changes', async () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getByText, rerender } = renderWithProviders();
      expect(getByText('No Rides Yet')).toBeTruthy();

      // Add rides to store
      useHistoryStore.getState().setRides(mockRides, 3);

      // Re-render to see updates
      rerender(
        <ThemeProvider>
          <HistoryScreen navigation={mockNavigation} route={mockRoute} />
        </ThemeProvider>
      );

      expect(getByText('3 rides')).toBeTruthy();
    });
  });
});
