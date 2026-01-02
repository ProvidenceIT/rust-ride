//! Text-to-Speech Provider
//!
//! Cross-platform TTS using the tts crate.
//!
//! Platform-specific backends:
//! - Windows: SAPI (Speech API)
//! - macOS: AVSpeechSynthesizer (or NSSpeechSynthesizer)
//! - Linux: speech-dispatcher

use super::AudioError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Voice information
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    /// Voice identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Language code (e.g., "en-US")
    pub language: String,
    /// Whether this is the default voice
    pub is_default: bool,
}

/// Trait for TTS providers
pub trait TtsProvider: Send + Sync {
    /// Initialize TTS
    fn initialize(&self) -> Result<(), AudioError>;

    /// Get available voices
    fn get_voices(&self) -> Vec<VoiceInfo>;

    /// Set the current voice by ID
    fn set_voice(&self, voice_id: &str) -> Result<(), AudioError>;

    /// Get the current voice ID
    fn get_current_voice(&self) -> Option<String>;

    /// Set speech rate (0.5 - 2.0, where 1.0 is normal)
    fn set_rate(&self, rate: f32);

    /// Get current speech rate
    fn get_rate(&self) -> f32;

    /// Set volume (0.0 - 1.0)
    fn set_volume(&self, volume: f32);

    /// Get current volume
    fn get_volume(&self) -> f32;

    /// Speak text (blocking)
    fn speak(&self, text: &str) -> Result<(), AudioError>;

    /// Speak text asynchronously
    fn speak_async(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<(), AudioError>> + Send;

    /// Stop current speech
    fn stop(&self);

    /// Check if currently speaking
    fn is_speaking(&self) -> bool;
}

/// Default TTS provider using the tts crate
///
/// Note: The underlying `tts::Tts` is not `Send` on all platforms (especially macOS).
/// This implementation uses a mutex-protected Option to handle this, with actual
/// thread-safety improvements planned for subtask 1.4.
pub struct DefaultTtsProvider {
    /// The underlying TTS engine (lazily initialized)
    /// Protected by mutex since Tts is not Send+Sync on all platforms
    engine: Mutex<Option<tts::Tts>>,
    /// Whether TTS has been initialized
    initialized: AtomicBool,
    /// Speech rate (0.5 - 2.0, where 1.0 is normal)
    rate: Mutex<f32>,
    /// Volume (0.0 - 1.0)
    volume: Mutex<f32>,
    /// Current voice ID
    current_voice: Mutex<Option<String>>,
    /// Whether currently speaking (tracked separately for thread-safety)
    is_speaking: AtomicBool,
}

impl Default for DefaultTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTtsProvider {
    /// Create a new TTS provider
    ///
    /// The TTS engine is not initialized until `initialize()` is called.
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
            initialized: AtomicBool::new(false),
            rate: Mutex::new(1.0),
            volume: Mutex::new(1.0),
            current_voice: Mutex::new(None),
            is_speaking: AtomicBool::new(false),
        }
    }

    /// Ensure TTS is initialized, initializing lazily if needed
    fn ensure_initialized(&self) -> Result<(), AudioError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }
        self.initialize()
    }

    /// Apply the current rate setting to the TTS engine
    fn apply_rate(&self, tts: &mut tts::Tts) {
        let rate = *self.rate.lock().unwrap();
        // The tts crate expects rate as a value where 1.0 is normal
        // Our range is 0.5-2.0, which maps well to the crate's expectations
        if let Err(e) = tts.set_rate(rate) {
            tracing::warn!("Failed to set TTS rate: {}", e);
        }
    }

    /// Apply the current volume setting to the TTS engine
    fn apply_volume(&self, tts: &mut tts::Tts) {
        let volume = *self.volume.lock().unwrap();
        // Volume is 0.0-1.0 in both our API and the tts crate
        if let Err(e) = tts.set_volume(volume) {
            tracing::warn!("Failed to set TTS volume: {}", e);
        }
    }
}

