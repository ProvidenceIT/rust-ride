//! Push-to-talk functionality for voice control.
//!
//! This module provides keyboard shortcut handling for push-to-talk voice activation.
//! When the configured key is held down, audio capture and recognition are active.
//! When released, the final result is processed.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rustride::voice::push_to_talk::{PushToTalkHandler, PushToTalkConfig, PushToTalkKey};
//! use rustride::voice::VoiceEngine;
//!
//! let config = PushToTalkConfig::default(); // F4 key
//! let handler = PushToTalkHandler::new(config);
//!
//! // In your input handling loop:
//! if handler.handle_key_event(key, is_pressed) {
//!     if is_pressed {
//!         engine.activate()?;
//!     } else {
//!         engine.deactivate()?;
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use egui::Key;

/// Default push-to-talk key (F4).
pub const DEFAULT_PUSH_TO_TALK_KEY: Key = Key::F4;

/// Minimum hold duration before registering as valid push-to-talk (in milliseconds).
/// This prevents accidental activations from quick key presses.
pub const DEFAULT_MIN_HOLD_DURATION_MS: u64 = 100;

/// Maximum hold duration for push-to-talk (in milliseconds).
/// After this duration, the push-to-talk will auto-release to prevent stuck states.
pub const DEFAULT_MAX_HOLD_DURATION_MS: u64 = 30000; // 30 seconds

/// Push-to-talk key definition.
///
/// Wraps an egui Key for push-to-talk functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PushToTalkKey {
    /// The key to use for push-to-talk.
    pub key: Key,
}

impl PushToTalkKey {
    /// Create a new push-to-talk key.
    pub fn new(key: Key) -> Self {
        Self { key }
    }

    /// Check if this key matches a pressed key.
    pub fn matches(&self, key: Key) -> bool {
        self.key == key
    }

    /// Get the display name for this key.
    pub fn display(&self) -> String {
        key_display_name(self.key).to_string()
    }
}

impl Default for PushToTalkKey {
    fn default() -> Self {
        Self {
            key: DEFAULT_PUSH_TO_TALK_KEY,
        }
    }
}

impl From<Key> for PushToTalkKey {
    fn from(key: Key) -> Self {
        Self::new(key)
    }
}

/// Configuration for push-to-talk functionality.
#[derive(Debug, Clone)]
pub struct PushToTalkConfig {
    /// The key binding for push-to-talk.
    pub key: PushToTalkKey,
    /// Minimum hold duration before registering as valid push-to-talk (milliseconds).
    pub min_hold_duration_ms: u64,
    /// Maximum hold duration before auto-release (milliseconds).
    pub max_hold_duration_ms: u64,
    /// Whether push-to-talk is enabled.
    pub enabled: bool,
    /// Whether to play a tone when push-to-talk is activated.
    pub play_activation_tone: bool,
    /// Whether to play a tone when push-to-talk is deactivated.
    pub play_deactivation_tone: bool,
}

impl Default for PushToTalkConfig {
    fn default() -> Self {
        Self {
            key: PushToTalkKey::default(),
            min_hold_duration_ms: DEFAULT_MIN_HOLD_DURATION_MS,
            max_hold_duration_ms: DEFAULT_MAX_HOLD_DURATION_MS,
            enabled: true,
            play_activation_tone: true,
            play_deactivation_tone: true,
        }
    }
}

impl PushToTalkConfig {
    /// Create a new config with the specified key.
    pub fn new(key: Key) -> Self {
        Self {
            key: PushToTalkKey::new(key),
            ..Default::default()
        }
    }

    /// Set the key binding.
    pub fn with_key(mut self, key: Key) -> Self {
        self.key = PushToTalkKey::new(key);
        self
    }

    /// Set the minimum hold duration.
    pub fn with_min_hold_duration(mut self, duration_ms: u64) -> Self {
        self.min_hold_duration_ms = duration_ms;
        self
    }

