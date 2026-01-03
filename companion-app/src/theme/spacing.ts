/**
 * RustRide Companion App - Spacing System
 *
 * Consistent spacing values for margins, padding, and gaps.
 * Based on a 4px base unit for visual harmony.
 */

/**
 * Base spacing unit in pixels
 */
export const BASE_UNIT = 4;

/**
 * Spacing scale
 * All values are multiples of 4px for consistency
 */
export const spacing = {
  // 0px - no spacing
  none: 0,
  // 2px - hairline spacing
  hairline: 2,
  // 4px - minimal spacing
  xs: 4,
  // 8px - small spacing
  sm: 8,
  // 12px - medium-small spacing
  md: 12,
  // 16px - standard spacing
  lg: 16,
  // 20px - medium-large spacing
  xl: 20,
  // 24px - large spacing
  '2xl': 24,
  // 32px - extra large spacing
  '3xl': 32,
  // 40px - huge spacing
  '4xl': 40,
  // 48px - massive spacing
  '5xl': 48,
  // 64px - gigantic spacing
  '6xl': 64,
} as const;

/**
 * Border radius values
 */
export const borderRadius = {
  // No radius
  none: 0,
  // Small radius - buttons, inputs
  sm: 4,
  // Medium radius - cards
  md: 8,
  // Large radius - modals, sheets
  lg: 12,
  // Extra large radius - large cards
  xl: 16,
  // 2X large radius - pills, badges
  '2xl': 20,
  // Full radius - circular elements
  full: 9999,
} as const;

/**
 * Component-specific spacing
 */
export const componentSpacing = {
  // Screen padding
  screenPaddingHorizontal: spacing.lg,
  screenPaddingVertical: spacing.md,

  // Card padding
  cardPadding: spacing.lg,
  cardPaddingSmall: spacing.md,
  cardPaddingLarge: spacing['2xl'],

  // Button padding
  buttonPaddingHorizontal: spacing.lg,
  buttonPaddingVertical: spacing.md,
  buttonPaddingHorizontalSmall: spacing.md,
  buttonPaddingVerticalSmall: spacing.sm,

  // Input padding
  inputPaddingHorizontal: spacing.md,
  inputPaddingVertical: spacing.md,

  // List item padding
  listItemPaddingHorizontal: spacing.lg,
  listItemPaddingVertical: spacing.md,

  // Header/navigation heights
  headerHeight: 56,
  tabBarHeight: 60,
  statusBarHeight: 44, // iOS default, will be overridden by SafeAreaView

  // Gap between elements
  gridGap: spacing.md,
  sectionGap: spacing['2xl'],
  itemGap: spacing.sm,

  // Icon sizes
  iconSizeSmall: 16,
  iconSizeMedium: 24,
  iconSizeLarge: 32,
  iconSizeXLarge: 48,

  // Metric card sizes
  metricCardMinHeight: 100,
  metricCardLargeMinHeight: 160,

  // Badge/status indicator sizes
  statusDotSize: 8,
  statusDotSizeLarge: 12,
  badgeHeight: 24,
  badgeHeightSmall: 20,
} as const;

/**
 * Shadow definitions for elevation
 */
export const shadows = {
  // No shadow
  none: {
    shadowColor: 'transparent',
    shadowOffset: { width: 0, height: 0 },
    shadowOpacity: 0,
    shadowRadius: 0,
    elevation: 0,
  },
  // Small shadow - cards
  sm: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.18,
    shadowRadius: 1.0,
    elevation: 1,
  },
  // Medium shadow - elevated cards
  md: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.2,
    shadowRadius: 2.0,
    elevation: 3,
  },
  // Large shadow - modals, overlays
  lg: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.22,
    shadowRadius: 5.0,
    elevation: 5,
  },
  // Extra large shadow - floating buttons
  xl: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.25,
    shadowRadius: 8.0,
    elevation: 8,
  },
} as const;

/**
 * Hit slop for touchable elements
 * Increases touch target without changing visual size
 */
export const hitSlop = {
  small: {
    top: 8,
    right: 8,
    bottom: 8,
    left: 8,
  },
  medium: {
    top: 12,
    right: 12,
    bottom: 12,
    left: 12,
  },
  large: {
    top: 16,
    right: 16,
    bottom: 16,
    left: 16,
  },
} as const;

/**
 * Animation durations in milliseconds
 */
export const animation = {
  // Fast transitions
  fast: 150,
  // Normal transitions
  normal: 250,
  // Slow transitions
  slow: 350,
  // Very slow transitions (page transitions)
  verySlow: 500,
} as const;

/**
 * Spacing type
 */
export type Spacing = typeof spacing;

/**
 * Border radius type
 */
export type BorderRadius = typeof borderRadius;
