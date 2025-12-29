//! Workout loader for integrating plan workouts with the workout library.
//!
//! T069: Integrate plan workouts with existing workout library.

use uuid::Uuid;

use super::plan::{PlanWorkout, WorkoutType};

/// Loader for workout definitions.
#[derive(Debug, Default)]
pub struct WorkoutLoader {
    /// Custom workout mappings.
    custom_mappings: Vec<WorkoutMapping>,
}

impl WorkoutLoader {
    /// Create a new workout loader.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom workout mapping.
    pub fn add_mapping(&mut self, name: &str, workout_id: Uuid) {
        self.custom_mappings.push(WorkoutMapping {
            name: name.to_string(),
            workout_id,
        });
    }

    /// Find a workout ID by name.
    pub fn find_workout(&self, name: &str) -> Option<Uuid> {
        // First check custom mappings
        if let Some(mapping) = self.custom_mappings.iter().find(|m| m.name == name) {
            return Some(mapping.workout_id);
        }

        // Then check built-in workout suggestions
        self.suggest_workout(name)
    }

    /// Suggest a workout based on name and type.
    pub fn suggest_workout(&self, name: &str) -> Option<Uuid> {
        // This would integrate with the actual workout library
        // For now, return None to indicate no specific workout found
        // The UI can then show workout suggestions or let user pick
        let _ = name;
        None
    }

    /// Get suggested workouts for a workout type.
    pub fn workouts_for_type(&self, workout_type: WorkoutType) -> Vec<WorkoutSuggestion> {
        // Return built-in workout suggestions based on type
        match workout_type {
            WorkoutType::Endurance => vec![
                WorkoutSuggestion::builtin("Zone 2 Endurance", "60 minutes at 65-75% FTP"),
                WorkoutSuggestion::builtin("Aerobic Base", "90 minutes at 55-70% FTP"),
            ],
            WorkoutType::Recovery => vec![
                WorkoutSuggestion::builtin("Active Recovery", "30-45 minutes at 50-60% FTP"),
                WorkoutSuggestion::builtin("Easy Spin", "45 minutes very easy"),
            ],
            WorkoutType::Tempo => vec![
                WorkoutSuggestion::builtin("Tempo Intervals", "3x15min at 76-90% FTP"),
                WorkoutSuggestion::builtin("Sweet Spot", "2x20min at 88-93% FTP"),
            ],
            WorkoutType::Threshold => vec![
                WorkoutSuggestion::builtin("Threshold Intervals", "2x20min at 95-105% FTP"),
                WorkoutSuggestion::builtin("Over-Unders", "4x12min alternating 95%/105%"),
            ],
            WorkoutType::Vo2Max => vec![
                WorkoutSuggestion::builtin("VO2max Intervals", "5x5min at 105-120% FTP"),
                WorkoutSuggestion::builtin("Short VO2", "8x3min at 110-120% FTP"),
            ],
            WorkoutType::Anaerobic => vec![
                WorkoutSuggestion::builtin("Micro Bursts", "15x1min at 120-150% FTP"),
                WorkoutSuggestion::builtin("Tabata", "8x20sec all-out, 10sec rest"),
            ],
            WorkoutType::Sprint => vec![
                WorkoutSuggestion::builtin("Sprint Repeats", "10x15sec max effort"),
                WorkoutSuggestion::builtin("Standing Starts", "6x30sec from standstill"),
            ],
            WorkoutType::RaceSimulation => vec![
                WorkoutSuggestion::builtin("Race Simulation", "Varied intensity race practice"),
                WorkoutSuggestion::builtin("Attack Practice", "Surges and counter-attacks"),
            ],
            WorkoutType::Test => vec![
                WorkoutSuggestion::builtin("20min FTP Test", "All-out 20-minute effort"),
                WorkoutSuggestion::builtin("Ramp Test", "Progressive ramp to failure"),
            ],
            WorkoutType::Mixed => vec![
                WorkoutSuggestion::builtin("Openers", "Short high-intensity efforts"),
                WorkoutSuggestion::builtin("Mixed Intervals", "Various intensity intervals"),
            ],
        }
    }

