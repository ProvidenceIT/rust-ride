/**
 * useAutoReconnect Hook
 *
 * Custom hook that handles automatic reconnection to the last connected
 * RustRide server on app launch. Integrates with StorageService for
 * persistent preferences and ConnectionService for WebSocket management.
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { AppState, type AppStateStatus } from 'react-native';
import { getStorageService, type StoredServer } from '@/services/StorageService';
import { getConnectionService } from '@/services/ConnectionService';
import {
  useConnectionStore,
  selectConnectionStatus,
} from '@/stores/connectionStore';

/**
 * Auto-reconnect state
 */
export interface AutoReconnectState {
  /** Whether auto-reconnect initialization is in progress */
  isInitializing: boolean;
  /** Whether auto-reconnect is currently attempting to connect */
  isAutoConnecting: boolean;
  /** The last connected server info if available */
  lastServer: StoredServer | null;
  /** Whether auto-reconnect is enabled in preferences */
  autoReconnectEnabled: boolean;
  /** Whether remember PIN is enabled in preferences */
  rememberPinEnabled: boolean;
  /** Error message if auto-reconnect failed */
  error: string | null;
}

/**
 * Auto-reconnect actions
 */
export interface AutoReconnectActions {
  /** Manually trigger auto-reconnect */
  reconnectToLastServer: () => Promise<void>;
  /** Set auto-reconnect preference */
  setAutoReconnect: (enabled: boolean) => Promise<void>;
  /** Set remember PIN preference */
  setRememberPin: (enabled: boolean) => Promise<void>;
  /** Save the current connection as last server */
  saveCurrentServer: (name: string, host: string, port: number, version?: string) => Promise<void>;
  /** Clear last server data */
  clearLastServer: () => Promise<void>;
  /** Clear any auto-reconnect errors */
  clearError: () => void;
}

/**
 * Timeout for auto-reconnect attempt (10 seconds)
 */
const AUTO_RECONNECT_TIMEOUT_MS = 10000;

/**
 * Hook for managing automatic reconnection to the last server
 *
 * @example
 * ```tsx
 * function App() {
 *   const { state, actions } = useAutoReconnect();
 *
 *   if (state.isInitializing) {
 *     return <LoadingScreen />;
 *   }
 *
 *   return <MainApp />;
 * }
 * ```
 */
