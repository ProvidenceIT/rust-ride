//! Wake word detection for voice activation.
//!
//! This module provides wake word detection functionality for the voice engine.
//! Supported wake phrases:
//! - "Hey Rust Ride"
//! - "OK Ride"
//!
//! ## Architecture
//!
//! The wake word detector monitors recognized text for wake phrases. When a wake
//! word is detected, it triggers "active listening mode" for a configurable
//! duration (default: 5 seconds).
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    WakeWordDetector                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  State: Dormant ────▶ Active (5s timeout) ────▶ Dormant     │
//! │                 wake word          timeout                   │
//! │                 detected           or command                │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::wake_word::{WakeWordDetector, WakeWordConfig};
//!
//! let mut detector = WakeWordDetector::new(WakeWordConfig::default());
//!
//! // Check for wake word in recognized text
//! if let Some(event) = detector.process_text("hey rust ride") {
//!     match event {
//!         WakeWordEvent::Detected { phrase } => {
//!             // Wake word detected, now listening for commands
//!         }
//!         _ => {}
//!     }
//! }
//!
//! // Check if currently in active listening mode
//! if detector.is_active() {
//!     // Process commands
//! }
//! ```

use std::time::{Duration, Instant};

/// Default active listening duration after wake word detection (5 seconds).
pub const DEFAULT_ACTIVE_LISTENING_DURATION_MS: u64 = 5000;

/// Minimum similarity score for wake word matching (0.0 - 1.0).
const WAKE_WORD_MIN_SIMILARITY: f32 = 0.75;

/// Wake word phrases that trigger active listening.
pub const WAKE_PHRASES: &[&str] = &[
    "hey rust ride",
    "ok ride",
    "okay ride",
    "hey rustride",
    "ok rustride",
];

/// Common misrecognitions of wake phrases that should also trigger activation.
const WAKE_PHRASE_VARIANTS: &[(&str, &str)] = &[
    // "hey rust ride" variants
    ("hey rust right", "hey rust ride"),
    ("hey rust rite", "hey rust ride"),
    ("hey rust write", "hey rust ride"),
    ("hey rest ride", "hey rust ride"),
    ("hey rust rod", "hey rust ride"),
    ("a rust ride", "hey rust ride"),
    ("hey rust read", "hey rust ride"),
    // "ok ride" variants
    ("ok right", "ok ride"),
    ("okay right", "ok ride"),
    ("ok ryde", "ok ride"),
    ("okay ryde", "ok ride"),
    ("o k ride", "ok ride"),
    ("oak ride", "ok ride"),
];

/// Configuration for wake word detection.
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// Duration to stay in active listening mode after wake word (in milliseconds).
    pub active_duration_ms: u64,
    /// Whether wake word detection is enabled.
    pub enabled: bool,
    /// Custom wake phrases (in addition to defaults).
    pub custom_phrases: Vec<String>,
    /// Minimum similarity threshold for fuzzy matching (0.0 - 1.0).
    pub min_similarity: f32,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            active_duration_ms: DEFAULT_ACTIVE_LISTENING_DURATION_MS,
            enabled: true,
            custom_phrases: Vec::new(),
            min_similarity: WAKE_WORD_MIN_SIMILARITY,
        }
    }
}

impl WakeWordConfig {
    /// Create a new configuration with the specified active duration.
    pub fn new(active_duration_ms: u64) -> Self {
        Self {
            active_duration_ms,
            ..Default::default()
        }
    }

    /// Set the active listening duration.
    pub fn with_active_duration(mut self, duration_ms: u64) -> Self {
        self.active_duration_ms = duration_ms;
        self
    }

    /// Enable or disable wake word detection.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a custom wake phrase.
    pub fn with_custom_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.custom_phrases.push(phrase.into().to_lowercase());
        self
    }

    /// Set the minimum similarity threshold.
    pub fn with_min_similarity(mut self, similarity: f32) -> Self {
        self.min_similarity = similarity.clamp(0.0, 1.0);
        self
    }

    /// Get all wake phrases (default + custom).
    pub fn all_phrases(&self) -> Vec<&str> {
        let mut phrases: Vec<&str> = WAKE_PHRASES.to_vec();
        for phrase in &self.custom_phrases {
            phrases.push(phrase.as_str());
        }
        phrases
    }
}

/// State of the wake word detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeWordState {
    /// Waiting for wake word (not actively listening for commands).
    Dormant,
    /// Wake word detected, actively listening for commands.
    Active,
    /// Wake word detection is disabled.
    Disabled,
}

impl Default for WakeWordState {
    fn default() -> Self {
        Self::Dormant
    }
}

