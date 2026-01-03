//! Sync notification widget for ride upload status.
//!
//! 3.5: Display toast/notification when ride sync starts and completes.
//!
//! Displays notifications for sync events including upload started,
//! upload completed, and upload failed with retry option.

use std::collections::VecDeque;
use std::time::Instant;

use egui::{Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};
use uuid::Uuid;

use crate::integrations::sync::{SyncEvent, SyncPlatform};

/// Strava brand color
const STRAVA_ORANGE: Color32 = Color32::from_rgb(252, 82, 0);

/// Success color (green)
const SUCCESS_GREEN: Color32 = Color32::from_rgb(52, 168, 83);

/// Error color (red)
const ERROR_RED: Color32 = Color32::from_rgb(234, 67, 53);

/// Warning color (amber)
const WARNING_AMBER: Color32 = Color32::from_rgb(251, 188, 4);

/// Configuration for sync notification display.
#[derive(Debug, Clone)]
pub struct SyncNotificationConfig {
    /// Width of the notification popup.
    pub width: f32,
    /// Height of the notification popup.
    pub height: f32,
    /// Time to display before auto-dismissing (seconds).
    pub display_duration_secs: f32,
    /// Offset from bottom-right corner.
    pub offset: Vec2,
    /// Animation duration for slide-in/out.
    pub animation_duration_secs: f32,
    /// Maximum queue size.
    pub max_queue_size: usize,
}

impl Default for SyncNotificationConfig {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 100.0,
            display_duration_secs: 5.0,
            offset: Vec2::new(-20.0, -20.0),
            animation_duration_secs: 0.3,
            max_queue_size: 10,
        }
    }
}

/// Type of sync notification.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncNotificationType {
    /// Upload started
    Started {
        ride_id: Uuid,
        platform: SyncPlatform,
    },
    /// Upload completed successfully
    Completed {
        ride_id: Uuid,
        platform: SyncPlatform,
        external_url: Option<String>,
    },
    /// Upload failed
    Failed {
        ride_id: Uuid,
        platform: SyncPlatform,
        error: String,
        will_retry: bool,
    },
    /// Re-authorization required
    ReauthRequired {
        platform: SyncPlatform,
    },
    /// Connectivity changed
    ConnectivityChanged {
        is_online: bool,
    },
    /// Workout sync started (for TrainingPeaks workout plan sync)
    WorkoutSyncStarted {
        platform: SyncPlatform,
    },
    /// Workout sync completed successfully
    WorkoutSyncCompleted {
        platform: SyncPlatform,
        /// Number of new workouts synced
        new_workouts: usize,
        /// Number of updated workouts
        updated_workouts: usize,
    },
    /// Workout sync failed
    WorkoutSyncFailed {
        platform: SyncPlatform,
        error: String,
    },
}

impl SyncNotificationType {
    /// Get the display title for this notification type.
    pub fn title(&self) -> String {
        match self {
            SyncNotificationType::Started { platform, .. } => {
                format!("Uploading to {}", platform.display_name())
            }
            SyncNotificationType::Completed { platform, .. } => {
                format!("Synced to {}", platform.display_name())
            }
            SyncNotificationType::Failed { platform, .. } => {
                format!("{} sync failed", platform.display_name())
            }
            SyncNotificationType::ReauthRequired { platform } => {
                format!("{} requires reconnection", platform.display_name())
            }
            SyncNotificationType::ConnectivityChanged { is_online } => {
                if *is_online {
                    "Back online".to_string()
                } else {
                    "Offline".to_string()
                }
            }
            SyncNotificationType::WorkoutSyncStarted { platform } => {
                format!("Syncing {} workouts", platform.display_name())
            }
            SyncNotificationType::WorkoutSyncCompleted { platform, .. } => {
                format!("{} workouts synced", platform.display_name())
            }
            SyncNotificationType::WorkoutSyncFailed { platform, .. } => {
                format!("{} workout sync failed", platform.display_name())
            }
        }
    }

