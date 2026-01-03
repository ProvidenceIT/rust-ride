/**
 * useWorkoutControls Hook Tests
 */

import { renderHook, act } from '@testing-library/react-native';
import { useWorkoutControls } from '../../src/hooks/useWorkoutControls';
import { useSessionStore } from '../../src/stores/sessionStore';
import { getConnectionService } from '../../src/services/ConnectionService';

// Define the mock service interface
interface MockConnectionService {
  pauseWorkout: jest.Mock;
  resumeWorkout: jest.Mock;
  skipInterval: jest.Mock;
  stopWorkout: jest.Mock;
}

// Create the mock service
const mockService: MockConnectionService = {
  pauseWorkout: jest.fn(),
  resumeWorkout: jest.fn(),
  skipInterval: jest.fn(),
  stopWorkout: jest.fn(),
};

// Mock the ConnectionService module
jest.mock('../../src/services/ConnectionService', () => ({
  ConnectionService: jest.fn(() => mockService),
  getConnectionService: jest.fn(() => mockService),
}));

describe('useWorkoutControls', () => {
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
      const { result } = renderHook(() => useWorkoutControls());

      expect(result.current.pauseState).toEqual({ isLoading: false, error: null });
      expect(result.current.resumeState).toEqual({ isLoading: false, error: null });
      expect(result.current.skipState).toEqual({ isLoading: false, error: null });
      expect(result.current.stopState).toEqual({ isLoading: false, error: null });
      expect(result.current.isPauseResumeLoading).toBe(false);
      expect(result.current.isSkipLoading).toBe(false);
      expect(result.current.isStopLoading).toBe(false);
    });
  });

  describe('pause', () => {
    beforeEach(() => {
      // Set up an active session
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should do nothing if already paused', async () => {
      useSessionStore.getState().setPaused(true);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.pause();
      });

      expect(mockConnectionService.pauseWorkout).not.toHaveBeenCalled();
    });

    it('should do nothing if no active session', async () => {
      useSessionStore.getState().reset();

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.pause();
      });

      expect(mockConnectionService.pauseWorkout).not.toHaveBeenCalled();
    });

    it('should call pauseWorkout on ConnectionService', async () => {
      mockConnectionService.pauseWorkout.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.pause();
      });

      expect(mockConnectionService.pauseWorkout).toHaveBeenCalled();
    });

    it('should optimistically update paused state', async () => {
      let resolvePause: () => void;
      mockConnectionService.pauseWorkout.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolvePause = resolve; }),
      );

      const { result } = renderHook(() => useWorkoutControls());

      // Start pause
      let pausePromise: Promise<void>;
      act(() => {
        pausePromise = result.current.pause();
      });

      // Check optimistic update happened immediately
      expect(useSessionStore.getState().isPaused).toBe(true);
      expect(result.current.pauseState.isLoading).toBe(true);

      // Complete pause
      await act(async () => {
        resolvePause!();
        await pausePromise;
      });

      expect(result.current.pauseState.isLoading).toBe(false);
    });

    it('should rollback on error', async () => {
      mockConnectionService.pauseWorkout.mockRejectedValueOnce(new Error('Failed to pause'));

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.pause();
      });

      // State should be rolled back
      expect(useSessionStore.getState().isPaused).toBe(false);
      expect(result.current.pauseState.error).toBe('Failed to pause');
      expect(result.current.pauseState.isLoading).toBe(false);
    });

    it('should set isPauseResumeLoading during operation', async () => {
      let resolvePause: () => void;
      mockConnectionService.pauseWorkout.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolvePause = resolve; }),
      );

      const { result } = renderHook(() => useWorkoutControls());

      let pausePromise: Promise<void>;
      act(() => {
        pausePromise = result.current.pause();
      });

      expect(result.current.isPauseResumeLoading).toBe(true);

      await act(async () => {
        resolvePause!();
        await pausePromise;
      });

      expect(result.current.isPauseResumeLoading).toBe(false);
    });
  });

  describe('resume', () => {
    beforeEach(() => {
      // Set up a paused session
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: true,
        elapsed_secs: 100,
      });
    });

    it('should do nothing if not paused', async () => {
      useSessionStore.getState().setPaused(false);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.resume();
      });

      expect(mockConnectionService.resumeWorkout).not.toHaveBeenCalled();
    });

    it('should do nothing if no active session', async () => {
      useSessionStore.getState().reset();

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.resume();
      });

      expect(mockConnectionService.resumeWorkout).not.toHaveBeenCalled();
    });

    it('should call resumeWorkout on ConnectionService', async () => {
      mockConnectionService.resumeWorkout.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.resume();
      });

      expect(mockConnectionService.resumeWorkout).toHaveBeenCalled();
    });

    it('should optimistically update resumed state', async () => {
      let resolveResume: () => void;
      mockConnectionService.resumeWorkout.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveResume = resolve; }),
      );

      const { result } = renderHook(() => useWorkoutControls());

      // Start resume
      let resumePromise: Promise<void>;
      act(() => {
        resumePromise = result.current.resume();
      });

      // Check optimistic update happened immediately
      expect(useSessionStore.getState().isPaused).toBe(false);
      expect(result.current.resumeState.isLoading).toBe(true);

      // Complete resume
      await act(async () => {
        resolveResume!();
        await resumePromise;
      });

      expect(result.current.resumeState.isLoading).toBe(false);
    });

    it('should rollback on error', async () => {
      mockConnectionService.resumeWorkout.mockRejectedValueOnce(new Error('Failed to resume'));

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.resume();
      });

      // State should be rolled back
      expect(useSessionStore.getState().isPaused).toBe(true);
      expect(result.current.resumeState.error).toBe('Failed to resume');
      expect(result.current.resumeState.isLoading).toBe(false);
    });
  });

  describe('skip', () => {
    beforeEach(() => {
      // Set up an active workout session with intervals
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 100,
        current_interval_index: 0,
        total_intervals: 3,
        current_interval_name: 'Warmup',
      });
    });

    it('should do nothing if no active session', async () => {
      useSessionStore.getState().reset();

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.skip();
      });

      expect(mockConnectionService.skipInterval).not.toHaveBeenCalled();
    });

    it('should do nothing if no interval info', async () => {
      useSessionStore.getState().updateInterval(null);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.skip();
      });

      expect(mockConnectionService.skipInterval).not.toHaveBeenCalled();
    });

    it('should call skipInterval on ConnectionService', async () => {
      mockConnectionService.skipInterval.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.skip();
      });

      expect(mockConnectionService.skipInterval).toHaveBeenCalled();
    });

    it('should set loading state during skip', async () => {
      let resolveSkip: () => void;
      mockConnectionService.skipInterval.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveSkip = resolve; }),
      );

      const { result } = renderHook(() => useWorkoutControls());

      let skipPromise: Promise<void>;
      act(() => {
        skipPromise = result.current.skip();
      });

      expect(result.current.skipState.isLoading).toBe(true);
      expect(result.current.isSkipLoading).toBe(true);

      await act(async () => {
        resolveSkip!();
        await skipPromise;
      });

      expect(result.current.skipState.isLoading).toBe(false);
      expect(result.current.isSkipLoading).toBe(false);
    });

    it('should set error on failure', async () => {
      mockConnectionService.skipInterval.mockRejectedValueOnce(new Error('Cannot skip'));

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.skip();
      });

      expect(result.current.skipState.error).toBe('Cannot skip');
      expect(result.current.skipState.isLoading).toBe(false);
    });
  });

  describe('stop', () => {
    beforeEach(() => {
      // Set up an active session
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 100,
      });
    });

    it('should do nothing if no active session', async () => {
      useSessionStore.getState().reset();

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.stop();
      });

      expect(mockConnectionService.stopWorkout).not.toHaveBeenCalled();
    });

    it('should call stopWorkout on ConnectionService', async () => {
      mockConnectionService.stopWorkout.mockResolvedValueOnce(undefined);

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.stop();
      });

      expect(mockConnectionService.stopWorkout).toHaveBeenCalled();
    });

    it('should set loading state during stop', async () => {
      let resolveStop: () => void;
      mockConnectionService.stopWorkout.mockImplementationOnce(
        () => new Promise<void>(resolve => { resolveStop = resolve; }),
      );

      const { result } = renderHook(() => useWorkoutControls());

      let stopPromise: Promise<void>;
      act(() => {
        stopPromise = result.current.stop();
      });

      expect(result.current.stopState.isLoading).toBe(true);
      expect(result.current.isStopLoading).toBe(true);

      await act(async () => {
        resolveStop!();
        await stopPromise;
      });

      expect(result.current.stopState.isLoading).toBe(false);
      expect(result.current.isStopLoading).toBe(false);
    });

    it('should set error on failure', async () => {
      mockConnectionService.stopWorkout.mockRejectedValueOnce(new Error('Failed to stop'));

      const { result } = renderHook(() => useWorkoutControls());

      await act(async () => {
        await result.current.stop();
      });

      expect(result.current.stopState.error).toBe('Failed to stop');
      expect(result.current.stopState.isLoading).toBe(false);
    });
  });

  describe('clearError', () => {
    it('should clear all errors', async () => {
      // Set up sessions for all actions
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        is_paused: false,
        elapsed_secs: 100,
        current_interval_index: 0,
        total_intervals: 3,
      });

      // Cause errors
      mockConnectionService.pauseWorkout.mockRejectedValueOnce(new Error('Pause error'));
      mockConnectionService.stopWorkout.mockRejectedValueOnce(new Error('Stop error'));

      const { result } = renderHook(() => useWorkoutControls());

      // Trigger pause error
      await act(async () => {
        await result.current.pause();
      });

      // Reset pause state and trigger stop error
      useSessionStore.getState().setPaused(false);
      await act(async () => {
        await result.current.stop();
      });

      expect(result.current.pauseState.error).toBe('Pause error');
      expect(result.current.stopState.error).toBe('Stop error');

      // Clear all errors
      act(() => {
        result.current.clearError();
      });

      expect(result.current.pauseState.error).toBeNull();
      expect(result.current.resumeState.error).toBeNull();
      expect(result.current.skipState.error).toBeNull();
      expect(result.current.stopState.error).toBeNull();
    });
  });
});
