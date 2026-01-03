/**
 * GlobalErrorHandlerProvider Tests
 *
 * Tests for the global error handler provider and hook.
 */

import React, { useEffect } from 'react';
import { render, act, renderHook, waitFor, fireEvent } from '@testing-library/react-native';
import { Text, View, TouchableOpacity } from 'react-native';
import { GlobalErrorHandlerProvider, useGlobalErrorHandler, type ErrorCategory } from '../GlobalErrorHandlerProvider';
import { ToastProvider, useToast } from '@/hooks/useToast';
import { getConnectionService } from '@/services/ConnectionService';

// Mock the connection service
jest.mock('@/services/ConnectionService', () => ({
  getConnectionService: jest.fn(() => ({
    setCallbacks: jest.fn(),
    isConnected: jest.fn(() => true),
  })),
}));

// Mock connection store
const mockConnectionStore = {
  status: 'disconnected' as const,
  error: null,
  isAuthenticated: false,
  serverUrl: null,
  currentServer: null,
  discoveredServers: [],
  isScanning: false,
  lastConnectedAt: null,
  reconnectAttempts: 0,
  maxReconnectAttempts: 5,
  savedPin: null,
  connect: jest.fn(),
  disconnect: jest.fn(),
  setConnected: jest.fn(),
  setAuthenticated: jest.fn(),
  setStatus: jest.fn(),
  setError: jest.fn(),
  clearError: jest.fn(),
  setCurrentServer: jest.fn(),
  incrementReconnectAttempts: jest.fn(),
  resetReconnectAttempts: jest.fn(),
  addDiscoveredServer: jest.fn(),
  removeDiscoveredServer: jest.fn(),
  clearDiscoveredServers: jest.fn(),
  setScanning: jest.fn(),
  savePin: jest.fn(),
  clearSavedPin: jest.fn(),
  reset: jest.fn(),
};

jest.mock('@/stores/connectionStore', () => ({
  useConnectionStore: jest.fn((selector) => {
    if (selector) {
      return selector(mockConnectionStore);
    }
    return mockConnectionStore;
  }),
  selectConnectionError: jest.fn((state) => state.error),
  selectConnectionStatus: jest.fn((state) => state.status),
}));

/**
 * Wrapper component that provides all required contexts
 */
function TestWrapper({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <ToastProvider>
      <GlobalErrorHandlerProvider>
        {children}
      </GlobalErrorHandlerProvider>
    </ToastProvider>
  );
}

/**
 * Combined hooks test component that uses both hooks
 */
interface CombinedHooksTestProps {
  onToastsChange?: (toasts: Array<{ message: string; variant: string }>) => void;
  action?: (handlers: ReturnType<typeof useGlobalErrorHandler>) => void;
}

function CombinedHooksTest({ onToastsChange, action }: CombinedHooksTestProps): React.JSX.Element {
  const handlers = useGlobalErrorHandler();
  const { toasts } = useToast();

  useEffect(() => {
    onToastsChange?.(toasts.map(t => ({ message: t.message, variant: t.variant })));
  }, [toasts, onToastsChange]);

  const handleTrigger = () => {
    action?.(handlers);
  };

  return (
    <View testID="test-container">
      <Text testID="toast-count">{toasts.length}</Text>
      <Text testID="latest-toast-message">{toasts[toasts.length - 1]?.message ?? 'none'}</Text>
      <Text testID="latest-toast-variant">{toasts[toasts.length - 1]?.variant ?? 'none'}</Text>
      <TouchableOpacity testID="trigger-action" onPress={handleTrigger}>
        <Text>Trigger</Text>
      </TouchableOpacity>
    </View>
  );
}

/**
 * Test component that uses the hook
 */
function TestComponent(): React.JSX.Element {
  const { handleError, notifyCommandFailed, notifyConnectionLost, notifyReconnecting, notifyReconnected } = useGlobalErrorHandler();
  const { toasts } = useToast();

  return (
    <View testID="test-component">
      <Text testID="toast-count">{toasts.length}</Text>
      <Text testID="latest-toast">{toasts[toasts.length - 1]?.message ?? 'none'}</Text>
      <TouchableOpacity
        testID="trigger-error"
        onPress={() => handleError(new Error('Test error'), 'unknown')}
      >
        <Text>Trigger Error</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="trigger-connection-lost"
        onPress={() => notifyConnectionLost('Server closed')}
      >
        <Text>Connection Lost</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="trigger-command-failed"
        onPress={() => notifyCommandFailed('workout_pause', 'No active session')}
      >
        <Text>Command Failed</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="trigger-reconnecting"
        onPress={() => notifyReconnecting(1)}
      >
        <Text>Reconnecting</Text>
      </TouchableOpacity>
      <TouchableOpacity
        testID="trigger-reconnected"
        onPress={() => {
          notifyReconnecting(1);
          notifyReconnected();
        }}
      >
        <Text>Reconnected</Text>
      </TouchableOpacity>
    </View>
  );
}

