/**
 * Connection Store
 *
 * Manages WebSocket connection state to the RustRide desktop app.
 * Handles connection status, server URL, authentication, and discovered servers.
 * Integrates with StorageService for persistent storage of connection preferences.
 */

import { create } from 'zustand';
import type { ConnectionStatus, DiscoveredServer } from '@/types';
import { getStorageService } from '@/services/StorageService';

/**
 * Connection error information
 */
interface ConnectionError {
  code: string;
  message: string;
  timestamp: number;
}

/**
 * Connection store state
 */
interface ConnectionState {
  // Connection status
  status: ConnectionStatus;
  serverUrl: string | null;
  isAuthenticated: boolean;

  // Current server info (for persistence)
  currentServer: DiscoveredServer | null;

  // Server discovery
  discoveredServers: DiscoveredServer[];
  isScanning: boolean;

  // Connection details
  lastConnectedAt: number | null;
  reconnectAttempts: number;
  maxReconnectAttempts: number;
  error: ConnectionError | null;

  // PIN storage (for auto-reconnect)
  savedPin: string | null;
}

/**
 * Connection store actions
 */
interface ConnectionActions {
  // Connection lifecycle
  connect: (url: string) => void;
  disconnect: () => void;
  setConnected: () => void;
  setAuthenticated: () => void;

  // Connection state updates
  setStatus: (status: ConnectionStatus) => void;
  setError: (code: string, message: string) => void;
  clearError: () => void;

  // Current server management
  setCurrentServer: (server: DiscoveredServer | null) => void;

  // Reconnection handling
  incrementReconnectAttempts: () => void;
  resetReconnectAttempts: () => void;

  // Server discovery
  addDiscoveredServer: (server: DiscoveredServer) => void;
  removeDiscoveredServer: (host: string, port: number) => void;
  clearDiscoveredServers: () => void;
  setScanning: (isScanning: boolean) => void;

  // PIN management
  savePin: (pin: string) => void;
  clearSavedPin: () => void;

  // Reset store
  reset: () => void;
}

/**
 * Initial connection state
 */
const initialState: ConnectionState = {
  status: 'disconnected',
  serverUrl: null,
  isAuthenticated: false,
  currentServer: null,
  discoveredServers: [],
  isScanning: false,
  lastConnectedAt: null,
  reconnectAttempts: 0,
  maxReconnectAttempts: 5,
  error: null,
  savedPin: null,
};

/**
 * Connection store
 *
 * Manages all connection-related state including WebSocket status,
 * server discovery, and authentication.
 */
export const useConnectionStore = create<ConnectionState & ConnectionActions>()((set, get) => ({
  ...initialState,

  // Connection lifecycle
  connect: (url: string) => {
    set({
      serverUrl: url,
      status: 'connecting',
      error: null,
    });
  },

  disconnect: () => {
    set({
      status: 'disconnected',
      isAuthenticated: false,
      error: null,
    });
  },

  setConnected: () => {
    set({
      status: 'connected',
      lastConnectedAt: Date.now(),
      reconnectAttempts: 0,
    });
  },

  setAuthenticated: () => {
    set({
      status: 'authenticated',
      isAuthenticated: true,
      reconnectAttempts: 0,
    });

    // Persist current server to storage after successful authentication
    const currentServer = get().currentServer;
    if (currentServer) {
      getStorageService().saveLastServer(currentServer).catch(() => {
        // Ignore storage errors
      });
    }
  },

  // Connection state updates
  setStatus: (status: ConnectionStatus) => {
    set({ status });
  },

  setError: (code: string, message: string) => {
    set({
      error: {
        code,
        message,
        timestamp: Date.now(),
      },
      status: 'disconnected',
    });
  },

  clearError: () => {
    set({ error: null });
  },

  // Current server management
  setCurrentServer: (server: DiscoveredServer | null) => {
    set({ currentServer: server });
  },

  // Reconnection handling
  incrementReconnectAttempts: () => {
    const current = get().reconnectAttempts;
    set({ reconnectAttempts: current + 1 });
  },

  resetReconnectAttempts: () => {
    set({ reconnectAttempts: 0 });
  },

  // Server discovery
  addDiscoveredServer: (server: DiscoveredServer) => {
    const existing = get().discoveredServers;
    // Avoid duplicates based on host and port
    const isDuplicate = existing.some(s => s.host === server.host && s.port === server.port);
    if (!isDuplicate) {
      set({ discoveredServers: [...existing, server] });
    }
  },

  removeDiscoveredServer: (host: string, port: number) => {
    const servers = get().discoveredServers.filter(s => !(s.host === host && s.port === port));
    set({ discoveredServers: servers });
  },

  clearDiscoveredServers: () => {
    set({ discoveredServers: [] });
  },

  setScanning: (isScanning: boolean) => {
    set({ isScanning });
  },

  // PIN management
  savePin: (pin: string) => {
    set({ savedPin: pin });

    // Persist PIN to storage (only if rememberPin is enabled)
    getStorageService().savePin(pin).catch(() => {
      // Ignore storage errors
    });
  },

  clearSavedPin: () => {
    set({ savedPin: null });

    // Clear PIN from storage
    getStorageService().clearSavedPin().catch(() => {
      // Ignore storage errors
    });
  },

  // Reset store
  reset: () => {
    set(initialState);
  },
}));

// Selectors for optimized component subscriptions
export const selectConnectionStatus = (state: ConnectionState & ConnectionActions) => state.status;

export const selectServerUrl = (state: ConnectionState & ConnectionActions) => state.serverUrl;

export const selectIsAuthenticated = (state: ConnectionState & ConnectionActions) =>
  state.isAuthenticated;

export const selectDiscoveredServers = (state: ConnectionState & ConnectionActions) =>
  state.discoveredServers;

export const selectIsScanning = (state: ConnectionState & ConnectionActions) => state.isScanning;

export const selectConnectionError = (state: ConnectionState & ConnectionActions) => state.error;

export const selectCanReconnect = (state: ConnectionState & ConnectionActions) =>
  state.reconnectAttempts < state.maxReconnectAttempts;

export const selectIsConnecting = (state: ConnectionState & ConnectionActions) =>
  state.status === 'connecting';

export const selectIsConnected = (state: ConnectionState & ConnectionActions) =>
  state.status === 'connected' || state.status === 'authenticated';

export const selectCurrentServer = (state: ConnectionState & ConnectionActions) =>
  state.currentServer;

export const selectSavedPin = (state: ConnectionState & ConnectionActions) => state.savedPin;
