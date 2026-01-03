//! Action Executor
//!
//! Concrete implementation of the ActionExecutor trait that handles all
//! ButtonAction types by calling appropriate app subsystems.

use super::actions::{ActionContext, ActionError, ActionExecutor, ActionInfo, ActionResult, ButtonAction};
use crate::audio::AudioEngine;
use crate::integrations::mqtt::{FanController, MqttError};
use crate::recording::RideRecorder;
use crate::workouts::engine::WorkoutEngine;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::broadcast;
use uuid::Uuid;

/// App context state for determining action availability
#[derive(Debug, Clone, Copy, Default)]
pub struct AppContext {
    /// Whether a ride is currently active
    pub ride_active: bool,
    /// Whether a structured workout is active
    pub workout_active: bool,
    /// Whether the ride is paused
    pub ride_paused: bool,
}

/// Lap marker for tracking lap times during a ride
#[derive(Debug, Clone)]
pub struct LapMarker {
    /// Lap number (1-indexed)
    pub lap_number: u32,
    /// Elapsed seconds when lap was marked
    pub elapsed_seconds: u32,
    /// Timestamp when lap was marked
    pub timestamp: Instant,
}

/// Events emitted by the action executor for UI integration
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    /// Action was executed
    ActionExecuted(ActionResult),
    /// Lap was marked
    LapMarked(LapMarker),
    /// UI navigation requested
    NavigationRequest(NavigationTarget),
    /// Fullscreen toggle requested
    FullscreenToggle,
    /// Custom action executed
    CustomAction { command: String },
}

/// Navigation targets for UI actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTarget {
    /// Show metrics/data display
    Metrics,
    /// Show map view
    Map,
    /// Show workout view
    Workout,
}

/// Default action executor implementation
///
/// This executor handles all button actions by delegating to the appropriate
/// app subsystems. It uses dependency injection to receive references to:
/// - WorkoutEngine for workout control
/// - RideRecorder for ride control
/// - AudioEngine for volume control
/// - FanController for fan control
pub struct DefaultActionExecutor<A: AudioEngine, F: FanController> {
    /// Current app context (ride/workout state)
    context: Arc<RwLock<AppContext>>,
    /// Workout engine for workout control actions
    workout_engine: Option<Arc<RwLock<WorkoutEngine>>>,
    /// Ride recorder for ride control actions
    ride_recorder: Option<Arc<RwLock<RideRecorder>>>,
    /// Audio engine for volume control
    audio_engine: Option<Arc<A>>,
    /// Fan controller for fan actions
    fan_controller: Option<Arc<F>>,
    /// Lap markers tracked during ride
    lap_markers: Arc<RwLock<Vec<LapMarker>>>,
    /// Current volume level (0-100)
    volume_level: Arc<RwLock<u8>>,
    /// Whether audio is muted
    is_muted: Arc<RwLock<bool>>,
    /// Current fan speed (0-100)
    fan_speed: Arc<RwLock<u8>>,
    /// Whether fan is on
    fan_on: Arc<RwLock<bool>>,
    /// Active fan profile ID
    fan_profile_id: Arc<RwLock<Option<Uuid>>>,
    /// Event broadcast channel for action results
    result_tx: broadcast::Sender<ActionResult>,
    /// Event broadcast channel for executor events
    event_tx: broadcast::Sender<ExecutorEvent>,
}

impl<A: AudioEngine, F: FanController> DefaultActionExecutor<A, F> {
    /// Create a new action executor
    pub fn new() -> Self {
        let (result_tx, _) = broadcast::channel(100);
        let (event_tx, _) = broadcast::channel(100);

        Self {
            context: Arc::new(RwLock::new(AppContext::default())),
            workout_engine: None,
            ride_recorder: None,
            audio_engine: None,
            fan_controller: None,
            lap_markers: Arc::new(RwLock::new(Vec::new())),
            volume_level: Arc::new(RwLock::new(80)),
            is_muted: Arc::new(RwLock::new(false)),
            fan_speed: Arc::new(RwLock::new(50)),
            fan_on: Arc::new(RwLock::new(false)),
            fan_profile_id: Arc::new(RwLock::new(None)),
            result_tx,
            event_tx,
        }
    }

    /// Set the workout engine
    pub fn with_workout_engine(mut self, engine: Arc<RwLock<WorkoutEngine>>) -> Self {
        self.workout_engine = Some(engine);
        self
    }

