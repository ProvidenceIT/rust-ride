//! Voice Command Executor
//!
//! Converts voice commands to button actions and executes them through
//! the existing action execution pipeline.
//!
//! ## Context-Sensitive Commands
//!
//! Some voice commands are context-sensitive:
//! - `Skip` - Only valid during an active workout
//! - `TakeLap` - Only valid during an active ride
//! - `Pause`/`Resume` - Only valid during an active ride
//! - `End` - Only valid during an active ride
//!
//! The executor validates context before converting commands to actions.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::executor::{VoiceCommandExecutor, ExecutorContext};
//! use rustride::accessibility::voice_control::VoiceCommand;
//!
//! // Create executor
//! let executor = VoiceCommandExecutor::new();
//!
//! // Set context
//! let context = ExecutorContext {
//!     ride_active: true,
//!     workout_active: true,
//!     ride_paused: false,
//! };
//!
//! // Convert command to action
//! match executor.to_action(&VoiceCommand::Skip, &context) {
//!     Some(action) => println!("Execute: {:?}", action),
//!     None => println!("Command not valid in current context"),
//! }
//! ```

use thiserror::Error;

use crate::accessibility::voice_control::VoiceCommand;
use crate::hid::actions::{ActionContext, ActionError, ButtonAction};

/// Errors that can occur during voice command execution.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VoiceExecutorError {
    /// The command is not valid in the current context.
    #[error("Command not valid: {reason}")]
    InvalidContext { reason: String },

    /// The command could not be mapped to an action.
    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    /// The action execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// No active ride for this command.
    #[error("No active ride")]
    NoActiveRide,

    /// No active workout for this command.
    #[error("No active workout")]
    NoActiveWorkout,
}

impl From<ActionError> for VoiceExecutorError {
    fn from(err: ActionError) -> Self {
        match err {
            ActionError::NoActiveRide => VoiceExecutorError::NoActiveRide,
            ActionError::NoActiveWorkout => VoiceExecutorError::NoActiveWorkout,
            ActionError::NotAvailable(reason) => VoiceExecutorError::InvalidContext { reason },
            ActionError::ExecutionFailed(msg) => VoiceExecutorError::ExecutionFailed(msg),
        }
    }
}

/// Context for voice command execution.
///
/// This mirrors the `AppContext` from `hid/executor.rs` to allow
/// context-sensitive command validation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutorContext {
    /// Whether a ride is currently active.
    pub ride_active: bool,
    /// Whether a structured workout is active.
    pub workout_active: bool,
    /// Whether the ride is paused.
    pub ride_paused: bool,
}

impl ExecutorContext {
    /// Create a new context with all flags set to false.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context for an active ride.
    pub fn active_ride() -> Self {
        Self {
            ride_active: true,
            workout_active: false,
            ride_paused: false,
        }
    }

    /// Create a context for an active ride with workout.
    pub fn active_workout() -> Self {
        Self {
            ride_active: true,
            workout_active: true,
            ride_paused: false,
        }
    }

    /// Create a context for a paused ride.
    pub fn paused_ride() -> Self {
        Self {
            ride_active: true,
            workout_active: false,
            ride_paused: true,
        }
    }

    /// Set ride active state.
    pub fn with_ride_active(mut self, active: bool) -> Self {
        self.ride_active = active;
        self
    }

    /// Set workout active state.
    pub fn with_workout_active(mut self, active: bool) -> Self {
        self.workout_active = active;
        self
    }

    /// Set ride paused state.
    pub fn with_ride_paused(mut self, paused: bool) -> Self {
        self.ride_paused = paused;
        self
    }
}

/// Result of command-to-action mapping with context validation.
#[derive(Debug, Clone)]
pub struct MappingResult {
    /// The action to execute, if the command is valid.
    pub action: Option<ButtonAction>,
    /// The context requirement for this command.
    pub required_context: ActionContext,
    /// Whether the command is valid in the current context.
    pub valid: bool,
    /// Reason if the command is not valid.
    pub reason: Option<String>,
}

impl MappingResult {
    /// Create a successful mapping result.
    pub fn success(action: ButtonAction, required_context: ActionContext) -> Self {
        Self {
            action: Some(action),
            required_context,
            valid: true,
            reason: None,
        }
    }

    /// Create a context-invalid mapping result.
    pub fn invalid_context(required: ActionContext, reason: impl Into<String>) -> Self {
        Self {
            action: None,
            required_context: required,
            valid: false,
            reason: Some(reason.into()),
        }
    }

