/**
 * QRScannerModal Component
 *
 * Modal dialog for scanning QR codes to connect to RustRide servers.
 * Uses react-native-camera-kit for camera access and QR code scanning.
 *
 * Features:
 * - Camera permission handling
 * - QR code detection and parsing
 * - Visual feedback on scan
 * - Auto-connect after successful scan
 */

import React, { useState, useCallback, useEffect, useRef } from 'react';
import {
  View,
  Text,
  Modal,
  StyleSheet,
  Pressable,
  Platform,
  Linking,
} from 'react-native';
import { Camera, type OnReadCodeData } from 'react-native-camera-kit';
import { useTheme } from '@/theme';
import { Button } from './Button';
import { LoadingSpinner } from './LoadingSpinner';
import type { QrConnectionData } from '@/types';
import { parseQrConnectionData, parseWebSocketUrl } from '@/types';
import Icon from 'react-native-vector-icons/Ionicons';

/**
 * QRScannerModal props
 */
export interface QRScannerModalProps {
  /** Whether the modal is visible */
  visible: boolean;
  /** Called when the modal should close */
  onClose: () => void;
  /** Called when a valid QR code is scanned */
  onScan: (data: QrConnectionData) => void;
  /** Whether connection is in progress */
  isConnecting?: boolean;
}

/**
 * Permission status for camera access
 */
type PermissionStatus = 'checking' | 'granted' | 'denied' | 'unavailable';

/**
 * QRScannerModal Component
 *
 * Provides QR code scanning for connecting to RustRide servers.
 * Handles camera permissions and parses the QR code data.
 *
 * @example
 * ```tsx
 * <QRScannerModal
 *   visible={showScanner}
 *   onClose={() => setShowScanner(false)}
 *   onScan={(data) => handleConnection(data)}
 * />
 * ```
 */
