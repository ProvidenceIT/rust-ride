/**
 * Connection Screen
 *
 * Shows discovered servers and allows connection to RustRide desktop app.
 * Includes QR code scanning and manual entry options.
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme, TouchableOpacity, ActivityIndicator } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { RootStackScreenProps } from '@/navigation/types';

const Colors = {
  light: {
    background: '#FFFFFF',
    surface: '#F5F5F5',
    primary: '#007AFF',
    text: '#1C1C1E',
    textSecondary: '#8E8E93',
    border: '#E5E5EA',
  },
  dark: {
    background: '#000000',
    surface: '#1C1C1E',
    primary: '#0A84FF',
    text: '#FFFFFF',
    textSecondary: '#8E8E93',
    border: '#38383A',
  },
};

type Props = RootStackScreenProps<'Connection'>;

export function ConnectionScreen({ navigation }: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;
  const [isScanning, setIsScanning] = React.useState(true);

  // Simulate scanning for servers
  React.useEffect(() => {
    const timer = setTimeout(() => {
      setIsScanning(false);
    }, 2000);
    return () => clearTimeout(timer);
  }, []);

  const handleScanQR = () => {
    // TODO: Open QR scanner
  };

  const handleManualEntry = () => {
    // TODO: Open manual entry dialog
  };

  const handleCancel = () => {
    navigation.goBack();
  };

  return (
    <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]}>
      <View style={styles.header}>
        <Text style={[styles.title, { color: colors.text }]}>Connect to RustRide</Text>
        <Text style={[styles.subtitle, { color: colors.textSecondary }]}>
          Searching for RustRide on your local network
        </Text>
      </View>

      <View style={styles.content}>
        {/* Server discovery section */}
        <View style={[styles.discoverySection, { backgroundColor: colors.surface }]}>
          <View style={styles.discoverySectionHeader}>
            <Text style={[styles.sectionTitle, { color: colors.text }]}>Available Servers</Text>
            {isScanning && <ActivityIndicator size="small" color={colors.primary} />}
          </View>

          {isScanning ? (
            <View style={styles.scanningState}>
              <Text style={[styles.scanningText, { color: colors.textSecondary }]}>
                Scanning...
              </Text>
            </View>
          ) : (
            <View style={styles.emptyState}>
              <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>
                No servers found on this network
              </Text>
              <TouchableOpacity
                style={[styles.retryButton, { borderColor: colors.border }]}
                onPress={() => setIsScanning(true)}
              >
                <Text style={[styles.retryButtonText, { color: colors.primary }]}>Retry</Text>
              </TouchableOpacity>
            </View>
          )}
        </View>

        {/* Alternative connection methods */}
        <View style={styles.alternativeMethods}>
          <TouchableOpacity
            style={[styles.methodButton, { backgroundColor: colors.primary }]}
            onPress={handleScanQR}
            activeOpacity={0.7}
          >
            <Text style={styles.methodButtonText}>Scan QR Code</Text>
          </TouchableOpacity>

          <TouchableOpacity
            style={[styles.methodButton, styles.methodButtonSecondary, { borderColor: colors.border }]}
            onPress={handleManualEntry}
            activeOpacity={0.7}
          >
            <Text style={[styles.methodButtonText, { color: colors.text }]}>Enter IP Manually</Text>
          </TouchableOpacity>
        </View>
      </View>

      <TouchableOpacity style={styles.cancelButton} onPress={handleCancel} activeOpacity={0.7}>
        <Text style={[styles.cancelButtonText, { color: colors.textSecondary }]}>Cancel</Text>
      </TouchableOpacity>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    padding: 24,
    alignItems: 'center',
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    textAlign: 'center',
  },
  content: {
    flex: 1,
    padding: 16,
  },
  discoverySection: {
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
  },
  discoverySectionHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 16,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
  },
  scanningState: {
    paddingVertical: 32,
    alignItems: 'center',
  },
  scanningText: {
    fontSize: 14,
  },
  emptyState: {
    paddingVertical: 24,
    alignItems: 'center',
  },
  emptyStateText: {
    fontSize: 14,
    textAlign: 'center',
    marginBottom: 16,
  },
  retryButton: {
    paddingVertical: 10,
    paddingHorizontal: 20,
    borderRadius: 20,
    borderWidth: 1,
  },
  retryButtonText: {
    fontSize: 14,
    fontWeight: '500',
  },
  alternativeMethods: {
    gap: 12,
  },
  methodButton: {
    paddingVertical: 16,
    borderRadius: 12,
    alignItems: 'center',
  },
  methodButtonSecondary: {
    backgroundColor: 'transparent',
    borderWidth: 1,
  },
  methodButtonText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#FFFFFF',
  },
  cancelButton: {
    padding: 20,
    alignItems: 'center',
  },
  cancelButtonText: {
    fontSize: 16,
  },
});