    /// Set the maximum hold duration.
    pub fn with_max_hold_duration(mut self, duration_ms: u64) -> Self {
        self.max_hold_duration_ms = duration_ms;
        self
    }

    /// Enable or disable push-to-talk.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Enable or disable activation tone.
    pub fn with_activation_tone(mut self, play: bool) -> Self {
        self.play_activation_tone = play;
        self
    }

    /// Enable or disable deactivation tone.
    pub fn with_deactivation_tone(mut self, play: bool) -> Self {
        self.play_deactivation_tone = play;
        self
    }
}

/// State of the push-to-talk key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushToTalkState {
    /// Key is not pressed, not listening.
    Idle,
    /// Key is pressed but minimum hold duration not yet reached.
    Pending,
    /// Key is held and listening is active.
    Active,
    /// Key was released, processing final result.
    Releasing,
}

impl Default for PushToTalkState {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for PushToTalkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushToTalkState::Idle => write!(f, "Idle"),
            PushToTalkState::Pending => write!(f, "Pending"),
            PushToTalkState::Active => write!(f, "Active"),
            PushToTalkState::Releasing => write!(f, "Releasing"),
        }
    }
}

/// Events emitted by the push-to-talk handler.
#[derive(Debug, Clone)]
pub enum PushToTalkEvent {
    /// Push-to-talk key was pressed and minimum hold duration reached.
    /// Voice engine should be activated.
    Activated,
    /// Push-to-talk key was released after being active.
    /// Voice engine should be deactivated and process final result.
    Deactivated {
        /// Duration the key was held (milliseconds).
        hold_duration_ms: u64,
    },
    /// Push-to-talk was cancelled (key released before minimum hold).
    Cancelled,
    /// Push-to-talk timed out (exceeded maximum hold duration).
    TimedOut {
        /// Duration the key was held before timeout (milliseconds).
        hold_duration_ms: u64,
    },
    /// Key binding was changed.
    KeyBindingChanged {
        /// The new key binding.
        key: PushToTalkKey,
    },
}

/// Handler for push-to-talk keyboard shortcuts.
///
/// This handler tracks the state of the push-to-talk key and emits events
/// when the key is pressed and released. It supports configurable key bindings
/// and minimum/maximum hold durations.
pub struct PushToTalkHandler {
    /// Configuration for push-to-talk.
    config: RwLock<PushToTalkConfig>,
    /// Current state.
    state: RwLock<PushToTalkState>,
    /// Whether the key is currently pressed.
    key_pressed: AtomicBool,
    /// When the key was pressed (if currently pressed).
    press_time: RwLock<Option<Instant>>,
}

impl PushToTalkHandler {
    /// Create a new push-to-talk handler with the given configuration.
    pub fn new(config: PushToTalkConfig) -> Self {
        Self {
            config: RwLock::new(config),
            state: RwLock::new(PushToTalkState::Idle),
            key_pressed: AtomicBool::new(false),
            press_time: RwLock::new(None),
        }
    }

    /// Create a handler with default configuration (F4 key).
    pub fn with_default_key() -> Self {
        Self::new(PushToTalkConfig::default())
    }

    /// Get the current configuration.
    pub fn config(&self) -> PushToTalkConfig {
        self.config.read().unwrap().clone()
    }

    /// Update the configuration.
    pub fn update_config(&self, config: PushToTalkConfig) {
        *self.config.write().unwrap() = config;
    }

    /// Get the current key binding.
    pub fn key(&self) -> PushToTalkKey {
        self.config.read().unwrap().key
    }

    /// Set the key binding.
    pub fn set_key(&self, key: Key) -> PushToTalkEvent {
        let new_key = PushToTalkKey::new(key);
        self.config.write().unwrap().key = new_key;
        PushToTalkEvent::KeyBindingChanged { key: new_key }
    }

