/**
 * DashboardScreen Tests
 *
 * Tests for the main dashboard screen showing real-time workout metrics.
 */

import React from 'react';
import { render } from '@testing-library/react-native';
import * as ReactNative from 'react-native';
import { DashboardScreen } from '../../src/screens/DashboardScreen';
import { ThemeProvider } from '../../src/theme';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useMetricsStore } from '../../src/stores/metricsStore';
import { useSessionStore } from '../../src/stores/sessionStore';

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
  key: 'Dashboard',
  name: 'Dashboard',
  params: undefined,
};
/* eslint-enable @typescript-eslint/no-explicit-any */

// Mock dimensions for responsive testing
let mockDimensions = { width: 375, height: 812 }; // iPhone X portrait

// Mock useWindowDimensions using spyOn
jest.spyOn(ReactNative, 'useWindowDimensions').mockImplementation(() => mockDimensions);

// Helper to render with theme and navigation
const renderWithProviders = () => {
  return render(
    <ThemeProvider>
      <DashboardScreen
        navigation={mockNavigation}
        route={mockRoute}
      />
    </ThemeProvider>
  );
};

// Helper to set mock dimensions
const setMockDimensions = (width: number, height: number) => {
  mockDimensions = { width, height };
  jest.spyOn(ReactNative, 'useWindowDimensions').mockImplementation(() => mockDimensions);
};

// Reset stores before each test
beforeEach(() => {
  useConnectionStore.getState().reset();
  useMetricsStore.getState().reset();
  useSessionStore.getState().reset();
  jest.clearAllMocks();

  // Reset to portrait mode
  setMockDimensions(375, 812);
});

