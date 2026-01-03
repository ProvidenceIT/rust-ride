/**
 * useHaptics Hook Tests
 *
 * Tests for the haptic feedback hook functionality.
 */

import { renderHook, act } from '@testing-library/react-native';
import { Vibration } from 'react-native';
import { useHaptics, triggerHapticFeedback } from '../../src/hooks/useHaptics';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Vibration is mocked in jest.setup.js

describe('useHaptics', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('with haptics enabled (medium intensity)', () => {
    beforeEach(() => {
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
    });

    it('returns haptic functions', () => {
      const { result } = renderHook(() => useHaptics());

      expect(typeof result.current.triggerHaptic).toBe('function');
      expect(typeof result.current.selectionHaptic).toBe('function');
      expect(typeof result.current.impactHaptic).toBe('function');
      expect(typeof result.current.successHaptic).toBe('function');
      expect(typeof result.current.warningHaptic).toBe('function');
      expect(typeof result.current.errorHaptic).toBe('function');
    });

    it('indicates haptics are enabled', () => {
      const { result } = renderHook(() => useHaptics());

      expect(result.current.isHapticEnabled).toBe(true);
      expect(result.current.hapticIntensity).toBe('medium');
    });

    it('triggers vibration on impactHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.impactHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(25); // Medium impact = 25ms
    });

    it('triggers vibration on selectionHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.selectionHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(10); // Medium selection = 10ms
    });

    it('triggers vibration on successHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.successHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(20); // Medium success = 20ms
    });

    it('triggers vibration on warningHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.warningHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(40); // Medium warning = 40ms
    });

    it('triggers vibration on errorHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.errorHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(60); // Medium error = 60ms
    });

    it('triggers specific feedback type', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.triggerHaptic('notification');
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(30); // Medium notification = 30ms
    });
  });

  describe('with haptics disabled', () => {
    beforeEach(() => {
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
    });

    it('indicates haptics are disabled', () => {
      const { result } = renderHook(() => useHaptics());

      expect(result.current.isHapticEnabled).toBe(false);
      expect(result.current.hapticIntensity).toBe('off');
    });

    it('does not trigger vibration when haptics are off', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.impactHaptic();
      });

      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });

    it('does not trigger any haptic type when off', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.selectionHaptic();
        result.current.successHaptic();
        result.current.warningHaptic();
        result.current.errorHaptic();
        result.current.triggerHaptic('impact');
      });

      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });

  describe('with light intensity', () => {
    beforeEach(() => {
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
    });

    it('triggers lighter vibration on impactHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.impactHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(10); // Light impact = 10ms
    });

    it('triggers lighter vibration on selectionHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.selectionHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(5); // Light selection = 5ms
    });
  });

  describe('with strong intensity', () => {
    beforeEach(() => {
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
    });

    it('triggers stronger vibration on impactHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.impactHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(50); // Strong impact = 50ms
    });

    it('triggers stronger vibration on errorHaptic', () => {
      const { result } = renderHook(() => useHaptics());

      act(() => {
        result.current.errorHaptic();
      });

      expect(Vibration.vibrate).toHaveBeenCalledWith(100); // Strong error = 100ms
    });
  });
});

describe('triggerHapticFeedback', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('triggers vibration with default medium intensity', () => {
    act(() => {
      triggerHapticFeedback('impact');
    });

    expect(Vibration.vibrate).toHaveBeenCalledWith(25);
  });

  it('triggers vibration with specified intensity', () => {
    act(() => {
      triggerHapticFeedback('impact', 'strong');
    });

    expect(Vibration.vibrate).toHaveBeenCalledWith(50);
  });

  it('triggers vibration with light intensity', () => {
    act(() => {
      triggerHapticFeedback('selection', 'light');
    });

    expect(Vibration.vibrate).toHaveBeenCalledWith(5);
  });

  it('does not trigger vibration when intensity is off', () => {
    act(() => {
      triggerHapticFeedback('impact', 'off');
    });

    expect(Vibration.vibrate).not.toHaveBeenCalled();
  });

  it('defaults to impact type when not specified', () => {
    act(() => {
      triggerHapticFeedback();
    });

    expect(Vibration.vibrate).toHaveBeenCalledWith(25); // Default is impact with medium = 25ms
  });

  it('supports all feedback types', () => {
    const types = ['selection', 'impact', 'notification', 'success', 'warning', 'error'] as const;

    types.forEach(type => {
      jest.clearAllMocks();
      act(() => {
        triggerHapticFeedback(type, 'medium');
      });
      expect(Vibration.vibrate).toHaveBeenCalled();
    });
  });
});
