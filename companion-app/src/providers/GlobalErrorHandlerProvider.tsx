/**
 * GlobalErrorHandlerProvider
 *
 * Provider component that handles global error notifications with user-friendly toast messages.
 * Monitors connection state and provides error handling utilities for the app.
 *
 * Features:
 * - Connection lost notifications
 * - Command failure notifications
 * - Authentication failure notifications
 * - Network error notifications
 * - User-friendly error messages
 */

import React, { createContext, useContext, useCallback, useEffect, useRef, type ReactNode } from 'react';
import { useToast } from '@/hooks/useToast';
import { useConnectionStore, selectConnectionError, selectConnectionStatus } from '@/stores/connectionStore';
import { getConnectionService, type ConnectionServiceCallbacks } from '@/services/ConnectionService';
import type { ConnectionStatus } from '@/types';

/**
 * Error categories for user-friendly messaging
 */
export type ErrorCategory =
  | 'connection_lost'
  | 'connection_failed'
  | 'auth_failed'
  | 'auth_required'
  | 'command_failed'
  | 'network_error'
  | 'timeout'
  | 'unknown';

/**
 * Error handler context value
 */
interface GlobalErrorHandlerContextValue {
  /**
   * Handle an error with a user-friendly toast notification
   * @param error The error to handle
   * @param category Optional error category for specific messaging
   * @param context Additional context for the error message
   */
  handleError: (error: Error | string, category?: ErrorCategory, context?: string) => void;

  /**
   * Show a connection lost toast
   * @param reason Optional reason for disconnection
   */
  notifyConnectionLost: (reason?: string) => void;

  /**
   * Show a command failed toast
   * @param command The command that failed
   * @param reason Optional reason for failure
   */
  notifyCommandFailed: (command: string, reason?: string) => void;

  /**
   * Show a reconnecting notification
   * @param attempt Current reconnection attempt number
   */
  notifyReconnecting: (attempt: number) => void;

  /**
   * Show a reconnected success notification
   */
  notifyReconnected: () => void;
}

/**
 * Context for global error handling
 */
const GlobalErrorHandlerContext = createContext<GlobalErrorHandlerContextValue | null>(null);

/**
 * User-friendly error messages for each category
 */
const ERROR_MESSAGES: Record<ErrorCategory, string> = {
  connection_lost: 'Connection to RustRide lost',
  connection_failed: 'Could not connect to RustRide',
  auth_failed: 'Authentication failed',
  auth_required: 'Please enter your PIN to continue',
  command_failed: 'Command could not be completed',
  network_error: 'Network error occurred',
  timeout: 'Request timed out',
  unknown: 'An unexpected error occurred',
};

/**
 * Map error codes from ConnectionService to error categories
 */
function mapErrorCodeToCategory(code: string): ErrorCategory {
  switch (code) {
    case 'CONNECTION_FAILED':
    case 'CONNECTION_ERROR':
      return 'connection_failed';
    case 'AUTH_FAILED':
      return 'auth_failed';
    case 'AUTH_REQUIRED':
      return 'auth_required';
    case 'MAX_RECONNECT_ATTEMPTS':
      return 'connection_failed';
    case 'TIMEOUT':
    case 'REQUEST_TIMEOUT':
      return 'timeout';
    default:
      return 'unknown';
  }
}

/**
 * Map command names to user-friendly action names
 */
function formatCommandName(command: string): string {
  const commandMap: Record<string, string> = {
    workout_pause: 'Pause',
    workout_resume: 'Resume',
    workout_skip: 'Skip interval',
    workout_stop: 'Stop session',
    adjust_resistance: 'Adjust resistance',
    subscribe_metrics: 'Subscribe to metrics',
    get_session_status: 'Get session status',
    get_ride_history: 'Load ride history',
    get_ride_details: 'Load ride details',
  };

  return commandMap[command] ?? command.replace(/_/g, ' ');
}

/**
 * Provider props
 */
interface GlobalErrorHandlerProviderProps {
  children: ReactNode;
}

