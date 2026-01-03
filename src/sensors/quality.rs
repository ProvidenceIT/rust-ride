//! Connection quality monitoring for sensor connections.
//!
//! This module provides comprehensive connection quality tracking for BLE/ANT+
//! sensors by monitoring RSSI, data rate, packet loss rate, and latency.
//! An overall quality score (0-100) is calculated from these metrics.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default RSSI threshold for excellent signal (dBm).
pub const DEFAULT_RSSI_EXCELLENT: i16 = -50;

/// Default RSSI threshold for good signal (dBm).
pub const DEFAULT_RSSI_GOOD: i16 = -70;

/// Default RSSI threshold for fair signal (dBm).
pub const DEFAULT_RSSI_FAIR: i16 = -85;

/// Default minimum acceptable RSSI (dBm).
pub const DEFAULT_RSSI_MIN: i16 = -100;

/// Default expected data rate (packets per second).
pub const DEFAULT_EXPECTED_DATA_RATE: f32 = 1.0;

/// Default latency threshold for excellent quality (ms).
pub const DEFAULT_LATENCY_EXCELLENT_MS: u64 = 50;

/// Default latency threshold for good quality (ms).
pub const DEFAULT_LATENCY_GOOD_MS: u64 = 100;

/// Default latency threshold for fair quality (ms).
pub const DEFAULT_LATENCY_FAIR_MS: u64 = 200;

/// Default packet loss threshold for excellent quality (percentage).
pub const DEFAULT_PACKET_LOSS_EXCELLENT: f32 = 0.5;

/// Default packet loss threshold for good quality (percentage).
pub const DEFAULT_PACKET_LOSS_GOOD: f32 = 2.0;

/// Default packet loss threshold for fair quality (percentage).
pub const DEFAULT_PACKET_LOSS_FAIR: f32 = 5.0;

/// Connection quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityLevel {
    /// Connection quality is poor (may experience issues).
    Poor,
    /// Connection quality is fair (usable but may have occasional issues).
    Fair,
    /// Connection quality is good (reliable connection).
    Good,
    /// Connection quality is excellent (optimal performance).
    Excellent,
}

impl std::fmt::Display for QualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityLevel::Excellent => write!(f, "Excellent"),
            QualityLevel::Good => write!(f, "Good"),
            QualityLevel::Fair => write!(f, "Fair"),
            QualityLevel::Poor => write!(f, "Poor"),
        }
    }
}

impl QualityLevel {
    /// Convert quality level to a score range (0-100).
    pub fn to_score_range(&self) -> (u8, u8) {
        match self {
            QualityLevel::Excellent => (85, 100),
            QualityLevel::Good => (65, 84),
            QualityLevel::Fair => (40, 64),
            QualityLevel::Poor => (0, 39),
        }
    }

    /// Get quality level from a score (0-100).
    pub fn from_score(score: u8) -> Self {
        if score >= 85 {
            QualityLevel::Excellent
        } else if score >= 65 {
            QualityLevel::Good
        } else if score >= 40 {
            QualityLevel::Fair
        } else {
            QualityLevel::Poor
        }
    }

    /// Get the number of signal bars (1-4) for this quality level.
    pub fn signal_bars(&self) -> u8 {
        match self {
            QualityLevel::Excellent => 4,
            QualityLevel::Good => 3,
            QualityLevel::Fair => 2,
            QualityLevel::Poor => 1,
        }
    }
}