    /// Get the description for this notification type.
    pub fn description(&self) -> String {
        match self {
            SyncNotificationType::Started { .. } => "Your ride is being uploaded...".to_string(),
            SyncNotificationType::Completed { external_url, .. } => {
                if external_url.is_some() {
                    "Ride uploaded successfully. Click to view.".to_string()
                } else {
                    "Ride uploaded successfully.".to_string()
                }
            }
            SyncNotificationType::Failed { error, will_retry, .. } => {
                if *will_retry {
                    format!("{} Will retry automatically.", error)
                } else {
                    error.clone()
                }
            }
            SyncNotificationType::ReauthRequired { .. } => {
                "Please reconnect your account in Settings.".to_string()
            }
            SyncNotificationType::ConnectivityChanged { is_online } => {
                if *is_online {
                    "Syncing will resume automatically.".to_string()
                } else {
                    "Pending uploads will sync when connection is restored.".to_string()
                }
            }
            SyncNotificationType::WorkoutSyncStarted { .. } => {
                "Downloading workout plans...".to_string()
            }
            SyncNotificationType::WorkoutSyncCompleted {
                new_workouts,
                updated_workouts,
                ..
            } => {
                match (*new_workouts, *updated_workouts) {
                    (0, 0) => "Workouts are up to date.".to_string(),
                    (n, 0) => format!("{} new workout{} added.", n, if n == 1 { "" } else { "s" }),
                    (0, u) => format!("{} workout{} updated.", u, if u == 1 { "" } else { "s" }),
                    (n, u) => format!(
                        "{} new, {} updated workout{}.",
                        n,
                        u,
                        if n + u == 1 { "" } else { "s" }
                    ),
                }
            }
            SyncNotificationType::WorkoutSyncFailed { error, .. } => error.clone(),
        }
    }

    /// Get the accent color for this notification type.
    pub fn color(&self) -> Color32 {
        match self {
            SyncNotificationType::Started { platform, .. } => platform_color(*platform),
            SyncNotificationType::Completed { platform, .. } => platform_color(*platform),
            SyncNotificationType::Failed { .. } => ERROR_RED,
            SyncNotificationType::ReauthRequired { .. } => WARNING_AMBER,
            SyncNotificationType::ConnectivityChanged { is_online } => {
                if *is_online {
                    SUCCESS_GREEN
                } else {
                    WARNING_AMBER
                }
            }
            SyncNotificationType::WorkoutSyncStarted { platform } => platform_color(*platform),
            SyncNotificationType::WorkoutSyncCompleted { platform, .. } => platform_color(*platform),
            SyncNotificationType::WorkoutSyncFailed { .. } => ERROR_RED,
        }
    }

    /// Get the background color for this notification type.
    pub fn background_color(&self) -> Color32 {
        match self {
            SyncNotificationType::Started { .. } => Color32::from_rgba_unmultiplied(40, 40, 50, 240),
            SyncNotificationType::Completed { .. } => {
                Color32::from_rgba_unmultiplied(30, 60, 40, 240)
            }
            SyncNotificationType::Failed { .. } => Color32::from_rgba_unmultiplied(60, 30, 30, 240),
            SyncNotificationType::ReauthRequired { .. } => {
                Color32::from_rgba_unmultiplied(60, 50, 20, 240)
            }
            SyncNotificationType::ConnectivityChanged { is_online } => {
                if *is_online {
                    Color32::from_rgba_unmultiplied(30, 60, 40, 240)
                } else {
                    Color32::from_rgba_unmultiplied(60, 50, 20, 240)
                }
            }
            SyncNotificationType::WorkoutSyncStarted { .. } => {
                Color32::from_rgba_unmultiplied(40, 40, 50, 240)
            }
            SyncNotificationType::WorkoutSyncCompleted { .. } => {
                Color32::from_rgba_unmultiplied(30, 60, 40, 240)
            }
            SyncNotificationType::WorkoutSyncFailed { .. } => {
                Color32::from_rgba_unmultiplied(60, 30, 30, 240)
            }
        }
    }

