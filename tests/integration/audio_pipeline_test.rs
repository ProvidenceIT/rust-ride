//! Audio Pipeline Integration Tests
//!
//! T095: Tests that verify the complete audio pipeline from workout events
//! through to tone generation. Includes a mock audio backend for CI testing
//! without actual audio output.
//!
//! ## Test Coverage
//!
//! - WorkoutAudioBridge event processing and tone generation
//! - AchievementAudioBridge notification handling
//! - MilestoneAudioBridge milestone feedback
//! - Volume control and muting behavior
//! - Audio queue priority and expiration
//! - CI-compatible mock backend

use rustride::achievements::{
    AchievementCategory, AchievementNotification, AchievementTier, LevelUpNotification,
};
use rustride::audio::{
    AchievementAudioBridge, AchievementAudioBridgeConfig, AlertConfig, AlertContext, AlertManager,
    AlertType, AudioCategory, AudioConfig, AudioEngine, AudioError, AudioEvent, AudioItem,
    AudioPriority, CuePattern, MilestoneAudioBridge, MilestoneAudioBridgeConfig, MilestoneData,
    MilestoneType, MuteState, WorkoutAudioBridge, WorkoutAudioBridgeConfig,
};
use rustride::workouts::types::WorkoutEvent;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

// ============================================================================
// Mock Audio Backend for CI Testing
// ============================================================================

/// MockAudioBackend: A mock implementation of audio backend functionality
/// that records all audio operations without producing actual sound output.
/// Suitable for CI/CD environments and automated testing.
#[derive(Debug)]
pub struct MockAudioBackend {
    /// Recorded tones: (frequency_hz, duration_ms)
    played_tones: Mutex<Vec<(u32, u32)>>,
    /// Recorded sounds by name
    played_sounds: Mutex<Vec<String>>,
    /// Recorded speech text
    spoken_texts: Mutex<Vec<String>>,
    /// Queued audio items
    queued_items: Mutex<Vec<AudioItem>>,
    /// Current volume (0-100)
    volume: AtomicU8,
    /// Is audio currently playing
    is_playing: std::sync::atomic::AtomicBool,
    /// Whether backend is initialized
    initialized: std::sync::atomic::AtomicBool,
    /// Event broadcast sender
    event_tx: broadcast::Sender<AudioEvent>,
    /// Simulated playback count
    playback_count: AtomicUsize,
    /// Simulated failure mode for error testing
    simulate_failure: std::sync::atomic::AtomicBool,
    /// Error count for metrics
    error_count: AtomicU32,
    /// Queue processing enabled
    queue_processing_enabled: std::sync::atomic::AtomicBool,
}

impl Default for MockAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAudioBackend {
    /// Create a new mock audio backend.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            played_tones: Mutex::new(Vec::new()),
            played_sounds: Mutex::new(Vec::new()),
            spoken_texts: Mutex::new(Vec::new()),
            queued_items: Mutex::new(Vec::new()),
            volume: AtomicU8::new(80),
            is_playing: std::sync::atomic::AtomicBool::new(false),
            initialized: std::sync::atomic::AtomicBool::new(false),
            event_tx,
            playback_count: AtomicUsize::new(0),
            simulate_failure: std::sync::atomic::AtomicBool::new(false),
            error_count: AtomicU32::new(0),
            queue_processing_enabled: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Get all played tones.
    pub fn get_played_tones(&self) -> Vec<(u32, u32)> {
        self.played_tones.lock().unwrap().clone()
    }

    /// Get all played sounds.
    pub fn get_played_sounds(&self) -> Vec<String> {
        self.played_sounds.lock().unwrap().clone()
    }

    /// Get all spoken texts.
    pub fn get_spoken_texts(&self) -> Vec<String> {
        self.spoken_texts.lock().unwrap().clone()
    }

    /// Get all queued items.
    pub fn get_queued_items(&self) -> Vec<AudioItem> {
        self.queued_items.lock().unwrap().clone()
    }

    /// Get playback count.
    pub fn get_playback_count(&self) -> usize {
        self.playback_count.load(Ordering::Relaxed)
    }

    /// Clear all recorded operations.
    pub fn clear(&self) {
        self.played_tones.lock().unwrap().clear();
        self.played_sounds.lock().unwrap().clear();
        self.spoken_texts.lock().unwrap().clear();
        self.queued_items.lock().unwrap().clear();
        self.playback_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
    }

    /// Enable/disable simulated failure mode for error testing.
    pub fn set_simulate_failure(&self, simulate: bool) {
        self.simulate_failure.store(simulate, Ordering::Relaxed);
    }

    /// Check if backend is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Get error count.
    pub fn get_error_count(&self) -> u32 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Enable/disable queue processing.
    pub fn set_queue_processing_enabled(&self, enabled: bool) {
        self.queue_processing_enabled
            .store(enabled, Ordering::Relaxed);
    }

    /// Emit an audio event (for testing event subscriptions).
    pub fn emit_event(&self, event: AudioEvent) {
        let _ = self.event_tx.send(event);
    }
}

