//! Workout execution engine.
//!
//! T063: Implement WorkoutEngine struct with load(), start(), pause(), resume(), stop()
//! T064: Implement tick() for time progression and segment transitions
//! T065: Implement skip_segment() and extend_segment()
//! T066: Implement power ramp calculation for smooth transitions
//! T067: Implement adjust_power() for manual +/- offset

use crate::workouts::types::{
    PowerTarget, SegmentProgress, SegmentType, Workout, WorkoutError, WorkoutState, WorkoutStatus,
};

/// Default ramp transition time in seconds.
const DEFAULT_RAMP_SECONDS: u32 = 3;

/// Workout execution engine.
///
/// Manages the state machine for workout execution, including:
/// - Loading and starting workouts
/// - Time progression and segment transitions
/// - Power target calculations with ramp smoothing
/// - Manual power adjustments
pub struct WorkoutEngine {
    /// Current workout state
    state: Option<WorkoutState>,
    /// Segment extension amount (seconds added to current segment)
    segment_extension: u32,
    /// Transition ramp duration in seconds
    ramp_duration: u32,
    /// Time spent in current ramp transition
    ramp_elapsed: u32,
    /// Previous segment's ending power (for smooth transitions)
    previous_power: Option<u16>,
}

impl WorkoutEngine {
    /// Create a new workout engine.
    pub fn new() -> Self {
        Self {
            state: None,
            segment_extension: 0,
            ramp_duration: DEFAULT_RAMP_SECONDS,
            ramp_elapsed: 0,
            previous_power: None,
        }
    }

    /// Load a workout for execution.
    pub fn load(&mut self, workout: Workout, user_ftp: u16) -> Result<(), WorkoutError> {
        if workout.segments.is_empty() {
            return Err(WorkoutError::InvalidWorkout(
                "Workout has no segments".to_string(),
            ));
        }

        self.state = Some(WorkoutState {
            workout,
            status: WorkoutStatus::NotStarted,
            total_elapsed_seconds: 0,
            segment_progress: None,
            power_offset: 0,
            user_ftp,
        });

        self.segment_extension = 0;
        self.ramp_elapsed = 0;
        self.previous_power = None;

        tracing::info!("Workout loaded");
        Ok(())
    }

    /// Start the loaded workout.
    pub fn start(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status != WorkoutStatus::NotStarted {
            return Err(WorkoutError::EngineError(
                "Workout already started".to_string(),
            ));
        }

        state.status = WorkoutStatus::InProgress;
        state.total_elapsed_seconds = 0;

        // Initialize first segment
        self.update_segment_progress();

        tracing::info!("Workout started");
        Ok(())
    }

    /// Pause the workout.
    pub fn pause(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status != WorkoutStatus::InProgress {
            return Err(WorkoutError::EngineError(
                "Workout not in progress".to_string(),
            ));
        }

        state.status = WorkoutStatus::Paused;
        tracing::info!("Workout paused");
        Ok(())
    }

    /// Resume the paused workout.
    pub fn resume(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status != WorkoutStatus::Paused {
            return Err(WorkoutError::EngineError("Workout not paused".to_string()));
        }

        state.status = WorkoutStatus::InProgress;
        tracing::info!("Workout resumed");
        Ok(())
    }

    /// Stop the workout early.
    pub fn stop(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        state.status = WorkoutStatus::Stopped;
        tracing::info!("Workout stopped");
        Ok(())
    }

    /// Advance the workout by one second.
    ///
    /// Should be called once per second when the workout is in progress.
    /// Note: Time does not advance when paused, stopped, or trainer is disconnected.
    pub fn tick(&mut self) {
        let state = match self.state.as_mut() {
            Some(s) if s.status == WorkoutStatus::InProgress => s,
            Some(s) if s.status == WorkoutStatus::TrainerDisconnected => {
                // Don't advance time when trainer is disconnected
                return;
            }
            _ => return,
        };

        state.total_elapsed_seconds += 1;

        // Update ramp transition
        if self.ramp_elapsed < self.ramp_duration {
            self.ramp_elapsed += 1;
        }

        // Update segment progress
        self.update_segment_progress();
    }

