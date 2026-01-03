/**
 * ConnectionStatus Component
 *
 * Displays the current connection status to the RustRide desktop app.
 * Shows status dot with optional text and pulse animation for connecting state.
 */

import React, { useEffect, useRef } from 'react';
import { View, Text, StyleSheet, ViewStyle, Animated, Easing } from 'react-native';
import { useTheme } from '@/theme';
import type { ConnectionStatus as ConnectionStatusType } from '@/types';

/**
 * ConnectionStatus display variants
 */
export type ConnectionStatusVariant = 'dot' | 'badge' | 'full';

/**
 * ConnectionStatus props
 */
export interface ConnectionStatusProps {
  /** Current connection status */
  status: ConnectionStatusType;
  /** Display variant */
  variant?: ConnectionStatusVariant;
  /** Whether to show pulsing animation for connecting state */
  animated?: boolean;
  /** Custom container style */
  style?: ViewStyle;
  /** Server name or URL to display (for 'full' variant) */
  serverName?: string;
}

/**
 * Status configuration mapping
 */
interface StatusConfig {
  color: string;
  label: string;
  accessibilityLabel: string;
}

/**
 * Get status configuration
 */
function getStatusConfig(
  status: ConnectionStatusType,
  colors: ReturnType<typeof useTheme>['colors'],
): StatusConfig {
  switch (status) {
    case 'connected':
      return {
        color: colors.success,
        label: 'Connected',
        accessibilityLabel: 'Connected to server',
      };
    case 'authenticated':
      return {
        color: colors.success,
        label: 'Authenticated',
        accessibilityLabel: 'Authenticated with server',
      };
    case 'connecting':
      return {
        color: colors.warning,
        label: 'Connecting...',
        accessibilityLabel: 'Connecting to server',
      };
    case 'disconnected':
    default:
      return {
        color: colors.textMuted,
        label: 'Disconnected',
        accessibilityLabel: 'Disconnected from server',
      };
  }
}

/**
 * ConnectionStatus Component
 *
 * Shows the connection status with a colored indicator dot and optional text.
 *
 * @example
 * ```tsx
 * <ConnectionStatus
 *   status="connected"
 *   variant="badge"
 *   animated
 * />
 * ```
 */
export function ConnectionStatus({
  status,
  variant = 'badge',
  animated = true,
  style,
  serverName,
}: ConnectionStatusProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius: themeRadius } = useTheme();
  const { textStyles } = typography;

  // Animation for pulse effect
  const pulseAnim = useRef(new Animated.Value(1)).current;

  // Pulse animation for connecting state
  useEffect(() => {
    if (status === 'connecting' && animated) {
      const pulse = Animated.loop(
        Animated.sequence([
          Animated.timing(pulseAnim, {
            toValue: 0.4,
            duration: 800,
            easing: Easing.inOut(Easing.ease),
            useNativeDriver: true,
          }),
          Animated.timing(pulseAnim, {
            toValue: 1,
            duration: 800,
            easing: Easing.inOut(Easing.ease),
            useNativeDriver: true,
          }),
        ]),
      );
      pulse.start();
      return () => pulse.stop();
    }
    // Reset opacity when not connecting
    pulseAnim.setValue(1);
    return undefined;
  }, [status, animated, pulseAnim]);

  const statusConfig = getStatusConfig(status, colors);

  // Status dot styles
  const dotStyle: ViewStyle = {
    width: variant === 'dot' ? 12 : 8,
    height: variant === 'dot' ? 12 : 8,
    borderRadius: variant === 'dot' ? 6 : 4,
    backgroundColor: statusConfig.color,
  };

  // Render based on variant
  const renderContent = () => {
    switch (variant) {
      case 'dot':
        return (
          <Animated.View
            style={[dotStyle, { opacity: pulseAnim }]}
            accessible
            accessibilityRole="image"
            accessibilityLabel={statusConfig.accessibilityLabel}
          />
        );

      case 'badge':
        return (
          <View
            style={[
              styles.badge,
              {
                backgroundColor: colors.surface,
                borderRadius: themeRadius['2xl'],
                paddingHorizontal: spacing.md,
                paddingVertical: spacing.xs,
              },
            ]}
            accessible
            accessibilityRole="text"
            accessibilityLabel={statusConfig.accessibilityLabel}
          >
            <Animated.View style={[styles.dot, dotStyle, { opacity: pulseAnim }]} />
            <Text
              style={[
                styles.badgeText,
                textStyles.statusBadge,
                { color: colors.textSecondary },
              ]}
            >
              {statusConfig.label}
            </Text>
          </View>
        );

      case 'full':
        return (
          <View
            style={[
              styles.full,
              {
                backgroundColor: colors.surface,
                borderRadius: themeRadius.md,
                padding: spacing.md,
              },
            ]}
            accessible
            accessibilityRole="text"
            accessibilityLabel={`${statusConfig.accessibilityLabel}${serverName ? `. Connected to ${serverName}` : ''}`}
          >
            <View style={styles.fullHeader}>
              <Animated.View style={[styles.dot, dotStyle, { opacity: pulseAnim }]} />
              <Text
                style={[
                  styles.statusLabel,
                  textStyles.listTitle,
                  { color: colors.textPrimary },
                ]}
              >
                {statusConfig.label}
              </Text>
            </View>
            {serverName && (
              <Text
                style={[
                  styles.serverName,
                  textStyles.listSubtitle,
                  { color: colors.textMuted },
                ]}
                numberOfLines={1}
              >
                {serverName}
              </Text>
            )}
          </View>
        );
      default:
        return null;
    }
  };

  return <View style={[styles.container, style]}>{renderContent()}</View>;
}

const styles = StyleSheet.create({
  container: {
    alignSelf: 'flex-start',
  },
  badge: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  dot: {
    marginRight: 6,
  },
  badgeText: {
    // Typography from theme
  },
  full: {
    alignSelf: 'stretch',
  },
  fullHeader: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  statusLabel: {
    // Typography from theme
  },
  serverName: {
    marginTop: 4,
    marginLeft: 14, // Align with text after dot
  },
});