    /// Set the ride recorder
    pub fn with_ride_recorder(mut self, recorder: Arc<RwLock<RideRecorder>>) -> Self {
        self.ride_recorder = Some(recorder);
        self
    }

    /// Set the audio engine
    pub fn with_audio_engine(mut self, engine: Arc<A>) -> Self {
        self.audio_engine = Some(engine);
        self
    }

    /// Set the fan controller
    pub fn with_fan_controller(mut self, controller: Arc<F>) -> Self {
        self.fan_controller = Some(controller);
        self
    }

    /// Set the active fan profile ID
    pub fn with_fan_profile(mut self, profile_id: Uuid) -> Self {
        *self.fan_profile_id.write().unwrap() = Some(profile_id);
        self
    }

    /// Update the app context
    pub fn update_context(&self, context: AppContext) {
        *self.context.write().unwrap() = context;
    }

    /// Set ride active state
    pub fn set_ride_active(&self, active: bool) {
        self.context.write().unwrap().ride_active = active;
        if !active {
            // Clear lap markers when ride ends
            self.lap_markers.write().unwrap().clear();
        }
    }

    /// Set workout active state
    pub fn set_workout_active(&self, active: bool) {
        self.context.write().unwrap().workout_active = active;
    }

    /// Set ride paused state
    pub fn set_ride_paused(&self, paused: bool) {
        self.context.write().unwrap().ride_paused = paused;
    }

    /// Get current lap markers
    pub fn get_lap_markers(&self) -> Vec<LapMarker> {
        self.lap_markers.read().unwrap().clone()
    }

    /// Subscribe to executor events
    pub fn subscribe_events(&self) -> broadcast::Receiver<ExecutorEvent> {
        self.event_tx.subscribe()
    }

    /// Get current volume level
    pub fn get_volume(&self) -> u8 {
        *self.volume_level.read().unwrap()
    }

    /// Get mute state
    pub fn is_muted(&self) -> bool {
        *self.is_muted.read().unwrap()
    }

    /// Get fan speed
    pub fn get_fan_speed(&self) -> u8 {
        *self.fan_speed.read().unwrap()
    }

    /// Get fan on/off state
    pub fn is_fan_on(&self) -> bool {
        *self.fan_on.read().unwrap()
    }

    /// Emit an action result
    fn emit_result(&self, action: ButtonAction, success: bool, message: Option<String>) {
        let result = ActionResult {
            action: action.clone(),
            success,
            message,
            timestamp: Instant::now(),
        };
        let _ = self.result_tx.send(result.clone());
        let _ = self.event_tx.send(ExecutorEvent::ActionExecuted(result));
    }

