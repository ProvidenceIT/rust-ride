//! Keyboard input handling and shortcuts.
//!
//! Provides keyboard navigation, shortcut registration, and key handling.

use egui::{Key, Modifiers};
use std::collections::HashMap;

/// A keyboard shortcut definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyboardShortcut {
    /// The primary key
    pub key: Key,
    /// Required modifiers
    pub modifiers: Modifiers,
}

impl KeyboardShortcut {
    /// Create a new shortcut with just a key.
    pub fn new(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Create a shortcut with Ctrl modifier.
    pub fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    /// Create a shortcut with Alt modifier.
    pub fn alt(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::ALT,
        }
    }

    /// Create a shortcut with Shift modifier.
    pub fn shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::SHIFT,
        }
    }

    /// Create a shortcut with Ctrl+Shift modifiers.
    pub fn ctrl_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        }
    }

    /// Check if this shortcut matches the given input.
    pub fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        self.key == key && self.modifiers == modifiers
    }

    /// Get a display string for the shortcut.
    pub fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.ctrl {
            parts.push("Ctrl");
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }

        parts.push(key_name(self.key));

        parts.join("+")
    }
}

/// Actions that can be triggered by keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    // Navigation
    FocusNext,
    FocusPrevious,
    Activate,
    Cancel,

    // Display modes
    ToggleTvMode,
    ToggleFlowMode,
    CycleFlowMetric,

    // Ride control
    StartRide,
    PauseRide,
    EndRide,
    SkipInterval,

    // Metrics
    AnnounceMetrics,

    // Help
    ShowShortcuts,

    // Settings
    OpenSettings,
}

impl KeyAction {
    /// Get the default shortcut for this action.
    pub fn default_shortcut(&self) -> KeyboardShortcut {
        match self {
            KeyAction::FocusNext => KeyboardShortcut::new(Key::Tab),
            KeyAction::FocusPrevious => KeyboardShortcut::shift(Key::Tab),
            KeyAction::Activate => KeyboardShortcut::new(Key::Enter),
            KeyAction::Cancel => KeyboardShortcut::new(Key::Escape),

            KeyAction::ToggleTvMode => KeyboardShortcut::new(Key::T),
            KeyAction::ToggleFlowMode => KeyboardShortcut::new(Key::F),
            KeyAction::CycleFlowMetric => KeyboardShortcut::new(Key::M),

            KeyAction::StartRide => KeyboardShortcut::new(Key::S),
            KeyAction::PauseRide => KeyboardShortcut::new(Key::P),
            KeyAction::EndRide => KeyboardShortcut::new(Key::E),
            KeyAction::SkipInterval => KeyboardShortcut::new(Key::N),

            KeyAction::AnnounceMetrics => KeyboardShortcut::ctrl(Key::M),

            KeyAction::ShowShortcuts => KeyboardShortcut::new(Key::F1),

            KeyAction::OpenSettings => KeyboardShortcut::ctrl(Key::Comma),
        }
    }

    /// Get the description for this action.
    pub fn description(&self) -> &'static str {
        match self {
            KeyAction::FocusNext => "Move focus to next element",
            KeyAction::FocusPrevious => "Move focus to previous element",
            KeyAction::Activate => "Activate focused element",
            KeyAction::Cancel => "Cancel or close modal",

            KeyAction::ToggleTvMode => "Toggle TV Mode",
            KeyAction::ToggleFlowMode => "Toggle Flow Mode",
            KeyAction::CycleFlowMetric => "Cycle Flow Mode metric",

            KeyAction::StartRide => "Start ride",
            KeyAction::PauseRide => "Pause/resume ride",
            KeyAction::EndRide => "End ride",
            KeyAction::SkipInterval => "Skip to next interval",

            KeyAction::AnnounceMetrics => "Announce current metrics",

            KeyAction::ShowShortcuts => "Show keyboard shortcuts",

            KeyAction::OpenSettings => "Open settings",
        }
    }
}

/// Keyboard input handler.
pub struct KeyboardHandler {
    /// Registered shortcuts
    shortcuts: HashMap<KeyboardShortcut, KeyAction>,
    /// Whether keyboard navigation is enabled
    enabled: bool,
}

impl Default for KeyboardHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardHandler {
    /// Create a new keyboard handler with default shortcuts.
    pub fn new() -> Self {
        let mut handler = Self {
            shortcuts: HashMap::new(),
            enabled: true,
        };
        handler.register_defaults();
        handler
    }

    /// Register default keyboard shortcuts.
    fn register_defaults(&mut self) {
        let actions = [
            KeyAction::FocusNext,
            KeyAction::FocusPrevious,
            KeyAction::Activate,
            KeyAction::Cancel,
            KeyAction::ToggleTvMode,
            KeyAction::ToggleFlowMode,
            KeyAction::CycleFlowMetric,
            KeyAction::StartRide,
            KeyAction::PauseRide,
            KeyAction::EndRide,
            KeyAction::SkipInterval,
            KeyAction::AnnounceMetrics,
            KeyAction::ShowShortcuts,
            KeyAction::OpenSettings,
        ];

        for action in actions {
            self.register(action.default_shortcut(), action);
        }
    }

    /// Register a shortcut for an action.
    pub fn register(&mut self, shortcut: KeyboardShortcut, action: KeyAction) {
        self.shortcuts.insert(shortcut, action);
    }

    /// Unregister a shortcut.
    pub fn unregister(&mut self, shortcut: &KeyboardShortcut) {
        self.shortcuts.remove(shortcut);
    }

    /// Get the action for a key press.
    pub fn get_action(&self, key: Key, modifiers: Modifiers) -> Option<KeyAction> {
        if !self.enabled {
            return None;
        }

        let shortcut = KeyboardShortcut { key, modifiers };
        self.shortcuts.get(&shortcut).copied()
    }

    /// Enable or disable keyboard handling.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if keyboard handling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get all registered shortcuts.
    pub fn all_shortcuts(&self) -> impl Iterator<Item = (&KeyboardShortcut, &KeyAction)> {
        self.shortcuts.iter()
    }

    /// Get the shortcut for an action.
    pub fn shortcut_for(&self, action: KeyAction) -> Option<&KeyboardShortcut> {
        self.shortcuts
            .iter()
            .find(|(_, &a)| a == action)
            .map(|(s, _)| s)
    }
}

/// Get a display name for a key.
fn key_name(key: Key) -> &'static str {
    match key {
        Key::Tab => "Tab",
        Key::Enter => "Enter",
        Key::Escape => "Esc",
        Key::Space => "Space",
        Key::ArrowUp => "↑",
        Key::ArrowDown => "↓",
        Key::ArrowLeft => "←",
        Key::ArrowRight => "→",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PgUp",
        Key::PageDown => "PgDn",
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
        Key::Comma => ",",
        _ => "?",
    }
}
