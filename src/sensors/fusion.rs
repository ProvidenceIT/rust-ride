//! Sensor fusion module for combining multiple sensor sources.
//!
//! T131: Create SensorFusionConfig and FusionDiagnostics types
//! T132: Implement SensorFusion trait
//! T133: Implement complementary filter algorithm for cadence fusion
//! T134: Implement sensor dropout detection and seamless fallback

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Configuration for sensor fusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorFusionConfig {
    /// Primary sensor weight (0.0-1.0)
    pub primary_weight: f32,
    /// Secondary sensor weight (0.0-1.0)
    pub secondary_weight: f32,
    /// Timeout before considering a sensor as dropped (milliseconds)
    pub dropout_timeout_ms: u32,
    /// Maximum allowed deviation between sensors before flagging inconsistency
    pub max_deviation_percent: f32,
    /// Enable automatic fallback to single sensor on dropout
    pub auto_fallback: bool,
    /// Smoothing factor for complementary filter (0.0-1.0)
    pub smoothing_factor: f32,
    /// Window size for moving average (number of samples)
    pub window_size: usize,
}

impl Default for SensorFusionConfig {
    fn default() -> Self {
        Self {
            primary_weight: 0.6,
            secondary_weight: 0.4,
            dropout_timeout_ms: 3000,
            max_deviation_percent: 20.0,
            auto_fallback: true,
            smoothing_factor: 0.3,
            window_size: 5,
        }
    }
}

impl SensorFusionConfig {
    /// Create a config that heavily favors the primary sensor.
    pub fn primary_dominant() -> Self {
        Self {
            primary_weight: 0.8,
            secondary_weight: 0.2,
            ..Default::default()
        }
    }

    /// Create a config with equal weights.
    pub fn equal_weights() -> Self {
        Self {
            primary_weight: 0.5,
            secondary_weight: 0.5,
            ..Default::default()
        }
    }

    /// Validate and normalize weights.
    pub fn normalize_weights(&mut self) {
        let total = self.primary_weight + self.secondary_weight;
        if total > 0.0 {
            self.primary_weight /= total;
            self.secondary_weight /= total;
        } else {
            self.primary_weight = 0.5;
            self.secondary_weight = 0.5;
        }
    }
}

/// Diagnostic information about sensor fusion state.
#[derive(Debug, Clone, Default)]
pub struct FusionDiagnostics {
    /// Whether primary sensor is active
    pub primary_active: bool,
    /// Whether secondary sensor is active
    pub secondary_active: bool,
    /// Last value from primary sensor
    pub primary_value: Option<f32>,
    /// Last value from secondary sensor
    pub secondary_value: Option<f32>,
    /// Current fused value
    pub fused_value: Option<f32>,
    /// Time since last primary reading
    pub primary_age_ms: u32,
    /// Time since last secondary reading
    pub secondary_age_ms: u32,
    /// Current deviation between sensors (percent)
    pub deviation_percent: Option<f32>,
    /// Whether sensors are in agreement (within max deviation)
    pub sensors_agree: bool,
    /// Current fusion mode
    pub mode: FusionMode,
    /// Number of samples in the current window
    pub sample_count: usize,
}

/// Fusion mode indicating data source state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FusionMode {
    /// Both sensors providing data, fusion active
    #[default]
    DualSensor,
    /// Only primary sensor active
    PrimaryOnly,
    /// Only secondary sensor active
    SecondaryOnly,
    /// No sensors active
    NoData,
    /// Sensors disagreeing significantly
    Inconsistent,
}

impl FusionMode {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            FusionMode::DualSensor => "Both sensors active",
            FusionMode::PrimaryOnly => "Primary sensor only",
            FusionMode::SecondaryOnly => "Secondary sensor only",
            FusionMode::NoData => "No sensor data",
            FusionMode::Inconsistent => "Sensors disagree",
        }
    }
}

/// Trait for sensor fusion algorithms.
pub trait SensorFusion: Send + Sync {
    /// Configure the fusion algorithm.
    fn configure_fusion(&mut self, config: SensorFusionConfig);

