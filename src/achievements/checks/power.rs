//! Power profile achievement checks.
//!
//! T057: Add power profile achievements (new PRs).
//!
//! Checks achievements related to power profile improvements and PRs.

use crate::achievements::achievement::Achievement;
use crate::achievements::types::{AchievementCategory, AchievementTier};
use crate::power_profile::{PowerProfileUpdateSummary, RideProcessResult, RiderType};

/// Checker for power profile achievements.
#[derive(Debug, Default)]
pub struct PowerProfileChecker;

impl PowerProfileChecker {
    /// Create a new power profile checker.
    pub fn new() -> Self {
        Self
    }

    /// Check achievements based on power profile updates.
    pub fn check(&self, result: &RideProcessResult, summary: &PowerProfileUpdateSummary) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // PR achievements
        achievements.extend(self.check_pr_achievements(result, summary));

        // Classification achievements
        if result.classification_changed {
            if let Some(ref classification) = result.classification {
                achievements.extend(self.check_classification_achievements(classification.rider_type));
            }
        }

        achievements
    }

    /// Check PR-related achievements.
    fn check_pr_achievements(&self, result: &RideProcessResult, summary: &PowerProfileUpdateSummary) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // First PR
        if summary.lifetime_pr_count > 0 {
            achievements.push(first_power_pr());
        }

        // Multiple PRs in one ride
        if summary.lifetime_pr_count >= 3 {
            achievements.push(triple_threat());
        }

        if summary.lifetime_pr_count >= 5 {
            achievements.push(pr_machine());
        }

        // Specific duration PRs
        for pr in &result.lifetime_prs {
            match pr.duration_secs {
                5 => achievements.push(sprint_king()),
                60 => achievements.push(minute_man()),
                300 => achievements.push(vo2max_master()),
                1200 => achievements.push(threshold_titan()),
                3600 => achievements.push(hour_of_power()),
                _ => {}
            }
        }

        // FTP improvement
        if summary.new_ftp.is_some() {
            achievements.push(ftp_breakthrough());
        }

        achievements
    }

    /// Check classification-related achievements.
    fn check_classification_achievements(&self, rider_type: RiderType) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // First classification
        achievements.push(classified());

        // Specific type achievements
        match rider_type {
            RiderType::Sprinter => achievements.push(sprinter_badge()),
            RiderType::Puncher => achievements.push(puncher_badge()),
            RiderType::Rouleur => achievements.push(rouleur_badge()),
            RiderType::Climber => achievements.push(climber_badge()),
            RiderType::AllRounder => achievements.push(all_rounder_badge()),
            RiderType::Unknown => {}
        }

        achievements
    }
}

//
// Power Profile Achievement Definitions
//

/// First power PR achieved.
pub fn first_power_pr() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000001").unwrap(),
        name: "power_first_pr".to_string(),
        title: "Personal Best".to_string(),
        description: "Set your first power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: None,
        is_secret: false,
        icon: Some("pr_first".to_string()),
        repeatable: false,
    }
}

/// Three or more PRs in one ride.
pub fn triple_threat() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000002").unwrap(),
        name: "power_triple_threat".to_string(),
        title: "Triple Threat".to_string(),
        description: "Set 3+ power PRs in a single ride".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(3.0),
        is_secret: false,
        icon: Some("pr_triple".to_string()),
        repeatable: false,
    }
}

/// Five or more PRs in one ride.
pub fn pr_machine() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000003").unwrap(),
        name: "power_pr_machine".to_string(),
        title: "PR Machine".to_string(),
        description: "Set 5+ power PRs in a single ride".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(5.0),
        is_secret: false,
        icon: Some("pr_machine".to_string()),
        repeatable: false,
    }
}

/// 5-second power PR.
pub fn sprint_king() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000004").unwrap(),
        name: "power_sprint_king".to_string(),
        title: "Sprint King".to_string(),
        description: "Set a new 5-second power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Bronze,
        xp_value: 75,
        threshold: None,
        is_secret: false,
        icon: Some("pr_sprint".to_string()),
        repeatable: false,
    }
}

/// 1-minute power PR.
pub fn minute_man() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000005").unwrap(),
        name: "power_minute_man".to_string(),
        title: "Minute Man".to_string(),
        description: "Set a new 1-minute power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Bronze,
        xp_value: 75,
        threshold: None,
        is_secret: false,
        icon: Some("pr_minute".to_string()),
        repeatable: false,
    }
}

/// 5-minute power PR.
pub fn vo2max_master() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000006").unwrap(),
        name: "power_vo2max_master".to_string(),
        title: "VO2max Master".to_string(),
        description: "Set a new 5-minute power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: None,
        is_secret: false,
        icon: Some("pr_vo2max".to_string()),
        repeatable: false,
    }
}

/// 20-minute power PR.
pub fn threshold_titan() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000007").unwrap(),
        name: "power_threshold_titan".to_string(),
        title: "Threshold Titan".to_string(),
        description: "Set a new 20-minute power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 150,
        threshold: None,
        is_secret: false,
        icon: Some("pr_threshold".to_string()),
        repeatable: false,
    }
}

/// 60-minute power PR.
pub fn hour_of_power() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000008").unwrap(),
        name: "power_hour_of_power".to_string(),
        title: "Hour of Power".to_string(),
        description: "Set a new 60-minute power personal record".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Diamond,
        xp_value: 300,
        threshold: None,
        is_secret: false,
        icon: Some("pr_hour".to_string()),
        repeatable: false,
    }
}

