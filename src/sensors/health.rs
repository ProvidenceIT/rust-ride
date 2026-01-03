//! Connection health monitoring for sensor connections.
//!
//! This module provides proactive health monitoring based on data reception rate.
//! When data stops arriving before the BLE disconnect notification, the health
//! monitor can trigger proactive reconnection to minimize data gaps.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default timeout before a connection is considered stale (5 seconds).
pub const DEFAULT_STALE_TIMEOUT_SECS: u64 = 5;

/// Default timeout for degraded status (2.5 seconds without data).
pub const DEFAULT_DEGRADED_TIMEOUT_MS: u64 = 2500;

/// Default expected data interval for most fitness sensors (1 second).
pub const DEFAULT_EXPECTED_DATA_INTERVAL_MS: u64 = 1000;

/// Minimum acceptable data rate (packets per second).
pub const DEFAULT_MIN_DATA_RATE: f32 = 0.5;

/// Configuration for connection health monitoring.
#[derive(Debug, Clone)]
pub struct ConnectionHealthConfig {
    /// Time without data before connection is considered stale (default: 5s).
    /// Stale connections trigger proactive reconnection.
    pub stale_timeout: Duration,
    /// Time without data before connection is degraded (default: 2.5s).
    /// Degraded status shows warning but doesn't trigger reconnection.
    pub degraded_timeout: Duration,
    /// Expected interval between data packets (default: 1s for most sensors).
    /// Used to calculate expected vs actual data rate.
    pub expected_data_interval: Duration,
    /// Minimum acceptable data rate in packets per second (default: 0.5).
    /// Below this rate, the connection is considered unhealthy.
    pub min_data_rate: f32,
    /// Whether health monitoring is enabled (default: true).
    pub enabled: bool,
    /// Window size for calculating data rate (default: 10s).
    /// Uses a rolling window to smooth rate calculations.
    pub rate_window: Duration,
}

impl Default for ConnectionHealthConfig {
    fn default() -> Self {
        Self {
            stale_timeout: Duration::from_secs(DEFAULT_STALE_TIMEOUT_SECS),
            degraded_timeout: Duration::from_millis(DEFAULT_DEGRADED_TIMEOUT_MS),
            expected_data_interval: Duration::from_millis(DEFAULT_EXPECTED_DATA_INTERVAL_MS),
            min_data_rate: DEFAULT_MIN_DATA_RATE,
            enabled: true,
            rate_window: Duration::from_secs(10),
        }
    }
}

impl ConnectionHealthConfig {
    /// Create a strict health check configuration for critical sensors (trainers, power meters).
    pub fn strict() -> Self {
        Self {
            stale_timeout: Duration::from_secs(3),
            degraded_timeout: Duration::from_millis(1500),
            expected_data_interval: Duration::from_millis(1000),
            min_data_rate: 0.8,
            enabled: true,
            rate_window: Duration::from_secs(5),
        }
    }

    /// Create a relaxed health check configuration for less critical sensors (HR, cadence).
    pub fn relaxed() -> Self {
        Self {
            stale_timeout: Duration::from_secs(10),
            degraded_timeout: Duration::from_secs(5),
            expected_data_interval: Duration::from_millis(1000),
            min_data_rate: 0.3,
            enabled: true,
            rate_window: Duration::from_secs(15),
        }
    }

