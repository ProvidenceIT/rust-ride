//! Audio Backend using Rodio
//!
//! Provides the `RodioAudioBackend` which manages rodio OutputStream and Sink
//! for playing sound files (WAV/MP3) and tones.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use rodio::source::SineWave;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use thiserror::Error;

/// Errors that can occur in the audio backend
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Audio device not available: {0}")]
    DeviceNotAvailable(String),

    #[error("Sound file not found: {0}")]
    SoundFileNotFound(String),

    #[error("Failed to decode audio file: {0}")]
    DecodeFailed(String),

    #[error("Playback failed: {0}")]
    PlaybackFailed(String),

    #[error("Audio backend not initialized")]
    NotInitialized,
}

/// Audio backend state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    /// Backend is not initialized
    Uninitialized,
    /// Backend is initialized and ready
    Ready,
    /// Backend initialization failed (no audio device)
    Failed,
    /// Backend is playing audio
    Playing,
}

/// Cached sound data for fast replay
#[derive(Clone)]
pub struct CachedSound {
    /// Raw audio samples (interleaved)
    samples: Arc<Vec<i16>>,
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u16,
}

impl CachedSound {
    /// Get the duration of the cached sound
    pub fn duration(&self) -> Duration {
        let total_samples = self.samples.len() as u64;
        let samples_per_second = self.sample_rate as u64 * self.channels as u64;
        Duration::from_millis((total_samples * 1000) / samples_per_second)
    }
}

/// Rodio-based audio backend for sound file playback
///
/// This backend manages the rodio OutputStream and provides methods for:
/// - Loading and playing WAV/MP3 files from disk
/// - Playing generated tones
/// - Volume control
/// - Sound caching for fast replay
pub struct RodioAudioBackend {
    /// The output stream (must be kept alive for audio playback)
    /// Wrapped in Option because initialization can fail
    stream: RwLock<Option<OutputStream>>,
    /// Handle to the output stream for creating sinks
    stream_handle: RwLock<Option<OutputStreamHandle>>,
    /// Current state of the backend
    state: RwLock<BackendState>,
    /// Sound file cache
    sound_cache: RwLock<HashMap<String, CachedSound>>,
    /// Base path for sound assets
    sounds_path: PathBuf,
    /// Master volume (0.0 - 1.0)
    volume: RwLock<f32>,
    /// Whether audio is muted
    muted: RwLock<bool>,
    /// Active sinks for stopping playback
    active_sinks: Mutex<Vec<Arc<Sink>>>,
}

impl Default for RodioAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RodioAudioBackend {
    /// Create a new audio backend with default settings
    pub fn new() -> Self {
        Self::with_sounds_path(PathBuf::from("assets/sounds"))
    }

    /// Create a new audio backend with a custom sounds directory
    pub fn with_sounds_path(sounds_path: PathBuf) -> Self {
        Self {
            stream: RwLock::new(None),
            stream_handle: RwLock::new(None),
            state: RwLock::new(BackendState::Uninitialized),
            sound_cache: RwLock::new(HashMap::new()),
            sounds_path,
            volume: RwLock::new(0.8),
            muted: RwLock::new(false),
            active_sinks: Mutex::new(Vec::new()),
        }
    }

