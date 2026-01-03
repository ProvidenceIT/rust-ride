//! UI dialogs for modal interactions.
//!
//! Dialogs provide focused, modal user interactions for specific tasks
//! like sensor conflict resolution, calibration, and confirmation prompts.

pub mod calibration;
pub mod sensor_conflict;

pub use calibration::{
    CalibrationDialog, CalibrationDialogAction, CalibrationDialogResponse, CalibrationDialogState,
    CalibrationReminderButton, CalibrationStatusIndicator,
};
pub use sensor_conflict::{
    ConflictIndicator, ConflictNotificationBanner, ConflictResolutionAction, SensorConflictDialog,
    SensorConflictDialogResponse, SensorConflictDialogState,
};
