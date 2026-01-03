/**
 * RustRide Companion App - Theme Definitions
 *
 * Complete theme objects combining colors, typography, and spacing.
 */

import { darkColors, lightColors, type ThemeColors } from './colors';
import { textStyles, fontSize, fontWeight, lineHeight, letterSpacing } from './typography';
import { spacing, borderRadius, componentSpacing, shadows, hitSlop, animation } from './spacing';

/**
 * Theme mode enum matching desktop app
 */
export type ThemeMode = 'dark' | 'light' | 'system';

/**
 * Complete theme object type
 */
export interface Theme {
  /** Theme mode identifier */
  mode: 'dark' | 'light';

  /** Color palette */
  colors: ThemeColors;

  /** Typography system */
  typography: {
    textStyles: typeof textStyles;
    fontSize: typeof fontSize;
    fontWeight: typeof fontWeight;
    lineHeight: typeof lineHeight;
    letterSpacing: typeof letterSpacing;
  };

  /** Spacing system */
  spacing: typeof spacing;

  /** Border radius values */
  borderRadius: typeof borderRadius;

  /** Component-specific spacing */
  componentSpacing: typeof componentSpacing;

  /** Shadow definitions */
  shadows: typeof shadows;

  /** Touch hit slop values */
  hitSlop: typeof hitSlop;

  /** Animation timing values */
  animation: typeof animation;
}

/**
 * Dark theme definition
 */
export const darkTheme: Theme = {
  mode: 'dark',
  colors: darkColors,
  typography: {
    textStyles,
    fontSize,
    fontWeight,
    lineHeight,
    letterSpacing,
  },
  spacing,
  borderRadius,
  componentSpacing,
  shadows,
  hitSlop,
  animation,
};

/**
 * Light theme definition
 */
export const lightTheme: Theme = {
  mode: 'light',
  colors: lightColors,
  typography: {
    textStyles,
    fontSize,
    fontWeight,
    lineHeight,
    letterSpacing,
  },
  spacing,
  borderRadius,
  componentSpacing,
  shadows,
  hitSlop,
  animation,
};

/**
 * Get theme by mode
 */
export function getTheme(mode: 'dark' | 'light'): Theme {
  return mode === 'dark' ? darkTheme : lightTheme;
}

/**
 * Default theme (dark mode)
 */
export const defaultTheme = darkTheme;
