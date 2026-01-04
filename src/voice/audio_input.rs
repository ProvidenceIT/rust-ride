//! Audio Input Capture Module
//!
//! Cross-platform microphone audio capture using cpal for voice recognition.
//!
//! ## Features
//!
//! - **Cross-Platform**: Works on Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA/PulseAudio)
//! - **Vosk-Compatible**: Configured for 16kHz mono audio required by Vosk speech recognition
//! - **Ring Buffer**: Thread-safe ring buffer for audio sample storage
//! - **Non-Blocking**: Audio capture runs on a dedicated thread
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::audio_input::{AudioInputCapture, AudioInputConfig};
//!
//! let config = AudioInputConfig::default(); // 16kHz mono
//! let mut capture = AudioInputCapture::new(config)?;
//!
//! capture.start()?;
//!
//! // Read samples for recognition
//! let samples = capture.read_samples(4096);
//!
//! capture.stop()?;
//! ```

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use thiserror::Error;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig};

/// Default sample rate for Vosk speech recognition (16kHz)
pub const VOSK_SAMPLE_RATE: u32 = 16000;

/// Default number of audio channels (mono for Vosk)
pub const VOSK_CHANNELS: u16 = 1;

/// Default ring buffer size (4 seconds of audio at 16kHz)
pub const DEFAULT_BUFFER_SIZE: usize = VOSK_SAMPLE_RATE as usize * 4;

/// Errors that can occur during audio input operations
#[derive(Debug, Error)]
pub enum AudioInputError {
    #[error("No audio input device available")]
    NoDevice,

    #[error("Failed to get default input config: {0}")]
    ConfigError(String),

    #[error("Unsupported sample format: {0:?}")]
    UnsupportedFormat(SampleFormat),

    #[error("Failed to build input stream: {0}")]
    StreamBuildError(String),

    #[error("Failed to start stream: {0}")]
    StreamStartError(String),

    #[error("Failed to stop stream: {0}")]
    StreamStopError(String),

    #[error("Audio capture not running")]
    NotRunning,

    #[error("Audio capture already running")]
    AlreadyRunning,

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Platform-specific error: {0}")]
    PlatformError(String),
}

/// Audio input device information
#[derive(Debug, Clone)]
pub struct AudioInputDeviceInfo {
    /// Device name
    pub name: String,
    /// Whether this is the default input device
    pub is_default: bool,
    /// Supported sample rates
    pub sample_rates: Vec<u32>,
    /// Maximum input channels
    pub max_channels: u16,
}

/// Configuration for audio input capture
#[derive(Debug, Clone)]
pub struct AudioInputConfig {
    /// Sample rate in Hz (default: 16000 for Vosk)
    pub sample_rate: u32,
    /// Number of audio channels (default: 1 mono for Vosk)
    pub channels: u16,
    /// Ring buffer size in samples
    pub buffer_size: usize,
    /// Device name to use (None for default)
    pub device_name: Option<String>,
}

impl Default for AudioInputConfig {
    fn default() -> Self {
        Self {
            sample_rate: VOSK_SAMPLE_RATE,
            channels: VOSK_CHANNELS,
            buffer_size: DEFAULT_BUFFER_SIZE,
            device_name: None,
        }
    }
}

impl AudioInputConfig {
    /// Create a new configuration with Vosk-compatible defaults
    pub fn for_vosk() -> Self {
        Self::default()
    }

    /// Create a configuration with custom sample rate
    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// Create a configuration with custom buffer size
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Create a configuration with a specific device
    pub fn with_device(mut self, device_name: impl Into<String>) -> Self {
        self.device_name = Some(device_name.into());
        self
    }
}

/// Thread-safe ring buffer for audio samples
///
/// Stores audio samples in a circular buffer, automatically discarding
/// oldest samples when the buffer is full.
#[derive(Debug)]
pub struct AudioRingBuffer {
    /// The circular buffer storage
    buffer: Mutex<VecDeque<i16>>,
    /// Maximum buffer capacity
    capacity: usize,
    /// Total samples written (for statistics)
    samples_written: AtomicBool,
}

