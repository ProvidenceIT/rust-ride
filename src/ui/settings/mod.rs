//! Settings UI components for the application.
//!
//! This module contains reusable settings panels that can be embedded
//! in the main settings screen or used independently.

pub mod audio_settings;

pub use audio_settings::{
    AudioSettingsAction, AudioSettingsPanel, AudioSettingsPanelConfig, AudioSettingsResponse,
    AudioTestType,
};
