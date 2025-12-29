//! Training plan achievement checks.
//!
//! T070: Add plan-related achievements (plan completion, streak).
//!
//! Checks achievements related to training plan completion and consistency.

use crate::achievements::achievement::Achievement;
use crate::achievements::types::{AchievementCategory, AchievementTier};

/// Summary of training plan progress for achievement checking.
#[derive(Debug, Clone, Default)]
pub struct TrainingPlanSummary {
    /// Total plans started.
    pub plans_started: u32,
    /// Total plans completed.
    pub plans_completed: u32,
    /// Total workouts completed across all plans.
    pub workouts_completed: u32,
    /// Current workout streak.
    pub current_streak: u32,
    /// Longest workout streak ever.
    pub longest_streak: u32,
    /// Current week in active plan.
    pub current_week: u8,
    /// Weeks completed in current plan.
    pub weeks_completed: u8,
    /// Current plan compliance percentage.
    pub compliance_percent: f32,
    /// Plans completed with 90%+ compliance.
    pub perfect_plans: u32,
}

/// Checker for training plan achievements.
#[derive(Debug, Default)]
pub struct TrainingChecker;

impl TrainingChecker {
    /// Create a new training checker.
    pub fn new() -> Self {
        Self
    }

    /// Check achievements based on training plan progress.
    pub fn check(&self, summary: &TrainingPlanSummary) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        // Plan start achievements
        if summary.plans_started >= 1 {
            achievements.push(first_plan());
        }

        // Plan completion achievements
        if summary.plans_completed >= 1 {
            achievements.push(plan_finisher());
        }
        if summary.plans_completed >= 3 {
            achievements.push(plan_veteran());
        }
        if summary.plans_completed >= 10 {
            achievements.push(plan_master());
        }

        // Perfect plan achievements
        if summary.perfect_plans >= 1 {
            achievements.push(perfect_execution());
        }
        if summary.perfect_plans >= 3 {
            achievements.push(consistency_king());
        }

        // Workout completion milestones
        if summary.workouts_completed >= 10 {
            achievements.push(workout_warrior_10());
        }
        if summary.workouts_completed >= 50 {
            achievements.push(workout_warrior_50());
        }
        if summary.workouts_completed >= 100 {
            achievements.push(century_workouts());
        }
        if summary.workouts_completed >= 500 {
            achievements.push(workout_legend());
        }

        // Streak achievements
        if summary.current_streak >= 3 {
            achievements.push(three_day_streak());
        }
        if summary.current_streak >= 7 || summary.longest_streak >= 7 {
            achievements.push(week_streak());
        }
        if summary.current_streak >= 14 || summary.longest_streak >= 14 {
            achievements.push(two_week_streak());
        }
        if summary.current_streak >= 30 || summary.longest_streak >= 30 {
            achievements.push(month_streak());
        }

        // Week milestones in current plan
        if summary.weeks_completed >= 4 {
            achievements.push(month_of_training());
        }
        if summary.weeks_completed >= 8 {
            achievements.push(two_months_committed());
        }
        if summary.weeks_completed >= 12 {
            achievements.push(quarter_year_dedication());
        }

        // Compliance achievements
        if summary.compliance_percent >= 90.0 && summary.workouts_completed >= 5 {
            achievements.push(high_compliance());
        }

        achievements
    }

    /// Check if a plan was just completed.
    pub fn check_plan_completion(
        &self,
        was_completed: bool,
        compliance_percent: f32,
    ) -> Vec<Achievement> {
        let mut achievements = Vec::new();

        if was_completed {
            achievements.push(plan_finisher());

            if compliance_percent >= 100.0 {
                achievements.push(perfect_100());
            } else if compliance_percent >= 90.0 {
                achievements.push(perfect_execution());
            }
        }

        achievements
    }
}

//
// Training Plan Achievement Definitions
//

/// Started first training plan.
pub fn first_plan() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000001").unwrap(),
        name: "training_first_plan".to_string(),
        title: "Plan Starter".to_string(),
        description: "Start your first training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: None,
        is_secret: false,
        icon: Some("plan_start".to_string()),
        repeatable: false,
    }
}

/// Completed a training plan.
pub fn plan_finisher() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000002").unwrap(),
        name: "training_plan_finisher".to_string(),
        title: "Plan Finisher".to_string(),
        description: "Complete a training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Silver,
        xp_value: 150,
        threshold: None,
        is_secret: false,
        icon: Some("plan_complete".to_string()),
        repeatable: false,
    }
}

