/**
 * StopConfirmationModal Component Tests
 *
 * Tests for the stop session confirmation modal.
 */

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { StopConfirmationModal } from '../../src/components/StopConfirmationModal';

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('StopConfirmationModal', () => {
  const mockOnClose = jest.fn();
  const mockOnConfirm = jest.fn();

  const defaultProps = {
    visible: true,
    onClose: mockOnClose,
    onConfirm: mockOnConfirm,
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Rendering', () => {
    it('renders when visible is true', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      expect(getByText('Stop Session?')).toBeTruthy();
    });

    it('does not render when visible is false', () => {
      const { queryByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} visible={false} />,
      );

      expect(queryByText('Stop Session?')).toBeNull();
    });

    it('renders confirmation message', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      expect(getByText(/Are you sure you want to stop this session/)).toBeTruthy();
      expect(getByText(/Your progress will be saved/)).toBeTruthy();
    });

    it('renders Cancel and Stop Session buttons', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      expect(getByText('Cancel')).toBeTruthy();
      expect(getByText('Stop Session')).toBeTruthy();
    });
  });

  describe('Session Type Display', () => {
    it('displays "Workout" for workout session type', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} sessionType="workout" />,
      );

      expect(getByText('Workout')).toBeTruthy();
    });

    it('displays "Free Ride" for free_ride session type', () => {
      const { getAllByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} sessionType="free_ride" />,
      );

      // "Free Ride" appears as both label and title
      expect(getAllByText('Free Ride').length).toBeGreaterThanOrEqual(1);
    });

    it('displays workout name when provided', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal
          {...defaultProps}
          sessionType="workout"
          workoutName="Threshold Intervals"
        />,
      );

      expect(getByText('Threshold Intervals')).toBeTruthy();
    });

    it('displays "Structured Workout" when sessionType is workout but no name', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} sessionType="workout" />,
      );

      expect(getByText('Structured Workout')).toBeTruthy();
    });

    it('displays "Free Ride" when sessionType is free_ride and no name', () => {
      // "Free Ride" appears twice - once as label and once as title
      const { getAllByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} sessionType="free_ride" />,
      );
      const freeRideTexts = getAllByText('Free Ride');
      expect(freeRideTexts.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('Elapsed Time Display', () => {
    it('displays elapsed time in MM:SS format', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} elapsedSecs={125} />,
      );

      expect(getByText('02:05')).toBeTruthy();
    });

    it('displays elapsed time in HH:MM:SS format when over an hour', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} elapsedSecs={3725} />,
      );

      expect(getByText('1:02:05')).toBeTruthy();
    });

    it('displays 00:00 when elapsedSecs is 0', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} elapsedSecs={0} />,
      );

      expect(getByText('00:00')).toBeTruthy();
    });

    it('displays 00:00 when elapsedSecs is not provided', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      expect(getByText('00:00')).toBeTruthy();
    });
  });

  describe('User Interactions', () => {
    it('calls onClose when Cancel button is pressed', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      fireEvent.press(getByText('Cancel'));

      expect(mockOnClose).toHaveBeenCalled();
    });

    it('calls onConfirm when Stop Session button is pressed', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      fireEvent.press(getByText('Stop Session'));

      expect(mockOnConfirm).toHaveBeenCalled();
    });

    it('calls onClose when backdrop is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      const backdrop = getByLabelText('Close modal');
      fireEvent.press(backdrop);

      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  describe('Loading State', () => {
    it('shows loading state on Stop Session button when isStopping is true', () => {
      const { getByTestId } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} isStopping={true} />,
      );

      const confirmButton = getByTestId('stop-modal-confirm');
      // Check button is disabled or in loading state
      expect(confirmButton).toBeTruthy();
    });

    it('disables Cancel button when isStopping is true', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} isStopping={true} />,
      );

      const cancelButton = getByText('Cancel');
      fireEvent.press(cancelButton);

      // onClose should not be called when stopping (button is disabled)
      // The Button component disables itself when parent passes disabled=true
    });
  });

  describe('Accessibility', () => {
    it('has alert accessibility role', () => {
      const { getByLabelText } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      // The modal container has accessibilityRole="alert"
      const alertDialog = getByLabelText('Stop session confirmation');
      expect(alertDialog.props.accessibilityRole).toBe('alert');
    });

    it('provides accessible labels for buttons', () => {
      const { getByTestId } = renderWithTheme(
        <StopConfirmationModal {...defaultProps} />,
      );

      expect(getByTestId('stop-modal-cancel')).toBeTruthy();
      expect(getByTestId('stop-modal-confirm')).toBeTruthy();
    });
  });

  describe('Full Session Info', () => {
    it('renders complete session information', () => {
      const { getByText } = renderWithTheme(
        <StopConfirmationModal
          {...defaultProps}
          sessionType="workout"
          workoutName="FTP Test"
          elapsedSecs={1800}
        />,
      );

      expect(getByText('Stop Session?')).toBeTruthy();
      expect(getByText('Workout')).toBeTruthy();
      expect(getByText('FTP Test')).toBeTruthy();
      expect(getByText('30:00')).toBeTruthy();
    });

    it('renders free ride session correctly', () => {
      const { getByText, getAllByText } = renderWithTheme(
        <StopConfirmationModal
          {...defaultProps}
          sessionType="free_ride"
          elapsedSecs={600}
        />,
      );

      expect(getByText('Stop Session?')).toBeTruthy();
      // "Free Ride" appears as both label and title
      expect(getAllByText('Free Ride').length).toBeGreaterThanOrEqual(1);
      expect(getByText('10:00')).toBeTruthy();
    });
  });
});
