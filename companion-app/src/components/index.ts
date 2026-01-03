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

// QRScannerModal - modal for QR code scanning
export { QRScannerModal } from './QRScannerModal';
export type { QRScannerModalProps } from './QRScannerModal';

// PinEntryModal - modal for PIN entry during authentication
export { PinEntryModal } from './PinEntryModal';
export type { PinEntryModalProps } from './PinEntryModal';

// PowerDisplay - large power display with zone indicator for dashboard
export { PowerDisplay } from './PowerDisplay';
export type { PowerDisplayProps } from './PowerDisplay';

// HeartRateDisplay - large heart rate display with zone indicator and pulse animation for dashboard
export { HeartRateDisplay } from './HeartRateDisplay';
export type { HeartRateDisplayProps } from './HeartRateDisplay';

// CadenceDisplay - cadence display with target and visual warning when outside range
export { CadenceDisplay } from './CadenceDisplay';
export type { CadenceDisplayProps } from './CadenceDisplay';

// Secondary Metrics - smaller cards for speed, distance, time, and calories
export {
  SpeedDisplay,
  DistanceDisplay,
  ElapsedTimeDisplay,
  CaloriesDisplay,
} from './SecondaryMetrics';
export type {
  SpeedDisplayProps,
  DistanceDisplayProps,
  ElapsedTimeDisplayProps,
  CaloriesDisplayProps,
} from './SecondaryMetrics';

// WorkoutIntervalDisplay - shows workout interval info, time remaining, and progress
export { WorkoutIntervalDisplay } from './WorkoutIntervalDisplay';
export type {
  WorkoutIntervalDisplayProps,
  IntervalInfo,
  NextIntervalInfo,
} from './WorkoutIntervalDisplay';

// NoSessionState - shows appropriate UI when no workout/ride is active
export { NoSessionState } from './NoSessionState';
export type { NoSessionStateProps } from './NoSessionState';

// WorkoutControlBar - fixed bottom bar with play/pause, skip, and stop buttons
export { WorkoutControlBar } from './WorkoutControlBar';
export type { WorkoutControlBarProps } from './WorkoutControlBar';

// Toast - notification toast for user feedback
export { Toast } from './Toast';
export type { ToastProps, ToastData, ToastVariant } from './Toast';

// ToastContainer - renders all active toasts
export { ToastContainer } from './ToastContainer';
export type { ToastContainerProps } from './ToastContainer';

// StopConfirmationModal - confirmation dialog before stopping session
export { StopConfirmationModal } from './StopConfirmationModal';
export type { StopConfirmationModalProps } from './StopConfirmationModal';

// ResistanceControl - +/- buttons for adjusting trainer resistance during free rides
export { ResistanceControl } from './ResistanceControl';
export type { ResistanceControlProps } from './ResistanceControl';

// ZoneDistributionBar - displays time in zones as a stacked horizontal bar
export {
  ZoneDistributionBar,
  getPowerZoneData,
  getHrZoneData,
} from './ZoneDistributionBar';
export type {
  ZoneDistributionBarProps,
  ZoneData,
} from './ZoneDistributionBar';

// RideStatisticsSummary - displays key training stats (TSS, IF, calories) in cards
export { RideStatisticsSummary } from './RideStatisticsSummary';
export type { RideStatisticsSummaryProps } from './RideStatisticsSummary';
