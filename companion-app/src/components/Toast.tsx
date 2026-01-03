/**
 * Toast Component
 *
 * A notification toast that appears at the top of the screen to provide
 * feedback for user actions. Supports success, error, warning, and info variants.
 *
 * Features:
 * - Animated entrance/exit with slide and fade
 * - Auto-dismisses after configurable duration
 * - Swipe to dismiss
 * - Multiple toast types with appropriate styling
 * - Accessible with ARIA live region
 */

import React, { useEffect, useRef, useCallback } from 'react';
import {
  StyleSheet,
  Text,
  View,
  Animated,
  PanResponder,
  TouchableOpacity,
  ViewStyle,
  AccessibilityInfo,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';

/**
 * Toast variant types
 */
export type ToastVariant = 'success' | 'error' | 'warning' | 'info';

/**
 * Toast data structure
 */
export interface ToastData {
  /** Unique identifier for the toast */
  id: string;
  /** Toast message */
  message: string;
  /** Toast variant (success, error, warning, info) */
  variant: ToastVariant;
  /** Duration in milliseconds (default: 3000) */
  duration?: number;
  /** Optional action button */
  action?: {
    label: string;
    onPress: () => void;
  };
}

/**
 * Toast component props
 */
export interface ToastProps {
  /** Toast data */
  toast: ToastData;
  /** Callback when toast is dismissed */
  onDismiss: (id: string) => void;
  /** Custom style */
  style?: ViewStyle;
  /** Test ID for testing */
  testID?: string;
}

/**
 * Default duration for toasts in milliseconds
 */
const DEFAULT_DURATION = 3000;

/**
 * Swipe threshold to dismiss toast
 */
const SWIPE_THRESHOLD = 50;

/**
 * Icon names for each variant
 */
const VARIANT_ICONS: Record<ToastVariant, string> = {
  success: 'checkmark-circle',
  error: 'alert-circle',
  warning: 'warning',
  info: 'information-circle',
};

/**
 * Toast Component
 *
 * Displays a notification toast with animated entrance/exit.
 */
export function Toast({
  toast,
  onDismiss,
  style,
  testID,
}: ToastProps): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();
  const insets = useSafeAreaInsets();

  // Animation values
  const translateY = useRef(new Animated.Value(-100)).current;
  const opacity = useRef(new Animated.Value(0)).current;
  const translateX = useRef(new Animated.Value(0)).current;

  // Timer ref for auto-dismiss
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Get variant-specific colors
  const getVariantColor = useCallback((): string => {
    switch (toast.variant) {
      case 'success':
        return colors.success;
      case 'error':
        return colors.error;
      case 'warning':
        return colors.warning;
      case 'info':
        return colors.info;
      default:
        return colors.info;
    }
  }, [toast.variant, colors]);

  const variantColor = getVariantColor();

  // Dismiss toast with animation
  const dismiss = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }

    Animated.parallel([
      Animated.timing(translateY, {
        toValue: -100,
        duration: 200,
        useNativeDriver: true,
      }),
      Animated.timing(opacity, {
        toValue: 0,
        duration: 200,
        useNativeDriver: true,
      }),
    ]).start(() => {
      onDismiss(toast.id);
    });
  }, [translateY, opacity, onDismiss, toast.id]);

  // Pan responder for swipe to dismiss
  const panResponder = useRef(
    PanResponder.create({
      onStartShouldSetPanResponder: () => true,
      onMoveShouldSetPanResponder: (_, gestureState) => {
        return Math.abs(gestureState.dy) > 10;
      },
      onPanResponderMove: (_, gestureState) => {
        // Only allow upward swipe
        if (gestureState.dy < 0) {
          translateY.setValue(gestureState.dy);
        }
      },
      onPanResponderRelease: (_, gestureState) => {
        if (gestureState.dy < -SWIPE_THRESHOLD) {
          dismiss();
        } else {
          // Snap back
          Animated.spring(translateY, {
            toValue: 0,
            useNativeDriver: true,
          }).start();
        }
      },
    }),
  ).current;

  // Animate in on mount
  useEffect(() => {
    Animated.parallel([
      Animated.spring(translateY, {
        toValue: 0,
        useNativeDriver: true,
        tension: 50,
        friction: 8,
      }),
      Animated.timing(opacity, {
        toValue: 1,
        duration: 200,
        useNativeDriver: true,
      }),
    ]).start();

    // Announce to screen readers
    AccessibilityInfo.announceForAccessibility(toast.message);

    // Auto-dismiss after duration
    const duration = toast.duration ?? DEFAULT_DURATION;
    timerRef.current = setTimeout(dismiss, duration);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [translateY, opacity, dismiss, toast.message, toast.duration]);

  // Handle action press
  const handleActionPress = useCallback(() => {
    toast.action?.onPress();
    dismiss();
  }, [toast.action, dismiss]);

  return (
    <Animated.View
      {...panResponder.panHandlers}
      style={[
        styles.container,
        {
          transform: [{ translateY }, { translateX }],
          opacity,
          backgroundColor: colors.card,
          borderLeftColor: variantColor,
          marginTop: insets.top + spacing.sm,
        },
        style,
      ]}
      testID={testID}
      accessibilityRole="alert"
      accessibilityLiveRegion="polite"
    >
      {/* Icon */}
      <View style={[styles.iconContainer, { backgroundColor: `${variantColor}20` }]}>
        <Icon
          name={VARIANT_ICONS[toast.variant]}
          size={20}
          color={variantColor}
        />
      </View>

      {/* Message */}
      <Text
        style={[
          styles.message,
          typography.textStyles.body,
          { color: colors.textPrimary, flex: 1 },
        ]}
        numberOfLines={2}
      >
        {toast.message}
      </Text>

      {/* Action button */}
      {toast.action && (
        <TouchableOpacity
          onPress={handleActionPress}
          style={styles.actionButton}
          hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
          accessibilityRole="button"
          accessibilityLabel={toast.action.label}
        >
          <Text style={[styles.actionText, { color: variantColor }]}>
            {toast.action.label}
          </Text>
        </TouchableOpacity>
      )}

      {/* Dismiss button */}
      <TouchableOpacity
        onPress={dismiss}
        style={styles.dismissButton}
        hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
        accessibilityRole="button"
        accessibilityLabel="Dismiss notification"
      >
        <Icon name="close" size={18} color={colors.textSecondary} />
      </TouchableOpacity>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  container: {
    position: 'absolute',
    left: 16,
    right: 16,
    top: 0,
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderRadius: 12,
    borderLeftWidth: 4,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.15,
    shadowRadius: 8,
    elevation: 8,
    zIndex: 9999,
    gap: 12,
  },
  iconContainer: {
    width: 32,
    height: 32,
    borderRadius: 16,
    justifyContent: 'center',
    alignItems: 'center',
  },
  message: {
    flex: 1,
  },
  actionButton: {
    paddingHorizontal: 8,
    paddingVertical: 4,
  },
  actionText: {
    fontSize: 14,
    fontWeight: '600',
  },
  dismissButton: {
    padding: 4,
  },
});
