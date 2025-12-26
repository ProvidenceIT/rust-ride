//! Real-time fatigue detection during rides.
//!
//! T031-T041: Fatigue detection implementation

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::MlError;

/// Result of fatigue analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueAnalysis {
    /// Aerobic decoupling score (HR drift percentage)
    pub aerobic_decoupling_score: f32,
    /// Power variability index
    pub power_variability_index: f32,
    /// HRV-based fatigue indicator (if available)
    pub hrv_indicator: Option<f32>,
    /// Whether an alert should be triggered
    pub alert_triggered: bool,
    /// Severity of fatigue
    pub severity: FatigueSeverity,
    /// Human-readable message
    pub message: String,
    /// Confidence in the analysis
    pub confidence: f32,
}

impl Default for FatigueAnalysis {
    fn default() -> Self {
        Self {
            aerobic_decoupling_score: 0.0,
            power_variability_index: 1.0,
            hrv_indicator: None,
            alert_triggered: false,
            severity: FatigueSeverity::None,
            message: String::new(),
            confidence: 0.0,
        }
    }
}

/// Severity level of fatigue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FatigueSeverity {
    None,
    Mild,
    Moderate,
    Severe,
}

impl FatigueSeverity {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            FatigueSeverity::None => "No Fatigue",
            FatigueSeverity::Mild => "Mild Fatigue",
            FatigueSeverity::Moderate => "Moderate Fatigue",
            FatigueSeverity::Severe => "Severe Fatigue",
        }
    }
}

impl std::fmt::Display for FatigueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Athlete baseline for fatigue comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleteBaseline {
    /// Resting heart rate
    pub resting_hr: u8,
    /// Maximum heart rate
    pub max_hr: u8,
    /// Typical aerobic decoupling percentage
    pub typical_decoupling: f32,
    /// Typical power variability
    pub typical_variability: f32,
}

impl Default for AthleteBaseline {
    fn default() -> Self {
        Self {
            resting_hr: 60,
            max_hr: 185,
            typical_decoupling: 0.05,
            typical_variability: 1.0,
        }
    }
}

/// State of fatigue alerts during a ride.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueState {
    /// Current ride ID
    pub ride_id: Uuid,
    /// Whether an alert is currently active
    pub alert_active: bool,
    /// Whether the current alert has been dismissed
    pub alert_dismissed: bool,
    /// When the alert was dismissed
    pub dismissed_at: Option<DateTime<Utc>>,
    /// When the cooldown expires (can alert again)
    pub cooldown_expires_at: Option<DateTime<Utc>>,
    /// Last analysis result
    pub last_analysis: Option<FatigueAnalysis>,
    /// Last analysis timestamp
    pub last_analyzed_at: Option<DateTime<Utc>>,
}

impl FatigueState {
    /// Create a new fatigue state for a ride.
    pub fn new(ride_id: Uuid) -> Self {
        Self {
            ride_id,
            alert_active: false,
            alert_dismissed: false,
            dismissed_at: None,
            cooldown_expires_at: None,
            last_analysis: None,
            last_analyzed_at: None,
        }
    }

    /// Check if currently in cooldown period.
    pub fn is_in_cooldown(&self) -> bool {
        if let Some(expires) = self.cooldown_expires_at {
            Utc::now() < expires
        } else {
            false
        }
    }

    /// Dismiss the current alert with cooldown.
    pub fn dismiss(&mut self, cooldown_minutes: u32) {
        self.alert_dismissed = true;
        self.dismissed_at = Some(Utc::now());
        self.cooldown_expires_at = Some(Utc::now() + Duration::minutes(cooldown_minutes as i64));
    }

    /// Reset alert state after cooldown.
    pub fn reset_if_cooldown_expired(&mut self) {
        if let Some(expires) = self.cooldown_expires_at {
            if Utc::now() >= expires {
                self.alert_dismissed = false;
                self.dismissed_at = None;
                self.cooldown_expires_at = None;
            }
        }
    }

    /// Update with new analysis.
    pub fn update(&mut self, analysis: FatigueAnalysis) {
        self.alert_active = analysis.alert_triggered;
        self.last_analysis = Some(analysis);
        self.last_analyzed_at = Some(Utc::now());
    }
}

/// Real-time fatigue detector.
pub struct FatigueDetector {
    client: Option<Arc<MlClient>>,
    baseline: AthleteBaseline,
    decoupling_threshold: f32,
    variability_threshold: f32,
}

