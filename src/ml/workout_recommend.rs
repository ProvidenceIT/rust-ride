//! Adaptive workout recommendations.
//!
//! T042-T052: Workout recommendation implementation

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::MlClient;
use super::types::MlError;
use crate::goals::types::TrainingGoal;
use crate::metrics::analytics::DailyLoad;
use crate::workouts::library::BuiltInWorkout;

/// A workout recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutRecommendation {
    /// Unique ID of this recommendation
    pub id: Uuid,
    /// Referenced workout ID
    pub workout_id: Uuid,
    /// Source of the workout
    pub source: WorkoutSource,
    /// Workout title for display
    pub title: String,
    /// Suitability score (0.0 - 1.0)
    pub suitability_score: f32,
    /// Why this workout was recommended
    pub reasoning: String,
    /// Target energy systems
    pub energy_systems: Vec<EnergySystem>,
    /// Expected TSS
    pub expected_tss: f32,
    /// Expected duration in minutes
    pub duration_minutes: u16,
    /// Estimated difficulty (1-10)
    pub difficulty: f32,
    /// Linked training goal (if any)
    pub goal_alignment: Option<Uuid>,
    /// Training gap being addressed
    pub training_gap: Option<String>,
    /// When this was recommended
    pub recommended_at: DateTime<Utc>,
    /// Current status
    pub status: RecommendationStatus,
}

/// Source of a workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkoutSource {
    /// Built-in library workout
    BuiltIn,
    /// User-imported workout
    UserImport,
}

impl WorkoutSource {
    pub fn label(&self) -> &'static str {
        match self {
            WorkoutSource::BuiltIn => "Built-in",
            WorkoutSource::UserImport => "Imported",
        }
    }
}

/// Energy system targeted by workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergySystem {
    Neuromuscular,
    Anaerobic,
    Vo2max,
    Threshold,
    SweetSpot,
    Endurance,
    Recovery,
}

impl EnergySystem {
    pub fn label(&self) -> &'static str {
        match self {
            EnergySystem::Neuromuscular => "Neuromuscular",
            EnergySystem::Anaerobic => "Anaerobic",
            EnergySystem::Vo2max => "VO2max",
            EnergySystem::Threshold => "Threshold",
            EnergySystem::SweetSpot => "Sweet Spot",
            EnergySystem::Endurance => "Endurance",
            EnergySystem::Recovery => "Recovery",
        }
    }
}

impl std::fmt::Display for EnergySystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Status of a recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationStatus {
    /// Recommendation is pending user action
    Pending,
    /// User accepted the recommendation
    Accepted,
    /// User declined the recommendation
    Declined,
    /// User completed the workout
    Completed,
    /// Recommendation expired
    Expired,
}

impl RecommendationStatus {
    pub fn label(&self) -> &'static str {
        match self {
            RecommendationStatus::Pending => "Pending",
            RecommendationStatus::Accepted => "Accepted",
            RecommendationStatus::Declined => "Declined",
            RecommendationStatus::Completed => "Completed",
            RecommendationStatus::Expired => "Expired",
        }
    }
}

/// Workout recommender with cloud and local logic.
pub struct WorkoutRecommender {
    client: Option<Arc<MlClient>>,
}

impl WorkoutRecommender {
    /// Create a new recommender with cloud client.
    pub fn new(client: Arc<MlClient>) -> Self {
        Self {
            client: Some(client),
        }
    }

    /// Create a new recommender for local-only recommendations.
    pub fn local_only() -> Self {
        Self { client: None }
    }

    /// Get personalized workout recommendations.
    pub async fn recommend(
        &self,
        _user_id: Uuid,
        goals: &[TrainingGoal],
        current_load: &DailyLoad,
        available_minutes: u16,
        recently_completed: &[Uuid],
        workouts: &[BuiltInWorkout],
    ) -> Result<Vec<WorkoutRecommendation>, MlError> {
        // Try cloud first if available
        if let Some(_client) = &self.client {
            // TODO: Implement cloud API call when backend is ready
        }

        // Local recommendation logic
        Ok(self.recommend_local(goals, current_load, available_minutes, recently_completed, workouts))
    }

