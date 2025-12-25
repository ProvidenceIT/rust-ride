//! Performance trend forecasting.
//!
//! T053-T064: Performance forecasting implementation

use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::{MlError, PredictionSource};
use crate::goals::types::TrainingGoal;
use crate::metrics::analytics::DailyLoad;

/// Performance projection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProjection {
    /// Unique ID
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// When this projection was created
    pub projected_at: DateTime<Utc>,
    /// Number of weeks forecasted
    pub forecast_weeks: u8,
    /// Projected data points
    pub data_points: Vec<ProjectedCtl>,
    /// Overall trend direction
    pub trend: TrendDirection,
    /// Trend slope (CTL change per day)
    pub slope: f32,
    /// Whether a plateau was detected
    pub plateau_detected: bool,
    /// Detraining risk assessment
    pub detraining_risk: DetrainingRisk,
    /// Event readiness (if goal set)
    pub event_readiness: Option<EventReadiness>,
    /// Source of projection
    pub source: PredictionSource,
}

/// A single projected CTL data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedCtl {
    /// Date of projection
    pub date: NaiveDate,
    /// Projected CTL value
    pub projected_ctl: f32,
    /// Lower confidence bound
    pub confidence_low: f32,
    /// Upper confidence bound
    pub confidence_high: f32,
}

/// Trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
}

impl TrendDirection {
    pub fn label(&self) -> &'static str {
        match self {
            TrendDirection::Improving => "Improving",
            TrendDirection::Stable => "Stable",
            TrendDirection::Declining => "Declining",
        }
    }
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Detraining risk level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetrainingRisk {
    None,
    Low,
    Medium,
    High,
}

impl DetrainingRisk {
    pub fn label(&self) -> &'static str {
        match self {
            DetrainingRisk::None => "None",
            DetrainingRisk::Low => "Low",
            DetrainingRisk::Medium => "Medium",
            DetrainingRisk::High => "High",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DetrainingRisk::None => "Training frequency is good",
            DetrainingRisk::Low => "Minor fitness loss possible without increased training",
            DetrainingRisk::Medium => "Fitness loss expected if training pattern continues",
            DetrainingRisk::High => "Significant fitness loss imminent - increase training",
        }
    }
}

impl std::fmt::Display for DetrainingRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Event readiness assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReadiness {
    /// Associated goal ID
    pub goal_id: Uuid,
    /// Target CTL for the event
    pub target_ctl: f32,
    /// Projected CTL at event date
    pub projected_ctl_at_event: f32,
    /// Gap between target and projected (negative = behind)
    pub gap: f32,
    /// Whether currently on track
    pub on_track: bool,
    /// Recommendation to close the gap
    pub recommendation: String,
}

/// Performance forecaster.
pub struct PerformanceForecaster {
    client: Option<Arc<MlClient>>,
}

