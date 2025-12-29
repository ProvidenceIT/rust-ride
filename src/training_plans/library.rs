//! Training plan library definitions.
//!
//! T063: Define initial training plan library (8 plans: 2 per discipline).

use once_cell::sync::Lazy;
use uuid::Uuid;

use super::disciplines::{DifficultyLevel, Discipline};
use super::plan::{PlanWeek, PlanWorkout, TrainingPhase, TrainingPlan, WorkoutType};

/// Get all available training plans.
pub fn all_plans() -> Vec<TrainingPlan> {
    PLAN_LIBRARY.plans.clone()
}

/// Get plans for a specific discipline.
pub fn get_plans_for_discipline(discipline: Discipline) -> Vec<TrainingPlan> {
    PLAN_LIBRARY
        .plans
        .iter()
        .filter(|p| p.discipline == discipline)
        .cloned()
        .collect()
}

/// Get a plan by its ID.
pub fn get_plan_by_id(id: Uuid) -> Option<TrainingPlan> {
    PLAN_LIBRARY.plans.iter().find(|p| p.id == id).cloned()
}

/// Training plan library.
pub struct PlanLibrary {
    /// All available plans.
    pub plans: Vec<TrainingPlan>,
}

impl PlanLibrary {
    /// Get plans by difficulty level.
    pub fn by_difficulty(&self, level: DifficultyLevel) -> Vec<&TrainingPlan> {
        self.plans
            .iter()
            .filter(|p| p.difficulty == level)
            .collect()
    }

    /// Get featured plans.
    pub fn featured(&self) -> Vec<&TrainingPlan> {
        self.plans.iter().filter(|p| p.is_featured).collect()
    }

    /// Search plans by name.
    pub fn search(&self, query: &str) -> Vec<&TrainingPlan> {
        let query_lower = query.to_lowercase();
        self.plans
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

/// The global plan library.
static PLAN_LIBRARY: Lazy<PlanLibrary> = Lazy::new(|| {
    PlanLibrary {
        plans: vec![
            // Road Racing Plans
            road_beginner_plan(),
            road_intermediate_plan(),
            // Gravel Plans
            gravel_beginner_plan(),
            gravel_intermediate_plan(),
            // Triathlon Plans
            triathlon_beginner_plan(),
            triathlon_intermediate_plan(),
            // MTB Plans
            mtb_beginner_plan(),
            mtb_intermediate_plan(),
            // General Fitness Plans
            fitness_beginner_plan(),
            fitness_intermediate_plan(),
        ],
    }
});

//
// Road Racing Plans
//

fn road_beginner_plan() -> TrainingPlan {
    // UUIDs are deterministic for consistent testing
    let id = Uuid::parse_str("10000001-0000-4000-8000-000000000001").unwrap();

    TrainingPlan::new(
        id,
        "Road Racing Fundamentals",
        Discipline::Road,
        DifficultyLevel::Beginner,
        "An 8-week introduction to road racing training. Build your aerobic base and develop the power needed for criteriums and road races.",
    )
    .with_weeks(vec![
        week(1, "Foundation Week 1", TrainingPhase::Base, vec![
            workout(1, "Easy Spin", 45, 30.0, WorkoutType::Recovery),
            workout(3, "Endurance Ride", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Weekend Endurance", 90, 60.0, WorkoutType::Endurance),
        ]),
        week(2, "Foundation Week 2", TrainingPhase::Base, vec![
            workout(1, "Easy Spin", 45, 30.0, WorkoutType::Recovery),
            workout(3, "Tempo Intervals", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 105, 70.0, WorkoutType::Endurance),
        ]),
        week(3, "Building Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery Spin", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Sweet Spot", 60, 60.0, WorkoutType::Tempo),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 120, 80.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery Week", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(5, "Endurance", 60, 40.0, WorkoutType::Endurance),
        ]),
        week(5, "Intensity Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Threshold Intervals", 60, 65.0, WorkoutType::Threshold),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 120, 85.0, WorkoutType::Endurance),
        ]),
        week(6, "Intensity Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "VO2max Intervals", 60, 75.0, WorkoutType::Vo2Max),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 105, 75.0, WorkoutType::Endurance),
        ]),
        week(7, "Peak Week", TrainingPhase::Peak, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Race Simulation", 75, 80.0, WorkoutType::RaceSimulation),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 90, 65.0, WorkoutType::Endurance),
        ]),
        week(8, "Taper Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 50.0, WorkoutType::Mixed),
            workout(6, "Race Day", 60, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["road".to_string(), "beginner".to_string(), "8-week".to_string()])
    .featured()
}

