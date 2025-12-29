//! Power profile analysis for strength/weakness identification.
//!
//! T047: Create ProfileAnalysis and DurationStrength structs.

use serde::{Deserialize, Serialize};

use super::profile::PowerProfile;
use super::types::PROFILE_DURATIONS;

/// Energy system category for power classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnergySystem {
    /// 5-15 seconds: Neuromuscular/sprint power
    Neuromuscular,
    /// 30-60 seconds: Anaerobic capacity
    Anaerobic,
    /// 3-5 minutes: VO2max / aerobic power
    Aerobic,
    /// 20-60 minutes: Threshold / endurance
    Threshold,
}

impl EnergySystem {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Neuromuscular => "Neuromuscular (Sprint)",
            Self::Anaerobic => "Anaerobic Capacity",
            Self::Aerobic => "VO2max (Aerobic)",
            Self::Threshold => "Threshold (Endurance)",
        }
    }

    /// Get short name.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Neuromuscular => "Sprint",
            Self::Anaerobic => "Anaerobic",
            Self::Aerobic => "VO2max",
            Self::Threshold => "Threshold",
        }
    }

    /// Get durations associated with this energy system.
    pub fn durations(&self) -> &[u32] {
        match self {
            Self::Neuromuscular => &[5, 15],
            Self::Anaerobic => &[30, 60],
            Self::Aerobic => &[180, 300],
            Self::Threshold => &[600, 1200, 3600],
        }
    }

    /// Get all energy systems.
    pub fn all() -> &'static [EnergySystem] {
        &[
            Self::Neuromuscular,
            Self::Anaerobic,
            Self::Aerobic,
            Self::Threshold,
        ]
    }

    /// Classify a duration into an energy system.
    pub fn from_duration(duration_secs: u32) -> Self {
        if duration_secs <= 15 {
            Self::Neuromuscular
        } else if duration_secs <= 60 {
            Self::Anaerobic
        } else if duration_secs <= 300 {
            Self::Aerobic
        } else {
            Self::Threshold
        }
    }
}

/// Strength classification for a duration or energy system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrengthLevel {
    /// Much below average (<-15%)
    VeryWeak,
    /// Below average (-5% to -15%)
    Weak,
    /// Average (-5% to +5%)
    Average,
    /// Above average (+5% to +15%)
    Strong,
    /// Much above average (>+15%)
    VeryStrong,
}

impl StrengthLevel {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::VeryWeak => "Very Weak",
            Self::Weak => "Weak",
            Self::Average => "Average",
            Self::Strong => "Strong",
            Self::VeryStrong => "Very Strong",
        }
    }

    /// Get color hint for UI (0.0-1.0 scale, 0=weak, 1=strong).
    pub fn color_value(&self) -> f32 {
        match self {
            Self::VeryWeak => 0.0,
            Self::Weak => 0.25,
            Self::Average => 0.5,
            Self::Strong => 0.75,
            Self::VeryStrong => 1.0,
        }
    }

    /// Create from percentage deviation from average.
    pub fn from_deviation(deviation_percent: f64) -> Self {
        if deviation_percent < -15.0 {
            Self::VeryWeak
        } else if deviation_percent < -5.0 {
            Self::Weak
        } else if deviation_percent <= 5.0 {
            Self::Average
        } else if deviation_percent <= 15.0 {
            Self::Strong
        } else {
            Self::VeryStrong
        }
    }
}

/// Strength analysis for a single duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStrength {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Power at this duration.
    pub power_watts: u16,
    /// Watts per kg (if weight known).
    pub watts_per_kg: Option<f64>,
    /// Deviation from the profile's balanced curve (%).
    pub deviation_percent: f64,
    /// Strength classification.
    pub strength_level: StrengthLevel,
    /// Energy system this belongs to.
    pub energy_system: EnergySystem,
}

impl DurationStrength {
    /// Check if this is a strength (above average).
    pub fn is_strength(&self) -> bool {
        matches!(
            self.strength_level,
            StrengthLevel::Strong | StrengthLevel::VeryStrong
        )
    }

    /// Check if this is a weakness (below average).
    pub fn is_weakness(&self) -> bool {
        matches!(
            self.strength_level,
            StrengthLevel::Weak | StrengthLevel::VeryWeak
        )
    }
}

