/**
 * Settings Store
 *
 * Manages user preferences including unit system (metric/imperial),
 * display preferences, and other app settings.
 *
 * Settings are persisted to AsyncStorage for cross-session retention.
 */

import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';
import type { AppSettings } from '@/types';

/**
 * Unit system types
 */
export type UnitSystem = 'metric' | 'imperial';

/**
 * Haptic feedback intensity
 */
export type HapticIntensity = 'off' | 'light' | 'medium' | 'strong';

/**
 * Theme preference
 */
export type ThemePreference = 'system' | 'light' | 'dark';

/**
 * Storage key for settings
 */
const SETTINGS_STORAGE_KEY = '@rustride/settings';

/**
 * Default app settings
 */
const defaultSettings: AppSettings = {
  units: 'metric',
  keepScreenAwake: true,
  hapticFeedback: 'medium',
  theme: 'system',
};

/**
 * Settings store state
 */
interface SettingsState {
  /** Current app settings */
  settings: AppSettings;

  /** Whether settings have been loaded from storage */
  isLoaded: boolean;

  /** Whether settings are currently being saved */
  isSaving: boolean;
}

/**
 * Settings store actions
 */
interface SettingsActions {
  /** Load settings from AsyncStorage */
  loadSettings: () => Promise<void>;

  /** Set the unit system (metric/imperial) */
  setUnits: (units: UnitSystem) => Promise<void>;

  /** Toggle the unit system between metric and imperial */
  toggleUnits: () => Promise<void>;

  /** Set keep screen awake preference */
  setKeepScreenAwake: (enabled: boolean) => Promise<void>;

  /** Set haptic feedback intensity */
  setHapticFeedback: (intensity: HapticIntensity) => Promise<void>;

  /** Set theme preference */
  setTheme: (theme: ThemePreference) => Promise<void>;

  /** Update multiple settings at once */
  updateSettings: (updates: Partial<AppSettings>) => Promise<void>;

  /** Reset to default settings */
  resetSettings: () => Promise<void>;
}

/**
 * Initial settings state
 */
const initialState: SettingsState = {
  settings: defaultSettings,
  isLoaded: false,
  isSaving: false,
};

/**
 * Settings store
 *
 * Manages user preferences with persistence to AsyncStorage.
 */