/// Completed 3 training plans.
pub fn plan_veteran() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000003").unwrap(),
        name: "training_plan_veteran".to_string(),
        title: "Plan Veteran".to_string(),
        description: "Complete 3 training plans".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Gold,
        xp_value: 300,
        threshold: Some(3.0),
        is_secret: false,
        icon: Some("plan_veteran".to_string()),
        repeatable: false,
    }
}

/// Completed 10 training plans.
pub fn plan_master() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000004").unwrap(),
        name: "training_plan_master".to_string(),
        title: "Plan Master".to_string(),
        description: "Complete 10 training plans".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Diamond,
        xp_value: 500,
        threshold: Some(10.0),
        is_secret: false,
        icon: Some("plan_master".to_string()),
        repeatable: false,
    }
}

/// Completed a plan with 90%+ compliance.
pub fn perfect_execution() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000005").unwrap(),
        name: "training_perfect_execution".to_string(),
        title: "Perfect Execution".to_string(),
        description: "Complete a plan with 90%+ compliance".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(90.0),
        is_secret: false,
        icon: Some("perfect".to_string()),
        repeatable: false,
    }
}

/// Completed a plan with 100% compliance.
pub fn perfect_100() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000006").unwrap(),
        name: "training_perfect_100".to_string(),
        title: "Flawless".to_string(),
        description: "Complete a plan with 100% compliance".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Diamond,
        xp_value: 400,
        threshold: Some(100.0),
        is_secret: false,
        icon: Some("flawless".to_string()),
        repeatable: false,
    }
}

/// Completed 3 plans with 90%+ compliance.
pub fn consistency_king() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000007").unwrap(),
        name: "training_consistency_king".to_string(),
        title: "Consistency King".to_string(),
        description: "Complete 3 plans with 90%+ compliance".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Diamond,
        xp_value: 500,
        threshold: Some(3.0),
        is_secret: false,
        icon: Some("consistency".to_string()),
        repeatable: false,
    }
}

/// Completed 10 plan workouts.
pub fn workout_warrior_10() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000008").unwrap(),
        name: "training_warrior_10".to_string(),
        title: "Workout Warrior".to_string(),
        description: "Complete 10 plan workouts".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Bronze,
        xp_value: 50,
        threshold: Some(10.0),
        is_secret: false,
        icon: Some("warrior".to_string()),
        repeatable: false,
    }
}

/// Completed 50 plan workouts.
pub fn workout_warrior_50() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000009").unwrap(),
        name: "training_warrior_50".to_string(),
        title: "Dedicated Trainer".to_string(),
        description: "Complete 50 plan workouts".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Silver,
        xp_value: 150,
        threshold: Some(50.0),
        is_secret: false,
        icon: Some("dedicated".to_string()),
        repeatable: false,
    }
}

/// Completed 100 plan workouts.
pub fn century_workouts() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000010").unwrap(),
        name: "training_century_workouts".to_string(),
        title: "Century of Workouts".to_string(),
        description: "Complete 100 plan workouts".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Gold,
        xp_value: 300,
        threshold: Some(100.0),
        is_secret: false,
        icon: Some("century".to_string()),
        repeatable: false,
    }
}

/// Completed 500 plan workouts.
pub fn workout_legend() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000011").unwrap(),
        name: "training_legend".to_string(),
        title: "Training Legend".to_string(),
        description: "Complete 500 plan workouts".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Diamond,
        xp_value: 750,
        threshold: Some(500.0),
        is_secret: false,
        icon: Some("legend".to_string()),
        repeatable: false,
    }
}

/// 3-day workout streak.
pub fn three_day_streak() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000012").unwrap(),
        name: "training_streak_3".to_string(),
        title: "Three Day Streak".to_string(),
        description: "Complete plan workouts 3 days in a row".to_string(),
        category: AchievementCategory::Consistency,
        tier: AchievementTier::Bronze,
        xp_value: 30,
        threshold: Some(3.0),
        is_secret: false,
        icon: Some("streak".to_string()),
        repeatable: false,
    }
}

/// 7-day workout streak.
pub fn week_streak() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000013").unwrap(),
        name: "training_streak_7".to_string(),
        title: "Week of Dedication".to_string(),
        description: "Complete plan workouts 7 days in a row".to_string(),
        category: AchievementCategory::Consistency,
        tier: AchievementTier::Silver,
        xp_value: 75,
        threshold: Some(7.0),
        is_secret: false,
        icon: Some("week_streak".to_string()),
        repeatable: false,
    }
}

