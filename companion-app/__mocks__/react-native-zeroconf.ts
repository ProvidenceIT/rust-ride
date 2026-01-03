/**
 * Mock for react-native-zeroconf
 *
 * This mock is used in tests to simulate mDNS/Zeroconf service discovery
 * without requiring the actual native module.
 */

interface ZeroconfService {
  name: string;
  fullName: string;
  host: string;
  port: number;
  addresses: string[];
  txt: Record<string, string>;
}

type ZeroconfEventHandler = (...args: unknown[]) => void;

// Store handlers at module level for access across instances
const handlers: Map<string, ZeroconfEventHandler[]> = new Map();

// Store the current instance for test access using a WeakRef-like pattern
let instanceRef: { current: MockZeroconf | null } = { current: null };

class MockZeroconf {
  private services: Map<string, ZeroconfService> = new Map();
  public scan: jest.Mock;
  public stop: jest.Mock;
  public publishService: jest.Mock;
  public unpublishService: jest.Mock;

  constructor() {
    MockZeroconf.setInstance(this);

    // Initialize mock functions in constructor
    this.scan = jest.fn((_type?: string, _protocol?: string) => {
      this.emitEvent('start');
    });

    this.stop = jest.fn(() => {
      this.emitEvent('stop');
    });

    this.publishService = jest.fn();
    this.unpublishService = jest.fn();
  }

  private static setInstance(inst: MockZeroconf): void {
    instanceRef.current = inst;
  }

  on(event: string, handler: ZeroconfEventHandler): void {
    const eventHandlers = handlers.get(event) || [];
    eventHandlers.push(handler);
    handlers.set(event, eventHandlers);
  }

  off(event: string, handler: ZeroconfEventHandler): void {
    const eventHandlers = handlers.get(event) || [];
    const index = eventHandlers.indexOf(handler);
    if (index > -1) {
      eventHandlers.splice(index, 1);
    }
  }

  removeDeviceListeners(): void {
    handlers.clear();
  }

  getServices(): Record<string, ZeroconfService> {
    const result: Record<string, ZeroconfService> = {};
    this.services.forEach((service, name) => {
      result[name] = service;
    });
    return result;
  }

  // Test helpers - emit events to registered handlers
  emitEvent(event: string, ...args: unknown[]): void {
    const eventHandlers = handlers.get(event) || [];
    eventHandlers.forEach(handler => handler(...args));
  }

  simulateServiceResolved(service: ZeroconfService): void {
    this.services.set(service.name, service);
    this.emitEvent('resolved', service);
  }

  simulateServiceRemoved(name: string): void {
    this.services.delete(name);
    this.emitEvent('remove', name);
  }

  simulateError(error: Error): void {
    this.emitEvent('error', error);
  }

  // Static getter for accessing the current instance in tests
  static get instance(): MockZeroconf | null {
    return instanceRef.current;
  }

  static clearInstance(): void {
    instanceRef = { current: null };
    handlers.clear();
  }
}

export default MockZeroconf;
