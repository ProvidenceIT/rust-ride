//! Input handling module for keyboard, touch, and gesture support.
//!
//! Provides unified input handling including keyboard shortcuts,
//! touch gestures, and gesture recognition.

pub mod gestures;
pub mod keyboard;
pub mod touch;

// Re-export types
pub use gestures::{GestureHandler, GestureType, SwipeDirection};
pub use keyboard::{KeyAction, KeyboardHandler, KeyboardShortcut};
pub use touch::{TouchEvent, TouchHandler, TouchState};