    /// Get the current fused value.
    fn get_fused_value(&self) -> Option<f32>;

    /// Update with new sensor readings.
    fn update(&mut self, primary: Option<f32>, secondary: Option<f32>);

    /// Get current diagnostic information.
    fn get_diagnostics(&self) -> FusionDiagnostics;

    /// Reset the fusion state.
    fn reset(&mut self);
}

/// T133: Complementary filter implementation for cadence fusion.
///
/// Combines two cadence sources using a weighted complementary filter
/// with dropout detection and automatic fallback.
pub struct CadenceFusion {
    /// Fusion configuration
    config: SensorFusionConfig,
    /// Last primary sensor reading
    primary_value: Option<f32>,
    /// Last secondary sensor reading
    secondary_value: Option<f32>,
    /// Timestamp of last primary reading
    primary_timestamp: Option<Instant>,
    /// Timestamp of last secondary reading
    secondary_timestamp: Option<Instant>,
    /// Current fused cadence value
    fused_value: Option<f32>,
    /// Window of recent fused values for smoothing
    value_window: VecDeque<f32>,
    /// Current fusion mode
    mode: FusionMode,
}

impl CadenceFusion {
    /// Create a new cadence fusion instance.
    pub fn new() -> Self {
        Self {
            config: SensorFusionConfig::default(),
            primary_value: None,
            secondary_value: None,
            primary_timestamp: None,
            secondary_timestamp: None,
            fused_value: None,
            value_window: VecDeque::new(),
            mode: FusionMode::NoData,
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: SensorFusionConfig) -> Self {
        let mut fusion = Self::new();
        fusion.configure_fusion(config);
        fusion
    }

    /// Check if a sensor has dropped out based on timeout.
    fn is_dropped_out(&self, timestamp: Option<Instant>) -> bool {
        match timestamp {
            Some(ts) => ts.elapsed() > Duration::from_millis(self.config.dropout_timeout_ms as u64),
            None => true,
        }
    }

    /// Calculate deviation between two values as a percentage.
    fn calculate_deviation(&self, a: f32, b: f32) -> f32 {
        if a == 0.0 && b == 0.0 {
            return 0.0;
        }
        let max_val = a.max(b);
        if max_val == 0.0 {
            return 0.0;
        }
        ((a - b).abs() / max_val) * 100.0
    }

    /// Apply complementary filter to combine values.
    fn apply_complementary_filter(&self, primary: f32, secondary: f32) -> f32 {
        let mut config = self.config.clone();
        config.normalize_weights();
        config.primary_weight * primary + config.secondary_weight * secondary
    }

    /// Apply smoothing to the fused value using a moving average.
    fn apply_smoothing(&mut self, new_value: f32) -> f32 {
        // Add to window
        self.value_window.push_back(new_value);

        // Maintain window size
        while self.value_window.len() > self.config.window_size {
            self.value_window.pop_front();
        }

        // Calculate exponential moving average
        if self.value_window.is_empty() {
            return new_value;
        }

        let alpha = self.config.smoothing_factor;
        let mut ema = self.value_window[0];
        for &val in self.value_window.iter().skip(1) {
            ema = alpha * val + (1.0 - alpha) * ema;
        }
        ema
    }

    /// Determine the current fusion mode.
    fn determine_mode(&self) -> FusionMode {
        let primary_active = !self.is_dropped_out(self.primary_timestamp);
        let secondary_active = !self.is_dropped_out(self.secondary_timestamp);

        match (primary_active, secondary_active) {
            (true, true) => {
                // Check for inconsistency
                if let (Some(p), Some(s)) = (self.primary_value, self.secondary_value) {
                    if self.calculate_deviation(p, s) > self.config.max_deviation_percent {
                        return FusionMode::Inconsistent;
                    }
                }
                FusionMode::DualSensor
            }
            (true, false) => FusionMode::PrimaryOnly,
            (false, true) => FusionMode::SecondaryOnly,
            (false, false) => FusionMode::NoData,
        }
    }
}

impl Default for CadenceFusion {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorFusion for CadenceFusion {
    fn configure_fusion(&mut self, config: SensorFusionConfig) {
        self.config = config;
        // Resize window if needed
        while self.value_window.len() > self.config.window_size {
            self.value_window.pop_front();
        }
    }

