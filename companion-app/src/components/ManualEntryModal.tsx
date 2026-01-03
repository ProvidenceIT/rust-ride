/**
 * ManualEntryModal Component
 *
 * Modal dialog for manually entering a server IP address and port.
 * Used when mDNS discovery doesn't find the server.
 */

import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  TextInput,
  Modal,
  StyleSheet,
  Pressable,
  KeyboardAvoidingView,
  Platform,
} from 'react-native';
import { useTheme } from '@/theme';
import { Button } from './Button';
import type { DiscoveredServer } from '@/types';

/**
 * ManualEntryModal props
 */
export interface ManualEntryModalProps {
  /** Whether the modal is visible */
  visible: boolean;
  /** Called when the modal should close */
  onClose: () => void;
  /** Called when the user submits a server address */
  onSubmit: (server: DiscoveredServer) => void;
  /** Whether connection is in progress */
  isConnecting?: boolean;
}

/**
 * Default RustRide companion server port
 */
const DEFAULT_PORT = 9876;

/**
 * Validate an IP address format
 */
function isValidIpAddress(ip: string): boolean {
  // Allow hostnames and IPv4 addresses
  const ipv4Pattern = /^(\d{1,3}\.){3}\d{1,3}$/;
  const hostnamePattern = /^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$/;

  if (ipv4Pattern.test(ip)) {
    // Validate each octet for IPv4
    const octets = ip.split('.').map(Number);
    return octets.every(octet => octet >= 0 && octet <= 255);
  }

  return hostnamePattern.test(ip);
}

/**
 * Validate a port number
 */
function isValidPort(port: string): boolean {
  const portNum = parseInt(port, 10);
  return !isNaN(portNum) && portNum > 0 && portNum <= 65535;
}

/**
 * ManualEntryModal Component
 *
 * Provides a form for manually entering server connection details.
 *
 * @example
 * ```tsx
 * <ManualEntryModal
 *   visible={showModal}
 *   onClose={() => setShowModal(false)}
 *   onSubmit={(server) => handleConnect(server)}
 * />
 * ```
 */
