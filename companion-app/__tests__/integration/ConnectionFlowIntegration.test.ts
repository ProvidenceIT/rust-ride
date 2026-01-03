/**
 * Integration Tests for Full Connection Flow
 *
 * Tests the complete connection flow:
 * 1. Discovery - Find RustRide server via mDNS
 * 2. Connect - Establish WebSocket connection
 * 3. Authenticate - Send PIN and receive auth_ok
 * 4. Receive metrics - Subscribe and receive real-time metrics
 * 5. Send commands - Workout control commands (pause/resume/skip/stop)
 */

import MockZeroconf from '../../__mocks__/react-native-zeroconf';
import { DiscoveryService, getDiscoveryService } from '../../src/services/DiscoveryService';
import { ConnectionService, getConnectionService } from '../../src/services/ConnectionService';
import { useConnectionStore } from '../../src/stores/connectionStore';
import { useMetricsStore } from '../../src/stores/metricsStore';
import { useSessionStore } from '../../src/stores/sessionStore';
import { useHistoryStore } from '../../src/stores/historyStore';

// Type definitions for WebSocket events (not available in RN type definitions)
interface MockCloseEvent {
  code?: number;
  reason?: string;
}

interface MockMessageEvent {
  data: string;
}

// Define ZeroconfService interface for testing
interface ZeroconfService {
  name: string;
  fullName: string;
  host: string;
  port: number;
  addresses: string[];
  txt: Record<string, string>;
}

// Mock WebSocket class
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

  onopen: ((event: unknown) => void) | null = null;
  onclose: ((event: MockCloseEvent) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: MockMessageEvent) => void) | null = null;

  private static instances: MockWebSocket[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send = jest.fn();

  close = jest.fn((code?: number, reason?: string) => {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose({ code, reason });
    }
  });

  // Test helpers
  simulateOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      this.onopen({});
    }
  }

  simulateMessage(data: string): void {
    if (this.onmessage) {
      this.onmessage({ data });
    }
  }

  simulateClose(reason = 'Connection closed'): void {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose({ reason });
    }
  }

  simulateError(): void {
    if (this.onerror) {
      this.onerror({});
    }
  }

  static getLastInstance(): MockWebSocket | undefined {
    return MockWebSocket.instances[MockWebSocket.instances.length - 1];
  }

  static clearInstances(): void {
    MockWebSocket.instances = [];
  }
}

// Install mock WebSocket - use globalThis for proper typing
// eslint-disable-next-line @typescript-eslint/no-explicit-any
(globalThis as any).WebSocket = MockWebSocket;

// Helper function for async delays
const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });

