//! Real-time metrics streaming handler for companion clients.
//!
//! This module implements the 1Hz metrics streaming from the desktop app
//! to connected mobile companion clients. It subscribes to sensor events
//! and aggregates them into periodic updates.
//!
//! ## Features
//!
//! - Subscribes to sensor data events (power, HR, cadence, speed)
//! - Aggregates metrics over 1-second intervals
//! - Broadcasts to all authenticated subscribed clients via CompanionServer
//! - Tracks cumulative session metrics (distance, calories, elapsed time)
//!
//! ## T005: Implement metrics streaming handler

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

use super::types::CompanionEvent;

/// Default metrics broadcast interval (1 Hz)
const METRICS_BROADCAST_INTERVAL: Duration = Duration::from_secs(1);

/// Configuration for the metrics streamer.
#[derive(Debug, Clone)]
pub struct MetricsStreamerConfig {
    /// How often to broadcast metrics (default: 1 second).
    pub broadcast_interval: Duration,
    /// Whether the streamer is enabled.
    pub enabled: bool,
}

impl Default for MetricsStreamerConfig {
    fn default() -> Self {
        Self {
            broadcast_interval: METRICS_BROADCAST_INTERVAL,
            enabled: true,
        }
    }
}

/// Current aggregated metrics for streaming.
///
/// These values represent the latest known metrics and are
/// updated as sensor data arrives.
#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    /// Current power in watts.
    pub power_watts: Option<u16>,
    /// Current heart rate in BPM.
    pub heart_rate_bpm: Option<u8>,
    /// Current cadence in RPM.
    pub cadence_rpm: Option<u8>,
    /// Current speed in km/h.
    pub speed_kmh: Option<f32>,
    /// Total distance in km.
    pub distance_km: f32,
    /// Elapsed time in seconds.
    pub elapsed_secs: u32,
    /// Estimated calories burned.
    pub calories: u32,
}

impl StreamingMetrics {
    /// Create a new empty streaming metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert to a CompanionEvent::Metrics for broadcasting.
    pub fn to_event(&self) -> CompanionEvent {
        CompanionEvent::Metrics {
            power_watts: self.power_watts,
            heart_rate_bpm: self.heart_rate_bpm,
            cadence_rpm: self.cadence_rpm,
            speed_kmh: self.speed_kmh,
            distance_km: self.distance_km,
            elapsed_secs: self.elapsed_secs,
            calories: self.calories,
        }
    }

    /// Update power reading.
    pub fn update_power(&mut self, watts: u16) {
        self.power_watts = Some(watts);
    }

    /// Update heart rate reading.
    pub fn update_heart_rate(&mut self, bpm: u8) {
        self.heart_rate_bpm = Some(bpm);
    }

    /// Update cadence reading.
    pub fn update_cadence(&mut self, rpm: u8) {
        self.cadence_rpm = Some(rpm);
    }

    /// Update speed reading.
    pub fn update_speed(&mut self, kmh: f32) {
        self.speed_kmh = Some(kmh);
    }

    /// Add distance increment.
    pub fn add_distance(&mut self, delta_km: f32) {
        self.distance_km += delta_km;
    }

    /// Set total distance.
    pub fn set_distance(&mut self, km: f32) {
        self.distance_km = km;
    }

    /// Update elapsed time.
    pub fn update_elapsed(&mut self, secs: u32) {
        self.elapsed_secs = secs;
    }

    /// Update calories burned.
    pub fn update_calories(&mut self, cals: u32) {
        self.calories = cals;
    }

