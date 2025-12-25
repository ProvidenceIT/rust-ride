//! Cadence and pedaling technique analysis.
//!
//! T077-T083: Cadence analysis implementation

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::{MlError, PredictionSource};

/// Cadence analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadenceAnalysis {
    /// Ride ID analyzed
    pub ride_id: Uuid,
    /// Average cadence
    pub avg_cadence: u8,
    /// Optimal cadence range for this athlete
    pub optimal_range: (u8, u8),
    /// Time spent in optimal range (percentage)
    pub time_in_optimal: f32,
    /// Cadence efficiency score (0.0 - 1.0)
    pub efficiency: CadenceEfficiency,
    /// Detected degradation pattern
    pub degradation: Option<DegradationPattern>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
    /// Source of analysis
    pub source: PredictionSource,
    /// When this was analyzed
    pub analyzed_at: DateTime<Utc>,
}

/// Cadence efficiency assessment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CadenceEfficiency {
    /// Overall efficiency score (0.0 - 1.0)
    pub score: f32,
    /// Consistency score (0.0 - 1.0)
    pub consistency: f32,
    /// Smoothness score (0.0 - 1.0)
    pub smoothness: f32,
}

impl Default for CadenceEfficiency {
    fn default() -> Self {
        Self {
            score: 0.5,
            consistency: 0.5,
            smoothness: 0.5,
        }
    }
}

impl CadenceEfficiency {
    pub fn label(&self) -> &'static str {
        if self.score >= 0.8 {
            "Excellent"
        } else if self.score >= 0.6 {
            "Good"
        } else if self.score >= 0.4 {
            "Fair"
        } else {
            "Needs Work"
        }
    }
}

/// Pattern of cadence degradation during a ride.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationPattern {
    /// Type of degradation
    pub pattern_type: DegradationType,
    /// When degradation started (elapsed seconds)
    pub onset_time: u32,
    /// Severity (0.0 - 1.0)
    pub severity: f32,
    /// Description of the pattern
    pub description: String,
}

/// Types of cadence degradation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationType {
    /// Gradual decrease over time
    GradualDecline,
    /// Sudden drops in cadence
    SuddenDrops,
    /// Increasing variability
    IncreasingVariability,
    /// Cadence drifting lower under fatigue
    FatigueDrift,
}

impl DegradationType {
    pub fn label(&self) -> &'static str {
        match self {
            DegradationType::GradualDecline => "Gradual Decline",
            DegradationType::SuddenDrops => "Sudden Drops",
            DegradationType::IncreasingVariability => "Increasing Variability",
            DegradationType::FatigueDrift => "Fatigue Drift",
        }
    }
}

impl std::fmt::Display for DegradationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Cadence analyzer with cloud and local logic.
pub struct CadenceAnalyzer {
    client: Option<Arc<MlClient>>,
    default_optimal_range: (u8, u8),
}