    /// Get the icon for this notification type.
    pub fn icon(&self) -> &'static str {
        match self {
            SyncNotificationType::Started { .. } => "\u{2191}", // Up arrow
            SyncNotificationType::Completed { .. } => "\u{2713}", // Checkmark
            SyncNotificationType::Failed { .. } => "\u{2717}",  // X mark
            SyncNotificationType::ReauthRequired { .. } => "\u{26A0}", // Warning
            SyncNotificationType::ConnectivityChanged { is_online } => {
                if *is_online {
                    "\u{2713}" // Checkmark
                } else {
                    "\u{2022}" // Bullet point
                }
            }
            SyncNotificationType::WorkoutSyncStarted { .. } => "\u{2193}", // Down arrow (downloading)
            SyncNotificationType::WorkoutSyncCompleted { .. } => "\u{2713}", // Checkmark
            SyncNotificationType::WorkoutSyncFailed { .. } => "\u{2717}", // X mark
        }
    }

    /// Check if this notification has a clickable link.
    pub fn has_link(&self) -> bool {
        matches!(
            self,
            SyncNotificationType::Completed {
                external_url: Some(_),
                ..
            }
        )
    }

    /// Get the link URL if available.
    pub fn link_url(&self) -> Option<&str> {
        match self {
            SyncNotificationType::Completed { external_url, .. } => external_url.as_deref(),
            _ => None,
        }
    }
}

/// Get platform-specific color.
fn platform_color(platform: SyncPlatform) -> Color32 {
    match platform {
        SyncPlatform::Strava => STRAVA_ORANGE,
        SyncPlatform::GarminConnect => Color32::from_rgb(30, 144, 255), // Dodger blue
        SyncPlatform::TrainingPeaks => Color32::from_rgb(0, 128, 128),  // Teal
        SyncPlatform::IntervalsIcu => Color32::from_rgb(138, 43, 226),  // Blue violet
        #[cfg(target_os = "macos")]
        SyncPlatform::HealthKit => Color32::from_rgb(255, 45, 85), // iOS red
    }
}

/// A sync notification item in the queue.
#[derive(Debug, Clone)]
pub struct SyncNotificationItem {
    /// Unique ID for this notification.
    pub id: Uuid,
    /// The notification type and data.
    pub notification_type: SyncNotificationType,
    /// When this notification was created.
    pub created_at: Instant,
}

impl SyncNotificationItem {
    /// Create a new notification item.
    pub fn new(notification_type: SyncNotificationType) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type,
            created_at: Instant::now(),
        }
    }
}

/// Queue for managing sync notifications.
#[derive(Debug, Default)]
pub struct SyncNotificationQueue {
    /// Pending notifications.
    queue: VecDeque<SyncNotificationItem>,
    /// Maximum queue size.
    max_size: usize,
    /// Configuration.
    config: SyncNotificationConfig,
}

