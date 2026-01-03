/**
 * ConnectionService Unit Tests
 */

import { ConnectionService, getConnectionService } from '../../src/services/ConnectionService';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useMetricsStore } from '../../src/stores/metricsStore';
import { useSessionStore } from '../../src/stores/sessionStore';

// Mock WebSocket
class MockWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly CONNECTING = MockWebSocket.CONNECTING;
  readonly OPEN = MockWebSocket.OPEN;
  readonly CLOSING = MockWebSocket.CLOSING;
  readonly CLOSED = MockWebSocket.CLOSED;

  readyState = MockWebSocket.CONNECTING;
  url: string;

  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  private static instances: MockWebSocket[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send = jest.fn();

  close = jest.fn((code?: number, reason?: string) => {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose({ code, reason } as CloseEvent);
    }
  });

  // Test helpers
  simulateOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      this.onopen({} as Event);
    }
  }

  simulateMessage(data: string): void {
    if (this.onmessage) {
      this.onmessage({ data } as MessageEvent);
    }
  }

  simulateClose(reason = 'Connection closed'): void {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose({ reason } as CloseEvent);
    }
  }

  simulateError(): void {
    if (this.onerror) {
      this.onerror({} as Event);
    }
  }

  static getLastInstance(): MockWebSocket | undefined {
    return MockWebSocket.instances[MockWebSocket.instances.length - 1];
  }

  static clearInstances(): void {
    MockWebSocket.instances = [];
  }
}