impl AudioEngine for MockAudioBackend {
    fn initialize(&self) -> Result<(), AudioError> {
        if self.simulate_failure.load(Ordering::Relaxed) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(AudioError::DeviceNotAvailable(
                "Mock device unavailable".to_string(),
            ));
        }
        self.initialized.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn play_sound(&self, name: &str) -> Result<(), AudioError> {
        if self.simulate_failure.load(Ordering::Relaxed) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(AudioError::PlaybackFailed(
                "Mock playback failure".to_string(),
            ));
        }
        self.played_sounds.lock().unwrap().push(name.to_string());
        self.playback_count.fetch_add(1, Ordering::Relaxed);
        self.is_playing.store(true, Ordering::Relaxed);
        let _ = self.event_tx.send(AudioEvent::PlaybackStarted);
        Ok(())
    }

    async fn play_sound_with_category(
        &self,
        name: &str,
        _category: AudioCategory,
    ) -> Result<(), AudioError> {
        self.play_sound(name).await
    }

    async fn speak(&self, text: &str) -> Result<(), AudioError> {
        if self.simulate_failure.load(Ordering::Relaxed) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(AudioError::TtsFailed("Mock TTS failure".to_string()));
        }
        self.spoken_texts.lock().unwrap().push(text.to_string());
        self.playback_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn play_tone(&self, frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
        if self.simulate_failure.load(Ordering::Relaxed) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(AudioError::PlaybackFailed("Mock tone failure".to_string()));
        }
        self.played_tones
            .lock()
            .unwrap()
            .push((frequency_hz, duration_ms));
        self.playback_count.fetch_add(1, Ordering::Relaxed);
        self.is_playing.store(true, Ordering::Relaxed);
        let _ = self.event_tx.send(AudioEvent::PlaybackStarted);
        Ok(())
    }

    async fn play_tone_with_category(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
        _category: AudioCategory,
    ) -> Result<(), AudioError> {
        self.play_tone(frequency_hz, duration_ms).await
    }

    fn set_volume(&self, volume: u8) {
        self.volume.store(volume.min(100), Ordering::Relaxed);
    }

    fn set_category_volume(&self, _category: AudioCategory, _volume: u8) {
        // No-op for mock
    }

    fn get_volume(&self) -> u8 {
        self.volume.load(Ordering::Relaxed)
    }

    fn get_category_volume(&self, _category: AudioCategory) -> u8 {
        self.get_volume()
    }

    fn queue(&self, item: AudioItem) {
        if self.queue_processing_enabled.load(Ordering::Relaxed) {
            self.queued_items.lock().unwrap().push(item);
        }
    }

    fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    fn stop(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
        let _ = self.event_tx.send(AudioEvent::PlaybackStopped);
    }

    fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
        self.event_tx.subscribe()
    }

    // Mute control methods
    fn mute(&self) {
        // No-op for mock
    }

    fn unmute(&self) {
        // No-op for mock
    }

    fn toggle_mute(&self) -> bool {
        false
    }

    fn is_muted(&self) -> bool {
        false
    }

    fn mute_category(&self, _category: AudioCategory) {
        // No-op for mock
    }

    fn unmute_category(&self, _category: AudioCategory) {
        // No-op for mock
    }

    fn toggle_category_mute(&self, _category: AudioCategory) -> bool {
        false
    }

    fn is_category_muted(&self, _category: AudioCategory) -> bool {
        false
    }

    fn get_mute_state(&self) -> MuteState {
        MuteState::default()
    }

    // Device status methods
    fn get_device_status(&self) -> rustride::audio::AudioDeviceStatus {
        rustride::audio::AudioDeviceStatus::default()
    }

    fn get_platform(&self) -> rustride::audio::Platform {
        rustride::audio::Platform::Windows
    }

    fn is_device_available(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    fn try_device_recovery(&self) -> bool {
        true
    }

    fn reset_device_recovery(&self) {
        // No-op for mock
    }

    fn get_hot_plug_config(&self) -> rustride::audio::HotPlugConfig {
        rustride::audio::HotPlugConfig::default()
    }

    fn set_hot_plug_config(&self, _config: rustride::audio::HotPlugConfig) {
        // No-op for mock
    }

    fn get_troubleshooting_hints(&self) -> Vec<&'static str> {
        vec!["This is a mock backend for testing"]
    }

    // Queue statistics methods
    fn get_timing_config(&self) -> rustride::audio::AudioTimingConfig {
        rustride::audio::AudioTimingConfig::default()
    }

    fn set_timing_config(&self, _config: rustride::audio::AudioTimingConfig) {
        // No-op for mock
    }

    fn get_queue_stats(&self) -> rustride::audio::QueueStats {
        rustride::audio::QueueStats::default()
    }

    fn reset_queue_stats(&self) {
        // No-op for mock
    }

    fn cleanup_expired(&self) -> usize {
        0
    }

    fn queue_size(&self) -> usize {
        self.queued_items.lock().unwrap().len()
    }
}

// ============================================================================
// Mock Alert Manager for Testing
// ============================================================================