impl SyncNotificationQueue {
    /// Create a new notification queue.
    pub fn new() -> Self {
        let config = SyncNotificationConfig::default();
        Self {
            queue: VecDeque::new(),
            max_size: config.max_queue_size,
            config,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SyncNotificationConfig) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size: config.max_queue_size,
            config,
        }
    }

    /// Push a new notification to the queue.
    pub fn push(&mut self, notification_type: SyncNotificationType) {
        // Don't add duplicate started notifications for the same ride
        if let SyncNotificationType::Started { ride_id, platform } = &notification_type {
            if self.queue.iter().any(|n| {
                matches!(&n.notification_type, SyncNotificationType::Started { ride_id: r, platform: p } if r == ride_id && p == platform)
            }) {
                return;
            }
        }

        let item = SyncNotificationItem::new(notification_type);
        self.queue.push_back(item);

        // Limit queue size
        while self.queue.len() > self.max_size {
            self.queue.pop_front();
        }
    }

    /// Handle a SyncEvent and convert it to a notification.
    pub fn handle_event(&mut self, event: &SyncEvent) {
        match event {
            SyncEvent::UploadStarted {
                ride_id, platform, ..
            } => {
                self.push(SyncNotificationType::Started {
                    ride_id: *ride_id,
                    platform: *platform,
                });
            }
            SyncEvent::UploadCompleted {
                ride_id,
                platform,
                external_url,
                ..
            } => {
                // Remove any started notification for this ride
                self.queue.retain(|n| {
                    !matches!(&n.notification_type, SyncNotificationType::Started { ride_id: r, platform: p } if r == ride_id && p == platform)
                });
                self.push(SyncNotificationType::Completed {
                    ride_id: *ride_id,
                    platform: *platform,
                    external_url: external_url.clone(),
                });
            }
            SyncEvent::UploadFailed {
                ride_id,
                platform,
                error,
                will_retry,
                ..
            } => {
                // Remove any started notification for this ride
                self.queue.retain(|n| {
                    !matches!(&n.notification_type, SyncNotificationType::Started { ride_id: r, platform: p } if r == ride_id && p == platform)
                });
                self.push(SyncNotificationType::Failed {
                    ride_id: *ride_id,
                    platform: *platform,
                    error: error.clone(),
                    will_retry: *will_retry,
                });
            }
            SyncEvent::ReauthorizationRequired { platform } => {
                self.push(SyncNotificationType::ReauthRequired {
                    platform: *platform,
                });
            }
            SyncEvent::ConnectivityChanged { is_online } => {
                self.push(SyncNotificationType::ConnectivityChanged {
                    is_online: *is_online,
                });
            }
            // Ignore other events for notifications
            _ => {}
        }
    }

    /// Get the current notification (oldest in queue).
    pub fn current(&self) -> Option<&SyncNotificationItem> {
        self.queue.front()
    }

    /// Dismiss the current notification.
    pub fn dismiss_current(&mut self) {
        self.queue.pop_front();
    }

    /// Get the number of pending notifications.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clear all notifications.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Update the queue, removing expired notifications.
    pub fn update(&mut self) {
        let display_duration = self.config.display_duration_secs;
        self.queue.retain(|n| {
            n.created_at.elapsed().as_secs_f32() < display_duration + 1.0
        });
    }

    /// Get display progress (0.0 to 1.0) for the current notification.
    pub fn display_progress(&self) -> f32 {
        self.queue.front().map_or(0.0, |n| {
            (n.created_at.elapsed().as_secs_f32() / self.config.display_duration_secs).min(1.0)
        })
    }
}

/// Action result from sync notification interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncNotificationAction {
    /// No action taken.
    None,
    /// User dismissed the notification.
    Dismissed,
    /// User clicked the link to view activity.
    ViewActivity { url: String },
    /// User clicked retry on a failed notification.
    Retry { ride_id: Uuid, platform: SyncPlatform },
}

/// Widget for displaying sync notifications.
pub struct SyncNotificationWidget {
    /// Display configuration.
    config: SyncNotificationConfig,
    /// Animation progress (0.0 to 1.0).
    animation_progress: f32,
    /// Whether currently animating in.
    animating_in: bool,
    /// Animation start time.
    animation_start: Option<Instant>,
    /// ID of the notification being displayed.
    current_notification_id: Option<Uuid>,
}

