/**
 * Theme Module Tests
 *
 * Tests for the RustRide Companion App theme system.
 */

import {
  darkColors,
  lightColors,
  zoneColors,
  getPowerZoneColor,
  getHrZoneColor,
  darkTheme,
  lightTheme,
  getTheme,
} from '@theme';

describe('Colors', () => {
  describe('darkColors', () => {
    it('has background color matching desktop DarkTheme', () => {
      // rgb(18, 18, 24) = #121218
      expect(darkColors.background).toBe('#121218');
    });

    it('has primary text color matching desktop DarkTheme', () => {
      // rgb(240, 240, 245) = #F0F0F5
      expect(darkColors.textPrimary).toBe('#F0F0F5');
    });

    it('has accent color matching desktop DarkTheme', () => {
      // rgb(66, 133, 244) = #4285F4
      expect(darkColors.accent).toBe('#4285F4');
    });

    it('has semantic colors defined', () => {
      expect(darkColors.success).toBe('#34A853');
      expect(darkColors.warning).toBe('#FBBC04');
      expect(darkColors.error).toBe('#EA4335');
    });
  });

  describe('lightColors', () => {
    it('has background color matching desktop LightTheme', () => {
      // rgb(250, 250, 252) = #FAFAFC
      expect(lightColors.background).toBe('#FAFAFC');
    });

    it('has primary text color matching desktop LightTheme', () => {
      // rgb(32, 32, 40) = #202028
      expect(lightColors.textPrimary).toBe('#202028');
    });

    it('has accent color matching desktop LightTheme', () => {
      // rgb(26, 115, 232) = #1A73E8
      expect(lightColors.accent).toBe('#1A73E8');
    });
  });

  describe('zoneColors', () => {
    it('has all 7 power zones defined', () => {
      expect(zoneColors.z1Recovery).toBeDefined();
      expect(zoneColors.z2Endurance).toBeDefined();
      expect(zoneColors.z3Tempo).toBeDefined();
      expect(zoneColors.z4Threshold).toBeDefined();
      expect(zoneColors.z5Vo2max).toBeDefined();
      expect(zoneColors.z6Anaerobic).toBeDefined();
      expect(zoneColors.z7Neuromuscular).toBeDefined();
    });

    it('matches desktop zone colors', () => {
      expect(zoneColors.z1Recovery).toBe('#808080'); // Gray
      expect(zoneColors.z4Threshold).toBe('#FFC800'); // Yellow
      expect(zoneColors.z6Anaerobic).toBe('#FF3232'); // Red
    });
  });
});

describe('Zone Color Functions', () => {
  describe('getPowerZoneColor', () => {
    it('returns correct color for each zone', () => {
      expect(getPowerZoneColor(1)).toBe(zoneColors.z1Recovery);
      expect(getPowerZoneColor(2)).toBe(zoneColors.z2Endurance);
      expect(getPowerZoneColor(3)).toBe(zoneColors.z3Tempo);
      expect(getPowerZoneColor(4)).toBe(zoneColors.z4Threshold);
      expect(getPowerZoneColor(5)).toBe(zoneColors.z5Vo2max);
      expect(getPowerZoneColor(6)).toBe(zoneColors.z6Anaerobic);
      expect(getPowerZoneColor(7)).toBe(zoneColors.z7Neuromuscular);
    });

    it('returns gray for unknown zones', () => {
      expect(getPowerZoneColor(0)).toBe('#808080');
      expect(getPowerZoneColor(8)).toBe('#808080');
    });
  });

  describe('getHrZoneColor', () => {
    it('returns correct color for each HR zone', () => {
      expect(getHrZoneColor(1)).toBe(zoneColors.z1Recovery);
      expect(getHrZoneColor(2)).toBe(zoneColors.z2Endurance);
      expect(getHrZoneColor(3)).toBe(zoneColors.z3Tempo);
      expect(getHrZoneColor(4)).toBe(zoneColors.z4Threshold);
      expect(getHrZoneColor(5)).toBe(zoneColors.z6Anaerobic); // Zone 5 uses anaerobic color
    });

    it('returns gray for unknown zones', () => {
      expect(getHrZoneColor(0)).toBe('#808080');
      expect(getHrZoneColor(6)).toBe('#808080');
    });
  });
});

describe('Theme Objects', () => {
  describe('darkTheme', () => {
    it('has dark mode set', () => {
      expect(darkTheme.mode).toBe('dark');
    });

    it('uses dark colors', () => {
      expect(darkTheme.colors).toBe(darkColors);
    });

    it('has typography defined', () => {
      expect(darkTheme.typography.textStyles).toBeDefined();
      expect(darkTheme.typography.fontSize).toBeDefined();
    });

    it('has spacing defined', () => {
      expect(darkTheme.spacing).toBeDefined();
      expect(darkTheme.spacing.lg).toBe(16);
    });

    it('has border radius defined', () => {
      expect(darkTheme.borderRadius).toBeDefined();
      expect(darkTheme.borderRadius.md).toBe(8);
    });

    it('has shadows defined', () => {
      expect(darkTheme.shadows).toBeDefined();
      expect(darkTheme.shadows.sm).toBeDefined();
    });
  });

  describe('lightTheme', () => {
    it('has light mode set', () => {
      expect(lightTheme.mode).toBe('light');
    });

    it('uses light colors', () => {
      expect(lightTheme.colors).toBe(lightColors);
    });

    it('has same structure as dark theme', () => {
      expect(Object.keys(lightTheme)).toEqual(Object.keys(darkTheme));
    });
  });
});

describe('getTheme', () => {
  it('returns dark theme for dark mode', () => {
    expect(getTheme('dark')).toBe(darkTheme);
  });

  it('returns light theme for light mode', () => {
    expect(getTheme('light')).toBe(lightTheme);
  });
});