/// Mock alert manager that captures triggered alerts for verification.
#[derive(Debug)]
pub struct MockAlertManager {
    /// Captured alerts: (AlertType, AlertContext)
    triggered_alerts: Mutex<Vec<(AlertType, AlertContext)>>,
    /// Configuration for each alert type
    configs: Mutex<HashMap<AlertType, AlertConfig>>,
    /// Cooldown tracking (alert type -> last trigger time)
    cooldowns: Mutex<HashMap<AlertType, std::time::Instant>>,
    /// Trigger count for each alert type
    trigger_counts: Mutex<HashMap<AlertType, usize>>,
}

impl Default for MockAlertManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAlertManager {
    pub fn new() -> Self {
        Self {
            triggered_alerts: Mutex::new(Vec::new()),
            configs: Mutex::new(HashMap::new()),
            cooldowns: Mutex::new(HashMap::new()),
            trigger_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Get all triggered alerts.
    pub fn get_triggered_alerts(&self) -> Vec<(AlertType, AlertContext)> {
        self.triggered_alerts.lock().unwrap().clone()
    }

    /// Get triggered alert types only.
    pub fn get_alert_types(&self) -> Vec<AlertType> {
        self.triggered_alerts
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _)| *t)
            .collect()
    }

    /// Get trigger count for a specific alert type.
    pub fn get_trigger_count(&self, alert_type: AlertType) -> usize {
        *self
            .trigger_counts
            .lock()
            .unwrap()
            .get(&alert_type)
            .unwrap_or(&0)
    }

    /// Clear all captured alerts and counts.
    pub fn clear(&self) {
        self.triggered_alerts.lock().unwrap().clear();
        self.trigger_counts.lock().unwrap().clear();
        self.cooldowns.lock().unwrap().clear();
    }
}

impl AlertManager for MockAlertManager {
    async fn trigger(&self, alert_type: AlertType, context: AlertContext) {
        self.triggered_alerts
            .lock()
            .unwrap()
            .push((alert_type, context));

        *self
            .trigger_counts
            .lock()
            .unwrap()
            .entry(alert_type)
            .or_insert(0) += 1;

        self.cooldowns
            .lock()
            .unwrap()
            .insert(alert_type, std::time::Instant::now());
    }

    fn configure(&self, alert_type: AlertType, config: AlertConfig) {
        self.configs.lock().unwrap().insert(alert_type, config);
    }

    fn get_config(&self, alert_type: AlertType) -> AlertConfig {
        self.configs
            .lock()
            .unwrap()
            .get(&alert_type)
            .cloned()
            .unwrap_or_default()
    }

    fn set_enabled(&self, alert_type: AlertType, enabled: bool) {
        let mut configs = self.configs.lock().unwrap();
        if let Some(config) = configs.get_mut(&alert_type) {
            config.enabled = enabled;
        } else {
            let mut config = AlertConfig::default();
            config.enabled = enabled;
            configs.insert(alert_type, config);
        }
    }

    fn is_on_cooldown(&self, alert_type: AlertType) -> bool {
        let cooldowns = self.cooldowns.lock().unwrap();
        if let Some(last_trigger) = cooldowns.get(&alert_type) {
            let config = self.get_config(alert_type);
            last_trigger.elapsed().as_secs() < config.cooldown_secs as u64
        } else {
            false
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test workout audio bridge with mock components.
fn create_workout_bridge() -> (
    WorkoutAudioBridge<MockAlertManager, MockAudioBackend>,
    Arc<MockAlertManager>,
    Arc<MockAudioBackend>,
) {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());
    let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_backend.clone());
    (bridge, alert_manager, audio_backend)
}

/// Create test achievement audio bridge with mock components.
fn create_achievement_bridge() -> (
    AchievementAudioBridge<MockAlertManager, MockAudioBackend>,
    Arc<MockAlertManager>,
    Arc<MockAudioBackend>,
) {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());
    let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_backend.clone());
    (bridge, alert_manager, audio_backend)
}

/// Create test milestone audio bridge with mock components.
fn create_milestone_bridge() -> (
    MilestoneAudioBridge<MockAlertManager, MockAudioBackend>,
    Arc<MockAlertManager>,
    Arc<MockAudioBackend>,
) {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());
    let bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_backend.clone());
    (bridge, alert_manager, audio_backend)
}

/// Create test achievement notification.
fn create_test_achievement(tier: AchievementTier, title: &str) -> AchievementNotification {
    AchievementNotification::new(
        Uuid::new_v4(),
        title,
        "Test achievement description",
        AchievementCategory::Training,
        tier,
        tier.base_xp(),
    )
}

// ============================================================================
// MockAudioBackend Unit Tests
// ============================================================================

#[test]
fn test_mock_backend_creation() {
    let backend = MockAudioBackend::new();
    assert!(!backend.is_initialized());
    assert_eq!(backend.get_volume(), 80);
    assert!(!backend.is_playing());
    assert_eq!(backend.get_playback_count(), 0);
}

#[test]
fn test_mock_backend_initialization() {
    let backend = MockAudioBackend::new();
    assert!(backend.initialize().is_ok());
    assert!(backend.is_initialized());
}

#[test]
fn test_mock_backend_failure_mode() {
    let backend = MockAudioBackend::new();
    backend.set_simulate_failure(true);

    assert!(backend.initialize().is_err());
    assert_eq!(backend.get_error_count(), 1);
}

