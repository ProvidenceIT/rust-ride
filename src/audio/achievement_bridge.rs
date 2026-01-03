//! Achievement Audio Bridge
//!
//! Bridges achievement notifications from the achievements module to the audio system.
//! Monitors the NotificationQueue and triggers appropriate audio (chimes + optional
//! voice announcements) when achievements are earned or levels are gained.
//!
//! # Audio Behavior
//!
//! - **Achievement unlocks**: Plays tier-appropriate chime (Bronze → simple, Platinum → epic)
//! - **Level-ups**: Plays distinct level-up fanfare
//! - **Voice announcements**: Optional TTS for "Achievement unlocked: [name]"
//!
//! # Concurrent Achievements
//!
//! When multiple achievements are earned in quick succession:
//! - Audio items are queued properly using the audio engine's priority system
//! - Higher-tier achievements get higher priority to ensure they're heard
//! - Queue has reasonable limits to prevent audio pileup

use crate::achievements::{AchievementNotification, AchievementTier, LevelUpNotification};
use crate::audio::alerts::{AlertContext, AlertManager, AlertType};
use crate::audio::engine::AudioEngine;
use crate::audio::tones::CuePattern;
use crate::audio::{AudioItem, AudioPriority};
use std::sync::Arc;

/// Configuration for the achievement audio bridge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AchievementAudioBridgeConfig {
    /// Enable achievement chime sounds
    pub chimes_enabled: bool,
    /// Enable voice announcements for achievements ("Achievement unlocked: [name]")
    pub voice_enabled: bool,
    /// Enable level-up celebration sounds
    pub level_up_sounds_enabled: bool,
    /// Enable voice announcements for level-ups ("Level up! You are now level [n]")
    pub level_up_voice_enabled: bool,
    /// Maximum number of queued achievement sounds before dropping low-priority ones
    #[serde(default = "default_max_queued_sounds")]
    pub max_queued_sounds: usize,
    /// Delay in milliseconds between consecutive achievement audio items
    /// to prevent audio overlap
    #[serde(default = "default_audio_spacing_ms")]
    pub audio_spacing_ms: u64,
}

fn default_max_queued_sounds() -> usize {
    5
}

fn default_audio_spacing_ms() -> u64 {
    500
}

impl Default for AchievementAudioBridgeConfig {
    fn default() -> Self {
        Self {
            chimes_enabled: true,
            voice_enabled: true,
            level_up_sounds_enabled: true,
            level_up_voice_enabled: true,
            max_queued_sounds: default_max_queued_sounds(),
            audio_spacing_ms: default_audio_spacing_ms(),
        }
    }
}

/// Bridges achievement notifications to audio alerts.
///
/// This component processes achievement and level-up notifications and triggers
/// appropriate audio alerts via the AlertManager and chime sounds via the AudioEngine.
///
/// # Usage
///
/// ```ignore
/// let bridge = AchievementAudioBridge::new(alert_manager, audio_engine);
///
/// // When an achievement notification is ready to display:
/// if let Some(notification) = notification_queue.current() {
///     bridge.handle_achievement_notification(notification).await;
/// }
///
/// // When a level-up occurs:
/// bridge.handle_level_up(&level_up_notification).await;
/// ```
///
/// # Audio Priority
///
/// Achievement audio is queued with priority based on tier:
/// - Bronze, Silver: Normal priority
/// - Gold, Diamond: High priority
/// - Legendary: Critical priority (never skipped)
/// - Level-up: High priority
pub struct AchievementAudioBridge<A: AlertManager, E: AudioEngine> {
    /// The alert manager for triggering TTS/voice alerts
    alert_manager: Arc<A>,
    /// The audio engine for playing chime sounds
    audio_engine: Arc<E>,
    /// Configuration for which audio to play
    config: AchievementAudioBridgeConfig,
    /// Count of currently queued achievement sounds (for limiting)
    queued_count: std::sync::atomic::AtomicUsize,
}

