/**
 * History Screen
 *
 * Displays a list of past rides with summary information.
 * Tapping a ride navigates to the detail screen.
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme, FlatList, TouchableOpacity } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
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

// Placeholder data structure for ride items
interface RideItem {
  id: string;
  date: string;
  workoutName: string | null;
  duration: string;
  distance: string;
  avgPower: number;
}

type Props = MainTabScreenProps<'History'>;

export function HistoryScreen(_props: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;
  const navigation = useNavigation();

  // Empty state - no rides yet
  const rides: RideItem[] = [];

  const handleRidePress = (rideId: string) => {
    navigation.navigate('RideDetail', { rideId });
  };

  const renderRideItem = ({ item }: { item: RideItem }) => (
    <TouchableOpacity
      style={[styles.rideCard, { backgroundColor: colors.surface }]}
      onPress={() => handleRidePress(item.id)}
      activeOpacity={0.7}>
      <View style={styles.rideHeader}>
        <Text style={[styles.rideDate, { color: colors.text }]}>{item.date}</Text>
        {item.workoutName && (
          <View style={[styles.workoutBadge, { backgroundColor: colors.primary }]}>
            <Text style={styles.workoutBadgeText}>{item.workoutName}</Text>
          </View>
        )}
      </View>
      <View style={styles.rideStats}>
        <View style={styles.rideStat}>
          <Text style={[styles.rideStatValue, { color: colors.text }]}>{item.duration}</Text>
          <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>Duration</Text>
        </View>
        <View style={styles.rideStat}>
          <Text style={[styles.rideStatValue, { color: colors.text }]}>{item.distance}</Text>
          <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>Distance</Text>
        </View>
        <View style={styles.rideStat}>
          <Text style={[styles.rideStatValue, { color: colors.text }]}>{item.avgPower}w</Text>
          <Text style={[styles.rideStatLabel, { color: colors.textSecondary }]}>Avg Power</Text>
        </View>
      </View>
    </TouchableOpacity>
  );

  const renderEmptyState = () => (
    <View style={[styles.emptyState, { backgroundColor: colors.surface }]}>
      <Text style={[styles.emptyStateTitle, { color: colors.text }]}>No Rides Yet</Text>
      <Text style={[styles.emptyStateText, { color: colors.textSecondary }]}>
        Complete a ride on your desktop app and it will appear here
      </Text>
    </View>
  );

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['top']}>
      <View style={styles.header}>
        <Text style={[styles.title, { color: colors.text }]}>History</Text>
      </View>

      <FlatList
        data={rides}
        keyExtractor={item => item.id}
        renderItem={renderRideItem}
        contentContainerStyle={styles.listContent}
        ListEmptyComponent={renderEmptyState}
        showsVerticalScrollIndicator={false}
      />
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
  listContent: {
    padding: 16,
    gap: 12,
    flexGrow: 1,
  },
  rideCard: {
    padding: 16,
    borderRadius: 12,
  },
  rideHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  rideDate: {
    fontSize: 16,
    fontWeight: '600',
  },
  workoutBadge: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
  },
  workoutBadgeText: {
    fontSize: 12,
    fontWeight: '500',
    color: '#FFFFFF',
  },
  rideStats: {
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  rideStat: {
    alignItems: 'center',
  },
  rideStatValue: {
    fontSize: 18,
    fontWeight: '600',
    fontVariant: ['tabular-nums'],
  },
  rideStatLabel: {
    fontSize: 12,
    marginTop: 4,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 24,
    borderRadius: 12,
    marginTop: 48,
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
