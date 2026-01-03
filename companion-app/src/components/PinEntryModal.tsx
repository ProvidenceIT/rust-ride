/**
 * PinEntryModal Component
 *
 * Modal dialog for entering a 6-digit PIN when the server requires authentication.
 * Features:
 * - 6-digit numeric input with visual dots
 * - Custom numeric keypad
 * - Shake animation on wrong PIN
 * - Auto-submit when 6 digits are entered
 */

import React, { useState, useCallback, useEffect, useRef } from 'react';
import {
  View,
  Text,
  Modal,
  StyleSheet,
  Pressable,
  Animated,
  Vibration,
  Platform,
} from 'react-native';
import { useTheme } from '@/theme';
import { Button } from './Button';
import { LoadingSpinner } from './LoadingSpinner';
import Icon from 'react-native-vector-icons/Ionicons';

/**
 * PIN length constant
 */
const PIN_LENGTH = 6;

/**
 * Keypad layout
 */
const KEYPAD_ROWS = [
  ['1', '2', '3'],
  ['4', '5', '6'],
  ['7', '8', '9'],
  ['', '0', 'delete'],
];

/**
 * PinEntryModal props
 */
export interface PinEntryModalProps {
  /** Whether the modal is visible */
  visible: boolean;
  /** Called when the modal should close */
  onClose: () => void;
  /** Called when the user submits a PIN */
  onSubmit: (pin: string) => void;
  /** Whether authentication is in progress */
  isAuthenticating?: boolean;
  /** Error message to display (triggers shake animation) */
  error?: string | null;
  /** Server name for display */
  serverName?: string;
}

/**
 * PinEntryModal Component
 *
 * Provides a secure PIN entry interface for server authentication.
 *
 * @example
 * ```tsx
 * <PinEntryModal
 *   visible={showPinModal}
 *   onClose={() => setShowPinModal(false)}
 *   onSubmit={(pin) => handleAuthenticate(pin)}
 *   error={authError}
 *   serverName="RustRide-PC"
 * />
 * ```
 */
