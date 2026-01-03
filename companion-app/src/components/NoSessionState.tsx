/**
 * NoSessionState Component
 *
 * Displays appropriate UI when no workout or ride is active.
 * Shows connection status and provides helpful hints to guide the user.
 *
 * States:
 * 1. Not connected: Prompts user to connect to RustRide desktop
 * 2. Connected but no session: Hints to start a session on desktop
 */

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  ViewStyle,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { ConnectionStatus } from './ConnectionStatus';
import { Button } from './Button';
import type { ConnectionStatus as ConnectionStatusType } from '@/types';

/**
 * NoSessionState props
 */
export interface NoSessionStateProps {
  /** Current connection status */
  connectionStatus: ConnectionStatusType;
  /** Server name to display (optional) */
  serverName?: string;
  /** Callback when connect button is pressed (not connected state) */
  onConnectPress?: () => void;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * NoSessionState Component
 *
 * Shows a helpful message and action when there's no active workout or ride.
 * The content changes based on connection status:
 * - Not connected: Shows connection prompt with button to connect
 * - Connected: Shows hint to start session on desktop app
 *
 * @example
 * ```tsx
 * <NoSessionState
 *   connectionStatus="disconnected"
 *   onConnectPress={() => navigation.navigate('Connection')}
 * />
 * ```
 */
export function NoSessionState({
  connectionStatus,
  serverName,
  onConnectPress,
  style,
}: NoSessionStateProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();

  const isConnected = connectionStatus === 'connected' || connectionStatus === 'authenticated';
  const isConnecting = connectionStatus === 'connecting';

  // Determine icon, title, and description based on connection state
  const getContent = () => {
    if (isConnecting) {
      return {
        icon: 'sync-outline' as const,
        iconColor: colors.warning,
        title: 'Connecting...',
        description: 'Establishing connection to RustRide desktop app.',
        showButton: false,
      };
    }

    if (!isConnected) {
      return {
        icon: 'desktop-outline' as const,
        iconColor: colors.textMuted,
        title: 'Not Connected',
        description: 'Connect to your RustRide desktop app to control workouts and view live metrics.',
        showButton: true,
      };
    }

    // Connected but no session active
    return {
      icon: 'bicycle-outline' as const,
      iconColor: colors.accent,
      title: 'Ready to Ride',
      description: 'Start a workout or free ride on the desktop app to see live metrics here.',
      showButton: false,
    };
  };

  const content = getContent();

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
      accessibilityLabel={`${content.title}. ${content.description}`}
    >
      {/* Status icon */}
      <View
        style={[
          styles.iconContainer,
          {
            backgroundColor: colors.surface,
            borderRadius: borderRadius.full,
            marginBottom: spacing.lg,
          },
        ]}
      >
        <Icon
          name={content.icon}
          size={48}
          color={content.iconColor}
        />
      </View>

      {/* Title */}
      <Text
        style={[
          styles.title,
          typography.textStyles.sectionTitle,
          { color: colors.textPrimary, marginBottom: spacing.sm },
        ]}
      >
        {content.title}
      </Text>

      {/* Description */}
      <Text
        style={[
          styles.description,
          typography.textStyles.body,
          { color: colors.textSecondary, marginBottom: spacing.lg },
        ]}
      >
        {content.description}
      </Text>

      {/* Connection status badge (when connected) */}
      {isConnected && (
        <ConnectionStatus
          status={connectionStatus}
          variant="full"
          serverName={serverName}
          style={{ marginBottom: spacing.md }}
        />
      )}

      {/* Tips for connected state */}
      {isConnected && (
        <View
          style={[
            styles.tipsContainer,
            {
              backgroundColor: colors.surface,
              borderRadius: borderRadius.md,
              padding: spacing.md,
            },
          ]}
        >
          <View style={styles.tipRow}>
            <Icon
              name="information-circle-outline"
              size={20}
              color={colors.info}
              style={{ marginRight: spacing.sm }}
            />
            <Text
              style={[
                styles.tipText,
                typography.textStyles.bodySecondary,
                { color: colors.textSecondary, flex: 1 },
              ]}
            >
              Use your phone as a remote control during workouts
            </Text>
          </View>
          <View style={[styles.tipRow, { marginTop: spacing.sm }]}>
            <Icon
              name="pulse-outline"
              size={20}
              color={colors.info}
              style={{ marginRight: spacing.sm }}
            />
            <Text
              style={[
                styles.tipText,
                typography.textStyles.bodySecondary,
                { color: colors.textSecondary, flex: 1 },
              ]}
            >
              View real-time power, heart rate, and cadence
            </Text>
          </View>
        </View>
      )}

      {/* Connect button (when not connected) */}
      {content.showButton && onConnectPress && (
        <Button
          title="Connect to Desktop"
          variant="primary"
          size="large"
          fullWidth
          onPress={onConnectPress}
          leftIcon={
            <Icon
              name="link-outline"
              size={20}
              color={colors.textInverse}
            />
          }
          accessibilityHint="Opens the connection screen to connect to RustRide desktop"
        />
      )}

      {/* Connecting state indicator */}
      {isConnecting && (
        <ConnectionStatus
          status={connectionStatus}
          variant="badge"
          animated
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
  tipsContainer: {
    width: '100%',
  },
  tipRow: {
    flexDirection: 'row',
    alignItems: 'flex-start',
  },
  tipText: {
    lineHeight: 20,
  },
});
