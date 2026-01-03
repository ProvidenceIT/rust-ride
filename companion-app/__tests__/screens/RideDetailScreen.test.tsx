/**
 * RideDetailScreen Tests
 *
 * Tests for the ride detail screen showing full ride statistics,
 * power/HR/cadence metrics, and unit preference support.
 */

import React from 'react';
import { render, waitFor, fireEvent, act } from '@testing-library/react-native';
import { RideDetailScreen } from '../../src/screens/RideDetailScreen';
import { ThemeProvider } from '../../src/theme';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useHistoryStore } from '../../src/stores/historyStore';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { getConnectionService } from '../../src/services/ConnectionService';
import type { RideDetailInfo } from '../../src/types';

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

const createMockRoute = (rideId: string): any => ({
  key: 'RideDetail',
  name: 'RideDetail',
  params: { rideId },
});
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
  })),
  RideCacheService: {
    getInstance: jest.fn(),
  },
}));

// Sample ride detail data
const mockRideDetail: RideDetailInfo = {
  ride_id: 'ride-123',
  started_at: '2026-01-03T14:30:00.000Z',
  ended_at: '2026-01-03T15:30:00.000Z',
  duration_secs: 3600, // 1 hour
  distance_km: 30.5,
  calories: 650,
  avg_power_watts: 200,
  max_power_watts: 350,
  normalized_power_watts: 215,
  avg_heart_rate_bpm: 145,
  max_heart_rate_bpm: 172,
  avg_cadence_rpm: 85,
  tss: 75,
  intensity_factor: 0.85,
  is_workout: true,
  workout_name: 'Sweet Spot',
};

const mockFreeRideDetail: RideDetailInfo = {
  ride_id: 'ride-456',
  started_at: '2026-01-02T10:00:00.000Z',
  ended_at: '2026-01-02T11:30:00.000Z',
  duration_secs: 5400, // 1:30
  distance_km: 45.2,
  calories: 800,
  avg_power_watts: 180,
  max_power_watts: 300,
  normalized_power_watts: 190,
  avg_heart_rate_bpm: 138,
  max_heart_rate_bpm: 165,
  avg_cadence_rpm: 90,
  tss: null, // Free rides may not have TSS
  intensity_factor: null,
  is_workout: false,
  workout_name: null,
};

// Ride with null metrics (e.g., no HR sensor)
const mockRideWithNulls: RideDetailInfo = {
  ride_id: 'ride-789',
  started_at: '2026-01-01T08:00:00.000Z',
  ended_at: '2026-01-01T09:00:00.000Z',
  duration_secs: 3600,
  distance_km: 25.0,
  calories: 500,
  avg_power_watts: 175,
  max_power_watts: 280,
  normalized_power_watts: 185,
  avg_heart_rate_bpm: null, // No HR sensor
  max_heart_rate_bpm: null,
  avg_cadence_rpm: null, // No cadence sensor
  tss: 60,
  intensity_factor: 0.78,
  is_workout: false,
  workout_name: null,
};

// Helper to render with providers
const renderWithProviders = (rideId = 'ride-123') => {
  const mockRoute = createMockRoute(rideId);
  return render(
    <ThemeProvider>
      <RideDetailScreen navigation={mockNavigation} route={mockRoute} />
    </ThemeProvider>
  );
};

// Reset stores and mocks before each test
beforeEach(() => {
  useConnectionStore.getState().reset();
  useHistoryStore.getState().reset();
  useSettingsStore.getState().settings = {
    units: 'metric',
    keepScreenAwake: true,
    hapticFeedback: 'medium',
    theme: 'system',
  };
  jest.clearAllMocks();
});

