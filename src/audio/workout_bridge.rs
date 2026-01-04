//! Workout Audio Bridge
//!
//! Bridges workout events from the WorkoutEngine to the audio alert system.
//! Subscribes to workout events and triggers appropriate voice announcements
//! and countdown sounds.
//!
//! # Countdown Sound System
//!
//! The bridge implements a differentiated countdown sound system:
//! - **10 seconds**: Voice announcement ("10 seconds") + gentle tick tone
//! - **5 seconds**: Voice announcement ("5 seconds") + attention tick tone
//! - **3, 2, 1 seconds**: Tone-only with escalating urgency (no voice to avoid overlap)
//!
//! This approach ensures countdown cues are clear and don't overlap with each other.
//!
//! # Timing Safeguards
//!
//! Countdown sounds have a maximum age threshold (default 500ms). If a countdown
//! tone cannot be played within this window, it is skipped to maintain synchronization
//! with the actual workout timing. This prevents audio lag from accumulating.

use crate::audio::alerts::{AlertContext, AlertManager, AlertType};
use crate::audio::engine::AudioEngine;
use crate::audio::tones::CuePattern;
use crate::audio::AudioItem;
use crate::workouts::types::WorkoutEvent;
use std::sync::Arc;
use std::time::Instant;

/// Configuration for the workout audio bridge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkoutAudioBridgeConfig {
    /// Enable interval change announcements
    pub announce_interval_changes: bool,
    /// Enable countdown announcements
    pub announce_countdowns: bool,
    /// Enable workout start/complete announcements
    pub announce_workout_lifecycle: bool,
    /// Enable trainer disconnect/reconnect announcements
    pub announce_trainer_status: bool,
    /// Enable recovery interval special announcements
    pub announce_recovery_intervals: bool,
    /// Enable motivational messages during intervals
    pub announce_motivational_messages: bool,
    /// Enable countdown sound effects (tones)
    pub countdown_sounds_enabled: bool,
    /// Enable voice announcements for countdown (10s, 5s)
    /// When false, only plays tones for all countdown values
    pub countdown_voice_enabled: bool,
    /// Which countdown seconds to announce (voice and/or tones).
    /// Default: [10, 5, 3, 2, 1]
    /// This allows customization of which countdown values trigger audio feedback.
    #[serde(default = "default_countdown_thresholds")]
    pub countdown_thresholds: Vec<u32>,
    /// Thresholds that receive voice announcements (subset of countdown_thresholds).
    /// Default: [10, 5]
    /// Other thresholds in countdown_thresholds will receive tone-only feedback.
    #[serde(default = "default_countdown_voice_thresholds")]
    pub countdown_voice_thresholds: Vec<u32>,
    /// Maximum age in milliseconds for countdown sounds to be played.
    /// If a countdown tone cannot be played within this window from when
    /// the countdown event was received, it will be skipped to maintain
    /// synchronization with workout timing. Default: 500ms
    #[serde(default = "default_countdown_max_age_ms")]
    pub countdown_max_age_ms: u64,
    /// Whether to use the queue-based approach for countdown tones.
    /// When true, countdown tones are queued through the audio engine
    /// with proper expiration handling. When false, tones are played
    /// directly (legacy behavior). Default: true
    #[serde(default = "default_use_queued_countdown")]
    pub use_queued_countdown: bool,
}

fn default_countdown_thresholds() -> Vec<u32> {
    vec![10, 5, 3, 2, 1]
}

fn default_countdown_voice_thresholds() -> Vec<u32> {
    vec![10, 5]
}

fn default_countdown_max_age_ms() -> u64 {
    500 // Countdown sounds must play within 500ms to stay synchronized
}

fn default_use_queued_countdown() -> bool {
    true // Use queue-based approach for better timing management
}

impl Default for WorkoutAudioBridgeConfig {
    fn default() -> Self {
        Self {
            announce_interval_changes: true,
            announce_countdowns: true,
            announce_workout_lifecycle: true,
            announce_trainer_status: true,
            announce_recovery_intervals: true,
            announce_motivational_messages: false, // Optional by default to avoid annoyance
            countdown_sounds_enabled: true,
            countdown_voice_enabled: true,
            countdown_thresholds: default_countdown_thresholds(),
            countdown_voice_thresholds: default_countdown_voice_thresholds(),
            countdown_max_age_ms: default_countdown_max_age_ms(),
            use_queued_countdown: default_use_queued_countdown(),
        }
    }
}

