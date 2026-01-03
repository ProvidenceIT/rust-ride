/**
 * Dashboard Screen
 *
 * Main screen showing real-time workout metrics including power,
 * heart rate, cadence, speed, distance, and calories.
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme } from 'react-native';
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

type Props = MainTabScreenProps<'Dashboard'>;

export function DashboardScreen(_props: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]} edges={['top']}>
      <View style={styles.header}>
        <Text style={[styles.title, { color: colors.text }]}>Dashboard</Text>
        <View style={[styles.statusBadge, { backgroundColor: colors.surface }]}>
          <View style={[styles.statusDot, { backgroundColor: colors.textSecondary }]} />
          <Text style={[styles.statusText, { color: colors.textSecondary }]}>Not connected</Text>
        </View>
      </View>

      <View style={styles.content}>
        <View style={styles.metricsGrid}>
          {/* Power metric - primary */}
          <View style={[styles.metricCard, styles.metricCardLarge, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, styles.metricValueLarge, { color: colors.text }]}>---</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>watts</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Power</Text>
          </View>

          {/* Heart rate */}
          <View style={[styles.metricCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, { color: colors.text }]}>---</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>bpm</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Heart Rate</Text>
          </View>

          {/* Cadence */}
          <View style={[styles.metricCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, { color: colors.text }]}>---</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>rpm</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Cadence</Text>
          </View>

          {/* Speed */}
          <View style={[styles.metricCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, { color: colors.text }]}>---</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>km/h</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Speed</Text>
          </View>

          {/* Distance */}
          <View style={[styles.metricCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, { color: colors.text }]}>0.00</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>km</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Distance</Text>
          </View>

          {/* Calories */}
          <View style={[styles.metricCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.metricValue, { color: colors.text }]}>0</Text>
            <Text style={[styles.metricUnit, { color: colors.textSecondary }]}>kcal</Text>
            <Text style={[styles.metricLabel, { color: colors.textSecondary }]}>Calories</Text>
          </View>
        </View>

        {/* Empty state message */}
        <View style={[styles.emptyState, { backgroundColor: colors.surface }]}>
          <Text style={[styles.emptyStateTitle, { color: colors.text }]}>No Active Session</Text>
          <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>
            Connect to your desktop app and start a workout to see live metrics
          </Text>
        </View>
      </View>
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
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  statusBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 16,
  },
  statusDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
    marginRight: 6,
  },
  statusText: {
    fontSize: 12,
    fontWeight: '500',
  },
  content: {
    flex: 1,
    padding: 16,
  },
  metricsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  metricCard: {
    width: '48%',
    padding: 16,
    borderRadius: 12,
    alignItems: 'center',
  },
  metricCardLarge: {
    width: '100%',
    paddingVertical: 24,
  },
  metricValue: {
    fontSize: 32,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  metricValueLarge: {
    fontSize: 56,
  },
  metricUnit: {
    fontSize: 14,
    marginTop: 4,
  },
  metricLabel: {
    fontSize: 12,
    marginTop: 8,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  emptyState: {
    marginTop: 24,
    padding: 24,
    borderRadius: 12,
    alignItems: 'center',
  },
  emptyStateTitle: {
    fontSize: 18,
    fontWeight: '600',
    marginBottom: 8,
  },
  emptyStateText: {
    fontSize: 14,
    textAlign: 'center',
    lineHeight: 20,
  },
});
