/**
 * Dashboard Screen
 *
 * Main screen showing real-time workout metrics including power,
 * heart rate, cadence, speed, distance, and calories.
 *
 * Features:
 * - Responsive grid layout that adapts to portrait/landscape orientations
 * - Connection status indicator in the header
 * - Real-time metrics from metricsStore
 * - Power zone color coding
 * - Unit preference support (metric/imperial) for speed and distance
 */

import React, { useMemo, useEffect } from 'react';
import {
  StyleSheet,
  Text,
  View,
  ScrollView,
  useWindowDimensions,
  RefreshControl,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { MainTabScreenProps } from '@/navigation/types';
import { useTheme } from '@/theme';
import {
  ConnectionStatus,
  PowerDisplay,
  HeartRateDisplay,
  CadenceDisplay,
  SpeedDisplay,
  DistanceDisplay,
  ElapsedTimeDisplay,
  CaloriesDisplay,
} from '@/components';
import { useConnectionStore, selectConnectionStatus, selectCurrentServer } from '@/stores/connectionStore';
import {
  useMetricsStore,
  selectCurrentPower,
  selectPower3sAvg,
  selectHeartRate,
  selectHeartRateMax,
  selectCadence,
  selectSpeed,
  selectDistance,
  selectCalories,
  selectTargetPower,
  selectTargetCadence,
  getPowerZone,
  getHeartRateZone,
} from '@/stores/metricsStore';
import { useSessionStore, selectIsSessionActive } from '@/stores/sessionStore';
import { useSettingsStore } from '@/stores/settingsStore';

/**
 * Orientation type for layout calculations
 */
type Orientation = 'portrait' | 'landscape';

/**
 * Grid configuration based on orientation
 */
interface GridConfig {
  columns: number;
  powerCardSpan: 'full' | 'half';
  gap: number;
}

/**
 * Get grid configuration based on screen dimensions
 */
function getGridConfig(width: number, height: number): GridConfig {
  const orientation: Orientation = width > height ? 'landscape' : 'portrait';

  if (orientation === 'landscape') {
    return {
      columns: 4,
      powerCardSpan: 'half',
      gap: 12,
    };
  }

  return {
    columns: 2,
    powerCardSpan: 'full',
    gap: 12,
  };
}


/**
 * Default FTP for zone calculations (should come from user settings)
 * TODO: Get from user settings when available
 */
const DEFAULT_FTP = 200;

/**
 * Default max HR for zone calculations (should come from user settings)
 * TODO: Get from user settings when available
 */
const DEFAULT_MAX_HR = 185;

type Props = MainTabScreenProps<'Dashboard'>;

export function DashboardScreen(_props: Props): React.JSX.Element {
  const { colors, spacing, typography, borderRadius } = useTheme();
  const { width, height } = useWindowDimensions();

  // Connection state
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const currentServer = useConnectionStore(selectCurrentServer);

  // Session state
  const isSessionActive = useSessionStore(selectIsSessionActive);
  const elapsedSecs = useSessionStore(state => state.elapsedSecs);

  // Settings state - load settings on mount
  const loadSettings = useSettingsStore(state => state.loadSettings);
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Metrics state
  const power = useMetricsStore(selectCurrentPower);
  const power3sAvg = useMetricsStore(selectPower3sAvg);
  const heartRate = useMetricsStore(selectHeartRate);
  const heartRateMax = useMetricsStore(selectHeartRateMax);
  const cadence = useMetricsStore(selectCadence);
  const speed = useMetricsStore(selectSpeed);
  const distance = useMetricsStore(selectDistance);
  const calories = useMetricsStore(selectCalories);
  const targetPower = useMetricsStore(selectTargetPower);
  const targetCadence = useMetricsStore(selectTargetCadence);

  // Calculate grid configuration
  const gridConfig = useMemo(() => getGridConfig(width, height), [width, height]);

  // Calculate power zone
  const powerZone = useMemo(() => getPowerZone(power, DEFAULT_FTP), [power]);

  // Calculate HR zone
  const hrZone = useMemo(
    () => (heartRate ? getHeartRateZone(heartRate, DEFAULT_MAX_HR) : null),
    [heartRate]
  );

  // Determine if we're connected and can show metrics
  const isConnected = connectionStatus === 'connected' || connectionStatus === 'authenticated';
  const showMetrics = isConnected && isSessionActive;

  // Server name for status display
  const serverName = currentServer
    ? `${currentServer.name || currentServer.host}:${currentServer.port}`
    : undefined;

  // Calculate card widths based on grid columns
  const cardWidth = useMemo(() => {
    const totalGap = (gridConfig.columns - 1) * gridConfig.gap;
    const padding = spacing.md * 2;
    const availableWidth = width - padding - totalGap;
    return availableWidth / gridConfig.columns;
  }, [width, gridConfig, spacing.md]);

  // Power card width (full width in portrait, half in landscape)
  const powerCardWidth = useMemo(() => {
    if (gridConfig.powerCardSpan === 'full') {
      return width - spacing.md * 2;
    }
    return cardWidth * 2 + gridConfig.gap;
  }, [gridConfig, cardWidth, width, spacing.md]);

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['top']}
    >
      {/* Header with title and connection status */}
      <View style={[styles.header, { paddingHorizontal: spacing.md }]}>
        <Text style={[styles.title, typography.textStyles.screenTitle, { color: colors.textPrimary }]}>
          Dashboard
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
        contentContainerStyle={[
          styles.content,
          { padding: spacing.md, gap: gridConfig.gap },
        ]}
        showsVerticalScrollIndicator={false}
        refreshControl={
          <RefreshControl
            refreshing={false}
            onRefresh={() => {
              // Could trigger a metrics refresh here
            }}
            tintColor={colors.accent}
          />
        }
      >
        {/* Metrics Grid */}
        <View style={[styles.metricsGrid, { gap: gridConfig.gap }]}>
          {/* Power Display - Primary metric with zone indicator */}
          <PowerDisplay
            power={power}
            power3sAvg={power3sAvg}
            powerZone={powerZone}
            targetPower={targetPower}
            showMetrics={showMetrics}
            style={{ width: powerCardWidth }}
          />

          {/* Heart Rate Display - with zone indicator and pulse animation */}
          <HeartRateDisplay
            heartRate={heartRate}
            hrZone={hrZone}
            maxHeartRate={heartRateMax}
            showMetrics={showMetrics}
            showPulseAnimation={showMetrics}
            style={{ width: cardWidth }}
          />

          {/* Cadence Display - with target and visual warning */}
          <CadenceDisplay
            cadence={cadence}
            targetCadence={targetCadence}
            showMetrics={showMetrics}
            style={{ width: cardWidth }}
          />

          {/* Speed - with unit preference support */}
          <SpeedDisplay
            speedKph={speed}
            showMetrics={showMetrics}
            style={{ width: cardWidth }}
          />

          {/* Distance - with unit preference support */}
          <DistanceDisplay
            distanceKm={distance}
            showMetrics={showMetrics}
            style={{ width: cardWidth }}
          />

          {/* Elapsed Time */}
          <ElapsedTimeDisplay
            elapsedSecs={elapsedSecs}
            showMetrics={showMetrics}
            style={{ width: cardWidth }}
          />

          {/* Calories */}
          <CaloriesDisplay
            calories={calories}
            showMetrics={showMetrics}
            style={{ width: cardWidth }}
          />
        </View>

        {/* No Session State */}
        {!showMetrics && (
          <View
            style={[
              styles.emptyState,
              {
                backgroundColor: colors.card,
                borderRadius: borderRadius.md,
                padding: spacing.xl,
              },
            ]}
          >
            <Text
              style={[
                styles.emptyStateTitle,
                typography.textStyles.sectionTitle,
                { color: colors.textPrimary },
              ]}
            >
              {!isConnected ? 'Not Connected' : 'No Active Session'}
            </Text>
            <Text
              style={[
                styles.emptyStateText,
                typography.textStyles.body,
                { color: colors.textSecondary },
              ]}
            >
              {!isConnected
                ? 'Connect to your RustRide desktop app to see live metrics'
                : 'Start a workout or free ride on the desktop app to see live metrics here'}
            </Text>
          </View>
        )}
      </ScrollView>
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
  },
  metricsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
  },
  emptyState: {
    alignItems: 'center',
    marginTop: 24,
  },
  emptyStateTitle: {
    marginBottom: 8,
    textAlign: 'center',
  },
  emptyStateText: {
    textAlign: 'center',
    lineHeight: 20,
  },
});
