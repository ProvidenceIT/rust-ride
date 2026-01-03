/**
 * Deep Linking Configuration
 *
 * Configures URL schemes and deep links for the app.
 * Supports linking to specific screens via rustride:// URLs.
 */

import type { LinkingOptions } from '@react-navigation/native';
import type { RootStackParamList } from './types';

/**
 * Deep linking configuration for React Navigation.
 *
 * URL patterns:
 * - rustride://dashboard - Opens Dashboard tab
 * - rustride://workout - Opens Workout tab
 * - rustride://history - Opens History tab
 * - rustride://history/ride/:rideId - Opens specific ride detail
 * - rustride://settings - Opens Settings tab
 * - rustride://connect - Opens connection screen
 * - rustride://connect?url=ws://host:port&pin=123456 - Auto-connect with params
 */
export const linking: LinkingOptions<RootStackParamList> = {
  prefixes: ['rustride://', 'https://rustride.app'],
  config: {
    screens: {
      Main: {
        screens: {
          Dashboard: 'dashboard',
          Workout: 'workout',
          History: 'history',
          Settings: 'settings',
        },
      },
      RideDetail: 'history/ride/:rideId',
      Connection: {
        path: 'connect',
        parse: {
          // Parse query params for auto-connection
          // e.g., rustride://connect?url=ws://192.168.1.100:9876&pin=123456
        },
      },
    },
  },
};

/**
 * Get deep link for a specific route.
 * Useful for generating shareable links.
 */
export function getDeepLink(path: string): string {
  return `rustride://${path}`;
}

/**
 * Parse connection params from a deep link URL.
 * Returns null if the URL doesn't contain connection params.
 *
 * Uses simple string parsing since React Native doesn't have full URL API.
 */
export function parseConnectionParams(url: string): { url: string; pin?: string } | null {
  try {
    // Check if this is a connect URL
    if (!url.includes('connect')) {
      return null;
    }

    // Extract query string
    const queryIndex = url.indexOf('?');
    if (queryIndex === -1) {
      return null;
    }

    const queryString = url.slice(queryIndex + 1);
    const params: Record<string, string> = {};

    // Parse query parameters
    queryString.split('&').forEach((pair) => {
      const [key, value] = pair.split('=');
      if (key && value) {
        params[decodeURIComponent(key)] = decodeURIComponent(value);
      }
    });

    const serverUrl = params['url'];
    if (!serverUrl) {
      return null;
    }

    const pin = params['pin'];

    return { url: serverUrl, pin };
  } catch {
    return null;
  }
}