    /// Execute a ride control action
    async fn execute_ride_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        match action {
            ButtonAction::AddLapMarker => self.add_lap_marker(),
            ButtonAction::PauseResume => self.pause_resume_ride().await,
            ButtonAction::EndRide => self.end_ride().await,
            _ => Err(ActionError::NotAvailable("Not a ride action".to_string())),
        }
    }

    /// Add a lap marker to the current ride
    fn add_lap_marker(&self) -> Result<(), ActionError> {
        let context = self.context.read().unwrap();
        if !context.ride_active {
            return Err(ActionError::NoActiveRide);
        }
        drop(context);

        let elapsed = if let Some(recorder) = &self.ride_recorder {
            let mut rec = recorder.write().unwrap();
            // Call the recorder's add_lap method to properly track laps
            match rec.add_lap() {
                Ok(recorder_lap) => recorder_lap.elapsed_seconds,
                Err(e) => {
                    tracing::warn!("Failed to add lap to recorder: {}", e);
                    rec.get_live_summary().elapsed_seconds
                }
            }
        } else {
            0
        };

        let mut markers = self.lap_markers.write().unwrap();
        let lap_number = markers.len() as u32 + 1;

        let marker = LapMarker {
            lap_number,
            elapsed_seconds: elapsed,
            timestamp: Instant::now(),
        };

        markers.push(marker.clone());
        let _ = self.event_tx.send(ExecutorEvent::LapMarked(marker));

        tracing::info!("Lap {} marked at {}s", lap_number, elapsed);
        Ok(())
    }

    /// Pause or resume the current ride
    async fn pause_resume_ride(&self) -> Result<(), ActionError> {
        let context = self.context.read().unwrap();
        if !context.ride_active {
            return Err(ActionError::NoActiveRide);
        }

        let is_paused = context.ride_paused;
        drop(context);

        if let Some(recorder) = &self.ride_recorder {
            let mut rec = recorder.write().unwrap();
            let result = if is_paused {
                rec.resume()
            } else {
                rec.pause()
            };

            match result {
                Ok(()) => {
                    self.set_ride_paused(!is_paused);
                    // Also pause/resume workout if active
                    if let Some(engine) = &self.workout_engine {
                        let mut eng = engine.write().unwrap();
                        if is_paused {
                            let _ = eng.resume();
                        } else {
                            let _ = eng.pause();
                        }
                    }
                    tracing::info!(
                        "Ride {}",
                        if is_paused { "resumed" } else { "paused" }
                    );
                    Ok(())
                }
                Err(e) => Err(ActionError::ExecutionFailed(e.to_string())),
            }
        } else {
            Err(ActionError::NotAvailable(
                "Ride recorder not available".to_string(),
            ))
        }
    }

    /// End the current ride
    async fn end_ride(&self) -> Result<(), ActionError> {
        let context = self.context.read().unwrap();
        if !context.ride_active {
            return Err(ActionError::NoActiveRide);
        }
        drop(context);

        // Stop workout if active
        if let Some(engine) = &self.workout_engine {
            let mut eng = engine.write().unwrap();
            let _ = eng.stop();
        }

        // Note: The actual ride saving is typically handled by the app
        // We just signal the intent here and update state
        self.set_ride_active(false);
        self.set_workout_active(false);

        tracing::info!("Ride end requested");
        Ok(())
    }

    /// Execute a workout control action
    async fn execute_workout_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        let context = self.context.read().unwrap();
        if !context.workout_active {
            return Err(ActionError::NoActiveWorkout);
        }
        drop(context);

        let engine = self.workout_engine.as_ref().ok_or_else(|| {
            ActionError::NotAvailable("Workout engine not available".to_string())
        })?;

        let mut eng = engine.write().unwrap();

        match action {
            ButtonAction::SkipInterval => {
                eng.skip_segment()
                    .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;
                tracing::info!("Skipped to next interval");
                Ok(())
            }
            ButtonAction::ExtendInterval { seconds } => {
                eng.extend_segment(*seconds)
                    .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;
                tracing::info!("Extended interval by {} seconds", seconds);
                Ok(())
            }
            ButtonAction::RestartInterval => {
                eng.restart_segment()
                    .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;
                tracing::info!("Restarted current interval");
                Ok(())
            }
            _ => Err(ActionError::NotAvailable("Not a workout action".to_string())),
        }
    }

    /// Execute an audio control action
    async fn execute_audio_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        match action {
            ButtonAction::VolumeUp => {
                let mut vol = self.volume_level.write().unwrap();
                *vol = (*vol + 10).min(100);
                let new_vol = *vol;
                drop(vol);

                if let Some(engine) = &self.audio_engine {
                    engine.set_volume(new_vol);
                }
                tracing::debug!("Volume up to {}", new_vol);
                Ok(())
            }
            ButtonAction::VolumeDown => {
                let mut vol = self.volume_level.write().unwrap();
                *vol = vol.saturating_sub(10);
                let new_vol = *vol;
                drop(vol);

                if let Some(engine) = &self.audio_engine {
                    engine.set_volume(new_vol);
                }
                tracing::debug!("Volume down to {}", new_vol);
                Ok(())
            }
            ButtonAction::MuteToggle => {
                let mut muted = self.is_muted.write().unwrap();
                *muted = !*muted;
                let is_now_muted = *muted;
                drop(muted);

                if let Some(engine) = &self.audio_engine {
                    if is_now_muted {
                        engine.set_volume(0);
                    } else {
                        engine.set_volume(*self.volume_level.read().unwrap());
                    }
                }
                tracing::debug!("Audio {}", if is_now_muted { "muted" } else { "unmuted" });
                Ok(())
            }
            _ => Err(ActionError::NotAvailable("Not an audio action".to_string())),
        }
    }

    /// Execute a fan control action
    async fn execute_fan_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        let controller = self.fan_controller.as_ref().ok_or_else(|| {
            ActionError::NotAvailable("Fan controller not available".to_string())
        })?;

        let profile_id = self
            .fan_profile_id
            .read()
            .unwrap()
            .ok_or_else(|| ActionError::NotAvailable("No fan profile configured".to_string()))?;

        match action {
            ButtonAction::FanSpeedUp => {
                let mut speed = self.fan_speed.write().unwrap();
                *speed = (*speed + 10).min(100);
                let new_speed = *speed;
                drop(speed);

                *self.fan_on.write().unwrap() = true;
                controller.set_speed(&profile_id, new_speed).await.map_err(
                    |e: MqttError| ActionError::ExecutionFailed(e.to_string()),
                )?;
                tracing::debug!("Fan speed up to {}", new_speed);
                Ok(())
            }
            ButtonAction::FanSpeedDown => {
                let mut speed = self.fan_speed.write().unwrap();
                *speed = speed.saturating_sub(10);
                let new_speed = *speed;
                drop(speed);

                if new_speed == 0 {
                    *self.fan_on.write().unwrap() = false;
                }
                controller.set_speed(&profile_id, new_speed).await.map_err(
                    |e: MqttError| ActionError::ExecutionFailed(e.to_string()),
                )?;
                tracing::debug!("Fan speed down to {}", new_speed);
                Ok(())
            }
            ButtonAction::FanToggle => {
                let mut on = self.fan_on.write().unwrap();
                *on = !*on;
                let is_now_on = *on;
                drop(on);

                let speed = if is_now_on {
                    *self.fan_speed.read().unwrap()
                } else {
                    0
                };
                controller.set_speed(&profile_id, speed).await.map_err(
                    |e: MqttError| ActionError::ExecutionFailed(e.to_string()),
                )?;
                tracing::debug!("Fan {}", if is_now_on { "on" } else { "off" });
                Ok(())
            }
            _ => Err(ActionError::NotAvailable("Not a fan action".to_string())),
        }
    }

    /// Execute a navigation action
    async fn execute_navigation_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        match action {
            ButtonAction::ShowMetrics => {
                let _ = self
                    .event_tx
                    .send(ExecutorEvent::NavigationRequest(NavigationTarget::Metrics));
                tracing::debug!("Navigation: show metrics");
                Ok(())
            }
            ButtonAction::ShowMap => {
                let _ = self
                    .event_tx
                    .send(ExecutorEvent::NavigationRequest(NavigationTarget::Map));
                tracing::debug!("Navigation: show map");
                Ok(())
            }
            ButtonAction::ShowWorkout => {
                let context = self.context.read().unwrap();
                if !context.workout_active {
                    return Err(ActionError::NoActiveWorkout);
                }
                drop(context);

                let _ = self
                    .event_tx
                    .send(ExecutorEvent::NavigationRequest(NavigationTarget::Workout));
                tracing::debug!("Navigation: show workout");
                Ok(())
            }
            ButtonAction::ToggleFullscreen => {
                let _ = self.event_tx.send(ExecutorEvent::FullscreenToggle);
                tracing::debug!("Navigation: toggle fullscreen");
                Ok(())
            }
            _ => Err(ActionError::NotAvailable(
                "Not a navigation action".to_string(),
            )),
        }
    }

    /// Execute a camera action (placeholder for future 3D world integration)
    async fn execute_camera_action(&self, action: &ButtonAction) -> Result<(), ActionError> {
        match action {
            ButtonAction::CameraZoomIn => {
                // Placeholder - would integrate with 3D world camera
                tracing::debug!("Camera: zoom in (not implemented)");
                Ok(())
            }
            ButtonAction::CameraZoomOut => {
                // Placeholder - would integrate with 3D world camera
                tracing::debug!("Camera: zoom out (not implemented)");
                Ok(())
            }
            ButtonAction::CameraRotate { degrees } => {
                // Placeholder - would integrate with 3D world camera
                tracing::debug!("Camera: rotate {} degrees (not implemented)", degrees);
                Ok(())
            }
            _ => Err(ActionError::NotAvailable("Not a camera action".to_string())),
        }
    }

    /// Execute a custom action
    async fn execute_custom_action(&self, command: &str) -> Result<(), ActionError> {
        let _ = self.event_tx.send(ExecutorEvent::CustomAction {
            command: command.to_string(),
        });
        tracing::info!("Custom action: {}", command);
        Ok(())
    }
}