impl AudioRingBuffer {
    /// Create a new ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            samples_written: AtomicBool::new(false),
        }
    }

    /// Write samples to the buffer
    ///
    /// If the buffer would overflow, oldest samples are discarded.
    pub fn write(&self, samples: &[i16]) {
        let mut buffer = self.buffer.lock().unwrap();

        for &sample in samples {
            if buffer.len() >= self.capacity {
                buffer.pop_front();
            }
            buffer.push_back(sample);
        }

        self.samples_written.store(true, Ordering::Release);
    }

    /// Read up to `count` samples from the buffer
    ///
    /// Returns the samples read, which may be fewer than requested if
    /// the buffer doesn't have enough samples.
    pub fn read(&self, count: usize) -> Vec<i16> {
        let mut buffer = self.buffer.lock().unwrap();
        let actual_count = count.min(buffer.len());

        buffer.drain(..actual_count).collect()
    }

    /// Read all available samples from the buffer
    pub fn read_all(&self) -> Vec<i16> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.drain(..).collect()
    }

    /// Peek at samples without removing them
    ///
    /// Returns up to `count` samples from the buffer.
    pub fn peek(&self, count: usize) -> Vec<i16> {
        let buffer = self.buffer.lock().unwrap();
        let actual_count = count.min(buffer.len());

        buffer.iter().take(actual_count).copied().collect()
    }

    /// Get the number of samples currently in the buffer
    pub fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.lock().unwrap().is_empty()
    }

    /// Check if any samples have been written
    pub fn has_data(&self) -> bool {
        self.samples_written.load(Ordering::Acquire)
    }

    /// Clear all samples from the buffer
    pub fn clear(&self) {
        self.buffer.lock().unwrap().clear();
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the fill percentage (0.0 - 1.0)
    pub fn fill_ratio(&self) -> f32 {
        let len = self.len();
        len as f32 / self.capacity as f32
    }
}

/// State of the audio capture system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Not started
    Idle,
    /// Currently capturing audio
    Running,
    /// Stopped
    Stopped,
    /// Error occurred
    Error,
}

/// Audio input capture manager
///
/// Manages microphone audio capture using cpal, storing samples in a
/// ring buffer for consumption by the speech recognizer.
pub struct AudioInputCapture {
    /// Configuration
    config: AudioInputConfig,
    /// Current state
    state: RwLock<CaptureState>,
    /// Ring buffer for samples
    buffer: Arc<AudioRingBuffer>,
    /// Audio stream (None when not running)
    stream: Mutex<Option<Stream>>,
    /// Whether capture is active
    is_running: AtomicBool,
    /// Last error message
    last_error: RwLock<Option<String>>,
    /// Selected device info
    device_info: RwLock<Option<AudioInputDeviceInfo>>,
}

// Stream contains raw pointers, but we manage it carefully
// The stream is only accessed from one thread at a time via Mutex
unsafe impl Send for AudioInputCapture {}
unsafe impl Sync for AudioInputCapture {}

impl AudioInputCapture {
    /// Create a new audio input capture with the given configuration
    pub fn new(config: AudioInputConfig) -> Result<Self, AudioInputError> {
        let buffer = Arc::new(AudioRingBuffer::new(config.buffer_size));

        Ok(Self {
            config,
            state: RwLock::new(CaptureState::Idle),
            buffer,
            stream: Mutex::new(None),
            is_running: AtomicBool::new(false),
            last_error: RwLock::new(None),
            device_info: RwLock::new(None),
        })
    }

    /// Create a new audio input capture with Vosk-compatible defaults
    pub fn for_vosk() -> Result<Self, AudioInputError> {
        Self::new(AudioInputConfig::for_vosk())
    }

    /// Get the current configuration
    pub fn config(&self) -> &AudioInputConfig {
        &self.config
    }

    /// Get the current state
    pub fn state(&self) -> CaptureState {
        *self.state.read().unwrap()
    }