    /// Update segment progress based on elapsed time.
    fn update_segment_progress(&mut self) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        let mut elapsed_in_workout = 0u32;
        let mut current_segment_idx = 0usize;
        let mut elapsed_in_segment = 0u32;

        // Find current segment
        for (i, segment) in state.workout.segments.iter().enumerate() {
            let segment_duration = if i
                == state
                    .segment_progress
                    .as_ref()
                    .map(|p| p.segment_index)
                    .unwrap_or(0)
            {
                segment.duration_seconds + self.segment_extension
            } else {
                segment.duration_seconds
            };

            if elapsed_in_workout + segment_duration > state.total_elapsed_seconds {
                current_segment_idx = i;
                elapsed_in_segment = state.total_elapsed_seconds - elapsed_in_workout;
                break;
            }

            elapsed_in_workout += segment_duration;
            current_segment_idx = i + 1;
        }

        // Check if workout is complete
        if current_segment_idx >= state.workout.segments.len() {
            state.status = WorkoutStatus::Completed;
            state.segment_progress = None;
            tracing::info!("Workout completed");
            return;
        }

        // Check for segment transition
        let previous_idx = state.segment_progress.as_ref().map(|p| p.segment_index);
        if previous_idx != Some(current_segment_idx) {
            // Store previous power for smooth transition
            if let Some(progress) = &state.segment_progress {
                self.previous_power = Some(progress.target_power);
            }
            self.ramp_elapsed = 0;
            self.segment_extension = 0;
            tracing::debug!("Transitioned to segment {}", current_segment_idx);
        }

        let segment = &state.workout.segments[current_segment_idx];
        let total_segment_duration = segment.duration_seconds + self.segment_extension;
        let remaining = total_segment_duration.saturating_sub(elapsed_in_segment);
        let progress = if total_segment_duration > 0 {
            elapsed_in_segment as f32 / total_segment_duration as f32
        } else {
            0.0
        };

        // Calculate target power
        let base_power = segment.power_target.to_watts_at(state.user_ftp, progress);

        // Apply ramp smoothing for segment transitions
        let smoothed_power = if let Some(prev) = self.previous_power {
            if self.ramp_elapsed < self.ramp_duration {
                let ramp_progress = self.ramp_elapsed as f32 / self.ramp_duration as f32;
                let diff = base_power as i32 - prev as i32;
                (prev as i32 + (diff as f32 * ramp_progress) as i32) as u16
            } else {
                base_power
            }
        } else {
            base_power
        };

        // Apply power offset
        let target_power = (smoothed_power as i32 + state.power_offset as i32).max(0) as u16;

