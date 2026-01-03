/**
 * DiscoveryService - mDNS Service Discovery Manager
 *
 * Handles discovery of RustRide servers on the local network using
 * mDNS (Bonjour/Zeroconf). Integrates with the connection store to
 * provide available servers to the UI.
 */

import Zeroconf, { type ZeroconfService } from 'react-native-zeroconf';
import { useConnectionStore } from '@/stores/connectionStore';
import type { DiscoveredServer } from '@/types';

/**
 * RustRide mDNS service type
 * Matches the service advertised by the desktop app in discovery.rs
 */
const RUSTRIDE_SERVICE_TYPE = '_rustride._tcp.';

/**
 * mDNS protocol domain
 */
const MDNS_PROTOCOL = 'local.';

/**
 * Default scan timeout in milliseconds (30 seconds)
 */
const DEFAULT_SCAN_TIMEOUT_MS = 30000;

/**
 * Minimum scan interval to prevent excessive scanning (5 seconds)
 */
const MIN_SCAN_INTERVAL_MS = 5000;

/**
 * Discovery service event callbacks
 */
export interface DiscoveryServiceCallbacks {
  /** Called when a new server is discovered */
  onServerFound?: (server: DiscoveredServer) => void;
  /** Called when a server is no longer available */
  onServerLost?: (serverName: string) => void;
  /** Called when scanning starts */
  onScanStarted?: () => void;
  /** Called when scanning stops */
  onScanStopped?: () => void;
  /** Called on discovery error */
  onError?: (error: Error) => void;
}

/**
 * Parse TXT records from a Zeroconf service to extract RustRide metadata.
 * TXT records contain key-value pairs like port, version, and protocol.
 *
 * @param txt TXT record object from mDNS
 * @returns Parsed port and version, or null values if not found
 */
function parseTxtRecords(txt: Record<string, string>): {
  port: number | null;
  version: string | null;
} {
  let port: number | null = null;
  let version: string | null = null;

  // Port from TXT record (fallback, usually in service port field)
  if (txt.port) {
    const parsedPort = parseInt(txt.port, 10);
    if (!isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535) {
      port = parsedPort;
    }
  }

  // Version from TXT record
  if (txt.version) {
    version = txt.version;
  }

  return { port, version };
}

/**
 * Convert a ZeroconfService to a DiscoveredServer
 *
 * @param service Zeroconf service from mDNS discovery
 * @returns DiscoveredServer for use in the UI
 */
function zeroconfToDiscoveredServer(service: ZeroconfService): DiscoveredServer {
  const { port: txtPort, version } = parseTxtRecords(service.txt || {});

  // Use the first IPv4 address if available, otherwise use host
  const ipv4Addresses = service.addresses.filter(addr => addr.includes('.') && !addr.includes(':'));
  const host = ipv4Addresses.length > 0 ? ipv4Addresses[0] : service.host || service.addresses[0];

  // Port priority: service.port (from SRV record), then TXT record
  const port = service.port || txtPort || 9876; // Default to 9876 if not specified

  return {
    name: service.name,
    host,
    port,
    version: version ?? undefined,
  };
}

/**
 * DiscoveryService class
 *
 * Singleton service for discovering RustRide servers on the local network
 * using mDNS/Bonjour/Zeroconf. Automatically updates the connection store
 * with discovered servers.
 */
export class DiscoveryService {
  private static instance: DiscoveryService | null = null;

  private zeroconf: Zeroconf | null = null;
  private isScanning = false;
  private scanTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private lastScanTime = 0;
  private callbacks: DiscoveryServiceCallbacks = {};

  /**
   * Private constructor for singleton pattern
   */
  private constructor() {
    this.initializeZeroconf();
  }

  /**
   * Get the singleton instance
   */
  public static getInstance(): DiscoveryService {
    if (!DiscoveryService.instance) {
      DiscoveryService.instance = new DiscoveryService();
    }
    return DiscoveryService.instance;
  }

  /**
   * Initialize the Zeroconf instance and set up event handlers
   */
  private initializeZeroconf(): void {
    try {
      this.zeroconf = new Zeroconf();
      this.setupEventHandlers();
    } catch (error) {
      // Zeroconf may not be available on all platforms (e.g., some simulators)
      const errorMessage = error instanceof Error ? error.message : 'Failed to initialize Zeroconf';
      this.callbacks.onError?.(new Error(errorMessage));
    }
  }

  /**
   * Set up Zeroconf event handlers
   */
  private setupEventHandlers(): void {
    if (!this.zeroconf) return;

    this.zeroconf.on('start', this.handleScanStart.bind(this));
    this.zeroconf.on('stop', this.handleScanStop.bind(this));
    this.zeroconf.on('found', this.handleServiceFound.bind(this));
    this.zeroconf.on('resolved', this.handleServiceResolved.bind(this));
    this.zeroconf.on('remove', this.handleServiceRemoved.bind(this));
    this.zeroconf.on('error', this.handleError.bind(this));
  }

  /**
   * Handle scan start event
   */
  private handleScanStart(): void {
    this.isScanning = true;
    useConnectionStore.getState().setScanning(true);
    this.callbacks.onScanStarted?.();
  }

