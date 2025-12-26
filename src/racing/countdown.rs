//! Countdown synchronization for race starts.
//!
//! Handles synchronized race starts across LAN peers.

use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use tokio::sync::broadcast;

use super::events::constants;

/// Countdown event.
#[derive(Debug, Clone)]
pub enum CountdownEvent {
    /// Countdown started.
    CountdownStarted { seconds: u8 },
    /// Countdown tick.
    CountdownTick { seconds: u8 },
    /// Race started.
    RaceStart,
    /// Sync error occurred.
    SyncError { message: String },
}

/// Countdown synchronizer for race starts.
pub struct CountdownSync {
    /// Target start time.
    start_time: DateTime<Utc>,
    /// Clock offset with peers in milliseconds.
    clock_offset_ms: AtomicI64,
    /// Whether the race has started.
    started: AtomicBool,
    /// Event sender.
    event_tx: broadcast::Sender<CountdownEvent>,
}

impl CountdownSync {
    /// Create a new countdown synchronizer.
    pub fn new() -> (Self, broadcast::Receiver<CountdownEvent>) {
        let (tx, rx) = broadcast::channel(16);

        (
            Self {
                start_time: Utc::now(),
                clock_offset_ms: AtomicI64::new(0),
                started: AtomicBool::new(false),
                event_tx: tx,
            },
            rx,
        )
    }

    /// Start countdown synchronization.
    pub fn start_countdown(&mut self, start_time: DateTime<Utc>) -> Result<(), CountdownError> {
        let now = Utc::now();

        // Validate start time is in the future
        if start_time <= now {
            return Err(CountdownError::InvalidStartTime);
        }

        // Validate countdown is not too long
        let seconds_until = (start_time - now).num_seconds();
        if seconds_until > constants::COUNTDOWN_SECONDS as i64 * 2 {
            return Err(CountdownError::InvalidStartTime);
        }

        self.start_time = start_time;
        self.started.store(false, Ordering::SeqCst);

        let _ = self.event_tx.send(CountdownEvent::CountdownStarted {
            seconds: seconds_until.min(255) as u8,
        });

        Ok(())
    }

    /// Get synchronized countdown value.
    pub fn current_countdown(&self) -> Option<u8> {
        if self.started.load(Ordering::SeqCst) {
            return None;
        }

        let now = Utc::now();
        let offset = Duration::milliseconds(self.clock_offset_ms.load(Ordering::SeqCst));
        let adjusted_now = now + offset;

        let remaining = (self.start_time - adjusted_now).num_seconds();

        if remaining <= 0 {
            None
        } else if remaining > 255 {
            Some(255)
        } else {
            Some(remaining as u8)
        }
    }

    /// Check if race should start.
    pub fn should_start(&self) -> bool {
        if self.started.load(Ordering::SeqCst) {
            return false;
        }

        let now = Utc::now();
        let offset = Duration::milliseconds(self.clock_offset_ms.load(Ordering::SeqCst));
        let adjusted_now = now + offset;

        adjusted_now >= self.start_time
    }

    /// Mark race as started.
    pub fn mark_started(&self) {
        self.started.store(true, Ordering::SeqCst);
        let _ = self.event_tx.send(CountdownEvent::RaceStart);
    }

    /// Get clock offset with peers.
    pub fn clock_offset_ms(&self) -> i64 {
        self.clock_offset_ms.load(Ordering::SeqCst)
    }

    /// Update clock offset based on peer time sync.
    pub fn update_clock_offset(&self, offset_ms: i64) {
        // Only accept reasonable offsets
        if offset_ms.abs() <= constants::MAX_CLOCK_SYNC_ERROR * 10 {
            self.clock_offset_ms.store(offset_ms, Ordering::SeqCst);
        }
    }

    /// Subscribe to countdown events.
    pub fn subscribe(&self) -> broadcast::Receiver<CountdownEvent> {
        self.event_tx.subscribe()
    }

    /// Tick the countdown (call every second).
    pub fn tick(&self) {
        if let Some(seconds) = self.current_countdown() {
            let _ = self.event_tx.send(CountdownEvent::CountdownTick { seconds });
        }

        if self.should_start() && !self.started.load(Ordering::SeqCst) {
            self.mark_started();
        }
    }
}

impl Default for CountdownSync {
    fn default() -> Self {
        let (sync, _) = Self::new();
        sync
    }
}

/// Countdown errors.
#[derive(Debug, thiserror::Error)]
pub enum CountdownError {
    #[error("Invalid start time")]
    InvalidStartTime,

    #[error("Clock sync failed: {0}")]
    ClockSyncFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_countdown_basic() {
        let (mut sync, _rx) = CountdownSync::new();

        let start_time = Utc::now() + Duration::seconds(10);
        sync.start_countdown(start_time).unwrap();

        let countdown = sync.current_countdown();
        assert!(countdown.is_some());
        assert!(countdown.unwrap() <= 10);
        assert!(!sync.should_start());
    }

    #[test]
    fn test_countdown_expired() {
        let (mut sync, _rx) = CountdownSync::new();

        let start_time = Utc::now() - Duration::seconds(1);
        let result = sync.start_countdown(start_time);

        assert!(result.is_err());
    }
}