impl TtsProvider for DefaultTtsProvider {
    fn initialize(&self) -> Result<(), AudioError> {
        // Check if already initialized
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        tracing::info!("Initializing TTS provider");

        // Platform-specific initialization notes:
        // - Windows: Uses SAPI (Speech API), generally reliable
        // - macOS: Uses AVSpeechSynthesizer, well-supported
        // - Linux: Uses speech-dispatcher, requires speechd installed

        let tts_result = tts::Tts::default();

        match tts_result {
            Ok(mut tts_engine) => {
                // Log the backend being used
                #[cfg(target_os = "windows")]
                tracing::info!("TTS initialized with Windows SAPI backend");
                #[cfg(target_os = "macos")]
                tracing::info!("TTS initialized with macOS speech backend");
                #[cfg(target_os = "linux")]
                tracing::info!("TTS initialized with Linux speech-dispatcher backend");
                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                tracing::info!("TTS initialized with platform backend");

                // Apply initial rate and volume settings
                self.apply_rate(&mut tts_engine);
                self.apply_volume(&mut tts_engine);

                // Store the engine
                let mut engine_guard = self.engine.lock().unwrap();
                *engine_guard = Some(tts_engine);

                // Mark as initialized
                self.initialized.store(true, Ordering::Release);

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("{}", e);

                // Provide platform-specific troubleshooting hints
                #[cfg(target_os = "linux")]
                tracing::error!(
                    "TTS initialization failed: {}. On Linux, ensure speech-dispatcher is installed: \
                     sudo apt install speech-dispatcher",
                    error_msg
                );

                #[cfg(target_os = "windows")]
                tracing::error!(
                    "TTS initialization failed: {}. Windows SAPI should be available by default. \
                     Try running: Add-WindowsCapability -Online -Name Language.Speech~~~en-US~0.0.1.0",
                    error_msg
                );

                #[cfg(target_os = "macos")]
                tracing::error!(
                    "TTS initialization failed: {}. macOS speech synthesis should be available by default.",
                    error_msg
                );

                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                tracing::error!("TTS initialization failed: {}", error_msg);

                Err(AudioError::TtsInitFailed(error_msg))
            }
        }
    }

