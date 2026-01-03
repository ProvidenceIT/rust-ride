/**
 * useIntervalChangeHaptics Hook Tests
 *
 * Tests for the interval change haptic feedback hook.
 */

import { renderHook, act } from '@testing-library/react-native';
import { Vibration } from 'react-native';
import { useIntervalChangeHaptics } from '../../src/hooks/useIntervalChangeHaptics';
import { useSessionStore } from '../../src/stores/sessionStore';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Vibration is mocked in jest.setup.js

describe('useIntervalChangeHaptics', () => {
  beforeEach(() => {
    jest.clearAllMocks();

    // Reset session store
    useSessionStore.setState({
      isActive: false,
      sessionId: null,
      sessionType: null,
      sessionState: 'idle',
      workoutName: null,
      workoutPath: null,
      elapsedSecs: 0,
      isPaused: false,
      currentInterval: null,
      targetPowerWatts: null,
      resistanceLevel: 0,
      lastStatusUpdate: null,
    });

    // Reset settings store with haptics enabled
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'system',
      },
      isLoaded: true,
      isSaving: false,
    });
  });

  describe('initialization', () => {
    it('returns current interval index as null when no session active', () => {
      const { result } = renderHook(() => useIntervalChangeHaptics());

      expect(result.current.currentIntervalIndex).toBeNull();
      expect(result.current.isEnabled).toBe(false);
    });

    it('returns current interval index when workout is active', () => {
      // Set up an active workout with interval
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionType: 'workout',
          sessionState: 'active',
          currentInterval: {
            index: 2,
            total: 10,
            name: 'Threshold',
            remainingSecs: 180,
          },
        });
      });

      const { result } = renderHook(() => useIntervalChangeHaptics());

      expect(result.current.currentIntervalIndex).toBe(2);
      expect(result.current.isEnabled).toBe(true);
    });
  });

  describe('interval change detection', () => {
    it('does not trigger haptic on initial mount', () => {
      // Set up an active workout
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionType: 'workout',
          sessionState: 'active',
          currentInterval: {
            index: 0,
            total: 5,
            name: 'Warmup',
            remainingSecs: 300,
          },
        });
      });

      renderHook(() => useIntervalChangeHaptics());

      // Should not trigger haptic on initial mount
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });

    it('triggers haptic when interval changes', () => {
      // Start with initial interval
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // No haptic on initial render
      expect(Vibration.vibrate).not.toHaveBeenCalled();

      // Change to next interval
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });

      rerender({});

      // Should trigger notification haptic (medium = 30ms)
      expect(Vibration.vibrate).toHaveBeenCalledWith(30);
    });

    it('triggers haptic on each interval change', () => {
      // Start with interval 0
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // Change to interval 1
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });
      rerender({});

      // Change to interval 2
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 2,
            total: 5,
            name: 'Recovery',
            remainingSecs: 120,
          },
        });
      });
      rerender({});

      // Should have triggered twice (once per change)
      expect(Vibration.vibrate).toHaveBeenCalledTimes(2);
    });

    it('does not trigger haptic when interval does not change', () => {
      // Start with initial interval
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // Update remaining time (same interval index)
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 0,
            total: 5,
            name: 'Warmup',
            remainingSecs: 299,
          },
        });
      });

      rerender({});

      // Should not trigger because interval index didn't change
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });

  describe('respects haptic settings', () => {
    it('does not trigger haptic when haptics are disabled', () => {
      // Disable haptics
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: true,
            hapticFeedback: 'off',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });

      // Start with initial interval
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { result, rerender } = renderHook(() => useIntervalChangeHaptics());

      // Hook should report as not enabled
      expect(result.current.isEnabled).toBe(false);

      // Change interval
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });

      rerender({});

      // Should not trigger because haptics are off
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });

    it('uses light intensity when set to light', () => {
      // Set light intensity
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: true,
            hapticFeedback: 'light',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });

      // Start with initial interval
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // Change interval
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });

      rerender({});

      // Light notification = 15ms
      expect(Vibration.vibrate).toHaveBeenCalledWith(15);
    });

    it('uses strong intensity when set to strong', () => {
      // Set strong intensity
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: true,
            hapticFeedback: 'strong',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });

      // Start with initial interval
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // Change interval
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });

      rerender({});

      // Strong notification = 60ms
      expect(Vibration.vibrate).toHaveBeenCalledWith(60);
    });
  });

  describe('free ride handling', () => {
    it('does not track interval changes for free rides', () => {
      // Set up a free ride (not a workout)
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionType: 'free_ride',
          sessionState: 'active',
          currentInterval: null,
        });
      });

      const { result } = renderHook(() => useIntervalChangeHaptics());

      // Should not be enabled for free rides
      expect(result.current.isEnabled).toBe(false);
    });
  });

  describe('enabled option', () => {
    it('can be disabled via option', () => {
      // Set up an active workout
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 0,
          total: 5,
          name: 'Warmup',
          remainingSecs: 300,
        },
      });

      const { result, rerender } = renderHook(() =>
        useIntervalChangeHaptics({ enabled: false })
      );

      // Should report as not enabled
      expect(result.current.isEnabled).toBe(false);

      // Change interval
      act(() => {
        useSessionStore.setState({
          currentInterval: {
            index: 1,
            total: 5,
            name: 'Interval 1',
            remainingSecs: 60,
          },
        });
      });

      rerender({});

      // Should not trigger because hook is disabled
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });

  describe('session lifecycle', () => {
    it('resets state when session ends', () => {
      // Start with active workout
      useSessionStore.setState({
        isActive: true,
        sessionType: 'workout',
        sessionState: 'active',
        currentInterval: {
          index: 2,
          total: 5,
          name: 'Interval',
          remainingSecs: 60,
        },
      });

      const { rerender } = renderHook(() => useIntervalChangeHaptics());

      // End the session
      act(() => {
        useSessionStore.setState({
          isActive: false,
          sessionType: null,
          sessionState: 'idle',
          currentInterval: null,
        });
      });

      rerender({});

      // Start a new workout at interval 0
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionType: 'workout',
          sessionState: 'active',
          currentInterval: {
            index: 0,
            total: 3,
            name: 'Warmup',
            remainingSecs: 300,
          },
        });
      });

      rerender({});

      // Should not trigger haptic for the initial interval of a new session
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });
});
