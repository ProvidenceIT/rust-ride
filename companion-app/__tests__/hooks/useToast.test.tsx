/**
 * useToast Hook Tests
 *
 * Tests for the toast notification hook and provider.
 */

import React from 'react';
import { Text, Button, View } from 'react-native';
import { render, fireEvent, act } from '@testing-library/react-native';
import { ToastProvider, useToast } from '../../src/hooks/useToast';

describe('useToast', () => {
  // Test component that uses the hook
  function TestComponent() {
    const {
      toasts,
      showToast,
      showSuccess,
      showError,
      showWarning,
      showInfo,
      dismissToast,
      dismissAllToasts,
    } = useToast();

    return (
      <View>
        <Text testID="toast-count">{toasts.length}</Text>
        {toasts.map(toast => (
          <Text key={toast.id} testID={`toast-${toast.id}`}>
            {toast.variant}: {toast.message}
          </Text>
        ))}
        <Button
          testID="show-toast"
          title="Show Toast"
          onPress={() => showToast({ message: 'Generic toast' })}
        />
        <Button
          testID="show-success"
          title="Success"
          onPress={() => showSuccess('Success message')}
        />
        <Button
          testID="show-error"
          title="Error"
          onPress={() => showError('Error message')}
        />
        <Button
          testID="show-warning"
          title="Warning"
          onPress={() => showWarning('Warning message')}
        />
        <Button
          testID="show-info"
          title="Info"
          onPress={() => showInfo('Info message')}
        />
        <Button
          testID="dismiss-all"
          title="Dismiss All"
          onPress={dismissAllToasts}
        />
        <Button
          testID="dismiss-first"
          title="Dismiss First"
          onPress={() => {
            if (toasts.length > 0) {
              dismissToast(toasts[0].id);
            }
          }}
        />
      </View>
    );
  }

  // Helper to render with provider
  const renderWithProvider = (maxToasts?: number) => {
    return render(
      <ToastProvider maxToasts={maxToasts}>
        <TestComponent />
      </ToastProvider>,
    );
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('ToastProvider', () => {
    it('provides toast context to children', () => {
      const { getByTestId } = renderWithProvider();
      expect(getByTestId('toast-count')).toBeTruthy();
    });

    it('throws error when useToast is used without provider', () => {
      const consoleError = jest.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        render(<TestComponent />);
      }).toThrow('useToast must be used within a ToastProvider');

      consoleError.mockRestore();
    });
  });

  describe('showToast', () => {
    it('adds a toast to the queue', () => {
      const { getByTestId } = renderWithProvider();

      expect(getByTestId('toast-count').props.children).toBe(0);

      act(() => {
        fireEvent.press(getByTestId('show-toast'));
      });

      expect(getByTestId('toast-count').props.children).toBe(1);
    });

    it('defaults to info variant', () => {
      const { getByTestId, getByText } = renderWithProvider();

      act(() => {
        fireEvent.press(getByTestId('show-toast'));
      });

      expect(getByText(/info: Generic toast/)).toBeTruthy();
    });
  });

  describe('showSuccess', () => {
    it('adds a success toast', () => {
      const { getByTestId, getByText } = renderWithProvider();

      act(() => {
        fireEvent.press(getByTestId('show-success'));
      });

      expect(getByText(/success: Success message/)).toBeTruthy();
    });
  });

  describe('showError', () => {
    it('adds an error toast', () => {
      const { getByTestId, getByText } = renderWithProvider();

      act(() => {
        fireEvent.press(getByTestId('show-error'));
      });

      expect(getByText(/error: Error message/)).toBeTruthy();
    });
  });

  describe('showWarning', () => {
    it('adds a warning toast', () => {
      const { getByTestId, getByText } = renderWithProvider();

      act(() => {
        fireEvent.press(getByTestId('show-warning'));
      });

      expect(getByText(/warning: Warning message/)).toBeTruthy();
    });
  });

  describe('showInfo', () => {
    it('adds an info toast', () => {
      const { getByTestId, getByText } = renderWithProvider();

      act(() => {
        fireEvent.press(getByTestId('show-info'));
      });

      expect(getByText(/info: Info message/)).toBeTruthy();
    });
  });

  describe('dismissToast', () => {
    it('removes a specific toast by id', () => {
      const { getByTestId } = renderWithProvider();

      // Add a toast
      act(() => {
        fireEvent.press(getByTestId('show-success'));
      });
      expect(getByTestId('toast-count').props.children).toBe(1);

      // Dismiss the toast
      act(() => {
        fireEvent.press(getByTestId('dismiss-first'));
      });

      expect(getByTestId('toast-count').props.children).toBe(0);
    });
  });

  describe('dismissAllToasts', () => {
    it('removes all toasts', () => {
      const { getByTestId } = renderWithProvider();

      // Add multiple toasts
      act(() => {
        fireEvent.press(getByTestId('show-success'));
        fireEvent.press(getByTestId('show-error'));
        fireEvent.press(getByTestId('show-warning'));
      });
      expect(getByTestId('toast-count').props.children).toBe(3);

      // Dismiss all
      act(() => {
        fireEvent.press(getByTestId('dismiss-all'));
      });

      expect(getByTestId('toast-count').props.children).toBe(0);
    });
  });

  describe('maxToasts', () => {
    it('limits the number of toasts to maxToasts', () => {
      const { getByTestId } = renderWithProvider(2);

      // Add 3 toasts
      act(() => {
        fireEvent.press(getByTestId('show-success'));
        fireEvent.press(getByTestId('show-error'));
        fireEvent.press(getByTestId('show-warning'));
      });

      // Should only have 2 toasts (oldest removed)
      expect(getByTestId('toast-count').props.children).toBe(2);
    });

    it('removes oldest toasts when exceeding maxToasts', () => {
      const { getByTestId, queryByText, getByText } = renderWithProvider(2);

      // Add 3 toasts
      act(() => {
        fireEvent.press(getByTestId('show-success')); // First - should be removed
        fireEvent.press(getByTestId('show-error'));   // Second - should remain
        fireEvent.press(getByTestId('show-warning')); // Third - should remain
      });

      // The first (success) toast should be removed
      expect(queryByText(/success: Success message/)).toBeNull();
      // The last two should remain
      expect(getByText(/error: Error message/)).toBeTruthy();
      expect(getByText(/warning: Warning message/)).toBeTruthy();
    });
  });

  describe('Toast IDs', () => {
    it('returns unique toast IDs', () => {
      let id1: string = '';
      let id2: string = '';

      function TestWithIds() {
        const { showSuccess } = useToast();
        return (
          <View>
            <Button
              testID="add-first"
              title="First"
              onPress={() => {
                id1 = showSuccess('First');
              }}
            />
            <Button
              testID="add-second"
              title="Second"
              onPress={() => {
                id2 = showSuccess('Second');
              }}
            />
          </View>
        );
      }

      const { getByTestId } = render(
        <ToastProvider>
          <TestWithIds />
        </ToastProvider>,
      );

      act(() => {
        fireEvent.press(getByTestId('add-first'));
        fireEvent.press(getByTestId('add-second'));
      });

      expect(id1).toBeTruthy();
      expect(id2).toBeTruthy();
      expect(id1).not.toBe(id2);
    });
  });
});