    /// Check if capture is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Acquire)
    }

    /// Get access to the audio buffer
    pub fn buffer(&self) -> Arc<AudioRingBuffer> {
        Arc::clone(&self.buffer)
    }

    /// Read samples from the buffer
    ///
    /// Returns the samples read, which may be fewer than requested.
    pub fn read_samples(&self, count: usize) -> Vec<i16> {
        self.buffer.read(count)
    }

    /// Read all available samples from the buffer
    pub fn read_all_samples(&self) -> Vec<i16> {
        self.buffer.read_all()
    }

    /// Get the number of samples available in the buffer
    pub fn available_samples(&self) -> usize {
        self.buffer.len()
    }

    /// Get the last error message, if any
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().unwrap().clone()
    }

    /// Get information about the selected device
    pub fn device_info(&self) -> Option<AudioInputDeviceInfo> {
        self.device_info.read().unwrap().clone()
    }

    /// List available input devices
    pub fn list_devices() -> Result<Vec<AudioInputDeviceInfo>, AudioInputError> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device.as_ref().and_then(|d| d.name().ok());

        let devices = host
            .input_devices()
            .map_err(|e| AudioInputError::DeviceError(e.to_string()))?;

        let mut result = Vec::new();

        for device in devices {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let is_default = default_name.as_ref().map(|n| n == &name).unwrap_or(false);

            // Get supported configs to determine sample rates and channels
            let (sample_rates, max_channels) =
                if let Ok(configs) = device.supported_input_configs() {
                    let mut rates = Vec::new();
                    let mut max_ch = 1u16;

                    for config in configs {
                        // Add common sample rates that fall within the supported range
                        let min_rate = config.min_sample_rate().0;
                        let max_rate = config.max_sample_rate().0;

                        for &rate in &[8000, 16000, 22050, 44100, 48000, 96000] {
                            if rate >= min_rate && rate <= max_rate && !rates.contains(&rate) {
                                rates.push(rate);
                            }
                        }

                        max_ch = max_ch.max(config.channels());
                    }

                    rates.sort();
                    (rates, max_ch)
                } else {
                    (vec![16000, 44100, 48000], 2)
                };

            result.push(AudioInputDeviceInfo {
                name,
                is_default,
                sample_rates,
                max_channels,
            });
        }

        Ok(result)
    }

    /// Get the default input device
    fn get_device(&self) -> Result<Device, AudioInputError> {
        let host = cpal::default_host();

        match &self.config.device_name {
            Some(name) => {
                // Find device by name
                let devices = host
                    .input_devices()
                    .map_err(|e| AudioInputError::DeviceError(e.to_string()))?;

                for device in devices {
                    if device.name().ok().as_ref() == Some(name) {
                        return Ok(device);
                    }
                }

                Err(AudioInputError::DeviceError(format!(
                    "Device not found: {}",
                    name
                )))
            }
            None => {
                // Use default device
                host.default_input_device().ok_or(AudioInputError::NoDevice)
            }
        }
    }

    /// Start audio capture
    ///
    /// Begins capturing audio from the configured input device.
    pub fn start(&self) -> Result<(), AudioInputError> {
        if self.is_running() {
            return Err(AudioInputError::AlreadyRunning);
        }

        let device = self.get_device()?;

        // Store device info
        {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let mut info = self.device_info.write().unwrap();
            *info = Some(AudioInputDeviceInfo {
                name,
                is_default: true,
                sample_rates: vec![self.config.sample_rate],
                max_channels: self.config.channels,
            });
        }

        // Get supported configuration
        let supported_config = device
            .supported_input_configs()
            .map_err(|e| AudioInputError::ConfigError(e.to_string()))?
            .find(|c| {
                c.channels() >= self.config.channels
                    && c.min_sample_rate().0 <= self.config.sample_rate
                    && c.max_sample_rate().0 >= self.config.sample_rate
            })
            .ok_or_else(|| {
                AudioInputError::ConfigError(format!(
                    "No supported config for {}Hz {} channel(s)",
                    self.config.sample_rate, self.config.channels
                ))
            })?;

        let sample_format = supported_config.sample_format();
        let stream_config = StreamConfig {
            channels: self.config.channels,
            sample_rate: SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Clear buffer before starting
        self.buffer.clear();

        // Build stream based on sample format
        let buffer = Arc::clone(&self.buffer);
        let channels = self.config.channels as usize;

        let stream = match sample_format {
            SampleFormat::I16 => self.build_stream_i16(&device, &stream_config, buffer, channels)?,
            SampleFormat::F32 => self.build_stream_f32(&device, &stream_config, buffer, channels)?,
            SampleFormat::I8 => self.build_stream_i8(&device, &stream_config, buffer, channels)?,
            SampleFormat::I32 => self.build_stream_i32(&device, &stream_config, buffer, channels)?,
            SampleFormat::U8 => self.build_stream_u8(&device, &stream_config, buffer, channels)?,
            SampleFormat::U16 => self.build_stream_u16(&device, &stream_config, buffer, channels)?,
            SampleFormat::U32 => self.build_stream_u32(&device, &stream_config, buffer, channels)?,
            SampleFormat::F64 => self.build_stream_f64(&device, &stream_config, buffer, channels)?,
            SampleFormat::I64 | SampleFormat::U64 => {
                return Err(AudioInputError::UnsupportedFormat(sample_format));
            }
            _ => {
                return Err(AudioInputError::UnsupportedFormat(sample_format));
            }
        };

        // Start the stream
        stream
            .play()
            .map_err(|e| AudioInputError::StreamStartError(e.to_string()))?;

        // Store stream and update state
        *self.stream.lock().unwrap() = Some(stream);
        *self.state.write().unwrap() = CaptureState::Running;
        self.is_running.store(true, Ordering::Release);
        *self.last_error.write().unwrap() = None;

        tracing::info!(
            "Audio input started: {}Hz, {} channel(s)",
            self.config.sample_rate,
            self.config.channels
        );

        Ok(())
    }

    /// Build stream for i16 sample format
    fn build_stream_i16(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    // Convert to mono if needed
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                                (sum / channels as i32) as i16
                            })
                            .collect()
                    } else {
                        data.to_vec()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for f32 sample format
    fn build_stream_f32(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 to i16 and to mono if needed
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: f32 = chunk.iter().sum();
                                let avg = sum / channels as f32;
                                (avg * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32)
                                    as i16
                            })
                            .collect()
                    } else {
                        data.iter()
                            .map(|&s| {
                                (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
                            })
                            .collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for i8 sample format
    fn build_stream_i8(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[i8], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                                ((sum / channels as i32) as i16) << 8
                            })
                            .collect()
                    } else {
                        data.iter().map(|&s| (s as i16) << 8).collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for i32 sample format
    fn build_stream_i32(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i64 = chunk.iter().map(|&s| s as i64).sum();
                                ((sum / channels as i64) >> 16) as i16
                            })
                            .collect()
                    } else {
                        data.iter().map(|&s| (s >> 16) as i16).collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for u8 sample format
    fn build_stream_u8(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[u8], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i32 = chunk.iter().map(|&s| (s as i32) - 128).sum();
                                ((sum / channels as i32) as i16) << 8
                            })
                            .collect()
                    } else {
                        data.iter()
                            .map(|&s| (((s as i16) - 128) << 8))
                            .collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for u16 sample format
    fn build_stream_u16(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i32 = chunk.iter().map(|&s| (s as i32) - 32768).sum();
                                (sum / channels as i32) as i16
                            })
                            .collect()
                    } else {
                        data.iter().map(|&s| (s as i32 - 32768) as i16).collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for u32 sample format
    fn build_stream_u32(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[u32], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: i64 =
                                    chunk.iter().map(|&s| (s as i64) - 2147483648).sum();
                                ((sum / channels as i64) >> 16) as i16
                            })
                            .collect()
                    } else {
                        data.iter()
                            .map(|&s| (((s as i64) - 2147483648) >> 16) as i16)
                            .collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Build stream for f64 sample format
    fn build_stream_f64(
        &self,
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<AudioRingBuffer>,
        channels: usize,
    ) -> Result<Stream, AudioInputError> {
        let err_fn = |err| {
            tracing::error!("Audio input stream error: {}", err);
        };

        device
            .build_input_stream(
                config,
                move |data: &[f64], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                let sum: f64 = chunk.iter().sum();
                                let avg = sum / channels as f64;
                                (avg * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64)
                                    as i16
                            })
                            .collect()
                    } else {
                        data.iter()
                            .map(|&s| {
                                (s * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16
                            })
                            .collect()
                    };
                    buffer.write(&samples);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioInputError::StreamBuildError(e.to_string()))
    }

    /// Stop audio capture
    pub fn stop(&self) -> Result<(), AudioInputError> {
        if !self.is_running() {
            return Err(AudioInputError::NotRunning);
        }

        // Take and drop the stream to stop it
        let mut stream_guard = self.stream.lock().unwrap();
        if let Some(stream) = stream_guard.take() {
            if let Err(e) = stream.pause() {
                tracing::warn!("Error pausing stream: {}", e);
            }
            // Stream is dropped here, which stops it
        }

        *self.state.write().unwrap() = CaptureState::Stopped;
        self.is_running.store(false, Ordering::Release);

        tracing::info!("Audio input stopped");

        Ok(())
    }

    /// Pause audio capture (keeps stream alive but paused)
    pub fn pause(&self) -> Result<(), AudioInputError> {
        if !self.is_running() {
            return Err(AudioInputError::NotRunning);
        }

        let stream_guard = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream
                .pause()
                .map_err(|e| AudioInputError::StreamStopError(e.to_string()))?;
        }

        self.is_running.store(false, Ordering::Release);

        Ok(())
    }

    /// Resume paused audio capture
    pub fn resume(&self) -> Result<(), AudioInputError> {
        let stream_guard = self.stream.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream
                .play()
                .map_err(|e| AudioInputError::StreamStartError(e.to_string()))?;
            self.is_running.store(true, Ordering::Release);
            Ok(())
        } else {
            Err(AudioInputError::NotRunning)
        }
    }

    /// Get duration of audio currently in the buffer
    pub fn buffered_duration(&self) -> std::time::Duration {
        let samples = self.buffer.len();
        let sample_rate = self.config.sample_rate as u64;
        std::time::Duration::from_millis((samples as u64 * 1000) / sample_rate)
    }
}