    fn get_voices(&self) -> Vec<VoiceInfo> {
        // Ensure TTS is initialized before enumerating voices
        if let Err(e) = self.ensure_initialized() {
            tracing::warn!("Failed to initialize TTS for voice enumeration: {}", e);
            return Vec::new();
        }

        let engine_guard = self.engine.lock().unwrap();
        let Some(ref tts) = *engine_guard else {
            tracing::warn!("TTS engine not available for voice enumeration");
            return Vec::new();
        };

        // Get voices from the TTS engine
        match tts.voices() {
            Ok(voices) => {
                // Get the current voice to determine which is default
                let current_voice_id = tts.voice().ok().flatten().map(|v| v.id().to_string());

                voices
                    .into_iter()
                    .enumerate()
                    .map(|(idx, voice)| {
                        let id = voice.id().to_string();
                        let name = voice.name().to_string();
                        // Get language from the voice's language tag
                        let language = voice
                            .language()
                            .map(|lang| lang.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        // Mark as default if it matches the current voice, or if it's the first voice
                        let is_default = current_voice_id
                            .as_ref()
                            .map(|current| current == &id)
                            .unwrap_or(idx == 0);

                        VoiceInfo {
                            id,
                            name,
                            language,
                            is_default,
                        }
                    })
                    .collect()
            }
            Err(e) => {
                tracing::warn!("Failed to enumerate TTS voices: {}", e);
                Vec::new()
            }
        }
    }

    fn set_voice(&self, voice_id: &str) -> Result<(), AudioError> {
        // Ensure TTS is initialized
        self.ensure_initialized()?;

        let mut engine_guard = self.engine.lock().unwrap();
        let Some(ref mut tts) = *engine_guard else {
            return Err(AudioError::TtsInitFailed(
                "TTS engine not available".to_string(),
            ));
        };

        // Get the list of voices from the engine and find the matching one
        let voices = match tts.voices() {
            Ok(v) => v,
            Err(e) => {
                return Err(AudioError::VoiceNotAvailable(format!(
                    "Failed to enumerate voices: {}",
                    e
                )));
            }
        };

        // Find the voice with matching ID
        let voice = voices.into_iter().find(|v| v.id() == voice_id);

        match voice {
            Some(v) => {
                // Actually set the voice in the TTS engine
                if let Err(e) = tts.set_voice(&v) {
                    tracing::error!("Failed to set TTS voice '{}': {}", voice_id, e);
                    return Err(AudioError::VoiceNotAvailable(format!(
                        "Failed to set voice: {}",
                        e
                    )));
                }

                tracing::info!("TTS voice set to: {} ({})", v.name(), voice_id);

                // Store the voice ID for tracking
                *self.current_voice.lock().unwrap() = Some(voice_id.to_string());

                Ok(())
            }
            None => {
                tracing::warn!("Voice not found: {}", voice_id);
                Err(AudioError::VoiceNotAvailable(voice_id.to_string()))
            }
        }
    }

    fn get_current_voice(&self) -> Option<String> {
        self.current_voice.lock().unwrap().clone()
    }

    fn set_rate(&self, rate: f32) {
        *self.rate.lock().unwrap() = rate.clamp(0.5, 2.0);
    }

    fn get_rate(&self) -> f32 {
        *self.rate.lock().unwrap()
    }

    fn set_volume(&self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }

    fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    fn speak(&self, text: &str) -> Result<(), AudioError> {
        if text.is_empty() {
            return Ok(());
        }

        // Ensure TTS is initialized
        self.ensure_initialized()?;

        self.is_speaking.store(true, Ordering::Release);

        tracing::debug!("TTS speaking: {}", text);

        let result = {
            let mut engine_guard = self.engine.lock().unwrap();
            if let Some(ref mut tts) = *engine_guard {
                // Apply current settings before speaking
                self.apply_rate(tts);
                self.apply_volume(tts);

                // Speak the text (interrupt=false to queue if already speaking)
                match tts.speak(text, false) {
                    Ok(_utterance_id) => {
                        // Wait for speech to complete by polling is_speaking
                        // This is a blocking implementation; async version is separate
                        loop {
                            match tts.is_speaking() {
                                Ok(true) => {
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }
                                Ok(false) => break,
                                Err(e) => {
                                    tracing::warn!("Error checking TTS speaking state: {}", e);
                                    // Estimate duration based on text length as fallback
                                    let estimated_ms = (text.len() as u64 * 60).min(10000);
                                    std::thread::sleep(std::time::Duration::from_millis(estimated_ms));
                                    break;
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("TTS speak failed: {}", e);
                        Err(AudioError::PlaybackFailed(e.to_string()))
                    }
                }
            } else {
                Err(AudioError::TtsInitFailed("TTS engine not initialized".to_string()))
            }
        };

        self.is_speaking.store(false, Ordering::Release);
        result
    }

    async fn speak_async(&self, text: &str) -> Result<(), AudioError> {
        if text.is_empty() {
            return Ok(());
        }

        // Ensure TTS is initialized
        self.ensure_initialized()?;

        self.is_speaking.store(true, Ordering::Release);

        tracing::debug!("TTS speaking async: {}", text);

        // The TTS crate's operations are blocking, so we need to handle this carefully.
        // Since tts::Tts is not Send, we cannot easily move it to a spawn_blocking task.
        // For now, we'll do the speak call synchronously but use async sleep for waiting.
        //
        // Note: Subtask 1.4 will implement proper thread-based TTS handling.

        let speak_result = {
            let mut engine_guard = self.engine.lock().unwrap();
            if let Some(ref mut tts) = *engine_guard {
                // Apply current settings before speaking
                self.apply_rate(tts);
                self.apply_volume(tts);

                // Initiate speech (non-blocking, just starts the speech)
                match tts.speak(text, false) {
                    Ok(_utterance_id) => Ok(()),
                    Err(e) => {
                        tracing::error!("TTS speak failed: {}", e);
                        Err(AudioError::PlaybackFailed(e.to_string()))
                    }
                }
            } else {
                Err(AudioError::TtsInitFailed("TTS engine not initialized".to_string()))
            }
        };

        // If speak initiation failed, return early
        if let Err(e) = speak_result {
            self.is_speaking.store(false, Ordering::Release);
            return Err(e);
        }

        // Poll for speech completion asynchronously
        loop {
            // Check speaking state
            let still_speaking = {
                let engine_guard = self.engine.lock().unwrap();
                if let Some(ref tts) = *engine_guard {
                    match tts.is_speaking() {
                        Ok(speaking) => speaking,
                        Err(e) => {
                            tracing::warn!("Error checking TTS speaking state: {}", e);
                            // Assume not speaking on error, fall back to time-based wait
                            let estimated_ms = (text.len() as u64 * 60).min(10000);
                            tokio::time::sleep(std::time::Duration::from_millis(estimated_ms)).await;
                            false
                        }
                    }
                } else {
                    false
                }
            };

            if !still_speaking {
                break;
            }

            // Yield control while waiting
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        self.is_speaking.store(false, Ordering::Release);
        Ok(())
    }

    fn stop(&self) {
        tracing::debug!("TTS stop requested");

        // Try to stop the TTS engine
        let mut engine_guard = self.engine.lock().unwrap();
        if let Some(ref mut tts) = *engine_guard {
            if let Err(e) = tts.stop() {
                tracing::warn!("Failed to stop TTS: {}", e);
            }
        }

        self.is_speaking.store(false, Ordering::Release);
    }

    fn is_speaking(&self) -> bool {
        // First check our tracked state
        if !self.is_speaking.load(Ordering::Acquire) {
            return false;
        }

        // If we think we're speaking, verify with the TTS engine
        let engine_guard = self.engine.lock().unwrap();
        if let Some(ref tts) = *engine_guard {
            match tts.is_speaking() {
                Ok(speaking) => {
                    // Update our state if TTS engine says we're not speaking
                    if !speaking {
                        drop(engine_guard);
                        self.is_speaking.store(false, Ordering::Release);
                    }
                    speaking
                }
                Err(_) => {
                    // On error, trust our tracked state
                    true
                }
            }
        } else {
            // No engine means not speaking
            false
        }
    }
}

/// Utility functions for text preprocessing
pub mod text_utils {
    /// Convert number to spoken form
    pub fn number_to_words(n: u32) -> String {
        match n {
            0 => "zero".to_string(),
            1 => "one".to_string(),
            2 => "two".to_string(),
            3 => "three".to_string(),
            4 => "four".to_string(),
            5 => "five".to_string(),
            6 => "six".to_string(),
            7 => "seven".to_string(),
            8 => "eight".to_string(),
            9 => "nine".to_string(),
            10 => "ten".to_string(),
            11 => "eleven".to_string(),
            12 => "twelve".to_string(),
            13 => "thirteen".to_string(),
            14 => "fourteen".to_string(),
            15 => "fifteen".to_string(),
            16 => "sixteen".to_string(),
            17 => "seventeen".to_string(),
            18 => "eighteen".to_string(),
            19 => "nineteen".to_string(),
            20 => "twenty".to_string(),
            30 => "thirty".to_string(),
            _ if n < 30 => format!("twenty {}", number_to_words(n - 20)),
            _ => n.to_string(), // Just use digits for larger numbers
        }
    }

    /// Make text more TTS-friendly
    pub fn preprocess_for_tts(text: &str) -> String {
        let mut result = text.to_string();

        // Expand common abbreviations
        result = result.replace("km/h", "kilometers per hour");
        result = result.replace("mph", "miles per hour");
        result = result.replace("bpm", "beats per minute");
        result = result.replace("rpm", "revolutions per minute");
        result = result.replace("FTP", "F T P");
        result = result.replace("HR", "heart rate");
        result = result.replace("NP", "normalized power");
        result = result.replace("IF", "intensity factor");
        result = result.replace("TSS", "T S S");

        // Add pauses for better phrasing
        result = result.replace(". ", "... ");
        result = result.replace(", ", ".. ");

        result
    }

    /// Format power value for TTS
    pub fn format_power(watts: u16) -> String {
        format!("{} watts", watts)
    }

    /// Format heart rate for TTS
    pub fn format_heart_rate(bpm: u8) -> String {
        format!("{} beats per minute", bpm)
    }

    /// Format duration for TTS
    pub fn format_duration(seconds: u32) -> String {
        if seconds < 60 {
            format!("{} seconds", seconds)
        } else if seconds < 3600 {
            let mins = seconds / 60;
            let secs = seconds % 60;
            if secs == 0 {
                format!("{} minutes", mins)
            } else {
                format!("{} minutes and {} seconds", mins, secs)
            }
        } else {
            let hours = seconds / 3600;
            let mins = (seconds % 3600) / 60;
            if mins == 0 {
                format!("{} hours", hours)
            } else {
                format!("{} hours and {} minutes", hours, mins)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_provider_creation() {
        let provider = DefaultTtsProvider::new();
        assert!(!provider.is_speaking());
        assert_eq!(provider.get_rate(), 1.0);
        assert_eq!(provider.get_volume(), 1.0);
        // Not initialized until initialize() is called
        assert!(!provider.initialized.load(Ordering::Acquire));
    }

    #[test]
    fn test_rate_clamping() {
        let provider = DefaultTtsProvider::new();
        provider.set_rate(3.0);
        assert_eq!(provider.get_rate(), 2.0);

        provider.set_rate(0.1);
        assert_eq!(provider.get_rate(), 0.5);

        // Normal rate should pass through
        provider.set_rate(1.5);
        assert_eq!(provider.get_rate(), 1.5);
    }

    #[test]
    fn test_volume_clamping() {
        let provider = DefaultTtsProvider::new();
        provider.set_volume(1.5);
        assert_eq!(provider.get_volume(), 1.0);

        provider.set_volume(-0.5);
        assert_eq!(provider.get_volume(), 0.0);

        // Normal volume should pass through
        provider.set_volume(0.75);
        assert_eq!(provider.get_volume(), 0.75);
    }

    #[test]
    fn test_empty_text_speak() {
        let provider = DefaultTtsProvider::new();
        // Empty text should return Ok without initializing TTS
        let result = provider.speak("");
        assert!(result.is_ok());
        assert!(!provider.initialized.load(Ordering::Acquire));
    }

    #[test]
    fn test_voice_setting() {
        let provider = DefaultTtsProvider::new();

        // Initially no voice is set
        assert!(provider.get_current_voice().is_none());

        // Get available voices (this also initializes TTS)
        let voices = provider.get_voices();

        // If TTS is available and has voices, test setting one
        if !voices.is_empty() {
            let first_voice_id = voices[0].id.clone();

            // Set a valid voice from the enumerated list
            let result = provider.set_voice(&first_voice_id);
            assert!(result.is_ok(), "Should be able to set an available voice");
            assert_eq!(provider.get_current_voice(), Some(first_voice_id.clone()));

            // Invalid voice should fail
            let result = provider.set_voice("nonexistent_voice_id_12345");
            assert!(result.is_err(), "Should fail for non-existent voice");

            // Voice should still be the previously set one after failed attempt
            assert_eq!(provider.get_current_voice(), Some(first_voice_id));
        } else {
            // TTS not available or no voices - just verify invalid voice fails
            // This may fail to initialize, which is acceptable in CI environments
            let result = provider.set_voice("nonexistent_voice");
            // Either fails because TTS not available or because voice doesn't exist
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_voice_enumeration() {
        let provider = DefaultTtsProvider::new();

        // Get available voices
        let voices = provider.get_voices();

        // If TTS is available, we should have at least one voice
        // Skip validation if TTS is not available (e.g., in CI)
        if provider.initialized.load(Ordering::Acquire) {
            // All voices should have non-empty IDs and names
            for voice in &voices {
                assert!(!voice.id.is_empty(), "Voice ID should not be empty");
                assert!(!voice.name.is_empty(), "Voice name should not be empty");
            }

            // There should be at most one default voice
            let default_count = voices.iter().filter(|v| v.is_default).count();
            assert!(
                default_count <= 1,
                "Should have at most one default voice, found {}",
                default_count
            );
        }
    }

    #[test]
    fn test_number_to_words() {
        assert_eq!(text_utils::number_to_words(5), "five");
        assert_eq!(text_utils::number_to_words(15), "fifteen");
        assert_eq!(text_utils::number_to_words(25), "twenty five");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(text_utils::format_duration(30), "30 seconds");
        assert_eq!(text_utils::format_duration(90), "1 minutes and 30 seconds");
        assert_eq!(text_utils::format_duration(3600), "1 hours");
        assert_eq!(text_utils::format_duration(3660), "1 hours and 1 minutes");
    }

    #[test]
    fn test_preprocess_for_tts() {
        let input = "Your FTP is 250. Heart rate at 150 bpm.";
        let output = text_utils::preprocess_for_tts(input);
        assert!(output.contains("F T P"));
        assert!(output.contains("beats per minute"));
    }
}