    /// Create an unknown command result.
    pub fn unknown(phrase: impl Into<String>) -> Self {
        Self {
            action: None,
            required_context: ActionContext::Always,
            valid: false,
            reason: Some(format!("Unknown command: {}", phrase.into())),
        }
    }
}

/// Voice command executor that converts voice commands to button actions.
///
/// This executor handles context-sensitive command validation and mapping
/// to ensure commands are only executed when appropriate.
#[derive(Debug, Clone, Default)]
pub struct VoiceCommandExecutor {
    /// Whether to allow "Start" command to begin a ride.
    /// This is typically disabled to prevent accidental ride starts.
    allow_start_command: bool,
}

impl VoiceCommandExecutor {
    /// Create a new voice command executor.
    pub fn new() -> Self {
        Self {
            allow_start_command: false,
        }
    }

    /// Create an executor that allows the "Start" command.
    pub fn with_start_command(mut self, allow: bool) -> Self {
        self.allow_start_command = allow;
        self
    }

    /// Map a voice command to a button action, checking context validity.
    ///
    /// Returns `Some(action)` if the command is valid in the current context,
    /// or `None` if the command is not applicable.
    pub fn to_action(
        &self,
        command: &VoiceCommand,
        context: &ExecutorContext,
    ) -> Option<ButtonAction> {
        let result = self.map_with_context(command, context);
        if result.valid {
            result.action
        } else {
            None
        }
    }

    /// Map a voice command with full context validation result.
    ///
    /// Returns a `MappingResult` containing the action (if valid),
    /// the required context, and any error reason.
    pub fn map_with_context(
        &self,
        command: &VoiceCommand,
        context: &ExecutorContext,
    ) -> MappingResult {
        match command {
            VoiceCommand::Start => {
                if !self.allow_start_command {
                    return MappingResult::invalid_context(
                        ActionContext::NotDuringRide,
                        "Start command is disabled",
                    );
                }
                if context.ride_active {
                    MappingResult::invalid_context(
                        ActionContext::NotDuringRide,
                        "Ride already active",
                    )
                } else {
                    // Start doesn't have a direct ButtonAction mapping
                    // It would need to trigger the ride start flow
                    MappingResult::invalid_context(
                        ActionContext::NotDuringRide,
                        "Start command requires UI interaction",
                    )
                }
            }

            VoiceCommand::Pause => {
                if !context.ride_active {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "No active ride to pause",
                    )
                } else if context.ride_paused {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "Ride already paused",
                    )
                } else {
                    MappingResult::success(ButtonAction::PauseResume, ActionContext::DuringRide)
                }
            }

