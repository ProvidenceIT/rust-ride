/**
 * RustRide Companion App - Theme Provider
 *
 * Provides theme context to the app with dark/light mode support.
 * Supports system theme detection and manual theme switching.
 */

import React, { createContext, useContext, useMemo, useState, useEffect, useCallback } from 'react';
import { useColorScheme, Appearance } from 'react-native';
import { Theme, ThemeMode, darkTheme, getTheme } from './theme';

/**
 * Theme context value type
 */
interface ThemeContextValue {
  /** Current theme object */
  theme: Theme;
  /** Current theme mode setting */
  themeMode: ThemeMode;
  /** Whether dark mode is active */
  isDarkMode: boolean;
  /** Set the theme mode */
  setThemeMode: (mode: ThemeMode) => void;
  /** Toggle between dark and light mode */
  toggleTheme: () => void;
}

/**
 * Theme context with default values
 */
const ThemeContext = createContext<ThemeContextValue>({
  theme: darkTheme,
  themeMode: 'system',
  isDarkMode: true,
  setThemeMode: () => {},
  toggleTheme: () => {},
});

/**
 * Theme provider props
 */
interface ThemeProviderProps {
  /** Child components */
  children: React.ReactNode;
  /** Initial theme mode (defaults to 'system') */
  initialThemeMode?: ThemeMode;
}

/**
 * Resolve theme mode to actual dark/light based on system preference
 * ColorSchemeName from React Native can be 'light', 'dark', 'unspecified', null, or undefined
 */
function resolveThemeMode(
  mode: ThemeMode,
  systemColorScheme: string | null | undefined,
): 'dark' | 'light' {
  if (mode === 'system') {
    // Only use light mode if explicitly set to 'light'
    return systemColorScheme === 'light' ? 'light' : 'dark';
  }
  return mode;
}

/**
 * Theme Provider Component
 *
 * Wraps the app to provide theme context to all children.
 * Handles system theme detection and manual theme switching.
 *
 * @example
 * ```tsx
 * <ThemeProvider>
 *   <App />
 * </ThemeProvider>
 * ```
 */
export function ThemeProvider({
  children,
  initialThemeMode = 'system',
}: ThemeProviderProps): React.JSX.Element {
  // Get system color scheme
  const systemColorScheme = useColorScheme();

  // Current theme mode setting
  const [themeMode, setThemeModeState] = useState<ThemeMode>(initialThemeMode);

  // Resolve actual theme based on mode and system preference
  const resolvedMode = resolveThemeMode(themeMode, systemColorScheme);
  const theme = getTheme(resolvedMode);
  const isDarkMode = resolvedMode === 'dark';

  // Listen for system theme changes when in system mode
  useEffect(() => {
    if (themeMode !== 'system') {
      return;
    }

    const subscription = Appearance.addChangeListener(({ colorScheme }) => {
      // Force re-render when system theme changes
      // The useColorScheme hook should handle this, but we add this as a backup
      if (colorScheme !== systemColorScheme) {
        // State change will trigger re-render
      }
    });

    return () => {
      subscription.remove();
    };
  }, [themeMode, systemColorScheme]);

  // Set theme mode
  const setThemeMode = useCallback((mode: ThemeMode) => {
    setThemeModeState(mode);
  }, []);

  // Toggle between dark and light mode
  const toggleTheme = useCallback(() => {
    setThemeModeState(current => {
      if (current === 'system') {
        // If following system, switch to the opposite of current
        return isDarkMode ? 'light' : 'dark';
      }
      // Toggle between dark and light
      return current === 'dark' ? 'light' : 'dark';
    });
  }, [isDarkMode]);

  // Memoize context value to prevent unnecessary re-renders
  const contextValue = useMemo(
    () => ({
      theme,
      themeMode,
      isDarkMode,
      setThemeMode,
      toggleTheme,
    }),
    [theme, themeMode, isDarkMode, setThemeMode, toggleTheme],
  );

  return <ThemeContext.Provider value={contextValue}>{children}</ThemeContext.Provider>;
}

/**
 * Hook to access the current theme
 *
 * @returns The current theme object
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const theme = useTheme();
 *   return (
 *     <View style={{ backgroundColor: theme.colors.background }}>
 *       <Text style={{ color: theme.colors.textPrimary }}>Hello</Text>
 *     </View>
 *   );
 * }
 * ```
 */
export function useTheme(): Theme {
  const context = useContext(ThemeContext);
  return context.theme;
}

/**
 * Hook to access full theme context including mode switching
 *
 * @returns Theme context with theme object and mode controls
 *
 * @example
 * ```tsx
 * function ThemeToggle() {
 *   const { isDarkMode, toggleTheme } = useThemeContext();
 *   return (
 *     <Switch value={isDarkMode} onValueChange={toggleTheme} />
 *   );
 * }
 * ```
 */
export function useThemeContext(): ThemeContextValue {
  return useContext(ThemeContext);
}

/**
 * Hook to check if dark mode is active
 *
 * @returns True if dark mode is active
 */
export function useIsDarkMode(): boolean {
  const context = useContext(ThemeContext);
  return context.isDarkMode;
}

/**
 * Hook to get themed colors
 *
 * @returns The color palette for the current theme
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const colors = useColors();
 *   return (
 *     <View style={{ backgroundColor: colors.surface }}>
 *       <Text style={{ color: colors.textPrimary }}>Hello</Text>
 *     </View>
 *   );
 * }
 * ```
 */
export function useColors() {
  const theme = useTheme();
  return theme.colors;
}

/**
 * Hook to get themed spacing
 *
 * @returns The spacing values
 */
export function useSpacing() {
  const theme = useTheme();
  return theme.spacing;
}

/**
 * Hook to get themed typography
 *
 * @returns The typography system
 */
export function useTypography() {
  const theme = useTheme();
  return theme.typography;
}
