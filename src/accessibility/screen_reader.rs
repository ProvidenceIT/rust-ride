//! Screen reader support via accesskit integration.
//!
//! Provides accessible labels, live regions, and announcements for screen reader users.
//!
//! T116: Create ScreenReaderSupport trait implementation
//! T119: Implement live region for interval change announcements
//! T120: Implement metrics hotkey (Ctrl+M) for on-demand announcement
//! T122: Ensure all alerts/errors announced immediately

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Trait for providing screen reader support.
pub trait ScreenReaderSupport {
    /// Announce a message to the screen reader.
    fn announce(&self, message: &str);

    /// Announce an urgent/alert message (interrupts current speech).
    fn announce_urgent(&self, message: &str);

    /// Get the current metrics announcement.
    fn announce_metrics(&self) -> String;

    /// Check if screen reader support is enabled.
    fn is_enabled(&self) -> bool;

    /// Enable or disable screen reader support.
    fn set_enabled(&mut self, enabled: bool);
}

/// Accessible label for a widget.
#[derive(Debug, Clone)]
pub struct AccessibleLabel {
    /// The accessible name (read by screen reader)
    pub name: String,
    /// Optional description for more context
    pub description: Option<String>,
    /// Role hint (button, slider, etc.)
    pub role: AccessibleRole,
}

impl AccessibleLabel {
    /// Create a new accessible label with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            role: AccessibleRole::Generic,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the role.
    pub fn with_role(mut self, role: AccessibleRole) -> Self {
        self.role = role;
        self
    }

    /// Create a button label.
    pub fn button(name: impl Into<String>) -> Self {
        Self::new(name).with_role(AccessibleRole::Button)
    }

    /// Create a slider label.
    pub fn slider(name: impl Into<String>) -> Self {
        Self::new(name).with_role(AccessibleRole::Slider)
    }

    /// Create a toggle label.
    pub fn toggle(name: impl Into<String>, checked: bool) -> Self {
        Self::new(name)
            .with_role(AccessibleRole::Toggle)
            .with_description(if checked { "checked" } else { "unchecked" })
    }
}

/// Accessible role hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessibleRole {
    /// Generic widget
    #[default]
    Generic,
    /// Button (activatable)
    Button,
    /// Toggle/checkbox
    Toggle,
    /// Slider/range input
    Slider,
    /// Text input
    TextInput,
    /// Link
    Link,
    /// Heading
    Heading,
    /// Live region (auto-announced updates)
    LiveRegion,
    /// Alert (urgent live region)
    Alert,
}

/// Live region for dynamic content updates.
#[derive(Debug, Clone)]
pub struct LiveRegion {
    /// Content to be announced
    content: String,
    /// Whether this is an assertive (interrupting) announcement
    assertive: bool,
    /// Whether content has changed since last read
    dirty: bool,
}

impl LiveRegion {
    /// Create a new polite live region.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            assertive: false,
            dirty: false,
        }
    }

    /// Create an assertive (interrupting) live region.
    pub fn assertive() -> Self {
        Self {
            content: String::new(),
            assertive: true,
            dirty: false,
        }
    }

    /// Update the content (triggers announcement if changed).
    pub fn set_content(&mut self, content: impl Into<String>) {
        let new_content = content.into();
        if self.content != new_content {
            self.content = new_content;
            self.dirty = true;
        }
    }

    /// Get the content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if content needs to be announced.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as announced.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Check if this is an assertive region.
    pub fn is_assertive(&self) -> bool {
        self.assertive
    }
}

impl Default for LiveRegion {
    fn default() -> Self {
        Self::new()
    }
}

/// Formats a metric value for screen reader announcement.
pub fn format_metric_announcement(name: &str, value: &str, unit: Option<&str>) -> String {
    match unit {
        Some(u) => format!("{}: {} {}", name, value, u),
        None => format!("{}: {}", name, value),
    }
}

/// Formats all current metrics for a full announcement.
pub fn format_all_metrics_announcement(
    power: u16,
    heart_rate: Option<u8>,
    cadence: Option<u8>,
    duration_secs: u32,
) -> String {
    let mut parts = vec![format!("Power {} watts", power)];

    if let Some(hr) = heart_rate {
        parts.push(format!("Heart rate {} bpm", hr));
    }

    if let Some(cad) = cadence {
        parts.push(format!("Cadence {} rpm", cad));
    }

    let hours = duration_secs / 3600;
    let minutes = (duration_secs % 3600) / 60;
    let seconds = duration_secs % 60;

    if hours > 0 {
        parts.push(format!(
            "Duration {} hours {} minutes {} seconds",
            hours, minutes, seconds
        ));
    } else if minutes > 0 {
        parts.push(format!("Duration {} minutes {} seconds", minutes, seconds));
    } else {
        parts.push(format!("Duration {} seconds", seconds));
    }

    parts.join(". ")
}

/// T116: Default implementation of screen reader support.
pub struct DefaultScreenReaderSupport {
    /// Whether screen reader support is enabled
    enabled: bool,
    /// Queue of pending announcements (polite)
    announcement_queue: Arc<Mutex<VecDeque<String>>>,
    /// Queue of urgent announcements (assertive)
    urgent_queue: Arc<Mutex<VecDeque<String>>>,
    /// T119: Live region for interval changes
    interval_region: LiveRegion,
    /// T122: Live region for alerts/errors
    alert_region: LiveRegion,
    /// Current metrics for on-demand announcement
    current_metrics: Arc<Mutex<CurrentMetrics>>,
}

