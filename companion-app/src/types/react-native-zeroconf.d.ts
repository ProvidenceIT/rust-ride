/**
 * Type definitions for react-native-zeroconf
 *
 * Provides TypeScript types for the react-native-zeroconf library
 * which handles mDNS/Bonjour service discovery on local networks.
 */

declare module 'react-native-zeroconf' {
  /**
   * Resolved service information from mDNS discovery
   */
  export interface ZeroconfService {
    /** Service name (e.g., "RustRide") */
    name: string;
    /** Full service type (e.g., "_rustride._tcp.local.") */
    fullName: string;
    /** Hostname of the service */
    host: string;
    /** Port number the service is running on */
    port: number;
    /** Array of IP addresses for the service */
    addresses: string[];
    /** TXT record key-value pairs */
    txt: Record<string, string>;
  }

  /**
   * Events emitted by Zeroconf
   */
  export type ZeroconfEvent =
    | 'start'
    | 'stop'
    | 'found'
    | 'resolved'
    | 'remove'
    | 'update'
    | 'error';

  /**
   * Event handler types for each Zeroconf event
   */
  export interface ZeroconfEventHandlers {
    start: () => void;
    stop: () => void;
    found: (name: string) => void;
    resolved: (service: ZeroconfService) => void;
    remove: (name: string) => void;
    update: () => void;
    error: (error: Error) => void;
  }

  /**
   * Zeroconf class for mDNS/Bonjour service discovery
   *
   * @example
   * ```typescript
   * const zeroconf = new Zeroconf();
   * zeroconf.on('resolved', (service) => {
   *   console.log('Found service:', service.name);
   * });
   * zeroconf.scan('_rustride._tcp.', 'local.');
   * ```
   */
  export default class Zeroconf {
    /**
     * Register an event listener
     * @param event Event name to listen for
     * @param handler Callback function for the event
     */
    on<E extends ZeroconfEvent>(event: E, handler: ZeroconfEventHandlers[E]): void;

    /**
     * Remove an event listener
     * @param event Event name to remove listener from
     * @param handler Handler function to remove
     */
    off<E extends ZeroconfEvent>(event: E, handler: ZeroconfEventHandlers[E]): void;

    /**
     * Remove all event listeners
     */
    removeDeviceListeners(): void;

    /**
     * Start scanning for services of a specific type
     * @param type Service type to scan for (e.g., "_rustride._tcp.")
     * @param protocol Protocol domain (e.g., "local.")
     */
    scan(type?: string, protocol?: string): void;

    /**
     * Stop the current scan
     */
    stop(): void;

    /**
     * Get all currently discovered services
     * @returns Object mapping service names to service info
     */
    getServices(): Record<string, ZeroconfService>;

    /**
     * Publish/advertise a service on the network
     * @param type Service type (e.g., "_http._tcp.")
     * @param protocol Protocol domain (e.g., "local.")
     * @param name Service name
     * @param port Port number
     */
    publishService(type: string, protocol: string, name: string, port: number): void;

    /**
     * Unpublish/stop advertising a service
     * @param name Service name to unpublish
     */
    unpublishService(name: string): void;
  }
}
