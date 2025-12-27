//! Screen reader support via accesskit integration.
//!
//! Provides accessible labels, live regions, and announcements for screen reader users.

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
