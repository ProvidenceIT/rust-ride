/**
 * Ride Detail Screen
 *
 * Shows full details of a completed ride including all metrics,
 * zone distributions, and statistics.
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme, ScrollView } from 'react-native';
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

type Props = RootStackScreenProps<'RideDetail'>;

interface StatCardProps {
  label: string;
  value: string;
  unit?: string;
  colors: typeof Colors.light;
}

function StatCard({ label, value, unit, colors }: StatCardProps): React.JSX.Element {
  return (
    <View style={[styles.statCard, { backgroundColor: colors.surface }]}>
      <Text style={[styles.statLabel, { color: colors.textSecondary }]}>{label}</Text>
      <View style={styles.statValueContainer}>
        <Text style={[styles.statValue, { color: colors.text }]}>{value}</Text>
        {unit && <Text style={[styles.statUnit, { color: colors.textSecondary }]}>{unit}</Text>}
      </View>
    </View>
  );
}

export function RideDetailScreen({ route }: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;
  const { rideId } = route.params;

  // Placeholder data - will be fetched from store/API
  const ride = {
    id: rideId,
    date: 'January 3, 2026',
    time: '14:30',
    workoutName: null as string | null,
    duration: '1:00:00',
    distance: '25.5 km',
    avgPower: 180,
    maxPower: 350,
    normalizedPower: 195,
    avgHeartRate: 145,
    maxHeartRate: 172,
    avgCadence: 85,
    calories: 650,
    tss: 75,
    intensityFactor: 0.82,
  };

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['bottom']}>
      <ScrollView style={styles.scrollView} showsVerticalScrollIndicator={false}>
        {/* Header info */}
        <View style={[styles.header, { backgroundColor: colors.surface }]}>
          <Text style={[styles.date, { color: colors.text }]}>{ride.date}</Text>
          <Text style={[styles.time, { color: colors.textSecondary }]}>{ride.time}</Text>
          {ride.workoutName && (
            <View style={[styles.workoutBadge, { backgroundColor: colors.primary }]}>
              <Text style={styles.workoutBadgeText}>{ride.workoutName}</Text>
            </View>
          )}
        </View>

        {/* Summary stats */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Summary</Text>
          <View style={styles.statsGrid}>
            <StatCard label="Duration" value={ride.duration} colors={colors} />
            <StatCard label="Distance" value={ride.distance} colors={colors} />
            <StatCard label="Calories" value={String(ride.calories)} unit="kcal" colors={colors} />
            <StatCard label="TSS" value={String(ride.tss)} colors={colors} />
          </View>
        </View>

        {/* Power stats */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Power</Text>
          <View style={styles.statsGrid}>
            <StatCard label="Average" value={String(ride.avgPower)} unit="w" colors={colors} />
            <StatCard label="Maximum" value={String(ride.maxPower)} unit="w" colors={colors} />
            <StatCard
              label="Normalized"
              value={String(ride.normalizedPower)}
              unit="w"
              colors={colors}
            />
            <StatCard
              label="Intensity Factor"
              value={ride.intensityFactor.toFixed(2)}
              colors={colors}
            />
          </View>
        </View>

        {/* Heart rate stats */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Heart Rate</Text>
          <View style={styles.statsGrid}>
            <StatCard
              label="Average"
              value={String(ride.avgHeartRate)}
              unit="bpm"
              colors={colors}
            />
            <StatCard
              label="Maximum"
              value={String(ride.maxHeartRate)}
              unit="bpm"
              colors={colors}
            />
          </View>
        </View>

        {/* Cadence stats */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Cadence</Text>
          <View style={styles.statsGrid}>
            <StatCard label="Average" value={String(ride.avgCadence)} unit="rpm" colors={colors} />
          </View>
        </View>

        <View style={styles.footer} />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  scrollView: {
    flex: 1,
  },
  header: {
    padding: 20,
    alignItems: 'center',
    marginBottom: 16,
  },
  date: {
    fontSize: 20,
    fontWeight: '600',
    marginBottom: 4,
  },
  time: {
    fontSize: 16,
  },
  workoutBadge: {
    marginTop: 12,
    paddingHorizontal: 16,
    paddingVertical: 6,
    borderRadius: 16,
  },
  workoutBadgeText: {
    fontSize: 14,
    fontWeight: '500',
    color: '#FFFFFF',
  },
  section: {
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 12,
    paddingHorizontal: 16,
  },
  statsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    paddingHorizontal: 12,
    gap: 8,
  },
  statCard: {
    width: '48%',
    padding: 16,
    borderRadius: 12,
    flexGrow: 1,
    minWidth: 150,
  },
  statLabel: {
    fontSize: 12,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 4,
  },
  statValueContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
    gap: 4,
  },
  statValue: {
    fontSize: 24,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  statUnit: {
    fontSize: 14,
  },
  footer: {
    height: 40,
  },
});