/**
 * GlobalErrorHandlerProvider Component
 *
 * Wraps the app to provide global error handling with toast notifications.
 * Automatically monitors connection state and shows appropriate toasts.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <ToastProvider>
 *       <GlobalErrorHandlerProvider>
 *         <MainApp />
 *       </GlobalErrorHandlerProvider>
 *     </ToastProvider>
 *   );
 * }
 * ```
 */
export function GlobalErrorHandlerProvider({
  children,
}: GlobalErrorHandlerProviderProps): React.JSX.Element {
  const { showError, showWarning, showInfo, showSuccess } = useToast();

  // Track previous connection status to detect changes
  const previousStatusRef = useRef<ConnectionStatus | null>(null);
  const hasShownReconnectingRef = useRef(false);

  // Track shown errors to avoid duplicates
  const shownErrorsRef = useRef<Set<string>>(new Set());

  /**
   * Generate a unique key for an error to prevent duplicates
   */
  const getErrorKey = useCallback((category: ErrorCategory, context?: string): string => {
    return `${category}:${context ?? 'default'}:${Math.floor(Date.now() / 5000)}`;
  }, []);

  /**
   * Check if an error was recently shown
   */
  const wasRecentlyShown = useCallback((key: string): boolean => {
    if (shownErrorsRef.current.has(key)) {
      return true;
    }
    shownErrorsRef.current.add(key);
    // Clear after 5 seconds to allow showing again
    setTimeout(() => {
      shownErrorsRef.current.delete(key);
    }, 5000);
    return false;
  }, []);

  /**
   * Handle an error with a user-friendly toast notification
   */
  const handleError = useCallback((
    error: Error | string,
    category: ErrorCategory = 'unknown',
    context?: string,
  ): void => {
    const errorKey = getErrorKey(category, context);
    if (wasRecentlyShown(errorKey)) {
      return;
    }

    const errorMessage = typeof error === 'string' ? error : error.message;
    const baseMessage = ERROR_MESSAGES[category];

    // Construct user-friendly message
    let message = baseMessage;
    if (context) {
      message = `${context}: ${baseMessage.toLowerCase()}`;
    }

    // Show toast based on category severity
    switch (category) {
      case 'connection_lost':
      case 'connection_failed':
      case 'auth_failed':
        showError(message, 5000);
        break;

      case 'auth_required':
        showWarning(message, 4000);
        break;

      case 'command_failed':
      case 'network_error':
      case 'timeout':
        showError(message, 4000);
        break;

      default:
        // For unknown errors, include the original message if helpful
        if (errorMessage && errorMessage !== baseMessage) {
          showError(`${baseMessage}: ${errorMessage}`, 4000);
        } else {
          showError(baseMessage, 4000);
        }
    }
  }, [getErrorKey, wasRecentlyShown, showError, showWarning]);

  /**
   * Show a connection lost toast
   */
  const notifyConnectionLost = useCallback((reason?: string): void => {
    const errorKey = getErrorKey('connection_lost', reason);
    if (wasRecentlyShown(errorKey)) {
      return;
    }

    const message = reason
      ? `Connection lost: ${reason}`
      : 'Connection to RustRide lost';

    showError(message, 5000);
    hasShownReconnectingRef.current = false;
  }, [getErrorKey, wasRecentlyShown, showError]);

  /**
   * Show a command failed toast
   */
  const notifyCommandFailed = useCallback((command: string, reason?: string): void => {
    const friendlyCommand = formatCommandName(command);
    const errorKey = getErrorKey('command_failed', command);

    if (wasRecentlyShown(errorKey)) {
      return;
    }

    const message = reason
      ? `${friendlyCommand} failed: ${reason}`
      : `${friendlyCommand} failed`;

    showError(message, 4000);
  }, [getErrorKey, wasRecentlyShown, showError]);

  /**
   * Show a reconnecting notification
   */
  const notifyReconnecting = useCallback((attempt: number): void => {
    if (hasShownReconnectingRef.current) {
      return;
    }
    hasShownReconnectingRef.current = true;

    showInfo(`Reconnecting to RustRide (attempt ${attempt})...`, 3000);
  }, [showInfo]);

  /**
   * Show a reconnected success notification
   */
  const notifyReconnected = useCallback((): void => {
    if (hasShownReconnectingRef.current) {
      showSuccess('Reconnected to RustRide', 3000);
      hasShownReconnectingRef.current = false;
    }
  }, [showSuccess]);

  // Subscribe to connection store errors
  const connectionError = useConnectionStore(selectConnectionError);
  const connectionStatus = useConnectionStore(selectConnectionStatus);

  // Handle connection store errors
  useEffect(() => {
    if (connectionError) {
      const category = mapErrorCodeToCategory(connectionError.code);
      handleError(connectionError.message, category);
    }
  }, [connectionError, handleError]);

  // Handle connection status changes
  useEffect(() => {
    const previousStatus = previousStatusRef.current;
    previousStatusRef.current = connectionStatus;

    if (previousStatus === null) {
      // Initial render, don't show notification
      return;
    }

    // Detect disconnect (was connected/authenticated, now disconnected)
    const wasConnected = previousStatus === 'connected' || previousStatus === 'authenticated';
    const isDisconnected = connectionStatus === 'disconnected';

    if (wasConnected && isDisconnected && !connectionError) {
      // Connection lost without explicit error
      notifyConnectionLost();
    }

    // Detect successful reconnection
    const isAuthenticated = connectionStatus === 'authenticated';
    const wasDisconnected = previousStatus === 'disconnected' || previousStatus === 'connecting';

    if (isAuthenticated && wasDisconnected && hasShownReconnectingRef.current) {
      notifyReconnected();
    }
  }, [connectionStatus, connectionError, notifyConnectionLost, notifyReconnected]);

  // Set up ConnectionService callbacks
  useEffect(() => {
    const connectionService = getConnectionService();

    const callbacks: ConnectionServiceCallbacks = {
      onDisconnected: (reason: string) => {
        // Only notify if we were previously connected
        if (previousStatusRef.current === 'connected' || previousStatusRef.current === 'authenticated') {
          notifyConnectionLost(reason);
        }
      },
      onAuthFailed: (reason: string) => {
        handleError(reason, 'auth_failed');
      },
      onAuthRequired: () => {
        handleError('Authentication required', 'auth_required');
      },
      onError: (error: Error) => {
        // Determine error category from message
        let category: ErrorCategory = 'unknown';
        if (error.message.includes('timeout')) {
          category = 'timeout';
        } else if (error.message.includes('network') || error.message.includes('Network')) {
          category = 'network_error';
        }
        handleError(error, category);
      },
    };

    connectionService.setCallbacks(callbacks);

    // Cleanup: don't clear callbacks as other parts of the app may set them
    return () => {
      // No cleanup needed - callbacks persist for reconnection handling
    };
  }, [handleError, notifyConnectionLost]);

  const value: GlobalErrorHandlerContextValue = {
    handleError,
    notifyConnectionLost,
    notifyCommandFailed,
    notifyReconnecting,
    notifyReconnected,
  };

  return (
    <GlobalErrorHandlerContext.Provider value={value}>
      {children}
    </GlobalErrorHandlerContext.Provider>
  );
}

/**
 * useGlobalErrorHandler Hook
 *
 * Access the global error handler to show user-friendly toast notifications.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { handleError, notifyCommandFailed } = useGlobalErrorHandler();
 *
 *   const handleSave = async () => {
 *     try {
 *       await saveData();
 *     } catch (error) {
 *       handleError(error, 'command_failed', 'Save');
 *     }
 *   };
 * }
 * ```
 */
export function useGlobalErrorHandler(): GlobalErrorHandlerContextValue {
  const context = useContext(GlobalErrorHandlerContext);

  if (!context) {
    throw new Error('useGlobalErrorHandler must be used within GlobalErrorHandlerProvider');
  }

  return context;
}

export type { GlobalErrorHandlerContextValue };