export function ManualEntryModal({
  visible,
  onClose,
  onSubmit,
  isConnecting = false,
}: ManualEntryModalProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  const [ipAddress, setIpAddress] = useState('');
  const [port, setPort] = useState(String(DEFAULT_PORT));
  const [ipError, setIpError] = useState<string | null>(null);
  const [portError, setPortError] = useState<string | null>(null);

  // Reset form when modal opens
  React.useEffect(() => {
    if (visible) {
      setIpAddress('');
      setPort(String(DEFAULT_PORT));
      setIpError(null);
      setPortError(null);
    }
  }, [visible]);

  const validateAndSubmit = useCallback(() => {
    let hasError = false;

    // Validate IP
    if (!ipAddress.trim()) {
      setIpError('IP address is required');
      hasError = true;
    } else if (!isValidIpAddress(ipAddress.trim())) {
      setIpError('Invalid IP address or hostname');
      hasError = true;
    } else {
      setIpError(null);
    }

    // Validate port
    if (!port.trim()) {
      setPortError('Port is required');
      hasError = true;
    } else if (!isValidPort(port.trim())) {
      setPortError('Invalid port (1-65535)');
      hasError = true;
    } else {
      setPortError(null);
    }

    if (hasError) {
      return;
    }

    const server: DiscoveredServer = {
      name: `Manual (${ipAddress.trim()})`,
      host: ipAddress.trim(),
      port: parseInt(port.trim(), 10),
    };

    onSubmit(server);
  }, [ipAddress, port, onSubmit]);

  const handleIpChange = useCallback((text: string) => {
    setIpAddress(text);
    if (ipError) {
      setIpError(null);
    }
  }, [ipError]);

  const handlePortChange = useCallback((text: string) => {
    // Only allow numeric input
    const numericText = text.replace(/[^0-9]/g, '');
    setPort(numericText);
    if (portError) {
      setPortError(null);
    }
  }, [portError]);

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <KeyboardAvoidingView
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
        style={styles.overlay}
      >
        <Pressable style={styles.backdrop} onPress={onClose}>
          <View />
        </Pressable>

        <View
          style={[
            styles.modal,
            {
              backgroundColor: colors.background,
              borderRadius: borderRadius.lg,
              padding: spacing.lg,
            },
          ]}
        >
          {/* Header */}
          <Text style={[styles.title, textStyles.sectionTitle, { color: colors.textPrimary }]}>
            Manual Connection
          </Text>
          <Text
            style={[styles.subtitle, textStyles.body, { color: colors.textSecondary, marginTop: spacing.xs }]}
          >
            Enter the IP address and port of your RustRide server
          </Text>

          {/* IP Address input */}
          <View style={[styles.inputGroup, { marginTop: spacing.lg }]}>
            <Text style={[styles.label, textStyles.label, { color: colors.textSecondary }]}>
              IP Address or Hostname
            </Text>
            <TextInput
              style={[
                styles.input,
                textStyles.inputPlaceholder,
                {
                  backgroundColor: colors.surface,
                  borderColor: ipError ? colors.error : colors.border,
                  borderRadius: borderRadius.sm,
                  color: colors.textPrimary,
                  paddingHorizontal: spacing.md,
                  paddingVertical: spacing.sm,
                },
              ]}
              value={ipAddress}
              onChangeText={handleIpChange}
              placeholder="192.168.1.100"
              placeholderTextColor={colors.textMuted}
              keyboardType="default"
              autoCapitalize="none"
              autoCorrect={false}
              autoComplete="off"
              editable={!isConnecting}
              accessibilityLabel="IP address"
              accessibilityHint="Enter the IP address of the RustRide server"
            />
            {ipError && (
              <Text style={[styles.errorText, textStyles.caption, { color: colors.error }]}>
                {ipError}
              </Text>
            )}
          </View>

          {/* Port input */}
          <View style={[styles.inputGroup, { marginTop: spacing.md }]}>
            <Text style={[styles.label, textStyles.label, { color: colors.textSecondary }]}>
              Port
            </Text>
            <TextInput
              style={[
                styles.input,
                textStyles.inputPlaceholder,
                {
                  backgroundColor: colors.surface,
                  borderColor: portError ? colors.error : colors.border,
                  borderRadius: borderRadius.sm,
                  color: colors.textPrimary,
                  paddingHorizontal: spacing.md,
                  paddingVertical: spacing.sm,
                },
              ]}
              value={port}
              onChangeText={handlePortChange}
              placeholder={String(DEFAULT_PORT)}
              placeholderTextColor={colors.textMuted}
              keyboardType="number-pad"
              maxLength={5}
              editable={!isConnecting}
              accessibilityLabel="Port number"
              accessibilityHint="Enter the port number, default is 9876"
            />
            {portError && (
              <Text style={[styles.errorText, textStyles.caption, { color: colors.error }]}>
                {portError}
              </Text>
            )}
          </View>

          {/* Buttons */}
          <View style={[styles.buttons, { marginTop: spacing.lg }]}>
            <Button
              title="Cancel"
              variant="ghost"
              onPress={onClose}
              disabled={isConnecting}
              style={styles.cancelButton}
            />
            <Button
              title="Connect"
              variant="primary"
              onPress={validateAndSubmit}
              loading={isConnecting}
              style={styles.connectButton}
            />
          </View>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  backdrop: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
  },
  modal: {
    width: '90%',
    maxWidth: 400,
  },
  title: {
    textAlign: 'center',
  },
  subtitle: {
    textAlign: 'center',
  },
  inputGroup: {
    // Spacing applied inline
  },
  label: {
    marginBottom: 6,
  },
  input: {
    borderWidth: 1,
  },
  errorText: {
    marginTop: 4,
  },
  buttons: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    gap: 12,
  },
  cancelButton: {
    minWidth: 80,
  },
  connectButton: {
    minWidth: 100,
  },
});