    /// Initialize the audio backend
    ///
    /// Attempts to open the default audio output device. If no audio device
    /// is available, the backend enters a "failed" state but doesn't panic.
    pub fn initialize(&self) -> Result<(), BackendError> {
        tracing::info!("Initializing RodioAudioBackend");

        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                *self.stream.write().unwrap() = Some(stream);
                *self.stream_handle.write().unwrap() = Some(handle);
                *self.state.write().unwrap() = BackendState::Ready;
                tracing::info!("Audio backend initialized successfully");
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::warn!("Failed to initialize audio device: {}", error_msg);
                *self.state.write().unwrap() = BackendState::Failed;
                Err(BackendError::DeviceNotAvailable(error_msg))
            }
        }
    }

    /// Reinitialize the audio backend (for hot-plug support)
    ///
    /// This can be called to attempt to recover from a failed state
    /// when an audio device becomes available.
    pub fn reinitialize(&self) -> Result<(), BackendError> {
        // Clean up existing state
        self.cleanup_active_sinks();
        *self.stream_handle.write().unwrap() = None;
        *self.stream.write().unwrap() = None;

        // Try to initialize again
        self.initialize()
    }

    /// Check if the backend is ready for playback
    pub fn is_ready(&self) -> bool {
        *self.state.read().unwrap() == BackendState::Ready
    }

    /// Get the current state
    pub fn state(&self) -> BackendState {
        *self.state.read().unwrap()
    }

    /// Set the master volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        *self.volume.write().unwrap() = volume.clamp(0.0, 1.0);
    }

    /// Get the current master volume
    pub fn volume(&self) -> f32 {
        *self.volume.read().unwrap()
    }

    /// Set muted state
    pub fn set_muted(&self, muted: bool) {
        *self.muted.write().unwrap() = muted;
    }

    /// Check if audio is muted
    pub fn is_muted(&self) -> bool {
        *self.muted.read().unwrap()
    }

    /// Load a sound file from the sounds directory
    ///
    /// The name should be the base name without extension.
    /// This method will search for .wav and .mp3 files.
    pub fn load_sound(&self, name: &str) -> Result<CachedSound, BackendError> {
        // Check cache first
        {
            let cache = self.sound_cache.read().unwrap();
            if let Some(cached) = cache.get(name) {
                tracing::debug!("Sound '{}' loaded from cache", name);
                return Ok(cached.clone());
            }
        }

        // Find the sound file
        let file_path = self.find_sound_file(name)?;

        // Load and decode the file
        let cached = self.load_sound_from_path(&file_path)?;

        // Add to cache
        {
            let mut cache = self.sound_cache.write().unwrap();
            cache.insert(name.to_string(), cached.clone());
        }

        tracing::debug!("Sound '{}' loaded and cached from {:?}", name, file_path);
        Ok(cached)
    }

    /// Find a sound file by name, checking for common extensions
    fn find_sound_file(&self, name: &str) -> Result<PathBuf, BackendError> {
        let extensions = ["wav", "mp3"];

        for ext in &extensions {
            let path = self.sounds_path.join(format!("{}.{}", name, ext));
            if path.exists() {
                return Ok(path);
            }
        }

        // Also check if name already has extension
        let path_with_name = self.sounds_path.join(name);
        if path_with_name.exists() {
            return Ok(path_with_name);
        }

        Err(BackendError::SoundFileNotFound(format!(
            "Sound '{}' not found in {:?}",
            name, self.sounds_path
        )))
    }

    /// Load sound data from a specific file path
    fn load_sound_from_path(&self, path: &Path) -> Result<CachedSound, BackendError> {
        let file = File::open(path).map_err(|e| {
            BackendError::SoundFileNotFound(format!("{}: {}", path.display(), e))
        })?;

        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader).map_err(|e| {
            BackendError::DecodeFailed(format!("{}: {}", path.display(), e))
        })?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();

        // Collect all samples
        let samples: Vec<i16> = decoder.collect();

        Ok(CachedSound {
            samples: Arc::new(samples),
            sample_rate,
            channels,
        })
    }

    /// Play a sound by name
    ///
    /// Returns immediately - playback happens on a separate thread.
    /// Returns the approximate duration of the sound if successful.
    pub fn play_sound(&self, name: &str) -> Result<Duration, BackendError> {
        self.play_sound_with_volume(name, None)
    }

    /// Play a sound by name with a specific volume override
    ///
    /// If volume_override is Some, uses that volume instead of the global master volume.
    /// Volume is in the range 0.0 - 1.0.
    pub fn play_sound_with_volume(
        &self,
        name: &str,
        volume_override: Option<f32>,
    ) -> Result<Duration, BackendError> {
        if self.is_muted() {
            return Ok(Duration::ZERO);
        }

        if !self.is_ready() {
            // Try to reinitialize if we're in a failed state
            if *self.state.read().unwrap() == BackendState::Failed {
                if let Err(e) = self.reinitialize() {
                    tracing::debug!("Reinitialize failed, skipping sound: {}", e);
                    return Ok(Duration::ZERO);
                }
            } else {
                return Err(BackendError::NotInitialized);
            }
        }

        let cached = self.load_sound(name)?;
        let duration = cached.duration();

        self.play_cached_sound_with_volume(&cached, volume_override)?;

        Ok(duration)
    }

    /// Play a cached sound using the default master volume
    fn play_cached_sound(&self, cached: &CachedSound) -> Result<(), BackendError> {
        self.play_cached_sound_with_volume(cached, None)
    }

    /// Play a cached sound with optional volume override
    ///
    /// If volume_override is Some, uses that volume instead of the global master volume.
    fn play_cached_sound_with_volume(
        &self,
        cached: &CachedSound,
        volume_override: Option<f32>,
    ) -> Result<(), BackendError> {
        let handle_guard = self.stream_handle.read().unwrap();
        let handle = handle_guard
            .as_ref()
            .ok_or(BackendError::NotInitialized)?;

        let sink = Sink::try_new(handle)
            .map_err(|e| BackendError::PlaybackFailed(e.to_string()))?;

        // Create a source from the cached samples
        let source = CachedSoundSource::new(cached.clone());
        let volume = volume_override.unwrap_or_else(|| self.volume());

        sink.append(source.amplify(volume));

        // Store sink to keep it alive and allow stopping
        let sink = Arc::new(sink);
        {
            let mut sinks = self.active_sinks.lock().unwrap();
            sinks.push(Arc::clone(&sink));
        }

        // Spawn a task to clean up the sink when done
        let sink_clone = Arc::clone(&sink);
        std::thread::spawn(move || {
            sink_clone.sleep_until_end();
        });

        Ok(())
    }

    /// Play a sound file directly from a path
    pub fn play_file(&self, path: &Path) -> Result<Duration, BackendError> {
        if self.is_muted() {
            return Ok(Duration::ZERO);
        }

        if !self.is_ready() {
            return Err(BackendError::NotInitialized);
        }

        let cached = self.load_sound_from_path(path)?;
        let duration = cached.duration();

        self.play_cached_sound(&cached)?;

        Ok(duration)
    }

    /// Play a tone (sine wave)
    ///
    /// Returns immediately - playback happens on a separate thread.
    pub fn play_tone(&self, frequency_hz: f32, duration: Duration) -> Result<(), BackendError> {
        self.play_tone_with_volume(frequency_hz, duration, None)
    }

    /// Play a tone (sine wave) with a specific volume override
    ///
    /// If volume_override is Some, uses that volume instead of the global master volume.
    /// Volume is in the range 0.0 - 1.0.
    pub fn play_tone_with_volume(
        &self,
        frequency_hz: f32,
        duration: Duration,
        volume_override: Option<f32>,
    ) -> Result<(), BackendError> {
        if self.is_muted() || frequency_hz <= 0.0 {
            return Ok(());
        }

        if !self.is_ready() {
            // Try to reinitialize if we're in a failed state
            if *self.state.read().unwrap() == BackendState::Failed {
                if let Err(e) = self.reinitialize() {
                    tracing::debug!("Reinitialize failed, skipping tone: {}", e);
                    return Ok(());
                }
            } else {
                return Err(BackendError::NotInitialized);
            }
        }

        let handle_guard = self.stream_handle.read().unwrap();
        let handle = handle_guard
            .as_ref()
            .ok_or(BackendError::NotInitialized)?;

        let sink = Sink::try_new(handle)
            .map_err(|e| BackendError::PlaybackFailed(e.to_string()))?;

        let volume = volume_override.unwrap_or_else(|| self.volume());
        let source = SineWave::new(frequency_hz)
            .take_duration(duration)
            .amplify(volume);

        sink.append(source);

        // Keep sink alive until done
        std::thread::spawn(move || {
            sink.sleep_until_end();
        });

        Ok(())
    }

    /// Stop all currently playing audio
    pub fn stop_all(&self) {
        self.cleanup_active_sinks();
    }

    /// Clean up completed sinks and stop active ones
    fn cleanup_active_sinks(&self) {
        let mut sinks = self.active_sinks.lock().unwrap();
        for sink in sinks.iter() {
            sink.stop();
        }
        sinks.clear();
    }

    /// Clear the sound cache
    pub fn clear_cache(&self) {
        self.sound_cache.write().unwrap().clear();
        tracing::debug!("Sound cache cleared");
    }

    /// Get the number of cached sounds
    pub fn cache_size(&self) -> usize {
        self.sound_cache.read().unwrap().len()
    }

    /// Check if a sound is cached
    pub fn is_cached(&self, name: &str) -> bool {
        self.sound_cache.read().unwrap().contains_key(name)
    }
}