describe('GlobalErrorHandlerProvider', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Reset mock store
    mockConnectionStore.status = 'disconnected';
    mockConnectionStore.error = null;
  });

  describe('Provider rendering', () => {
    it('renders children correctly', () => {
      const { getByText } = render(
        <TestWrapper>
          <Text>Child Component</Text>
        </TestWrapper>,
      );

      expect(getByText('Child Component')).toBeTruthy();
    });

    it('sets up ConnectionService callbacks on mount', () => {
      const mockSetCallbacks = jest.fn();
      (getConnectionService as jest.Mock).mockReturnValue({
        setCallbacks: mockSetCallbacks,
        isConnected: jest.fn(() => true),
      });

      render(
        <TestWrapper>
          <Text>Test</Text>
        </TestWrapper>,
      );

      expect(mockSetCallbacks).toHaveBeenCalled();
      expect(mockSetCallbacks).toHaveBeenCalledWith(expect.objectContaining({
        onDisconnected: expect.any(Function),
        onAuthFailed: expect.any(Function),
        onAuthRequired: expect.any(Function),
        onError: expect.any(Function),
      }));
    });
  });

  describe('useGlobalErrorHandler hook', () => {
    it('throws error when used outside provider', () => {
      // Suppress console.error for this test
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        renderHook(() => useGlobalErrorHandler());
      }).toThrow('useGlobalErrorHandler must be used within GlobalErrorHandlerProvider');

      consoleSpy.mockRestore();
    });

    it('returns all handler functions', () => {
      const { result } = renderHook(() => useGlobalErrorHandler(), {
        wrapper: TestWrapper,
      });

      expect(result.current.handleError).toBeDefined();
      expect(result.current.notifyConnectionLost).toBeDefined();
      expect(result.current.notifyCommandFailed).toBeDefined();
      expect(result.current.notifyReconnecting).toBeDefined();
      expect(result.current.notifyReconnected).toBeDefined();
    });
  });

  describe('handleError', () => {
    it('shows error toast for unknown category', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <TestComponent />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-error'));

      await waitFor(() => {
        expect(getByTestId('toast-count').props.children).toBe(1);
      });
    });

    it('handles Error objects', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.handleError(new Error('Test error message'), 'network_error')}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('toast-count').props.children).toBeGreaterThan(0);
      });
    });

    it('handles string errors', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.handleError('String error message', 'timeout')}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('toast-count').props.children).toBeGreaterThan(0);
      });
    });

    it('includes context in error message', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.handleError('Error', 'command_failed', 'Save')}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('latest-toast-message').props.children).toContain('Save');
      });
    });

    it('prevents duplicate toasts within 5 seconds', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => {
              handlers.handleError('Error 1', 'network_error');
              handlers.handleError('Error 2', 'network_error');
              handlers.handleError('Error 3', 'network_error');
            }}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        // Only one toast should be shown for the same category
        expect(getByTestId('toast-count').props.children).toBe(1);
      });
    });
  });

  describe('notifyConnectionLost', () => {
    it('shows connection lost toast with reason', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <TestComponent />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-connection-lost'));

      await waitFor(() => {
        const latestToast = getByTestId('latest-toast').props.children;
        expect(latestToast).toContain('Connection lost');
        expect(latestToast).toContain('Server closed');
      });
    });

    it('shows connection lost toast without reason', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.notifyConnectionLost()}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('latest-toast-message').props.children).toBe('Connection to RustRide lost');
      });
    });
  });

  describe('notifyCommandFailed', () => {
    it('shows command failed toast with reason', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <TestComponent />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-command-failed'));

      await waitFor(() => {
        const latestToast = getByTestId('latest-toast').props.children;
        expect(latestToast).toContain('Pause failed');
        expect(latestToast).toContain('No active session');
      });
    });

    it('formats command names to user-friendly text', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.notifyCommandFailed('workout_skip')}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('latest-toast-message').props.children).toContain('Skip interval');
      });
    });
  });

  describe('notifyReconnecting', () => {
    it('shows reconnecting info toast', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.notifyReconnecting(1)}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('latest-toast-message').props.children).toContain('Reconnecting');
        expect(getByTestId('latest-toast-message').props.children).toContain('attempt 1');
      });
    });

    it('only shows one reconnecting toast', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => {
              handlers.notifyReconnecting(1);
              handlers.notifyReconnecting(2);
              handlers.notifyReconnecting(3);
            }}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        // Only one reconnecting toast
        expect(getByTestId('toast-count').props.children).toBe(1);
      });
    });
  });

  describe('notifyReconnected', () => {
    it('shows reconnected success toast after reconnecting', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => {
              handlers.notifyReconnecting(1);
              handlers.notifyReconnected();
            }}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        // Should have both toasts
        expect(getByTestId('toast-count').props.children).toBe(2);
      });
    });

    it('does not show reconnected toast if reconnecting was not shown', async () => {
      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.notifyReconnected()}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      // Wait a bit and then check
      await act(async () => {
        await new Promise<void>(resolve => setTimeout(resolve, 100));
      });

      expect(getByTestId('toast-count').props.children).toBe(0);
    });
  });

  describe('Error categories', () => {
    const testCases: Array<[ErrorCategory, string]> = [
      ['connection_lost', 'error'],
      ['connection_failed', 'error'],
      ['auth_failed', 'error'],
      ['auth_required', 'warning'],
      ['command_failed', 'error'],
      ['network_error', 'error'],
      ['timeout', 'error'],
      ['unknown', 'error'],
    ];

    it.each(testCases)('shows correct toast variant for %s category', async (category, expectedVariant) => {
      // Use unique context to avoid duplicate prevention
      const uniqueContext = `test-${category}-${Date.now()}`;

      const { getByTestId } = render(
        <TestWrapper>
          <CombinedHooksTest
            action={(handlers) => handlers.handleError('Test error', category, uniqueContext)}
          />
        </TestWrapper>,
      );

      fireEvent.press(getByTestId('trigger-action'));

      await waitFor(() => {
        expect(getByTestId('latest-toast-variant').props.children).toBe(expectedVariant);
      });
    });
  });
});