            VoiceCommand::Resume => {
                if !context.ride_active {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "No active ride to resume",
                    )
                } else if !context.ride_paused {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "Ride not paused",
                    )
                } else {
                    MappingResult::success(ButtonAction::PauseResume, ActionContext::DuringRide)
                }
            }

            VoiceCommand::End => {
                if !context.ride_active {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "No active ride to end",
                    )
                } else {
                    MappingResult::success(ButtonAction::EndRide, ActionContext::DuringRide)
                }
            }

            VoiceCommand::Skip => {
                if !context.workout_active {
                    MappingResult::invalid_context(
                        ActionContext::DuringWorkout,
                        "Skip requires an active workout",
                    )
                } else {
                    MappingResult::success(ButtonAction::SkipInterval, ActionContext::DuringWorkout)
                }
            }

            VoiceCommand::Increase => {
                // Always available - maps to volume up
                MappingResult::success(ButtonAction::VolumeUp, ActionContext::Always)
            }

            VoiceCommand::Decrease => {
                // Always available - maps to volume down
                MappingResult::success(ButtonAction::VolumeDown, ActionContext::Always)
            }

            VoiceCommand::Status => {
                // Status is always available but doesn't map to a ButtonAction
                // It triggers TTS readout of current metrics
                MappingResult::invalid_context(
                    ActionContext::Always,
                    "Status command requires TTS announcement",
                )
            }

            VoiceCommand::TakeLap => {
                if !context.ride_active {
                    MappingResult::invalid_context(
                        ActionContext::DuringRide,
                        "No active ride for lap marker",
                    )
                } else {
                    MappingResult::success(ButtonAction::AddLapMarker, ActionContext::DuringRide)
                }
            }

            VoiceCommand::Unknown(phrase) => MappingResult::unknown(phrase),
        }
    }

    /// Get the required context for a voice command.
    ///
    /// This indicates when a command is valid, regardless of current context.
    pub fn required_context(&self, command: &VoiceCommand) -> ActionContext {
        match command {
            VoiceCommand::Start => ActionContext::NotDuringRide,
            VoiceCommand::Pause => ActionContext::DuringRide,
            VoiceCommand::Resume => ActionContext::DuringRide,
            VoiceCommand::End => ActionContext::DuringRide,
            VoiceCommand::Skip => ActionContext::DuringWorkout,
            VoiceCommand::Increase => ActionContext::Always,
            VoiceCommand::Decrease => ActionContext::Always,
            VoiceCommand::Status => ActionContext::Always,
            VoiceCommand::TakeLap => ActionContext::DuringRide,
            VoiceCommand::Unknown(_) => ActionContext::Always,
        }
    }

    /// Check if a command is valid in the given context.
    pub fn is_valid(&self, command: &VoiceCommand, context: &ExecutorContext) -> bool {
        self.map_with_context(command, context).valid
    }

    /// Get a user-friendly error message for an invalid command.
    pub fn get_error_message(&self, command: &VoiceCommand, context: &ExecutorContext) -> Option<String> {
        let result = self.map_with_context(command, context);
        if result.valid {
            None
        } else {
            result.reason
        }
    }

    /// Map a voice command to a button action without context checking.
    ///
    /// This returns the action that would be executed if the context was valid.
    /// Returns `None` for commands that don't map to actions (e.g., Start, Status).
    pub fn to_action_unchecked(&self, command: &VoiceCommand) -> Option<ButtonAction> {
        match command {
            VoiceCommand::Start => None, // No direct mapping
            VoiceCommand::Pause => Some(ButtonAction::PauseResume),
            VoiceCommand::Resume => Some(ButtonAction::PauseResume),
            VoiceCommand::End => Some(ButtonAction::EndRide),
            VoiceCommand::Skip => Some(ButtonAction::SkipInterval),
            VoiceCommand::Increase => Some(ButtonAction::VolumeUp),
            VoiceCommand::Decrease => Some(ButtonAction::VolumeDown),
            VoiceCommand::Status => None, // Handled separately via TTS
            VoiceCommand::TakeLap => Some(ButtonAction::AddLapMarker),
            VoiceCommand::Unknown(_) => None,
        }
    }

    /// Get all voice commands that are valid in the given context.
    pub fn available_commands(&self, context: &ExecutorContext) -> Vec<VoiceCommand> {
        let commands = [
            VoiceCommand::Pause,
            VoiceCommand::Resume,
            VoiceCommand::End,
            VoiceCommand::Skip,
            VoiceCommand::Increase,
            VoiceCommand::Decrease,
            VoiceCommand::Status,
            VoiceCommand::TakeLap,
        ];

        commands
            .into_iter()
            .filter(|cmd| self.is_valid(cmd, context))
            .collect()
    }
}

/// Convert ExecutorContext to hid::executor::AppContext.
impl From<ExecutorContext> for crate::hid::executor::AppContext {
    fn from(ctx: ExecutorContext) -> Self {
        Self {
            ride_active: ctx.ride_active,
            workout_active: ctx.workout_active,
            ride_paused: ctx.ride_paused,
        }
    }
}