    /// Check if push-to-talk is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.read().unwrap().enabled
    }

    /// Enable or disable push-to-talk.
    pub fn set_enabled(&self, enabled: bool) {
        self.config.write().unwrap().enabled = enabled;

        // If disabling while active, reset state
        if !enabled && self.is_active() {
            self.reset();
        }
    }

    /// Get the current state.
    pub fn state(&self) -> PushToTalkState {
        *self.state.read().unwrap()
    }

    /// Check if currently in active listening mode.
    pub fn is_active(&self) -> bool {
        matches!(self.state(), PushToTalkState::Active)
    }

    /// Check if the key is currently pressed.
    pub fn is_key_pressed(&self) -> bool {
        self.key_pressed.load(Ordering::Acquire)
    }

    /// Get the duration the key has been held (if pressed).
    pub fn hold_duration(&self) -> Option<Duration> {
        self.press_time.read().unwrap().map(|t| t.elapsed())
    }

    /// Get the hold duration in milliseconds (if pressed).
    pub fn hold_duration_ms(&self) -> Option<u64> {
        self.hold_duration().map(|d| d.as_millis() as u64)
    }

    /// Handle a key press event.
    ///
    /// Returns `Some(event)` if the key was handled and an event should be emitted.
    /// Returns `None` if the key was not the push-to-talk key or push-to-talk is disabled.
    pub fn handle_key_press(&self, key: Key) -> Option<PushToTalkEvent> {
        let config = self.config.read().unwrap();

        // Check if this is the push-to-talk key and it's enabled
        if !config.enabled || !config.key.matches(key) {
            return None;
        }

        // Already pressed? Ignore (key repeat)
        if self.key_pressed.load(Ordering::Acquire) {
            return None;
        }

        // Mark as pressed
        self.key_pressed.store(true, Ordering::Release);
        *self.press_time.write().unwrap() = Some(Instant::now());

        // If min hold duration is 0, go directly to active
        if config.min_hold_duration_ms == 0 {
            *self.state.write().unwrap() = PushToTalkState::Active;
            return Some(PushToTalkEvent::Activated);
        }

        // Enter pending state
        *self.state.write().unwrap() = PushToTalkState::Pending;

        None
    }

    /// Handle a key release event.
    ///
    /// Returns `Some(event)` if the key was handled and an event should be emitted.
    /// Returns `None` if the key was not the push-to-talk key.
    pub fn handle_key_release(&self, key: Key) -> Option<PushToTalkEvent> {
        let config = self.config.read().unwrap();

        // Check if this is the push-to-talk key
        if !config.key.matches(key) {
            return None;
        }

        // Not pressed? Ignore
        if !self.key_pressed.load(Ordering::Acquire) {
            return None;
        }

        let press_time = self.press_time.read().unwrap().take();
        let hold_duration_ms = press_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        // Clear the press time
        *self.press_time.write().unwrap() = None;
        self.key_pressed.store(false, Ordering::Release);

        let current_state = *self.state.read().unwrap();
        *self.state.write().unwrap() = PushToTalkState::Idle;

        match current_state {
            PushToTalkState::Pending => {
                // Released before minimum hold - cancel
                Some(PushToTalkEvent::Cancelled)
            }
            PushToTalkState::Active => {
                // Normal release after being active
                Some(PushToTalkEvent::Deactivated { hold_duration_ms })
            }
            _ => None,
        }
    }

    /// Update the handler state (call periodically to check for timeouts and state transitions).
    ///
    /// Returns `Some(event)` if a state transition occurred.
    pub fn update(&self) -> Option<PushToTalkEvent> {
        if !self.key_pressed.load(Ordering::Acquire) {
            return None;
        }

        let config = self.config.read().unwrap();
        let press_time = *self.press_time.read().unwrap();

        let Some(press_time) = press_time else {
            return None;
        };

        let hold_duration_ms = press_time.elapsed().as_millis() as u64;
        let current_state = *self.state.read().unwrap();

        match current_state {
            PushToTalkState::Pending => {
                // Check if minimum hold duration reached
                if hold_duration_ms >= config.min_hold_duration_ms {
                    *self.state.write().unwrap() = PushToTalkState::Active;
                    return Some(PushToTalkEvent::Activated);
                }
            }
            PushToTalkState::Active => {
                // Check for timeout
                if hold_duration_ms >= config.max_hold_duration_ms {
                    // Force release
                    *self.state.write().unwrap() = PushToTalkState::Idle;
                    *self.press_time.write().unwrap() = None;
                    self.key_pressed.store(false, Ordering::Release);
                    return Some(PushToTalkEvent::TimedOut { hold_duration_ms });
                }
            }
            _ => {}
        }

        None
    }

    /// Reset the handler state (e.g., when window loses focus).
    pub fn reset(&self) {
        self.key_pressed.store(false, Ordering::Release);
        *self.press_time.write().unwrap() = None;
        *self.state.write().unwrap() = PushToTalkState::Idle;
    }

    /// Handle a combined key event (press or release).
    ///
    /// This is a convenience method that combines `handle_key_press` and `handle_key_release`.
    ///
    /// # Arguments
    /// * `key` - The key that was pressed or released
    /// * `is_pressed` - Whether the key is now pressed (true) or released (false)
    ///
    /// Returns `Some(event)` if the key was handled and an event should be emitted.
    pub fn handle_key_event(&self, key: Key, is_pressed: bool) -> Option<PushToTalkEvent> {
        if is_pressed {
            self.handle_key_press(key)
        } else {
            self.handle_key_release(key)
        }
    }

    /// Check if a key is the configured push-to-talk key.
    pub fn is_push_to_talk_key(&self, key: Key) -> bool {
        self.config.read().unwrap().key.matches(key)
    }
}

