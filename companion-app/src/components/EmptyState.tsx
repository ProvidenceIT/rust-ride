/**
 * EmptyState Component
 *
 * A reusable component for displaying empty states with helpful messages.
 * Provides consistent styling and animations across all screens.
 *
 * Variants:
 * - no-rides: When ride history is empty
 * - no-connection: When not connected to server
 * - error: When an error occurs
 * - no-results: When a filter/search returns no results
 * - offline: When viewing cached data or offline
 * - loading-failed: When data loading fails
 * - custom: For custom empty states
 */

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
  Animated,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { Button } from './Button';

/**
 * Empty state variant types
 */
export type EmptyStateVariant =
  | 'no-rides'
  | 'no-connection'
  | 'error'
  | 'no-results'
  | 'offline'
  | 'loading-failed'
  | 'no-session'
  | 'custom';

/**
 * Predefined empty state configurations
 */
interface EmptyStateConfig {
  icon: string;
  iconColor?: string;
  title: string;
  description: string;
  actionLabel?: string;
}

/**
 * Get default configuration for each variant
 */
function getDefaultConfig(
  variant: EmptyStateVariant,
  colors: ReturnType<typeof useTheme>['colors']
): EmptyStateConfig {
  switch (variant) {
    case 'no-rides':
      return {
        icon: 'bicycle-outline',
        title: 'No Rides Yet',
        description: 'Complete a ride on your desktop app and it will appear here.',
      };
    case 'no-connection':
      return {
        icon: 'desktop-outline',
        iconColor: colors.textMuted,
        title: 'Not Connected',
        description: 'Connect to your RustRide desktop app to view data and control workouts.',
        actionLabel: 'Connect',
      };
    case 'error':
      return {
        icon: 'warning-outline',
        iconColor: colors.error,
        title: 'Something Went Wrong',
        description: 'An error occurred while loading data. Please try again.',
        actionLabel: 'Try Again',
      };
    case 'no-results':
      return {
        icon: 'filter-outline',
        title: 'No Matching Results',
        description: 'No items match your current filters. Try adjusting your filters or clear them.',
        actionLabel: 'Clear Filters',
      };
    case 'offline':
      return {
        icon: 'cloud-offline-outline',
        iconColor: colors.warning,
        title: 'You\'re Offline',
        description: 'Connect to the internet to sync your data.',
        actionLabel: 'Retry',
      };
    case 'loading-failed':
      return {
        icon: 'reload-outline',
        iconColor: colors.error,
        title: 'Failed to Load',
        description: 'We couldn\'t load the data. Please check your connection and try again.',
        actionLabel: 'Try Again',
      };
    case 'no-session':
      return {
        icon: 'bicycle-outline',
        iconColor: colors.accent,
        title: 'Ready to Ride',
        description: 'Start a workout or free ride on the desktop app to see live metrics here.',
      };
    case 'custom':
    default:
      return {
        icon: 'information-circle-outline',
        title: '',
        description: '',
      };
  }
}

/**
 * EmptyState props
 */
export interface EmptyStateProps {
  /** Empty state variant - determines default icon, title, and description */
  variant: EmptyStateVariant;
  /** Custom icon name (overrides variant default) */
  icon?: string;
  /** Custom icon color (overrides variant default) */
  iconColor?: string;
  /** Custom title (overrides variant default) */
  title?: string;
  /** Custom description (overrides variant default) */
  description?: string;
  /** Action button label */
  actionLabel?: string;
  /** Action button callback */
  onAction?: () => void;
  /** Secondary action button label */
  secondaryActionLabel?: string;
  /** Secondary action button callback */
  onSecondaryAction?: () => void;
  /** Whether action is loading */
  isActionLoading?: boolean;
  /** Whether to animate the icon */
  animateIcon?: boolean;
  /** Additional content to render below description */
  children?: React.ReactNode;
  /** Custom container style */
  style?: ViewStyle;
  /** Test ID for testing */
  testID?: string;
}

/**
 * EmptyState Component
 *
 * Displays a helpful empty state with icon, title, description, and optional action button.
 * Use this component for consistent empty state UI across all screens.
 *
 * @example
 * ```tsx
 * // Using predefined variant
 * <EmptyState
 *   variant="no-rides"
 *   onAction={() => navigation.navigate('Dashboard')}
 * />
 *
 * // Custom empty state
 * <EmptyState
 *   variant="custom"
 *   icon="search-outline"
 *   title="No Results Found"
 *   description="Try a different search term"
 *   actionLabel="Clear Search"
 *   onAction={() => clearSearch()}
 * />
 * ```
 */
