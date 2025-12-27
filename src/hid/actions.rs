//! Button Actions
//!
//! Defines actions that can be triggered by button presses.

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use tokio::sync::broadcast;

/// Errors that can occur when executing actions
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Action not available: {0}")]
    NotAvailable(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("No active ride")]
    NoActiveRide,

    #[error("No active workout")]
    NoActiveWorkout,
}

/// Button actions that can be triggered
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonAction {
    // Ride control
    /// Add a lap marker
    AddLapMarker,
    /// Pause or resume the ride
    PauseResume,
    /// End the current ride
    EndRide,

    // Workout control
    /// Skip to next interval
    SkipInterval,
    /// Extend current interval
    ExtendInterval { seconds: u32 },
    /// Restart current interval
    RestartInterval,

    // Audio control
    /// Increase volume
    VolumeUp,
    /// Decrease volume
    VolumeDown,
    /// Mute/unmute
    MuteToggle,

    // Fan control (if MQTT enabled)
    /// Increase fan speed
    FanSpeedUp,
    /// Decrease fan speed
    FanSpeedDown,
    /// Toggle fan on/off
    FanToggle,

    // UI navigation
    /// Show metrics view
    ShowMetrics,
    /// Show map view
    ShowMap,
    /// Show workout view
    ShowWorkout,
    /// Toggle fullscreen mode
    ToggleFullscreen,

    // Camera (if 3D world enabled)
    /// Zoom camera in
    CameraZoomIn,
    /// Zoom camera out
    CameraZoomOut,
    /// Rotate camera
    CameraRotate { degrees: i16 },

    // Custom action
    /// Custom command
    Custom { command: String },
}

impl ButtonAction {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AddLapMarker => "Add Lap Marker",
            Self::PauseResume => "Pause/Resume",
            Self::EndRide => "End Ride",
            Self::SkipInterval => "Skip Interval",
            Self::ExtendInterval { .. } => "Extend Interval",
            Self::RestartInterval => "Restart Interval",
            Self::VolumeUp => "Volume Up",
            Self::VolumeDown => "Volume Down",
            Self::MuteToggle => "Mute/Unmute",
            Self::FanSpeedUp => "Fan Speed Up",
            Self::FanSpeedDown => "Fan Speed Down",
            Self::FanToggle => "Fan On/Off",
            Self::ShowMetrics => "Show Metrics",
            Self::ShowMap => "Show Map",
            Self::ShowWorkout => "Show Workout",
            Self::ToggleFullscreen => "Toggle Fullscreen",
            Self::CameraZoomIn => "Zoom In",
            Self::CameraZoomOut => "Zoom Out",
            Self::CameraRotate { .. } => "Rotate Camera",
            Self::Custom { .. } => "Custom Action",
        }
    }

    /// Get action category
    pub fn category(&self) -> ActionCategory {
        match self {
            Self::AddLapMarker | Self::PauseResume | Self::EndRide => ActionCategory::RideControl,
            Self::SkipInterval | Self::ExtendInterval { .. } | Self::RestartInterval => {
                ActionCategory::WorkoutControl
            }
            Self::VolumeUp | Self::VolumeDown | Self::MuteToggle => ActionCategory::Audio,
            Self::FanSpeedUp | Self::FanSpeedDown | Self::FanToggle => ActionCategory::Fan,
            Self::ShowMetrics | Self::ShowMap | Self::ShowWorkout | Self::ToggleFullscreen => {
                ActionCategory::Navigation
            }
            Self::CameraZoomIn | Self::CameraZoomOut | Self::CameraRotate { .. } => {
                ActionCategory::Camera
            }
            Self::Custom { .. } => ActionCategory::Custom,
        }
    }

    /// Get all available actions
    pub fn all_actions() -> Vec<ButtonAction> {
        vec![
            ButtonAction::AddLapMarker,
            ButtonAction::PauseResume,
            ButtonAction::EndRide,
            ButtonAction::SkipInterval,
            ButtonAction::ExtendInterval { seconds: 30 },
            ButtonAction::RestartInterval,
            ButtonAction::VolumeUp,
            ButtonAction::VolumeDown,
            ButtonAction::MuteToggle,
            ButtonAction::FanSpeedUp,
            ButtonAction::FanSpeedDown,
            ButtonAction::FanToggle,
            ButtonAction::ShowMetrics,
            ButtonAction::ShowMap,
            ButtonAction::ShowWorkout,
            ButtonAction::ToggleFullscreen,
            ButtonAction::CameraZoomIn,
            ButtonAction::CameraZoomOut,
            ButtonAction::CameraRotate { degrees: 45 },
        ]
    }
}

/// Action categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    RideControl,
    WorkoutControl,
    Audio,
    Fan,
    Navigation,
    Camera,
    Custom,
}

impl ActionCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ActionCategory::RideControl => "Ride Control",
            ActionCategory::WorkoutControl => "Workout Control",
            ActionCategory::Audio => "Audio",
            ActionCategory::Fan => "Fan Control",
            ActionCategory::Navigation => "Navigation",
            ActionCategory::Camera => "Camera",
            ActionCategory::Custom => "Custom",
        }
    }
}