/// Configuration for connection quality tracking.
#[derive(Debug, Clone)]
pub struct ConnectionQualityConfig {
    /// RSSI threshold for excellent signal (dBm).
    pub rssi_excellent: i16,
    /// RSSI threshold for good signal (dBm).
    pub rssi_good: i16,
    /// RSSI threshold for fair signal (dBm).
    pub rssi_fair: i16,
    /// Minimum acceptable RSSI (dBm).
    pub rssi_min: i16,
    /// Expected data rate (packets per second).
    pub expected_data_rate: f32,
    /// Latency threshold for excellent quality (ms).
    pub latency_excellent_ms: u64,
    /// Latency threshold for good quality (ms).
    pub latency_good_ms: u64,
    /// Latency threshold for fair quality (ms).
    pub latency_fair_ms: u64,
    /// Packet loss threshold for excellent quality (percentage).
    pub packet_loss_excellent: f32,
    /// Packet loss threshold for good quality (percentage).
    pub packet_loss_good: f32,
    /// Packet loss threshold for fair quality (percentage).
    pub packet_loss_fair: f32,
    /// Window size for calculating metrics (default: 30s).
    pub metrics_window: Duration,
    /// Weight for RSSI in overall score (0-1).
    pub rssi_weight: f32,
    /// Weight for data rate in overall score (0-1).
    pub data_rate_weight: f32,
    /// Weight for packet loss in overall score (0-1).
    pub packet_loss_weight: f32,
    /// Weight for latency in overall score (0-1).
    pub latency_weight: f32,
    /// Whether quality monitoring is enabled (default: true).
    pub enabled: bool,
}

impl Default for ConnectionQualityConfig {
    fn default() -> Self {
        Self {
            rssi_excellent: DEFAULT_RSSI_EXCELLENT,
            rssi_good: DEFAULT_RSSI_GOOD,
            rssi_fair: DEFAULT_RSSI_FAIR,
            rssi_min: DEFAULT_RSSI_MIN,
            expected_data_rate: DEFAULT_EXPECTED_DATA_RATE,
            latency_excellent_ms: DEFAULT_LATENCY_EXCELLENT_MS,
            latency_good_ms: DEFAULT_LATENCY_GOOD_MS,
            latency_fair_ms: DEFAULT_LATENCY_FAIR_MS,
            packet_loss_excellent: DEFAULT_PACKET_LOSS_EXCELLENT,
            packet_loss_good: DEFAULT_PACKET_LOSS_GOOD,
            packet_loss_fair: DEFAULT_PACKET_LOSS_FAIR,
            metrics_window: Duration::from_secs(30),
            rssi_weight: 0.35,
            data_rate_weight: 0.30,
            packet_loss_weight: 0.20,
            latency_weight: 0.15,
            enabled: true,
        }
    }
}

impl ConnectionQualityConfig {
    /// Create a strict quality configuration for critical sensors (trainers, power meters).
    pub fn strict() -> Self {
        Self {
            rssi_excellent: -45,
            rssi_good: -65,
            rssi_fair: -80,
            rssi_min: -95,
            expected_data_rate: 1.0,
            latency_excellent_ms: 30,
            latency_good_ms: 75,
            latency_fair_ms: 150,
            packet_loss_excellent: 0.2,
            packet_loss_good: 1.0,
            packet_loss_fair: 3.0,
            metrics_window: Duration::from_secs(15),
            rssi_weight: 0.30,
            data_rate_weight: 0.35,
            packet_loss_weight: 0.25,
            latency_weight: 0.10,
            enabled: true,
        }
    }

    /// Create a relaxed quality configuration for less critical sensors (HR, cadence).
    pub fn relaxed() -> Self {
        Self {
            rssi_excellent: -55,
            rssi_good: -75,
            rssi_fair: -90,
            rssi_min: -105,
            expected_data_rate: 0.5,
            latency_excellent_ms: 100,
            latency_good_ms: 200,
            latency_fair_ms: 400,
            packet_loss_excellent: 1.0,
            packet_loss_good: 3.0,
            packet_loss_fair: 8.0,
            metrics_window: Duration::from_secs(60),
            rssi_weight: 0.40,
            data_rate_weight: 0.25,
            packet_loss_weight: 0.20,
            latency_weight: 0.15,
            enabled: true,
        }
    }

