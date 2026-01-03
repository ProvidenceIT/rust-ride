/**
 * useAuthentication Hook Tests
 */

import { renderHook, act, waitFor } from '@testing-library/react-native';
import { useAuthentication } from '../../src/hooks/useAuthentication';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { getConnectionService } from '../../src/services/ConnectionService';

// Define the mock service interface
interface MockConnectionService {
  authenticate: jest.Mock;
  disconnect: jest.Mock;
  setCallbacks: jest.Mock;
  reset: jest.Mock;
}

// Create the mock service
const mockService: MockConnectionService = {
  authenticate: jest.fn(),
  disconnect: jest.fn(),
  setCallbacks: jest.fn(),
  reset: jest.fn(),
};

// Mock the ConnectionService module
jest.mock('../../src/services/ConnectionService', () => ({
  ConnectionService: jest.fn(() => mockService),
  getConnectionService: jest.fn(() => mockService),
}));

describe('useAuthentication', () => {
  let mockConnectionService: MockConnectionService;

  beforeEach(() => {
    // Reset stores
    useConnectionStore.getState().reset();

    // Get mock service
    mockConnectionService = getConnectionService() as unknown as MockConnectionService;

    // Reset all mocks
    jest.clearAllMocks();
  });

  describe('initial state', () => {
    it('should have correct initial state', () => {
      const { result } = renderHook(() => useAuthentication());

      expect(result.current.state.showPinModal).toBe(false);
      expect(result.current.state.isAuthenticating).toBe(false);
      expect(result.current.state.authError).toBeNull();
      expect(result.current.state.serverName).toBeNull();
    });

    it('should set up callbacks on mount', () => {
      renderHook(() => useAuthentication());

      expect(mockConnectionService.setCallbacks).toHaveBeenCalledWith(
        expect.objectContaining({
          onAuthRequired: expect.any(Function),
          onAuthFailed: expect.any(Function),
          onDisconnected: expect.any(Function),
          onError: expect.any(Function),
        }),
      );
    });
  });

  describe('submitPin', () => {
    it('should call authenticate with PIN', async () => {
      mockConnectionService.authenticate.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useAuthentication());

      await act(async () => {
        await result.current.actions.submitPin('123456');
      });

      expect(mockConnectionService.authenticate).toHaveBeenCalledWith('123456');
    });

    it('should set isAuthenticating during auth', async () => {
      let resolveAuth: () => void;
      mockConnectionService.authenticate.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveAuth = resolve; }),
      );

      const { result } = renderHook(() => useAuthentication());

      // Start authentication
      let authPromise: Promise<void>;
      act(() => {
        authPromise = result.current.actions.submitPin('123456');
      });

      // Check authenticating state
      expect(result.current.state.isAuthenticating).toBe(true);

      // Complete authentication
      await act(async () => {
        resolveAuth!();
        await authPromise;
      });
    });

    it('should save PIN on success', async () => {
      mockConnectionService.authenticate.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useAuthentication());

      await act(async () => {
        await result.current.actions.submitPin('123456');
      });

      expect(useConnectionStore.getState().savedPin).toBe('123456');
    });

    it('should set error on failure', async () => {
      mockConnectionService.authenticate.mockRejectedValueOnce(new Error('Invalid PIN'));

      const { result } = renderHook(() => useAuthentication());

      await act(async () => {
        await result.current.actions.submitPin('000000');
      });

      expect(result.current.state.authError).toBe('Invalid PIN');
      expect(result.current.state.isAuthenticating).toBe(false);
    });
  });

  describe('closePinModal', () => {
    it('should close modal and clear state', () => {
      const { result } = renderHook(() => useAuthentication());

      // First show the modal
      act(() => {
        result.current.actions.requestPinEntry('Test Server');
      });

      expect(result.current.state.showPinModal).toBe(true);

      // Then close it
      act(() => {
        result.current.actions.closePinModal();
      });

      expect(result.current.state.showPinModal).toBe(false);
      expect(result.current.state.isAuthenticating).toBe(false);
      expect(result.current.state.authError).toBeNull();
    });

    it('should disconnect if connected but not authenticated', () => {
      // Set connection status to connected
      act(() => {
        useConnectionStore.getState().setConnected();
      });

      const { result } = renderHook(() => useAuthentication());

      // Show modal then close
      act(() => {
        result.current.actions.requestPinEntry();
      });

      act(() => {
        result.current.actions.closePinModal();
      });

      expect(mockConnectionService.disconnect).toHaveBeenCalled();
    });
  });

  describe('clearAuthError', () => {
    it('should clear auth error', async () => {
      mockConnectionService.authenticate.mockRejectedValueOnce(new Error('Invalid PIN'));

      const { result } = renderHook(() => useAuthentication());

      // Cause an error
      await act(async () => {
        await result.current.actions.submitPin('000000');
      });

      expect(result.current.state.authError).toBe('Invalid PIN');

      // Clear the error
      act(() => {
        result.current.actions.clearAuthError();
      });

      expect(result.current.state.authError).toBeNull();
    });
  });

  describe('requestPinEntry', () => {
    it('should show PIN modal', () => {
      const { result } = renderHook(() => useAuthentication());

      act(() => {
        result.current.actions.requestPinEntry();
      });

      expect(result.current.state.showPinModal).toBe(true);
    });

    it('should set server name', () => {
      const { result } = renderHook(() => useAuthentication());

      act(() => {
        result.current.actions.requestPinEntry('My RustRide PC');
      });

      expect(result.current.state.serverName).toBe('My RustRide PC');
    });

    it('should clear previous error', async () => {
      mockConnectionService.authenticate.mockRejectedValueOnce(new Error('Bad PIN'));

      const { result } = renderHook(() => useAuthentication());

      // Cause an error
      await act(async () => {
        await result.current.actions.submitPin('000000');
      });

      expect(result.current.state.authError).toBe('Bad PIN');

      // Request PIN entry again
      act(() => {
        result.current.actions.requestPinEntry();
      });

      expect(result.current.state.authError).toBeNull();
    });
  });

  describe('auth callback handling', () => {
    it('should show PIN modal when onAuthRequired is called', () => {
      const { result } = renderHook(() => useAuthentication());

      // Get the callback that was set
      const callbacks = mockConnectionService.setCallbacks.mock.calls[0][0];

      // Simulate auth required
      act(() => {
        callbacks.onAuthRequired();
      });

      expect(result.current.state.showPinModal).toBe(true);
    });

    it('should set error when onAuthFailed is called', () => {
      const { result } = renderHook(() => useAuthentication());

      // Get the callback that was set
      const callbacks = mockConnectionService.setCallbacks.mock.calls[0][0];

      // First start authenticating
      act(() => {
        result.current.actions.requestPinEntry();
      });

      // Simulate auth failed
      act(() => {
        callbacks.onAuthFailed('Wrong PIN');
      });

      expect(result.current.state.authError).toBe('Wrong PIN');
      expect(result.current.state.isAuthenticating).toBe(false);
    });

    it('should close modal when onDisconnected is called', () => {
      const { result } = renderHook(() => useAuthentication());

      // Get the callback that was set
      const callbacks = mockConnectionService.setCallbacks.mock.calls[0][0];

      // Show modal first
      act(() => {
        result.current.actions.requestPinEntry();
      });

      expect(result.current.state.showPinModal).toBe(true);

      // Simulate disconnect
      act(() => {
        callbacks.onDisconnected();
      });

      expect(result.current.state.showPinModal).toBe(false);
    });
  });

  describe('connection error handling', () => {
    it('should show PIN modal on AUTH_FAILED connection error', async () => {
      const { result } = renderHook(() => useAuthentication());

      // Simulate auth failed error from connection store
      act(() => {
        useConnectionStore.getState().setError('AUTH_FAILED', 'Invalid PIN');
      });

      await waitFor(() => {
        expect(result.current.state.showPinModal).toBe(true);
        expect(result.current.state.authError).toBe('Invalid PIN');
      });
    });

    it('should show PIN modal on AUTH_REQUIRED connection error', async () => {
      const { result } = renderHook(() => useAuthentication());

      // Simulate auth required error from connection store
      act(() => {
        useConnectionStore.getState().setError('AUTH_REQUIRED', 'Authentication required');
      });

      await waitFor(() => {
        expect(result.current.state.showPinModal).toBe(true);
      });
    });
  });

  describe('authentication success', () => {
    it('should close modal when status becomes authenticated', async () => {
      const { result } = renderHook(() => useAuthentication());

      // Show modal
      act(() => {
        result.current.actions.requestPinEntry();
      });

      expect(result.current.state.showPinModal).toBe(true);

      // Simulate successful authentication
      act(() => {
        useConnectionStore.getState().setAuthenticated();
      });

      await waitFor(() => {
        expect(result.current.state.showPinModal).toBe(false);
      });
    });
  });
});