/// Source that plays cached sound samples
struct CachedSoundSource {
    samples: Arc<Vec<i16>>,
    sample_rate: u32,
    channels: u16,
    position: usize,
}

impl CachedSoundSource {
    fn new(cached: CachedSound) -> Self {
        Self {
            samples: cached.samples,
            sample_rate: cached.sample_rate,
            channels: cached.channels,
            position: 0,
        }
    }
}

impl Iterator for CachedSoundSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for CachedSoundSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let total_samples = self.samples.len() as u64;
        let samples_per_second = self.sample_rate as u64 * self.channels as u64;
        Some(Duration::from_millis((total_samples * 1000) / samples_per_second))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = RodioAudioBackend::new();
        assert_eq!(backend.state(), BackendState::Uninitialized);
        assert!(!backend.is_ready());
        assert_eq!(backend.volume(), 0.8);
        assert!(!backend.is_muted());
    }

    #[test]
    fn test_backend_with_custom_path() {
        let backend = RodioAudioBackend::with_sounds_path(PathBuf::from("custom/sounds"));
        assert_eq!(backend.sounds_path, PathBuf::from("custom/sounds"));
    }

    #[test]
    fn test_volume_clamping() {
        let backend = RodioAudioBackend::new();

        backend.set_volume(1.5);
        assert_eq!(backend.volume(), 1.0);

        backend.set_volume(-0.5);
        assert_eq!(backend.volume(), 0.0);

        backend.set_volume(0.5);
        assert_eq!(backend.volume(), 0.5);
    }

    #[test]
    fn test_mute_state() {
        let backend = RodioAudioBackend::new();

        assert!(!backend.is_muted());

        backend.set_muted(true);
        assert!(backend.is_muted());

        backend.set_muted(false);
        assert!(!backend.is_muted());
    }

    #[test]
    fn test_sound_not_found() {
        let backend = RodioAudioBackend::new();
        let result = backend.find_sound_file("nonexistent_sound");
        assert!(matches!(result, Err(BackendError::SoundFileNotFound(_))));
    }

    #[test]
    fn test_cache_operations() {
        let backend = RodioAudioBackend::new();
        assert_eq!(backend.cache_size(), 0);
        assert!(!backend.is_cached("test"));
    }

    #[test]
    fn test_cached_sound_duration() {
        let cached = CachedSound {
            samples: Arc::new(vec![0i16; 44100]), // 1 second at 44.1kHz mono
            sample_rate: 44100,
            channels: 1,
        };
        let duration = cached.duration();
        // Should be approximately 1 second
        assert!(duration.as_millis() >= 990 && duration.as_millis() <= 1010);
    }

    #[test]
    fn test_cached_sound_source_iteration() {
        let samples = vec![1i16, 2, 3, 4, 5];
        let cached = CachedSound {
            samples: Arc::new(samples.clone()),
            sample_rate: 44100,
            channels: 1,
        };

        let source = CachedSoundSource::new(cached);
        let collected: Vec<i16> = source.collect();
        assert_eq!(collected, samples);
    }
}
