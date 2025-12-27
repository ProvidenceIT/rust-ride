//! Colorblind-safe color palettes.
//!
//! Provides color palettes optimized for users with various types of color vision deficiency.
//! Based on Paul Tol's colorblind-safe palette research.

use egui::Color32;

/// Color vision mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Normal color vision
    #[default]
    Normal,
    /// Red-green colorblindness (most common, ~6% of males)
    Protanopia,
    /// Red-green colorblindness (similar to protanopia)
    Deuteranopia,
    /// Blue-yellow colorblindness (rare, ~0.01%)
    Tritanopia,
}

impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorMode::Normal => write!(f, "Normal"),
            ColorMode::Protanopia => write!(f, "Protanopia (Red-Green)"),
            ColorMode::Deuteranopia => write!(f, "Deuteranopia (Red-Green)"),
            ColorMode::Tritanopia => write!(f, "Tritanopia (Blue-Yellow)"),
        }
    }
}

/// A palette of colors for power zones.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Zone 1 - Recovery
    pub zone1: Color32,
    /// Zone 2 - Endurance
    pub zone2: Color32,
    /// Zone 3 - Tempo
    pub zone3: Color32,
    /// Zone 4 - Threshold
    pub zone4: Color32,
    /// Zone 5 - VO2max
    pub zone5: Color32,
    /// Zone 6 - Anaerobic
    pub zone6: Color32,
    /// Zone 7 - Neuromuscular
    pub zone7: Color32,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::normal()
    }
}

impl ColorPalette {
    /// Normal vision color palette (standard colors).
    pub fn normal() -> Self {
        Self {
            zone1: Color32::from_rgb(128, 128, 128), // Gray
            zone2: Color32::from_rgb(0, 128, 255),   // Blue
            zone3: Color32::from_rgb(0, 200, 100),   // Green
            zone4: Color32::from_rgb(255, 200, 0),   // Yellow
            zone5: Color32::from_rgb(255, 128, 0),   // Orange
            zone6: Color32::from_rgb(255, 50, 50),   // Red
            zone7: Color32::from_rgb(180, 0, 180),   // Purple
        }
    }

    /// Protanopia-safe palette (Paul Tol scheme).
    /// Uses blue-yellow gradient that is distinguishable for red-green colorblindness.
    pub fn protanopia() -> Self {
        Self {
            zone1: Color32::from_rgb(153, 153, 153), // Gray
            zone2: Color32::from_rgb(136, 204, 238), // Light blue
            zone3: Color32::from_rgb(68, 170, 153),  // Teal
            zone4: Color32::from_rgb(221, 204, 119), // Yellow
            zone5: Color32::from_rgb(204, 102, 119), // Rose
            zone6: Color32::from_rgb(170, 68, 102),  // Dark rose
            zone7: Color32::from_rgb(136, 34, 85),   // Dark magenta
        }
    }

    /// Deuteranopia-safe palette.
    /// Similar to protanopia but with slightly different adjustments.
    pub fn deuteranopia() -> Self {
        Self {
            zone1: Color32::from_rgb(153, 153, 153), // Gray
            zone2: Color32::from_rgb(136, 204, 238), // Light blue
            zone3: Color32::from_rgb(68, 170, 153),  // Teal
            zone4: Color32::from_rgb(221, 204, 119), // Yellow
            zone5: Color32::from_rgb(204, 102, 119), // Rose
            zone6: Color32::from_rgb(170, 68, 102),  // Dark rose
            zone7: Color32::from_rgb(136, 34, 85),   // Dark magenta
        }
    }

    /// Tritanopia-safe palette.
    /// Uses red-green gradient that is distinguishable for blue-yellow colorblindness.
    pub fn tritanopia() -> Self {
        Self {
            zone1: Color32::from_rgb(153, 153, 153), // Gray
            zone2: Color32::from_rgb(238, 136, 102), // Salmon
            zone3: Color32::from_rgb(204, 102, 68),  // Coral
            zone4: Color32::from_rgb(170, 68, 102),  // Rose
            zone5: Color32::from_rgb(136, 102, 136), // Lavender
            zone6: Color32::from_rgb(102, 68, 136),  // Purple
            zone7: Color32::from_rgb(68, 34, 102),   // Dark purple
        }
    }

    /// Get palette for the given color mode.
    pub fn for_mode(mode: ColorMode) -> Self {
        match mode {
            ColorMode::Normal => Self::normal(),
            ColorMode::Protanopia => Self::protanopia(),
            ColorMode::Deuteranopia => Self::deuteranopia(),
            ColorMode::Tritanopia => Self::tritanopia(),
        }
    }

    /// Get the color for a power zone (1-7).
    pub fn zone_color(&self, zone: u8) -> Color32 {
        match zone {
            1 => self.zone1,
            2 => self.zone2,
            3 => self.zone3,
            4 => self.zone4,
            5 => self.zone5,
            6 => self.zone6,
            7 => self.zone7,
            _ => Color32::GRAY,
        }
    }

    /// Get the color for a HR zone (1-5).
    /// Maps to power zones with appropriate intensity.
    pub fn hr_zone_color(&self, zone: u8) -> Color32 {
        match zone {
            1 => self.zone1, // Recovery
            2 => self.zone2, // Endurance
            3 => self.zone3, // Tempo
            4 => self.zone4, // Threshold
            5 => self.zone6, // Max effort (maps to anaerobic)
            _ => Color32::GRAY,
        }
    }
}

/// Trait for providing color palettes.
pub trait ColorPaletteProvider {
    /// Get the current color mode.
    fn color_mode(&self) -> ColorMode;

    /// Set the color mode.
    fn set_color_mode(&mut self, mode: ColorMode);

    /// Get the current color palette.
    fn palette(&self) -> ColorPalette {
        ColorPalette::for_mode(self.color_mode())
    }

    /// Get a zone color from the current palette.
    fn zone_color(&self, zone: u8) -> Color32 {
        self.palette().zone_color(zone)
    }

    /// Get an HR zone color from the current palette.
    fn hr_zone_color(&self, zone: u8) -> Color32 {
        self.palette().hr_zone_color(zone)
    }
}
