/**
 * Workout Screen
 *
 * Screen for viewing active workout details and controlling the workout
 * (pause, resume, skip interval, stop).
 *
 * Features:
 * - Display current workout name and status
 * - Show current interval with progress bar
 * - Display target power and elapsed time
 * - Control buttons for pause/resume, skip interval, stop
 * - Toast notifications on skip success/failure
 */

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { StyleSheet, Text, View, ScrollView } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { MainTabScreenProps } from '@/navigation/types';
import { useTheme } from '@/theme';
import { WorkoutControlBar, NoSessionState, ConnectionStatus, StopConfirmationModal } from '@/components';
import { useWorkoutControls, useToast, useHaptics } from '@/hooks';
import {
  useSessionStore,
  selectIsSessionActive,
  selectIsPaused,
  selectCurrentInterval,
  selectWorkoutName,
  selectTargetPower,
  selectIsWorkout,
  selectCanSkip,
  selectSessionType,
} from '@/stores/sessionStore';
import { useConnectionStore, selectConnectionStatus, selectCurrentServer } from '@/stores/connectionStore';
import { useNavigation } from '@react-navigation/native';
import type { RootStackNavigationProp } from '@/navigation/types';

/**
 * Format seconds to MM:SS or HH:MM:SS
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

type Props = MainTabScreenProps<'Workout'>;

export function WorkoutScreen(_props: Props): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();
  const navigation = useNavigation<RootStackNavigationProp>();

  // Toast notifications
  const { showSuccess, showError } = useToast();

  // Haptic feedback
  const { successHaptic, errorHaptic, warningHaptic } = useHaptics();

  // Stop confirmation modal state
  const [showStopModal, setShowStopModal] = useState(false);

  // Session state
  const isSessionActive = useSessionStore(selectIsSessionActive);
  const isPaused = useSessionStore(selectIsPaused);
  const currentInterval = useSessionStore(selectCurrentInterval);
  const workoutName = useSessionStore(selectWorkoutName);
  const targetPower = useSessionStore(selectTargetPower);
  const elapsedSecs = useSessionStore(state => state.elapsedSecs);
  const isWorkout = useSessionStore(selectIsWorkout);
  const canSkip = useSessionStore(selectCanSkip);
  const sessionType = useSessionStore(selectSessionType);

  // Connection state
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const currentServer = useConnectionStore(selectCurrentServer);
  const isConnected = connectionStatus === 'connected' || connectionStatus === 'authenticated';

  // Workout controls
  const {
    pause,
    resume,
    skip,
    stop,
    isPauseResumeLoading,
    isSkipLoading,
    isStopLoading,
    skipState,
  } = useWorkoutControls();

  // Track previous interval to detect changes (for toast on skip)
  const prevIntervalRef = useRef(currentInterval?.index);
  const skipPendingRef = useRef(false);
  const stopPendingRef = useRef(false);

  // Show toast when interval changes after skip
  useEffect(() => {
    // If we triggered a skip and the interval changed
    if (skipPendingRef.current && currentInterval) {
      if (prevIntervalRef.current !== currentInterval.index) {
        // Successfully skipped
        skipPendingRef.current = false;
        const intervalLabel = currentInterval.name
          ? `Now: ${currentInterval.name}`
          : `Interval ${currentInterval.index + 1} of ${currentInterval.total}`;
        showSuccess(intervalLabel);
        successHaptic();
      }
    }
    prevIntervalRef.current = currentInterval?.index;
  }, [currentInterval, showSuccess, successHaptic]);

  // Handle skip error
  useEffect(() => {
    if (skipState.error) {
      skipPendingRef.current = false;
      showError(`Failed to skip: ${skipState.error}`);
      errorHaptic();
    }
  }, [skipState.error, showError, errorHaptic]);

  // Handle session end after stop - navigate to Dashboard
  useEffect(() => {
    if (stopPendingRef.current && !isSessionActive) {
      stopPendingRef.current = false;
      setShowStopModal(false);
      showSuccess('Session saved');
      successHaptic();
      // Navigate to Dashboard tab
      navigation.navigate('Main', { screen: 'Dashboard' });
    }
  }, [isSessionActive, navigation, showSuccess, successHaptic]);

  // Handle connect press
  const handleConnectPress = useCallback(() => {
    navigation.navigate('Connection');
  }, [navigation]);

  // Handle pause
  const handlePause = useCallback(async () => {
    await pause();
  }, [pause]);

  // Handle resume
  const handleResume = useCallback(async () => {
    await resume();
  }, [resume]);

  // Handle skip with toast
  const handleSkip = useCallback(async () => {
    skipPendingRef.current = true;
    await skip();
  }, [skip]);

  // Handle stop button press - show confirmation modal
  const handleStopPress = useCallback(() => {
    warningHaptic();
    setShowStopModal(true);
  }, [warningHaptic]);

  // Handle stop modal close
  const handleStopModalClose = useCallback(() => {
    setShowStopModal(false);
  }, []);

  // Handle stop confirmation - actually stop the session
  const handleStopConfirm = useCallback(async () => {
    stopPendingRef.current = true;
    try {
      await stop();
      // Navigation happens in the effect above when isSessionActive becomes false
    } catch {
      stopPendingRef.current = false;
      setShowStopModal(false);
      showError('Failed to stop session');
      errorHaptic();
    }
  }, [stop, showError, errorHaptic]);

  // Calculate interval progress
  const intervalProgress = currentInterval
    ? ((currentInterval.index + 1) / currentInterval.total) * 100
    : 0;

  // Server name for status display
  const serverName = currentServer
    ? `${currentServer.name || currentServer.host}:${currentServer.port}`
    : undefined;

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['top']}
    >
      {/* Header */}
      <View style={[styles.header, { paddingHorizontal: spacing.md }]}>
        <Text style={[styles.title, typography.textStyles.screenTitle, { color: colors.textPrimary }]}>
          Workout
        </Text>
        <ConnectionStatus
          status={connectionStatus}
          variant="badge"
          animated
          serverName={serverName}
        />
      </View>

      <ScrollView
        style={styles.scrollView}
        contentContainerStyle={[styles.content, { padding: spacing.md, gap: spacing.md }]}
        showsVerticalScrollIndicator={false}
      >
        {/* Show content only when connected and has active session */}
        {isConnected && isSessionActive ? (
          <>
            {/* Workout info card */}
            <View style={[styles.card, { backgroundColor: colors.surface }]}>
              <Text style={[styles.workoutName, typography.textStyles.sectionTitle, { color: colors.textPrimary }]}>
                {workoutName || (isWorkout ? 'Structured Workout' : 'Free Ride')}
              </Text>
              <Text style={[styles.workoutStatus, typography.textStyles.bodySecondary, { color: colors.textSecondary }]}>
                {isPaused ? 'Paused' : 'In Progress'}
              </Text>
            </View>

            {/* Interval progress - only for workouts */}
            {isWorkout && currentInterval && (
              <View style={[styles.card, { backgroundColor: colors.surface }]}>
                <View style={styles.intervalHeader}>
                  <Text style={[styles.sectionTitle, typography.textStyles.label, { color: colors.textSecondary }]}>
                    Current Interval
                  </Text>
                  <Text style={[styles.intervalCount, typography.textStyles.body, { color: colors.textSecondary }]}>
                    {currentInterval.index + 1} / {currentInterval.total}
                  </Text>
                </View>
                <View style={[styles.progressBar, { backgroundColor: colors.border }]}>
                  <View
                    style={[
                      styles.progressFill,
                      {
                        width: `${intervalProgress}%`,
                        backgroundColor: colors.accent,
                      },
                    ]}
                  />
                </View>
                <View style={styles.intervalDetails}>
                  <Text style={[styles.intervalName, typography.textStyles.listTitle, { color: colors.textPrimary }]}>
                    {currentInterval.name || `Interval ${currentInterval.index + 1}`}
                  </Text>
                  {currentInterval.remainingSecs != null && (
                    <Text
                      style={[
                        styles.intervalTime,
                        typography.textStyles.listTitle,
                        { color: colors.textSecondary },
                      ]}
                    >
                      {formatTime(currentInterval.remainingSecs)}
                    </Text>
                  )}
                </View>
                {!canSkip && (
                  <Text style={[styles.lastIntervalHint, typography.textStyles.caption, { color: colors.textMuted }]}>
                    This is the last interval
                  </Text>
                )}
              </View>
            )}

            {/* Target power - only for workouts */}
            {isWorkout && targetPower != null && (
              <View style={[styles.card, { backgroundColor: colors.surface }]}>
                <Text style={[styles.sectionTitle, typography.textStyles.label, { color: colors.textSecondary }]}>
                  Target Power
                </Text>
                <View style={styles.targetValue}>
                  <Text style={[styles.targetPower, { color: colors.accent }]}>
                    {targetPower}
                  </Text>
                  <Text style={[styles.targetUnit, typography.textStyles.body, { color: colors.textSecondary }]}>
                    watts
                  </Text>
                </View>
              </View>
            )}

            {/* Elapsed time */}
            <View style={[styles.card, { backgroundColor: colors.surface }]}>
              <Text style={[styles.sectionTitle, typography.textStyles.label, { color: colors.textSecondary }]}>
                Elapsed Time
              </Text>
              <Text style={[styles.elapsedTime, { color: colors.textPrimary }]}>
                {formatTime(elapsedSecs)}
              </Text>
            </View>
          </>
        ) : (
          /* No Session State */
          <NoSessionState
            connectionStatus={connectionStatus}
            serverName={serverName}
            onConnectPress={handleConnectPress}
          />
        )}
      </ScrollView>

      {/* Control bar - always visible when connected */}
      {isConnected && (
        <WorkoutControlBar
          onPause={handlePause}
          onResume={handleResume}
          onSkip={handleSkip}
          onStop={handleStopPress}
          isPauseLoading={isPauseResumeLoading}
          isSkipLoading={isSkipLoading}
          isStopLoading={isStopLoading}
          testID="workout-control-bar"
        />
      )}

      {/* Stop confirmation modal */}
      <StopConfirmationModal
        visible={showStopModal}
        onClose={handleStopModalClose}
        onConfirm={handleStopConfirm}
        isStopping={isStopLoading}
        sessionType={sessionType}
        workoutName={workoutName}
        elapsedSecs={elapsedSecs}
      />
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 12,
  },
  title: {
    // Typography from theme
  },
  scrollView: {
    flex: 1,
  },
  content: {
    flexGrow: 1,
    // Allow space for the control bar at the bottom
    paddingBottom: 120,
  },
  card: {
    padding: 20,
    borderRadius: 12,
  },
  workoutName: {
    marginBottom: 4,
    textAlign: 'center',
  },
  workoutStatus: {
    textAlign: 'center',
  },
  intervalHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  sectionTitle: {
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  intervalCount: {
    // Typography from theme
  },
  progressBar: {
    height: 8,
    borderRadius: 4,
    marginBottom: 12,
    overflow: 'hidden',
  },
  progressFill: {
    height: '100%',
    borderRadius: 4,
  },
  intervalDetails: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  intervalName: {
    flex: 1,
  },
  intervalTime: {
    fontVariant: ['tabular-nums'],
  },
  lastIntervalHint: {
    marginTop: 8,
    textAlign: 'center',
    fontStyle: 'italic',
  },
  targetValue: {
    flexDirection: 'row',
    alignItems: 'baseline',
    marginTop: 8,
    gap: 8,
  },
  targetPower: {
    fontSize: 48,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  targetUnit: {
    // Typography from theme
  },
  elapsedTime: {
    fontSize: 36,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
    marginTop: 8,
  },
});
