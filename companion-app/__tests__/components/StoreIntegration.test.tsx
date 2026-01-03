/**
 * Store Integration Tests
 *
 * Tests for UI components that interact with Zustand stores.
 * Validates that components properly read from and update store state.
 */

import React from 'react';
import { render, fireEvent, act } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';

// Import stores
import { useMetricsStore, getPowerZone, getHeartRateZone } from '../../src/stores/metricsStore';
import { useSessionStore } from '../../src/stores/sessionStore';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Import components that use stores
import {
  PowerDisplay,
  HeartRateDisplay,
  CadenceDisplay,
  ConnectionStatus,
  NoSessionState,
} from '../../src/components';

// Reset stores before each test
beforeEach(async () => {
  act(() => {
    useMetricsStore.getState().reset();
    useSessionStore.getState().reset();
    useConnectionStore.getState().reset();
  });
  // Settings store uses async resetSettings
  await useSettingsStore.getState().resetSettings();
});

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('Metrics Store Integration', () => {
  describe('Power Zone Calculation', () => {
    it('calculates recovery zone correctly (< 55% FTP)', () => {
      expect(getPowerZone(100, 200)).toBe('recovery');
      expect(getPowerZone(0, 200)).toBe('recovery');
      expect(getPowerZone(109, 200)).toBe('recovery');
    });

    it('calculates endurance zone correctly (55-75% FTP)', () => {
      expect(getPowerZone(110, 200)).toBe('endurance');
      expect(getPowerZone(140, 200)).toBe('endurance');
      expect(getPowerZone(149, 200)).toBe('endurance');
    });

    it('calculates tempo zone correctly (75-90% FTP)', () => {
      expect(getPowerZone(150, 200)).toBe('tempo');
      expect(getPowerZone(170, 200)).toBe('tempo');
      expect(getPowerZone(179, 200)).toBe('tempo');
    });

    it('calculates threshold zone correctly (90-105% FTP)', () => {
      expect(getPowerZone(180, 200)).toBe('threshold');
      expect(getPowerZone(200, 200)).toBe('threshold');
      expect(getPowerZone(209, 200)).toBe('threshold');
    });

    it('calculates vo2max zone correctly (105-120% FTP)', () => {
      expect(getPowerZone(210, 200)).toBe('vo2max');
      expect(getPowerZone(230, 200)).toBe('vo2max');
      expect(getPowerZone(239, 200)).toBe('vo2max');
    });

    it('calculates anaerobic zone correctly (120-150% FTP)', () => {
      expect(getPowerZone(240, 200)).toBe('anaerobic');
      expect(getPowerZone(280, 200)).toBe('anaerobic');
      expect(getPowerZone(299, 200)).toBe('anaerobic');
    });

    it('calculates neuromuscular zone correctly (> 150% FTP)', () => {
      expect(getPowerZone(300, 200)).toBe('neuromuscular');
      expect(getPowerZone(400, 200)).toBe('neuromuscular');
    });

    it('handles zero FTP gracefully', () => {
      expect(getPowerZone(200, 0)).toBe('recovery');
    });
  });

  describe('Heart Rate Zone Calculation', () => {
    it('calculates zone1 correctly (< 60% max HR)', () => {
      expect(getHeartRateZone(90, 180)).toBe('zone1');
      expect(getHeartRateZone(107, 180)).toBe('zone1');
    });

    it('calculates zone2 correctly (60-70% max HR)', () => {
      expect(getHeartRateZone(108, 180)).toBe('zone2');
      expect(getHeartRateZone(125, 180)).toBe('zone2');
    });

    it('calculates zone3 correctly (70-80% max HR)', () => {
      expect(getHeartRateZone(126, 180)).toBe('zone3');
      expect(getHeartRateZone(143, 180)).toBe('zone3');
    });

    it('calculates zone4 correctly (80-90% max HR)', () => {
      expect(getHeartRateZone(144, 180)).toBe('zone4');
      expect(getHeartRateZone(161, 180)).toBe('zone4');
    });

    it('calculates zone5 correctly (90-100% max HR)', () => {
      expect(getHeartRateZone(162, 180)).toBe('zone5');
      expect(getHeartRateZone(180, 180)).toBe('zone5');
    });

    it('handles zero max HR gracefully', () => {
      expect(getHeartRateZone(120, 0)).toBe('zone1');
    });
  });

  describe('Metrics Store Updates', () => {
    it('updates metrics correctly', () => {
      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 250,
          heart_rate_bpm: 145,
          cadence_rpm: 90,
          speed_kph: 35,
          distance_km: 10.5,
          calories: 500,
        });
      });

      const state = useMetricsStore.getState();
      expect(state.metrics.power_watts).toBe(250);
      expect(state.metrics.heart_rate_bpm).toBe(145);
      expect(state.metrics.cadence_rpm).toBe(90);
      expect(state.metrics.speed_kph).toBe(35);
      expect(state.metrics.distance_km).toBe(10.5);
      expect(state.metrics.calories).toBe(500);
    });

    it('calculates 3-second power average', () => {
      // Add 3 samples
      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 200,
          heart_rate_bpm: null,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 250,
          heart_rate_bpm: null,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 300,
          heart_rate_bpm: null,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      const state = useMetricsStore.getState();
      // Average of 200, 250, 300 = 250
      expect(state.power3sAvg).toBe(250);
    });

    it('tracks max heart rate', () => {
      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 200,
          heart_rate_bpm: 140,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 200,
          heart_rate_bpm: 165,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 200,
          heart_rate_bpm: 150,
          cadence_rpm: null,
          speed_kph: 0,
          distance_km: 0,
          calories: 0,
        });
      });

      const state = useMetricsStore.getState();
      expect(state.heartRateMax).toBe(165);
    });

    it('sets target power and cadence', () => {
      act(() => {
        useMetricsStore.getState().setTargetPower(280);
        useMetricsStore.getState().setTargetCadence(90);
      });

      const state = useMetricsStore.getState();
      expect(state.targetPower).toBe(280);
      expect(state.targetCadence).toBe(90);
    });

    it('resets store correctly', () => {
      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 250,
          heart_rate_bpm: 145,
          cadence_rpm: 90,
          speed_kph: 35,
          distance_km: 10.5,
          calories: 500,
        });
        useMetricsStore.getState().setTargetPower(280);
      });

      act(() => {
        useMetricsStore.getState().reset();
      });

      const state = useMetricsStore.getState();
      expect(state.metrics.power_watts).toBe(0);
      expect(state.targetPower).toBeNull();
    });
  });
});