    /// Create a disabled quality tracking configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

/// RSSI (signal strength) sample with timestamp.
#[derive(Debug, Clone, Copy)]
struct RssiSample {
    /// RSSI value in dBm.
    value: i16,
    /// When this sample was recorded.
    timestamp: Instant,
}

/// Latency sample with timestamp.
#[derive(Debug, Clone, Copy)]
struct LatencySample {
    /// Latency in milliseconds.
    value_ms: u64,
    /// When this sample was recorded.
    timestamp: Instant,
}

/// Data packet record for rate and loss calculation.
#[derive(Debug, Clone, Copy)]
struct PacketRecord {
    /// Expected sequence number.
    expected_seq: Option<u32>,
    /// Actual sequence number received.
    actual_seq: Option<u32>,
    /// When this packet was recorded.
    timestamp: Instant,
}

/// Connection quality tracking for a single sensor.
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// Device identifier.
    device_id: String,
    /// When quality tracking started.
    started_at: Instant,
    /// RSSI samples within the metrics window.
    rssi_samples: Vec<RssiSample>,
    /// Latency samples within the metrics window.
    latency_samples: Vec<LatencySample>,
    /// Packet records for rate and loss calculation.
    packet_records: Vec<PacketRecord>,
    /// Current overall quality score (0-100).
    quality_score: u8,
    /// Current quality level.
    quality_level: QualityLevel,
    /// Configuration for quality tracking.
    config: ConnectionQualityConfig,
    /// Last calculated metrics.
    last_metrics: QualityMetrics,
    /// Last expected sequence number for packet loss tracking.
    last_seq: Option<u32>,
    /// Total packets expected (for loss calculation).
    packets_expected: u64,
    /// Total packets lost (for loss calculation).
    packets_lost: u64,
}

/// Quality metrics calculated from samples.
#[derive(Debug, Clone, Default)]
pub struct QualityMetrics {
    /// Average RSSI in dBm.
    pub rssi_avg: Option<i16>,
    /// Minimum RSSI in dBm (worst signal).
    pub rssi_min: Option<i16>,
    /// Maximum RSSI in dBm (best signal).
    pub rssi_max: Option<i16>,
    /// Current data rate (packets per second).
    pub data_rate: f32,
    /// Packet loss rate (percentage 0-100).
    pub packet_loss_rate: f32,
    /// Average latency in milliseconds.
    pub latency_avg_ms: Option<u64>,
    /// Minimum latency in milliseconds.
    pub latency_min_ms: Option<u64>,
    /// Maximum latency in milliseconds.
    pub latency_max_ms: Option<u64>,
    /// RSSI component score (0-100).
    pub rssi_score: u8,
    /// Data rate component score (0-100).
    pub data_rate_score: u8,
    /// Packet loss component score (0-100).
    pub packet_loss_score: u8,
    /// Latency component score (0-100).
    pub latency_score: u8,
}

impl ConnectionQuality {
    /// Create a new connection quality tracker.
    pub fn new(device_id: String) -> Self {
        Self::with_config(device_id, ConnectionQualityConfig::default())
    }

