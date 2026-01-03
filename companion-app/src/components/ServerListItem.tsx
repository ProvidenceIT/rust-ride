/**
 * ServerListItem Component
 *
 * Displays a discovered RustRide server with name, IP, port, and version.
 * Used in the connection screen server list.
 */

import React from 'react';
import { View, Text, StyleSheet, Pressable, ViewStyle } from 'react-native';
import { useTheme } from '@/theme';
import type { DiscoveredServer } from '@/types';

/**
 * ServerListItem props
 */
export interface ServerListItemProps {
  /** Server information */
  server: DiscoveredServer;
  /** Whether this server is currently selected/connecting */
  isConnecting?: boolean;
  /** Called when the server is pressed */
  onPress: (server: DiscoveredServer) => void;
  /** Custom container style */
  style?: ViewStyle;
}

/**
 * ServerListItem Component
 *
 * Displays a discovered server with its details in a pressable card.
 *
 * @example
 * ```tsx
 * <ServerListItem
 *   server={{ name: 'RustRide', host: '192.168.1.100', port: 9876, version: '1.0' }}
 *   onPress={(server) => handleConnect(server)}
 * />
 * ```
 */
export function ServerListItem({
  server,
  isConnecting = false,
  onPress,
  style,
}: ServerListItemProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  const handlePress = () => {
    if (!isConnecting) {
      onPress(server);
    }
  };

  // Format the connection address
  const connectionAddress = `${server.host}:${server.port}`;

  return (
    <Pressable
      style={({ pressed }) => [
        styles.container,
        {
          backgroundColor: pressed && !isConnecting ? colors.surface : colors.elevated,
          borderRadius: borderRadius.md,
          padding: spacing.md,
          opacity: isConnecting ? 0.6 : 1,
        },
        style,
      ]}
      onPress={handlePress}
      disabled={isConnecting}
      accessibilityRole="button"
      accessibilityLabel={`Connect to ${server.name} at ${connectionAddress}`}
      accessibilityState={{ disabled: isConnecting, busy: isConnecting }}
    >
      <View style={styles.content}>
        <View style={styles.mainInfo}>
          {/* Server icon indicator */}
          <View
            style={[
              styles.iconContainer,
              {
                backgroundColor: colors.accent,
                borderRadius: borderRadius.full,
                width: spacing.xl + spacing.sm,
                height: spacing.xl + spacing.sm,
              },
            ]}
          >
            <Text style={[styles.iconText, { color: colors.textInverse }]}>R</Text>
          </View>

          {/* Server details */}
          <View style={styles.textContainer}>
            <Text
              style={[styles.name, textStyles.listTitle, { color: colors.textPrimary }]}
              numberOfLines={1}
            >
              {server.name}
            </Text>
            <Text
              style={[styles.address, textStyles.listSubtitle, { color: colors.textSecondary }]}
              numberOfLines={1}
            >
              {connectionAddress}
            </Text>
          </View>
        </View>

        {/* Version badge (if available) */}
        {server.version && (
          <View
            style={[
              styles.versionBadge,
              {
                backgroundColor: colors.surface,
                borderRadius: borderRadius.sm,
                paddingHorizontal: spacing.sm,
                paddingVertical: spacing.xs,
              },
            ]}
          >
            <Text style={[styles.versionText, textStyles.caption, { color: colors.textMuted }]}>
              v{server.version}
            </Text>
          </View>
        )}

        {/* Connection indicator */}
        {isConnecting && (
          <View
            style={[
              styles.connectingBadge,
              {
                backgroundColor: colors.accent,
                borderRadius: borderRadius.full,
                paddingHorizontal: spacing.md,
                paddingVertical: spacing.xs,
              },
            ]}
          >
            <Text style={[styles.connectingText, textStyles.buttonSmall, { color: colors.textInverse }]}>
              Connecting...
            </Text>
          </View>
        )}
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  container: {
    // Dynamic styles applied inline
  },
  content: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  mainInfo: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
  },
  iconContainer: {
    alignItems: 'center',
    justifyContent: 'center',
  },
  iconText: {
    fontSize: 18,
    fontWeight: 'bold',
  },
  textContainer: {
    flex: 1,
    marginLeft: 12,
  },
  name: {
    // Typography from theme
  },
  address: {
    marginTop: 2,
  },
  versionBadge: {
    // Dynamic styles applied inline
  },
  versionText: {
    // Typography from theme
  },
  connectingBadge: {
    marginLeft: 8,
  },
  connectingText: {
    // Typography from theme
  },
});