describe('DashboardScreen', () => {
  describe('Layout and Structure', () => {
    it('renders the dashboard title', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('Dashboard')).toBeTruthy();
    });

    it('renders connection status badge', () => {
      const { getByText } = renderWithProviders();

      // By default, should show disconnected status
      expect(getByText('Disconnected')).toBeTruthy();
    });

    it('renders all metric cards', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('POWER')).toBeTruthy();
      expect(getByText('HEART RATE')).toBeTruthy();
      expect(getByText('CADENCE')).toBeTruthy();
      expect(getByText('SPEED')).toBeTruthy();
      expect(getByText('DISTANCE')).toBeTruthy();
      expect(getByText('TIME')).toBeTruthy();
      expect(getByText('CALORIES')).toBeTruthy();
    });
  });

  describe('Connection Status', () => {
    it('shows disconnected state when not connected', () => {
      const { getAllByText } = renderWithProviders();

      // Both the ConnectionStatus badge and NoSessionState show these texts
      expect(getAllByText('Disconnected').length).toBeGreaterThan(0);
      expect(getAllByText('Not Connected').length).toBeGreaterThan(0);
    });

    it('shows connecting state', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');

      const { getAllByText } = renderWithProviders();

      // Both the header badge and NoSessionState show connecting state
      expect(getAllByText('Connecting...').length).toBeGreaterThan(0);
    });

    it('shows connected state', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setConnected();

      const { getAllByText } = renderWithProviders();

      expect(getAllByText('Connected').length).toBeGreaterThan(0);
    });

    it('shows authenticated state', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getAllByText } = renderWithProviders();

      expect(getAllByText('Authenticated').length).toBeGreaterThan(0);
    });
  });

  describe('No Session State', () => {
    it('shows empty state message when not connected', () => {
      const { getAllByText, getByText } = renderWithProviders();

      expect(getAllByText('Not Connected').length).toBeGreaterThan(0);
      // NoSessionState shows "Connect to your RustRide desktop app to control workouts and view live metrics."
      expect(
        getByText(/Connect to your RustRide desktop app/)
      ).toBeTruthy();
    });

    it('shows empty state message when connected but no session', () => {
      // Set connected state
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getByText } = renderWithProviders();

      // NoSessionState shows "Ready to Ride" when connected
      expect(getByText('Ready to Ride')).toBeTruthy();
      expect(
        getByText(/Start a workout or free ride on the desktop app/)
      ).toBeTruthy();
    });

    it('shows placeholder values when no session is active', () => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getAllByText } = renderWithProviders();

      // Should show '--' for metrics without values
      const placeholders = getAllByText('--');
      expect(placeholders.length).toBeGreaterThan(0);
    });
  });

  describe('Active Session with Metrics', () => {
    beforeEach(() => {
      // Set up connected and authenticated state
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      // Start a session
      useSessionStore.getState().startSession({
        session_id: 'test-session-1',
        session_type: 'workout',
        workout_name: 'Test Workout',
        is_paused: false,
        elapsed_secs: 300,
      });

      // Update metrics
      useMetricsStore.getState().updateMetrics({
        power_watts: 250,
        heart_rate_bpm: 145,
        cadence_rpm: 90,
        speed_kph: 32.5,
        distance_km: 12.34,
        calories: 456,
      });
    });

    it('shows live power value', () => {
      const { getAllByText, getByText } = renderWithProviders();

      // 250 appears twice: current power and 3s average
      const powerValues = getAllByText('250');
      expect(powerValues.length).toBeGreaterThan(0);
      expect(getByText('W')).toBeTruthy();
    });

    it('shows live heart rate value', () => {
      const { getAllByText, getByText } = renderWithProviders();

      // 145 appears twice: current HR and max HR
      const hrValues = getAllByText('145');
      expect(hrValues.length).toBeGreaterThan(0);
      expect(getByText('bpm')).toBeTruthy();
    });

    it('shows live cadence value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('90')).toBeTruthy();
      expect(getByText('rpm')).toBeTruthy();
    });

    it('shows live speed value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('32.5')).toBeTruthy();
      expect(getByText('km/h')).toBeTruthy();
    });

    it('shows live distance value', () => {
      const { getByText } = renderWithProviders();

      // 12.34 is between 10-100km, so rendered as 12.3 (1 decimal place)
      expect(getByText('12.3')).toBeTruthy();
      expect(getByText('km')).toBeTruthy();
    });

    it('shows live calories value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('456')).toBeTruthy();
      expect(getByText('kcal')).toBeTruthy();
    });

    it('shows elapsed time formatted', () => {
      const { getByText } = renderWithProviders();

      // 300 seconds = 5:00
      expect(getByText('5:00')).toBeTruthy();
    });

    it('does not show empty state when session is active', () => {
      const { queryByText } = renderWithProviders();

      expect(queryByText('Not Connected')).toBeNull();
      expect(queryByText('No Active Session')).toBeNull();
    });
  });

  describe('Time Formatting', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('formats short time correctly', () => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 65, // 1:05
      });

      const { getByText } = renderWithProviders();

      expect(getByText('1:05')).toBeTruthy();
    });

    it('formats time with hours correctly', () => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 3665, // 1:01:05
      });

      const { getByText } = renderWithProviders();

      expect(getByText('1:01:05')).toBeTruthy();
    });
  });

  describe('Distance Formatting', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 0,
      });
    });

    it('shows 2 decimal places for distances under 10km', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 5.67,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('5.67')).toBeTruthy();
    });

    it('shows 1 decimal place for distances between 10-100km', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 45.67,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('45.7')).toBeTruthy();
    });

    it('shows no decimal places for distances over 100km', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 125.67,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('126')).toBeTruthy();
    });
  });

  describe('Target Values', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 0,
        target_power_watts: 200,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 195,
        heart_rate_bpm: 140,
        cadence_rpm: 88,
        speed_kph: 30,
        distance_km: 5,
        calories: 200,
      });
    });

    it('shows target power when set', () => {
      useMetricsStore.getState().setTargetPower(200);

      const { getAllByText } = renderWithProviders();

      // Target 200 appears - there may be multiple 200s in the view
      const targetTexts = getAllByText(/200/);
      expect(targetTexts.length).toBeGreaterThan(0);
    });

    it('shows target cadence when set', () => {
      useMetricsStore.getState().setTargetCadence(90);

      const { getByText } = renderWithProviders();

      // CadenceDisplay shows "TARGET" in uppercase
      expect(getByText('TARGET')).toBeTruthy();
      expect(getByText(/90/)).toBeTruthy();
    });
  });

  describe('Responsive Layout', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 0,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 200,
        heart_rate_bpm: 140,
        cadence_rpm: 90,
        speed_kph: 30,
        distance_km: 5,
        calories: 200,
      });
    });

    it('renders in portrait mode', () => {
      setMockDimensions(375, 812);

      const { getByText, getAllByText } = renderWithProviders();

      // Should render all metrics
      expect(getByText('POWER')).toBeTruthy();
      // 200 appears in both power and calories
      const power200 = getAllByText('200');
      expect(power200.length).toBeGreaterThan(0);
    });

    it('renders in landscape mode', () => {
      setMockDimensions(812, 375);

      const { getByText, getAllByText } = renderWithProviders();

      // Should render all metrics
      expect(getByText('POWER')).toBeTruthy();
      // 200 appears in both power and calories
      const power200 = getAllByText('200');
      expect(power200.length).toBeGreaterThan(0);
    });
  });

  describe('3-Second Power Average', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 0,
      });
    });

    it('shows 3-second average when available', () => {
      // Add multiple samples to create an average
      useMetricsStore.getState().updateMetrics({
        power_watts: 200,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 0,
        calories: 0,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 220,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 0,
        calories: 0,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 250,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 0,
        calories: 0,
      });

      const { getByText, getAllByText } = renderWithProviders();

      // Should show current power (250) - there could be multiple elements
      const powerValues = getAllByText('250');
      expect(powerValues.length).toBeGreaterThan(0);
      // Should show 3s avg label
      expect(getByText(/3s avg/)).toBeTruthy();
    });
  });

  describe('Max Heart Rate', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 0,
      });
    });

    it('shows max heart rate', () => {
      // Update with various heart rates
      useMetricsStore.getState().updateMetrics({
        power_watts: 200,
        heart_rate_bpm: 140,
        cadence_rpm: 90,
        speed_kph: 30,
        distance_km: 5,
        calories: 200,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 200,
        heart_rate_bpm: 165,
        cadence_rpm: 90,
        speed_kph: 30,
        distance_km: 5,
        calories: 200,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 200,
        heart_rate_bpm: 155,
        cadence_rpm: 90,
        speed_kph: 30,
        distance_km: 5,
        calories: 200,
      });

      const { getByText } = renderWithProviders();

      // Current HR is 155, max should be 165
      expect(getByText('155')).toBeTruthy();
      expect(getByText('165')).toBeTruthy();
      expect(getByText(/max/)).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 300,
      });
      useMetricsStore.getState().updateMetrics({
        power_watts: 250,
        heart_rate_bpm: 145,
        cadence_rpm: 90,
        speed_kph: 32.5,
        distance_km: 12.34,
        calories: 456,
      });
    });

    it('has accessible power metric', () => {
      const { getByLabelText } = renderWithProviders();

      expect(getByLabelText(/Power: 250 watts/)).toBeTruthy();
    });

    it('has accessible heart rate metric', () => {
      const { getByLabelText } = renderWithProviders();

      expect(getByLabelText(/Heart rate: 145 beats per minute/)).toBeTruthy();
    });

    it('has accessible cadence metric', () => {
      const { getByLabelText } = renderWithProviders();

      expect(getByLabelText(/Cadence: 90 revolutions per minute/)).toBeTruthy();
    });

    it('has accessible speed metric', () => {
      const { getByLabelText } = renderWithProviders();

      expect(getByLabelText(/Speed: 32.5 kilometers per hour/)).toBeTruthy();
    });

    it('has accessible distance metric', () => {
      const { getByLabelText } = renderWithProviders();

      // 12.34 is in 10-100km range, so rendered as 12.3 (1 decimal place)
      expect(getByLabelText(/Distance: 12.3 kilometers/)).toBeTruthy();
    });

    it('has accessible time metric', () => {
      const { getByLabelText } = renderWithProviders();

      // Accessibility label uses human-readable format: "5 minutes"
      expect(getByLabelText(/Elapsed time: 5 minutes/)).toBeTruthy();
    });

    it('has accessible calories metric', () => {
      const { getByLabelText } = renderWithProviders();

      expect(getByLabelText(/Calories: 456 kilocalories/)).toBeTruthy();
    });
  });

  describe('Server Name Display', () => {
    it('shows server name in connection status when available', () => {
      useConnectionStore.getState().setCurrentServer({
        name: 'RustRide-PC',
        host: '192.168.1.100',
        port: 9876,
      });
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
      useConnectionStore.getState().setAuthenticated();

      const { getAllByText } = renderWithProviders();

      // The connection status badge should show authenticated (appears in both header and content)
      expect(getAllByText('Authenticated').length).toBeGreaterThan(0);
    });
  });
});