export const useSettingsStore = create<SettingsState & SettingsActions>()((set, get) => ({
  ...initialState,

  // Load settings from AsyncStorage
  loadSettings: async () => {
    try {
      const json = await AsyncStorage.getItem(SETTINGS_STORAGE_KEY);
      if (json) {
        const loaded = JSON.parse(json) as Partial<AppSettings>;
        // Merge with defaults to ensure all fields exist
        const settings: AppSettings = {
          ...defaultSettings,
          ...loaded,
        };
        set({ settings, isLoaded: true });
      } else {
        set({ isLoaded: true });
      }
    } catch {
      // On error, use defaults
      set({ isLoaded: true });
    }
  },

  // Set the unit system
  setUnits: async (units: UnitSystem) => {
    const { settings } = get();
    const newSettings = { ...settings, units };
    set({ settings: newSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    } finally {
      set({ isSaving: false });
    }
  },

  // Toggle the unit system between metric and imperial
  toggleUnits: async () => {
    const { settings, setUnits } = get();
    const newUnits: UnitSystem = settings.units === 'metric' ? 'imperial' : 'metric';
    await setUnits(newUnits);
  },

  // Set keep screen awake preference
  setKeepScreenAwake: async (enabled: boolean) => {
    const { settings } = get();
    const newSettings = { ...settings, keepScreenAwake: enabled };
    set({ settings: newSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    } finally {
      set({ isSaving: false });
    }
  },

  // Set haptic feedback intensity
  setHapticFeedback: async (intensity: HapticIntensity) => {
    const { settings } = get();
    const newSettings = { ...settings, hapticFeedback: intensity };
    set({ settings: newSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    } finally {
      set({ isSaving: false });
    }
  },

  // Set theme preference
  setTheme: async (theme: ThemePreference) => {
    const { settings } = get();
    const newSettings = { ...settings, theme };
    set({ settings: newSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    } finally {
      set({ isSaving: false });
    }
  },

  // Update multiple settings at once
  updateSettings: async (updates: Partial<AppSettings>) => {
    const { settings } = get();
    const newSettings = { ...settings, ...updates };
    set({ settings: newSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    } finally {
      set({ isSaving: false });
    }
  },

  // Reset to default settings
  resetSettings: async () => {
    set({ settings: defaultSettings, isSaving: true });

    try {
      await AsyncStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(defaultSettings));
    } finally {
      set({ isSaving: false });
    }
  },
}));

// ============================================================
// Selectors
// ============================================================

export const selectSettings = (state: SettingsState & SettingsActions) => state.settings;
export const selectUnits = (state: SettingsState & SettingsActions) => state.settings.units;
export const selectIsMetric = (state: SettingsState & SettingsActions) =>
  state.settings.units === 'metric';
export const selectIsImperial = (state: SettingsState & SettingsActions) =>
  state.settings.units === 'imperial';
export const selectKeepScreenAwake = (state: SettingsState & SettingsActions) =>
  state.settings.keepScreenAwake;
export const selectHapticFeedback = (state: SettingsState & SettingsActions) =>
  state.settings.hapticFeedback;
export const selectTheme = (state: SettingsState & SettingsActions) => state.settings.theme;
export const selectIsLoaded = (state: SettingsState & SettingsActions) => state.isLoaded;
export const selectIsSaving = (state: SettingsState & SettingsActions) => state.isSaving;

// ============================================================
// Unit Conversion Utilities
// ============================================================

/** Kilometers to miles conversion factor */
const KM_TO_MI = 0.621371;

/** km/h to mph conversion factor */
const KPH_TO_MPH = 0.621371;

/**
 * Convert speed from km/h to the user's preferred unit
 *
 * @param speedKph Speed in km/h
 * @param units Unit system ('metric' | 'imperial')
 * @returns Converted speed value
 */
export function convertSpeed(speedKph: number, units: UnitSystem): number {
  if (units === 'imperial') {
    return speedKph * KPH_TO_MPH;
  }
  return speedKph;
}

/**
 * Get the speed unit label based on unit system
 *
 * @param units Unit system ('metric' | 'imperial')
 * @returns Unit label ('km/h' or 'mph')
 */
export function getSpeedUnit(units: UnitSystem): string {
  return units === 'imperial' ? 'mph' : 'km/h';
}

/**
 * Convert distance from km to the user's preferred unit
 *
 * @param distanceKm Distance in kilometers
 * @param units Unit system ('metric' | 'imperial')
 * @returns Converted distance value
 */
export function convertDistance(distanceKm: number, units: UnitSystem): number {
  if (units === 'imperial') {
    return distanceKm * KM_TO_MI;
  }
  return distanceKm;
}

/**
 * Get the distance unit label based on unit system
 *
 * @param units Unit system ('metric' | 'imperial')
 * @returns Unit label ('km' or 'mi')
 */
export function getDistanceUnit(units: UnitSystem): string {
  return units === 'imperial' ? 'mi' : 'km';
}

/**
 * Format speed with appropriate precision
 *
 * @param speedKph Speed in km/h
 * @param units Unit system ('metric' | 'imperial')
 * @returns Formatted speed string
 */
export function formatSpeed(speedKph: number, units: UnitSystem): string {
  const converted = convertSpeed(speedKph, units);
  return converted.toFixed(1);
}

/**
 * Format distance with appropriate precision based on magnitude
 *
 * @param distanceKm Distance in kilometers
 * @param units Unit system ('metric' | 'imperial')
 * @returns Formatted distance string
 */
export function formatDistance(distanceKm: number, units: UnitSystem): string {
  const converted = convertDistance(distanceKm, units);

  if (converted < 10) {
    return converted.toFixed(2);
  }
  if (converted < 100) {
    return converted.toFixed(1);
  }
  return converted.toFixed(0);
}

/**
 * Format elapsed time as HH:MM:SS or M:SS
 *
 * @param seconds Total elapsed seconds
 * @returns Formatted time string
 */
export function formatElapsedTime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Format calories with appropriate precision
 *
 * @param calories Calorie value
 * @returns Formatted calorie string
 */
export function formatCalories(calories: number): string {
  if (calories < 1000) {
    return Math.round(calories).toString();
  }
  // Show as X.Xk for large values
  return `${(calories / 1000).toFixed(1)}k`;
}
