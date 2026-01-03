/**
 * RustRide Companion App - Color Palette
 *
 * Color definitions matching the desktop app's theme.
 * See src/ui/theme.rs for the Rust desktop color definitions.
 */

/**
 * Dark theme colors matching desktop DarkTheme
 */
export const darkColors = {
  // Backgrounds
  background: '#121218', // rgb(18, 18, 24)
  surface: '#1C1C24', // rgb(28, 28, 36) - Panel background
  card: '#262630', // rgb(38, 38, 48) - Card background
  elevated: '#323240', // Slightly elevated surfaces

  // Text
  textPrimary: '#F0F0F5', // rgb(240, 240, 245)
  textSecondary: '#A0A0AA', // rgb(160, 160, 170)
  textMuted: '#64646E', // rgb(100, 100, 110)
  textInverse: '#121218', // For text on light backgrounds

  // Brand & Accent
  accent: '#4285F4', // rgb(66, 133, 244) - Blue
  accentLight: '#6BA1F7', // Lighter accent for hover/active states
  accentDark: '#2A6BCF', // Darker accent for pressed states

  // Semantic colors
  success: '#34A853', // rgb(52, 168, 83) - Green
  warning: '#FBBC04', // rgb(251, 188, 4) - Yellow/Orange
  error: '#EA4335', // rgb(234, 67, 53) - Red
  info: '#4285F4', // Same as accent

  // Borders
  border: '#3C3C46', // rgb(60, 60, 70)
  borderLight: '#4A4A58', // Lighter border for hover
  borderFocus: '#4285F4', // Accent color for focus state

  // Overlays
  overlay: 'rgba(0, 0, 0, 0.5)', // Modal overlays
  overlayLight: 'rgba(0, 0, 0, 0.3)',

  // Tab bar
  tabBarBackground: '#1C1C24',
  tabBarBorder: '#3C3C46',
  tabBarActive: '#4285F4',
  tabBarInactive: '#A0A0AA',
} as const;

/**
 * Light theme colors matching desktop LightTheme
 */
export const lightColors = {
  // Backgrounds
  background: '#FAFAFC', // rgb(250, 250, 252)
  surface: '#FFFFFF', // rgb(255, 255, 255) - Panel background
  card: '#F5F5F8', // rgb(245, 245, 248) - Card background
  elevated: '#FFFFFF', // Elevated surfaces

  // Text
  textPrimary: '#202028', // rgb(32, 32, 40)
  textSecondary: '#606068', // rgb(96, 96, 104)
  textMuted: '#909098', // rgb(144, 144, 152)
  textInverse: '#FFFFFF', // For text on dark backgrounds

  // Brand & Accent
  accent: '#1A73E8', // rgb(26, 115, 232) - Blue
  accentLight: '#4A90EC', // Lighter accent for hover states
  accentDark: '#1557B0', // Darker accent for pressed states

  // Semantic colors
  success: '#188038', // rgb(24, 128, 56) - Green
  warning: '#EAA000', // rgb(234, 160, 0) - Yellow/Orange
  error: '#C83228', // rgb(200, 50, 40) - Red
  info: '#1A73E8', // Same as accent

  // Borders
  border: '#DADAE0', // rgb(218, 218, 224)
  borderLight: '#E8E8EC', // Lighter border
  borderFocus: '#1A73E8', // Accent color for focus state

  // Overlays
  overlay: 'rgba(0, 0, 0, 0.4)', // Modal overlays
  overlayLight: 'rgba(0, 0, 0, 0.2)',

  // Tab bar
  tabBarBackground: '#FFFFFF',
  tabBarBorder: '#DADAE0',
  tabBarActive: '#1A73E8',
  tabBarInactive: '#606068',
} as const;

/**
 * Power zone colors (consistent across light/dark modes)
 * Matches zone_colors from src/ui/theme.rs
 */
export const zoneColors = {
  z1Recovery: '#808080', // rgb(128, 128, 128) - Gray
  z2Endurance: '#0080FF', // rgb(0, 128, 255) - Blue
  z3Tempo: '#00C864', // rgb(0, 200, 100) - Green
  z4Threshold: '#FFC800', // rgb(255, 200, 0) - Yellow
  z5Vo2max: '#FF8000', // rgb(255, 128, 0) - Orange
  z6Anaerobic: '#FF3232', // rgb(255, 50, 50) - Red
  z7Neuromuscular: '#B400B4', // rgb(180, 0, 180) - Purple
} as const;

/**
 * Heart rate zone colors (subset of power zones)
 */
export const hrZoneColors = {
  z1: zoneColors.z1Recovery,
  z2: zoneColors.z2Endurance,
  z3: zoneColors.z3Tempo,
  z4: zoneColors.z4Threshold,
  z5: zoneColors.z6Anaerobic, // Red for max effort
} as const;

/**
 * Get power zone color by zone number (1-7)
 */
export function getPowerZoneColor(zone: number): string {
  switch (zone) {
    case 1:
      return zoneColors.z1Recovery;
    case 2:
      return zoneColors.z2Endurance;
    case 3:
      return zoneColors.z3Tempo;
    case 4:
      return zoneColors.z4Threshold;
    case 5:
      return zoneColors.z5Vo2max;
    case 6:
      return zoneColors.z6Anaerobic;
    case 7:
      return zoneColors.z7Neuromuscular;
    default:
      return '#808080'; // Gray for unknown zones
  }
}

/**
 * Get heart rate zone color by zone number (1-5)
 */
export function getHrZoneColor(zone: number): string {
  switch (zone) {
    case 1:
      return hrZoneColors.z1;
    case 2:
      return hrZoneColors.z2;
    case 3:
      return hrZoneColors.z3;
    case 4:
      return hrZoneColors.z4;
    case 5:
      return hrZoneColors.z5;
    default:
      return '#808080'; // Gray for unknown zones
  }
}

/**
 * Type for theme color palette
 * Using interface with string values for flexibility between themes
 */
export interface ThemeColors {
  // Backgrounds
  background: string;
  surface: string;
  card: string;
  elevated: string;

  // Text
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  textInverse: string;

  // Brand & Accent
  accent: string;
  accentLight: string;
  accentDark: string;

  // Semantic colors
  success: string;
  warning: string;
  error: string;
  info: string;

  // Borders
  border: string;
  borderLight: string;
  borderFocus: string;

  // Overlays
  overlay: string;
  overlayLight: string;

  // Tab bar
  tabBarBackground: string;
  tabBarBorder: string;
  tabBarActive: string;
  tabBarInactive: string;
}

/**
 * Colors type re-export
 */
export type Colors = ThemeColors;
