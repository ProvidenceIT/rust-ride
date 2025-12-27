//! Gesture recognition for touch and mouse input.
//!
//! Provides swipe, pinch, and other gesture detection.

use egui::{Pos2, Vec2};

/// Types of gestures that can be recognized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureType {
    /// Single tap
    Tap,
    /// Double tap
    DoubleTap,
    /// Long press
    LongPress,
    /// Swipe in a direction
    Swipe(SwipeDirection),
    /// Pinch zoom (scale factor)
    Pinch(f32),
    /// Two-finger pan
    Pan(Vec2),
}

/// Direction of a swipe gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SwipeDirection {
    /// Detect swipe direction from a movement vector.
    pub fn from_delta(delta: Vec2) -> Option<Self> {
        let min_distance = 50.0; // Minimum swipe distance

        if delta.length() < min_distance {
            return None;
        }

        // Determine primary direction
        if delta.x.abs() > delta.y.abs() {
            // Horizontal swipe
            if delta.x > 0.0 {
                Some(SwipeDirection::Right)
            } else {
                Some(SwipeDirection::Left)
            }
        } else {
            // Vertical swipe
            if delta.y > 0.0 {
                Some(SwipeDirection::Down)
            } else {
                Some(SwipeDirection::Up)
            }
        }
    }
}

impl std::fmt::Display for SwipeDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwipeDirection::Up => write!(f, "Up"),
            SwipeDirection::Down => write!(f, "Down"),
            SwipeDirection::Left => write!(f, "Left"),
            SwipeDirection::Right => write!(f, "Right"),
        }
    }
}

/// Gesture recognition handler.
pub struct GestureHandler {
    /// Last tap time for double-tap detection
    last_tap_time: Option<std::time::Instant>,
    /// Last tap position for double-tap detection
    last_tap_pos: Option<Pos2>,
    /// Double-tap time threshold
    double_tap_threshold: std::time::Duration,
    /// Double-tap distance threshold
    double_tap_distance: f32,
    /// Minimum swipe distance
    min_swipe_distance: f32,
    /// Long press duration
    long_press_duration: std::time::Duration,
}

impl Default for GestureHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureHandler {
    /// Create a new gesture handler.
    pub fn new() -> Self {
        Self {
            last_tap_time: None,
            last_tap_pos: None,
            double_tap_threshold: std::time::Duration::from_millis(300),
            double_tap_distance: 30.0,
            min_swipe_distance: 50.0,
            long_press_duration: std::time::Duration::from_millis(500),
        }
    }

    /// Detect a tap gesture.
    pub fn detect_tap(
        &mut self,
        pos: Pos2,
        duration: std::time::Duration,
        moved: bool,
    ) -> Option<GestureType> {
        // Must not have moved and must be quick
        if moved || duration > self.double_tap_threshold {
            return None;
        }

        // Check for double tap
        if let (Some(last_time), Some(last_pos)) = (self.last_tap_time, self.last_tap_pos) {
            let time_diff = last_time.elapsed();
            let dist = (pos - last_pos).length();

            if time_diff < self.double_tap_threshold && dist < self.double_tap_distance {
                self.last_tap_time = None;
                self.last_tap_pos = None;
                return Some(GestureType::DoubleTap);
            }
        }

        // Record this tap for potential double-tap
        self.last_tap_time = Some(std::time::Instant::now());
        self.last_tap_pos = Some(pos);

        Some(GestureType::Tap)
    }

    /// Detect a long press gesture.
    pub fn detect_long_press(&self, duration: std::time::Duration, moved: bool) -> bool {
        !moved && duration >= self.long_press_duration
    }

    /// Detect a swipe gesture from movement.
    pub fn detect_swipe(&self, start: Pos2, end: Pos2) -> Option<GestureType> {
        let delta = end - start;
        SwipeDirection::from_delta(delta).map(GestureType::Swipe)
    }

    /// Detect pinch gesture from two touch points.
    pub fn detect_pinch(
        &self,
        start1: Pos2,
        start2: Pos2,
        current1: Pos2,
        current2: Pos2,
    ) -> Option<GestureType> {
        let start_dist = (start1 - start2).length();
        let current_dist = (current1 - current2).length();

        if start_dist < 10.0 {
            return None;
        }

        let scale = current_dist / start_dist;

        // Only report significant scale changes
        if (scale - 1.0).abs() > 0.1 {
            Some(GestureType::Pinch(scale))
        } else {
            None
        }
    }

    /// Detect pan gesture from two touch points.
    pub fn detect_pan(&self, start_center: Pos2, current_center: Pos2) -> Option<GestureType> {
        let delta = current_center - start_center;

        if delta.length() > 10.0 {
            Some(GestureType::Pan(delta))
        } else {
            None
        }
    }

    /// Set the double-tap time threshold.
    pub fn set_double_tap_threshold(&mut self, duration: std::time::Duration) {
        self.double_tap_threshold = duration;
    }

    /// Set the minimum swipe distance.
    pub fn set_min_swipe_distance(&mut self, distance: f32) {
        self.min_swipe_distance = distance;
    }

    /// Set the long press duration.
    pub fn set_long_press_duration(&mut self, duration: std::time::Duration) {
        self.long_press_duration = duration;
    }
}