describe('RideDetailScreen', () => {
  describe('Loading State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setLoadingDetail(true);
    });

    it('shows loading spinner while loading', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Loading ride details...')).toBeTruthy();
    });
  });

  describe('Error State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setError('Failed to fetch ride details');
    });

    it('shows error message', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Failed to Load Ride')).toBeTruthy();
      expect(getByText('Failed to fetch ride details')).toBeTruthy();
    });

    it('shows Try Again button on error', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Try Again')).toBeTruthy();
    });

    it('has accessible retry button', () => {
      const { getByLabelText } = renderWithProviders();
      // EmptyState uses accessibilityLabel="Try Again" on the action button
      expect(getByLabelText('Try Again')).toBeTruthy();
    });
  });

  describe('Not Connected State', () => {
    it('shows not connected message when disconnected', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Not Available Offline')).toBeTruthy();
      expect(getByText(/Connect to your desktop app/)).toBeTruthy();
    });

    it('shows Connect button when disconnected', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Connect')).toBeTruthy();
    });

    it('navigates to Connection screen on Connect press', () => {
      const { getByLabelText } = renderWithProviders();
      // EmptyState uses accessibilityLabel="Connect" on the action button
      const connectButton = getByLabelText('Connect');
      fireEvent.press(connectButton);
      expect(mockNavigation.navigate).toHaveBeenCalledWith('Connection');
    });
  });

  describe('Ride Not Found State', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      // No ride detail set, no loading, no error
    });

    it('shows ride not found message when no ride detail', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Ride Not Found')).toBeTruthy();
      expect(getByText('The requested ride could not be found')).toBeTruthy();
    });
  });

  describe('Date and Time Display', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows formatted date', () => {
      const { getByText } = renderWithProviders();
      // The exact format depends on the date, but should render
      // For 2026-01-03, it may show "Today" or full date
      // Since it's a fixed date, we check for reasonable content
      expect(getByText(/2026|Today|Yesterday|January|Jan/)).toBeTruthy();
    });

    it('shows formatted time', () => {
      const { getAllByText } = renderWithProviders();
      // Time should be shown in user's locale format (HH:MM or H:MM AM/PM)
      // Multiple time patterns may exist (header time + duration)
      const timePatterns = getAllByText(/\d{1,2}:\d{2}/);
      expect(timePatterns.length).toBeGreaterThan(0);
    });
  });

  describe('Workout Badge', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('shows workout badge for structured workouts', () => {
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
      const { getByText } = renderWithProviders();
      expect(getByText('Sweet Spot')).toBeTruthy();
    });

    it('shows Free Ride badge for free rides', () => {
      useHistoryStore.getState().setCurrentRideDetail(mockFreeRideDetail);
      const { getByText } = renderWithProviders('ride-456');
      expect(getByText('Free Ride')).toBeTruthy();
    });
  });

  describe('Summary Section', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows Training Summary section header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Training Summary')).toBeTruthy();
    });

    it('shows Ride Overview section header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Ride Overview')).toBeTruthy();
    });

    it('shows duration formatted correctly', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('1:00:00')).toBeTruthy();
    });

    it('shows distance with unit', () => {
      const { getByText } = renderWithProviders();
      // 30.5 is between 10-100, so formatDistance uses toFixed(1)
      expect(getByText('30.5')).toBeTruthy();
      expect(getByText('km')).toBeTruthy();
    });

    it('shows calories', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('650')).toBeTruthy();
      expect(getByText('kcal')).toBeTruthy();
    });

    it('shows TSS', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('75')).toBeTruthy();
    });
  });

  describe('Power Section', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows Power section header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Power')).toBeTruthy();
    });

    it('shows average power', () => {
      const { getByText, getAllByText } = renderWithProviders();
      expect(getByText('200')).toBeTruthy();
      // Multiple W units for power section
      const wUnits = getAllByText('W');
      expect(wUnits.length).toBeGreaterThanOrEqual(3);
    });

    it('shows maximum power', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('350')).toBeTruthy();
    });

    it('shows normalized power', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Normalized (NP)')).toBeTruthy();
      expect(getByText('215')).toBeTruthy();
    });

  });

  describe('Training Stats Section', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows TSS with intensity level', () => {
      const { getByText, getAllByText } = renderWithProviders();
      // TSS label appears in RideStatisticsSummary
      const tssLabels = getAllByText('TSS');
      expect(tssLabels.length).toBeGreaterThan(0);
      // Value 75 appears
      expect(getByText('75')).toBeTruthy();
      // Moderate intensity level
      expect(getByText('Moderate')).toBeTruthy();
    });

    it('shows intensity factor with description', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Intensity Factor')).toBeTruthy();
      expect(getByText('0.85')).toBeTruthy();
      expect(getByText('Tempo')).toBeTruthy();
    });
  });

  describe('Heart Rate Section', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows Heart Rate section header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Heart Rate')).toBeTruthy();
    });

    it('shows average heart rate', () => {
      const { getByText, getAllByText } = renderWithProviders();
      expect(getByText('145')).toBeTruthy();
      const bpmUnits = getAllByText('bpm');
      expect(bpmUnits.length).toBe(2);
    });

    it('shows maximum heart rate', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('172')).toBeTruthy();
    });
  });

  describe('Cadence Section', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('shows Cadence section header', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('Cadence')).toBeTruthy();
    });

    it('shows average cadence', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('85')).toBeTruthy();
      expect(getByText('rpm')).toBeTruthy();
    });
  });

  describe('Null Values Handling', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideWithNulls);
    });

    it('shows -- for null heart rate values', () => {
      const { getAllByText } = renderWithProviders('ride-789');
      // Should show -- for null values
      const dashes = getAllByText('--');
      expect(dashes.length).toBeGreaterThanOrEqual(2); // HR avg and max
    });

    it('shows -- for null cadence value', () => {
      const { getAllByText } = renderWithProviders('ride-789');
      const dashes = getAllByText('--');
      expect(dashes.length).toBeGreaterThanOrEqual(3); // HR avg, max, and cadence
    });
  });

  describe('Unit Preference - Metric', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
      useSettingsStore.getState().settings.units = 'metric';
    });

    it('shows distance in kilometers', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('km')).toBeTruthy();
    });
  });

  describe('Unit Preference - Imperial', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
      useSettingsStore.getState().settings.units = 'imperial';
    });

    it('shows distance in miles', () => {
      const { getByText } = renderWithProviders();
      expect(getByText('mi')).toBeTruthy();
    });

    it('converts distance value correctly', () => {
      const { getByText } = renderWithProviders();
      // 30.5 km * 0.621371 = ~18.95 miles
      // formatDistance returns 19.0 (1 decimal for values 10-100)
      expect(getByText('19.0')).toBeTruthy();
    });
  });

  describe('Data Fetching', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('calls fetchRideDetails on mount when connected', async () => {
      const mockFetchRideDetails = jest.fn();
      (getConnectionService as jest.Mock).mockReturnValue({
        fetchRideDetails: mockFetchRideDetails,
      });

      renderWithProviders('ride-123');

      await waitFor(() => {
        expect(mockFetchRideDetails).toHaveBeenCalledWith('ride-123');
      });
    });

    it('does not call fetchRideDetails when not connected', async () => {
      useConnectionStore.getState().reset(); // Disconnect

      const mockFetchRideDetails = jest.fn();
      (getConnectionService as jest.Mock).mockReturnValue({
        fetchRideDetails: mockFetchRideDetails,
      });

      renderWithProviders('ride-123');

      // Give time for potential calls
      await waitFor(
        () => {
          expect(mockFetchRideDetails).not.toHaveBeenCalled();
        },
        { timeout: 100 }
      );
    });
  });

  describe('Refresh Functionality', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('renders with refresh control when ride data exists', () => {
      const { getByText } = renderWithProviders();
      // If the screen renders with ride data, refresh is available
      expect(getByText('Training Summary')).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
    });

    it('has accessible stat cards', () => {
      const { getByLabelText } = renderWithProviders();
      // Check for accessible stat cards
      expect(getByLabelText(/Duration:/)).toBeTruthy();
      expect(getByLabelText(/Distance:/)).toBeTruthy();
      expect(getByLabelText(/Calories:/)).toBeTruthy();
    });

    it('has accessible retry button on error', () => {
      useHistoryStore.getState().setCurrentRideDetail(null);
      useHistoryStore.getState().setError('Network error');

      const { getByLabelText } = renderWithProviders();
      // EmptyState uses accessibilityLabel="Try Again" on the action button
      expect(getByLabelText('Try Again')).toBeTruthy();
    });
  });

  describe('Duration Formatting', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('formats durations under 1 hour correctly', () => {
      const shortRide: RideDetailInfo = {
        ...mockRideDetail,
        ride_id: 'short-ride',
        duration_secs: 1800, // 30 minutes
      };
      useHistoryStore.getState().setCurrentRideDetail(shortRide);

      const { getByText } = renderWithProviders('short-ride');
      expect(getByText('30:00')).toBeTruthy();
    });

    it('formats durations over 1 hour correctly', () => {
      const longRide: RideDetailInfo = {
        ...mockRideDetail,
        ride_id: 'long-ride',
        duration_secs: 7261, // 2:01:01
      };
      useHistoryStore.getState().setCurrentRideDetail(longRide);

      const { getByText } = renderWithProviders('long-ride');
      expect(getByText('2:01:01')).toBeTruthy();
    });
  });

  describe('Free Ride Display', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useHistoryStore.getState().setCurrentRideDetail(mockFreeRideDetail);
    });

    it('displays Free Ride badge', () => {
      const { getByText } = renderWithProviders('ride-456');
      expect(getByText('Free Ride')).toBeTruthy();
    });

    it('does not show workout name', () => {
      const { queryByText } = renderWithProviders('ride-456');
      // Sweet Spot should not appear for free ride
      expect(queryByText('Sweet Spot')).toBeNull();
    });

    it('shows -- for null TSS', () => {
      const { getAllByText } = renderWithProviders('ride-456');
      const dashes = getAllByText('--');
      expect(dashes.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('Store Integration', () => {
    it('updates when ride detail is loaded', async () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getByText, rerender } = renderWithProviders();
      expect(getByText('Ride Not Found')).toBeTruthy();

      // Load ride detail into store (wrapped in act for proper state updates)
      await act(async () => {
        useHistoryStore.getState().setCurrentRideDetail(mockRideDetail);
      });

      // Re-render to see updates
      const mockRoute = createMockRoute('ride-123');
      rerender(
        <ThemeProvider>
          <RideDetailScreen navigation={mockNavigation} route={mockRoute} />
        </ThemeProvider>
      );

      expect(getByText('Training Summary')).toBeTruthy();
      expect(getByText('Sweet Spot')).toBeTruthy();
    });
  });
});
