//! Reference curve comparison for power profile analysis.
//!
//! T053: Implement reference curve comparison for analysis.

use serde::{Deserialize, Serialize};

use super::analysis::EnergySystem;
use super::profile::PowerProfile;
use super::types::PROFILE_DURATIONS;

/// Reference power curves for different training levels.
///
/// Based on Training Peaks power profile zones.
/// Values are in W/kg at each duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceLevel {
    /// Untrained: little to no cycling background
    Untrained,
    /// Recreational: casual riders, fitness focus
    Recreational,
    /// Trained: regular training, club level
    Trained,
    /// Competitive: racing at local/regional level
    Competitive,
    /// Elite: national/professional level
    Elite,
    /// World Class: international/world tour level
    WorldClass,
}

impl ReferenceLevel {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Untrained => "Untrained",
            Self::Recreational => "Recreational",
            Self::Trained => "Trained",
            Self::Competitive => "Competitive",
            Self::Elite => "Elite",
            Self::WorldClass => "World Class",
        }
    }

    /// Get all levels in order.
    pub fn all() -> &'static [ReferenceLevel] {
        &[
            Self::Untrained,
            Self::Recreational,
            Self::Trained,
            Self::Competitive,
            Self::Elite,
            Self::WorldClass,
        ]
    }

    /// Get the next level up.
    pub fn next_level(&self) -> Option<ReferenceLevel> {
        match self {
            Self::Untrained => Some(Self::Recreational),
            Self::Recreational => Some(Self::Trained),
            Self::Trained => Some(Self::Competitive),
            Self::Competitive => Some(Self::Elite),
            Self::Elite => Some(Self::WorldClass),
            Self::WorldClass => None,
        }
    }
}

/// Reference W/kg values for male riders.
///
/// Based on Training Peaks power profile data.
/// Returns (5s, 60s, 5min, 20min) W/kg values for each level.
pub fn male_reference_wpk(level: ReferenceLevel) -> ReferenceCurve {
    let (p5, p60, p300, p1200) = match level {
        ReferenceLevel::Untrained => (10.0, 5.5, 3.0, 2.0),
        ReferenceLevel::Recreational => (14.0, 7.5, 4.0, 3.0),
        ReferenceLevel::Trained => (17.0, 9.0, 5.0, 4.0),
        ReferenceLevel::Competitive => (20.0, 10.5, 5.5, 4.5),
        ReferenceLevel::Elite => (23.0, 12.0, 6.0, 5.0),
        ReferenceLevel::WorldClass => (26.0, 13.5, 6.8, 6.0),
    };

    ReferenceCurve::new(level, p5, p60, p300, p1200)
}

/// Reference W/kg values for female riders.
///
/// Approximately 10-15% lower than male reference values.
pub fn female_reference_wpk(level: ReferenceLevel) -> ReferenceCurve {
    let (p5, p60, p300, p1200) = match level {
        ReferenceLevel::Untrained => (8.5, 4.7, 2.6, 1.7),
        ReferenceLevel::Recreational => (11.9, 6.4, 3.4, 2.6),
        ReferenceLevel::Trained => (14.5, 7.7, 4.3, 3.4),
        ReferenceLevel::Competitive => (17.0, 8.9, 4.7, 3.8),
        ReferenceLevel::Elite => (19.6, 10.2, 5.1, 4.3),
        ReferenceLevel::WorldClass => (22.1, 11.5, 5.8, 5.1),
    };

    ReferenceCurve::new(level, p5, p60, p300, p1200)
}

/// A reference power curve at a specific level.
#[derive(Debug, Clone)]
pub struct ReferenceCurve {
    /// Training level.
    pub level: ReferenceLevel,
    /// W/kg at 5 seconds.
    pub wpk_5s: f64,
    /// W/kg at 1 minute.
    pub wpk_60s: f64,
    /// W/kg at 5 minutes.
    pub wpk_300s: f64,
    /// W/kg at 20 minutes (FTP proxy).
    pub wpk_1200s: f64,
}

