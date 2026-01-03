/**
 * StorageService - Persistent Storage Manager
 *
 * Handles persistent storage of user preferences and connection data
 * using AsyncStorage. Provides type-safe access to stored values.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import type { DiscoveredServer } from '@/types';

/**
 * Storage keys for persistent data
 */
const STORAGE_KEYS = {
  /** Last successfully connected server */
  LAST_SERVER: '@rustride/last_server',
  /** Saved PIN for auto-auth (encrypted in production) */
  SAVED_PIN: '@rustride/saved_pin',
  /** User preference for remembering PIN */
  REMEMBER_PIN: '@rustride/remember_pin',
  /** Auto-reconnect preference */
  AUTO_RECONNECT: '@rustride/auto_reconnect',
} as const;

/**
 * Stored server information
 */
export interface StoredServer {
  /** Server display name */
  name: string;
  /** Server host/IP address */
  host: string;
  /** Server port */
  port: number;
  /** Server version if known */
  version?: string;
  /** Timestamp of last successful connection */
  lastConnected: number;
}

/**
 * Connection preferences
 */
export interface ConnectionPreferences {
  /** Whether to auto-reconnect on app launch */
  autoReconnect: boolean;
  /** Whether to remember the PIN */
  rememberPin: boolean;
}

/**
 * StorageService class
 *
 * Singleton service for persistent storage of connection preferences
 * and server information using AsyncStorage.
 */
export class StorageService {
  private static instance: StorageService | null = null;

  /**
   * Private constructor for singleton pattern
   */
  private constructor() {}

  /**
   * Get the singleton instance
   */
  public static getInstance(): StorageService {
    if (!StorageService.instance) {
      StorageService.instance = new StorageService();
    }
    return StorageService.instance;
  }

  // ===== Server Storage =====

  /**
   * Save the last connected server
   *
   * @param server The server to save
   */
  public async saveLastServer(server: DiscoveredServer): Promise<void> {
    const storedServer: StoredServer = {
      name: server.name,
      host: server.host,
      port: server.port,
      version: server.version,
      lastConnected: Date.now(),
    };

    await AsyncStorage.setItem(STORAGE_KEYS.LAST_SERVER, JSON.stringify(storedServer));
  }

  /**
   * Get the last connected server
   *
   * @returns The last connected server, or null if none saved
   */
  public async getLastServer(): Promise<StoredServer | null> {
    try {
      const json = await AsyncStorage.getItem(STORAGE_KEYS.LAST_SERVER);
      if (!json) return null;

      const stored = JSON.parse(json) as StoredServer;

      // Validate required fields
      if (!stored.host || !stored.port) {
        return null;
      }

      return stored;
    } catch {
      return null;
    }
  }

  /**
   * Clear the last connected server
   */
  public async clearLastServer(): Promise<void> {
    await AsyncStorage.removeItem(STORAGE_KEYS.LAST_SERVER);
  }

  // ===== PIN Storage =====

  /**
   * Save the PIN for auto-authentication
   *
   * NOTE: In a production app, this should be stored securely using
   * react-native-keychain or similar secure storage solution.
   *
   * @param pin The PIN to save
   */
  public async savePin(pin: string): Promise<void> {
    // Only save if rememberPin preference is enabled
    const rememberPin = await this.getRememberPin();
    if (!rememberPin) {
      return;
    }

    await AsyncStorage.setItem(STORAGE_KEYS.SAVED_PIN, pin);
  }

  /**
   * Get the saved PIN
   *
   * @returns The saved PIN, or null if none saved
   */
  public async getSavedPin(): Promise<string | null> {
    try {
      const rememberPin = await this.getRememberPin();
      if (!rememberPin) {
        return null;
      }

      return await AsyncStorage.getItem(STORAGE_KEYS.SAVED_PIN);
    } catch {
      return null;
    }
  }

  /**
   * Clear the saved PIN
   */
  public async clearSavedPin(): Promise<void> {
    await AsyncStorage.removeItem(STORAGE_KEYS.SAVED_PIN);
  }

  // ===== Preferences =====

  /**
   * Set the auto-reconnect preference
   *
   * @param enabled Whether to auto-reconnect on app launch
   */
  public async setAutoReconnect(enabled: boolean): Promise<void> {
    await AsyncStorage.setItem(STORAGE_KEYS.AUTO_RECONNECT, JSON.stringify(enabled));
  }

  /**
   * Get the auto-reconnect preference
   *
   * @returns Whether auto-reconnect is enabled (defaults to true)
   */
  public async getAutoReconnect(): Promise<boolean> {
    try {
      const value = await AsyncStorage.getItem(STORAGE_KEYS.AUTO_RECONNECT);
      if (value === null) {
        // Default to true for better UX
        return true;
      }
      return JSON.parse(value) as boolean;
    } catch {
      return true;
    }
  }

  /**
   * Set the remember PIN preference
   *
   * @param enabled Whether to remember the PIN
   */
  public async setRememberPin(enabled: boolean): Promise<void> {
    await AsyncStorage.setItem(STORAGE_KEYS.REMEMBER_PIN, JSON.stringify(enabled));

    // If disabling remember PIN, also clear any saved PIN
    if (!enabled) {
      await this.clearSavedPin();
    }
  }

  /**
   * Get the remember PIN preference
   *
   * @returns Whether remember PIN is enabled (defaults to false for security)
   */
  public async getRememberPin(): Promise<boolean> {
    try {
      const value = await AsyncStorage.getItem(STORAGE_KEYS.REMEMBER_PIN);
      if (value === null) {
        // Default to false for security
        return false;
      }
      return JSON.parse(value) as boolean;
    } catch {
      return false;
    }
  }

  /**
   * Get all connection preferences
   *
   * @returns The current connection preferences
   */
  public async getPreferences(): Promise<ConnectionPreferences> {
    const [autoReconnect, rememberPin] = await Promise.all([
      this.getAutoReconnect(),
      this.getRememberPin(),
    ]);

    return {
      autoReconnect,
      rememberPin,
    };
  }

  // ===== Utility Methods =====

  /**
   * Clear all stored connection data
   */
  public async clearAll(): Promise<void> {
    await AsyncStorage.multiRemove([
      STORAGE_KEYS.LAST_SERVER,
      STORAGE_KEYS.SAVED_PIN,
      STORAGE_KEYS.REMEMBER_PIN,
      STORAGE_KEYS.AUTO_RECONNECT,
    ]);
  }

  /**
   * Check if there's a server available for auto-reconnect
   *
   * @returns Whether auto-reconnect can be attempted
   */
  public async canAutoReconnect(): Promise<boolean> {
    const [autoReconnect, lastServer] = await Promise.all([
      this.getAutoReconnect(),
      this.getLastServer(),
    ]);

    return autoReconnect && lastServer !== null;
  }
}

/**
 * Get the singleton StorageService instance
 */
export function getStorageService(): StorageService {
  return StorageService.getInstance();
}