export function PinEntryModal({
  visible,
  onClose,
  onSubmit,
  isAuthenticating = false,
  error = null,
  serverName,
}: PinEntryModalProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  // PIN state
  const [pin, setPin] = useState('');
  const [hasError, setHasError] = useState(false);

  // Animation values
  const shakeAnimation = useRef(new Animated.Value(0)).current;
  const lastErrorRef = useRef<string | null>(null);

  // Reset PIN when modal opens
  useEffect(() => {
    if (visible) {
      setPin('');
      setHasError(false);
      lastErrorRef.current = null;
    }
  }, [visible]);

  // Handle error - trigger shake animation
  useEffect(() => {
    if (error && error !== lastErrorRef.current) {
      lastErrorRef.current = error;
      setHasError(true);
      setPin('');

      // Haptic feedback
      if (Platform.OS === 'ios') {
        Vibration.vibrate(50);
      } else {
        Vibration.vibrate(100);
      }

      // Shake animation sequence
      Animated.sequence([
        Animated.timing(shakeAnimation, {
          toValue: 10,
          duration: 50,
          useNativeDriver: true,
        }),
        Animated.timing(shakeAnimation, {
          toValue: -10,
          duration: 50,
          useNativeDriver: true,
        }),
        Animated.timing(shakeAnimation, {
          toValue: 10,
          duration: 50,
          useNativeDriver: true,
        }),
        Animated.timing(shakeAnimation, {
          toValue: -10,
          duration: 50,
          useNativeDriver: true,
        }),
        Animated.timing(shakeAnimation, {
          toValue: 0,
          duration: 50,
          useNativeDriver: true,
        }),
      ]).start(() => {
        // Clear error state after animation
        setTimeout(() => setHasError(false), 500);
      });
    }
  }, [error, shakeAnimation]);

  // Handle digit press
  const handleDigitPress = useCallback(
    (digit: string) => {
      if (isAuthenticating || pin.length >= PIN_LENGTH) {
        return;
      }

      const newPin = pin + digit;
      setPin(newPin);
      setHasError(false);

      // Auto-submit when PIN is complete
      if (newPin.length === PIN_LENGTH) {
        onSubmit(newPin);
      }
    },
    [pin, isAuthenticating, onSubmit],
  );

  // Handle delete press
  const handleDeletePress = useCallback(() => {
    if (isAuthenticating || pin.length === 0) {
      return;
    }

    setPin(pin.slice(0, -1));
    setHasError(false);
  }, [pin, isAuthenticating]);

  // Handle keypad key press
  const handleKeyPress = useCallback(
    (key: string) => {
      if (key === 'delete') {
        handleDeletePress();
      } else if (key !== '') {
        handleDigitPress(key);
      }
    },
    [handleDigitPress, handleDeletePress],
  );

  // Render PIN dots
  const renderPinDots = () => {
    const dots = [];
    for (let i = 0; i < PIN_LENGTH; i++) {
      const isFilled = i < pin.length;
      dots.push(
        <View
          key={i}
          style={[
            styles.pinDot,
            {
              backgroundColor: isFilled
                ? hasError
                  ? colors.error
                  : colors.accent
                : 'transparent',
              borderColor: hasError ? colors.error : colors.border,
            },
          ]}
          accessible={false}
        />,
      );
    }
    return dots;
  };

  // Render keypad button
  const renderKeypadButton = (key: string) => {
    if (key === '') {
      return <View style={styles.keypadButton} />;
    }

    const isDelete = key === 'delete';
    const isDisabled = isAuthenticating || (isDelete && pin.length === 0);

    return (
      <Pressable
        key={key}
        style={({ pressed }) => [
          styles.keypadButton,
          {
            backgroundColor: pressed && !isDisabled ? colors.surface : 'transparent',
            borderRadius: borderRadius.full,
            opacity: isDisabled ? 0.3 : 1,
          },
        ]}
        onPress={() => handleKeyPress(key)}
        disabled={isDisabled}
        accessibilityRole="button"
        accessibilityLabel={isDelete ? 'Delete' : key}
        accessibilityHint={isDelete ? 'Delete last digit' : `Enter digit ${key}`}
      >
        {isDelete ? (
          <Icon name="backspace-outline" size={28} color={colors.textPrimary} />
        ) : (
          <Text style={[styles.keypadDigit, { color: colors.textPrimary }]}>{key}</Text>
        )}
      </Pressable>
    );
  };

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <View style={[styles.overlay, { backgroundColor: colors.overlay }]}>
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
          <View style={styles.header}>
            <Icon name="lock-closed" size={32} color={colors.accent} />
            <Text
              style={[
                styles.title,
                textStyles.sectionTitle,
                { color: colors.textPrimary, marginTop: spacing.md },
              ]}
            >
              Enter PIN
            </Text>
            <Text
              style={[
                styles.subtitle,
                textStyles.body,
                { color: colors.textSecondary, marginTop: spacing.xs },
              ]}
            >
              {serverName
                ? `Enter the PIN shown on ${serverName}`
                : 'Enter the PIN shown on the RustRide desktop app'}
            </Text>
          </View>

          {/* PIN Display */}
          <Animated.View
            style={[
              styles.pinContainer,
              { marginTop: spacing['2xl'], transform: [{ translateX: shakeAnimation }] },
            ]}
            accessible
            accessibilityRole="text"
            accessibilityLabel={`PIN entry, ${pin.length} of ${PIN_LENGTH} digits entered`}
            accessibilityLiveRegion="polite"
          >
            {renderPinDots()}
          </Animated.View>

          {/* Error message */}
          {hasError && error && (
            <View style={[styles.errorContainer, { marginTop: spacing.md }]}>
              <Icon name="alert-circle" size={16} color={colors.error} />
              <Text
                style={[
                  styles.errorText,
                  textStyles.caption,
                  { color: colors.error, marginLeft: spacing.xs },
                ]}
              >
                {error}
              </Text>
            </View>
          )}

          {/* Authenticating indicator */}
          {isAuthenticating && (
            <View style={[styles.authenticatingContainer, { marginTop: spacing.md }]}>
              <LoadingSpinner size="small" centered={false} />
              <Text
                style={[
                  styles.authenticatingText,
                  textStyles.body,
                  { color: colors.textSecondary, marginLeft: spacing.sm },
                ]}
              >
                Authenticating...
              </Text>
            </View>
          )}

          {/* Keypad */}
          <View style={[styles.keypad, { marginTop: spacing.xl }]}>
            {KEYPAD_ROWS.map((row, rowIndex) => (
              <View key={rowIndex} style={styles.keypadRow}>
                {row.map((key, keyIndex) => (
                  <React.Fragment key={`${rowIndex}-${keyIndex}`}>
                    {renderKeypadButton(key)}
                  </React.Fragment>
                ))}
              </View>
            ))}
          </View>

          {/* Cancel button */}
          <Button
            title="Cancel"
            variant="ghost"
            onPress={onClose}
            disabled={isAuthenticating}
            fullWidth
            style={{ marginTop: spacing.lg }}
          />
        </View>
      </View>
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
  },
  modal: {
    width: '90%',
    maxWidth: 360,
    alignItems: 'center',
  },
  header: {
    alignItems: 'center',
  },
  title: {
    textAlign: 'center',
  },
  subtitle: {
    textAlign: 'center',
    maxWidth: 280,
  },
  pinContainer: {
    flexDirection: 'row',
    justifyContent: 'center',
    gap: 12,
  },
  pinDot: {
    width: 16,
    height: 16,
    borderRadius: 8,
    borderWidth: 2,
  },
  errorContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
  },
  errorText: {
    textAlign: 'center',
  },
  authenticatingContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
  },
  authenticatingText: {
    textAlign: 'center',
  },
  keypad: {
    width: '100%',
    maxWidth: 280,
  },
  keypadRow: {
    flexDirection: 'row',
    justifyContent: 'space-around',
    marginBottom: 12,
  },
  keypadButton: {
    width: 72,
    height: 72,
    justifyContent: 'center',
    alignItems: 'center',
  },
  keypadDigit: {
    fontSize: 28,
    fontWeight: '500',
  },
});