impl ReferenceCurve {
    /// Create a new reference curve.
    pub fn new(
        level: ReferenceLevel,
        wpk_5s: f64,
        wpk_60s: f64,
        wpk_300s: f64,
        wpk_1200s: f64,
    ) -> Self {
        Self {
            level,
            wpk_5s,
            wpk_60s,
            wpk_300s,
            wpk_1200s,
        }
    }

    /// Get W/kg at a specific duration using interpolation.
    pub fn wpk_at(&self, duration_secs: u32) -> f64 {
        match duration_secs {
            d if d <= 5 => self.wpk_5s,
            d if d <= 15 => interpolate(5, self.wpk_5s, 60, self.wpk_60s, d),
            d if d <= 30 => interpolate(5, self.wpk_5s, 60, self.wpk_60s, d),
            d if d <= 60 => interpolate(5, self.wpk_5s, 60, self.wpk_60s, d),
            d if d <= 180 => interpolate(60, self.wpk_60s, 300, self.wpk_300s, d),
            d if d <= 300 => interpolate(60, self.wpk_60s, 300, self.wpk_300s, d),
            d if d <= 600 => interpolate(300, self.wpk_300s, 1200, self.wpk_1200s, d),
            d if d <= 1200 => interpolate(300, self.wpk_300s, 1200, self.wpk_1200s, d),
            _ => self.wpk_1200s * 0.95, // Approximate for longer durations
        }
    }

    /// Convert to absolute watts given weight.
    pub fn watts_at(&self, duration_secs: u32, weight_kg: f64) -> u16 {
        (self.wpk_at(duration_secs) * weight_kg).round() as u16
    }

    /// Generate full curve at standard durations.
    pub fn full_curve(&self, weight_kg: f64) -> Vec<(u32, u16)> {
        PROFILE_DURATIONS
            .iter()
            .map(|&d| (d, self.watts_at(d, weight_kg)))
            .collect()
    }
}

/// Linear interpolation helper.
fn interpolate(d1: u32, v1: f64, d2: u32, v2: f64, d: u32) -> f64 {
    let ratio = (d - d1) as f64 / (d2 - d1) as f64;
    v1 + ratio * (v2 - v1)
}

/// Result of comparing a profile to reference curves.
#[derive(Debug, Clone)]
pub struct ProfileComparison {
    /// Comparison at each duration.
    pub duration_comparisons: Vec<DurationComparison>,
    /// Overall level (based on FTP/20-min).
    pub overall_level: ReferenceLevel,
    /// Level at each energy system.
    pub system_levels: Vec<(EnergySystem, ReferenceLevel)>,
}

/// Comparison at a single duration.
#[derive(Debug, Clone)]
pub struct DurationComparison {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Actual power.
    pub actual_power: u16,
    /// Actual W/kg.
    pub actual_wpk: f64,
    /// Reference level this matches.
    pub level: ReferenceLevel,
    /// Percentage into the next level (0-100%).
    pub progress_to_next: f64,
}

impl DurationComparison {
    /// Get the energy system for this duration.
    pub fn energy_system(&self) -> EnergySystem {
        EnergySystem::from_duration(self.duration_secs)
    }
}

/// Compare a power profile against reference curves.
pub struct ProfileComparer {
    /// User's weight in kg.
    weight_kg: f64,
    /// Gender for reference selection.
    use_female_reference: bool,
}

impl ProfileComparer {
    /// Create a new comparer.
    pub fn new(weight_kg: f64, use_female_reference: bool) -> Self {
        Self {
            weight_kg,
            use_female_reference,
        }
    }

