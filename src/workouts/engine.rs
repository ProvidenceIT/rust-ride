//! Workout execution engine.
//!
//! T063: Implement WorkoutEngine struct
//! Placeholder for Phase 4 implementation

use crate::workouts::types::{Workout, WorkoutError, WorkoutState, WorkoutStatus};

/// Executes structured workouts with ERG mode control.
pub struct WorkoutEngine {
    /// Current workout state
    state: Option<WorkoutState>,
}

impl WorkoutEngine {
    /// Create a new workout engine.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Load a workout for execution.
    pub fn load(&mut self, workout: Workout, user_ftp: u16) -> Result<(), WorkoutError> {
        self.state = Some(WorkoutState {
            workout,
            status: WorkoutStatus::NotStarted,
            total_elapsed_seconds: 0,
            segment_progress: None,
            power_offset: 0,
            user_ftp,
        });
        tracing::info!("Workout loaded");
        Ok(())
    }

    /// Start or resume the workout.
    pub fn start(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        match state.status {
            WorkoutStatus::NotStarted | WorkoutStatus::Paused => {
                state.status = WorkoutStatus::InProgress;
                tracing::info!("Workout started");
                Ok(())
            }
            WorkoutStatus::InProgress => Ok(()), // Already running
            WorkoutStatus::Completed | WorkoutStatus::Stopped => {
                Err(WorkoutError::EngineError("Workout already finished".to_string()))
            }
        }
    }

    /// Pause the workout.
    pub fn pause(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status == WorkoutStatus::InProgress {
            state.status = WorkoutStatus::Paused;
            tracing::info!("Workout paused");
        }
        Ok(())
    }

    /// Resume the workout.
    pub fn resume(&mut self) -> Result<(), WorkoutError> {
        self.start()
    }

    /// Stop the workout.
    pub fn stop(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;
        state.status = WorkoutStatus::Stopped;
        tracing::info!("Workout stopped");
        Ok(())
    }

    /// Get the current workout state.
    pub fn state(&self) -> Option<&WorkoutState> {
        self.state.as_ref()
    }

    /// Check if a workout is loaded.
    pub fn has_workout(&self) -> bool {
        self.state.is_some()
    }

    /// Get the current target power.
    pub fn target_power(&self) -> Option<u16> {
        self.state
            .as_ref()
            .and_then(|s| s.segment_progress.as_ref())
            .map(|p| p.target_power)
    }

    /// Adjust power offset (+/- watts).
    pub fn adjust_power(&mut self, delta: i16) {
        if let Some(state) = &mut self.state {
            state.power_offset = state.power_offset.saturating_add(delta);
            tracing::debug!("Power offset adjusted to {}", state.power_offset);
        }
    }

    /// Tick the engine forward by one second.
    ///
    /// Returns the target power for this tick.
    pub fn tick(&mut self) -> Option<u16> {
        // TODO: Implement full tick logic in Phase 4 (T064)
        let state = self.state.as_mut()?;

        if state.status != WorkoutStatus::InProgress {
            return None;
        }

        state.total_elapsed_seconds += 1;

        // Placeholder: return None (no target) for now
        None
    }
}

impl Default for WorkoutEngine {
    fn default() -> Self {
        Self::new()
    }
}