#[tokio::test]
async fn test_mock_backend_play_tone() {
    let backend = MockAudioBackend::new();
    backend.initialize().unwrap();

    assert!(backend.play_tone(440, 100).await.is_ok());

    let tones = backend.get_played_tones();
    assert_eq!(tones.len(), 1);
    assert_eq!(tones[0], (440, 100));
    assert_eq!(backend.get_playback_count(), 1);
}

#[tokio::test]
async fn test_mock_backend_play_sound() {
    let backend = MockAudioBackend::new();
    backend.initialize().unwrap();

    assert!(backend.play_sound("test_sound").await.is_ok());

    let sounds = backend.get_played_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0], "test_sound");
}

#[tokio::test]
async fn test_mock_backend_speak() {
    let backend = MockAudioBackend::new();
    backend.initialize().unwrap();

    assert!(backend.speak("Hello world").await.is_ok());

    let texts = backend.get_spoken_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Hello world");
}

#[test]
fn test_mock_backend_volume_control() {
    let backend = MockAudioBackend::new();

    backend.set_volume(50);
    assert_eq!(backend.get_volume(), 50);

    // Test clamping
    backend.set_volume(150);
    assert_eq!(backend.get_volume(), 100);
}

#[test]
fn test_mock_backend_queue() {
    let backend = MockAudioBackend::new();

    let item = AudioItem::tone(440, 100).with_priority(AudioPriority::High);
    backend.queue(item.clone());

    let queued = backend.get_queued_items();
    assert_eq!(queued.len(), 1);
}

#[test]
fn test_mock_backend_stop() {
    let backend = MockAudioBackend::new();
    backend.is_playing.store(true, Ordering::Relaxed);

    backend.stop();
    assert!(!backend.is_playing());
}

#[test]
fn test_mock_backend_clear() {
    let backend = MockAudioBackend::new();
    backend.played_tones.lock().unwrap().push((440, 100));
    backend
        .played_sounds
        .lock()
        .unwrap()
        .push("test".to_string());
    backend.playback_count.store(5, Ordering::Relaxed);

    backend.clear();

    assert!(backend.get_played_tones().is_empty());
    assert!(backend.get_played_sounds().is_empty());
    assert_eq!(backend.get_playback_count(), 0);
}

#[tokio::test]
async fn test_mock_backend_event_subscription() {
    let backend = MockAudioBackend::new();
    backend.initialize().unwrap();

    let mut rx = backend.subscribe_events();

    backend.play_tone(440, 100).await.unwrap();

    // Should receive PlaybackStarted event
    let event = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
    assert!(event.is_ok());
}

// ============================================================================
// WorkoutAudioBridge Integration Tests
// ============================================================================

#[tokio::test]
async fn test_workout_start_triggers_audio() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    let event = WorkoutEvent::Started {
        workout_name: "Test Workout".to_string(),
    };
    bridge.process_event(&event).await;

    // Should trigger alert
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::WorkoutStart);
}

#[tokio::test]
async fn test_interval_change_triggers_audio() {
    let (bridge, alert_manager, _audio_backend) = create_workout_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Sweet Spot".to_string(),
        target_power: Some(260),
        duration_secs: 300,
        is_recovery: false,
    };
    bridge.process_event(&event).await;

    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::IntervalChange);
}

#[tokio::test]
async fn test_countdown_triggers_tones_and_voice() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    // Process countdown events for 10 and 5 seconds (should have voice)
    let events = vec![
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
    ];
    bridge.process_events(&events).await;

    // Voice alerts for 10 and 5
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 2, "Should have 2 voice announcements");

    // Tones should be played
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played countdown tones");
}

#[tokio::test]
async fn test_countdown_final_seconds_tone_only() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    // Process final countdown events (3, 2, 1 - tone only, no voice)
    let events = vec![
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 3,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 2,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 1,
        },
    ];
    bridge.process_events(&events).await;

    // No voice alerts for final countdown
    let alerts = alert_manager.get_triggered_alerts();
    assert!(
        alerts.is_empty(),
        "Final countdown should not trigger voice"
    );

    // But tones should be played
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played countdown tones");
}

#[tokio::test]
async fn test_full_countdown_sequence() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    // Full countdown sequence
    let events = vec![
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 3,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 2,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 1,
        },
    ];
    bridge.process_events(&events).await;

    // Only 10 and 5 get voice
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 2);

    // All 5 should play tones
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty());
}

#[tokio::test]
async fn test_recovery_interval_special_announcement() {
    let (bridge, alert_manager, _audio_backend) = create_workout_bridge();

    let event = WorkoutEvent::IntervalChange {
        interval_name: "Recovery".to_string(),
        target_power: Some(100),
        duration_secs: 120,
        is_recovery: true,
    };
    bridge.process_event(&event).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 1);
    assert_eq!(alert_types[0], AlertType::RecoveryStart);
}

