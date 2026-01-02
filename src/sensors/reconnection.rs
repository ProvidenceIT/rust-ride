//! Reconnection module with exponential backoff for sensor connections.
//!
//! This module provides intelligent reconnection strategies that reduce
//! connection spam while ensuring reliable sensor connectivity.

use std::time::Duration;

/// Default initial delay for reconnection attempts (1 second).
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;

/// Default maximum delay between reconnection attempts (30 seconds).
pub const DEFAULT_MAX_DELAY_MS: u64 = 30000;

/// Default multiplier for exponential backoff (2x = double each attempt).
pub const DEFAULT_MULTIPLIER: f64 = 2.0;

/// Configuration for exponential backoff reconnection.
#[derive(Debug, Clone)]
pub struct ExponentialBackoffConfig {
    /// Initial delay before first reconnection attempt (default: 1s).
    pub initial_delay: Duration,
    /// Maximum delay between attempts (default: 30s).
    pub max_delay: Duration,
    /// Multiplier for each subsequent attempt (default: 2.0).
    pub multiplier: f64,
    /// Maximum number of attempts before giving up (0 = unlimited).
    pub max_attempts: u32,
    /// Optional jitter factor (0.0-1.0) to randomize delays.
    /// Helps prevent thundering herd problem when multiple sensors reconnect.
    pub jitter_factor: f64,
}

impl Default for ExponentialBackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(DEFAULT_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(DEFAULT_MAX_DELAY_MS),
            multiplier: DEFAULT_MULTIPLIER,
            max_attempts: 5,
            jitter_factor: 0.0,
        }
    }
}

impl ExponentialBackoffConfig {
    /// Create a configuration for aggressive reconnection (fast initial retry).
    pub fn aggressive() -> Self {
        Self {
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(15),
            multiplier: 2.0,
            max_attempts: 8,
            jitter_factor: 0.1,
        }
    }

    /// Create a configuration for conservative reconnection (slower, longer waits).
    pub fn conservative() -> Self {
        Self {
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_attempts: 3,
            jitter_factor: 0.2,
        }
    }

    /// Create a configuration with jitter for multiple sensors.
    pub fn with_jitter() -> Self {
        Self {
            jitter_factor: 0.25,
            ..Self::default()
        }
    }
}