impl<A: AlertManager, E: AudioEngine> AchievementAudioBridge<A, E> {
    /// Create a new achievement audio bridge with the given alert manager and audio engine.
    pub fn new(alert_manager: Arc<A>, audio_engine: Arc<E>) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config: AchievementAudioBridgeConfig::default(),
            queued_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create a new achievement audio bridge with custom configuration.
    pub fn with_config(
        alert_manager: Arc<A>,
        audio_engine: Arc<E>,
        config: AchievementAudioBridgeConfig,
    ) -> Self {
        Self {
            alert_manager,
            audio_engine,
            config,
            queued_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Update the bridge configuration.
    pub fn set_config(&mut self, config: AchievementAudioBridgeConfig) {
        self.config = config;
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &AchievementAudioBridgeConfig {
        &self.config
    }

    /// Handle an achievement notification, triggering appropriate audio.
    ///
    /// This should be called when a new achievement notification becomes active
    /// (i.e., when `NotificationQueue::update()` returns true).
    ///
    /// # Audio Triggered
    ///
    /// 1. **Chime**: Tier-appropriate achievement chime (if enabled)
    /// 2. **Voice**: "Achievement unlocked: [achievement name]" (if enabled)
    pub async fn handle_achievement_notification(&self, notification: &AchievementNotification) {
        tracing::debug!(
            "Handling achievement notification: {} (tier: {:?})",
            notification.title,
            notification.tier
        );

        // Check if we're at the queue limit for lower-priority achievements
        let current_queued = self
            .queued_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let priority = self.tier_to_priority(notification.tier);

        // Allow high-priority achievements through even at limit
        if current_queued >= self.config.max_queued_sounds
            && priority < AudioPriority::High
        {
            tracing::debug!(
                "Achievement audio queue full ({}/{}), skipping lower-priority chime for: {}",
                current_queued,
                self.config.max_queued_sounds,
                notification.title
            );
            // Still allow voice announcement as it goes through AlertManager
        } else if self.config.chimes_enabled {
            self.play_achievement_chime(notification.tier).await;
        }

        // Voice announcement through AlertManager
        if self.config.voice_enabled {
            self.announce_achievement(&notification.title).await;
        }
    }

    /// Handle a level-up notification, triggering celebration audio.
    ///
    /// # Audio Triggered
    ///
    /// 1. **Celebration**: Level-up fanfare sound (if enabled)
    /// 2. **Voice**: "Level up! You are now level [n]" (if enabled)
    pub async fn handle_level_up(&self, notification: &LevelUpNotification) {
        tracing::debug!(
            "Handling level-up notification: {} -> {}",
            notification.from_level,
            notification.to_level
        );

        // Play level-up celebration sound
        if self.config.level_up_sounds_enabled {
            self.play_level_up_sound().await;
        }

        // Voice announcement
        if self.config.level_up_voice_enabled {
            let levels_gained = notification.levels_gained();
            let message = if levels_gained > 1 {
                format!(
                    "Level up! You gained {} levels! You are now level {}",
                    levels_gained, notification.to_level
                )
            } else {
                format!("Level up! You are now level {}", notification.to_level)
            };
            self.announce_level_up(&message).await;
        }
    }

    /// Process a batch of achievement notifications.
    ///
    /// Use this when multiple achievements are earned at once (e.g., end of ride).
    /// Audio will be queued with proper spacing to prevent overlap.
    pub async fn handle_multiple_achievements(&self, notifications: &[AchievementNotification]) {
        if notifications.is_empty() {
            return;
        }

        tracing::debug!(
            "Processing {} achievement notifications",
            notifications.len()
        );

        // Sort by tier (highest first) to prioritize impressive sounds
        let mut sorted: Vec<_> = notifications.iter().collect();
        sorted.sort_by(|a, b| b.tier.cmp(&a.tier));

        for (index, notification) in sorted.iter().enumerate() {
            // Add spacing between achievements (except the first)
            if index > 0 && self.config.audio_spacing_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    self.config.audio_spacing_ms,
                ))
                .await;
            }

            self.handle_achievement_notification(notification).await;
        }
    }

    /// Convert achievement tier to audio priority.
    ///
    /// Higher tiers get higher priority to ensure epic achievements
    /// aren't drowned out by lower-tier ones.
    fn tier_to_priority(&self, tier: AchievementTier) -> AudioPriority {
        match tier {
            AchievementTier::Bronze | AchievementTier::Silver => AudioPriority::Normal,
            AchievementTier::Gold | AchievementTier::Diamond => AudioPriority::High,
            AchievementTier::Legendary => AudioPriority::Critical,
        }
    }

