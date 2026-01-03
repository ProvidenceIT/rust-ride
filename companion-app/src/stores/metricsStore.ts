/**
 * Metrics Store
 *
 * Manages real-time workout metrics received from the RustRide desktop app.
 * Updates at 1Hz when connected and subscribed to metrics.
 */

import { create } from 'zustand';
import type { LiveMetrics } from '@/types';

/**
 * Power zone definitions
 */
export type PowerZone =
  | 'recovery'
  | 'endurance'
  | 'tempo'
  | 'threshold'
  | 'vo2max'
  | 'anaerobic'
  | 'neuromuscular';

/**
 * Heart rate zone definitions
 */
export type HeartRateZone = 'zone1' | 'zone2' | 'zone3' | 'zone4' | 'zone5';

/**
 * Metrics history entry for averaging
 */
interface MetricsSample {
  timestamp: number;
  power: number;
  heartRate: number | null;
  cadence: number | null;
}

/**
 * Metrics store state
 */
interface MetricsState {
  // Current live metrics
  metrics: LiveMetrics;

  // Computed averages
  power3sAvg: number;
  powerSessionAvg: number;
  heartRateMax: number;

  // Recent samples for averaging (last 10 seconds)
  recentSamples: MetricsSample[];

  // Subscription status
  isSubscribed: boolean;
  lastUpdateAt: number | null;

  // Target values (during structured workouts)
  targetPower: number | null;
  targetCadence: number | null;
}

/**
 * Metrics store actions
 */
interface MetricsActions {
  // Metrics updates
  updateMetrics: (metrics: LiveMetrics) => void;
  setTargetPower: (watts: number | null) => void;
  setTargetCadence: (rpm: number | null) => void;

  // Subscription management
  setSubscribed: (isSubscribed: boolean) => void;

  // Session stats
  resetSessionStats: () => void;

  // Reset store
  reset: () => void;
}

/**
 * Default metrics values (no data)
 */
const defaultMetrics: LiveMetrics = {
  power_watts: 0,
  heart_rate_bpm: null,
  cadence_rpm: null,
  speed_kph: 0,
  distance_km: 0,
  calories: 0,
};

/**
 * Initial metrics state
 */
const initialState: MetricsState = {
  metrics: defaultMetrics,
  power3sAvg: 0,
  powerSessionAvg: 0,
  heartRateMax: 0,
  recentSamples: [],
  isSubscribed: false,
  lastUpdateAt: null,
  targetPower: null,
  targetCadence: null,
};

/**
 * Maximum samples to keep for averaging (10 seconds at 1Hz)
 */
const MAX_SAMPLES = 10;

/**
 * Samples to use for 3-second average
 */
const POWER_3S_SAMPLES = 3;

/**
 * Calculate average from samples
 */
function calculatePowerAverage(samples: MetricsSample[], count: number): number {
  const relevantSamples = samples.slice(-count);
  if (relevantSamples.length === 0) {
    return 0;
  }
  const sum = relevantSamples.reduce((acc, s) => acc + s.power, 0);
  return Math.round(sum / relevantSamples.length);
}

/**
 * Metrics store
 *
 * Manages real-time workout metrics with computed averages
 * and zone calculations.
 */
export const useMetricsStore = create<MetricsState & MetricsActions>()((set, get) => ({
  ...initialState,

  // Metrics updates
  updateMetrics: (metrics: LiveMetrics) => {
    const state = get();
    const now = Date.now();

    // Add new sample
    const newSample: MetricsSample = {
      timestamp: now,
      power: metrics.power_watts,
      heartRate: metrics.heart_rate_bpm,
      cadence: metrics.cadence_rpm,
    };

    // Keep only recent samples
    const recentSamples = [...state.recentSamples, newSample].slice(-MAX_SAMPLES);

    // Calculate 3-second power average
    const power3sAvg = calculatePowerAverage(recentSamples, POWER_3S_SAMPLES);

    // Calculate session average (simplified - uses all available samples)
    // Note: For full accuracy, session average should be tracked differently
    const powerSessionAvg = calculatePowerAverage(recentSamples, recentSamples.length);

    // Track max heart rate
    const heartRateMax = Math.max(state.heartRateMax, metrics.heart_rate_bpm ?? 0);

    set({
      metrics,
      recentSamples,
      power3sAvg,
      powerSessionAvg,
      heartRateMax,
      lastUpdateAt: now,
    });
  },

  setTargetPower: (watts: number | null) => {
    set({ targetPower: watts });
  },

  setTargetCadence: (rpm: number | null) => {
    set({ targetCadence: rpm });
  },

  // Subscription management
  setSubscribed: (isSubscribed: boolean) => {
    set({ isSubscribed });
  },

  // Session stats
  resetSessionStats: () => {
    set({
      powerSessionAvg: 0,
      heartRateMax: 0,
      recentSamples: [],
    });
  },

  // Reset store
  reset: () => {
    set(initialState);
  },
}));

