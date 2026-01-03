//! Audio Backend using Rodio
//!
//! Provides the `RodioAudioBackend` which manages rodio OutputStream and Sink
//! for playing sound files (WAV/MP3) and tones.
//!
//! ## Platform Support
//!
//! The audio backend handles platform-specific differences for Windows, macOS, and Linux:
//!
//! - **Windows**: Uses WASAPI (Windows Audio Session API) by default
//! - **macOS**: Uses CoreAudio
//! - **Linux**: Uses ALSA or PulseAudio depending on system configuration
//!
//! ## Hot-Plug Support
//!
//! The backend implements device hot-plug recovery through periodic reinitialization
//! attempts when the audio device becomes unavailable. This allows the application
//! to recover automatically when headphones are plugged in or audio devices are
//! connected.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use rodio::source::SineWave;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use thiserror::Error;

/// Detected operating system platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

impl Platform {
    /// Detect the current platform at runtime
    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Platform::Unknown
        }
    }

    /// Get the audio backend name used on this platform
    pub fn audio_backend_name(&self) -> &'static str {
        match self {
            Platform::Windows => "WASAPI (Windows Audio Session API)",
            Platform::MacOS => "CoreAudio",
            Platform::Linux => "ALSA/PulseAudio",
            Platform::Unknown => "Unknown audio backend",
        }
    }

    /// Get platform-specific troubleshooting hints for audio device issues
    pub fn troubleshooting_hints(&self) -> Vec<&'static str> {
        match self {
            Platform::Windows => vec![
                "Check that audio output is not disabled in Windows Sound Settings",
                "Ensure audio drivers are up to date",
                "Try selecting a different audio output device in Sound Settings",
                "Check if audio is being routed to a disconnected device (e.g., unplugged headphones)",
                "Restart the Windows Audio service (services.msc -> Windows Audio)",
                "Run the Windows Audio Troubleshooter (Settings -> System -> Sound -> Troubleshoot)",
            ],
            Platform::MacOS => vec![
                "Check System Preferences -> Sound for audio output selection",
                "Ensure no other application has exclusive access to the audio device",
                "Try resetting Core Audio (sudo killall coreaudiod)",
                "Check if audio is being routed to AirPlay or other external devices",
                "Verify microphone/audio permissions in System Preferences -> Security & Privacy",
            ],
            Platform::Linux => vec![
                "Check PulseAudio/PipeWire status: `systemctl --user status pulseaudio` or `pipewire`",
                "List available audio devices: `pactl list sinks` or `aplay -l`",
                "Ensure your user is in the 'audio' group: `groups $USER`",
                "Try restarting PulseAudio: `pulseaudio -k && pulseaudio --start`",
                "Check ALSA configuration: `cat /etc/asound.conf` or `~/.asoundrc`",
                "Install required packages: `sudo apt install pulseaudio alsa-utils` (Debian/Ubuntu)",
                "For PipeWire systems: `systemctl --user restart pipewire pipewire-pulse`",
            ],
            Platform::Unknown => vec![
                "Verify that an audio output device is connected and configured",
                "Check system audio settings for output device selection",
            ],
        }
    }
}

/// Detailed audio device error with platform-specific context
#[derive(Debug, Clone)]
pub struct AudioDeviceError {
    /// The original error message from the audio subsystem
    pub raw_error: String,
    /// The detected platform
    pub platform: Platform,
    /// User-friendly error message
    pub message: String,
    /// Troubleshooting hints for this platform
    pub hints: Vec<String>,
}

