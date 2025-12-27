//! UI module for egui-based user interface.

pub mod display_modes;
pub mod layout;
pub mod screens;
pub mod theme;
pub mod widgets;

pub use display_modes::{DisplayMode, DisplayModeManager};
pub use layout::{LayoutProfile, LayoutProfileManager};
pub use theme::Theme;
