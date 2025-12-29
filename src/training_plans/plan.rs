//! Training plan definitions.
//!
//! T059: Create TrainingPlan and PlanWeek structs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DifficultyLevel, Discipline};

/// A complete training plan for a specific discipline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPlan {
    /// Unique identifier for this plan.
    pub id: Uuid,
    /// Display name of the plan.
    pub name: String,
    /// Target cycling discipline.
    pub discipline: Discipline,
    /// Plan duration in weeks.
    pub duration_weeks: u8,
    /// Recommended workouts per week.
    pub workouts_per_week: u8,
    /// Plan description.
    pub description: String,
    /// Difficulty level.
    pub difficulty: DifficultyLevel,
    /// Weekly structure of the plan.
    pub weeks: Vec<PlanWeek>,
    /// Optional tags for filtering.
    pub tags: Vec<String>,
    /// Whether this plan is featured/recommended.
    pub is_featured: bool,
}

impl TrainingPlan {
    /// Create a new training plan.
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        discipline: Discipline,
        difficulty: DifficultyLevel,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            discipline,
            duration_weeks: 0,
            workouts_per_week: 0,
            description: description.into(),
            difficulty,
            weeks: Vec::new(),
            tags: Vec::new(),
            is_featured: false,
        }
    }

    /// Add a week to the plan.
    pub fn add_week(&mut self, week: PlanWeek) {
        self.weeks.push(week);
        self.duration_weeks = self.weeks.len() as u8;
        self.recalculate_workouts_per_week();
    }

    /// Set weeks from a vector.
    pub fn with_weeks(mut self, weeks: Vec<PlanWeek>) -> Self {
        self.weeks = weeks;
        self.duration_weeks = self.weeks.len() as u8;
        self.recalculate_workouts_per_week();
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Mark as featured.
    pub fn featured(mut self) -> Self {
        self.is_featured = true;
        self
    }

    /// Recalculate average workouts per week.
    fn recalculate_workouts_per_week(&mut self) {
        if self.weeks.is_empty() {
            self.workouts_per_week = 0;
        } else {
            let total_workouts: usize = self.weeks.iter().map(|w| w.workouts.len()).sum();
            self.workouts_per_week = (total_workouts as f32 / self.weeks.len() as f32).round() as u8;
        }
    }

    /// Get the total number of workouts in the plan.
    pub fn total_workouts(&self) -> usize {
        self.weeks.iter().map(|w| w.workouts.len()).sum()
    }

    /// Get the total estimated hours for the plan.
    pub fn total_estimated_hours(&self) -> f32 {
        self.weeks.iter().map(|w| w.total_hours()).sum()
    }

    /// Get a specific week by number (1-indexed).
    pub fn get_week(&self, week_number: u8) -> Option<&PlanWeek> {
        if week_number == 0 || week_number as usize > self.weeks.len() {
            None
        } else {
            self.weeks.get(week_number as usize - 1)
        }
    }

    /// Get a summary of the plan.
    pub fn summary(&self) -> String {
        format!(
            "{} weeks, {} workouts/week, {} total workouts",
            self.duration_weeks, self.workouts_per_week, self.total_workouts()
        )
    }
}

/// A single week within a training plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWeek {
    /// Week number within the plan (1-indexed).
    pub week_number: u8,
    /// Week title (e.g., "Base Building", "Recovery Week").
    pub title: String,
    /// Week description/focus.
    pub description: String,
    /// Phase of training (e.g., "Base", "Build", "Peak", "Taper").
    pub phase: TrainingPhase,
    /// Workouts scheduled for this week.
    pub workouts: Vec<PlanWorkout>,
    /// Target total hours for the week.
    pub target_hours: f32,
}

