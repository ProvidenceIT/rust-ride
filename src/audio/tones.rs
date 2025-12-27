//! Tone Generation for Audio Cues
//!
//! T077: Create ToneGenerator using rodio SineWave
//! T078: Define tone frequencies and durations for cues
//! T081: Add zone change cue (ascending/descending tones)
//! T082: Implement ZoneChangeDetector with debouncing

use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// T078: Standard tone frequencies for different cue types.
pub mod frequencies {
    /// Low tone for recovery, zone 1-2
    pub const LOW: f32 = 261.63; // C4
    /// Medium tone for tempo, zone 3
    pub const MEDIUM: f32 = 329.63; // E4
    /// High tone for threshold, zone 4
    pub const HIGH: f32 = 392.00; // G4
    /// Very high tone for VO2max+, zone 5-7
    pub const VERY_HIGH: f32 = 523.25; // C5
    /// Alert tone for warnings
    pub const ALERT: f32 = 880.00; // A5
    /// Success tone for achievements
    pub const SUCCESS: f32 = 1046.50; // C6
    /// Error tone for issues
    pub const ERROR: f32 = 220.00; // A3
}

/// T078: Standard tone durations in milliseconds.
pub mod durations {
    /// Short beep (50ms)
    pub const BEEP: u64 = 50;
    /// Quick tone (100ms)
    pub const QUICK: u64 = 100;
    /// Standard tone (200ms)
    pub const STANDARD: u64 = 200;
    /// Long tone (500ms)
    pub const LONG: u64 = 500;
    /// Very long tone (1000ms)
    pub const VERY_LONG: u64 = 1000;
}

/// T078: Predefined cue patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuePattern {
    /// Single beep - general notification
    SingleBeep,
    /// Double beep - interval transition
    DoubleBeep,
    /// Triple beep - workout start/end
    TripleBeep,
    /// Ascending tones - zone increase
    Ascending,
    /// Descending tones - zone decrease
    Descending,
    /// Quick burst - lap marker
    QuickBurst,
    /// Alert pattern - warning
    Alert,
    /// Success fanfare
    Success,
    /// Error tone
    Error,
    /// Countdown tick
    CountdownTick,
}

impl CuePattern {
    /// Get the tone sequence for this pattern.
    pub fn tones(&self) -> Vec<Tone> {
        match self {
            CuePattern::SingleBeep => vec![Tone::new(frequencies::MEDIUM, durations::STANDARD)],

            CuePattern::DoubleBeep => vec![
                Tone::new(frequencies::MEDIUM, durations::QUICK),
                Tone::pause(50),
                Tone::new(frequencies::MEDIUM, durations::QUICK),
            ],

            CuePattern::TripleBeep => vec![
                Tone::new(frequencies::HIGH, durations::QUICK),
                Tone::pause(50),
                Tone::new(frequencies::HIGH, durations::QUICK),
                Tone::pause(50),
                Tone::new(frequencies::VERY_HIGH, durations::STANDARD),
            ],

            CuePattern::Ascending => vec![
                Tone::new(frequencies::LOW, durations::QUICK),
                Tone::pause(30),
                Tone::new(frequencies::MEDIUM, durations::QUICK),
                Tone::pause(30),
                Tone::new(frequencies::HIGH, durations::STANDARD),
            ],

            CuePattern::Descending => vec![
                Tone::new(frequencies::HIGH, durations::QUICK),
                Tone::pause(30),
                Tone::new(frequencies::MEDIUM, durations::QUICK),
                Tone::pause(30),
                Tone::new(frequencies::LOW, durations::STANDARD),
            ],

            CuePattern::QuickBurst => vec![
                Tone::new(frequencies::VERY_HIGH, durations::BEEP),
                Tone::pause(30),
                Tone::new(frequencies::VERY_HIGH, durations::BEEP),
            ],

            CuePattern::Alert => vec![
                Tone::new(frequencies::ALERT, durations::QUICK),
                Tone::pause(100),
                Tone::new(frequencies::ALERT, durations::QUICK),
                Tone::pause(100),
                Tone::new(frequencies::ALERT, durations::STANDARD),
            ],

            CuePattern::Success => vec![
                Tone::new(frequencies::MEDIUM, durations::QUICK),
                Tone::pause(50),
                Tone::new(frequencies::HIGH, durations::QUICK),
                Tone::pause(50),
                Tone::new(frequencies::SUCCESS, durations::LONG),
            ],

            CuePattern::Error => vec![Tone::new(frequencies::ERROR, durations::LONG)],

            CuePattern::CountdownTick => vec![Tone::new(frequencies::MEDIUM, durations::BEEP)],
        }
    }

    /// Get total duration of the pattern in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.tones().iter().map(|t| t.duration_ms).sum()
    }
}