impl FatigueDetector {
    /// Create a new fatigue detector with cloud client.
    pub fn new(client: Arc<MlClient>, baseline: AthleteBaseline) -> Self {
        Self {
            client: Some(client),
            baseline,
            decoupling_threshold: 0.10,  // 10% HR drift
            variability_threshold: 1.40, // 40% above baseline
        }
    }

    /// Create a new fatigue detector for local-only analysis.
    pub fn local_only(baseline: AthleteBaseline) -> Self {
        Self {
            client: None,
            baseline,
            decoupling_threshold: 0.10,
            variability_threshold: 1.40,
        }
    }

    /// Create a test detector with default baseline.
    pub fn new_test() -> Self {
        Self::local_only(AthleteBaseline::default())
    }

    /// Set detection thresholds.
    pub fn with_thresholds(mut self, decoupling: f32, variability: f32) -> Self {
        self.decoupling_threshold = decoupling;
        self.variability_threshold = variability;
        self
    }

    /// Analyze fatigue from ride samples.
    ///
    /// Requires at least 5 minutes (300 samples at 1Hz) of data.
    pub async fn analyze(
        &self,
        _ride_id: Uuid,
        samples: &[RideSample],
        target_power: Option<u16>,
    ) -> Result<FatigueAnalysis, MlError> {
        if samples.len() < 300 {
            return Ok(FatigueAnalysis::default());
        }

        // Try cloud analysis if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Fall back to local analysis
        Ok(self.analyze_local(samples, target_power))
    }

    /// Analyze fatigue using local algorithms only.
    pub fn analyze_local(
        &self,
        samples: &[RideSample],
        target_power: Option<u16>,
    ) -> FatigueAnalysis {
        if samples.len() < 300 {
            return FatigueAnalysis::default();
        }

        let decoupling = self.aerobic_decoupling(samples);
        let variability = self.power_variability_index(samples, target_power);

        let (severity, message) = self.classify_fatigue(decoupling, variability);
        let alert_triggered = self.should_alert_from_values(decoupling, variability);
        let confidence = self.calculate_confidence(samples);

        FatigueAnalysis {
            aerobic_decoupling_score: decoupling,
            power_variability_index: variability,
            hrv_indicator: None, // Would need RR intervals
            alert_triggered,
            severity,
            message,
            confidence,
        }
    }

    /// Calculate aerobic decoupling (HR drift vs constant power).
    pub fn aerobic_decoupling(&self, samples: &[RideSample]) -> f32 {
        if samples.len() < 300 {
            return 0.0;
        }

        // Split samples into first half and second half
        let mid = samples.len() / 2;
        let first_half = &samples[..mid];
        let second_half = &samples[mid..];

        // Calculate efficiency ratio (power / HR) for each half
        let first_efficiency = self.calculate_efficiency(first_half);
        let second_efficiency = self.calculate_efficiency(second_half);

        if first_efficiency == 0.0 {
            return 0.0;
        }

        // Decoupling is the decrease in efficiency
        (first_efficiency - second_efficiency) / first_efficiency
    }

    fn calculate_efficiency(&self, samples: &[RideSample]) -> f32 {
        let valid_samples: Vec<_> = samples
            .iter()
            .filter(|s| s.heart_rate_bpm.is_some() && s.power_watts.is_some())
            .collect();

        if valid_samples.is_empty() {
            return 0.0;
        }

        let total_power: u32 = valid_samples
            .iter()
            .filter_map(|s| s.power_watts)
            .map(|p| p as u32)
            .sum();

        let total_hr: u32 = valid_samples
            .iter()
            .filter_map(|s| s.heart_rate_bpm)
            .map(|hr| hr as u32)
            .sum();

        if total_hr == 0 {
            return 0.0;
        }

        total_power as f32 / total_hr as f32
    }

    /// Calculate power variability index.
    pub fn power_variability_index(
        &self,
        samples: &[RideSample],
        target_power: Option<u16>,
    ) -> f32 {
        let powers: Vec<f32> = samples
            .iter()
            .filter_map(|s| s.power_watts)
            .map(|p| p as f32)
            .collect();

        if powers.is_empty() {
            return 1.0;
        }

        let mean = powers.iter().sum::<f32>() / powers.len() as f32;
        let variance = powers.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / powers.len() as f32;
        let std_dev = variance.sqrt();

        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };

        // Compare to baseline or target
        let baseline_cv = if let Some(_target) = target_power {
            // If there's a target, lower variability is expected
            0.05
        } else {
            self.baseline.typical_variability * 0.1
        };