impl PlanWeek {
    /// Create a new plan week.
    pub fn new(week_number: u8, title: impl Into<String>, phase: TrainingPhase) -> Self {
        Self {
            week_number,
            title: title.into(),
            description: String::new(),
            phase,
            workouts: Vec::new(),
            target_hours: 0.0,
        }
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set target hours.
    pub fn with_target_hours(mut self, hours: f32) -> Self {
        self.target_hours = hours;
        self
    }

    /// Add a workout to the week.
    pub fn add_workout(&mut self, workout: PlanWorkout) {
        self.workouts.push(workout);
    }

    /// Set workouts.
    pub fn with_workouts(mut self, workouts: Vec<PlanWorkout>) -> Self {
        self.workouts = workouts;
        self
    }

    /// Get total hours from workouts.
    pub fn total_hours(&self) -> f32 {
        self.workouts.iter().map(|w| w.duration_minutes as f32 / 60.0).sum()
    }

    /// Get the number of workouts.
    pub fn workout_count(&self) -> usize {
        self.workouts.len()
    }
}

/// A workout within a plan week.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWorkout {
    /// Day of the week (1 = Monday, 7 = Sunday).
    pub day_of_week: u8,
    /// Reference to a workout definition (by ID or name).
    pub workout_id: Option<Uuid>,
    /// Workout name/title for display.
    pub workout_name: String,
    /// Brief description of the workout.
    pub description: String,
    /// Estimated duration in minutes.
    pub duration_minutes: u16,
    /// Estimated TSS for the workout.
    pub estimated_tss: f32,
    /// Whether this workout is optional.
    pub is_optional: bool,
    /// Alternative workout if the primary can't be done.
    pub alternative_id: Option<Uuid>,
    /// Workout type category.
    pub workout_type: WorkoutType,
}

impl PlanWorkout {
    /// Create a new plan workout.
    pub fn new(
        day_of_week: u8,
        workout_name: impl Into<String>,
        duration_minutes: u16,
    ) -> Self {
        Self {
            day_of_week,
            workout_id: None,
            workout_name: workout_name.into(),
            description: String::new(),
            duration_minutes,
            estimated_tss: 0.0,
            is_optional: false,
            alternative_id: None,
            workout_type: WorkoutType::Endurance,
        }
    }

    /// Set workout ID.
    pub fn with_workout_id(mut self, id: Uuid) -> Self {
        self.workout_id = Some(id);
        self
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set estimated TSS.
    pub fn with_tss(mut self, tss: f32) -> Self {
        self.estimated_tss = tss;
        self
    }

    /// Mark as optional.
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Set alternative workout.
    pub fn with_alternative(mut self, alt_id: Uuid) -> Self {
        self.alternative_id = Some(alt_id);
        self
    }

    /// Set workout type.
    pub fn with_type(mut self, workout_type: WorkoutType) -> Self {
        self.workout_type = workout_type;
        self
    }

    /// Get the day name.
    pub fn day_name(&self) -> &'static str {
        match self.day_of_week {
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            7 => "Sunday",
            _ => "Unknown",
        }
    }
}

/// Training phase within a periodized plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrainingPhase {
    /// Foundation building, lower intensity.
    Base,
    /// Intensity building phase.
    Build,
    /// Peak fitness phase.
    Peak,
    /// Pre-event taper.
    Taper,
    /// Recovery week.
    Recovery,
    /// Specialty/race-specific work.
    Specialty,
    /// Transition/off-season.
    Transition,
}

impl TrainingPhase {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Build => "Build",
            Self::Peak => "Peak",
            Self::Taper => "Taper",
            Self::Recovery => "Recovery",
            Self::Specialty => "Specialty",
            Self::Transition => "Transition",
        }
    }

    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Base => "Building aerobic foundation with lower intensity training",
            Self::Build => "Increasing intensity and adding sport-specific work",
            Self::Peak => "High intensity work to maximize fitness",
            Self::Taper => "Reducing volume while maintaining intensity before an event",
            Self::Recovery => "Easy week to allow adaptation and prevent overtraining",
            Self::Specialty => "Race-specific and event-targeted training",
            Self::Transition => "Active recovery and off-season maintenance",
        }
    }

    /// Get typical intensity factor range.
    pub fn intensity_range(&self) -> (f32, f32) {
        match self {
            Self::Base => (0.60, 0.75),
            Self::Build => (0.70, 0.85),
            Self::Peak => (0.80, 0.95),
            Self::Taper => (0.65, 0.80),
            Self::Recovery => (0.50, 0.65),
            Self::Specialty => (0.75, 0.90),
            Self::Transition => (0.45, 0.60),
        }
    }
}