/// Convert hid::executor::AppContext to ExecutorContext.
impl From<crate::hid::executor::AppContext> for ExecutorContext {
    fn from(ctx: crate::hid::executor::AppContext) -> Self {
        Self {
            ride_active: ctx.ride_active,
            workout_active: ctx.workout_active,
            ride_paused: ctx.ride_paused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // VoiceCommandExecutor Tests
    // ========================================

    #[test]
    fn test_executor_creation() {
        let executor = VoiceCommandExecutor::new();
        assert!(!executor.allow_start_command);
    }

    #[test]
    fn test_executor_with_start_command() {
        let executor = VoiceCommandExecutor::new().with_start_command(true);
        assert!(executor.allow_start_command);
    }

    // ========================================
    // Context-Sensitive Command Tests
    // ========================================

    #[test]
    fn test_pause_requires_active_ride() {
        let executor = VoiceCommandExecutor::new();

        // No active ride - should fail
        let ctx = ExecutorContext::new();
        assert!(!executor.is_valid(&VoiceCommand::Pause, &ctx));
        assert!(executor.to_action(&VoiceCommand::Pause, &ctx).is_none());

        // Active ride - should succeed
        let ctx = ExecutorContext::active_ride();
        assert!(executor.is_valid(&VoiceCommand::Pause, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::Pause, &ctx),
            Some(ButtonAction::PauseResume)
        );
    }

    #[test]
    fn test_pause_not_valid_when_already_paused() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::paused_ride();

        assert!(!executor.is_valid(&VoiceCommand::Pause, &ctx));
        let result = executor.map_with_context(&VoiceCommand::Pause, &ctx);
        assert!(!result.valid);
        assert_eq!(result.reason, Some("Ride already paused".to_string()));
    }

    #[test]
    fn test_resume_requires_paused_ride() {
        let executor = VoiceCommandExecutor::new();

        // Active ride, not paused - should fail
        let ctx = ExecutorContext::active_ride();
        assert!(!executor.is_valid(&VoiceCommand::Resume, &ctx));

        // Paused ride - should succeed
        let ctx = ExecutorContext::paused_ride();
        assert!(executor.is_valid(&VoiceCommand::Resume, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::Resume, &ctx),
            Some(ButtonAction::PauseResume)
        );
    }

    #[test]
    fn test_end_requires_active_ride() {
        let executor = VoiceCommandExecutor::new();

        // No active ride - should fail
        let ctx = ExecutorContext::new();
        assert!(!executor.is_valid(&VoiceCommand::End, &ctx));

        // Active ride - should succeed
        let ctx = ExecutorContext::active_ride();
        assert!(executor.is_valid(&VoiceCommand::End, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::End, &ctx),
            Some(ButtonAction::EndRide)
        );
    }

    #[test]
    fn test_skip_requires_active_workout() {
        let executor = VoiceCommandExecutor::new();

        // No workout - should fail
        let ctx = ExecutorContext::active_ride();
        assert!(!executor.is_valid(&VoiceCommand::Skip, &ctx));

        // Active workout - should succeed
        let ctx = ExecutorContext::active_workout();
        assert!(executor.is_valid(&VoiceCommand::Skip, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::Skip, &ctx),
            Some(ButtonAction::SkipInterval)
        );
    }

    #[test]
    fn test_take_lap_requires_active_ride() {
        let executor = VoiceCommandExecutor::new();

        // No active ride - should fail
        let ctx = ExecutorContext::new();
        assert!(!executor.is_valid(&VoiceCommand::TakeLap, &ctx));

        // Active ride - should succeed
        let ctx = ExecutorContext::active_ride();
        assert!(executor.is_valid(&VoiceCommand::TakeLap, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::TakeLap, &ctx),
            Some(ButtonAction::AddLapMarker)
        );
    }

    // ========================================
    // Always-Available Command Tests
    // ========================================

    #[test]
    fn test_increase_always_available() {
        let executor = VoiceCommandExecutor::new();

        // Available even with no ride
        let ctx = ExecutorContext::new();
        assert!(executor.is_valid(&VoiceCommand::Increase, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::Increase, &ctx),
            Some(ButtonAction::VolumeUp)
        );

        // Also available during ride
        let ctx = ExecutorContext::active_ride();
        assert!(executor.is_valid(&VoiceCommand::Increase, &ctx));
    }

    #[test]
    fn test_decrease_always_available() {
        let executor = VoiceCommandExecutor::new();

        let ctx = ExecutorContext::new();
        assert!(executor.is_valid(&VoiceCommand::Decrease, &ctx));
        assert_eq!(
            executor.to_action(&VoiceCommand::Decrease, &ctx),
            Some(ButtonAction::VolumeDown)
        );
    }

    // ========================================
    // Special Command Tests
    // ========================================

    #[test]
    fn test_start_disabled_by_default() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::new();

        assert!(!executor.is_valid(&VoiceCommand::Start, &ctx));
        let result = executor.map_with_context(&VoiceCommand::Start, &ctx);
        assert_eq!(result.reason, Some("Start command is disabled".to_string()));
    }

    #[test]
    fn test_status_requires_tts() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::active_ride();

