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
}
