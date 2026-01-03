//! Built-in workout library.
//!
//! T019: Create BuiltInWorkout and WorkoutLibrary structs
//! T020: Implement workout seeding logic
//! T021: Add search/filter methods

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::WorkoutSegment;

/// A built-in curated workout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInWorkout {
    /// Unique identifier
    pub id: Uuid,
    /// Display title
    pub title: String,
    /// Description of the workout
    pub description: String,
    /// Workout category
    pub category: WorkoutCategory,
    /// Target energy systems
    pub energy_systems: Vec<EnergySystem>,
    /// Goal types this workout aligns with
    pub goal_alignment: Vec<String>,
    /// Difficulty tier
    pub difficulty_tier: DifficultyTier,
    /// Duration in minutes
    pub duration_minutes: u16,
    /// Base TSS estimate
    pub base_tss: f32,
    /// Workout segments
    pub segments: Vec<WorkoutSegment>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl BuiltInWorkout {
    /// Create a new built-in workout.
    pub fn new(
        title: String,
        description: String,
        category: WorkoutCategory,
        duration_minutes: u16,
        base_tss: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            category,
            energy_systems: Vec::new(),
            goal_alignment: Vec::new(),
            difficulty_tier: DifficultyTier::Moderate,
            duration_minutes,
            base_tss,
            segments: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Set energy systems for this workout.
    pub fn with_energy_systems(mut self, systems: Vec<EnergySystem>) -> Self {
        self.energy_systems = systems;
        self
    }

    /// Set goal alignment.
    pub fn with_goal_alignment(mut self, goals: Vec<String>) -> Self {
        self.goal_alignment = goals;
        self
    }

    /// Set difficulty tier.
    pub fn with_difficulty(mut self, tier: DifficultyTier) -> Self {
        self.difficulty_tier = tier;
        self
    }

    /// Set workout segments.
    pub fn with_segments(mut self, segments: Vec<WorkoutSegment>) -> Self {
        self.segments = segments;
        self
    }
}

/// Workout category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkoutCategory {
    Recovery,
    Endurance,
    SweetSpot,
    Threshold,
    Vo2max,
    Sprint,
    Mixed,
}

impl WorkoutCategory {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            WorkoutCategory::Recovery => "Recovery",
            WorkoutCategory::Endurance => "Endurance",
            WorkoutCategory::SweetSpot => "Sweet Spot",
            WorkoutCategory::Threshold => "Threshold",
            WorkoutCategory::Vo2max => "VO2max",
            WorkoutCategory::Sprint => "Sprint",
            WorkoutCategory::Mixed => "Mixed",
        }
    }

    /// Get all categories.
    pub fn all() -> Vec<WorkoutCategory> {
        vec![
            WorkoutCategory::Recovery,
            WorkoutCategory::Endurance,
            WorkoutCategory::SweetSpot,
            WorkoutCategory::Threshold,
            WorkoutCategory::Vo2max,
            WorkoutCategory::Sprint,
            WorkoutCategory::Mixed,
        ]
    }
}

impl std::fmt::Display for WorkoutCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
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
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
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
        write!(f, "{}", self.display_name())
    }
}

/// Difficulty tier for workouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DifficultyTier {
    Easy,
    Moderate,
    Hard,
    VeryHard,
}

impl DifficultyTier {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            DifficultyTier::Easy => "Easy",
            DifficultyTier::Moderate => "Moderate",
            DifficultyTier::Hard => "Hard",
            DifficultyTier::VeryHard => "Very Hard",
        }
    }

    /// Get numeric difficulty range (1-10 scale).
    pub fn difficulty_range(&self) -> (f32, f32) {
        match self {
            DifficultyTier::Easy => (1.0, 3.0),
            DifficultyTier::Moderate => (3.0, 5.0),
            DifficultyTier::Hard => (5.0, 7.0),
            DifficultyTier::VeryHard => (7.0, 10.0),
        }
    }
}

impl std::fmt::Display for DifficultyTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Built-in workout library.
pub struct WorkoutLibrary<'a> {
    conn: &'a Connection,
}

