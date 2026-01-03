/**
 * useAuthentication Hook
 *
 * Custom hook that manages the authentication flow with the RustRide server.
 * Handles:
 * - Detecting when authentication is required
 * - Sending auth messages with PIN
 * - Handling auth_ok and auth_failed responses
 * - Updating Zustand connection store with auth state
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { getConnectionService } from '@/services/ConnectionService';
import {
  useConnectionStore,
  selectConnectionStatus,
  selectConnectionError,
} from '@/stores/connectionStore';

/**
 * Authentication state returned by the hook
 */
export interface AuthenticationState {
  /** Whether the PIN entry modal should be shown */
  showPinModal: boolean;
  /** Whether authentication is in progress */
  isAuthenticating: boolean;
  /** Error message if authentication failed */
  authError: string | null;
  /** Name of the server being connected to */
  serverName: string | null;
}

/**
 * Authentication actions returned by the hook
 */
export interface AuthenticationActions {
  /** Submit a PIN for authentication */
  submitPin: (pin: string) => Promise<void>;
  /** Close the PIN modal */
  closePinModal: () => void;
  /** Clear authentication error */
  clearAuthError: () => void;
  /** Manually trigger PIN entry (for servers that require auth immediately) */
  requestPinEntry: (serverName?: string) => void;
}

/**
 * Hook for managing authentication flow
 *
 * @example
 * ```tsx
 * const { state, actions } = useAuthentication();
 *
 * // Show PIN modal when required
 * <PinEntryModal
 *   visible={state.showPinModal}
 *   onClose={actions.closePinModal}
 *   onSubmit={actions.submitPin}
 *   isAuthenticating={state.isAuthenticating}
 *   error={state.authError}
 * />
 * ```
 */
export function useAuthentication(): {
  state: AuthenticationState;
  actions: AuthenticationActions;
} {
  // Local state
  const [showPinModal, setShowPinModal] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [serverName, setServerName] = useState<string | null>(null);

  // Store selectors
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const connectionError = useConnectionStore(selectConnectionError);

  // Service ref
  const connectionService = getConnectionService();
  const callbacksSetRef = useRef(false);

  // Set up callbacks for auth events
  useEffect(() => {
    if (callbacksSetRef.current) {
      return;
    }

    callbacksSetRef.current = true;

    connectionService.setCallbacks({
      onAuthRequired: () => {
        // Server requires authentication - show PIN modal
        setShowPinModal(true);
        setAuthError(null);
      },
      onAuthFailed: (reason: string) => {
        // Authentication failed - show error, allow retry
        setIsAuthenticating(false);
        setAuthError(reason);
        // Don't close modal - user can retry with correct PIN
      },
      onDisconnected: () => {
        // Connection lost - close modal and reset state
        setShowPinModal(false);
        setIsAuthenticating(false);
        setAuthError(null);
      },
      onError: () => {
        // Connection error - close modal if open
        if (showPinModal) {
          setShowPinModal(false);
          setIsAuthenticating(false);
        }
      },
    });

    return () => {
      callbacksSetRef.current = false;
    };
  }, [connectionService, showPinModal]);

  // Handle auth_failed from connection error
  useEffect(() => {
    if (connectionError?.code === 'AUTH_FAILED') {
      setIsAuthenticating(false);
      setAuthError(connectionError.message);
      // Show PIN modal if not already showing
      if (!showPinModal) {
        setShowPinModal(true);
      }
      // Clear the connection error
      useConnectionStore.getState().clearError();
    } else if (connectionError?.code === 'AUTH_REQUIRED') {
      // Server requires authentication
      setShowPinModal(true);
      setAuthError(null);
      useConnectionStore.getState().clearError();
    }
  }, [connectionError, showPinModal]);

  // Close modal when authenticated
  useEffect(() => {
    if (connectionStatus === 'authenticated' && showPinModal) {
      setShowPinModal(false);
      setIsAuthenticating(false);
      setAuthError(null);
    }
  }, [connectionStatus, showPinModal]);

  // Submit PIN for authentication
  const submitPin = useCallback(async (pin: string): Promise<void> => {
    setIsAuthenticating(true);
    setAuthError(null);

    try {
      await connectionService.authenticate(pin);
      // Success - modal will close via useEffect when status becomes 'authenticated'
      // Save PIN for potential auto-reconnect
      useConnectionStore.getState().savePin(pin);
    } catch (error) {
      // Error is handled by the callback or the catch block
      setIsAuthenticating(false);
      const errorMessage = error instanceof Error ? error.message : 'Authentication failed';
      setAuthError(errorMessage);
    }
  }, [connectionService]);

  // Close PIN modal
  const closePinModal = useCallback(() => {
    setShowPinModal(false);
    setIsAuthenticating(false);
    setAuthError(null);
    // Disconnect if we were waiting for auth
    if (connectionStatus === 'connected') {
      connectionService.disconnect();
    }
  }, [connectionService, connectionStatus]);

  // Clear auth error
  const clearAuthError = useCallback(() => {
    setAuthError(null);
  }, []);

  // Manually request PIN entry
  const requestPinEntry = useCallback((name?: string) => {
    setServerName(name ?? null);
    setShowPinModal(true);
    setAuthError(null);
    setIsAuthenticating(false);
  }, []);

  return {
    state: {
      showPinModal,
      isAuthenticating,
      authError,
      serverName,
    },
    actions: {
      submitPin,
      closePinModal,
      clearAuthError,
      requestPinEntry,
    },
  };
}

/**
 * Type export for external use
 */
export type UseAuthenticationReturn = ReturnType<typeof useAuthentication>;