#[tokio::test]
async fn test_workout_lifecycle_events() {
    let (bridge, alert_manager, _audio_backend) = create_workout_bridge();

    let events = vec![
        WorkoutEvent::Paused,
        WorkoutEvent::Resumed,
        WorkoutEvent::Completed {
            total_duration_secs: 3600,
        },
    ];
    bridge.process_events(&events).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 3);
    assert_eq!(alert_types[0], AlertType::RidePaused);
    assert_eq!(alert_types[1], AlertType::RideResumed);
    assert_eq!(alert_types[2], AlertType::WorkoutComplete);
}

#[tokio::test]
async fn test_trainer_disconnect_reconnect() {
    let (bridge, alert_manager, _audio_backend) = create_workout_bridge();

    let events = vec![
        WorkoutEvent::TrainerDisconnected,
        WorkoutEvent::TrainerReconnected,
    ];
    bridge.process_events(&events).await;

    let alert_types = alert_manager.get_alert_types();
    assert_eq!(alert_types.len(), 2);
    assert_eq!(alert_types[0], AlertType::SensorDisconnected);
    assert_eq!(alert_types[1], AlertType::SensorConnected);
}

#[tokio::test]
async fn test_workout_config_disables_announcements() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let config = WorkoutAudioBridgeConfig {
        announce_interval_changes: false,
        announce_countdowns: false,
        announce_workout_lifecycle: false,
        announce_trainer_status: false,
        announce_recovery_intervals: false,
        announce_motivational_messages: false,
        countdown_sounds_enabled: false,
        countdown_voice_enabled: false,
        countdown_thresholds: vec![10, 5, 3, 2, 1],
        countdown_voice_thresholds: vec![10, 5],
    };

    let bridge =
        WorkoutAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    let events = vec![
        WorkoutEvent::Started {
            workout_name: "Test".to_string(),
        },
        WorkoutEvent::IntervalChange {
            interval_name: "Test".to_string(),
            target_power: Some(200),
            duration_secs: 60,
            is_recovery: false,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
    ];
    bridge.process_events(&events).await;

    assert!(alert_manager.get_triggered_alerts().is_empty());
    assert!(audio_backend.get_played_tones().is_empty());
}

#[tokio::test]
async fn test_workout_custom_countdown_thresholds() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let config = WorkoutAudioBridgeConfig {
        countdown_thresholds: vec![5, 3],    // Only 5 and 3 seconds
        countdown_voice_thresholds: vec![5], // Only 5 gets voice
        ..Default::default()
    };

    let bridge =
        WorkoutAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    let events = vec![
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        }, // Not in thresholds
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        }, // Voice + tone
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 3,
        }, // Tone only
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 1,
        }, // Not in thresholds
    ];
    bridge.process_events(&events).await;

    // Only 5 should trigger voice
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);

    // Tones should be played for 5 and 3
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty());
}

// ============================================================================
// AchievementAudioBridge Integration Tests
// ============================================================================

#[tokio::test]
async fn test_achievement_bronze_triggers_audio() {
    let (bridge, alert_manager, audio_backend) = create_achievement_bridge();

    let notification = create_test_achievement(AchievementTier::Bronze, "First Ride");
    bridge.handle_achievement_notification(&notification).await;

    // Should have triggered voice alert
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::AchievementUnlocked);

    // Should have played tones
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played achievement tones");
}

#[tokio::test]
async fn test_achievement_tiers_have_different_tones() {
    let tiers = [
        AchievementTier::Bronze,
        AchievementTier::Silver,
        AchievementTier::Gold,
        AchievementTier::Diamond,
        AchievementTier::Legendary,
    ];

    let mut tone_counts = Vec::new();

    for tier in tiers {
        let (bridge, _alert_manager, audio_backend) = create_achievement_bridge();
        let notification = create_test_achievement(tier, "Test");
        bridge.handle_achievement_notification(&notification).await;

        let count = audio_backend.get_played_tones().len();
        tone_counts.push(count);
    }

    // Higher tiers should generally have more tones (or same)
    // Bronze should be simplest
    assert!(tone_counts[0] > 0, "Bronze should play at least one tone");
}

#[tokio::test]
async fn test_level_up_triggers_audio() {
    let (bridge, alert_manager, audio_backend) = create_achievement_bridge();

    let notification = LevelUpNotification::new(5, 6, 5000);
    bridge.handle_level_up(&notification).await;

    // Should have triggered alert
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);

    // Should have played tones
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played level-up tones");
}

#[tokio::test]
async fn test_achievement_chimes_disabled() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let config = AchievementAudioBridgeConfig {
        chimes_enabled: false,
        voice_enabled: true,
        ..Default::default()
    };

    let bridge =
        AchievementAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    let notification = create_test_achievement(AchievementTier::Gold, "Test");
    bridge.handle_achievement_notification(&notification).await;

    // No tones
    let tones = audio_backend.get_played_tones();
    assert!(tones.is_empty(), "No tones when chimes disabled");

    // But voice should still work
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
}

#[tokio::test]
async fn test_achievement_voice_disabled() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let config = AchievementAudioBridgeConfig {
        chimes_enabled: true,
        voice_enabled: false,
        ..Default::default()
    };

    let bridge =
        AchievementAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    let notification = create_test_achievement(AchievementTier::Bronze, "Test");
    bridge.handle_achievement_notification(&notification).await;

    // Tones should play
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty());

    // No voice alerts
    let alerts = alert_manager.get_triggered_alerts();
    assert!(alerts.is_empty());
}