describe('Integration: Full Connection Flow', () => {
  let discoveryService: DiscoveryService;
  let connectionService: ConnectionService;

  beforeEach(() => {
    // Reset all stores
    useConnectionStore.getState().reset();
    useMetricsStore.getState().reset();
    useSessionStore.getState().reset();
    useHistoryStore.getState().reset();

    // Clear mocks
    MockZeroconf.clearInstance();
    MockWebSocket.clearInstances();

    // Reset singleton instances
    (DiscoveryService as unknown as { instance: DiscoveryService | null }).instance = null;

    // Get fresh service instances
    discoveryService = getDiscoveryService();
    connectionService = getConnectionService();
    connectionService.reset();
  });

  afterEach(async () => {
    // Ensure any pending timeouts are cleared
    jest.clearAllTimers();

    // Reset services (this will clear pending requests)
    try {
      connectionService.reset();
    } catch {
      // Ignore errors during reset (e.g., pending request rejections)
    }

    discoveryService.cleanup();
    jest.clearAllMocks();

    // Allow any pending state updates to settle
    await delay(0);
  });

  describe('Discovery → Connect → Authenticate Flow', () => {
    it('should complete full flow: discover server, connect, authenticate', async () => {
      // Step 1: Start discovery
      await discoveryService.startScan();
      expect(useConnectionStore.getState().isScanning).toBe(true);

      // Step 2: Simulate server discovery via mDNS
      const zeroconf = MockZeroconf.instance;
      const mockServer: ZeroconfService = {
        name: 'My RustRide',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: { version: '1.0.0' },
      };
      zeroconf?.simulateServiceResolved(mockServer);

      // Verify server was discovered
      const servers = useConnectionStore.getState().discoveredServers;
      expect(servers).toHaveLength(1);
      expect(servers[0].name).toBe('My RustRide');
      expect(servers[0].host).toBe('192.168.1.100');
      expect(servers[0].port).toBe(9876);

      // Step 3: Build URL and connect
      const serverUrl = discoveryService.buildServerUrl(servers[0]);
      expect(serverUrl).toBe('ws://192.168.1.100:9876');

      const connectPromise = connectionService.connect(serverUrl);

      // Verify connecting state
      expect(useConnectionStore.getState().status).toBe('connecting');

      // Simulate WebSocket connection
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Verify connected state
      expect(useConnectionStore.getState().status).toBe('connected');
      expect(connectionService.isConnected()).toBe(true);

      // Step 4: Authenticate with PIN
      const authPromise = connectionService.authenticate('123456');

      // Verify auth request was sent
      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'auth', pin: '123456' })
      );

      // Simulate successful auth response
      ws?.simulateMessage(
        JSON.stringify({ type: 'auth_ok', session_id: 'session-123' })
      );

      await authPromise;

      // Verify authenticated state
      expect(useConnectionStore.getState().status).toBe('authenticated');
      expect(useConnectionStore.getState().isAuthenticated).toBe(true);
    });

    it('should handle auth failure gracefully', async () => {
      // Connect first
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Attempt authentication with wrong PIN
      const authPromise = connectionService.authenticate('000000');

      // Simulate failed auth
      ws?.simulateMessage(
        JSON.stringify({ type: 'auth_failed', reason: 'Invalid PIN' })
      );

      await expect(authPromise).rejects.toThrow('Invalid PIN');

      // Verify still connected but not authenticated
      expect(connectionService.isConnected()).toBe(true);
      expect(connectionService.isAuthenticated()).toBe(false);
    });

    it('should handle connection failure during discovery flow', async () => {
      // Discover server
      await discoveryService.startScan();
      const zeroconf = MockZeroconf.instance;
      zeroconf?.simulateServiceResolved({
        name: 'Offline Server',
        fullName: '_rustride._tcp.local.',
        host: 'offline.local',
        port: 9876,
        addresses: ['192.168.1.50'],
        txt: {},
      });

      const servers = useConnectionStore.getState().discoveredServers;
      const serverUrl = discoveryService.buildServerUrl(servers[0]);

      // Attempt connection
      const connectPromise = connectionService.connect(serverUrl);

      // Simulate connection error
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateError();
      ws?.simulateClose('Connection refused');

      await expect(connectPromise).rejects.toThrow();

      // Verify error state
      expect(useConnectionStore.getState().status).toBe('disconnected');
    });
  });

  describe('Metrics Streaming Flow', () => {
    beforeEach(async () => {
      // Set up connected and authenticated state
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Authenticate
      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Handle auto-subscribe and auto-status requests that happen after auth_ok
      // These are fire-and-forget in the service but create pending promises
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      ws?.simulateMessage(JSON.stringify({ type: 'session_status', active: false, session: null }));
      // Allow async state updates to complete
      await delay(10);
    });

    it('should subscribe to metrics and receive updates', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      // Subscribe to metrics
      const subscribePromise = connectionService.subscribeMetrics();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'subscribe_metrics' })
      );

      // Simulate subscription confirmation
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      await subscribePromise;

      expect(useMetricsStore.getState().isSubscribed).toBe(true);

      // Simulate receiving metrics
      ws?.simulateMessage(
        JSON.stringify({
          type: 'metrics',
          power_watts: 250,
          heart_rate_bpm: 155,
          cadence_rpm: 92,
          speed_kmh: 35,
          distance_km: 15.5,
          elapsed_secs: 3600,
          calories: 750,
        })
      );

      // Verify metrics updated in store
      const metrics = useMetricsStore.getState().metrics;
      expect(metrics.power_watts).toBe(250);
      expect(metrics.heart_rate_bpm).toBe(155);
      expect(metrics.cadence_rpm).toBe(92);
      expect(metrics.speed_kph).toBe(35);
      expect(metrics.distance_km).toBe(15.5);
      expect(metrics.calories).toBe(750);
    });

    it('should handle continuous metrics stream', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Subscribe
      connectionService.subscribeMetrics();
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));

      // Simulate multiple metrics updates (like real-time streaming)
      const metricsSequence = [
        { power_watts: 200, heart_rate_bpm: 140, cadence_rpm: 85 },
        { power_watts: 220, heart_rate_bpm: 145, cadence_rpm: 88 },
        { power_watts: 250, heart_rate_bpm: 152, cadence_rpm: 90 },
        { power_watts: 180, heart_rate_bpm: 138, cadence_rpm: 82 },
      ];

      for (const metric of metricsSequence) {
        ws?.simulateMessage(
          JSON.stringify({
            type: 'metrics',
            ...metric,
            speed_kmh: 30,
            distance_km: 10,
            elapsed_secs: 1800,
            calories: 400,
          })
        );
      }

      // Verify final metrics state
      const metrics = useMetricsStore.getState().metrics;
      expect(metrics.power_watts).toBe(180);
      expect(metrics.heart_rate_bpm).toBe(138);
      expect(metrics.cadence_rpm).toBe(82);
    });

    it('should unsubscribe from metrics', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      // Subscribe first
      connectionService.subscribeMetrics();
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      expect(useMetricsStore.getState().isSubscribed).toBe(true);

      ws?.send.mockClear();

      // Unsubscribe
      const unsubPromise = connectionService.unsubscribeMetrics();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'unsubscribe_metrics' })
      );

      ws?.simulateMessage(JSON.stringify({ type: 'unsubscribed_metrics' }));
      await unsubPromise;

      expect(useMetricsStore.getState().isSubscribed).toBe(false);
    });
  });

  describe('Session State Flow', () => {
    beforeEach(async () => {
      // Set up connected and authenticated state
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Handle auto-subscribe and auto-status requests that happen after auth_ok
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      ws?.simulateMessage(JSON.stringify({ type: 'session_status', active: false, session: null }));
      await delay(10);
    });

    it('should receive and update session status', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      // Request session status
      const statusPromise = connectionService.getSessionStatus();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_session_status' })
      );

      // Simulate session status response with active workout
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_status',
          active: true,
          session: {
            session_id: 'workout-001',
            session_type: 'workout',
            workout_name: 'Sweet Spot Base',
            is_paused: false,
            elapsed_secs: 1200,
            current_interval_index: 3,
            total_intervals: 8,
            current_interval_name: 'Sweet Spot',
            target_power_watts: 220,
            interval_remaining_secs: 180,
          },
        })
      );

      await statusPromise;

      // Verify session state
      const session = useSessionStore.getState();
      expect(session.isActive).toBe(true);
      expect(session.workoutName).toBe('Sweet Spot Base');
      expect(session.targetPowerWatts).toBe(220);
    });

    it('should handle session state change events', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Simulate session starting
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'session-001',
            session_type: 'workout',
            workout_name: 'FTP Test',
            is_paused: false,
            elapsed_secs: 0,
          },
        })
      );

      expect(useSessionStore.getState().isActive).toBe(true);
      expect(useSessionStore.getState().workoutName).toBe('FTP Test');

      // Simulate pause
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'paused',
          session: null,
        })
      );

      expect(useSessionStore.getState().isPaused).toBe(true);

      // Simulate resume (back to active)
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'session-001',
            session_type: 'workout',
            workout_name: 'FTP Test',
            is_paused: false,
            elapsed_secs: 300,
          },
        })
      );

      expect(useSessionStore.getState().isPaused).toBe(false);

      // Simulate session completion
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'completed',
          session: null,
        })
      );

      expect(useSessionStore.getState().isActive).toBe(false);
    });

    it('should handle interval change events', async () => {
      const ws = MockWebSocket.getLastInstance();

      // Start a session first
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'workout-001',
            session_type: 'workout',
            workout_name: 'Intervals',
            is_paused: false,
            elapsed_secs: 0,
          },
        })
      );

      // Simulate interval change
      ws?.simulateMessage(
        JSON.stringify({
          type: 'interval_changed',
          interval_index: 2,
          total_intervals: 10,
          interval_name: 'VO2max Interval',
          target_power_watts: 350,
          duration_secs: 180,
        })
      );

      const session = useSessionStore.getState();
      expect(session.currentInterval?.index).toBe(2);
      expect(session.currentInterval?.total).toBe(10);
      expect(session.currentInterval?.name).toBe('VO2max Interval');
      expect(session.targetPowerWatts).toBe(350);

      // Verify metrics store also updated
      expect(useMetricsStore.getState().targetPower).toBe(350);
    });
  });

  describe('Workout Control Commands', () => {
    beforeEach(async () => {
      // Set up connected and authenticated state
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Handle auto-subscribe and auto-status requests that happen after auth_ok
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      ws?.simulateMessage(JSON.stringify({ type: 'session_status', active: false, session: null }));
      await delay(10);

      // Start a session
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'workout-001',
            session_type: 'workout',
            workout_name: 'Test Workout',
            is_paused: false,
            elapsed_secs: 0,
          },
        })
      );
    });

    it('should send pause command and handle success', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const pausePromise = connectionService.pauseWorkout();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'workout_pause' })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'workout_pause' })
      );

      await pausePromise;
      // Command succeeded
    });

    it('should send resume command and handle success', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const resumePromise = connectionService.resumeWorkout();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'workout_resume' })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'workout_resume' })
      );

      await resumePromise;
    });

    it('should send skip command and handle success', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const skipPromise = connectionService.skipInterval();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'workout_skip' })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'workout_skip' })
      );

      await skipPromise;
    });

    it('should send stop command and handle success', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const stopPromise = connectionService.stopWorkout();

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'workout_stop' })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'workout_stop' })
      );

      await stopPromise;
    });

    it('should handle command failure', async () => {
      const ws = MockWebSocket.getLastInstance();

      const pausePromise = connectionService.pauseWorkout();

      ws?.simulateMessage(
        JSON.stringify({
          type: 'command_failed',
          command: 'workout_pause',
          error: 'Session already paused',
        })
      );

      await expect(pausePromise).rejects.toThrow('Session already paused');
    });

    it('should send resistance adjustment for free rides', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const adjustPromise = connectionService.adjustResistance(5);

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'adjust_resistance', delta: 5 })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' })
      );

      await adjustPromise;
    });

    it('should send negative resistance adjustment', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      const adjustPromise = connectionService.adjustResistance(-10);

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'adjust_resistance', delta: -10 })
      );

      ws?.simulateMessage(
        JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' })
      );

      await adjustPromise;
    });
  });

  describe('Ride History Flow', () => {
    beforeEach(async () => {
      // Set up connected and authenticated state
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Handle auto-subscribe and auto-status requests that happen after auth_ok
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      ws?.simulateMessage(JSON.stringify({ type: 'session_status', active: false, session: null }));
      await delay(10);
    });

    it('should fetch ride history', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      connectionService.fetchRideHistory(20, 0);

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_ride_history', limit: 20, offset: 0 })
      );

      // Simulate response
      ws?.simulateMessage(
        JSON.stringify({
          type: 'ride_history',
          rides: [
            {
              id: 'ride-001',
              date: '2024-01-15T10:00:00Z',
              duration_secs: 3600,
              distance_km: 40,
              avg_power_watts: 200,
              workout_name: 'Morning Ride',
              is_workout: false,
            },
            {
              id: 'ride-002',
              date: '2024-01-14T18:00:00Z',
              duration_secs: 2700,
              distance_km: 30,
              avg_power_watts: 220,
              workout_name: 'Sweet Spot',
              is_workout: true,
            },
          ],
          total: 50,
        })
      );

      // Wait for async state updates
      await delay(10);

      const historyState = useHistoryStore.getState();
      expect(historyState.rides).toHaveLength(2);
      expect(historyState.pagination.total).toBe(50);
    });

    it('should fetch ride details', async () => {
      const ws = MockWebSocket.getLastInstance();
      ws?.send.mockClear();

      connectionService.fetchRideDetails('ride-001');

      expect(ws?.send).toHaveBeenCalledWith(
        JSON.stringify({ type: 'get_ride_details', ride_id: 'ride-001' })
      );

      // Simulate response
      ws?.simulateMessage(
        JSON.stringify({
          type: 'ride_details',
          ride: {
            ride_id: 'ride-001',
            started_at: '2024-01-15T10:00:00Z',
            ended_at: '2024-01-15T11:00:00Z',
            duration_secs: 3600,
            distance_km: 40,
            calories: 800,
            avg_power_watts: 200,
            max_power_watts: 350,
            normalized_power_watts: 210,
            avg_heart_rate_bpm: 145,
            max_heart_rate_bpm: 175,
            avg_cadence_rpm: 88,
            tss: 65,
            intensity_factor: 0.85,
            is_workout: false,
            workout_name: null,
          },
        })
      );

      // Wait for async state updates
      await delay(10);

      const historyState = useHistoryStore.getState();
      expect(historyState.currentRideDetail?.ride_id).toBe('ride-001');
      expect(historyState.currentRideDetail?.avg_power_watts).toBe(200);
      expect(historyState.currentRideDetail?.tss).toBe(65);
    });
  });

  describe('Error Recovery Flow', () => {
    it('should handle disconnection and update stores', async () => {
      // Connect and authenticate
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Subscribe to metrics
      useMetricsStore.getState().setSubscribed(true);

      // Verify initial state
      expect(connectionService.isConnected()).toBe(true);
      expect(connectionService.isAuthenticated()).toBe(true);

      // Simulate unexpected disconnection
      ws?.simulateClose('Server shutdown');

      // Verify disconnected state
      expect(useConnectionStore.getState().status).toBe('disconnected');
      expect(useMetricsStore.getState().isSubscribed).toBe(false);
    });

    it('should handle server-initiated disconnection event', async () => {
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const onDisconnected = jest.fn();
      connectionService.setCallbacks({ onDisconnected });

      // Simulate disconnecting event from server
      ws?.simulateMessage(
        JSON.stringify({
          type: 'disconnecting',
          reason: 'Server maintenance',
        })
      );

      expect(onDisconnected).toHaveBeenCalledWith('Server maintenance');
    });

    it('should handle AUTH_REQUIRED error', async () => {
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const onAuthRequired = jest.fn();
      connectionService.setCallbacks({ onAuthRequired });

      // Simulate AUTH_REQUIRED error (e.g., after session timeout)
      ws?.simulateMessage(
        JSON.stringify({
          type: 'error',
          code: 'AUTH_REQUIRED',
          message: 'Session expired',
        })
      );

      expect(onAuthRequired).toHaveBeenCalled();
    });

    it('should handle multiple command sequences', async () => {
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Start multiple commands in sequence
      const pausePromise = connectionService.pauseWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_pause' }));
      await pausePromise;

      const resumePromise = connectionService.resumeWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_resume' }));
      await resumePromise;

      const skipPromise = connectionService.skipInterval();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_skip' }));
      await skipPromise;

      // All commands should complete successfully
      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_pause' }));
      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_resume' }));
      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'workout_skip' }));
    });
  });

  describe('Complete End-to-End Flow', () => {
    it('should handle complete workout session from discovery to completion', async () => {
      // Phase 1: Discovery
      await discoveryService.startScan();
      const zeroconf = MockZeroconf.instance;
      zeroconf?.simulateServiceResolved({
        name: 'Home Trainer',
        fullName: '_rustride._tcp.local.',
        host: 'trainer.local',
        port: 9876,
        addresses: ['192.168.1.200'],
        txt: { version: '2.0.0' },
      });

      const servers = useConnectionStore.getState().discoveredServers;
      expect(servers).toHaveLength(1);

      // Phase 2: Connection
      const serverUrl = discoveryService.buildServerUrl(servers[0]);
      const connectPromise = connectionService.connect(serverUrl);
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      // Phase 3: Authentication
      const authPromise = connectionService.authenticate('654321');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'main-session' }));
      await authPromise;

      // Handle auto-subscribe and auto-status requests that happen after auth_ok
      ws?.simulateMessage(JSON.stringify({ type: 'subscribed_metrics' }));
      ws?.simulateMessage(JSON.stringify({ type: 'session_status', active: false, session: null }));
      await delay(10);

      expect(useConnectionStore.getState().isAuthenticated).toBe(true);

      // Phase 4: Get Session Status (now showing active session)
      const statusPromise = connectionService.getSessionStatus();
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_status',
          active: true,
          session: {
            session_id: 'workout-001',
            session_type: 'workout',
            workout_name: 'FTP Builder',
            is_paused: false,
            elapsed_secs: 600,
            current_interval_index: 1,
            total_intervals: 5,
            current_interval_name: 'Warmup',
            target_power_watts: 150,
            interval_remaining_secs: 120,
          },
        })
      );
      await statusPromise;

      expect(useSessionStore.getState().isActive).toBe(true);
      expect(useSessionStore.getState().workoutName).toBe('FTP Builder');

      // Phase 5: Receive Metrics
      for (let i = 0; i < 5; i++) {
        ws?.simulateMessage(
          JSON.stringify({
            type: 'metrics',
            power_watts: 150 + i * 10,
            heart_rate_bpm: 130 + i * 2,
            cadence_rpm: 85 + i,
            speed_kmh: 28 + i,
            distance_km: 5 + i * 0.5,
            elapsed_secs: 600 + i * 60,
            calories: 300 + i * 20,
          })
        );
      }

      // Verify final metrics
      const metrics = useMetricsStore.getState().metrics;
      expect(metrics.power_watts).toBe(190);
      expect(metrics.heart_rate_bpm).toBe(138);

      // Phase 6: Interval Change
      ws?.simulateMessage(
        JSON.stringify({
          type: 'interval_changed',
          interval_index: 2,
          total_intervals: 5,
          interval_name: 'Sweet Spot',
          target_power_watts: 220,
          duration_secs: 600,
        })
      );

      expect(useSessionStore.getState().currentInterval?.index).toBe(2);
      expect(useSessionStore.getState().currentInterval?.name).toBe('Sweet Spot');

      // Phase 7: Pause and Resume
      const pausePromise = connectionService.pauseWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_pause' }));
      await pausePromise;

      ws?.simulateMessage(
        JSON.stringify({ type: 'session_state_changed', state: 'paused', session: null })
      );
      expect(useSessionStore.getState().isPaused).toBe(true);

      const resumePromise = connectionService.resumeWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_resume' }));
      await resumePromise;

      // Phase 8: Skip Interval
      const skipPromise = connectionService.skipInterval();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_skip' }));
      await skipPromise;

      // Phase 9: Stop Workout
      const stopPromise = connectionService.stopWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_stop' }));
      await stopPromise;

      ws?.simulateMessage(
        JSON.stringify({ type: 'session_state_changed', state: 'completed', session: null })
      );

      expect(useSessionStore.getState().isActive).toBe(false);

      // Phase 10: View Ride History
      const historyPromise = connectionService.fetchRideHistory();
      ws?.simulateMessage(
        JSON.stringify({
          type: 'ride_history',
          rides: [
            {
              id: 'ride-new',
              date: new Date().toISOString(),
              duration_secs: 2400,
              distance_km: 25,
              avg_power_watts: 195,
              workout_name: 'FTP Builder',
              is_workout: true,
            },
          ],
          total: 1,
        })
      );

      // Wait for the history fetch to complete
      try {
        await historyPromise;
      } catch {
        // Ignore any errors from the fetch
      }
      await delay(10);
      expect(useHistoryStore.getState().rides).toHaveLength(1);

      // Phase 11: Disconnect (reset clears pending requests gracefully)
      connectionService.reset();
      expect(useConnectionStore.getState().status).toBe('disconnected');
    });
  });

  describe('Free Ride Flow', () => {
    it('should handle free ride session with resistance adjustments', async () => {
      // Connect and authenticate
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      const authPromise = connectionService.authenticate('123456');
      ws?.simulateMessage(JSON.stringify({ type: 'auth_ok', session_id: 'session-123' }));
      await authPromise;

      // Start free ride session
      ws?.simulateMessage(
        JSON.stringify({
          type: 'session_state_changed',
          state: 'active',
          session: {
            session_id: 'free-ride-001',
            session_type: 'free_ride',
            is_paused: false,
            elapsed_secs: 0,
          },
        })
      );

      const session = useSessionStore.getState();
      expect(session.isActive).toBe(true);
      expect(session.sessionType).toBe('free_ride');

      // Adjust resistance up
      const adjustUpPromise = connectionService.adjustResistance(10);
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' }));
      await adjustUpPromise;

      // Adjust resistance down
      const adjustDownPromise = connectionService.adjustResistance(-5);
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'adjust_resistance' }));
      await adjustDownPromise;

      // Receive continuous metrics
      ws?.simulateMessage(
        JSON.stringify({
          type: 'metrics',
          power_watts: 180,
          heart_rate_bpm: 135,
          cadence_rpm: 90,
          speed_kmh: 32,
          distance_km: 8.5,
          elapsed_secs: 900,
          calories: 200,
        })
      );

      expect(useMetricsStore.getState().metrics.power_watts).toBe(180);

      // Stop free ride
      const stopPromise = connectionService.stopWorkout();
      ws?.simulateMessage(JSON.stringify({ type: 'command_ok', command: 'workout_stop' }));
      await stopPromise;

      ws?.simulateMessage(
        JSON.stringify({ type: 'session_state_changed', state: 'completed', session: null })
      );

      expect(useSessionStore.getState().isActive).toBe(false);
    });
  });

  describe('Ping/Pong Keep-Alive', () => {
    it('should send ping and receive pong', async () => {
      const connectPromise = connectionService.connect('ws://192.168.1.100:9876');
      const ws = MockWebSocket.getLastInstance();
      ws?.simulateOpen();
      await connectPromise;

      ws?.send.mockClear();

      // Send ping manually (simulating keep-alive)
      const pingPromise = connectionService.send({ type: 'ping' });

      expect(ws?.send).toHaveBeenCalledWith(JSON.stringify({ type: 'ping' }));

      ws?.simulateMessage(JSON.stringify({ type: 'pong' }));

      const response = await pingPromise;
      expect(response.type).toBe('pong');
    });
  });
});