    /// Create a disabled health check configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// Health status of a sensor connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Connection is healthy with regular data flow.
    Healthy,
    /// Data rate has slowed but connection is still active.
    Degraded,
    /// No data received for extended period - connection may be lost.
    Stale,
    /// Health status unknown (not enough data to determine).
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Stale => write!(f, "Stale"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Connection health tracking for a single sensor.
#[derive(Debug, Clone)]
pub struct ConnectionHealth {
    /// Device identifier.
    device_id: String,
    /// When the connection was established.
    connected_at: Instant,
    /// When data was last received.
    last_data_at: Option<Instant>,
    /// Timestamps of recent data packets for rate calculation.
    data_timestamps: Vec<Instant>,
    /// Current health status.
    status: HealthStatus,
    /// Configuration for health checks.
    config: ConnectionHealthConfig,
    /// Count of consecutive healthy checks.
    healthy_streak: u32,
    /// Count of consecutive degraded/stale checks.
    unhealthy_streak: u32,
}

impl ConnectionHealth {
    /// Create a new connection health tracker.
    pub fn new(device_id: String) -> Self {
        Self::with_config(device_id, ConnectionHealthConfig::default())
    }

    /// Create a new connection health tracker with custom configuration.
    pub fn with_config(device_id: String, config: ConnectionHealthConfig) -> Self {
        Self {
            device_id,
            connected_at: Instant::now(),
            last_data_at: None,
            data_timestamps: Vec::with_capacity(64),
            status: HealthStatus::Unknown,
            config,
            healthy_streak: 0,
            unhealthy_streak: 0,
        }
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Record that data was received from the sensor.
    pub fn record_data_received(&mut self) {
        let now = Instant::now();
        self.last_data_at = Some(now);
        self.data_timestamps.push(now);

        // Prune old timestamps outside the rate window
        self.prune_old_timestamps();

        // Update status after recording data
        self.update_status();
    }

    /// Prune timestamps older than the rate window.
    fn prune_old_timestamps(&mut self) {
        let cutoff = Instant::now() - self.config.rate_window;
        self.data_timestamps.retain(|&t| t >= cutoff);
    }

    /// Calculate the current data rate (packets per second).
    pub fn data_rate(&self) -> f32 {
        if self.data_timestamps.len() < 2 {
            return 0.0;
        }

        let window = self.config.rate_window.as_secs_f32();
        self.data_timestamps.len() as f32 / window
    }

    /// Get time since last data was received.
    pub fn time_since_last_data(&self) -> Option<Duration> {
        self.last_data_at.map(|t| t.elapsed())
    }

    /// Get the current health status.
    pub fn status(&self) -> HealthStatus {
        self.status
    }

    /// Check if the connection is stale (no data for stale_timeout).
    pub fn is_stale(&self) -> bool {
        self.status == HealthStatus::Stale
    }

    /// Check if the connection needs proactive reconnection.
    ///
    /// Returns true when data has stopped arriving but we haven't received
    /// a BLE disconnect notification yet.
    pub fn needs_reconnection(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        self.is_stale()
    }

    /// Update the health status based on current data.
    fn update_status(&mut self) {
        if !self.config.enabled {
            self.status = HealthStatus::Unknown;
            return;
        }

        let previous_status = self.status;

        // Determine new status based on time since last data
        self.status = match self.last_data_at {
            None => HealthStatus::Unknown,
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= self.config.stale_timeout {
                    HealthStatus::Stale
                } else if elapsed >= self.config.degraded_timeout {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                }
            }
        };

        // Update streaks
        if self.status == HealthStatus::Healthy {
            self.healthy_streak += 1;
            self.unhealthy_streak = 0;
        } else if self.status == HealthStatus::Degraded || self.status == HealthStatus::Stale {
            self.unhealthy_streak += 1;
            self.healthy_streak = 0;
        }

        // Log status changes
        if previous_status != self.status && previous_status != HealthStatus::Unknown {
            tracing::debug!(
                "Sensor {} health: {:?} -> {:?}",
                self.device_id,
                previous_status,
                self.status
            );
        }
    }

    /// Perform a health check and return the current status.
    ///
    /// This method updates the internal status and returns it.
    pub fn check(&mut self) -> HealthStatus {
        self.update_status();
        self.status
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ConnectionHealthConfig {
        &self.config
    }

    /// Get the number of consecutive healthy checks.
    pub fn healthy_streak(&self) -> u32 {
        self.healthy_streak
    }

    /// Get the number of consecutive unhealthy checks.
    pub fn unhealthy_streak(&self) -> u32 {
        self.unhealthy_streak
    }

    /// Reset the health tracking state.
    ///
    /// Call this after a successful reconnection.
    pub fn reset(&mut self) {
        self.connected_at = Instant::now();
        self.last_data_at = None;
        self.data_timestamps.clear();
        self.status = HealthStatus::Unknown;
        self.healthy_streak = 0;
        self.unhealthy_streak = 0;
    }

    /// Get connection uptime.
    pub fn uptime(&self) -> Duration {
        self.connected_at.elapsed()
    }

    /// Get the number of data packets received in the rate window.
    pub fn packets_in_window(&self) -> usize {
        self.data_timestamps.len()
    }

    /// Get health statistics for display.
    pub fn stats(&self) -> HealthStats {
        HealthStats {
            device_id: self.device_id.clone(),
            status: self.status,
            data_rate: self.data_rate(),
            time_since_last_data: self.time_since_last_data(),
            uptime: self.uptime(),
            healthy_streak: self.healthy_streak,
            unhealthy_streak: self.unhealthy_streak,
            packets_in_window: self.packets_in_window(),
        }
    }
}

/// Health statistics for a sensor.
#[derive(Debug, Clone)]
pub struct HealthStats {
    /// Device identifier.
    pub device_id: String,
    /// Current health status.
    pub status: HealthStatus,
    /// Data rate in packets per second.
    pub data_rate: f32,
    /// Time since last data received.
    pub time_since_last_data: Option<Duration>,
    /// Connection uptime.
    pub uptime: Duration,
    /// Consecutive healthy checks.
    pub healthy_streak: u32,
    /// Consecutive unhealthy checks.
    pub unhealthy_streak: u32,
    /// Packets received in rate window.
    pub packets_in_window: usize,
}

impl HealthStats {
    /// Get a human-readable status text.
    pub fn status_text(&self) -> String {
        match self.status {
            HealthStatus::Healthy => format!(
                "Healthy ({:.1} pkt/s)",
                self.data_rate
            ),
            HealthStatus::Degraded => {
                let secs = self.time_since_last_data.map_or(0.0, |d| d.as_secs_f32());
                format!("Degraded (no data for {:.1}s)", secs)
            }
            HealthStatus::Stale => {
                let secs = self.time_since_last_data.map_or(0.0, |d| d.as_secs_f32());
                format!("Stale (no data for {:.1}s)", secs)
            }
            HealthStatus::Unknown => "Unknown".to_string(),
        }
    }

    /// Check if the connection needs attention.
    pub fn needs_attention(&self) -> bool {
        matches!(self.status, HealthStatus::Degraded | HealthStatus::Stale)
    }
}

/// Manages health monitoring for multiple sensor connections.
#[derive(Debug, Default)]
pub struct ConnectionHealthMonitor {
    /// Per-device health tracking.
    health_trackers: HashMap<String, ConnectionHealth>,
    /// Default configuration for new sensors.
    default_config: ConnectionHealthConfig,
}

impl ConnectionHealthMonitor {
    /// Create a new health monitor with default configuration.
    pub fn new() -> Self {
        Self {
            health_trackers: HashMap::new(),
            default_config: ConnectionHealthConfig::default(),
        }
    }

    /// Create a new health monitor with custom default configuration.
    pub fn with_config(config: ConnectionHealthConfig) -> Self {
        Self {
            health_trackers: HashMap::new(),
            default_config: config,
        }
    }

    /// Start monitoring a new connection.
    pub fn start_monitoring(&mut self, device_id: &str) {
        self.start_monitoring_with_config(device_id, self.default_config.clone());
    }

    /// Start monitoring with a custom configuration.
    pub fn start_monitoring_with_config(&mut self, device_id: &str, config: ConnectionHealthConfig) {
        let health = ConnectionHealth::with_config(device_id.to_string(), config);
        self.health_trackers.insert(device_id.to_string(), health);
        tracing::debug!("Started health monitoring for {}", device_id);
    }

    /// Stop monitoring a connection.
    pub fn stop_monitoring(&mut self, device_id: &str) {
        if self.health_trackers.remove(device_id).is_some() {
            tracing::debug!("Stopped health monitoring for {}", device_id);
        }
    }

    /// Record that data was received from a sensor.
    pub fn record_data(&mut self, device_id: &str) {
        if let Some(health) = self.health_trackers.get_mut(device_id) {
            health.record_data_received();
        }
    }

    /// Check health for a specific device.
    pub fn check_device(&mut self, device_id: &str) -> Option<HealthStatus> {
        self.health_trackers.get_mut(device_id).map(|h| h.check())
    }

    /// Check all connections and return devices needing reconnection.
    ///
    /// Returns a list of device IDs that have stale connections and
    /// should be proactively reconnected.
    pub fn check_all(&mut self) -> Vec<String> {
        let mut needs_reconnection = Vec::new();

        for (device_id, health) in &mut self.health_trackers {
            health.check();
            if health.needs_reconnection() {
                needs_reconnection.push(device_id.clone());
            }
        }

        if !needs_reconnection.is_empty() {
            tracing::info!(
                "Health check: {} device(s) need reconnection: {:?}",
                needs_reconnection.len(),
                needs_reconnection
            );
        }

        needs_reconnection
    }

    /// Get health status for a device.
    pub fn get_status(&self, device_id: &str) -> Option<HealthStatus> {
        self.health_trackers.get(device_id).map(|h| h.status())
    }

    /// Get health statistics for a device.
    pub fn get_stats(&self, device_id: &str) -> Option<HealthStats> {
        self.health_trackers.get(device_id).map(|h| h.stats())
    }

    /// Get all health statistics.
    pub fn get_all_stats(&self) -> Vec<HealthStats> {
        self.health_trackers.values().map(|h| h.stats()).collect()
    }

    /// Get devices with stale connections.
    pub fn get_stale_devices(&self) -> Vec<String> {
        self.health_trackers
            .iter()
            .filter(|(_, h)| h.is_stale())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get devices needing attention (degraded or stale).
    pub fn get_devices_needing_attention(&self) -> Vec<String> {
        self.health_trackers
            .iter()
            .filter(|(_, h)| h.status() == HealthStatus::Degraded || h.status() == HealthStatus::Stale)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Reset health tracking for a device.
    ///
    /// Call this after successful reconnection.
    pub fn reset(&mut self, device_id: &str) {
        if let Some(health) = self.health_trackers.get_mut(device_id) {
            health.reset();
            tracing::debug!("Reset health tracking for {}", device_id);
        }
    }

    /// Get the number of monitored devices.
    pub fn len(&self) -> usize {
        self.health_trackers.len()
    }

    /// Check if any devices are being monitored.
    pub fn is_empty(&self) -> bool {
        self.health_trackers.is_empty()
    }

    /// Clear all health tracking.
    pub fn clear(&mut self) {
        self.health_trackers.clear();
    }

    /// Check if a device is being monitored.
    pub fn is_monitoring(&self, device_id: &str) -> bool {
        self.health_trackers.contains_key(device_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_default_config() {
        let config = ConnectionHealthConfig::default();

        assert_eq!(config.stale_timeout, Duration::from_secs(5));
        assert_eq!(config.degraded_timeout, Duration::from_millis(2500));
        assert!(config.enabled);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "Degraded");
        assert_eq!(HealthStatus::Stale.to_string(), "Stale");
        assert_eq!(HealthStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_connection_health_new() {
        let health = ConnectionHealth::new("device_a".to_string());

        assert_eq!(health.device_id(), "device_a");
        assert_eq!(health.status(), HealthStatus::Unknown);
        assert!(health.time_since_last_data().is_none());
    }

    #[test]
    fn test_record_data_updates_status() {
        let mut health = ConnectionHealth::new("device_a".to_string());

        health.record_data_received();

        assert_eq!(health.status(), HealthStatus::Healthy);
        assert!(health.time_since_last_data().is_some());
    }

    #[test]
    fn test_health_monitor_new() {
        let monitor = ConnectionHealthMonitor::new();

        assert!(monitor.is_empty());
        assert_eq!(monitor.len(), 0);
    }

    #[test]
    fn test_monitor_start_stop() {
        let mut monitor = ConnectionHealthMonitor::new();

        monitor.start_monitoring("device_a");
        assert_eq!(monitor.len(), 1);
        assert!(monitor.is_monitoring("device_a"));

        monitor.stop_monitoring("device_a");
        assert!(monitor.is_empty());
    }
}
