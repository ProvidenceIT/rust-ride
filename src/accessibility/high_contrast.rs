//! High contrast theme for WCAG AAA compliance.
//!
//! Provides colors with 7:1 contrast ratio for users who need high contrast.

use egui::{Color32, Visuals};

/// High contrast theme colors meeting WCAG AAA requirements (7:1 ratio).
pub struct HighContrastTheme;

impl HighContrastTheme {
    // Background colors
    pub const BACKGROUND: Color32 = Color32::BLACK;
    pub const PANEL_BG: Color32 = Color32::from_rgb(10, 10, 10);
    pub const CARD_BG: Color32 = Color32::from_rgb(20, 20, 20);

    // Text colors (high contrast against backgrounds)
    pub const TEXT_PRIMARY: Color32 = Color32::WHITE;
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(220, 220, 220);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(180, 180, 180);

    // Accent colors (bright for visibility)
    pub const ACCENT: Color32 = Color32::from_rgb(0, 200, 255); // Cyan
    pub const SUCCESS: Color32 = Color32::from_rgb(0, 255, 100); // Bright green
    pub const WARNING: Color32 = Color32::from_rgb(255, 220, 0); // Bright yellow
    pub const ERROR: Color32 = Color32::from_rgb(255, 80, 80); // Bright red

    // Border colors
    pub const BORDER: Color32 = Color32::WHITE;
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(0, 200, 255);

    /// Create high contrast visuals for egui.
    pub fn visuals() -> Visuals {
        let mut visuals = Visuals::dark();

        visuals.window_fill = Self::PANEL_BG;
        visuals.panel_fill = Self::PANEL_BG;
        visuals.faint_bg_color = Self::CARD_BG;
        visuals.extreme_bg_color = Self::BACKGROUND;

        // Widget backgrounds
        visuals.widgets.noninteractive.bg_fill = Self::CARD_BG;
        visuals.widgets.inactive.bg_fill = Self::CARD_BG;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 40, 40);
        visuals.widgets.active.bg_fill = Self::ACCENT;

        // Selection
        visuals.selection.bg_fill = Self::ACCENT.linear_multiply(0.5);
        visuals.selection.stroke.color = Self::ACCENT;

        // Text colors
        visuals.widgets.noninteractive.fg_stroke.color = Self::TEXT_PRIMARY;
        visuals.widgets.inactive.fg_stroke.color = Self::TEXT_SECONDARY;
        visuals.widgets.hovered.fg_stroke.color = Self::TEXT_PRIMARY;
        visuals.widgets.active.fg_stroke.color = Self::BACKGROUND;

        // Borders - more prominent for high contrast
        visuals.widgets.noninteractive.bg_stroke.color = Self::BORDER;
        visuals.widgets.inactive.bg_stroke.color = Self::BORDER;
        visuals.widgets.noninteractive.bg_stroke.width = 1.5;
        visuals.widgets.inactive.bg_stroke.width = 1.5;

        visuals
    }

    /// Calculate contrast ratio between two colors.
    /// Returns a value between 1 and 21 (21 being black on white).
    pub fn contrast_ratio(fg: Color32, bg: Color32) -> f32 {
        let fg_lum = Self::relative_luminance(fg);
        let bg_lum = Self::relative_luminance(bg);

        let (lighter, darker) = if fg_lum > bg_lum {
            (fg_lum, bg_lum)
        } else {
            (bg_lum, fg_lum)
        };

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Calculate relative luminance of a color.
    /// https://www.w3.org/TR/WCAG21/#dfn-relative-luminance
    fn relative_luminance(color: Color32) -> f32 {
        let r = Self::linearize(color.r() as f32 / 255.0);
        let g = Self::linearize(color.g() as f32 / 255.0);
        let b = Self::linearize(color.b() as f32 / 255.0);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Linearize a color channel value.
    fn linearize(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Check if contrast ratio meets WCAG AA standard (4.5:1 for normal text).
    pub fn meets_aa(fg: Color32, bg: Color32) -> bool {
        Self::contrast_ratio(fg, bg) >= 4.5
    }

    /// Check if contrast ratio meets WCAG AAA standard (7:1 for normal text).
    pub fn meets_aaa(fg: Color32, bg: Color32) -> bool {
        Self::contrast_ratio(fg, bg) >= 7.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contrast_ratio_black_white() {
        let ratio = HighContrastTheme::contrast_ratio(Color32::WHITE, Color32::BLACK);
        assert!(ratio > 20.0, "Black on white should be ~21:1");
    }

    #[test]
    fn test_high_contrast_theme_meets_aaa() {
        // Primary text on background should meet AAA
        assert!(HighContrastTheme::meets_aaa(
            HighContrastTheme::TEXT_PRIMARY,
            HighContrastTheme::BACKGROUND
        ));

        // Primary text on panel should meet AAA
        assert!(HighContrastTheme::meets_aaa(
            HighContrastTheme::TEXT_PRIMARY,
            HighContrastTheme::PANEL_BG
        ));
    }
}