impl<A: AudioEngine, F: FanController> Default for DefaultActionExecutor<A, F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AudioEngine + 'static, F: FanController + 'static> ActionExecutor
    for DefaultActionExecutor<A, F>
{
    async fn execute(&self, action: &ButtonAction) -> Result<(), ActionError> {
        // Check availability first
        if !self.is_available(action) {
            let reason = match action.category() {
                crate::hid::actions::ActionCategory::RideControl => "No active ride",
                crate::hid::actions::ActionCategory::WorkoutControl => "No active workout",
                _ => "Action not available",
            };
            return Err(ActionError::NotAvailable(reason.to_string()));
        }

        let result = match action {
            // Ride control
            ButtonAction::AddLapMarker | ButtonAction::PauseResume | ButtonAction::EndRide => {
                self.execute_ride_action(action).await
            }

            // Workout control
            ButtonAction::SkipInterval
            | ButtonAction::ExtendInterval { .. }
            | ButtonAction::RestartInterval => self.execute_workout_action(action).await,

            // Audio control
            ButtonAction::VolumeUp | ButtonAction::VolumeDown | ButtonAction::MuteToggle => {
                self.execute_audio_action(action).await
            }

            // Fan control
            ButtonAction::FanSpeedUp | ButtonAction::FanSpeedDown | ButtonAction::FanToggle => {
                self.execute_fan_action(action).await
            }

            // Navigation
            ButtonAction::ShowMetrics
            | ButtonAction::ShowMap
            | ButtonAction::ShowWorkout
            | ButtonAction::ToggleFullscreen => self.execute_navigation_action(action).await,

            // Camera
            ButtonAction::CameraZoomIn
            | ButtonAction::CameraZoomOut
            | ButtonAction::CameraRotate { .. } => self.execute_camera_action(action).await,

            // Custom
            ButtonAction::Custom { command } => self.execute_custom_action(command).await,
        };

        // Emit result
        let success = result.is_ok();
        let message = result.as_ref().err().map(|e| e.to_string());
        self.emit_result(action.clone(), success, message);

        result
    }

    fn available_actions() -> Vec<ActionInfo> {
        ButtonAction::all_actions()
            .into_iter()
            .map(ActionInfo::new)
            .collect()
    }

    fn is_available(&self, action: &ButtonAction) -> bool {
        let context = self.context.read().unwrap();
        let info = ActionInfo::new(action.clone());

        match info.available_during {
            ActionContext::Always => true,
            ActionContext::DuringRide => context.ride_active,
            ActionContext::DuringWorkout => context.workout_active,
            ActionContext::NotDuringRide => !context.ride_active,
        }
    }

    fn subscribe_results(&self) -> broadcast::Receiver<ActionResult> {
        self.result_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::DefaultAudioEngine;
    use crate::integrations::mqtt::{DefaultFanController, DefaultMqttClient};

    // Helper to create test executor
    fn create_test_executor(
    ) -> DefaultActionExecutor<DefaultAudioEngine, DefaultFanController<DefaultMqttClient>> {
        DefaultActionExecutor::new()
    }

    #[test]
    fn test_executor_creation() {
        let executor = create_test_executor();
        assert_eq!(executor.get_volume(), 80);
        assert!(!executor.is_muted());
        assert!(!executor.is_fan_on());
    }

    #[test]
    fn test_context_updates() {
        let executor = create_test_executor();

        executor.set_ride_active(true);
        assert!(executor.context.read().unwrap().ride_active);

        executor.set_workout_active(true);
        assert!(executor.context.read().unwrap().workout_active);

        executor.set_ride_paused(true);
        assert!(executor.context.read().unwrap().ride_paused);
    }

    #[test]
    fn test_action_availability_always() {
        let executor = create_test_executor();

        // Volume actions should always be available
        assert!(executor.is_available(&ButtonAction::VolumeUp));
        assert!(executor.is_available(&ButtonAction::VolumeDown));
        assert!(executor.is_available(&ButtonAction::MuteToggle));
    }

    #[test]
    fn test_action_availability_during_ride() {
        let executor = create_test_executor();

        // Ride actions should not be available without active ride
        assert!(!executor.is_available(&ButtonAction::AddLapMarker));
        assert!(!executor.is_available(&ButtonAction::PauseResume));

        // Enable ride
        executor.set_ride_active(true);
        assert!(executor.is_available(&ButtonAction::AddLapMarker));
        assert!(executor.is_available(&ButtonAction::PauseResume));
    }

    #[test]
    fn test_action_availability_during_workout() {
        let executor = create_test_executor();

        // Workout actions should not be available without active workout
        assert!(!executor.is_available(&ButtonAction::SkipInterval));
        assert!(!executor.is_available(&ButtonAction::ExtendInterval { seconds: 30 }));

        // Enable workout
        executor.set_workout_active(true);
        assert!(executor.is_available(&ButtonAction::SkipInterval));
        assert!(executor.is_available(&ButtonAction::ExtendInterval { seconds: 30 }));
    }

    #[test]
    fn test_available_actions() {
        let actions = DefaultActionExecutor::<
            DefaultAudioEngine,
            DefaultFanController<DefaultMqttClient>,
        >::available_actions();

        assert!(!actions.is_empty());

        // Check that we have actions from different categories
        let has_ride = actions.iter().any(|a| a.name == "Add Lap Marker");
        let has_workout = actions.iter().any(|a| a.name == "Skip Interval");
        let has_audio = actions.iter().any(|a| a.name == "Volume Up");

        assert!(has_ride);
        assert!(has_workout);
        assert!(has_audio);
    }

    #[test]
    fn test_lap_markers() {
        let executor = create_test_executor();
        executor.set_ride_active(true);

        // Add lap marker
        let result = executor.add_lap_marker();
        assert!(result.is_ok());

        let markers = executor.get_lap_markers();
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].lap_number, 1);

        // Add another
        let _ = executor.add_lap_marker();
        let markers = executor.get_lap_markers();
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[1].lap_number, 2);
    }

    #[test]
    fn test_lap_marker_requires_active_ride() {
        let executor = create_test_executor();

        let result = executor.add_lap_marker();
        assert!(matches!(result, Err(ActionError::NoActiveRide)));
    }

    #[test]
    fn test_lap_markers_cleared_on_ride_end() {
        let executor = create_test_executor();
        executor.set_ride_active(true);

        // Add some laps
        let _ = executor.add_lap_marker();
        let _ = executor.add_lap_marker();
        assert_eq!(executor.get_lap_markers().len(), 2);

        // End ride
        executor.set_ride_active(false);
        assert!(executor.get_lap_markers().is_empty());
    }

    #[test]
    fn test_event_subscription() {
        let executor = create_test_executor();

        let mut rx = executor.subscribe_events();
        executor.set_ride_active(true);

        // Add lap marker should emit event
        let _ = executor.add_lap_marker();

        // Check event was emitted (using try_recv since we're not async)
        // In a real async test, we'd await this
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_audio_volume_actions() {
        let executor = create_test_executor();

        // Volume up
        let result = executor.execute_audio_action(&ButtonAction::VolumeUp).await;
        assert!(result.is_ok());
        assert_eq!(executor.get_volume(), 90);

        // Volume down
        let result = executor
            .execute_audio_action(&ButtonAction::VolumeDown)
            .await;
        assert!(result.is_ok());
        assert_eq!(executor.get_volume(), 80);
    }

    #[tokio::test]
    async fn test_mute_toggle() {
        let executor = create_test_executor();

        assert!(!executor.is_muted());

        let result = executor
            .execute_audio_action(&ButtonAction::MuteToggle)
            .await;
        assert!(result.is_ok());
        assert!(executor.is_muted());

        let result = executor
            .execute_audio_action(&ButtonAction::MuteToggle)
            .await;
        assert!(result.is_ok());
        assert!(!executor.is_muted());
    }

    #[tokio::test]
    async fn test_navigation_actions() {
        let executor = create_test_executor();
        let mut rx = executor.subscribe_events();

        let result = executor
            .execute_navigation_action(&ButtonAction::ShowMetrics)
            .await;
        assert!(result.is_ok());

        // Check navigation event was emitted
        if let Ok(event) = rx.try_recv() {
            assert!(matches!(
                event,
                ExecutorEvent::NavigationRequest(NavigationTarget::Metrics)
            ));
        }
    }

    #[tokio::test]
    async fn test_show_workout_requires_active_workout() {
        let executor = create_test_executor();

        let result = executor
            .execute_navigation_action(&ButtonAction::ShowWorkout)
            .await;
        assert!(matches!(result, Err(ActionError::NoActiveWorkout)));

        executor.set_workout_active(true);
        let result = executor
            .execute_navigation_action(&ButtonAction::ShowWorkout)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_custom_action() {
        let executor = create_test_executor();
        let mut rx = executor.subscribe_events();

        let result = executor.execute_custom_action("test_command").await;
        assert!(result.is_ok());

        // Check custom action event was emitted
        if let Ok(event) = rx.try_recv() {
            match event {
                ExecutorEvent::CustomAction { command } => {
                    assert_eq!(command, "test_command");
                }
                _ => panic!("Expected CustomAction event"),
            }
        }
    }
}
