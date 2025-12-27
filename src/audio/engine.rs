//! Audio Engine
//!
//! Core audio playback engine using rodio.

use super::{AudioConfig, AudioError, AudioEvent, AudioItem, AudioPriority, AudioType};
use std::collections::BinaryHeap;
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
}

impl DefaultAudioEngine {
    /// Create a new audio engine with the given configuration
    pub fn new(config: AudioConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            config: Arc::new(Mutex::new(config)),
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            sequence_counter: Arc::new(Mutex::new(0)),
            is_playing: Arc::new(Mutex::new(false)),
            event_tx,
        }
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
    pub async fn process_queue(&self) {
        while let Some(item) = self.pop_next() {
            let result: Result<(), AudioError> = match &item.audio_type {
                AudioType::Speech { text } => self.speak(text).await,
                AudioType::SoundEffect { name } => self.play_sound(name).await,
                AudioType::Tone {
                    frequency_hz,
                    duration_ms,
                } => self.play_tone(*frequency_hz, *duration_ms).await,
            };

            if let Err(e) = result {
                let _ = self.event_tx.send(AudioEvent::Error {
                    message: e.to_string(),
                });
            }
        }
    }
}

impl AudioEngine for DefaultAudioEngine {
    fn initialize(&self) -> Result<(), AudioError> {
        tracing::info!("Initializing audio engine");

        // TODO: Initialize rodio output stream
        // let (_stream, stream_handle) = rodio::OutputStream::try_default()
        //     .map_err(|e| AudioError::DeviceNotAvailable)?;

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

        *self.is_playing.lock().unwrap() = true;

        let _ = self.event_tx.send(AudioEvent::SpeechStarted {
            text: text.to_string(),
        });

        // TODO: Use TTS crate to speak
        // For now, just simulate based on text length
        let duration = Duration::from_millis((text.len() as u64 * 50).min(5000));
        tokio::time::sleep(duration).await;

        let _ = self.event_tx.send(AudioEvent::SpeechCompleted);

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }

    async fn play_tone(&self, _frequency_hz: u32, duration_ms: u32) -> Result<(), AudioError> {
        {
            let config = self.config.lock().unwrap();
            if !config.enabled {
                return Ok(());
            }
        }

        *self.is_playing.lock().unwrap() = true;

        // TODO: Generate and play tone using rodio
        tokio::time::sleep(Duration::from_millis(duration_ms as u64)).await;

        *self.is_playing.lock().unwrap() = false;

        Ok(())
    }

    fn set_volume(&self, volume: u8) {
        let mut config = self.config.lock().unwrap();
        config.volume = volume.min(100);
    }

    fn get_volume(&self) -> u8 {
        self.config.lock().unwrap().volume
    }

    fn queue(&self, item: AudioItem) {
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
        // TODO: Actually stop playback
        *self.is_playing.lock().unwrap() = false;
    }

    fn subscribe_events(&self) -> broadcast::Receiver<AudioEvent> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