impl Default for PushToTalkHandler {
    fn default() -> Self {
        Self::new(PushToTalkConfig::default())
    }
}

// Verify PushToTalkHandler is Send + Sync
fn _assert_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<PushToTalkHandler>();
    assert_sync::<PushToTalkHandler>();
}

/// Get display name for a key.
fn key_display_name(key: Key) -> &'static str {
    match key {
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Tab => "Tab",
        Key::Enter => "Enter",
        Key::Escape => "Escape",
        Key::Space => "Space",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_to_talk_key_default() {
        let key = PushToTalkKey::default();
        assert_eq!(key.key, Key::F4);
        assert_eq!(key.display(), "F4");
    }

    #[test]
    fn test_push_to_talk_key_matches() {
        let key = PushToTalkKey::new(Key::F4);
        assert!(key.matches(Key::F4));
        assert!(!key.matches(Key::F5));
    }

    #[test]
    fn test_push_to_talk_key_from() {
        let key: PushToTalkKey = Key::F5.into();
        assert_eq!(key.key, Key::F5);
    }

    #[test]
    fn test_config_default() {
        let config = PushToTalkConfig::default();
        assert_eq!(config.key.key, Key::F4);
        assert_eq!(config.min_hold_duration_ms, DEFAULT_MIN_HOLD_DURATION_MS);
        assert_eq!(config.max_hold_duration_ms, DEFAULT_MAX_HOLD_DURATION_MS);
        assert!(config.enabled);
        assert!(config.play_activation_tone);
        assert!(config.play_deactivation_tone);
    }

    #[test]
    fn test_config_builder() {
        let config = PushToTalkConfig::new(Key::F5)
            .with_min_hold_duration(200)
            .with_max_hold_duration(10000)
            .with_enabled(false)
            .with_activation_tone(false)
            .with_deactivation_tone(false);

        assert_eq!(config.key.key, Key::F5);
        assert_eq!(config.min_hold_duration_ms, 200);
        assert_eq!(config.max_hold_duration_ms, 10000);
        assert!(!config.enabled);
        assert!(!config.play_activation_tone);
        assert!(!config.play_deactivation_tone);
    }

    #[test]
    fn test_state_default() {
        assert_eq!(PushToTalkState::default(), PushToTalkState::Idle);
    }

    #[test]
    fn test_state_display() {
        assert_eq!(PushToTalkState::Idle.to_string(), "Idle");
        assert_eq!(PushToTalkState::Pending.to_string(), "Pending");
        assert_eq!(PushToTalkState::Active.to_string(), "Active");
        assert_eq!(PushToTalkState::Releasing.to_string(), "Releasing");
    }

    #[test]
    fn test_handler_default() {
        let handler = PushToTalkHandler::default();
        assert_eq!(handler.state(), PushToTalkState::Idle);
        assert!(!handler.is_active());
        assert!(!handler.is_key_pressed());
        assert!(handler.is_enabled());
    }

    #[test]
    fn test_handler_with_default_key() {
        let handler = PushToTalkHandler::with_default_key();
        assert_eq!(handler.key().key, Key::F4);
    }

    #[test]
    fn test_handler_key_press_wrong_key() {
        let handler = PushToTalkHandler::default();
        let event = handler.handle_key_press(Key::F5);
        assert!(event.is_none());
        assert_eq!(handler.state(), PushToTalkState::Idle);
    }

    #[test]
    fn test_handler_key_press_disabled() {
        let config = PushToTalkConfig::default().with_enabled(false);
        let handler = PushToTalkHandler::new(config);
        let event = handler.handle_key_press(Key::F4);
        assert!(event.is_none());
        assert_eq!(handler.state(), PushToTalkState::Idle);
    }

    #[test]
    fn test_handler_key_press_enters_pending() {
        let handler = PushToTalkHandler::default();
        let event = handler.handle_key_press(Key::F4);

        // With default min_hold_duration > 0, should enter pending
        assert!(event.is_none());
        assert_eq!(handler.state(), PushToTalkState::Pending);
        assert!(handler.is_key_pressed());
    }

    #[test]
    fn test_handler_key_press_immediate_activation() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);
        let event = handler.handle_key_press(Key::F4);

        // With min_hold_duration = 0, should immediately activate
        assert!(matches!(event, Some(PushToTalkEvent::Activated)));
        assert_eq!(handler.state(), PushToTalkState::Active);
        assert!(handler.is_active());
    }

    #[test]
    fn test_handler_key_release_from_pending() {
        let handler = PushToTalkHandler::default();

        // Press key
        handler.handle_key_press(Key::F4);
        assert_eq!(handler.state(), PushToTalkState::Pending);

        // Release immediately (before min hold duration)
        let event = handler.handle_key_release(Key::F4);
        assert!(matches!(event, Some(PushToTalkEvent::Cancelled)));
        assert_eq!(handler.state(), PushToTalkState::Idle);
        assert!(!handler.is_key_pressed());
    }

    #[test]
    fn test_handler_key_release_from_active() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);

        // Press key (immediately activates)
        handler.handle_key_press(Key::F4);
        assert_eq!(handler.state(), PushToTalkState::Active);

        // Release key
        let event = handler.handle_key_release(Key::F4);
        assert!(matches!(event, Some(PushToTalkEvent::Deactivated { .. })));
        assert_eq!(handler.state(), PushToTalkState::Idle);
    }

    #[test]
    fn test_handler_key_release_wrong_key() {
        let handler = PushToTalkHandler::default();
        handler.handle_key_press(Key::F4);

        let event = handler.handle_key_release(Key::F5);
        assert!(event.is_none());
        // Should still be in pressed state
        assert!(handler.is_key_pressed());
    }

    #[test]
    fn test_handler_update_pending_to_active() {
        let config = PushToTalkConfig::default().with_min_hold_duration(10);
        let handler = PushToTalkHandler::new(config);

        handler.handle_key_press(Key::F4);
        assert_eq!(handler.state(), PushToTalkState::Pending);

        // Wait for min hold duration
        std::thread::sleep(std::time::Duration::from_millis(15));

        let event = handler.update();
        assert!(matches!(event, Some(PushToTalkEvent::Activated)));
        assert_eq!(handler.state(), PushToTalkState::Active);
    }

    #[test]
    fn test_handler_update_timeout() {
        let config = PushToTalkConfig::default()
            .with_min_hold_duration(0)
            .with_max_hold_duration(10);
        let handler = PushToTalkHandler::new(config);

        handler.handle_key_press(Key::F4);
        assert_eq!(handler.state(), PushToTalkState::Active);

        // Wait for timeout
        std::thread::sleep(std::time::Duration::from_millis(15));

        let event = handler.update();
        assert!(matches!(event, Some(PushToTalkEvent::TimedOut { .. })));
        assert_eq!(handler.state(), PushToTalkState::Idle);
        assert!(!handler.is_key_pressed());
    }

    #[test]
    fn test_handler_reset() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);

        handler.handle_key_press(Key::F4);
        assert!(handler.is_active());

        handler.reset();
        assert_eq!(handler.state(), PushToTalkState::Idle);
        assert!(!handler.is_key_pressed());
        assert!(handler.hold_duration().is_none());
    }

    #[test]
    fn test_handler_set_key() {
        let handler = PushToTalkHandler::default();
        assert_eq!(handler.key().key, Key::F4);

        let event = handler.set_key(Key::F5);
        assert!(matches!(event, PushToTalkEvent::KeyBindingChanged { key } if key.key == Key::F5));
        assert_eq!(handler.key().key, Key::F5);
    }

    #[test]
    fn test_handler_set_enabled() {
        let handler = PushToTalkHandler::default();
        assert!(handler.is_enabled());

        handler.set_enabled(false);
        assert!(!handler.is_enabled());

        // Press should be ignored when disabled
        let event = handler.handle_key_press(Key::F4);
        assert!(event.is_none());
    }

    #[test]
    fn test_handler_set_enabled_resets_active_state() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);

        handler.handle_key_press(Key::F4);
        assert!(handler.is_active());

        // Disabling should reset the state
        handler.set_enabled(false);
        assert_eq!(handler.state(), PushToTalkState::Idle);
    }

    #[test]
    fn test_handler_handle_key_event() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);

        // Press
        let event = handler.handle_key_event(Key::F4, true);
        assert!(matches!(event, Some(PushToTalkEvent::Activated)));

        // Release
        let event = handler.handle_key_event(Key::F4, false);
        assert!(matches!(event, Some(PushToTalkEvent::Deactivated { .. })));
    }

    #[test]
    fn test_handler_is_push_to_talk_key() {
        let handler = PushToTalkHandler::default();
        assert!(handler.is_push_to_talk_key(Key::F4));
        assert!(!handler.is_push_to_talk_key(Key::F5));
    }

    #[test]
    fn test_handler_hold_duration() {
        let handler = PushToTalkHandler::default();
        assert!(handler.hold_duration().is_none());
        assert!(handler.hold_duration_ms().is_none());

        handler.handle_key_press(Key::F4);
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(handler.hold_duration().is_some());
        assert!(handler.hold_duration_ms().unwrap() >= 10);
    }

    #[test]
    fn test_handler_repeat_key_press_ignored() {
        let config = PushToTalkConfig::default().with_min_hold_duration(0);
        let handler = PushToTalkHandler::new(config);

        // First press
        let event = handler.handle_key_press(Key::F4);
        assert!(matches!(event, Some(PushToTalkEvent::Activated)));

        // Repeat press should be ignored
        let event = handler.handle_key_press(Key::F4);
        assert!(event.is_none());
    }

    #[test]
    fn test_handler_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<PushToTalkHandler>();
        assert_sync::<PushToTalkHandler>();
    }

    #[test]
    fn test_event_variants() {
        let _e1 = PushToTalkEvent::Activated;
        let _e2 = PushToTalkEvent::Deactivated { hold_duration_ms: 500 };
        let _e3 = PushToTalkEvent::Cancelled;
        let _e4 = PushToTalkEvent::TimedOut { hold_duration_ms: 30000 };
        let _e5 = PushToTalkEvent::KeyBindingChanged { key: PushToTalkKey::default() };
    }
}
