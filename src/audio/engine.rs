//! Audio Engine
//!
//! Core audio playback engine using rodio.
//!
//! # Priority-Based Interruption
//!
//! The audio engine implements a priority queue system where higher-priority
//! items (like interval changes) can interrupt lower-priority items currently
//! playing. Priority levels are:
//!
//! - `Critical` - Interrupts everything immediately
//! - `High` - Interrupts Normal and Low priority items
//! - `Normal` - Standard priority, queued normally
//! - `Low` - Can be skipped if queue is full

use super::backend::{AudioDeviceStatus, HotPlugConfig, Platform, RodioAudioBackend};
use super::tts::TtsProvider;
use super::{
    AudioCategory, AudioConfig, AudioError, AudioEvent, AudioItem, AudioPriority,
    AudioTimingConfig, AudioType, MuteState, QueueStats, ThreadSafeTtsProvider,
};
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// Trait for audio engine implementations
pub trait AudioEngine: Send + Sync {
    /// Initialize the audio engine
    fn initialize(&self) -> Result<(), AudioError>;

    /// Play a sound effect by name
    fn play_sound(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Play a sound effect by name with a specific category for volume mixing
    fn play_sound_with_category(
        &self,
        name: &str,
        category: AudioCategory,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Speak text using TTS
    fn speak(&self, text: &str)
        -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Play a tone
    fn play_tone(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Play a tone with a specific category for volume mixing
    fn play_tone_with_category(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
        category: AudioCategory,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Set master volume (0-100)
    fn set_volume(&self, volume: u8);

    /// Set volume for a specific audio category (0-100)
    fn set_category_volume(&self, category: AudioCategory, volume: u8);

    /// Get current master volume
    fn get_volume(&self) -> u8;

    /// Get volume for a specific audio category (0-100)
    fn get_category_volume(&self, category: AudioCategory) -> u8;

    /// Queue an audio item
    fn queue(&self, item: AudioItem);

    /// Check if currently playing
    fn is_playing(&self) -> bool;

    /// Stop current playback
    fn stop(&self);

    /// Subscribe to audio events
    fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent>;

    // ========== Mute Control Methods ==========

    /// Mute all audio globally
    ///
    /// This silences all audio output but preserves volume settings.
    /// Call `unmute()` to restore audio at the previous volume.
    fn mute(&self);

    /// Unmute all audio globally
    ///
    /// This restores audio output at the previous volume level.
    fn unmute(&self);

    /// Toggle global mute state
    ///
    /// If currently muted, unmutes. If currently unmuted, mutes.
    /// Returns the new mute state (true = muted).
    fn toggle_mute(&self) -> bool;

    /// Check if audio is globally muted
    fn is_muted(&self) -> bool;

    /// Mute a specific audio category
    ///
    /// This silences audio for the specified category but preserves its volume setting.
    fn mute_category(&self, category: AudioCategory);

    /// Unmute a specific audio category
    ///
    /// This restores audio for the specified category at its previous volume level.
    fn unmute_category(&self, category: AudioCategory);

    /// Toggle mute state for a specific category
    ///
    /// Returns the new mute state for that category (true = muted).
    fn toggle_category_mute(&self, category: AudioCategory) -> bool;

    /// Check if a specific category is muted
    ///
    /// Returns true if the category is muted (either globally or category-specific).
    fn is_category_muted(&self, category: AudioCategory) -> bool;

    /// Get the current mute state for all categories
    ///
    /// Returns a snapshot of the mute state for UI display.
    fn get_mute_state(&self) -> MuteState;

    // ========== Device Status Methods ==========

    /// Get the current audio device status
    ///
    /// Returns information about device availability, platform, and any errors.
    /// Useful for displaying audio device status in the UI.
    fn get_device_status(&self) -> AudioDeviceStatus;

    /// Get the detected platform
    ///
    /// Returns the platform (Windows, macOS, Linux) being used for audio.
    fn get_platform(&self) -> Platform;

    /// Check if audio device is currently available
    ///
    /// Returns true if the audio backend is ready for playback.
    fn is_device_available(&self) -> bool;

    /// Attempt to recover the audio device
    ///
    /// Call this when the audio device becomes unavailable and you want to
    /// try to reconnect. Uses hot-plug configuration for backoff and retry limits.
    ///
    /// Returns true if recovery was attempted (regardless of success).
    fn try_device_recovery(&self) -> bool;

    /// Reset device recovery state
    ///
    /// Resets the failed attempt counter and allows new recovery attempts.
    /// Call this after a user manually connects an audio device.
    fn reset_device_recovery(&self);

    /// Get hot-plug configuration
    fn get_hot_plug_config(&self) -> HotPlugConfig;

    /// Update hot-plug configuration
    fn set_hot_plug_config(&self, config: HotPlugConfig);

    /// Get platform-specific troubleshooting hints
    ///
    /// Returns a list of troubleshooting steps for the current platform
    /// that can help users resolve audio device issues.
    fn get_troubleshooting_hints(&self) -> Vec<&'static str>;

    // ========== Queue Statistics and Timing Methods ==========

    /// Get the current audio timing configuration
    fn get_timing_config(&self) -> AudioTimingConfig;

    /// Update the audio timing configuration
    fn set_timing_config(&self, config: AudioTimingConfig);

    /// Get current queue statistics
    ///
    /// Returns information about queue size, expired/dropped items,
    /// and pressure status. Useful for debugging and monitoring.
    fn get_queue_stats(&self) -> QueueStats;

    /// Reset queue statistics counters
    ///
    /// Resets the expired and dropped counts back to zero.
    fn reset_queue_stats(&self);

    /// Clean up expired items from the queue
    ///
    /// Manually trigger cleanup of stale items. This is normally
    /// done automatically during queue operations when aggressive_cleanup
    /// is enabled.
    fn cleanup_expired(&self) -> usize;

    /// Get the current queue size
    fn queue_size(&self) -> usize;
}

/// Queue entry with priority ordering
#[derive(Debug)]
struct QueueEntry {
    item: AudioItem,
    sequence: u64, // For stable ordering within same priority
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.item.priority == other.item.priority && self.sequence == other.sequence
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then lower sequence (earlier queued)
        // BinaryHeap is a max-heap, so larger items are popped first.
        // We want high priority to come first, so compare priorities directly.
        match self.item.priority.cmp(&other.item.priority) {
            std::cmp::Ordering::Equal => {
                // For same priority, earlier sequence should come first (reverse order)
                other.sequence.cmp(&self.sequence)
            }
            priority_cmp => priority_cmp,
        }
    }
}

/// Default audio engine implementation
pub struct DefaultAudioEngine {
    config: Arc<Mutex<AudioConfig>>,
    queue: Arc<Mutex<BinaryHeap<QueueEntry>>>,
    sequence_counter: Arc<Mutex<u64>>,
    is_playing: Arc<Mutex<bool>>,
    event_tx: broadcast::Sender<AudioEvent>,
    /// Thread-safe TTS provider for speech synthesis
    tts_provider: Arc<ThreadSafeTtsProvider>,
    /// Rodio audio backend for sound files and tones
    audio_backend: Arc<RodioAudioBackend>,
    /// Priority of the currently playing item (None if not playing)
    current_priority: Arc<Mutex<Option<AudioPriority>>>,
    /// Flag to signal that current playback should be interrupted
    interrupt_requested: Arc<AtomicBool>,
    // ========== Queue Statistics Tracking ==========
    /// Count of items expired before playback
    expired_count: Arc<AtomicUsize>,
    /// Count of items dropped due to queue pressure
    dropped_count: Arc<AtomicUsize>,
    /// When the last audio item finished playing (for gap enforcement)
    last_playback_end: Arc<Mutex<Option<Instant>>>,
}

impl DefaultAudioEngine {
    /// Create a new audio engine with the given configuration
    pub fn new(config: AudioConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        // Create and configure the TTS provider
        let tts_provider = Arc::new(ThreadSafeTtsProvider::new());

        // Apply configuration settings to TTS provider
        // Convert voice_volume from 0-100 to 0.0-1.0
        let voice_volume = config.voice_volume as f32 / 100.0;
        tts_provider.set_volume(voice_volume);
        tts_provider.set_rate(config.speech_rate);

        // Create the rodio audio backend
        let audio_backend = Arc::new(RodioAudioBackend::new());

        // Apply volume settings from config (convert 0-100 to 0.0-1.0)
        let master_volume = config.volume as f32 / 100.0;
        audio_backend.set_volume(master_volume);

        // Apply mute state based on enabled flag
        audio_backend.set_muted(!config.enabled);

        Self {
            config: Arc::new(Mutex::new(config)),
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            sequence_counter: Arc::new(Mutex::new(0)),
            is_playing: Arc::new(Mutex::new(false)),
            event_tx,
            tts_provider,
            audio_backend,
            current_priority: Arc::new(Mutex::new(None)),
            interrupt_requested: Arc::new(AtomicBool::new(false)),
            expired_count: Arc::new(AtomicUsize::new(0)),
            dropped_count: Arc::new(AtomicUsize::new(0)),
            last_playback_end: Arc::new(Mutex::new(None)),
        }
    }

    /// Update TTS settings from the current config
    fn update_tts_settings(&self) {
        let config = self.config.lock().unwrap();
        // Convert voice_volume from 0-100 to 0.0-1.0
        let voice_volume = config.voice_volume as f32 / 100.0;
        self.tts_provider.set_volume(voice_volume);
        self.tts_provider.set_rate(config.speech_rate);
    }

    /// Update audio backend settings from the current config
    fn update_backend_settings(&self) {
        let config = self.config.lock().unwrap();
        // Convert master volume from 0-100 to 0.0-1.0
        let master_volume = config.volume as f32 / 100.0;
        self.audio_backend.set_volume(master_volume);
        // Mute when audio is disabled
        self.audio_backend.set_muted(!config.enabled);
    }

    /// Get access to the TTS provider for voice enumeration and configuration
    pub fn tts_provider(&self) -> &ThreadSafeTtsProvider {
        &self.tts_provider
    }

    /// Get access to the audio backend for advanced sound operations
    pub fn audio_backend(&self) -> &RodioAudioBackend {
        &self.audio_backend
    }

    /// Update audio configuration
    pub fn update_config(&self, config: AudioConfig) {
        let mut current = self.config.lock().unwrap();
        *current = config;
        drop(current);
        self.update_tts_settings();
        self.update_backend_settings();
    }

    /// Check if a new item's priority should interrupt the current playback
    fn should_interrupt(&self, new_priority: AudioPriority) -> bool {
        let current = self.current_priority.lock().unwrap();
        match *current {
            None => false, // Nothing playing, no need to interrupt
            Some(current_priority) => {
                // High and Critical priority items interrupt lower-priority items
                // Critical interrupts everything except other Critical items
                // High interrupts Normal and Low
                match new_priority {
                    AudioPriority::Critical => current_priority != AudioPriority::Critical,
                    AudioPriority::High => {
                        current_priority == AudioPriority::Normal
                            || current_priority == AudioPriority::Low
                    }
                    _ => false,
                }
            }
        }
    }

    /// Request interruption of current playback
    fn request_interrupt(&self) {
        self.interrupt_requested.store(true, Ordering::Release);
        // Actually stop the TTS
        self.tts_provider.stop();
    }

    /// Clear the interrupt flag
    fn clear_interrupt(&self) {
        self.interrupt_requested.store(false, Ordering::Release);
    }

    /// Check if interruption was requested
    fn is_interrupt_requested(&self) -> bool {
        self.interrupt_requested.load(Ordering::Acquire)
    }

    /// Clear items from the queue below the given priority
    fn clear_lower_priority(&self, min_priority: AudioPriority) {
        let mut queue = self.queue.lock().unwrap();
        let entries: Vec<QueueEntry> = queue.drain().collect();

        // Re-add only items with priority >= min_priority
        for entry in entries {
            if entry.item.priority >= min_priority {
                queue.push(entry);
            } else {
                tracing::debug!(
                    "Cleared lower-priority audio item: {:?}",
                    entry.item.audio_type
                );
            }
        }
    }

    /// Clear all items from the queue
    pub fn clear_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        tracing::debug!("Audio queue cleared");
    }

    /// Get the highest priority item in the queue without removing it
    fn peek_highest_priority(&self) -> Option<AudioPriority> {
        let queue = self.queue.lock().unwrap();
        queue.peek().map(|entry| entry.item.priority)
    }

    /// Get the next item from the queue, removing expired items
    fn pop_next(&self) -> Option<AudioItem> {
        let mut queue = self.queue.lock().unwrap();

        while let Some(entry) = queue.pop() {
            // Check if item has expired
            if entry.item.is_expired() {
                let age_ms = entry.item.age_ms();
                let type_desc = entry.item.type_description();
                tracing::debug!(
                    "Audio item expired after {}ms: {}",
                    age_ms,
                    type_desc
                );
                self.expired_count.fetch_add(1, Ordering::Relaxed);

                // Emit expired event
                let _ = self.event_tx.send(AudioEvent::ItemExpired {
                    audio_type: type_desc,
                    age_ms,
                });

                continue;
            }
            return Some(entry.item);
        }

        None
    }

    /// Clean up expired items from the queue and return the count
    fn cleanup_expired_items(&self) -> usize {
        let mut queue = self.queue.lock().unwrap();
        let original_len = queue.len();

        // Collect non-expired items
        let entries: Vec<QueueEntry> = queue.drain().collect();
        let mut expired_count = 0;

        for entry in entries {
            if entry.item.is_expired() {
                let age_ms = entry.item.age_ms();
                let type_desc = entry.item.type_description();
                tracing::debug!(
                    "Cleanup: expired item after {}ms: {}",
                    age_ms,
                    type_desc
                );
                expired_count += 1;
                self.expired_count.fetch_add(1, Ordering::Relaxed);

                let _ = self.event_tx.send(AudioEvent::ItemExpired {
                    audio_type: type_desc,
                    age_ms,
                });
            } else {
                queue.push(entry);
            }
        }

        if expired_count > 0 {
            tracing::debug!(
                "Cleaned up {} expired items, {} remaining",
                expired_count,
                queue.len()
            );
        }

        expired_count
    }

    /// Get the current queue size
    fn get_queue_size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Collect queue statistics
    fn collect_queue_stats(&self) -> QueueStats {
        let queue = self.queue.lock().unwrap();
        let timing_config = self.config.lock().unwrap().timing.clone();

        let mut low_priority_count = 0;
        let mut high_priority_count = 0;

        for entry in queue.iter() {
            match entry.item.priority {
                AudioPriority::Low => low_priority_count += 1,
                AudioPriority::High | AudioPriority::Critical => high_priority_count += 1,
                _ => {}
            }
        }

        let item_count = queue.len();
        let under_pressure = timing_config.is_queue_under_pressure(item_count);

        QueueStats {
            item_count,
            expired_count: self.expired_count.load(Ordering::Relaxed),
            dropped_count: self.dropped_count.load(Ordering::Relaxed),
            low_priority_count,
            high_priority_count,
            under_pressure,
        }
    }

    /// Drop low-priority items when queue is under pressure
    fn drop_low_priority_under_pressure(&self) -> usize {
        let timing_config = {
            let config = self.config.lock().unwrap();
            config.timing.clone()
        };

        let mut queue = self.queue.lock().unwrap();
        let current_size = queue.len();

        if !timing_config.is_queue_under_pressure(current_size) {
            return 0;
        }

        // Collect and filter entries
        let entries: Vec<QueueEntry> = queue.drain().collect();
        let mut dropped_count = 0;

        for entry in entries {
            // Drop low-priority items when under pressure
            if entry.item.priority == AudioPriority::Low {
                let type_desc = entry.item.type_description();
                tracing::debug!(
                    "Queue pressure: dropping low-priority item: {}",
                    type_desc
                );
                dropped_count += 1;
                self.dropped_count.fetch_add(1, Ordering::Relaxed);

                let _ = self.event_tx.send(AudioEvent::ItemDropped {
                    audio_type: type_desc,
                    priority: AudioPriority::Low,
                });
            } else {
                queue.push(entry);
            }
        }

        if dropped_count > 0 {
            tracing::debug!(
                "Dropped {} low-priority items due to queue pressure",
                dropped_count
            );
        }

        dropped_count
    }

    /// Record when playback ends for gap enforcement
    fn record_playback_end(&self) {
        *self.last_playback_end.lock().unwrap() = Some(Instant::now());
    }

    /// Check if minimum audio gap has elapsed since last playback
    fn min_gap_elapsed(&self) -> bool {
        let last_end = self.last_playback_end.lock().unwrap();
        let timing_config = self.config.lock().unwrap().timing.clone();

        match *last_end {
            Some(end_time) => {
                let elapsed = end_time.elapsed();
                elapsed >= Duration::from_millis(timing_config.min_audio_gap_ms)
            }
            None => true, // No previous playback, gap is satisfied
        }
    }

    /// Process the audio queue
    ///
    /// This method processes queued audio items in priority order. Higher-priority
    /// items are played first, and if a high-priority item is queued while a
    /// lower-priority item is playing, the current item will be interrupted.
    ///
    /// Each audio item is played with its category-specific volume, which combines
    /// the master volume with the category volume.
    pub async fn process_queue(&self) {
        // Clear any stale interrupt flag
        self.clear_interrupt();

        while let Some(item) = self.pop_next() {
            // Check for interrupt before starting
            if self.is_interrupt_requested() {
                self.clear_interrupt();
                tracing::debug!("Interrupt requested before playing item, skipping to next");
                continue;
            }

            // Check if this audio item should play (enabled and not muted)
            {
                let config = self.config.lock().unwrap();
                if !item.should_play(&config) {
                    tracing::debug!(
                        "Skipping audio item (disabled or muted): {:?} (category: {:?})",
                        item.audio_type,
                        item.category
                    );
                    continue;
                }
            }

            // Calculate the effective volume for this item
            let effective_volume = {
                let config = self.config.lock().unwrap();
                item.effective_volume(&config)
            };

            // Set current priority before playing
            *self.current_priority.lock().unwrap() = Some(item.priority);

            let result: Result<(), AudioError> = match &item.audio_type {
                AudioType::Speech { text } => self.speak_with_interrupt_check(text).await,
                AudioType::SoundEffect { name } => {
                    self.play_sound_with_volume(name, effective_volume).await
                }
                AudioType::Tone {
                    frequency_hz,
                    duration_ms,
                } => {
                    self.play_tone_with_volume(*frequency_hz, *duration_ms, effective_volume)
                        .await
                }
            };

            // Clear current priority after playing
            *self.current_priority.lock().unwrap() = None;

            // If we were interrupted, clear the flag and continue to next (higher priority) item
            if self.is_interrupt_requested() {
                self.clear_interrupt();
                tracing::debug!("Item was interrupted, continuing to next item");
            }

            if let Err(e) = result {
                let _ = self.event_tx.send(AudioEvent::Error {
                    message: e.to_string(),
                });
            }
        }
    }

    /// Speak text with interrupt checking
    ///
    /// This is similar to speak() but checks for interruption during playback.
    async fn speak_with_interrupt_check(&self, text: &str) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled || !config.voice_enabled {
                return Ok(());
            }
        }

        // Update TTS settings from config before speaking
        self.update_tts_settings();

        *self.is_playing.lock().unwrap() = true;

        let _ = self.event_tx.send(AudioEvent::SpeechStarted {
            text: text.to_string(),
        });

        // Use the TTS provider to speak
        let result = self.tts_provider.speak_async(text).await;

        // Check if we were interrupted
        let was_interrupted = self.is_interrupt_requested();

        let _ = self.event_tx.send(AudioEvent::SpeechCompleted);

        *self.is_playing.lock().unwrap() = false;

        if was_interrupted {
            tracing::debug!("Speech was interrupted");
        }

        result
    }

