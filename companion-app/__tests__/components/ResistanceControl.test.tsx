/**
 * ResistanceControl Tests
 *
 * Tests for the resistance control component with +/- buttons.
 */

import React from 'react';
import { Vibration } from 'react-native';
import { render, fireEvent, act } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { ResistanceControl } from '../../src/components';
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

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('ResistanceControl', () => {
  const mockOnIncrease = jest.fn();
  const mockOnDecrease = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    // Set haptic feedback to medium for consistent testing
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

  describe('Rendering', () => {
    it('renders with all elements', () => {
      const { getByText, getByLabelText, getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      expect(getByText('Resistance / Grade')).toBeTruthy();
      // Value is in accessibility label since it's hidden from accessibility tree
      const container = getByTestId('resistance');
      expect(container.props.accessibilityLabel).toContain('0%');
      expect(container.props.accessibilityLabel).toContain('Flat');
      expect(getByLabelText(/Increase resistance/)).toBeTruthy();
      expect(getByLabelText(/Decrease resistance/)).toBeTruthy();
    });

    it('renders with testID', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      expect(getByTestId('resistance')).toBeTruthy();
    });

    it('has correct accessibility role', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={50}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityRole).toBe('adjustable');
    });
  });

  describe('Resistance Level Display', () => {
    it('shows positive resistance with + sign', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={25}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityLabel).toContain('+25%');
      expect(container.props.accessibilityLabel).toContain('Resistance');
    });

    it('shows negative resistance with - sign', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={-30}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityLabel).toContain('-30%');
      expect(container.props.accessibilityLabel).toContain('Assist');
    });

    it('shows zero resistance as flat', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityLabel).toContain('0%');
      expect(container.props.accessibilityLabel).toContain('Flat');
    });

    it('shows step size hint in accessibility', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          stepSize={5}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityHint).toContain('5%');
    });

    it('shows custom step size in accessibility hint', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          stepSize={10}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityHint).toContain('10%');
    });
  });

  describe('Button Interactions', () => {
    it('calls onIncrease when increase button is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      expect(mockOnIncrease).toHaveBeenCalledTimes(1);
    });

    it('calls onDecrease when decrease button is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Decrease resistance/));
      expect(mockOnDecrease).toHaveBeenCalledTimes(1);
    });

    it('does not call onIncrease when canIncrease is false', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={100}
          canAdjust={true}
          canIncrease={false}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      expect(mockOnIncrease).not.toHaveBeenCalled();
    });

    it('does not call onDecrease when canDecrease is false', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={-100}
          canAdjust={true}
          canIncrease={true}
          canDecrease={false}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Decrease resistance/));
      expect(mockOnDecrease).not.toHaveBeenCalled();
    });

    it('does not call callbacks when loading', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          isLoading={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      fireEvent.press(getByLabelText(/Decrease resistance/));

      expect(mockOnIncrease).not.toHaveBeenCalled();
      expect(mockOnDecrease).not.toHaveBeenCalled();
    });
  });

  describe('Disabled States', () => {
    it('disables increase button when canIncrease is false', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={100}
          canAdjust={true}
          canIncrease={false}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      const increaseButton = getByLabelText(/Increase resistance/);
      expect(increaseButton.props.accessibilityState.disabled).toBe(true);
    });

    it('disables decrease button when canDecrease is false', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={-100}
          canAdjust={true}
          canIncrease={true}
          canDecrease={false}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      const decreaseButton = getByLabelText(/Decrease resistance/);
      expect(decreaseButton.props.accessibilityState.disabled).toBe(true);
    });

    it('disables both buttons when isLoading is true', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          isLoading={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      const increaseButton = getByLabelText(/Increase resistance/);
      const decreaseButton = getByLabelText(/Decrease resistance/);

      expect(increaseButton.props.accessibilityState.disabled).toBe(true);
      expect(decreaseButton.props.accessibilityState.disabled).toBe(true);
    });
  });

  describe('Haptic Feedback', () => {
    it('triggers haptic feedback on increase button press', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      expect(Vibration.vibrate).toHaveBeenCalled();
    });

    it('triggers haptic feedback on decrease button press', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Decrease resistance/));
      expect(Vibration.vibrate).toHaveBeenCalled();
    });

    it('does not trigger haptic feedback when haptics are off', () => {
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
        <ResistanceControl
          resistanceLevel={0}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });

    it('does not trigger haptic feedback when button is disabled', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={100}
          canAdjust={true}
          canIncrease={false}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      fireEvent.press(getByLabelText(/Increase resistance/));
      expect(Vibration.vibrate).not.toHaveBeenCalled();
    });
  });

  describe('Accessibility', () => {
    it('has correct accessibility label with current level', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={25}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityLabel).toContain('+25%');
      expect(container.props.accessibilityLabel).toContain('Resistance');
    });

    it('has correct accessibility value', () => {
      const { getByTestId } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={50}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          testID="resistance"
        />,
      );

      const container = getByTestId('resistance');
      expect(container.props.accessibilityValue).toEqual({
        min: -100,
        max: 100,
        now: 50,
        text: '+50%',
      });
    });

    it('has correct hint for increase button', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={25}
          canAdjust={true}
          canIncrease={true}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
          stepSize={5}
        />,
      );

      const increaseButton = getByLabelText(/Increase resistance/);
      expect(increaseButton.props.accessibilityLabel).toContain('5%');
    });

    it('has correct hint when at maximum', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={100}
          canAdjust={true}
          canIncrease={false}
          canDecrease={true}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      const increaseButton = getByLabelText(/Increase resistance/);
      expect(increaseButton.props.accessibilityHint).toContain('maximum');
    });

    it('has correct hint when at minimum', () => {
      const { getByLabelText } = renderWithTheme(
        <ResistanceControl
          resistanceLevel={-100}
          canAdjust={true}
          canIncrease={true}
          canDecrease={false}
          onIncrease={mockOnIncrease}
          onDecrease={mockOnDecrease}
        />,
      );

      const decreaseButton = getByLabelText(/Decrease resistance/);
      expect(decreaseButton.props.accessibilityHint).toContain('minimum');
    });
  });
});
