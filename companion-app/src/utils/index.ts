/**
 * RustRide Companion App - Utility Functions
 */

/**
 * Format duration in seconds to HH:MM:SS string
 */
export function formatDuration(seconds: number): string {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Format distance based on unit preference
 */
export function formatDistance(km: number, units: 'metric' | 'imperial'): string {
  if (units === 'imperial') {
    const miles = km * 0.621371;
    return `${miles.toFixed(1)} mi`;
  }
  return `${km.toFixed(1)} km`;
}

/**
 * Format speed based on unit preference
 */
export function formatSpeed(kph: number, units: 'metric' | 'imperial'): string {
  if (units === 'imperial') {
    const mph = kph * 0.621371;
    return `${mph.toFixed(1)} mph`;
  }
  return `${kph.toFixed(1)} km/h`;
}

/**
 * Format power with optional target comparison
 */
export function formatPower(watts: number, target?: number): string {
  if (target) {
    const diff = watts - target;
    const sign = diff >= 0 ? '+' : '';
    return `${watts}W (${sign}${diff})`;
  }
  return `${watts}W`;
}

/**
 * Clamp a value between min and max
 */
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
