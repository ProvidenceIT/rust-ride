//! T093: Integration tests for HID button control.
//!
//! Tests the HID device manager, button mapping, and action execution.

use rustride::hid::{
    ButtonAction, ButtonInputHandler, ButtonMapping, DefaultButtonInputHandler,
    DefaultHidDeviceManager, HidConfig, HidDevice, HidDeviceManager, HidDeviceStatus,
    KNOWN_DEVICES,
};
use uuid::Uuid;

/// Test HID config defaults.
#[test]
fn test_hid_config_defaults() {
    let config = HidConfig::default();
    assert!(config.enabled);
    assert!(config.devices.is_empty());
}

/// Test known devices list.
#[test]
fn test_known_devices() {
    assert!(!KNOWN_DEVICES.is_empty());

    // Stream Deck should be in the list
    let stream_deck = KNOWN_DEVICES
        .iter()
        .find(|d| d.name.contains("Stream Deck"));
    assert!(stream_deck.is_some());

    // Elgato vendor ID
    assert!(KNOWN_DEVICES.iter().any(|d| d.vendor_id == 0x0FD9));
}

/// Test HID device creation.
#[test]
fn test_hid_device_creation() {
    let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

    assert!(device.is_known);
    assert_eq!(device.vendor_id, 0x0FD9);
    assert_eq!(device.product_id, 0x0060);
    assert_eq!(device.button_count, Some(15));
    assert_eq!(device.status, HidDeviceStatus::Detected);
    assert!(!device.is_open());
}

/// Test unknown device detection.
#[test]
fn test_unknown_device() {
    let device = HidDevice::new(0x1234, 0x5678, "Unknown USB".to_string());

    assert!(!device.is_known);
    assert!(device.button_count.is_none());
}

/// Test device display path.
#[test]
fn test_device_display_path() {
    let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
    assert_eq!(device.display_path(), "0FD9:0060");
}

/// Test HID device manager creation.
#[test]
fn test_device_manager_creation() {
    let config = HidConfig::default();
    let manager = DefaultHidDeviceManager::new(config);

    // Should be able to scan (returns empty without actual devices)
    let devices = manager.scan_devices();
    assert!(devices.is_empty());
}

/// Test button action display names.
#[test]
fn test_button_action_display_names() {
    assert_eq!(ButtonAction::AddLapMarker.display_name(), "Add Lap Marker");
    assert_eq!(ButtonAction::PauseResume.display_name(), "Pause/Resume");
    assert_eq!(ButtonAction::EndRide.display_name(), "End Ride");
    assert_eq!(ButtonAction::SkipInterval.display_name(), "Skip Interval");
    assert_eq!(ButtonAction::VolumeUp.display_name(), "Volume Up");
    assert_eq!(ButtonAction::FanSpeedUp.display_name(), "Fan Speed Up");
}

/// Test button action categories.
#[test]
fn test_button_action_categories() {
    use rustride::hid::actions::ActionCategory;

    assert_eq!(
        ButtonAction::AddLapMarker.category(),
        ActionCategory::RideControl
    );
    assert_eq!(
        ButtonAction::SkipInterval.category(),
        ActionCategory::WorkoutControl
    );
    assert_eq!(ButtonAction::VolumeUp.category(), ActionCategory::Audio);
    assert_eq!(ButtonAction::FanSpeedUp.category(), ActionCategory::Fan);
    assert_eq!(
        ButtonAction::ShowMetrics.category(),
        ActionCategory::Navigation
    );
    assert_eq!(
        ButtonAction::CameraZoomIn.category(),
        ActionCategory::Camera
    );
}

/// Test all available actions.
#[test]
fn test_all_button_actions() {
    let actions = ButtonAction::all_actions();
    assert!(!actions.is_empty());

    // Should include common actions
    assert!(actions.contains(&ButtonAction::AddLapMarker));
    assert!(actions.contains(&ButtonAction::PauseResume));
    assert!(actions.contains(&ButtonAction::SkipInterval));
}

/// Test button mapping creation.
#[test]
fn test_button_mapping_creation() {
    let device_id = Uuid::new_v4();
    let mapping = ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker);

    assert_eq!(mapping.device_id, device_id);
    assert_eq!(mapping.button_code, 1);
    assert_eq!(mapping.action, ButtonAction::AddLapMarker);
    assert!(mapping.enabled);
    assert!(mapping.label.is_none());
}

/// Test button mapping with label.
#[test]
fn test_button_mapping_with_label() {
    let device_id = Uuid::new_v4();
    let mapping =
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker).with_label("Lap Button");

    assert_eq!(mapping.label, Some("Lap Button".to_string()));
}

/// Test button input handler creation.
#[test]
fn test_button_input_handler_creation() {
    let handler = DefaultButtonInputHandler::new();
    assert!(!handler.is_learning());
    assert!(handler.get_learned_button().is_none());
}

/// Test registering button mappings.
#[test]
fn test_register_mappings() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mappings = vec![
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
        ButtonMapping::new(device_id, 2, ButtonAction::PauseResume),
        ButtonMapping::new(device_id, 3, ButtonAction::EndRide),
    ];

    handler.register_mappings(&device_id, mappings);

    let retrieved = handler.get_mappings(&device_id);
    assert_eq!(retrieved.len(), 3);
}

/// Test adding individual mapping.
#[test]
fn test_add_mapping() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mapping = ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker);
    handler.add_mapping(&device_id, mapping);

    let mappings = handler.get_mappings(&device_id);
    assert_eq!(mappings.len(), 1);
}

/// Test clearing mappings.
#[test]
fn test_clear_mappings() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    let mappings = vec![
        ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
        ButtonMapping::new(device_id, 2, ButtonAction::PauseResume),
    ];

    handler.register_mappings(&device_id, mappings);
    assert_eq!(handler.get_mappings(&device_id).len(), 2);

    handler.clear_mappings(&device_id);
    assert_eq!(handler.get_mappings(&device_id).len(), 0);
}

/// Test learning mode.
#[test]
fn test_learning_mode() {
    let handler = DefaultButtonInputHandler::new();
    let device_id = Uuid::new_v4();

    assert!(!handler.is_learning());

    handler.start_learning_mode(&device_id);
    assert!(handler.is_learning());

    handler.stop_learning_mode();
    assert!(!handler.is_learning());
}

/// Test event subscription.
#[test]
fn test_event_subscription() {
    let handler = DefaultButtonInputHandler::new();

    let _action_rx = handler.subscribe_actions();
    let _raw_rx = handler.subscribe_raw();
    // Should be able to subscribe without panic
}

/// Test device manager event subscription.
#[test]
fn test_device_manager_events() {
    let config = HidConfig::default();
    let manager = DefaultHidDeviceManager::new(config);

    let _rx = manager.subscribe_events();
    // Should be able to subscribe without panic
}

/// Test device status transitions.
#[test]
fn test_device_status() {
    let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

    assert_eq!(device.status, HidDeviceStatus::Detected);
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Opening;
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Open;
    assert!(device.is_open());

    device.status = HidDeviceStatus::Error("test error".to_string());
    assert!(!device.is_open());

    device.status = HidDeviceStatus::Disconnected;
    assert!(!device.is_open());
}

/// Test known device lookup.
#[test]
fn test_find_known_device() {
    use rustride::hid::find_known_device;

    // Stream Deck should be found
    let device = find_known_device(0x0FD9, 0x0060);
    assert!(device.is_some());
    assert_eq!(device.unwrap().name, "Elgato Stream Deck");

    // Unknown device should return None
    let unknown = find_known_device(0x1234, 0x5678);
    assert!(unknown.is_none());
}

/// Test action info creation.
#[test]
fn test_action_info() {
    use rustride::hid::actions::{ActionContext, ActionInfo};

    let info = ActionInfo::new(ButtonAction::AddLapMarker);
    assert_eq!(info.name, "Add Lap Marker");
    assert_eq!(info.available_during, ActionContext::DuringRide);

    let volume_info = ActionInfo::new(ButtonAction::VolumeUp);
    assert_eq!(volume_info.available_during, ActionContext::Always);

    let skip_info = ActionInfo::new(ButtonAction::SkipInterval);
    assert_eq!(skip_info.available_during, ActionContext::DuringWorkout);
}

/// Test device with serial number.
#[test]
fn test_device_with_serial() {
    let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
    device.serial_number = Some("ABC123456".to_string());

    assert_eq!(device.serial_number, Some("ABC123456".to_string()));
}

