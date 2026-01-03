/**
 * ConnectionService - WebSocket Connection Manager
 *
 * Handles WebSocket connection to the RustRide desktop app with:
 * - Connection and authentication
 * - Automatic reconnection with exponential backoff
 * - Message parsing and store updates
 * - Request/response correlation
 */

import { useConnectionStore } from '@/stores/connectionStore';
import { useMetricsStore } from '@/stores/metricsStore';
import { useSessionStore } from '@/stores/sessionStore';
import { useHistoryStore } from '@/stores/historyStore';
import type {
  CompanionRequest,
  CompanionResponse,
  CompanionEvent,
  ServerMessage,
  LiveMetrics,
  RideSummary,
  RideDetailInfo,
} from '@/types';
import { isCompanionEvent } from '@/types';

/**
 * Backoff configuration for reconnection
 */
interface BackoffConfig {
  /** Initial delay in milliseconds */
  initialDelayMs: number;
  /** Maximum delay in milliseconds */
  maxDelayMs: number;
  /** Multiplier for exponential backoff */
  multiplier: number;
}

/**
 * Default backoff configuration
 * Starts at 1s, doubles each time, max 30s
 */
const DEFAULT_BACKOFF: BackoffConfig = {
  initialDelayMs: 1000,
  maxDelayMs: 30000,
  multiplier: 2,
};

/**
 * Ping interval to keep connection alive (30 seconds)
 */
const PING_INTERVAL_MS = 30000;

/**
 * Pending request with resolve/reject handlers
 */
interface PendingRequest {
  type: string;
  resolve: (response: CompanionResponse) => void;
  reject: (error: Error) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

/**
 * Request timeout in milliseconds
 */
const REQUEST_TIMEOUT_MS = 10000;

/**
 * Connection service event callbacks
 */
export interface ConnectionServiceCallbacks {
  /** Called when authentication is required */
  onAuthRequired?: () => void;
  /** Called when authentication fails */
  onAuthFailed?: (reason: string) => void;
  /** Called when connection is lost */
  onDisconnected?: (reason: string) => void;
  /** Called on connection error */
  onError?: (error: Error) => void;
}

/**
 * ConnectionService class
 *
 * Singleton service for managing WebSocket connection to the RustRide desktop app.
 * Handles connection lifecycle, message routing, and store updates.
 */
export class ConnectionService {
  private static instance: ConnectionService | null = null;

  private ws: WebSocket | null = null;
  private serverUrl: string | null = null;
  private reconnectAttempt = 0;
  private reconnectTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private pingIntervalId: ReturnType<typeof setInterval> | null = null;
  private pendingRequests: Map<string, PendingRequest> = new Map();
  private requestIdCounter = 0;
  private callbacks: ConnectionServiceCallbacks = {};
  private isIntentionalDisconnect = false;
  private backoffConfig: BackoffConfig = DEFAULT_BACKOFF;

  /**
   * Private constructor for singleton pattern
   */
  private constructor() {}

  /**
   * Get the singleton instance
   */
  public static getInstance(): ConnectionService {
    if (!ConnectionService.instance) {
      ConnectionService.instance = new ConnectionService();
    }
    return ConnectionService.instance;
  }

  /**
   * Set callbacks for connection events
   */
  public setCallbacks(callbacks: ConnectionServiceCallbacks): void {
    this.callbacks = callbacks;
  }

  /**
   * Connect to a RustRide server
   * @param url WebSocket URL (e.g., ws://192.168.1.100:9876)
   */
  public connect(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.disconnect();
      }

      this.serverUrl = url;
      this.isIntentionalDisconnect = false;
      this.reconnectAttempt = 0;

      // Update connection store
      const connectionStore = useConnectionStore.getState();
      connectionStore.connect(url);

