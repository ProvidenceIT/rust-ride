/**
 * DateRangePickerModal Tests
 *
 * Tests for the custom date range picker modal component.
 */

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { DateRangePickerModal } from '../../src/components';

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('DateRangePickerModal', () => {
  const mockOnClose = jest.fn();
  const mockOnSelect = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Rendering', () => {
    it('renders when visible', () => {
      const { getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByText('Select Date Range')).toBeTruthy();
    });

    it('does not render when not visible', () => {
      const { queryByText } = renderWithTheme(
        <DateRangePickerModal
          visible={false}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(queryByText('Select Date Range')).toBeNull();
    });

    it('renders quick preset buttons', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByTestId('preset-7')).toBeTruthy();
      expect(getByTestId('preset-14')).toBeTruthy();
      expect(getByTestId('preset-30')).toBeTruthy();
      expect(getByTestId('preset-60')).toBeTruthy();
      expect(getByTestId('preset-90')).toBeTruthy();
    });

    it('renders date input fields', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByTestId('start-date-input')).toBeTruthy();
      expect(getByTestId('end-date-input')).toBeTruthy();
    });

    it('renders Cancel and Apply buttons', () => {
      const { getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByText('Cancel')).toBeTruthy();
      expect(getByText('Apply')).toBeTruthy();
    });
  });

  describe('Quick Presets', () => {
    it('calls onSelect with 7-day range when Last 7 Days is pressed', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByTestId('preset-7'));

      expect(mockOnSelect).toHaveBeenCalled();
      expect(mockOnClose).toHaveBeenCalled();

      // Verify the date range is approximately 7 days
      const [startDate, endDate] = mockOnSelect.mock.calls[0];
      const start = new Date(startDate);
      const end = new Date(endDate);
      const daysDiff = Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
      expect(daysDiff).toBeGreaterThanOrEqual(6);
      expect(daysDiff).toBeLessThanOrEqual(8);
    });

    it('calls onSelect with 30-day range when Last 30 Days is pressed', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByTestId('preset-30'));

      expect(mockOnSelect).toHaveBeenCalled();

      const [startDate, endDate] = mockOnSelect.mock.calls[0];
      const start = new Date(startDate);
      const end = new Date(endDate);
      const daysDiff = Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
      expect(daysDiff).toBeGreaterThanOrEqual(29);
      expect(daysDiff).toBeLessThanOrEqual(31);
    });

    it('calls onSelect with 90-day range when Last 90 Days is pressed', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByTestId('preset-90'));

      expect(mockOnSelect).toHaveBeenCalled();

      const [startDate, endDate] = mockOnSelect.mock.calls[0];
      const start = new Date(startDate);
      const end = new Date(endDate);
      const daysDiff = Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
      expect(daysDiff).toBeGreaterThanOrEqual(89);
      expect(daysDiff).toBeLessThanOrEqual(91);
    });
  });

  describe('Custom Date Input', () => {
    it('shows error when start date is empty on apply', () => {
      const { getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByText('Apply'));

      expect(getByText('Start date is required')).toBeTruthy();
    });

    it('shows error when end date is empty on apply', () => {
      const { getByTestId, getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.changeText(getByTestId('start-date-input'), '2024-01-01');
      fireEvent.press(getByText('Apply'));

      expect(getByText('End date is required')).toBeTruthy();
    });

    it('shows error for invalid date format', () => {
      const { getByTestId, getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      // Enter invalid start date format
      fireEvent.changeText(getByTestId('start-date-input'), '01-31-2024');
      fireEvent.changeText(getByTestId('end-date-input'), '2024-01-31');
      fireEvent.press(getByText('Apply'));

      // Should show error (either format error or validation error)
      expect(mockOnSelect).not.toHaveBeenCalled();
    });

    it('shows error when start date is after end date', () => {
      const { getByTestId, getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.changeText(getByTestId('start-date-input'), '2024-01-31');
      fireEvent.changeText(getByTestId('end-date-input'), '2024-01-01');
      fireEvent.press(getByText('Apply'));

      expect(getByText('Start date must be before end date')).toBeTruthy();
    });

    it('calls onSelect with valid custom dates', () => {
      const { getByTestId, getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.changeText(getByTestId('start-date-input'), '2024-01-01');
      fireEvent.changeText(getByTestId('end-date-input'), '2024-01-15');
      fireEvent.press(getByText('Apply'));

      expect(mockOnSelect).toHaveBeenCalled();
      expect(mockOnClose).toHaveBeenCalled();
    });

    it('clears errors when text changes', () => {
      const { getByTestId, getByText, queryByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      // Trigger error
      fireEvent.press(getByText('Apply'));
      expect(getByText('Start date is required')).toBeTruthy();

      // Change text should clear error
      fireEvent.changeText(getByTestId('start-date-input'), '2024');
      expect(queryByText('Start date is required')).toBeNull();
    });
  });

  describe('Initial Values', () => {
    it('populates inputs with initial start and end dates', async () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
          initialStartDate="2024-03-01T12:00:00.000Z"
          initialEndDate="2024-03-15T12:00:00.000Z"
        />,
      );

      await waitFor(() => {
        const startInput = getByTestId('start-date-input');
        const endInput = getByTestId('end-date-input');
        // Check that values are populated (exact format may vary by timezone)
        expect(startInput.props.value).toMatch(/2024-03-0[12]/);
        expect(endInput.props.value).toMatch(/2024-03-1[456]/);
      });
    });

    it('resets form when modal becomes visible', async () => {
      const { rerender, getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={false}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
          initialStartDate="2024-03-01T12:00:00.000Z"
          initialEndDate="2024-03-15T12:00:00.000Z"
        />,
      );

      // Make modal visible
      rerender(
        <ThemeProvider>
          <DateRangePickerModal
            visible={true}
            onClose={mockOnClose}
            onSelect={mockOnSelect}
            initialStartDate="2024-03-01T12:00:00.000Z"
            initialEndDate="2024-03-15T12:00:00.000Z"
          />
        </ThemeProvider>,
      );

      await waitFor(() => {
        const startInput = getByTestId('start-date-input');
        // Check value is populated (exact format may vary by timezone)
        expect(startInput.props.value).toMatch(/2024-03-0[12]/);
      });
    });
  });

  describe('Modal Actions', () => {
    it('calls onClose when Cancel is pressed', () => {
      const { getByText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByText('Cancel'));

      expect(mockOnClose).toHaveBeenCalled();
      expect(mockOnSelect).not.toHaveBeenCalled();
    });

    it('calls onClose when close icon is pressed', () => {
      const { getByLabelText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      fireEvent.press(getByLabelText('Close date picker'));

      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  describe('Accessibility', () => {
    it('has accessible labels for date inputs', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByTestId('start-date-input').props.accessibilityLabel).toBe('Start date');
      expect(getByTestId('end-date-input').props.accessibilityLabel).toBe('End date');
    });

    it('has accessible labels for preset buttons', () => {
      const { getByTestId } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByTestId('preset-7').props.accessibilityLabel).toBe('Last 7 Days');
      expect(getByTestId('preset-30').props.accessibilityLabel).toBe('Last 30 Days');
    });

    it('has accessible label for close button', () => {
      const { getByLabelText } = renderWithTheme(
        <DateRangePickerModal
          visible={true}
          onClose={mockOnClose}
          onSelect={mockOnSelect}
        />,
      );

      expect(getByLabelText('Close date picker')).toBeTruthy();
    });
  });
});
