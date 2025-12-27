//! Accessibility module for UX & Accessibility feature.
//!
//! This module provides accessibility features including:
//! - Focus management for keyboard navigation
//! - Screen reader support via accesskit
//! - Colorblind-safe color palettes
//! - High contrast mode
//! - Voice control (optional)

pub mod colorblind;
pub mod focus;
pub mod high_contrast;
pub mod screen_reader;
#[cfg(feature = "voice-control")]
pub mod voice_control;

// Re-export primary types
pub use colorblind::{ColorMode, ColorPalette, ColorPaletteProvider};
pub use focus::{FocusIndicatorStyle, FocusManager, FocusableWidget};
pub use high_contrast::HighContrastTheme;
pub use screen_reader::ScreenReaderSupport;
#[cfg(feature = "voice-control")]
pub use voice_control::{VoiceCommand, VoiceControl, VoiceControlState};
