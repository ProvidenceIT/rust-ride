//! Rider type classification based on power profile.
//!
//! T048: Create RiderType enum and classification logic.

use serde::{Deserialize, Serialize};

use super::analysis::{EnergySystem, ProfileAnalysis};

/// Rider type classification based on power profile characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiderType {
    /// Strong at short durations (5-30s): Criterium, track sprints
    Sprinter,
    /// Strong at VO2max durations (3-5min): Short climbs, attacks
    Puncher,
    /// Strong at threshold (20-60min): Time trials, breakaways
    Rouleur,
    /// Strong at long durations with low weight: Mountain stages
    Climber,
    /// Balanced across all durations: Stage racing
    AllRounder,
    /// Not enough data to classify
    Unknown,
}

impl RiderType {
    /// Get display name for the rider type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sprinter => "Sprinter",
            Self::Puncher => "Puncher",
            Self::Rouleur => "Rouleur (Time Trialist)",
            Self::Climber => "Climber",
            Self::AllRounder => "All-Rounder",
            Self::Unknown => "Unclassified",
        }
    }

    /// Get description of the rider type.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sprinter => "Excels at short explosive efforts. Ideal for criteriums, track racing, and field sprints.",
            Self::Puncher => "Strong at mid-range efforts. Great for short climbs, attacks, and aggressive racing.",
            Self::Rouleur => "Strong at sustained threshold power. Suited for time trials and breakaways.",
            Self::Climber => "Excels at long durations with excellent power-to-weight. Ideal for mountain stages.",
            Self::AllRounder => "Balanced power profile across all durations. Adaptable to various race situations.",
            Self::Unknown => "Need more ride data to determine rider type.",
        }
    }

    /// Get recommended training focus for this rider type.
    pub fn training_focus(&self) -> &'static str {
        match self {
            Self::Sprinter => "Focus on threshold and VO2max work to become more versatile.",
            Self::Puncher => "Build anaerobic capacity and sprint power for race-winning attacks.",
            Self::Rouleur => "Develop VO2max for more explosive power on climbs.",
            Self::Climber => "Build sprint power for stage finishes and attacks.",
            Self::AllRounder => "Specialize based on goal events - develop your weakest system.",
            Self::Unknown => "Complete more rides to gather power data for analysis.",
        }
    }

    /// Get all rider types.
    pub fn all() -> &'static [RiderType] {
        &[
            Self::Sprinter,
            Self::Puncher,
            Self::Rouleur,
            Self::Climber,
            Self::AllRounder,
        ]
    }

    /// Classify rider type from a profile analysis.
    pub fn from_analysis(analysis: &ProfileAnalysis, watts_per_kg_ftp: Option<f64>) -> Self {
        // Need at least some data
        if analysis.duration_analyses.is_empty() {
            return Self::Unknown;
        }

        // Check for climber: high W/kg FTP (>4.0 W/kg)
        if let Some(wpk) = watts_per_kg_ftp {
            if wpk > 4.0 {
                // Climbers have high W/kg and strong threshold
                if matches!(analysis.primary_strength, Some(EnergySystem::Threshold)) {
                    return Self::Climber;
                }
            }
        }

        // Check primary strength/weakness
        match (analysis.primary_strength, analysis.primary_weakness) {
            // Strong neuromuscular = Sprinter
            (Some(EnergySystem::Neuromuscular), _) => Self::Sprinter,

            // Strong anaerobic = Puncher (if not also weak at threshold)
            (Some(EnergySystem::Anaerobic), Some(EnergySystem::Threshold)) => Self::Puncher,
            (Some(EnergySystem::Anaerobic), _) => Self::Puncher,

            // Strong aerobic/VO2max = Puncher
            (Some(EnergySystem::Aerobic), _) => Self::Puncher,

            // Strong threshold = Rouleur
            (Some(EnergySystem::Threshold), _) => Self::Rouleur,

            // No clear strength = All-Rounder
            (None, None) => Self::AllRounder,

            // Fallback
            _ => Self::AllRounder,
        }
    }

    /// Classify based on profile scores directly.
    /// Uses the deviation scores for each energy system.
    pub fn from_energy_system_scores(
        neuromuscular: f64,
        anaerobic: f64,
        aerobic: f64,
        threshold: f64,
        watts_per_kg_ftp: Option<f64>,
    ) -> Self {
        // Check for climber first (high W/kg + strong threshold)
        if let Some(wpk) = watts_per_kg_ftp {
            if wpk > 4.0 && threshold > 5.0 {
                return Self::Climber;
            }
        }

        // Find the highest score
        let scores = [
            (EnergySystem::Neuromuscular, neuromuscular),
            (EnergySystem::Anaerobic, anaerobic),
            (EnergySystem::Aerobic, aerobic),
            (EnergySystem::Threshold, threshold),
        ];

        let max_score = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(system, score)| (*system, *score));

        let min_score = scores
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, score)| *score);

        // Check if profile is balanced (all within 10%)
        if let (Some((_, max)), Some(min)) = (max_score, min_score) {
            if (max - min).abs() < 10.0 {
                return Self::AllRounder;
            }
        }

        // Classify by primary strength
        match max_score {
            Some((EnergySystem::Neuromuscular, s)) if s > 5.0 => Self::Sprinter,
            Some((EnergySystem::Anaerobic, s)) if s > 5.0 => Self::Puncher,
            Some((EnergySystem::Aerobic, s)) if s > 5.0 => Self::Puncher,
            Some((EnergySystem::Threshold, s)) if s > 5.0 => Self::Rouleur,
            _ => Self::AllRounder,
        }
    }
}