/// Test multiple devices.
#[test]
fn test_multiple_device_mappings() {
    let handler = DefaultButtonInputHandler::new();

    let device1 = Uuid::new_v4();
    let device2 = Uuid::new_v4();

    handler.register_mappings(
        &device1,
        vec![ButtonMapping::new(device1, 1, ButtonAction::AddLapMarker)],
    );
    handler.register_mappings(
        &device2,
        vec![ButtonMapping::new(device2, 1, ButtonAction::PauseResume)],
    );

    assert_eq!(handler.get_mappings(&device1).len(), 1);
    assert_eq!(handler.get_mappings(&device2).len(), 1);

    // Different actions for same button code on different devices
    let m1 = handler.get_mappings(&device1);
    let m2 = handler.get_mappings(&device2);
    assert_eq!(m1[0].action, ButtonAction::AddLapMarker);
    assert_eq!(m2[0].action, ButtonAction::PauseResume);
}

// ============================================================================
// Integration Tests: Action Execution with Subsystem Integration
// ============================================================================

mod action_integration_tests {
    use super::*;
    use rustride::audio::{AudioEngine, AudioError, AudioEvent, AudioItem};
    use rustride::hid::actions::{ActionContext, ActionError, ActionExecutor, ActionResult};
    use rustride::hid::executor::{
        AppContext, DefaultActionExecutor, ExecutorEvent, LapMarker, NavigationTarget,
    };
    use rustride::integrations::mqtt::{FanController, FanProfile, FanState, MqttError};
    use rustride::recording::recorder::RideRecorder;
    use rustride::recording::types::RecorderConfig;
    use rustride::workouts::engine::WorkoutEngine;
    use rustride::workouts::types::{
        PowerTarget, SegmentType, Workout, WorkoutSegment, WorkoutStatus,
    };
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, RwLock};
    use tokio::sync::broadcast;

    // ========================================================================
    // Mock AudioEngine for Integration Testing
    // ========================================================================

    /// Mock audio engine that tracks all method calls for verification
    struct MockAudioEngine {
        volume: AtomicU8,
        muted: AtomicBool,
        set_volume_calls: std::sync::Mutex<Vec<u8>>,
        initialized: AtomicBool,
    }

    impl MockAudioEngine {
        fn new() -> Self {
            Self {
                volume: AtomicU8::new(80),
                muted: AtomicBool::new(false),
                set_volume_calls: std::sync::Mutex::new(Vec::new()),
                initialized: AtomicBool::new(false),
            }
        }

        fn get_set_volume_calls(&self) -> Vec<u8> {
            self.set_volume_calls.lock().unwrap().clone()
        }

        fn last_set_volume(&self) -> Option<u8> {
            self.set_volume_calls.lock().unwrap().last().copied()
        }
    }

    impl AudioEngine for MockAudioEngine {
        fn initialize(&self) -> Result<(), AudioError> {
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn play_sound(&self, _name: &str) -> Result<(), AudioError> {
            Ok(())
        }

        async fn speak(&self, _text: &str) -> Result<(), AudioError> {
            Ok(())
        }

        async fn play_tone(&self, _frequency_hz: u32, _duration_ms: u32) -> Result<(), AudioError> {
            Ok(())
        }

        fn set_volume(&self, volume: u8) {
            self.volume.store(volume, Ordering::SeqCst);
            self.set_volume_calls.lock().unwrap().push(volume);
        }

        fn get_volume(&self) -> u8 {
            self.volume.load(Ordering::SeqCst)
        }

        fn queue(&self, _item: AudioItem) {}

        fn is_playing(&self) -> bool {
            false
        }

        fn stop(&self) {}

        fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
            let (tx, rx) = broadcast::channel(10);
            drop(tx);
            rx
        }
    }

    // ========================================================================
    // Mock FanController for Integration Testing
    // ========================================================================

    /// Mock fan controller that tracks all method calls for verification
    struct MockFanController {
        set_speed_calls: std::sync::Mutex<Vec<(Uuid, u8)>>,
        should_fail: AtomicBool,
        auto_mode: std::sync::Mutex<HashMap<Uuid, bool>>,
    }

    impl MockFanController {
        fn new() -> Self {
            Self {
                set_speed_calls: std::sync::Mutex::new(Vec::new()),
                should_fail: AtomicBool::new(false),
                auto_mode: std::sync::Mutex::new(HashMap::new()),
            }
        }

        fn set_should_fail(&self, fail: bool) {
            self.should_fail.store(fail, Ordering::SeqCst);
        }

        fn get_set_speed_calls(&self) -> Vec<(Uuid, u8)> {
            self.set_speed_calls.lock().unwrap().clone()
        }

        fn last_set_speed(&self) -> Option<(Uuid, u8)> {
            self.set_speed_calls.lock().unwrap().last().copied()
        }
    }

    impl FanController for MockFanController {
        fn configure(&self, _profiles: Vec<FanProfile>) {}

        async fn start(&self) -> Result<(), MqttError> {
            Ok(())
        }

        async fn stop(&self) -> Result<(), MqttError> {
            Ok(())
        }

        fn update_metrics(
            &self,
            _power: u16,
            _hr: Option<u8>,
            _power_zone: u8,
            _hr_zone: Option<u8>,
        ) {
        }

        async fn set_speed(&self, profile_id: &Uuid, speed: u8) -> Result<(), MqttError> {
            if self.should_fail.load(Ordering::SeqCst) {
                return Err(MqttError::NotConnected);
            }
            self.set_speed_calls
                .lock()
                .unwrap()
                .push((*profile_id, speed));
            Ok(())
        }

        fn get_states(&self) -> HashMap<Uuid, FanState> {
            HashMap::new()
        }

        async fn test_fan(&self, _profile_id: &Uuid) -> Result<(), MqttError> {
            Ok(())
        }

        fn set_auto_mode(&self, profile_id: &Uuid, enabled: bool) {
            self.auto_mode.lock().unwrap().insert(*profile_id, enabled);
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a simple test workout with multiple segments
    fn create_test_workout() -> Workout {
        Workout::new(
            "Test HID Workout".to_string(),
            vec![
                WorkoutSegment {
                    segment_type: SegmentType::Warmup,
                    duration_seconds: 60,
                    power_target: PowerTarget::percent_ftp(50),
                    cadence_target: Some(85),
                    text_event: Some("Warmup".to_string()),
                },
                WorkoutSegment {
                    segment_type: SegmentType::SteadyState,
                    duration_seconds: 120,
                    power_target: PowerTarget::percent_ftp(75),
                    cadence_target: Some(90),
                    text_event: Some("Steady".to_string()),
                },
                WorkoutSegment {
                    segment_type: SegmentType::Interval,
                    duration_seconds: 30,
                    power_target: PowerTarget::percent_ftp(120),
                    cadence_target: Some(100),
                    text_event: Some("Sprint!".to_string()),
                },
            ],
        )
    }

    /// Create executor with mock audio engine for testing
    fn create_executor_with_audio() -> (
        DefaultActionExecutor<MockAudioEngine, MockFanController>,
        Arc<MockAudioEngine>,
    ) {
        let audio = Arc::new(MockAudioEngine::new());
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new()
            .with_audio_engine(audio.clone());
        (executor, audio)
    }

    /// Create executor with mock fan controller for testing
    fn create_executor_with_fan() -> (
        DefaultActionExecutor<MockAudioEngine, MockFanController>,
        Arc<MockFanController>,
        Uuid,
    ) {
        let fan = Arc::new(MockFanController::new());
        let profile_id = Uuid::new_v4();
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new()
            .with_fan_controller(fan.clone())
            .with_fan_profile(profile_id);
        (executor, fan, profile_id)
    }

    /// Create executor with ride recorder for testing
    fn create_executor_with_recorder() -> (
        DefaultActionExecutor<MockAudioEngine, MockFanController>,
        Arc<RwLock<RideRecorder>>,
    ) {
        let recorder = Arc::new(RwLock::new(RideRecorder::with_defaults()));
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new()
            .with_ride_recorder(recorder.clone());
        (executor, recorder)
    }

    /// Create executor with workout engine for testing
    fn create_executor_with_workout_engine() -> (
        DefaultActionExecutor<MockAudioEngine, MockFanController>,
        Arc<RwLock<WorkoutEngine>>,
    ) {
        let engine = Arc::new(RwLock::new(WorkoutEngine::new()));
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new()
            .with_workout_engine(engine.clone());
        (executor, engine)
    }

    /// Create a fully wired executor with all subsystems
    fn create_fully_wired_executor() -> (
        DefaultActionExecutor<MockAudioEngine, MockFanController>,
        Arc<MockAudioEngine>,
        Arc<MockFanController>,
        Arc<RwLock<RideRecorder>>,
        Arc<RwLock<WorkoutEngine>>,
        Uuid,
    ) {
        let audio = Arc::new(MockAudioEngine::new());
        let fan = Arc::new(MockFanController::new());
        let recorder = Arc::new(RwLock::new(RideRecorder::with_defaults()));
        let engine = Arc::new(RwLock::new(WorkoutEngine::new()));
        let profile_id = Uuid::new_v4();

        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new()
            .with_audio_engine(audio.clone())
            .with_fan_controller(fan.clone())
            .with_fan_profile(profile_id)
            .with_ride_recorder(recorder.clone())
            .with_workout_engine(engine.clone());

        (executor, audio, fan, recorder, engine, profile_id)
    }

    // ========================================================================
    // Integration Tests: Lap Marker Action with Ride Recorder
    // ========================================================================

    /// Test that AddLapMarker action properly adds lap to the ride recorder
    #[tokio::test]
    async fn test_lap_marker_action_adds_lap_to_recorder() {
        let (executor, recorder) = create_executor_with_recorder();

        // Start a ride to enable lap markers
        {
            let mut rec = recorder.write().unwrap();
            rec.start(Uuid::new_v4(), 250).unwrap();
        }
        executor.set_ride_active(true);

        // Execute the lap marker action
        let result = executor.execute(&ButtonAction::AddLapMarker).await;
        assert!(result.is_ok(), "Lap marker action should succeed");

        // Verify lap was added to the recorder
        {
            let rec = recorder.read().unwrap();
            assert!(rec.has_laps(), "Recorder should have laps");
            assert_eq!(rec.lap_count(), 1, "Recorder should have exactly 1 lap");
        }

        // Add another lap
        let result = executor.execute(&ButtonAction::AddLapMarker).await;
        assert!(result.is_ok(), "Second lap marker action should succeed");

        // Verify second lap was added
        {
            let rec = recorder.read().unwrap();
            assert_eq!(rec.lap_count(), 2, "Recorder should have exactly 2 laps");
        }
    }

    /// Test that AddLapMarker fails when no ride is active
    #[tokio::test]
    async fn test_lap_marker_action_fails_without_active_ride() {
        let (executor, _recorder) = create_executor_with_recorder();

        // Don't start a ride - executor.set_ride_active(false) by default
        let result = executor.execute(&ButtonAction::AddLapMarker).await;
        assert!(
            result.is_err(),
            "Lap marker should fail without active ride"
        );

        match result {
            Err(ActionError::NotAvailable(_)) => {}
            other => panic!("Expected NotAvailable error, got {:?}", other),
        }
    }

    /// Test that lap markers are tracked in executor and emitted as events
    #[tokio::test]
    async fn test_lap_marker_action_emits_event() {
        let (executor, _recorder) = create_executor_with_recorder();
        let mut event_rx = executor.subscribe_events();

        // Start a ride
        executor.set_ride_active(true);

        // Execute the lap marker action
        let result = executor.execute(&ButtonAction::AddLapMarker).await;
        assert!(result.is_ok());

        // Check for LapMarked event
        let mut found_lap_event = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(event, ExecutorEvent::LapMarked(_)) {
                found_lap_event = true;
                break;
            }
        }
        assert!(found_lap_event, "Should emit LapMarked event");

        // Verify executor's internal lap tracking
        let laps = executor.get_lap_markers();
        assert_eq!(laps.len(), 1, "Executor should track 1 lap");
        assert_eq!(laps[0].lap_number, 1, "Lap number should be 1");
    }

    // ========================================================================
    // Integration Tests: Pause/Resume Action with Ride State
    // ========================================================================

    /// Test that PauseResume action toggles ride state correctly
    #[tokio::test]
    async fn test_pause_resume_toggles_ride_state() {
        let (executor, recorder) = create_executor_with_recorder();

        // Start a ride
        {
            let mut rec = recorder.write().unwrap();
            rec.start(Uuid::new_v4(), 250).unwrap();
        }
        executor.set_ride_active(true);
        executor.set_ride_paused(false);

        // Initial state: not paused
        assert!(!executor.context.read().unwrap().ride_paused);

        // Pause the ride
        let result = executor.execute(&ButtonAction::PauseResume).await;
        assert!(result.is_ok(), "Pause action should succeed");
        assert!(
            executor.context.read().unwrap().ride_paused,
            "Ride should be paused"
        );

        // Resume the ride
        let result = executor.execute(&ButtonAction::PauseResume).await;
        assert!(result.is_ok(), "Resume action should succeed");
        assert!(
            !executor.context.read().unwrap().ride_paused,
            "Ride should be resumed"
        );
    }

    /// Test that PauseResume also pauses/resumes the workout engine
    #[tokio::test]
    async fn test_pause_resume_affects_workout_engine() {
        let (executor, audio, _fan, recorder, engine, _profile_id) = create_fully_wired_executor();

        // Start a ride
        {
            let mut rec = recorder.write().unwrap();
            rec.start(Uuid::new_v4(), 250).unwrap();
        }
        executor.set_ride_active(true);
        executor.set_ride_paused(false);

        // Load and start a workout
        {
            let mut eng = engine.write().unwrap();
            eng.load(create_test_workout(), 250).unwrap();
            eng.start().unwrap();
        }
        executor.set_workout_active(true);

        // Verify workout is in progress
        {
            let eng = engine.read().unwrap();
            assert_eq!(eng.state().unwrap().status, WorkoutStatus::InProgress);
        }

        // Pause the ride (should also pause workout)
        let result = executor.execute(&ButtonAction::PauseResume).await;
        assert!(result.is_ok());

        // Verify workout is paused
        {
            let eng = engine.read().unwrap();
            assert_eq!(eng.state().unwrap().status, WorkoutStatus::Paused);
        }

        // Resume
        let result = executor.execute(&ButtonAction::PauseResume).await;
        assert!(result.is_ok());

        // Verify workout is resumed
        {
            let eng = engine.read().unwrap();
            assert_eq!(eng.state().unwrap().status, WorkoutStatus::InProgress);
        }
    }

    // ========================================================================
    // Integration Tests: Skip Interval Action with Workout Engine
    // ========================================================================

    /// Test that SkipInterval action advances the workout to the next segment
    #[tokio::test]
    async fn test_skip_interval_advances_workout() {
        let (executor, engine) = create_executor_with_workout_engine();

        // Load and start a workout
        {
            let mut eng = engine.write().unwrap();
            eng.load(create_test_workout(), 250).unwrap();
            eng.start().unwrap();
        }
        executor.set_workout_active(true);

        // Verify we're in segment 0 (warmup)
        {
            let eng = engine.read().unwrap();
            let progress = eng.state().unwrap().segment_progress.as_ref().unwrap();
            assert_eq!(progress.segment_index, 0, "Should start in segment 0");
        }

        // Skip to next segment
        let result = executor.execute(&ButtonAction::SkipInterval).await;
        assert!(result.is_ok(), "Skip interval should succeed");

        // Verify we're now in segment 1 (steady state)
        {
            let eng = engine.read().unwrap();
            let progress = eng.state().unwrap().segment_progress.as_ref().unwrap();
            assert_eq!(
                progress.segment_index, 1,
                "Should be in segment 1 after skip"
            );
        }

        // Skip again
        let result = executor.execute(&ButtonAction::SkipInterval).await;
        assert!(result.is_ok());

        // Verify we're now in segment 2 (interval)
        {
            let eng = engine.read().unwrap();
            let progress = eng.state().unwrap().segment_progress.as_ref().unwrap();
            assert_eq!(
                progress.segment_index, 2,
                "Should be in segment 2 after second skip"
            );
        }
    }

    /// Test that SkipInterval fails when no workout is active
    #[tokio::test]
    async fn test_skip_interval_fails_without_active_workout() {
        let (executor, _engine) = create_executor_with_workout_engine();

        // Don't start a workout
        let result = executor.execute(&ButtonAction::SkipInterval).await;
        assert!(
            result.is_err(),
            "Skip interval should fail without active workout"
        );

        match result {
            Err(ActionError::NotAvailable(_)) => {}
            other => panic!("Expected NotAvailable error, got {:?}", other),
        }
    }

    /// Test that ExtendInterval extends the current segment duration
    #[tokio::test]
    async fn test_extend_interval_extends_segment() {
        let (executor, engine) = create_executor_with_workout_engine();

        // Load and start a workout
        {
            let mut eng = engine.write().unwrap();
            eng.load(create_test_workout(), 250).unwrap();
            eng.start().unwrap();
        }
        executor.set_workout_active(true);

        // Get initial remaining time
        let initial_remaining = {
            let eng = engine.read().unwrap();
            eng.state()
                .unwrap()
                .segment_progress
                .as_ref()
                .unwrap()
                .remaining_seconds
        };

        // Extend by 30 seconds
        let result = executor
            .execute(&ButtonAction::ExtendInterval { seconds: 30 })
            .await;
        assert!(result.is_ok(), "Extend interval should succeed");

        // Verify remaining time increased
        {
            let eng = engine.read().unwrap();
            let remaining = eng
                .state()
                .unwrap()
                .segment_progress
                .as_ref()
                .unwrap()
                .remaining_seconds;
            assert!(
                remaining >= initial_remaining + 25,
                "Remaining time should increase by ~30s (got {} from {})",
                remaining,
                initial_remaining
            );
        }
    }

    /// Test that RestartInterval resets the current segment
    #[tokio::test]
    async fn test_restart_interval_resets_segment() {
        let (executor, engine) = create_executor_with_workout_engine();

        // Load and start a workout
        {
            let mut eng = engine.write().unwrap();
            eng.load(create_test_workout(), 250).unwrap();
            eng.start().unwrap();
        }
        executor.set_workout_active(true);

        // Advance time by ticking
        {
            let mut eng = engine.write().unwrap();
            for _ in 0..30 {
                eng.tick();
            }
        }

        // Verify some time has passed
        {
            let eng = engine.read().unwrap();
            let elapsed = eng.state().unwrap().total_elapsed_seconds;
            assert!(elapsed >= 30, "Should have elapsed 30+ seconds");
        }

        // Restart the interval
        let result = executor.execute(&ButtonAction::RestartInterval).await;
        assert!(result.is_ok(), "Restart interval should succeed");

        // Verify segment restarted (elapsed time reset to start of segment)
        {
            let eng = engine.read().unwrap();
            let elapsed = eng.state().unwrap().total_elapsed_seconds;
            // Elapsed time should be reset to 0 (start of segment 0)
            assert!(elapsed < 30, "Elapsed time should be reset after restart");
        }
    }

    // ========================================================================
    // Integration Tests: Volume Actions with Audio Engine
    // ========================================================================

    /// Test that VolumeUp action increases audio volume
    #[tokio::test]
    async fn test_volume_up_changes_audio_level() {
        let (executor, audio) = create_executor_with_audio();

        // Get initial volume
        let initial_volume = executor.get_volume();

        // Volume up
        let result = executor.execute(&ButtonAction::VolumeUp).await;
        assert!(result.is_ok(), "Volume up should succeed");

        // Verify volume increased
        let new_volume = executor.get_volume();
        assert!(new_volume > initial_volume, "Volume should increase");

        // Verify audio engine was called
        let calls = audio.get_set_volume_calls();
        assert!(!calls.is_empty(), "Audio engine should be called");
        assert_eq!(
            calls[0], new_volume,
            "Audio engine should receive new volume"
        );
    }

    /// Test that VolumeDown action decreases audio volume
    #[tokio::test]
    async fn test_volume_down_changes_audio_level() {
        let (executor, audio) = create_executor_with_audio();

        // Get initial volume
        let initial_volume = executor.get_volume();

        // Volume down
        let result = executor.execute(&ButtonAction::VolumeDown).await;
        assert!(result.is_ok(), "Volume down should succeed");

        // Verify volume decreased
        let new_volume = executor.get_volume();
        assert!(new_volume < initial_volume, "Volume should decrease");

        // Verify audio engine was called
        let calls = audio.get_set_volume_calls();
        assert!(!calls.is_empty(), "Audio engine should be called");
        assert_eq!(
            calls[0], new_volume,
            "Audio engine should receive new volume"
        );
    }

    /// Test that MuteToggle action toggles audio mute state
    #[tokio::test]
    async fn test_mute_toggle_changes_audio_state() {
        let (executor, audio) = create_executor_with_audio();

        // Initially not muted
        assert!(!executor.is_muted(), "Should not be muted initially");

        // Toggle mute (mute)
        let result = executor.execute(&ButtonAction::MuteToggle).await;
        assert!(result.is_ok(), "Mute toggle should succeed");
        assert!(executor.is_muted(), "Should be muted after toggle");

        // Verify audio engine was set to volume 0
        assert_eq!(
            audio.last_set_volume(),
            Some(0),
            "Audio volume should be 0 when muted"
        );

        // Toggle mute (unmute)
        let result = executor.execute(&ButtonAction::MuteToggle).await;
        assert!(result.is_ok(), "Unmute toggle should succeed");
        assert!(
            !executor.is_muted(),
            "Should be unmuted after second toggle"
        );

        // Verify audio engine was restored to previous volume
        let restored_volume = audio.last_set_volume().unwrap();
        assert!(
            restored_volume > 0,
            "Volume should be restored after unmute"
        );
    }

    /// Test that volume actions clamp at boundaries
    #[tokio::test]
    async fn test_volume_clamped_at_boundaries() {
        let (executor, audio) = create_executor_with_audio();

        // Set volume near max
        for _ in 0..15 {
            let _ = executor.execute(&ButtonAction::VolumeUp).await;
        }

        // Verify clamped at 100
        assert_eq!(
            executor.get_volume(),
            100,
            "Volume should be clamped at 100"
        );

        // Set volume near min
        for _ in 0..15 {
            let _ = executor.execute(&ButtonAction::VolumeDown).await;
        }

        // Verify clamped at 0
        assert_eq!(executor.get_volume(), 0, "Volume should be clamped at 0");
    }

    // ========================================================================
    // Integration Tests: Fan Actions with Fan Controller
    // ========================================================================

    /// Test that FanSpeedUp action increases fan speed
    #[tokio::test]
    async fn test_fan_speed_up_calls_controller() {
        let (executor, fan, profile_id) = create_executor_with_fan();

        // Get initial speed
        let initial_speed = executor.get_fan_speed();

        // Speed up
        let result = executor.execute(&ButtonAction::FanSpeedUp).await;
        assert!(result.is_ok(), "Fan speed up should succeed");

        // Verify speed increased
        let new_speed = executor.get_fan_speed();
        assert!(new_speed > initial_speed, "Fan speed should increase");

        // Verify controller was called with correct profile and speed
        let calls = fan.get_set_speed_calls();
        assert!(!calls.is_empty(), "Fan controller should be called");
        assert_eq!(calls[0].0, profile_id, "Should use correct profile ID");
        assert_eq!(calls[0].1, new_speed, "Should set correct speed");
    }

    /// Test that FanToggle action toggles fan on/off
    #[tokio::test]
    async fn test_fan_toggle_calls_controller() {
        let (executor, fan, profile_id) = create_executor_with_fan();

        // Initially fan is off
        assert!(!executor.is_fan_on(), "Fan should be off initially");

        // Toggle on
        let result = executor.execute(&ButtonAction::FanToggle).await;
        assert!(result.is_ok(), "Fan toggle should succeed");
        assert!(executor.is_fan_on(), "Fan should be on after toggle");

        // Verify controller was called with current speed
        let calls = fan.get_set_speed_calls();
        let current_speed = executor.get_fan_speed();
        assert_eq!(
            calls[0].1, current_speed,
            "Should set to current speed when turning on"
        );

        // Toggle off
        let result = executor.execute(&ButtonAction::FanToggle).await;
        assert!(result.is_ok(), "Fan toggle off should succeed");
        assert!(
            !executor.is_fan_on(),
            "Fan should be off after second toggle"
        );

        // Verify controller was called with speed 0
        let calls = fan.get_set_speed_calls();
        assert_eq!(calls[1].1, 0, "Should set speed to 0 when turning off");
    }

    /// Test that fan actions fail gracefully when MQTT is not connected
    #[tokio::test]
    async fn test_fan_action_graceful_failure_on_mqtt_error() {
        let (executor, fan, _profile_id) = create_executor_with_fan();

        // Configure mock to fail
        fan.set_should_fail(true);

        // Try fan action
        let result = executor.execute(&ButtonAction::FanSpeedUp).await;
        assert!(result.is_err(), "Should fail when MQTT not connected");

        match result {
            Err(ActionError::ExecutionFailed(msg)) => {
                assert!(
                    msg.contains("Not connected"),
                    "Error should mention connection"
                );
            }
            other => panic!("Expected ExecutionFailed error, got {:?}", other),
        }
    }

    // ========================================================================
    // Integration Tests: Navigation Actions with Events
    // ========================================================================

    /// Test that navigation actions emit correct events
    #[tokio::test]
    async fn test_navigation_actions_emit_events() {
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new();
        let mut event_rx = executor.subscribe_events();

        // ShowMetrics
        let result = executor.execute(&ButtonAction::ShowMetrics).await;
        assert!(result.is_ok());

        // Check for navigation event
        let mut found_event = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(
                event,
                ExecutorEvent::NavigationRequest(NavigationTarget::Metrics)
            ) {
                found_event = true;
                break;
            }
        }
        assert!(found_event, "Should emit NavigationRequest(Metrics) event");

        // ShowMap
        let result = executor.execute(&ButtonAction::ShowMap).await;
        assert!(result.is_ok());

        let mut found_event = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(
                event,
                ExecutorEvent::NavigationRequest(NavigationTarget::Map)
            ) {
                found_event = true;
                break;
            }
        }
        assert!(found_event, "Should emit NavigationRequest(Map) event");

        // ToggleFullscreen
        let result = executor.execute(&ButtonAction::ToggleFullscreen).await;
        assert!(result.is_ok());

        let mut found_event = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(event, ExecutorEvent::FullscreenToggle) {
                found_event = true;
                break;
            }
        }
        assert!(found_event, "Should emit FullscreenToggle event");
    }

    /// Test that ShowWorkout requires active workout
    #[tokio::test]
    async fn test_show_workout_requires_active_workout() {
        let executor = DefaultActionExecutor::<MockAudioEngine, MockFanController>::new();

        // Without active workout
        let result = executor.execute(&ButtonAction::ShowWorkout).await;
        assert!(
            result.is_err(),
            "ShowWorkout should fail without active workout"
        );

        // With active workout
        executor.set_workout_active(true);
        let result = executor.execute(&ButtonAction::ShowWorkout).await;
        assert!(
            result.is_ok(),
            "ShowWorkout should succeed with active workout"
        );
    }

    // ========================================================================
    // Integration Tests: Mock Device Manager for CI Environments
    // ========================================================================

    /// Test that mock device manager works without actual hardware
    #[test]
    fn test_mock_device_manager_for_ci() {
        let config = HidConfig::default();
        let manager = DefaultHidDeviceManager::new(config);

        // Should be able to scan (returns empty without actual devices)
        let devices = manager.scan_devices();
        // In CI, this will be empty (no real devices connected)
        // This test verifies the system doesn't crash/panic without hardware

        // Should be able to subscribe to events
        let _event_rx = manager.subscribe_events();

        // Should be able to get devices (empty)
        assert!(devices.is_empty() || devices.iter().all(|d| d.is_known));
    }

    /// Test that executor works correctly with all mock subsystems
    #[tokio::test]
    async fn test_fully_mocked_executor_integration() {
        let (executor, audio, fan, recorder, engine, profile_id) = create_fully_wired_executor();

        // Start a ride
        {
            let mut rec = recorder.write().unwrap();
            rec.start(Uuid::new_v4(), 250).unwrap();
        }
        executor.set_ride_active(true);

        // Load and start a workout
        {
            let mut eng = engine.write().unwrap();
            eng.load(create_test_workout(), 250).unwrap();
            eng.start().unwrap();
        }
        executor.set_workout_active(true);

        // Test all action types work correctly

        // Ride control
        let result = executor.execute(&ButtonAction::AddLapMarker).await;
        assert!(result.is_ok(), "Lap marker should succeed");

        // Workout control
        let result = executor.execute(&ButtonAction::SkipInterval).await;
        assert!(result.is_ok(), "Skip interval should succeed");

        // Audio control
        let result = executor.execute(&ButtonAction::VolumeUp).await;
        assert!(result.is_ok(), "Volume up should succeed");

        // Fan control
        let result = executor.execute(&ButtonAction::FanSpeedUp).await;
        assert!(result.is_ok(), "Fan speed up should succeed");

        // Navigation
        let result = executor.execute(&ButtonAction::ShowMetrics).await;
        assert!(result.is_ok(), "Show metrics should succeed");

        // Verify all subsystems were called
        assert!(
            audio.get_set_volume_calls().len() >= 1,
            "Audio should be called"
        );
        assert!(fan.get_set_speed_calls().len() >= 1, "Fan should be called");
        {
            let rec = recorder.read().unwrap();
            assert!(rec.has_laps(), "Recorder should have laps");
        }
        {
            let eng = engine.read().unwrap();
            let progress = eng.state().unwrap().segment_progress.as_ref().unwrap();
            assert_eq!(
                progress.segment_index, 1,
                "Should be in segment 1 after skip"
            );
        }
    }

    /// Test that action results are emitted correctly
    #[tokio::test]
    async fn test_action_results_emitted() {
        let (executor, _audio) = create_executor_with_audio();
        let mut event_rx = executor.subscribe_events();

        // Execute an action
        let _ = executor.execute(&ButtonAction::VolumeUp).await;

        // Check for ActionExecuted event
        let mut found_result = false;
        while let Ok(event) = event_rx.try_recv() {
            if let ExecutorEvent::ActionExecuted(result) = event {
                assert!(result.success, "Action should be successful");
                assert_eq!(result.action, ButtonAction::VolumeUp);
                found_result = true;
                break;
            }
        }
        assert!(found_result, "Should emit ActionExecuted event");
    }

    /// Test end-to-end button press to action execution flow
    #[tokio::test]
    async fn test_end_to_end_button_to_action_flow() {
        use rustride::hid::mapping::{ButtonActionEvent, RawButtonEvent};
        use std::time::Instant;

        // Create button input handler
        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register a mapping
        let mapping = ButtonMapping::new(device_id, 1, ButtonAction::VolumeUp);
        handler.add_mapping(&device_id, mapping);

        // Subscribe to action events
        let mut action_rx = handler.subscribe_actions();

        // Simulate a raw button press event
        let raw_event = RawButtonEvent {
            device_id,
            button_code: 1,
            pressed: true,
            timestamp: Instant::now(),
        };

        // Process the event
        handler.process_event(raw_event).await;

        // Verify action event was emitted
        let action_event = action_rx.try_recv();
        assert!(action_event.is_ok(), "Should receive action event");

        let event = action_event.unwrap();
        assert_eq!(event.device_id, device_id);
        assert_eq!(event.action, ButtonAction::VolumeUp);

        // Now test that the action executes against a subsystem
        let (executor, audio) = create_executor_with_audio();
        let result = executor.execute(&event.action).await;
        assert!(result.is_ok(), "Action should execute successfully");

        // Verify audio was affected
        assert!(
            !audio.get_set_volume_calls().is_empty(),
            "Audio should be called"
        );
    }

    /// Test that disabled mappings don't trigger actions
    #[tokio::test]
    async fn test_disabled_mappings_ignored() {
        use rustride::hid::mapping::RawButtonEvent;
        use std::time::Instant;

        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register a disabled mapping
        let mut mapping = ButtonMapping::new(device_id, 1, ButtonAction::VolumeUp);
        mapping.enabled = false;
        handler.add_mapping(&device_id, mapping);

        // Subscribe to action events
        let mut action_rx = handler.subscribe_actions();

        // Simulate a raw button press
        let raw_event = RawButtonEvent {
            device_id,
            button_code: 1,
            pressed: true,
            timestamp: Instant::now(),
        };

        handler.process_event(raw_event).await;

        // Verify NO action event was emitted
        let action_event = action_rx.try_recv();
        assert!(
            action_event.is_err(),
            "Disabled mapping should not trigger action"
        );
    }

    /// Test learning mode captures button but doesn't emit action
    #[tokio::test]
    async fn test_learning_mode_captures_button_no_action() {
        use rustride::hid::mapping::RawButtonEvent;
        use std::time::Instant;

        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register a mapping
        let mapping = ButtonMapping::new(device_id, 1, ButtonAction::VolumeUp);
        handler.add_mapping(&device_id, mapping);

        // Start learning mode
        handler.start_learning_mode(&device_id);
        assert!(handler.is_learning(), "Should be in learning mode");

        // Subscribe to action events
        let mut action_rx = handler.subscribe_actions();

        // Simulate a button press
        let raw_event = RawButtonEvent {
            device_id,
            button_code: 5, // Different button
            pressed: true,
            timestamp: Instant::now(),
        };

        handler.process_event(raw_event).await;

        // Verify button was learned
        let learned = handler.get_learned_button();
        assert_eq!(learned, Some(5), "Should capture button code 5");

        // Verify NO action was triggered
        let action_event = action_rx.try_recv();
        assert!(
            action_event.is_err(),
            "Learning mode should not trigger actions"
        );

        // Stop learning mode
        handler.stop_learning_mode();
        assert!(!handler.is_learning(), "Should not be in learning mode");
    }
}