impl Drop for AudioInputCapture {
    fn drop(&mut self) {
        if self.is_running() {
            let _ = self.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = AudioInputConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 64000);
        assert!(config.device_name.is_none());
    }

    #[test]
    fn test_config_for_vosk() {
        let config = AudioInputConfig::for_vosk();
        assert_eq!(config.sample_rate, VOSK_SAMPLE_RATE);
        assert_eq!(config.channels, VOSK_CHANNELS);
    }

    #[test]
    fn test_config_builder() {
        let config = AudioInputConfig::default()
            .with_sample_rate(48000)
            .with_buffer_size(96000)
            .with_device("Test Device".to_string());

        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.buffer_size, 96000);
        assert_eq!(config.device_name, Some("Test Device".to_string()));
    }

    #[test]
    fn test_ring_buffer_creation() {
        let buffer = AudioRingBuffer::new(1000);
        assert_eq!(buffer.capacity(), 1000);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert!(!buffer.has_data());
    }

    #[test]
    fn test_ring_buffer_write_read() {
        let buffer = AudioRingBuffer::new(100);

        // Write some samples
        buffer.write(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer.len(), 5);
        assert!(buffer.has_data());

        // Read some samples
        let samples = buffer.read(3);
        assert_eq!(samples, vec![1, 2, 3]);
        assert_eq!(buffer.len(), 2);

        // Read remaining
        let samples = buffer.read_all();
        assert_eq!(samples, vec![4, 5]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let buffer = AudioRingBuffer::new(5);

        // Write more than capacity
        buffer.write(&[1, 2, 3, 4, 5, 6, 7]);

        // Should only have last 5 samples
        assert_eq!(buffer.len(), 5);
        let samples = buffer.read_all();
        assert_eq!(samples, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_ring_buffer_peek() {
        let buffer = AudioRingBuffer::new(100);
        buffer.write(&[1, 2, 3, 4, 5]);

        // Peek should not remove samples
        let peeked = buffer.peek(3);
        assert_eq!(peeked, vec![1, 2, 3]);
        assert_eq!(buffer.len(), 5);
    }

    #[test]
    fn test_ring_buffer_clear() {
        let buffer = AudioRingBuffer::new(100);
        buffer.write(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer.len(), 5);

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_fill_ratio() {
        let buffer = AudioRingBuffer::new(100);
        assert_eq!(buffer.fill_ratio(), 0.0);

        buffer.write(&[0i16; 50]);
        assert!((buffer.fill_ratio() - 0.5).abs() < 0.01);

        buffer.write(&[0i16; 50]);
        assert!((buffer.fill_ratio() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_capture_creation() {
        let config = AudioInputConfig::default();
        let capture = AudioInputCapture::new(config);
        assert!(capture.is_ok());

        let capture = capture.unwrap();
        assert_eq!(capture.state(), CaptureState::Idle);
        assert!(!capture.is_running());
        assert!(capture.last_error().is_none());
    }

    #[test]
    fn test_capture_for_vosk() {
        let capture = AudioInputCapture::for_vosk();
        assert!(capture.is_ok());

        let capture = capture.unwrap();
        assert_eq!(capture.config().sample_rate, VOSK_SAMPLE_RATE);
        assert_eq!(capture.config().channels, VOSK_CHANNELS);
    }

    #[test]
    fn test_buffered_duration() {
        let config = AudioInputConfig::default(); // 16kHz
        let capture = AudioInputCapture::new(config).unwrap();

        // Write 16000 samples (1 second at 16kHz)
        capture.buffer.write(&[0i16; 16000]);

        let duration = capture.buffered_duration();
        assert!((duration.as_millis() as i64 - 1000).abs() < 10);
    }

    #[test]
    fn test_stop_when_not_running() {
        let capture = AudioInputCapture::for_vosk().unwrap();
        let result = capture.stop();
        assert!(matches!(result, Err(AudioInputError::NotRunning)));
    }

    // Note: Tests that require actual audio hardware are skipped in CI
    // They can be run locally with: cargo test --features voice-control -- --ignored

    #[test]
    #[ignore] // Requires audio hardware
    fn test_list_devices() {
        let devices = AudioInputCapture::list_devices();
        assert!(devices.is_ok());

        let devices = devices.unwrap();
        // Most systems have at least one input device
        // but this might fail in headless CI environments
        if !devices.is_empty() {
            let default_count = devices.iter().filter(|d| d.is_default).count();
            assert!(default_count <= 1, "At most one default device expected");
        }
    }

    #[test]
    #[ignore] // Requires audio hardware
    fn test_start_stop_capture() {
        let capture = AudioInputCapture::for_vosk().unwrap();

        // Start capture
        let result = capture.start();
        if result.is_err() {
            // Skip if no audio device available
            return;
        }

        assert!(capture.is_running());
        assert_eq!(capture.state(), CaptureState::Running);

        // Let it capture for a moment
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Stop capture
        let result = capture.stop();
        assert!(result.is_ok());
        assert!(!capture.is_running());
        assert_eq!(capture.state(), CaptureState::Stopped);
    }

    #[test]
    #[ignore] // Requires audio hardware
    fn test_start_when_already_running() {
        let capture = AudioInputCapture::for_vosk().unwrap();

        if capture.start().is_err() {
            return; // Skip if no audio device
        }

        // Try to start again
        let result = capture.start();
        assert!(matches!(result, Err(AudioInputError::AlreadyRunning)));

        let _ = capture.stop();
    }

    // ==========================================================================
    // Additional coverage tests
    // ==========================================================================

    #[test]
    fn test_audio_input_error_display() {
        // NoDevice
        let err = AudioInputError::NoDevice;
        assert!(err.to_string().contains("No audio input device"));

        // ConfigError
        let err = AudioInputError::ConfigError("Invalid config".to_string());
        assert!(err.to_string().contains("Failed to get default input config"));
        assert!(err.to_string().contains("Invalid config"));

        // UnsupportedFormat
        let err = AudioInputError::UnsupportedFormat(SampleFormat::I64);
        assert!(err.to_string().contains("Unsupported sample format"));

        // StreamBuildError
        let err = AudioInputError::StreamBuildError("Build failed".to_string());
        assert!(err.to_string().contains("Failed to build input stream"));

        // StreamStartError
        let err = AudioInputError::StreamStartError("Start failed".to_string());
        assert!(err.to_string().contains("Failed to start stream"));

        // StreamStopError
        let err = AudioInputError::StreamStopError("Stop failed".to_string());
        assert!(err.to_string().contains("Failed to stop stream"));

        // NotRunning
        let err = AudioInputError::NotRunning;
        assert!(err.to_string().contains("not running"));

        // AlreadyRunning
        let err = AudioInputError::AlreadyRunning;
        assert!(err.to_string().contains("already running"));

        // DeviceError
        let err = AudioInputError::DeviceError("Device gone".to_string());
        assert!(err.to_string().contains("Device error"));

        // PlatformError
        let err = AudioInputError::PlatformError("Platform issue".to_string());
        assert!(err.to_string().contains("Platform-specific error"));
    }

    #[test]
    fn test_audio_input_device_info_clone_and_debug() {
        let info = AudioInputDeviceInfo {
            name: "Test Device".to_string(),
            is_default: true,
            sample_rates: vec![16000, 44100, 48000],
            max_channels: 2,
        };

        // Clone
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.is_default, cloned.is_default);
        assert_eq!(info.sample_rates, cloned.sample_rates);
        assert_eq!(info.max_channels, cloned.max_channels);

        // Debug
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("Test Device"));
        assert!(debug_str.contains("16000"));
    }

    #[test]
    fn test_audio_input_config_clone_and_debug() {
        let config = AudioInputConfig::default();
        let cloned = config.clone();
        assert_eq!(config.sample_rate, cloned.sample_rate);
        assert_eq!(config.channels, cloned.channels);
        assert_eq!(config.buffer_size, cloned.buffer_size);
        assert_eq!(config.device_name, cloned.device_name);

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("16000"));
    }

    #[test]
    fn test_capture_state_enum() {
        assert_eq!(CaptureState::Idle, CaptureState::Idle);
        assert_ne!(CaptureState::Idle, CaptureState::Running);
        assert_ne!(CaptureState::Running, CaptureState::Stopped);
        assert_ne!(CaptureState::Stopped, CaptureState::Error);

        // Copy trait
        let state = CaptureState::Running;
        let copied = state;
        assert_eq!(state, copied);

        // Clone trait
        let cloned = state.clone();
        assert_eq!(state, cloned);

        // Debug trait
        let debug_str = format!("{:?}", CaptureState::Error);
        assert!(debug_str.contains("Error"));
    }

    #[test]
    fn test_ring_buffer_read_more_than_available() {
        let buffer = AudioRingBuffer::new(100);
        buffer.write(&[1, 2, 3, 4, 5]);

        // Request more than available
        let samples = buffer.read(10);
        assert_eq!(samples, vec![1, 2, 3, 4, 5]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_peek_more_than_available() {
        let buffer = AudioRingBuffer::new(100);
        buffer.write(&[1, 2, 3]);

        // Peek more than available
        let peeked = buffer.peek(10);
        assert_eq!(peeked, vec![1, 2, 3]);
        // Buffer should still have all samples
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_ring_buffer_read_empty() {
        let buffer = AudioRingBuffer::new(100);

        // Read from empty buffer
        let samples = buffer.read(10);
        assert!(samples.is_empty());

        let samples = buffer.read_all();
        assert!(samples.is_empty());
    }

    #[test]
    fn test_ring_buffer_debug() {
        let buffer = AudioRingBuffer::new(100);
        buffer.write(&[1, 2, 3]);

        let debug_str = format!("{:?}", buffer);
        assert!(debug_str.contains("AudioRingBuffer"));
    }

    #[test]
    fn test_capture_read_samples() {
        let capture = AudioInputCapture::for_vosk().unwrap();

        // Write directly to buffer
        capture.buffer.write(&[1, 2, 3, 4, 5]);

        // Read via capture
        let samples = capture.read_samples(3);
        assert_eq!(samples, vec![1, 2, 3]);

        let samples = capture.read_all_samples();
        assert_eq!(samples, vec![4, 5]);
    }

    #[test]
    fn test_capture_available_samples() {
        let capture = AudioInputCapture::for_vosk().unwrap();

        assert_eq!(capture.available_samples(), 0);

        capture.buffer.write(&[0i16; 100]);
        assert_eq!(capture.available_samples(), 100);
    }

    #[test]
    fn test_capture_device_info_none_initially() {
        let capture = AudioInputCapture::for_vosk().unwrap();
        assert!(capture.device_info().is_none());
    }

    #[test]
    fn test_pause_when_not_running() {
        let capture = AudioInputCapture::for_vosk().unwrap();
        let result = capture.pause();
        assert!(matches!(result, Err(AudioInputError::NotRunning)));
    }

    #[test]
    fn test_resume_when_not_running() {
        let capture = AudioInputCapture::for_vosk().unwrap();
        let result = capture.resume();
        assert!(matches!(result, Err(AudioInputError::NotRunning)));
    }

    #[test]
    fn test_constants() {
        assert_eq!(VOSK_SAMPLE_RATE, 16000);
        assert_eq!(VOSK_CHANNELS, 1);
        assert_eq!(DEFAULT_BUFFER_SIZE, 16000 * 4); // 4 seconds
    }

    #[test]
    fn test_ring_buffer_multiple_writes() {
        let buffer = AudioRingBuffer::new(100);

        buffer.write(&[1, 2, 3]);
        buffer.write(&[4, 5, 6]);
        buffer.write(&[7, 8, 9]);

        assert_eq!(buffer.len(), 9);
        let samples = buffer.read_all();
        assert_eq!(samples, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_ring_buffer_overflow_in_chunks() {
        let buffer = AudioRingBuffer::new(5);

        // Write in multiple chunks that overflow
        buffer.write(&[1, 2, 3]);
        buffer.write(&[4, 5, 6]);
        buffer.write(&[7, 8]);

        // Should only have last 5 samples
        assert_eq!(buffer.len(), 5);
        let samples = buffer.read_all();
        assert_eq!(samples, vec![4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_config_builder_chained() {
        let config = AudioInputConfig::for_vosk()
            .with_sample_rate(44100)
            .with_buffer_size(88200)
            .with_device("Microphone");

        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.buffer_size, 88200);
        assert_eq!(config.device_name, Some("Microphone".to_string()));
    }

    #[test]
    fn test_buffered_duration_empty() {
        let capture = AudioInputCapture::for_vosk().unwrap();
        let duration = capture.buffered_duration();
        assert_eq!(duration.as_millis(), 0);
    }

    #[test]
    fn test_buffered_duration_various_sizes() {
        let config = AudioInputConfig::default(); // 16kHz
        let capture = AudioInputCapture::new(config).unwrap();

        // 8000 samples = 0.5 seconds
        capture.buffer.write(&[0i16; 8000]);
        let duration = capture.buffered_duration();
        assert!((duration.as_millis() as i64 - 500).abs() < 10);

        // Clear and add 32000 samples = 2 seconds
        capture.buffer.clear();
        capture.buffer.write(&[0i16; 32000]);
        let duration = capture.buffered_duration();
        assert!((duration.as_millis() as i64 - 2000).abs() < 10);
    }

    #[test]
    fn test_ring_buffer_has_data_persists() {
        let buffer = AudioRingBuffer::new(100);

        assert!(!buffer.has_data());

        buffer.write(&[1, 2, 3]);
        assert!(buffer.has_data());

        // has_data should persist even after reading all
        buffer.read_all();
        assert!(buffer.has_data());

        // has_data persists even after clear
        buffer.clear();
        assert!(buffer.has_data());
    }

    #[test]
    fn test_buffer_arc_clone() {
        let capture = AudioInputCapture::for_vosk().unwrap();

        let buffer1 = capture.buffer();
        let buffer2 = capture.buffer();

        // Both should point to same buffer
        buffer1.write(&[1, 2, 3]);
        assert_eq!(buffer2.len(), 3);
    }
}
