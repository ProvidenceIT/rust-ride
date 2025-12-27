//! Touch input handling.
//!
//! Provides touch event processing and state tracking.

use egui::Pos2;

/// Touch event types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchEvent {
    /// Touch started
    Start { id: u64, pos: Pos2 },
    /// Touch moved
    Move { id: u64, pos: Pos2 },
    /// Touch ended
    End { id: u64, pos: Pos2 },
    /// Touch cancelled
    Cancel { id: u64 },
}

impl TouchEvent {
    /// Get the touch ID.
    pub fn id(&self) -> u64 {
        match self {
            TouchEvent::Start { id, .. } => *id,
            TouchEvent::Move { id, .. } => *id,
            TouchEvent::End { id, .. } => *id,
            TouchEvent::Cancel { id } => *id,
        }
    }

    /// Get the position, if available.
    pub fn pos(&self) -> Option<Pos2> {
        match self {
            TouchEvent::Start { pos, .. } => Some(*pos),
            TouchEvent::Move { pos, .. } => Some(*pos),
            TouchEvent::End { pos, .. } => Some(*pos),
            TouchEvent::Cancel { .. } => None,
        }
    }
}

/// State of an active touch.
#[derive(Debug, Clone)]
pub struct TouchState {
    /// Touch ID
    pub id: u64,
    /// Starting position
    pub start_pos: Pos2,
    /// Current position
    pub current_pos: Pos2,
    /// Whether this touch has moved significantly
    pub is_moving: bool,
    /// Start time (for tap detection)
    pub start_time: std::time::Instant,
}

impl TouchState {
    /// Create a new touch state.
    pub fn new(id: u64, pos: Pos2) -> Self {
        Self {
            id,
            start_pos: pos,
            current_pos: pos,
            is_moving: false,
            start_time: std::time::Instant::now(),
        }
    }

    /// Update position.
    pub fn update_pos(&mut self, pos: Pos2) {
        self.current_pos = pos;

        // Check if moved beyond threshold (10 pixels)
        let delta = self.current_pos - self.start_pos;
        if delta.length() > 10.0 {
            self.is_moving = true;
        }
    }

    /// Get the delta from start to current position.
    pub fn delta(&self) -> egui::Vec2 {
        self.current_pos - self.start_pos
    }

    /// Get the duration of this touch.
    pub fn duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Check if this is a tap (short duration, no movement).
    pub fn is_tap(&self) -> bool {
        !self.is_moving && self.duration().as_millis() < 300
    }

    /// Check if this is a long press.
    pub fn is_long_press(&self) -> bool {
        !self.is_moving && self.duration().as_millis() >= 500
    }
}

/// Touch input handler.
pub struct TouchHandler {
    /// Active touches
    touches: Vec<TouchState>,
    /// Minimum touch target size (WCAG 2.1 requires 44x44)
    min_target_size: f32,
}

impl Default for TouchHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchHandler {
    /// Create a new touch handler.
    pub fn new() -> Self {
        Self {
            touches: Vec::new(),
            min_target_size: 44.0,
        }
    }

    /// Handle a touch event.
    pub fn handle_event(&mut self, event: TouchEvent) {
        match event {
            TouchEvent::Start { id, pos } => {
                self.touches.push(TouchState::new(id, pos));
            }
            TouchEvent::Move { id, pos } => {
                if let Some(touch) = self.touches.iter_mut().find(|t| t.id == id) {
                    touch.update_pos(pos);
                }
            }
            TouchEvent::End { id, pos } => {
                if let Some(idx) = self.touches.iter().position(|t| t.id == id) {
                    let mut touch = self.touches.remove(idx);
                    touch.update_pos(pos);
                    // The touch state is available for gesture detection
                }
            }
            TouchEvent::Cancel { id } => {
                self.touches.retain(|t| t.id != id);
            }
        }
    }

    /// Get all active touches.
    pub fn active_touches(&self) -> &[TouchState] {
        &self.touches
    }

    /// Get the number of active touches.
    pub fn touch_count(&self) -> usize {
        self.touches.len()
    }

    /// Check if any touch is active.
    pub fn is_touching(&self) -> bool {
        !self.touches.is_empty()
    }

    /// Get the primary (first) touch.
    pub fn primary_touch(&self) -> Option<&TouchState> {
        self.touches.first()
    }

    /// Get the minimum touch target size.
    pub fn min_target_size(&self) -> f32 {
        self.min_target_size
    }

    /// Check if two touches are pinching (moving towards/away from each other).
    pub fn is_pinching(&self) -> bool {
        self.touches.len() >= 2
    }

    /// Get pinch scale factor (1.0 = no change).
    pub fn pinch_scale(&self) -> f32 {
        if self.touches.len() < 2 {
            return 1.0;
        }

        let touch1 = &self.touches[0];
        let touch2 = &self.touches[1];

        let start_dist = (touch1.start_pos - touch2.start_pos).length();
        let current_dist = (touch1.current_pos - touch2.current_pos).length();

        if start_dist > 0.0 {
            current_dist / start_dist
        } else {
            1.0
        }
    }

    /// Get the center point of all active touches.
    pub fn touch_center(&self) -> Option<Pos2> {
        if self.touches.is_empty() {
            return None;
        }

        let sum: egui::Vec2 = self
            .touches
            .iter()
            .map(|t| t.current_pos.to_vec2())
            .fold(egui::Vec2::ZERO, |acc, v| acc + v);

        Some(Pos2::new(
            sum.x / self.touches.len() as f32,
            sum.y / self.touches.len() as f32,
        ))
    }
}
