//! Workout difficulty estimation.
//!
//! T065-T076: Difficulty estimation implementation

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::{MlError, PredictionSource};
use crate::workouts::library::BuiltInWorkout;
use crate::workouts::types::Workout;

/// Difficulty estimate for a workout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyEstimate {
    /// Workout ID
    pub workout_id: Uuid,
    /// Personalized difficulty score (1-10)
    pub personalized_score: f32,
    /// Generic difficulty score (1-10)
    pub generic_score: f32,
    /// Contributing factors
    pub factors: DifficultyFactors,
    /// Estimated completion probability
    pub completion_probability: f32,
    /// Recommendation for the athlete
    pub recommendation: DifficultyRecommendation,
    /// Source of estimate
    pub source: PredictionSource,
    /// When this was estimated
    pub estimated_at: DateTime<Utc>,
}

/// Factors contributing to difficulty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyFactors {
    /// Duration factor (0.0 - 1.0)
    pub duration_factor: f32,
    /// Intensity factor (0.0 - 1.0)
    pub intensity_factor: f32,
    /// Interval structure complexity (0.0 - 1.0)
    pub complexity_factor: f32,
    /// Recovery vs work ratio factor (0.0 - 1.0)
    pub recovery_factor: f32,
    /// Comparison to athlete's history (0.0 - 1.0)
    pub history_factor: f32,
}

impl Default for DifficultyFactors {
    fn default() -> Self {
        Self {
            duration_factor: 0.5,
            intensity_factor: 0.5,
            complexity_factor: 0.5,
            recovery_factor: 0.5,
            history_factor: 0.5,
        }
    }
}

/// Recommendation based on difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifficultyRecommendation {
    /// Workout is too easy, suggest harder
    TooEasy,
    /// Workout is appropriate
    Appropriate,
    /// Workout is challenging but doable
    Challenging,
    /// Workout may be too hard
    TooHard,
}

impl DifficultyRecommendation {
    pub fn label(&self) -> &'static str {
        match self {
            DifficultyRecommendation::TooEasy => "Too Easy",
            DifficultyRecommendation::Appropriate => "Appropriate",
            DifficultyRecommendation::Challenging => "Challenging",
            DifficultyRecommendation::TooHard => "Too Hard",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DifficultyRecommendation::TooEasy => "This workout is below your current level",
            DifficultyRecommendation::Appropriate => "This workout matches your current fitness",
            DifficultyRecommendation::Challenging => "This workout will push your limits",
            DifficultyRecommendation::TooHard => "Consider an easier alternative",
        }
    }
}

impl std::fmt::Display for DifficultyRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Workout difficulty estimator.
pub struct DifficultyEstimator {
    client: Option<Arc<MlClient>>,
}