    /// Play a sound effect with a specific volume (0.0 - 1.0)
    ///
    /// This is an internal helper method used by process_queue to apply
    /// category-specific volumes.
    async fn play_sound_with_volume(&self, name: &str, volume: f32) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled {
                return Ok(());
            }
        }

        *self.is_playing.lock().unwrap() = true;

        let _ = self.event_tx.send(AudioEvent::SoundPlayed {
            name: name.to_string(),
        });

        // Play the sound using the rodio audio backend with the specified volume
        let backend = Arc::clone(&self.audio_backend);
        let sound_name = name.to_string();
        let clamped_volume = volume.clamp(0.0, 1.0);

        let result = tokio::task::spawn_blocking(move || {
            backend.play_sound_with_volume(&sound_name, Some(clamped_volume))
        })
        .await
        .map_err(|e| AudioError::PlaybackFailed(format!("Task join error: {}", e)))?;

        // Handle the result - log warnings for missing sounds but don't crash
        match result {
            Ok(duration) => {
                // Wait for the sound to finish playing
                if !duration.is_zero() {
                    tokio::time::sleep(duration).await;
                }
            }
            Err(e) => {
                tracing::warn!("Sound playback failed for '{}': {}", name, e);
            }
        }

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }

    /// Play a tone with a specific volume (0.0 - 1.0)
    ///
    /// This is an internal helper method used by process_queue to apply
    /// category-specific volumes.
    async fn play_tone_with_volume(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
        volume: f32,
    ) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled {
                return Ok(());
            }
        }

        *self.is_playing.lock().unwrap() = true;

        // Play the tone using the rodio audio backend with the specified volume
        let duration = Duration::from_millis(duration_ms as u64);
        let frequency = frequency_hz as f32;
        let clamped_volume = volume.clamp(0.0, 1.0);

        let backend = Arc::clone(&self.audio_backend);
        let result = tokio::task::spawn_blocking(move || {
            backend.play_tone_with_volume(frequency, duration, Some(clamped_volume))
        })
        .await
        .map_err(|e| AudioError::PlaybackFailed(format!("Task join error: {}", e)))?;

        if let Err(e) = result {
            tracing::debug!("Tone playback failed: {}", e);
        }

        // Wait for the tone duration to complete
        tokio::time::sleep(duration).await;

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }
}

