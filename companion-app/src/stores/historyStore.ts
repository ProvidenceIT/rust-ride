/**
 * History Store
 *
 * Manages ride history state with pagination, caching, and filtering.
 * Fetches ride data from the RustRide desktop app via WebSocket.
 */

import { create } from 'zustand';
import type { RideSummary, RideDetailInfo } from '@/types';

/**
 * Date range filter options
 */
export type DateRangeFilter = 'all' | 'week' | 'month' | 'year' | 'custom';

/**
 * Ride type filter options
 */
export type RideTypeFilter = 'all' | 'workout' | 'free_ride';

/**
 * Pagination info
 */
interface PaginationInfo {
  offset: number;
  limit: number;
  total: number;
  hasMore: boolean;
}

/**
 * Filter state
 */
interface FilterState {
  dateRange: DateRangeFilter;
  customStartDate: string | null;
  customEndDate: string | null;
  rideType: RideTypeFilter;
}

/**
 * History store state
 */
interface HistoryState {
  // Ride list
  rides: RideSummary[];
  pagination: PaginationInfo;

  // Loading states
  isLoading: boolean;
  isLoadingMore: boolean;
  error: string | null;

  // Filters
  filters: FilterState;

  // Detail cache (ride_id -> RideDetailInfo)
  rideDetailsCache: Map<string, RideDetailInfo>;
  currentRideDetail: RideDetailInfo | null;
  isLoadingDetail: boolean;

  // Last fetch timestamp for cache invalidation
  lastFetchedAt: number | null;
}

/**
 * History store actions
 */
interface HistoryActions {
  // Ride list management
  setRides: (rides: RideSummary[], total: number, append?: boolean) => void;
  clearRides: () => void;

  // Loading states
  setLoading: (isLoading: boolean) => void;
  setLoadingMore: (isLoadingMore: boolean) => void;
  setError: (error: string | null) => void;

  // Pagination
  loadNextPage: () => void;
  resetPagination: () => void;

  // Filters
  setDateRangeFilter: (range: DateRangeFilter, startDate?: string, endDate?: string) => void;
  setRideTypeFilter: (type: RideTypeFilter) => void;
  clearFilters: () => void;

  // Ride details
  setCurrentRideDetail: (detail: RideDetailInfo | null) => void;
  cacheRideDetail: (detail: RideDetailInfo) => void;
  getCachedRideDetail: (rideId: string) => RideDetailInfo | undefined;
  setLoadingDetail: (isLoading: boolean) => void;

  // Cache management
  clearCache: () => void;
  updateLastFetchedAt: () => void;

  // Reset store
  reset: () => void;
}

/**
 * Default page size for ride list
 */
const PAGE_SIZE = 20;

/**
 * Maximum cached ride details
 */
const MAX_CACHED_DETAILS = 50;

/**
 * Initial filter state
 */
const initialFilters: FilterState = {
  dateRange: 'all',
  customStartDate: null,
  customEndDate: null,
  rideType: 'all',
};

/**
 * Initial history state
 */
const initialState: HistoryState = {
  rides: [],
  pagination: {
    offset: 0,
    limit: PAGE_SIZE,
    total: 0,
    hasMore: false,
  },
  isLoading: false,
  isLoadingMore: false,
  error: null,
  filters: initialFilters,
  rideDetailsCache: new Map(),
  currentRideDetail: null,
  isLoadingDetail: false,
  lastFetchedAt: null,
};

/**
 * History store
 *
 * Manages ride history with pagination, filtering, and detail caching.
 */