/// Bridges workout events to audio alerts.
///
/// This component processes events from the WorkoutEngine and triggers
/// appropriate audio alerts via the AlertManager and countdown sounds
/// via the AudioEngine.
///
/// # Usage
///
/// ```ignore
/// let bridge = WorkoutAudioBridge::new(alert_manager, audio_engine);
///
/// // In your main loop:
/// let events = workout_engine.take_events();
/// bridge.process_events(&events).await;
/// ```
///
/// # Countdown Sound Behavior
///
/// The bridge implements intelligent countdown sound handling:
/// - **10s, 5s**: Voice announcement + countdown tick tone
/// - **3s, 2s, 1s**: Tone-only (final countdown patterns with escalating urgency)
///
/// This prevents voice announcements from overlapping during the final countdown.
pub struct WorkoutAudioBridge<A: AlertManager, E: AudioEngine> {
    /// The alert manager for triggering audio alerts
    alert_manager: Arc<A>,
    /// The audio engine for playing countdown tones
    audio_engine: Arc<E>,
    /// Configuration for which events to announce
    config: WorkoutAudioBridgeConfig,
}

impl<A: AlertManager, E: AudioEngine> WorkoutAudioBridge<A, E> {
    /// Create a new workout audio bridge with the given alert manager and audio engine.
    pub fn new(alert_manager: Arc<A>, audio_engine: Arc<E>) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config: WorkoutAudioBridgeConfig::default(),
        }
    }

    /// Create a new workout audio bridge with custom configuration.
    pub fn with_config(
        alert_manager: Arc<A>,
        audio_engine: Arc<E>,
        config: WorkoutAudioBridgeConfig,
    ) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config,
        }
    }

    /// Update the bridge configuration.
    pub fn set_config(&mut self, config: WorkoutAudioBridgeConfig) {
        self.config = config;
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &WorkoutAudioBridgeConfig {
        &self.config
    }

    /// Process a batch of workout events from the engine.
    ///
    /// Events are processed in order. Each event is mapped to an appropriate
    /// AlertType and AlertContext, then triggered via the AlertManager.
    pub async fn process_events(&self, events: &[WorkoutEvent]) {
        for event in events {
            self.process_event(event).await;
        }
    }

    /// Process a single workout event.
    pub async fn process_event(&self, event: &WorkoutEvent) {
        match event {
            WorkoutEvent::Started { workout_name } => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout start: {}", workout_name);
                    self.alert_manager
                        .trigger(AlertType::WorkoutStart, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::IntervalChange {
                interval_name,
                target_power,
                duration_secs,
                is_recovery,
            } => {
                // Handle recovery intervals specially if enabled
                if *is_recovery && self.config.announce_recovery_intervals {
                    tracing::debug!("Announcing recovery interval: {}", interval_name);
                    self.alert_manager
                        .trigger(AlertType::RecoveryStart, AlertContext::simple())
                        .await;

                    // Add motivational recovery message if enabled
                    if self.config.announce_motivational_messages {
                        tracing::debug!("Announcing motivational recovery message");
                        self.alert_manager
                            .trigger(AlertType::MotivationalRecovery, AlertContext::simple())
                            .await;
                    }
                } else if self.config.announce_interval_changes {
                    tracing::debug!(
                        "Announcing interval change: {} ({} watts, {} secs)",
                        interval_name,
                        target_power.unwrap_or(0),
                        duration_secs
                    );
                    self.alert_manager
                        .trigger(
                            AlertType::IntervalChange,
                            AlertContext::interval_change(
                                interval_name.clone(),
                                *target_power,
                                *duration_secs,
                            ),
                        )
                        .await;

                    // Add motivational high-intensity message if enabled (for non-recovery intervals)
                    if self.config.announce_motivational_messages {
                        tracing::debug!("Announcing motivational high-intensity message");
                        self.alert_manager
                            .trigger(AlertType::MotivationalHighIntensity, AlertContext::simple())
                            .await;
                    }
                }
            }

            WorkoutEvent::IntervalCountdown { seconds_remaining } => {
                if self.config.announce_countdowns {
                    self.handle_countdown(*seconds_remaining).await;
                }
            }

            WorkoutEvent::Paused => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout paused");
                    self.alert_manager
                        .trigger(AlertType::RidePaused, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Resumed => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout resumed");
                    self.alert_manager
                        .trigger(AlertType::RideResumed, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Completed {
                total_duration_secs,
            } => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!(
                        "Announcing workout complete (duration: {} secs)",
                        total_duration_secs
                    );
                    self.alert_manager
                        .trigger(AlertType::WorkoutComplete, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::Stopped => {
                if self.config.announce_workout_lifecycle {
                    tracing::debug!("Announcing workout stopped");
                    // Use RidePaused as a reasonable fallback since there's no explicit "stopped" alert type
                    self.alert_manager
                        .trigger(AlertType::RidePaused, AlertContext::simple())
                        .await;
                }
            }

            WorkoutEvent::TrainerDisconnected => {
                if self.config.announce_trainer_status {
                    tracing::debug!("Announcing trainer disconnected");
                    self.alert_manager
                        .trigger(
                            AlertType::SensorDisconnected,
                            AlertContext::sensor("Trainer", "smart_trainer"),
                        )
                        .await;
                }
            }

            WorkoutEvent::TrainerReconnected => {
                if self.config.announce_trainer_status {
                    tracing::debug!("Announcing trainer reconnected");
                    self.alert_manager
                        .trigger(
                            AlertType::SensorConnected,
                            AlertContext::sensor("Trainer", "smart_trainer"),
                        )
                        .await;
                }
            }
        }
    }

    /// Handle countdown event with differentiated audio behavior.
    ///
    /// # Countdown Strategy
    ///
    /// Uses configurable `countdown_thresholds` to determine which seconds trigger audio.
    /// Uses `countdown_voice_thresholds` to determine which of those get voice announcements.
    ///
    /// Default behavior:
    /// - **10, 5 seconds**: Voice announcement + countdown tick tone
    /// - **3, 2, 1 seconds**: Tone-only with escalating urgency patterns
    ///
    /// # Timing Safeguards
    ///
    /// Countdown sounds are queued with a short expiration time (default 500ms).
    /// If the audio queue is backed up and a countdown tone cannot be played
    /// within its expiration window, it will be dropped to maintain synchronization
    /// with the actual workout timing.
    ///
    /// This approach prevents voice announcements from overlapping during the
    /// final rapid countdown while still providing clear audio feedback.
    async fn handle_countdown(&self, seconds_remaining: u32) {
        let event_time = Instant::now();
        tracing::debug!("Handling countdown: {} seconds", seconds_remaining);

        // Check if this second is in our configured thresholds
        let is_countdown_threshold = self.config.countdown_thresholds.contains(&seconds_remaining);
        if !is_countdown_threshold {
            tracing::trace!(
                "Countdown {} seconds - no audio (not a configured threshold)",
                seconds_remaining
            );
            return;
        }

        // Check if this second should get voice announcement
        let is_voice_threshold = self.config.countdown_voice_thresholds.contains(&seconds_remaining);

        // Get the appropriate countdown pattern for this second
        let pattern = CuePattern::for_countdown_seconds(seconds_remaining);

        // Play countdown tone if sounds are enabled
        if self.config.countdown_sounds_enabled {
            if let Some(cue_pattern) = pattern {
                if self.config.use_queued_countdown {
                    // Queue-based approach: better timing management via engine
                    self.queue_countdown_tones(cue_pattern, event_time).await;
                } else {
                    // Legacy direct approach: play tones immediately
                    self.play_countdown_tone_direct(cue_pattern, event_time).await;
                }
            }
        }

        // Voice announcement if enabled and this is a voice threshold
        if self.config.countdown_voice_enabled && is_voice_threshold {
            self.alert_manager
                .trigger(
                    AlertType::IntervalCountdown,
                    AlertContext::countdown(seconds_remaining),
                )
                .await;
        }
    }

    /// Queue countdown tones through the audio engine for proper timing management.
    ///
    /// This method queues each tone in the pattern as an AudioItem with a short
    /// expiration time. The audio engine will drop expired items, keeping
    /// countdown sounds synchronized with workout timing.
    async fn queue_countdown_tones(&self, pattern: CuePattern, event_time: Instant) {
        let tones = pattern.tones();
        let max_age_ms = self.config.countdown_max_age_ms;

        for tone in tones {
            // Check if we've exceeded the max age before processing more tones
            let elapsed = event_time.elapsed().as_millis() as u64;
            if elapsed > max_age_ms {
                tracing::debug!(
                    "Countdown pattern expired after {}ms, skipping remaining tones",
                    elapsed
                );
                break;
            }

            if tone.is_pause() {
                // Sleep for pause duration
                tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
            } else {
                // Queue the tone with countdown category and short expiration
                // Adjust remaining expiration time based on how much time has passed
                let remaining_ms = max_age_ms.saturating_sub(elapsed);
                if remaining_ms > 0 {
                    let item = AudioItem::countdown_tone_with_timing(
                        tone.frequency_hz as u32,
                        tone.duration_ms as u32,
                        remaining_ms,
                    );
                    self.audio_engine.queue(item);

                    // Wait for the tone duration before queueing next tone
                    // This maintains proper timing between pattern notes
                    tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
                }
            }
        }
    }

    /// Play countdown tones directly (legacy approach) with timing check.
    ///
    /// This method plays tones immediately via play_tone but adds a timing
    /// check to ensure we don't play stale countdown sounds.
    async fn play_countdown_tone_direct(&self, pattern: CuePattern, event_time: Instant) {
        let tones = pattern.tones();
        let max_age_ms = self.config.countdown_max_age_ms;

        for tone in tones {
            // Check if we've exceeded the max age before playing
            let elapsed = event_time.elapsed().as_millis() as u64;
            if elapsed > max_age_ms {
                tracing::debug!(
                    "Countdown pattern expired after {}ms (direct mode), skipping remaining tones",
                    elapsed
                );
                break;
            }

            if tone.is_pause() {
                // Sleep for pause duration
                tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
            } else {
                // Play the tone directly
                if let Err(e) = self
                    .audio_engine
                    .play_tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                    .await
                {
                    tracing::warn!("Failed to play countdown tone: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::alerts::{AlertConfig, AlertData};
    use crate::audio::{AudioError, AudioEvent, AudioItem};
    use std::sync::Mutex;
    use tokio::sync::broadcast;

    /// Mock alert manager for testing
    struct MockAlertManager {
        triggered_alerts: Mutex<Vec<(AlertType, AlertContext)>>,
        configs: Mutex<std::collections::HashMap<AlertType, AlertConfig>>,
    }

    impl MockAlertManager {
        fn new() -> Self {
            Self {
                triggered_alerts: Mutex::new(Vec::new()),
                configs: Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn get_triggered_alerts(&self) -> Vec<(AlertType, AlertContext)> {
            self.triggered_alerts.lock().unwrap().clone()
        }
    }

    impl AlertManager for MockAlertManager {
        async fn trigger(&self, alert_type: AlertType, context: AlertContext) {
            self.triggered_alerts
                .lock()
                .unwrap()
                .push((alert_type, context));
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

        fn is_on_cooldown(&self, _alert_type: AlertType) -> bool {
            false
        }
    }

    /// Mock audio engine for testing
    struct MockAudioEngine {
        played_tones: Mutex<Vec<(u32, u32)>>, // (frequency_hz, duration_ms)
        played_sounds: Mutex<Vec<String>>,
        spoken_texts: Mutex<Vec<String>>,
        queued_items: Mutex<Vec<AudioItem>>,
        volume: Mutex<u8>,
        event_tx: broadcast::Sender<AudioEvent>,
    }

    impl MockAudioEngine {
        fn new() -> Self {
            let (event_tx, _) = broadcast::channel(100);
            Self {
                played_tones: Mutex::new(Vec::new()),
                played_sounds: Mutex::new(Vec::new()),
                spoken_texts: Mutex::new(Vec::new()),
                queued_items: Mutex::new(Vec::new()),
                volume: Mutex::new(80),
                event_tx,
            }
        }

        fn get_played_tones(&self) -> Vec<(u32, u32)> {
            self.played_tones.lock().unwrap().clone()
        }

        fn get_queued_items(&self) -> Vec<AudioItem> {
            self.queued_items.lock().unwrap().clone()
        }
    }

    impl AudioEngine for MockAudioEngine {
        fn initialize(&self) -> Result<(), AudioError> {
            Ok(())
        }

        async fn play_sound(&self, name: &str) -> Result<(), AudioError> {
            self.played_sounds.lock().unwrap().push(name.to_string());
            Ok(())
        }

        async fn speak(&self, text: &str) -> Result<(), AudioError> {
            self.spoken_texts.lock().unwrap().push(text.to_string());
            Ok(())
        }

        async fn play_tone(&self, frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
            self.played_tones
                .lock()
                .unwrap()
                .push((frequency_hz, duration_ms));
            Ok(())
        }

        fn set_volume(&self, volume: u8) {
            *self.volume.lock().unwrap() = volume;
        }

        fn get_volume(&self) -> u8 {
            *self.volume.lock().unwrap()
        }

        fn queue(&self, item: AudioItem) {
            self.queued_items.lock().unwrap().push(item);
        }

        fn is_playing(&self) -> bool {
            false
        }

        fn stop(&self) {
            // No-op for mock
        }

        fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
            self.event_tx.subscribe()
        }
    }

    /// Create a config that uses direct tones (not queued) for backward compatible tests
    fn direct_countdown_config() -> WorkoutAudioBridgeConfig {
        WorkoutAudioBridgeConfig {
            use_queued_countdown: false,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_workout_start_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine);

        let events = vec![WorkoutEvent::Started {
            workout_name: "Test Workout".to_string(),
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::WorkoutStart);
    }

    #[tokio::test]
    async fn test_interval_change_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine);

        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Sweet Spot".to_string(),
            target_power: Some(260),
            duration_secs: 300,
            is_recovery: false,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::IntervalChange);

        // Verify context data
        match &triggered[0].1.data {
            AlertData::IntervalChange {
                new_interval_name,
                target_power,
                duration_secs,
            } => {
                assert_eq!(new_interval_name, "Sweet Spot");
                assert_eq!(*target_power, Some(260));
                assert_eq!(*duration_secs, 300);
            }
            _ => panic!("Expected IntervalChange data"),
        }
    }

    #[tokio::test]
    async fn test_recovery_interval_event() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine);

        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 120,
            is_recovery: true,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].0, AlertType::RecoveryStart);
    }

    #[tokio::test]
    async fn test_countdown_event_voice_and_tones() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Use direct countdown mode for backward-compatible test
        let config = direct_countdown_config();
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        // Test 10 and 5 seconds - should trigger both voice and tone
        let events = vec![
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 10,
            },
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 5,
            },
        ];
        bridge.process_events(&events).await;

        // Voice announcements should be triggered for 10s and 5s
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2, "Should have 2 voice announcements for 10s and 5s");

        for (i, (alert_type, context)) in triggered.iter().enumerate() {
            assert_eq!(*alert_type, AlertType::IntervalCountdown);
            match &context.data {
                AlertData::Countdown { seconds_remaining } => {
                    let expected = [10, 5][i];
                    assert_eq!(*seconds_remaining, expected);
                }
                _ => panic!("Expected Countdown data"),
            }
        }

        // Tones should also be played for 10s and 5s (direct mode)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played countdown tones for 10s and 5s");
    }

    #[tokio::test]
    async fn test_countdown_event_final_tones_only() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Use direct countdown mode for backward-compatible test
        let config = direct_countdown_config();
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        // Test 3, 2, 1 seconds - should trigger tones only (no voice)
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

        // No voice announcements for final countdown (3, 2, 1)
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 0, "Final countdown (3, 2, 1) should not trigger voice announcements");

        // But tones should be played (direct mode)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played countdown tones for 3, 2, 1 seconds");
    }

    #[tokio::test]
    async fn test_countdown_event_all_seconds() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Use direct countdown mode for backward-compatible test
        let config = direct_countdown_config();
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        // Test full countdown sequence
        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 3 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 2 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 1 },
        ];
        bridge.process_events(&events).await;

        // Only 10 and 5 should trigger voice announcements
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2, "Only 10s and 5s should trigger voice");

        // All 5 should trigger tones (direct mode)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played tones for all countdown seconds");
    }

    #[tokio::test]
    async fn test_countdown_event_queued_mode() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Default config uses queued countdown
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        // Test 10 seconds countdown - should queue tones instead of playing directly
        let events = vec![WorkoutEvent::IntervalCountdown {
            seconds_remaining: 10,
        }];
        bridge.process_events(&events).await;

        // In queued mode, tones should be queued not played directly
        let direct_tones = audio_engine.get_played_tones();
        assert!(direct_tones.is_empty(), "In queued mode, tones should not be played directly");

        // Items should be queued
        let queued = audio_engine.get_queued_items();
        assert!(!queued.is_empty(), "Should have queued countdown tones");

        // All queued items should have countdown category
        for item in &queued {
            assert!(item.is_time_critical(), "Countdown items should be time-critical");
        }
    }

    #[tokio::test]
    async fn test_countdown_timing_safeguards() {
        // Test that countdown items have proper expiration settings
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            countdown_max_age_ms: 300, // Custom 300ms max age
            use_queued_countdown: true,
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let events = vec![WorkoutEvent::IntervalCountdown {
            seconds_remaining: 5,
        }];
        bridge.process_events(&events).await;

        // Check that queued items have appropriate expiration
        let queued = audio_engine.get_queued_items();
        assert!(!queued.is_empty(), "Should have queued items");

        for item in &queued {
            // Max queue time should be <= our configured max age
            assert!(
                item.max_queue_time.as_millis() <= 300,
                "Item max queue time ({:?}) should be <= 300ms",
                item.max_queue_time
            );
        }
    }

    #[tokio::test]
    async fn test_workout_lifecycle_events() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine);

        let events = vec![
            WorkoutEvent::Paused,
            WorkoutEvent::Resumed,
            WorkoutEvent::Completed {
                total_duration_secs: 3600,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 3);
        assert_eq!(triggered[0].0, AlertType::RidePaused);
        assert_eq!(triggered[1].0, AlertType::RideResumed);
        assert_eq!(triggered[2].0, AlertType::WorkoutComplete);
    }

    #[tokio::test]
    async fn test_trainer_disconnect_events() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = WorkoutAudioBridge::new(alert_manager.clone(), audio_engine);

        let events = vec![
            WorkoutEvent::TrainerDisconnected,
            WorkoutEvent::TrainerReconnected,
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2);
        assert_eq!(triggered[0].0, AlertType::SensorDisconnected);
        assert_eq!(triggered[1].0, AlertType::SensorConnected);

        // Verify sensor context
        match &triggered[0].1.data {
            AlertData::Sensor {
                sensor_name,
                sensor_type,
            } => {
                assert_eq!(sensor_name, "Trainer");
                assert_eq!(sensor_type, "smart_trainer");
            }
            _ => panic!("Expected Sensor data"),
        }
    }

    #[tokio::test]
    async fn test_config_disables_announcements() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_interval_changes: false,
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_recovery_intervals: false,
            announce_motivational_messages: false,
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

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
            WorkoutEvent::Paused,
            WorkoutEvent::TrainerDisconnected,
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 0, "No alerts should be triggered when all announcements are disabled");

        // No tones should be played either when countdowns are disabled
        let tones = audio_engine.get_played_tones();
        let queued = audio_engine.get_queued_items();
        assert!(tones.is_empty() && queued.is_empty(), "No countdown tones when announce_countdowns is false");
    }

    #[tokio::test]
    async fn test_selective_config() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_interval_changes: true,
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_recovery_intervals: false,
            announce_motivational_messages: false,
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine, config);

        let events = vec![
            WorkoutEvent::Started {
                workout_name: "Test".to_string(),
            },
            WorkoutEvent::IntervalChange {
                interval_name: "Interval 1".to_string(),
                target_power: Some(200),
                duration_secs: 60,
                is_recovery: false,
            },
            WorkoutEvent::IntervalCountdown {
                seconds_remaining: 5,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1, "Only interval change should be triggered");
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
    }

    #[tokio::test]
    async fn test_recovery_interval_with_config_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_recovery_intervals: false, // Disable recovery-specific announcements
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine, config);

        // Recovery interval should fall back to regular interval change
        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 120,
            is_recovery: true,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1);
        // Should be a regular IntervalChange, not RecoveryStart
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
    }

    #[test]
    fn test_default_config() {
        let config = WorkoutAudioBridgeConfig::default();
        assert!(config.announce_interval_changes);
        assert!(config.announce_countdowns);
        assert!(config.announce_workout_lifecycle);
        assert!(config.announce_trainer_status);
        assert!(config.announce_recovery_intervals);
        assert!(!config.announce_motivational_messages); // Disabled by default
        assert!(config.countdown_sounds_enabled);
        assert!(config.countdown_voice_enabled);
        assert_eq!(config.countdown_thresholds, vec![10, 5, 3, 2, 1]);
        assert_eq!(config.countdown_voice_thresholds, vec![10, 5]);
        // New timing safeguard settings
        assert_eq!(config.countdown_max_age_ms, 500); // 500ms default
        assert!(config.use_queued_countdown); // Queued mode by default
    }

    #[tokio::test]
    async fn test_motivational_messages_high_intensity() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_recovery_intervals: false,
            announce_motivational_messages: true, // Enable motivational messages
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine, config);

        // Non-recovery interval should trigger IntervalChange + MotivationalHighIntensity
        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Sweet Spot".to_string(),
            target_power: Some(260),
            duration_secs: 300,
            is_recovery: false,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2);
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
        assert_eq!(triggered[1].0, AlertType::MotivationalHighIntensity);
    }

    #[tokio::test]
    async fn test_motivational_messages_recovery() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            announce_motivational_messages: true, // Enable motivational messages
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine, config);

        // Recovery interval should trigger RecoveryStart + MotivationalRecovery
        let events = vec![WorkoutEvent::IntervalChange {
            interval_name: "Recovery".to_string(),
            target_power: Some(100),
            duration_secs: 120,
            is_recovery: true,
        }];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2);
        assert_eq!(triggered[0].0, AlertType::RecoveryStart);
        assert_eq!(triggered[1].0, AlertType::MotivationalRecovery);
    }

    #[tokio::test]
    async fn test_motivational_messages_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            announce_countdowns: false,
            announce_workout_lifecycle: false,
            announce_trainer_status: false,
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine, config);

        // Should only trigger the base alerts without motivational messages
        let events = vec![
            WorkoutEvent::IntervalChange {
                interval_name: "Sweet Spot".to_string(),
                target_power: Some(260),
                duration_secs: 300,
                is_recovery: false,
            },
            WorkoutEvent::IntervalChange {
                interval_name: "Recovery".to_string(),
                target_power: Some(100),
                duration_secs: 120,
                is_recovery: true,
            },
        ];
        bridge.process_events(&events).await;

        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2);
        assert_eq!(triggered[0].0, AlertType::IntervalChange);
        assert_eq!(triggered[1].0, AlertType::RecoveryStart);
        // No motivational messages
        assert!(
            !triggered.iter().any(|(t, _)| *t == AlertType::MotivationalHighIntensity
                || *t == AlertType::MotivationalRecovery),
            "No motivational messages should be triggered when disabled"
        );
    }

    #[tokio::test]
    async fn test_countdown_sounds_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            countdown_sounds_enabled: false, // Disable countdown sounds
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 3 },
        ];
        bridge.process_events(&events).await;

        // Voice should still be triggered for 10 and 5
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 2, "Voice should be triggered for 10s and 5s");

        // But no tones should be played or queued
        let tones = audio_engine.get_played_tones();
        let queued = audio_engine.get_queued_items();
        assert!(tones.is_empty() && queued.is_empty(), "No tones when countdown_sounds_enabled is false");
    }

    #[tokio::test]
    async fn test_countdown_voice_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = WorkoutAudioBridgeConfig {
            countdown_voice_enabled: false, // Disable countdown voice
            use_queued_countdown: false,    // Use direct mode so we can check played_tones
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 3 },
        ];
        bridge.process_events(&events).await;

        // No voice should be triggered
        let triggered = alert_manager.get_triggered_alerts();
        assert!(triggered.is_empty(), "No voice when countdown_voice_enabled is false");

        // But tones should still be played (direct mode)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Tones should still play when only voice is disabled");
    }

    #[tokio::test]
    async fn test_custom_countdown_thresholds() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Custom thresholds: only 5 and 3 second countdown
        let config = WorkoutAudioBridgeConfig {
            countdown_thresholds: vec![5, 3], // Only 5 and 3 seconds
            countdown_voice_thresholds: vec![5], // Only 5 seconds gets voice
            use_queued_countdown: false, // Use direct mode for checking played_tones
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 }, // Not in thresholds
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },  // In thresholds, voice
            WorkoutEvent::IntervalCountdown { seconds_remaining: 3 },  // In thresholds, no voice
            WorkoutEvent::IntervalCountdown { seconds_remaining: 1 },  // Not in thresholds
        ];
        bridge.process_events(&events).await;

        // Only 5 should trigger voice (it's in both thresholds and voice_thresholds)
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1, "Only 5 seconds should trigger voice");
        match &triggered[0].1.data {
            AlertData::Countdown { seconds_remaining } => {
                assert_eq!(*seconds_remaining, 5);
            }
            _ => panic!("Expected Countdown data"),
        }

        // Tones should be played for 5 and 3 (both in thresholds)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played tones for 5 and 3 seconds");
    }

    #[tokio::test]
    async fn test_countdown_thresholds_out_of_range() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Custom thresholds: only 3, 2, 1
        let config = WorkoutAudioBridgeConfig {
            countdown_thresholds: vec![3, 2, 1],
            countdown_voice_thresholds: vec![3],
            use_queued_countdown: false, // Use direct mode for checking played_tones
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        // Send countdown for 10 and 5 which are NOT in thresholds
        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },
        ];
        bridge.process_events(&events).await;

        // No alerts should be triggered (10 and 5 are not in thresholds)
        let triggered = alert_manager.get_triggered_alerts();
        assert!(triggered.is_empty(), "10 and 5 are not in thresholds, should have no alerts");

        // No tones either
        let tones = audio_engine.get_played_tones();
        let queued = audio_engine.get_queued_items();
        assert!(tones.is_empty() && queued.is_empty(), "No tones for out-of-threshold seconds");
    }

    #[tokio::test]
    async fn test_countdown_voice_thresholds_subset() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        // Voice thresholds is a subset of countdown thresholds
        let config = WorkoutAudioBridgeConfig {
            countdown_voice_thresholds: vec![10], // Only 10 gets voice
            use_queued_countdown: false, // Use direct mode for checking played_tones
            ..Default::default()
        };
        let bridge = WorkoutAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let events = vec![
            WorkoutEvent::IntervalCountdown { seconds_remaining: 10 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 5 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 3 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 2 },
            WorkoutEvent::IntervalCountdown { seconds_remaining: 1 },
        ];
        bridge.process_events(&events).await;

        // Only 10 should trigger voice
        let triggered = alert_manager.get_triggered_alerts();
        assert_eq!(triggered.len(), 1, "Only 10 seconds should trigger voice");
        match &triggered[0].1.data {
            AlertData::Countdown { seconds_remaining } => {
                assert_eq!(*seconds_remaining, 10);
            }
            _ => panic!("Expected Countdown data"),
        }

        // All 5 should have triggered tones
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "All 5 seconds should have played tones");
    }

    #[test]
    fn test_serde_serialization() {
        let config = WorkoutAudioBridgeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("countdown_thresholds"));
        assert!(json.contains("countdown_voice_thresholds"));
        assert!(json.contains("countdown_max_age_ms"));
        assert!(json.contains("use_queued_countdown"));

        let deserialized: WorkoutAudioBridgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.countdown_thresholds, vec![10, 5, 3, 2, 1]);
        assert_eq!(deserialized.countdown_voice_thresholds, vec![10, 5]);
        assert_eq!(deserialized.countdown_max_age_ms, 500);
        assert!(deserialized.use_queued_countdown);
    }

    #[test]
    fn test_serde_deserialization_with_defaults() {
        // Test deserializing without the new fields (backward compatibility)
        let json = r#"{
            "announce_interval_changes": true,
            "announce_countdowns": true,
            "announce_workout_lifecycle": true,
            "announce_trainer_status": true,
            "announce_recovery_intervals": true,
            "announce_motivational_messages": false,
            "countdown_sounds_enabled": true,
            "countdown_voice_enabled": true
        }"#;

        let config: WorkoutAudioBridgeConfig = serde_json::from_str(json).unwrap();
        // Should use defaults for missing fields
        assert_eq!(config.countdown_thresholds, vec![10, 5, 3, 2, 1]);
        assert_eq!(config.countdown_voice_thresholds, vec![10, 5]);
        // New timing fields should also get defaults
        assert_eq!(config.countdown_max_age_ms, 500);
        assert!(config.use_queued_countdown);
    }
}
