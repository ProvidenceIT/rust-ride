/**
 * Type declarations for @react-native-async-storage/async-storage
 *
 * This file provides type declarations for the AsyncStorage package.
 * The actual package should be installed when running the app.
 */

declare module '@react-native-async-storage/async-storage' {
  /**
   * AsyncStorage is a simple, unencrypted, asynchronous, persistent,
   * key-value storage system that is global to the app.
   */
  interface AsyncStorageStatic {
    /**
     * Get a value from storage.
     * @param key - Key for the value to retrieve
     * @returns Promise with the value or null if not found
     */
    getItem(key: string): Promise<string | null>;

    /**
     * Set a value in storage.
     * @param key - Key for the value
     * @param value - The value to store
     * @returns Promise that resolves when complete
     */
    setItem(key: string, value: string): Promise<void>;

    /**
     * Remove a value from storage.
     * @param key - Key for the value to remove
     * @returns Promise that resolves when complete
     */
    removeItem(key: string): Promise<void>;

    /**
     * Remove multiple values from storage.
     * @param keys - Array of keys to remove
     * @returns Promise that resolves when complete
     */
    multiRemove(keys: readonly string[]): Promise<void>;

    /**
     * Get multiple values from storage.
     * @param keys - Array of keys to retrieve
     * @returns Promise with array of key-value pairs
     */
    multiGet(keys: readonly string[]): Promise<readonly [string, string | null][]>;

    /**
     * Set multiple values in storage.
     * @param keyValuePairs - Array of key-value pairs
     * @returns Promise that resolves when complete
     */
    multiSet(keyValuePairs: readonly [string, string][]): Promise<void>;

    /**
     * Clear all values from storage.
     * @returns Promise that resolves when complete
     */
    clear(): Promise<void>;

    /**
     * Get all keys in storage.
     * @returns Promise with array of all keys
     */
    getAllKeys(): Promise<readonly string[]>;
  }

  const AsyncStorage: AsyncStorageStatic;
  export default AsyncStorage;
}