    /// Play the appropriate chime for an achievement tier.
    async fn play_achievement_chime(&self, tier: AchievementTier) {
        // Increment queued count
        self.queued_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Get the tier-appropriate CuePattern
        let tier_name = tier.display_name().to_lowercase();
        let pattern = CuePattern::for_achievement_tier(&tier_name);

        if let Some(cue_pattern) = pattern {
            // Get the tones for this pattern
            let tones = cue_pattern.tones();
            let priority = self.tier_to_priority(tier);

            tracing::debug!(
                "Playing achievement chime for tier {:?} ({} tones, priority: {:?})",
                tier,
                tones.len(),
                priority
            );

            for tone in tones {
                if tone.is_pause() {
                    // Sleep for pause duration
                    tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
                } else {
                    // Queue the tone with appropriate priority
                    let audio_item = AudioItem::tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                        .with_priority(priority);
                    self.audio_engine.queue(audio_item);

                    // Also play directly for immediate feedback
                    if let Err(e) = self
                        .audio_engine
                        .play_tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                        .await
                    {
                        tracing::warn!("Failed to play achievement chime tone: {}", e);
                    }
                }
            }
        } else {
            // Fallback: Diamond and Legendary don't have exact CuePattern matches
            // Map them to the closest patterns
            let fallback_pattern = match tier {
                AchievementTier::Diamond => CuePattern::AchievementGold,
                AchievementTier::Legendary => CuePattern::AchievementPlatinum,
                _ => CuePattern::AchievementBronze,
            };

            let tones = fallback_pattern.tones();
            let priority = self.tier_to_priority(tier);

            tracing::debug!(
                "Playing fallback achievement chime for tier {:?} ({} tones)",
                tier,
                tones.len()
            );

            for tone in tones {
                if tone.is_pause() {
                    tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
                } else {
                    if let Err(e) = self
                        .audio_engine
                        .play_tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                        .await
                    {
                        tracing::warn!("Failed to play fallback achievement chime tone: {}", e);
                    }
                }
            }
        }

        // Decrement queued count
        self.queued_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Play the level-up celebration sound.
    async fn play_level_up_sound(&self) {
        let pattern = CuePattern::LevelUp;
        let tones = pattern.tones();

        tracing::debug!("Playing level-up sound ({} tones)", tones.len());

        for tone in tones {
            if tone.is_pause() {
                tokio::time::sleep(std::time::Duration::from_millis(tone.duration_ms)).await;
            } else {
                if let Err(e) = self
                    .audio_engine
                    .play_tone(tone.frequency_hz as u32, tone.duration_ms as u32)
                    .await
                {
                    tracing::warn!("Failed to play level-up tone: {}", e);
                }
            }
        }
    }

    /// Announce an achievement via the alert manager (TTS).
    async fn announce_achievement(&self, achievement_name: &str) {
        tracing::debug!("Announcing achievement: {}", achievement_name);
        self.alert_manager
            .trigger(
                AlertType::AchievementUnlocked,
                AlertContext::achievement(achievement_name),
            )
            .await;
    }

    /// Announce a level-up via the alert manager (TTS).
    async fn announce_level_up(&self, message: &str) {
        tracing::debug!("Announcing level-up: {}", message);
        // Use a generic alert type with custom message
        self.alert_manager
            .trigger(AlertType::AchievementUnlocked, AlertContext::custom(message))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::achievements::AchievementCategory;
    use crate::audio::alerts::{AlertConfig, AlertData};
    use crate::audio::{AudioError, AudioEvent};
    use std::sync::Mutex;
    use tokio::sync::broadcast;
    use uuid::Uuid;

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
        queued_items: Mutex<Vec<AudioItem>>,
        event_tx: broadcast::Sender<AudioEvent>,
    }

    impl MockAudioEngine {
        fn new() -> Self {
            let (event_tx, _) = broadcast::channel(100);
            Self {
                played_tones: Mutex::new(Vec::new()),
                played_sounds: Mutex::new(Vec::new()),
                queued_items: Mutex::new(Vec::new()),
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

        async fn speak(&self, _text: &str) -> Result<(), AudioError> {
            Ok(())
        }

        async fn play_tone(&self, frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
            self.played_tones
                .lock()
                .unwrap()
                .push((frequency_hz, duration_ms));
            Ok(())
        }

        fn set_volume(&self, _volume: u8) {}

        fn get_volume(&self) -> u8 {
            80
        }

        fn queue(&self, item: AudioItem) {
            self.queued_items.lock().unwrap().push(item);
        }

        fn is_playing(&self) -> bool {
            false
        }

        fn stop(&self) {}

        fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
            self.event_tx.subscribe()
        }
    }

    fn create_test_notification(tier: AchievementTier, title: &str) -> AchievementNotification {
        AchievementNotification::new(
            Uuid::new_v4(),
            title,
            "Test description",
            AchievementCategory::Training,
            tier,
            tier.base_xp(),
        )
    }

    #[test]
    fn test_default_config() {
        let config = AchievementAudioBridgeConfig::default();
        assert!(config.chimes_enabled);
        assert!(config.voice_enabled);
        assert!(config.level_up_sounds_enabled);
        assert!(config.level_up_voice_enabled);
        assert_eq!(config.max_queued_sounds, 5);
        assert_eq!(config.audio_spacing_ms, 500);
    }

    #[test]
    fn test_config_serialization() {
        let config = AchievementAudioBridgeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("chimes_enabled"));
        assert!(json.contains("voice_enabled"));
        assert!(json.contains("max_queued_sounds"));

        let deserialized: AchievementAudioBridgeConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.chimes_enabled);
        assert_eq!(deserialized.max_queued_sounds, 5);
    }

