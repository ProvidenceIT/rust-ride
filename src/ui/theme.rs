//! UI theme definitions.
//!
//! T052: Implement dark theme colors
//! T061: Integrate dark-light crate for system theme detection
//! T062: Implement ThemePreference enum handling
//! T063: Add system theme polling
//! T064: Implement smooth theme transition

use egui::{Color32, Visuals};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// T061: Detect the current system theme using dark-light crate.
pub fn detect_system_theme() -> Theme {
    match dark_light::detect() {
        dark_light::Mode::Dark => Theme::Dark,
        dark_light::Mode::Light => Theme::Light,
        dark_light::Mode::Default => Theme::Dark, // Default to dark if unknown
    }
}

/// T062: Resolve the actual theme from a ThemePreference.
pub fn resolve_theme(preference: crate::storage::config::ThemePreference) -> Theme {
    use crate::storage::config::ThemePreference;

    match preference {
        ThemePreference::FollowSystem => detect_system_theme(),
        ThemePreference::Light => Theme::Light,
        ThemePreference::Dark => Theme::Dark,
    }
}

/// T063: System theme monitor for polling and change detection.
pub struct ThemeMonitor {
    /// Current detected system theme
    current_system_theme: Theme,
    /// Last time we checked the system theme
    last_check: Instant,
    /// Polling interval (default: 5 seconds as per spec)
    poll_interval: Duration,
    /// Whether a theme change is in progress (for animation)
    transitioning: Arc<AtomicBool>,
    /// Transition progress (0.0 to 1.0)
    transition_progress: f32,
    /// Theme we're transitioning from
    transition_from: Option<Theme>,
    /// Theme we're transitioning to
    transition_to: Option<Theme>,
}

impl Default for ThemeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeMonitor {
    /// Create a new theme monitor with default settings.
    pub fn new() -> Self {
        Self {
            current_system_theme: detect_system_theme(),
            last_check: Instant::now(),
            poll_interval: Duration::from_secs(5),
            transitioning: Arc::new(AtomicBool::new(false)),
            transition_progress: 1.0,
            transition_from: None,
            transition_to: None,
        }
    }

    /// Create a theme monitor with a custom polling interval.
    pub fn with_poll_interval(interval: Duration) -> Self {
        Self {
            poll_interval: interval,
            ..Self::new()
        }
    }

    /// Poll for system theme changes.
    ///
    /// Returns Some(Theme) if the theme changed, None otherwise.
    pub fn poll(&mut self) -> Option<Theme> {
        let now = Instant::now();
        if now.duration_since(self.last_check) >= self.poll_interval {
            self.last_check = now;

            let new_theme = detect_system_theme();
            if new_theme != self.current_system_theme {
                let old_theme = self.current_system_theme;
                self.current_system_theme = new_theme;

                // Start transition animation
                self.start_transition(old_theme, new_theme);

                return Some(new_theme);
            }
        }
        None
    }

    /// Get the current system theme.
    pub fn current_theme(&self) -> Theme {
        self.current_system_theme
    }

    /// T064: Start a smooth theme transition.
    fn start_transition(&mut self, from: Theme, to: Theme) {
        self.transitioning.store(true, Ordering::SeqCst);
        self.transition_from = Some(from);
        self.transition_to = Some(to);
        self.transition_progress = 0.0;
    }

    /// T064: Update the transition animation.
    ///
    /// Call this each frame during a transition.
    /// Returns the current effective theme (may be transitioning).
    pub fn update_transition(&mut self, delta_seconds: f32) -> Theme {
        if !self.transitioning.load(Ordering::SeqCst) {
            return self.current_system_theme;
        }

        // Transition speed: complete in ~0.3 seconds
        let speed = 3.0;
        self.transition_progress += delta_seconds * speed;

        if self.transition_progress >= 1.0 {
            self.transition_progress = 1.0;
            self.transitioning.store(false, Ordering::SeqCst);
            self.transition_from = None;

            if let Some(to) = self.transition_to.take() {
                return to;
            }
        }

        // During transition, return target theme (egui handles blending)
        self.transition_to.unwrap_or(self.current_system_theme)
    }

    /// Check if a theme transition is in progress.
    pub fn is_transitioning(&self) -> bool {
        self.transitioning.load(Ordering::SeqCst)
    }

    /// Get the transition progress (0.0 to 1.0).
    pub fn transition_progress(&self) -> f32 {
        self.transition_progress
    }

    /// Force a theme check immediately.
    pub fn check_now(&mut self) -> Theme {
        self.last_check = Instant::now();
        self.current_system_theme = detect_system_theme();
        self.current_system_theme
    }
}

/// T064: Blend two visuals for smooth theme transition.
///
/// Note: This is a simplified implementation that doesn't interpolate
/// all visual properties. For a fully smooth transition, consider
/// using egui's built-in animation capabilities.
pub fn blend_visuals(from: &Visuals, to: &Visuals, t: f32) -> Visuals {
    if t <= 0.0 {
        return from.clone();
    }
    if t >= 1.0 {
        return to.clone();
    }

    // For now, snap to target after halfway point
    // A more sophisticated implementation would interpolate colors
    if t >= 0.5 {
        to.clone()
    } else {
        from.clone()
    }
}

/// T064: Interpolate between two colors.
#[allow(dead_code)]
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;

    Color32::from_rgba_unmultiplied(
        (a.r() as f32 * inv_t + b.r() as f32 * t) as u8,
        (a.g() as f32 * inv_t + b.g() as f32 * t) as u8,
        (a.b() as f32 * inv_t + b.b() as f32 * t) as u8,
        (a.a() as f32 * inv_t + b.a() as f32 * t) as u8,
    )
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
    use crate::accessibility::{ColorMode, ColorPalette};
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
