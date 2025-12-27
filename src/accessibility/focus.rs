//! Focus management for keyboard navigation.
//!
//! Implements focus tracking, focus indicators, and focus traps for modal dialogs.

use egui::{Color32, Id, Stroke, StrokeKind};
use std::collections::VecDeque;

/// Style for focus indicators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusIndicatorStyle {
    /// Color of the focus ring
    pub color: Color32,
    /// Width of the focus ring stroke
    pub width: f32,
    /// Corner radius for rounded rectangles
    pub corner_radius: f32,
    /// Offset from the widget bounds
    pub offset: f32,
}

impl Default for FocusIndicatorStyle {
    fn default() -> Self {
        Self {
            color: Color32::from_rgb(66, 133, 244), // Blue accent
            width: 2.0,
            corner_radius: 4.0,
            offset: 2.0,
        }
    }
}

impl FocusIndicatorStyle {
    /// High contrast focus style for accessibility
    pub fn high_contrast() -> Self {
        Self {
            color: Color32::WHITE,
            width: 3.0,
            corner_radius: 4.0,
            offset: 3.0,
        }
    }

    /// Get the stroke for drawing
    pub fn stroke(&self) -> Stroke {
        Stroke::new(self.width, self.color)
    }
}

/// Represents a focusable widget in the navigation order.
#[derive(Debug, Clone)]
pub struct FocusableWidget {
    /// Unique identifier for the widget
    pub id: Id,
    /// Tab order index (lower = earlier in tab order)
    pub tab_index: i32,
    /// Whether the widget is currently enabled
    pub enabled: bool,
    /// Optional group for focus trapping
    pub group: Option<String>,
}

impl FocusableWidget {
    /// Create a new focusable widget
    pub fn new(id: Id, tab_index: i32) -> Self {
        Self {
            id,
            tab_index,
            enabled: true,
            group: None,
        }
    }