export function EmptyState({
  variant,
  icon,
  iconColor,
  title,
  description,
  actionLabel,
  onAction,
  secondaryActionLabel,
  onSecondaryAction,
  isActionLoading = false,
  animateIcon = false,
  children,
  style,
  testID,
}: EmptyStateProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  // Get default config for variant
  const config = getDefaultConfig(variant, colors);

  // Merge with overrides
  const displayIcon = icon || config.icon;
  const displayIconColor = iconColor || config.iconColor || colors.textSecondary;
  const displayTitle = title ?? config.title;
  const displayDescription = description ?? config.description;
  const displayActionLabel = actionLabel || config.actionLabel;

  // Animation for icon (subtle floating effect)
  const animatedValue = React.useRef(new Animated.Value(0)).current;

  React.useEffect(() => {
    if (animateIcon) {
      const animation = Animated.loop(
        Animated.sequence([
          Animated.timing(animatedValue, {
            toValue: 1,
            duration: 1500,
            useNativeDriver: true,
          }),
          Animated.timing(animatedValue, {
            toValue: 0,
            duration: 1500,
            useNativeDriver: true,
          }),
        ])
      );
      animation.start();
      return () => animation.stop();
    }
    return undefined;
  }, [animateIcon, animatedValue]);

  const translateY = animatedValue.interpolate({
    inputRange: [0, 1],
    outputRange: [0, -8],
  });

  const iconContainerStyle = animateIcon
    ? [
        styles.iconContainer,
        {
          backgroundColor: colors.surface,
          borderRadius: borderRadius.full,
          transform: [{ translateY }],
        },
      ]
    : [
        styles.iconContainer,
        {
          backgroundColor: colors.surface,
          borderRadius: borderRadius.full,
        },
      ];

  const IconContainer = animateIcon ? Animated.View : View;

  return (
    <View
      style={[
        styles.container,
        {
          backgroundColor: colors.card,
          borderRadius: borderRadius.lg,
          padding: spacing.xl,
        },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={`${displayTitle}. ${displayDescription}`}
      testID={testID}
    >
      {/* Icon with circular background */}
      <IconContainer style={iconContainerStyle}>
        <Icon
          name={displayIcon}
          size={48}
          color={displayIconColor}
        />
      </IconContainer>

      {/* Title */}
      {displayTitle ? (
        <Text
          style={[
            styles.title,
            typography.textStyles.sectionTitle,
            { color: colors.textPrimary, marginTop: spacing.lg },
          ]}
        >
          {displayTitle}
        </Text>
      ) : null}

      {/* Description */}
      {displayDescription ? (
        <Text
          style={[
            styles.description,
            typography.textStyles.body,
            { color: colors.textSecondary, marginTop: spacing.sm },
          ]}
        >
          {displayDescription}
        </Text>
      ) : null}

      {/* Additional content */}
      {children}

      {/* Action buttons */}
      {(displayActionLabel && onAction) || (secondaryActionLabel && onSecondaryAction) ? (
        <View style={[styles.actions, { marginTop: spacing.lg, gap: spacing.sm }]}>
          {displayActionLabel && onAction && (
            <Button
              title={displayActionLabel}
              variant="primary"
              onPress={onAction}
              loading={isActionLoading}
              disabled={isActionLoading}
              fullWidth
              accessibilityHint={`Tap to ${displayActionLabel.toLowerCase()}`}
            />
          )}
          {secondaryActionLabel && onSecondaryAction && (
            <Button
              title={secondaryActionLabel}
              variant="outline"
              onPress={onSecondaryAction}
              disabled={isActionLoading}
              fullWidth
            />
          )}
        </View>
      ) : null}
    </View>
  );
}

/**
 * Compact empty state for inline use within lists
 */
export interface CompactEmptyStateProps {
  /** Icon name */
  icon?: string;
  /** Message to display */
  message: string;
  /** Action button label */
  actionLabel?: string;
  /** Action callback */
  onAction?: () => void;
  /** Custom style */
  style?: ViewStyle;
  /** Test ID */
  testID?: string;
}

/**
 * CompactEmptyState Component
 *
 * A smaller, inline empty state for use within lists or smaller containers.
 *
 * @example
 * ```tsx
 * <CompactEmptyState
 *   icon="search-outline"
 *   message="No results found"
 *   actionLabel="Clear search"
 *   onAction={() => clearSearch()}
 * />
 * ```
 */
export function CompactEmptyState({
  icon = 'information-circle-outline',
  message,
  actionLabel,
  onAction,
  style,
  testID,
}: CompactEmptyStateProps): React.JSX.Element {
  const { colors, spacing } = useTheme();

  return (
    <View
      style={[
        styles.compactContainer,
        { paddingVertical: spacing.lg, paddingHorizontal: spacing.md },
        style,
      ]}
      accessible
      accessibilityRole="text"
      accessibilityLabel={message}
      testID={testID}
    >
      <Icon name={icon} size={32} color={colors.textMuted} />
      <Text
        style={[
          styles.compactMessage,
          { color: colors.textSecondary, marginTop: spacing.sm },
        ]}
      >
        {message}
      </Text>
      {actionLabel && onAction && (
        <Button
          title={actionLabel}
          variant="ghost"
          size="small"
          onPress={onAction}
          style={{ marginTop: spacing.sm }}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    alignItems: 'center',
  },
  iconContainer: {
    width: 96,
    height: 96,
    alignItems: 'center',
    justifyContent: 'center',
  },
  title: {
    textAlign: 'center',
  },
  description: {
    textAlign: 'center',
    lineHeight: 22,
    maxWidth: 300,
  },
  actions: {
    width: '100%',
  },
  compactContainer: {
    alignItems: 'center',
    justifyContent: 'center',
  },
  compactMessage: {
    fontSize: 14,
    textAlign: 'center',
    lineHeight: 20,
  },
});