/// FTP improvement.
pub fn ftp_breakthrough() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000009").unwrap(),
        name: "power_ftp_breakthrough".to_string(),
        title: "FTP Breakthrough".to_string(),
        description: "Improve your estimated FTP".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Silver,
        xp_value: 125,
        threshold: None,
        is_secret: false,
        icon: Some("ftp_up".to_string()),
        repeatable: true,
    }
}

/// First rider classification.
pub fn classified() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000010").unwrap(),
        name: "power_classified".to_string(),
        title: "Classified".to_string(),
        description: "Achieve your first rider type classification".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: None,
        is_secret: false,
        icon: Some("classified".to_string()),
        repeatable: false,
    }
}

/// Sprinter classification.
pub fn sprinter_badge() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000011").unwrap(),
        name: "power_sprinter".to_string(),
        title: "Sprinter".to_string(),
        description: "Classified as a Sprinter - strong at short explosive efforts".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 150,
        threshold: None,
        is_secret: false,
        icon: Some("type_sprinter".to_string()),
        repeatable: false,
    }
}

/// Puncher classification.
pub fn puncher_badge() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000012").unwrap(),
        name: "power_puncher".to_string(),
        title: "Puncher".to_string(),
        description: "Classified as a Puncher - strong at mid-range VO2max efforts".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 150,
        threshold: None,
        is_secret: false,
        icon: Some("type_puncher".to_string()),
        repeatable: false,
    }
}

/// Rouleur classification.
pub fn rouleur_badge() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000013").unwrap(),
        name: "power_rouleur".to_string(),
        title: "Rouleur".to_string(),
        description: "Classified as a Rouleur - strong at sustained threshold power".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 150,
        threshold: None,
        is_secret: false,
        icon: Some("type_rouleur".to_string()),
        repeatable: false,
    }
}

/// Climber classification.
pub fn climber_badge() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000014").unwrap(),
        name: "power_climber".to_string(),
        title: "Climber".to_string(),
        description: "Classified as a Climber - excellent power-to-weight for long efforts".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Diamond,
        xp_value: 200,
        threshold: None,
        is_secret: false,
        icon: Some("type_climber".to_string()),
        repeatable: false,
    }
}

/// All-Rounder classification.
pub fn all_rounder_badge() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000001-0000-4000-8000-000000000015").unwrap(),
        name: "power_all_rounder".to_string(),
        title: "All-Rounder".to_string(),
        description: "Classified as an All-Rounder - balanced across all durations".to_string(),
        category: AchievementCategory::Power,
        tier: AchievementTier::Gold,
        xp_value: 175,
        threshold: None,
        is_secret: false,
        icon: Some("type_all_rounder".to_string()),
        repeatable: false,
    }
}

/// Get all power profile achievements.
pub fn all_power_achievements() -> Vec<Achievement> {
    vec![
        first_power_pr(),
        triple_threat(),
        pr_machine(),
        sprint_king(),
        minute_man(),
        vo2max_master(),
        threshold_titan(),
        hour_of_power(),
        ftp_breakthrough(),
        classified(),
        sprinter_badge(),
        puncher_badge(),
        rouleur_badge(),
        climber_badge(),
        all_rounder_badge(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power_profile::{PowerProfilePoint, RiderClassification};

    #[test]
    fn test_all_power_achievements() {
        let achievements = all_power_achievements();
        assert_eq!(achievements.len(), 15);

        // Check all have unique IDs
        let ids: std::collections::HashSet<_> = achievements.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), achievements.len());
    }

    #[test]
    fn test_pr_achievements() {
        let checker = PowerProfileChecker::new();

        let result = RideProcessResult {
            rolling_prs: vec![
                PowerProfilePoint::new(5, 800),
                PowerProfilePoint::new(60, 400),
                PowerProfilePoint::new(300, 320),
            ],
            lifetime_prs: vec![
                PowerProfilePoint::new(5, 800),
                PowerProfilePoint::new(60, 400),
                PowerProfilePoint::new(300, 320),
            ],
            classification_changed: false,
            classification: None,
        };

        let summary = PowerProfileUpdateSummary {
            rolling_pr_count: 3,
            lifetime_pr_count: 3,
            new_ftp: None,
            previous_ftp: None,
            classification_changed: false,
            rider_type_name: None,
            pr_durations: vec!["5 sec".to_string(), "1 min".to_string(), "5 min".to_string()],
        };

        let achievements = checker.check(&result, &summary);

        // Should have: first_power_pr, triple_threat, sprint_king, minute_man, vo2max_master
        assert!(achievements.len() >= 5);
    }

    #[test]
    fn test_classification_achievements() {
        let checker = PowerProfileChecker::new();

        let result = RideProcessResult {
            rolling_prs: Vec::new(),
            lifetime_prs: Vec::new(),
            classification_changed: true,
            classification: Some(RiderClassification {
                rider_type: RiderType::Sprinter,
                confidence: 0.8,
                secondary_type: None,
                watts_per_kg: Some(4.0),
                system_scores: Vec::new(),
            }),
        };

        let summary = PowerProfileUpdateSummary {
            rolling_pr_count: 0,
            lifetime_pr_count: 0,
            new_ftp: None,
            previous_ftp: None,
            classification_changed: true,
            rider_type_name: Some("Sprinter".to_string()),
            pr_durations: Vec::new(),
        };

        let achievements = checker.check(&result, &summary);

        // Should have: classified, sprinter_badge
        assert!(achievements.len() >= 2);
        assert!(achievements.iter().any(|a| a.name == "power_classified"));
        assert!(achievements.iter().any(|a| a.name == "power_sprinter"));
    }
}