impl PerformanceForecaster {
    /// Create a new forecaster with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
        }
    }

    /// Create a new forecaster for local-only projections.
    pub fn local_only() -> Self {
        Self { client: None }
    }

    /// Generate CTL forecast.
    pub async fn forecast(
        &self,
        user_id: Uuid,
        ctl_history: &[(NaiveDate, DailyLoad)],
        forecast_weeks: u8,
        target_event: Option<&TrainingGoal>,
    ) -> Result<PerformanceProjection, MlError> {
        if ctl_history.len() < 28 {
            return Err(MlError::InsufficientData {
                message: "At least 4 weeks of training data required".into(),
                guidance: "Continue training consistently to build enough history for forecasting."
                    .into(),
            });
        }

        // Try cloud first if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Local EWMA projection
        Ok(self.forecast_local(user_id, ctl_history, forecast_weeks, target_event))
    }

    /// Generate local forecast using EWMA projection.
    pub fn forecast_local(
        &self,
        user_id: Uuid,
        ctl_history: &[(NaiveDate, DailyLoad)],
        forecast_weeks: u8,
        target_event: Option<&TrainingGoal>,
    ) -> PerformanceProjection {
        // Calculate trend from recent 30 days (keep chronological order for correct slope)
        let start_idx = ctl_history.len().saturating_sub(30);
        let recent: Vec<_> = ctl_history[start_idx..].iter().collect();
        let slope = self.calculate_slope(&recent);
        let trend = self.classify_trend(slope);
        let plateau_detected = self.detect_plateau(&recent);

        // Get current CTL
        let current_ctl = ctl_history.last().map(|(_, l)| l.ctl).unwrap_or(0.0);
        let today = Utc::now().date_naive();

        // Generate projection points (weekly)
        let mut data_points = Vec::new();
        for week in 1..=forecast_weeks {
            let target_date = today + Duration::days(week as i64 * 7);
            let days_ahead = week as f32 * 7.0;

            // Simple linear projection with decay towards mean
            let projected = current_ctl + (slope * days_ahead);

            // Confidence interval widens with time
            let uncertainty = 0.1 * days_ahead / 7.0; // 10% per week
            let confidence_low = projected * (1.0 - uncertainty);
            let confidence_high = projected * (1.0 + uncertainty);

            data_points.push(ProjectedCtl {
                date: target_date,
                projected_ctl: projected.max(0.0),
                confidence_low: confidence_low.max(0.0),
                confidence_high,
            });
        }

        // Assess detraining risk
        let detraining_risk = self.assess_detraining_risk(ctl_history);

        // Event readiness if goal set
        let event_readiness = target_event.and_then(|goal| {
            goal.target_date.map(|event_date| {
                let days_to_event = (event_date - today).num_days();
                let projected_at_event = current_ctl + (slope * days_to_event as f32);
                let target_ctl = goal
                    .target_metric
                    .as_ref()
                    .map(|m| m.target_value)
                    .unwrap_or(current_ctl * 1.1);

                let gap = projected_at_event - target_ctl;
                let on_track = gap >= 0.0;

                let recommendation = if on_track {
                    "You're on track to meet your target fitness!".to_string()
                } else {
                    let weekly_increase_needed = -gap / (days_to_event as f32 / 7.0);
                    format!(
                        "Increase weekly TSS by {:.0}% to reach your target",
                        (weekly_increase_needed / current_ctl * 100.0).abs().min(30.0)
                    )
                };

                EventReadiness {
                    goal_id: goal.id,
                    target_ctl,
                    projected_ctl_at_event: projected_at_event,
                    gap,
                    on_track,
                    recommendation,
                }
            })
        });

        PerformanceProjection {
            id: Uuid::new_v4(),
            user_id,
            projected_at: Utc::now(),
            forecast_weeks,
            data_points,
            trend,
            slope,
            plateau_detected,
            detraining_risk,
            event_readiness,
            source: PredictionSource::LocalFallback,
        }
    }

    /// Detect if athlete is plateauing.
    pub fn detect_plateau(&self, recent_history: &[&(NaiveDate, DailyLoad)]) -> bool {
        if recent_history.len() < 14 {
            return false;
        }

        let slope = self.calculate_slope(recent_history);

        // Plateau if slope is very low and we have consistent training
        let has_consistent_training = recent_history
            .iter()
            .filter(|(_, l)| l.tss > 20.0)
            .count()
            >= recent_history.len() / 2;

        slope.abs() < 0.5 && has_consistent_training
    }

    /// Assess detraining risk based on recent activity.
    pub fn assess_detraining_risk(&self, ctl_history: &[(NaiveDate, DailyLoad)]) -> DetrainingRisk {
        if ctl_history.is_empty() {
            return DetrainingRisk::High;
        }

        let today = Utc::now().date_naive();
        let recent_week: Vec<_> = ctl_history
            .iter()
            .filter(|(date, _)| (*date - today).num_days().abs() <= 7)
            .collect();

        let training_days = recent_week.iter().filter(|(_, l)| l.tss > 30.0).count();
        let recent_trend = self.calculate_slope(
            &ctl_history
                .iter()
                .rev()
                .take(14)
                .collect::<Vec<_>>(),
        );

        if training_days == 0 || recent_trend < -1.0 {
            DetrainingRisk::High
        } else if training_days <= 2 || recent_trend < -0.5 {
            DetrainingRisk::Medium
        } else if training_days <= 3 || recent_trend < 0.0 {
            DetrainingRisk::Low
        } else {
            DetrainingRisk::None
        }
    }

    /// Calculate event readiness gap.
    pub fn event_gap(
        &self,
        projection: &PerformanceProjection,
        _goal: &TrainingGoal,
    ) -> Option<EventReadiness> {
        projection.event_readiness.clone()
    }

    fn calculate_slope(&self, history: &[&(NaiveDate, DailyLoad)]) -> f32 {
        if history.len() < 2 {
            return 0.0;
        }

        // Simple linear regression
        let n = history.len() as f32;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, (_, load)) in history.iter().enumerate() {
            let x = i as f32;
            let y = load.ctl;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.abs() < 0.001 {
            return 0.0;
        }

        (n * sum_xy - sum_x * sum_y) / denominator
    }

    fn classify_trend(&self, slope: f32) -> TrendDirection {
        if slope > 0.5 {
            TrendDirection::Improving
        } else if slope < -0.5 {
            TrendDirection::Declining
        } else {
            TrendDirection::Stable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_history(days: i64, trend: f32) -> Vec<(NaiveDate, DailyLoad)> {
        let today = Utc::now().date_naive();
        (0..days)
            .map(|i| {
                let date = today - Duration::days(days - i);
                let ctl = 50.0 + trend * i as f32;
                (
                    date,
                    DailyLoad {
                        tss: 60.0,
                        atl: ctl + 10.0,
                        ctl,
                        tsb: -10.0,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn test_improving_forecast() {
        let forecaster = PerformanceForecaster::local_only();
        let history = create_test_history(30, 1.0); // Clear improving trend (1 CTL/day)

        let projection = forecaster.forecast_local(Uuid::new_v4(), &history, 8, None);

        assert_eq!(projection.trend, TrendDirection::Improving);
        assert!(projection.slope > 0.5);
        assert!(!projection.data_points.is_empty());
    }

    #[test]
    fn test_declining_forecast() {
        let forecaster = PerformanceForecaster::local_only();
        let history = create_test_history(30, -0.8); // Declining trend

        let projection = forecaster.forecast_local(Uuid::new_v4(), &history, 8, None);

        assert_eq!(projection.trend, TrendDirection::Declining);
        assert!(projection.slope < 0.0);
    }

    #[test]
    fn test_plateau_detection() {
        let forecaster = PerformanceForecaster::local_only();
        let history = create_test_history(30, 0.0); // Flat trend
        let refs: Vec<_> = history.iter().collect();

        let plateau = forecaster.detect_plateau(&refs);
        assert!(plateau, "Flat trend should be detected as plateau");
    }

    #[test]
    fn test_detraining_risk_no_training() {
        let forecaster = PerformanceForecaster::local_only();

        // History with very old data only
        let history: Vec<(NaiveDate, DailyLoad)> = vec![];

        let risk = forecaster.assess_detraining_risk(&history);
        assert_eq!(risk, DetrainingRisk::High);
    }

    #[test]
    fn test_projection_confidence_intervals() {
        let forecaster = PerformanceForecaster::local_only();
        let history = create_test_history(30, 0.3);

        let projection = forecaster.forecast_local(Uuid::new_v4(), &history, 12, None);

        // Confidence intervals should widen over time
        if projection.data_points.len() >= 2 {
            let first = &projection.data_points[0];
            let last = &projection.data_points[projection.data_points.len() - 1];

            let first_range = first.confidence_high - first.confidence_low;
            let last_range = last.confidence_high - last.confidence_low;

            assert!(last_range > first_range, "Uncertainty should increase over time");
        }
    }
}
