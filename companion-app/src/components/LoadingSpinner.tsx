/**
 * LoadingSpinner Component
 *
 * An animated loading spinner for async operations.
 * Supports multiple sizes and optional message text.
 */

import React from 'react';
import { View, Text, ActivityIndicator, StyleSheet, ViewStyle, TextStyle } from 'react-native';
import { useTheme } from '@/theme';

/**
 * LoadingSpinner sizes
 */
export type LoadingSpinnerSize = 'small' | 'medium' | 'large';

/**
 * LoadingSpinner props
 */
export interface LoadingSpinnerProps {
  /** Spinner size */
  size?: LoadingSpinnerSize;
  /** Optional loading message */
  message?: string;
  /** Custom spinner color */
  color?: string;
  /** Whether to center in the container */
  centered?: boolean;
  /** Whether to overlay on top of content */
  overlay?: boolean;
  /** Custom container style */
  style?: ViewStyle;
  /** Custom text style */
  textStyle?: TextStyle;
}

/**
 * Get ActivityIndicator size prop
 */
function getIndicatorSize(size: LoadingSpinnerSize): 'small' | 'large' {
  // React Native ActivityIndicator only supports 'small' and 'large'
  switch (size) {
    case 'small':
      return 'small';
    case 'medium':
    case 'large':
    default:
      return 'large';
  }
}

/**
 * Get custom spinner dimensions for consistent sizing
 */
function getSpinnerDimensions(size: LoadingSpinnerSize) {
  switch (size) {
    case 'small':
      return { containerSize: 24, scale: 0.8 };
    case 'medium':
      return { containerSize: 40, scale: 1 };
    case 'large':
      return { containerSize: 64, scale: 1.2 };
  }
}

/**
 * LoadingSpinner Component
 *
 * Shows a loading spinner with optional message text.
 * Useful for indicating async operations.
 *
 * @example
 * ```tsx
 * <LoadingSpinner size="large" message="Loading rides..." />
 * ```
 */
export function LoadingSpinner({
  size = 'medium',
  message,
  color,
  centered = true,
  overlay = false,
  style,
  textStyle,
}: LoadingSpinnerProps): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();

  const spinnerColor = color || colors.accent;
  const indicatorSize = getIndicatorSize(size);
  const { scale } = getSpinnerDimensions(size);

  // Container styles based on props
  const containerStyle: ViewStyle = {
    ...(centered && styles.centered),
    ...(overlay && {
      ...StyleSheet.absoluteFillObject,
      backgroundColor: colors.overlay,
      zIndex: 1000,
    }),
  };

  return (
    <View
      style={[styles.container, containerStyle, style]}
      accessible
      accessibilityRole="progressbar"
      accessibilityLabel={message || 'Loading'}
      accessibilityState={{ busy: true }}
    >
      <View style={[styles.spinnerContainer, { transform: [{ scale }] }]}>
        <ActivityIndicator size={indicatorSize} color={spinnerColor} />
      </View>
      {message && (
        <Text
          style={[
            styles.message,
            typography.textStyles.body,
            { color: colors.textSecondary, marginTop: spacing.md },
            textStyle,
          ]}
        >
          {message}
        </Text>
      )}
    </View>
  );
}

/**
 * FullScreenLoader Component
 *
 * A loading spinner that covers the entire screen with an overlay.
 * Useful for blocking UI during critical operations.
 *
 * @example
 * ```tsx
 * {isLoading && <FullScreenLoader message="Connecting..." />}
 * ```
 */
export function FullScreenLoader({
  message,
  color,
}: Pick<LoadingSpinnerProps, 'message' | 'color'>): React.JSX.Element {
  return <LoadingSpinner size="large" message={message} color={color} overlay centered />;
}

/**
 * InlineLoader Component
 *
 * A small inline loading indicator for use within content.
 * Does not center itself and takes minimal space.
 *
 * @example
 * ```tsx
 * <Text>
 *   Loading data <InlineLoader />
 * </Text>
 * ```
 */
export function InlineLoader({
  color,
  style,
}: Pick<LoadingSpinnerProps, 'color' | 'style'>): React.JSX.Element {
  return <LoadingSpinner size="small" color={color} centered={false} style={style} />;
}

const styles = StyleSheet.create({
  container: {
    alignItems: 'center',
    justifyContent: 'center',
  },
  centered: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  spinnerContainer: {
    // Scale transform applied inline
  },
  message: {
    textAlign: 'center',
  },
});