        // Status doesn't map to a ButtonAction
        assert!(!executor.is_valid(&VoiceCommand::Status, &ctx));
        let result = executor.map_with_context(&VoiceCommand::Status, &ctx);
        assert_eq!(
            result.reason,
            Some("Status command requires TTS announcement".to_string())
        );
    }

    #[test]
    fn test_unknown_command() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::active_ride();

        let unknown = VoiceCommand::Unknown("blah blah".to_string());
        assert!(!executor.is_valid(&unknown, &ctx));
        let result = executor.map_with_context(&unknown, &ctx);
        assert_eq!(result.reason, Some("Unknown command: blah blah".to_string()));
    }

    // ========================================
    // Required Context Tests
    // ========================================

    #[test]
    fn test_required_context() {
        let executor = VoiceCommandExecutor::new();

        assert_eq!(
            executor.required_context(&VoiceCommand::Start),
            ActionContext::NotDuringRide
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::Pause),
            ActionContext::DuringRide
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::Resume),
            ActionContext::DuringRide
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::End),
            ActionContext::DuringRide
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::Skip),
            ActionContext::DuringWorkout
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::TakeLap),
            ActionContext::DuringRide
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::Increase),
            ActionContext::Always
        );
        assert_eq!(
            executor.required_context(&VoiceCommand::Decrease),
            ActionContext::Always
        );
    }

    // ========================================
    // Unchecked Mapping Tests
    // ========================================

    #[test]
    fn test_to_action_unchecked() {
        let executor = VoiceCommandExecutor::new();

        assert_eq!(executor.to_action_unchecked(&VoiceCommand::Start), None);
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::Pause),
            Some(ButtonAction::PauseResume)
        );
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::Resume),
            Some(ButtonAction::PauseResume)
        );
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::End),
            Some(ButtonAction::EndRide)
        );
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::Skip),
            Some(ButtonAction::SkipInterval)
        );
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::Increase),
            Some(ButtonAction::VolumeUp)
        );
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::Decrease),
            Some(ButtonAction::VolumeDown)
        );
        assert_eq!(executor.to_action_unchecked(&VoiceCommand::Status), None);
        assert_eq!(
            executor.to_action_unchecked(&VoiceCommand::TakeLap),
            Some(ButtonAction::AddLapMarker)
        );
    }

    // ========================================
    // Available Commands Tests
    // ========================================

    #[test]
    fn test_available_commands_no_ride() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::new();

        let available = executor.available_commands(&ctx);

        // Only volume commands should be available
        assert!(available.contains(&VoiceCommand::Increase));
        assert!(available.contains(&VoiceCommand::Decrease));
        assert!(!available.contains(&VoiceCommand::Pause));
        assert!(!available.contains(&VoiceCommand::Skip));
    }

    #[test]
    fn test_available_commands_active_ride() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::active_ride();

        let available = executor.available_commands(&ctx);

        assert!(available.contains(&VoiceCommand::Pause));
        assert!(available.contains(&VoiceCommand::End));
        assert!(available.contains(&VoiceCommand::TakeLap));
        assert!(available.contains(&VoiceCommand::Increase));
        assert!(available.contains(&VoiceCommand::Decrease));
        assert!(!available.contains(&VoiceCommand::Resume)); // Not paused
        assert!(!available.contains(&VoiceCommand::Skip)); // No workout
    }

    #[test]
    fn test_available_commands_active_workout() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::active_workout();

        let available = executor.available_commands(&ctx);

        assert!(available.contains(&VoiceCommand::Pause));
        assert!(available.contains(&VoiceCommand::End));
        assert!(available.contains(&VoiceCommand::Skip));
        assert!(available.contains(&VoiceCommand::TakeLap));
    }

    #[test]
    fn test_available_commands_paused_ride() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::paused_ride();

        let available = executor.available_commands(&ctx);

        assert!(available.contains(&VoiceCommand::Resume));
        assert!(available.contains(&VoiceCommand::End));
        assert!(available.contains(&VoiceCommand::TakeLap));
        assert!(!available.contains(&VoiceCommand::Pause)); // Already paused
    }

    // ========================================
    // Error Message Tests
    // ========================================

    #[test]
    fn test_get_error_message() {
        let executor = VoiceCommandExecutor::new();
        let ctx = ExecutorContext::new();

        // Valid command has no error
        assert!(executor.get_error_message(&VoiceCommand::Increase, &ctx).is_none());

        // Invalid command has error message
        let error = executor.get_error_message(&VoiceCommand::Pause, &ctx);
        assert_eq!(error, Some("No active ride to pause".to_string()));

        let error = executor.get_error_message(&VoiceCommand::Skip, &ctx);
        assert_eq!(error, Some("Skip requires an active workout".to_string()));
    }

    // ========================================
    // Context Builder Tests
    // ========================================

    #[test]
    fn test_context_builders() {
        let ctx = ExecutorContext::new();
        assert!(!ctx.ride_active);
        assert!(!ctx.workout_active);
        assert!(!ctx.ride_paused);

        let ctx = ExecutorContext::active_ride();
        assert!(ctx.ride_active);
        assert!(!ctx.workout_active);
        assert!(!ctx.ride_paused);

        let ctx = ExecutorContext::active_workout();
        assert!(ctx.ride_active);
        assert!(ctx.workout_active);
        assert!(!ctx.ride_paused);

        let ctx = ExecutorContext::paused_ride();
        assert!(ctx.ride_active);
        assert!(!ctx.workout_active);
        assert!(ctx.ride_paused);
    }

    #[test]
    fn test_context_with_methods() {
        let ctx = ExecutorContext::new()
            .with_ride_active(true)
            .with_workout_active(true)
            .with_ride_paused(false);

        assert!(ctx.ride_active);
        assert!(ctx.workout_active);
        assert!(!ctx.ride_paused);
    }

    // ========================================
    // Context Conversion Tests
    // ========================================

    #[test]
    fn test_context_to_app_context() {
        let ctx = ExecutorContext::active_workout();
        let app_ctx: crate::hid::executor::AppContext = ctx.into();

        assert!(app_ctx.ride_active);
        assert!(app_ctx.workout_active);
        assert!(!app_ctx.ride_paused);
    }

    #[test]
    fn test_app_context_to_executor_context() {
        let app_ctx = crate::hid::executor::AppContext {
            ride_active: true,
            workout_active: false,
            ride_paused: true,
        };
        let ctx: ExecutorContext = app_ctx.into();

        assert!(ctx.ride_active);
        assert!(!ctx.workout_active);
        assert!(ctx.ride_paused);
    }

    // ========================================
    // Error Type Tests
    // ========================================

    #[test]
    fn test_error_from_action_error() {
        let err: VoiceExecutorError = ActionError::NoActiveRide.into();
        assert!(matches!(err, VoiceExecutorError::NoActiveRide));

        let err: VoiceExecutorError = ActionError::NoActiveWorkout.into();
        assert!(matches!(err, VoiceExecutorError::NoActiveWorkout));

        let err: VoiceExecutorError = ActionError::NotAvailable("test".to_string()).into();
        assert!(matches!(err, VoiceExecutorError::InvalidContext { .. }));

        let err: VoiceExecutorError = ActionError::ExecutionFailed("fail".to_string()).into();
        assert!(matches!(err, VoiceExecutorError::ExecutionFailed(_)));
    }

    #[test]
    fn test_error_display() {
        let err = VoiceExecutorError::NoActiveRide;
        assert_eq!(err.to_string(), "No active ride");

        let err = VoiceExecutorError::NoActiveWorkout;
        assert_eq!(err.to_string(), "No active workout");

        let err = VoiceExecutorError::InvalidContext {
            reason: "test reason".to_string(),
        };
        assert_eq!(err.to_string(), "Command not valid: test reason");

        let err = VoiceExecutorError::UnknownCommand("xyz".to_string());
        assert_eq!(err.to_string(), "Unknown command: xyz");
    }

    // ========================================
    // MappingResult Tests
    // ========================================

    #[test]
    fn test_mapping_result_success() {
        let result = MappingResult::success(ButtonAction::PauseResume, ActionContext::DuringRide);

        assert!(result.valid);
        assert_eq!(result.action, Some(ButtonAction::PauseResume));
        assert_eq!(result.required_context, ActionContext::DuringRide);
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_mapping_result_invalid_context() {
        let result = MappingResult::invalid_context(ActionContext::DuringRide, "No ride");

        assert!(!result.valid);
        assert!(result.action.is_none());
        assert_eq!(result.required_context, ActionContext::DuringRide);
        assert_eq!(result.reason, Some("No ride".to_string()));
    }

    #[test]
    fn test_mapping_result_unknown() {
        let result = MappingResult::unknown("gibberish");

        assert!(!result.valid);
        assert!(result.action.is_none());
        assert_eq!(result.required_context, ActionContext::Always);
        assert_eq!(result.reason, Some("Unknown command: gibberish".to_string()));
    }
}
