/**
 * RustRide Companion App
 *
 * Mobile companion app for remote workout control, real-time metrics viewing,
 * and ride history access. Connects to the desktop app over local network.
 *
 * Supports dark and light themes with system preference detection and manual toggle.
 *
 * @format
 */

import React from 'react';
import { StatusBar } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavigationContainer, Theme as NavTheme } from '@react-navigation/native';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { AppNavigator, linking } from '@/navigation';
import { AutoReconnectProvider, GlobalErrorHandlerProvider } from '@/providers';
import { ToastProvider, useKeepAwake, useThemeSync } from '@/hooks';
import { ToastContainer } from '@/components';
import { ThemeProvider, useTheme, useIsDarkMode } from '@/theme';

/**
 * KeepAwakeManager Component
 *
 * Manages screen wake lock based on session state and user settings.
 * Must be rendered inside providers that give access to stores.
 */
function KeepAwakeManager(): null {
  useKeepAwake();
  return null;
}

/**
 * ThemeSyncManager Component
 *
 * Syncs the persisted theme preference from settings with the ThemeProvider.
 * Must be rendered inside ThemeProvider context.
 */
function ThemeSyncManager(): null {
  useThemeSync();
  return null;
}

/**
 * Build React Navigation theme from our theme system
 */
function useNavigationTheme(): NavTheme {
  const theme = useTheme();
  const isDarkMode = useIsDarkMode();

  return {
    dark: isDarkMode,
    colors: {
      primary: theme.colors.accent,
      background: theme.colors.background,
      card: theme.colors.surface,
      text: theme.colors.textPrimary,
      border: theme.colors.border,
      notification: theme.colors.error,
    },
    fonts: {
      regular: {
        fontFamily: 'System',
        fontWeight: '400',
      },
      medium: {
        fontFamily: 'System',
        fontWeight: '500',
      },
      bold: {
        fontFamily: 'System',
        fontWeight: '700',
      },
      heavy: {
        fontFamily: 'System',
        fontWeight: '900',
      },
    },
  };
}

/**
 * AppContent Component
 *
 * Main app content that uses theme hooks.
 * Must be rendered inside ThemeProvider context.
 */
function AppContent(): React.JSX.Element {
  const theme = useTheme();
  const isDarkMode = useIsDarkMode();
  const navigationTheme = useNavigationTheme();

  return (
    <>
      <StatusBar
        barStyle={isDarkMode ? 'light-content' : 'dark-content'}
        backgroundColor={theme.colors.background}
      />
      <ToastProvider>
        <GlobalErrorHandlerProvider>
          <AutoReconnectProvider>
            <ThemeSyncManager />
            <KeepAwakeManager />
            <NavigationContainer linking={linking} theme={navigationTheme}>
              <AppNavigator />
            </NavigationContainer>
          </AutoReconnectProvider>
        </GlobalErrorHandlerProvider>
        <ToastContainer testID="toast-container" />
      </ToastProvider>
    </>
  );
}

/**
 * App Component
 *
 * Root component that sets up providers and navigation.
 * Wraps everything in ThemeProvider for consistent theming.
 */
function App(): React.JSX.Element {
  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <SafeAreaProvider>
        <ThemeProvider>
          <AppContent />
        </ThemeProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}

export default App;