    /// Create a workout definition from a plan workout.
    pub fn create_workout_definition(&self, plan_workout: &PlanWorkout) -> WorkoutDefinition {
        WorkoutDefinition {
            name: plan_workout.workout_name.clone(),
            description: plan_workout.description.clone(),
            duration_minutes: plan_workout.duration_minutes,
            estimated_tss: plan_workout.estimated_tss,
            workout_type: plan_workout.workout_type,
            linked_workout_id: plan_workout.workout_id,
        }
    }
}

/// Mapping from workout name to workout ID.
#[derive(Debug, Clone)]
struct WorkoutMapping {
    name: String,
    workout_id: Uuid,
}

/// Suggested workout for a type.
#[derive(Debug, Clone)]
pub struct WorkoutSuggestion {
    /// Workout name.
    pub name: String,
    /// Brief description.
    pub description: String,
    /// Linked workout ID (if available).
    pub workout_id: Option<Uuid>,
    /// Whether this is a built-in suggestion.
    pub is_builtin: bool,
}

impl WorkoutSuggestion {
    /// Create a built-in suggestion.
    pub fn builtin(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            workout_id: None,
            is_builtin: true,
        }
    }

    /// Create a suggestion with a linked workout.
    pub fn with_workout(name: &str, description: &str, workout_id: Uuid) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            workout_id: Some(workout_id),
            is_builtin: false,
        }
    }
}

/// Workout definition for execution.
#[derive(Debug, Clone)]
pub struct WorkoutDefinition {
    /// Workout name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Duration in minutes.
    pub duration_minutes: u16,
    /// Estimated TSS.
    pub estimated_tss: f32,
    /// Workout type.
    pub workout_type: WorkoutType,
    /// Linked workout ID for structured workout.
    pub linked_workout_id: Option<Uuid>,
}

impl WorkoutDefinition {
    /// Check if this has a linked structured workout.
    pub fn has_structured_workout(&self) -> bool {
        self.linked_workout_id.is_some()
    }

    /// Get estimated hours.
    pub fn estimated_hours(&self) -> f32 {
        self.duration_minutes as f32 / 60.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workout_loader() {
        let loader = WorkoutLoader::new();
        let suggestions = loader.workouts_for_type(WorkoutType::Threshold);

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].is_builtin);
    }

    #[test]
    fn test_custom_mapping() {
        let mut loader = WorkoutLoader::new();
        let workout_id = Uuid::new_v4();
        loader.add_mapping("My Custom Workout", workout_id);

        let found = loader.find_workout("My Custom Workout");
        assert_eq!(found, Some(workout_id));
    }

    #[test]
    fn test_workout_definition() {
        let plan_workout = PlanWorkout::new(1, "Test Workout", 60)
            .with_tss(50.0)
            .with_type(WorkoutType::Threshold);

        let loader = WorkoutLoader::new();
        let definition = loader.create_workout_definition(&plan_workout);

        assert_eq!(definition.name, "Test Workout");
        assert_eq!(definition.duration_minutes, 60);
        assert!(!definition.has_structured_workout());
    }

    #[test]
    fn test_suggestions_for_all_types() {
        let loader = WorkoutLoader::new();

        for workout_type in [
            WorkoutType::Endurance,
            WorkoutType::Recovery,
            WorkoutType::Tempo,
            WorkoutType::Threshold,
            WorkoutType::Vo2Max,
            WorkoutType::Anaerobic,
            WorkoutType::Sprint,
            WorkoutType::RaceSimulation,
            WorkoutType::Test,
            WorkoutType::Mixed,
        ] {
            let suggestions = loader.workouts_for_type(workout_type);
            assert!(
                !suggestions.is_empty(),
                "No suggestions for {:?}",
                workout_type
            );
        }
    }
}
