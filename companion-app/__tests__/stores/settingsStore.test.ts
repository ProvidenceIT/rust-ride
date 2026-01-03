/**
 * Settings Store Tests
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  useSettingsStore,
  convertSpeed,
  convertDistance,
  getSpeedUnit,
  getDistanceUnit,
  formatSpeed,
  formatDistance,
  formatElapsedTime,
  formatCalories,
  type UnitSystem,
} from '../../src/stores/settingsStore';

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () => ({
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  multiRemove: jest.fn(),
}));

describe('settingsStore', () => {
  beforeEach(() => {
    // Reset store state before each test
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'system',
      },
      isLoaded: false,
      isSaving: false,
    });
    jest.clearAllMocks();
  });

  describe('loadSettings', () => {
    it('should load settings from AsyncStorage', async () => {
      const savedSettings = {
        units: 'imperial',
        keepScreenAwake: false,
        hapticFeedback: 'strong',
        theme: 'dark',
      };
      (AsyncStorage.getItem as jest.Mock).mockResolvedValue(
        JSON.stringify(savedSettings)
      );

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().settings.units).toBe('imperial');
      expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(false);
      expect(useSettingsStore.getState().settings.hapticFeedback).toBe('strong');
      expect(useSettingsStore.getState().settings.theme).toBe('dark');
      expect(useSettingsStore.getState().isLoaded).toBe(true);
    });

    it('should use default settings when storage is empty', async () => {
      (AsyncStorage.getItem as jest.Mock).mockResolvedValue(null);

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().settings.units).toBe('metric');
      expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(true);
      expect(useSettingsStore.getState().isLoaded).toBe(true);
    });

    it('should merge partial saved settings with defaults', async () => {
      (AsyncStorage.getItem as jest.Mock).mockResolvedValue(
        JSON.stringify({ units: 'imperial' })
      );

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().settings.units).toBe('imperial');
      expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(true);
      expect(useSettingsStore.getState().settings.hapticFeedback).toBe('medium');
    });

    it('should handle storage errors gracefully', async () => {
      (AsyncStorage.getItem as jest.Mock).mockRejectedValue(
        new Error('Storage error')
      );

      await useSettingsStore.getState().loadSettings();

      expect(useSettingsStore.getState().isLoaded).toBe(true);
      expect(useSettingsStore.getState().settings.units).toBe('metric');
    });
  });

  describe('setUnits', () => {
    it('should update unit setting and save to storage', async () => {
      await useSettingsStore.getState().setUnits('imperial');

      expect(useSettingsStore.getState().settings.units).toBe('imperial');
      expect(AsyncStorage.setItem).toHaveBeenCalled();
    });
  });

  describe('toggleUnits', () => {
    it('should toggle from metric to imperial', async () => {
      await useSettingsStore.getState().toggleUnits();

      expect(useSettingsStore.getState().settings.units).toBe('imperial');
    });

    it('should toggle from imperial to metric', async () => {
      useSettingsStore.setState({
        settings: {
          ...useSettingsStore.getState().settings,
          units: 'imperial',
        },
      });

      await useSettingsStore.getState().toggleUnits();

      expect(useSettingsStore.getState().settings.units).toBe('metric');
    });
  });

  describe('resetSettings', () => {
    it('should reset to default settings', async () => {
      useSettingsStore.setState({
        settings: {
          units: 'imperial',
          keepScreenAwake: false,
          hapticFeedback: 'off',
          theme: 'dark',
        },
      });

      await useSettingsStore.getState().resetSettings();

      expect(useSettingsStore.getState().settings.units).toBe('metric');
      expect(useSettingsStore.getState().settings.keepScreenAwake).toBe(true);
    });
  });
});

describe('Unit Conversion Utilities', () => {
  describe('convertSpeed', () => {
    it('should not convert metric speed', () => {
      expect(convertSpeed(30, 'metric')).toBe(30);
    });

    it('should convert km/h to mph', () => {
      const result = convertSpeed(30, 'imperial');
      expect(result).toBeCloseTo(18.64, 1);
    });
  });

  describe('convertDistance', () => {
    it('should not convert metric distance', () => {
      expect(convertDistance(10, 'metric')).toBe(10);
    });

    it('should convert km to miles', () => {
      const result = convertDistance(10, 'imperial');
      expect(result).toBeCloseTo(6.21, 1);
    });
  });

  describe('getSpeedUnit', () => {
    it('should return km/h for metric', () => {
      expect(getSpeedUnit('metric')).toBe('km/h');
    });

    it('should return mph for imperial', () => {
      expect(getSpeedUnit('imperial')).toBe('mph');
    });
  });

  describe('getDistanceUnit', () => {
    it('should return km for metric', () => {
      expect(getDistanceUnit('metric')).toBe('km');
    });

    it('should return mi for imperial', () => {
      expect(getDistanceUnit('imperial')).toBe('mi');
    });
  });

  describe('formatSpeed', () => {
    it('should format metric speed with one decimal', () => {
      expect(formatSpeed(32.567, 'metric')).toBe('32.6');
    });

    it('should format imperial speed with one decimal', () => {
      expect(formatSpeed(32.567, 'imperial')).toBe('20.2');
    });
  });

  describe('formatDistance', () => {
    it('should format small distances with 2 decimals', () => {
      expect(formatDistance(5.678, 'metric')).toBe('5.68');
    });

    it('should format medium distances with 1 decimal', () => {
      expect(formatDistance(45.678, 'metric')).toBe('45.7');
    });

    it('should format large distances with no decimals', () => {
      expect(formatDistance(123.456, 'metric')).toBe('123');
    });

    it('should convert and format imperial distances', () => {
      const result = formatDistance(16.09, 'imperial');
      expect(result).toBe('10.00'); // ~10 miles (16.09 * 0.621371 = 9.9959 rounds to 10.00)
    });
  });

  describe('formatElapsedTime', () => {
    it('should format seconds under a minute', () => {
      expect(formatElapsedTime(45)).toBe('0:45');
    });

    it('should format minutes and seconds', () => {
      expect(formatElapsedTime(125)).toBe('2:05');
    });

    it('should format hours, minutes, and seconds', () => {
      expect(formatElapsedTime(3725)).toBe('1:02:05');
    });

    it('should pad single digits', () => {
      expect(formatElapsedTime(61)).toBe('1:01');
    });
  });

  describe('formatCalories', () => {
    it('should format small calorie values', () => {
      expect(formatCalories(523)).toBe('523');
    });

    it('should round calorie values', () => {
      expect(formatCalories(523.7)).toBe('524');
    });

    it('should format large calorie values as X.Xk', () => {
      expect(formatCalories(1500)).toBe('1.5k');
    });

    it('should format very large calorie values', () => {
      expect(formatCalories(2345)).toBe('2.3k');
    });
  });
});