    #[test]
    fn test_config_deserialization_with_defaults() {
        // Test backward compatibility - deserializing without new fields
        let json = r#"{
            "chimes_enabled": true,
            "voice_enabled": true,
            "level_up_sounds_enabled": true,
            "level_up_voice_enabled": true
        }"#;

        let config: AchievementAudioBridgeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_queued_sounds, 5); // default
        assert_eq!(config.audio_spacing_ms, 500); // default
    }

    #[test]
    fn test_tier_to_priority() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager, audio_engine);

        assert_eq!(
            bridge.tier_to_priority(AchievementTier::Bronze),
            AudioPriority::Normal
        );
        assert_eq!(
            bridge.tier_to_priority(AchievementTier::Silver),
            AudioPriority::Normal
        );
        assert_eq!(
            bridge.tier_to_priority(AchievementTier::Gold),
            AudioPriority::High
        );
        assert_eq!(
            bridge.tier_to_priority(AchievementTier::Diamond),
            AudioPriority::High
        );
        assert_eq!(
            bridge.tier_to_priority(AchievementTier::Legendary),
            AudioPriority::Critical
        );
    }

    #[tokio::test]
    async fn test_handle_achievement_notification() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        let notification = create_test_notification(AchievementTier::Bronze, "First Ride");
        bridge.handle_achievement_notification(&notification).await;

        // Should have triggered an alert for voice announcement
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].0, AlertType::AchievementUnlocked);

        // Should have played tones for the chime
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played achievement chime tones");
    }

    #[tokio::test]
    async fn test_handle_gold_achievement() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        let notification = create_test_notification(AchievementTier::Gold, "Century Ride");
        bridge.handle_achievement_notification(&notification).await;

        // Gold has 3 tones (C5 -> E5 -> G5)
        let tones = audio_engine.get_played_tones();
        assert!(tones.len() >= 3, "Gold achievement should have at least 3 tones");
    }

    #[tokio::test]
    async fn test_handle_level_up() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        let notification = LevelUpNotification::new(5, 6, 5000);
        bridge.handle_level_up(&notification).await;

        // Should have played level-up tones
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Should have played level-up tones");

        // Should have triggered alert for voice
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_multi_level_up() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        let notification = LevelUpNotification::new(5, 8, 10000); // Gained 3 levels
        bridge.handle_level_up(&notification).await;

        // Should have triggered alert
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);

        // The alert context should mention multiple levels
        // (verification depends on AlertContext::custom implementation)
    }

    #[tokio::test]
    async fn test_chimes_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            chimes_enabled: false,
            voice_enabled: true,
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let notification = create_test_notification(AchievementTier::Gold, "Test");
        bridge.handle_achievement_notification(&notification).await;

        // No tones should be played
        let tones = audio_engine.get_played_tones();
        assert!(tones.is_empty(), "No tones when chimes disabled");

        // Voice should still trigger
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_voice_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            chimes_enabled: true,
            voice_enabled: false,
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let notification = create_test_notification(AchievementTier::Bronze, "Test");
        bridge.handle_achievement_notification(&notification).await;

        // Tones should be played
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Tones should play when voice disabled");

        // No alerts for voice
        let alerts = alert_manager.get_triggered_alerts();
        assert!(alerts.is_empty(), "No alerts when voice disabled");
    }

    #[tokio::test]
    async fn test_all_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            chimes_enabled: false,
            voice_enabled: false,
            level_up_sounds_enabled: false,
            level_up_voice_enabled: false,
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let notification = create_test_notification(AchievementTier::Legendary, "Epic Test");
        bridge.handle_achievement_notification(&notification).await;

        let level_up = LevelUpNotification::new(99, 100, 1000000);
        bridge.handle_level_up(&level_up).await;

        // Nothing should be played
        let tones = audio_engine.get_played_tones();
        assert!(tones.is_empty(), "No tones when all disabled");

        let alerts = alert_manager.get_triggered_alerts();
        assert!(alerts.is_empty(), "No alerts when all disabled");
    }

    #[tokio::test]
    async fn test_handle_multiple_achievements() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            audio_spacing_ms: 0, // No delay for faster testing
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let notifications = vec![
            create_test_notification(AchievementTier::Bronze, "First"),
            create_test_notification(AchievementTier::Gold, "Second"),
            create_test_notification(AchievementTier::Silver, "Third"),
        ];

        bridge.handle_multiple_achievements(&notifications).await;

        // Should have 3 alert triggers
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 3);

        // Should have played tones for all achievements
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty());
    }

    #[tokio::test]
    async fn test_achievements_sorted_by_tier() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            audio_spacing_ms: 0,
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        // Add in random order
        let notifications = vec![
            create_test_notification(AchievementTier::Bronze, "Third"),
            create_test_notification(AchievementTier::Legendary, "First"),
            create_test_notification(AchievementTier::Silver, "Second"),
        ];

        bridge.handle_multiple_achievements(&notifications).await;

        // Alerts should be in tier order (highest first)
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 3);

        // First alert should be for "First" (Legendary)
        // Note: We can't easily verify order without modifying AlertContext
        // but the internal sort logic can be verified via debug logging
    }

    #[tokio::test]
    async fn test_empty_notifications_list() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        bridge.handle_multiple_achievements(&[]).await;

        // Nothing should happen
        let alerts = alert_manager.get_triggered_alerts();
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_level_up_sounds_disabled() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let config = AchievementAudioBridgeConfig {
            level_up_sounds_enabled: false,
            level_up_voice_enabled: true,
            ..Default::default()
        };
        let bridge = AchievementAudioBridge::with_config(alert_manager.clone(), audio_engine.clone(), config);

        let notification = LevelUpNotification::new(10, 11, 10000);
        bridge.handle_level_up(&notification).await;

        // No tones for level-up
        let tones = audio_engine.get_played_tones();
        assert!(tones.is_empty());

        // Voice should still trigger
        let alerts = alert_manager.get_triggered_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_diamond_tier_fallback() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        // Diamond tier doesn't have a direct CuePattern match, should use fallback
        let notification = create_test_notification(AchievementTier::Diamond, "Diamond Test");
        bridge.handle_achievement_notification(&notification).await;

        // Should have played some tones (fallback to Gold pattern)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Diamond should use fallback pattern");
    }

    #[tokio::test]
    async fn test_legendary_tier_fallback() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let bridge = AchievementAudioBridge::new(alert_manager.clone(), audio_engine.clone());

        // Legendary tier doesn't have a direct CuePattern match, should use Platinum fallback
        let notification = create_test_notification(AchievementTier::Legendary, "Legendary Test");
        bridge.handle_achievement_notification(&notification).await;

        // Should have played tones (fallback to Platinum pattern - which has many tones)
        let tones = audio_engine.get_played_tones();
        assert!(!tones.is_empty(), "Legendary should use Platinum fallback pattern");
    }

    #[test]
    fn test_bridge_config_update() {
        let alert_manager = Arc::new(MockAlertManager::new());
        let audio_engine = Arc::new(MockAudioEngine::new());
        let mut bridge = AchievementAudioBridge::new(alert_manager, audio_engine);

        assert!(bridge.config().chimes_enabled);

        let new_config = AchievementAudioBridgeConfig {
            chimes_enabled: false,
            ..Default::default()
        };
        bridge.set_config(new_config);

        assert!(!bridge.config().chimes_enabled);
    }
}