describe('Session Store Integration', () => {
  it('starts session correctly', () => {
    act(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-123',
        session_type: 'workout',
        workout_name: 'FTP Builder',
        workout_path: '/workouts/ftp.zwo',
        is_paused: false,
        elapsed_secs: 300,
        current_interval_index: 2,
        total_intervals: 5,
        current_interval_name: 'Interval 3',
        target_power_watts: 250,
        interval_remaining_secs: 60,
      });
    });

    const state = useSessionStore.getState();
    expect(state.isActive).toBe(true);
    expect(state.sessionId).toBe('test-123');
    expect(state.sessionType).toBe('workout');
    expect(state.workoutName).toBe('FTP Builder');
    expect(state.isPaused).toBe(false);
    expect(state.elapsedSecs).toBe(300);
    expect(state.currentInterval?.index).toBe(2);
    expect(state.currentInterval?.total).toBe(5);
    expect(state.targetPowerWatts).toBe(250);
  });

  it('ends session correctly', () => {
    act(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-123',
        session_type: 'workout',
        workout_name: 'FTP Builder',
        is_paused: false,
        elapsed_secs: 300,
      });
    });

    act(() => {
      useSessionStore.getState().endSession();
    });

    const state = useSessionStore.getState();
    expect(state.isActive).toBe(false);
    expect(state.sessionState).toBe('completed');
  });

  it('updates pause state correctly', () => {
    act(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-123',
        session_type: 'workout',
        workout_name: 'Test',
        is_paused: false,
        elapsed_secs: 0,
      });
    });

    act(() => {
      useSessionStore.getState().setPaused(true);
    });

    expect(useSessionStore.getState().isPaused).toBe(true);
    expect(useSessionStore.getState().sessionState).toBe('paused');

    act(() => {
      useSessionStore.getState().setPaused(false);
    });

    expect(useSessionStore.getState().isPaused).toBe(false);
    expect(useSessionStore.getState().sessionState).toBe('active');
  });

  it('adjusts resistance level correctly', () => {
    act(() => {
      useSessionStore.getState().setResistanceLevel(0);
    });

    act(() => {
      useSessionStore.getState().adjustResistanceLevel(10);
    });
    expect(useSessionStore.getState().resistanceLevel).toBe(10);

    act(() => {
      useSessionStore.getState().adjustResistanceLevel(-15);
    });
    expect(useSessionStore.getState().resistanceLevel).toBe(-5);
  });

  it('clamps resistance level to valid range', () => {
    act(() => {
      useSessionStore.getState().setResistanceLevel(50);
      useSessionStore.getState().adjustResistanceLevel(100);
    });
    expect(useSessionStore.getState().resistanceLevel).toBe(100);

    act(() => {
      useSessionStore.getState().adjustResistanceLevel(-300);
    });
    expect(useSessionStore.getState().resistanceLevel).toBe(-100);
  });
});