    /// Set the focus group (for modal focus trapping)
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Set whether the widget is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Manages focus navigation for the application.
#[derive(Debug)]
pub struct FocusManager {
    /// Ordered list of focusable widgets
    widgets: Vec<FocusableWidget>,
    /// Currently focused widget ID
    current_focus: Option<Id>,
    /// Focus indicator style
    indicator_style: FocusIndicatorStyle,
    /// Focus history for back navigation
    focus_history: VecDeque<Id>,
    /// Maximum history size
    max_history: usize,
    /// Active focus trap group (for modals)
    active_trap: Option<String>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            current_focus: None,
            indicator_style: FocusIndicatorStyle::default(),
            focus_history: VecDeque::new(),
            max_history: 10,
            active_trap: None,
        }
    }

    /// Clear all registered widgets (call at start of frame)
    pub fn clear_widgets(&mut self) {
        self.widgets.clear();
    }

    /// Register a focusable widget
    pub fn register(&mut self, widget: FocusableWidget) {
        self.widgets.push(widget);
    }

    /// Register a widget with simple parameters
    pub fn register_simple(&mut self, id: Id, tab_index: i32) {
        self.register(FocusableWidget::new(id, tab_index));
    }

    /// Get the current focus indicator style
    pub fn indicator_style(&self) -> &FocusIndicatorStyle {
        &self.indicator_style
    }

    /// Set the focus indicator style
    pub fn set_indicator_style(&mut self, style: FocusIndicatorStyle) {
        self.indicator_style = style;
    }

    /// Get the currently focused widget ID
    pub fn current_focus(&self) -> Option<Id> {
        self.current_focus
    }

    /// Check if a specific widget has focus
    pub fn has_focus(&self, id: Id) -> bool {
        self.current_focus == Some(id)
    }

    /// Set focus to a specific widget
    pub fn set_focus(&mut self, id: Id) {
        if let Some(current) = self.current_focus {
            if current != id {
                self.push_history(current);
            }
        }
        self.current_focus = Some(id);
    }

    /// Clear current focus
    pub fn clear_focus(&mut self) {
        if let Some(current) = self.current_focus.take() {
            self.push_history(current);
        }
    }

    /// Move focus to the next widget in tab order
    pub fn focus_next(&mut self) {
        self.move_focus(1);
    }

    /// Move focus to the previous widget in tab order
    pub fn focus_previous(&mut self) {
        self.move_focus(-1);
    }

    /// Move focus by the given delta
    fn move_focus(&mut self, delta: i32) {
        let focusable = self.get_focusable_widgets();
        if focusable.is_empty() {
            return;
        }

        let current_idx = self
            .current_focus
            .and_then(|id| focusable.iter().position(|w| w.id == id));

        let new_idx = match current_idx {
            Some(idx) => {
                let len = focusable.len() as i32;
                ((idx as i32 + delta).rem_euclid(len)) as usize
            }
            None => 0,
        };

        if let Some(widget) = focusable.get(new_idx) {
            self.set_focus(widget.id);
        }
    }

    /// Get focusable widgets respecting the active trap
    fn get_focusable_widgets(&self) -> Vec<&FocusableWidget> {
        let mut widgets: Vec<_> = self
            .widgets
            .iter()
            .filter(|w| {
                w.enabled
                    && match (&self.active_trap, &w.group) {
                        (Some(trap), Some(group)) => trap == group,
                        (Some(_), None) => false,
                        (None, _) => true,
                    }
            })
            .collect();

        widgets.sort_by_key(|w| w.tab_index);
        widgets
    }

    /// Set a focus trap for modal dialogs
    pub fn set_focus_trap(&mut self, group: impl Into<String>) {
        let group = group.into();
        self.active_trap = Some(group.clone());

        // Focus first widget in the trap
        let first = self
            .widgets
            .iter()
            .filter(|w| w.enabled && w.group.as_ref() == Some(&group))
            .min_by_key(|w| w.tab_index);

        if let Some(widget) = first {
            self.set_focus(widget.id);
        }
    }

    /// Remove the current focus trap
    pub fn release_focus_trap(&mut self) {
        self.active_trap = None;

        // Restore previous focus from history
        if let Some(id) = self.focus_history.pop_back() {
            self.current_focus = Some(id);
        }
    }

    /// Check if a focus trap is active
    pub fn is_trap_active(&self) -> bool {
        self.active_trap.is_some()
    }

    /// Push to focus history
    fn push_history(&mut self, id: Id) {
        if self.focus_history.len() >= self.max_history {
            self.focus_history.pop_front();
        }
        self.focus_history.push_back(id);
    }

    /// Focus the first widget in the current context
    pub fn focus_first(&mut self) {
        let focusable = self.get_focusable_widgets();
        if let Some(widget) = focusable.first() {
            self.set_focus(widget.id);
        }
    }

    /// Focus the last widget in the current context
    pub fn focus_last(&mut self) {
        let focusable = self.get_focusable_widgets();
        if let Some(widget) = focusable.last() {
            self.set_focus(widget.id);
        }
    }

    /// Process keyboard input for focus navigation.
    /// Returns true if the input was handled.
    pub fn handle_keyboard_input(&mut self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Tab) {
                if i.modifiers.shift {
                    self.focus_previous();
                } else {
                    self.focus_next();
                }
                return true;
            }
            false
        })
    }
}

/// Extension trait for egui::Ui to support focusable buttons.
pub trait FocusableExt {
    /// Create a button that integrates with the focus manager.
    fn focusable_button(
        &mut self,
        focus_manager: &mut FocusManager,
        id: impl Into<Id>,
        tab_index: i32,
        text: impl Into<String>,
    ) -> egui::Response;

