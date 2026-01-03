/**
 * useThemeSync Hook
 *
 * Syncs the persisted theme preference from settingsStore with the ThemeProvider.
 * This ensures that the app respects the user's saved theme preference on launch
 * and keeps both in sync when changes are made.
 */

import { useEffect, useRef } from 'react';
import { useSettingsStore, selectTheme, selectIsLoaded } from '@/stores/settingsStore';
import { useThemeContext } from '@/theme';

/**
 * Hook that synchronizes the theme between settingsStore and ThemeProvider
 *
 * This hook should be used at the app root level to ensure the theme
 * is properly synced on app launch and when settings change.
 *
 * @example
 * ```tsx
 * function App() {
 *   useThemeSync();
 *   return <NavigationContainer>...</NavigationContainer>;
 * }
 * ```
 */
export function useThemeSync(): void {
  const { setThemeMode } = useThemeContext();
  const theme = useSettingsStore(selectTheme);
  const isLoaded = useSettingsStore(selectIsLoaded);
  const loadSettings = useSettingsStore(state => state.loadSettings);

  // Track if we've applied the initial theme to avoid duplicate updates
  const hasAppliedInitialTheme = useRef(false);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Sync theme when settings are loaded or theme changes
  useEffect(() => {
    if (isLoaded && !hasAppliedInitialTheme.current) {
      // Apply the persisted theme on initial load
      setThemeMode(theme);
      hasAppliedInitialTheme.current = true;
    } else if (isLoaded && hasAppliedInitialTheme.current) {
      // Sync when theme changes after initial load
      setThemeMode(theme);
    }
  }, [isLoaded, theme, setThemeMode]);
}

/**
 * Hook to check if dark mode is currently active
 * Convenience wrapper around useThemeContext for components that only need
 * to know if dark mode is active.
 */
export { useIsDarkMode } from '@/theme';
