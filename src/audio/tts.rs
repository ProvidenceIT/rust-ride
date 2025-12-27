//! Text-to-Speech Provider
//!
//! Cross-platform TTS using the tts crate.

use super::AudioError;
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
pub struct DefaultTtsProvider {
    rate: Mutex<f32>,
    volume: Mutex<f32>,
    current_voice: Mutex<Option<String>>,
    is_speaking: Mutex<bool>,
}

impl Default for DefaultTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTtsProvider {
    /// Create a new TTS provider
    pub fn new() -> Self {
        Self {
            rate: Mutex::new(1.0),
            volume: Mutex::new(1.0),
            current_voice: Mutex::new(None),
            is_speaking: Mutex::new(false),
        }
    }
}

impl TtsProvider for DefaultTtsProvider {
    fn initialize(&self) -> Result<(), AudioError> {
        tracing::info!("Initializing TTS provider");

        // TODO: Actually initialize TTS
        // let tts = tts::Tts::default()
        //     .map_err(|e| AudioError::TtsInitFailed(e.to_string()))?;

        Ok(())
    }

    fn get_voices(&self) -> Vec<VoiceInfo> {
        // TODO: Get actual voices from TTS engine
        // For now, return placeholder voices
        vec![
            VoiceInfo {
                id: "default".to_string(),
                name: "System Default".to_string(),
                language: "en-US".to_string(),
                is_default: true,
            },
            #[cfg(target_os = "macos")]
            VoiceInfo {
                id: "samantha".to_string(),
                name: "Samantha".to_string(),
                language: "en-US".to_string(),
                is_default: false,
            },
            #[cfg(target_os = "windows")]
            VoiceInfo {
                id: "david".to_string(),
                name: "Microsoft David".to_string(),
                language: "en-US".to_string(),
                is_default: false,
            },
        ]
    }

    fn set_voice(&self, voice_id: &str) -> Result<(), AudioError> {
        // Verify voice exists
        let voices = self.get_voices();
        if !voices.iter().any(|v| v.id == voice_id) {
            return Err(AudioError::VoiceNotAvailable(voice_id.to_string()));
        }

        *self.current_voice.lock().unwrap() = Some(voice_id.to_string());

        // TODO: Actually set the voice in TTS engine

        Ok(())
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

        *self.is_speaking.lock().unwrap() = true;

        tracing::debug!("TTS speaking: {}", text);

        // TODO: Actually speak using TTS
        // tts.speak(text, false)?;

        // Simulate speech duration
        let duration = std::time::Duration::from_millis((text.len() as u64 * 50).min(10000));
        std::thread::sleep(duration);

        *self.is_speaking.lock().unwrap() = false;

        Ok(())
    }

    async fn speak_async(&self, text: &str) -> Result<(), AudioError> {
        if text.is_empty() {
            return Ok(());
        }

        *self.is_speaking.lock().unwrap() = true;

        tracing::debug!("TTS speaking async: {}", text);

        // TODO: Actually speak using TTS asynchronously
        let duration = std::time::Duration::from_millis((text.len() as u64 * 50).min(10000));
        tokio::time::sleep(duration).await;

        *self.is_speaking.lock().unwrap() = false;

        Ok(())
    }

    fn stop(&self) {
        *self.is_speaking.lock().unwrap() = false;
        // TODO: Actually stop TTS
    }

    fn is_speaking(&self) -> bool {
        *self.is_speaking.lock().unwrap()
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
    }

    #[test]
    fn test_rate_clamping() {
        let provider = DefaultTtsProvider::new();
        provider.set_rate(3.0);
        assert_eq!(provider.get_rate(), 2.0);

        provider.set_rate(0.1);
        assert_eq!(provider.get_rate(), 0.5);
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