impl<'a> WorkoutLibrary<'a> {
    /// Create a new workout library.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Seed the library with initial workouts if empty.
    pub fn seed_if_empty(&self) -> Result<usize, LibraryError> {
        let count: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM builtin_workouts", [], |row| {
                    row.get(0)
                })?;

        if count > 0 {
            return Ok(0);
        }

        let workouts = generate_seed_workouts();
        for workout in &workouts {
            self.insert(workout)?;
        }

        Ok(workouts.len())
    }

    /// Insert a workout into the library.
    pub fn insert(&self, workout: &BuiltInWorkout) -> Result<(), LibraryError> {
        self.conn.execute(
            "INSERT INTO builtin_workouts
             (id, title, description, category, energy_systems, goal_alignment,
              difficulty_tier, duration_minutes, base_tss, segments, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                workout.id.to_string(),
                workout.title,
                workout.description,
                format!("{:?}", workout.category),
                serde_json::to_string(&workout.energy_systems)?,
                serde_json::to_string(&workout.goal_alignment)?,
                format!("{:?}", workout.difficulty_tier),
                workout.duration_minutes,
                workout.base_tss,
                serde_json::to_string(&workout.segments)?,
                workout.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get a workout by ID.
    pub fn get(&self, id: Uuid) -> Result<Option<BuiltInWorkout>, LibraryError> {
        self.conn
            .query_row(
                "SELECT id, title, description, category, energy_systems, goal_alignment,
                        difficulty_tier, duration_minutes, base_tss, segments, created_at
                 FROM builtin_workouts WHERE id = ?1",
                params![id.to_string()],
                parse_workout_row,
            )
            .optional()
            .map_err(LibraryError::from)
    }

    /// Get all workouts.
    pub fn get_all(&self) -> Result<Vec<BuiltInWorkout>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, category, energy_systems, goal_alignment,
                    difficulty_tier, duration_minutes, base_tss, segments, created_at
             FROM builtin_workouts ORDER BY category, title",
        )?;

        let rows = stmt.query_map([], parse_workout_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::from)
    }

    /// Search workouts by criteria.
    pub fn search(&self, criteria: &SearchCriteria) -> Result<Vec<BuiltInWorkout>, LibraryError> {
        let mut workouts = self.get_all()?;

        // Filter by category
        if let Some(category) = &criteria.category {
            workouts.retain(|w| &w.category == category);
        }

        // Filter by energy system
        if let Some(energy_system) = &criteria.energy_system {
            workouts.retain(|w| w.energy_systems.contains(energy_system));
        }

        // Filter by max duration
        if let Some(max_duration) = criteria.max_duration_minutes {
            workouts.retain(|w| w.duration_minutes <= max_duration);
        }

        // Filter by difficulty range
        if let Some((min, max)) = criteria.difficulty_range {
            workouts.retain(|w| {
                let (tier_min, tier_max) = w.difficulty_tier.difficulty_range();
                tier_min >= min && tier_max <= max
            });
        }

        // Filter by goal alignment
        if let Some(goal) = &criteria.goal_type {
            workouts.retain(|w| w.goal_alignment.iter().any(|g| g == goal));
        }

        Ok(workouts)
    }

    /// Get workouts by category.
    pub fn get_by_category(
        &self,
        category: WorkoutCategory,
    ) -> Result<Vec<BuiltInWorkout>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, category, energy_systems, goal_alignment,
                    difficulty_tier, duration_minutes, base_tss, segments, created_at
             FROM builtin_workouts WHERE category = ?1 ORDER BY title",
        )?;

        let rows = stmt.query_map(params![format!("{:?}", category)], parse_workout_row)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::from)
    }

    /// Get workout count.
    pub fn count(&self) -> Result<usize, LibraryError> {
        let count: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM builtin_workouts", [], |row| {
                    row.get(0)
                })?;
        Ok(count as usize)
    }
}

/// Search criteria for workouts.
#[derive(Debug, Default)]
pub struct SearchCriteria {
    pub category: Option<WorkoutCategory>,
    pub energy_system: Option<EnergySystem>,
    pub max_duration_minutes: Option<u16>,
    pub difficulty_range: Option<(f32, f32)>,
    pub goal_type: Option<String>,
}

fn parse_workout_row(row: &rusqlite::Row) -> rusqlite::Result<BuiltInWorkout> {
    let id_str: String = row.get(0)?;
    let category_str: String = row.get(3)?;
    let energy_systems_json: String = row.get(4)?;
    let goal_alignment_json: String = row.get(5)?;
    let difficulty_str: String = row.get(6)?;
    let segments_json: String = row.get(9)?;
    let created_at_str: String = row.get(10)?;

    let category = match category_str.as_str() {
        "Recovery" => WorkoutCategory::Recovery,
        "Endurance" => WorkoutCategory::Endurance,
        "SweetSpot" => WorkoutCategory::SweetSpot,
        "Threshold" => WorkoutCategory::Threshold,
        "Vo2max" => WorkoutCategory::Vo2max,
        "Sprint" => WorkoutCategory::Sprint,
        _ => WorkoutCategory::Mixed,
    };

    let difficulty_tier = match difficulty_str.as_str() {
        "Easy" => DifficultyTier::Easy,
        "Moderate" => DifficultyTier::Moderate,
        "Hard" => DifficultyTier::Hard,
        "VeryHard" => DifficultyTier::VeryHard,
        _ => DifficultyTier::Moderate,
    };

    Ok(BuiltInWorkout {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        title: row.get(1)?,
        description: row.get(2)?,
        category,
        energy_systems: serde_json::from_str(&energy_systems_json).unwrap_or_default(),
        goal_alignment: serde_json::from_str(&goal_alignment_json).unwrap_or_default(),
        difficulty_tier,
        duration_minutes: row.get(7)?,
        base_tss: row.get(8)?,
        segments: serde_json::from_str(&segments_json).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

/// Generate the initial set of 80 seed workouts.
fn generate_seed_workouts() -> Vec<BuiltInWorkout> {
    let mut workouts = Vec::with_capacity(80);

    // Recovery workouts (10)
    workouts.extend(generate_recovery_workouts());

    // Endurance workouts (15)
    workouts.extend(generate_endurance_workouts());

    // Sweet Spot workouts (15)
    workouts.extend(generate_sweet_spot_workouts());

    // Threshold workouts (15)
    workouts.extend(generate_threshold_workouts());

    // VO2max workouts (15)
    workouts.extend(generate_vo2max_workouts());

    // Sprint workouts (10)
    workouts.extend(generate_sprint_workouts());

    workouts
}

fn generate_recovery_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "Easy Spin 30min".into(),
            "Light recovery spin".into(),
            WorkoutCategory::Recovery,
            30,
            20.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Recovery 45min".into(),
            "Easy recovery ride".into(),
            WorkoutCategory::Recovery,
            45,
            30.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Active Recovery 60min".into(),
            "Long easy spin".into(),
            WorkoutCategory::Recovery,
            60,
            40.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Legs Opener".into(),
            "Pre-race openers".into(),
            WorkoutCategory::Recovery,
            45,
            35.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery, EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Coffee Ride".into(),
            "Social pace easy ride".into(),
            WorkoutCategory::Recovery,
            60,
            35.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Cool Down 20min".into(),
            "Post-race cooldown".into(),
            WorkoutCategory::Recovery,
            20,
            12.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Recovery Intervals".into(),
            "Easy spin with form focus".into(),
            WorkoutCategory::Recovery,
            40,
            25.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Rest Day Spin".into(),
            "Very easy 20 min".into(),
            WorkoutCategory::Recovery,
            20,
            10.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Flush Ride".into(),
            "Clear legs after hard day".into(),
            WorkoutCategory::Recovery,
            30,
            18.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Light Spin".into(),
            "Minimal effort recovery".into(),
            WorkoutCategory::Recovery,
            25,
            15.0,
        )
        .with_energy_systems(vec![EnergySystem::Recovery])
        .with_difficulty(DifficultyTier::Easy),
    ]
}