/// Complete analysis of a power profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAnalysis {
    /// Analysis of each duration.
    pub duration_analyses: Vec<DurationStrength>,
    /// Primary strength (best energy system).
    pub primary_strength: Option<EnergySystem>,
    /// Primary weakness (worst energy system).
    pub primary_weakness: Option<EnergySystem>,
    /// Overall fitness score (based on all durations).
    pub fitness_score: f64,
    /// Estimated FTP from profile.
    pub estimated_ftp: Option<u16>,
    /// Balanced power at each duration (for comparison).
    pub balanced_curve: Vec<(u32, u16)>,
}

impl ProfileAnalysis {
    /// Analyze a power profile.
    pub fn from_profile(profile: &PowerProfile, weight_kg: Option<f64>) -> Self {
        // Build the actual curve from profile
        let mut duration_analyses = Vec::new();

        // Calculate FTP for baseline
        let ftp = profile.estimated_ftp().unwrap_or(200);

        // Generate balanced curve based on FTP
        let balanced_curve = generate_balanced_curve(ftp);

        // Analyze each standard duration
        for &duration in &PROFILE_DURATIONS {
            if let Some(power) = profile.power_at_duration(duration) {
                // Find expected power from balanced curve
                let expected = balanced_curve
                    .iter()
                    .find(|(d, _)| *d == duration)
                    .map(|(_, p)| *p)
                    .unwrap_or(power);

                let deviation = if expected > 0 {
                    ((power as f64 - expected as f64) / expected as f64) * 100.0
                } else {
                    0.0
                };

                let wpk = weight_kg.map(|w| power as f64 / w);
                let energy_system = EnergySystem::from_duration(duration);

                duration_analyses.push(DurationStrength {
                    duration_secs: duration,
                    power_watts: power,
                    watts_per_kg: wpk,
                    deviation_percent: deviation,
                    strength_level: StrengthLevel::from_deviation(deviation),
                    energy_system,
                });
            }
        }

        // Find primary strength and weakness by energy system
        let (primary_strength, primary_weakness) =
            find_primary_strength_weakness(&duration_analyses);

        // Calculate fitness score (average deviation from balanced curve)
        let fitness_score = calculate_fitness_score(&duration_analyses);

        Self {
            duration_analyses,
            primary_strength,
            primary_weakness,
            fitness_score,
            estimated_ftp: profile.estimated_ftp(),
            balanced_curve,
        }
    }

    /// Get strengths (durations significantly above average).
    pub fn get_strengths(&self) -> Vec<&DurationStrength> {
        self.duration_analyses
            .iter()
            .filter(|d| d.is_strength())
            .collect()
    }

    /// Get weaknesses (durations significantly below average).
    pub fn get_weaknesses(&self) -> Vec<&DurationStrength> {
        self.duration_analyses
            .iter()
            .filter(|d| d.is_weakness())
            .collect()
    }

    /// Get analysis for a specific duration.
    pub fn get_duration(&self, duration_secs: u32) -> Option<&DurationStrength> {
        self.duration_analyses
            .iter()
            .find(|d| d.duration_secs == duration_secs)
    }

    /// Get average deviation for an energy system.
    pub fn energy_system_score(&self, system: EnergySystem) -> Option<f64> {
        let relevant: Vec<_> = self
            .duration_analyses
            .iter()
            .filter(|d| d.energy_system == system)
            .collect();

        if relevant.is_empty() {
            return None;
        }

        let avg = relevant.iter().map(|d| d.deviation_percent).sum::<f64>() / relevant.len() as f64;
        Some(avg)
    }
}

/// Generate a balanced power curve based on FTP.
/// Uses approximate power law decay from sprint to threshold.
fn generate_balanced_curve(ftp: u16) -> Vec<(u32, u16)> {
    // Power decay coefficients (approximate)
    // Based on typical rider profile where FTP is ~95% of 20-min power
    // and sprint power is roughly 3x FTP
    let ftp_f = ftp as f64;

    PROFILE_DURATIONS
        .iter()
        .map(|&duration| {
            let power = match duration {
                5 => ftp_f * 2.8,     // ~280% FTP
                15 => ftp_f * 2.2,    // ~220% FTP
                30 => ftp_f * 1.8,    // ~180% FTP
                60 => ftp_f * 1.5,    // ~150% FTP
                180 => ftp_f * 1.25,  // ~125% FTP
                300 => ftp_f * 1.15,  // ~115% FTP (VO2max)
                600 => ftp_f * 1.08,  // ~108% FTP
                1200 => ftp_f * 1.05, // ~105% FTP (20-min gives FTP)
                3600 => ftp_f * 0.95, // ~95% FTP (hour power)
                _ => ftp_f,
            };
            (duration, power.round() as u16)
        })
        .collect()
}

