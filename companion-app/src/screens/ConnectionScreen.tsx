/**
 * Connection Screen
 *
 * Shows discovered servers and allows connection to RustRide desktop app.
 * Features:
 * - Automatic mDNS server discovery
 * - Pull to refresh
 * - Manual IP:port entry
 * - Connection status indicator
 */

import React, { useCallback, useEffect, useState } from 'react';
import {
  StyleSheet,
  Text,
  View,
  FlatList,
  RefreshControl,
  Alert,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { RootStackScreenProps } from '@/navigation/types';
import { useTheme } from '@/theme';
import { Button } from '@/components/Button';
import { ConnectionStatus } from '@/components/ConnectionStatus';
import { LoadingSpinner } from '@/components/LoadingSpinner';
import { ServerListItem } from '@/components/ServerListItem';
import { ManualEntryModal } from '@/components/ManualEntryModal';
import { QRScannerModal } from '@/components/QRScannerModal';
import {
  useConnectionStore,
  selectDiscoveredServers,
  selectIsScanning,
  selectConnectionStatus,
  selectConnectionError,
} from '@/stores/connectionStore';
import { getDiscoveryService } from '@/services/DiscoveryService';
import { getConnectionService } from '@/services/ConnectionService';
import type { DiscoveredServer, QrConnectionData } from '@/types';
import { parseWebSocketUrl } from '@/types';

type Props = RootStackScreenProps<'Connection'>;

/**
 * ConnectionScreen Component
 *
 * Main screen for discovering and connecting to RustRide servers on the local network.
 */
export function ConnectionScreen({ navigation }: Props): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  // Store state
  const discoveredServers = useConnectionStore(selectDiscoveredServers);
  const isScanning = useConnectionStore(selectIsScanning);
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const connectionError = useConnectionStore(selectConnectionError);

  // Local state
  const [isManualEntryVisible, setIsManualEntryVisible] = useState(false);
  const [isQRScannerVisible, setIsQRScannerVisible] = useState(false);
  const [connectingServer, setConnectingServer] = useState<DiscoveredServer | null>(null);

  // Get services
  const discoveryService = getDiscoveryService();
  const connectionService = getConnectionService();

  // Start discovery on mount
  useEffect(() => {
    discoveryService.startScan().catch(() => {
      // Discovery may not be available on all devices
    });

    // Cleanup on unmount
    return () => {
      discoveryService.stopScan();
    };
  }, [discoveryService]);

  // Handle connection success/failure
  useEffect(() => {
    if (connectionStatus === 'authenticated' || connectionStatus === 'connected') {
      // Connection successful, navigate back
      setConnectingServer(null);
      navigation.goBack();
    } else if (connectionError && connectingServer) {
      // Connection failed
      setConnectingServer(null);
      Alert.alert(
        'Connection Failed',
        connectionError.message || 'Could not connect to the server. Please try again.',
        [{ text: 'OK' }]
      );
      useConnectionStore.getState().clearError();
    }
  }, [connectionStatus, connectionError, connectingServer, navigation]);

  // Handle pull to refresh
  const handleRefresh = useCallback(async () => {
    try {
      await discoveryService.refresh();
    } catch {
      // Refresh failed, user can try again
    }
  }, [discoveryService]);

  // Handle server selection
  const handleServerPress = useCallback(
    async (server: DiscoveredServer) => {
      setConnectingServer(server);

      const serverUrl = discoveryService.buildServerUrl(server);

      try {
        await connectionService.connect(serverUrl);
        // Connection initiated, auth may be required
        // Navigation happens in the useEffect when status changes
      } catch {
        setConnectingServer(null);
        Alert.alert(
          'Connection Failed',
          `Could not connect to ${server.name}. Please check the server is running and try again.`,
          [{ text: 'OK' }]
        );
      }
    },
    [connectionService, discoveryService]
  );

  // Handle manual entry
  const handleManualEntry = useCallback(() => {
    setIsManualEntryVisible(true);
  }, []);

  const handleManualEntryClose = useCallback(() => {
    setIsManualEntryVisible(false);
  }, []);

  // Handle QR scanner
  const handleQRScan = useCallback(() => {
    setIsQRScannerVisible(true);
  }, []);

  const handleQRScannerClose = useCallback(() => {
    setIsQRScannerVisible(false);
  }, []);

  const handleQRCodeScanned = useCallback(
    async (qrData: QrConnectionData) => {
      // Parse the WebSocket URL to get host and port
      const urlParts = parseWebSocketUrl(qrData.url);
      if (!urlParts) {
        Alert.alert('Invalid QR Code', 'The QR code contains an invalid connection URL.');
        setIsQRScannerVisible(false);
        return;
      }

      // Create a server object from QR data
      const server: DiscoveredServer = {
        name: `RustRide (${urlParts.host})`,
        host: urlParts.host,
        port: urlParts.port,
        version: qrData.version,
      };

      // Store the PIN if provided (will be used for authentication)
      if (qrData.pin) {
        // Store PIN in connection store for automatic authentication
        useConnectionStore.getState().savePin(qrData.pin);
      }

      setConnectingServer(server);
      setIsQRScannerVisible(false);

      try {
        // Connect using the URL from QR code
        await connectionService.connect(qrData.url);

        // If we have a PIN, authenticate automatically
        if (qrData.pin) {
          try {
            await connectionService.authenticate(qrData.pin);
          } catch (authError) {
            // Auth failed, but connection might still be open
            // User may need to enter PIN manually
          }
        }
      } catch {
        setConnectingServer(null);
        Alert.alert(
          'Connection Failed',
          `Could not connect to ${server.name}. Please check the server is running and try again.`,
          [{ text: 'OK' }]
        );
      }
    },
    [connectionService]
  );

  const handleManualEntrySubmit = useCallback(
    async (server: DiscoveredServer) => {
      // Add to discovered servers list
      discoveryService.addManualServer(server);
      setIsManualEntryVisible(false);

      // Connect to the server
      await handleServerPress(server);
    },
    [discoveryService, handleServerPress]
  );

  // Handle cancel
  const handleCancel = useCallback(() => {
    connectionService.disconnect();
    navigation.goBack();
  }, [connectionService, navigation]);

  // Render server list item
  const renderServerItem = useCallback(
    ({ item }: { item: DiscoveredServer }) => (
      <ServerListItem
        server={item}
        isConnecting={connectingServer?.host === item.host && connectingServer?.port === item.port}
        onPress={handleServerPress}
        style={{ marginBottom: spacing.sm }}
      />
    ),
    [connectingServer, handleServerPress, spacing.sm]
  );

  // Render empty state
  const renderEmptyState = useCallback(() => {
    if (isScanning) {
      return (
        <View style={styles.emptyState}>
          <LoadingSpinner size="medium" message="Searching for RustRide servers..." />
        </View>
      );
    }

    return (
      <View style={styles.emptyState}>
        <View
          style={[
            styles.emptyIconContainer,
            {
              backgroundColor: colors.surface,
              borderRadius: borderRadius.full,
              width: 80,
              height: 80,
            },
          ]}
        >
          <Text style={[styles.emptyIcon, { color: colors.textMuted }]}>?</Text>
        </View>
        <Text
          style={[
            styles.emptyTitle,
            textStyles.sectionTitle,
            { color: colors.textPrimary, marginTop: spacing.lg },
          ]}
        >
          No Servers Found
        </Text>
        <Text
          style={[
            styles.emptyDescription,
            textStyles.body,
            { color: colors.textSecondary, marginTop: spacing.sm },
          ]}
        >
          Make sure RustRide is running on your computer and the companion server is enabled.
        </Text>
        <Button
          title="Retry Scan"
          variant="outline"
          onPress={handleRefresh}
          style={{ marginTop: spacing.lg }}
        />
      </View>
    );
  }, [isScanning, colors, borderRadius, textStyles, spacing, handleRefresh]);

  // Key extractor for FlatList
  const keyExtractor = useCallback(
    (item: DiscoveredServer) => `${item.host}:${item.port}`,
    []
  );

  const isConnecting = connectionStatus === 'connecting';

  return (
    <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]}>
      {/* Header */}
      <View style={[styles.header, { padding: spacing.lg }]}>
        <Text style={[styles.title, textStyles.screenTitle, { color: colors.textPrimary }]}>
          Connect to RustRide
        </Text>
        <Text
          style={[
            styles.subtitle,
            textStyles.body,
            { color: colors.textSecondary, marginTop: spacing.xs },
          ]}
        >
          {isScanning
            ? 'Searching for RustRide on your local network...'
            : discoveredServers.length > 0
              ? 'Select a server to connect'
              : 'No servers found on your network'}
        </Text>

        {/* Connection status indicator */}
        {(isConnecting || connectionStatus === 'connected') && (
          <ConnectionStatus
            status={connectionStatus}
            variant="badge"
            style={{ marginTop: spacing.md }}
          />
        )}
      </View>

      {/* Server list */}
      <View style={[styles.content, { paddingHorizontal: spacing.md }]}>
        <View
          style={[
            styles.listContainer,
            {
              backgroundColor: colors.surface,
              borderRadius: borderRadius.lg,
              padding: spacing.md,
            },
          ]}
        >
          <View style={[styles.listHeader, { marginBottom: spacing.md }]}>
            <Text style={[styles.sectionTitle, textStyles.sectionTitle, { color: colors.textPrimary }]}>
              Available Servers
            </Text>
            {isScanning && (
              <LoadingSpinner size="small" centered={false} />
            )}
          </View>

          <FlatList
            data={discoveredServers}
            renderItem={renderServerItem}
            keyExtractor={keyExtractor}
            ListEmptyComponent={renderEmptyState}
            refreshControl={
              <RefreshControl
                refreshing={isScanning && discoveredServers.length > 0}
                onRefresh={handleRefresh}
                tintColor={colors.accent}
                colors={[colors.accent]}
              />
            }
            contentContainerStyle={
              discoveredServers.length === 0 ? styles.emptyListContent : undefined
            }
            showsVerticalScrollIndicator={false}
          />
        </View>
      </View>

      {/* Alternative connection methods */}
      <View style={[styles.footer, { padding: spacing.md }]}>
        <Button
          title="Scan QR Code"
          variant="primary"
          onPress={handleQRScan}
          fullWidth
          disabled={isConnecting}
          style={{ marginBottom: spacing.sm }}
        />

        <Button
          title="Enter IP Manually"
          variant="secondary"
          onPress={handleManualEntry}
          fullWidth
          disabled={isConnecting}
          style={{ marginBottom: spacing.md }}
        />

        <Button
          title="Cancel"
          variant="ghost"
          onPress={handleCancel}
          fullWidth
        />
      </View>

      {/* Manual entry modal */}
      <ManualEntryModal
        visible={isManualEntryVisible}
        onClose={handleManualEntryClose}
        onSubmit={handleManualEntrySubmit}
        isConnecting={isConnecting}
      />

      {/* QR scanner modal */}
      <QRScannerModal
        visible={isQRScannerVisible}
        onClose={handleQRScannerClose}
        onScan={handleQRCodeScanned}
        isConnecting={isConnecting}
      />
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    alignItems: 'center',
  },
  title: {
    textAlign: 'center',
  },
  subtitle: {
    textAlign: 'center',
  },
  content: {
    flex: 1,
  },
  listContainer: {
    flex: 1,
  },
  listHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  sectionTitle: {
    // Typography from theme
  },
  emptyListContent: {
    flexGrow: 1,
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 24,
  },
  emptyIconContainer: {
    justifyContent: 'center',
    alignItems: 'center',
  },
  emptyIcon: {
    fontSize: 40,
    fontWeight: 'bold',
  },
  emptyTitle: {
    textAlign: 'center',
  },
  emptyDescription: {
    textAlign: 'center',
  },
  footer: {
    // Spacing applied inline
  },
});