fn generate_endurance_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "Endurance 60min Z2".into(),
            "Steady endurance".into(),
            WorkoutCategory::Endurance,
            60,
            50.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Endurance 90min Z2".into(),
            "Long endurance".into(),
            WorkoutCategory::Endurance,
            90,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Endurance 120min".into(),
            "Extended endurance".into(),
            WorkoutCategory::Endurance,
            120,
            100.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Tempo 45min".into(),
            "Tempo ride at 76-90% FTP".into(),
            WorkoutCategory::Endurance,
            45,
            45.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Base Miles 75min".into(),
            "Aerobic base building".into(),
            WorkoutCategory::Endurance,
            75,
            60.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Progression Ride".into(),
            "Build from Z2 to Z3".into(),
            WorkoutCategory::Endurance,
            60,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Aerobic Efficiency".into(),
            "Focus on fat burning".into(),
            WorkoutCategory::Endurance,
            90,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Long Slow Distance".into(),
            "Classic LSD training".into(),
            WorkoutCategory::Endurance,
            150,
            120.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Tempo Intervals".into(),
            "3x15min tempo".into(),
            WorkoutCategory::Endurance,
            75,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Zone 2 Foundation".into(),
            "Pure Z2 work".into(),
            WorkoutCategory::Endurance,
            60,
            45.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Endurance Builder".into(),
            "Progressive Z2 ride".into(),
            WorkoutCategory::Endurance,
            80,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Steady State 70min".into(),
            "Consistent effort".into(),
            WorkoutCategory::Endurance,
            70,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Weekend Warrior".into(),
            "Long weekend ride".into(),
            WorkoutCategory::Endurance,
            180,
            150.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Base Building".into(),
            "Early season base".into(),
            WorkoutCategory::Endurance,
            75,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
        BuiltInWorkout::new(
            "Fatburner".into(),
            "Low intensity high duration".into(),
            WorkoutCategory::Endurance,
            90,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Easy),
    ]
}

fn generate_sweet_spot_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "Sweet Spot 2x20min".into(),
            "Classic SS intervals".into(),
            WorkoutCategory::SweetSpot,
            60,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Sweet Spot 3x15min".into(),
            "SS intervals".into(),
            WorkoutCategory::SweetSpot,
            65,
            72.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Sweet Spot 4x10min".into(),
            "Shorter SS efforts".into(),
            WorkoutCategory::SweetSpot,
            60,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Under/Overs SS".into(),
            "88-92% oscillations".into(),
            WorkoutCategory::SweetSpot,
            60,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Sweet Spot 45min".into(),
            "Continuous SS".into(),
            WorkoutCategory::SweetSpot,
            60,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "SS Progressions".into(),
            "Build through SS zone".into(),
            WorkoutCategory::SweetSpot,
            75,
            78.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Sweet Spot 1x30min".into(),
            "Extended SS effort".into(),
            WorkoutCategory::SweetSpot,
            50,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "SS Base Builder".into(),
            "High volume SS".into(),
            WorkoutCategory::SweetSpot,
            90,
            90.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Over-Unders Light".into(),
            "SS with surges".into(),
            WorkoutCategory::SweetSpot,
            60,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Sweet Spot Tempo".into(),
            "Extended SS tempo".into(),
            WorkoutCategory::SweetSpot,
            70,
            72.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "FTP Builder".into(),
            "SS to raise FTP".into(),
            WorkoutCategory::SweetSpot,
            75,
            80.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "SS Endurance Mix".into(),
            "SS with Z2 recovery".into(),
            WorkoutCategory::SweetSpot,
            90,
            85.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot, EnergySystem::Endurance])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Threshold Prep".into(),
            "SS approach to FTP".into(),
            WorkoutCategory::SweetSpot,
            60,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "Sweet Spot 5x8min".into(),
            "High rep SS".into(),
            WorkoutCategory::SweetSpot,
            65,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Moderate),
        BuiltInWorkout::new(
            "SS Power Builder".into(),
            "Focus on power".into(),
            WorkoutCategory::SweetSpot,
            70,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Hard),
    ]
}

fn generate_threshold_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "Threshold 3x10min".into(),
            "Classic FTP intervals".into(),
            WorkoutCategory::Threshold,
            55,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Threshold 2x20min".into(),
            "Long FTP intervals".into(),
            WorkoutCategory::Threshold,
            60,
            80.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Threshold 4x8min".into(),
            "Medium FTP reps".into(),
            WorkoutCategory::Threshold,
            55,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Over-Unders 3x12min".into(),
            "FTP oscillations".into(),
            WorkoutCategory::Threshold,
            60,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold, EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "FTP 1x30min".into(),
            "Continuous threshold".into(),
            WorkoutCategory::Threshold,
            50,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Threshold Ladder".into(),
            "Progressive FTP".into(),
            WorkoutCategory::Threshold,
            60,
            72.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Criss-Cross".into(),
            "FTP with spikes".into(),
            WorkoutCategory::Threshold,
            55,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold, EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "FTP Test Prep".into(),
            "Pre-test threshold".into(),
            WorkoutCategory::Threshold,
            45,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Threshold 5x6min".into(),
            "High rep FTP".into(),
            WorkoutCategory::Threshold,
            50,
            62.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Race Simulation".into(),
            "TT race effort".into(),
            WorkoutCategory::Threshold,
            60,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Threshold Extension".into(),
            "Build FTP duration".into(),
            WorkoutCategory::Threshold,
            70,
            85.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "FTP Maintenance".into(),
            "Maintain threshold".into(),
            WorkoutCategory::Threshold,
            50,
            60.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Threshold Bursts".into(),
            "Short FTP surges".into(),
            WorkoutCategory::Threshold,
            55,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "20min Test".into(),
            "FTP test protocol".into(),
            WorkoutCategory::Threshold,
            45,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Tempo to FTP".into(),
            "Progressive build".into(),
            WorkoutCategory::Threshold,
            60,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::Threshold, EnergySystem::SweetSpot])
        .with_difficulty(DifficultyTier::Hard),
    ]
}