    /// Generate recommendations using local logic.
    pub fn recommend_local(
        &self,
        goals: &[TrainingGoal],
        current_load: &DailyLoad,
        available_minutes: u16,
        recently_completed: &[Uuid],
        workouts: &[BuiltInWorkout],
    ) -> Vec<WorkoutRecommendation> {
        let acwr = if current_load.ctl > 0.0 {
            current_load.atl / current_load.ctl
        } else {
            1.0
        };

        // Filter workouts by duration
        let candidates: Vec<_> = workouts
            .iter()
            .filter(|w| w.duration_minutes <= available_minutes)
            .filter(|w| !recently_completed.contains(&w.id))
            .collect();

        // Score each workout
        let mut recommendations: Vec<WorkoutRecommendation> = candidates
            .iter()
            .map(|workout| {
                let (score, reasoning, gap) = self.score_workout(workout, goals, acwr);

                WorkoutRecommendation {
                    id: Uuid::new_v4(),
                    workout_id: workout.id,
                    source: WorkoutSource::BuiltIn,
                    title: workout.title.clone(),
                    suitability_score: score,
                    reasoning,
                    energy_systems: workout
                        .energy_systems
                        .iter()
                        .map(convert_energy_system)
                        .collect(),
                    expected_tss: workout.base_tss,
                    duration_minutes: workout.duration_minutes,
                    difficulty: workout.difficulty_tier.difficulty_range().0,
                    goal_alignment: goals.first().map(|g| g.id),
                    training_gap: gap,
                    recommended_at: Utc::now(),
                    status: RecommendationStatus::Pending,
                }
            })
            .collect();

        // Sort by suitability score descending
        recommendations.sort_by(|a, b| {
            b.suitability_score
                .partial_cmp(&a.suitability_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top 3
        recommendations.truncate(3);
        recommendations
    }

    fn score_workout(
        &self,
        workout: &BuiltInWorkout,
        goals: &[TrainingGoal],
        acwr: f32,
    ) -> (f32, String, Option<String>) {
        let mut score: f32 = 0.5; // Base score
        let mut reasons: Vec<String> = Vec::new();
        let mut gap = None;

        // ACWR adjustment - prioritize recovery if overreaching
        if acwr > 1.3 {
            if workout.energy_systems.iter().any(|e| matches!(e, crate::workouts::library::EnergySystem::Recovery)) {
                score += 0.3;
                reasons.push("Recovery prioritized due to high training load".to_string());
            } else {
                score -= 0.2;
            }
        } else if acwr < 0.8 {
            // Can handle harder workouts
            if workout.energy_systems.iter().any(|e| {
                matches!(
                    e,
                    crate::workouts::library::EnergySystem::Vo2max
                        | crate::workouts::library::EnergySystem::Threshold
                )
            }) {
                score += 0.15;
                reasons.push("Training load allows for intensity".to_string());
            }
        }

        // Goal alignment
        for goal in goals {
            if let Some(target_system) = goal.primary_energy_system() {
                let matches_system = workout.energy_systems.iter().any(|e| {
                    let system_name = format!("{:?}", e).to_lowercase();
                    system_name.contains(&target_system.to_lowercase())
                });

                if matches_system {
                    score += 0.25;
                    reasons.push(format!("Aligns with {} goal", goal.title));
                    gap = Some(format!("Targets {} for your goal", target_system));
                }
            }
        }

        // Build final reasoning
        let reasoning = if reasons.is_empty() {
            "Balanced workout option".to_string()
        } else {
            reasons.join("; ")
        };

        (score.min(1.0), reasoning, gap)
    }

    /// Recommend workouts for a specific goal.
    pub fn recommend_for_goal(
        &self,
        goal: &TrainingGoal,
        current_load: &DailyLoad,
        workouts: &[BuiltInWorkout],
    ) -> Vec<WorkoutRecommendation> {
        self.recommend_local(std::slice::from_ref(goal), current_load, 120, &[], workouts)
    }
}

fn convert_energy_system(system: &crate::workouts::library::EnergySystem) -> EnergySystem {
    match system {
        crate::workouts::library::EnergySystem::Neuromuscular => EnergySystem::Neuromuscular,
        crate::workouts::library::EnergySystem::Anaerobic => EnergySystem::Anaerobic,
        crate::workouts::library::EnergySystem::Vo2max => EnergySystem::Vo2max,
        crate::workouts::library::EnergySystem::Threshold => EnergySystem::Threshold,
        crate::workouts::library::EnergySystem::SweetSpot => EnergySystem::SweetSpot,
        crate::workouts::library::EnergySystem::Endurance => EnergySystem::Endurance,
        crate::workouts::library::EnergySystem::Recovery => EnergySystem::Recovery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goals::types::GoalType;
    use crate::workouts::library::{DifficultyTier, WorkoutCategory};

    fn create_test_workouts() -> Vec<BuiltInWorkout> {
        vec![
            BuiltInWorkout::new(
                "Recovery Spin".into(),
                "Easy recovery".into(),
                WorkoutCategory::Recovery,
                30,
                20.0,
            )
            .with_energy_systems(vec![crate::workouts::library::EnergySystem::Recovery])
            .with_difficulty(DifficultyTier::Easy),
            BuiltInWorkout::new(
                "VO2max 5x4min".into(),
                "Classic VO2 intervals".into(),
                WorkoutCategory::Vo2max,
                55,
                75.0,
            )
            .with_energy_systems(vec![crate::workouts::library::EnergySystem::Vo2max])
            .with_difficulty(DifficultyTier::VeryHard),
            BuiltInWorkout::new(
                "Sweet Spot 2x20".into(),
                "SS intervals".into(),
                WorkoutCategory::SweetSpot,
                60,
                70.0,
            )
            .with_energy_systems(vec![crate::workouts::library::EnergySystem::SweetSpot])
            .with_difficulty(DifficultyTier::Moderate),
        ]
    }

    #[test]
    fn test_local_recommendations() {
        let recommender = WorkoutRecommender::local_only();
        let workouts = create_test_workouts();
        let load = DailyLoad {
            tss: 50.0,
            atl: 60.0,
            ctl: 55.0,
            tsb: -5.0,
        };

        let recs = recommender.recommend_local(&[], &load, 60, &[], &workouts);

        assert!(!recs.is_empty());
        assert!(recs.len() <= 3);
    }

    #[test]
    fn test_high_acwr_prioritizes_recovery() {
        let recommender = WorkoutRecommender::local_only();
        let workouts = create_test_workouts();

        // High ACWR (overreaching)
        let load = DailyLoad {
            tss: 100.0,
            atl: 100.0, // High acute load
            ctl: 60.0,  // Lower chronic load
            tsb: -40.0,
        };

        let recs = recommender.recommend_local(&[], &load, 60, &[], &workouts);

        // Recovery should be highly ranked
        assert!(recs.iter().any(|r| r.title.contains("Recovery")));
    }

    #[test]
    fn test_goal_alignment() {
        let recommender = WorkoutRecommender::local_only();
        let workouts = create_test_workouts();
        let load = DailyLoad {
            tss: 50.0,
            atl: 50.0,
            ctl: 50.0,
            tsb: 0.0,
        };

        let goal = TrainingGoal::new(Uuid::new_v4(), GoalType::ImproveVo2max, "VO2max goal".into());

        let recs = recommender.recommend_for_goal(&goal, &load, &workouts);

        // VO2max workout should be recommended for VO2max goal
        assert!(recs.iter().any(|r| r.title.contains("VO2max")));
    }
}