impl Default for SyncNotificationWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncNotificationWidget {
    /// Create a new sync notification widget.
    pub fn new() -> Self {
        Self {
            config: SyncNotificationConfig::default(),
            animation_progress: 0.0,
            animating_in: true,
            animation_start: None,
            current_notification_id: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SyncNotificationConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Show the notification queue.
    ///
    /// Call this once per frame to display any pending notifications.
    /// Returns an action if the user interacted with the notification.
    pub fn show(&mut self, ui: &mut Ui, queue: &mut SyncNotificationQueue) -> SyncNotificationAction {
        // Update the queue
        queue.update();

        // Get current notification if any
        let notification = match queue.current() {
            Some(n) => n.clone(),
            None => {
                // Reset animation state when no notification
                self.animation_progress = 0.0;
                self.animation_start = None;
                self.current_notification_id = None;
                return SyncNotificationAction::None;
            }
        };

        // Check if this is a new notification
        if self.current_notification_id != Some(notification.id) {
            self.current_notification_id = Some(notification.id);
            self.animation_start = Some(Instant::now());
            self.animating_in = true;
            self.animation_progress = 0.0;
        }

        // Update animation
        self.update_animation();

        // Calculate position (bottom-right with offset)
        let screen_rect = ui.ctx().available_rect();
        let x = screen_rect.max.x + self.config.offset.x - self.config.width;
        let y = screen_rect.max.y + self.config.offset.y - self.config.height;

        // Apply slide animation from bottom
        let slide_offset = (1.0 - self.animation_progress) * (self.config.height + 40.0);
        let final_y = y + slide_offset;

        let rect = Rect::from_min_size(
            Pos2::new(x, final_y),
            Vec2::new(self.config.width, self.config.height),
        );

        // Draw background
        let bg_color = notification.notification_type.background_color();
        let border_color = notification.notification_type.color();

        ui.painter().rect_filled(rect, 8.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(2.0, border_color),
            StrokeKind::Middle,
        );

        // Draw content
        let content_rect = rect.shrink(12.0);
        let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));

        let action = self.draw_content(&mut ui_child, &notification, queue);

        // Request repaint for animation
        if self.animation_progress < 1.0 {
            ui.ctx().request_repaint();
        }

        action
    }

    /// Update animation state.
    fn update_animation(&mut self) {
        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = self.config.animation_duration_secs;

            if self.animating_in {
                self.animation_progress = (elapsed / duration).clamp(0.0, 1.0);
                // Apply easing (ease-out)
                self.animation_progress = ease_out_cubic(self.animation_progress);
            }
        }
    }

    /// Draw notification content.
    fn draw_content(
        &self,
        ui: &mut Ui,
        notification: &SyncNotificationItem,
        queue: &mut SyncNotificationQueue,
    ) -> SyncNotificationAction {
        let mut action = SyncNotificationAction::None;
        let notification_type = &notification.notification_type;

        ui.vertical(|ui| {
            // Header row: icon + title + dismiss button
            ui.horizontal(|ui| {
                // Icon
                let icon_color = notification_type.color();
                ui.label(
                    RichText::new(notification_type.icon())
                        .color(icon_color)
                        .strong()
                        .size(16.0),
                );

                // Title
                ui.label(
                    RichText::new(notification_type.title())
                        .strong()
                        .size(14.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Dismiss button
                    if ui.small_button("\u{2715}").clicked() {
                        queue.dismiss_current();
                        action = SyncNotificationAction::Dismissed;
                    }
                });
            });

            ui.add_space(4.0);

            // Description
            ui.label(
                RichText::new(notification_type.description())
                    .weak()
                    .size(12.0),
            );

            ui.add_space(4.0);

            // Action row: progress bar, link, or retry button
            ui.horizontal(|ui| {
                // Show action buttons based on notification type
                match notification_type {
                    SyncNotificationType::Completed {
                        platform,
                        external_url: Some(url),
                        ..
                    } => {
                        let button_text = format!("View on {}", platform.display_name());
                        if ui.small_button(button_text).clicked() {
                            action = SyncNotificationAction::ViewActivity { url: url.clone() };
                        }
                    }
                    SyncNotificationType::Failed { ride_id, platform, will_retry, .. } => {
                        if !will_retry {
                            if ui.small_button("Retry").clicked() {
                                action = SyncNotificationAction::Retry {
                                    ride_id: *ride_id,
                                    platform: *platform,
                                };
                            }
                        }
                    }
                    _ => {}
                }

                // Progress bar showing time remaining (right-aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let progress = queue.display_progress();
                    let remaining = 1.0 - progress;
                    let bar_width = 60.0;

                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(bar_width, 3.0), egui::Sense::hover());

                    // Background
                    ui.painter().rect_filled(rect, 1.5, Color32::from_gray(60));

                    // Progress
                    let filled_rect = Rect::from_min_size(
                        rect.min,
                        Vec2::new(rect.width() * remaining, rect.height()),
                    );
                    ui.painter()
                        .rect_filled(filled_rect, 1.5, notification_type.color());

                    // Queue indicator
                    let pending = queue.pending_count();
                    if pending > 1 {
                        ui.label(
                            RichText::new(format!("+{}", pending - 1))
                                .weak()
                                .small(),
                        );
                    }
                });
            });
        });

        action
    }
}