export const useHistoryStore = create<HistoryState & HistoryActions>()((set, get) => ({
  ...initialState,

  // Ride list management
  setRides: (rides: RideSummary[], total: number, append = false) => {
    const state = get();
    const newRides = append ? [...state.rides, ...rides] : rides;
    const newOffset = append ? state.pagination.offset + rides.length : rides.length;

    set({
      rides: newRides,
      pagination: {
        ...state.pagination,
        offset: newOffset,
        total,
        hasMore: newOffset < total,
      },
      isLoading: false,
      isLoadingMore: false,
      error: null,
      lastFetchedAt: Date.now(),
    });
  },

  clearRides: () => {
    set({
      rides: [],
      pagination: {
        offset: 0,
        limit: PAGE_SIZE,
        total: 0,
        hasMore: false,
      },
    });
  },

  // Loading states
  setLoading: (isLoading: boolean) => {
    set({ isLoading });
  },

  setLoadingMore: (isLoadingMore: boolean) => {
    set({ isLoadingMore });
  },

  setError: (error: string | null) => {
    set({
      error,
      isLoading: false,
      isLoadingMore: false,
    });
  },

  // Pagination
  loadNextPage: () => {
    const state = get();
    if (state.pagination.hasMore && !state.isLoadingMore) {
      set({ isLoadingMore: true });
      // Note: Actual fetch is triggered by the component/service
    }
  },

  resetPagination: () => {
    set({
      pagination: {
        offset: 0,
        limit: PAGE_SIZE,
        total: 0,
        hasMore: false,
      },
    });
  },

  // Filters
  setDateRangeFilter: (range: DateRangeFilter, startDate?: string, endDate?: string) => {
    set({
      filters: {
        ...get().filters,
        dateRange: range,
        customStartDate: range === 'custom' ? startDate ?? null : null,
        customEndDate: range === 'custom' ? endDate ?? null : null,
      },
    });
  },

  setRideTypeFilter: (type: RideTypeFilter) => {
    set({
      filters: {
        ...get().filters,
        rideType: type,
      },
    });
  },

  clearFilters: () => {
    set({ filters: initialFilters });
  },

  // Ride details
  setCurrentRideDetail: (detail: RideDetailInfo | null) => {
    set({
      currentRideDetail: detail,
      isLoadingDetail: false,
    });

    // Also cache it if not null
    if (detail) {
      get().cacheRideDetail(detail);
    }
  },

  cacheRideDetail: (detail: RideDetailInfo) => {
    const cache = new Map(get().rideDetailsCache);

    // Enforce cache limit with LRU-like behavior (remove oldest entries)
    if (cache.size >= MAX_CACHED_DETAILS) {
      const keysIterator = cache.keys();
      const firstKey = keysIterator.next().value;
      if (firstKey !== undefined) {
        cache.delete(firstKey);
      }
    }

    cache.set(detail.ride_id, detail);
    set({ rideDetailsCache: cache });
  },

  getCachedRideDetail: (rideId: string): RideDetailInfo | undefined => {
    return get().rideDetailsCache.get(rideId);
  },

  setLoadingDetail: (isLoading: boolean) => {
    set({ isLoadingDetail: isLoading });
  },

  // Cache management
  clearCache: () => {
    set({
      rideDetailsCache: new Map(),
      currentRideDetail: null,
    });
  },

  updateLastFetchedAt: () => {
    set({ lastFetchedAt: Date.now() });
  },

  // Reset store
  reset: () => {
    set({
      ...initialState,
      rideDetailsCache: new Map(),
    });
  },
}));

// Selectors for optimized component subscriptions
export const selectRides = (state: HistoryState & HistoryActions) => state.rides;

export const selectIsLoading = (state: HistoryState & HistoryActions) => state.isLoading;

export const selectIsLoadingMore = (state: HistoryState & HistoryActions) => state.isLoadingMore;

export const selectError = (state: HistoryState & HistoryActions) => state.error;

export const selectHasMore = (state: HistoryState & HistoryActions) => state.pagination.hasMore;

export const selectTotal = (state: HistoryState & HistoryActions) => state.pagination.total;

export const selectFilters = (state: HistoryState & HistoryActions) => state.filters;

export const selectCurrentRideDetail = (state: HistoryState & HistoryActions) =>
  state.currentRideDetail;

export const selectIsLoadingDetail = (state: HistoryState & HistoryActions) =>
  state.isLoadingDetail;

export const selectPagination = (state: HistoryState & HistoryActions) => state.pagination;

export const selectRideCount = (state: HistoryState & HistoryActions) => state.rides.length;

export const selectIsEmpty = (state: HistoryState & HistoryActions) =>
  state.rides.length === 0 && !state.isLoading;

export const selectHasFiltersApplied = (state: HistoryState & HistoryActions): boolean =>
  state.filters.dateRange !== 'all' || state.filters.rideType !== 'all';

/**
 * Get date range for filter
 */
export function getFilterDateRange(filter: DateRangeFilter): {
  start: Date | null;
  end: Date | null;
} {
  const now = new Date();
  const end = now;

  switch (filter) {
    case 'week': {
      const start = new Date(now);
      start.setDate(start.getDate() - 7);
      return { start, end };
    }
    case 'month': {
      const start = new Date(now);
      start.setMonth(start.getMonth() - 1);
      return { start, end };
    }
    case 'year': {
      const start = new Date(now);
      start.setFullYear(start.getFullYear() - 1);
      return { start, end };
    }
    case 'all':
    case 'custom':
    default:
      return { start: null, end: null };
  }
}