/// A single tone with frequency and duration.
#[derive(Debug, Clone, Copy)]
pub struct Tone {
    /// Frequency in Hz (0 for silence/pause)
    pub frequency_hz: f32,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl Tone {
    /// Create a new tone.
    pub fn new(frequency_hz: f32, duration_ms: u64) -> Self {
        Self {
            frequency_hz,
            duration_ms,
        }
    }

    /// Create a pause (silence).
    pub fn pause(duration_ms: u64) -> Self {
        Self {
            frequency_hz: 0.0,
            duration_ms,
        }
    }

    /// Check if this is a pause.
    pub fn is_pause(&self) -> bool {
        self.frequency_hz <= 0.0
    }
}

/// T077: Tone generator using rodio.
pub struct ToneGenerator {
    /// Volume level (0.0 - 1.0)
    volume: Arc<Mutex<f32>>,
    /// Whether audio is muted
    muted: Arc<Mutex<bool>>,
    /// Whether the generator is enabled
    enabled: Arc<Mutex<bool>>,
}

impl Default for ToneGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ToneGenerator {
    /// Create a new tone generator.
    pub fn new() -> Self {
        Self {
            volume: Arc::new(Mutex::new(0.8)),
            muted: Arc::new(Mutex::new(false)),
            enabled: Arc::new(Mutex::new(true)),
        }
    }

    /// Set the volume level (0.0 - 1.0).
    pub fn set_volume(&self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }

    /// Get the current volume level.
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    /// Set muted state.
    pub fn set_muted(&self, muted: bool) {
        *self.muted.lock().unwrap() = muted;
    }

    /// Check if muted.
    pub fn is_muted(&self) -> bool {
        *self.muted.lock().unwrap()
    }

    /// Enable or disable the generator.
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
    }

    /// Check if enabled.
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Play a single tone.
    pub fn play_tone(&self, frequency_hz: f32, duration_ms: u64) -> Result<(), ToneError> {
        if !self.is_enabled() || self.is_muted() {
            return Ok(());
        }

        if frequency_hz <= 0.0 {
            // This is a pause, just sleep
            std::thread::sleep(Duration::from_millis(duration_ms));
            return Ok(());
        }

        // Get output stream
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| ToneError::DeviceError(e.to_string()))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| ToneError::PlaybackError(e.to_string()))?;

        // Create sine wave source
        let source = SineWave::new(frequency_hz)
            .take_duration(Duration::from_millis(duration_ms))
            .amplify(self.get_volume());

        sink.append(source);
        sink.sleep_until_end();

        Ok(())
    }

    /// Play a cue pattern.
    pub fn play_pattern(&self, pattern: CuePattern) -> Result<(), ToneError> {
        if !self.is_enabled() || self.is_muted() {
            return Ok(());
        }

        for tone in pattern.tones() {
            if tone.is_pause() {
                std::thread::sleep(Duration::from_millis(tone.duration_ms));
            } else {
                self.play_tone(tone.frequency_hz, tone.duration_ms)?;
            }
        }

        Ok(())
    }

    /// Play a cue pattern asynchronously.
    pub async fn play_pattern_async(&self, pattern: CuePattern) -> Result<(), ToneError> {
        if !self.is_enabled() || self.is_muted() {
            return Ok(());
        }

        // Clone Arc values for the async block
        let volume = self.get_volume();
        let tones = pattern.tones();

        tokio::task::spawn_blocking(move || {
            // Get output stream (must be in same thread as playback)
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => return Err(ToneError::DeviceError(e.to_string())),
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(s) => s,
                Err(e) => return Err(ToneError::PlaybackError(e.to_string())),
            };

            for tone in tones {
                if tone.is_pause() {
                    std::thread::sleep(Duration::from_millis(tone.duration_ms));
                } else {
                    let source = SineWave::new(tone.frequency_hz)
                        .take_duration(Duration::from_millis(tone.duration_ms))
                        .amplify(volume);

                    sink.append(source);
                    sink.sleep_until_end();
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| ToneError::PlaybackError(e.to_string()))?
    }
}

/// T082: Zone change detector with debouncing.
pub struct ZoneChangeDetector {
    /// Last detected zone
    last_zone: Option<u8>,
    /// Time of last zone change
    last_change: Option<Instant>,
    /// Debounce duration (minimum time between zone change notifications)
    debounce_duration: Duration,
    /// Minimum samples in new zone before triggering
    min_samples: u32,
    /// Current sample count in new zone
    sample_count: u32,
    /// Pending zone (not yet confirmed)
    pending_zone: Option<u8>,
}

impl Default for ZoneChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoneChangeDetector {
    /// Create a new zone change detector with default settings.
    pub fn new() -> Self {
        Self {
            last_zone: None,
            last_change: None,
            debounce_duration: Duration::from_secs(3),
            min_samples: 3,
            sample_count: 0,
            pending_zone: None,
        }
    }

    /// Create with custom debounce settings.
    pub fn with_debounce(debounce_secs: u64, min_samples: u32) -> Self {
        Self {
            debounce_duration: Duration::from_secs(debounce_secs),
            min_samples,
            ..Self::new()
        }
    }

    /// Update with a new zone reading.
    ///
    /// Returns Some(ZoneChange) if a zone change should be announced.
    pub fn update(&mut self, current_zone: u8) -> Option<ZoneChange> {
        // First reading
        if self.last_zone.is_none() {
            self.last_zone = Some(current_zone);
            return None;
        }

        let last_zone = self.last_zone.unwrap();

        // Same zone as last confirmed zone
        if current_zone == last_zone {
            self.pending_zone = None;
            self.sample_count = 0;
            return None;
        }

        // Check debounce
        if let Some(last_change) = self.last_change {
            if last_change.elapsed() < self.debounce_duration {
                return None;
            }
        }

        // New potential zone or continuing in pending zone
        if self.pending_zone == Some(current_zone) {
            self.sample_count += 1;
        } else {
            self.pending_zone = Some(current_zone);
            self.sample_count = 1;
        }

        // Enough samples to confirm the change
        if self.sample_count >= self.min_samples {
            let direction = if current_zone > last_zone {
                ZoneDirection::Ascending
            } else {
                ZoneDirection::Descending
            };

            let change = ZoneChange {
                from_zone: last_zone,
                to_zone: current_zone,
                direction,
            };

            self.last_zone = Some(current_zone);
            self.last_change = Some(Instant::now());
            self.pending_zone = None;
            self.sample_count = 0;

            return Some(change);
        }

        None
    }

    /// Reset the detector.
    pub fn reset(&mut self) {
        self.last_zone = None;
        self.last_change = None;
        self.pending_zone = None;
        self.sample_count = 0;
    }

    /// Get the current zone.
    pub fn current_zone(&self) -> Option<u8> {
        self.last_zone
    }
}

/// Direction of a zone change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneDirection {
    /// Moving to a higher zone
    Ascending,
    /// Moving to a lower zone
    Descending,
}

