/**
 * RustRide Companion App - UI Components
 *
 * Reusable UI components for the mobile companion app.
 * All components use the theme system for consistent styling.
 */

// MetricCard - displays workout metrics (power, HR, cadence, etc.)
export { MetricCard } from './MetricCard';
export type { MetricCardProps, MetricCardSize } from './MetricCard';

// Button - customizable button with variants
export { Button } from './Button';
export type { ButtonProps, ButtonVariant, ButtonSize } from './Button';

// IconButton - icon-only button for toolbar actions
export { IconButton } from './IconButton';
export type { IconButtonProps, IconButtonVariant, IconButtonSize } from './IconButton';

// ConnectionStatus - connection status indicator
export { ConnectionStatus } from './ConnectionStatus';
export type { ConnectionStatusProps, ConnectionStatusVariant } from './ConnectionStatus';

// LoadingSpinner - loading indicators
export { LoadingSpinner, FullScreenLoader, InlineLoader } from './LoadingSpinner';
export type { LoadingSpinnerProps, LoadingSpinnerSize } from './LoadingSpinner';

// ServerListItem - displays discovered server in a list
export { ServerListItem } from './ServerListItem';
export type { ServerListItemProps } from './ServerListItem';

// ManualEntryModal - modal for manual IP:port entry
export { ManualEntryModal } from './ManualEntryModal';
export type { ManualEntryModalProps } from './ManualEntryModal';
