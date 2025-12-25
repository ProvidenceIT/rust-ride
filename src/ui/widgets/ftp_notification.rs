//! FTP notification widget for auto-detected FTP changes.
//!
//! T073: Add FTP notification UI component
//! T074: Add FTP accept/dismiss dialog

use egui::{Color32, RichText, Ui, Window};

use crate::metrics::analytics::ftp_detection::{FtpConfidence, FtpEstimate, FtpMethod};

/// Action taken on an FTP notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtpNotificationAction {
    /// User accepted the new FTP
    Accept,
    /// User dismissed the notification
    Dismiss,
    /// No action taken yet
    Pending,
}

/// FTP notification widget state.
pub struct FtpNotification {
    /// The FTP estimate to display
    estimate: Option<FtpEstimate>,
    /// Current user FTP for comparison
    current_ftp: u16,
    /// Whether the dialog is visible
    visible: bool,
    /// Last action taken
    last_action: FtpNotificationAction,
}

impl Default for FtpNotification {
    fn default() -> Self {
        Self::new()
    }
}

impl FtpNotification {
    /// Create a new FTP notification widget.
    pub fn new() -> Self {
        Self {
            estimate: None,
            current_ftp: 0,
            visible: false,
            last_action: FtpNotificationAction::Pending,
        }
    }

    /// Set the FTP estimate to display.
    pub fn set_estimate(&mut self, estimate: FtpEstimate, current_ftp: u16) {
        self.estimate = Some(estimate);
        self.current_ftp = current_ftp;
        self.visible = true;
        self.last_action = FtpNotificationAction::Pending;
    }

    /// Clear the notification.
    pub fn clear(&mut self) {
        self.estimate = None;
        self.visible = false;
    }

    /// Check if an FTP estimate is pending.
    pub fn has_pending(&self) -> bool {
        self.visible && self.estimate.is_some()
    }

    /// Get the last action taken.
    pub fn last_action(&self) -> FtpNotificationAction {
        self.last_action
    }

    /// Get the accepted FTP value if user accepted.
    pub fn accepted_ftp(&self) -> Option<u16> {
        if self.last_action == FtpNotificationAction::Accept {
            self.estimate.as_ref().map(|e| e.ftp_watts)
        } else {
            None
        }
    }

    /// Show the notification dialog.
    ///
    /// Returns the action taken if any.
    pub fn show(&mut self, ctx: &egui::Context) -> FtpNotificationAction {
        if !self.visible {
            return FtpNotificationAction::Pending;
        }

        let estimate = match &self.estimate {
            Some(e) => e,
            None => return FtpNotificationAction::Pending,
        };

        let mut action = FtpNotificationAction::Pending;

        Window::new("New FTP Detected")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Header with confidence indicator
                    let confidence_color = confidence_color(estimate.confidence);
                    ui.label(
                        RichText::new(format!("{} Confidence", confidence_label(estimate.confidence)))
                            .color(confidence_color)
                            .strong(),
                    );

                    ui.add_space(16.0);

                    // Main FTP display
                    ui.label("Your estimated FTP:");
                    ui.heading(RichText::new(format!("{}W", estimate.ftp_watts)).size(32.0));

                    // Change from current
                    if self.current_ftp > 0 {
                        let diff = estimate.ftp_watts as i32 - self.current_ftp as i32;
                        let pct_change = (diff as f32 / self.current_ftp as f32) * 100.0;
                        let change_color = if diff > 0 {
                            Color32::from_rgb(50, 205, 50) // Green
                        } else {
                            Color32::from_rgb(220, 20, 60) // Red
                        };
                        ui.label(
                            RichText::new(format!(
                                "{:+}W ({:+.1}%) from current {}W",
                                diff, pct_change, self.current_ftp
                            ))
                            .color(change_color),
                        );
                    }

                    ui.add_space(8.0);

                    // Method used
                    ui.label(
                        RichText::new(format!("Method: {}", method_label(estimate.method)))
                            .weak()
                            .small(),
                    );

                    ui.add_space(16.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("Accept & Update Zones").strong())
                            .clicked()
                        {
                            action = FtpNotificationAction::Accept;
                            self.visible = false;
                            self.last_action = FtpNotificationAction::Accept;
                        }

                        if ui.button("Dismiss").clicked() {
                            action = FtpNotificationAction::Dismiss;
                            self.visible = false;
                            self.last_action = FtpNotificationAction::Dismiss;
                        }
                    });

                    ui.add_space(8.0);

                    // Info text
                    ui.label(
                        RichText::new(
                            "Accepting will update your power zones based on the new FTP.",
                        )
                        .weak()
                        .small(),
                    );
                });
            });

        action
    }

    /// Show a compact notification banner (for non-modal display).
    pub fn show_banner(&mut self, ui: &mut Ui) -> FtpNotificationAction {
        if !self.visible {
            return FtpNotificationAction::Pending;
        }

        let estimate = match &self.estimate {
            Some(e) => e,
            None => return FtpNotificationAction::Pending,
        };

        let mut action = FtpNotificationAction::Pending;

        ui.horizontal(|ui| {
            let confidence_color = confidence_color(estimate.confidence);
            ui.label(
                RichText::new("New FTP detected:")
                    .color(confidence_color)
                    .strong(),
            );

            ui.label(format!("{}W", estimate.ftp_watts));

            if ui.small_button("Accept").clicked() {
                action = FtpNotificationAction::Accept;
                self.visible = false;
                self.last_action = FtpNotificationAction::Accept;
            }

            if ui.small_button("Dismiss").clicked() {
                action = FtpNotificationAction::Dismiss;
                self.visible = false;
                self.last_action = FtpNotificationAction::Dismiss;
            }
        });

        action
    }
}

/// Get color for confidence level.
fn confidence_color(confidence: FtpConfidence) -> Color32 {
    match confidence {
        FtpConfidence::High => Color32::from_rgb(50, 205, 50),   // Green
        FtpConfidence::Medium => Color32::from_rgb(255, 165, 0), // Orange
        FtpConfidence::Low => Color32::from_rgb(220, 20, 60),    // Red
    }
}

/// Get label for confidence level.
fn confidence_label(confidence: FtpConfidence) -> &'static str {
    match confidence {
        FtpConfidence::High => "High",
        FtpConfidence::Medium => "Medium",
        FtpConfidence::Low => "Low",
    }
}

/// Get label for detection method.
fn method_label(method: FtpMethod) -> &'static str {
    match method {
        FtpMethod::TwentyMinute => "20-minute test (95% rule)",
        FtpMethod::ExtendedDuration => "Extended duration effort",
        FtpMethod::CriticalPower => "Critical Power model",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftp_notification_lifecycle() {
        let mut notification = FtpNotification::new();

        // Initially no pending
        assert!(!notification.has_pending());
        assert_eq!(notification.last_action(), FtpNotificationAction::Pending);

        // Set an estimate
        let estimate = FtpEstimate {
            ftp_watts: 280,
            method: FtpMethod::TwentyMinute,
            confidence: FtpConfidence::High,
            supporting_data: vec![(1200, 295)], // 20min at 295W -> FTP 280
        };
        notification.set_estimate(estimate, 260);

        // Now pending
        assert!(notification.has_pending());

        // Clear
        notification.clear();
        assert!(!notification.has_pending());
    }

    #[test]
    fn test_confidence_colors() {
        assert_eq!(
            confidence_color(FtpConfidence::High),
            Color32::from_rgb(50, 205, 50)
        );
        assert_eq!(
            confidence_color(FtpConfidence::Low),
            Color32::from_rgb(220, 20, 60)
        );
    }
}