impl AudioDeviceError {
    /// Create a new audio device error with platform-specific context
    pub fn new(raw_error: impl Into<String>) -> Self {
        let raw_error = raw_error.into();
        let platform = Platform::detect();
        let message = Self::create_user_message(&raw_error, platform);
        let hints = platform
            .troubleshooting_hints()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            raw_error,
            platform,
            message,
            hints,
        }
    }

    /// Create a user-friendly error message based on the raw error and platform
    fn create_user_message(raw_error: &str, platform: Platform) -> String {
        let raw_lower = raw_error.to_lowercase();

        // Common error patterns and their user-friendly messages
        if raw_lower.contains("no device")
            || raw_lower.contains("no output")
            || raw_lower.contains("device not found")
        {
            return format!(
                "No audio output device found. Please connect headphones or speakers. ({})",
                platform.audio_backend_name()
            );
        }

        if raw_lower.contains("permission") || raw_lower.contains("access denied") {
            return match platform {
                Platform::Linux => {
                    "Permission denied accessing audio device. Ensure your user is in the 'audio' group.".to_string()
                }
                Platform::MacOS => {
                    "Permission denied accessing audio device. Check audio permissions in System Preferences.".to_string()
                }
                _ => "Permission denied accessing audio device. Check your system audio permissions.".to_string(),
            };
        }

        if raw_lower.contains("busy") || raw_lower.contains("exclusive") {
            return "Audio device is in use by another application. Close other audio applications and try again.".to_string();
        }

        if raw_lower.contains("timeout") {
            return "Audio device timed out. The device may be unresponsive or disconnected."
                .to_string();
        }

        if raw_lower.contains("pulse") || raw_lower.contains("alsa") {
            return format!(
                "Linux audio subsystem error ({}). Check PulseAudio/PipeWire status.",
                raw_error
            );
        }

        // Default message with platform context
        format!(
            "Audio device error on {}: {}",
            platform.audio_backend_name(),
            raw_error
        )
    }

    /// Format the error for logging (includes all details)
    pub fn to_log_string(&self) -> String {
        format!(
            "Audio device error [{}]: {} (raw: {})",
            self.platform.audio_backend_name(),
            self.message,
            self.raw_error
        )
    }

    /// Get a formatted help message with troubleshooting hints
    pub fn help_message(&self) -> String {
        let mut msg = format!("{}\n\nTroubleshooting steps:", self.message);
        for (i, hint) in self.hints.iter().enumerate() {
            msg.push_str(&format!("\n  {}. {}", i + 1, hint));
        }
        msg
    }
}

impl std::fmt::Display for AudioDeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AudioDeviceError {}

/// Status of the audio device for monitoring
#[derive(Debug, Clone)]
pub struct AudioDeviceStatus {
    /// Whether a device is currently available
    pub available: bool,
    /// Current state of the backend
    pub state: BackendState,
    /// Platform being used
    pub platform: Platform,
    /// Last successful initialization timestamp
    pub last_initialized: Option<Instant>,
    /// Last error if any
    pub last_error: Option<AudioDeviceError>,
    /// Number of successful reinitializations
    pub recovery_count: u64,
    /// Number of failed initialization attempts since last success
    pub failed_attempts: u64,
}

impl AudioDeviceStatus {
    /// Check if device recovery is in progress (failed but attempting to recover)
    pub fn is_recovering(&self) -> bool {
        !self.available && self.failed_attempts > 0
    }

    /// Check if the device has been stable (no recent failures)
    pub fn is_stable(&self) -> bool {
        self.available && self.failed_attempts == 0
    }

    /// Get a user-friendly status message
    pub fn status_message(&self) -> String {
        if self.available {
            if self.recovery_count > 0 {
                format!(
                    "Audio device available (recovered {} time{})",
                    self.recovery_count,
                    if self.recovery_count == 1 { "" } else { "s" }
                )
            } else {
                "Audio device available".to_string()
            }
        } else if self.failed_attempts > 0 {
            format!(
                "Audio device unavailable - attempting recovery ({} attempts)",
                self.failed_attempts
            )
        } else {
            "Audio device not initialized".to_string()
        }
    }

    /// Get an icon hint for UI display
    pub fn icon_hint(&self) -> &'static str {
        if self.available {
            "audio_available"
        } else if self.is_recovering() {
            "audio_recovering"
        } else {
            "audio_unavailable"
        }
    }
}

