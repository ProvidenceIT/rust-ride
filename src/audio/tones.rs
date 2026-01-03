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

    // Countdown-specific frequencies with escalating urgency
    /// Countdown tick at 10 seconds - gentle reminder (G4)
    pub const COUNTDOWN_10: f32 = 392.00; // G4
    /// Countdown tick at 5 seconds - attention (A4)
    pub const COUNTDOWN_5: f32 = 440.00; // A4
    /// Final countdown 3 seconds - preparation (B4)
    pub const COUNTDOWN_3: f32 = 493.88; // B4
    /// Final countdown 2 seconds - alert (C5)
    pub const COUNTDOWN_2: f32 = 523.25; // C5
    /// Final countdown 1 second - urgent (D5)
    pub const COUNTDOWN_1: f32 = 587.33; // D5
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

    // Countdown-specific durations (all under 200ms to not overlap with speech)
    /// Countdown tick at 10 seconds - brief (60ms)
    pub const COUNTDOWN_TICK_10: u64 = 60;
    /// Countdown tick at 5 seconds - slightly longer (80ms)
    pub const COUNTDOWN_TICK_5: u64 = 80;
    /// Final countdown 3 seconds (100ms)
    pub const COUNTDOWN_FINAL_3: u64 = 100;
    /// Final countdown 2 seconds (120ms)
    pub const COUNTDOWN_FINAL_2: u64 = 120;
    /// Final countdown 1 second - most impactful (150ms)
    pub const COUNTDOWN_FINAL_1: u64 = 150;
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
    /// Generic countdown tick (legacy, use specific countdown patterns for new code)
    CountdownTick,

    // Countdown-specific patterns with escalating urgency
    /// Countdown tick at 10 seconds - gentle reminder
    CountdownTick10,
    /// Countdown tick at 5 seconds - attention
    CountdownTick5,
    /// Final countdown 3 seconds - preparation begins
    CountdownFinal3,
    /// Final countdown 2 seconds - almost there
    CountdownFinal2,
    /// Final countdown 1 second - imminent transition
    CountdownFinal1,
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

            // Countdown-specific patterns with escalating urgency
            // Each pattern has a distinct frequency and duration for clear differentiation
            CuePattern::CountdownTick10 => vec![
                Tone::new(frequencies::COUNTDOWN_10, durations::COUNTDOWN_TICK_10),
            ],

            CuePattern::CountdownTick5 => vec![
                Tone::new(frequencies::COUNTDOWN_5, durations::COUNTDOWN_TICK_5),
            ],

            // Final countdown patterns have double-tone patterns for increased urgency
            CuePattern::CountdownFinal3 => vec![
                Tone::new(frequencies::COUNTDOWN_3, durations::COUNTDOWN_FINAL_3),
            ],

            CuePattern::CountdownFinal2 => vec![
                Tone::new(frequencies::COUNTDOWN_2, durations::COUNTDOWN_FINAL_2 / 2),
                Tone::pause(20),
                Tone::new(frequencies::COUNTDOWN_2, durations::COUNTDOWN_FINAL_2 / 2),
            ],

            CuePattern::CountdownFinal1 => vec![
                Tone::new(frequencies::COUNTDOWN_1, durations::COUNTDOWN_FINAL_1 / 3),
                Tone::pause(15),
                Tone::new(frequencies::COUNTDOWN_1, durations::COUNTDOWN_FINAL_1 / 3),
                Tone::pause(15),
                Tone::new(frequencies::COUNTDOWN_1, durations::COUNTDOWN_FINAL_1 / 3),
            ],
        }
    }

    /// Get total duration of the pattern in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.tones().iter().map(|t| t.duration_ms).sum()
    }

    /// Get the appropriate countdown pattern for a given number of seconds remaining.
    ///
    /// Returns `Some(CuePattern)` for countdown-relevant seconds (10, 5, 3, 2, 1),
    /// or `None` for other values that don't have specific countdown patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustride::audio::tones::CuePattern;
    ///
    /// assert!(CuePattern::for_countdown_seconds(10).is_some());
    /// assert!(CuePattern::for_countdown_seconds(5).is_some());
    /// assert!(CuePattern::for_countdown_seconds(3).is_some());
    /// assert!(CuePattern::for_countdown_seconds(7).is_none()); // No pattern for 7 seconds
    /// ```
    pub fn for_countdown_seconds(seconds: u32) -> Option<Self> {
        match seconds {
            10 => Some(CuePattern::CountdownTick10),
            5 => Some(CuePattern::CountdownTick5),
            3 => Some(CuePattern::CountdownFinal3),
            2 => Some(CuePattern::CountdownFinal2),
            1 => Some(CuePattern::CountdownFinal1),
            _ => None,
        }
    }

    /// Check if this is a countdown-related pattern.
    pub fn is_countdown_pattern(&self) -> bool {
        matches!(
            self,
            CuePattern::CountdownTick
                | CuePattern::CountdownTick10
                | CuePattern::CountdownTick5
                | CuePattern::CountdownFinal3
                | CuePattern::CountdownFinal2
                | CuePattern::CountdownFinal1
        )
    }

    /// Check if this is a final countdown pattern (3, 2, or 1 second).
    pub fn is_final_countdown(&self) -> bool {
        matches!(
            self,
            CuePattern::CountdownFinal3 | CuePattern::CountdownFinal2 | CuePattern::CountdownFinal1
        )
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

        let sink =
            Sink::try_new(&stream_handle).map_err(|e| ToneError::PlaybackError(e.to_string()))?;

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

    #[test]
    fn test_countdown_pattern_for_seconds() {
        // Test that we get the correct pattern for each countdown second
        assert_eq!(
            CuePattern::for_countdown_seconds(10),
            Some(CuePattern::CountdownTick10)
        );
        assert_eq!(
            CuePattern::for_countdown_seconds(5),
            Some(CuePattern::CountdownTick5)
        );
        assert_eq!(
            CuePattern::for_countdown_seconds(3),
            Some(CuePattern::CountdownFinal3)
        );
        assert_eq!(
            CuePattern::for_countdown_seconds(2),
            Some(CuePattern::CountdownFinal2)
        );
        assert_eq!(
            CuePattern::for_countdown_seconds(1),
            Some(CuePattern::CountdownFinal1)
        );

        // Test that other seconds return None
        assert_eq!(CuePattern::for_countdown_seconds(0), None);
        assert_eq!(CuePattern::for_countdown_seconds(4), None);
        assert_eq!(CuePattern::for_countdown_seconds(6), None);
        assert_eq!(CuePattern::for_countdown_seconds(7), None);
        assert_eq!(CuePattern::for_countdown_seconds(15), None);
    }

    #[test]
    fn test_countdown_pattern_durations_under_200ms() {
        // All countdown patterns should be under 200ms to not overlap with speech
        let countdown_patterns = [
            CuePattern::CountdownTick10,
            CuePattern::CountdownTick5,
            CuePattern::CountdownFinal3,
            CuePattern::CountdownFinal2,
            CuePattern::CountdownFinal1,
        ];

        for pattern in countdown_patterns {
            let duration = pattern.total_duration_ms();
            assert!(
                duration < 200,
                "{:?} has duration {}ms which is >= 200ms",
                pattern,
                duration
            );
        }
    }

    #[test]
    fn test_countdown_pattern_escalating_frequencies() {
        // Verify that final countdown frequencies escalate (higher frequency = more urgent)
        let freq_3 = frequencies::COUNTDOWN_3;
        let freq_2 = frequencies::COUNTDOWN_2;
        let freq_1 = frequencies::COUNTDOWN_1;

        assert!(
            freq_2 > freq_3,
            "Countdown 2 frequency ({}) should be > Countdown 3 frequency ({})",
            freq_2,
            freq_3
        );
        assert!(
            freq_1 > freq_2,
            "Countdown 1 frequency ({}) should be > Countdown 2 frequency ({})",
            freq_1,
            freq_2
        );
    }

    #[test]
    fn test_is_countdown_pattern() {
        // Test positive cases
        assert!(CuePattern::CountdownTick.is_countdown_pattern());
        assert!(CuePattern::CountdownTick10.is_countdown_pattern());
        assert!(CuePattern::CountdownTick5.is_countdown_pattern());
        assert!(CuePattern::CountdownFinal3.is_countdown_pattern());
        assert!(CuePattern::CountdownFinal2.is_countdown_pattern());
        assert!(CuePattern::CountdownFinal1.is_countdown_pattern());

        // Test negative cases
        assert!(!CuePattern::SingleBeep.is_countdown_pattern());
        assert!(!CuePattern::Alert.is_countdown_pattern());
        assert!(!CuePattern::Success.is_countdown_pattern());
    }

    #[test]
    fn test_is_final_countdown() {
        // Test positive cases
        assert!(CuePattern::CountdownFinal3.is_final_countdown());
        assert!(CuePattern::CountdownFinal2.is_final_countdown());
        assert!(CuePattern::CountdownFinal1.is_final_countdown());

        // Test negative cases (including other countdown patterns)
        assert!(!CuePattern::CountdownTick.is_final_countdown());
        assert!(!CuePattern::CountdownTick10.is_final_countdown());
        assert!(!CuePattern::CountdownTick5.is_final_countdown());
        assert!(!CuePattern::SingleBeep.is_final_countdown());
    }

    #[test]
    fn test_countdown_patterns_have_distinct_sounds() {
        // Verify each countdown pattern produces different tones
        let patterns = [
            CuePattern::CountdownTick10,
            CuePattern::CountdownTick5,
            CuePattern::CountdownFinal3,
            CuePattern::CountdownFinal2,
            CuePattern::CountdownFinal1,
        ];

        for i in 0..patterns.len() {
            for j in (i + 1)..patterns.len() {
                let tones_i = patterns[i].tones();
                let tones_j = patterns[j].tones();

                // They should differ in either frequency, duration, or number of tones
                let different = tones_i.len() != tones_j.len()
                    || tones_i.iter().zip(tones_j.iter()).any(|(a, b)| {
                        (a.frequency_hz - b.frequency_hz).abs() > 0.01
                            || a.duration_ms != b.duration_ms
                    });

                assert!(
                    different,
                    "{:?} and {:?} should have distinct sounds",
                    patterns[i], patterns[j]
                );
            }
        }
    }
}
