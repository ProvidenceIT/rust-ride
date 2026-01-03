/**
 * Button Component
 *
 * Customizable button with multiple variants and sizes.
 * Supports primary, secondary, outline, and danger variants.
 */

import React, { useCallback } from 'react';
import {
  Pressable,
  Text,
  StyleSheet,
  ViewStyle,
  TextStyle,
  PressableProps,
  ActivityIndicator,
} from 'react-native';
import { useTheme } from '@/theme';

/**
 * Button variants
 */
export type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'danger' | 'ghost';

/**
 * Button sizes
 */
export type ButtonSize = 'small' | 'medium' | 'large';

/**
 * Button props
 */
export interface ButtonProps extends Omit<PressableProps, 'style'> {
  /** Button text */
  title: string;
  /** Button variant */
  variant?: ButtonVariant;
  /** Button size */
  size?: ButtonSize;
  /** Whether the button is in a loading state */
  loading?: boolean;
  /** Whether the button takes full width */
  fullWidth?: boolean;
  /** Custom container style */
  style?: ViewStyle;
  /** Custom text style */
  textStyle?: TextStyle;
  /** Left icon element */
  leftIcon?: React.ReactNode;
  /** Right icon element */
  rightIcon?: React.ReactNode;
}

/**
 * Button Component
 *
 * A customizable button component with multiple variants for different actions.
 *
 * @example
 * ```tsx
 * <Button
 *   title="Pause Workout"
 *   variant="primary"
 *   onPress={() => handlePause()}
 * />
 * ```
 */
export function Button({
  title,
  variant = 'primary',
  size = 'medium',
  loading = false,
  fullWidth = false,
  disabled,
  style,
  textStyle,
  leftIcon,
  rightIcon,
  onPress,
  ...pressableProps
}: ButtonProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius: themeRadius } = useTheme();

  // Get variant-specific colors
  const variantColors = getVariantColors(variant, colors);

  // Get size-specific styles
  const sizeStyles = getSizeStyles(size, spacing, typography);

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
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: themeRadius.sm,
      paddingHorizontal: sizeStyles.paddingHorizontal,
      paddingVertical: sizeStyles.paddingVertical,
      backgroundColor: variantColors.background,
      borderWidth: variant === 'outline' ? 1 : 0,
      borderColor: variant === 'outline' ? variantColors.border : undefined,
      opacity: disabled || loading ? 0.5 : 1,
      ...(fullWidth ? { width: '100%' } : {}),
    };

    // Apply pressed state
    if (pressed && !disabled && !loading) {
      baseStyle.backgroundColor = variantColors.backgroundPressed;
    }

    return baseStyle;
  };

  // Accessibility props
  const accessibilityLabel = pressableProps.accessibilityLabel || title;
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
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      accessibilityState={{
        disabled: disabled || loading,
        busy: loading,
      }}
      {...pressableProps}
    >
      {loading ? (
        <ActivityIndicator
          size="small"
          color={variantColors.text}
          style={styles.loadingIndicator}
        />
      ) : (
        <>
          {leftIcon && <>{leftIcon}</>}
          <Text
            style={[
              styles.text,
              sizeStyles.text,
              { color: variantColors.text },
              leftIcon ? styles.textWithLeftIcon : undefined,
              rightIcon ? styles.textWithRightIcon : undefined,
              textStyle,
            ]}
            numberOfLines={1}
          >
            {title}
          </Text>
          {rightIcon && <>{rightIcon}</>}
        </>
      )}
    </Pressable>
  );
}

/**
 * Get variant-specific colors
 */
function getVariantColors(
  variant: ButtonVariant,
  colors: ReturnType<typeof useTheme>['colors'],
) {
  switch (variant) {
    case 'primary':
      return {
        background: colors.accent,
        backgroundPressed: colors.accentDark,
        text: colors.textInverse,
        border: colors.accent,
      };
    case 'secondary':
      return {
        background: colors.surface,
        backgroundPressed: colors.elevated,
        text: colors.textPrimary,
        border: colors.border,
      };
    case 'outline':
      return {
        background: 'transparent',
        backgroundPressed: colors.surface,
        text: colors.accent,
        border: colors.accent,
      };
    case 'danger':
      return {
        background: colors.error,
        backgroundPressed: '#B02020',
        text: colors.textInverse,
        border: colors.error,
      };
    case 'ghost':
      return {
        background: 'transparent',
        backgroundPressed: colors.surface,
        text: colors.textPrimary,
        border: 'transparent',
      };
  }
}

/**
 * Get size-specific styles
 */
function getSizeStyles(
  size: ButtonSize,
  spacing: ReturnType<typeof useTheme>['spacing'],
  typography: ReturnType<typeof useTheme>['typography'],
) {
  const { textStyles } = typography;
  switch (size) {
    case 'large':
      return {
        paddingHorizontal: spacing.xl,
        paddingVertical: spacing.lg,
        text: {
          ...textStyles.button,
          fontSize: 18,
        },
      };
    case 'medium':
      return {
        paddingHorizontal: spacing.lg,
        paddingVertical: spacing.md,
        text: textStyles.button,
      };
    case 'small':
      return {
        paddingHorizontal: spacing.md,
        paddingVertical: spacing.sm,
        text: textStyles.buttonSmall,
      };
  }
}

const styles = StyleSheet.create({
  text: {
    textAlign: 'center',
  },
  textWithLeftIcon: {
    marginLeft: 8,
  },
  textWithRightIcon: {
    marginRight: 8,
  },
  loadingIndicator: {
    marginVertical: 2,
  },
});