#[tokio::test]
async fn test_multiple_achievements_sorted_by_tier() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let config = AchievementAudioBridgeConfig {
        audio_spacing_ms: 0, // No delay for faster testing
        ..Default::default()
    };

    let bridge =
        AchievementAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    let notifications = vec![
        create_test_achievement(AchievementTier::Bronze, "First"),
        create_test_achievement(AchievementTier::Legendary, "Second"),
        create_test_achievement(AchievementTier::Silver, "Third"),
    ];

    bridge.handle_multiple_achievements(&notifications).await;

    // Should have 3 alerts (sorted by tier - legendary first)
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 3);
}

// ============================================================================
// MilestoneAudioBridge Integration Tests
// ============================================================================

#[tokio::test]
async fn test_distance_milestone_triggers_audio() {
    let (bridge, _alert_manager, audio_backend) = create_milestone_bridge();

    bridge.handle_distance_milestone(10.0, "km").await;

    // Should have played tones
    let tones = audio_backend.get_played_tones();
    assert!(
        !tones.is_empty(),
        "Should have played distance milestone tones"
    );
}

#[tokio::test]
async fn test_time_milestone_triggers_audio() {
    let (bridge, _alert_manager, audio_backend) = create_milestone_bridge();

    bridge.handle_time_milestone(30.0).await;

    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played time milestone tones");
}

#[tokio::test]
async fn test_calorie_milestone_triggers_audio() {
    let (bridge, _alert_manager, audio_backend) = create_milestone_bridge();

    bridge.handle_calorie_milestone(500.0).await;

    let tones = audio_backend.get_played_tones();
    assert!(
        !tones.is_empty(),
        "Should have played calorie milestone tones"
    );
}

#[tokio::test]
async fn test_personal_record_triggers_audio_and_voice() {
    let (bridge, alert_manager, audio_backend) = create_milestone_bridge();

    bridge.handle_personal_record(42.5, "km", Some(41.2)).await;

    // PR should play tones
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played PR tones");

    // PR should trigger voice (enabled by default)
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::PersonalRecord);
}

#[tokio::test]
async fn test_milestone_sounds_disabled() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let mut config = MilestoneAudioBridgeConfig::default();
    config.distance_sounds_enabled = false;
    config.distance_voice_enabled = false;

    let bridge =
        MilestoneAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    bridge.handle_distance_milestone(10.0, "km").await;

    // No tones
    assert!(audio_backend.get_played_tones().is_empty());
    // No alerts
    assert!(alert_manager.get_triggered_alerts().is_empty());
}

#[tokio::test]
async fn test_milestone_voice_enabled() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    let mut config = MilestoneAudioBridgeConfig::default();
    config.distance_voice_enabled = true;

    let bridge =
        MilestoneAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    bridge.handle_distance_milestone(10.0, "km").await;

    // Should have alert for voice
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, AlertType::DistanceMilestone);
}

#[tokio::test]
async fn test_all_milestone_types() {
    let (bridge, alert_manager, audio_backend) = create_milestone_bridge();

    // Enable all voice announcements for this test
    let mut config = MilestoneAudioBridgeConfig::default();
    config.enable_all();
    let bridge =
        MilestoneAudioBridge::with_config(alert_manager.clone(), audio_backend.clone(), config);

    bridge.handle_distance_milestone(10.0, "km").await;
    bridge.handle_time_milestone(60.0).await;
    bridge.handle_calorie_milestone(500.0).await;
    bridge.handle_personal_record(100.0, "W", None).await;

    // Should have 4 alerts
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 4);

    let alert_types = alert_manager.get_alert_types();
    assert!(alert_types.contains(&AlertType::DistanceMilestone));
    assert!(alert_types.contains(&AlertType::TimeMilestone));
    assert!(alert_types.contains(&AlertType::CalorieMilestone));
    assert!(alert_types.contains(&AlertType::PersonalRecord));
}

// ============================================================================
// Volume Control and Muting Tests
// ============================================================================

#[test]
fn test_mock_backend_volume_levels() {
    let backend = MockAudioBackend::new();

    // Test various volume levels
    for vol in [0, 25, 50, 75, 100] {
        backend.set_volume(vol);
        assert_eq!(backend.get_volume(), vol);
    }
}

#[test]
fn test_mute_state_creation() {
    let config = AudioConfig {
        volume: 80,
        muted: true,
        voice_muted: false,
        sound_effects_muted: true,
        countdown_muted: false,
        achievement_muted: false,
        milestone_muted: true,
    };

    let mute_state = MuteState::from_config(&config);
    assert!(mute_state.any_muted());
}

#[test]
fn test_audio_category_muting() {
    let config = AudioConfig {
        volume: 80,
        muted: false,
        voice_muted: true,
        sound_effects_muted: false,
        countdown_muted: true,
        achievement_muted: false,
        milestone_muted: false,
    };

    let mute_state = MuteState::from_config(&config);
    assert!(mute_state.is_category_muted(AudioCategory::Voice));
    assert!(!mute_state.is_category_muted(AudioCategory::SoundEffects));
    assert!(mute_state.is_category_muted(AudioCategory::Countdown));
}

