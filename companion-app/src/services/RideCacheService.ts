/**
 * RideCacheService - Offline Ride Data Cache Manager
 *
 * Handles persistent caching of ride data in AsyncStorage for offline access.
 * Features:
 * - Cache recently viewed ride summaries (up to 50)
 * - Cache ride details for offline viewing
 * - Sync status tracking for reconnection
 * - LRU-like cache eviction when limits exceeded
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import type { RideSummary, RideDetailInfo } from '@/types';

/**
 * Storage keys for ride cache
 */
const STORAGE_KEYS = {
  /** Cached ride summaries list */
  RIDE_SUMMARIES: '@rustride/cached_ride_summaries',
  /** Prefix for cached ride details */
  RIDE_DETAIL_PREFIX: '@rustride/cached_ride_detail_',
  /** Last sync timestamp */
  LAST_SYNC: '@rustride/last_sync',
  /** Cache metadata (for tracking) */
  CACHE_METADATA: '@rustride/cache_metadata',
} as const;

/**
 * Maximum number of ride summaries to cache
 */
const MAX_CACHED_SUMMARIES = 50;

/**
 * Maximum number of ride details to cache
 */
const MAX_CACHED_DETAILS = 20;

/**
 * Cache metadata for tracking cached items
 */
interface CacheMetadata {
  /** List of cached ride detail IDs (in order of access) */
  cachedDetailIds: string[];
  /** Last access timestamp for each detail */
  detailAccessTimes: Record<string, number>;
}

/**
 * Cached ride summary with cache timestamp
 */
export interface CachedRideSummary extends RideSummary {
  /** Timestamp when this item was cached */
  cachedAt: number;
}

/**
 * Cached ride detail with cache timestamp
 */
export interface CachedRideDetail extends RideDetailInfo {
  /** Timestamp when this item was cached */
  cachedAt: number;
}

/**
 * Cache sync status
 */
export interface CacheSyncStatus {
  /** Last successful sync timestamp */
  lastSyncAt: number | null;
  /** Number of cached summaries */
  cachedSummaryCount: number;
  /** Number of cached details */
  cachedDetailCount: number;
  /** Whether cache needs sync */
  needsSync: boolean;
}

/**
 * RideCacheService class
 *
 * Singleton service for caching ride data in AsyncStorage for offline access.
 */
export class RideCacheService {
  private static instance: RideCacheService | null = null;

  /** In-memory cache for quick access */
  private summariesCache: CachedRideSummary[] | null = null;
  private detailsCache: Map<string, CachedRideDetail> = new Map();
  private metadata: CacheMetadata | null = null;

  /**
   * Private constructor for singleton pattern
   */
  private constructor() {}

  /**
   * Get the singleton instance
   */
  public static getInstance(): RideCacheService {
    if (!RideCacheService.instance) {
      RideCacheService.instance = new RideCacheService();
    }
    return RideCacheService.instance;
  }

  // ===== Ride Summaries Caching =====

