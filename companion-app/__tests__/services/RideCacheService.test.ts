/**
 * RideCacheService Unit Tests
 *
 * Tests for offline ride data caching using AsyncStorage
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  RideCacheService,
  getRideCacheService,
  type CachedRideSummary,
  type CachedRideDetail,
} from '../../src/services/RideCacheService';
import type { RideSummary, RideDetailInfo } from '../../src/types';

// Storage keys matching the service
const STORAGE_KEYS = {
  RIDE_SUMMARIES: '@rustride/cached_ride_summaries',
  RIDE_DETAIL_PREFIX: '@rustride/cached_ride_detail_',
  LAST_SYNC: '@rustride/last_sync',
  CACHE_METADATA: '@rustride/cache_metadata',
};

// Mock ride data
const mockRideSummary: RideSummary = {
  id: 'ride-1',
  date: '2024-01-15T10:30:00Z',
  duration_secs: 3600,
  distance_km: 25.5,
  avg_power_watts: 180,
  workout_name: 'Tempo Intervals',
  is_workout: true,
};

const mockRideSummary2: RideSummary = {
  id: 'ride-2',
  date: '2024-01-14T09:00:00Z',
  duration_secs: 5400,
  distance_km: 40.2,
  avg_power_watts: 150,
};

const mockRideDetail: RideDetailInfo = {
  ride_id: 'ride-1',
  started_at: '2024-01-15T10:30:00Z',
  ended_at: '2024-01-15T11:30:00Z',
  duration_secs: 3600,
  distance_km: 25.5,
  calories: 600,
  avg_power_watts: 180,
  max_power_watts: 320,
  normalized_power_watts: 195,
  avg_heart_rate_bpm: 145,
  max_heart_rate_bpm: 175,
  avg_cadence_rpm: 85,
  tss: 65,
  intensity_factor: 0.78,
  is_workout: true,
  workout_name: 'Tempo Intervals',
};

describe('RideCacheService', () => {
  let service: RideCacheService;

  beforeEach(async () => {
    // Clear AsyncStorage mock
    await AsyncStorage.clear();

    // Reset singleton instance
    (RideCacheService as unknown as { instance: RideCacheService | null }).instance = null;
    service = getRideCacheService();
    service.invalidateCache();
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('getInstance', () => {
    it('should return singleton instance', () => {
      const instance1 = RideCacheService.getInstance();
      const instance2 = RideCacheService.getInstance();
      expect(instance1).toBe(instance2);
    });
  });

  describe('cacheRideSummaries', () => {
    it('should cache ride summaries to AsyncStorage', async () => {
      await service.cacheRideSummaries([mockRideSummary, mockRideSummary2]);

      const stored = await AsyncStorage.getItem(STORAGE_KEYS.RIDE_SUMMARIES);
      expect(stored).not.toBeNull();

      const parsed = JSON.parse(stored!) as CachedRideSummary[];
      expect(parsed).toHaveLength(2);
      expect(parsed[0].id).toBe('ride-1');
      expect(parsed[1].id).toBe('ride-2');
    });

    it('should add cachedAt timestamp to rides', async () => {
      const before = Date.now();
      await service.cacheRideSummaries([mockRideSummary]);
      const after = Date.now();

      const stored = await AsyncStorage.getItem(STORAGE_KEYS.RIDE_SUMMARIES);
      const parsed = JSON.parse(stored!) as CachedRideSummary[];

      expect(parsed[0].cachedAt).toBeGreaterThanOrEqual(before);
      expect(parsed[0].cachedAt).toBeLessThanOrEqual(after);
    });

    it('should merge with existing cache', async () => {
      // Cache first ride
      await service.cacheRideSummaries([mockRideSummary]);

      // Cache second ride
      await service.cacheRideSummaries([mockRideSummary2]);

      const cached = await service.getCachedRideSummaries();
      expect(cached).toHaveLength(2);
    });

    it('should update existing rides in cache', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      // Update with modified ride
      const updatedRide = { ...mockRideSummary, avg_power_watts: 200 };
      await service.cacheRideSummaries([updatedRide]);

      const cached = await service.getCachedRideSummaries();
      expect(cached).toHaveLength(1);
      expect(cached[0].avg_power_watts).toBe(200);
    });

    it('should sort rides by date (newest first)', async () => {
      await service.cacheRideSummaries([mockRideSummary2, mockRideSummary]);

      const cached = await service.getCachedRideSummaries();
      // mockRideSummary is newer (Jan 15) than mockRideSummary2 (Jan 14)
      expect(cached[0].id).toBe('ride-1');
      expect(cached[1].id).toBe('ride-2');
    });

    it('should limit cache to 50 entries', async () => {
      const manyRides: RideSummary[] = [];
      for (let i = 0; i < 60; i++) {
        manyRides.push({
          id: `ride-${i}`,
          date: new Date(Date.now() - i * 86400000).toISOString(),
          duration_secs: 3600,
          distance_km: 25,
          avg_power_watts: 180,
        });
      }

      await service.cacheRideSummaries(manyRides);

      const cached = await service.getCachedRideSummaries();
      expect(cached).toHaveLength(50);
    });
  });

  describe('getCachedRideSummaries', () => {
    it('should return empty array when no cache exists', async () => {
      const cached = await service.getCachedRideSummaries();
      expect(cached).toEqual([]);
    });

    it('should return cached rides', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      const cached = await service.getCachedRideSummaries();
      expect(cached).toHaveLength(1);
      expect(cached[0].id).toBe('ride-1');
    });

    it('should use in-memory cache on subsequent calls', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      // First call loads from storage
      await service.getCachedRideSummaries();

      // Clear storage to verify in-memory cache is used
      await AsyncStorage.removeItem(STORAGE_KEYS.RIDE_SUMMARIES);

      // Should still return cached data from memory
      const cached = await service.getCachedRideSummaries();
      expect(cached).toHaveLength(1);
    });
  });

  describe('hasCachedSummaries', () => {
    it('should return false when no cache exists', async () => {
      const hasCached = await service.hasCachedSummaries();
      expect(hasCached).toBe(false);
    });

    it('should return true when cache exists', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      const hasCached = await service.hasCachedSummaries();
      expect(hasCached).toBe(true);
    });
  });

  describe('cacheRideDetail', () => {
    it('should cache ride detail to AsyncStorage', async () => {
      await service.cacheRideDetail(mockRideDetail);

      const key = `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}${mockRideDetail.ride_id}`;
      const stored = await AsyncStorage.getItem(key);
      expect(stored).not.toBeNull();

      const parsed = JSON.parse(stored!) as CachedRideDetail;
      expect(parsed.ride_id).toBe('ride-1');
      expect(parsed.cachedAt).toBeDefined();
    });

    it('should update in-memory cache', async () => {
      await service.cacheRideDetail(mockRideDetail);

      // Should be immediately available
      const cached = await service.getCachedRideDetail('ride-1');
      expect(cached).not.toBeNull();
      expect(cached!.ride_id).toBe('ride-1');
    });
  });

  describe('getCachedRideDetail', () => {
    it('should return null for non-cached ride', async () => {
      const cached = await service.getCachedRideDetail('non-existent');
      expect(cached).toBeNull();
    });

    it('should return cached ride detail', async () => {
      await service.cacheRideDetail(mockRideDetail);

      const cached = await service.getCachedRideDetail('ride-1');
      expect(cached).not.toBeNull();
      expect(cached!.ride_id).toBe('ride-1');
      expect(cached!.avg_power_watts).toBe(180);
    });

    it('should use in-memory cache on subsequent calls', async () => {
      await service.cacheRideDetail(mockRideDetail);

      // First call
      await service.getCachedRideDetail('ride-1');

      // Clear storage
      const key = `${STORAGE_KEYS.RIDE_DETAIL_PREFIX}ride-1`;
      await AsyncStorage.removeItem(key);

      // Should still return from memory
      const cached = await service.getCachedRideDetail('ride-1');
      expect(cached).not.toBeNull();
    });
  });

  describe('hasRideDetail', () => {
    it('should return false for non-cached ride', async () => {
      const hasDetail = await service.hasRideDetail('non-existent');
      expect(hasDetail).toBe(false);
    });

    it('should return true for cached ride', async () => {
      await service.cacheRideDetail(mockRideDetail);

      const hasDetail = await service.hasRideDetail('ride-1');
      expect(hasDetail).toBe(true);
    });
  });

  describe('sync management', () => {
    it('should update last sync timestamp', async () => {
      const before = Date.now();
      await service.updateLastSync();
      const after = Date.now();

      const lastSync = await service.getLastSync();
      expect(lastSync).toBeGreaterThanOrEqual(before);
      expect(lastSync).toBeLessThanOrEqual(after);
    });

    it('should return null when no sync timestamp exists', async () => {
      const lastSync = await service.getLastSync();
      expect(lastSync).toBeNull();
    });

    it('should return sync status', async () => {
      await service.cacheRideSummaries([mockRideSummary]);
      await service.cacheRideDetail(mockRideDetail);
      await service.updateLastSync();

      const status = await service.getSyncStatus();

      expect(status.lastSyncAt).not.toBeNull();
      expect(status.cachedSummaryCount).toBe(1);
      expect(status.cachedDetailCount).toBe(1);
      expect(status.needsSync).toBe(false);
    });

    it('should indicate needs sync when last sync is old', async () => {
      // Set old sync timestamp (2 hours ago)
      const oldTime = Date.now() - 2 * 60 * 60 * 1000;
      await AsyncStorage.setItem(STORAGE_KEYS.LAST_SYNC, JSON.stringify(oldTime));

      service.invalidateCache();
      const status = await service.getSyncStatus();

      expect(status.needsSync).toBe(true);
    });
  });

  describe('clearCache', () => {
    it('should clear all cached data', async () => {
      await service.cacheRideSummaries([mockRideSummary]);
      await service.cacheRideDetail(mockRideDetail);
      await service.updateLastSync();

      await service.clearCache();

      const summaries = await service.getCachedRideSummaries();
      const detail = await service.getCachedRideDetail('ride-1');
      const lastSync = await service.getLastSync();

      expect(summaries).toEqual([]);
      expect(detail).toBeNull();
      expect(lastSync).toBeNull();
    });
  });

  describe('invalidateCache', () => {
    it('should force reload from storage on next access', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      // Load into memory
      await service.getCachedRideSummaries();

      // Invalidate
      service.invalidateCache();

      // Modify storage directly
      await AsyncStorage.setItem(
        STORAGE_KEYS.RIDE_SUMMARIES,
        JSON.stringify([{ ...mockRideSummary, id: 'ride-modified', cachedAt: Date.now() }])
      );

      // Should reload from storage
      const cached = await service.getCachedRideSummaries();
      expect(cached[0].id).toBe('ride-modified');
    });
  });

  describe('preloadCache', () => {
    it('should preload cache into memory', async () => {
      await service.cacheRideSummaries([mockRideSummary]);

      // Reset singleton to clear in-memory cache
      (RideCacheService as unknown as { instance: RideCacheService | null }).instance = null;
      const newService = getRideCacheService();

      // Preload
      await newService.preloadCache();

      // Clear storage
      await AsyncStorage.removeItem(STORAGE_KEYS.RIDE_SUMMARIES);

      // Should still have data from memory
      const cached = await newService.getCachedRideSummaries();
      expect(cached).toHaveLength(1);
    });
  });

  describe('LRU eviction for details', () => {
    it('should evict oldest details when limit exceeded', async () => {
      // Cache 25 ride details (limit is 20)
      for (let i = 0; i < 25; i++) {
        await service.cacheRideDetail({
          ...mockRideDetail,
          ride_id: `ride-${i}`,
        });
      }

      // First 5 should be evicted (oldest accessed)
      for (let i = 0; i < 5; i++) {
        const cached = await service.getCachedRideDetail(`ride-${i}`);
        expect(cached).toBeNull();
      }

      // Last 20 should still be cached
      for (let i = 5; i < 25; i++) {
        const cached = await service.getCachedRideDetail(`ride-${i}`);
        expect(cached).not.toBeNull();
      }
    });
  });
});
