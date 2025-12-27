//! UI theme definitions.
//!
//! T052: Implement dark theme colors
//! T126: Implement light theme colors (placeholder)

use egui::{Color32, Visuals};

/// Theme configuration for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// Get the egui Visuals for this theme.
    pub fn visuals(&self) -> Visuals {
        match self {
            Theme::Dark => dark_visuals(),
            Theme::Light => light_visuals(),
        }
    }
}

/// Dark theme colors.
pub struct DarkTheme;

impl DarkTheme {
    /// Background color
    pub const BACKGROUND: Color32 = Color32::from_rgb(18, 18, 24);
    /// Panel background
    pub const PANEL_BG: Color32 = Color32::from_rgb(28, 28, 36);
    /// Card background
    pub const CARD_BG: Color32 = Color32::from_rgb(38, 38, 48);
    /// Primary text
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);
    /// Secondary text
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 170);
    /// Muted text
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 110);
    /// Accent color (blue)
    pub const ACCENT: Color32 = Color32::from_rgb(66, 133, 244);
    /// Success color (green)
    pub const SUCCESS: Color32 = Color32::from_rgb(52, 168, 83);
    /// Warning color (yellow/orange)
    pub const WARNING: Color32 = Color32::from_rgb(251, 188, 4);
    /// Error color (red)
    pub const ERROR: Color32 = Color32::from_rgb(234, 67, 53);
    /// Border color
    pub const BORDER: Color32 = Color32::from_rgb(60, 60, 70);
}

/// Light theme colors.
pub struct LightTheme;

impl LightTheme {
    /// Background color
    pub const BACKGROUND: Color32 = Color32::from_rgb(250, 250, 252);
    /// Panel background
    pub const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);
    /// Card background
    pub const CARD_BG: Color32 = Color32::from_rgb(245, 245, 248);
    /// Primary text
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(32, 32, 40);
    /// Secondary text
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(96, 96, 104);
    /// Muted text
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(144, 144, 152);
    /// Accent color (blue)
    pub const ACCENT: Color32 = Color32::from_rgb(26, 115, 232);
    /// Success color (green)
    pub const SUCCESS: Color32 = Color32::from_rgb(24, 128, 56);
    /// Warning color (yellow/orange)
    pub const WARNING: Color32 = Color32::from_rgb(234, 160, 0);
    /// Error color (red)
    pub const ERROR: Color32 = Color32::from_rgb(200, 50, 40);
    /// Border color
    pub const BORDER: Color32 = Color32::from_rgb(218, 218, 224);
}

/// Create dark theme visuals.
fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();

    visuals.window_fill = DarkTheme::PANEL_BG;
    visuals.panel_fill = DarkTheme::PANEL_BG;
    visuals.faint_bg_color = DarkTheme::CARD_BG;
    visuals.extreme_bg_color = DarkTheme::BACKGROUND;

    visuals.widgets.noninteractive.bg_fill = DarkTheme::CARD_BG;
    visuals.widgets.inactive.bg_fill = DarkTheme::CARD_BG;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 50, 62);
    visuals.widgets.active.bg_fill = DarkTheme::ACCENT;

    visuals.selection.bg_fill = DarkTheme::ACCENT.linear_multiply(0.4);
    visuals.selection.stroke.color = DarkTheme::ACCENT;

    visuals.widgets.noninteractive.fg_stroke.color = DarkTheme::TEXT_PRIMARY;
    visuals.widgets.inactive.fg_stroke.color = DarkTheme::TEXT_SECONDARY;
    visuals.widgets.hovered.fg_stroke.color = DarkTheme::TEXT_PRIMARY;
    visuals.widgets.active.fg_stroke.color = DarkTheme::TEXT_PRIMARY;

    visuals.widgets.noninteractive.bg_stroke.color = DarkTheme::BORDER;
    visuals.widgets.inactive.bg_stroke.color = DarkTheme::BORDER;

    visuals
}

/// Create light theme visuals.
fn light_visuals() -> Visuals {
    let mut visuals = Visuals::light();

    visuals.window_fill = LightTheme::PANEL_BG;
    visuals.panel_fill = LightTheme::PANEL_BG;
    visuals.faint_bg_color = LightTheme::CARD_BG;
    visuals.extreme_bg_color = LightTheme::BACKGROUND;

    visuals.widgets.noninteractive.bg_fill = LightTheme::CARD_BG;
    visuals.widgets.inactive.bg_fill = LightTheme::CARD_BG;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(230, 230, 235);
    visuals.widgets.active.bg_fill = LightTheme::ACCENT;

    visuals.selection.bg_fill = LightTheme::ACCENT.linear_multiply(0.2);
    visuals.selection.stroke.color = LightTheme::ACCENT;

    visuals.widgets.noninteractive.fg_stroke.color = LightTheme::TEXT_PRIMARY;
    visuals.widgets.inactive.fg_stroke.color = LightTheme::TEXT_SECONDARY;
    visuals.widgets.hovered.fg_stroke.color = LightTheme::TEXT_PRIMARY;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;

    visuals.widgets.noninteractive.bg_stroke.color = LightTheme::BORDER;
    visuals.widgets.inactive.bg_stroke.color = LightTheme::BORDER;

    visuals
}

