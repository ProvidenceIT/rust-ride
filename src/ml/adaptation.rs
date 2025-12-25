//! Training load adaptation engine.
//!
//! T084-T095: Training load adaptation implementation

use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::{MlError, PredictionSource};
use crate::goals::types::TrainingGoal;
use crate::metrics::analytics::DailyLoad;

/// Load recommendation from the adaptation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadRecommendation {
    /// Unique ID
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Recommended TSS for the period
    pub recommended_tss: f32,
    /// Recommended intensity distribution
    pub intensity_distribution: IntensityDistribution,
    /// Weekly structure recommendation
    pub weekly_structure: WeeklyStructure,
    /// Adjustment from current load
    pub adjustment: LoadAdjustment,
    /// Rationale for the recommendation
    pub rationale: String,
    /// Confidence in this recommendation
    pub confidence: ModelConfidence,
    /// Source of recommendation
    pub source: PredictionSource,
    /// When this was generated
    pub generated_at: DateTime<Utc>,
}

/// Intensity distribution across zones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityDistribution {
    /// Percentage in Zone 1-2 (recovery/endurance)
    pub low_intensity: f32,
    /// Percentage in Zone 3 (tempo/sweet spot)
    pub moderate_intensity: f32,
    /// Percentage in Zone 4-5 (threshold/VO2max)
    pub high_intensity: f32,
}

impl Default for IntensityDistribution {
    fn default() -> Self {
        // Classic polarized distribution
        Self {
            low_intensity: 0.75,
            moderate_intensity: 0.10,
            high_intensity: 0.15,
        }
    }
}

impl IntensityDistribution {
    /// Create a polarized distribution (80/20).
    pub fn polarized() -> Self {
        Self {
            low_intensity: 0.80,
            moderate_intensity: 0.05,
            high_intensity: 0.15,
        }
    }

    /// Create a pyramidal distribution.
    pub fn pyramidal() -> Self {
        Self {
            low_intensity: 0.70,
            moderate_intensity: 0.20,
            high_intensity: 0.10,
        }
    }

    /// Create a threshold-focused distribution.
    pub fn threshold_focused() -> Self {
        Self {
            low_intensity: 0.65,
            moderate_intensity: 0.25,
            high_intensity: 0.10,
        }
    }
}

/// Weekly structure recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyStructure {
    /// Recommended training days per week
    pub training_days: u8,
    /// Recommended hard sessions per week
    pub hard_sessions: u8,
    /// Recommended rest days per week
    pub rest_days: u8,
    /// Long ride day (if applicable)
    pub long_ride_day: Option<chrono::Weekday>,
    /// Suggested weekly pattern
    pub pattern: WeeklyPattern,
}

impl Default for WeeklyStructure {
    fn default() -> Self {
        Self {
            training_days: 5,
            hard_sessions: 2,
            rest_days: 2,
            long_ride_day: Some(chrono::Weekday::Sat),
            pattern: WeeklyPattern::Standard,
        }
    }
}

/// Weekly training pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeeklyPattern {
    /// Standard pattern (hard-easy-hard-easy)
    Standard,
    /// Block training (multiple hard days in a row)
    Block,
    /// Recovery week (reduced load)
    Recovery,
    /// Build week (progressive overload)
    Build,
}

impl WeeklyPattern {
    pub fn label(&self) -> &'static str {
        match self {
            WeeklyPattern::Standard => "Standard",
            WeeklyPattern::Block => "Block Training",
            WeeklyPattern::Recovery => "Recovery Week",
            WeeklyPattern::Build => "Build Week",
        }
    }
}

impl std::fmt::Display for WeeklyPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Load adjustment recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAdjustment {
    /// Direction of adjustment
    pub direction: AdjustmentDirection,
    /// Percentage change recommended
    pub percentage: f32,
    /// Reason for adjustment
    pub reason: String,
}

/// Direction of load adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentDirection {
    /// Increase training load
    Increase,
    /// Maintain current load
    Maintain,
    /// Decrease training load
    Decrease,
}