/// Find primary strength and weakness energy systems.
fn find_primary_strength_weakness(
    analyses: &[DurationStrength],
) -> (Option<EnergySystem>, Option<EnergySystem>) {
    let mut system_scores: Vec<(EnergySystem, f64)> = Vec::new();

    for system in EnergySystem::all() {
        let relevant: Vec<_> = analyses
            .iter()
            .filter(|d| d.energy_system == *system)
            .collect();
        if !relevant.is_empty() {
            let avg =
                relevant.iter().map(|d| d.deviation_percent).sum::<f64>() / relevant.len() as f64;
            system_scores.push((*system, avg));
        }
    }

    if system_scores.is_empty() {
        return (None, None);
    }

    system_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let strength = system_scores.first().map(|(s, _)| *s);
    let weakness = system_scores.last().map(|(s, _)| *s);

    (strength, weakness)
}

/// Calculate overall fitness score from analyses.
fn calculate_fitness_score(analyses: &[DurationStrength]) -> f64 {
    if analyses.is_empty() {
        return 0.0;
    }

    // Base score of 50, adjusted by average deviation
    let avg_deviation =
        analyses.iter().map(|d| d.deviation_percent).sum::<f64>() / analyses.len() as f64;

    // Clamp between 0 and 100
    (50.0 + avg_deviation).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power_profile::profile::PowerProfilePoint;
    use uuid::Uuid;

    #[test]
    fn test_energy_system_classification() {
        assert_eq!(EnergySystem::from_duration(5), EnergySystem::Neuromuscular);
        assert_eq!(EnergySystem::from_duration(15), EnergySystem::Neuromuscular);
        assert_eq!(EnergySystem::from_duration(30), EnergySystem::Anaerobic);
        assert_eq!(EnergySystem::from_duration(60), EnergySystem::Anaerobic);
        assert_eq!(EnergySystem::from_duration(180), EnergySystem::Aerobic);
        assert_eq!(EnergySystem::from_duration(300), EnergySystem::Aerobic);
        assert_eq!(EnergySystem::from_duration(1200), EnergySystem::Threshold);
    }

    #[test]
    fn test_strength_level_from_deviation() {
        assert_eq!(
            StrengthLevel::from_deviation(-20.0),
            StrengthLevel::VeryWeak
        );
        assert_eq!(StrengthLevel::from_deviation(-10.0), StrengthLevel::Weak);
        assert_eq!(StrengthLevel::from_deviation(0.0), StrengthLevel::Average);
        assert_eq!(StrengthLevel::from_deviation(10.0), StrengthLevel::Strong);
        assert_eq!(
            StrengthLevel::from_deviation(20.0),
            StrengthLevel::VeryStrong
        );
    }

    #[test]
    fn test_balanced_curve_generation() {
        let curve = generate_balanced_curve(250);

        // Check 5-second power is highest
        let p5 = curve
            .iter()
            .find(|(d, _)| *d == 5)
            .map(|(_, p)| *p)
            .unwrap();
        let p3600 = curve
            .iter()
            .find(|(d, _)| *d == 3600)
            .map(|(_, p)| *p)
            .unwrap();

        assert!(p5 > p3600, "5s power should be higher than 60min power");

        // Check 20-min power is close to FTP
        let p1200 = curve
            .iter()
            .find(|(d, _)| *d == 1200)
            .map(|(_, p)| *p)
            .unwrap();
        assert!(
            (p1200 as i32 - 263).abs() < 10,
            "20-min should be ~105% of FTP"
        );
    }

    #[test]
    fn test_profile_analysis() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, super::super::types::ProfileType::Current);

        // Add points for a "sprinter" profile (strong short, weak long)
        profile.update_point(PowerProfilePoint::new(5, 900)); // Very strong sprint
        profile.update_point(PowerProfilePoint::new(15, 650)); // Strong
        profile.update_point(PowerProfilePoint::new(60, 350)); // Average
        profile.update_point(PowerProfilePoint::new(300, 280)); // Weak VO2max
        profile.update_point(PowerProfilePoint::new(1200, 240)); // Weak FTP

        let analysis = ProfileAnalysis::from_profile(&profile, Some(70.0));

        // Should identify neuromuscular as primary strength
        assert!(analysis.primary_strength.is_some());

        // Should have some strengths and weaknesses
        assert!(!analysis.get_strengths().is_empty() || !analysis.get_weaknesses().is_empty());

        // FTP estimate should be based on 20-min power
        assert!(analysis.estimated_ftp.is_some());
    }
}