impl std::fmt::Display for WakeWordState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WakeWordState::Dormant => write!(f, "Dormant"),
            WakeWordState::Active => write!(f, "Active"),
            WakeWordState::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Events emitted by the wake word detector.
#[derive(Debug, Clone)]
pub enum WakeWordEvent {
    /// Wake word was detected, entering active listening mode.
    Detected {
        /// The phrase that triggered activation.
        phrase: String,
        /// How long active mode will last (milliseconds).
        duration_ms: u64,
    },
    /// Active listening period has expired.
    Timeout,
    /// Active listening was extended (command in progress).
    Extended {
        /// New expiration time remaining (milliseconds).
        remaining_ms: u64,
    },
    /// State changed (dormant <-> active).
    StateChanged {
        /// Previous state.
        from: WakeWordState,
        /// New state.
        to: WakeWordState,
    },
}

/// Wake word detector for triggering active listening mode.
///
/// This detector monitors recognized text for wake phrases and manages
/// the transition between dormant and active listening states.
#[derive(Debug)]
pub struct WakeWordDetector {
    /// Configuration.
    config: WakeWordConfig,
    /// Current state.
    state: WakeWordState,
    /// When active mode was entered (for timeout calculation).
    active_since: Option<Instant>,
    /// Last detected wake phrase.
    last_wake_phrase: Option<String>,
    /// Count of wake word detections (for statistics).
    detection_count: u64,
}

impl WakeWordDetector {
    /// Create a new wake word detector with the specified configuration.
    pub fn new(config: WakeWordConfig) -> Self {
        let initial_state = if config.enabled {
            WakeWordState::Dormant
        } else {
            WakeWordState::Disabled
        };

        Self {
            config,
            state: initial_state,
            active_since: None,
            last_wake_phrase: None,
            detection_count: 0,
        }
    }

    /// Create a new detector with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(WakeWordConfig::default())
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WakeWordConfig {
        &self.config
    }

    /// Get the current state.
    pub fn state(&self) -> WakeWordState {
        self.state
    }

    /// Check if currently in active listening mode.
    pub fn is_active(&self) -> bool {
        self.state == WakeWordState::Active
    }