fn generate_vo2max_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "VO2max 5x4min".into(),
            "Classic VO2 intervals".into(),
            WorkoutCategory::Vo2max,
            55,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max 3x5min".into(),
            "Longer VO2 efforts".into(),
            WorkoutCategory::Vo2max,
            50,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max 6x3min".into(),
            "High rep VO2".into(),
            WorkoutCategory::Vo2max,
            50,
            72.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Billats".into(),
            "30/30 intervals".into(),
            WorkoutCategory::Vo2max,
            45,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "VO2max Pyramid".into(),
            "2-3-4-3-2 min".into(),
            WorkoutCategory::Vo2max,
            55,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max 4x4min".into(),
            "Norwegian style".into(),
            WorkoutCategory::Vo2max,
            50,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "40/20s".into(),
            "Microbursts VO2".into(),
            WorkoutCategory::Vo2max,
            45,
            62.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max, EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "VO2max Builder".into(),
            "Progressive VO2".into(),
            WorkoutCategory::Vo2max,
            55,
            72.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max 8x2min".into(),
            "Short sharp VO2".into(),
            WorkoutCategory::Vo2max,
            50,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Race Pace VO2".into(),
            "Race simulation".into(),
            WorkoutCategory::Vo2max,
            55,
            75.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max 2x8min".into(),
            "Extended VO2".into(),
            WorkoutCategory::Vo2max,
            45,
            62.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Tabata Style".into(),
            "20/10 intervals".into(),
            WorkoutCategory::Vo2max,
            40,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max, EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max Attack".into(),
            "High intensity".into(),
            WorkoutCategory::Vo2max,
            50,
            70.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Climbing Intervals".into(),
            "Simulate climbs".into(),
            WorkoutCategory::Vo2max,
            60,
            78.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max, EnergySystem::Threshold])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "VO2max Intro".into(),
            "Beginner VO2".into(),
            WorkoutCategory::Vo2max,
            45,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::Hard),
    ]
}