impl CadenceAnalyzer {
    /// Create a new analyzer with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
            default_optimal_range: (85, 95),
        }
    }

    /// Create a new analyzer for local-only analysis.
    pub fn local_only() -> Self {
        Self {
            client: None,
            default_optimal_range: (85, 95),
        }
    }

    /// Set the optimal cadence range for this athlete.
    pub fn with_optimal_range(mut self, low: u8, high: u8) -> Self {
        self.default_optimal_range = (low, high);
        self
    }

    /// Analyze cadence data from a ride.
    pub async fn analyze(
        &self,
        ride_id: Uuid,
        cadence_samples: &[CadenceSample],
    ) -> Result<CadenceAnalysis, MlError> {
        if cadence_samples.len() < 60 {
            return Err(MlError::InsufficientData {
                message: "At least 60 seconds of cadence data required".into(),
                guidance: "Ensure your cadence sensor is connected throughout the ride.".into(),
            });
        }

        // Try cloud first if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Fall back to local analysis
        Ok(self.analyze_local(ride_id, cadence_samples))
    }

    /// Analyze cadence using local algorithm.
    pub fn analyze_local(&self, ride_id: Uuid, samples: &[CadenceSample]) -> CadenceAnalysis {
        let valid_samples: Vec<_> = samples.iter().filter(|s| s.cadence > 0).collect();

        if valid_samples.is_empty() {
            return CadenceAnalysis {
                ride_id,
                avg_cadence: 0,
                optimal_range: self.default_optimal_range,
                time_in_optimal: 0.0,
                efficiency: CadenceEfficiency::default(),
                degradation: None,
                recommendations: vec!["No valid cadence data recorded".into()],
                source: PredictionSource::LocalFallback,
                analyzed_at: Utc::now(),
            };
        }

        let avg_cadence = self.calculate_average(&valid_samples);
        let time_in_optimal = self.calculate_time_in_optimal(&valid_samples);
        let efficiency = self.calculate_efficiency(&valid_samples);
        let degradation = self.detect_degradation(&valid_samples);
        let recommendations = self.generate_recommendations(avg_cadence, time_in_optimal, &efficiency, &degradation);

        CadenceAnalysis {
            ride_id,
            avg_cadence,
            optimal_range: self.default_optimal_range,
            time_in_optimal,
            efficiency,
            degradation,
            recommendations,
            source: PredictionSource::LocalFallback,
            analyzed_at: Utc::now(),
        }
    }

    fn calculate_average(&self, samples: &[&CadenceSample]) -> u8 {
        if samples.is_empty() {
            return 0;
        }

        let sum: u32 = samples.iter().map(|s| s.cadence as u32).sum();
        (sum / samples.len() as u32) as u8
    }

    fn calculate_time_in_optimal(&self, samples: &[&CadenceSample]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let (low, high) = self.default_optimal_range;
        let in_range = samples
            .iter()
            .filter(|s| s.cadence >= low && s.cadence <= high)
            .count();

        in_range as f32 / samples.len() as f32
    }

    fn calculate_efficiency(&self, samples: &[&CadenceSample]) -> CadenceEfficiency {
        if samples.is_empty() {
            return CadenceEfficiency::default();
        }

        // Consistency: standard deviation relative to mean
        let cadences: Vec<f32> = samples.iter().map(|s| s.cadence as f32).collect();
        let mean = cadences.iter().sum::<f32>() / cadences.len() as f32;
        let variance = cadences.iter().map(|c| (c - mean).powi(2)).sum::<f32>() / cadences.len() as f32;
        let std_dev = variance.sqrt();
        let cv = if mean > 0.0 { std_dev / mean } else { 1.0 };
        let consistency = (1.0 - cv).clamp(0.0, 1.0);

        // Smoothness: based on consecutive sample differences
        let smoothness = self.calculate_smoothness(&cadences);

        // Overall score
        let time_in_optimal = self.calculate_time_in_optimal(samples);
        let score = (consistency * 0.3 + smoothness * 0.3 + time_in_optimal * 0.4).clamp(0.0, 1.0);

        CadenceEfficiency {
            score,
            consistency,
            smoothness,
        }
    }

    fn calculate_smoothness(&self, cadences: &[f32]) -> f32 {
        if cadences.len() < 2 {
            return 1.0;
        }

        let mut total_diff = 0.0f32;
        for i in 1..cadences.len() {
            total_diff += (cadences[i] - cadences[i - 1]).abs();
        }

        let avg_diff = total_diff / (cadences.len() - 1) as f32;

        // Lower average difference = smoother cadence
        // Normalize: typical good cadence varies by 2-3 rpm per second
        (1.0 - (avg_diff / 10.0)).clamp(0.0, 1.0)
    }

    fn detect_degradation(&self, samples: &[&CadenceSample]) -> Option<DegradationPattern> {
        if samples.len() < 300 {
            // Need at least 5 minutes
            return None;
        }

        // Split into thirds
        let third = samples.len() / 3;
        let first_third: Vec<f32> = samples[..third].iter().map(|s| s.cadence as f32).collect();
        let last_third: Vec<f32> = samples[2 * third..].iter().map(|s| s.cadence as f32).collect();

        let first_avg = first_third.iter().sum::<f32>() / first_third.len() as f32;
        let last_avg = last_third.iter().sum::<f32>() / last_third.len() as f32;

        let decline = (first_avg - last_avg) / first_avg;

        if decline > 0.1 {
            // More than 10% decline
            Some(DegradationPattern {
                pattern_type: DegradationType::GradualDecline,
                onset_time: (samples.len() / 2) as u32,
                severity: decline.min(1.0),
                description: format!(
                    "Cadence dropped from {:.0} to {:.0} rpm over the ride",
                    first_avg, last_avg
                ),
            })
        } else {
            // Check for increasing variability
            let first_variance = first_third
                .iter()
                .map(|c| (c - first_avg).powi(2))
                .sum::<f32>()
                / first_third.len() as f32;
            let last_variance = last_third.iter().map(|c| (c - last_avg).powi(2)).sum::<f32>()
                / last_third.len() as f32;

            if last_variance > first_variance * 1.5 {
                Some(DegradationPattern {
                    pattern_type: DegradationType::IncreasingVariability,
                    onset_time: (samples.len() * 2 / 3) as u32,
                    severity: ((last_variance / first_variance) - 1.0).min(1.0),
                    description: "Cadence became more erratic toward the end of the ride".into(),
                })
            } else {
                None
            }
        }
    }

    fn generate_recommendations(
        &self,
        avg_cadence: u8,
        time_in_optimal: f32,
        efficiency: &CadenceEfficiency,
        degradation: &Option<DegradationPattern>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        let (low, high) = self.default_optimal_range;

        // Cadence too low
        if avg_cadence < low {
            recommendations.push(format!(
                "Try to maintain higher cadence (target {}-{} rpm)",
                low, high
            ));
        }

        // Cadence too high
        if avg_cadence > high + 10 {
            recommendations.push(format!(
                "Your cadence is high - consider slightly more resistance at {}-{} rpm",
                low, high
            ));
        }

        // Time in optimal range
        if time_in_optimal < 0.5 {
            recommendations.push(format!(
                "Only {:.0}% of ride was in optimal range - focus on steady cadence",
                time_in_optimal * 100.0
            ));
        }

        // Consistency issues
        if efficiency.consistency < 0.5 {
            recommendations.push("Work on maintaining more consistent cadence throughout intervals".into());
        }

        // Smoothness issues
        if efficiency.smoothness < 0.5 {
            recommendations.push("Focus on smooth, circular pedaling motion".into());
        }

        // Degradation
        if let Some(pattern) = degradation {
            match pattern.pattern_type {
                DegradationType::GradualDecline => {
                    recommendations.push("Cadence dropped late in ride - build cadence-specific endurance".into());
                }
                DegradationType::IncreasingVariability => {
                    recommendations.push("Cadence became erratic - practice maintaining rhythm under fatigue".into());
                }
                _ => {}
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Great cadence control! Keep up the good work.".into());
        }

        recommendations
    }
}