/// Ease out cubic function for smooth animation.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SyncNotificationConfig::default();
        assert!(config.width > 0.0);
        assert!(config.height > 0.0);
        assert!(config.display_duration_secs > 0.0);
        assert!(config.max_queue_size > 0);
    }

    #[test]
    fn test_notification_type_titles() {
        let started = SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        };
        assert!(started.title().contains("Strava"));

        let completed = SyncNotificationType::Completed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            external_url: None,
        };
        assert!(completed.title().contains("Synced"));

        let failed = SyncNotificationType::Failed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            error: "Test error".to_string(),
            will_retry: false,
        };
        assert!(failed.title().contains("failed"));
    }

    #[test]
    fn test_notification_queue_push() {
        let mut queue = SyncNotificationQueue::new();
        assert!(queue.is_empty());

        queue.push(SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        });

        assert_eq!(queue.pending_count(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_notification_queue_dismiss() {
        let mut queue = SyncNotificationQueue::new();
        queue.push(SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        });

        queue.dismiss_current();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_notification_queue_deduplicate_started() {
        let mut queue = SyncNotificationQueue::new();
        let ride_id = Uuid::new_v4();

        queue.push(SyncNotificationType::Started {
            ride_id,
            platform: SyncPlatform::Strava,
        });
        queue.push(SyncNotificationType::Started {
            ride_id,
            platform: SyncPlatform::Strava,
        });

        // Should only have one notification
        assert_eq!(queue.pending_count(), 1);
    }

    #[test]
    fn test_notification_queue_max_size() {
        let config = SyncNotificationConfig {
            max_queue_size: 3,
            ..Default::default()
        };
        let mut queue = SyncNotificationQueue::with_config(config);

        // Push 5 notifications
        for _ in 0..5 {
            queue.push(SyncNotificationType::ConnectivityChanged { is_online: true });
        }

        // Should only keep 3
        assert_eq!(queue.pending_count(), 3);
    }

    #[test]
    fn test_handle_upload_started_event() {
        let mut queue = SyncNotificationQueue::new();
        let ride_id = Uuid::new_v4();

        queue.handle_event(&SyncEvent::UploadStarted {
            record_id: Uuid::new_v4(),
            ride_id,
            platform: SyncPlatform::Strava,
        });

        assert_eq!(queue.pending_count(), 1);
        let current = queue.current().unwrap();
        assert!(matches!(
            current.notification_type,
            SyncNotificationType::Started { .. }
        ));
    }

    #[test]
    fn test_handle_upload_completed_removes_started() {
        let mut queue = SyncNotificationQueue::new();
        let ride_id = Uuid::new_v4();

        // First add started
        queue.handle_event(&SyncEvent::UploadStarted {
            record_id: Uuid::new_v4(),
            ride_id,
            platform: SyncPlatform::Strava,
        });

        // Then completed
        queue.handle_event(&SyncEvent::UploadCompleted {
            record_id: Uuid::new_v4(),
            ride_id,
            platform: SyncPlatform::Strava,
            external_id: Some("12345".to_string()),
            external_url: Some("https://www.strava.com/activities/12345".to_string()),
        });

        // Should only have the completed notification
        assert_eq!(queue.pending_count(), 1);
        let current = queue.current().unwrap();
        assert!(matches!(
            current.notification_type,
            SyncNotificationType::Completed { .. }
        ));
    }

    #[test]
    fn test_notification_has_link() {
        let with_link = SyncNotificationType::Completed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            external_url: Some("https://strava.com/activities/123".to_string()),
        };
        assert!(with_link.has_link());

        let without_link = SyncNotificationType::Completed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
            external_url: None,
        };
        assert!(!without_link.has_link());

        let started = SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        };
        assert!(!started.has_link());
    }

    #[test]
    fn test_platform_colors() {
        assert_eq!(platform_color(SyncPlatform::Strava), STRAVA_ORANGE);
        // Ensure all platforms have a color
        assert_ne!(platform_color(SyncPlatform::GarminConnect), Color32::TRANSPARENT);
        assert_ne!(platform_color(SyncPlatform::TrainingPeaks), Color32::TRANSPARENT);
    }

    #[test]
    fn test_ease_out_cubic() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);
        // Ease out should be > linear at midpoint
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn test_widget_creation() {
        let widget = SyncNotificationWidget::new();
        assert!(widget.animation_start.is_none());
        assert!(widget.current_notification_id.is_none());
    }

    #[test]
    fn test_notification_action_variants() {
        let none = SyncNotificationAction::None;
        let dismissed = SyncNotificationAction::Dismissed;
        let view = SyncNotificationAction::ViewActivity {
            url: "https://test.com".to_string(),
        };
        let retry = SyncNotificationAction::Retry {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        };

        // Just verify they can be created and compared
        assert_ne!(none, dismissed);
        assert_ne!(view, retry);
    }

    // TrainingPeaks-specific tests

    #[test]
    fn test_trainingpeaks_platform_color() {
        let tp_color = platform_color(SyncPlatform::TrainingPeaks);
        // TrainingPeaks teal color
        assert_eq!(tp_color, Color32::from_rgb(0, 128, 128));
    }

    #[test]
    fn test_trainingpeaks_notification_titles() {
        let started = SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::TrainingPeaks,
        };
        assert!(started.title().contains("TrainingPeaks"));

        let completed = SyncNotificationType::Completed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::TrainingPeaks,
            external_url: Some("https://trainingpeaks.com/activity/123".to_string()),
        };
        assert!(completed.title().contains("TrainingPeaks"));

        let failed = SyncNotificationType::Failed {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::TrainingPeaks,
            error: "API error".to_string(),
            will_retry: true,
        };
        assert!(failed.title().contains("TrainingPeaks"));
    }

    #[test]
    fn test_trainingpeaks_upload_notifications() {
        let mut queue = SyncNotificationQueue::new();
        let ride_id = Uuid::new_v4();

        // TrainingPeaks upload started
        queue.push(SyncNotificationType::Started {
            ride_id,
            platform: SyncPlatform::TrainingPeaks,
        });
        assert_eq!(queue.pending_count(), 1);

        let current = queue.current().unwrap();
        assert!(current.notification_type.title().contains("TrainingPeaks"));
        assert_eq!(
            current.notification_type.color(),
            platform_color(SyncPlatform::TrainingPeaks)
        );
    }

    // Workout sync notification tests

    #[test]
    fn test_workout_sync_started_notification() {
        let notification = SyncNotificationType::WorkoutSyncStarted {
            platform: SyncPlatform::TrainingPeaks,
        };

        assert!(notification.title().contains("TrainingPeaks"));
        assert!(notification.title().contains("workouts"));
        assert!(notification.description().contains("Downloading"));
        assert_eq!(notification.icon(), "\u{2193}"); // Down arrow
        assert_eq!(
            notification.color(),
            platform_color(SyncPlatform::TrainingPeaks)
        );
    }

    #[test]
    fn test_workout_sync_completed_notification() {
        // Test with new workouts only
        let notification = SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 3,
            updated_workouts: 0,
        };
        assert!(notification.title().contains("TrainingPeaks"));
        assert!(notification.description().contains("3 new"));
        assert_eq!(notification.icon(), "\u{2713}"); // Checkmark

        // Test with updates only
        let notification_updates = SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 0,
            updated_workouts: 2,
        };
        assert!(notification_updates.description().contains("2 workout"));
        assert!(notification_updates.description().contains("updated"));

        // Test with both new and updated
        let notification_both = SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 5,
            updated_workouts: 3,
        };
        assert!(notification_both.description().contains("5 new"));
        assert!(notification_both.description().contains("3 updated"));

        // Test with no changes
        let notification_none = SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 0,
            updated_workouts: 0,
        };
        assert!(notification_none.description().contains("up to date"));
    }

    #[test]
    fn test_workout_sync_failed_notification() {
        let notification = SyncNotificationType::WorkoutSyncFailed {
            platform: SyncPlatform::TrainingPeaks,
            error: "Network timeout".to_string(),
        };

        assert!(notification.title().contains("TrainingPeaks"));
        assert!(notification.title().contains("failed"));
        assert!(notification.description().contains("Network timeout"));
        assert_eq!(notification.icon(), "\u{2717}"); // X mark
        assert_eq!(notification.color(), ERROR_RED);
    }

    #[test]
    fn test_workout_sync_notification_queue() {
        let mut queue = SyncNotificationQueue::new();

        // Push workout sync started
        queue.push(SyncNotificationType::WorkoutSyncStarted {
            platform: SyncPlatform::TrainingPeaks,
        });
        assert_eq!(queue.pending_count(), 1);

        // Push workout sync completed
        queue.push(SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 5,
            updated_workouts: 2,
        });
        assert_eq!(queue.pending_count(), 2);

        // Verify first notification is started
        let current = queue.current().unwrap();
        assert!(matches!(
            current.notification_type,
            SyncNotificationType::WorkoutSyncStarted { .. }
        ));

        // Dismiss and verify completed is next
        queue.dismiss_current();
        let current = queue.current().unwrap();
        assert!(matches!(
            current.notification_type,
            SyncNotificationType::WorkoutSyncCompleted { .. }
        ));
    }

    #[test]
    fn test_workout_sync_background_colors() {
        let started = SyncNotificationType::WorkoutSyncStarted {
            platform: SyncPlatform::TrainingPeaks,
        };
        let completed = SyncNotificationType::WorkoutSyncCompleted {
            platform: SyncPlatform::TrainingPeaks,
            new_workouts: 1,
            updated_workouts: 0,
        };
        let failed = SyncNotificationType::WorkoutSyncFailed {
            platform: SyncPlatform::TrainingPeaks,
            error: "Error".to_string(),
        };

        // Started should have neutral background
        assert_eq!(
            started.background_color(),
            Color32::from_rgba_unmultiplied(40, 40, 50, 240)
        );
        // Completed should have green-ish background
        assert_eq!(
            completed.background_color(),
            Color32::from_rgba_unmultiplied(30, 60, 40, 240)
        );
        // Failed should have red-ish background
        assert_eq!(
            failed.background_color(),
            Color32::from_rgba_unmultiplied(60, 30, 30, 240)
        );
    }

    #[test]
    fn test_multiple_platforms_in_queue() {
        let mut queue = SyncNotificationQueue::new();

        // Push Strava upload
        queue.push(SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::Strava,
        });

        // Push TrainingPeaks upload (different ride can have same ride_id for different platforms)
        queue.push(SyncNotificationType::Started {
            ride_id: Uuid::new_v4(),
            platform: SyncPlatform::TrainingPeaks,
        });

        // Push TrainingPeaks workout sync
        queue.push(SyncNotificationType::WorkoutSyncStarted {
            platform: SyncPlatform::TrainingPeaks,
        });

        assert_eq!(queue.pending_count(), 3);
    }
}