    /// Compare a profile against reference curves.
    pub fn compare(&self, profile: &PowerProfile) -> ProfileComparison {
        let mut duration_comparisons = Vec::new();

        for &duration in &PROFILE_DURATIONS {
            if let Some(power) = profile.power_at_duration(duration) {
                let wpk = power as f64 / self.weight_kg;
                let (level, progress) = self.find_level_at(duration, wpk);

                duration_comparisons.push(DurationComparison {
                    duration_secs: duration,
                    actual_power: power,
                    actual_wpk: wpk,
                    level,
                    progress_to_next: progress,
                });
            }
        }

        // Determine overall level from 20-min power
        let overall_level = duration_comparisons
            .iter()
            .find(|c| c.duration_secs == 1200)
            .map(|c| c.level)
            .unwrap_or(ReferenceLevel::Untrained);

        // Determine level at each energy system
        let system_levels = self.calculate_system_levels(&duration_comparisons);

        ProfileComparison {
            duration_comparisons,
            overall_level,
            system_levels,
        }
    }

    /// Find the reference level for a given W/kg at a duration.
    fn find_level_at(&self, duration_secs: u32, wpk: f64) -> (ReferenceLevel, f64) {
        let mut current_level = ReferenceLevel::Untrained;
        let mut progress = 0.0;

        for level in ReferenceLevel::all() {
            let reference = if self.use_female_reference {
                female_reference_wpk(*level)
            } else {
                male_reference_wpk(*level)
            };

            let level_wpk = reference.wpk_at(duration_secs);

            if wpk >= level_wpk {
                current_level = *level;

                // Calculate progress to next level
                if let Some(next) = level.next_level() {
                    let next_ref = if self.use_female_reference {
                        female_reference_wpk(next)
                    } else {
                        male_reference_wpk(next)
                    };
                    let next_wpk = next_ref.wpk_at(duration_secs);
                    let range = next_wpk - level_wpk;
                    if range > 0.0 {
                        progress = ((wpk - level_wpk) / range * 100.0).min(100.0);
                    }
                }
            } else {
                break;
            }
        }

        (current_level, progress)
    }

    /// Calculate level at each energy system.
    fn calculate_system_levels(
        &self,
        comparisons: &[DurationComparison],
    ) -> Vec<(EnergySystem, ReferenceLevel)> {
        let mut system_levels = Vec::new();

        for system in EnergySystem::all() {
            let system_comparisons: Vec<_> = comparisons
                .iter()
                .filter(|c| c.energy_system() == *system)
                .collect();

            if !system_comparisons.is_empty() {
                // Use the average level (take the lowest to be conservative)
                let level = system_comparisons
                    .iter()
                    .map(|c| c.level)
                    .min_by_key(|l| *l as u8)
                    .unwrap_or(ReferenceLevel::Untrained);

                system_levels.push((*system, level));
            }
        }

        system_levels
    }

    /// Get reference curve for a specific level.
    pub fn reference_curve(&self, level: ReferenceLevel) -> ReferenceCurve {
        if self.use_female_reference {
            female_reference_wpk(level)
        } else {
            male_reference_wpk(level)
        }
    }

