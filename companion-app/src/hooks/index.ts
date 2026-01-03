/**
 * Custom Hooks
 *
 * Re-exports all custom hooks for the companion app.
 */

export { useAuthentication } from './useAuthentication';
export type { AuthenticationState, AuthenticationActions, UseAuthenticationReturn } from './useAuthentication';

export { useAutoReconnect } from './useAutoReconnect';
export type { AutoReconnectState, AutoReconnectActions, UseAutoReconnectReturn } from './useAutoReconnect';