      try {
        this.ws = new WebSocket(url);
        this.setupWebSocketHandlers(resolve, reject);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        connectionStore.setError('CONNECTION_FAILED', errorMessage);
        reject(error);
      }
    });
  }

  /**
   * Set up WebSocket event handlers
   */
  private setupWebSocketHandlers(
    onConnected: () => void,
    onConnectFailed: (error: Error) => void,
  ): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      const connectionStore = useConnectionStore.getState();
      connectionStore.setConnected();
      this.reconnectAttempt = 0;
      this.startPingInterval();
      onConnected();
    };

    this.ws.onclose = event => {
      this.handleDisconnect(event.reason || 'Connection closed');
    };

    this.ws.onerror = () => {
      const error = new Error('WebSocket connection error');
      const connectionStore = useConnectionStore.getState();

      if (connectionStore.status === 'connecting') {
        connectionStore.setError('CONNECTION_ERROR', error.message);
        onConnectFailed(error);
      }

      this.callbacks.onError?.(error);
    };

    this.ws.onmessage = event => {
      this.handleMessage(event.data);
    };
  }

  /**
   * Handle WebSocket disconnection
   */
  private handleDisconnect(reason: string): void {
    this.stopPingInterval();
    this.clearPendingRequests('Connection closed');

    const connectionStore = useConnectionStore.getState();
    connectionStore.disconnect();

    // Reset metrics subscription state
    const metricsStore = useMetricsStore.getState();
    metricsStore.setSubscribed(false);

    this.callbacks.onDisconnected?.(reason);

    // Attempt reconnection if not intentional
    if (!this.isIntentionalDisconnect && this.serverUrl) {
      this.scheduleReconnect();
    }
  }

  /**
   * Schedule a reconnection attempt with exponential backoff
   */
  private scheduleReconnect(): void {
    const connectionStore = useConnectionStore.getState();

    // Check if we've exceeded max attempts
    if (!connectionStore.reconnectAttempts) {
      connectionStore.resetReconnectAttempts();
    }

    if (connectionStore.reconnectAttempts >= connectionStore.maxReconnectAttempts) {
      connectionStore.setError('MAX_RECONNECT_ATTEMPTS', 'Maximum reconnection attempts reached');
      return;
    }

    // Calculate backoff delay
    const delay = Math.min(
      this.backoffConfig.initialDelayMs *
        Math.pow(this.backoffConfig.multiplier, this.reconnectAttempt),
      this.backoffConfig.maxDelayMs,
    );

    this.reconnectAttempt++;
    connectionStore.incrementReconnectAttempts();

    this.reconnectTimeoutId = setTimeout(() => {
      if (this.serverUrl && !this.isIntentionalDisconnect) {
        this.connect(this.serverUrl).catch(() => {
          // Error handling is done in connect()
        });
      }
    }, delay);
  }

  /**
   * Disconnect from the server
   */
  public disconnect(): void {
    this.isIntentionalDisconnect = true;
    this.cancelReconnect();
    this.stopPingInterval();
    this.clearPendingRequests('Disconnected by user');

    if (this.ws) {
      this.ws.close(1000, 'User disconnected');
      this.ws = null;
    }

    const connectionStore = useConnectionStore.getState();
    connectionStore.disconnect();

    // Reset stores
    const metricsStore = useMetricsStore.getState();
    metricsStore.reset();

    const sessionStore = useSessionStore.getState();
    sessionStore.reset();
  }

  /**
   * Cancel any pending reconnection
   */
  private cancelReconnect(): void {
    if (this.reconnectTimeoutId) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = null;
    }
  }

  /**
   * Start ping interval to keep connection alive
   */
  private startPingInterval(): void {
    this.stopPingInterval();
    this.pingIntervalId = setInterval(() => {
      this.send({ type: 'ping' }).catch(() => {
        // Ping failed, connection may be dead
      });
    }, PING_INTERVAL_MS);
  }

  /**
   * Stop ping interval
   */
  private stopPingInterval(): void {
    if (this.pingIntervalId) {
      clearInterval(this.pingIntervalId);
      this.pingIntervalId = null;
    }
  }

  /**
   * Clear all pending requests with an error
   */
  private clearPendingRequests(reason: string): void {
    for (const [, pending] of this.pendingRequests) {
      clearTimeout(pending.timeoutId);
      pending.reject(new Error(reason));
    }
    this.pendingRequests.clear();
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data) as ServerMessage;

      if (isCompanionEvent(message)) {
        this.handleEvent(message);
      } else {
        this.handleResponse(message);
      }
    } catch {
      // Invalid JSON, ignore
    }
  }

  /**
   * Handle event messages from server
   */
  private handleEvent(event: CompanionEvent): void {
    switch (event.type) {
      case 'metrics':
        this.handleMetricsEvent(event);
        break;

      case 'session_state_changed':
        this.handleSessionStateChangedEvent(event);
        break;

      case 'interval_changed':
        this.handleIntervalChangedEvent(event);
        break;

      case 'disconnecting':
        // Server is disconnecting us
        this.callbacks.onDisconnected?.(event.reason);
        break;
    }
  }

  /**
   * Handle metrics event from server
   */
  private handleMetricsEvent(event: Extract<CompanionEvent, { type: 'metrics' }>): void {
    const metricsStore = useMetricsStore.getState();

    const metrics: LiveMetrics = {
      power_watts: event.power_watts ?? 0,
      heart_rate_bpm: event.heart_rate_bpm,
      cadence_rpm: event.cadence_rpm,
      speed_kph: event.speed_kmh ?? 0,
      distance_km: event.distance_km,
      calories: event.calories,
    };

    metricsStore.updateMetrics(metrics);

    // Also update session elapsed time
    const sessionStore = useSessionStore.getState();
    if (sessionStore.isActive) {
      sessionStore.updateElapsedTime(event.elapsed_secs);
    }
  }

  /**
   * Handle session state changed event
   */
  private handleSessionStateChangedEvent(
    event: Extract<CompanionEvent, { type: 'session_state_changed' }>,
  ): void {
    const sessionStore = useSessionStore.getState();

    switch (event.state) {
      case 'active':
        if (event.session) {
          sessionStore.startSession(event.session);
        }
        break;

      case 'paused':
        sessionStore.setPaused(true);
        break;

      case 'idle':
      case 'completed':
      case 'stopping':
        sessionStore.endSession();
        break;

      case 'starting':
        // Session starting, wait for active
        break;
    }
  }

  /**
   * Handle interval changed event
   */
  private handleIntervalChangedEvent(
    event: Extract<CompanionEvent, { type: 'interval_changed' }>,
  ): void {
    const sessionStore = useSessionStore.getState();
    const metricsStore = useMetricsStore.getState();

    sessionStore.updateInterval({
      index: event.interval_index,
      total: event.total_intervals,
      name: event.interval_name,
      remainingSecs: event.duration_secs,
    });

    sessionStore.setTargetPower(event.target_power_watts);
    metricsStore.setTargetPower(event.target_power_watts);
  }

  /**
   * Handle response messages from server
   */
  private handleResponse(response: CompanionResponse): void {
    // Check for pending requests that match this response
    const pendingKey = this.findPendingRequestKey(response);
    if (pendingKey) {
      const pending = this.pendingRequests.get(pendingKey);
      if (pending) {
        clearTimeout(pending.timeoutId);
        this.pendingRequests.delete(pendingKey);
        pending.resolve(response);
        // Note: Don't return early - we still need to process state updates for auth responses
      }
    }

    // Handle specific response types that update state
    switch (response.type) {
      case 'auth_ok':
        this.handleAuthOk(response);
        break;

      case 'auth_failed':
        this.handleAuthFailed(response);
        break;

      case 'session_status':
        this.handleSessionStatus(response);
        break;

      case 'subscribed_metrics':
        useMetricsStore.getState().setSubscribed(true);
        break;

      case 'unsubscribed_metrics':
        useMetricsStore.getState().setSubscribed(false);
        break;

      case 'ride_history':
        this.handleRideHistory(response);
        break;

      case 'ride_details':
        this.handleRideDetails(response);
        break;

      case 'error':
        this.handleError(response);
        break;

      case 'command_ok':
      case 'command_failed':
      case 'pong':
        // These are typically handled by pending requests
        break;
    }
  }

  /**
   * Find the pending request key that matches a response
   */
  private findPendingRequestKey(response: CompanionResponse): string | undefined {
    // Map response types to request types
    const responseToRequestType: Record<string, string> = {
      auth_ok: 'auth',
      auth_failed: 'auth',
      session_status: 'get_session_status',
      subscribed_metrics: 'subscribe_metrics',
      unsubscribed_metrics: 'unsubscribe_metrics',
      ride_history: 'get_ride_history',
      ride_details: 'get_ride_details',
      pong: 'ping',
    };

    const requestType = responseToRequestType[response.type];
    if (requestType) {
      // Find the oldest pending request of this type
      for (const [key, pending] of this.pendingRequests) {
        if (pending.type === requestType) {
          return key;
        }
      }
    }

    // Handle command responses
    if (response.type === 'command_ok' || response.type === 'command_failed') {
      const command = response.command;
      for (const [key, pending] of this.pendingRequests) {
        if (pending.type === command) {
          return key;
        }
      }
    }

    return undefined;
  }

  /**
   * Handle auth_ok response
   */
  private handleAuthOk(response: Extract<CompanionResponse, { type: 'auth_ok' }>): void {
    const connectionStore = useConnectionStore.getState();
    connectionStore.setAuthenticated();

    // Auto-subscribe to metrics after authentication
    this.send({ type: 'subscribe_metrics' }).catch(() => {
      // Subscription failed, will retry on user action
    });

    // Request current session status
    this.send({ type: 'get_session_status' }).catch(() => {
      // Status request failed
    });

    // Store session ID if needed (response.session_id is available)
    void response.session_id;
  }

  /**
   * Handle auth_failed response
   */
  private handleAuthFailed(response: Extract<CompanionResponse, { type: 'auth_failed' }>): void {
    const connectionStore = useConnectionStore.getState();
    connectionStore.setError('AUTH_FAILED', response.reason);
    this.callbacks.onAuthFailed?.(response.reason);
  }

  /**
   * Handle session_status response
   */
  private handleSessionStatus(
    response: Extract<CompanionResponse, { type: 'session_status' }>,
  ): void {
    const sessionStore = useSessionStore.getState();

    if (response.active && response.session) {
      sessionStore.updateStatus(response.session);

      // Update target power in metrics store
      if (response.session.target_power_watts) {
        const metricsStore = useMetricsStore.getState();
        metricsStore.setTargetPower(response.session.target_power_watts);
      }
    } else {
      sessionStore.reset();
    }
  }

  /**
   * Handle ride_history response
   */
  private handleRideHistory(response: Extract<CompanionResponse, { type: 'ride_history' }>): void {
    const historyStore = useHistoryStore.getState();

    // Map server RideSummary to client RideSummary format
    const rides: RideSummary[] = response.rides.map(ride => ({
      id: ride.id,
      date: ride.date,
      duration_secs: ride.duration_secs,
      distance_km: ride.distance_km,
      avg_power_watts: ride.avg_power_watts,
      workout_name: ride.workout_name,
      is_workout: ride.is_workout,
    }));

    // Check if this is appending (loading more) or replacing
    const isAppending = historyStore.isLoadingMore;
    historyStore.setRides(rides, response.total, isAppending);
  }

  /**
   * Handle ride_details response
   */
  private handleRideDetails(response: Extract<CompanionResponse, { type: 'ride_details' }>): void {
    const historyStore = useHistoryStore.getState();
    const detail: RideDetailInfo = response.ride;
    historyStore.setCurrentRideDetail(detail);
  }

  /**
   * Handle error response
   */
  private handleError(response: Extract<CompanionResponse, { type: 'error' }>): void {
    const connectionStore = useConnectionStore.getState();

    if (response.code === 'AUTH_REQUIRED') {
      this.callbacks.onAuthRequired?.();
    } else {
      connectionStore.setError(response.code, response.message);
    }
  }

  /**
   * Send a request to the server
   * @param request The request to send
   * @returns Promise that resolves with the response
   */
  public send(request: CompanionRequest): Promise<CompanionResponse> {
    return new Promise((resolve, reject) => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
        reject(new Error('Not connected'));
        return;
      }

      try {
        const requestId = `${request.type}_${++this.requestIdCounter}`;

        // Set up timeout for this request
        const timeoutId = setTimeout(() => {
          this.pendingRequests.delete(requestId);
          reject(new Error('Request timeout'));
        }, REQUEST_TIMEOUT_MS);

        // Store pending request
        this.pendingRequests.set(requestId, {
          type: request.type,
          resolve,
          reject,
          timeoutId,
        });

        // Send the request
        this.ws.send(JSON.stringify(request));
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Authenticate with the server using a PIN
   * @param pin The 6-digit PIN
   */
  public async authenticate(pin: string): Promise<void> {
    const response = await this.send({ type: 'auth', pin });

    if (response.type === 'auth_failed') {
      throw new Error(response.reason);
    }
  }

  /**
   * Subscribe to real-time metrics
   */
  public async subscribeMetrics(): Promise<void> {
    await this.send({ type: 'subscribe_metrics' });
  }

  /**
   * Unsubscribe from real-time metrics
   */
  public async unsubscribeMetrics(): Promise<void> {
    await this.send({ type: 'unsubscribe_metrics' });
  }

  /**
   * Get current session status
   */
  public async getSessionStatus(): Promise<void> {
    await this.send({ type: 'get_session_status' });
  }

  /**
   * Pause the current workout
   */
  public async pauseWorkout(): Promise<void> {
    const response = await this.send({ type: 'workout_pause' });
    if (response.type === 'command_failed') {
      throw new Error(response.error);
    }
  }

  /**
   * Resume the current workout
   */
  public async resumeWorkout(): Promise<void> {
    const response = await this.send({ type: 'workout_resume' });
    if (response.type === 'command_failed') {
      throw new Error(response.error);
    }
  }

  /**
   * Skip to the next interval
   */
  public async skipInterval(): Promise<void> {
    const response = await this.send({ type: 'workout_skip' });
    if (response.type === 'command_failed') {
      throw new Error(response.error);
    }
  }

  /**
   * Stop the current workout/ride
   */
  public async stopWorkout(): Promise<void> {
    const response = await this.send({ type: 'workout_stop' });
    if (response.type === 'command_failed') {
      throw new Error(response.error);
    }
  }

  /**
   * Adjust resistance (for free rides)
   * @param delta Change in resistance (-100 to 100)
   */
  public async adjustResistance(delta: number): Promise<void> {
    const response = await this.send({ type: 'adjust_resistance', delta });
    if (response.type === 'command_failed') {
      throw new Error(response.error);
    }
  }

  /**
   * Fetch ride history
   * @param limit Maximum number of rides to fetch
   * @param offset Offset for pagination
   */
  public async fetchRideHistory(limit: number = 20, offset: number = 0): Promise<void> {
    const historyStore = useHistoryStore.getState();

    if (offset === 0) {
      historyStore.setLoading(true);
    } else {
      historyStore.setLoadingMore(true);
    }

    try {
      await this.send({ type: 'get_ride_history', limit, offset });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch ride history';
      historyStore.setError(errorMessage);
    }
  }

  /**
   * Fetch details for a specific ride
   * @param rideId The ride ID to fetch
   */
  public async fetchRideDetails(rideId: string): Promise<void> {
    const historyStore = useHistoryStore.getState();

    // Check cache first
    const cached = historyStore.getCachedRideDetail(rideId);
    if (cached) {
      historyStore.setCurrentRideDetail(cached);
      return;
    }

    historyStore.setLoadingDetail(true);

    try {
      await this.send({ type: 'get_ride_details', ride_id: rideId });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch ride details';
      historyStore.setError(errorMessage);
      historyStore.setLoadingDetail(false);
    }
  }

  /**
   * Check if currently connected
   */
  public isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Check if authenticated
   */
  public isAuthenticated(): boolean {
    return useConnectionStore.getState().isAuthenticated;
  }

  /**
   * Get the current server URL
   */
  public getServerUrl(): string | null {
    return this.serverUrl;
  }

  /**
   * Configure backoff settings
   */
  public setBackoffConfig(config: Partial<BackoffConfig>): void {
    this.backoffConfig = { ...this.backoffConfig, ...config };
  }

  /**
   * Reset the connection service (for testing)
   */
  public reset(): void {
    this.disconnect();
    this.serverUrl = null;
    this.reconnectAttempt = 0;
    this.requestIdCounter = 0;
    this.callbacks = {};
    this.backoffConfig = DEFAULT_BACKOFF;
  }
}

/**
 * Get the singleton ConnectionService instance
 */
export function getConnectionService(): ConnectionService {
  return ConnectionService.getInstance();
}
