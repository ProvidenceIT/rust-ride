//! UI dialogs for modal interactions.
//!
//! Dialogs provide focused, modal user interactions for specific tasks
//! like sensor conflict resolution, calibration, and confirmation prompts.

pub mod sensor_conflict;

pub use sensor_conflict::{
    ConflictIndicator, ConflictNotificationBanner, ConflictResolutionAction, SensorConflictDialog,
    SensorConflictDialogResponse, SensorConflictDialogState,
};