        state.segment_progress = Some(SegmentProgress {
            segment_index: current_segment_idx,
            elapsed_seconds: elapsed_in_segment,
            remaining_seconds: remaining,
            progress,
            target_power,
        });
    }

    /// Skip to the next segment.
    pub fn skip_segment(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status != WorkoutStatus::InProgress && state.status != WorkoutStatus::Paused {
            return Err(WorkoutError::EngineError("Workout not active".to_string()));
        }

        let current_idx = state
            .segment_progress
            .as_ref()
            .map(|p| p.segment_index)
            .unwrap_or(0);

        // Calculate time to end of current segment
        let mut elapsed = 0u32;
        for (i, segment) in state.workout.segments.iter().enumerate() {
            if i < current_idx {
                elapsed += segment.duration_seconds;
            } else if i == current_idx {
                elapsed += segment.duration_seconds + self.segment_extension;
                break;
            }
        }

        // Set elapsed time to start of next segment
        state.total_elapsed_seconds = elapsed;

        // Store current power for smooth transition
        if let Some(progress) = &state.segment_progress {
            self.previous_power = Some(progress.target_power);
        }

        self.segment_extension = 0;
        self.ramp_elapsed = 0;

        // Update progress
        self.update_segment_progress();

        tracing::debug!("Skipped to segment {}", current_idx + 1);
        Ok(())
    }

    /// Extend the current segment by the specified seconds.
    pub fn extend_segment(&mut self, seconds: u32) -> Result<(), WorkoutError> {
        let state = self.state.as_ref().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status != WorkoutStatus::InProgress && state.status != WorkoutStatus::Paused {
            return Err(WorkoutError::EngineError("Workout not active".to_string()));
        }

        self.segment_extension += seconds;

        // Update progress to reflect new duration
        self.update_segment_progress();

        tracing::debug!("Extended segment by {} seconds", seconds);
        Ok(())
    }

    /// Adjust the power offset by the specified amount.
    pub fn adjust_power(&mut self, offset_delta: i16) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        state.power_offset += offset_delta;

        // Update progress to reflect new target
        self.update_segment_progress();

        tracing::debug!("Power offset adjusted to {}", state.power_offset);
        Ok(())
    }

    /// Get the current workout state.
    pub fn state(&self) -> Option<&WorkoutState> {
        self.state.as_ref()
    }

    /// Get the current target power in watts.
    pub fn current_target_power(&self) -> Option<u16> {
        self.state
            .as_ref()
            .and_then(|s| s.segment_progress.as_ref())
            .map(|p| p.target_power)
    }

    /// Get the current segment's text event, if any.
    pub fn current_text_event(&self) -> Option<String> {
        let state = self.state.as_ref()?;
        let progress = state.segment_progress.as_ref()?;
        let segment = state.workout.segments.get(progress.segment_index)?;
        segment.text_event.clone()
    }

    /// Get the current segment type.
    pub fn current_segment_type(&self) -> Option<SegmentType> {
        let state = self.state.as_ref()?;
        let progress = state.segment_progress.as_ref()?;
        let segment = state.workout.segments.get(progress.segment_index)?;
        Some(segment.segment_type)
    }

    /// Check if a workout is loaded.
    pub fn has_workout(&self) -> bool {
        self.state.is_some()
    }

    /// Get the current target power (alias for current_target_power).
    pub fn target_power(&self) -> Option<u16> {
        self.current_target_power()
    }

    /// Check if workout is complete.
    pub fn is_complete(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| s.status == WorkoutStatus::Completed)
            .unwrap_or(false)
    }

    /// Check if workout is active (in progress, paused, or trainer disconnected).
    pub fn is_active(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| {
                s.status == WorkoutStatus::InProgress
                    || s.status == WorkoutStatus::Paused
                    || s.status == WorkoutStatus::TrainerDisconnected
            })
            .unwrap_or(false)
    }

    /// Set the ramp duration for power transitions.
    pub fn set_ramp_duration(&mut self, seconds: u32) {
        self.ramp_duration = seconds;
    }

    /// Reset the engine, clearing the loaded workout.
    pub fn reset(&mut self) {
        self.state = None;
        self.segment_extension = 0;
        self.ramp_elapsed = 0;
        self.previous_power = None;
    }

    /// Handle trainer disconnection during workout.
    ///
    /// This method should be called when the trainer loses connection.
    /// It pauses the workout and sets it to the TrainerDisconnected state,
    /// preserving the current progress so the workout can be resumed when
    /// the trainer reconnects.
    pub fn on_trainer_disconnect(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        match state.status {
            WorkoutStatus::InProgress => {
                state.status = WorkoutStatus::TrainerDisconnected;
                tracing::warn!("Trainer disconnected during workout - workout paused");
                Ok(())
            }
            WorkoutStatus::TrainerDisconnected => {
                // Already in disconnected state
                Ok(())
            }
            _ => {
                // Workout not actively running, no action needed
                tracing::debug!("Trainer disconnected but workout not in progress");
                Ok(())
            }
        }
    }

    /// Handle trainer reconnection during workout.
    ///
    /// This method should be called when the trainer reconnects.
    /// If the workout was paused due to disconnection, it will resume.
    pub fn on_trainer_reconnect(&mut self) -> Result<(), WorkoutError> {
        let state = self.state.as_mut().ok_or(WorkoutError::NoWorkoutLoaded)?;

        if state.status == WorkoutStatus::TrainerDisconnected {
            state.status = WorkoutStatus::InProgress;
            tracing::info!("Trainer reconnected - workout resumed");
        }

        Ok(())
    }

    /// Check if the workout is paused due to trainer disconnection.
    pub fn is_trainer_disconnected(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| s.status == WorkoutStatus::TrainerDisconnected)
            .unwrap_or(false)
    }
}

