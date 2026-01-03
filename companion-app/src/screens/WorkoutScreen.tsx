/**
 * Workout Screen
 *
 * Screen for viewing active workout details and controlling the workout
 * (pause, resume, skip interval, stop).
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme, TouchableOpacity } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { MainTabScreenProps } from '@/navigation/types';

const Colors = {
  light: {
    background: '#FFFFFF',
    surface: '#F5F5F5',
    primary: '#007AFF',
    text: '#1C1C1E',
    textSecondary: '#8E8E93',
    border: '#E5E5EA',
    destructive: '#FF3B30',
  },
  dark: {
    background: '#000000',
    surface: '#1C1C1E',
    primary: '#0A84FF',
    text: '#FFFFFF',
    textSecondary: '#8E8E93',
    border: '#38383A',
    destructive: '#FF453A',
  },
};

type Props = MainTabScreenProps<'Workout'>;

export function WorkoutScreen(_props: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]} edges={['top']}>
      <View style={styles.header}>
        <Text style={[styles.title, { color: colors.text }]}>Workout</Text>
      </View>

      <View style={styles.content}>
        {/* Workout info card */}
        <View style={[styles.workoutCard, { backgroundColor: colors.surface }]}>
          <Text style={[styles.workoutName, { color: colors.text }]}>No Active Workout</Text>
          <Text style={[styles.workoutStatus, { color: colors.textSecondary }]}>
            Start a workout from the desktop app
          </Text>
        </View>

        {/* Interval progress */}
        <View style={[styles.intervalSection, { backgroundColor: colors.surface }]}>
          <View style={styles.intervalHeader}>
            <Text style={[styles.sectionTitle, { color: colors.text }]}>Current Interval</Text>
            <Text style={[styles.intervalCount, { color: colors.textSecondary }]}>- / -</Text>
          </View>
          <View style={[styles.progressBar, { backgroundColor: colors.border }]}>
            <View style={[styles.progressFill, { width: '0%', backgroundColor: colors.primary }]} />
          </View>
          <View style={styles.intervalDetails}>
            <Text style={[styles.intervalName, { color: colors.text }]}>---</Text>
            <Text style={[styles.intervalTime, { color: colors.textSecondary }]}>--:--</Text>
          </View>
        </View>

        {/* Target power */}
        <View style={[styles.targetSection, { backgroundColor: colors.surface }]}>
          <Text style={[styles.sectionTitle, { color: colors.text }]}>Target Power</Text>
          <View style={styles.targetValue}>
            <Text style={[styles.targetPower, { color: colors.primary }]}>---</Text>
            <Text style={[styles.targetUnit, { color: colors.textSecondary }]}>watts</Text>
          </View>
        </View>

        {/* Elapsed time */}
        <View style={[styles.timeSection, { backgroundColor: colors.surface }]}>
          <Text style={[styles.sectionTitle, { color: colors.text }]}>Elapsed Time</Text>
          <Text style={[styles.elapsedTime, { color: colors.text }]}>00:00:00</Text>
        </View>
      </View>

      {/* Control buttons */}
      <View style={[styles.controls, { backgroundColor: colors.surface, borderTopColor: colors.border }]}>
        <TouchableOpacity
          style={[styles.controlButton, { backgroundColor: colors.primary }]}
          disabled={true}
          activeOpacity={0.7}
        >
          <Text style={styles.controlButtonText}>Pause</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.controlButton, styles.controlButtonSecondary, { borderColor: colors.border }]}
          disabled={true}
          activeOpacity={0.7}
        >
          <Text style={[styles.controlButtonText, { color: colors.text }]}>Skip</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.controlButton, styles.controlButtonDestructive, { backgroundColor: colors.destructive }]}
          disabled={true}
          activeOpacity={0.7}
        >
          <Text style={styles.controlButtonText}>Stop</Text>
        </TouchableOpacity>
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  content: {
    flex: 1,
    padding: 16,
    gap: 16,
  },
  workoutCard: {
    padding: 20,
    borderRadius: 12,
    alignItems: 'center',
  },
  workoutName: {
    fontSize: 20,
    fontWeight: '600',
    marginBottom: 4,
  },
  workoutStatus: {
    fontSize: 14,
  },
  intervalSection: {
    padding: 16,
    borderRadius: 12,
  },
  intervalHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  intervalCount: {
    fontSize: 14,
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
    fontSize: 18,
    fontWeight: '500',
  },
  intervalTime: {
    fontSize: 18,
    fontVariant: ['tabular-nums'],
  },
  targetSection: {
    padding: 16,
    borderRadius: 12,
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
    fontSize: 18,
  },
  timeSection: {
    padding: 16,
    borderRadius: 12,
  },
  elapsedTime: {
    fontSize: 36,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
    marginTop: 8,
  },
  controls: {
    flexDirection: 'row',
    padding: 16,
    gap: 12,
    borderTopWidth: 1,
  },
  controlButton: {
    flex: 1,
    paddingVertical: 14,
    borderRadius: 12,
    alignItems: 'center',
    justifyContent: 'center',
  },
  controlButtonSecondary: {
    backgroundColor: 'transparent',
    borderWidth: 1,
  },
  controlButtonDestructive: {
    flex: 0.6,
  },
  controlButtonText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#FFFFFF',
  },
});
