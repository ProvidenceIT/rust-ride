/**
 * AutoReconnectProvider
 *
 * Provider component that handles auto-reconnection to the last connected
 * server on app launch. Shows a loading state while initializing.
 */

import React, { createContext, useContext, type ReactNode } from 'react';
import { View, Text, ActivityIndicator, StyleSheet, useColorScheme } from 'react-native';
import {
  useAutoReconnect,
  type AutoReconnectState,
  type AutoReconnectActions,
} from '@/hooks/useAutoReconnect';

/**
 * Context value type
 */
interface AutoReconnectContextValue {
  state: AutoReconnectState;
  actions: AutoReconnectActions;
}

/**
 * Context for auto-reconnect state and actions
 */
const AutoReconnectContext = createContext<AutoReconnectContextValue | null>(null);

/**
 * Provider props
 */
interface AutoReconnectProviderProps {
  children: ReactNode;
  /** Whether to show loading screen during initialization */
  showLoadingScreen?: boolean;
}

/**
 * Loading screen shown during initialization
 */
function LoadingScreen(): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';

  return (
    <View style={[styles.loadingContainer, isDarkMode && styles.loadingContainerDark]}>
      <ActivityIndicator
        size="large"
        color={isDarkMode ? '#0A84FF' : '#007AFF'}
        style={styles.spinner}
      />
      <Text style={[styles.loadingText, isDarkMode && styles.loadingTextDark]}>
        Connecting...
      </Text>
    </View>
  );
}

/**
 * AutoReconnectProvider component
 *
 * Wraps the app to provide auto-reconnect functionality and optionally
 * shows a loading screen during initialization.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <AutoReconnectProvider>
 *       <MainApp />
 *     </AutoReconnectProvider>
 *   );
 * }
 * ```
 */
export function AutoReconnectProvider({
  children,
  showLoadingScreen = false,
}: AutoReconnectProviderProps): React.JSX.Element {
  const autoReconnect = useAutoReconnect();

  // Show loading screen during initialization if enabled
  if (showLoadingScreen && autoReconnect.state.isInitializing) {
    return <LoadingScreen />;
  }

  return (
    <AutoReconnectContext.Provider value={autoReconnect}>
      {children}
    </AutoReconnectContext.Provider>
  );
}

/**
 * Hook to access auto-reconnect context
 *
 * @returns Auto-reconnect state and actions
 * @throws Error if used outside of AutoReconnectProvider
 */
export function useAutoReconnectContext(): AutoReconnectContextValue {
  const context = useContext(AutoReconnectContext);

  if (!context) {
    throw new Error('useAutoReconnectContext must be used within AutoReconnectProvider');
  }

  return context;
}

const styles = StyleSheet.create({
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: '#FFFFFF',
  },
  loadingContainerDark: {
    backgroundColor: '#000000',
  },
  spinner: {
    marginBottom: 16,
  },
  loadingText: {
    fontSize: 16,
    color: '#1C1C1E',
  },
  loadingTextDark: {
    color: '#FFFFFF',
  },
});