// ============================================================================
// Reconnection Stress Tests: Device Disconnect/Reconnect Scenarios
// ============================================================================

mod reconnection_stress_tests {
    use super::*;
    use rustride::hid::{
        get_default_mappings, DefaultHidDeviceManager, HidConfig, HidDevice, HidDeviceEvent,
        HidDeviceManager, HidDeviceStatus,
    };
    use tokio::sync::broadcast;

    // ========================================================================
    // Test Device Disconnect Emits Correct Events
    // ========================================================================

    /// Test that device disconnect event contains correct device ID
    #[test]
    fn test_disconnect_event_contains_device_id() {
        let device_id = Uuid::new_v4();
        let event = HidDeviceEvent::DeviceDisconnected(device_id);

        match event {
            HidDeviceEvent::DeviceDisconnected(id) => {
                assert_eq!(
                    id, device_id,
                    "Disconnect event should contain correct device ID"
                );
            }
            _ => panic!("Expected DeviceDisconnected event"),
        }
    }

    /// Test that device connected event contains device info
    #[test]
    fn test_connect_event_contains_device_info() {
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        let device_id = device.id;
        let event = HidDeviceEvent::DeviceConnected(device);

        match event {
            HidDeviceEvent::DeviceConnected(dev) => {
                assert_eq!(dev.id, device_id);
                assert_eq!(dev.vendor_id, 0x0FD9);
                assert_eq!(dev.product_id, 0x0060);
                assert_eq!(dev.name, "Stream Deck");
            }
            _ => panic!("Expected DeviceConnected event"),
        }
    }

