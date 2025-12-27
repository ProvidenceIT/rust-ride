//! Layout editor for drag-and-drop widget arrangement.

use super::{LayoutProfile, WidgetPlacement};
use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// State of the layout editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorState {
    /// Normal viewing mode
    #[default]
    Viewing,
    /// Edit mode (can drag widgets)
    Editing,
    /// Currently dragging a widget
    Dragging,
    /// Currently resizing a widget
    Resizing,
}

/// The layout editor for arranging widgets.
pub struct LayoutEditor {
    /// Current editor state
    state: EditorState,
    /// Index of the widget being manipulated
    selected_widget: Option<usize>,
    /// Drag start position
    drag_start: Option<Pos2>,
    /// Widget position at drag start
    widget_start: Option<(f32, f32)>,
    /// Widget size at resize start
    widget_size_start: Option<(f32, f32)>,
    /// Resize handle being dragged
    resize_handle: Option<ResizeHandle>,
    /// Grid size for snapping (0 = no snapping)
    grid_size: f32,
    /// Whether to show grid
    show_grid: bool,
}

/// Resize handle positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Default for LayoutEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEditor {
    /// Create a new layout editor.
    pub fn new() -> Self {
        Self {
            state: EditorState::Viewing,
            selected_widget: None,
            drag_start: None,
            widget_start: None,
            widget_size_start: None,
            resize_handle: None,
            grid_size: 0.05, // 5% grid
            show_grid: true,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> EditorState {
        self.state
    }

    /// Enter edit mode.
    pub fn enter_edit_mode(&mut self) {
        self.state = EditorState::Editing;
        self.selected_widget = None;
    }

    /// Exit edit mode.
    pub fn exit_edit_mode(&mut self) {
        self.state = EditorState::Viewing;
        self.selected_widget = None;
    }

    /// Toggle edit mode.
    pub fn toggle_edit_mode(&mut self) {
        if self.state == EditorState::Viewing {
            self.enter_edit_mode();
        } else {
            self.exit_edit_mode();
        }
    }

    /// Check if in edit mode.
    pub fn is_editing(&self) -> bool {
        self.state != EditorState::Viewing
    }

    /// Get the selected widget index.
    pub fn selected_widget(&self) -> Option<usize> {
        self.selected_widget
    }

    /// Select a widget by index.
    pub fn select_widget(&mut self, index: usize) {
        self.selected_widget = Some(index);
    }

    /// Deselect the current widget.
    pub fn deselect(&mut self) {
        self.selected_widget = None;
    }

    /// Start dragging a widget.
    pub fn start_drag(&mut self, widget_index: usize, pos: Pos2, widget: &WidgetPlacement) {
        self.state = EditorState::Dragging;
        self.selected_widget = Some(widget_index);
        self.drag_start = Some(pos);
        self.widget_start = Some((widget.x, widget.y));
    }

    /// Update drag position.
    pub fn update_drag(&mut self, current_pos: Pos2, widget: &mut WidgetPlacement) {
        if let (Some(start), Some((wx, wy))) = (self.drag_start, self.widget_start) {
            let delta = current_pos - start;

            // Calculate new position (normalized)
            let new_x = (wx + delta.x).clamp(0.0, 1.0 - widget.width);
            let new_y = (wy + delta.y).clamp(0.0, 1.0 - widget.height);

            // Snap to grid if enabled
            widget.x = self.snap_to_grid(new_x);
            widget.y = self.snap_to_grid(new_y);
        }
    }

    /// End drag operation.
    pub fn end_drag(&mut self) {
        if self.state == EditorState::Dragging {
            self.state = EditorState::Editing;
        }
        self.drag_start = None;
        self.widget_start = None;
    }

    /// Start resizing a widget.
    pub fn start_resize(
        &mut self,
        widget_index: usize,
        handle: ResizeHandle,
        pos: Pos2,
        widget: &WidgetPlacement,
    ) {
        self.state = EditorState::Resizing;
        self.selected_widget = Some(widget_index);
        self.resize_handle = Some(handle);
        self.drag_start = Some(pos);
        self.widget_start = Some((widget.x, widget.y));
        self.widget_size_start = Some((widget.width, widget.height));
    }

    /// Update resize.
    pub fn update_resize(&mut self, current_pos: Pos2, widget: &mut WidgetPlacement) {
        if let (Some(start), Some(handle), Some((sw, sh)), Some((sx, sy))) = (
            self.drag_start,
            self.resize_handle,
            self.widget_size_start,
            self.widget_start,
        ) {
            let delta = current_pos - start;
            let min_size = widget.widget_type.min_size();
            let min_w = min_size.0 / 1000.0; // Approximate normalization
            let min_h = min_size.1 / 1000.0;

            match handle {
                ResizeHandle::Right => {
                    widget.width = self.snap_to_grid((sw + delta.x).max(min_w).min(1.0 - widget.x));
                }
                ResizeHandle::Bottom => {
                    widget.height = self.snap_to_grid((sh + delta.y).max(min_h).min(1.0 - widget.y));
                }
                ResizeHandle::BottomRight => {
                    widget.width = self.snap_to_grid((sw + delta.x).max(min_w).min(1.0 - widget.x));
                    widget.height = self.snap_to_grid((sh + delta.y).max(min_h).min(1.0 - widget.y));
                }
                ResizeHandle::Left => {
                    let new_x = self.snap_to_grid((sx + delta.x).max(0.0));
                    let new_w = sw - (new_x - sx);
                    if new_w >= min_w {
                        widget.x = new_x;
                        widget.width = new_w;
                    }
                }
                ResizeHandle::Top => {
                    let new_y = self.snap_to_grid((sy + delta.y).max(0.0));
                    let new_h = sh - (new_y - sy);
                    if new_h >= min_h {
                        widget.y = new_y;
                        widget.height = new_h;
                    }
                }
                _ => {
                    // Handle other corners similarly
                }
            }
        }
    }

    /// End resize operation.
    pub fn end_resize(&mut self) {
        if self.state == EditorState::Resizing {
            self.state = EditorState::Editing;
        }
        self.resize_handle = None;
        self.drag_start = None;
        self.widget_start = None;
        self.widget_size_start = None;
    }

    /// Snap a value to the grid.
    fn snap_to_grid(&self, value: f32) -> f32 {
        if self.grid_size > 0.0 {
            (value / self.grid_size).round() * self.grid_size
        } else {
            value
        }
    }

    /// Set the grid size.
    pub fn set_grid_size(&mut self, size: f32) {
        self.grid_size = size;
    }

    /// Toggle grid visibility.
    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    /// Check if grid should be shown.
    pub fn should_show_grid(&self) -> bool {
        self.show_grid && self.is_editing()
    }

    /// Check for collision with other widgets.
    pub fn check_collision(
        &self,
        widget_index: usize,
        placement: &WidgetPlacement,
        profile: &LayoutProfile,
    ) -> bool {
        profile
            .widgets
            .iter()
            .enumerate()
            .any(|(i, w)| i != widget_index && w.visible && placement.intersects(w))
    }

    /// Get the resize handle at a position.
    pub fn get_resize_handle_at(
        &self,
        pos: Pos2,
        widget_rect: Rect,
        handle_size: f32,
    ) -> Option<ResizeHandle> {
        let handles = [
            (ResizeHandle::TopLeft, widget_rect.left_top()),
            (ResizeHandle::Top, widget_rect.center_top()),
            (ResizeHandle::TopRight, widget_rect.right_top()),
            (ResizeHandle::Left, widget_rect.left_center()),
            (ResizeHandle::Right, widget_rect.right_center()),
            (ResizeHandle::BottomLeft, widget_rect.left_bottom()),
            (ResizeHandle::Bottom, widget_rect.center_bottom()),
            (ResizeHandle::BottomRight, widget_rect.right_bottom()),
        ];

        for (handle, center) in handles {
            let handle_rect = Rect::from_center_size(center, Vec2::splat(handle_size));
            if handle_rect.contains(pos) {
                return Some(handle);
            }
        }

        None
    }

    /// Draw edit mode overlay.
    pub fn draw_overlay(&self, ui: &mut egui::Ui, profile: &LayoutProfile, container: Rect) {
        if !self.is_editing() {
            return;
        }

        let painter = ui.painter();

        // Draw grid
        if self.should_show_grid() && self.grid_size > 0.0 {
            let grid_color = Color32::from_rgba_unmultiplied(100, 100, 100, 50);
            let step_x = container.width() * self.grid_size;
            let step_y = container.height() * self.grid_size;

            let mut x = container.left();
            while x <= container.right() {
                painter.line_segment(
                    [Pos2::new(x, container.top()), Pos2::new(x, container.bottom())],
                    Stroke::new(1.0, grid_color),
                );
                x += step_x;
            }

            let mut y = container.top();
            while y <= container.bottom() {
                painter.line_segment(
                    [Pos2::new(container.left(), y), Pos2::new(container.right(), y)],
                    Stroke::new(1.0, grid_color),
                );
                y += step_y;
            }
        }

        // Draw widget outlines
        for (i, widget) in profile.widgets.iter().enumerate() {
            if !widget.visible {
                continue;
            }

            let rect = widget.rect(container.width(), container.height());
            let rect = rect.translate(container.left_top().to_vec2());

            let is_selected = self.selected_widget == Some(i);
            let stroke_color = if is_selected {
                Color32::from_rgb(66, 133, 244)
            } else {
                Color32::from_rgba_unmultiplied(150, 150, 150, 100)
            };

            painter.rect_stroke(rect, 2.0, Stroke::new(2.0, stroke_color), StrokeKind::Middle);

            // Draw resize handles for selected widget
            if is_selected && widget.widget_type.is_resizable() {
                let handle_size = 8.0;
                let handle_color = Color32::WHITE;

                for pos in [
                    rect.right_center(),
                    rect.center_bottom(),
                    rect.right_bottom(),
                ] {
                    painter.circle_filled(pos, handle_size / 2.0, handle_color);
                    painter.circle_stroke(pos, handle_size / 2.0, Stroke::new(1.0, Color32::BLACK));
                }
            }
        }
    }
}