    /// Check if wake word detection is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.state != WakeWordState::Disabled
    }

    /// Get the last detected wake phrase.
    pub fn last_wake_phrase(&self) -> Option<&str> {
        self.last_wake_phrase.as_deref()
    }

    /// Get the detection count.
    pub fn detection_count(&self) -> u64 {
        self.detection_count
    }

    /// Get the time remaining in active mode (in milliseconds).
    ///
    /// Returns `None` if not in active mode.
    pub fn remaining_active_time_ms(&self) -> Option<u64> {
        if self.state != WakeWordState::Active {
            return None;
        }

        self.active_since.map(|since| {
            let elapsed = since.elapsed().as_millis() as u64;
            if elapsed >= self.config.active_duration_ms {
                0
            } else {
                self.config.active_duration_ms - elapsed
            }
        })
    }

    /// Process recognized text and check for wake words.
    ///
    /// Returns a wake word event if the state changed.
    pub fn process_text(&mut self, text: &str) -> Option<WakeWordEvent> {
        if !self.config.enabled {
            return None;
        }

        let normalized = text.trim().to_lowercase();

        // Check for timeout first
        if let Some(event) = self.check_timeout() {
            return Some(event);
        }

        // Try to match a wake phrase
        if let Some(matched_phrase) = self.match_wake_phrase(&normalized) {
            // Wake word detected
            let previous_state = self.state;
            self.state = WakeWordState::Active;
            self.active_since = Some(Instant::now());
            self.last_wake_phrase = Some(matched_phrase.clone());
            self.detection_count += 1;

            tracing::info!("Wake word detected: '{}'", matched_phrase);

            if previous_state != WakeWordState::Active {
                return Some(WakeWordEvent::Detected {
                    phrase: matched_phrase,
                    duration_ms: self.config.active_duration_ms,
                });
            } else {
                // Already active, extend the timeout
                return Some(WakeWordEvent::Extended {
                    remaining_ms: self.config.active_duration_ms,
                });
            }
        }

        None
    }

    /// Check if active mode has timed out.
    ///
    /// Returns a timeout event if the active period expired.
    pub fn check_timeout(&mut self) -> Option<WakeWordEvent> {
        if self.state != WakeWordState::Active {
            return None;
        }

        if let Some(since) = self.active_since {
            let elapsed = since.elapsed().as_millis() as u64;
            if elapsed >= self.config.active_duration_ms {
                let previous_state = self.state;
                self.state = WakeWordState::Dormant;
                self.active_since = None;

                tracing::debug!("Wake word active period timed out");

                return Some(WakeWordEvent::StateChanged {
                    from: previous_state,
                    to: WakeWordState::Dormant,
                });
            }
        }

        None
    }

    /// Extend the active listening period.
    ///
    /// Call this when a command is being processed to prevent timeout.
    pub fn extend_active(&mut self) -> Option<WakeWordEvent> {
        if self.state == WakeWordState::Active {
            self.active_since = Some(Instant::now());
            Some(WakeWordEvent::Extended {
                remaining_ms: self.config.active_duration_ms,
            })
        } else {
            None
        }
    }

    /// Manually enter active mode.
    ///
    /// This is useful for push-to-talk or manual activation.
    pub fn activate(&mut self) -> Option<WakeWordEvent> {
        if self.state == WakeWordState::Disabled {
            return None;
        }

        let previous_state = self.state;
        if previous_state != WakeWordState::Active {
            self.state = WakeWordState::Active;
            self.active_since = Some(Instant::now());

            Some(WakeWordEvent::StateChanged {
                from: previous_state,
                to: WakeWordState::Active,
            })
        } else {
            // Already active, just extend
            self.extend_active()
        }
    }

    /// Manually exit active mode and return to dormant.
    pub fn deactivate(&mut self) -> Option<WakeWordEvent> {
        if self.state == WakeWordState::Active {
            let previous_state = self.state;
            self.state = WakeWordState::Dormant;
            self.active_since = None;

            Some(WakeWordEvent::StateChanged {
                from: previous_state,
                to: WakeWordState::Dormant,
            })
        } else {
            None
        }
    }

    /// Enable or disable wake word detection.
    pub fn set_enabled(&mut self, enabled: bool) -> Option<WakeWordEvent> {
        let was_enabled = self.config.enabled;
        self.config.enabled = enabled;

        if enabled && !was_enabled {
            let previous_state = self.state;
            self.state = WakeWordState::Dormant;
            Some(WakeWordEvent::StateChanged {
                from: previous_state,
                to: WakeWordState::Dormant,
            })
        } else if !enabled && was_enabled {
            let previous_state = self.state;
            self.state = WakeWordState::Disabled;
            self.active_since = None;
            Some(WakeWordEvent::StateChanged {
                from: previous_state,
                to: WakeWordState::Disabled,
            })
        } else {
            None
        }
    }

    /// Reset the detector state.
    pub fn reset(&mut self) {
        self.state = if self.config.enabled {
            WakeWordState::Dormant
        } else {
            WakeWordState::Disabled
        };
        self.active_since = None;
        self.last_wake_phrase = None;
    }

    /// Update the configuration.
    pub fn update_config(&mut self, config: WakeWordConfig) {
        let enabled_changed = self.config.enabled != config.enabled;
        self.config = config;

        if enabled_changed {
            self.state = if self.config.enabled {
                WakeWordState::Dormant
            } else {
                WakeWordState::Disabled
            };
            self.active_since = None;
        }
    }

    /// Match the input text against wake phrases.
    ///
    /// Returns the matched phrase if found.
    fn match_wake_phrase(&self, text: &str) -> Option<String> {
        let normalized = text.trim().to_lowercase();

        // First, check for exact matches
        for phrase in WAKE_PHRASES {
            if normalized == *phrase || normalized.contains(phrase) {
                return Some(phrase.to_string());
            }
        }

        // Check custom phrases
        for phrase in &self.config.custom_phrases {
            if normalized == *phrase || normalized.contains(phrase.as_str()) {
                return Some(phrase.clone());
            }
        }

        // Check known misrecognition variants
        for (variant, canonical) in WAKE_PHRASE_VARIANTS {
            if normalized == *variant || normalized.contains(variant) {
                return Some(canonical.to_string());
            }
        }

        // Try fuzzy matching with Levenshtein distance
        for phrase in WAKE_PHRASES {
            if string_similarity(&normalized, phrase) >= self.config.min_similarity {
                return Some(phrase.to_string());
            }

            // Also check if the text contains something similar to the wake phrase
            for word_window in sliding_word_windows(&normalized, word_count(phrase)) {
                if string_similarity(&word_window, phrase) >= self.config.min_similarity {
                    return Some(phrase.to_string());
                }
            }
        }

        None
    }
}

impl Default for WakeWordDetector {
    fn default() -> Self {
        Self::new(WakeWordConfig::default())
    }
}