    /// Create a new connection quality tracker with custom configuration.
    pub fn with_config(device_id: String, config: ConnectionQualityConfig) -> Self {
        Self {
            device_id,
            started_at: Instant::now(),
            rssi_samples: Vec::with_capacity(256),
            latency_samples: Vec::with_capacity(256),
            packet_records: Vec::with_capacity(512),
            quality_score: 0,
            quality_level: QualityLevel::Poor,
            config,
            last_metrics: QualityMetrics::default(),
            last_seq: None,
            packets_expected: 0,
            packets_lost: 0,
        }
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Record an RSSI value.
    pub fn record_rssi(&mut self, rssi: i16) {
        if !self.config.enabled {
            return;
        }

        self.rssi_samples.push(RssiSample {
            value: rssi,
            timestamp: Instant::now(),
        });

        self.prune_old_samples();
        self.update_quality();
    }

    /// Record a latency measurement.
    pub fn record_latency(&mut self, latency_ms: u64) {
        if !self.config.enabled {
            return;
        }

        self.latency_samples.push(LatencySample {
            value_ms: latency_ms,
            timestamp: Instant::now(),
        });

        self.prune_old_samples();
        self.update_quality();
    }

    /// Record that a data packet was received.
    ///
    /// If a sequence number is provided, it's used to calculate packet loss.
    pub fn record_packet(&mut self, sequence: Option<u32>) {
        if !self.config.enabled {
            return;
        }

        let now = Instant::now();

        // Calculate packet loss if sequence numbers are available
        if let (Some(seq), Some(last)) = (sequence, self.last_seq) {
            let expected_seq = last.wrapping_add(1);
            if seq != expected_seq && seq > last {
                // Packets were lost
                let lost = seq.wrapping_sub(expected_seq) as u64;
                self.packets_lost += lost;
                self.packets_expected += lost + 1;
            } else {
                self.packets_expected += 1;
            }
        } else if sequence.is_some() {
            self.packets_expected += 1;
        }

        self.last_seq = sequence;

        self.packet_records.push(PacketRecord {
            expected_seq: self.last_seq.map(|s| s.wrapping_add(1)),
            actual_seq: sequence,
            timestamp: now,
        });

        self.prune_old_samples();
        self.update_quality();
    }

    /// Record a data packet with all metrics at once.
    pub fn record_data(&mut self, rssi: Option<i16>, latency_ms: Option<u64>, sequence: Option<u32>) {
        if !self.config.enabled {
            return;
        }

        let now = Instant::now();

        if let Some(r) = rssi {
            self.rssi_samples.push(RssiSample {
                value: r,
                timestamp: now,
            });
        }

        if let Some(l) = latency_ms {
            self.latency_samples.push(LatencySample {
                value_ms: l,
                timestamp: now,
            });
        }

        // Handle sequence number for packet loss tracking
        if let (Some(seq), Some(last)) = (sequence, self.last_seq) {
            let expected_seq = last.wrapping_add(1);
            if seq != expected_seq && seq > last {
                let lost = seq.wrapping_sub(expected_seq) as u64;
                self.packets_lost += lost;
                self.packets_expected += lost + 1;
            } else {
                self.packets_expected += 1;
            }
        } else if sequence.is_some() {
            self.packets_expected += 1;
        }

        self.last_seq = sequence;

        self.packet_records.push(PacketRecord {
            expected_seq: self.last_seq.map(|s| s.wrapping_add(1)),
            actual_seq: sequence,
            timestamp: now,
        });

        self.prune_old_samples();
        self.update_quality();
    }

    /// Prune samples older than the metrics window.
    fn prune_old_samples(&mut self) {
        let cutoff = Instant::now() - self.config.metrics_window;

        self.rssi_samples.retain(|s| s.timestamp >= cutoff);
        self.latency_samples.retain(|s| s.timestamp >= cutoff);
        self.packet_records.retain(|r| r.timestamp >= cutoff);
    }

    /// Calculate RSSI score (0-100) from average RSSI.
    fn calculate_rssi_score(&self, avg_rssi: i16) -> u8 {
        if avg_rssi >= self.config.rssi_excellent {
            100
        } else if avg_rssi >= self.config.rssi_good {
            let range = (self.config.rssi_excellent - self.config.rssi_good) as f32;
            let diff = (self.config.rssi_excellent - avg_rssi) as f32;
            (100.0 - (diff / range * 25.0)) as u8
        } else if avg_rssi >= self.config.rssi_fair {
            let range = (self.config.rssi_good - self.config.rssi_fair) as f32;
            let diff = (self.config.rssi_good - avg_rssi) as f32;
            (75.0 - (diff / range * 25.0)) as u8
        } else if avg_rssi >= self.config.rssi_min {
            let range = (self.config.rssi_fair - self.config.rssi_min) as f32;
            let diff = (self.config.rssi_fair - avg_rssi) as f32;
            (50.0 - (diff / range * 40.0)).max(10.0) as u8
        } else {
            10 // Minimum score for very poor signal
        }
    }

    /// Calculate data rate score (0-100) from actual rate vs expected.
    fn calculate_data_rate_score(&self, rate: f32) -> u8 {
        if rate <= 0.0 {
            return 0;
        }

        let ratio = rate / self.config.expected_data_rate;

        if ratio >= 1.0 {
            100
        } else if ratio >= 0.8 {
            ((ratio - 0.8) / 0.2 * 15.0 + 85.0) as u8
        } else if ratio >= 0.5 {
            ((ratio - 0.5) / 0.3 * 25.0 + 60.0) as u8
        } else if ratio >= 0.2 {
            ((ratio - 0.2) / 0.3 * 30.0 + 30.0) as u8
        } else {
            (ratio / 0.2 * 30.0) as u8
        }
    }

    /// Calculate packet loss score (0-100) from loss percentage.
    fn calculate_packet_loss_score(&self, loss_rate: f32) -> u8 {
        if loss_rate <= self.config.packet_loss_excellent {
            100
        } else if loss_rate <= self.config.packet_loss_good {
            let range = self.config.packet_loss_good - self.config.packet_loss_excellent;
            let diff = loss_rate - self.config.packet_loss_excellent;
            (100.0 - (diff / range * 15.0)) as u8
        } else if loss_rate <= self.config.packet_loss_fair {
            let range = self.config.packet_loss_fair - self.config.packet_loss_good;
            let diff = loss_rate - self.config.packet_loss_good;
            (85.0 - (diff / range * 25.0)) as u8
        } else if loss_rate <= 20.0 {
            let range = 20.0 - self.config.packet_loss_fair;
            let diff = loss_rate - self.config.packet_loss_fair;
            (60.0 - (diff / range * 40.0)).max(10.0) as u8
        } else {
            10 // Minimum score for very high packet loss
        }
    }

    /// Calculate latency score (0-100) from average latency.
    fn calculate_latency_score(&self, avg_latency_ms: u64) -> u8 {
        if avg_latency_ms <= self.config.latency_excellent_ms {
            100
        } else if avg_latency_ms <= self.config.latency_good_ms {
            let range = (self.config.latency_good_ms - self.config.latency_excellent_ms) as f32;
            let diff = (avg_latency_ms - self.config.latency_excellent_ms) as f32;
            (100.0 - (diff / range * 15.0)) as u8
        } else if avg_latency_ms <= self.config.latency_fair_ms {
            let range = (self.config.latency_fair_ms - self.config.latency_good_ms) as f32;
            let diff = (avg_latency_ms - self.config.latency_good_ms) as f32;
            (85.0 - (diff / range * 25.0)) as u8
        } else if avg_latency_ms <= 500 {
            let range = (500 - self.config.latency_fair_ms) as f32;
            let diff = (avg_latency_ms - self.config.latency_fair_ms) as f32;
            (60.0 - (diff / range * 40.0)).max(10.0) as u8
        } else {
            10 // Minimum score for very high latency
        }
    }

    /// Update quality metrics and score.
    fn update_quality(&mut self) {
        if !self.config.enabled {
            return;
        }

        // Calculate RSSI metrics
        let (rssi_avg, rssi_min, rssi_max, rssi_score) = if !self.rssi_samples.is_empty() {
            let sum: i32 = self.rssi_samples.iter().map(|s| s.value as i32).sum();
            let avg = (sum / self.rssi_samples.len() as i32) as i16;
            let min = self.rssi_samples.iter().map(|s| s.value).min().unwrap();
            let max = self.rssi_samples.iter().map(|s| s.value).max().unwrap();
            let score = self.calculate_rssi_score(avg);
            (Some(avg), Some(min), Some(max), score)
        } else {
            (None, None, None, 50) // Default to mid score if no RSSI data
        };

        // Calculate data rate
        let data_rate = if self.packet_records.len() >= 2 {
            let window_secs = self.config.metrics_window.as_secs_f32();
            self.packet_records.len() as f32 / window_secs
        } else {
            0.0
        };
        let data_rate_score = self.calculate_data_rate_score(data_rate);

        // Calculate packet loss rate
        let packet_loss_rate = if self.packets_expected > 0 {
            (self.packets_lost as f32 / self.packets_expected as f32) * 100.0
        } else {
            0.0
        };
        let packet_loss_score = self.calculate_packet_loss_score(packet_loss_rate);

        // Calculate latency metrics
        let (latency_avg, latency_min, latency_max, latency_score) = if !self.latency_samples.is_empty() {
            let sum: u64 = self.latency_samples.iter().map(|s| s.value_ms).sum();
            let avg = sum / self.latency_samples.len() as u64;
            let min = self.latency_samples.iter().map(|s| s.value_ms).min().unwrap();
            let max = self.latency_samples.iter().map(|s| s.value_ms).max().unwrap();
            let score = self.calculate_latency_score(avg);
            (Some(avg), Some(min), Some(max), score)
        } else {
            (None, None, None, 50) // Default to mid score if no latency data
        };

        // Store metrics
        self.last_metrics = QualityMetrics {
            rssi_avg,
            rssi_min,
            rssi_max,
            data_rate,
            packet_loss_rate,
            latency_avg_ms: latency_avg,
            latency_min_ms: latency_min,
            latency_max_ms: latency_max,
            rssi_score,
            data_rate_score,
            packet_loss_score,
            latency_score,
        };

        // Calculate weighted overall score
        let weighted_score =
            (rssi_score as f32 * self.config.rssi_weight) +
            (data_rate_score as f32 * self.config.data_rate_weight) +
            (packet_loss_score as f32 * self.config.packet_loss_weight) +
            (latency_score as f32 * self.config.latency_weight);

        self.quality_score = weighted_score.round() as u8;
        self.quality_level = QualityLevel::from_score(self.quality_score);
    }

    /// Get the current quality score (0-100).
    pub fn score(&self) -> u8 {
        self.quality_score
    }

    /// Get the current quality level.
    pub fn level(&self) -> QualityLevel {
        self.quality_level
    }

    /// Get the current quality metrics.
    pub fn metrics(&self) -> &QualityMetrics {
        &self.last_metrics
    }

    /// Get the number of signal bars (1-4).
    pub fn signal_bars(&self) -> u8 {
        self.quality_level.signal_bars()
    }

    /// Check if the connection quality is good enough.
    pub fn is_acceptable(&self) -> bool {
        self.quality_level >= QualityLevel::Fair
    }

    /// Check if the connection quality needs attention.
    pub fn needs_attention(&self) -> bool {
        self.quality_level == QualityLevel::Poor
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ConnectionQualityConfig {
        &self.config
    }

    /// Reset quality tracking state.
    ///
    /// Call this after reconnection.
    pub fn reset(&mut self) {
        self.started_at = Instant::now();
        self.rssi_samples.clear();
        self.latency_samples.clear();
        self.packet_records.clear();
        self.quality_score = 0;
        self.quality_level = QualityLevel::Poor;
        self.last_metrics = QualityMetrics::default();
        self.last_seq = None;
        self.packets_expected = 0;
        self.packets_lost = 0;
    }

    /// Get tracking uptime.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get quality statistics for display.
    pub fn stats(&self) -> QualityStats {
        QualityStats {
            device_id: self.device_id.clone(),
            score: self.quality_score,
            level: self.quality_level,
            metrics: self.last_metrics.clone(),
            signal_bars: self.signal_bars(),
            uptime: self.uptime(),
            sample_count: self.packet_records.len(),
        }
    }
}

/// Quality statistics for a sensor.
#[derive(Debug, Clone)]
pub struct QualityStats {
    /// Device identifier.
    pub device_id: String,
    /// Overall quality score (0-100).
    pub score: u8,
    /// Quality level.
    pub level: QualityLevel,
    /// Detailed metrics.
    pub metrics: QualityMetrics,
    /// Number of signal bars (1-4).
    pub signal_bars: u8,
    /// Tracking uptime.
    pub uptime: Duration,
    /// Number of samples recorded.
    pub sample_count: usize,
}

impl QualityStats {
    /// Get a human-readable quality summary.
    pub fn summary(&self) -> String {
        format!(
            "{} ({}%)",
            self.level,
            self.score
        )
    }

    /// Get a detailed quality text.
    pub fn detail_text(&self) -> String {
        let mut parts = Vec::new();

        if let Some(rssi) = self.metrics.rssi_avg {
            parts.push(format!("RSSI: {} dBm", rssi));
        }

        parts.push(format!("Rate: {:.1} pkt/s", self.metrics.data_rate));

        if self.metrics.packet_loss_rate > 0.0 {
            parts.push(format!("Loss: {:.1}%", self.metrics.packet_loss_rate));
        }

        if let Some(latency) = self.metrics.latency_avg_ms {
            parts.push(format!("Latency: {} ms", latency));
        }

        parts.join(", ")
    }

    /// Check if quality needs attention.
    pub fn needs_attention(&self) -> bool {
        self.level == QualityLevel::Poor
    }

    /// Check if quality is degraded (fair or poor).
    pub fn is_degraded(&self) -> bool {
        matches!(self.level, QualityLevel::Poor | QualityLevel::Fair)
    }
}

/// Manages connection quality monitoring for multiple sensors.
#[derive(Debug, Default)]
pub struct ConnectionQualityMonitor {
    /// Per-device quality tracking.
    quality_trackers: HashMap<String, ConnectionQuality>,
    /// Default configuration for new sensors.
    default_config: ConnectionQualityConfig,
}

impl ConnectionQualityMonitor {
    /// Create a new quality monitor with default configuration.
    pub fn new() -> Self {
        Self {
            quality_trackers: HashMap::new(),
            default_config: ConnectionQualityConfig::default(),
        }
    }

    /// Create a new quality monitor with custom default configuration.
    pub fn with_config(config: ConnectionQualityConfig) -> Self {
        Self {
            quality_trackers: HashMap::new(),
            default_config: config,
        }
    }

    /// Start monitoring quality for a device.
    pub fn start_monitoring(&mut self, device_id: &str) {
        self.start_monitoring_with_config(device_id, self.default_config.clone());
    }

    /// Start monitoring with a custom configuration.
    pub fn start_monitoring_with_config(&mut self, device_id: &str, config: ConnectionQualityConfig) {
        let quality = ConnectionQuality::with_config(device_id.to_string(), config);
        self.quality_trackers.insert(device_id.to_string(), quality);
        tracing::debug!("Started quality monitoring for {}", device_id);
    }

    /// Stop monitoring quality for a device.
    pub fn stop_monitoring(&mut self, device_id: &str) {
        if self.quality_trackers.remove(device_id).is_some() {
            tracing::debug!("Stopped quality monitoring for {}", device_id);
        }
    }

    /// Record an RSSI value for a device.
    pub fn record_rssi(&mut self, device_id: &str, rssi: i16) {
        if let Some(quality) = self.quality_trackers.get_mut(device_id) {
            quality.record_rssi(rssi);
        }
    }

    /// Record latency for a device.
    pub fn record_latency(&mut self, device_id: &str, latency_ms: u64) {
        if let Some(quality) = self.quality_trackers.get_mut(device_id) {
            quality.record_latency(latency_ms);
        }
    }

    /// Record a packet for a device.
    pub fn record_packet(&mut self, device_id: &str, sequence: Option<u32>) {
        if let Some(quality) = self.quality_trackers.get_mut(device_id) {
            quality.record_packet(sequence);
        }
    }

    /// Record data with all metrics at once.
    pub fn record_data(&mut self, device_id: &str, rssi: Option<i16>, latency_ms: Option<u64>, sequence: Option<u32>) {
        if let Some(quality) = self.quality_trackers.get_mut(device_id) {
            quality.record_data(rssi, latency_ms, sequence);
        }
    }

    /// Get the quality score for a device.
    pub fn get_score(&self, device_id: &str) -> Option<u8> {
        self.quality_trackers.get(device_id).map(|q| q.score())
    }

    /// Get the quality level for a device.
    pub fn get_level(&self, device_id: &str) -> Option<QualityLevel> {
        self.quality_trackers.get(device_id).map(|q| q.level())
    }

    /// Get quality statistics for a device.
    pub fn get_stats(&self, device_id: &str) -> Option<QualityStats> {
        self.quality_trackers.get(device_id).map(|q| q.stats())
    }

    /// Get all quality statistics.
    pub fn get_all_stats(&self) -> Vec<QualityStats> {
        self.quality_trackers.values().map(|q| q.stats()).collect()
    }

    /// Get devices with poor quality connections.
    pub fn get_poor_quality_devices(&self) -> Vec<String> {
        self.quality_trackers
            .iter()
            .filter(|(_, q)| q.needs_attention())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get devices with degraded quality (fair or poor).
    pub fn get_degraded_devices(&self) -> Vec<String> {
        self.quality_trackers
            .iter()
            .filter(|(_, q)| q.level() <= QualityLevel::Fair)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Reset quality tracking for a device.
    ///
    /// Call this after successful reconnection.
    pub fn reset(&mut self, device_id: &str) {
        if let Some(quality) = self.quality_trackers.get_mut(device_id) {
            quality.reset();
            tracing::debug!("Reset quality tracking for {}", device_id);
        }
    }

    /// Get the number of monitored devices.
    pub fn len(&self) -> usize {
        self.quality_trackers.len()
    }

    /// Check if any devices are being monitored.
    pub fn is_empty(&self) -> bool {
        self.quality_trackers.is_empty()
    }

    /// Clear all quality tracking.
    pub fn clear(&mut self) {
        self.quality_trackers.clear();
    }

    /// Check if a device is being monitored.
    pub fn is_monitoring(&self, device_id: &str) -> bool {
        self.quality_trackers.contains_key(device_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ConnectionQualityConfig::default();

        assert_eq!(config.rssi_excellent, DEFAULT_RSSI_EXCELLENT);
        assert_eq!(config.rssi_good, DEFAULT_RSSI_GOOD);
        assert!(config.enabled);
    }

    #[test]
    fn test_quality_level_display() {
        assert_eq!(QualityLevel::Excellent.to_string(), "Excellent");
        assert_eq!(QualityLevel::Good.to_string(), "Good");
        assert_eq!(QualityLevel::Fair.to_string(), "Fair");
        assert_eq!(QualityLevel::Poor.to_string(), "Poor");
    }

    #[test]
    fn test_quality_level_from_score() {
        assert_eq!(QualityLevel::from_score(100), QualityLevel::Excellent);
        assert_eq!(QualityLevel::from_score(85), QualityLevel::Excellent);
        assert_eq!(QualityLevel::from_score(84), QualityLevel::Good);
        assert_eq!(QualityLevel::from_score(65), QualityLevel::Good);
        assert_eq!(QualityLevel::from_score(64), QualityLevel::Fair);
        assert_eq!(QualityLevel::from_score(40), QualityLevel::Fair);
        assert_eq!(QualityLevel::from_score(39), QualityLevel::Poor);
        assert_eq!(QualityLevel::from_score(0), QualityLevel::Poor);
    }

    #[test]
    fn test_signal_bars() {
        assert_eq!(QualityLevel::Excellent.signal_bars(), 4);
        assert_eq!(QualityLevel::Good.signal_bars(), 3);
        assert_eq!(QualityLevel::Fair.signal_bars(), 2);
        assert_eq!(QualityLevel::Poor.signal_bars(), 1);
    }

    #[test]
    fn test_connection_quality_new() {
        let quality = ConnectionQuality::new("device_a".to_string());

        assert_eq!(quality.device_id(), "device_a");
        assert_eq!(quality.score(), 0);
        assert_eq!(quality.level(), QualityLevel::Poor);
    }

    #[test]
    fn test_quality_monitor_new() {
        let monitor = ConnectionQualityMonitor::new();

        assert!(monitor.is_empty());
        assert_eq!(monitor.len(), 0);
    }

    #[test]
    fn test_monitor_start_stop() {
        let mut monitor = ConnectionQualityMonitor::new();

        monitor.start_monitoring("device_a");
        assert_eq!(monitor.len(), 1);
        assert!(monitor.is_monitoring("device_a"));

        monitor.stop_monitoring("device_a");
        assert!(monitor.is_empty());
    }
}