        if baseline_cv > 0.0 {
            cv / baseline_cv
        } else {
            cv
        }
    }

    /// Determine if an alert should be triggered.
    pub fn should_alert(&self, analysis: &FatigueAnalysis) -> bool {
        self.should_alert_from_values(
            analysis.aerobic_decoupling_score,
            analysis.power_variability_index,
        )
    }

    fn should_alert_from_values(&self, decoupling: f32, variability: f32) -> bool {
        decoupling > self.decoupling_threshold || variability > self.variability_threshold
    }

    fn classify_fatigue(&self, decoupling: f32, variability: f32) -> (FatigueSeverity, String) {
        let decoupling_severity = if decoupling > 0.15 {
            3
        } else if decoupling > 0.10 {
            2
        } else if decoupling > 0.05 {
            1
        } else {
            0
        };

        let variability_severity = if variability > 1.6 {
            3
        } else if variability > 1.4 {
            2
        } else if variability > 1.2 {
            1
        } else {
            0
        };

        let max_severity = decoupling_severity.max(variability_severity);

        match max_severity {
            3 => (
                FatigueSeverity::Severe,
                "Significant fatigue detected - consider reducing intensity or stopping".into(),
            ),
            2 => (
                FatigueSeverity::Moderate,
                "Moderate fatigue detected - heart rate drift indicates reduced efficiency".into(),
            ),
            1 => (
                FatigueSeverity::Mild,
                "Mild fatigue signs - monitor your effort level".into(),
            ),
            _ => (
                FatigueSeverity::None,
                "No significant fatigue detected".into(),
            ),
        }
    }

    fn calculate_confidence(&self, samples: &[RideSample]) -> f32 {
        // Confidence based on:
        // 1. Sample count (more = better)
        // 2. Heart rate availability
        // 3. Power data consistency

        let sample_factor = (samples.len() as f32 / 600.0).min(1.0); // Max at 10 min

        let hr_count = samples
            .iter()
            .filter(|s| s.heart_rate_bpm.is_some())
            .count();
        let hr_factor = hr_count as f32 / samples.len() as f32;

        let power_count = samples.iter().filter(|s| s.power_watts.is_some()).count();
        let power_factor = power_count as f32 / samples.len() as f32;

        sample_factor * 0.3 + hr_factor * 0.4 + power_factor * 0.3
    }
}

/// A single ride sample for fatigue analysis.
#[derive(Debug, Clone)]
pub struct RideSample {
    /// Elapsed time in seconds
    pub elapsed_seconds: u32,
    /// Power in watts
    pub power_watts: Option<u16>,
    /// Heart rate in BPM
    pub heart_rate_bpm: Option<u8>,
    /// Cadence in RPM
    pub cadence_rpm: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_samples(count: usize, with_drift: bool) -> Vec<RideSample> {
        (0..count)
            .map(|i| {
                let base_hr = 140;
                let hr_drift = if with_drift {
                    (i as f32 / count as f32 * 20.0) as u8 // Drift up to 20 bpm
                } else {
                    0
                };

                RideSample {
                    elapsed_seconds: i as u32,
                    power_watts: Some(200),
                    heart_rate_bpm: Some(base_hr + hr_drift),
                    cadence_rpm: Some(90),
                }
            })
            .collect()
    }

    #[test]
    fn test_fatigue_state_cooldown() {
        let mut state = FatigueState::new(Uuid::new_v4());

        assert!(!state.is_in_cooldown());

        state.dismiss(5);
        assert!(state.is_in_cooldown());
        assert!(state.alert_dismissed);
    }

    #[test]
    fn test_aerobic_decoupling_no_drift() {
        let detector = FatigueDetector::new_test();
        let samples = create_test_samples(600, false);

        let decoupling = detector.aerobic_decoupling(&samples);
        assert!(
            decoupling.abs() < 0.05,
            "No drift should have low decoupling"
        );
    }

    #[test]
    fn test_aerobic_decoupling_with_drift() {
        let detector = FatigueDetector::new_test();
        let samples = create_test_samples(600, true);

        let decoupling = detector.aerobic_decoupling(&samples);
        assert!(decoupling > 0.05, "HR drift should show decoupling");
    }

    #[test]
    fn test_analyze_local() {
        let detector = FatigueDetector::new_test();
        let samples = create_test_samples(600, true);

        let analysis = detector.analyze_local(&samples, Some(200));

        assert!(analysis.aerobic_decoupling_score > 0.0);
        assert!(analysis.confidence > 0.0);
    }

    #[test]
    fn test_insufficient_samples() {
        let detector = FatigueDetector::new_test();
        let samples = create_test_samples(100, false); // Too few

        let analysis = detector.analyze_local(&samples, None);

        assert_eq!(analysis.aerobic_decoupling_score, 0.0);
        assert!(!analysis.alert_triggered);
    }
}
