/**
 * DiscoveryService Unit Tests
 *
 * Uses manual mock for react-native-zeroconf via jest.config.js moduleNameMapper
 */

import MockZeroconf from '../../__mocks__/react-native-zeroconf';
import { DiscoveryService, getDiscoveryService } from '../../src/services/DiscoveryService';
import { useConnectionStore } from '../../src/stores/connectionStore';

// Define ZeroconfService interface for testing
interface ZeroconfService {
  name: string;
  fullName: string;
  host: string;
  port: number;
  addresses: string[];
  txt: Record<string, string>;
}

describe('DiscoveryService', () => {
  let service: DiscoveryService;

  beforeEach(() => {
    // Reset stores
    useConnectionStore.getState().reset();

    // Clear mock
    MockZeroconf.clearInstance();

    // Get fresh service instance - need to reset singleton
    // Access private static to reset
    (DiscoveryService as unknown as { instance: DiscoveryService | null }).instance = null;
    service = getDiscoveryService();
  });

  afterEach(() => {
    service.cleanup();
    jest.clearAllMocks();
  });

  describe('getInstance', () => {
    it('should return singleton instance', () => {
      const instance1 = DiscoveryService.getInstance();
      const instance2 = DiscoveryService.getInstance();
      expect(instance1).toBe(instance2);
    });
  });

  describe('startScan', () => {
    it('should start scanning for RustRide services', async () => {
      await service.startScan();

      const zeroconf = MockZeroconf.instance;
      expect(zeroconf?.scan).toHaveBeenCalledWith('_rustride._tcp.', 'local.');
    });

    it('should update connection store scanning state', async () => {
      await service.startScan();

      expect(useConnectionStore.getState().isScanning).toBe(true);
    });

    it('should clear discovered servers on new scan', async () => {
      // Add a server first
      useConnectionStore.getState().addDiscoveredServer({
        name: 'Old Server',
        host: '192.168.1.50',
        port: 9876,
      });

      expect(useConnectionStore.getState().discoveredServers).toHaveLength(1);

      await service.startScan();

      // Should clear before starting new scan
      expect(useConnectionStore.getState().discoveredServers).toHaveLength(0);
    });

    it('should call onScanStarted callback', async () => {
      const onScanStarted = jest.fn();
      service.setCallbacks({ onScanStarted });

      await service.startScan();

      expect(onScanStarted).toHaveBeenCalled();
    });
  });

  describe('stopScan', () => {
    it('should stop scanning', async () => {
      await service.startScan();
      service.stopScan();

      const zeroconf = MockZeroconf.instance;
      expect(zeroconf?.stop).toHaveBeenCalled();
    });

    it('should update connection store scanning state', async () => {
      await service.startScan();
      expect(useConnectionStore.getState().isScanning).toBe(true);

      service.stopScan();
      expect(useConnectionStore.getState().isScanning).toBe(false);
    });
  });

  describe('service discovery events', () => {
    beforeEach(async () => {
      await service.startScan();
    });

    it('should add discovered server to store on resolved event', () => {
      const zeroconf = MockZeroconf.instance;

      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {
          version: '1.0.0',
        },
      };

      zeroconf?.simulateServiceResolved(mockService);

      const servers = useConnectionStore.getState().discoveredServers;
      expect(servers).toHaveLength(1);
      expect(servers[0]).toEqual({
        name: 'RustRide Server',
        host: '192.168.1.100',
        port: 9876,
        version: '1.0.0',
      });
    });

    it('should prefer IPv4 addresses', () => {
      const zeroconf = MockZeroconf.instance;

      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['fe80::1', '192.168.1.100', '10.0.0.1'],
        txt: {},
      };

      zeroconf?.simulateServiceResolved(mockService);

      const servers = useConnectionStore.getState().discoveredServers;
      expect(servers[0].host).toBe('192.168.1.100');
    });

    it('should parse TXT records for port and version', () => {
      const zeroconf = MockZeroconf.instance;

      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {
          port: '9877',
          version: '2.0.0',
        },
      };

      zeroconf?.simulateServiceResolved(mockService);

      const servers = useConnectionStore.getState().discoveredServers;
      // Port from service should take precedence over TXT record
      expect(servers[0].port).toBe(9876);
      expect(servers[0].version).toBe('2.0.0');
    });

    it('should call onServerFound callback', () => {
      const onServerFound = jest.fn();
      service.setCallbacks({ onServerFound });

      const zeroconf = MockZeroconf.instance;

      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {},
      };

      zeroconf?.simulateServiceResolved(mockService);

      expect(onServerFound).toHaveBeenCalledWith({
        name: 'RustRide Server',
        host: '192.168.1.100',
        port: 9876,
        version: undefined,
      });
    });

    it('should remove server from store on remove event', () => {
      const zeroconf = MockZeroconf.instance;

      // First add a server
      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {},
      };

      zeroconf?.simulateServiceResolved(mockService);
      expect(useConnectionStore.getState().discoveredServers).toHaveLength(1);

      // Then remove it
      zeroconf?.simulateServiceRemoved('RustRide Server');
      expect(useConnectionStore.getState().discoveredServers).toHaveLength(0);
    });

    it('should call onServerLost callback on remove event', () => {
      const onServerLost = jest.fn();
      service.setCallbacks({ onServerLost });

      const zeroconf = MockZeroconf.instance;

      // First add a server
      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {},
      };

      zeroconf?.simulateServiceResolved(mockService);

      // Then remove it
      zeroconf?.simulateServiceRemoved('RustRide Server');

      expect(onServerLost).toHaveBeenCalledWith('RustRide Server');
    });
  });

  describe('error handling', () => {
    it('should call onError callback on error event', async () => {
      const onError = jest.fn();
      service.setCallbacks({ onError });

      await service.startScan();

      const zeroconf = MockZeroconf.instance;
      const error = new Error('Discovery failed');
      zeroconf?.simulateError(error);

      expect(onError).toHaveBeenCalledWith(error);
    });
  });

  describe('helper methods', () => {
    it('isScanningActive should return correct state', async () => {
      expect(service.isScanningActive()).toBe(false);

      await service.startScan();
      expect(service.isScanningActive()).toBe(true);

      service.stopScan();
      expect(service.isScanningActive()).toBe(false);
    });

    it('getDiscoveredServers should return servers from store', async () => {
      await service.startScan();

      const zeroconf = MockZeroconf.instance;
      const mockService: ZeroconfService = {
        name: 'RustRide Server',
        fullName: '_rustride._tcp.local.',
        host: 'rustride.local',
        port: 9876,
        addresses: ['192.168.1.100'],
        txt: {},
      };

      zeroconf?.simulateServiceResolved(mockService);

      const servers = service.getDiscoveredServers();
      expect(servers).toHaveLength(1);
      expect(servers[0].name).toBe('RustRide Server');
    });

    it('addManualServer should add server to store', () => {
      service.addManualServer({
        name: 'Manual Server',
        host: '10.0.0.1',
        port: 9876,
      });

      const servers = useConnectionStore.getState().discoveredServers;
      expect(servers).toHaveLength(1);
      expect(servers[0].name).toBe('Manual Server');
    });

    it('buildServerUrl should return correct WebSocket URL', () => {
      const url = service.buildServerUrl({
        name: 'Test',
        host: '192.168.1.100',
        port: 9876,
      });

      expect(url).toBe('ws://192.168.1.100:9876');
    });
  });

  describe('scan timeout', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('should auto-stop scan after timeout', async () => {
      await service.startScan(5000); // 5 second timeout

      expect(service.isScanningActive()).toBe(true);

      // Advance time past timeout
      jest.advanceTimersByTime(5000);

      expect(service.isScanningActive()).toBe(false);
    });
  });

  describe('refresh', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('should restart the scan', async () => {
      await service.startScan();

      const zeroconf = MockZeroconf.instance;
      expect(zeroconf?.scan).toHaveBeenCalledTimes(1);

      // Advance time past the minimum scan interval (5 seconds)
      jest.advanceTimersByTime(6000);

      await service.refresh();

      expect(zeroconf?.stop).toHaveBeenCalled();
      expect(zeroconf?.scan).toHaveBeenCalledTimes(2);
    });
  });

  describe('cleanup', () => {
    it('should stop scan and remove listeners', async () => {
      await service.startScan();

      const zeroconf = MockZeroconf.instance;
      service.cleanup();

      expect(zeroconf?.stop).toHaveBeenCalled();
    });
  });
});
