/**
 * useKeepAwake Hook Tests
 *
 * Tests for the keep screen awake functionality.
 */

import { renderHook, act } from '@testing-library/react-native';
import KeepAwake from 'react-native-keep-awake';
import { useKeepAwake } from '../../src/hooks/useKeepAwake';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { useSessionStore } from '../../src/stores/sessionStore';

// KeepAwake is mocked in jest.setup.js

describe('useKeepAwake', () => {
  beforeEach(() => {
    jest.clearAllMocks();

    // Reset stores to default state
    act(() => {
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
    });
  });

  describe('when setting is enabled but no active session', () => {
    it('returns correct state values', () => {
      const { result } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(result.current.isSettingEnabled).toBe(true);
      expect(result.current.hasActiveSession).toBe(false);
    });

    it('does not activate KeepAwake', () => {
      renderHook(() => useKeepAwake());

      expect(KeepAwake.activate).not.toHaveBeenCalled();
      expect(KeepAwake.deactivate).not.toHaveBeenCalled();
    });
  });

  describe('when setting is disabled and no active session', () => {
    beforeEach(() => {
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: false,
            hapticFeedback: 'medium',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });
    });

    it('returns correct state values', () => {
      const { result } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(result.current.isSettingEnabled).toBe(false);
      expect(result.current.hasActiveSession).toBe(false);
    });

    it('does not activate KeepAwake', () => {
      renderHook(() => useKeepAwake());

      expect(KeepAwake.activate).not.toHaveBeenCalled();
      expect(KeepAwake.deactivate).not.toHaveBeenCalled();
    });
  });

  describe('when setting is enabled and session becomes active', () => {
    it('activates KeepAwake when session starts', () => {
      const { result, rerender } = renderHook(() => useKeepAwake());

      // Initially not active
      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(KeepAwake.activate).not.toHaveBeenCalled();

      // Start a session
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });

      rerender({});

      expect(result.current.isKeepAwakeActive).toBe(true);
      expect(result.current.hasActiveSession).toBe(true);
      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);
    });

    it('deactivates KeepAwake when session ends', () => {
      // Start with active session
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });

      const { result, rerender } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(true);
      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);

      // End the session
      act(() => {
        useSessionStore.setState({
          isActive: false,
          sessionId: null,
          sessionType: null,
          sessionState: 'idle',
        });
      });

      rerender({});

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(KeepAwake.deactivate).toHaveBeenCalledTimes(1);
    });
  });

  describe('when setting is disabled and session is active', () => {
    beforeEach(() => {
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: false,
            hapticFeedback: 'medium',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });

        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });
    });

    it('does not activate KeepAwake', () => {
      const { result } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(result.current.isSettingEnabled).toBe(false);
      expect(result.current.hasActiveSession).toBe(true);
      expect(KeepAwake.activate).not.toHaveBeenCalled();
    });
  });

  describe('when setting is toggled during active session', () => {
    beforeEach(() => {
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });
    });

    it('activates KeepAwake when setting is enabled', () => {
      // Start with setting disabled
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: false,
            hapticFeedback: 'medium',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });

      const { result, rerender } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(KeepAwake.activate).not.toHaveBeenCalled();

      // Enable the setting
      act(() => {
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

      rerender({});

      expect(result.current.isKeepAwakeActive).toBe(true);
      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);
    });

    it('deactivates KeepAwake when setting is disabled', () => {
      const { result, rerender } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(true);
      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);

      // Disable the setting
      act(() => {
        useSettingsStore.setState({
          settings: {
            units: 'metric',
            keepScreenAwake: false,
            hapticFeedback: 'medium',
            theme: 'system',
          },
          isLoaded: true,
          isSaving: false,
        });
      });

      rerender({});

      expect(result.current.isKeepAwakeActive).toBe(false);
      expect(KeepAwake.deactivate).toHaveBeenCalledTimes(1);
    });
  });

  describe('cleanup on unmount', () => {
    it('deactivates KeepAwake when component unmounts during active session', () => {
      // Start with active session and setting enabled
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });

      const { unmount } = renderHook(() => useKeepAwake());

      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);

      // Unmount the hook
      unmount();

      expect(KeepAwake.deactivate).toHaveBeenCalledTimes(1);
    });

    it('does not call deactivate on unmount if not active', () => {
      // No active session
      const { unmount } = renderHook(() => useKeepAwake());

      expect(KeepAwake.activate).not.toHaveBeenCalled();

      // Unmount the hook
      unmount();

      expect(KeepAwake.deactivate).not.toHaveBeenCalled();
    });
  });

  describe('works with free ride sessions', () => {
    it('activates KeepAwake for free ride session', () => {
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'free-ride-123',
          sessionType: 'free_ride',
          sessionState: 'active',
        });
      });

      const { result } = renderHook(() => useKeepAwake());

      expect(result.current.isKeepAwakeActive).toBe(true);
      expect(result.current.hasActiveSession).toBe(true);
      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);
    });
  });

  describe('does not activate multiple times', () => {
    it('only calls activate once even if rerendered multiple times', () => {
      act(() => {
        useSessionStore.setState({
          isActive: true,
          sessionId: 'session-123',
          sessionType: 'workout',
          sessionState: 'active',
        });
      });

      const { rerender } = renderHook(() => useKeepAwake());

      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);

      // Rerender multiple times without state change
      rerender({});
      rerender({});
      rerender({});

      expect(KeepAwake.activate).toHaveBeenCalledTimes(1);
    });
  });
});