export function useAutoReconnect(): {
  state: AutoReconnectState;
  actions: AutoReconnectActions;
} {
  // Local state
  const [isInitializing, setIsInitializing] = useState(true);
  const [isAutoConnecting, setIsAutoConnecting] = useState(false);
  const [lastServer, setLastServer] = useState<StoredServer | null>(null);
  const [autoReconnectEnabled, setAutoReconnectEnabledState] = useState(true);
  const [rememberPinEnabled, setRememberPinEnabledState] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Refs
  const hasInitialized = useRef(false);
  const autoConnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Store selectors
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  // Note: isAuthenticated selector available for future use if needed

  // Services
  const storageService = getStorageService();
  const connectionService = getConnectionService();

  /**
   * Load preferences and last server info from storage
   */
  const loadStoredData = useCallback(async () => {
    try {
      const [preferences, server] = await Promise.all([
        storageService.getPreferences(),
        storageService.getLastServer(),
      ]);

      setAutoReconnectEnabledState(preferences.autoReconnect);
      setRememberPinEnabledState(preferences.rememberPin);
      setLastServer(server);

      return { preferences, server };
    } catch {
      return { preferences: { autoReconnect: true, rememberPin: false }, server: null };
    }
  }, [storageService]);

  /**
   * Attempt auto-reconnect to the last server
   */
  const attemptAutoReconnect = useCallback(
    async (server: StoredServer): Promise<void> => {
      setIsAutoConnecting(true);
      setError(null);

      // Build WebSocket URL
      const url = `ws://${server.host}:${server.port}`;

      // Set up timeout
      const timeoutPromise = new Promise<never>((_, reject) => {
        autoConnectTimeoutRef.current = setTimeout(() => {
          reject(new Error('Connection timeout'));
        }, AUTO_RECONNECT_TIMEOUT_MS);
      });

      try {
        // Race between connection and timeout
        await Promise.race([connectionService.connect(url), timeoutPromise]);

        // Clear timeout on success
        if (autoConnectTimeoutRef.current) {
          clearTimeout(autoConnectTimeoutRef.current);
          autoConnectTimeoutRef.current = null;
        }

        // If we connected successfully and have a saved PIN, try auto-auth
        if (connectionStatus === 'connected') {
          const savedPin = await storageService.getSavedPin();
          if (savedPin) {
            try {
              await connectionService.authenticate(savedPin);
              // Update connection store with saved PIN for future reconnects
              useConnectionStore.getState().savePin(savedPin);
            } catch {
              // Auth failed with saved PIN - let user re-enter
              // Don't clear the saved PIN - might be network issue
            }
          }
        }
      } catch (err) {
        // Clear timeout
        if (autoConnectTimeoutRef.current) {
          clearTimeout(autoConnectTimeoutRef.current);
          autoConnectTimeoutRef.current = null;
        }

        const errorMessage = err instanceof Error ? err.message : 'Connection failed';
        setError(errorMessage);
      } finally {
        setIsAutoConnecting(false);
      }
    },
    [connectionService, connectionStatus, storageService],
  );

  /**
   * Initialize on mount - load preferences and attempt auto-reconnect
   */
  useEffect(() => {
    if (hasInitialized.current) return;
    hasInitialized.current = true;

    const initialize = async () => {
      const { preferences, server } = await loadStoredData();

      // Attempt auto-reconnect if enabled and we have a last server
      if (preferences.autoReconnect && server) {
        await attemptAutoReconnect(server);
      }

      setIsInitializing(false);
    };

    initialize();

    return () => {
      // Clear timeout on unmount
      if (autoConnectTimeoutRef.current) {
        clearTimeout(autoConnectTimeoutRef.current);
      }
    };
  }, [loadStoredData, attemptAutoReconnect]);

  /**
   * Handle app state changes - reconnect when app becomes active
   */
  useEffect(() => {
    const handleAppStateChange = async (nextAppState: AppStateStatus) => {
      if (nextAppState === 'active' && connectionStatus === 'disconnected') {
        // App came to foreground and we're disconnected
        const canReconnect = await storageService.canAutoReconnect();
        if (canReconnect) {
          const server = await storageService.getLastServer();
          if (server) {
            attemptAutoReconnect(server);
          }
        }
      }
    };

    const subscription = AppState.addEventListener('change', handleAppStateChange);

    return () => {
      subscription.remove();
    };
  }, [connectionStatus, storageService, attemptAutoReconnect]);

  // ===== Actions =====

  /**
   * Manually trigger reconnect to last server
   */
  const reconnectToLastServer = useCallback(async (): Promise<void> => {
    const server = await storageService.getLastServer();
    if (server) {
      await attemptAutoReconnect(server);
    } else {
      setError('No saved server found');
    }
  }, [storageService, attemptAutoReconnect]);

  /**
   * Set auto-reconnect preference
   */
  const setAutoReconnect = useCallback(
    async (enabled: boolean): Promise<void> => {
      await storageService.setAutoReconnect(enabled);
      setAutoReconnectEnabledState(enabled);
    },
    [storageService],
  );

  /**
   * Set remember PIN preference
   */
  const setRememberPin = useCallback(
    async (enabled: boolean): Promise<void> => {
      await storageService.setRememberPin(enabled);
      setRememberPinEnabledState(enabled);
    },
    [storageService],
  );

  /**
   * Save the current connection as last server
   */
  const saveCurrentServer = useCallback(
    async (name: string, host: string, port: number, version?: string): Promise<void> => {
      const server = { name, host, port, version };
      await storageService.saveLastServer(server);
      setLastServer({
        ...server,
        lastConnected: Date.now(),
      });
    },
    [storageService],
  );

  /**
   * Clear last server data
   */
  const clearLastServer = useCallback(async (): Promise<void> => {
    await storageService.clearLastServer();
    setLastServer(null);
  }, [storageService]);

  /**
   * Clear error state
   */
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    state: {
      isInitializing,
      isAutoConnecting,
      lastServer,
      autoReconnectEnabled,
      rememberPinEnabled,
      error,
    },
    actions: {
      reconnectToLastServer,
      setAutoReconnect,
      setRememberPin,
      saveCurrentServer,
      clearLastServer,
      clearError,
    },
  };
}

/**
 * Type export for external use
 */
export type UseAutoReconnectReturn = ReturnType<typeof useAutoReconnect>;