// Selectors for optimized component subscriptions
export const selectCurrentPower = (state: MetricsState & MetricsActions) =>
  state.metrics.power_watts;

export const selectPower3sAvg = (state: MetricsState & MetricsActions) => state.power3sAvg;

export const selectHeartRate = (state: MetricsState & MetricsActions) =>
  state.metrics.heart_rate_bpm;

export const selectHeartRateMax = (state: MetricsState & MetricsActions) => state.heartRateMax;

export const selectCadence = (state: MetricsState & MetricsActions) => state.metrics.cadence_rpm;

export const selectSpeed = (state: MetricsState & MetricsActions) => state.metrics.speed_kph;

export const selectDistance = (state: MetricsState & MetricsActions) => state.metrics.distance_km;

export const selectCalories = (state: MetricsState & MetricsActions) => state.metrics.calories;

export const selectTargetPower = (state: MetricsState & MetricsActions) => state.targetPower;

export const selectTargetCadence = (state: MetricsState & MetricsActions) => state.targetCadence;

export const selectIsSubscribed = (state: MetricsState & MetricsActions) => state.isSubscribed;

export const selectLiveMetrics = (state: MetricsState & MetricsActions) => state.metrics;

/**
 * Get power zone based on FTP percentage
 * Zones: Recovery <55%, Endurance 55-75%, Tempo 75-90%, Threshold 90-105%,
 * VO2max 105-120%, Anaerobic 120-150%, Neuromuscular >150%
 */
export function getPowerZone(power: number, ftp: number): PowerZone {
  if (ftp <= 0) {
    return 'recovery';
  }
  const percentage = (power / ftp) * 100;

  if (percentage < 55) {
    return 'recovery';
  }
  if (percentage < 75) {
    return 'endurance';
  }
  if (percentage < 90) {
    return 'tempo';
  }
  if (percentage < 105) {
    return 'threshold';
  }
  if (percentage < 120) {
    return 'vo2max';
  }
  if (percentage < 150) {
    return 'anaerobic';
  }
  return 'neuromuscular';
}

/**
 * Get heart rate zone based on max HR percentage
 * Zone 1: 50-60%, Zone 2: 60-70%, Zone 3: 70-80%, Zone 4: 80-90%, Zone 5: 90-100%
 */
export function getHeartRateZone(hr: number, maxHr: number): HeartRateZone {
  if (maxHr <= 0) {
    return 'zone1';
  }
  const percentage = (hr / maxHr) * 100;

  if (percentage < 60) {
    return 'zone1';
  }
  if (percentage < 70) {
    return 'zone2';
  }
  if (percentage < 80) {
    return 'zone3';
  }
  if (percentage < 90) {
    return 'zone4';
  }
  return 'zone5';
}

/**
 * Get color for power zone
 */
export function getPowerZoneColor(zone: PowerZone): string {
  const colors: Record<PowerZone, string> = {
    recovery: '#808080', // Gray
    endurance: '#2196F3', // Blue
    tempo: '#4CAF50', // Green
    threshold: '#FFC107', // Yellow/Amber
    vo2max: '#FF9800', // Orange
    anaerobic: '#FF5722', // Deep Orange
    neuromuscular: '#F44336', // Red
  };
  return colors[zone];
}

/**
 * Get color for heart rate zone
 */
export function getHeartRateZoneColor(zone: HeartRateZone): string {
  const colors: Record<HeartRateZone, string> = {
    zone1: '#808080', // Gray
    zone2: '#2196F3', // Blue
    zone3: '#4CAF50', // Green
    zone4: '#FFC107', // Yellow/Amber
    zone5: '#F44336', // Red
  };
  return colors[zone];
}