  /**
   * Handle scan stop event
   */
  private handleScanStop(): void {
    this.isScanning = false;
    useConnectionStore.getState().setScanning(false);
    this.callbacks.onScanStopped?.();
  }

  /**
   * Handle service found event (before resolution)
   */
  private handleServiceFound(_name: string): void {
    // Service found but not yet resolved
    // We wait for the 'resolved' event to get full details
  }

  /**
   * Handle service resolved event (full details available)
   */
  private handleServiceResolved(service: ZeroconfService): void {
    const server = zeroconfToDiscoveredServer(service);

    // Add to connection store
    useConnectionStore.getState().addDiscoveredServer(server);

    // Notify callback
    this.callbacks.onServerFound?.(server);
  }

  /**
   * Handle service removed event
   */
  private handleServiceRemoved(name: string): void {
    // Find and remove the server from the store
    const { discoveredServers, removeDiscoveredServer } = useConnectionStore.getState();

    // Find server by name and remove it
    const serverToRemove = discoveredServers.find(s => s.name === name);
    if (serverToRemove) {
      removeDiscoveredServer(serverToRemove.host, serverToRemove.port);
    }

    // Notify callback
    this.callbacks.onServerLost?.(name);
  }

  /**
   * Handle discovery error
   */
  private handleError(error: Error): void {
    this.callbacks.onError?.(error);
  }

  /**
   * Set callbacks for discovery events
   */
  public setCallbacks(callbacks: DiscoveryServiceCallbacks): void {
    this.callbacks = callbacks;
  }

  /**
   * Start scanning for RustRide servers on the network
   *
   * @param timeoutMs Optional scan timeout in milliseconds (default 30s)
   * @returns Promise that resolves when scan starts, rejects on error
   */
  public startScan(timeoutMs: number = DEFAULT_SCAN_TIMEOUT_MS): Promise<void> {
    return new Promise((resolve, reject) => {
      // Check if Zeroconf is available
      if (!this.zeroconf) {
        reject(new Error('mDNS discovery not available on this device'));
        return;
      }

      // Prevent rapid successive scans
      const now = Date.now();
      if (now - this.lastScanTime < MIN_SCAN_INTERVAL_MS && this.isScanning) {
        resolve(); // Already scanning, no need to start again
        return;
      }

      // Stop any existing scan
      if (this.isScanning) {
        this.stopScan();
      }

      // Clear discovered servers for fresh scan
      useConnectionStore.getState().clearDiscoveredServers();

      this.lastScanTime = now;

      try {
        // Start scanning for RustRide services
        this.zeroconf.scan(RUSTRIDE_SERVICE_TYPE, MDNS_PROTOCOL);

        // Set up auto-stop timeout
        this.scanTimeoutId = setTimeout(() => {
          this.stopScan();
        }, timeoutMs);

        resolve();
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to start scan';
        reject(new Error(errorMessage));
      }
    });
  }

  /**
   * Stop the current scan
   */
  public stopScan(): void {
    // Clear timeout
    if (this.scanTimeoutId) {
      clearTimeout(this.scanTimeoutId);
      this.scanTimeoutId = null;
    }

    // Stop Zeroconf scanning
    if (this.zeroconf && this.isScanning) {
      try {
        this.zeroconf.stop();
      } catch {
        // Ignore errors when stopping
      }
    }

    this.isScanning = false;
    useConnectionStore.getState().setScanning(false);
  }

  /**
   * Get all currently discovered servers
   *
   * @returns Array of discovered servers from the connection store
   */
  public getDiscoveredServers(): DiscoveredServer[] {
    return useConnectionStore.getState().discoveredServers;
  }

  /**
   * Check if currently scanning
   */
  public isScanningActive(): boolean {
    return this.isScanning;
  }

  /**
   * Manually add a server (for manual IP entry)
   *
   * @param server Server to add
   */
  public addManualServer(server: DiscoveredServer): void {
    useConnectionStore.getState().addDiscoveredServer(server);
  }

  /**
   * Build a WebSocket URL for a discovered server
   *
   * @param server The discovered server
   * @returns WebSocket URL string (e.g., "ws://192.168.1.100:9876")
   */
  public buildServerUrl(server: DiscoveredServer): string {
    return `ws://${server.host}:${server.port}`;
  }

  /**
   * Refresh the server list by restarting the scan
   */
  public async refresh(): Promise<void> {
    await this.startScan();
  }

  /**
   * Clean up resources
   */
  public cleanup(): void {
    this.stopScan();

    if (this.zeroconf) {
      try {
        this.zeroconf.removeDeviceListeners();
      } catch {
        // Ignore cleanup errors
      }
      this.zeroconf = null;
    }

    this.callbacks = {};
  }

  /**
   * Reset the discovery service (for testing)
   */
  public reset(): void {
    this.cleanup();
    this.initializeZeroconf();
    useConnectionStore.getState().clearDiscoveredServers();
  }
}

/**
 * Get the singleton DiscoveryService instance
 */
export function getDiscoveryService(): DiscoveryService {
  return DiscoveryService.getInstance();
}
