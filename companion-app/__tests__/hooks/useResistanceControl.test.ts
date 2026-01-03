/**
 * useResistanceControl Hook Tests
 */

import { renderHook, act } from '@testing-library/react-native';
import { useResistanceControl } from '../../src/hooks/useResistanceControl';
import { useSessionStore } from '../../src/stores/sessionStore';
import { getConnectionService } from '../../src/services/ConnectionService';

// Define the mock service interface
interface MockConnectionService {
  adjustResistance: jest.Mock;
}

// Create the mock service
const mockService: MockConnectionService = {
  adjustResistance: jest.fn(),
};

// Mock the ConnectionService module
jest.mock('../../src/services/ConnectionService', () => ({
  ConnectionService: jest.fn(() => mockService),
  getConnectionService: jest.fn(() => mockService),
}));

describe('useResistanceControl', () => {
  let mockConnectionService: MockConnectionService;

  beforeEach(() => {
    // Reset session store
    useSessionStore.getState().reset();

    // Get mock service
    mockConnectionService = getConnectionService() as unknown as MockConnectionService;

    // Reset all mocks
    jest.clearAllMocks();
  });

  describe('initial state', () => {
    it('should have correct initial state', () => {
      const { result } = renderHook(() => useResistanceControl());

      expect(result.current.resistanceLevel).toBe(0);
      expect(result.current.isFreeRide).toBe(false);
      expect(result.current.canAdjust).toBe(false);
      expect(result.current.isLoading).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.stepSize).toBe(5);
      expect(result.current.canIncrease).toBe(false);
      expect(result.current.canDecrease).toBe(false);
    });

    it('should allow custom step size', () => {
      const { result } = renderHook(() => useResistanceControl(10));

      expect(result.current.stepSize).toBe(10);
    });
  });

  describe('free ride session', () => {
    beforeEach(() => {
      // Set up an active free ride session
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should enable adjustments during free ride', () => {
      const { result } = renderHook(() => useResistanceControl());

      expect(result.current.isFreeRide).toBe(true);
      expect(result.current.canAdjust).toBe(true);
      expect(result.current.canIncrease).toBe(true);
      expect(result.current.canDecrease).toBe(true);
    });

    it('should disable adjustments during workout', () => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 100,
      });

      const { result } = renderHook(() => useResistanceControl());

      expect(result.current.isFreeRide).toBe(false);
      expect(result.current.canAdjust).toBe(false);
    });
  });

  describe('increaseResistance', () => {
    beforeEach(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should call adjustResistance with positive delta', async () => {
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.increaseResistance();
      });

      expect(mockConnectionService.adjustResistance).toHaveBeenCalledWith(5);
    });

    it('should optimistically update resistance level', async () => {
      let resolveAdjust: () => void;
      mockConnectionService.adjustResistance.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveAdjust = resolve; }),
      );

      const { result } = renderHook(() => useResistanceControl());

      // Start increase
      let increasePromise: Promise<void>;
      act(() => {
        increasePromise = result.current.increaseResistance();
      });

      // Check optimistic update
      expect(useSessionStore.getState().resistanceLevel).toBe(5);
      expect(result.current.isLoading).toBe(true);

      // Complete
      await act(async () => {
        resolveAdjust!();
        await increasePromise;
      });

      expect(result.current.isLoading).toBe(false);
    });

    it('should rollback on error', async () => {
      mockConnectionService.adjustResistance.mockRejectedValueOnce(new Error('Failed'));

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.increaseResistance();
      });

      // State should be rolled back
      expect(useSessionStore.getState().resistanceLevel).toBe(0);
      expect(result.current.error).toBe('Failed');
      expect(result.current.isLoading).toBe(false);
    });

    it('should not increase beyond max (100)', async () => {
      // Set resistance near max
      useSessionStore.getState().setResistanceLevel(98);
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.increaseResistance();
      });

      // Should be clamped to 100
      expect(useSessionStore.getState().resistanceLevel).toBe(100);
    });

    it('should disable canIncrease at max', () => {
      useSessionStore.getState().setResistanceLevel(100);

      const { result } = renderHook(() => useResistanceControl());

      expect(result.current.canIncrease).toBe(false);
      expect(result.current.canDecrease).toBe(true);
    });

    it('should not call API when cannot increase', async () => {
      useSessionStore.getState().setResistanceLevel(100);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.increaseResistance();
      });

      expect(mockConnectionService.adjustResistance).not.toHaveBeenCalled();
    });
  });

  describe('decreaseResistance', () => {
    beforeEach(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should call adjustResistance with negative delta', async () => {
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.decreaseResistance();
      });

      expect(mockConnectionService.adjustResistance).toHaveBeenCalledWith(-5);
    });

    it('should optimistically update resistance level', async () => {
      let resolveAdjust: () => void;
      mockConnectionService.adjustResistance.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveAdjust = resolve; }),
      );

      const { result } = renderHook(() => useResistanceControl());

      // Start decrease
      let decreasePromise: Promise<void>;
      act(() => {
        decreasePromise = result.current.decreaseResistance();
      });

      // Check optimistic update
      expect(useSessionStore.getState().resistanceLevel).toBe(-5);
      expect(result.current.isLoading).toBe(true);

      // Complete
      await act(async () => {
        resolveAdjust!();
        await decreasePromise;
      });

      expect(result.current.isLoading).toBe(false);
    });

    it('should not decrease beyond min (-100)', async () => {
      // Set resistance near min
      useSessionStore.getState().setResistanceLevel(-98);
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.decreaseResistance();
      });

      // Should be clamped to -100
      expect(useSessionStore.getState().resistanceLevel).toBe(-100);
    });

    it('should disable canDecrease at min', () => {
      useSessionStore.getState().setResistanceLevel(-100);

      const { result } = renderHook(() => useResistanceControl());

      expect(result.current.canDecrease).toBe(false);
      expect(result.current.canIncrease).toBe(true);
    });

    it('should not call API when cannot decrease', async () => {
      useSessionStore.getState().setResistanceLevel(-100);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.decreaseResistance();
      });

      expect(mockConnectionService.adjustResistance).not.toHaveBeenCalled();
    });
  });

  describe('adjustResistance', () => {
    beforeEach(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should handle custom delta values', async () => {
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.adjustResistance(10);
      });

      expect(mockConnectionService.adjustResistance).toHaveBeenCalledWith(10);
      expect(useSessionStore.getState().resistanceLevel).toBe(10);
    });

    it('should not adjust when no active session', async () => {
      useSessionStore.getState().reset();

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.adjustResistance(5);
      });

      expect(mockConnectionService.adjustResistance).not.toHaveBeenCalled();
    });
  });

  describe('clearError', () => {
    it('should clear error state', async () => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });

      mockConnectionService.adjustResistance.mockRejectedValueOnce(new Error('Test error'));

      const { result } = renderHook(() => useResistanceControl());

      await act(async () => {
        await result.current.increaseResistance();
      });

      expect(result.current.error).toBe('Test error');

      act(() => {
        result.current.clearError();
      });

      expect(result.current.error).toBeNull();
    });
  });

  describe('custom step size', () => {
    beforeEach(() => {
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'free_ride',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should use custom step size for increase', async () => {
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl(10));

      await act(async () => {
        await result.current.increaseResistance();
      });

      expect(mockConnectionService.adjustResistance).toHaveBeenCalledWith(10);
    });

    it('should use custom step size for decrease', async () => {
      mockConnectionService.adjustResistance.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useResistanceControl(15));

      await act(async () => {
        await result.current.decreaseResistance();
      });

      expect(mockConnectionService.adjustResistance).toHaveBeenCalledWith(-15);
    });
  });
});
