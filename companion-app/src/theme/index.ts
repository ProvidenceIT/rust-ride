/**
 * RustRide Companion App - Theme Module
 *
 * Centralized theme system for consistent styling across the app.
 * Matches the desktop app's design language from src/ui/theme.rs
 */

// Colors
export {
  darkColors,
  lightColors,
  zoneColors,
  hrZoneColors,
  getPowerZoneColor,
  getHrZoneColor,
  type ThemeColors,
  type Colors,
} from './colors';

// Typography
export {
  fontFamily,
  fontSize,
  fontWeight,
  lineHeight,
  letterSpacing,
  textStyles,
  type Typography,
} from './typography';

// Spacing
export {
  BASE_UNIT,
  spacing,
  borderRadius,
  componentSpacing,
  shadows,
  hitSlop,
  animation,
  type Spacing,
  type BorderRadius,
} from './spacing';

// Theme
export { darkTheme, lightTheme, defaultTheme, getTheme, type Theme, type ThemeMode } from './theme';

// Theme Provider and hooks
export {
  ThemeProvider,
  useTheme,
  useThemeContext,
  useIsDarkMode,
  useColors,
  useSpacing,
  useTypography,
} from './ThemeProvider';