/// Power zone colors for display.
/// T045: Updated to use ColorPaletteProvider for accessibility.
pub mod zone_colors {
    use crate::accessibility::{ColorMode, ColorPalette, ColorPaletteProvider};
    use egui::Color32;

    /// Default zone colors (normal color vision).
    pub const Z1_RECOVERY: Color32 = Color32::from_rgb(128, 128, 128);
    pub const Z2_ENDURANCE: Color32 = Color32::from_rgb(0, 128, 255);
    pub const Z3_TEMPO: Color32 = Color32::from_rgb(0, 200, 100);
    pub const Z4_THRESHOLD: Color32 = Color32::from_rgb(255, 200, 0);
    pub const Z5_VO2MAX: Color32 = Color32::from_rgb(255, 128, 0);
    pub const Z6_ANAEROBIC: Color32 = Color32::from_rgb(255, 50, 50);
    pub const Z7_NEUROMUSCULAR: Color32 = Color32::from_rgb(180, 0, 180);

    /// Get the color for a power zone (1-7) using the default palette.
    pub fn power_zone_color(zone: u8) -> Color32 {
        match zone {
            1 => Z1_RECOVERY,
            2 => Z2_ENDURANCE,
            3 => Z3_TEMPO,
            4 => Z4_THRESHOLD,
            5 => Z5_VO2MAX,
            6 => Z6_ANAEROBIC,
            7 => Z7_NEUROMUSCULAR,
            _ => Color32::GRAY,
        }
    }

    /// Get the color for a power zone (1-7) using the specified color mode.
    /// T045: Colorblind-safe zone colors.
    pub fn power_zone_color_accessible(zone: u8, mode: ColorMode) -> Color32 {
        let palette = ColorPalette::for_mode(mode);
        palette.zone_color(zone)
    }

    /// Get the color for a HR zone (1-5) using the default palette.
    pub fn hr_zone_color(zone: u8) -> Color32 {
        match zone {
            1 => Z1_RECOVERY,
            2 => Z2_ENDURANCE,
            3 => Z3_TEMPO,
            4 => Z4_THRESHOLD,
            5 => Z6_ANAEROBIC, // Red for max effort
            _ => Color32::GRAY,
        }
    }

    /// Get the color for a HR zone (1-5) using the specified color mode.
    /// T045: Colorblind-safe zone colors.
    pub fn hr_zone_color_accessible(zone: u8, mode: ColorMode) -> Color32 {
        let palette = ColorPalette::for_mode(mode);
        palette.hr_zone_color(zone)
    }

    /// Zone color provider that caches the current palette.
    /// T045: Efficient zone color lookups with active palette.
    pub struct ZoneColorProvider {
        palette: ColorPalette,
        mode: ColorMode,
    }

    impl ZoneColorProvider {
        /// Create a new zone color provider with the specified color mode.
        pub fn new(mode: ColorMode) -> Self {
            Self {
                palette: ColorPalette::for_mode(mode),
                mode,
            }
        }

        /// Update the color mode.
        pub fn set_mode(&mut self, mode: ColorMode) {
            self.mode = mode;
            self.palette = ColorPalette::for_mode(mode);
        }

        /// Get the current color mode.
        pub fn mode(&self) -> ColorMode {
            self.mode
        }

        /// Get the power zone color (1-7).
        pub fn power_zone(&self, zone: u8) -> Color32 {
            self.palette.zone_color(zone)
        }

        /// Get the HR zone color (1-5).
        pub fn hr_zone(&self, zone: u8) -> Color32 {
            self.palette.hr_zone_color(zone)
        }

        /// Get zone 4 (threshold) color for primary accent.
        pub fn primary(&self) -> Color32 {
            self.palette.zone_color(4)
        }

        /// Get zone 3 (tempo) color for success.
        pub fn success(&self) -> Color32 {
            self.palette.zone_color(3)
        }

        /// Get zone 5 (VO2max) color for warning.
        pub fn warning(&self) -> Color32 {
            self.palette.zone_color(5)
        }

        /// Get zone 6 (anaerobic) color for error.
        pub fn error(&self) -> Color32 {
            self.palette.zone_color(6)
        }
    }

    impl Default for ZoneColorProvider {
        fn default() -> Self {
            Self::new(ColorMode::Normal)
        }
    }
}
