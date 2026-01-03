/**
 * RustRide Companion App - Services
 *
 * Service layer for network communication and external integrations.
 */

// Connection service - WebSocket connection management
export {
  ConnectionService,
  getConnectionService,
  type ConnectionServiceCallbacks,
} from './ConnectionService';

// Discovery service - mDNS service discovery for RustRide servers
export {
  DiscoveryService,
  getDiscoveryService,
  type DiscoveryServiceCallbacks,
} from './DiscoveryService';

// Storage service - Persistent storage for connection preferences
export {
  StorageService,
  getStorageService,
  type StoredServer,
  type ConnectionPreferences,
} from './StorageService';
