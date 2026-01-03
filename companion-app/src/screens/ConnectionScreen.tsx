/**
 * Connection Screen
 *
 * Shows discovered servers and allows connection to RustRide desktop app.
 * Features:
 * - Automatic mDNS server discovery
 * - Pull to refresh
 * - Manual IP:port entry
 * - Connection status indicator
 * - PIN authentication flow
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
import { PinEntryModal } from '@/components/PinEntryModal';
import { EmptyState } from '@/components/EmptyState';
import {
  useConnectionStore,
  selectDiscoveredServers,
  selectIsScanning,
  selectConnectionStatus,
  selectConnectionError,
} from '@/stores/connectionStore';
import { getDiscoveryService } from '@/services/DiscoveryService';
import { getConnectionService } from '@/services/ConnectionService';
import { useAuthentication } from '@/hooks/useAuthentication';
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

  // Authentication hook
  const { state: authState, actions: authActions } = useAuthentication();

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

      // Set the current server in the store for persistence
      useConnectionStore.getState().setCurrentServer(server);

      const serverUrl = discoveryService.buildServerUrl(server);

      try {
        await connectionService.connect(serverUrl);
        // Connection initiated, auth may be required
        // Navigation happens in the useEffect when status changes
      } catch {
        setConnectingServer(null);
        useConnectionStore.getState().setCurrentServer(null);
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

      setConnectingServer(server);
      setIsQRScannerVisible(false);

      // Set the current server in the store for persistence
      useConnectionStore.getState().setCurrentServer(server);

      try {
        // Connect using the URL from QR code
        await connectionService.connect(qrData.url);

        // If we have a PIN from QR code, authenticate automatically
        if (qrData.pin) {
          // Store PIN for potential reconnection
          useConnectionStore.getState().savePin(qrData.pin);
          // Authenticate with the PIN
          await authActions.submitPin(qrData.pin);
        }
        // If no PIN provided, wait for server to request auth (handled by useAuthentication hook)
      } catch {
        setConnectingServer(null);
        useConnectionStore.getState().setCurrentServer(null);
        Alert.alert(
          'Connection Failed',
          `Could not connect to ${server.name}. Please check the server is running and try again.`,
          [{ text: 'OK' }]
        );
      }
    },
    [connectionService, authActions]
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
        <EmptyState
          variant="custom"
          icon="help-circle-outline"
          title="No Servers Found"
          description="Make sure RustRide is running on your computer and the companion server is enabled."
          actionLabel="Retry Scan"
          onAction={handleRefresh}
          testID="connection-empty-state"
        />
      </View>
    );
  }, [isScanning, handleRefresh]);

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

      {/* PIN entry modal */}
      <PinEntryModal
        visible={authState.showPinModal}
        onClose={authActions.closePinModal}
        onSubmit={authActions.submitPin}
        isAuthenticating={authState.isAuthenticating}
        error={authState.authError}
        serverName={connectingServer?.name ?? authState.serverName ?? undefined}
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
