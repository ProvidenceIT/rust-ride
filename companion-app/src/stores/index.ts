/**
 * RustRide Companion App - State Management
 *
 * Zustand stores for managing app state.
 */

// Connection store - WebSocket connection state
export {
  useConnectionStore,
  selectConnectionStatus,
  selectServerUrl,
  selectIsAuthenticated,
  selectDiscoveredServers,
  selectIsScanning,
  selectConnectionError,
  selectCanReconnect,
  selectIsConnecting,
  selectIsConnected,
} from './connectionStore';

// Metrics store - Real-time workout metrics
export {
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
  selectIsSubscribed,
  selectLiveMetrics,
  getPowerZone,
  getHeartRateZone,
  getPowerZoneColor,
  getHeartRateZoneColor,
  type PowerZone,
  type HeartRateZone,
} from './metricsStore';

// Session store - Workout/ride session state
export {
  useSessionStore,
  selectIsSessionActive,
  selectSessionId,
  selectSessionType,
  selectSessionState,
  selectWorkoutName,
  selectIsPaused,
  selectElapsedSecs,
  selectCurrentInterval,
  selectTargetPower as selectSessionTargetPower,
  selectIsWorkout,
  selectIsFreeRide,
  selectIntervalProgress,
  selectCanPause,
  selectCanResume,
  selectCanSkip,
  selectCanStop,
} from './sessionStore';

// History store - Ride history and details
export {
  useHistoryStore,
  selectRides,
  selectIsLoading,
  selectIsLoadingMore,
  selectError,
  selectHasMore,
  selectTotal,
  selectFilters,
  selectCurrentRideDetail,
  selectIsLoadingDetail,
  selectPagination,
  selectRideCount,
  selectIsEmpty,
  selectHasFiltersApplied,
  getFilterDateRange,
  type DateRangeFilter,
  type RideTypeFilter,
} from './historyStore';

// Settings store - User preferences including unit system
export {
  useSettingsStore,
  selectSettings,
  selectUnits,
  selectIsMetric,
  selectIsImperial,
  selectKeepScreenAwake,
  selectHapticFeedback,
  selectTheme,
  selectIsLoaded as selectSettingsIsLoaded,
  selectIsSaving as selectSettingsIsSaving,
  // Unit conversion utilities
  convertSpeed,
  convertDistance,
  getSpeedUnit,
  getDistanceUnit,
  formatSpeed,
  formatDistance,
  formatElapsedTime,
  formatCalories,
  type UnitSystem,
  type HapticIntensity,
  type ThemePreference,
} from './settingsStore';