    fn get_fused_value(&self) -> Option<f32> {
        self.fused_value
    }

    fn update(&mut self, primary: Option<f32>, secondary: Option<f32>) {
        let now = Instant::now();

        // Update primary sensor state
        if let Some(val) = primary {
            self.primary_value = Some(val);
            self.primary_timestamp = Some(now);
        }

        // Update secondary sensor state
        if let Some(val) = secondary {
            self.secondary_value = Some(val);
            self.secondary_timestamp = Some(now);
        }

        // Determine current mode
        self.mode = self.determine_mode();

        // Calculate fused value based on mode
        let raw_fused = match self.mode {
            FusionMode::DualSensor => {
                if let (Some(p), Some(s)) = (self.primary_value, self.secondary_value) {
                    Some(self.apply_complementary_filter(p, s))
                } else {
                    None
                }
            }
            FusionMode::PrimaryOnly if self.config.auto_fallback => self.primary_value,
            FusionMode::SecondaryOnly if self.config.auto_fallback => self.secondary_value,
            FusionMode::Inconsistent => {
                // When inconsistent, use primary if auto_fallback, otherwise average
                if self.config.auto_fallback {
                    self.primary_value
                } else if let (Some(p), Some(s)) = (self.primary_value, self.secondary_value) {
                    Some((p + s) / 2.0)
                } else {
                    None
                }
            }
            FusionMode::NoData => None,
            _ => None,
        };

        // Apply smoothing if we have a value
        if let Some(raw) = raw_fused {
            self.fused_value = Some(self.apply_smoothing(raw));
        } else if self.mode == FusionMode::NoData {
            // Clear fused value when no data
            self.fused_value = None;
        }
    }

    fn get_diagnostics(&self) -> FusionDiagnostics {
        let now = Instant::now();

        let primary_age = self
            .primary_timestamp
            .map(|ts| now.duration_since(ts).as_millis() as u32)
            .unwrap_or(u32::MAX);

        let secondary_age = self
            .secondary_timestamp
            .map(|ts| now.duration_since(ts).as_millis() as u32)
            .unwrap_or(u32::MAX);

        let deviation = match (self.primary_value, self.secondary_value) {
            (Some(p), Some(s)) => Some(self.calculate_deviation(p, s)),
            _ => None,
        };

        let sensors_agree = deviation
            .map(|d| d <= self.config.max_deviation_percent)
            .unwrap_or(false);

        FusionDiagnostics {
            primary_active: !self.is_dropped_out(self.primary_timestamp),
            secondary_active: !self.is_dropped_out(self.secondary_timestamp),
            primary_value: self.primary_value,
            secondary_value: self.secondary_value,
            fused_value: self.fused_value,
            primary_age_ms: primary_age,
            secondary_age_ms: secondary_age,
            deviation_percent: deviation,
            sensors_agree,
            mode: self.mode,
            sample_count: self.value_window.len(),
        }
    }