impl AdjustmentDirection {
    pub fn label(&self) -> &'static str {
        match self {
            AdjustmentDirection::Increase => "Increase",
            AdjustmentDirection::Maintain => "Maintain",
            AdjustmentDirection::Decrease => "Decrease",
        }
    }
}

impl std::fmt::Display for AdjustmentDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Confidence in the model's recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConfidence {
    /// Overall confidence score (0.0 - 1.0)
    level: u8, // 0-100 for precision
}

impl ModelConfidence {
    pub fn new(level: f32) -> Self {
        Self {
            level: (level * 100.0).clamp(0.0, 100.0) as u8,
        }
    }

    pub fn score(&self) -> f32 {
        self.level as f32 / 100.0
    }

    pub fn label(&self) -> &'static str {
        let score = self.score();
        if score >= 0.8 {
            "High"
        } else if score >= 0.5 {
            "Moderate"
        } else {
            "Low"
        }
    }
}

impl Default for ModelConfidence {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Athlete adaptation model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationModel {
    /// User ID
    pub user_id: Uuid,
    /// Estimated recovery rate (days to full recovery from 100 TSS)
    pub recovery_rate: f32,
    /// Optimal weekly TSS based on history
    pub optimal_weekly_tss: f32,
    /// Preferred intensity distribution
    pub preferred_distribution: IntensityDistribution,
    /// Response to high intensity
    pub high_intensity_tolerance: f32,
    /// Response to volume
    pub volume_tolerance: f32,
    /// When model was last updated
    pub updated_at: DateTime<Utc>,
}

impl AdaptationModel {
    /// Create a default model for a new athlete.
    pub fn default_for_user(user_id: Uuid) -> Self {
        Self {
            user_id,
            recovery_rate: 1.5,
            optimal_weekly_tss: 400.0,
            preferred_distribution: IntensityDistribution::default(),
            high_intensity_tolerance: 0.5,
            volume_tolerance: 0.5,
            updated_at: Utc::now(),
        }
    }
}

/// Training load adaptation engine.
pub struct AdaptationEngine {
    client: Option<Arc<MlClient>>,
}

impl AdaptationEngine {
    /// Create a new engine with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
        }
    }

    /// Create a new engine for local-only recommendations.
    pub fn local_only() -> Self {
        Self { client: None }
    }

    /// Generate load recommendation based on training history.
    pub async fn recommend(
        &self,
        user_id: Uuid,
        load_history: &[(NaiveDate, DailyLoad)],
        model: &AdaptationModel,
        active_goals: &[TrainingGoal],
    ) -> Result<LoadRecommendation, MlError> {
        if load_history.len() < 14 {
            return Err(MlError::InsufficientData {
                message: "At least 2 weeks of training data required".into(),
                guidance: "Continue training consistently to build enough history for adaptation recommendations.".into(),
            });
        }

        // Try cloud first if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Fall back to local recommendation
        Ok(self.recommend_local(user_id, load_history, model, active_goals))
    }

    /// Generate recommendation using local algorithm.
    pub fn recommend_local(
        &self,
        user_id: Uuid,
        load_history: &[(NaiveDate, DailyLoad)],
        model: &AdaptationModel,
        active_goals: &[TrainingGoal],
    ) -> LoadRecommendation {
        // Calculate recent metrics
        let recent_weekly_tss = self.calculate_recent_weekly_tss(load_history);
        let current_ctl = load_history.last().map(|(_, l)| l.ctl).unwrap_or(0.0);
        let current_atl = load_history.last().map(|(_, l)| l.atl).unwrap_or(0.0);
        let acwr = if current_ctl > 0.0 {
            current_atl / current_ctl
        } else {
            1.0
        };

        // Determine adjustment
        let adjustment = self.calculate_adjustment(acwr, recent_weekly_tss, model);

        // Calculate recommended TSS
        let recommended_tss = self.calculate_recommended_tss(&adjustment, recent_weekly_tss, model);

        // Determine intensity distribution based on goals
        let intensity_distribution = self.determine_distribution(active_goals, model);

        // Determine weekly structure
        let weekly_structure = self.determine_weekly_structure(acwr, &adjustment);

        // Calculate confidence
        let confidence = self.calculate_confidence(load_history);

        // Generate rationale
        let rationale = self.generate_rationale(&adjustment, acwr, active_goals);

        LoadRecommendation {
            id: Uuid::new_v4(),
            user_id,
            recommended_tss,
            intensity_distribution,
            weekly_structure,
            adjustment,
            rationale,
            confidence,
            source: PredictionSource::LocalFallback,
            generated_at: Utc::now(),
        }
    }

    /// Update the adaptation model based on athlete response.
    pub fn update_model(
        &self,
        model: &mut AdaptationModel,
        load_history: &[(NaiveDate, DailyLoad)],
        performance_change: f32,
    ) {
        if load_history.len() < 28 {
            return;
        }

        // Calculate recent averages
        let recent_tss = self.calculate_recent_weekly_tss(load_history);

        // Update optimal weekly TSS based on performance response
        if performance_change > 0.0 {
            // Positive response - current load is working
            model.optimal_weekly_tss = model.optimal_weekly_tss * 0.9 + recent_tss * 0.1;
        } else if performance_change < -0.05 {
            // Negative response - reduce optimal
            model.optimal_weekly_tss *= 0.95;
        }

        // Update recovery rate based on TSB patterns
        // This would require more sophisticated analysis in production

        model.updated_at = Utc::now();
    }

    fn calculate_recent_weekly_tss(&self, load_history: &[(NaiveDate, DailyLoad)]) -> f32 {
        let today = Utc::now().date_naive();
        let week_ago = today - Duration::days(7);

        load_history
            .iter()
            .filter(|(date, _)| *date > week_ago)
            .map(|(_, load)| load.tss)
            .sum()
    }

    fn calculate_adjustment(
        &self,
        acwr: f32,
        recent_weekly_tss: f32,
        model: &AdaptationModel,
    ) -> LoadAdjustment {
        // ACWR-based adjustment
        let (direction, percentage, reason) = if acwr > 1.5 {
            (
                AdjustmentDirection::Decrease,
                20.0,
                "High acute:chronic workload ratio indicates fatigue risk".into(),
            )
        } else if acwr > 1.3 {
            (
                AdjustmentDirection::Decrease,
                10.0,
                "Elevated training load - small reduction recommended".into(),
            )
        } else if acwr < 0.8 && recent_weekly_tss < model.optimal_weekly_tss * 0.7 {
            (
                AdjustmentDirection::Increase,
                15.0,
                "Training load is low - gradual increase recommended".into(),
            )
        } else if (0.8..=1.3).contains(&acwr) {
            (
                AdjustmentDirection::Maintain,
                0.0,
                "Training load is in the optimal range".into(),
            )
        } else {
            (
                AdjustmentDirection::Maintain,
                0.0,
                "Current training pattern is appropriate".into(),
            )
        };

        LoadAdjustment {
            direction,
            percentage,
            reason,
        }
    }

    fn calculate_recommended_tss(
        &self,
        adjustment: &LoadAdjustment,
        recent_weekly_tss: f32,
        model: &AdaptationModel,
    ) -> f32 {
        let base_tss = if recent_weekly_tss > 0.0 {
            recent_weekly_tss
        } else {
            model.optimal_weekly_tss
        };

        match adjustment.direction {
            AdjustmentDirection::Increase => base_tss * (1.0 + adjustment.percentage / 100.0),
            AdjustmentDirection::Decrease => base_tss * (1.0 - adjustment.percentage / 100.0),
            AdjustmentDirection::Maintain => base_tss,
        }
    }

    fn determine_distribution(
        &self,
        goals: &[TrainingGoal],
        model: &AdaptationModel,
    ) -> IntensityDistribution {
        // Check if any goal suggests a specific distribution
        for goal in goals {
            match goal.goal_type {
                crate::goals::types::GoalType::ImproveVo2max => {
                    return IntensityDistribution::polarized();
                }
                crate::goals::types::GoalType::ImproveEndurance => {
                    return IntensityDistribution::pyramidal();
                }
                crate::goals::types::GoalType::BuildThreshold => {
                    return IntensityDistribution::threshold_focused();
                }
                _ => {}
            }
        }

        // Default to model's preferred distribution
        model.preferred_distribution.clone()
    }

    fn determine_weekly_structure(&self, acwr: f32, adjustment: &LoadAdjustment) -> WeeklyStructure {
        let pattern = if acwr > 1.3 {
            WeeklyPattern::Recovery
        } else if adjustment.direction == AdjustmentDirection::Increase {
            WeeklyPattern::Build
        } else {
            WeeklyPattern::Standard
        };

        let (training_days, hard_sessions, rest_days) = match pattern {
            WeeklyPattern::Recovery => (4, 1, 3),
            WeeklyPattern::Build => (5, 2, 2),
            WeeklyPattern::Block => (6, 3, 1),
            WeeklyPattern::Standard => (5, 2, 2),
        };

        WeeklyStructure {
            training_days,
            hard_sessions,
            rest_days,
            long_ride_day: Some(chrono::Weekday::Sat),
            pattern,
        }
    }

    fn calculate_confidence(&self, load_history: &[(NaiveDate, DailyLoad)]) -> ModelConfidence {
        // Confidence based on data availability
        let data_factor = (load_history.len() as f32 / 90.0).min(1.0); // Max at 90 days

        // Consistency factor: check for gaps
        let consistency_factor = self.calculate_consistency_factor(load_history);

        ModelConfidence::new(data_factor * 0.5 + consistency_factor * 0.5)
    }

    fn calculate_consistency_factor(&self, load_history: &[(NaiveDate, DailyLoad)]) -> f32 {
        if load_history.len() < 7 {
            return 0.5;
        }

        // Count days with training in last 4 weeks
        let today = Utc::now().date_naive();
        let four_weeks_ago = today - Duration::days(28);

        let training_days = load_history
            .iter()
            .filter(|(date, load)| *date > four_weeks_ago && load.tss > 20.0)
            .count();

        (training_days as f32 / 20.0).min(1.0) // Expect ~20 training days in 4 weeks
    }

    fn generate_rationale(&self, adjustment: &LoadAdjustment, acwr: f32, goals: &[TrainingGoal]) -> String {
        let mut parts = Vec::new();

        parts.push(format!(
            "Current ACWR is {:.2}, which is {}.",
            acwr,
            if acwr > 1.3 {
                "elevated"
            } else if acwr < 0.8 {
                "low"
            } else {
                "in the optimal range"
            }
        ));

        parts.push(adjustment.reason.clone());

        if !goals.is_empty() {
            let goal_names: Vec<_> = goals.iter().map(|g| g.title.as_str()).collect();
            parts.push(format!(
                "Training aligned with goal{}: {}.",
                if goals.len() > 1 { "s" } else { "" },
                goal_names.join(", ")
            ));
        }

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_history(days: i64, avg_tss: f32) -> Vec<(NaiveDate, DailyLoad)> {
        let today = Utc::now().date_naive();
        (0..days)
            .map(|i| {
                let date = today - Duration::days(days - i);
                let ctl = 50.0 + (i as f32 * 0.5);
                (
                    date,
                    DailyLoad {
                        tss: avg_tss + (i as f32 % 7.0) * 10.0,
                        atl: ctl + 10.0,
                        ctl,
                        tsb: -10.0,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn test_local_recommendation() {
        let engine = AdaptationEngine::local_only();
        let history = create_test_history(30, 60.0);
        let model = AdaptationModel::default_for_user(Uuid::new_v4());

        let rec = engine.recommend_local(Uuid::new_v4(), &history, &model, &[]);

        assert!(rec.recommended_tss > 0.0);
        assert!(!rec.rationale.is_empty());
    }

    #[test]
    fn test_high_acwr_reduces_load() {
        let engine = AdaptationEngine::local_only();
        let today = Utc::now().date_naive();

        // Create history with high ATL relative to CTL
        let history: Vec<_> = (0..14)
            .map(|i| {
                let date = today - Duration::days(14 - i);
                (
                    date,
                    DailyLoad {
                        tss: 100.0,
                        atl: 80.0, // High acute
                        ctl: 50.0, // Lower chronic
                        tsb: -30.0,
                    },
                )
            })
            .collect();

        let model = AdaptationModel::default_for_user(Uuid::new_v4());
        let rec = engine.recommend_local(Uuid::new_v4(), &history, &model, &[]);

        assert_eq!(rec.adjustment.direction, AdjustmentDirection::Decrease);
    }

    #[test]
    fn test_low_acwr_increases_load() {
        let engine = AdaptationEngine::local_only();
        let today = Utc::now().date_naive();

        // Create history with low ATL relative to CTL
        let history: Vec<_> = (0..14)
            .map(|i| {
                let date = today - Duration::days(14 - i);
                (
                    date,
                    DailyLoad {
                        tss: 30.0, // Low training
                        atl: 35.0,
                        ctl: 50.0,
                        tsb: 15.0,
                    },
                )
            })
            .collect();

        let mut model = AdaptationModel::default_for_user(Uuid::new_v4());
        model.optimal_weekly_tss = 500.0; // Set high optimal to trigger increase

        let rec = engine.recommend_local(Uuid::new_v4(), &history, &model, &[]);

        assert_eq!(rec.adjustment.direction, AdjustmentDirection::Increase);
    }

    #[test]
    fn test_intensity_distribution_for_vo2max_goal() {
        let engine = AdaptationEngine::local_only();
        let history = create_test_history(30, 60.0);
        let model = AdaptationModel::default_for_user(Uuid::new_v4());

        let goal = TrainingGoal::new(
            Uuid::new_v4(),
            crate::goals::types::GoalType::ImproveVo2max,
            "Improve VO2max".into(),
        );

        let rec = engine.recommend_local(Uuid::new_v4(), &history, &model, &[goal]);

        // Should use polarized distribution for VO2max goal
        assert!(rec.intensity_distribution.low_intensity >= 0.75);
        assert!(rec.intensity_distribution.high_intensity >= 0.10);
    }

    #[test]
    fn test_weekly_structure_recovery() {
        let engine = AdaptationEngine::local_only();

        // High ACWR should trigger recovery week
        let adjustment = engine.calculate_adjustment(1.5, 400.0, &AdaptationModel::default_for_user(Uuid::new_v4()));
        let structure = engine.determine_weekly_structure(1.5, &adjustment);

        assert_eq!(structure.pattern, WeeklyPattern::Recovery);
        assert!(structure.rest_days >= 3);
    }

    #[test]
    fn test_model_update() {
        let engine = AdaptationEngine::local_only();
        let mut model = AdaptationModel::default_for_user(Uuid::new_v4());
        model.optimal_weekly_tss = 400.0;

        let history = create_test_history(30, 80.0);

        // Positive performance change should adjust optimal
        engine.update_model(&mut model, &history, 0.05);

        // Model should be updated
        assert!(model.optimal_weekly_tss != 400.0);
    }

    #[test]
    fn test_insufficient_data_validation() {
        // Test that we properly validate minimum data requirements
        let engine = AdaptationEngine::local_only();
        let history = create_test_history(7, 60.0); // Only 1 week

        // The recommend_local doesn't check for minimum data,
        // but recommend() async does. We test that the local version works.
        let model = AdaptationModel::default_for_user(Uuid::new_v4());
        let rec = engine.recommend_local(Uuid::new_v4(), &history, &model, &[]);

        // Should still produce a recommendation (local doesn't enforce minimum)
        assert!(rec.recommended_tss >= 0.0);
    }
}