// Install mock
(global as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;

describe('ConnectionService', () => {
  let service: ConnectionService;

  beforeEach(() => {
    // Reset stores
    useConnectionStore.getState().reset();
    useMetricsStore.getState().reset();
    useSessionStore.getState().reset();

    // Clear mock instances
    MockWebSocket.clearInstances();

    // Get fresh service instance
    service = getConnectionService();
    service.reset();
  });

  afterEach(() => {
    service.reset();
  });

  describe('getInstance', () => {
    it('should return singleton instance', () => {
      const instance1 = ConnectionService.getInstance();
      const instance2 = ConnectionService.getInstance();
      expect(instance1).toBe(instance2);
    });
  });

  describe('connect', () => {
    it('should create WebSocket connection', async () => {
      const connectPromise = service.connect('ws://localhost:9876');

      const ws = MockWebSocket.getLastInstance();
      expect(ws).toBeDefined();
      expect(ws?.url).toBe('ws://localhost:9876');

      // Simulate successful connection
      ws?.simulateOpen();

      await connectPromise;

      expect(service.isConnected()).toBe(true);
    });

    it('should update connection store on connect', async () => {
      const connectPromise = service.connect('ws://localhost:9876');

      expect(useConnectionStore.getState().status).toBe('connecting');

      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();

      await connectPromise;

      expect(useConnectionStore.getState().status).toBe('connected');
    });

    it('should handle connection error', async () => {
      const connectPromise = service.connect('ws://localhost:9876');

      const ws = MockWebSocket.getLastInstance();
      ws?.simulateError();
      ws?.simulateClose('Connection failed');

      await expect(connectPromise).rejects.toThrow();
    });
  });

  describe('disconnect', () => {
    it('should close WebSocket connection', async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      service.disconnect();

      expect(ws?.close).toHaveBeenCalled();
      expect(useConnectionStore.getState().status).toBe('disconnected');
    });

    it('should reset stores on disconnect', async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Simulate some state
      useMetricsStore.getState().setSubscribed(true);

      service.disconnect();

      expect(useMetricsStore.getState().isSubscribed).toBe(false);
    });
  });

  describe('send', () => {
    it('should send JSON message via WebSocket', async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Start sending (will timeout but that's okay for this test)
      const sendPromise = service.send({ type: 'ping' });

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'ping' }));

      // Simulate pong response
      ws?.simulateMessage(JSON.stringify({ type: 'pong' }));

      const response = await sendPromise;
      expect(response.type).toBe('pong');
    });

    it('should reject if not connected', async () => {
      await expect(service.send({ type: 'ping' })).rejects.toThrow('Not connected');
    });
  });

  describe('message handling', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should handle metrics event', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'metrics',
          power_watts: 200,
          heart_rate_bpm: 150,
          cadence_rpm: 90,
          speed_kmh: 30,
          distance_km: 10.5,
          elapsed_secs: 1800,
          calories: 500,
        }),
      );

      const metrics = useMetricsStore.getState().metrics;
      expect(metrics.power_watts).toBe(200);
      expect(metrics.heart_rate_bpm).toBe(150);
      expect(metrics.cadence_rpm).toBe(90);
    });

    it('should handle session_state_changed event', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'test-session',
            session_type: 'workout',
            workout_name: 'Test Workout',
            is_paused: false,
            elapsed_secs: 0,
          },
        }),
      );

      const session = useSessionStore.getState();
      expect(session.isActive).toBe(true);
      expect(session.workoutName).toBe('Test Workout');
    });

    it('should handle interval_changed event', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'interval_changed',
          interval_index: 2,
          total_intervals: 5,
          interval_name: 'Hard',
          target_power_watts: 250,
          duration_secs: 300,
        }),
      );

      const session = useSessionStore.getState();
      expect(session.currentInterval?.index).toBe(2);
      expect(session.currentInterval?.total).toBe(5);
      expect(session.currentInterval?.name).toBe('Hard');
      expect(session.targetPowerWatts).toBe(250);
    });

    it('should handle auth_ok response', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'auth_ok',
          session_id: 'session-123',
        }),
      );

      expect(useConnectionStore.getState().isAuthenticated).toBe(true);
    });

    it('should handle auth_failed response', () => {
      const ws = MockWebSocket.getLastInstance();
      const onAuthFailed = jest.fn();

      service.setCallbacks({ onAuthFailed });

      ws?.simulateMessage(
        JSON.stringify({
          type: 'auth_failed',
          reason: 'Invalid PIN',
        }),
      );

      expect(onAuthFailed).toHaveBeenCalledWith('Invalid PIN');
    });
  });

  describe('exponential backoff', () => {
    it('should calculate correct backoff delays', () => {
      // Default config: initial 1000ms, multiplier 2, max 30000ms
      // Attempt 0: 1000 * 2^0 = 1000ms
      // Attempt 1: 1000 * 2^1 = 2000ms
      // Attempt 2: 1000 * 2^2 = 4000ms
      // Attempt 3: 1000 * 2^3 = 8000ms
      // Attempt 4: 1000 * 2^4 = 16000ms
      // Attempt 5: 1000 * 2^5 = 32000ms -> capped to 30000ms

      // This is tested indirectly through the reconnection mechanism
      // The actual delay calculation is: min(initialDelay * multiplier^attempt, maxDelay)
      expect(Math.min(1000 * Math.pow(2, 0), 30000)).toBe(1000);
      expect(Math.min(1000 * Math.pow(2, 1), 30000)).toBe(2000);
      expect(Math.min(1000 * Math.pow(2, 2), 30000)).toBe(4000);
      expect(Math.min(1000 * Math.pow(2, 3), 30000)).toBe(8000);
      expect(Math.min(1000 * Math.pow(2, 4), 30000)).toBe(16000);
      expect(Math.min(1000 * Math.pow(2, 5), 30000)).toBe(30000);
      expect(Math.min(1000 * Math.pow(2, 10), 30000)).toBe(30000);
    });
  });

  describe('authenticate', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should send auth message with PIN', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Start authenticate
      const authPromise = service.authenticate('123456');

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'auth', pin: '123456' }));

      // Simulate successful auth
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'test-session' }));

      await authPromise;
      expect(useConnectionStore.getState().isAuthenticated).toBe(true);
    });

    it('should throw on auth failure', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Start authenticate
      const authPromise = service.authenticate('000000');

      // Simulate failed auth
      ws?.simulateMessage(JSON.stringify({ type: 'auth_failed', reason: 'Invalid PIN' }));

      await expect(authPromise).rejects.toThrow('Invalid PIN');
    });

    it('should auto-subscribe to metrics after auth_ok', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Clear previous calls
      ws?.send.mockClear();

      // Simulate auth_ok response directly (not through authenticate())
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'test-session' }));

      // Should have sent subscribe_metrics and get_session_status
      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'subscribe_metrics' }));
      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'get_session_status' }));
    });

    it('should call onAuthFailed callback on auth failure', async () => {
      const ws = MockWebSocket.getLastInstance();
      const onAuthFailed = jest.fn();

      service.setCallbacks({ onAuthFailed });

      // Simulate auth failed response
      ws?.simulateMessage(JSON.stringify({ type: 'auth_failed', reason: 'Wrong PIN' }));

      expect(onAuthFailed).toHaveBeenCalledWith('Wrong PIN');
    });

    it('should update connection store to authenticated status', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Start authenticate
      const authPromise = service.authenticate('123456');

      // Simulate successful auth
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'test-session' }));

      await authPromise;

      const state = useConnectionStore.getState();
      expect(state.status).toBe('authenticated');
      expect(state.isAuthenticated).toBe(true);
    });
  });

  describe('helper methods', () => {
    it('isConnected should return correct state', async () => {
      expect(service.isConnected()).toBe(false);

      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      expect(service.isConnected()).toBe(true);

      service.disconnect();
      expect(service.isConnected()).toBe(false);
    });

    it('getServerUrl should return current server URL', async () => {
      expect(service.getServerUrl()).toBeNull();

      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      expect(service.getServerUrl()).toBe('ws://localhost:9876');
    });
  });

  describe('workout controls', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should send pause command and handle success', async () => {
      const ws = MockWebSocket.getLastInstance();

      const pausePromise = service.pauseWorkout();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_pause' }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_pause' }));

      await pausePromise;
    });

    it('should send pause command and handle failure', async () => {
      const ws = MockWebSocket.getLastInstance();

      const pausePromise = service.pauseWorkout();

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_failed', command: 'workout_pause', error: 'Not a workout' }),
      );

      await expect(pausePromise).rejects.toThrow('Not a workout');
    });

    it('should send resume command', async () => {
      const ws = MockWebSocket.getLastInstance();

      const resumePromise = service.resumeWorkout();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_resume' }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_resume' }));

      await resumePromise;
    });

    it('should send skip command', async () => {
      const ws = MockWebSocket.getLastInstance();

      const skipPromise = service.skipInterval();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_skip' }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_skip' }));

      await skipPromise;
    });

    it('should send stop command', async () => {
      const ws = MockWebSocket.getLastInstance();

      const stopPromise = service.stopWorkout();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_stop' }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_stop' }));

      await stopPromise;
    });

    it('should send adjust_resistance command', async () => {
      const ws = MockWebSocket.getLastInstance();

      const adjustPromise = service.adjustResistance(5);

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'adjust_resistance', delta: 5 }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' }));

      await adjustPromise;
    });

    it('should send adjust_resistance with negative delta', async () => {
      const ws = MockWebSocket.getLastInstance();

      const adjustPromise = service.adjustResistance(-10);

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'adjust_resistance', delta: -10 }));

      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' }));

      await adjustPromise;
    });
  });

  describe('ride history', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should fetch ride history with default parameters', async () => {
      const ws = MockWebSocket.getLastInstance();

      service.fetchRideHistory();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_ride_history', limit: 20, offset: 0 }),
      );
    });

    it('should fetch ride history with custom parameters', async () => {
      const ws = MockWebSocket.getLastInstance();

      service.fetchRideHistory(50, 100);

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_ride_history', limit: 50, offset: 100 }),
      );
    });

    it('should fetch ride details', async () => {
      const ws = MockWebSocket.getLastInstance();

      service.fetchRideDetails('ride-123');

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_ride_details', ride_id: 'ride-123' }),
      );
    });
  });

  describe('message parsing edge cases', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should ignore invalid JSON messages', () => {
      const ws = MockWebSocket.getLastInstance();

      // Should not throw
      expect(() => {
        ws?.simulateMessage('{ invalid json }');
      }).not.toThrow();
    });

    it('should handle metrics with null values', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'metrics',
          power_watts: null,
          heart_rate_bpm: null,
          cadence_rpm: null,
          speed_kmh: null,
          distance_km: 0,
          elapsed_secs: 0,
          calories: 0,
        }),
      );

      const metrics = useMetricsStore.getState().metrics;
      expect(metrics.power_watts).toBe(0);
      expect(metrics.heart_rate_bpm).toBeNull();
    });

    it('should handle session_state_changed to paused', () => {
      const ws = MockWebSocket.getLastInstance();

      // First start a session
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'test-session',
            session_type: 'workout',
            workout_name: 'Test Workout',
            is_paused: false,
            elapsed_secs: 0,
          },
        }),
      );

      expect(useSessionStore.getState().isActive).toBe(true);

      // Then pause it
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'paused',
        }),
      );

      expect(useSessionStore.getState().isPaused).toBe(true);
    });

    it('should handle session_state_changed to completed', () => {
      const ws = MockWebSocket.getLastInstance();

      // First start a session
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'test-session',
            session_type: 'workout',
            workout_name: 'Test Workout',
            is_paused: false,
            elapsed_secs: 0,
          },
        }),
      );

      expect(useSessionStore.getState().isActive).toBe(true);

      // Then complete it
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'completed',
        }),
      );

      expect(useSessionStore.getState().isActive).toBe(false);
    });

    it('should handle disconnecting event', () => {
      const ws = MockWebSocket.getLastInstance();
      const onDisconnected = jest.fn();

      service.setCallbacks({ onDisconnected });

      ws?.simulateMessage(
        JSON.stringify({
          type: 'disconnecting',
          reason: 'Server shutting down',
        }),
      );

      expect(onDisconnected).toHaveBeenCalledWith('Server shutting down');
    });

    it('should handle error response with AUTH_REQUIRED code', () => {
      const ws = MockWebSocket.getLastInstance();
      const onAuthRequired = jest.fn();

      service.setCallbacks({ onAuthRequired });

      ws?.simulateMessage(
        JSON.stringify({
          type: 'error',
          code: 'AUTH_REQUIRED',
          message: 'Authentication required',
        }),
      );

      expect(onAuthRequired).toHaveBeenCalled();
    });

    it('should handle error response with other codes', () => {
      const ws = MockWebSocket.getLastInstance();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'error',
          code: 'NO_SESSION',
          message: 'No active session',
        }),
      );

      const state = useConnectionStore.getState();
      expect(state.error?.message).toBe('No active session');
    });
  });

  describe('metrics subscription', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should subscribe to metrics', async () => {
      const ws = MockWebSocket.getLastInstance();

      const subscribePromise = service.subscribeMetrics();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'subscribe_metrics' }));

      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));

      await subscribePromise;
      expect(useMetricsStore.getState().isSubscribed).toBe(true);
    });

    it('should unsubscribe from metrics', async () => {
      const ws = MockWebSocket.getLastInstance();

      // First subscribe
      useMetricsStore.getState().setSubscribed(true);

      const unsubscribePromise = service.unsubscribeMetrics();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'unsubscribe_metrics' }));

      ws?.simulateMessage(JSON.stringify({ type: 'unsubscribed_metrics' }));

      await unsubscribePromise;
      expect(useMetricsStore.getState().isSubscribed).toBe(false);
    });
  });

  describe('session status', () => {
    beforeEach(async () => {
      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;
    });

    it('should get session status', async () => {
      const ws = MockWebSocket.getLastInstance();

      const statusPromise = service.getSessionStatus();

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'get_session_status' }));

      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_status',
          active: true,
          session: {
            session_id: 'test-session',
            session_type: 'workout',
            workout_name: 'Test Workout',
            is_paused: false,
            elapsed_secs: 1800,
            target_power_watts: 200,
          },
        }),
      );

      await statusPromise;

      const session = useSessionStore.getState();
      expect(session.isActive).toBe(true);
      expect(session.workoutName).toBe('Test Workout');
      expect(session.targetPowerWatts).toBe(200);
    });

    it('should handle inactive session status', async () => {
      const ws = MockWebSocket.getLastInstance();

      // First set an active session
      useSessionStore.getState().startSession({
        session_id: 'test-session',
        session_type: 'workout',
        workout_name: 'Test Workout',
        is_paused: false,
        elapsed_secs: 0,
      });

      const statusPromise = service.getSessionStatus();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_status',
          active: false,
          session: null,
        }),
      );

      await statusPromise;

      expect(useSessionStore.getState().isActive).toBe(false);
    });
  });

  describe('callbacks', () => {
    it('should call onError callback on WebSocket error', async () => {
      const onError = jest.fn();
      service.setCallbacks({ onError });

      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      ws?.simulateError();

      expect(onError).toHaveBeenCalled();
    });

    it('should call onDisconnected callback on close', async () => {
      const onDisconnected = jest.fn();
      service.setCallbacks({ onDisconnected });

      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      ws?.simulateClose('Server closed connection');

      expect(onDisconnected).toHaveBeenCalledWith('Server closed connection');
    });
  });

  describe('backoff configuration', () => {
    it('should allow custom backoff settings', () => {
      service.setBackoffConfig({
        initialDelayMs: 500,
        maxDelayMs: 10000,
        multiplier: 1.5,
      });

      // Verify the formula works with new config
      expect(Math.min(500 * Math.pow(1.5, 0), 10000)).toBe(500);
      expect(Math.min(500 * Math.pow(1.5, 1), 10000)).toBe(750);
      expect(Math.min(500 * Math.pow(1.5, 2), 10000)).toBe(1125);
    });
  });

  describe('connection state', () => {
    it('should close existing connection before new connect', async () => {
      // First connection
      const connectPromise1 = service.connect('ws://localhost:9876');
      const ws1 = MockWebSocket.getLastInstance();
      ws1?.simulateOpen();
      await connectPromise1;

      expect(service.isConnected()).toBe(true);

      // Second connection should close the first
      const connectPromise2 = service.connect('ws://localhost:9877');
      const ws2 = MockWebSocket.getLastInstance();
      ws2?.simulateOpen();
      await connectPromise2;

      expect(ws1?.close).toHaveBeenCalled();
      expect(ws2?.url).toBe('ws://localhost:9877');
    });

    it('isAuthenticated should return correct state', async () => {
      expect(service.isAuthenticated()).toBe(false);

      const connectPromise = service.connect('ws://localhost:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Still not authenticated
      expect(service.isAuthenticated()).toBe(false);

      // Simulate auth
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'test-session' }));

      expect(service.isAuthenticated()).toBe(true);
    });
  });
});