    /// Test device status transitions on disconnect
    #[test]
    fn test_device_status_transitions_on_disconnect() {
        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        // Initial state
        assert_eq!(device.status, HidDeviceStatus::Detected);

        // Simulate open
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());

        // Simulate disconnect
        device.status = HidDeviceStatus::Disconnected;
        assert!(!device.is_open());
        assert_eq!(device.status, HidDeviceStatus::Disconnected);
    }

    /// Test that event channel supports multiple disconnect events
    #[test]
    fn test_multiple_disconnect_events() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let device1_id = Uuid::new_v4();
        let device2_id = Uuid::new_v4();
        let device3_id = Uuid::new_v4();

        // Send multiple disconnect events
        tx.send(HidDeviceEvent::DeviceDisconnected(device1_id))
            .unwrap();
        tx.send(HidDeviceEvent::DeviceDisconnected(device2_id))
            .unwrap();
        tx.send(HidDeviceEvent::DeviceDisconnected(device3_id))
            .unwrap();

        // Verify all events received in order
        let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert_eq!(events.len(), 3);

        match &events[0] {
            HidDeviceEvent::DeviceDisconnected(id) => assert_eq!(*id, device1_id),
            _ => panic!("Expected disconnect event"),
        }
        match &events[1] {
            HidDeviceEvent::DeviceDisconnected(id) => assert_eq!(*id, device2_id),
            _ => panic!("Expected disconnect event"),
        }
        match &events[2] {
            HidDeviceEvent::DeviceDisconnected(id) => assert_eq!(*id, device3_id),
            _ => panic!("Expected disconnect event"),
        }
    }

    // ========================================================================
    // Test Device Reconnect Restores Mappings
    // ========================================================================

    /// Test that mappings are preserved after device reconnect
    #[tokio::test]
    async fn test_mappings_preserved_after_reconnect() {
        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register mappings
        let mappings = vec![
            ButtonMapping::new(device_id, 0, ButtonAction::PauseResume),
            ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
            ButtonMapping::new(device_id, 2, ButtonAction::SkipInterval),
        ];
        handler.register_mappings(&device_id, mappings.clone());

        // Verify mappings exist
        assert_eq!(handler.get_mappings(&device_id).len(), 3);

        // Simulate disconnect (mappings should still be in memory for handler)
        // In real scenario, ButtonInputHandler keeps mappings even when device disconnects

        // Verify mappings still accessible after "disconnect"
        let retrieved = handler.get_mappings(&device_id);
        assert_eq!(retrieved.len(), 3);
        assert_eq!(retrieved[0].action, ButtonAction::PauseResume);
        assert_eq!(retrieved[1].action, ButtonAction::AddLapMarker);
        assert_eq!(retrieved[2].action, ButtonAction::SkipInterval);
    }

    /// Test that default mappings can be restored for known devices
    #[test]
    fn test_default_mappings_restored_for_known_device() {
        // Get default mappings for Stream Deck
        let mappings = get_default_mappings(0x0FD9, 0x0060);

        assert!(
            !mappings.is_empty(),
            "Stream Deck should have default mappings"
        );
        assert_eq!(mappings[0].button_code, 0);
        assert_eq!(mappings[0].action, ButtonAction::PauseResume);

        // Verify we can re-apply these mappings
        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        let button_mappings: Vec<ButtonMapping> = mappings
            .iter()
            .map(|config| ButtonMapping::new(device_id, config.button_code, config.action.clone()))
            .collect();

        handler.register_mappings(&device_id, button_mappings);

        let registered = handler.get_mappings(&device_id);
        assert_eq!(registered.len(), mappings.len());
    }

    /// Test that reconnected device emits correct event
    #[test]
    fn test_reconnected_event_emitted() {
        let device_id = Uuid::new_v4();
        let event = HidDeviceEvent::DeviceReconnected(device_id);

        match event {
            HidDeviceEvent::DeviceReconnected(id) => {
                assert_eq!(
                    id, device_id,
                    "Reconnected event should contain correct device ID"
                );
            }
            _ => panic!("Expected DeviceReconnected event"),
        }
    }

    /// Test that device can transition back to Open status after reconnect
    #[test]
    fn test_device_status_restored_after_reconnect() {
        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        // Initial state
        assert_eq!(device.status, HidDeviceStatus::Detected);

        // Open
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());

        // Disconnect
        device.status = HidDeviceStatus::Disconnected;
        assert!(!device.is_open());

        // Reconnect (back to open)
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());
        assert_eq!(device.status, HidDeviceStatus::Open);
    }

    // ========================================================================
    // Test Rapid Disconnect/Reconnect Cycles
    // ========================================================================

    /// Test rapid status changes don't corrupt device state
    #[test]
    fn test_rapid_status_changes() {
        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        // Simulate rapid cycling
        for _ in 0..100 {
            device.status = HidDeviceStatus::Opening;
            device.status = HidDeviceStatus::Open;
            device.status = HidDeviceStatus::Disconnected;
            device.status = HidDeviceStatus::Detected;
        }

        // Device should be in valid state
        assert_eq!(device.status, HidDeviceStatus::Detected);
        assert!(!device.is_open());

        // Open again after rapid cycling
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());
    }

    /// Test rapid event emission doesn't block
    #[test]
    fn test_rapid_event_emission() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let device_id = Uuid::new_v4();

        // Rapidly emit connect/disconnect events
        for i in 0..50 {
            let device = HidDevice::new(0x0FD9, 0x0060, format!("Device {}", i));
            tx.send(HidDeviceEvent::DeviceConnected(device)).unwrap();
            tx.send(HidDeviceEvent::DeviceDisconnected(device_id))
                .unwrap();
        }

        // All 100 events should be sent (50 connect + 50 disconnect)
        let mut event_count = 0;
        while rx.try_recv().is_ok() {
            event_count += 1;
        }
        assert_eq!(event_count, 100, "All rapid events should be received");
    }

    /// Test rapid mapping registration/clear cycles
    #[tokio::test]
    async fn test_rapid_mapping_cycles() {
        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        for cycle in 0..50 {
            // Register mappings
            let mappings = vec![
                ButtonMapping::new(device_id, 0, ButtonAction::PauseResume),
                ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
            ];
            handler.register_mappings(&device_id, mappings);
            assert_eq!(
                handler.get_mappings(&device_id).len(),
                2,
                "Cycle {}: Should have 2 mappings",
                cycle
            );

            // Clear mappings (simulate disconnect cleanup)
            handler.clear_mappings(&device_id);
            assert_eq!(
                handler.get_mappings(&device_id).len(),
                0,
                "Cycle {}: Should have 0 mappings after clear",
                cycle
            );
        }

        // Final state should be consistent
        assert_eq!(handler.get_mappings(&device_id).len(), 0);

        // Can still add mappings after rapid cycles
        handler.add_mapping(
            &device_id,
            ButtonMapping::new(device_id, 5, ButtonAction::VolumeUp),
        );
        assert_eq!(handler.get_mappings(&device_id).len(), 1);
    }

    /// Test interleaved operations from multiple devices
    #[tokio::test]
    async fn test_interleaved_multi_device_operations() {
        let handler = DefaultButtonInputHandler::new();

        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();
        let device3 = Uuid::new_v4();

        // Interleave operations across devices
        for _ in 0..20 {
            handler.add_mapping(
                &device1,
                ButtonMapping::new(device1, 0, ButtonAction::PauseResume),
            );
            handler.add_mapping(
                &device2,
                ButtonMapping::new(device2, 0, ButtonAction::VolumeUp),
            );
            handler.add_mapping(
                &device3,
                ButtonMapping::new(device3, 0, ButtonAction::FanToggle),
            );

            handler.clear_mappings(&device2);

            handler.add_mapping(
                &device1,
                ButtonMapping::new(device1, 1, ButtonAction::AddLapMarker),
            );
            handler.add_mapping(
                &device3,
                ButtonMapping::new(device3, 1, ButtonAction::MuteToggle),
            );
        }

        // Verify each device has correct state
        let d1_mappings = handler.get_mappings(&device1);
        let d2_mappings = handler.get_mappings(&device2);
        let d3_mappings = handler.get_mappings(&device3);

        // Device 1: 40 mappings (20 button 0 + 20 button 1)
        assert_eq!(d1_mappings.len(), 40);

        // Device 2: 0 mappings (cleared every iteration)
        assert_eq!(d2_mappings.len(), 0);

        // Device 3: 40 mappings (20 button 0 + 20 button 1)
        assert_eq!(d3_mappings.len(), 40);
    }

    // ========================================================================
    // Test Multiple Device Disconnect Scenarios
    // ========================================================================

    /// Test all devices disconnect simultaneously
    #[test]
    fn test_all_devices_disconnect() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let devices: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

        // Simulate all devices disconnecting
        for device_id in &devices {
            tx.send(HidDeviceEvent::DeviceDisconnected(*device_id))
                .unwrap();
        }

        // Collect all disconnect events
        let events: Vec<Uuid> = std::iter::from_fn(|| {
            rx.try_recv().ok().and_then(|e| {
                if let HidDeviceEvent::DeviceDisconnected(id) = e {
                    Some(id)
                } else {
                    None
                }
            })
        })
        .collect();

        // All devices should have disconnect events
        assert_eq!(events.len(), 5);
        for device_id in &devices {
            assert!(
                events.contains(device_id),
                "Missing disconnect for device {:?}",
                device_id
            );
        }
    }

    /// Test partial device disconnect (some remain connected)
    #[test]
    fn test_partial_device_disconnect() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let devices: Vec<(Uuid, bool)> = vec![
            (Uuid::new_v4(), true),  // disconnects
            (Uuid::new_v4(), false), // stays connected
            (Uuid::new_v4(), true),  // disconnects
            (Uuid::new_v4(), false), // stays connected
            (Uuid::new_v4(), true),  // disconnects
        ];

        // Only disconnect some devices
        for (device_id, should_disconnect) in &devices {
            if *should_disconnect {
                tx.send(HidDeviceEvent::DeviceDisconnected(*device_id))
                    .unwrap();
            }
        }

        // Count disconnect events
        let mut disconnect_count = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, HidDeviceEvent::DeviceDisconnected(_)) {
                disconnect_count += 1;
            }
        }

        assert_eq!(disconnect_count, 3, "Should only have 3 disconnect events");
    }

    /// Test device mappings isolated across devices during disconnect
    #[tokio::test]
    async fn test_device_mappings_isolated_on_disconnect() {
        let handler = DefaultButtonInputHandler::new();

        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();
        let device3 = Uuid::new_v4();

        // Register mappings for all devices
        handler.register_mappings(
            &device1,
            vec![
                ButtonMapping::new(device1, 0, ButtonAction::PauseResume),
                ButtonMapping::new(device1, 1, ButtonAction::AddLapMarker),
            ],
        );
        handler.register_mappings(
            &device2,
            vec![
                ButtonMapping::new(device2, 0, ButtonAction::VolumeUp),
                ButtonMapping::new(device2, 1, ButtonAction::VolumeDown),
            ],
        );
        handler.register_mappings(
            &device3,
            vec![ButtonMapping::new(device3, 0, ButtonAction::FanSpeedUp)],
        );

        // Verify initial state
        assert_eq!(handler.get_mappings(&device1).len(), 2);
        assert_eq!(handler.get_mappings(&device2).len(), 2);
        assert_eq!(handler.get_mappings(&device3).len(), 1);

        // Simulate device1 disconnect (clear its mappings)
        handler.clear_mappings(&device1);

        // Device1 should have no mappings, others should be unaffected
        assert_eq!(handler.get_mappings(&device1).len(), 0);
        assert_eq!(handler.get_mappings(&device2).len(), 2);
        assert_eq!(handler.get_mappings(&device3).len(), 1);

        // Simulate device2 disconnect
        handler.clear_mappings(&device2);

        assert_eq!(handler.get_mappings(&device1).len(), 0);
        assert_eq!(handler.get_mappings(&device2).len(), 0);
        assert_eq!(handler.get_mappings(&device3).len(), 1);

        // Device3 still has its mapping
        let d3_mappings = handler.get_mappings(&device3);
        assert_eq!(d3_mappings[0].action, ButtonAction::FanSpeedUp);
    }

    /// Test sequential reconnection of multiple devices
    #[test]
    fn test_sequential_multi_device_reconnect() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let device_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

        // Disconnect all
        for id in &device_ids {
            tx.send(HidDeviceEvent::DeviceDisconnected(*id)).unwrap();
        }

        // Reconnect in reverse order
        for id in device_ids.iter().rev() {
            tx.send(HidDeviceEvent::DeviceReconnected(*id)).unwrap();
        }

        // Collect events
        let mut disconnects = Vec::new();
        let mut reconnects = Vec::new();

        while let Ok(event) = rx.try_recv() {
            match event {
                HidDeviceEvent::DeviceDisconnected(id) => disconnects.push(id),
                HidDeviceEvent::DeviceReconnected(id) => reconnects.push(id),
                _ => {}
            }
        }

        assert_eq!(disconnects.len(), 3);
        assert_eq!(reconnects.len(), 3);

        // Reconnects should be in reverse order
        assert_eq!(reconnects[0], device_ids[2]);
        assert_eq!(reconnects[1], device_ids[1]);
        assert_eq!(reconnects[2], device_ids[0]);
    }

    // ========================================================================
    // Test HID Manager Auto-Reconnect Tracking
    // ========================================================================

    /// Test device manager tracks devices for auto-reconnect
    #[test]
    fn test_device_manager_auto_reconnect_tracking() {
        let config = HidConfig::default();
        let manager = DefaultHidDeviceManager::new(config);

        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device.serial_number = Some("SN12345".to_string());

        // Verify auto-reconnect is not tracked initially
        assert!(!manager.should_auto_reconnect(&device.identity_key()));

        // Track device for reconnect
        manager.track_for_reconnect(&device);

        // Now should be tracked
        assert!(manager.should_auto_reconnect(&device.identity_key()));
    }

    /// Test device manager removes from tracking on untrack
    #[test]
    fn test_device_manager_untrack_reconnect() {
        let config = HidConfig::default();
        let manager = DefaultHidDeviceManager::new(config);

        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device.serial_number = Some("SN12345".to_string());

        // Track and then untrack
        manager.track_for_reconnect(&device);
        assert!(manager.should_auto_reconnect(&device.identity_key()));

        manager.untrack_for_reconnect(&device);
        assert!(!manager.should_auto_reconnect(&device.identity_key()));
    }

    /// Test multiple devices tracked independently for reconnect
    #[test]
    fn test_multiple_devices_independent_reconnect_tracking() {
        let config = HidConfig::default();
        let manager = DefaultHidDeviceManager::new(config);

        let mut device1 = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device1.serial_number = Some("SN001".to_string());

        let mut device2 = HidDevice::new(0x0FD9, 0x006C, "Stream Deck Mini".to_string());
        device2.serial_number = Some("SN002".to_string());

        let mut device3 = HidDevice::new(0x0FD9, 0x0086, "Stream Deck Pedal".to_string());
        device3.serial_number = Some("SN003".to_string());

        // Track all devices
        manager.track_for_reconnect(&device1);
        manager.track_for_reconnect(&device2);
        manager.track_for_reconnect(&device3);

        // All should be tracked
        assert!(manager.should_auto_reconnect(&device1.identity_key()));
        assert!(manager.should_auto_reconnect(&device2.identity_key()));
        assert!(manager.should_auto_reconnect(&device3.identity_key()));

        // Untrack device2
        manager.untrack_for_reconnect(&device2);

        // Device2 should not be tracked, others should still be tracked
        assert!(manager.should_auto_reconnect(&device1.identity_key()));
        assert!(!manager.should_auto_reconnect(&device2.identity_key()));
        assert!(manager.should_auto_reconnect(&device3.identity_key()));
    }

    /// Test reconnect config can be updated at runtime
    #[tokio::test]
    async fn test_reconnect_config_update() {
        let config = HidConfig::default();
        let manager = DefaultHidDeviceManager::new(config);

        // Check default config
        let (enabled, delay) = manager.get_reconnect_config().await;
        assert!(enabled);
        assert_eq!(delay, 1000);

        // Update config
        manager.set_reconnect_config(false, 500).await;

        // Verify updated
        let (enabled, delay) = manager.get_reconnect_config().await;
        assert!(!enabled);
        assert_eq!(delay, 500);

        // Update again with different values
        manager.set_reconnect_config(true, 2000).await;

        let (enabled, delay) = manager.get_reconnect_config().await;
        assert!(enabled);
        assert_eq!(delay, 2000);
    }

    /// Test device identity key generation for tracking
    #[test]
    fn test_device_identity_key_variations() {
        // With serial number
        let mut device1 = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device1.serial_number = Some("ABC123".to_string());
        assert_eq!(device1.identity_key(), "0FD9:0060:ABC123");

        // With path only
        let mut device2 = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device2.device_path = Some("/dev/hidraw0".to_string());
        assert_eq!(device2.identity_key(), "0FD9:0060:/dev/hidraw0");

        // With both (serial takes precedence)
        let mut device3 = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device3.serial_number = Some("XYZ789".to_string());
        device3.device_path = Some("/dev/hidraw1".to_string());
        assert_eq!(device3.identity_key(), "0FD9:0060:XYZ789");

        // With neither (VID:PID only)
        let device4 = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        assert_eq!(device4.identity_key(), "0FD9:0060");
    }

    // ========================================================================
    // Test Reconnection with Button Event Processing
    // ========================================================================

    /// Test button events still processed after simulated reconnect cycle
    #[tokio::test]
    async fn test_button_events_processed_after_reconnect() {
        use rustride::hid::mapping::RawButtonEvent;
        use std::time::Instant;

        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register mappings
        handler.register_mappings(
            &device_id,
            vec![
                ButtonMapping::new(device_id, 0, ButtonAction::PauseResume),
                ButtonMapping::new(device_id, 1, ButtonAction::AddLapMarker),
            ],
        );

        // Subscribe to action events
        let mut action_rx = handler.subscribe_actions();

        // Process event before "disconnect"
        let event1 = RawButtonEvent {
            device_id,
            button_code: 0,
            pressed: true,
            timestamp: Instant::now(),
        };
        handler.process_event(event1).await;

        // Verify action emitted
        let action = action_rx.try_recv();
        assert!(action.is_ok(), "Should receive action before disconnect");
        assert_eq!(action.unwrap().action, ButtonAction::PauseResume);

        // Simulate "disconnect" (mappings still exist in handler)
        // In real scenario, the device handle would be closed but mappings remain

        // Simulate "reconnect" (device comes back)
        // Process event after "reconnect"
        let event2 = RawButtonEvent {
            device_id,
            button_code: 1,
            pressed: true,
            timestamp: Instant::now(),
        };
        handler.process_event(event2).await;

        // Verify action still emitted
        let action = action_rx.try_recv();
        assert!(action.is_ok(), "Should receive action after reconnect");
        assert_eq!(action.unwrap().action, ButtonAction::AddLapMarker);
    }

    /// Test rapid button events during reconnect cycle
    #[tokio::test]
    async fn test_rapid_button_events_during_reconnect() {
        use rustride::hid::mapping::RawButtonEvent;
        use std::time::Instant;

        let handler = DefaultButtonInputHandler::new();
        let device_id = Uuid::new_v4();

        // Register mappings
        handler.register_mappings(
            &device_id,
            vec![ButtonMapping::new(device_id, 0, ButtonAction::VolumeUp)],
        );

        let mut action_rx = handler.subscribe_actions();

        // Simulate rapid press/release during reconnect scenario
        for i in 0..20 {
            let pressed = i % 2 == 0;
            let event = RawButtonEvent {
                device_id,
                button_code: 0,
                pressed,
                timestamp: Instant::now(),
            };
            handler.process_event(event).await;
        }

        // Count received actions (should be 10 presses, releases don't emit by default)
        let mut action_count = 0;
        while action_rx.try_recv().is_ok() {
            action_count += 1;
        }

        // Only pressed events (true) should trigger actions
        assert_eq!(action_count, 10, "Should receive 10 press events");
    }

    // ========================================================================
    // Test Error Event Handling
    // ========================================================================

    /// Test error events during reconnect attempts
    #[test]
    fn test_error_events_during_reconnect() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let device_id = Uuid::new_v4();

        // Simulate reconnect attempt that fails
        tx.send(HidDeviceEvent::Error {
            device_id: Some(device_id),
            error: "Auto-reconnect failed: device busy".to_string(),
        })
        .unwrap();

        let event = rx.try_recv().unwrap();
        match event {
            HidDeviceEvent::Error {
                device_id: Some(id),
                error,
            } => {
                assert_eq!(id, device_id);
                assert!(error.contains("Auto-reconnect failed"));
            }
            _ => panic!("Expected Error event"),
        }
    }

    /// Test error events without device ID (general errors)
    #[test]
    fn test_general_error_events() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        tx.send(HidDeviceEvent::Error {
            device_id: None,
            error: "HID API initialization failed".to_string(),
        })
        .unwrap();

        let event = rx.try_recv().unwrap();
        match event {
            HidDeviceEvent::Error {
                device_id: None,
                error,
            } => {
                assert!(error.contains("HID API"));
            }
            _ => panic!("Expected Error event with no device ID"),
        }
    }

    /// Test mixed event stream during reconnect cycle
    #[test]
    fn test_mixed_event_stream_during_reconnect() {
        let (tx, mut rx) = broadcast::channel::<HidDeviceEvent>(100);

        let device_id = Uuid::new_v4();
        let device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        // Simulate a typical reconnect cycle event sequence
        tx.send(HidDeviceEvent::DeviceDisconnected(device_id))
            .unwrap();
        tx.send(HidDeviceEvent::Error {
            device_id: Some(device_id),
            error: "First reconnect attempt failed".to_string(),
        })
        .unwrap();
        tx.send(HidDeviceEvent::DeviceConnected(device.clone()))
            .unwrap();
        tx.send(HidDeviceEvent::DeviceReconnected(device_id))
            .unwrap();
        tx.send(HidDeviceEvent::DeviceOpened(device_id)).unwrap();

        // Collect and verify all events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], HidDeviceEvent::DeviceDisconnected(_)));
        assert!(matches!(events[1], HidDeviceEvent::Error { .. }));
        assert!(matches!(events[2], HidDeviceEvent::DeviceConnected(_)));
        assert!(matches!(events[3], HidDeviceEvent::DeviceReconnected(_)));
        assert!(matches!(events[4], HidDeviceEvent::DeviceOpened(_)));
    }

    // ========================================================================
    // Test Device State Consistency
    // ========================================================================

    /// Test device state remains consistent through lifecycle
    #[test]
    fn test_device_state_consistency() {
        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());
        device.serial_number = Some("TEST123".to_string());

        // Track all state through lifecycle
        let initial_id = device.id;
        let initial_serial = device.serial_number.clone();

        // Simulate full lifecycle
        assert_eq!(device.status, HidDeviceStatus::Detected);

        device.status = HidDeviceStatus::Opening;
        assert_eq!(device.id, initial_id);
        assert_eq!(device.serial_number, initial_serial);

        device.status = HidDeviceStatus::Open;
        assert_eq!(device.id, initial_id);
        assert_eq!(device.serial_number, initial_serial);

        device.status = HidDeviceStatus::Disconnected;
        assert_eq!(device.id, initial_id);
        assert_eq!(device.serial_number, initial_serial);

        device.status = HidDeviceStatus::Detected;
        assert_eq!(device.id, initial_id);
        assert_eq!(device.serial_number, initial_serial);

        device.status = HidDeviceStatus::Open;
        assert_eq!(device.id, initial_id);
        assert_eq!(device.serial_number, initial_serial);

        // Core properties unchanged
        assert_eq!(device.vendor_id, 0x0FD9);
        assert_eq!(device.product_id, 0x0060);
        assert_eq!(device.name, "Stream Deck");
    }

    /// Test error state recovery
    #[test]
    fn test_error_state_recovery() {
        let mut device = HidDevice::new(0x0FD9, 0x0060, "Stream Deck".to_string());

        // Start normally
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());

        // Simulate error
        device.status = HidDeviceStatus::Error("Permission denied".to_string());
        assert!(!device.is_open());

        // Recover from error
        device.status = HidDeviceStatus::Detected;
        assert!(!device.is_open());

        // Successfully open again
        device.status = HidDeviceStatus::Open;
        assert!(device.is_open());
    }
}