describe('Connection Store Integration', () => {
  it('manages connection state', () => {
    act(() => {
      useConnectionStore.getState().setStatus('connecting');
    });
    expect(useConnectionStore.getState().status).toBe('connecting');

    act(() => {
      useConnectionStore.getState().setStatus('connected');
    });
    expect(useConnectionStore.getState().status).toBe('connected');

    act(() => {
      useConnectionStore.getState().setAuthenticated();
    });
    expect(useConnectionStore.getState().isAuthenticated).toBe(true);
    expect(useConnectionStore.getState().status).toBe('authenticated');
  });

  it('manages server URL via connect', () => {
    act(() => {
      useConnectionStore.getState().connect('ws://192.168.1.100:9876');
    });
    expect(useConnectionStore.getState().serverUrl).toBe('ws://192.168.1.100:9876');
    expect(useConnectionStore.getState().status).toBe('connecting');
  });

  it('tracks discovered servers', () => {
    const server = {
      name: 'RustRide-PC',
      host: '192.168.1.100',
      port: 9876,
      version: '1.0',
    };

    act(() => {
      useConnectionStore.getState().addDiscoveredServer(server);
    });

    const servers = useConnectionStore.getState().discoveredServers;
    expect(servers).toHaveLength(1);
    expect(servers[0].name).toBe('RustRide-PC');

    act(() => {
      useConnectionStore.getState().removeDiscoveredServer('192.168.1.100', 9876);
    });

    expect(useConnectionStore.getState().discoveredServers).toHaveLength(0);
  });

  it('prevents duplicate servers', () => {
    const server = {
      name: 'RustRide-PC',
      host: '192.168.1.100',
      port: 9876,
      version: '1.0',
    };

    act(() => {
      useConnectionStore.getState().addDiscoveredServer(server);
      useConnectionStore.getState().addDiscoveredServer(server);
    });

    expect(useConnectionStore.getState().discoveredServers).toHaveLength(1);
  });

  it('manages error state', () => {
    act(() => {
      useConnectionStore.getState().setError('CONNECTION_FAILED', 'Unable to connect');
    });

    expect(useConnectionStore.getState().error?.code).toBe('CONNECTION_FAILED');
    expect(useConnectionStore.getState().error?.message).toBe('Unable to connect');
    expect(useConnectionStore.getState().status).toBe('disconnected');

    act(() => {
      useConnectionStore.getState().clearError();
    });

    expect(useConnectionStore.getState().error).toBeNull();
  });
});

