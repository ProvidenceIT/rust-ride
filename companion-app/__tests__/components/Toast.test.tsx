/**
 * Toast Component Tests
 *
 * Tests for the Toast notification component.
 */

import React from 'react';
import { AccessibilityInfo } from 'react-native';
import { render, fireEvent, act, waitFor } from '@testing-library/react-native';
import { ThemeProvider } from '../../src/theme';
import { Toast, ToastData } from '../../src/components/Toast';

// Mock safe area context
jest.mock('react-native-safe-area-context', () => ({
  useSafeAreaInsets: () => ({
    top: 44,
    right: 0,
    bottom: 34,
    left: 0,
  }),
  SafeAreaView: ({ children }: { children: React.ReactNode }) => children,
  SafeAreaProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

// Spy on AccessibilityInfo.announceForAccessibility
const mockAnnounce = jest.spyOn(AccessibilityInfo, 'announceForAccessibility')
  .mockImplementation(() => {});

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

describe('Toast', () => {
  const mockOnDismiss = jest.fn();

  const createToast = (overrides: Partial<ToastData> = {}): ToastData => ({
    id: 'toast-1',
    message: 'Test message',
    variant: 'info',
    ...overrides,
  });

  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  describe('Rendering', () => {
    it('renders toast with message', () => {
      const toast = createToast({ message: 'Hello World' });
      const { getByText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByText('Hello World')).toBeTruthy();
    });

    it('renders success variant', () => {
      const toast = createToast({ variant: 'success' });
      const { getByText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByText('Test message')).toBeTruthy();
    });

    it('renders error variant', () => {
      const toast = createToast({ variant: 'error' });
      const { getByText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByText('Test message')).toBeTruthy();
    });

    it('renders warning variant', () => {
      const toast = createToast({ variant: 'warning' });
      const { getByText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByText('Test message')).toBeTruthy();
    });

    it('renders info variant', () => {
      const toast = createToast({ variant: 'info' });
      const { getByText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByText('Test message')).toBeTruthy();
    });

    it('renders with testID', () => {
      const toast = createToast();
      const { getByTestId } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} testID="test-toast" />,
      );

      expect(getByTestId('test-toast')).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    it('has alert accessibility role', () => {
      const toast = createToast();
      const { getByTestId } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} testID="toast-test" />,
      );

      const toastElement = getByTestId('toast-test');
      expect(toastElement.props.accessibilityRole).toBe('alert');
    });

    it('renders dismiss button with accessibility label', () => {
      const toast = createToast();
      const { getByLabelText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByLabelText('Dismiss notification')).toBeTruthy();
    });

    it('announces message to screen readers', () => {
      const toast = createToast({ message: 'Announced message' });
      renderWithTheme(<Toast toast={toast} onDismiss={mockOnDismiss} />);

      expect(mockAnnounce).toHaveBeenCalledWith('Announced message');
    });
  });

  describe('Dismiss Button', () => {
    it('calls onDismiss when dismiss button is pressed', async () => {
      const toast = createToast({ id: 'toast-123' });
      const { getByLabelText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      const dismissButton = getByLabelText('Dismiss notification');
      fireEvent.press(dismissButton);

      // Wait for animation to complete
      act(() => {
        jest.advanceTimersByTime(250);
      });

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith('toast-123');
      });
    });
  });

  describe('Auto-dismiss', () => {
    it('auto-dismisses after default duration (3000ms)', async () => {
      const toast = createToast({ id: 'toast-auto' });
      renderWithTheme(<Toast toast={toast} onDismiss={mockOnDismiss} />);

      // Should not have called onDismiss yet
      expect(mockOnDismiss).not.toHaveBeenCalled();

      // Advance past default duration
      act(() => {
        jest.advanceTimersByTime(3000);
      });

      // Wait for animation to complete
      act(() => {
        jest.advanceTimersByTime(250);
      });

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith('toast-auto');
      });
    });

    it('auto-dismisses after custom duration', async () => {
      const toast = createToast({ id: 'toast-custom', duration: 5000 });
      renderWithTheme(<Toast toast={toast} onDismiss={mockOnDismiss} />);

      // Should not dismiss at 3000ms
      act(() => {
        jest.advanceTimersByTime(3000);
      });
      expect(mockOnDismiss).not.toHaveBeenCalled();

      // Should dismiss at 5000ms
      act(() => {
        jest.advanceTimersByTime(2000);
      });

      // Wait for animation to complete
      act(() => {
        jest.advanceTimersByTime(250);
      });

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith('toast-custom');
      });
    });
  });

  describe('Action Button', () => {
    it('renders action button when provided', () => {
      const mockAction = jest.fn();
      const toast = createToast({
        action: { label: 'Undo', onPress: mockAction },
      });
      const { getByLabelText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      expect(getByLabelText('Undo')).toBeTruthy();
    });

    it('calls action onPress and dismisses when action button pressed', async () => {
      const mockAction = jest.fn();
      const toast = createToast({
        id: 'toast-action',
        action: { label: 'Undo', onPress: mockAction },
      });
      const { getByLabelText } = renderWithTheme(
        <Toast toast={toast} onDismiss={mockOnDismiss} />,
      );

      const actionButton = getByLabelText('Undo');
      fireEvent.press(actionButton);

      expect(mockAction).toHaveBeenCalled();

      // Wait for animation to complete
      act(() => {
        jest.advanceTimersByTime(250);
      });

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith('toast-action');
      });
    });
  });
});
