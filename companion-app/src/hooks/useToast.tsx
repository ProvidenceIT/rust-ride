/**
 * useToast Hook
 *
 * Provides toast notification functionality with a context-based approach.
 * Manages toast queue, display timing, and dismissal.
 *
 * Features:
 * - Queue multiple toasts
 * - Auto-dismiss with configurable duration
 * - Success, error, warning, and info variants
 * - Convenient shorthand methods (showSuccess, showError, etc.)
 */

import React, { createContext, useContext, useState, useCallback, useMemo } from 'react';
import type { ToastData, ToastVariant } from '@/components/Toast';

/**
 * Options for showing a toast
 */
export interface ShowToastOptions {
  /** Toast message */
  message: string;
  /** Toast variant (defaults to 'info') */
  variant?: ToastVariant;
  /** Duration in milliseconds (defaults to 3000) */
  duration?: number;
  /** Optional action button */
  action?: {
    label: string;
    onPress: () => void;
  };
}

/**
 * Toast context value
 */
interface ToastContextValue {
  /** Array of active toasts */
  toasts: ToastData[];
  /** Show a toast with options */
  showToast: (options: ShowToastOptions) => string;
  /** Show a success toast */
  showSuccess: (message: string, duration?: number) => string;
  /** Show an error toast */
  showError: (message: string, duration?: number) => string;
  /** Show a warning toast */
  showWarning: (message: string, duration?: number) => string;
  /** Show an info toast */
  showInfo: (message: string, duration?: number) => string;
  /** Dismiss a specific toast by ID */
  dismissToast: (id: string) => void;
  /** Dismiss all toasts */
  dismissAllToasts: () => void;
}

/**
 * Toast context
 */
const ToastContext = createContext<ToastContextValue | null>(null);

/**
 * Generate a unique toast ID
 */
let toastIdCounter = 0;
function generateToastId(): string {
  return `toast-${Date.now()}-${++toastIdCounter}`;
}

/**
 * Toast provider props
 */
export interface ToastProviderProps {
  children: React.ReactNode;
  /** Maximum number of toasts to display at once (default: 3) */
  maxToasts?: number;
}

/**
 * ToastProvider Component
 *
 * Provides toast context to the app. Should wrap the app at a high level.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <ToastProvider>
 *       <NavigationContainer>
 *         {...}
 *       </NavigationContainer>
 *       <ToastContainer />
 *     </ToastProvider>
 *   );
 * }
 * ```
 */
export function ToastProvider({
  children,
  maxToasts = 3,
}: ToastProviderProps): React.JSX.Element {
  const [toasts, setToasts] = useState<ToastData[]>([]);

  /**
   * Show a toast with options
   */
  const showToast = useCallback((options: ShowToastOptions): string => {
    const id = generateToastId();
    const toast: ToastData = {
      id,
      message: options.message,
      variant: options.variant ?? 'info',
      duration: options.duration,
      action: options.action,
    };

    setToasts(prev => {
      // Remove oldest toasts if we exceed maxToasts
      const newToasts = [...prev, toast];
      if (newToasts.length > maxToasts) {
        return newToasts.slice(-maxToasts);
      }
      return newToasts;
    });

    return id;
  }, [maxToasts]);

  /**
   * Show a success toast
   */
  const showSuccess = useCallback((message: string, duration?: number): string => {
    return showToast({ message, variant: 'success', duration });
  }, [showToast]);

  /**
   * Show an error toast
   */
  const showError = useCallback((message: string, duration?: number): string => {
    return showToast({ message, variant: 'error', duration });
  }, [showToast]);

  /**
   * Show a warning toast
   */
  const showWarning = useCallback((message: string, duration?: number): string => {
    return showToast({ message, variant: 'warning', duration });
  }, [showToast]);

  /**
   * Show an info toast
   */
  const showInfo = useCallback((message: string, duration?: number): string => {
    return showToast({ message, variant: 'info', duration });
  }, [showToast]);

  /**
   * Dismiss a specific toast by ID
   */
  const dismissToast = useCallback((id: string): void => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  }, []);

  /**
   * Dismiss all toasts
   */
  const dismissAllToasts = useCallback((): void => {
    setToasts([]);
  }, []);

  const value = useMemo<ToastContextValue>(() => ({
    toasts,
    showToast,
    showSuccess,
    showError,
    showWarning,
    showInfo,
    dismissToast,
    dismissAllToasts,
  }), [toasts, showToast, showSuccess, showError, showWarning, showInfo, dismissToast, dismissAllToasts]);

  return (
    <ToastContext.Provider value={value}>
      {children}
    </ToastContext.Provider>
  );
}

/**
 * useToast Hook
 *
 * Access the toast context to show and manage toasts.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { showSuccess, showError } = useToast();
 *
 *   const handleSave = async () => {
 *     try {
 *       await saveData();
 *       showSuccess('Data saved successfully');
 *     } catch (error) {
 *       showError('Failed to save data');
 *     }
 *   };
 * }
 * ```
 */
export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return context;
}

export type { ToastContextValue };
