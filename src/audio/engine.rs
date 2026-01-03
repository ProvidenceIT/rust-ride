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

use super::backend::RodioAudioBackend;
use super::tts::TtsProvider;
use super::{
    AudioConfig, AudioError, AudioEvent, AudioItem, AudioPriority, AudioType,
    ThreadSafeTtsProvider,
};
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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

    /// Speak text using TTS
    fn speak(&self, text: &str)
        -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Play a tone
    fn play_tone(
        &self,
        frequency_hz: u32,
        duration_ms: u32,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Set master volume (0-100)
    fn set_volume(&self, volume: u8);

    /// Get current volume
    fn get_volume(&self) -> u8;

    /// Queue an audio item
    fn queue(&self, item: AudioItem);

    /// Check if currently playing
    fn is_playing(&self) -> bool;

    /// Stop current playback
    fn stop(&self);

    /// Subscribe to audio events
    fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent>;
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
            let elapsed = entry.item.queued_at.elapsed();
            if elapsed < entry.item.max_queue_time {
                return Some(entry.item);
            }
            // Item expired, try next
            tracing::debug!("Audio item expired after {:?}", elapsed);
        }

        None
    }

    /// Process the audio queue
    ///
    /// This method processes queued audio items in priority order. Higher-priority
    /// items are played first, and if a high-priority item is queued while a
    /// lower-priority item is playing, the current item will be interrupted.
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

            // Set current priority before playing
            *self.current_priority.lock().unwrap() = Some(item.priority);

            let result: Result<(), AudioError> = match &item.audio_type {
                AudioType::Speech { text } => self.speak_with_interrupt_check(text).await,
                AudioType::SoundEffect { name } => self.play_sound(name).await,
                AudioType::Tone {
                    frequency_hz,
                    duration_ms,
                } => self.play_tone(*frequency_hz, *duration_ms).await,
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

        // TODO: Actually play the sound using rodio
        // For now, just simulate a short delay
        tokio::time::sleep(Duration::from_millis(100)).await;

        *self.is_playing.lock().unwrap() = false;

        Ok(())
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

    fn set_volume(&self, volume: u8) {
        let clamped_volume = volume.min(100);
        {
            let mut config = self.config.lock().unwrap();
            config.volume = clamped_volume;
        }
        // Also update the audio backend volume (convert 0-100 to 0.0-1.0)
        self.audio_backend.set_volume(clamped_volume as f32 / 100.0);
    }

    fn get_volume(&self) -> u8 {
        self.config.lock().unwrap().volume
    }

    fn queue(&self, item: AudioItem) {
        let priority = item.priority;

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
}
