/**
 * StopConfirmationModal Component
 *
 * Modal dialog for confirming workout/ride stop action.
 * Shows session information and requires user confirmation before stopping.
 *
 * Features:
 * - Shows session type (workout or free ride) and name
 * - Displays elapsed time
 * - Stop and Cancel buttons
 * - Loading state while stopping
 * - Accessible with proper ARIA labels
 */

import React from 'react';
import {
  View,
  Text,
  Modal,
  StyleSheet,
  Pressable,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { Button } from './Button';

/**
 * StopConfirmationModal props
 */
export interface StopConfirmationModalProps {
  /** Whether the modal is visible */
  visible: boolean;
  /** Called when the modal should close (user cancelled) */
  onClose: () => void;
  /** Called when the user confirms stop */
  onConfirm: () => void;
  /** Whether stop is in progress */
  isStopping?: boolean;
  /** Session type (workout or free_ride) */
  sessionType?: 'workout' | 'free_ride' | null;
  /** Workout name (if structured workout) */
  workoutName?: string | null;
  /** Elapsed time in seconds */
  elapsedSecs?: number;
}

/**
 * Format seconds to HH:MM:SS or MM:SS
 */
function formatTime(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
  }
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

/**
 * StopConfirmationModal Component
 *
 * Provides a confirmation dialog before stopping a workout or ride.
 * This prevents accidental stops and gives users a chance to review
 * their session before ending it.
 *
 * @example
 * ```tsx
 * <StopConfirmationModal
 *   visible={showStopModal}
 *   onClose={() => setShowStopModal(false)}
 *   onConfirm={handleStop}
 *   isStopping={isStopLoading}
 *   sessionType="workout"
 *   workoutName="Threshold Intervals"
 *   elapsedSecs={1234}
 * />
 * ```
 */
export function StopConfirmationModal({
  visible,
  onClose,
  onConfirm,
  isStopping = false,
  sessionType,
  workoutName,
  elapsedSecs = 0,
}: StopConfirmationModalProps): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { textStyles } = typography;

  // Determine session title
  const sessionTitle = workoutName
    ? workoutName
    : sessionType === 'workout'
      ? 'Structured Workout'
      : sessionType === 'free_ride'
        ? 'Free Ride'
        : 'Session';

  // Determine icon based on session type
  const iconName = sessionType === 'workout' ? 'barbell' : 'bicycle';

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <View style={[styles.overlay, { backgroundColor: colors.overlay }]}>
        <Pressable
          style={styles.backdrop}
          onPress={onClose}
          accessibilityLabel="Close modal"
          accessibilityRole="button"
        >
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
          accessibilityRole="alert"
          accessibilityLabel="Stop session confirmation"
        >
          {/* Warning icon */}
          <View
            style={[
              styles.iconContainer,
              { backgroundColor: `${colors.error}20` }, // 20 = 12.5% opacity in hex
            ]}
          >
            <Icon name="stop-circle" size={32} color={colors.error} />
          </View>

          {/* Title */}
          <Text
            style={[
              styles.title,
              textStyles.sectionTitle,
              { color: colors.textPrimary, marginTop: spacing.md },
            ]}
          >
            Stop Session?
          </Text>

          {/* Description */}
          <Text
            style={[
              styles.description,
              textStyles.body,
              { color: colors.textSecondary, marginTop: spacing.sm },
            ]}
          >
            Are you sure you want to stop this session? Your progress will be saved.
          </Text>

          {/* Session info card */}
          <View
            style={[
              styles.sessionCard,
              {
                backgroundColor: colors.surface,
                borderRadius: borderRadius.md,
                marginTop: spacing.lg,
                padding: spacing.md,
              },
            ]}
          >
            <View style={styles.sessionHeader}>
              <Icon name={iconName} size={20} color={colors.accent} />
              <Text
                style={[
                  styles.sessionType,
                  textStyles.label,
                  { color: colors.textSecondary, marginLeft: spacing.xs },
                ]}
              >
                {sessionType === 'workout' ? 'Workout' : 'Free Ride'}
              </Text>
            </View>
            <Text
              style={[
                styles.sessionName,
                textStyles.cardTitle,
                { color: colors.textPrimary, marginTop: spacing.xs },
              ]}
              numberOfLines={1}
            >
              {sessionTitle}
            </Text>
            <View style={[styles.timeContainer, { marginTop: spacing.sm }]}>
              <Icon name="time-outline" size={16} color={colors.textMuted} />
              <Text
                style={[
                  styles.elapsedTime,
                  textStyles.body,
                  { color: colors.textSecondary, marginLeft: spacing.xs },
                ]}
              >
                {formatTime(elapsedSecs)}
              </Text>
            </View>
          </View>

          {/* Buttons */}
          <View style={[styles.buttons, { marginTop: spacing.lg }]}>
            <Button
              title="Cancel"
              variant="ghost"
              onPress={onClose}
              disabled={isStopping}
              style={styles.cancelButton}
              testID="stop-modal-cancel"
            />
            <Button
              title="Stop Session"
              variant="danger"
              onPress={onConfirm}
              loading={isStopping}
              style={styles.stopButton}
              testID="stop-modal-confirm"
            />
          </View>
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
  iconContainer: {
    width: 64,
    height: 64,
    borderRadius: 32,
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
  sessionCard: {
    width: '100%',
  },
  sessionHeader: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  sessionType: {
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  sessionName: {
    fontWeight: '600',
  },
  timeContainer: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  elapsedTime: {
    fontVariant: ['tabular-nums'],
  },
  buttons: {
    flexDirection: 'row',
    width: '100%',
    gap: 12,
  },
  cancelButton: {
    flex: 1,
  },
  stopButton: {
    flex: 1,
  },
});