fn road_intermediate_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000001-0000-4000-8000-000000000002").unwrap();

    TrainingPlan::new(
        id,
        "Road Racing Performance",
        Discipline::Road,
        DifficultyLevel::Intermediate,
        "A 12-week structured plan for experienced cyclists looking to improve their road racing performance. Focuses on threshold power and race-specific intensity.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(4, "Tempo", 75, 65.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 150, 100.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "Endurance", 90, 65.0, WorkoutType::Endurance),
            workout(4, "Sweet Spot", 75, 70.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 165, 110.0, WorkoutType::Endurance),
        ]),
        week(3, "Base Week 3", TrainingPhase::Base, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "Endurance", 90, 65.0, WorkoutType::Endurance),
            workout(4, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 180, 120.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 60, 30.0, WorkoutType::Recovery),
            workout(4, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Easy Long Ride", 90, 55.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "VO2max Intervals", 75, 80.0, WorkoutType::Vo2Max),
            workout(4, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(6, "Long Ride with Intensity", 150, 105.0, WorkoutType::Mixed),
        ]),
        week(6, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "VO2max Intervals", 75, 85.0, WorkoutType::Vo2Max),
            workout(4, "Over-Unders", 75, 80.0, WorkoutType::Threshold),
            workout(6, "Group Ride Simulation", 165, 115.0, WorkoutType::RaceSimulation),
        ]),
        week(7, "Build Week 3", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "VO2max", 75, 85.0, WorkoutType::Vo2Max),
            workout(4, "Threshold Repeats", 90, 90.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 180, 120.0, WorkoutType::Endurance),
        ]),
        week(8, "Recovery", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 60, 30.0, WorkoutType::Recovery),
            workout(4, "Endurance", 75, 50.0, WorkoutType::Endurance),
            workout(6, "Easy Long", 105, 65.0, WorkoutType::Endurance),
        ]),
        week(9, "Specialty Week 1", TrainingPhase::Specialty, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "Attack Simulations", 75, 85.0, WorkoutType::Anaerobic),
            workout(4, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(6, "Race Simulation", 150, 110.0, WorkoutType::RaceSimulation),
        ]),
        week(10, "Specialty Week 2", TrainingPhase::Specialty, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(2, "Sprint Work", 60, 70.0, WorkoutType::Sprint),
            workout(4, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(6, "Race Simulation", 120, 95.0, WorkoutType::RaceSimulation),
        ]),
        week(11, "Taper Week 1", TrainingPhase::Taper, vec![
            workout(2, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(4, "Openers", 60, 55.0, WorkoutType::Mixed),
            workout(6, "Easy Endurance", 90, 55.0, WorkoutType::Endurance),
        ]),
        week(12, "Race Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 45.0, WorkoutType::Mixed),
            workout(6, "Race Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["road".to_string(), "intermediate".to_string(), "12-week".to_string(), "racing".to_string()])
}

//
// Gravel Plans
//

fn gravel_beginner_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000002-0000-4000-8000-000000000001").unwrap();

    TrainingPlan::new(
        id,
        "Gravel Essentials",
        Discipline::Gravel,
        DifficultyLevel::Beginner,
        "A 6-week introduction to gravel riding. Build endurance and learn to sustain power over varied terrain.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(2, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 90, 60.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(2, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(5, "Sweet Spot", 60, 60.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 105, 70.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Muscular Endurance", 60, 60.0, WorkoutType::Tempo),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 120, 80.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(3, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(6, "Endurance", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Tempo Bursts", 60, 65.0, WorkoutType::Mixed),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Gravel Simulation", 135, 90.0, WorkoutType::RaceSimulation),
        ]),
        week(6, "Event Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 45.0, WorkoutType::Mixed),
            workout(6, "Event Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["gravel".to_string(), "beginner".to_string(), "6-week".to_string()])
    .featured()
}

fn gravel_intermediate_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000002-0000-4000-8000-000000000002").unwrap();

    TrainingPlan::new(
        id,
        "Gravel Grinder Prep",
        Discipline::Gravel,
        DifficultyLevel::Intermediate,
        "An 8-week plan to prepare for long gravel events. Focus on sustained power and muscular endurance.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(3, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 150, 100.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(3, "Sweet Spot", 75, 65.0, WorkoutType::Tempo),
            workout(5, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 165, 110.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(3, "Muscular Endurance", 75, 70.0, WorkoutType::Tempo),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 180, 120.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 60, 30.0, WorkoutType::Recovery),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Easy Long", 90, 55.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(3, "Over-Unders", 75, 75.0, WorkoutType::Threshold),
            workout(5, "Tempo Bursts", 60, 65.0, WorkoutType::Mixed),
            workout(6, "Gravel Simulation", 195, 130.0, WorkoutType::RaceSimulation),
        ]),
        week(6, "Build Week 3", TrainingPhase::Build, vec![
            workout(1, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(3, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(5, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 210, 140.0, WorkoutType::Endurance),
        ]),
        week(7, "Taper Week", TrainingPhase::Taper, vec![
            workout(2, "Recovery", 60, 35.0, WorkoutType::Recovery),
            workout(4, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Easy Long", 120, 75.0, WorkoutType::Endurance),
        ]),
        week(8, "Event Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 45.0, WorkoutType::Mixed),
            workout(6, "Event Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["gravel".to_string(), "intermediate".to_string(), "8-week".to_string(), "endurance".to_string()])
}

//
// Triathlon Plans
//

fn triathlon_beginner_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000003-0000-4000-8000-000000000001").unwrap();

    TrainingPlan::new(
        id,
        "Triathlon Bike Foundation",
        Discipline::Triathlon,
        DifficultyLevel::Beginner,
        "A 6-week bike-focused plan for beginner triathletes. Build aerobic base and learn to sustain steady power.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(2, "Endurance", 45, 35.0, WorkoutType::Endurance),
            workout(4, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(2, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(4, "Tempo", 45, 45.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 90, 60.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(5, "Endurance", 45, 35.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 105, 70.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(3, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(6, "Endurance", 60, 40.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Threshold", 60, 60.0, WorkoutType::Threshold),
            workout(5, "Tempo", 45, 45.0, WorkoutType::Tempo),
            workout(6, "Race Simulation", 90, 65.0, WorkoutType::RaceSimulation),
        ]),
        week(6, "Race Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 30, 20.0, WorkoutType::Recovery),
            workout(4, "Openers", 30, 35.0, WorkoutType::Mixed),
            workout(6, "Race Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["triathlon".to_string(), "beginner".to_string(), "6-week".to_string(), "bike".to_string()])
}

fn triathlon_intermediate_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000003-0000-4000-8000-000000000002").unwrap();

    TrainingPlan::new(
        id,
        "Triathlon Bike Performance",
        Discipline::Triathlon,
        DifficultyLevel::Intermediate,
        "An 8-week plan to improve your triathlon bike leg. Focus on sustained power and race-pace efforts.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 120, 80.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Sweet Spot", 75, 65.0, WorkoutType::Tempo),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 135, 90.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 150, 100.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(5, "Endurance", 45, 35.0, WorkoutType::Endurance),
            workout(6, "Easy Long", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Race Pace Intervals", 75, 75.0, WorkoutType::Threshold),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 150, 100.0, WorkoutType::Endurance),
        ]),
        week(6, "Build Week 3", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Threshold", 75, 75.0, WorkoutType::Threshold),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Race Simulation", 120, 85.0, WorkoutType::RaceSimulation),
        ]),
        week(7, "Taper Week", TrainingPhase::Taper, vec![
            workout(2, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Tempo", 45, 45.0, WorkoutType::Tempo),
            workout(6, "Easy Long", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(8, "Race Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 30, 20.0, WorkoutType::Recovery),
            workout(4, "Openers", 30, 35.0, WorkoutType::Mixed),
            workout(6, "Race Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["triathlon".to_string(), "intermediate".to_string(), "8-week".to_string(), "bike".to_string()])
}

//
// MTB Plans
//

fn mtb_beginner_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000004-0000-4000-8000-000000000001").unwrap();

    TrainingPlan::new(
        id,
        "MTB Foundations",
        Discipline::MTB,
        DifficultyLevel::Beginner,
        "A 6-week plan to build MTB fitness. Focus on short power bursts and recovery between efforts.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(2, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(5, "Tempo Bursts", 45, 50.0, WorkoutType::Mixed),
            workout(6, "Long Ride", 90, 60.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(2, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(5, "Sprint Repeats", 45, 55.0, WorkoutType::Anaerobic),
            workout(6, "Long Ride", 105, 70.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Micro Intervals", 60, 60.0, WorkoutType::Anaerobic),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "MTB Simulation", 120, 80.0, WorkoutType::RaceSimulation),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(3, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(6, "Endurance", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Attack Training", 60, 70.0, WorkoutType::Anaerobic),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "MTB Simulation", 105, 75.0, WorkoutType::RaceSimulation),
        ]),
        week(6, "Race Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 50.0, WorkoutType::Mixed),
            workout(6, "Race Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["mtb".to_string(), "beginner".to_string(), "6-week".to_string()])
}

fn mtb_intermediate_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000004-0000-4000-8000-000000000002").unwrap();

    TrainingPlan::new(
        id,
        "MTB XC Performance",
        Discipline::MTB,
        DifficultyLevel::Intermediate,
        "An 8-week XC-focused plan. Develop explosive power and the ability to recover quickly between efforts.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Endurance", 75, 55.0, WorkoutType::Endurance),
            workout(5, "Micro Intervals", 60, 60.0, WorkoutType::Anaerobic),
            workout(6, "Long Ride", 120, 80.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Tempo Bursts", 75, 65.0, WorkoutType::Mixed),
            workout(5, "Sprint Repeats", 60, 65.0, WorkoutType::Sprint),
            workout(6, "Long Ride", 135, 90.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "VO2max Intervals", 75, 80.0, WorkoutType::Vo2Max),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "MTB Simulation", 120, 85.0, WorkoutType::RaceSimulation),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(5, "Endurance", 60, 40.0, WorkoutType::Endurance),
            workout(6, "Easy Long", 90, 55.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Attack Intervals", 75, 85.0, WorkoutType::Anaerobic),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 135, 90.0, WorkoutType::Endurance),
        ]),
        week(6, "Build Week 3", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Sprint Training", 60, 70.0, WorkoutType::Sprint),
            workout(5, "VO2max", 60, 75.0, WorkoutType::Vo2Max),
            workout(6, "Race Simulation", 105, 80.0, WorkoutType::RaceSimulation),
        ]),
        week(7, "Taper Week", TrainingPhase::Taper, vec![
            workout(2, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Micro Intervals", 45, 55.0, WorkoutType::Anaerobic),
            workout(6, "Easy Long", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(8, "Race Week", TrainingPhase::Taper, vec![
            workout(2, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(4, "Openers", 45, 55.0, WorkoutType::Mixed),
            workout(6, "Race Day", 0, 0.0, WorkoutType::RaceSimulation),
        ]),
    ])
    .with_tags(vec!["mtb".to_string(), "intermediate".to_string(), "8-week".to_string(), "xc".to_string()])
}

//
// General Fitness Plans
//

fn fitness_beginner_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000005-0000-4000-8000-000000000001").unwrap();

    TrainingPlan::new(
        id,
        "Get Fit Cycling",
        Discipline::GeneralFitness,
        DifficultyLevel::Beginner,
        "A 4-week introduction to structured indoor cycling. Perfect for beginners looking to build a fitness habit.",
    )
    .with_weeks(vec![
        week(1, "Week 1 - Getting Started", TrainingPhase::Base, vec![
            workout(2, "Easy Spin", 30, 20.0, WorkoutType::Recovery),
            workout(4, "Endurance", 30, 25.0, WorkoutType::Endurance),
            workout(6, "Weekend Ride", 45, 30.0, WorkoutType::Endurance),
        ]),
        week(2, "Week 2 - Building Consistency", TrainingPhase::Base, vec![
            workout(2, "Easy Spin", 30, 20.0, WorkoutType::Recovery),
            workout(4, "Endurance", 45, 35.0, WorkoutType::Endurance),
            workout(6, "Weekend Ride", 60, 40.0, WorkoutType::Endurance),
        ]),
        week(3, "Week 3 - Adding Intensity", TrainingPhase::Build, vec![
            workout(1, "Recovery", 30, 15.0, WorkoutType::Recovery),
            workout(3, "Tempo", 45, 45.0, WorkoutType::Tempo),
            workout(5, "Endurance", 30, 25.0, WorkoutType::Endurance),
            workout(6, "Weekend Ride", 60, 45.0, WorkoutType::Endurance),
        ]),
        week(4, "Week 4 - Consolidation", TrainingPhase::Build, vec![
            workout(1, "Recovery", 30, 15.0, WorkoutType::Recovery),
            workout(3, "Tempo Intervals", 45, 50.0, WorkoutType::Tempo),
            workout(5, "Endurance", 45, 35.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 75, 50.0, WorkoutType::Endurance),
        ]),
    ])
    .with_tags(vec!["fitness".to_string(), "beginner".to_string(), "4-week".to_string()])
    .featured()
}

fn fitness_intermediate_plan() -> TrainingPlan {
    let id = Uuid::parse_str("10000005-0000-4000-8000-000000000002").unwrap();

    TrainingPlan::new(
        id,
        "Fitness Builder",
        Discipline::GeneralFitness,
        DifficultyLevel::Intermediate,
        "A 6-week plan to improve overall cycling fitness. Balanced mix of endurance, tempo, and threshold work.",
    )
    .with_weeks(vec![
        week(1, "Base Week 1", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(5, "Tempo", 45, 45.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 90, 60.0, WorkoutType::Endurance),
        ]),
        week(2, "Base Week 2", TrainingPhase::Base, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Sweet Spot", 60, 55.0, WorkoutType::Tempo),
            workout(5, "Endurance", 60, 45.0, WorkoutType::Endurance),
            workout(6, "Long Ride", 105, 70.0, WorkoutType::Endurance),
        ]),
        week(3, "Build Week 1", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Long Ride", 120, 80.0, WorkoutType::Endurance),
        ]),
        week(4, "Recovery", TrainingPhase::Recovery, vec![
            workout(3, "Easy Spin", 45, 25.0, WorkoutType::Recovery),
            workout(6, "Endurance", 75, 50.0, WorkoutType::Endurance),
        ]),
        week(5, "Build Week 2", TrainingPhase::Build, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "VO2max Intervals", 60, 70.0, WorkoutType::Vo2Max),
            workout(5, "Threshold", 60, 65.0, WorkoutType::Threshold),
            workout(6, "Long Ride", 105, 75.0, WorkoutType::Endurance),
        ]),
        week(6, "Peak Week", TrainingPhase::Peak, vec![
            workout(1, "Recovery", 45, 25.0, WorkoutType::Recovery),
            workout(3, "Mixed Intervals", 60, 70.0, WorkoutType::Mixed),
            workout(5, "Tempo", 60, 55.0, WorkoutType::Tempo),
            workout(6, "Test Ride", 60, 70.0, WorkoutType::Test),
        ]),
    ])
    .with_tags(vec!["fitness".to_string(), "intermediate".to_string(), "6-week".to_string()])
}

//
// Helper functions
//

fn week(number: u8, title: &str, phase: TrainingPhase, workouts: Vec<PlanWorkout>) -> PlanWeek {
    PlanWeek::new(number, title, phase).with_workouts(workouts)
}

fn workout(day: u8, name: &str, duration: u16, tss: f32, workout_type: WorkoutType) -> PlanWorkout {
    PlanWorkout::new(day, name, duration)
        .with_tss(tss)
        .with_type(workout_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_plans() {
        let plans = all_plans();
        assert_eq!(plans.len(), 10); // 2 plans per discipline x 5 disciplines
    }

    #[test]
    fn test_get_plans_by_discipline() {
        let road_plans = get_plans_for_discipline(Discipline::Road);
        assert_eq!(road_plans.len(), 2);
        assert!(road_plans.iter().all(|p| p.discipline == Discipline::Road));
    }

    #[test]
    fn test_get_plan_by_id() {
        let id = Uuid::parse_str("10000001-0000-4000-8000-000000000001").unwrap();
        let plan = get_plan_by_id(id);
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().name, "Road Racing Fundamentals");
    }

    #[test]
    fn test_plan_library_search() {
        let results = PLAN_LIBRARY.search("road");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_featured_plans() {
        let featured = PLAN_LIBRARY.featured();
        assert!(!featured.is_empty());
        assert!(featured.iter().all(|p| p.is_featured));
    }

    #[test]
    fn test_plans_have_weeks() {
        for plan in all_plans() {
            assert!(plan.duration_weeks > 0, "Plan {} has no weeks", plan.name);
            assert!(
                plan.total_workouts() > 0,
                "Plan {} has no workouts",
                plan.name
            );
        }
    }
}
