/**
 * IconButton Component
 *
 * A button that displays only an icon, commonly used for toolbar actions.
 * Supports haptic feedback and accessibility features.
 */

import React, { useCallback } from 'react';
import {
  Pressable,
  ViewStyle,
  PressableProps,
  ActivityIndicator,
} from 'react-native';
import { useTheme, hitSlop as themeHitSlop } from '@/theme';

/**
 * IconButton variants
 */
export type IconButtonVariant = 'default' | 'primary' | 'danger' | 'ghost';

/**
 * IconButton sizes
 */
export type IconButtonSize = 'small' | 'medium' | 'large';

/**
 * Common icon component props
 */
interface IconProps {
  size?: number;
  color?: string;
}

/**
 * IconButton props
 */
export interface IconButtonProps extends Omit<PressableProps, 'style'> {
  /** Icon element to render */
  icon: React.ReactElement<IconProps>;
  /** Button variant */
  variant?: IconButtonVariant;
  /** Button size */
  size?: IconButtonSize;
  /** Whether the button is in a loading state */
  loading?: boolean;
  /** Whether the button should have a circular shape */
  circular?: boolean;
  /** Custom container style */
  style?: ViewStyle;
  /** Accessibility label (required for icon-only buttons) */
  accessibilityLabel: string;
}

/**
 * IconButton Component
 *
 * A button component that displays only an icon, used for actions like
 * play/pause, skip, stop, etc.
 *
 * @example
 * ```tsx
 * <IconButton
 *   icon={<Icon name="play" size={24} color="#fff" />}
 *   variant="primary"
 *   onPress={() => handlePlay()}
 *   accessibilityLabel="Play workout"
 * />
 * ```
 */
export function IconButton({
  icon,
  variant = 'default',
  size = 'medium',
  loading = false,
  circular = false,
  disabled,
  style,
  accessibilityLabel,
  onPress,
  ...pressableProps
}: IconButtonProps): React.JSX.Element {
  const { colors, borderRadius: themeRadius } = useTheme();

  // Get variant-specific colors
  const variantColors = getVariantColors(variant, colors);

  // Get size-specific dimensions
  const sizeStyles = getSizeStyles(size);

  // Handle press with loading state
  const handlePress = useCallback(
    (event: Parameters<NonNullable<PressableProps['onPress']>>[0]) => {
      if (!loading && onPress) {
        onPress(event);
      }
    },
    [loading, onPress],
  );

  // Dynamic button style
  const getButtonStyle = ({ pressed }: { pressed: boolean }): ViewStyle => {
    const baseStyle: ViewStyle = {
      width: sizeStyles.size,
      height: sizeStyles.size,
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: circular ? sizeStyles.size / 2 : themeRadius.sm,
      backgroundColor: variantColors.background,
      opacity: disabled || loading ? 0.5 : 1,
    };

    // Apply pressed state
    if (pressed && !disabled && !loading) {
      baseStyle.backgroundColor = variantColors.backgroundPressed;
    }

    return baseStyle;
  };

  // Clone icon with proper color if not in loading state
  const renderedIcon = loading ? (
    <ActivityIndicator size={sizeStyles.iconSize} color={variantColors.icon} />
  ) : (
    React.cloneElement(icon, {
      size: icon.props.size || sizeStyles.iconSize,
      color: icon.props.color || variantColors.icon,
    })
  );

  // Accessibility hint based on state
  const accessibilityHint = loading
    ? 'Button is loading'
    : disabled
      ? 'Button is disabled'
      : undefined;

  return (
    <Pressable
      style={({ pressed }) => [getButtonStyle({ pressed }), style]}
      onPress={handlePress}
      disabled={disabled || loading}
      hitSlop={themeHitSlop.medium}
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      accessibilityState={{
        disabled: disabled || loading,
        busy: loading,
      }}
      {...pressableProps}
    >
      {renderedIcon}
    </Pressable>
  );
}

/**
 * Get variant-specific colors
 */
function getVariantColors(
  variant: IconButtonVariant,
  colors: ReturnType<typeof useTheme>['colors'],
) {
  switch (variant) {
    case 'primary':
      return {
        background: colors.accent,
        backgroundPressed: colors.accentDark,
        icon: colors.textInverse,
      };
    case 'danger':
      return {
        background: colors.error,
        backgroundPressed: '#B02020',
        icon: colors.textInverse,
      };
    case 'ghost':
      return {
        background: 'transparent',
        backgroundPressed: colors.surface,
        icon: colors.textPrimary,
      };
    case 'default':
    default:
      return {
        background: colors.surface,
        backgroundPressed: colors.elevated,
        icon: colors.textPrimary,
      };
  }
}

/**
 * Get size-specific dimensions
 */
function getSizeStyles(size: IconButtonSize) {
  switch (size) {
    case 'large':
      return {
        size: 56,
        iconSize: 28,
      };
    case 'medium':
      return {
        size: 44,
        iconSize: 24,
      };
    case 'small':
      return {
        size: 32,
        iconSize: 18,
      };
  }
}

// Static styles (currently none - all styles are dynamic)
