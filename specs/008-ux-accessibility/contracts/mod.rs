//! UX & Accessibility Contracts
//!
//! This module contains the public API contracts for all UX and accessibility features.
//! These contracts define the interfaces that implementations must satisfy.

pub mod accessibility;
pub mod audio;
pub mod display_modes;
pub mod i18n;
pub mod layout;
pub mod onboarding;

// Re-export primary traits for convenience
pub use accessibility::{
    ColorPaletteProvider, FocusManager, HighContrastTheme, KeyboardShortcuts,
    ScreenReaderSupport, VoiceControl,
};
pub use audio::{AudioCueSystem, ToneGenerator, ZoneChangeDetector};
pub use display_modes::{DisplayModeManager, FlowModeRenderer, TvModeRenderer};
pub use i18n::{LocaleDetector, TranslationLoader, TranslationService};
pub use layout::{LayoutEditor, LayoutProfileManager, LayoutRenderer};
pub use onboarding::{Glossary, OnboardingWizard};