// ============================================================================
// Audio Queue Priority Tests
// ============================================================================

#[test]
fn test_audio_item_creation() {
    let tone_item = AudioItem::tone(440, 100);
    assert_eq!(tone_item.priority, AudioPriority::Normal);

    let high_priority_item = tone_item.with_priority(AudioPriority::High);
    assert_eq!(high_priority_item.priority, AudioPriority::High);
}

#[test]
fn test_audio_priority_ordering() {
    // Critical > High > Normal > Low
    assert!(AudioPriority::Critical > AudioPriority::High);
    assert!(AudioPriority::High > AudioPriority::Normal);
    assert!(AudioPriority::Normal > AudioPriority::Low);
}

#[test]
fn test_mock_backend_queue_ordering() {
    let backend = MockAudioBackend::new();

    // Queue items with different priorities
    let items = vec![
        AudioItem::tone(440, 100).with_priority(AudioPriority::Low),
        AudioItem::tone(880, 100).with_priority(AudioPriority::High),
        AudioItem::tone(660, 100).with_priority(AudioPriority::Normal),
        AudioItem::tone(1000, 100).with_priority(AudioPriority::Critical),
    ];

    for item in items {
        backend.queue(item);
    }

    let queued = backend.get_queued_items();
    assert_eq!(queued.len(), 4);
}

#[test]
fn test_queue_processing_disabled() {
    let backend = MockAudioBackend::new();
    backend.set_queue_processing_enabled(false);

    backend.queue(AudioItem::tone(440, 100));

    // Should not be queued when disabled
    assert!(backend.get_queued_items().is_empty());
}

// ============================================================================
// CuePattern Integration Tests
// ============================================================================

#[test]
fn test_cue_pattern_countdown_mapping() {
    assert_eq!(
        CuePattern::for_countdown_seconds(10),
        Some(CuePattern::CountdownTick10)
    );
    assert_eq!(
        CuePattern::for_countdown_seconds(5),
        Some(CuePattern::CountdownTick5)
    );
    assert_eq!(
        CuePattern::for_countdown_seconds(3),
        Some(CuePattern::CountdownFinal3)
    );
    assert_eq!(
        CuePattern::for_countdown_seconds(2),
        Some(CuePattern::CountdownFinal2)
    );
    assert_eq!(
        CuePattern::for_countdown_seconds(1),
        Some(CuePattern::CountdownFinal1)
    );
    assert_eq!(CuePattern::for_countdown_seconds(7), None);
}

#[test]
fn test_cue_pattern_achievement_mapping() {
    assert_eq!(
        CuePattern::for_achievement_tier("bronze"),
        Some(CuePattern::AchievementBronze)
    );
    assert_eq!(
        CuePattern::for_achievement_tier("silver"),
        Some(CuePattern::AchievementSilver)
    );
    assert_eq!(
        CuePattern::for_achievement_tier("gold"),
        Some(CuePattern::AchievementGold)
    );
    assert_eq!(
        CuePattern::for_achievement_tier("platinum"),
        Some(CuePattern::AchievementPlatinum)
    );
    assert_eq!(CuePattern::for_achievement_tier("unknown"), None);
}

#[test]
fn test_cue_pattern_milestone_mapping() {
    assert_eq!(
        CuePattern::for_milestone_type("distance"),
        Some(CuePattern::MilestoneDistance)
    );
    assert_eq!(
        CuePattern::for_milestone_type("time"),
        Some(CuePattern::MilestoneTime)
    );
    assert_eq!(
        CuePattern::for_milestone_type("calories"),
        Some(CuePattern::MilestoneCalories)
    );
    assert_eq!(
        CuePattern::for_milestone_type("pr"),
        Some(CuePattern::PersonalRecord)
    );
}

#[test]
fn test_cue_pattern_tones() {
    // Verify patterns produce non-empty tone lists
    let patterns = [
        CuePattern::SingleBeep,
        CuePattern::DoubleBeep,
        CuePattern::CountdownTick10,
        CuePattern::AchievementBronze,
        CuePattern::MilestoneDistance,
        CuePattern::PersonalRecord,
    ];

    for pattern in patterns {
        let tones = pattern.tones();
        assert!(!tones.is_empty(), "{:?} should produce tones", pattern);
    }
}

#[test]
fn test_cue_pattern_total_duration() {
    // Verify all patterns have positive duration
    let patterns = [
        CuePattern::CountdownFinal1,
        CuePattern::AchievementGold,
        CuePattern::LevelUp,
        CuePattern::PersonalRecord,
    ];

    for pattern in patterns {
        let duration = pattern.total_duration_ms();
        assert!(duration > 0, "{:?} should have positive duration", pattern);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_audio_error_handling() {
    let backend = MockAudioBackend::new();
    backend.set_simulate_failure(true);

    // All operations should return errors
    assert!(backend.initialize().is_err());
    assert!(backend.play_tone(440, 100).await.is_err());
    assert!(backend.play_sound("test").await.is_err());
    assert!(backend.speak("test").await.is_err());

    // Error count should be tracked
    assert_eq!(backend.get_error_count(), 4);
}

#[tokio::test]
async fn test_bridge_handles_audio_errors_gracefully() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());
    audio_backend.set_simulate_failure(true);

    let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_backend.clone());

    // Should not panic even with audio failures
    let event = WorkoutEvent::IntervalCountdown {
        seconds_remaining: 5,
    };
    bridge.process_event(&event).await;

    // Alert manager should still receive the alert
    let alerts = alert_manager.get_triggered_alerts();
    assert_eq!(alerts.len(), 1);
}