impl DifficultyEstimator {
    /// Create a new estimator with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
        }
    }

    /// Create a new estimator for local-only estimates.
    pub fn local_only() -> Self {
        Self { client: None }
    }

    /// Estimate difficulty for a structured workout.
    pub async fn estimate(
        &self,
        _user_id: Uuid,
        workout: &Workout,
        current_ftp: u16,
        recent_completion_rate: f32,
    ) -> Result<DifficultyEstimate, MlError> {
        // Try cloud first if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Fall back to local estimation
        Ok(self.estimate_local(workout, current_ftp, recent_completion_rate))
    }

    /// Estimate difficulty using local algorithm.
    pub fn estimate_local(
        &self,
        workout: &Workout,
        current_ftp: u16,
        recent_completion_rate: f32,
    ) -> DifficultyEstimate {
        let factors = self.calculate_factors(workout, current_ftp, recent_completion_rate);
        let generic_score = self.calculate_generic_score(&factors);
        let personalized_score =
            self.calculate_personalized_score(&factors, recent_completion_rate);

        let recommendation = self.classify_difficulty(personalized_score);
        let completion_probability = self.estimate_completion_probability(personalized_score);

        DifficultyEstimate {
            workout_id: workout.id,
            personalized_score,
            generic_score,
            factors,
            completion_probability,
            recommendation,
            source: PredictionSource::LocalFallback,
            estimated_at: Utc::now(),
        }
    }

    /// Estimate difficulty for a built-in library workout.
    pub fn estimate_builtin(
        &self,
        builtin: &BuiltInWorkout,
        current_ftp: u16,
        recent_completion_rate: f32,
    ) -> DifficultyEstimate {
        let factors = self.calculate_builtin_factors(builtin, current_ftp, recent_completion_rate);
        let generic_score = self.calculate_generic_score(&factors);
        let personalized_score =
            self.calculate_personalized_score(&factors, recent_completion_rate);

        let recommendation = self.classify_difficulty(personalized_score);
        let completion_probability = self.estimate_completion_probability(personalized_score);

        DifficultyEstimate {
            workout_id: builtin.id,
            personalized_score,
            generic_score,
            factors,
            completion_probability,
            recommendation,
            source: PredictionSource::LocalFallback,
            estimated_at: Utc::now(),
        }
    }

    fn calculate_factors(
        &self,
        workout: &Workout,
        _current_ftp: u16,
        recent_completion_rate: f32,
    ) -> DifficultyFactors {
        // Duration factor: longer workouts are harder
        let total_seconds = workout.total_duration_seconds;
        let duration_factor = (total_seconds as f32 / 7200.0).min(1.0); // Max at 2 hours

        // Intensity factor: based on segment power targets
        let intensity_factor = self.calculate_intensity_factor(workout);

        // Complexity factor: more segments = more complex
        let segment_count = workout.segments.len();
        let complexity_factor = (segment_count as f32 / 20.0).min(1.0);

        // Recovery factor: less recovery = harder
        let recovery_factor = self.calculate_recovery_factor(workout);

        // History factor: based on athlete's completion rate
        let history_factor = 1.0 - recent_completion_rate;

        DifficultyFactors {
            duration_factor,
            intensity_factor,
            complexity_factor,
            recovery_factor,
            history_factor,
        }
    }

    fn calculate_builtin_factors(
        &self,
        builtin: &BuiltInWorkout,
        _current_ftp: u16,
        recent_completion_rate: f32,
    ) -> DifficultyFactors {
        // Duration factor
        let duration_factor = (builtin.duration_minutes as f32 / 120.0).min(1.0);

        // Intensity factor from TSS
        let intensity_factor = (builtin.base_tss / 150.0).min(1.0);

        // Complexity factor from difficulty tier
        let (min_diff, max_diff) = builtin.difficulty_tier.difficulty_range();
        let complexity_factor = ((min_diff + max_diff) / 2.0 / 10.0).min(1.0);

        // Recovery factor (estimate based on category)
        let recovery_factor = match builtin.category {
            crate::workouts::library::WorkoutCategory::Recovery => 0.1,
            crate::workouts::library::WorkoutCategory::Endurance => 0.3,
            crate::workouts::library::WorkoutCategory::SweetSpot => 0.5,
            crate::workouts::library::WorkoutCategory::Threshold => 0.7,
            crate::workouts::library::WorkoutCategory::Vo2max => 0.8,
            crate::workouts::library::WorkoutCategory::Sprint => 0.6, // Short sprints have more recovery
            _ => 0.5,
        };

        // History factor
        let history_factor = 1.0 - recent_completion_rate;

        DifficultyFactors {
            duration_factor,
            intensity_factor,
            complexity_factor,
            recovery_factor,
            history_factor,
        }
    }

    fn calculate_intensity_factor(&self, workout: &Workout) -> f32 {
        if workout.segments.is_empty() {
            return 0.5;
        }

        let mut max_intensity = 0.0f32;
        let mut weighted_intensity = 0.0f32;
        let mut total_duration = 0u32;

        for segment in &workout.segments {
            let intensity = match &segment.power_target {
                crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                    *percent as f32 / 100.0
                }
                crate::workouts::types::PowerTarget::Absolute { watts } => {
                    (*watts as f32 / 250.0).min(1.5)
                } // Assume 250W FTP
                crate::workouts::types::PowerTarget::Range { start, end } => {
                    // Average of start and end for ramps
                    let start_pct = match start.as_ref() {
                        crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                            *percent as f32
                        }
                        crate::workouts::types::PowerTarget::Absolute { watts } => {
                            (*watts as f32 / 250.0) * 100.0
                        }
                        _ => 75.0,
                    };
                    let end_pct = match end.as_ref() {
                        crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                            *percent as f32
                        }
                        crate::workouts::types::PowerTarget::Absolute { watts } => {
                            (*watts as f32 / 250.0) * 100.0
                        }
                        _ => 75.0,
                    };
                    (start_pct + end_pct) / 2.0 / 100.0
                }
            };

            max_intensity = max_intensity.max(intensity);
            weighted_intensity += intensity * segment.duration_seconds as f32;
            total_duration += segment.duration_seconds;
        }

        if total_duration == 0 {
            return 0.5;
        }

        let avg_intensity = weighted_intensity / total_duration as f32;

        // Combine average and max intensity
        (avg_intensity * 0.6 + max_intensity * 0.4).min(1.0)
    }

    fn calculate_recovery_factor(&self, workout: &Workout) -> f32 {
        if workout.segments.is_empty() {
            return 0.5;
        }

        let mut work_time = 0u32;
        let mut recovery_time = 0u32;

        for segment in &workout.segments {
            let intensity = match &segment.power_target {
                crate::workouts::types::PowerTarget::PercentFtp { percent } => *percent as f32,
                crate::workouts::types::PowerTarget::Absolute { watts } => {
                    (*watts as f32 / 250.0) * 100.0
                }
                crate::workouts::types::PowerTarget::Range { start, end } => {
                    let start_pct = match start.as_ref() {
                        crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                            *percent as f32
                        }
                        crate::workouts::types::PowerTarget::Absolute { watts } => {
                            (*watts as f32 / 250.0) * 100.0
                        }
                        _ => 75.0,
                    };
                    let end_pct = match end.as_ref() {
                        crate::workouts::types::PowerTarget::PercentFtp { percent } => {
                            *percent as f32
                        }
                        crate::workouts::types::PowerTarget::Absolute { watts } => {
                            (*watts as f32 / 250.0) * 100.0
                        }
                        _ => 75.0,
                    };
                    (start_pct + end_pct) / 2.0
                }
            };

            if intensity < 60.0 {
                recovery_time += segment.duration_seconds;
            } else {
                work_time += segment.duration_seconds;
            }
        }

        let total = work_time + recovery_time;
        if total == 0 {
            return 0.5;
        }

        // Higher work ratio = higher difficulty
        work_time as f32 / total as f32
    }

    fn calculate_generic_score(&self, factors: &DifficultyFactors) -> f32 {
        // Weighted combination of factors
        let score = factors.duration_factor * 2.0
            + factors.intensity_factor * 3.0
            + factors.complexity_factor * 1.5
            + factors.recovery_factor * 2.5;

        // Scale to 1-10
        (score / 0.9).clamp(1.0, 10.0)
    }

    fn calculate_personalized_score(
        &self,
        factors: &DifficultyFactors,
        completion_rate: f32,
    ) -> f32 {
        let generic = self.calculate_generic_score(factors);

        // Adjust based on athlete's history
        let history_adjustment = (1.0 - completion_rate) * 2.0;

        (generic + history_adjustment).clamp(1.0, 10.0)
    }

    fn classify_difficulty(&self, score: f32) -> DifficultyRecommendation {
        if score < 3.0 {
            DifficultyRecommendation::TooEasy
        } else if score < 5.0 {
            DifficultyRecommendation::Appropriate
        } else if score < 7.5 {
            DifficultyRecommendation::Challenging
        } else {
            DifficultyRecommendation::TooHard
        }
    }

    fn estimate_completion_probability(&self, score: f32) -> f32 {
        // Higher difficulty = lower completion probability
        // Using a sigmoid-like function
        let x = (score - 5.0) / 2.0;
        1.0 / (1.0 + (x).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workouts::library::{DifficultyTier, WorkoutCategory};
    use crate::workouts::types::{PowerTarget, SegmentType, WorkoutSegment};

    fn create_test_workout() -> Workout {
        Workout::new(
            "Test Workout".into(),
            vec![
                WorkoutSegment {
                    segment_type: SegmentType::Warmup,
                    duration_seconds: 600,
                    power_target: PowerTarget::percent_ftp(55),
                    cadence_target: None,
                    text_event: None,
                },
                WorkoutSegment {
                    segment_type: SegmentType::Intervals,
                    duration_seconds: 1200,
                    power_target: PowerTarget::percent_ftp(95),
                    cadence_target: None,
                    text_event: None,
                },
                WorkoutSegment {
                    segment_type: SegmentType::Cooldown,
                    duration_seconds: 300,
                    power_target: PowerTarget::percent_ftp(50),
                    cadence_target: None,
                    text_event: None,
                },
            ],
        )
    }

    #[test]
    fn test_local_estimate() {
        let estimator = DifficultyEstimator::local_only();
        let workout = create_test_workout();

        let estimate = estimator.estimate_local(&workout, 250, 0.8);

        assert!(estimate.personalized_score >= 1.0);
        assert!(estimate.personalized_score <= 10.0);
        assert!(estimate.completion_probability >= 0.0);
        assert!(estimate.completion_probability <= 1.0);
    }

    #[test]
    fn test_builtin_estimate() {
        let estimator = DifficultyEstimator::local_only();
        let builtin = BuiltInWorkout::new(
            "Test VO2max".into(),
            "Hard intervals".into(),
            WorkoutCategory::Vo2max,
            60,
            90.0,
        )
        .with_difficulty(DifficultyTier::VeryHard);

        let estimate = estimator.estimate_builtin(&builtin, 250, 0.7);

        assert!(
            estimate.personalized_score > 5.0,
            "VO2max workout should be rated as hard"
        );
        assert!(matches!(
            estimate.recommendation,
            DifficultyRecommendation::Challenging | DifficultyRecommendation::TooHard
        ));
    }

    #[test]
    fn test_easy_workout_classification() {
        let estimator = DifficultyEstimator::local_only();
        let builtin = BuiltInWorkout::new(
            "Recovery Spin".into(),
            "Easy recovery".into(),
            WorkoutCategory::Recovery,
            30,
            20.0,
        )
        .with_difficulty(DifficultyTier::Easy);

        let estimate = estimator.estimate_builtin(&builtin, 250, 0.95);

        assert!(
            estimate.personalized_score < 5.0,
            "Recovery workout should be rated as easy"
        );
    }

    #[test]
    fn test_completion_probability() {
        let estimator = DifficultyEstimator::local_only();

        // Easy workout should have high completion probability
        let easy_prob = estimator.estimate_completion_probability(2.0);
        assert!(easy_prob > 0.8);

        // Hard workout should have lower completion probability
        let hard_prob = estimator.estimate_completion_probability(8.0);
        assert!(hard_prob < 0.3);
    }
}
