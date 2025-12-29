//! Achievement notification queue system.
//!
//! T034: Create AchievementNotification queue system.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{AchievementCategory, AchievementTier};

/// A notification for an earned achievement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementNotification {
    /// Unique notification ID.
    pub id: Uuid,
    /// Achievement ID.
    pub achievement_id: Uuid,
    /// Achievement title.
    pub title: String,
    /// Achievement description.
    pub description: String,
    /// Category.
    pub category: AchievementCategory,
    /// Tier.
    pub tier: AchievementTier,
    /// XP awarded.
    pub xp_awarded: u32,
    /// When earned.
    pub earned_at: DateTime<Utc>,
    /// Whether notification has been displayed.
    #[serde(skip)]
    pub displayed: bool,
    /// Whether notification has been dismissed.
    #[serde(skip)]
    pub dismissed: bool,
    /// Display timestamp for animation timing.
    #[serde(skip)]
    pub display_start: Option<Instant>,
}

impl AchievementNotification {
    /// Create a new notification.
    pub fn new(
        achievement_id: Uuid,
        title: impl Into<String>,
        description: impl Into<String>,
        category: AchievementCategory,
        tier: AchievementTier,
        xp_awarded: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            achievement_id,
            title: title.into(),
            description: description.into(),
            category,
            tier,
            xp_awarded,
            earned_at: Utc::now(),
            displayed: false,
            dismissed: false,
            display_start: None,
        }
    }

    /// Mark as displayed and start timer.
    pub fn mark_displayed(&mut self) {
        if !self.displayed {
            self.displayed = true;
            self.display_start = Some(Instant::now());
        }
    }

    /// Mark as dismissed.
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Check if display duration has elapsed.
    pub fn has_expired(&self, display_duration: Duration) -> bool {
        self.display_start
            .map(|start| start.elapsed() >= display_duration)
            .unwrap_or(false)
    }

    /// Get time remaining for display.
    pub fn time_remaining(&self, display_duration: Duration) -> Duration {
        self.display_start
            .map(|start| display_duration.saturating_sub(start.elapsed()))
            .unwrap_or(display_duration)
    }
}

/// Queue for managing achievement notifications.
#[derive(Debug, Default)]
pub struct NotificationQueue {
    /// Pending notifications.
    queue: VecDeque<AchievementNotification>,
    /// Currently displayed notification.
    current: Option<AchievementNotification>,
    /// Display duration per notification.
    display_duration: Duration,
    /// Maximum queue size.
    max_queue_size: usize,
}

impl NotificationQueue {
    /// Create a new notification queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            display_duration: Duration::from_secs(5),
            max_queue_size: 10,
        }
    }

    /// Create with custom settings.
    pub fn with_settings(display_secs: u64, max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            display_duration: Duration::from_secs(display_secs),
            max_queue_size: max_size,
        }
    }

    /// Add a notification to the queue.
    pub fn push(&mut self, notification: AchievementNotification) {
        // Limit queue size
        while self.queue.len() >= self.max_queue_size {
            self.queue.pop_front();
        }
        self.queue.push_back(notification);
    }

    /// Get the current notification (if any).
    pub fn current(&self) -> Option<&AchievementNotification> {
        self.current.as_ref()
    }

    /// Get mutable reference to current notification.
    pub fn current_mut(&mut self) -> Option<&mut AchievementNotification> {
        self.current.as_mut()
    }

    /// Update the queue (call each frame).
    ///
    /// Returns true if the current notification changed.
    pub fn update(&mut self) -> bool {
        // Check if current notification should be dismissed
        if let Some(ref notification) = self.current {
            if notification.dismissed || notification.has_expired(self.display_duration) {
                self.current = None;
            }
        }

        // If no current notification, try to get next from queue
        if self.current.is_none() {
            if let Some(mut next) = self.queue.pop_front() {
                next.mark_displayed();
                self.current = Some(next);
                return true;
            }
        }

        false
    }

    /// Dismiss the current notification.
    pub fn dismiss_current(&mut self) {
        if let Some(ref mut notification) = self.current {
            notification.dismiss();
        }
    }

    /// Check if there are pending notifications.
    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty() || self.current.is_some()
    }

    /// Get number of pending notifications.
    pub fn pending_count(&self) -> usize {
        self.queue.len() + if self.current.is_some() { 1 } else { 0 }
    }

    /// Clear all notifications.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.current = None;
    }

    /// Get progress of current notification display (0.0 to 1.0).
    pub fn display_progress(&self) -> f32 {
        self.current
            .as_ref()
            .and_then(|n| n.display_start)
            .map(|start| {
                (start.elapsed().as_secs_f32() / self.display_duration.as_secs_f32())
                    .clamp(0.0, 1.0)
            })
            .unwrap_or(0.0)
    }
}

/// Level up notification (separate from achievements).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpNotification {
    /// Previous level.
    pub from_level: u32,
    /// New level.
    pub to_level: u32,
    /// Total XP.
    pub total_xp: u64,
    /// Any rewards unlocked.
    pub rewards: Vec<String>,
    /// When occurred.
    pub occurred_at: DateTime<Utc>,
}

impl LevelUpNotification {
    /// Create a new level up notification.
    pub fn new(from_level: u32, to_level: u32, total_xp: u64) -> Self {
        Self {
            from_level,
            to_level,
            total_xp,
            rewards: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Add a reward to the notification.
    pub fn with_reward(mut self, reward: impl Into<String>) -> Self {
        self.rewards.push(reward.into());
        self
    }

    /// Add multiple rewards.
    pub fn with_rewards(mut self, rewards: impl IntoIterator<Item = String>) -> Self {
        self.rewards.extend(rewards);
        self
    }

    /// Check if multiple levels were gained.
    pub fn levels_gained(&self) -> u32 {
        self.to_level.saturating_sub(self.from_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let achievement_id = Uuid::new_v4();
        let notification = AchievementNotification::new(
            achievement_id,
            "Test Title",
            "Test Description",
            AchievementCategory::Training,
            AchievementTier::Bronze,
            100,
        );

        assert_eq!(notification.achievement_id, achievement_id);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.xp_awarded, 100);
        assert!(!notification.displayed);
    }

    #[test]
    fn test_notification_queue() {
        let mut queue = NotificationQueue::with_settings(1, 5);

        // Add notification
        let notification = AchievementNotification::new(
            Uuid::new_v4(),
            "Test",
            "Test",
            AchievementCategory::Training,
            AchievementTier::Bronze,
            100,
        );

        queue.push(notification);
        assert_eq!(queue.pending_count(), 1);

        // Update should move to current
        let changed = queue.update();
        assert!(changed);
        assert!(queue.current().is_some());
    }

    #[test]
    fn test_queue_max_size() {
        let mut queue = NotificationQueue::with_settings(5, 3);

        // Add more than max
        for i in 0..5 {
            let notification = AchievementNotification::new(
                Uuid::new_v4(),
                format!("Test {}", i),
                "Test",
                AchievementCategory::Training,
                AchievementTier::Bronze,
                100,
            );
            queue.push(notification);
        }

        // Should only have max_size
        assert_eq!(queue.queue.len(), 3);
    }

    #[test]
    fn test_level_up_notification() {
        let notification = LevelUpNotification::new(5, 7, 10000)
            .with_reward("New Jersey: Fire")
            .with_reward("New Frame: Carbon");

        assert_eq!(notification.levels_gained(), 2);
        assert_eq!(notification.rewards.len(), 2);
    }
}