/// State for tracking exponential backoff reconnection attempts.
///
/// Tracks the current attempt count and calculates the next delay
/// using exponential backoff: delay = initial * multiplier^attempt
/// capped at max_delay.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    /// Configuration for backoff behavior.
    config: ExponentialBackoffConfig,
    /// Current attempt number (0 = not started, 1 = first attempt).
    current_attempt: u32,
    /// The delay that will be used for the next attempt.
    next_delay: Duration,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff with default configuration.
    pub fn new() -> Self {
        Self::with_config(ExponentialBackoffConfig::default())
    }

    /// Create a new exponential backoff with custom configuration.
    pub fn with_config(config: ExponentialBackoffConfig) -> Self {
        let initial_delay = config.initial_delay;
        Self {
            config,
            current_attempt: 0,
            next_delay: initial_delay,
        }
    }

    /// Get the current attempt number.
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }

    /// Get the next delay duration.
    pub fn next_delay(&self) -> Duration {
        self.next_delay
    }

    /// Check if the maximum attempts have been exceeded.
    pub fn is_exhausted(&self) -> bool {
        self.config.max_attempts > 0 && self.current_attempt >= self.config.max_attempts
    }

    /// Get the remaining attempts (0 if unlimited or exhausted).
    pub fn remaining_attempts(&self) -> Option<u32> {
        if self.config.max_attempts == 0 {
            None // Unlimited
        } else {
            Some(self.config.max_attempts.saturating_sub(self.current_attempt))
        }
    }

    /// Record a reconnection attempt and advance to the next delay.
    ///
    /// Returns the delay duration to wait before this attempt.
    /// After calling this, `next_delay()` will return the delay for the
    /// subsequent attempt.
    pub fn record_attempt(&mut self) -> Duration {
        let delay = self.next_delay;
        self.current_attempt += 1;

        // Calculate next delay: current * multiplier, capped at max
        let next = Duration::from_secs_f64(
            delay.as_secs_f64() * self.config.multiplier
        );
        self.next_delay = next.min(self.config.max_delay);

        // Apply jitter if configured
        if self.config.jitter_factor > 0.0 {
            self.next_delay = self.apply_jitter(self.next_delay);
        }

        delay
    }

    /// Apply jitter to a delay duration.
    fn apply_jitter(&self, delay: Duration) -> Duration {
        // Use a simple deterministic jitter based on attempt count
        // In production, you might want to use rand crate for true randomness
        let jitter_range = delay.as_secs_f64() * self.config.jitter_factor;
        let jitter = jitter_range * (((self.current_attempt as f64).sin() + 1.0) / 2.0);
        Duration::from_secs_f64(delay.as_secs_f64() + jitter - jitter_range / 2.0)
    }

    /// Reset the backoff state after a successful connection.
    ///
    /// This should be called when a connection succeeds to reset
    /// the attempt counter and delay back to initial values.
    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.next_delay = self.config.initial_delay;
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ExponentialBackoffConfig {
        &self.config
    }

    /// Calculate the delay for a specific attempt number without modifying state.
    ///
    /// Useful for displaying what delays would be used for future attempts.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let mut delay = self.config.initial_delay;
        for _ in 1..attempt {
            delay = Duration::from_secs_f64(
                delay.as_secs_f64() * self.config.multiplier
            );
            delay = delay.min(self.config.max_delay);
        }
        delay
    }

    /// Get all delays that would be used for the configured max attempts.
    ///
    /// Returns a vector of delays from attempt 1 to max_attempts.
    pub fn all_delays(&self) -> Vec<Duration> {
        let count = if self.config.max_attempts == 0 {
            10 // Show first 10 for unlimited
        } else {
            self.config.max_attempts as usize
        };

        (1..=count as u32)
            .map(|attempt| self.delay_for_attempt(attempt))
            .collect()
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

/// State for tracking reconnection across multiple sensors.
///
/// Maintains per-device backoff state and provides methods for
/// managing reconnection attempts across all sensors.
#[derive(Debug, Default)]
pub struct ReconnectionManager {
    /// Per-device backoff state.
    backoffs: std::collections::HashMap<String, ExponentialBackoff>,
    /// Default configuration for new sensors.
    default_config: ExponentialBackoffConfig,
}

impl ReconnectionManager {
    /// Create a new reconnection manager with default configuration.
    pub fn new() -> Self {
        Self {
            backoffs: std::collections::HashMap::new(),
            default_config: ExponentialBackoffConfig::default(),
        }
    }

    /// Create a new reconnection manager with custom default configuration.
    pub fn with_config(config: ExponentialBackoffConfig) -> Self {
        Self {
            backoffs: std::collections::HashMap::new(),
            default_config: config,
        }
    }

    /// Get or create a backoff state for a device.
    pub fn get_or_create(&mut self, device_id: &str) -> &mut ExponentialBackoff {
        self.backoffs
            .entry(device_id.to_string())
            .or_insert_with(|| ExponentialBackoff::with_config(self.default_config.clone()))
    }

    /// Get the backoff state for a device if it exists.
    pub fn get(&self, device_id: &str) -> Option<&ExponentialBackoff> {
        self.backoffs.get(device_id)
    }

    /// Record a reconnection attempt for a device and get the delay.
    pub fn record_attempt(&mut self, device_id: &str) -> Duration {
        self.get_or_create(device_id).record_attempt()
    }

    /// Reset the backoff state for a device after successful connection.
    pub fn reset(&mut self, device_id: &str) {
        if let Some(backoff) = self.backoffs.get_mut(device_id) {
            backoff.reset();
        }
    }

    /// Check if a device has exhausted its reconnection attempts.
    pub fn is_exhausted(&self, device_id: &str) -> bool {
        self.backoffs
            .get(device_id)
            .map_or(false, |b| b.is_exhausted())
    }

    /// Remove the backoff state for a device.
    pub fn remove(&mut self, device_id: &str) {
        self.backoffs.remove(device_id);
    }

    /// Clear all backoff states.
    pub fn clear(&mut self) {
        self.backoffs.clear();
    }

    /// Get the number of devices being tracked.
    pub fn len(&self) -> usize {
        self.backoffs.len()
    }

    /// Check if any devices are being tracked.
    pub fn is_empty(&self) -> bool {
        self.backoffs.is_empty()
    }

    /// Get statistics for a device's reconnection attempts.
    pub fn get_stats(&self, device_id: &str) -> Option<ReconnectionStats> {
        self.backoffs.get(device_id).map(|b| ReconnectionStats {
            device_id: device_id.to_string(),
            current_attempt: b.current_attempt(),
            next_delay: b.next_delay(),
            is_exhausted: b.is_exhausted(),
            remaining_attempts: b.remaining_attempts(),
        })
    }
}

/// Statistics for a device's reconnection attempts.
#[derive(Debug, Clone)]
pub struct ReconnectionStats {
    /// Device identifier.
    pub device_id: String,
    /// Current attempt number.
    pub current_attempt: u32,
    /// Next delay before retry.
    pub next_delay: Duration,
    /// Whether max attempts have been exceeded.
    pub is_exhausted: bool,
    /// Remaining attempts (None if unlimited).
    pub remaining_attempts: Option<u32>,
}

impl ReconnectionStats {
    /// Format the status as a human-readable string.
    pub fn status_text(&self) -> String {
        if self.is_exhausted {
            format!("Gave up after {} attempts", self.current_attempt)
        } else {
            match self.remaining_attempts {
                Some(remaining) => format!(
                    "Attempt {} ({}s wait, {} remaining)",
                    self.current_attempt,
                    self.next_delay.as_secs(),
                    remaining
                ),
                None => format!(
                    "Attempt {} ({}s wait)",
                    self.current_attempt,
                    self.next_delay.as_secs()
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_backoff_sequence() {
        let backoff = ExponentialBackoff::new();

        // Verify default config delays: 1s, 2s, 4s, 8s, 16s, 30s (capped)
        assert_eq!(backoff.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(backoff.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(backoff.delay_for_attempt(3), Duration::from_secs(4));
        assert_eq!(backoff.delay_for_attempt(4), Duration::from_secs(8));
        assert_eq!(backoff.delay_for_attempt(5), Duration::from_secs(16));
        // 32s would exceed max of 30s, so should be capped
        assert_eq!(backoff.delay_for_attempt(6), Duration::from_secs(30));
        // Further attempts stay at 30s
        assert_eq!(backoff.delay_for_attempt(7), Duration::from_secs(30));
    }

    #[test]
    fn test_record_attempt_advances_delay() {
        let mut backoff = ExponentialBackoff::new();

        assert_eq!(backoff.current_attempt(), 0);

        // First attempt should return initial delay (1s)
        let delay1 = backoff.record_attempt();
        assert_eq!(delay1, Duration::from_secs(1));
        assert_eq!(backoff.current_attempt(), 1);

        // Next delay should be doubled (2s)
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));

        // Second attempt should return 2s
        let delay2 = backoff.record_attempt();
        assert_eq!(delay2, Duration::from_secs(2));
        assert_eq!(backoff.current_attempt(), 2);

        // Next delay should be 4s
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut backoff = ExponentialBackoff::new();

        // Make a few attempts
        backoff.record_attempt();
        backoff.record_attempt();
        backoff.record_attempt();

        assert_eq!(backoff.current_attempt(), 3);
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));

        // Reset should return to initial state
        backoff.reset();

        assert_eq!(backoff.current_attempt(), 0);
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    }

    #[test]
    fn test_max_delay_cap() {
        let config = ExponentialBackoffConfig {
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            max_attempts: 10,
            jitter_factor: 0.0,
        };
        let mut backoff = ExponentialBackoff::with_config(config);

        // 10s -> 20s -> 40s (capped to 30s)
        backoff.record_attempt(); // returns 10s, next is 20s
        backoff.record_attempt(); // returns 20s, next is 40s capped to 30s
        backoff.record_attempt(); // returns 30s, next is still 30s

        assert_eq!(backoff.next_delay(), Duration::from_secs(30));
    }

    #[test]
    fn test_exhaustion() {
        let config = ExponentialBackoffConfig {
            max_attempts: 3,
            ..ExponentialBackoffConfig::default()
        };
        let mut backoff = ExponentialBackoff::with_config(config);

        assert!(!backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), Some(3));

        backoff.record_attempt();
        assert!(!backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), Some(2));

        backoff.record_attempt();
        assert!(!backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), Some(1));

        backoff.record_attempt();
        assert!(backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), Some(0));
    }

    #[test]
    fn test_unlimited_attempts() {
        let config = ExponentialBackoffConfig {
            max_attempts: 0, // 0 = unlimited
            ..ExponentialBackoffConfig::default()
        };
        let mut backoff = ExponentialBackoff::with_config(config);

        assert!(!backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), None);

        // Even after many attempts, should not be exhausted
        for _ in 0..100 {
            backoff.record_attempt();
        }
        assert!(!backoff.is_exhausted());
        assert_eq!(backoff.remaining_attempts(), None);
    }

    #[test]
    fn test_reconnection_manager() {
        let mut manager = ReconnectionManager::new();

        assert!(manager.is_empty());

        // Record attempt for device A
        let delay = manager.record_attempt("device_a");
        assert_eq!(delay, Duration::from_secs(1));
        assert_eq!(manager.len(), 1);

        // Record attempt for device B
        manager.record_attempt("device_b");
        assert_eq!(manager.len(), 2);

        // Reset device A
        manager.reset("device_a");
        let stats = manager.get_stats("device_a").unwrap();
        assert_eq!(stats.current_attempt, 0);

        // Device B should be unaffected
        let stats_b = manager.get_stats("device_b").unwrap();
        assert_eq!(stats_b.current_attempt, 1);

        // Remove device A
        manager.remove("device_a");
        assert_eq!(manager.len(), 1);
        assert!(manager.get("device_a").is_none());
    }

    #[test]
    fn test_all_delays() {
        let config = ExponentialBackoffConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            max_attempts: 6,
            jitter_factor: 0.0,
        };
        let backoff = ExponentialBackoff::with_config(config);

        let delays = backoff.all_delays();
        assert_eq!(delays.len(), 6);
        assert_eq!(delays[0], Duration::from_secs(1));
        assert_eq!(delays[1], Duration::from_secs(2));
        assert_eq!(delays[2], Duration::from_secs(4));
        assert_eq!(delays[3], Duration::from_secs(8));
        assert_eq!(delays[4], Duration::from_secs(16));
        assert_eq!(delays[5], Duration::from_secs(30)); // capped
    }

    #[test]
    fn test_config_presets() {
        let aggressive = ExponentialBackoffConfig::aggressive();
        assert_eq!(aggressive.initial_delay, Duration::from_millis(500));
        assert_eq!(aggressive.max_attempts, 8);

        let conservative = ExponentialBackoffConfig::conservative();
        assert_eq!(conservative.initial_delay, Duration::from_secs(2));
        assert_eq!(conservative.max_attempts, 3);

        let with_jitter = ExponentialBackoffConfig::with_jitter();
        assert!(with_jitter.jitter_factor > 0.0);
    }
}