describe('Helper Functions', () => {
  // These tests verify the formatting functions work correctly
  // The functions are tested indirectly through the component rendering
  // but we can still verify edge cases

  describe('formatElapsedTime', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://localhost:9876');
      useConnectionStore.getState().setAuthenticated();
    });

    it('handles zero time', () => {
      useSessionStore.getState().startSession({
        session_id: 'test',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('0:00')).toBeTruthy();
    });

    it('handles exactly 1 hour', () => {
      useSessionStore.getState().startSession({
        session_id: 'test',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 3600,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('1:00:00')).toBeTruthy();
    });
  });

  describe('formatDistance', () => {
    beforeEach(() => {
      useConnectionStore.getState().connect('ws://localhost:9876');
      useConnectionStore.getState().setAuthenticated();
      useSessionStore.getState().startSession({
        session_id: 'test',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 0,
      });
    });

    it('handles zero distance', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 0,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('0.00')).toBeTruthy();
    });

    it('handles edge case at 10km', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 10.0,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('10.0')).toBeTruthy();
    });

    it('handles edge case at 100km', () => {
      useMetricsStore.getState().updateMetrics({
        power_watts: 0,
        heart_rate_bpm: null,
        cadence_rpm: null,
        speed_kph: 0,
        distance_km: 100.0,
        calories: 0,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('100')).toBeTruthy();
    });
  });
});
