/**
 * WorkoutControlBar Tests
 *
 * Tests for the workout control bar component with play/pause, skip, and stop buttons.
 */

import React from 'react';
import { Vibration } from 'react-native';
import { render, fireEvent, act } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { WorkoutControlBar } from '../../src/components';
import { useSessionStore } from '../../src/stores/sessionStore';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Mock safe area context
jest.mock('react-native-safe-area-context', () => ({
  useSafeAreaInsets: () => ({
    top: 0,
    right: 0,
    bottom: 34,
    left: 0,
  }),
  SafeAreaView: ({ children }: { children: React.ReactNode }) => children,
  SafeAreaProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Vibration is mocked in jest.setup.js

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('WorkoutControlBar', () => {
  const mockOnPause = jest.fn();
  const mockOnResume = jest.fn();
  const mockOnSkip = jest.fn();
  const mockOnStop = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    // Reset stores to initial state
    act(() => {
      useSessionStore.getState().reset();
      // Set haptic feedback to medium for consistent testing
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

  describe('Rendering', () => {
    it('renders all control buttons', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
          testID="control-bar"
        />,
      );

      expect(getByLabelText('Pause workout')).toBeTruthy();
      expect(getByLabelText('Skip to next interval')).toBeTruthy();
      expect(getByLabelText('Stop session')).toBeTruthy();
    });

    it('has correct accessibility role', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const toolbar = getByLabelText('Workout controls');
      expect(toolbar.props.accessibilityRole).toBe('toolbar');
    });

    it('renders with testID', () => {
      const { getByTestId } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
          testID="control-bar"
        />,
      );

      expect(getByTestId('control-bar')).toBeTruthy();
    });
  });

  describe('Disabled State (No Session)', () => {
    it('disables all buttons when no session is active', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const pauseButton = getByLabelText('Pause workout');
      const skipButton = getByLabelText('Skip to next interval');
      const stopButton = getByLabelText('Stop session');

      // All buttons should be disabled
      expect(pauseButton.props.accessibilityState.disabled).toBe(true);
      expect(skipButton.props.accessibilityState.disabled).toBe(true);
      expect(stopButton.props.accessibilityState.disabled).toBe(true);
    });

    it('does not call onPause when button is disabled', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(mockOnPause).not.toHaveBeenCalled();
    });

    it('does not call onSkip when button is disabled', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Skip to next interval'));
      expect(mockOnSkip).not.toHaveBeenCalled();
    });

    it('does not call onStop when button is disabled', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Stop session'));
      expect(mockOnStop).not.toHaveBeenCalled();
    });
  });

  describe('Active Session - Free Ride', () => {
    beforeEach(() => {
      // Set up an active free ride session
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'free_ride',
          is_paused: false,
          elapsed_secs: 120,
        });
      });
    });

    it('enables pause and stop buttons during active free ride', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const pauseButton = getByLabelText('Pause workout');
      const stopButton = getByLabelText('Stop session');

      expect(pauseButton.props.accessibilityState.disabled).toBe(false);
      expect(stopButton.props.accessibilityState.disabled).toBe(false);
    });

    it('disables skip button during free ride (no intervals)', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityState.disabled).toBe(true);
    });

    it('calls onPause when pause button is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(mockOnPause).toHaveBeenCalledTimes(1);
    });

    it('calls onStop when stop button is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Stop session'));
      expect(mockOnStop).toHaveBeenCalledTimes(1);
    });
  });

  describe('Active Session - Workout with Intervals', () => {
    beforeEach(() => {
      // Set up an active workout session with intervals
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-workout',
          session_type: 'workout',
          workout_name: 'Sweet Spot',
          is_paused: false,
          elapsed_secs: 300,
          current_interval_index: 2,
          total_intervals: 5,
          current_interval_name: 'Interval 3',
          target_power_watts: 250,
        });
      });
    });

    it('enables skip button during workout with remaining intervals', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityState.disabled).toBe(false);
    });

    it('calls onSkip when skip button is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Skip to next interval'));
      expect(mockOnSkip).toHaveBeenCalledTimes(1);
    });

    it('disables skip button on last interval', () => {
      // Update to last interval
      act(() => {
        useSessionStore.getState().updateInterval({
          index: 4, // 0-based, last of 5 intervals
          total: 5,
          name: 'Cooldown',
          remainingSecs: 60,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityState.disabled).toBe(true);
    });
  });

  describe('Pause/Resume Toggle', () => {
    it('shows pause button when session is active', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: false,
          elapsed_secs: 120,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      expect(getByLabelText('Pause workout')).toBeTruthy();
    });

    it('shows resume button when session is paused', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: true,
          elapsed_secs: 120,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      expect(getByLabelText('Resume workout')).toBeTruthy();
    });

    it('calls onResume when resume button is pressed', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: true,
          elapsed_secs: 120,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Resume workout'));
      expect(mockOnResume).toHaveBeenCalledTimes(1);
      expect(mockOnPause).not.toHaveBeenCalled();
    });

    it('calls onPause when pause button is pressed', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: false,
          elapsed_secs: 120,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(mockOnPause).toHaveBeenCalledTimes(1);
      expect(mockOnResume).not.toHaveBeenCalled();
    });
  });

  describe('Haptic Feedback', () => {
    beforeEach(() => {
      // Set up an active session
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: false,
          elapsed_secs: 120,
          current_interval_index: 0,
          total_intervals: 3,
        });
      });
    });

    it('triggers haptic feedback on pause button press', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(Vibration.vibrate).toHaveBeenCalled();
    });

    it('triggers haptic feedback on skip button press', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Skip to next interval'));
      expect(Vibration.vibrate).toHaveBeenCalled();
    });

    it('triggers haptic feedback on stop button press', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Stop session'));
      expect(Vibration.vibrate).toHaveBeenCalled();
    });

    it('does not trigger haptic feedback when haptics are off', () => {
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

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });

    it('does not trigger haptic feedback when button is disabled', () => {
      // Reset to no session (all buttons disabled)
      act(() => {
        useSessionStore.getState().reset();
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      fireEvent.press(getByLabelText('Pause workout'));
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });

  describe('Loading States', () => {
    beforeEach(() => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: false,
          elapsed_secs: 120,
          current_interval_index: 0,
          total_intervals: 3,
        });
      });
    });

    it('shows loading indicator on pause button when isPauseLoading is true', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
          isPauseLoading
        />,
      );

      const pauseButton = getByLabelText('Pause workout');
      expect(pauseButton.props.accessibilityState.busy).toBe(true);
    });

    it('shows loading indicator on skip button when isSkipLoading is true', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
          isSkipLoading
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityState.busy).toBe(true);
    });

    it('shows loading indicator on stop button when isStopLoading is true', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
          isStopLoading
        />,
      );

      const stopButton = getByLabelText('Stop session');
      expect(stopButton.props.accessibilityState.busy).toBe(true);
    });
  });

  describe('Accessibility', () => {
    it('has correct accessibility hints when disabled', () => {
      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const pauseButton = getByLabelText('Pause workout');
      expect(pauseButton.props.accessibilityHint).toContain('disabled');
    });

    it('has correct accessibility hints when skip is disabled for non-workout', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'free_ride',
          is_paused: false,
          elapsed_secs: 120,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityHint).toContain('structured workouts');
    });

    it('has correct accessibility hints when skip is disabled on last interval', () => {
      act(() => {
        useSessionStore.getState().startSession({
          session_id: 'test-session',
          session_type: 'workout',
          is_paused: false,
          elapsed_secs: 120,
          current_interval_index: 4,
          total_intervals: 5,
        });
      });

      const { getByLabelText } = renderWithTheme(
        <WorkoutControlBar
          onPause={mockOnPause}
          onResume={mockOnResume}
          onSkip={mockOnSkip}
          onStop={mockOnStop}
        />,
      );

      const skipButton = getByLabelText('Skip to next interval');
      expect(skipButton.props.accessibilityHint).toContain('last interval');
    });
  });
});