impl Default for AudioDeviceStatus {
    fn default() -> Self {
        Self {
            available: false,
            state: BackendState::Uninitialized,
            platform: Platform::detect(),
            last_initialized: None,
            last_error: None,
            recovery_count: 0,
            failed_attempts: 0,
        }
    }
}

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

    #[error("Platform-specific audio error: {0}")]
    PlatformError(#[from] AudioDeviceError),
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

/// Configuration for audio device hot-plug recovery
#[derive(Debug, Clone)]
pub struct HotPlugConfig {
    /// Whether hot-plug recovery is enabled
    pub enabled: bool,
    /// Minimum interval between recovery attempts
    pub retry_interval: Duration,
    /// Maximum number of consecutive failures before giving up temporarily
    pub max_consecutive_failures: u64,
    /// Backoff multiplier for consecutive failures (exponential backoff)
    pub backoff_multiplier: f32,
    /// Maximum backoff interval
    pub max_backoff: Duration,
}

impl Default for HotPlugConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retry_interval: Duration::from_secs(5),
            max_consecutive_failures: 10,
            backoff_multiplier: 1.5,
            max_backoff: Duration::from_secs(60),
        }
    }
}

/// Rodio-based audio backend for sound file playback
///
/// This backend manages the rodio OutputStream and provides methods for:
/// - Loading and playing WAV/MP3 files from disk
/// - Playing generated tones
/// - Volume control
/// - Sound caching for fast replay
/// - Platform-specific device initialization
/// - Hot-plug recovery for device reconnection
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
    /// Detected platform
    platform: Platform,
    /// Last successful initialization time
    last_initialized: RwLock<Option<Instant>>,
    /// Last error that occurred
    last_error: RwLock<Option<AudioDeviceError>>,
    /// Count of successful reinitializations (device recoveries)
    recovery_count: AtomicU64,
    /// Count of consecutive failed initialization attempts
    failed_attempts: AtomicU64,
    /// Last recovery attempt time
    last_recovery_attempt: RwLock<Option<Instant>>,
    /// Hot-plug configuration
    hot_plug_config: RwLock<HotPlugConfig>,
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
        let platform = Platform::detect();
        tracing::debug!(
            "Creating RodioAudioBackend for platform: {:?} ({})",
            platform,
            platform.audio_backend_name()
        );

        Self {
            stream: RwLock::new(None),
            stream_handle: RwLock::new(None),
            state: RwLock::new(BackendState::Uninitialized),
            sound_cache: RwLock::new(HashMap::new()),
            sounds_path,
            volume: RwLock::new(0.8),
            muted: RwLock::new(false),
            active_sinks: Mutex::new(Vec::new()),
            platform,
            last_initialized: RwLock::new(None),
            last_error: RwLock::new(None),
            recovery_count: AtomicU64::new(0),
            failed_attempts: AtomicU64::new(0),
            last_recovery_attempt: RwLock::new(None),
            hot_plug_config: RwLock::new(HotPlugConfig::default()),
        }
    }

    /// Create a new audio backend with custom hot-plug configuration
    pub fn with_hot_plug_config(sounds_path: PathBuf, hot_plug_config: HotPlugConfig) -> Self {
        let mut backend = Self::with_sounds_path(sounds_path);
        *backend.hot_plug_config.write().unwrap() = hot_plug_config;
        backend
    }

    /// Get the detected platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Get the current hot-plug configuration
    pub fn hot_plug_config(&self) -> HotPlugConfig {
        self.hot_plug_config.read().unwrap().clone()
    }

    /// Update hot-plug configuration
    pub fn set_hot_plug_config(&self, config: HotPlugConfig) {
        *self.hot_plug_config.write().unwrap() = config;
    }

    /// Initialize the audio backend
    ///
    /// Attempts to open the default audio output device. If no audio device
    /// is available, the backend enters a "failed" state but doesn't panic.
    ///
    /// # Platform-Specific Behavior
    ///
    /// - **Windows**: Uses WASAPI for low-latency audio output
    /// - **macOS**: Uses CoreAudio
    /// - **Linux**: Uses ALSA/PulseAudio through the CPAL backend
    ///
    /// On failure, the backend provides platform-specific troubleshooting hints
    /// that can be retrieved via `get_device_status()`.
    pub fn initialize(&self) -> Result<(), BackendError> {
        tracing::info!(
            "Initializing RodioAudioBackend on {} ({})",
            self.platform.audio_backend_name(),
            std::env::consts::OS
        );

        // Platform-specific pre-initialization checks
        self.platform_pre_init_checks();

        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                *self.stream.write().unwrap() = Some(stream);
                *self.stream_handle.write().unwrap() = Some(handle);
                *self.state.write().unwrap() = BackendState::Ready;
                *self.last_initialized.write().unwrap() = Some(Instant::now());
                *self.last_error.write().unwrap() = None;
                self.failed_attempts.store(0, Ordering::Release);

                tracing::info!(
                    "Audio backend initialized successfully using {}",
                    self.platform.audio_backend_name()
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                let device_error = AudioDeviceError::new(&error_msg);

                // Log with platform-specific context
                tracing::warn!("{}", device_error.to_log_string());

                // Log troubleshooting hints at debug level
                for hint in &device_error.hints {
                    tracing::debug!("Troubleshooting hint: {}", hint);
                }

                *self.state.write().unwrap() = BackendState::Failed;
                *self.last_error.write().unwrap() = Some(device_error.clone());
                self.failed_attempts.fetch_add(1, Ordering::Release);

                Err(BackendError::PlatformError(device_error))
            }
        }
    }

    /// Perform platform-specific pre-initialization checks
    fn platform_pre_init_checks(&self) {
        match self.platform {
            Platform::Linux => {
                // Check for common Linux audio issues
                tracing::debug!("Linux audio check: Using ALSA/PulseAudio backend");

                // Log environment info that might affect audio
                if let Ok(display) = std::env::var("DISPLAY") {
                    tracing::debug!("X11 DISPLAY set: {}", display);
                } else if std::env::var("WAYLAND_DISPLAY").is_ok() {
                    tracing::debug!("Running on Wayland");
                } else {
                    tracing::debug!("No display server detected (headless or TTY)");
                }

                if let Ok(pulse_server) = std::env::var("PULSE_SERVER") {
                    tracing::debug!("PulseAudio server configured: {}", pulse_server);
                }
            }
            Platform::Windows => {
                tracing::debug!("Windows audio check: Using WASAPI backend");
            }
            Platform::MacOS => {
                tracing::debug!("macOS audio check: Using CoreAudio backend");
            }
            Platform::Unknown => {
                tracing::warn!("Unknown platform - audio behavior may be unpredictable");
            }
        }
    }

    /// Reinitialize the audio backend (for hot-plug support)
    ///
    /// This can be called to attempt to recover from a failed state
    /// when an audio device becomes available.
    ///
    /// # Hot-Plug Recovery
    ///
    /// This method implements exponential backoff to avoid hammering the audio
    /// subsystem with recovery attempts. The backoff can be configured via
    /// `set_hot_plug_config()`.
    pub fn reinitialize(&self) -> Result<(), BackendError> {
        let was_ready = self.is_ready();

        // Clean up existing state
        self.cleanup_active_sinks();
        *self.stream_handle.write().unwrap() = None;
        *self.stream.write().unwrap() = None;

        // Record this attempt
        *self.last_recovery_attempt.write().unwrap() = Some(Instant::now());

        // Try to initialize again
        let result = self.initialize();

        // Track recovery success
        if result.is_ok() && !was_ready {
            let count = self.recovery_count.fetch_add(1, Ordering::Release) + 1;
            tracing::info!(
                "Audio device recovered successfully (recovery #{}) on {}",
                count,
                self.platform.audio_backend_name()
            );
        }

        result
    }

    /// Attempt recovery if appropriate based on hot-plug configuration
    ///
    /// This method checks if enough time has passed since the last recovery
    /// attempt and whether we haven't exceeded the maximum failure count.
    /// Uses exponential backoff for recovery attempts.
    ///
    /// Returns `true` if recovery was attempted, `false` if skipped.
    pub fn try_recovery(&self) -> bool {
        let config = self.hot_plug_config.read().unwrap().clone();

        if !config.enabled {
            return false;
        }

        // Don't try recovery if already ready
        if self.is_ready() {
            return false;
        }

        let failed_attempts = self.failed_attempts.load(Ordering::Acquire);

        // Check if we've exceeded max failures
        if failed_attempts >= config.max_consecutive_failures {
            tracing::debug!(
                "Skipping recovery: exceeded max consecutive failures ({})",
                config.max_consecutive_failures
            );
            return false;
        }

        // Calculate backoff interval
        let backoff = if failed_attempts > 0 {
            let multiplier = config.backoff_multiplier.powi(failed_attempts as i32 - 1);
            let interval = config.retry_interval.mul_f32(multiplier);
            std::cmp::min(interval, config.max_backoff)
        } else {
            config.retry_interval
        };

        // Check if enough time has passed since last attempt
        let should_retry = {
            let last_attempt = self.last_recovery_attempt.read().unwrap();
            match *last_attempt {
                Some(last) => last.elapsed() >= backoff,
                None => true, // No previous attempt
            }
        };

        if !should_retry {
            return false;
        }

        tracing::debug!(
            "Attempting audio device recovery (attempt {}/{})",
            failed_attempts + 1,
            config.max_consecutive_failures
        );

        // Attempt reinitialize
        match self.reinitialize() {
            Ok(()) => {
                tracing::info!("Audio device recovery successful");
                true
            }
            Err(e) => {
                tracing::debug!("Audio device recovery failed: {}", e);
                true
            }
        }
    }

    /// Reset the failed attempts counter and allow new recovery attempts
    ///
    /// Call this when you want to retry recovery after hitting the max failures limit,
    /// for example after a user manually connects an audio device.
    pub fn reset_recovery(&self) {
        self.failed_attempts.store(0, Ordering::Release);
        *self.last_recovery_attempt.write().unwrap() = None;
        tracing::debug!("Audio device recovery state reset");
    }

    /// Get the current device status for monitoring and UI display
    pub fn get_device_status(&self) -> AudioDeviceStatus {
        AudioDeviceStatus {
            available: self.is_ready(),
            state: self.state(),
            platform: self.platform,
            last_initialized: *self.last_initialized.read().unwrap(),
            last_error: self.last_error.read().unwrap().clone(),
            recovery_count: self.recovery_count.load(Ordering::Acquire),
            failed_attempts: self.failed_attempts.load(Ordering::Acquire),
        }
    }

    /// Get the last error that occurred, if any
    pub fn last_error(&self) -> Option<AudioDeviceError> {
        self.last_error.read().unwrap().clone()
    }

    /// Check if there's a device error with troubleshooting hints available
    pub fn has_error_with_hints(&self) -> bool {
        self.last_error.read().unwrap().is_some()
    }

    /// Get troubleshooting hints for the current platform
    pub fn get_troubleshooting_hints(&self) -> Vec<&'static str> {
        self.platform.troubleshooting_hints()
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

    // ========== Platform Detection Tests ==========

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        // Should detect one of the known platforms on CI/development machines
        assert!(matches!(
            platform,
            Platform::Windows | Platform::MacOS | Platform::Linux | Platform::Unknown
        ));
    }

    #[test]
    fn test_platform_audio_backend_name() {
        assert_eq!(
            Platform::Windows.audio_backend_name(),
            "WASAPI (Windows Audio Session API)"
        );
        assert_eq!(Platform::MacOS.audio_backend_name(), "CoreAudio");
        assert_eq!(Platform::Linux.audio_backend_name(), "ALSA/PulseAudio");
        assert_eq!(Platform::Unknown.audio_backend_name(), "Unknown audio backend");
    }

    #[test]
    fn test_platform_troubleshooting_hints() {
        // Each platform should have at least some hints
        assert!(!Platform::Windows.troubleshooting_hints().is_empty());
        assert!(!Platform::MacOS.troubleshooting_hints().is_empty());
        assert!(!Platform::Linux.troubleshooting_hints().is_empty());
        assert!(!Platform::Unknown.troubleshooting_hints().is_empty());

        // Linux should have the most hints (complex audio stack)
        let linux_hints = Platform::Linux.troubleshooting_hints();
        assert!(linux_hints.len() >= 5);

        // Hints should contain platform-specific content
        let linux_hints_str = linux_hints.join(" ");
        assert!(linux_hints_str.contains("PulseAudio") || linux_hints_str.contains("ALSA"));
    }

    // ========== AudioDeviceError Tests ==========

    #[test]
    fn test_audio_device_error_creation() {
        let error = AudioDeviceError::new("no device found");
        assert!(!error.raw_error.is_empty());
        assert!(!error.message.is_empty());
        assert!(!error.hints.is_empty());
    }

    #[test]
    fn test_audio_device_error_no_device_message() {
        let error = AudioDeviceError::new("no device available");
        assert!(error.message.contains("No audio output device found"));
        assert!(error.message.contains("headphones or speakers"));
    }

    #[test]
    fn test_audio_device_error_permission_message() {
        let error = AudioDeviceError::new("permission denied accessing device");
        assert!(error.message.contains("Permission denied"));
    }

    #[test]
    fn test_audio_device_error_busy_message() {
        let error = AudioDeviceError::new("device busy exclusive access");
        assert!(error.message.contains("in use by another application"));
    }

    #[test]
    fn test_audio_device_error_display() {
        let error = AudioDeviceError::new("test error");
        let display = format!("{}", error);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_audio_device_error_to_log_string() {
        let error = AudioDeviceError::new("test error");
        let log = error.to_log_string();
        assert!(log.contains("Audio device error"));
        assert!(log.contains("test error"));
    }

    #[test]
    fn test_audio_device_error_help_message() {
        let error = AudioDeviceError::new("no device");
        let help = error.help_message();
        assert!(help.contains("Troubleshooting steps"));
        assert!(help.contains("1."));
    }

    // ========== AudioDeviceStatus Tests ==========

    #[test]
    fn test_audio_device_status_default() {
        let status = AudioDeviceStatus::default();
        assert!(!status.available);
        assert_eq!(status.state, BackendState::Uninitialized);
        assert!(status.last_initialized.is_none());
        assert!(status.last_error.is_none());
        assert_eq!(status.recovery_count, 0);
        assert_eq!(status.failed_attempts, 0);
    }

    #[test]
    fn test_audio_device_status_is_recovering() {
        let mut status = AudioDeviceStatus::default();
        assert!(!status.is_recovering());

        status.failed_attempts = 3;
        assert!(status.is_recovering());

        status.available = true;
        assert!(!status.is_recovering()); // Available = not recovering
    }

    #[test]
    fn test_audio_device_status_is_stable() {
        let mut status = AudioDeviceStatus::default();
        status.available = true;
        status.failed_attempts = 0;
        assert!(status.is_stable());

        status.failed_attempts = 1;
        assert!(!status.is_stable());

        status.available = false;
        assert!(!status.is_stable());
    }

    #[test]
    fn test_audio_device_status_message() {
        let mut status = AudioDeviceStatus::default();

        // Uninitialized
        assert!(status.status_message().contains("not initialized"));

        // Available
        status.available = true;
        assert!(status.status_message().contains("available"));

        // Recovered
        status.recovery_count = 2;
        let msg = status.status_message();
        assert!(msg.contains("recovered"));
        assert!(msg.contains("2"));

        // Unavailable with recovery attempts
        status.available = false;
        status.failed_attempts = 3;
        let msg = status.status_message();
        assert!(msg.contains("unavailable"));
        assert!(msg.contains("3 attempts"));
    }

    #[test]
    fn test_audio_device_status_icon_hint() {
        let mut status = AudioDeviceStatus::default();
        assert_eq!(status.icon_hint(), "audio_unavailable");

        status.failed_attempts = 1;
        assert_eq!(status.icon_hint(), "audio_recovering");

        status.available = true;
        assert_eq!(status.icon_hint(), "audio_available");
    }

    // ========== HotPlugConfig Tests ==========

    #[test]
    fn test_hot_plug_config_default() {
        let config = HotPlugConfig::default();
        assert!(config.enabled);
        assert_eq!(config.retry_interval, Duration::from_secs(5));
        assert_eq!(config.max_consecutive_failures, 10);
        assert!(config.backoff_multiplier > 1.0);
        assert!(config.max_backoff >= config.retry_interval);
    }

    #[test]
    fn test_hot_plug_config_custom() {
        let config = HotPlugConfig {
            enabled: false,
            retry_interval: Duration::from_secs(10),
            max_consecutive_failures: 5,
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(120),
        };
        assert!(!config.enabled);
        assert_eq!(config.retry_interval, Duration::from_secs(10));
    }

    // ========== Backend Device Status Tests ==========

    #[test]
    fn test_backend_platform_detection() {
        let backend = RodioAudioBackend::new();
        let platform = backend.platform();
        // Should match the current platform
        assert_eq!(platform, Platform::detect());
    }

    #[test]
    fn test_backend_get_device_status() {
        let backend = RodioAudioBackend::new();
        let status = backend.get_device_status();

        // Initially uninitialized
        assert!(!status.available);
        assert_eq!(status.state, BackendState::Uninitialized);
        assert_eq!(status.recovery_count, 0);
        assert_eq!(status.failed_attempts, 0);
    }

    #[test]
    fn test_backend_hot_plug_config() {
        let backend = RodioAudioBackend::new();

        // Default config
        let config = backend.hot_plug_config();
        assert!(config.enabled);

        // Update config
        let new_config = HotPlugConfig {
            enabled: false,
            retry_interval: Duration::from_secs(30),
            max_consecutive_failures: 3,
            backoff_multiplier: 1.2,
            max_backoff: Duration::from_secs(300),
        };
        backend.set_hot_plug_config(new_config.clone());

        let retrieved = backend.hot_plug_config();
        assert!(!retrieved.enabled);
        assert_eq!(retrieved.retry_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_backend_with_hot_plug_config() {
        let config = HotPlugConfig {
            enabled: false,
            retry_interval: Duration::from_secs(15),
            ..Default::default()
        };

        let backend =
            RodioAudioBackend::with_hot_plug_config(PathBuf::from("sounds"), config.clone());

        let retrieved = backend.hot_plug_config();
        assert!(!retrieved.enabled);
        assert_eq!(retrieved.retry_interval, Duration::from_secs(15));
    }

    #[test]
    fn test_backend_troubleshooting_hints() {
        let backend = RodioAudioBackend::new();
        let hints = backend.get_troubleshooting_hints();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_backend_last_error_initially_none() {
        let backend = RodioAudioBackend::new();
        assert!(backend.last_error().is_none());
        assert!(!backend.has_error_with_hints());
    }

    #[test]
    fn test_backend_reset_recovery() {
        let backend = RodioAudioBackend::new();

        // Simulate some failures
        backend.failed_attempts.store(5, Ordering::Release);
        *backend.last_recovery_attempt.write().unwrap() = Some(Instant::now());

        // Reset
        backend.reset_recovery();

        assert_eq!(backend.failed_attempts.load(Ordering::Acquire), 0);
        assert!(backend.last_recovery_attempt.read().unwrap().is_none());
    }

    #[test]
    fn test_backend_try_recovery_disabled() {
        let backend = RodioAudioBackend::new();

        // Disable hot-plug
        let mut config = backend.hot_plug_config();
        config.enabled = false;
        backend.set_hot_plug_config(config);

        // Should not attempt recovery when disabled
        assert!(!backend.try_recovery());
    }

    #[test]
    fn test_backend_try_recovery_when_ready() {
        let backend = RodioAudioBackend::new();

        // Simulate ready state
        *backend.state.write().unwrap() = BackendState::Ready;

        // Should not attempt recovery when already ready
        assert!(!backend.try_recovery());
    }

    #[test]
    fn test_backend_try_recovery_max_failures() {
        let backend = RodioAudioBackend::new();

        // Set state to failed and exceed max failures
        *backend.state.write().unwrap() = BackendState::Failed;
        backend.failed_attempts.store(100, Ordering::Release);

        // Should not attempt recovery when max failures exceeded
        assert!(!backend.try_recovery());
    }

    // ========== BackendError Tests ==========

    #[test]
    fn test_backend_error_from_device_error() {
        let device_error = AudioDeviceError::new("test error");
        let backend_error: BackendError = device_error.into();

        match backend_error {
            BackendError::PlatformError(e) => {
                assert!(e.raw_error.contains("test error"));
            }
            _ => panic!("Expected PlatformError variant"),
        }
    }

    #[test]
    fn test_backend_error_display() {
        let error = BackendError::DeviceNotAvailable("no device".to_string());
        let display = format!("{}", error);
        assert!(display.contains("no device"));

        let error = BackendError::NotInitialized;
        let display = format!("{}", error);
        assert!(display.contains("not initialized"));
    }
}