    /// Get all reference curves for comparison chart.
    pub fn all_reference_curves(&self) -> Vec<(ReferenceLevel, Vec<(u32, u16)>)> {
        ReferenceLevel::all()
            .iter()
            .map(|&level| {
                let curve = self.reference_curve(level);
                (level, curve.full_curve(self.weight_kg))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::profile::PowerProfilePoint;
    use super::super::types::ProfileType;
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_reference_level_ordering() {
        assert_eq!(
            ReferenceLevel::Untrained.next_level(),
            Some(ReferenceLevel::Recreational)
        );
        assert_eq!(ReferenceLevel::WorldClass.next_level(), None);
    }

    #[test]
    fn test_male_reference_values() {
        let elite = male_reference_wpk(ReferenceLevel::Elite);
        let trained = male_reference_wpk(ReferenceLevel::Trained);

        // Elite should be higher than trained at all durations
        assert!(elite.wpk_5s > trained.wpk_5s);
        assert!(elite.wpk_60s > trained.wpk_60s);
        assert!(elite.wpk_300s > trained.wpk_300s);
        assert!(elite.wpk_1200s > trained.wpk_1200s);
    }

    #[test]
    fn test_female_reference_values() {
        let female = female_reference_wpk(ReferenceLevel::Trained);
        let male = male_reference_wpk(ReferenceLevel::Trained);

        // Female values should be lower than male
        assert!(female.wpk_5s < male.wpk_5s);
        assert!(female.wpk_1200s < male.wpk_1200s);
    }

    #[test]
    fn test_reference_curve_interpolation() {
        let curve = male_reference_wpk(ReferenceLevel::Trained);

        // Value at 30s should be between 5s and 60s
        let wpk_30 = curve.wpk_at(30);
        assert!(wpk_30 < curve.wpk_5s);
        assert!(wpk_30 > curve.wpk_60s);

        // Value at 180s should be between 60s and 300s
        let wpk_180 = curve.wpk_at(180);
        assert!(wpk_180 < curve.wpk_60s);
        assert!(wpk_180 > curve.wpk_300s);
    }

    #[test]
    fn test_profile_comparison() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        // Add data for a "trained" level rider at 70kg
        // Trained male: 5s=17W/kg, 60s=9W/kg, 300s=5W/kg, 1200s=4W/kg
        profile.update_point(PowerProfilePoint::new(5, 1200)); // ~17 W/kg
        profile.update_point(PowerProfilePoint::new(60, 630)); // ~9 W/kg
        profile.update_point(PowerProfilePoint::new(300, 350)); // ~5 W/kg
        profile.update_point(PowerProfilePoint::new(1200, 280)); // ~4 W/kg

        let comparer = ProfileComparer::new(70.0, false);
        let comparison = comparer.compare(&profile);

        // Should be around "Trained" level
        assert!(matches!(
            comparison.overall_level,
            ReferenceLevel::Trained | ReferenceLevel::Competitive
        ));
    }

    #[test]
    fn test_find_level() {
        let comparer = ProfileComparer::new(70.0, false);

        // 4 W/kg at 20-min is Trained level for men
        let (level, _) = comparer.find_level_at(1200, 4.0);
        assert_eq!(level, ReferenceLevel::Trained);

        // 5 W/kg at 20-min is Elite level
        let (level, _) = comparer.find_level_at(1200, 5.0);
        assert_eq!(level, ReferenceLevel::Elite);

        // 2 W/kg is Untrained
        let (level, _) = comparer.find_level_at(1200, 2.0);
        assert_eq!(level, ReferenceLevel::Untrained);
    }

    #[test]
    fn test_progress_to_next_level() {
        let comparer = ProfileComparer::new(70.0, false);

        // Trained is 4.0, Competitive is 4.5 W/kg at 1200s
        // 4.25 should be ~50% progress to Competitive
        let (level, progress) = comparer.find_level_at(1200, 4.25);
        assert_eq!(level, ReferenceLevel::Trained);
        assert!((progress - 50.0).abs() < 5.0);
    }

    #[test]
    fn test_all_reference_curves() {
        let comparer = ProfileComparer::new(70.0, false);
        let curves = comparer.all_reference_curves();

        assert_eq!(curves.len(), 6); // All 6 levels

        // Check each curve has standard durations
        for (_, curve_points) in &curves {
            assert_eq!(curve_points.len(), PROFILE_DURATIONS.len());
        }

        // World class should have highest values
        let world_class = &curves
            .iter()
            .find(|(l, _)| *l == ReferenceLevel::WorldClass)
            .unwrap()
            .1;
        let untrained = &curves
            .iter()
            .find(|(l, _)| *l == ReferenceLevel::Untrained)
            .unwrap()
            .1;

        assert!(world_class[0].1 > untrained[0].1);
    }
}