/// Context for action availability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionContext {
    /// Action available anytime
    Always,
    /// Only during an active ride
    DuringRide,
    /// Only during a workout
    DuringWorkout,
    /// Only when not in a ride
    NotDuringRide,
}

/// Result of action execution
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// The action that was executed
    pub action: ButtonAction,
    /// Whether it succeeded
    pub success: bool,
    /// Optional message
    pub message: Option<String>,
    /// When the action was executed
    pub timestamp: Instant,
}

/// Trait for executing button actions
pub trait ActionExecutor: Send + Sync {
    /// Execute an action
    fn execute(
        &self,
        action: &ButtonAction,
    ) -> impl std::future::Future<Output = Result<(), ActionError>> + Send;

    /// Get list of available actions
    fn available_actions() -> Vec<ActionInfo>;

    /// Check if action is available in current context
    fn is_available(&self, action: &ButtonAction) -> bool;

    /// Subscribe to action execution results
    fn subscribe_results(&self) -> broadcast::Receiver<ActionResult>;
}

/// Information about an action
#[derive(Debug, Clone)]
pub struct ActionInfo {
    /// The action
    pub action: ButtonAction,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Icon name (optional)
    pub icon: Option<String>,
    /// When action is available
    pub available_during: ActionContext,
}

impl ActionInfo {
    pub fn new(action: ButtonAction) -> Self {
        let (description, context) = match &action {
            ButtonAction::AddLapMarker => {
                ("Mark a lap in the current ride", ActionContext::DuringRide)
            }
            ButtonAction::PauseResume => (
                "Pause or resume the current ride",
                ActionContext::DuringRide,
            ),
            ButtonAction::EndRide => ("End the current ride and save", ActionContext::DuringRide),
            ButtonAction::SkipInterval => (
                "Skip to the next workout interval",
                ActionContext::DuringWorkout,
            ),
            ButtonAction::ExtendInterval { seconds } => {
                return Self {
                    action: action.clone(),
                    name: action.display_name().to_string(),
                    description: format!("Extend current interval by {} seconds", seconds),
                    icon: Some("plus".to_string()),
                    available_during: ActionContext::DuringWorkout,
                };
            }
            ButtonAction::RestartInterval => {
                ("Restart the current interval", ActionContext::DuringWorkout)
            }
            ButtonAction::VolumeUp => ("Increase audio volume", ActionContext::Always),
            ButtonAction::VolumeDown => ("Decrease audio volume", ActionContext::Always),
            ButtonAction::MuteToggle => ("Toggle audio mute", ActionContext::Always),
            ButtonAction::FanSpeedUp => ("Increase fan speed", ActionContext::DuringRide),
            ButtonAction::FanSpeedDown => ("Decrease fan speed", ActionContext::DuringRide),
            ButtonAction::FanToggle => ("Toggle fan on/off", ActionContext::Always),
            ButtonAction::ShowMetrics => ("Switch to metrics display", ActionContext::Always),
            ButtonAction::ShowMap => ("Switch to map display", ActionContext::Always),
            ButtonAction::ShowWorkout => {
                ("Switch to workout display", ActionContext::DuringWorkout)
            }
            ButtonAction::ToggleFullscreen => ("Toggle fullscreen mode", ActionContext::Always),
            ButtonAction::CameraZoomIn => ("Zoom camera in", ActionContext::DuringRide),
            ButtonAction::CameraZoomOut => ("Zoom camera out", ActionContext::DuringRide),
            ButtonAction::CameraRotate { .. } => ("Rotate camera view", ActionContext::DuringRide),
            ButtonAction::Custom { .. } => ("Execute custom command", ActionContext::Always),
        };

        Self {
            action: action.clone(),
            name: action.display_name().to_string(),
            description: description.to_string(),
            icon: None,
            available_during: context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_display_names() {
        assert_eq!(ButtonAction::AddLapMarker.display_name(), "Add Lap Marker");
        assert_eq!(ButtonAction::PauseResume.display_name(), "Pause/Resume");
    }

    #[test]
    fn test_action_categories() {
        assert_eq!(
            ButtonAction::AddLapMarker.category(),
            ActionCategory::RideControl
        );
        assert_eq!(
            ButtonAction::SkipInterval.category(),
            ActionCategory::WorkoutControl
        );
        assert_eq!(ButtonAction::VolumeUp.category(), ActionCategory::Audio);
    }

    #[test]
    fn test_all_actions() {
        let actions = ButtonAction::all_actions();
        assert!(!actions.is_empty());
        assert!(actions.contains(&ButtonAction::AddLapMarker));
    }

    #[test]
    fn test_action_info() {
        let info = ActionInfo::new(ButtonAction::AddLapMarker);
        assert_eq!(info.name, "Add Lap Marker");
        assert_eq!(info.available_during, ActionContext::DuringRide);
    }
}