describe('Settings Store Integration', () => {
  it('manages unit preference', async () => {
    expect(useSettingsStore.getState().settings.units).toBe('metric');

    await useSettingsStore.getState().setUnits('imperial');

    expect(useSettingsStore.getState().settings.units).toBe('imperial');
  });

  it('manages theme preference', async () => {
    expect(useSettingsStore.getState().settings.theme).toBe('system');

    await useSettingsStore.getState().setTheme('dark');

    expect(useSettingsStore.getState().settings.theme).toBe('dark');

    await useSettingsStore.getState().setTheme('light');

    expect(useSettingsStore.getState().settings.theme).toBe('light');
  });

  it('manages haptic feedback preference', async () => {
    expect(useSettingsStore.getState().settings.hapticFeedback).toBe('medium');

    await useSettingsStore.getState().setHapticFeedback('off');

    expect(useSettingsStore.getState().settings.hapticFeedback).toBe('off');
  });

  it('manages keep screen awake preference', async () => {
    expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(true);

    await useSettingsStore.getState().setKeepScreenAwake(false);

    expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(false);
  });

  it('updates multiple settings at once', async () => {
    await useSettingsStore.getState().updateSettings({
      units: 'imperial',
      theme: 'dark',
      hapticFeedback: 'strong',
    });

    const settings = useSettingsStore.getState().settings;
    expect(settings.units).toBe('imperial');
    expect(settings.theme).toBe('dark');
    expect(settings.hapticFeedback).toBe('strong');
  });

  it('toggles units between metric and imperial', async () => {
    expect(useSettingsStore.getState().settings.units).toBe('metric');

    await useSettingsStore.getState().toggleUnits();
    expect(useSettingsStore.getState().settings.units).toBe('imperial');

    await useSettingsStore.getState().toggleUnits();
    expect(useSettingsStore.getState().settings.units).toBe('metric');
  });
});

describe('Component with Store Integration', () => {
  describe('PowerDisplay with live store data', () => {
    it('displays data from metrics store', () => {
      // Pre-populate store with metrics
      act(() => {
        useMetricsStore.getState().updateMetrics({
          power_watts: 275,
          heart_rate_bpm: 150,
          cadence_rpm: 92,
          speed_kph: 38,
          distance_km: 15,
          calories: 650,
        });
      });

      const state = useMetricsStore.getState();

      const { getAllByText, getByText } = renderWithTheme(
        <PowerDisplay
          power={state.metrics.power_watts}
          power3sAvg={state.power3sAvg}
          powerZone="threshold"
          showMetrics={true}
        />,
      );

      // Power value appears (may be shown multiple times if 3s avg matches)
      expect(getAllByText('275').length).toBeGreaterThanOrEqual(1);
      // Unit should be present
      expect(getByText('W')).toBeTruthy();
      // Label should be present
      expect(getByText('POWER')).toBeTruthy();
    });
  });

  describe('ConnectionStatus with connection store', () => {
    it('reflects connection state from store', () => {
      act(() => {
        useConnectionStore.getState().setStatus('connected');
      });

      const { getByText } = renderWithTheme(
        <ConnectionStatus
          status={useConnectionStore.getState().status}
          variant="badge"
        />,
      );

      expect(getByText('Connected')).toBeTruthy();
    });

    it('shows connecting state', () => {
      act(() => {
        useConnectionStore.getState().setStatus('connecting');
      });

      const { getByText } = renderWithTheme(
        <ConnectionStatus
          status={useConnectionStore.getState().status}
          variant="badge"
        />,
      );

      expect(getByText('Connecting...')).toBeTruthy();
    });
  });

  describe('NoSessionState with connection and session stores', () => {
    it('shows connect button when disconnected', () => {
      act(() => {
        useConnectionStore.getState().setStatus('disconnected');
        useSessionStore.getState().reset();
      });

      const onConnectPress = jest.fn();

      const { getByText } = renderWithTheme(
        <NoSessionState
          connectionStatus="disconnected"
          onConnectPress={onConnectPress}
        />,
      );

      expect(getByText('Not Connected')).toBeTruthy();
      expect(getByText('Connect to Desktop')).toBeTruthy();

      fireEvent.press(getByText('Connect to Desktop'));
      expect(onConnectPress).toHaveBeenCalledTimes(1);
    });

    it('shows ready to ride when connected but no session', () => {
      act(() => {
        useConnectionStore.getState().setStatus('authenticated');
        useSessionStore.getState().reset();
      });

      const { getByText } = renderWithTheme(
        <NoSessionState
          connectionStatus="authenticated"
          serverName="RustRide-PC"
        />,
      );

      expect(getByText('Ready to Ride')).toBeTruthy();
      expect(getByText(/Start a workout or free ride/)).toBeTruthy();
    });
  });
});