export function QRScannerModal({
  visible,
  onClose,
  onScan,
  isConnecting = false,
}: QRScannerModalProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  const [permissionStatus, setPermissionStatus] = useState<PermissionStatus>('checking');
  const [hasScanned, setHasScanned] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const lastScanTime = useRef<number>(0);

  /**
   * Check camera permission status and request if needed
   */
  const checkCameraPermission = useCallback(async () => {
    setPermissionStatus('checking');

    try {
      // Check current permission status
      const status = await Camera.checkDeviceCameraAuthorizationStatus();

      if (status === true) {
        setPermissionStatus('granted');
        return;
      }

      if (status === -1) {
        // Permission not determined, request it
        const granted = await Camera.requestDeviceCameraAuthorization();
        setPermissionStatus(granted ? 'granted' : 'denied');
        return;
      }

      // Permission denied
      setPermissionStatus('denied');
    } catch {
      setPermissionStatus('unavailable');
    }
  }, []);

  // Check/request camera permission when modal opens
  useEffect(() => {
    if (visible) {
      checkCameraPermission();
      setHasScanned(false);
      setScanError(null);
    }
  }, [visible, checkCameraPermission]);

  /**
   * Open device settings to grant camera permission
   */
  const openSettings = useCallback(() => {
    if (Platform.OS === 'ios') {
      Linking.openURL('app-settings:');
    } else {
      Linking.openSettings();
    }
    onClose();
  }, [onClose]);

  /**
   * Handle QR code scan result
   */
  const handleCodeRead = useCallback(
    (event: OnReadCodeData) => {
      // Debounce scans to prevent multiple triggers
      const now = Date.now();
      if (now - lastScanTime.current < 2000) {
        return;
      }
      lastScanTime.current = now;

      // Already scanned or connecting
      if (hasScanned || isConnecting) {
        return;
      }

      const codeValue = event.nativeEvent.codeStringValue;

      // Parse the QR code data
      const connectionData = parseQrConnectionData(codeValue);

      if (!connectionData) {
        setScanError('Invalid QR code. Please scan a RustRide connection QR code.');
        // Reset error after 3 seconds to allow retry
        setTimeout(() => setScanError(null), 3000);
        return;
      }

      // Validate the URL can be parsed
      const urlParts = parseWebSocketUrl(connectionData.url);
      if (!urlParts) {
        setScanError('Invalid connection URL in QR code.');
        setTimeout(() => setScanError(null), 3000);
        return;
      }

      // Mark as scanned and trigger callback
      setHasScanned(true);
      setScanError(null);
      onScan(connectionData);
    },
    [hasScanned, isConnecting, onScan]
  );

  /**
   * Render permission denied state
   */
  const renderPermissionDenied = () => (
    <View style={styles.centerContent}>
      <View
        style={[
          styles.iconContainer,
          {
            backgroundColor: colors.surface,
            borderRadius: borderRadius.full,
          },
        ]}
      >
        <Icon name="camera-off-outline" size={48} color={colors.error} />
      </View>
      <Text
        style={[
          styles.title,
          textStyles.sectionTitle,
          { color: colors.textPrimary, marginTop: spacing.lg },
        ]}
      >
        Camera Access Required
      </Text>
      <Text
        style={[
          styles.description,
          textStyles.body,
          { color: colors.textSecondary, marginTop: spacing.sm },
        ]}
      >
        Camera access is required to scan QR codes. Please enable camera access in your device settings.
      </Text>
      <Button
        title="Open Settings"
        variant="primary"
        onPress={openSettings}
        style={{ marginTop: spacing.lg }}
      />
      <Button
        title="Cancel"
        variant="ghost"
        onPress={onClose}
        style={{ marginTop: spacing.sm }}
      />
    </View>
  );

  /**
   * Render camera unavailable state
   */
  const renderCameraUnavailable = () => (
    <View style={styles.centerContent}>
      <View
        style={[
          styles.iconContainer,
          {
            backgroundColor: colors.surface,
            borderRadius: borderRadius.full,
          },
        ]}
      >
        <Icon name="warning-outline" size={48} color={colors.warning} />
      </View>
      <Text
        style={[
          styles.title,
          textStyles.sectionTitle,
          { color: colors.textPrimary, marginTop: spacing.lg },
        ]}
      >
        Camera Unavailable
      </Text>
      <Text
        style={[
          styles.description,
          textStyles.body,
          { color: colors.textSecondary, marginTop: spacing.sm },
        ]}
      >
        The camera is not available on this device. Please use manual connection instead.
      </Text>
      <Button
        title="Close"
        variant="primary"
        onPress={onClose}
        style={{ marginTop: spacing.lg }}
      />
    </View>
  );

  /**
   * Render the camera scanner
   */
  const renderScanner = () => (
    <View style={styles.scannerContainer}>
      {/* Camera view */}
      <Camera
        style={styles.camera}
        scanBarcode={!hasScanned && !isConnecting}
        onReadCode={handleCodeRead}
        showFrame={true}
        frameColor={colors.accent}
        laserColor={colors.accent}
        cameraType="back"
        flashMode="auto"
        accessibilityLabel="QR code scanner camera"
        testID="qr-scanner-camera"
      />

      {/* Overlay */}
      <View style={styles.overlay}>
        {/* Top section with instructions */}
        <View style={[styles.overlaySection, { padding: spacing.lg }]}>
          <Text style={[styles.instructions, textStyles.sectionTitle, { color: '#FFFFFF' }]}>
            Scan QR Code
          </Text>
          <Text style={[styles.instructionsSub, textStyles.body, { color: 'rgba(255,255,255,0.8)' }]}>
            Point your camera at the QR code shown in the RustRide desktop app
          </Text>
        </View>

        {/* Scanner frame area - transparent */}
        <View style={styles.scanAreaContainer}>
          <View style={styles.scanArea}>
            {/* Corner markers */}
            <View style={[styles.corner, styles.cornerTopLeft, { borderColor: colors.accent }]} />
            <View style={[styles.corner, styles.cornerTopRight, { borderColor: colors.accent }]} />
            <View style={[styles.corner, styles.cornerBottomLeft, { borderColor: colors.accent }]} />
            <View style={[styles.corner, styles.cornerBottomRight, { borderColor: colors.accent }]} />
          </View>
        </View>

        {/* Bottom section with status/error */}
        <View style={[styles.overlaySection, { padding: spacing.lg }]}>
          {scanError ? (
            <View style={[styles.errorBanner, { backgroundColor: colors.error, borderRadius: borderRadius.sm }]}>
              <Icon name="alert-circle" size={20} color="#FFFFFF" />
              <Text style={[styles.errorText, textStyles.body, { color: '#FFFFFF', marginLeft: spacing.sm }]}>
                {scanError}
              </Text>
            </View>
          ) : hasScanned || isConnecting ? (
            <View style={styles.connectingContainer}>
              <LoadingSpinner size="small" centered={false} />
              <Text style={[styles.connectingText, textStyles.body, { color: '#FFFFFF', marginLeft: spacing.sm }]}>
                Connecting...
              </Text>
            </View>
          ) : null}

          <Button
            title="Cancel"
            variant="ghost"
            onPress={onClose}
            style={{ marginTop: spacing.md }}
            textStyle={{ color: '#FFFFFF' }}
            disabled={isConnecting}
          />
        </View>
      </View>
    </View>
  );

  return (
    <Modal
      visible={visible}
      transparent={false}
      animationType="slide"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <View style={[styles.container, { backgroundColor: colors.background }]}>
        {/* Header */}
        <View style={[styles.header, { paddingHorizontal: spacing.md, paddingTop: spacing.lg }]}>
          <Pressable
            onPress={onClose}
            style={styles.closeButton}
            accessibilityLabel="Close scanner"
            accessibilityRole="button"
            disabled={isConnecting}
          >
            <Icon name="close" size={28} color={colors.textPrimary} />
          </Pressable>
        </View>

        {/* Content */}
        <View style={styles.content}>
          {permissionStatus === 'checking' && (
            <View style={styles.centerContent}>
              <LoadingSpinner size="large" message="Checking camera permission..." />
            </View>
          )}

          {permissionStatus === 'denied' && renderPermissionDenied()}

          {permissionStatus === 'unavailable' && renderCameraUnavailable()}

          {permissionStatus === 'granted' && renderScanner()}
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    alignItems: 'center',
    zIndex: 10,
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    paddingTop: Platform.OS === 'ios' ? 50 : 16,
  },
  closeButton: {
    padding: 8,
  },
  content: {
    flex: 1,
  },
  centerContent: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 32,
  },
  iconContainer: {
    width: 96,
    height: 96,
    justifyContent: 'center',
    alignItems: 'center',
  },
  title: {
    textAlign: 'center',
  },
  description: {
    textAlign: 'center',
    maxWidth: 280,
  },
  scannerContainer: {
    flex: 1,
    position: 'relative',
  },
  camera: {
    flex: 1,
  },
  overlay: {
    ...StyleSheet.absoluteFillObject,
    justifyContent: 'space-between',
  },
  overlaySection: {
    backgroundColor: 'rgba(0, 0, 0, 0.6)',
    alignItems: 'center',
  },
  instructions: {
    textAlign: 'center',
    fontWeight: '600',
  },
  instructionsSub: {
    textAlign: 'center',
    marginTop: 8,
  },
  scanAreaContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  scanArea: {
    width: 250,
    height: 250,
    position: 'relative',
  },
  corner: {
    position: 'absolute',
    width: 30,
    height: 30,
    borderWidth: 4,
  },
  cornerTopLeft: {
    top: 0,
    left: 0,
    borderRightWidth: 0,
    borderBottomWidth: 0,
  },
  cornerTopRight: {
    top: 0,
    right: 0,
    borderLeftWidth: 0,
    borderBottomWidth: 0,
  },
  cornerBottomLeft: {
    bottom: 0,
    left: 0,
    borderRightWidth: 0,
    borderTopWidth: 0,
  },
  cornerBottomRight: {
    bottom: 0,
    right: 0,
    borderLeftWidth: 0,
    borderTopWidth: 0,
  },
  errorBanner: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    width: '100%',
  },
  errorText: {
    flex: 1,
  },
  connectingContainer: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  connectingText: {
    marginLeft: 8,
  },
});