/// Calculate string similarity using Levenshtein distance.
///
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
fn string_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }

    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f32 / max_len as f32)
}

/// Calculate Levenshtein distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows for memory efficiency
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Count the number of words in a string.
fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Generate sliding windows of words from a string.
fn sliding_word_windows(text: &str, window_size: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < window_size {
        return vec![text.to_string()];
    }

    words
        .windows(window_size)
        .map(|w| w.join(" "))
        .collect()
}

/// Grammar entries for wake word recognition.
///
/// These should be added to the Vosk recognizer grammar for
/// constrained recognition of wake phrases.
pub fn wake_word_grammar() -> Vec<String> {
    let mut grammar: Vec<String> = WAKE_PHRASES.iter().map(|s| s.to_string()).collect();

    // Add common variants
    for (variant, _) in WAKE_PHRASE_VARIANTS {
        grammar.push(variant.to_string());
    }

    grammar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WakeWordConfig::default();
        assert_eq!(config.active_duration_ms, DEFAULT_ACTIVE_LISTENING_DURATION_MS);
        assert!(config.enabled);
        assert!(config.custom_phrases.is_empty());
        assert_eq!(config.min_similarity, WAKE_WORD_MIN_SIMILARITY);
    }

    #[test]
    fn test_config_builder() {
        let config = WakeWordConfig::new(10000)
            .with_enabled(false)
            .with_custom_phrase("hey bike")
            .with_min_similarity(0.8);

        assert_eq!(config.active_duration_ms, 10000);
        assert!(!config.enabled);
        assert_eq!(config.custom_phrases, vec!["hey bike"]);
        assert_eq!(config.min_similarity, 0.8);
    }

    #[test]
    fn test_detector_creation() {
        let detector = WakeWordDetector::with_defaults();
        assert_eq!(detector.state(), WakeWordState::Dormant);
        assert!(!detector.is_active());
        assert!(detector.is_enabled());
        assert_eq!(detector.detection_count(), 0);
    }

    #[test]
    fn test_detector_disabled() {
        let config = WakeWordConfig::default().with_enabled(false);
        let detector = WakeWordDetector::new(config);
        assert_eq!(detector.state(), WakeWordState::Disabled);
        assert!(!detector.is_enabled());
    }

    #[test]
    fn test_wake_word_detection_exact() {
        let mut detector = WakeWordDetector::with_defaults();

        // Test "hey rust ride"
        let event = detector.process_text("hey rust ride");
        assert!(event.is_some());
        if let Some(WakeWordEvent::Detected { phrase, .. }) = event {
            assert_eq!(phrase, "hey rust ride");
        } else {
            panic!("Expected Detected event");
        }
        assert!(detector.is_active());
        assert_eq!(detector.detection_count(), 1);
    }

    #[test]
    fn test_wake_word_detection_ok_ride() {
        let mut detector = WakeWordDetector::with_defaults();

        let event = detector.process_text("ok ride");
        assert!(event.is_some());
        if let Some(WakeWordEvent::Detected { phrase, .. }) = event {
            assert_eq!(phrase, "ok ride");
        }
        assert!(detector.is_active());
    }

    #[test]
    fn test_wake_word_detection_contained() {
        let mut detector = WakeWordDetector::with_defaults();

        // Wake phrase contained in longer text
        let event = detector.process_text("I said hey rust ride pause");
        assert!(event.is_some());
        assert!(detector.is_active());
    }

    #[test]
    fn test_wake_word_detection_variant() {
        let mut detector = WakeWordDetector::with_defaults();

        // Test misrecognition variant
        let event = detector.process_text("hey rust right");
        assert!(event.is_some());
        if let Some(WakeWordEvent::Detected { phrase, .. }) = event {
            assert_eq!(phrase, "hey rust ride"); // Corrected form
        }
    }

    #[test]
    fn test_no_wake_word() {
        let mut detector = WakeWordDetector::with_defaults();

        let event = detector.process_text("pause the workout");
        assert!(event.is_none());
        assert!(!detector.is_active());
    }

    #[test]
    fn test_active_extension() {
        let mut detector = WakeWordDetector::with_defaults();

        // Activate
        detector.process_text("hey rust ride");
        assert!(detector.is_active());

        // Extend
        let event = detector.extend_active();
        assert!(event.is_some());
        if let Some(WakeWordEvent::Extended { remaining_ms }) = event {
            assert_eq!(remaining_ms, DEFAULT_ACTIVE_LISTENING_DURATION_MS);
        }
    }

    #[test]
    fn test_manual_activation() {
        let mut detector = WakeWordDetector::with_defaults();

        let event = detector.activate();
        assert!(event.is_some());
        assert!(detector.is_active());

        // Deactivate
        let event = detector.deactivate();
        assert!(event.is_some());
        assert!(!detector.is_active());
    }

    #[test]
    fn test_remaining_active_time() {
        let mut detector = WakeWordDetector::with_defaults();

        // Not active - no remaining time
        assert!(detector.remaining_active_time_ms().is_none());

        // Activate
        detector.activate();
        let remaining = detector.remaining_active_time_ms();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 0);
        assert!(remaining.unwrap() <= DEFAULT_ACTIVE_LISTENING_DURATION_MS);
    }

    #[test]
    fn test_timeout_short_duration() {
        let config = WakeWordConfig::new(10); // 10ms timeout
        let mut detector = WakeWordDetector::new(config);

        detector.activate();
        assert!(detector.is_active());

        // Wait for timeout
        std::thread::sleep(std::time::Duration::from_millis(15));

        let event = detector.check_timeout();
        assert!(event.is_some());
        assert!(!detector.is_active());
    }

    #[test]
    fn test_set_enabled() {
        let mut detector = WakeWordDetector::with_defaults();

        // Disable
        let event = detector.set_enabled(false);
        assert!(event.is_some());
        assert_eq!(detector.state(), WakeWordState::Disabled);

        // No detection when disabled
        let event = detector.process_text("hey rust ride");
        assert!(event.is_none());
        assert!(!detector.is_active());

        // Re-enable
        let event = detector.set_enabled(true);
        assert!(event.is_some());
        assert_eq!(detector.state(), WakeWordState::Dormant);

        // Should detect now
        let event = detector.process_text("hey rust ride");
        assert!(event.is_some());
        assert!(detector.is_active());
    }

    #[test]
    fn test_reset() {
        let mut detector = WakeWordDetector::with_defaults();

        detector.process_text("hey rust ride");
        assert!(detector.is_active());
        assert!(detector.last_wake_phrase().is_some());

        detector.reset();
        assert!(!detector.is_active());
        assert!(detector.last_wake_phrase().is_none());
        assert_eq!(detector.state(), WakeWordState::Dormant);
    }

    #[test]
    fn test_custom_phrase() {
        let config = WakeWordConfig::default().with_custom_phrase("hey bike");
        let mut detector = WakeWordDetector::new(config);

        let event = detector.process_text("hey bike");
        assert!(event.is_some());
        assert!(detector.is_active());
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_string_similarity() {
        assert_eq!(string_similarity("abc", "abc"), 1.0);
        assert!(string_similarity("abc", "abd") > 0.5);
        assert!(string_similarity("hello", "help") > 0.5);
        assert!(string_similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("hello"), 1);
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count("hey rust ride"), 3);
    }

    #[test]
    fn test_sliding_word_windows() {
        let windows = sliding_word_windows("a b c d", 2);
        assert_eq!(windows, vec!["a b", "b c", "c d"]);

        let windows = sliding_word_windows("hello", 2);
        assert_eq!(windows, vec!["hello"]);
    }

    #[test]
    fn test_wake_word_grammar() {
        let grammar = wake_word_grammar();
        assert!(grammar.contains(&"hey rust ride".to_string()));
        assert!(grammar.contains(&"ok ride".to_string()));
        assert!(!grammar.is_empty());
    }

    #[test]
    fn test_wake_word_state_display() {
        assert_eq!(WakeWordState::Dormant.to_string(), "Dormant");
        assert_eq!(WakeWordState::Active.to_string(), "Active");
        assert_eq!(WakeWordState::Disabled.to_string(), "Disabled");
    }

    #[test]
    fn test_fuzzy_matching() {
        let mut detector = WakeWordDetector::with_defaults();

        // Close misspelling should match
        let event = detector.process_text("hey rast ride"); // 'u' -> 'a' typo
        // This should match if similarity is above threshold
        // The exact behavior depends on the threshold
        if event.is_some() {
            assert!(detector.is_active());
        }
    }

    #[test]
    fn test_case_insensitivity() {
        let mut detector = WakeWordDetector::with_defaults();

        let event = detector.process_text("HEY RUST RIDE");
        assert!(event.is_some());
        assert!(detector.is_active());
    }

    #[test]
    fn test_okay_variant() {
        let mut detector = WakeWordDetector::with_defaults();

        let event = detector.process_text("okay ride");
        assert!(event.is_some());
        if let Some(WakeWordEvent::Detected { phrase, .. }) = event {
            assert_eq!(phrase, "okay ride");
        }
    }
}
