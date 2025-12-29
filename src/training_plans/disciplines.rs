//! Training discipline definitions.

use serde::{Deserialize, Serialize};

/// Cycling discipline for plan specialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Discipline {
    /// Road racing and criteriums
    Road,
    /// Gravel and adventure riding
    Gravel,
    /// Triathlon (bike-focused)
    Triathlon,
    /// Mountain biking
    MTB,
    /// General fitness and base building
    GeneralFitness,
}

impl Discipline {
    /// Get all discipline variants.
    pub fn all() -> &'static [Discipline] {
        &[
            Self::Road,
            Self::Gravel,
            Self::Triathlon,
            Self::MTB,
            Self::GeneralFitness,
        ]
    }

    /// Get display name for the discipline.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Road => "Road Racing",
            Self::Gravel => "Gravel",
            Self::Triathlon => "Triathlon",
            Self::MTB => "Mountain Bike",
            Self::GeneralFitness => "General Fitness",
        }
    }

    /// Get short name for the discipline.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Road => "Road",
            Self::Gravel => "Gravel",
            Self::Triathlon => "Tri",
            Self::MTB => "MTB",
            Self::GeneralFitness => "Fitness",
        }
    }

    /// Get description of the discipline's training focus.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Road => "High-intensity intervals, VO2max work, and threshold training for road racing and criteriums.",
            Self::Gravel => "Muscular endurance, tempo efforts, and sustained power for gravel and adventure rides.",
            Self::Triathlon => "Aerobic base building with brick workout suggestions for triathlon preparation.",
            Self::MTB => "Short power bursts, recovery skills, and technical terrain preparation for mountain biking.",
            Self::GeneralFitness => "Balanced fitness development with mixed intensity for overall health and cycling enjoyment.",
        }
    }
}

/// Plan difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DifficultyLevel {
    /// Beginner: 3-4 hours/week, lower intensity
    Beginner,
    /// Intermediate: 5-8 hours/week, mixed intensity
    Intermediate,
    /// Advanced: 8-12+ hours/week, high intensity
    Advanced,
}

impl DifficultyLevel {
    /// Get all difficulty levels.
    pub fn all() -> &'static [DifficultyLevel] {
        &[Self::Beginner, Self::Intermediate, Self::Advanced]
    }

    /// Get display name for the difficulty level.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Advanced => "Advanced",
        }
    }

    /// Get typical weekly hours for this difficulty level.
    pub fn weekly_hours_range(&self) -> (f32, f32) {
        match self {
            Self::Beginner => (3.0, 4.0),
            Self::Intermediate => (5.0, 8.0),
            Self::Advanced => (8.0, 12.0),
        }
    }

    /// Get description of the difficulty level.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Beginner => "Perfect for those new to structured training. Lower volume with focus on building base fitness.",
            Self::Intermediate => "For riders with some training experience. Mixed intensity with moderate volume.",
            Self::Advanced => "For experienced cyclists seeking peak performance. High volume and intensity.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_disciplines() {
        let disciplines = Discipline::all();
        assert_eq!(disciplines.len(), 5);
    }

    #[test]
    fn test_discipline_display_names() {
        assert_eq!(Discipline::Road.display_name(), "Road Racing");
        assert_eq!(Discipline::MTB.display_name(), "Mountain Bike");
    }

    #[test]
    fn test_difficulty_ordering() {
        assert!(DifficultyLevel::Beginner < DifficultyLevel::Intermediate);
        assert!(DifficultyLevel::Intermediate < DifficultyLevel::Advanced);
    }

    #[test]
    fn test_weekly_hours() {
        let (min, max) = DifficultyLevel::Beginner.weekly_hours_range();
        assert!(min < max);
        assert!(min >= 3.0);
        assert!(max <= 4.0);
    }
}