/// Information about a zone change.
#[derive(Debug, Clone, Copy)]
pub struct ZoneChange {
    /// Previous zone
    pub from_zone: u8,
    /// New zone
    pub to_zone: u8,
    /// Direction of change
    pub direction: ZoneDirection,
}

impl ZoneChange {
    /// Get the appropriate cue pattern for this zone change.
    pub fn cue_pattern(&self) -> CuePattern {
        match self.direction {
            ZoneDirection::Ascending => CuePattern::Ascending,
            ZoneDirection::Descending => CuePattern::Descending,
        }
    }
}

/// Errors from tone generation.
#[derive(Debug, thiserror::Error)]
pub enum ToneError {
    #[error("Audio device not available: {0}")]
    DeviceError(String),

    #[error("Playback failed: {0}")]
    PlaybackError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cue_pattern_durations() {
        let pattern = CuePattern::SingleBeep;
        assert_eq!(pattern.total_duration_ms(), durations::STANDARD);

        let double = CuePattern::DoubleBeep;
        assert!(double.total_duration_ms() > 0);
    }

    #[test]
    fn test_tone_creation() {
        let tone = Tone::new(440.0, 100);
        assert_eq!(tone.frequency_hz, 440.0);
        assert_eq!(tone.duration_ms, 100);
        assert!(!tone.is_pause());

        let pause = Tone::pause(50);
        assert!(pause.is_pause());
    }

    #[test]
    fn test_zone_change_detector() {
        let mut detector = ZoneChangeDetector::with_debounce(0, 2);

        // First reading - no change
        assert!(detector.update(3).is_none());

        // Same zone - no change
        assert!(detector.update(3).is_none());

        // New zone - need 2 samples to confirm
        assert!(detector.update(4).is_none()); // 1st sample

        // Confirm the change
        let change = detector.update(4).unwrap();
        assert_eq!(change.from_zone, 3);
        assert_eq!(change.to_zone, 4);
        assert_eq!(change.direction, ZoneDirection::Ascending);
    }

    #[test]
    fn test_zone_change_direction() {
        let ascending = ZoneChange {
            from_zone: 2,
            to_zone: 4,
            direction: ZoneDirection::Ascending,
        };
        assert_eq!(ascending.cue_pattern(), CuePattern::Ascending);

        let descending = ZoneChange {
            from_zone: 5,
            to_zone: 2,
            direction: ZoneDirection::Descending,
        };
        assert_eq!(descending.cue_pattern(), CuePattern::Descending);
    }
}