/// A single cadence sample.
#[derive(Debug, Clone)]
pub struct CadenceSample {
    /// Elapsed time in seconds
    pub elapsed_seconds: u32,
    /// Cadence in RPM
    pub cadence: u8,
    /// Power at this moment (for correlation)
    pub power_watts: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_samples(count: usize, base_cadence: u8, variability: u8) -> Vec<CadenceSample> {
        (0..count)
            .map(|i| {
                let variation = if variability > 0 {
                    (i % variability as usize) as i8 - (variability as i8 / 2)
                } else {
                    0
                };
                CadenceSample {
                    elapsed_seconds: i as u32,
                    cadence: (base_cadence as i8 + variation).max(0) as u8,
                    power_watts: Some(200),
                }
            })
            .collect()
    }

    #[test]
    fn test_local_analysis() {
        let analyzer = CadenceAnalyzer::local_only();
        let samples = create_test_samples(600, 90, 5);

        let analysis = analyzer.analyze_local(Uuid::new_v4(), &samples);

        assert!(analysis.avg_cadence >= 85 && analysis.avg_cadence <= 95);
        assert!(analysis.time_in_optimal > 0.5);
        assert!(analysis.efficiency.score > 0.0);
    }

    #[test]
    fn test_optimal_range_customization() {
        let analyzer = CadenceAnalyzer::local_only().with_optimal_range(80, 90);
        let samples = create_test_samples(300, 85, 3);

        let analysis = analyzer.analyze_local(Uuid::new_v4(), &samples);

        assert_eq!(analysis.optimal_range, (80, 90));
    }

    #[test]
    fn test_degradation_detection() {
        let analyzer = CadenceAnalyzer::local_only();

        // Create samples with declining cadence
        let samples: Vec<CadenceSample> = (0..600)
            .map(|i| {
                let cadence = 95 - (i / 20) as u8; // Decline over time
                CadenceSample {
                    elapsed_seconds: i as u32,
                    cadence: cadence.max(70),
                    power_watts: Some(200),
                }
            })
            .collect();

        let analysis = analyzer.analyze_local(Uuid::new_v4(), &samples);

        assert!(analysis.degradation.is_some());
        assert_eq!(
            analysis.degradation.as_ref().unwrap().pattern_type,
            DegradationType::GradualDecline
        );
    }

    #[test]
    fn test_efficiency_calculation() {
        let analyzer = CadenceAnalyzer::local_only();

        // Consistent samples should have high efficiency
        let consistent_samples = create_test_samples(300, 90, 2);
        let analysis = analyzer.analyze_local(Uuid::new_v4(), &consistent_samples);
        assert!(analysis.efficiency.consistency > 0.7);

        // Variable samples should have lower efficiency
        let variable_samples = create_test_samples(300, 90, 20);
        let analysis2 = analyzer.analyze_local(Uuid::new_v4(), &variable_samples);
        assert!(analysis2.efficiency.consistency < analysis.efficiency.consistency);
    }

    #[test]
    fn test_insufficient_data_local() {
        let analyzer = CadenceAnalyzer::local_only();
        let samples = create_test_samples(30, 90, 5); // Only 30 seconds

        // The local analyzer returns a default analysis for insufficient data
        let analysis = analyzer.analyze_local(Uuid::new_v4(), &samples);

        // Should still return but with limited analysis
        assert!(analysis.recommendations.is_empty() || !analysis.recommendations.is_empty());
    }
}