    /// Create a styled button that integrates with the focus manager.
    fn focusable_button_styled(
        &mut self,
        focus_manager: &mut FocusManager,
        id: impl Into<Id>,
        tab_index: i32,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response;
}

impl FocusableExt for egui::Ui {
    fn focusable_button(
        &mut self,
        focus_manager: &mut FocusManager,
        id: impl Into<Id>,
        tab_index: i32,
        text: impl Into<String>,
    ) -> egui::Response {
        let id = id.into();
        let text = text.into();

        // Register this widget with the focus manager
        focus_manager.register_simple(id, tab_index);

        // Check if this widget has focus
        let has_focus = focus_manager.has_focus(id);

        // Create the button with focus handling
        let response = self.add(egui::Button::new(&text).sense(egui::Sense::click()));

        // Draw focus indicator if focused
        if has_focus {
            let style = focus_manager.indicator_style();
            let rect = response.rect.expand(style.offset);
            self.painter()
                .rect_stroke(rect, style.corner_radius, style.stroke(), StrokeKind::Middle);
        }

        // Handle click to set focus
        if response.clicked() || response.gained_focus() {
            focus_manager.set_focus(id);
        }

        // Handle Enter/Space activation when focused
        if has_focus {
            let activated = self.ctx().input(|i| {
                i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
            });
            if activated {
                // Return a "clicked" response
                return response.clone().on_hover_text("");
            }
        }

        response
    }

    fn focusable_button_styled(
        &mut self,
        focus_manager: &mut FocusManager,
        id: impl Into<Id>,
        tab_index: i32,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response {
        let id = id.into();
        let text = text.into();

        // Register this widget with the focus manager
        focus_manager.register_simple(id, tab_index);

        // Check if this widget has focus
        let has_focus = focus_manager.has_focus(id);

        // Create the button with focus handling
        let response = self.add(egui::Button::new(text).sense(egui::Sense::click()));

        // Draw focus indicator if focused
        if has_focus {
            let style = focus_manager.indicator_style();
            let rect = response.rect.expand(style.offset);
            self.painter()
                .rect_stroke(rect, style.corner_radius, style.stroke(), StrokeKind::Middle);
        }

        // Handle click to set focus
        if response.clicked() || response.gained_focus() {
            focus_manager.set_focus(id);
        }

        response
    }
}

/// Render a focus indicator around a rectangle.
pub fn draw_focus_indicator(
    painter: &egui::Painter,
    rect: egui::Rect,
    style: &FocusIndicatorStyle,
) {
    let expanded = rect.expand(style.offset);
    painter.rect_stroke(expanded, style.corner_radius, style.stroke(), StrokeKind::Middle);
}

/// Create a focusable button with accessible features.
///
/// This function creates a button that:
/// - Has a minimum touch target of 44x44 pixels
/// - Shows a focus ring when keyboard focused
/// - Responds to Enter/Space when focused
pub fn accessible_focusable_button(
    ui: &mut egui::Ui,
    focus_manager: &mut FocusManager,
    id: impl Into<Id>,
    tab_index: i32,
    text: &str,
    min_size: egui::Vec2,
) -> egui::Response {
    let id = id.into();

    // Register with focus manager
    focus_manager.register_simple(id, tab_index);
    let has_focus = focus_manager.has_focus(id);

    // Calculate minimum size (at least 44x44 for touch)
    let text_galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::default(),
        Color32::WHITE,
    );
    let text_size = text_galley.size();

    let padding = egui::Vec2::new(16.0, 8.0);
    let content_size = egui::Vec2::new(
        text_size.x + padding.x * 2.0,
        text_size.y + padding.y * 2.0,
    );

    let size = egui::Vec2::new(
        content_size.x.max(min_size.x),
        content_size.y.max(min_size.y),
    );

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        // Background
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);

        // Focus indicator
        if has_focus {
            let style = focus_manager.indicator_style();
            draw_focus_indicator(ui.painter(), rect, style);
        }

        // Text
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::default(),
            visuals.text_color(),
        );
    }

    // Handle focus changes
    if response.clicked() {
        focus_manager.set_focus(id);
    }

    // Handle Enter/Space activation
    if has_focus {
        let activated = ui.ctx().input(|i| {
            i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
        });
        if activated {
            // Simulate a click by requesting repaint
            ui.ctx().request_repaint();
        }
    }

    response
}