impl AudioEngine for DefaultAudioEngine {
    fn initialize(&self) -> Result<(), AudioError> {
        tracing::info!("Initializing audio engine");

        // Initialize TTS provider
        self.tts_provider.initialize()?;

        // Set preferred voice if configured
        {
            let config = self.config.lock().unwrap();
            if let Some(ref voice_id) = config.preferred_voice {
                if let Err(e) = self.tts_provider.set_voice(voice_id) {
                    tracing::warn!("Failed to set preferred voice '{}': {}", voice_id, e);
                }
            }
        }

        // Initialize the rodio audio backend
        // This may fail if no audio device is available, but we log and continue
        // to allow TTS to work even if sound effects/tones fail
        if let Err(e) = self.audio_backend.initialize() {
            tracing::warn!("Audio backend initialization failed: {}. Tones and sound effects will be unavailable.", e);
        } else {
            tracing::info!("Audio backend initialized successfully");
        }

        // Apply current config settings to the backend
        self.update_backend_settings();

        Ok(())
    }

    async fn play_sound(&self, name: &str) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled || !config.sound_effects_enabled {
                return Ok(());
            }
        }

        *self.is_playing.lock().unwrap() = true;

        let _ = self.event_tx.send(AudioEvent::SoundPlayed {
            name: name.to_string(),
        });

        // Play the sound using the rodio audio backend
        // The backend handles volume, muting, caching, and device availability
        let backend = Arc::clone(&self.audio_backend);
        let sound_name = name.to_string();

        let result = tokio::task::spawn_blocking(move || backend.play_sound(&sound_name))
            .await
            .map_err(|e| AudioError::PlaybackFailed(format!("Task join error: {}", e)))?;

        // Handle the result - log warnings for missing sounds but don't crash
        match result {
            Ok(duration) => {
                // Wait for the sound to finish playing
                // This ensures is_playing flag reflects actual playback
                if !duration.is_zero() {
                    tokio::time::sleep(duration).await;
                }
            }
            Err(e) => {
                // Log the error but don't propagate - gracefully handle missing sounds
                tracing::warn!("Sound playback failed for '{}': {}", name, e);
            }
        }

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }

    async fn play_sound_with_category(
        &self,
        name: &str,
        category: AudioCategory,
    ) -> Result<(), AudioError> {
        let effective_volume = {
            let config = self.config.lock().unwrap();
            if !config.enabled || !category.is_enabled(&config) {
                return Ok(());
            }
            category.effective_volume(&config)
        };

        self.play_sound_with_volume(name, effective_volume).await
    }

    async fn speak(&self, text: &str) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled || !config.voice_enabled {
                return Ok(());
            }
        }

        // Update TTS settings from config before speaking
        self.update_tts_settings();

        *self.is_playing.lock().unwrap() = true;

        let _ = self.event_tx.send(AudioEvent::SpeechStarted {
            text: text.to_string(),
        });

        // Use the TTS provider to speak
        let result = self.tts_provider.speak_async(text).await;

        let _ = self.event_tx.send(AudioEvent::SpeechCompleted);

        *self.is_playing.lock().unwrap() = false;

        result
    }

    async fn play_tone(&self, frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled {
                return Ok(());
            }
        }

        *self.is_playing.lock().unwrap() = true;

        // Play the tone using the rodio audio backend
        // The backend handles volume, muting, and device availability
        let duration = Duration::from_millis(duration_ms as u64);
        let frequency = frequency_hz as f32;

        // Spawn blocking task since rodio operations are blocking
        let backend = Arc::clone(&self.audio_backend);
        let result = tokio::task::spawn_blocking(move || {
            backend.play_tone(frequency, duration)
        })
        .await
        .map_err(|e| AudioError::PlaybackFailed(format!("Task join error: {}", e)))?;

        // Convert backend error to audio error
        if let Err(e) = result {
            tracing::debug!("Tone playback failed: {}", e);
            // Don't propagate the error - gracefully handle missing audio device
        }

        // Wait for the tone duration to complete
        // This ensures the is_playing flag remains true while the tone is audible
        tokio::time::sleep(duration).await;

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }

    async fn play_tone_with_category(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
        category: AudioCategory,
    ) -> Result<(), AudioError> {
        let effective_volume = {
            let config = self.config.lock().unwrap();
            if !config.enabled || !category.is_enabled(&config) {
                return Ok(());
            }
            category.effective_volume(&config)
        };

        self.play_tone_with_volume(frequency_hz, duration_ms, effective_volume)
            .await
    }

    fn set_volume(&self, volume: u8) {
        let clamped_volume = volume.min(100);
        {
            let mut config = self.config.lock().unwrap();
            config.volume = clamped_volume;
        }
        // Also update the audio backend volume (convert 0-100 to 0.0-1.0)
        self.audio_backend.set_volume(clamped_volume as f32 / 100.0);
    }

    fn set_category_volume(&self, category: AudioCategory, volume: u8) {
        let clamped_volume = volume.min(100);
        let mut config = self.config.lock().unwrap();
        match category {
            AudioCategory::Voice => {
                config.voice_volume = clamped_volume;
                // Also update TTS provider volume
                drop(config);
                self.update_tts_settings();
            }
            AudioCategory::SoundEffect => config.sound_effects_volume = clamped_volume,
            AudioCategory::Countdown => config.countdown_volume = clamped_volume,
            AudioCategory::Achievement => config.achievement_volume = clamped_volume,
            AudioCategory::Milestone => config.milestone_volume = clamped_volume,
        }
    }

    fn get_volume(&self) -> u8 {
        self.config.lock().unwrap().volume
    }

    fn get_category_volume(&self, category: AudioCategory) -> u8 {
        let config = self.config.lock().unwrap();
        category.volume_from_config(&config)
    }

    fn queue(&self, item: AudioItem) {
        let priority = item.priority;
        let timing_config = self.config.lock().unwrap().timing.clone();

        // Aggressive cleanup: remove expired items before adding new ones
        if timing_config.aggressive_cleanup {
            self.cleanup_expired_items();
        }

        // Check queue size limits
        {
            let queue_size = self.get_queue_size();
            if queue_size >= timing_config.max_queue_size {
                // Queue is full - handle based on priority
                if priority == AudioPriority::Low {
                    // Drop low priority items when queue is full
                    tracing::debug!(
                        "Queue full ({}), dropping low-priority item: {}",
                        queue_size,
                        item.type_description()
                    );
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    let _ = self.event_tx.send(AudioEvent::ItemDropped {
                        audio_type: item.type_description(),
                        priority: item.priority,
                    });
                    return;
                } else {
                    // For higher priority items, drop low priority items to make room
                    self.drop_low_priority_under_pressure();
                }
            }
        }

        // Emit queue pressure warning if needed
        {
            let queue_size = self.get_queue_size();
            if timing_config.is_queue_under_pressure(queue_size) {
                let _ = self.event_tx.send(AudioEvent::QueuePressure {
                    current_size: queue_size,
                    max_size: timing_config.max_queue_size,
                });
            }
        }

        // Check if this high-priority item should interrupt current playback
        if self.should_interrupt(priority) {
            tracing::debug!(
                "High-priority item queued ({:?}), interrupting current playback",
                priority
            );
            self.request_interrupt();
        }

        // For high-priority items, also clear lower-priority items from the queue
        // to ensure the important message gets through quickly
        if priority >= AudioPriority::High {
            self.clear_lower_priority(AudioPriority::Normal);
        }

        let mut queue = self.queue.lock().unwrap();
        let mut counter = self.sequence_counter.lock().unwrap();

        *counter += 1;
        let sequence = *counter;

        queue.push(QueueEntry { item, sequence });
    }

    fn is_playing(&self) -> bool {
        *self.is_playing.lock().unwrap()
    }

    fn stop(&self) {
        tracing::debug!("Audio engine stop requested");

        // Set interrupt flag to signal any ongoing playback to stop
        self.interrupt_requested.store(true, Ordering::Release);

        // Stop TTS playback immediately
        self.tts_provider.stop();

        // Stop all audio backend playback (tones and sound effects)
        self.audio_backend.stop_all();

        // Clear the queue - stop() means stop everything
        self.clear_queue();

        // Reset state
        *self.is_playing.lock().unwrap() = false;
        *self.current_priority.lock().unwrap() = None;
    }

    fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
        self.event_tx.subscribe()
    }

    // ========== Mute Control Methods ==========

    fn mute(&self) {
        tracing::debug!("Global mute requested");
        {
            let mut config = self.config.lock().unwrap();
            config.muted = true;
        }
        // Also mute the audio backend for immediate effect
        self.audio_backend.set_muted(true);
    }

    fn unmute(&self) {
        tracing::debug!("Global unmute requested");
        {
            let mut config = self.config.lock().unwrap();
            config.muted = false;
        }
        // Unmute the audio backend if the audio system is enabled
        let enabled = self.config.lock().unwrap().enabled;
        self.audio_backend.set_muted(!enabled);
    }

    fn toggle_mute(&self) -> bool {
        let new_mute_state = {
            let mut config = self.config.lock().unwrap();
            config.muted = !config.muted;
            config.muted
        };
        tracing::debug!("Global mute toggled to: {}", new_mute_state);

        // Update backend mute state
        if new_mute_state {
            self.audio_backend.set_muted(true);
        } else {
            let enabled = self.config.lock().unwrap().enabled;
            self.audio_backend.set_muted(!enabled);
        }

        new_mute_state
    }

    fn is_muted(&self) -> bool {
        self.config.lock().unwrap().muted
    }

    fn mute_category(&self, category: AudioCategory) {
        tracing::debug!("Muting category: {:?}", category);
        let mut config = self.config.lock().unwrap();
        match category {
            AudioCategory::Voice => config.voice_muted = true,
            AudioCategory::SoundEffect => config.sound_effects_muted = true,
            AudioCategory::Countdown => config.countdown_muted = true,
            AudioCategory::Achievement => config.achievement_muted = true,
            AudioCategory::Milestone => config.milestone_muted = true,
        }
    }

    fn unmute_category(&self, category: AudioCategory) {
        tracing::debug!("Unmuting category: {:?}", category);
        let mut config = self.config.lock().unwrap();
        match category {
            AudioCategory::Voice => config.voice_muted = false,
            AudioCategory::SoundEffect => config.sound_effects_muted = false,
            AudioCategory::Countdown => config.countdown_muted = false,
            AudioCategory::Achievement => config.achievement_muted = false,
            AudioCategory::Milestone => config.milestone_muted = false,
        }
    }

    fn toggle_category_mute(&self, category: AudioCategory) -> bool {
        let new_mute_state = {
            let mut config = self.config.lock().unwrap();
            match category {
                AudioCategory::Voice => {
                    config.voice_muted = !config.voice_muted;
                    config.voice_muted
                }
                AudioCategory::SoundEffect => {
                    config.sound_effects_muted = !config.sound_effects_muted;
                    config.sound_effects_muted
                }
                AudioCategory::Countdown => {
                    config.countdown_muted = !config.countdown_muted;
                    config.countdown_muted
                }
                AudioCategory::Achievement => {
                    config.achievement_muted = !config.achievement_muted;
                    config.achievement_muted
                }
                AudioCategory::Milestone => {
                    config.milestone_muted = !config.milestone_muted;
                    config.milestone_muted
                }
            }
        };
        tracing::debug!("Category {:?} mute toggled to: {}", category, new_mute_state);
        new_mute_state
    }

    fn is_category_muted(&self, category: AudioCategory) -> bool {
        let config = self.config.lock().unwrap();
        category.is_muted(&config)
    }

    fn get_mute_state(&self) -> MuteState {
        let config = self.config.lock().unwrap();
        MuteState::from_config(&config)
    }

    // ========== Device Status Methods ==========

    fn get_device_status(&self) -> AudioDeviceStatus {
        self.audio_backend.get_device_status()
    }

    fn get_platform(&self) -> Platform {
        self.audio_backend.platform()
    }

    fn is_device_available(&self) -> bool {
        self.audio_backend.is_ready()
    }

    fn try_device_recovery(&self) -> bool {
        self.audio_backend.try_recovery()
    }

    fn reset_device_recovery(&self) {
        self.audio_backend.reset_recovery();
    }

    fn get_hot_plug_config(&self) -> HotPlugConfig {
        self.audio_backend.hot_plug_config()
    }

    fn set_hot_plug_config(&self, config: HotPlugConfig) {
        self.audio_backend.set_hot_plug_config(config);
    }

    fn get_troubleshooting_hints(&self) -> Vec<&'static str> {
        self.audio_backend.get_troubleshooting_hints()
    }

    // ========== Queue Statistics and Timing Methods ==========

    fn get_timing_config(&self) -> AudioTimingConfig {
        self.config.lock().unwrap().timing.clone()
    }

    fn set_timing_config(&self, timing_config: AudioTimingConfig) {
        self.config.lock().unwrap().timing = timing_config;
    }

    fn get_queue_stats(&self) -> QueueStats {
        self.collect_queue_stats()
    }

    fn reset_queue_stats(&self) {
        self.expired_count.store(0, Ordering::Relaxed);
        self.dropped_count.store(0, Ordering::Relaxed);
        tracing::debug!("Queue statistics reset");
    }

    fn cleanup_expired(&self) -> usize {
        self.cleanup_expired_items()
    }

    fn queue_size(&self) -> usize {
        self.get_queue_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioPriority;

    #[test]
    fn test_queue_priority_ordering() {
        let mut heap = BinaryHeap::new();

        heap.push(QueueEntry {
            item: AudioItem::speech("Low priority").with_priority(AudioPriority::Low),
            sequence: 1,
        });
        heap.push(QueueEntry {
            item: AudioItem::speech("High priority").with_priority(AudioPriority::High),
            sequence: 2,
        });
        heap.push(QueueEntry {
            item: AudioItem::speech("Normal priority").with_priority(AudioPriority::Normal),
            sequence: 3,
        });

        // Should pop in priority order: High, Normal, Low
        let first = heap.pop().unwrap();
        assert_eq!(first.item.priority, AudioPriority::High);

        let second = heap.pop().unwrap();
        assert_eq!(second.item.priority, AudioPriority::Normal);

        let third = heap.pop().unwrap();
        assert_eq!(third.item.priority, AudioPriority::Low);
    }

    #[test]
    fn test_engine_creation() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        assert_eq!(engine.get_volume(), 80);
        assert!(!engine.is_playing());
    }

    #[test]
    fn test_volume_clamping() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        engine.set_volume(150);
        assert_eq!(engine.get_volume(), 100);
    }

    #[test]
    fn test_should_interrupt_with_high_priority() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Simulate playing a Normal priority item
        *engine.current_priority.lock().unwrap() = Some(AudioPriority::Normal);

        // High priority should interrupt Normal
        assert!(engine.should_interrupt(AudioPriority::High));
        assert!(engine.should_interrupt(AudioPriority::Critical));

        // Normal and Low should not interrupt Normal
        assert!(!engine.should_interrupt(AudioPriority::Normal));
        assert!(!engine.should_interrupt(AudioPriority::Low));
    }

    #[test]
    fn test_should_interrupt_with_critical_priority() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Simulate playing a High priority item
        *engine.current_priority.lock().unwrap() = Some(AudioPriority::High);

        // Only Critical should interrupt High
        assert!(engine.should_interrupt(AudioPriority::Critical));
        assert!(!engine.should_interrupt(AudioPriority::High));
        assert!(!engine.should_interrupt(AudioPriority::Normal));
        assert!(!engine.should_interrupt(AudioPriority::Low));
    }

    #[test]
    fn test_should_not_interrupt_when_not_playing() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // No current priority (nothing playing)
        assert!(engine.current_priority.lock().unwrap().is_none());

        // Nothing should trigger interrupt when nothing is playing
        assert!(!engine.should_interrupt(AudioPriority::Critical));
        assert!(!engine.should_interrupt(AudioPriority::High));
        assert!(!engine.should_interrupt(AudioPriority::Normal));
        assert!(!engine.should_interrupt(AudioPriority::Low));
    }

    #[test]
    fn test_interrupt_flag_operations() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initially not interrupted
        assert!(!engine.is_interrupt_requested());

        // Request interrupt sets the flag
        engine.interrupt_requested.store(true, Ordering::Release);
        assert!(engine.is_interrupt_requested());

        // Clear interrupt clears the flag
        engine.clear_interrupt();
        assert!(!engine.is_interrupt_requested());
    }

    #[test]
    fn test_clear_lower_priority() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue items with different priorities
        engine.queue(AudioItem::speech("Low").with_priority(AudioPriority::Low));
        engine.queue(AudioItem::speech("Normal").with_priority(AudioPriority::Normal));
        engine.queue(AudioItem::speech("High").with_priority(AudioPriority::High));

        // Clear items below High priority
        engine.clear_lower_priority(AudioPriority::High);

        // Only High priority item should remain
        let queue = engine.queue.lock().unwrap();
        assert_eq!(queue.len(), 1);
        let entry = queue.peek().unwrap();
        assert_eq!(entry.item.priority, AudioPriority::High);
    }

    #[test]
    fn test_clear_queue() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue some items
        engine.queue(AudioItem::speech("One"));
        engine.queue(AudioItem::speech("Two"));
        engine.queue(AudioItem::speech("Three"));

        assert_eq!(engine.queue.lock().unwrap().len(), 3);

        // Clear queue
        engine.clear_queue();

        assert_eq!(engine.queue.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_stop_clears_queue_and_state() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue some items
        engine.queue(AudioItem::speech("One"));
        engine.queue(AudioItem::speech("Two"));

        // Set some state
        *engine.is_playing.lock().unwrap() = true;
        *engine.current_priority.lock().unwrap() = Some(AudioPriority::Normal);

        // Call stop
        engine.stop();

        // Verify everything is reset
        assert!(!engine.is_playing());
        assert!(engine.current_priority.lock().unwrap().is_none());
        assert_eq!(engine.queue.lock().unwrap().len(), 0);
        assert!(engine.is_interrupt_requested()); // Interrupt flag is set
    }

    #[test]
    fn test_high_priority_queue_triggers_interrupt() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Simulate playing a Normal priority item
        *engine.current_priority.lock().unwrap() = Some(AudioPriority::Normal);

        // Queue a High priority item
        engine.queue(AudioItem::speech("Important!").with_priority(AudioPriority::High));

        // Interrupt should have been requested
        assert!(engine.is_interrupt_requested());
    }

    #[test]
    fn test_normal_priority_queue_does_not_interrupt() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Simulate playing a Normal priority item
        *engine.current_priority.lock().unwrap() = Some(AudioPriority::Normal);

        // Queue a Normal priority item
        engine.queue(AudioItem::speech("Regular message").with_priority(AudioPriority::Normal));

        // Interrupt should NOT have been requested
        assert!(!engine.is_interrupt_requested());
    }

    #[test]
    fn test_peek_highest_priority() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Empty queue returns None
        assert!(engine.peek_highest_priority().is_none());

        // Queue items
        engine.queue(AudioItem::speech("Low").with_priority(AudioPriority::Low));
        assert_eq!(engine.peek_highest_priority(), Some(AudioPriority::Low));

        engine.queue(AudioItem::speech("High").with_priority(AudioPriority::High));
        assert_eq!(engine.peek_highest_priority(), Some(AudioPriority::High));

        engine.queue(AudioItem::speech("Normal").with_priority(AudioPriority::Normal));
        // Still High (highest in queue)
        assert_eq!(engine.peek_highest_priority(), Some(AudioPriority::High));
    }

    #[test]
    fn test_urgent_speech_helper() {
        let item = AudioItem::urgent_speech("Interval change!");
        assert_eq!(item.priority, AudioPriority::High);
        match item.audio_type {
            AudioType::Speech { text } => assert_eq!(text, "Interval change!"),
            _ => panic!("Expected Speech type"),
        }
    }

    #[test]
    fn test_audio_backend_created() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Audio backend should be accessible
        let backend = engine.audio_backend();
        // Default volume should be 0.8 (80/100)
        assert!((backend.volume() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_volume_propagates_to_backend() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Set volume to 50%
        engine.set_volume(50);

        // Backend should reflect the new volume (50/100 = 0.5)
        assert!((engine.audio_backend().volume() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_config_update_applies_to_backend() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Update config with new volume
        let mut new_config = AudioConfig::default();
        new_config.volume = 60;
        engine.update_config(new_config);

        // Backend should reflect the new volume (60/100 = 0.6)
        assert!((engine.audio_backend().volume() - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_disabled_audio_mutes_backend() {
        let mut config = AudioConfig::default();
        config.enabled = false;
        let engine = DefaultAudioEngine::new(config);

        // Backend should be muted when audio is disabled
        assert!(engine.audio_backend().is_muted());
    }

    #[test]
    fn test_enabled_audio_unmutes_backend() {
        let config = AudioConfig::default(); // enabled = true by default
        let engine = DefaultAudioEngine::new(config);

        // Backend should not be muted when audio is enabled
        assert!(!engine.audio_backend().is_muted());
    }

    #[test]
    fn test_tone_audio_item() {
        let item = AudioItem {
            audio_type: AudioType::Tone {
                frequency_hz: 440,
                duration_ms: 200,
            },
            priority: AudioPriority::Normal,
            queued_at: std::time::Instant::now(),
            max_queue_time: Duration::from_secs(5),
            category: None, // Generic tones have no category
        };

        match item.audio_type {
            AudioType::Tone {
                frequency_hz,
                duration_ms,
            } => {
                assert_eq!(frequency_hz, 440);
                assert_eq!(duration_ms, 200);
            }
            _ => panic!("Expected Tone type"),
        }
    }

    #[test]
    fn test_sound_audio_item() {
        let item = AudioItem::sound("countdown_tick");
        assert_eq!(item.priority, AudioPriority::Normal);
        match item.audio_type {
            AudioType::SoundEffect { name } => {
                assert_eq!(name, "countdown_tick");
            }
            _ => panic!("Expected SoundEffect type"),
        }
    }

    #[test]
    fn test_sound_with_priority() {
        let item = AudioItem::sound("achievement_chime").with_priority(AudioPriority::High);
        assert_eq!(item.priority, AudioPriority::High);
        match item.audio_type {
            AudioType::SoundEffect { name } => {
                assert_eq!(name, "achievement_chime");
            }
            _ => panic!("Expected SoundEffect type"),
        }
    }

    #[test]
    fn test_sound_effects_disabled_config() {
        let mut config = AudioConfig::default();
        config.sound_effects_enabled = false;
        let engine = DefaultAudioEngine::new(config);

        // With sound effects disabled, the config should reflect this
        let current_config = engine.config.lock().unwrap();
        assert!(!current_config.sound_effects_enabled);
    }

    #[test]
    fn test_backend_is_accessible_for_sound_operations() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Backend should be accessible
        let backend = engine.audio_backend();

        // Initially no sounds are cached
        assert_eq!(backend.cache_size(), 0);
        assert!(!backend.is_cached("test_sound"));
    }

    // ========== Category Volume Tests ==========

    #[test]
    fn test_set_category_volume() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Set countdown volume to 50%
        engine.set_category_volume(AudioCategory::Countdown, 50);
        assert_eq!(engine.get_category_volume(AudioCategory::Countdown), 50);

        // Set achievement volume to 75%
        engine.set_category_volume(AudioCategory::Achievement, 75);
        assert_eq!(engine.get_category_volume(AudioCategory::Achievement), 75);

        // Verify other volumes unchanged
        assert_eq!(engine.get_category_volume(AudioCategory::Voice), 100);
    }

    #[test]
    fn test_set_category_volume_clamping() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Setting volume above 100 should clamp to 100
        engine.set_category_volume(AudioCategory::SoundEffect, 150);
        assert_eq!(engine.get_category_volume(AudioCategory::SoundEffect), 100);
    }

    #[test]
    fn test_get_category_volume_all_categories() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Verify default volumes match config defaults
        assert_eq!(engine.get_category_volume(AudioCategory::Voice), 100);
        assert_eq!(engine.get_category_volume(AudioCategory::SoundEffect), 80);
        assert_eq!(engine.get_category_volume(AudioCategory::Countdown), 100);
        assert_eq!(engine.get_category_volume(AudioCategory::Achievement), 100);
        assert_eq!(engine.get_category_volume(AudioCategory::Milestone), 70);
    }

    #[test]
    fn test_voice_category_updates_tts() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Set voice volume - this should update the TTS provider
        engine.set_category_volume(AudioCategory::Voice, 60);
        assert_eq!(engine.get_category_volume(AudioCategory::Voice), 60);

        // Verify the config was updated
        let current_config = engine.config.lock().unwrap();
        assert_eq!(current_config.voice_volume, 60);
    }

    #[test]
    fn test_audio_item_category_in_queue() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue items with different categories
        engine.queue(AudioItem::countdown_tone(440, 100));
        engine.queue(AudioItem::achievement_sound("chime"));
        engine.queue(AudioItem::milestone_tone(330, 200));

        // Verify queue has 3 items
        let queue = engine.queue.lock().unwrap();
        assert_eq!(queue.len(), 3);
    }

    // ========== Mute Control Tests ==========

    #[test]
    fn test_global_mute_unmute() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initially not muted
        assert!(!engine.is_muted());
        assert!(!engine.audio_backend().is_muted());

        // Mute
        engine.mute();
        assert!(engine.is_muted());
        assert!(engine.audio_backend().is_muted());

        // Unmute
        engine.unmute();
        assert!(!engine.is_muted());
        assert!(!engine.audio_backend().is_muted());
    }

    #[test]
    fn test_toggle_mute() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initially not muted
        assert!(!engine.is_muted());

        // Toggle on
        let result = engine.toggle_mute();
        assert!(result); // Returns new state = muted
        assert!(engine.is_muted());

        // Toggle off
        let result = engine.toggle_mute();
        assert!(!result); // Returns new state = unmuted
        assert!(!engine.is_muted());
    }

    #[test]
    fn test_mute_category() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initially no category is muted
        assert!(!engine.is_category_muted(AudioCategory::Voice));
        assert!(!engine.is_category_muted(AudioCategory::SoundEffect));

        // Mute voice
        engine.mute_category(AudioCategory::Voice);
        assert!(engine.is_category_muted(AudioCategory::Voice));
        assert!(!engine.is_category_muted(AudioCategory::SoundEffect));

        // Unmute voice
        engine.unmute_category(AudioCategory::Voice);
        assert!(!engine.is_category_muted(AudioCategory::Voice));
    }

    #[test]
    fn test_toggle_category_mute() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initially not muted
        assert!(!engine.is_category_muted(AudioCategory::Countdown));

        // Toggle on
        let result = engine.toggle_category_mute(AudioCategory::Countdown);
        assert!(result);
        assert!(engine.is_category_muted(AudioCategory::Countdown));

        // Toggle off
        let result = engine.toggle_category_mute(AudioCategory::Countdown);
        assert!(!result);
        assert!(!engine.is_category_muted(AudioCategory::Countdown));
    }

    #[test]
    fn test_is_category_muted_respects_global_mute() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Mute globally
        engine.mute();

        // All categories should appear muted
        assert!(engine.is_category_muted(AudioCategory::Voice));
        assert!(engine.is_category_muted(AudioCategory::SoundEffect));
        assert!(engine.is_category_muted(AudioCategory::Countdown));
        assert!(engine.is_category_muted(AudioCategory::Achievement));
        assert!(engine.is_category_muted(AudioCategory::Milestone));
    }

    #[test]
    fn test_get_mute_state() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Initial state
        let state = engine.get_mute_state();
        assert!(!state.globally_muted);
        assert!(!state.voice_muted);

        // Mute globally and one category
        engine.mute();
        engine.mute_category(AudioCategory::Countdown);

        let state = engine.get_mute_state();
        assert!(state.globally_muted);
        assert!(state.countdown_muted);
        assert!(!state.voice_muted);
    }

    #[test]
    fn test_mute_all_categories_independently() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Mute each category independently
        engine.mute_category(AudioCategory::Voice);
        engine.mute_category(AudioCategory::SoundEffect);
        engine.mute_category(AudioCategory::Countdown);
        engine.mute_category(AudioCategory::Achievement);
        engine.mute_category(AudioCategory::Milestone);

        // Verify all are muted
        assert!(engine.is_category_muted(AudioCategory::Voice));
        assert!(engine.is_category_muted(AudioCategory::SoundEffect));
        assert!(engine.is_category_muted(AudioCategory::Countdown));
        assert!(engine.is_category_muted(AudioCategory::Achievement));
        assert!(engine.is_category_muted(AudioCategory::Milestone));

        // Global is not muted
        assert!(!engine.is_muted());

        // Unmute countdown only
        engine.unmute_category(AudioCategory::Countdown);
        assert!(!engine.is_category_muted(AudioCategory::Countdown));
        assert!(engine.is_category_muted(AudioCategory::Voice));
    }

    #[test]
    fn test_mute_preserves_volume() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Set a specific volume
        engine.set_volume(60);
        assert_eq!(engine.get_volume(), 60);

        // Mute and check volume is preserved
        engine.mute();
        assert_eq!(engine.get_volume(), 60);

        // Unmute and check volume is still 60
        engine.unmute();
        assert_eq!(engine.get_volume(), 60);
    }

    #[test]
    fn test_category_mute_preserves_category_volume() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Set a specific category volume
        engine.set_category_volume(AudioCategory::Countdown, 75);
        assert_eq!(engine.get_category_volume(AudioCategory::Countdown), 75);

        // Mute category and check volume is preserved
        engine.mute_category(AudioCategory::Countdown);
        assert_eq!(engine.get_category_volume(AudioCategory::Countdown), 75);

        // Unmute and check volume is still 75
        engine.unmute_category(AudioCategory::Countdown);
        assert_eq!(engine.get_category_volume(AudioCategory::Countdown), 75);
    }

    #[test]
    fn test_unmute_respects_enabled_state() {
        let mut config = AudioConfig::default();
        config.enabled = false; // Audio is disabled
        let engine = DefaultAudioEngine::new(config);

        // Backend should be muted because audio is disabled
        assert!(engine.audio_backend().is_muted());

        // Mute and then unmute
        engine.mute();
        engine.unmute();

        // Backend should still be muted because audio is disabled
        assert!(engine.audio_backend().is_muted());
    }

    #[test]
    fn test_mute_state_display_helpers() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        let state = engine.get_mute_state();
        assert_eq!(state.display_string(), "Audio Active");
        assert_eq!(state.icon_hint(), "volume_up");

        engine.mute_category(AudioCategory::Voice);
        let state = engine.get_mute_state();
        assert_eq!(state.display_string(), "Some Audio Muted");
        assert_eq!(state.icon_hint(), "volume_mute");

        engine.mute();
        let state = engine.get_mute_state();
        assert_eq!(state.display_string(), "All Audio Muted");
        assert_eq!(state.icon_hint(), "volume_off");
    }

    // ========== Device Status Tests ==========

    #[test]
    fn test_get_device_status() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        let status = engine.get_device_status();

        // Initially not available (backend not initialized)
        assert!(!status.available);
        assert_eq!(status.recovery_count, 0);
        assert_eq!(status.failed_attempts, 0);
    }

    #[test]
    fn test_get_platform() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        let platform = engine.get_platform();

        // Should be a valid platform
        assert!(matches!(
            platform,
            Platform::Windows | Platform::MacOS | Platform::Linux | Platform::Unknown
        ));
    }

    #[test]
    fn test_is_device_available() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Not available until initialized
        assert!(!engine.is_device_available());
    }

    #[test]
    fn test_get_troubleshooting_hints() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        let hints = engine.get_troubleshooting_hints();

        // Should have at least some hints
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_hot_plug_config_access() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Get default config
        let hot_plug_config = engine.get_hot_plug_config();
        assert!(hot_plug_config.enabled);

        // Update config
        let new_config = HotPlugConfig {
            enabled: false,
            retry_interval: Duration::from_secs(20),
            max_consecutive_failures: 5,
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(180),
        };
        engine.set_hot_plug_config(new_config);

        // Verify update
        let updated_config = engine.get_hot_plug_config();
        assert!(!updated_config.enabled);
        assert_eq!(updated_config.retry_interval, Duration::from_secs(20));
    }

    #[test]
    fn test_reset_device_recovery() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Reset should not panic on fresh engine
        engine.reset_device_recovery();

        // Device status should reflect reset
        let status = engine.get_device_status();
        assert_eq!(status.failed_attempts, 0);
    }

    #[test]
    fn test_try_device_recovery_when_not_needed() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Manually mark backend as ready
        // (In real use, this would happen through initialization)
        // Since we can't set the state directly, we test the behavior
        // when the engine is in initial (uninitialized) state

        // Try recovery - should attempt since backend is uninitialized
        // (will fail because no audio device in test environment typically)
        // But the important thing is it doesn't panic
        let _attempted = engine.try_device_recovery();
        // Result depends on environment - just verify no panic
    }

    // ========== Timing Safeguards Tests ==========

    #[test]
    fn test_timing_config_defaults() {
        let config = AudioConfig::default();
        let timing = config.timing;

        assert_eq!(timing.max_queue_size, 20);
        assert_eq!(timing.countdown_max_age_ms, 500);
        assert_eq!(timing.sound_max_age_ms, 3000);
        assert_eq!(timing.speech_max_age_ms, 10000);
        assert_eq!(timing.min_audio_gap_ms, 50);
        assert!(timing.aggressive_cleanup);
        assert_eq!(timing.queue_pressure_threshold, 70);
    }

    #[test]
    fn test_timing_config_queue_pressure() {
        let timing = super::super::AudioTimingConfig::default();

        // 14 items = 70% of 20 max = at threshold
        assert!(timing.is_queue_under_pressure(14));
        // 15 items = 75% = over threshold
        assert!(timing.is_queue_under_pressure(15));
        // 13 items = 65% = under threshold
        assert!(!timing.is_queue_under_pressure(13));
        // Empty queue = not under pressure
        assert!(!timing.is_queue_under_pressure(0));
    }

    #[test]
    fn test_countdown_tone_has_short_expiration() {
        let item = AudioItem::countdown_tone(440, 100);
        assert_eq!(item.max_queue_time, Duration::from_millis(500));
        assert!(item.is_time_critical());
        assert_eq!(item.category, Some(AudioCategory::Countdown));
    }

    #[test]
    fn test_countdown_tone_with_custom_timing() {
        let item = AudioItem::countdown_tone_with_timing(440, 100, 200);
        assert_eq!(item.max_queue_time, Duration::from_millis(200));
        assert!(item.is_time_critical());
    }

    #[test]
    fn test_audio_item_expiration_check() {
        // Create item with very short max queue time
        let item = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_millis(1));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(5));

        assert!(item.is_expired());
        assert!(item.time_remaining().is_none());
    }

    #[test]
    fn test_audio_item_not_expired() {
        let item = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_secs(60));

        assert!(!item.is_expired());
        assert!(item.time_remaining().is_some());
        assert!(item.time_remaining().unwrap() > Duration::from_secs(59));
    }

    #[test]
    fn test_audio_item_age_tracking() {
        let item = AudioItem::tone(440, 100);
        std::thread::sleep(Duration::from_millis(10));

        let age = item.age_ms();
        assert!(age >= 10, "Expected age >= 10ms, got {}ms", age);
    }

    #[test]
    fn test_audio_item_type_description() {
        let speech = AudioItem::speech("Hello world");
        assert!(speech.type_description().contains("Speech"));
        assert!(speech.type_description().contains("Hello"));

        let sound = AudioItem::sound("beep");
        assert!(sound.type_description().contains("Sound"));
        assert!(sound.type_description().contains("beep"));

        let tone = AudioItem::tone(440, 100);
        assert!(tone.type_description().contains("Tone"));
        assert!(tone.type_description().contains("440Hz"));
    }

    #[test]
    fn test_queue_stats_initial() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        let stats = engine.get_queue_stats();
        assert_eq!(stats.item_count, 0);
        assert_eq!(stats.expired_count, 0);
        assert_eq!(stats.dropped_count, 0);
        assert!(!stats.under_pressure);
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_queue_stats_after_queue() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue some items
        engine.queue(AudioItem::speech("One"));
        engine.queue(AudioItem::speech("Two"));
        engine.queue(AudioItem::tone(440, 100).with_priority(AudioPriority::Low));

        let stats = engine.get_queue_stats();
        assert_eq!(stats.item_count, 3);
        assert_eq!(stats.low_priority_count, 1);
    }

    #[test]
    fn test_queue_size_method() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        assert_eq!(engine.queue_size(), 0);

        engine.queue(AudioItem::speech("One"));
        assert_eq!(engine.queue_size(), 1);

        engine.queue(AudioItem::speech("Two"));
        assert_eq!(engine.queue_size(), 2);
    }

    #[test]
    fn test_reset_queue_stats() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Simulate expired count by manually incrementing
        engine.expired_count.fetch_add(5, Ordering::Relaxed);
        engine.dropped_count.fetch_add(3, Ordering::Relaxed);

        let stats = engine.get_queue_stats();
        assert_eq!(stats.expired_count, 5);
        assert_eq!(stats.dropped_count, 3);

        // Reset
        engine.reset_queue_stats();

        let stats = engine.get_queue_stats();
        assert_eq!(stats.expired_count, 0);
        assert_eq!(stats.dropped_count, 0);
    }

    #[test]
    fn test_get_set_timing_config() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Get default config
        let timing = engine.get_timing_config();
        assert_eq!(timing.max_queue_size, 20);

        // Set new config
        let mut new_timing = timing;
        new_timing.max_queue_size = 50;
        new_timing.countdown_max_age_ms = 250;
        engine.set_timing_config(new_timing);

        // Verify change
        let updated = engine.get_timing_config();
        assert_eq!(updated.max_queue_size, 50);
        assert_eq!(updated.countdown_max_age_ms, 250);
    }

    #[test]
    fn test_cleanup_expired_items() {
        let config = AudioConfig::default();
        let engine = DefaultAudioEngine::new(config);

        // Queue an item with very short expiration
        let short_lived = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_millis(1));
        engine.queue(short_lived);

        // Queue a long-lived item
        engine.queue(AudioItem::speech("Long lived"));

        assert_eq!(engine.queue_size(), 2);

        // Wait for short-lived item to expire
        std::thread::sleep(Duration::from_millis(10));

        // Manual cleanup
        let cleaned = engine.cleanup_expired();
        assert_eq!(cleaned, 1, "Expected 1 expired item");
        assert_eq!(engine.queue_size(), 1, "Expected 1 remaining item");

        // Stats should reflect the expired item
        let stats = engine.get_queue_stats();
        assert!(stats.expired_count >= 1);
    }

    #[test]
    fn test_queue_drops_low_priority_when_full() {
        let mut config = AudioConfig::default();
        config.timing.max_queue_size = 3;
        let engine = DefaultAudioEngine::new(config);

        // Fill the queue
        engine.queue(AudioItem::speech("One"));
        engine.queue(AudioItem::speech("Two"));
        engine.queue(AudioItem::speech("Three"));

        assert_eq!(engine.queue_size(), 3);

        // Try to add a low-priority item when full
        engine.queue(AudioItem::tone(440, 100).with_priority(AudioPriority::Low));

        // Should have been dropped
        assert_eq!(engine.queue_size(), 3);
        let stats = engine.get_queue_stats();
        assert_eq!(stats.dropped_count, 1);
    }

    #[test]
    fn test_queue_accepts_high_priority_when_full() {
        let mut config = AudioConfig::default();
        config.timing.max_queue_size = 3;
        let engine = DefaultAudioEngine::new(config);

        // Fill the queue with low priority items
        engine.queue(AudioItem::tone(440, 100).with_priority(AudioPriority::Low));
        engine.queue(AudioItem::tone(440, 100).with_priority(AudioPriority::Low));
        engine.queue(AudioItem::tone(440, 100).with_priority(AudioPriority::Low));

        assert_eq!(engine.queue_size(), 3);

        // Add a high-priority item - should make room by dropping low priority
        engine.queue(AudioItem::speech("Important!").with_priority(AudioPriority::High));

        // Queue should still work - low priority items were dropped
        let stats = engine.get_queue_stats();
        assert!(stats.high_priority_count >= 1);
    }

    #[test]
    fn test_aggressive_cleanup_on_queue() {
        let mut config = AudioConfig::default();
        config.timing.aggressive_cleanup = true;
        let engine = DefaultAudioEngine::new(config);

        // Queue an item with very short expiration
        let short_lived = AudioItem::tone(440, 100).with_max_queue_time(Duration::from_millis(1));
        engine.queue(short_lived);

        // Wait for it to expire
        std::thread::sleep(Duration::from_millis(10));

        // Queue another item - this should trigger cleanup
        engine.queue(AudioItem::speech("Trigger cleanup"));

        // The expired item should have been cleaned up
        // Only the new item should remain
        assert_eq!(engine.queue_size(), 1);
    }

    #[test]
    fn test_queue_stats_status_string() {
        let stats = QueueStats::default();
        assert!(stats.status_string().contains("OK"));
        assert!(stats.is_healthy());

        let pressure_stats = QueueStats {
            under_pressure: true,
            dropped_count: 5,
            item_count: 15,
            ..Default::default()
        };
        assert!(pressure_stats.status_string().contains("PRESSURE"));
        assert!(!pressure_stats.is_healthy());

        let expired_stats = QueueStats {
            expired_count: 3,
            item_count: 5,
            ..Default::default()
        };
        assert!(expired_stats.status_string().contains("ACTIVE"));
        assert!(!expired_stats.is_healthy());
    }

    #[test]
    fn test_timing_config_max_queue_time_for_category() {
        let timing = super::super::AudioTimingConfig::default();

        let countdown_time = timing.max_queue_time_for_category(Some(AudioCategory::Countdown));
        assert_eq!(countdown_time, Duration::from_millis(500));

        let voice_time = timing.max_queue_time_for_category(Some(AudioCategory::Voice));
        assert_eq!(voice_time, Duration::from_millis(10000));

        let sound_time = timing.max_queue_time_for_category(Some(AudioCategory::SoundEffect));
        assert_eq!(sound_time, Duration::from_millis(3000));

        let no_category = timing.max_queue_time_for_category(None);
        assert_eq!(no_category, Duration::from_millis(3000));
    }

    #[test]
    fn test_is_time_critical() {
        let countdown = AudioItem::countdown_tone(440, 100);
        assert!(countdown.is_time_critical());

        let speech = AudioItem::speech("Hello");
        assert!(!speech.is_time_critical());

        let sound = AudioItem::sound("beep");
        assert!(!sound.is_time_critical());

        let tone = AudioItem::tone(440, 100);
        assert!(!tone.is_time_critical());
    }
}