/// Detailed rider classification with confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiderClassification {
    /// Primary rider type.
    pub rider_type: RiderType,
    /// Confidence in classification (0.0 - 1.0).
    pub confidence: f32,
    /// Secondary type (if any).
    pub secondary_type: Option<RiderType>,
    /// W/kg at FTP (if known).
    pub watts_per_kg: Option<f64>,
    /// Energy system scores.
    pub system_scores: Vec<(EnergySystem, f64)>,
}

impl RiderClassification {
    /// Create classification from profile analysis.
    pub fn from_analysis(analysis: &ProfileAnalysis, weight_kg: Option<f64>) -> Self {
        // Calculate W/kg at FTP
        let watts_per_kg = analysis
            .estimated_ftp
            .and_then(|ftp| weight_kg.map(|w| ftp as f64 / w));

        // Get energy system scores
        let mut system_scores = Vec::new();
        for system in EnergySystem::all() {
            if let Some(score) = analysis.energy_system_score(*system) {
                system_scores.push((*system, score));
            }
        }

        // Sort by score descending
        system_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Classify primary type
        let rider_type = RiderType::from_analysis(analysis, watts_per_kg);

        // Calculate confidence based on score spread
        let confidence = if system_scores.len() >= 2 {
            let top_score = system_scores[0].1;
            let second_score = system_scores[1].1;
            let spread = (top_score - second_score).abs();

            // More spread = more confidence, clamped 0.5 - 1.0
            ((spread / 20.0) + 0.5).clamp(0.5, 1.0) as f32
        } else {
            0.5
        };

        // Determine secondary type if scores are close
        let secondary_type = if system_scores.len() >= 2 {
            let spread = (system_scores[0].1 - system_scores[1].1).abs();
            if spread < 10.0 {
                // Create a mock analysis for second type
                let second_system = system_scores[1].0;
                match second_system {
                    EnergySystem::Neuromuscular => Some(RiderType::Sprinter),
                    EnergySystem::Anaerobic | EnergySystem::Aerobic => Some(RiderType::Puncher),
                    EnergySystem::Threshold => Some(RiderType::Rouleur),
                }
            } else {
                None
            }
        } else {
            None
        };

        Self {
            rider_type,
            confidence,
            secondary_type,
            watts_per_kg,
            system_scores,
        }
    }

    /// Check if classification has high confidence.
    pub fn is_confident(&self) -> bool {
        self.confidence > 0.7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rider_type_display() {
        assert_eq!(RiderType::Sprinter.display_name(), "Sprinter");
        assert_eq!(RiderType::Climber.display_name(), "Climber");
        assert_eq!(RiderType::AllRounder.display_name(), "All-Rounder");
    }

    #[test]
    fn test_classification_from_scores_sprinter() {
        let rider_type = RiderType::from_energy_system_scores(
            20.0,  // Neuromuscular - very strong
            10.0,  // Anaerobic - strong
            -5.0,  // Aerobic - average
            -10.0, // Threshold - weak
            None,
        );
        assert_eq!(rider_type, RiderType::Sprinter);
    }

    #[test]
    fn test_classification_from_scores_rouleur() {
        let rider_type = RiderType::from_energy_system_scores(
            -5.0, // Neuromuscular - average
            0.0,  // Anaerobic - average
            5.0,  // Aerobic - slightly strong
            15.0, // Threshold - strong
            None,
        );
        assert_eq!(rider_type, RiderType::Rouleur);
    }

    #[test]
    fn test_classification_from_scores_climber() {
        let rider_type = RiderType::from_energy_system_scores(
            -10.0,     // Neuromuscular - weak
            -5.0,      // Anaerobic - average
            5.0,       // Aerobic - strong
            10.0,      // Threshold - strong
            Some(4.5), // High W/kg
        );
        assert_eq!(rider_type, RiderType::Climber);
    }

    #[test]
    fn test_classification_from_scores_allrounder() {
        let rider_type = RiderType::from_energy_system_scores(
            2.0,  // All within
            0.0,  // 10% of
            -2.0, // each other
            1.0, None,
        );
        assert_eq!(rider_type, RiderType::AllRounder);
    }
}
