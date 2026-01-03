//! UI module for egui-based user interface.

pub mod dialogs;
pub mod display_modes;
pub mod layout;
pub mod screens;
pub mod settings;
pub mod theme;
pub mod widgets;

pub use dialogs::{
    CalibrationDialog, CalibrationDialogAction, CalibrationDialogResponse, CalibrationDialogState,
    CalibrationReminderButton, CalibrationStatusIndicator, ConflictIndicator,
    ConflictNotificationBanner, ConflictResolutionAction, SensorConflictDialog,
    SensorConflictDialogResponse, SensorConflictDialogState,
};
pub use display_modes::{DisplayMode, DisplayModeManager};
pub use layout::{LayoutProfile, LayoutProfileManager};
pub use settings::{
    AudioSettingsAction, AudioSettingsPanel, AudioSettingsPanelConfig, AudioSettingsResponse,
    AudioTestType,
};
pub use theme::Theme;