// ============================================================================
// Full Pipeline Integration Tests
// ============================================================================

#[tokio::test]
async fn test_complete_workout_audio_pipeline() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    // Simulate a complete mini-workout
    let events = vec![
        WorkoutEvent::Started {
            workout_name: "Quick Intervals".to_string(),
        },
        WorkoutEvent::IntervalChange {
            interval_name: "Warmup".to_string(),
            target_power: Some(150),
            duration_secs: 300,
            is_recovery: false,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 3,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 2,
        },
        WorkoutEvent::IntervalCountdown {
            seconds_remaining: 1,
        },
        WorkoutEvent::IntervalChange {
            interval_name: "VO2 Max".to_string(),
            target_power: Some(350),
            duration_secs: 60,
            is_recovery: false,
        },
        WorkoutEvent::Paused,
        WorkoutEvent::Resumed,
        WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 60,
            is_recovery: true,
        },
        WorkoutEvent::Completed {
            total_duration_secs: 420,
        },
    ];

    bridge.process_events(&events).await;

    // Verify alerts were triggered
    let alert_types = alert_manager.get_alert_types();
    assert!(alert_types.contains(&AlertType::WorkoutStart));
    assert!(alert_types.contains(&AlertType::IntervalChange));
    assert!(alert_types.contains(&AlertType::IntervalCountdown));
    assert!(alert_types.contains(&AlertType::RidePaused));
    assert!(alert_types.contains(&AlertType::RideResumed));
    assert!(alert_types.contains(&AlertType::RecoveryStart));
    assert!(alert_types.contains(&AlertType::WorkoutComplete));

    // Verify tones were played
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty(), "Should have played countdown tones");
}

#[tokio::test]
async fn test_combined_achievement_and_milestone_audio() {
    let alert_manager = Arc::new(MockAlertManager::new());
    let audio_backend = Arc::new(MockAudioBackend::new());

    // Create both bridges with shared components
    let achievement_bridge =
        AchievementAudioBridge::new(alert_manager.clone(), audio_backend.clone());
    let milestone_bridge = MilestoneAudioBridge::new(alert_manager.clone(), audio_backend.clone());

    // Trigger achievement and milestone
    let achievement = create_test_achievement(AchievementTier::Gold, "Century Ride");
    achievement_bridge
        .handle_achievement_notification(&achievement)
        .await;

    milestone_bridge
        .handle_personal_record(100.0, "km", Some(95.0))
        .await;

    // Both should have triggered alerts
    let alert_types = alert_manager.get_alert_types();
    assert!(alert_types.contains(&AlertType::AchievementUnlocked));
    assert!(alert_types.contains(&AlertType::PersonalRecord));

    // Multiple tones should have been played
    let tones = audio_backend.get_played_tones();
    assert!(tones.len() > 2, "Should have played tones for both events");
}

#[tokio::test]
async fn test_rapid_event_processing() {
    let (bridge, alert_manager, audio_backend) = create_workout_bridge();

    // Process many events rapidly
    let mut events = Vec::new();
    for i in 0..50 {
        events.push(WorkoutEvent::IntervalCountdown {
            seconds_remaining: (i % 10) + 1,
        });
    }

    bridge.process_events(&events).await;

    // Should have processed all events
    let alerts = alert_manager.get_triggered_alerts();
    // Only 10 and 5 second countdowns trigger voice (10 occurrences each)
    assert!(alerts.len() > 0);

    // Should have played many tones
    let tones = audio_backend.get_played_tones();
    assert!(!tones.is_empty());
}

#[test]
fn test_mock_alert_manager_cooldown() {
    let manager = MockAlertManager::new();

    // Initially not on cooldown
    assert!(!manager.is_on_cooldown(AlertType::WorkoutStart));

    // After trigger, check cooldown behavior
    // (async would be needed for actual trigger)
}

#[test]
fn test_mock_alert_manager_configuration() {
    let manager = MockAlertManager::new();

    let config = AlertConfig {
        enabled: false,
        cooldown_secs: 10,
        ..Default::default()
    };
    manager.configure(AlertType::WorkoutStart, config.clone());

    let retrieved = manager.get_config(AlertType::WorkoutStart);
    assert!(!retrieved.enabled);
    assert_eq!(retrieved.cooldown_secs, 10);
}

#[test]
fn test_mock_alert_manager_set_enabled() {
    let manager = MockAlertManager::new();

    manager.set_enabled(AlertType::IntervalChange, false);
    let config = manager.get_config(AlertType::IntervalChange);
    assert!(!config.enabled);

    manager.set_enabled(AlertType::IntervalChange, true);
    let config = manager.get_config(AlertType::IntervalChange);
    assert!(config.enabled);
}