fn generate_sprint_workouts() -> Vec<BuiltInWorkout> {
    vec![
        BuiltInWorkout::new(
            "Sprint 6x30s".into(),
            "Classic sprints".into(),
            WorkoutCategory::Sprint,
            45,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Anaerobic, EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Sprint 8x15s".into(),
            "Short sprints".into(),
            WorkoutCategory::Sprint,
            40,
            48.0,
        )
        .with_energy_systems(vec![EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Anaerobic 4x1min".into(),
            "AC intervals".into(),
            WorkoutCategory::Sprint,
            45,
            58.0,
        )
        .with_energy_systems(vec![EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Standing Starts".into(),
            "Power from zero".into(),
            WorkoutCategory::Sprint,
            35,
            40.0,
        )
        .with_energy_systems(vec![EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Sprint Practice".into(),
            "Race sprints".into(),
            WorkoutCategory::Sprint,
            50,
            55.0,
        )
        .with_energy_systems(vec![EnergySystem::Neuromuscular, EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Anaerobic 3x2min".into(),
            "Extended AC".into(),
            WorkoutCategory::Sprint,
            50,
            65.0,
        )
        .with_energy_systems(vec![EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Sprint Ladder".into(),
            "10-20-30-20-10s".into(),
            WorkoutCategory::Sprint,
            40,
            45.0,
        )
        .with_energy_systems(vec![EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Max Power".into(),
            "Peak power focus".into(),
            WorkoutCategory::Sprint,
            35,
            38.0,
        )
        .with_energy_systems(vec![EnergySystem::Neuromuscular])
        .with_difficulty(DifficultyTier::Hard),
        BuiltInWorkout::new(
            "Crit Simulation".into(),
            "Race attacks".into(),
            WorkoutCategory::Sprint,
            55,
            68.0,
        )
        .with_energy_systems(vec![EnergySystem::Anaerobic, EnergySystem::Vo2max])
        .with_difficulty(DifficultyTier::VeryHard),
        BuiltInWorkout::new(
            "Sprint Endurance".into(),
            "Repeat sprints".into(),
            WorkoutCategory::Sprint,
            50,
            60.0,
        )
        .with_energy_systems(vec![EnergySystem::Anaerobic])
        .with_difficulty(DifficultyTier::VeryHard),
    ]
}

/// Library errors.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Workout not found: {0}")]
    NotFound(Uuid),
}

// ========== TrainingPeaks Workout Store (T016) ==========

use crate::workouts::types::{Workout, WorkoutFormat};
use chrono::NaiveDate;

/// Stored workout with TrainingPeaks sync metadata.
#[derive(Debug, Clone)]
pub struct StoredTrainingPeaksWorkout {
    /// The workout data
    pub workout: Workout,
    /// TrainingPeaks external workout ID
    pub external_id: i64,
    /// Platform identifier (always "trainingpeaks")
    pub external_platform: String,
    /// Scheduled date from TrainingPeaks
    pub scheduled_date: Option<NaiveDate>,
    /// Planned TSS from TrainingPeaks
    pub planned_tss: Option<f64>,
    /// Planned IF from TrainingPeaks
    pub planned_if: Option<f64>,
    /// When this workout was synced
    pub synced_at: DateTime<Utc>,
}

/// TrainingPeaks workout sync record.
#[derive(Debug, Clone)]
pub struct TrainingPeaksSyncRecord {
    /// Unique ID
    pub id: Uuid,
    /// External workout ID from TrainingPeaks
    pub external_workout_id: i64,
    /// Local workout ID in our database
    pub local_workout_id: Uuid,
    /// Scheduled date from TrainingPeaks
    pub scheduled_date: Option<String>,
    /// When this was synced
    pub synced_at: DateTime<Utc>,
    /// Last modified date from TrainingPeaks (for detecting updates)
    pub last_modified: Option<String>,
    /// Hash of workout content (for detecting changes)
    pub sync_hash: Option<String>,
}

/// Store for TrainingPeaks-imported workouts.
///
/// Handles persistence of workouts synced from TrainingPeaks with external ID
/// tracking for sync management and duplicate prevention.
pub struct TrainingPeaksWorkoutStore<'a> {
    conn: &'a Connection,
}

impl<'a> TrainingPeaksWorkoutStore<'a> {
    /// Create a new TrainingPeaks workout store.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Save a TrainingPeaks-imported workout to the database.
    ///
    /// This inserts the workout into the workouts table with TrainingPeaks-specific
    /// metadata (external_id, scheduled_date, planned_tss, planned_if) and creates
    /// a sync tracking record.
    pub fn save_imported_workout(
        &self,
        workout: &Workout,
        external_id: i64,
        scheduled_date: Option<NaiveDate>,
        planned_tss: Option<f64>,
        planned_if: Option<f64>,
    ) -> Result<(), LibraryError> {
        let segments_json = serde_json::to_string(&workout.segments)?;
        let tags_json = if workout.tags.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&workout.tags)?)
        };

        let source_format = workout
            .source_format
            .map(|f| format!("{:?}", f).to_lowercase());

        let scheduled_date_str = scheduled_date.map(|d| d.format("%Y-%m-%d").to_string());

        // Insert or update the workout
        self.conn.execute(
            "INSERT INTO workouts (id, name, description, author, source_file, source_format,
             segments_json, total_duration_seconds, estimated_tss, estimated_if, tags_json,
             created_at, external_id, external_platform, scheduled_date, planned_tss, planned_if)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                segments_json = excluded.segments_json,
                total_duration_seconds = excluded.total_duration_seconds,
                estimated_tss = excluded.estimated_tss,
                estimated_if = excluded.estimated_if,
                scheduled_date = excluded.scheduled_date,
                planned_tss = excluded.planned_tss,
                planned_if = excluded.planned_if",
            params![
                workout.id.to_string(),
                workout.name,
                workout.description,
                workout.author,
                workout.source_file,
                source_format,
                segments_json,
                workout.total_duration_seconds,
                workout.estimated_tss,
                workout.estimated_if,
                tags_json,
                workout.created_at.to_rfc3339(),
                external_id.to_string(),
                "trainingpeaks",
                scheduled_date_str,
                planned_tss,
                planned_if,
            ],
        )?;

        // Create or update sync tracking record
        let sync_id = Uuid::new_v4();
        self.conn.execute(
            "INSERT INTO trainingpeaks_workout_sync
             (id, external_workout_id, local_workout_id, scheduled_date, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(external_workout_id) DO UPDATE SET
                local_workout_id = excluded.local_workout_id,
                scheduled_date = excluded.scheduled_date,
                synced_at = excluded.synced_at",
            params![
                sync_id.to_string(),
                external_id,
                workout.id.to_string(),
                scheduled_date_str,
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Check if a workout with the given external ID has already been synced.
    pub fn is_workout_synced(&self, external_id: i64) -> Result<bool, LibraryError> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
            params![external_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get the local workout ID for a synced TrainingPeaks workout.
    pub fn get_local_workout_id(&self, external_id: i64) -> Result<Option<Uuid>, LibraryError> {
        let result: Result<String, rusqlite::Error> = self.conn.query_row(
            "SELECT local_workout_id FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
            params![external_id],
            |row| row.get(0),
        );

        match result {
            Ok(id_str) => {
                let uuid = Uuid::parse_str(&id_str)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        0, rusqlite::types::Type::Text, Box::new(e)
                    ))?;
                Ok(Some(uuid))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(LibraryError::DatabaseError(e)),
        }
    }

    /// Get all TrainingPeaks workouts for a given date range.
    pub fn get_workouts_by_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<StoredTrainingPeaksWorkout>, LibraryError> {
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let end_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.name, w.description, w.author, w.source_file, w.source_format,
                    w.segments_json, w.total_duration_seconds, w.estimated_tss, w.estimated_if,
                    w.tags_json, w.created_at, w.external_id, w.scheduled_date, w.planned_tss,
                    w.planned_if, s.synced_at
             FROM workouts w
             JOIN trainingpeaks_workout_sync s ON w.id = s.local_workout_id
             WHERE w.external_platform = 'trainingpeaks'
               AND w.scheduled_date >= ?1 AND w.scheduled_date <= ?2
             ORDER BY w.scheduled_date ASC",
        )?;

        let rows = stmt.query_map(params![start_str, end_str], |row| {
            self.map_stored_workout_row(row)
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::from)
    }

    /// Get all synced TrainingPeaks workouts.
    pub fn get_all_synced_workouts(&self) -> Result<Vec<StoredTrainingPeaksWorkout>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.name, w.description, w.author, w.source_file, w.source_format,
                    w.segments_json, w.total_duration_seconds, w.estimated_tss, w.estimated_if,
                    w.tags_json, w.created_at, w.external_id, w.scheduled_date, w.planned_tss,
                    w.planned_if, s.synced_at
             FROM workouts w
             JOIN trainingpeaks_workout_sync s ON w.id = s.local_workout_id
             WHERE w.external_platform = 'trainingpeaks'
             ORDER BY w.scheduled_date DESC NULLS LAST",
        )?;

        let rows = stmt.query_map([], |row| self.map_stored_workout_row(row))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::from)
    }

    /// Get a TrainingPeaks workout by its external ID.
    pub fn get_by_external_id(&self, external_id: i64) -> Result<Option<StoredTrainingPeaksWorkout>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.name, w.description, w.author, w.source_file, w.source_format,
                    w.segments_json, w.total_duration_seconds, w.estimated_tss, w.estimated_if,
                    w.tags_json, w.created_at, w.external_id, w.scheduled_date, w.planned_tss,
                    w.planned_if, s.synced_at
             FROM workouts w
             JOIN trainingpeaks_workout_sync s ON w.id = s.local_workout_id
             WHERE w.external_id = ?1 AND w.external_platform = 'trainingpeaks'",
        )?;

        let mut rows = stmt.query_map(params![external_id.to_string()], |row| {
            self.map_stored_workout_row(row)
        })?;

        match rows.next() {
            Some(Ok(workout)) => Ok(Some(workout)),
            Some(Err(e)) => Err(LibraryError::DatabaseError(e)),
            None => Ok(None),
        }
    }

    /// Get sync records for all TrainingPeaks workouts.
    pub fn get_sync_records(&self) -> Result<Vec<TrainingPeaksSyncRecord>, LibraryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, external_workout_id, local_workout_id, scheduled_date,
                    synced_at, last_modified, sync_hash
             FROM trainingpeaks_workout_sync
             ORDER BY synced_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let external_workout_id: i64 = row.get(1)?;
            let local_workout_id_str: String = row.get(2)?;
            let scheduled_date: Option<String> = row.get(3)?;
            let synced_at_str: String = row.get(4)?;
            let last_modified: Option<String> = row.get(5)?;
            let sync_hash: Option<String> = row.get(6)?;

            let id = Uuid::parse_str(&id_str).unwrap_or_default();
            let local_workout_id = Uuid::parse_str(&local_workout_id_str).unwrap_or_default();
            let synced_at = DateTime::parse_from_rfc3339(&synced_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(TrainingPeaksSyncRecord {
                id,
                external_workout_id,
                local_workout_id,
                scheduled_date,
                synced_at,
                last_modified,
                sync_hash,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LibraryError::from)
    }

    /// Get count of synced TrainingPeaks workouts.
    pub fn count_synced(&self) -> Result<usize, LibraryError> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM trainingpeaks_workout_sync",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Delete a TrainingPeaks workout sync record and the associated workout.
    pub fn delete_synced_workout(&self, external_id: i64) -> Result<(), LibraryError> {
        // First get the local workout ID
        if let Some(local_id) = self.get_local_workout_id(external_id)? {
            // Delete from sync tracking table first (due to foreign key)
            self.conn.execute(
                "DELETE FROM trainingpeaks_workout_sync WHERE external_workout_id = ?1",
                params![external_id],
            )?;
            // Delete the workout itself
            self.conn.execute(
                "DELETE FROM workouts WHERE id = ?1",
                params![local_id.to_string()],
            )?;
        }
        Ok(())
    }

    /// Clear all synced TrainingPeaks workouts.
    pub fn clear_all_synced(&self) -> Result<usize, LibraryError> {
        // Get all local workout IDs first
        let mut stmt = self.conn.prepare(
            "SELECT local_workout_id FROM trainingpeaks_workout_sync",
        )?;
        let ids: Vec<String> = stmt.query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete sync records
        let deleted = self.conn.execute(
            "DELETE FROM trainingpeaks_workout_sync",
            [],
        )?;

        // Delete workouts
        for id in ids {
            self.conn.execute(
                "DELETE FROM workouts WHERE id = ?1",
                params![id],
            )?;
        }

        Ok(deleted)
    }

    /// Map a database row to StoredTrainingPeaksWorkout.
    fn map_stored_workout_row(&self, row: &rusqlite::Row) -> rusqlite::Result<StoredTrainingPeaksWorkout> {
        let id_str: String = row.get(0)?;
        let name: String = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let author: Option<String> = row.get(3)?;
        let source_file: Option<String> = row.get(4)?;
        let source_format_str: Option<String> = row.get(5)?;
        let segments_json: String = row.get(6)?;
        let total_duration_seconds: u32 = row.get(7)?;
        let estimated_tss: Option<f32> = row.get(8)?;
        let estimated_if: Option<f32> = row.get(9)?;
        let tags_json: Option<String> = row.get(10)?;
        let created_at_str: String = row.get(11)?;
        let external_id_str: Option<String> = row.get(12)?;
        let scheduled_date_str: Option<String> = row.get(13)?;
        let planned_tss: Option<f64> = row.get(14)?;
        let planned_if: Option<f64> = row.get(15)?;
        let synced_at_str: String = row.get(16)?;

        let id = Uuid::parse_str(&id_str).unwrap_or_default();
        let source_format = source_format_str.and_then(|s| match s.as_str() {
            "trainingpeaks" => Some(WorkoutFormat::TrainingPeaks),
            "zwo" => Some(WorkoutFormat::Zwo),
            "mrc" => Some(WorkoutFormat::Mrc),
            "fit" => Some(WorkoutFormat::Fit),
            "native" => Some(WorkoutFormat::Native),
            _ => None,
        });

        let segments: Vec<super::types::WorkoutSegment> =
            serde_json::from_str(&segments_json).unwrap_or_default();
        let tags: Vec<String> = tags_json
            .map(|json| serde_json::from_str(&json).unwrap_or_default())
            .unwrap_or_default();

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let external_id = external_id_str
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let scheduled_date = scheduled_date_str
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        let synced_at = DateTime::parse_from_rfc3339(&synced_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let workout = Workout {
            id,
            name,
            description,
            author,
            source_file,
            source_format,
            segments,
            total_duration_seconds,
            estimated_tss,
            estimated_if,
            tags,
            created_at,
        };

        Ok(StoredTrainingPeaksWorkout {
            workout,
            external_id,
            external_platform: "trainingpeaks".to_string(),
            scheduled_date,
            planned_tss,
            planned_if,
            synced_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE builtin_workouts (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                energy_systems TEXT NOT NULL,
                goal_alignment TEXT NOT NULL,
                difficulty_tier TEXT NOT NULL,
                duration_minutes INTEGER NOT NULL,
                base_tss REAL NOT NULL,
                segments TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        (file, conn)
    }

    #[test]
    fn test_seed_workouts() {
        let (_file, conn) = setup_test_db();
        let library = WorkoutLibrary::new(&conn);

        let seeded = library.seed_if_empty().unwrap();
        assert_eq!(seeded, 80);

        // Should not seed again
        let seeded_again = library.seed_if_empty().unwrap();
        assert_eq!(seeded_again, 0);
    }

    #[test]
    fn test_get_by_category() {
        let (_file, conn) = setup_test_db();
        let library = WorkoutLibrary::new(&conn);
        library.seed_if_empty().unwrap();

        let recovery = library.get_by_category(WorkoutCategory::Recovery).unwrap();
        assert_eq!(recovery.len(), 10);

        let vo2max = library.get_by_category(WorkoutCategory::Vo2max).unwrap();
        assert_eq!(vo2max.len(), 15);
    }

    #[test]
    fn test_search_workouts() {
        let (_file, conn) = setup_test_db();
        let library = WorkoutLibrary::new(&conn);
        library.seed_if_empty().unwrap();

        let criteria = SearchCriteria {
            category: Some(WorkoutCategory::Threshold),
            max_duration_minutes: Some(60),
            ..Default::default()
        };

        let results = library.search(&criteria).unwrap();
        assert!(!results.is_empty());
        for workout in &results {
            assert_eq!(workout.category, WorkoutCategory::Threshold);
            assert!(workout.duration_minutes <= 60);
        }
    }

    #[test]
    fn test_workout_count() {
        let (_file, conn) = setup_test_db();
        let library = WorkoutLibrary::new(&conn);

        assert_eq!(library.count().unwrap(), 0);

        library.seed_if_empty().unwrap();
        assert_eq!(library.count().unwrap(), 80);
    }

    // ========== TrainingPeaksWorkoutStore Tests (T016) ==========

    fn setup_tp_test_db() -> (NamedTempFile, Connection) {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        // Create both workouts and trainingpeaks_workout_sync tables
        conn.execute_batch(
            r#"
            CREATE TABLE workouts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                author TEXT,
                source_file TEXT,
                source_format TEXT,
                segments_json TEXT NOT NULL,
                total_duration_seconds INTEGER NOT NULL,
                estimated_tss REAL,
                estimated_if REAL,
                tags_json TEXT,
                created_at TEXT NOT NULL,
                external_id TEXT,
                external_platform TEXT,
                scheduled_date TEXT,
                planned_tss REAL,
                planned_if REAL
            );

            CREATE INDEX idx_workouts_external ON workouts(external_platform, external_id);
            CREATE INDEX idx_workouts_scheduled_date ON workouts(scheduled_date);

            CREATE TABLE trainingpeaks_workout_sync (
                id TEXT PRIMARY KEY,
                external_workout_id INTEGER NOT NULL UNIQUE,
                local_workout_id TEXT NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
                scheduled_date TEXT,
                synced_at TEXT NOT NULL,
                last_modified TEXT,
                sync_hash TEXT
            );

            CREATE INDEX idx_tp_workout_sync_external ON trainingpeaks_workout_sync(external_workout_id);
            CREATE INDEX idx_tp_workout_sync_local ON trainingpeaks_workout_sync(local_workout_id);
            "#,
        )
        .unwrap();

        (file, conn)
    }

    fn create_test_workout() -> Workout {
        use crate::workouts::types::{PowerTarget, SegmentType, WorkoutSegment};

        Workout {
            id: Uuid::new_v4(),
            name: "Test Workout".to_string(),
            description: Some("A test workout from TrainingPeaks".to_string()),
            author: Some("Coach".to_string()),
            source_file: None,
            source_format: Some(WorkoutFormat::TrainingPeaks),
            segments: vec![
                WorkoutSegment {
                    segment_type: SegmentType::Warmup,
                    duration_seconds: 300,
                    power_target: PowerTarget::PercentFtp { percent: 50 },
                    cadence_target: None,
                    text_event: None,
                },
                WorkoutSegment {
                    segment_type: SegmentType::SteadyState,
                    duration_seconds: 1200,
                    power_target: PowerTarget::PercentFtp { percent: 90 },
                    cadence_target: None,
                    text_event: None,
                },
            ],
            total_duration_seconds: 1500,
            estimated_tss: Some(45.0),
            estimated_if: Some(0.85),
            tags: vec!["threshold".to_string()],
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_save_imported_workout() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;
        let scheduled_date = NaiveDate::from_ymd_opt(2026, 1, 15);

        let result = store.save_imported_workout(
            &workout,
            external_id,
            scheduled_date,
            Some(45.0),
            Some(0.85),
        );
        assert!(result.is_ok());

        // Verify workout was synced
        assert!(store.is_workout_synced(external_id).unwrap());
        assert_eq!(store.count_synced().unwrap(), 1);
    }

    #[test]
    fn test_is_workout_synced() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;

        // Not synced initially
        assert!(!store.is_workout_synced(external_id).unwrap());

        // After saving, should be synced
        store.save_imported_workout(&workout, external_id, None, None, None).unwrap();
        assert!(store.is_workout_synced(external_id).unwrap());

        // Different ID should not be synced
        assert!(!store.is_workout_synced(99999).unwrap());
    }

    #[test]
    fn test_get_local_workout_id() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;

        // Save the workout
        store.save_imported_workout(&workout, external_id, None, None, None).unwrap();

        // Get the local workout ID
        let local_id = store.get_local_workout_id(external_id).unwrap();
        assert!(local_id.is_some());
        assert_eq!(local_id.unwrap(), workout.id);

        // Non-existent external ID should return None
        let missing = store.get_local_workout_id(99999).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_by_external_id() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;
        let scheduled_date = NaiveDate::from_ymd_opt(2026, 1, 15);

        store.save_imported_workout(
            &workout,
            external_id,
            scheduled_date,
            Some(50.0),
            Some(0.88),
        ).unwrap();

        // Retrieve by external ID
        let retrieved = store.get_by_external_id(external_id).unwrap();
        assert!(retrieved.is_some());

        let stored = retrieved.unwrap();
        assert_eq!(stored.workout.name, "Test Workout");
        assert_eq!(stored.external_id, external_id);
        assert_eq!(stored.scheduled_date, scheduled_date);
        assert_eq!(stored.planned_tss, Some(50.0));
        assert_eq!(stored.planned_if, Some(0.88));
    }

    #[test]
    fn test_get_workouts_by_date_range() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        // Create multiple workouts with different dates
        let workout1 = create_test_workout();
        let mut workout2 = create_test_workout();
        workout2.id = Uuid::new_v4();
        workout2.name = "Workout 2".to_string();
        let mut workout3 = create_test_workout();
        workout3.id = Uuid::new_v4();
        workout3.name = "Workout 3".to_string();

        store.save_imported_workout(
            &workout1, 1001,
            NaiveDate::from_ymd_opt(2026, 1, 10),
            None, None,
        ).unwrap();
        store.save_imported_workout(
            &workout2, 1002,
            NaiveDate::from_ymd_opt(2026, 1, 15),
            None, None,
        ).unwrap();
        store.save_imported_workout(
            &workout3, 1003,
            NaiveDate::from_ymd_opt(2026, 1, 20),
            None, None,
        ).unwrap();

        // Query for mid-range dates
        let start = NaiveDate::from_ymd_opt(2026, 1, 12).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 1, 18).unwrap();
        let results = store.get_workouts_by_date_range(start, end).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].workout.name, "Workout 2");
    }

    #[test]
    fn test_get_all_synced_workouts() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        // Initially empty
        let all = store.get_all_synced_workouts().unwrap();
        assert!(all.is_empty());

        // Add some workouts
        let workout1 = create_test_workout();
        let mut workout2 = create_test_workout();
        workout2.id = Uuid::new_v4();
        workout2.name = "Workout 2".to_string();

        store.save_imported_workout(&workout1, 1001, None, None, None).unwrap();
        store.save_imported_workout(&workout2, 1002, None, None, None).unwrap();

        let all = store.get_all_synced_workouts().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_sync_records() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;
        let scheduled_date = NaiveDate::from_ymd_opt(2026, 1, 15);

        store.save_imported_workout(
            &workout,
            external_id,
            scheduled_date,
            None, None,
        ).unwrap();

        let records = store.get_sync_records().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].external_workout_id, external_id);
        assert_eq!(records[0].local_workout_id, workout.id);
    }

    #[test]
    fn test_delete_synced_workout() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let workout = create_test_workout();
        let external_id = 12345i64;

        store.save_imported_workout(&workout, external_id, None, None, None).unwrap();
        assert!(store.is_workout_synced(external_id).unwrap());
        assert_eq!(store.count_synced().unwrap(), 1);

        // Delete the synced workout
        store.delete_synced_workout(external_id).unwrap();
        assert!(!store.is_workout_synced(external_id).unwrap());
        assert_eq!(store.count_synced().unwrap(), 0);
    }

    #[test]
    fn test_clear_all_synced() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        // Add multiple workouts
        let workout1 = create_test_workout();
        let mut workout2 = create_test_workout();
        workout2.id = Uuid::new_v4();

        store.save_imported_workout(&workout1, 1001, None, None, None).unwrap();
        store.save_imported_workout(&workout2, 1002, None, None, None).unwrap();
        assert_eq!(store.count_synced().unwrap(), 2);

        // Clear all
        let deleted = store.clear_all_synced().unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.count_synced().unwrap(), 0);
    }

    #[test]
    fn test_update_existing_workout_on_resync() {
        let (_file, conn) = setup_tp_test_db();
        let store = TrainingPeaksWorkoutStore::new(&conn);

        let mut workout = create_test_workout();
        let external_id = 12345i64;

        // Initial save
        store.save_imported_workout(
            &workout, external_id,
            NaiveDate::from_ymd_opt(2026, 1, 15),
            Some(50.0), Some(0.85),
        ).unwrap();

        // Update workout and resync
        workout.name = "Updated Workout Name".to_string();
        workout.estimated_tss = Some(60.0);

        store.save_imported_workout(
            &workout, external_id,
            NaiveDate::from_ymd_opt(2026, 1, 16), // Updated date
            Some(60.0), Some(0.90),
        ).unwrap();

        // Should still only have one synced workout
        assert_eq!(store.count_synced().unwrap(), 1);

        // Verify the workout was updated
        let retrieved = store.get_by_external_id(external_id).unwrap().unwrap();
        assert_eq!(retrieved.workout.name, "Updated Workout Name");
        assert_eq!(retrieved.planned_tss, Some(60.0));
        assert_eq!(retrieved.planned_if, Some(0.90));
    }
}