    fn reset(&mut self) {
        self.primary_value = None;
        self.secondary_value = None;
        self.primary_timestamp = None;
        self.secondary_timestamp = None;
        self.fused_value = None;
        self.value_window.clear();
        self.mode = FusionMode::NoData;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_fusion_config_default() {
        let config = SensorFusionConfig::default();
        assert_eq!(config.primary_weight, 0.6);
        assert_eq!(config.secondary_weight, 0.4);
        assert!(config.auto_fallback);
    }

    #[test]
    fn test_fusion_config_normalize() {
        let mut config = SensorFusionConfig {
            primary_weight: 3.0,
            secondary_weight: 1.0,
            ..Default::default()
        };
        config.normalize_weights();
        assert!((config.primary_weight - 0.75).abs() < 0.001);
        assert!((config.secondary_weight - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_cadence_fusion_new() {
        let fusion = CadenceFusion::new();
        assert!(fusion.get_fused_value().is_none());
        assert_eq!(fusion.mode, FusionMode::NoData);
    }

    #[test]
    fn test_cadence_fusion_dual_sensor() {
        let mut fusion = CadenceFusion::new();
        fusion.update(Some(90.0), Some(90.0));

        let fused = fusion.get_fused_value();
        assert!(fused.is_some());
        // With equal inputs, fused should be close to 90
        assert!((fused.unwrap() - 90.0).abs() < 1.0);

        let diag = fusion.get_diagnostics();
        assert!(diag.primary_active);
        assert!(diag.secondary_active);
        assert_eq!(diag.mode, FusionMode::DualSensor);
    }

    #[test]
    fn test_cadence_fusion_weighted() {
        let config = SensorFusionConfig {
            primary_weight: 0.8,
            secondary_weight: 0.2,
            smoothing_factor: 0.0, // No smoothing for predictable test
            window_size: 1,
            ..Default::default()
        };
        let mut fusion = CadenceFusion::with_config(config);

        // Primary=100, Secondary=80, weights 0.8/0.2
        // Expected: 0.8*100 + 0.2*80 = 80 + 16 = 96
        fusion.update(Some(100.0), Some(80.0));

        let fused = fusion.get_fused_value().unwrap();
        assert!((fused - 96.0).abs() < 0.1);
    }

    #[test]
    fn test_cadence_fusion_primary_fallback() {
        let mut fusion = CadenceFusion::new();

        // Only primary data
        fusion.update(Some(85.0), None);

        let diag = fusion.get_diagnostics();
        assert!(diag.primary_active);
        assert!(!diag.secondary_active);
        assert_eq!(diag.mode, FusionMode::PrimaryOnly);

        // With auto_fallback, should use primary value
        assert!(fusion.get_fused_value().is_some());
    }

    #[test]
    fn test_cadence_fusion_secondary_fallback() {
        let mut fusion = CadenceFusion::new();

        // Only secondary data
        fusion.update(None, Some(75.0));

        let diag = fusion.get_diagnostics();
        assert!(!diag.primary_active);
        assert!(diag.secondary_active);
        assert_eq!(diag.mode, FusionMode::SecondaryOnly);

        // With auto_fallback, should use secondary value
        assert!(fusion.get_fused_value().is_some());
    }

    #[test]
    fn test_cadence_fusion_dropout_detection() {
        let config = SensorFusionConfig {
            dropout_timeout_ms: 50, // Very short timeout for testing
            ..Default::default()
        };
        let mut fusion = CadenceFusion::with_config(config);

        // Both sensors active
        fusion.update(Some(90.0), Some(90.0));
        assert_eq!(fusion.get_diagnostics().mode, FusionMode::DualSensor);

        // Wait for dropout
        sleep(Duration::from_millis(60));

        // Update with only primary
        fusion.update(Some(90.0), None);
        assert_eq!(fusion.get_diagnostics().mode, FusionMode::PrimaryOnly);
    }

    #[test]
    fn test_cadence_fusion_inconsistency() {
        let config = SensorFusionConfig {
            max_deviation_percent: 10.0,
            ..Default::default()
        };
        let mut fusion = CadenceFusion::with_config(config);

        // Values differ by more than 10%
        fusion.update(Some(100.0), Some(70.0)); // 30% difference

        let diag = fusion.get_diagnostics();
        assert_eq!(diag.mode, FusionMode::Inconsistent);
        assert!(!diag.sensors_agree);
    }

    #[test]
    fn test_cadence_fusion_reset() {
        let mut fusion = CadenceFusion::new();
        fusion.update(Some(90.0), Some(90.0));
        assert!(fusion.get_fused_value().is_some());

        fusion.reset();
        assert!(fusion.get_fused_value().is_none());
        assert_eq!(fusion.get_diagnostics().mode, FusionMode::NoData);
    }

    #[test]
    fn test_fusion_mode_descriptions() {
        assert_eq!(FusionMode::DualSensor.description(), "Both sensors active");
        assert_eq!(FusionMode::PrimaryOnly.description(), "Primary sensor only");
        assert_eq!(FusionMode::NoData.description(), "No sensor data");
    }
}