    /// Reset all metrics to defaults.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Metrics streamer that broadcasts sensor data to companion clients at 1Hz.
///
/// The streamer maintains the current state of all metrics and sends
/// periodic updates to the companion server's event broadcast channel.
pub struct MetricsStreamer {
    /// Configuration for the streamer.
    config: MetricsStreamerConfig,
    /// Whether the streamer is currently running.
    is_running: Arc<RwLock<bool>>,
    /// Current metrics state.
    metrics: Arc<RwLock<StreamingMetrics>>,
    /// When the session started (for elapsed time calculation).
    session_start: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Channel to send events to the companion server.
    event_tx: broadcast::Sender<CompanionEvent>,
    /// Handle for the broadcast loop task.
    broadcast_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl MetricsStreamer {
    /// Create a new metrics streamer.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Broadcast channel to send metrics events to the companion server
    /// * `config` - Optional configuration for the streamer
    pub fn new(
        event_tx: broadcast::Sender<CompanionEvent>,
        config: Option<MetricsStreamerConfig>,
    ) -> Self {
        Self {
            config: config.unwrap_or_default(),
            is_running: Arc::new(RwLock::new(false)),
            metrics: Arc::new(RwLock::new(StreamingMetrics::new())),
            session_start: Arc::new(RwLock::new(None)),
            event_tx,
            broadcast_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the metrics streaming loop.
    ///
    /// This starts a background task that broadcasts current metrics
    /// at the configured interval (default 1Hz).
    pub async fn start(&self) {
        if !self.config.enabled {
            tracing::debug!("Metrics streamer is disabled");
            return;
        }

        if *self.is_running.read().await {
            tracing::debug!("Metrics streamer already running");
            return;
        }

        // Set session start time
        *self.session_start.write().await = Some(Utc::now());
        *self.is_running.write().await = true;

        // Clone state for the broadcast loop
        let is_running = Arc::clone(&self.is_running);
        let metrics = Arc::clone(&self.metrics);
        let session_start = Arc::clone(&self.session_start);
        let event_tx = self.event_tx.clone();
        let interval = self.config.broadcast_interval;

        // Spawn the broadcast loop
        let handle = tokio::spawn(async move {
            Self::run_broadcast_loop(is_running, metrics, session_start, event_tx, interval).await;
        });

        *self.broadcast_handle.write().await = Some(handle);

        tracing::info!(
            "Metrics streamer started with {}ms interval",
            self.config.broadcast_interval.as_millis()
        );
    }

    /// Stop the metrics streaming loop.
    pub async fn stop(&self) {
        if !*self.is_running.read().await {
            return;
        }

        *self.is_running.write().await = false;

        // Cancel the broadcast task
        if let Some(handle) = self.broadcast_handle.write().await.take() {
            handle.abort();
        }

        // Reset metrics
        self.metrics.write().await.reset();
        *self.session_start.write().await = None;

        tracing::info!("Metrics streamer stopped");
    }

    /// Run the broadcast loop that sends metrics at regular intervals.
    async fn run_broadcast_loop(
        is_running: Arc<RwLock<bool>>,
        metrics: Arc<RwLock<StreamingMetrics>>,
        session_start: Arc<RwLock<Option<DateTime<Utc>>>>,
        event_tx: broadcast::Sender<CompanionEvent>,
        interval: Duration,
    ) {
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            if !*is_running.read().await {
                break;
            }

            // Update elapsed time
            if let Some(start) = *session_start.read().await {
                let elapsed = (Utc::now() - start).num_seconds().max(0) as u32;
                metrics.write().await.elapsed_secs = elapsed;
            }

            // Broadcast current metrics
            let current_metrics = metrics.read().await.clone();
            let event = current_metrics.to_event();

            // Send to broadcast channel (ignore errors if no receivers)
            let _ = event_tx.send(event);

            tracing::trace!(
                "Broadcast metrics: power={:?}W, hr={:?}bpm, cadence={:?}rpm",
                current_metrics.power_watts,
                current_metrics.heart_rate_bpm,
                current_metrics.cadence_rpm
            );
        }
    }

    /// Check if the streamer is currently running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Update power reading from sensor.
    pub async fn update_power(&self, watts: u16) {
        self.metrics.write().await.update_power(watts);
    }

    /// Update heart rate reading from sensor.
    pub async fn update_heart_rate(&self, bpm: u8) {
        self.metrics.write().await.update_heart_rate(bpm);
    }

    /// Update cadence reading from sensor.
    pub async fn update_cadence(&self, rpm: u8) {
        self.metrics.write().await.update_cadence(rpm);
    }

    /// Update speed reading from sensor.
    pub async fn update_speed(&self, kmh: f32) {
        self.metrics.write().await.update_speed(kmh);
    }

    /// Add distance increment.
    pub async fn add_distance(&self, delta_km: f32) {
        self.metrics.write().await.add_distance(delta_km);
    }

    /// Set total distance.
    pub async fn set_distance(&self, km: f32) {
        self.metrics.write().await.set_distance(km);
    }

    /// Update calories burned.
    pub async fn update_calories(&self, cals: u32) {
        self.metrics.write().await.update_calories(cals);
    }

    /// Set all metrics from a LiveMetrics snapshot.
    ///
    /// This is used to sync with the daemon's current session state.
    pub async fn set_from_live_metrics(
        &self,
        power_watts: Option<u16>,
        heart_rate_bpm: Option<u8>,
        cadence_rpm: Option<u8>,
        speed_kmh: Option<f32>,
        distance_km: f32,
        calories: u32,
    ) {
        let mut metrics = self.metrics.write().await;
        metrics.power_watts = power_watts;
        metrics.heart_rate_bpm = heart_rate_bpm;
        metrics.cadence_rpm = cadence_rpm;
        metrics.speed_kmh = speed_kmh;
        metrics.distance_km = distance_km;
        metrics.calories = calories;
    }

    /// Get current metrics snapshot.
    pub async fn current_metrics(&self) -> StreamingMetrics {
        self.metrics.read().await.clone()
    }

    /// Get elapsed time in seconds.
    pub async fn elapsed_secs(&self) -> u32 {
        self.metrics.read().await.elapsed_secs
    }
}

/// Adapter to process sensor events and update the metrics streamer.
///
/// This struct provides a bridge between the SensorManager's event channel
/// and the MetricsStreamer, converting SensorReading events into metric updates.
pub struct SensorEventProcessor {
    /// Reference to the metrics streamer to update.
    streamer: Arc<MetricsStreamer>,
    /// Whether the processor is running.
    is_running: Arc<RwLock<bool>>,
    /// Handle for the processing task.
    process_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl SensorEventProcessor {
    /// Create a new sensor event processor.
    ///
    /// # Arguments
    ///
    /// * `streamer` - The metrics streamer to update with sensor data
    pub fn new(streamer: Arc<MetricsStreamer>) -> Self {
        Self {
            streamer,
            is_running: Arc::new(RwLock::new(false)),
            process_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start processing sensor events from a crossbeam channel.
    ///
    /// This spawns a background task that receives SensorEvent::Data events
    /// and updates the metrics streamer accordingly.
    ///
    /// # Arguments
    ///
    /// * `sensor_rx` - Crossbeam receiver for sensor events from SensorManager
    pub async fn start_with_crossbeam(
        &self,
        sensor_rx: crossbeam::channel::Receiver<crate::sensors::SensorEvent>,
    ) {
        if *self.is_running.read().await {
            return;
        }

        *self.is_running.write().await = true;

        let streamer = Arc::clone(&self.streamer);
        let is_running = Arc::clone(&self.is_running);

        let handle = tokio::spawn(async move {
            Self::process_crossbeam_events(streamer, sensor_rx, is_running).await;
        });

        *self.process_handle.write().await = Some(handle);

        tracing::info!("Sensor event processor started");
    }

    /// Process events from a crossbeam channel.
    async fn process_crossbeam_events(
        streamer: Arc<MetricsStreamer>,
        sensor_rx: crossbeam::channel::Receiver<crate::sensors::SensorEvent>,
        is_running: Arc<RwLock<bool>>,
    ) {
        use crate::sensors::SensorEvent;

        loop {
            if !*is_running.read().await {
                break;
            }

            // Use try_recv with a small sleep to allow checking is_running
            match sensor_rx.try_recv() {
                Ok(event) => {
                    if let SensorEvent::Data(reading) = event {
                        // Update power
                        if let Some(watts) = reading.power_watts {
                            streamer.update_power(watts).await;
                        }

                        // Update heart rate
                        if let Some(bpm) = reading.heart_rate_bpm {
                            streamer.update_heart_rate(bpm).await;
                        }

                        // Update cadence
                        if let Some(rpm) = reading.cadence_rpm {
                            streamer.update_cadence(rpm).await;
                        }

                        // Update speed
                        if let Some(kmh) = reading.speed_kmh {
                            streamer.update_speed(kmh).await;
                        }

                        // Update distance
                        if let Some(delta_m) = reading.distance_delta_m {
                            let delta_km = delta_m / 1000.0;
                            streamer.add_distance(delta_km).await;
                        }
                    }
                }
                Err(crossbeam::channel::TryRecvError::Empty) => {
                    // No events available, sleep briefly
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(crossbeam::channel::TryRecvError::Disconnected) => {
                    tracing::warn!("Sensor event channel disconnected");
                    break;
                }
            }
        }

        tracing::info!("Sensor event processor stopped");
    }

    /// Stop processing sensor events.
    pub async fn stop(&self) {
        *self.is_running.write().await = false;

        if let Some(handle) = self.process_handle.write().await.take() {
            handle.abort();
        }

        tracing::info!("Sensor event processor stopped");
    }

    /// Check if the processor is running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_metrics_default() {
        let metrics = StreamingMetrics::new();
        assert_eq!(metrics.power_watts, None);
        assert_eq!(metrics.heart_rate_bpm, None);
        assert_eq!(metrics.cadence_rpm, None);
        assert_eq!(metrics.speed_kmh, None);
        assert_eq!(metrics.distance_km, 0.0);
        assert_eq!(metrics.elapsed_secs, 0);
        assert_eq!(metrics.calories, 0);
    }

    #[test]
    fn test_streaming_metrics_updates() {
        let mut metrics = StreamingMetrics::new();

        metrics.update_power(250);
        assert_eq!(metrics.power_watts, Some(250));

        metrics.update_heart_rate(145);
        assert_eq!(metrics.heart_rate_bpm, Some(145));

        metrics.update_cadence(90);
        assert_eq!(metrics.cadence_rpm, Some(90));

        metrics.update_speed(32.5);
        assert_eq!(metrics.speed_kmh, Some(32.5));

        metrics.add_distance(1.5);
        assert!((metrics.distance_km - 1.5).abs() < 0.001);

        metrics.add_distance(0.5);
        assert!((metrics.distance_km - 2.0).abs() < 0.001);

        metrics.update_elapsed(3600);
        assert_eq!(metrics.elapsed_secs, 3600);

        metrics.update_calories(500);
        assert_eq!(metrics.calories, 500);
    }

    #[test]
    fn test_streaming_metrics_reset() {
        let mut metrics = StreamingMetrics::new();

        metrics.update_power(200);
        metrics.update_heart_rate(140);
        metrics.add_distance(10.0);

        metrics.reset();

        assert_eq!(metrics.power_watts, None);
        assert_eq!(metrics.heart_rate_bpm, None);
        assert_eq!(metrics.distance_km, 0.0);
    }

    #[test]
    fn test_streaming_metrics_to_event() {
        let mut metrics = StreamingMetrics::new();
        metrics.update_power(200);
        metrics.update_heart_rate(140);
        metrics.update_cadence(90);
        metrics.update_speed(30.0);
        metrics.set_distance(15.5);
        metrics.update_elapsed(1800);
        metrics.update_calories(350);

        let event = metrics.to_event();

        match event {
            CompanionEvent::Metrics {
                power_watts,
                heart_rate_bpm,
                cadence_rpm,
                speed_kmh,
                distance_km,
                elapsed_secs,
                calories,
            } => {
                assert_eq!(power_watts, Some(200));
                assert_eq!(heart_rate_bpm, Some(140));
                assert_eq!(cadence_rpm, Some(90));
                assert_eq!(speed_kmh, Some(30.0));
                assert!((distance_km - 15.5).abs() < 0.001);
                assert_eq!(elapsed_secs, 1800);
                assert_eq!(calories, 350);
            }
            _ => panic!("Expected Metrics event"),
        }
    }

    #[test]
    fn test_config_default() {
        let config = MetricsStreamerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.broadcast_interval, Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_metrics_streamer_creation() {
        let (tx, _rx) = broadcast::channel(16);
        let streamer = MetricsStreamer::new(tx, None);

        assert!(!streamer.is_running().await);
        assert_eq!(streamer.elapsed_secs().await, 0);
    }

    #[tokio::test]
    async fn test_metrics_streamer_update_methods() {
        let (tx, _rx) = broadcast::channel(16);
        let streamer = MetricsStreamer::new(tx, None);

        streamer.update_power(250).await;
        streamer.update_heart_rate(145).await;
        streamer.update_cadence(95).await;
        streamer.update_speed(35.0).await;
        streamer.add_distance(5.0).await;
        streamer.update_calories(200).await;

        let metrics = streamer.current_metrics().await;
        assert_eq!(metrics.power_watts, Some(250));
        assert_eq!(metrics.heart_rate_bpm, Some(145));
        assert_eq!(metrics.cadence_rpm, Some(95));
        assert_eq!(metrics.speed_kmh, Some(35.0));
        assert!((metrics.distance_km - 5.0).abs() < 0.001);
        assert_eq!(metrics.calories, 200);
    }

    #[tokio::test]
    async fn test_metrics_streamer_set_from_live_metrics() {
        let (tx, _rx) = broadcast::channel(16);
        let streamer = MetricsStreamer::new(tx, None);

        streamer
            .set_from_live_metrics(Some(300), Some(150), Some(85), Some(28.0), 20.5, 600)
            .await;

        let metrics = streamer.current_metrics().await;
        assert_eq!(metrics.power_watts, Some(300));
        assert_eq!(metrics.heart_rate_bpm, Some(150));
        assert_eq!(metrics.cadence_rpm, Some(85));
        assert_eq!(metrics.speed_kmh, Some(28.0));
        assert!((metrics.distance_km - 20.5).abs() < 0.001);
        assert_eq!(metrics.calories, 600);
    }

    #[tokio::test]
    async fn test_metrics_streamer_start_stop() {
        let (tx, mut rx) = broadcast::channel(16);
        let config = MetricsStreamerConfig {
            broadcast_interval: Duration::from_millis(50),
            enabled: true,
        };
        let streamer = MetricsStreamer::new(tx, Some(config));

        // Set some initial metrics
        streamer.update_power(200).await;
        streamer.update_heart_rate(140).await;

        // Start streaming
        streamer.start().await;
        assert!(streamer.is_running().await);

        // Wait for a broadcast
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should receive a metrics event
        if let Ok(event) = rx.try_recv() {
            match event {
                CompanionEvent::Metrics { power_watts, .. } => {
                    assert_eq!(power_watts, Some(200));
                }
                _ => panic!("Expected Metrics event"),
            }
        }

        // Stop streaming
        streamer.stop().await;
        assert!(!streamer.is_running().await);
    }

    #[tokio::test]
    async fn test_metrics_streamer_disabled() {
        let (tx, _rx) = broadcast::channel(16);
        let config = MetricsStreamerConfig {
            broadcast_interval: Duration::from_millis(50),
            enabled: false,
        };
        let streamer = MetricsStreamer::new(tx, Some(config));

        // Start should not actually start when disabled
        streamer.start().await;
        assert!(!streamer.is_running().await);
    }
}