/// Current metrics state for announcements.
#[derive(Debug, Clone, Default)]
pub struct CurrentMetrics {
    /// Power in watts
    pub power: u16,
    /// Heart rate in bpm
    pub heart_rate: Option<u8>,
    /// Cadence in rpm
    pub cadence: Option<u8>,
    /// Duration in seconds
    pub duration_secs: u32,
    /// Current power zone (1-7)
    pub power_zone: Option<u8>,
    /// Current interval name
    pub interval_name: Option<String>,
}

impl Default for DefaultScreenReaderSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultScreenReaderSupport {
    /// Create a new screen reader support instance.
    pub fn new() -> Self {
        Self {
            enabled: false,
            announcement_queue: Arc::new(Mutex::new(VecDeque::new())),
            urgent_queue: Arc::new(Mutex::new(VecDeque::new())),
            interval_region: LiveRegion::new(),
            alert_region: LiveRegion::assertive(),
            current_metrics: Arc::new(Mutex::new(CurrentMetrics::default())),
        }
    }

    /// T119: Announce an interval change.
    pub fn announce_interval_change(&mut self, interval_name: &str, target_power: Option<u16>) {
        let message = match target_power {
            Some(power) => format!("Interval: {}, target {} watts", interval_name, power),
            None => format!("Interval: {}", interval_name),
        };
        self.interval_region.set_content(&message);
        if self.enabled {
            self.announce(&message);
        }
    }

    /// T122: Announce an alert immediately.
    pub fn announce_alert(&mut self, alert_type: &str, message: &str) {
        let full_message = format!("{}: {}", alert_type, message);
        self.alert_region.set_content(&full_message);
        if self.enabled {
            self.announce_urgent(&full_message);
        }
    }

    /// T122: Announce an error immediately.
    pub fn announce_error(&mut self, error: &str) {
        self.announce_alert("Error", error);
    }

    /// Update current metrics.
    pub fn update_metrics(&self, metrics: CurrentMetrics) {
        let mut current = self.current_metrics.lock().unwrap();
        *current = metrics;
    }

    /// Get the next polite announcement (for processing).
    pub fn pop_announcement(&self) -> Option<String> {
        self.announcement_queue.lock().unwrap().pop_front()
    }

    /// Get the next urgent announcement (for processing).
    pub fn pop_urgent(&self) -> Option<String> {
        self.urgent_queue.lock().unwrap().pop_front()
    }

    /// Check if there are pending announcements.
    pub fn has_pending(&self) -> bool {
        !self.announcement_queue.lock().unwrap().is_empty()
            || !self.urgent_queue.lock().unwrap().is_empty()
    }

    /// Get the interval live region.
    pub fn interval_region(&self) -> &LiveRegion {
        &self.interval_region
    }

    /// Get the alert live region.
    pub fn alert_region(&self) -> &LiveRegion {
        &self.alert_region
    }

    /// T120: Handle metrics hotkey (Ctrl+M).
    pub fn handle_metrics_hotkey(&self) {
        if !self.enabled {
            return;
        }

        let metrics = self.current_metrics.lock().unwrap();
        let announcement = format_all_metrics_announcement(
            metrics.power,
            metrics.heart_rate,
            metrics.cadence,
            metrics.duration_secs,
        );

        // Add zone information if available
        let full_announcement = if let Some(zone) = metrics.power_zone {
            format!("{}. Power zone {}", announcement, zone)
        } else {
            announcement
        };

        self.announcement_queue
            .lock()
            .unwrap()
            .push_back(full_announcement);
    }
}

impl ScreenReaderSupport for DefaultScreenReaderSupport {
    fn announce(&self, message: &str) {
        if self.enabled {
            self.announcement_queue
                .lock()
                .unwrap()
                .push_back(message.to_string());
            tracing::debug!("Screen reader announce: {}", message);
        }
    }

    fn announce_urgent(&self, message: &str) {
        if self.enabled {
            self.urgent_queue
                .lock()
                .unwrap()
                .push_back(message.to_string());
            tracing::debug!("Screen reader urgent: {}", message);
        }
    }

    fn announce_metrics(&self) -> String {
        let metrics = self.current_metrics.lock().unwrap();
        format_all_metrics_announcement(
            metrics.power,
            metrics.heart_rate,
            metrics.cadence,
            metrics.duration_secs,
        )
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            tracing::info!("Screen reader support enabled");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_reader_support() {
        let mut sr = DefaultScreenReaderSupport::new();
        assert!(!sr.is_enabled());

        sr.set_enabled(true);
        assert!(sr.is_enabled());

        sr.announce("Test message");
        assert!(sr.has_pending());
        assert_eq!(sr.pop_announcement(), Some("Test message".to_string()));
    }

    #[test]
    fn test_interval_announcement() {
        let mut sr = DefaultScreenReaderSupport::new();
        sr.set_enabled(true);

        sr.announce_interval_change("Threshold", Some(250));
        assert!(sr.has_pending());
    }

    #[test]
    fn test_metrics_announcement() {
        let sr = DefaultScreenReaderSupport::new();
        sr.update_metrics(CurrentMetrics {
            power: 200,
            heart_rate: Some(150),
            cadence: Some(90),
            duration_secs: 3600,
            power_zone: Some(3),
            interval_name: None,
        });

        let announcement = sr.announce_metrics();
        assert!(announcement.contains("200 watts"));
        assert!(announcement.contains("150 bpm"));
    }
}