/// Type of workout for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorkoutType {
    /// Zone 2 aerobic endurance.
    #[default]
    Endurance,
    /// Recovery ride.
    Recovery,
    /// Tempo/sweet spot work.
    Tempo,
    /// Threshold intervals.
    Threshold,
    /// VO2max intervals.
    Vo2Max,
    /// Anaerobic/neuromuscular power.
    Anaerobic,
    /// Sprint work.
    Sprint,
    /// Race simulation.
    RaceSimulation,
    /// FTP test.
    Test,
    /// Mixed/general workout.
    Mixed,
}

impl WorkoutType {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Endurance => "Endurance",
            Self::Recovery => "Recovery",
            Self::Tempo => "Tempo",
            Self::Threshold => "Threshold",
            Self::Vo2Max => "VO2max",
            Self::Anaerobic => "Anaerobic",
            Self::Sprint => "Sprint",
            Self::RaceSimulation => "Race Sim",
            Self::Test => "Test",
            Self::Mixed => "Mixed",
        }
    }

    /// Get typical zone.
    pub fn primary_zone(&self) -> u8 {
        match self {
            Self::Recovery => 1,
            Self::Endurance => 2,
            Self::Tempo => 3,
            Self::Threshold => 4,
            Self::Vo2Max => 5,
            Self::Anaerobic | Self::Sprint => 6,
            Self::RaceSimulation | Self::Test | Self::Mixed => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_plan() {
        let plan = TrainingPlan::new(
            Uuid::new_v4(),
            "8 Week Road Racing",
            Discipline::Road,
            DifficultyLevel::Intermediate,
            "Build fitness for road racing season",
        );

        assert_eq!(plan.name, "8 Week Road Racing");
        assert_eq!(plan.discipline, Discipline::Road);
        assert_eq!(plan.duration_weeks, 0);
    }

    #[test]
    fn test_plan_with_weeks() {
        let mut plan = TrainingPlan::new(
            Uuid::new_v4(),
            "Test Plan",
            Discipline::Road,
            DifficultyLevel::Beginner,
            "Test",
        );

        let week1 = PlanWeek::new(1, "Week 1", TrainingPhase::Base)
            .with_workouts(vec![
                PlanWorkout::new(1, "Endurance Ride", 60),
                PlanWorkout::new(3, "Tempo", 45),
                PlanWorkout::new(5, "Endurance Ride", 60),
            ]);

        let week2 = PlanWeek::new(2, "Week 2", TrainingPhase::Base)
            .with_workouts(vec![
                PlanWorkout::new(1, "Endurance Ride", 60),
                PlanWorkout::new(4, "Sweet Spot", 60),
            ]);

        plan.add_week(week1);
        plan.add_week(week2);

        assert_eq!(plan.duration_weeks, 2);
        assert_eq!(plan.total_workouts(), 5);
        assert_eq!(plan.workouts_per_week, 3); // (3+2)/2 rounded
    }

    #[test]
    fn test_week_hours() {
        let week = PlanWeek::new(1, "Test Week", TrainingPhase::Base)
            .with_workouts(vec![
                PlanWorkout::new(1, "Workout 1", 60),
                PlanWorkout::new(3, "Workout 2", 90),
            ]);

        assert!((week.total_hours() - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_day_name() {
        assert_eq!(PlanWorkout::new(1, "Test", 60).day_name(), "Monday");
        assert_eq!(PlanWorkout::new(7, "Test", 60).day_name(), "Sunday");
    }

    #[test]
    fn test_training_phases() {
        assert_eq!(TrainingPhase::Base.display_name(), "Base");
        let (min, max) = TrainingPhase::Build.intensity_range();
        assert!(min < max);
    }
}