impl Default for WorkoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workouts::types::WorkoutSegment;

    fn simple_workout() -> Workout {
        Workout::new(
            "Test".to_string(),
            vec![
                WorkoutSegment {
                    segment_type: SegmentType::SteadyState,
                    duration_seconds: 60,
                    power_target: PowerTarget::percent_ftp(75),
                    cadence_target: None,
                    text_event: None,
                },
                WorkoutSegment {
                    segment_type: SegmentType::SteadyState,
                    duration_seconds: 60,
                    power_target: PowerTarget::percent_ftp(100),
                    cadence_target: None,
                    text_event: None,
                },
            ],
        )
    }

    #[test]
    fn test_load_and_start() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        assert!(engine.is_active());
        assert_eq!(engine.state().unwrap().status, WorkoutStatus::InProgress);
    }

    #[test]
    fn test_tick_advances_time() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        for _ in 0..10 {
            engine.tick();
        }

        assert_eq!(engine.state().unwrap().total_elapsed_seconds, 10);
    }

    #[test]
    fn test_segment_transition() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        // Tick through first segment
        for _ in 0..61 {
            engine.tick();
        }

        let progress = engine.state().unwrap().segment_progress.as_ref().unwrap();
        assert_eq!(progress.segment_index, 1);
    }

    #[test]
    fn test_power_target_calculation() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        // 75% of 200W = 150W
        assert_eq!(engine.current_target_power(), Some(150));

        // Skip to second segment (100% = 200W)
        engine.skip_segment().unwrap();

        // After ramp transition, should be 200W
        for _ in 0..5 {
            engine.tick();
        }
        assert_eq!(engine.current_target_power(), Some(200));
    }

    #[test]
    fn test_trainer_disconnect_pauses_workout() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        // Tick for 10 seconds
        for _ in 0..10 {
            engine.tick();
        }
        assert_eq!(engine.state().unwrap().total_elapsed_seconds, 10);

        // Disconnect trainer
        engine.on_trainer_disconnect().unwrap();
        assert!(engine.is_trainer_disconnected());
        assert_eq!(
            engine.state().unwrap().status,
            WorkoutStatus::TrainerDisconnected
        );

        // Time should NOT advance while disconnected
        for _ in 0..5 {
            engine.tick();
        }
        assert_eq!(engine.state().unwrap().total_elapsed_seconds, 10);

        // Workout should still be considered "active"
        assert!(engine.is_active());
    }

    #[test]
    fn test_trainer_reconnect_resumes_workout() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        // Tick for 10 seconds
        for _ in 0..10 {
            engine.tick();
        }

        // Disconnect and reconnect
        engine.on_trainer_disconnect().unwrap();
        assert!(engine.is_trainer_disconnected());

        engine.on_trainer_reconnect().unwrap();
        assert!(!engine.is_trainer_disconnected());
        assert_eq!(engine.state().unwrap().status, WorkoutStatus::InProgress);

        // Time should advance again
        for _ in 0..5 {
            engine.tick();
        }
        assert_eq!(engine.state().unwrap().total_elapsed_seconds, 15);
    }

    #[test]
    fn test_disconnect_preserves_progress() {
        let mut engine = WorkoutEngine::new();
        engine.load(simple_workout(), 200).unwrap();
        engine.start().unwrap();

        // Get to specific point in workout
        for _ in 0..30 {
            engine.tick();
        }
        let elapsed_before = engine.state().unwrap().total_elapsed_seconds;
        let segment_before = engine
            .state()
            .unwrap()
            .segment_progress
            .as_ref()
            .unwrap()
            .segment_index;

        // Disconnect and reconnect
        engine.on_trainer_disconnect().unwrap();
        engine.on_trainer_reconnect().unwrap();

        // Progress should be preserved
        assert_eq!(
            engine.state().unwrap().total_elapsed_seconds,
            elapsed_before
        );
        assert_eq!(
            engine
                .state()
                .unwrap()
                .segment_progress
                .as_ref()
                .unwrap()
                .segment_index,
            segment_before
        );
    }
}
