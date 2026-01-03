/**
 * RustRide Companion App - Typography System
 *
 * Type scale and text styles for consistent typography.
 * Uses system fonts for optimal native appearance.
 */

import { Platform, TextStyle } from 'react-native';

/**
 * Font family constants
 * Uses system fonts for native feel on each platform
 */
export const fontFamily = {
  // Regular weight fonts
  regular: Platform.select({
    ios: 'System',
    android: 'Roboto',
    default: 'System',
  }),
  // Medium weight fonts
  medium: Platform.select({
    ios: 'System',
    android: 'Roboto-Medium',
    default: 'System',
  }),
  // Bold weight fonts (semibold on iOS)
  bold: Platform.select({
    ios: 'System',
    android: 'Roboto-Bold',
    default: 'System',
  }),
  // Monospace for numeric values
  mono: Platform.select({
    ios: 'Menlo',
    android: 'monospace',
    default: 'monospace',
  }),
} as const;

/**
 * Font size scale
 * Based on a modular scale for visual harmony
 */
export const fontSize = {
  // Extra small - labels, badges
  xs: 10,
  // Small - captions, helper text
  sm: 12,
  // Base - body text
  base: 14,
  // Medium - secondary headings, emphasized text
  md: 16,
  // Large - primary content
  lg: 18,
  // Extra large - section headings
  xl: 20,
  // 2X large - screen titles
  '2xl': 24,
  // 3X large - hero text
  '3xl': 28,
  // 4X large - large numbers/metrics
  '4xl': 32,
  // 5X large - primary metric display
  '5xl': 40,
  // 6X large - large metric display (power, etc)
  '6xl': 56,
} as const;

/**
 * Font weight values
 */
export const fontWeight = {
  regular: '400' as const,
  medium: '500' as const,
  semibold: '600' as const,
  bold: '700' as const,
} as const;

/**
 * Line height multipliers
 */
export const lineHeight = {
  tight: 1.1,
  normal: 1.4,
  relaxed: 1.6,
} as const;

/**
 * Letter spacing values
 */
export const letterSpacing = {
  tight: -0.5,
  normal: 0,
  wide: 0.5,
  wider: 1,
  widest: 1.5,
} as const;

/**
 * Pre-defined text styles for common use cases
 */
export const textStyles = {
  // Screen titles
  screenTitle: {
    fontSize: fontSize['3xl'],
    fontWeight: fontWeight.bold,
    lineHeight: fontSize['3xl'] * lineHeight.tight,
    letterSpacing: letterSpacing.tight,
  } as TextStyle,

  // Section headings
  sectionTitle: {
    fontSize: fontSize.xl,
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize.xl * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Card titles
  cardTitle: {
    fontSize: fontSize.lg,
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize.lg * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Body text
  body: {
    fontSize: fontSize.base,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.base * lineHeight.relaxed,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Secondary body text
  bodySecondary: {
    fontSize: fontSize.sm,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.sm * lineHeight.relaxed,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Caption/helper text
  caption: {
    fontSize: fontSize.xs,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.xs * lineHeight.normal,
    letterSpacing: letterSpacing.wide,
  } as TextStyle,

  // Button text
  button: {
    fontSize: fontSize.md,
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize.md * lineHeight.tight,
    letterSpacing: letterSpacing.wide,
  } as TextStyle,

  // Small button text
  buttonSmall: {
    fontSize: fontSize.sm,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.sm * lineHeight.tight,
    letterSpacing: letterSpacing.wide,
  } as TextStyle,

  // Tab bar labels
  tabLabel: {
    fontSize: fontSize.xs,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.xs * lineHeight.tight,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Labels (uppercase)
  label: {
    fontSize: fontSize.xs,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.xs * lineHeight.normal,
    letterSpacing: letterSpacing.widest,
    textTransform: 'uppercase',
  } as TextStyle,

  // Primary metric value (large)
  metricPrimary: {
    fontSize: fontSize['6xl'],
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize['6xl'] * lineHeight.tight,
    letterSpacing: letterSpacing.tight,
    fontVariant: ['tabular-nums'],
  } as TextStyle,

  // Secondary metric value
  metricSecondary: {
    fontSize: fontSize['4xl'],
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize['4xl'] * lineHeight.tight,
    letterSpacing: letterSpacing.tight,
    fontVariant: ['tabular-nums'],
  } as TextStyle,

  // Tertiary metric value (smaller cards)
  metricTertiary: {
    fontSize: fontSize['2xl'],
    fontWeight: fontWeight.semibold,
    lineHeight: fontSize['2xl'] * lineHeight.tight,
    letterSpacing: letterSpacing.normal,
    fontVariant: ['tabular-nums'],
  } as TextStyle,

  // Metric unit (watts, bpm, etc)
  metricUnit: {
    fontSize: fontSize.sm,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.sm * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Metric label (Power, Heart Rate, etc)
  metricLabel: {
    fontSize: fontSize.xs,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.xs * lineHeight.normal,
    letterSpacing: letterSpacing.widest,
    textTransform: 'uppercase',
  } as TextStyle,

  // Connection status badge
  statusBadge: {
    fontSize: fontSize.xs,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.xs * lineHeight.tight,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // Input placeholder
  inputPlaceholder: {
    fontSize: fontSize.md,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.md * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // List item title
  listTitle: {
    fontSize: fontSize.md,
    fontWeight: fontWeight.medium,
    lineHeight: fontSize.md * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,

  // List item subtitle
  listSubtitle: {
    fontSize: fontSize.sm,
    fontWeight: fontWeight.regular,
    lineHeight: fontSize.sm * lineHeight.normal,
    letterSpacing: letterSpacing.normal,
  } as TextStyle,
} as const;

/**
 * Typography type
 */
export type Typography = typeof textStyles;
