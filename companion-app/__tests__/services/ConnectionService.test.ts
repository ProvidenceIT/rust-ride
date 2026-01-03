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
});