  /**
   * Cache ride summaries
   * Merges with existing cache, keeping most recent entries
   *
   * @param rides The rides to cache
   */
  public async cacheRideSummaries(rides: RideSummary[]): Promise<void> {
    try {
      const now = Date.now();
      const existingCache = await this.getCachedRideSummaries();

      // Create map of existing cached items by ID
      const existingMap = new Map<string, CachedRideSummary>();
      for (const ride of existingCache) {
        existingMap.set(ride.id, ride);
      }

      // Add/update rides with cache timestamp
      for (const ride of rides) {
        const existing = existingMap.get(ride.id);
        existingMap.set(ride.id, {
          ...ride,
          cachedAt: existing?.cachedAt ?? now,
        });
      }

      // Convert to array and sort by date (newest first)
      let allRides = Array.from(existingMap.values());
      allRides.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());

      // Limit to max cached summaries
      if (allRides.length > MAX_CACHED_SUMMARIES) {
        allRides = allRides.slice(0, MAX_CACHED_SUMMARIES);
      }

      // Save to AsyncStorage
      await AsyncStorage.setItem(STORAGE_KEYS.RIDE_SUMMARIES, JSON.stringify(allRides));

      // Update in-memory cache
      this.summariesCache = allRides;
    } catch {
      // Ignore cache write errors
    }
  }

  /**
   * Get cached ride summaries
   *
   * @returns Array of cached ride summaries
   */
  public async getCachedRideSummaries(): Promise<CachedRideSummary[]> {
    // Return in-memory cache if available
    if (this.summariesCache !== null) {
      return this.summariesCache;
    }

    try {
      const json = await AsyncStorage.getItem(STORAGE_KEYS.RIDE_SUMMARIES);
      if (!json) {
        this.summariesCache = [];
        return [];
      }

      const rides = JSON.parse(json) as CachedRideSummary[];
      this.summariesCache = rides;
      return rides;
    } catch {
      this.summariesCache = [];
      return [];
    }
  }

  /**
   * Check if ride summaries are cached
   */
  public async hasCachedSummaries(): Promise<boolean> {
    const summaries = await this.getCachedRideSummaries();
    return summaries.length > 0;
  }

  // ===== Ride Details Caching =====

  /**
   * Cache a ride detail
   *
   * @param detail The ride detail to cache
   */
  public async cacheRideDetail(detail: RideDetailInfo): Promise<void> {
    try {
      const now = Date.now();
      const cachedDetail: CachedRideDetail = {
        ...detail,
        cachedAt: now,
      };

      // Save to AsyncStorage
      const key = `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}${detail.ride_id}`;
      await AsyncStorage.setItem(key, JSON.stringify(cachedDetail));

      // Update in-memory cache
      this.detailsCache.set(detail.ride_id, cachedDetail);

      // Update metadata for LRU eviction
      await this.updateDetailMetadata(detail.ride_id, now);
    } catch {
      // Ignore cache write errors
    }
  }

  /**
   * Get cached ride detail
   *
   * @param rideId The ride ID to get
   * @returns The cached ride detail, or null if not cached
   */
  public async getCachedRideDetail(rideId: string): Promise<CachedRideDetail | null> {
    // Check in-memory cache first
    if (this.detailsCache.has(rideId)) {
      return this.detailsCache.get(rideId) ?? null;
    }

    try {
      const key = `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}${rideId}`;
      const json = await AsyncStorage.getItem(key);
      if (!json) {
        return null;
      }

      const detail = JSON.parse(json) as CachedRideDetail;

      // Update in-memory cache
      this.detailsCache.set(rideId, detail);

      return detail;
    } catch {
      return null;
    }
  }

  /**
   * Check if a ride detail is cached
   *
   * @param rideId The ride ID to check
   */
  public async hasRideDetail(rideId: string): Promise<boolean> {
    // Check in-memory cache first
    if (this.detailsCache.has(rideId)) {
      return true;
    }

    const detail = await this.getCachedRideDetail(rideId);
    return detail !== null;
  }

  /**
   * Update detail metadata for LRU tracking
   */
  private async updateDetailMetadata(rideId: string, accessTime: number): Promise<void> {
    try {
      const metadata = await this.getMetadata();

      // Update access time
      metadata.detailAccessTimes[rideId] = accessTime;

      // Update cached IDs list
      const idIndex = metadata.cachedDetailIds.indexOf(rideId);
      if (idIndex !== -1) {
        metadata.cachedDetailIds.splice(idIndex, 1);
      }
      metadata.cachedDetailIds.push(rideId);

      // Evict old entries if over limit
      while (metadata.cachedDetailIds.length > MAX_CACHED_DETAILS) {
        const oldestId = metadata.cachedDetailIds.shift();
        if (oldestId) {
          delete metadata.detailAccessTimes[oldestId];
          await this.removeRideDetail(oldestId);
        }
      }

      // Save metadata
      await AsyncStorage.setItem(STORAGE_KEYS.CACHE_METADATA, JSON.stringify(metadata));
      this.metadata = metadata;
    } catch {
      // Ignore metadata errors
    }
  }

  /**
   * Get cache metadata
   */
  private async getMetadata(): Promise<CacheMetadata> {
    if (this.metadata !== null) {
      return this.metadata;
    }

    try {
      const json = await AsyncStorage.getItem(STORAGE_KEYS.CACHE_METADATA);
      if (!json) {
        const defaultMetadata: CacheMetadata = {
          cachedDetailIds: [],
          detailAccessTimes: {},
        };
        this.metadata = defaultMetadata;
        return defaultMetadata;
      }

      const metadata = JSON.parse(json) as CacheMetadata;
      this.metadata = metadata;
      return metadata;
    } catch {
      const defaultMetadata: CacheMetadata = {
        cachedDetailIds: [],
        detailAccessTimes: {},
      };
      this.metadata = defaultMetadata;
      return defaultMetadata;
    }
  }

  /**
   * Remove a specific ride detail from cache
   */
  private async removeRideDetail(rideId: string): Promise<void> {
    try {
      const key = `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}${rideId}`;
      await AsyncStorage.removeItem(key);
      this.detailsCache.delete(rideId);
    } catch {
      // Ignore removal errors
    }
  }

  // ===== Sync Management =====

  /**
   * Update last sync timestamp
   */
  public async updateLastSync(): Promise<void> {
    try {
      await AsyncStorage.setItem(STORAGE_KEYS.LAST_SYNC, JSON.stringify(Date.now()));
    } catch {
      // Ignore sync timestamp errors
    }
  }

  /**
   * Get last sync timestamp
   */
  public async getLastSync(): Promise<number | null> {
    try {
      const json = await AsyncStorage.getItem(STORAGE_KEYS.LAST_SYNC);
      if (!json) {
        return null;
      }
      return JSON.parse(json) as number;
    } catch {
      return null;
    }
  }

  /**
   * Get cache sync status
   */
  public async getSyncStatus(): Promise<CacheSyncStatus> {
    const lastSyncAt = await this.getLastSync();
    const summaries = await this.getCachedRideSummaries();
    const metadata = await this.getMetadata();

    // Consider cache stale if last sync was more than 1 hour ago
    const oneHourAgo = Date.now() - 60 * 60 * 1000;
    const needsSync = lastSyncAt === null || lastSyncAt < oneHourAgo;

    return {
      lastSyncAt,
      cachedSummaryCount: summaries.length,
      cachedDetailCount: metadata.cachedDetailIds.length,
      needsSync,
    };
  }

  // ===== Cache Management =====

  /**
   * Clear all cached ride data
   */
  public async clearCache(): Promise<void> {
    try {
      // Get all cached detail IDs
      const metadata = await this.getMetadata();

      // Remove all detail entries
      const keysToRemove = [
        STORAGE_KEYS.RIDE_SUMMARIES,
        STORAGE_KEYS.LAST_SYNC,
        STORAGE_KEYS.CACHE_METADATA,
        ...metadata.cachedDetailIds.map(id => `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}${id}`),
      ];

      await AsyncStorage.multiRemove(keysToRemove);

      // Clear in-memory caches
      this.summariesCache = null;
      this.detailsCache.clear();
      this.metadata = null;
    } catch {
      // Ignore clear errors
    }
  }

  /**
   * Invalidate in-memory cache (force reload from storage)
   */
  public invalidateCache(): void {
    this.summariesCache = null;
    this.detailsCache.clear();
    this.metadata = null;
  }

  /**
   * Preload cache from storage into memory
   */
  public async preloadCache(): Promise<void> {
    await this.getCachedRideSummaries();
    await this.getMetadata();
  }
}

/**
 * Get the singleton RideCacheService instance
 */
export function getRideCacheService(): RideCacheService {
  return RideCacheService.getInstance();
}