/// 14-day workout streak.
pub fn two_week_streak() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000014").unwrap(),
        name: "training_streak_14".to_string(),
        title: "Two Week Warrior".to_string(),
        description: "Complete plan workouts 14 days in a row".to_string(),
        category: AchievementCategory::Consistency,
        tier: AchievementTier::Gold,
        xp_value: 150,
        threshold: Some(14.0),
        is_secret: false,
        icon: Some("two_week_streak".to_string()),
        repeatable: false,
    }
}

/// 30-day workout streak.
pub fn month_streak() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000015").unwrap(),
        name: "training_streak_30".to_string(),
        title: "Month of Commitment".to_string(),
        description: "Complete plan workouts 30 days in a row".to_string(),
        category: AchievementCategory::Consistency,
        tier: AchievementTier::Diamond,
        xp_value: 300,
        threshold: Some(30.0),
        is_secret: false,
        icon: Some("month_streak".to_string()),
        repeatable: false,
    }
}

/// Completed 4 weeks of training.
pub fn month_of_training() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000016").unwrap(),
        name: "training_month".to_string(),
        title: "Month of Training".to_string(),
        description: "Complete 4 weeks in a training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(4.0),
        is_secret: false,
        icon: Some("month_training".to_string()),
        repeatable: false,
    }
}

/// Completed 8 weeks of training.
pub fn two_months_committed() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000017").unwrap(),
        name: "training_two_months".to_string(),
        title: "Two Months Committed".to_string(),
        description: "Complete 8 weeks in a training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Gold,
        xp_value: 200,
        threshold: Some(8.0),
        is_secret: false,
        icon: Some("two_months".to_string()),
        repeatable: false,
    }
}

/// Completed 12 weeks of training.
pub fn quarter_year_dedication() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000018").unwrap(),
        name: "training_quarter_year".to_string(),
        title: "Quarter Year Dedication".to_string(),
        description: "Complete 12 weeks in a training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Diamond,
        xp_value: 350,
        threshold: Some(12.0),
        is_secret: false,
        icon: Some("quarter_year".to_string()),
        repeatable: false,
    }
}

/// High compliance rate.
pub fn high_compliance() -> Achievement {
    Achievement {
        id: uuid::Uuid::parse_str("00000010-0000-4000-8000-000000000019").unwrap(),
        name: "training_high_compliance".to_string(),
        title: "Following the Plan".to_string(),
        description: "Maintain 90%+ compliance in your training plan".to_string(),
        category: AchievementCategory::Training,
        tier: AchievementTier::Silver,
        xp_value: 100,
        threshold: Some(90.0),
        is_secret: false,
        icon: Some("compliance".to_string()),
        repeatable: false,
    }
}

/// Get all training achievements.
pub fn all_training_achievements() -> Vec<Achievement> {
    vec![
        first_plan(),
        plan_finisher(),
        plan_veteran(),
        plan_master(),
        perfect_execution(),
        perfect_100(),
        consistency_king(),
        workout_warrior_10(),
        workout_warrior_50(),
        century_workouts(),
        workout_legend(),
        three_day_streak(),
        week_streak(),
        two_week_streak(),
        month_streak(),
        month_of_training(),
        two_months_committed(),
        quarter_year_dedication(),
        high_compliance(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_training_achievements() {
        let achievements = all_training_achievements();
        assert_eq!(achievements.len(), 19);

        // Check all have unique IDs
        let ids: std::collections::HashSet<_> = achievements.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), achievements.len());
    }

    #[test]
    fn test_first_plan_check() {
        let checker = TrainingChecker::new();
        let summary = TrainingPlanSummary {
            plans_started: 1,
            ..Default::default()
        };

        let achievements = checker.check(&summary);
        assert!(achievements.iter().any(|a| a.name == "training_first_plan"));
    }

    #[test]
    fn test_streak_achievements() {
        let checker = TrainingChecker::new();
        let summary = TrainingPlanSummary {
            current_streak: 7,
            ..Default::default()
        };

        let achievements = checker.check(&summary);
        assert!(achievements.iter().any(|a| a.name == "training_streak_3"));
        assert!(achievements.iter().any(|a| a.name == "training_streak_7"));
    }

    #[test]
    fn test_plan_completion_check() {
        let checker = TrainingChecker::new();
        let achievements = checker.check_plan_completion(true, 95.0);

        assert!(achievements.iter().any(|a| a.name == "training_plan_finisher"));
        assert!(achievements.iter().any(|a| a.name == "training_perfect_execution"));
    }

    #[test]
    fn test_perfect_100() {
        let checker = TrainingChecker::new();
        let achievements = checker.check_plan_completion(true, 100.0);

        assert!(achievements.iter().any(|a| a.name == "training_perfect_100"));
    }
}
