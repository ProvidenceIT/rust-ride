//! Plan scheduling logic.
//!
//! T064: Implement plan assignment and scheduling logic.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::plan::{TrainingPhase, TrainingPlan};
use super::workout::ScheduledWorkout;

/// Configuration for scheduling a plan.
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    /// Start date for the plan.
    pub start_date: NaiveDate,
    /// Available training days (bitmask: Mon=1, Tue=2, Wed=4, etc.).
    pub available_days: u8,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            start_date: chrono::Utc::now().date_naive(),
            available_days: 0b1111111, // All days available
        }
    }
}

impl ScheduleConfig {
    /// Create a new schedule config.
    pub fn new(start_date: NaiveDate) -> Self {
        Self {
            start_date,
            ..Default::default()
        }
    }

    /// Set available days.
    pub fn with_available_days(mut self, days: u8) -> Self {
        self.available_days = days;
        self
    }

    /// Check if a day is available.
    pub fn is_day_available(&self, weekday: Weekday) -> bool {
        let day_num = weekday.num_days_from_monday() as u8;
        (self.available_days & (1 << day_num)) != 0
    }

    /// Get the number of available days per week.
    pub fn days_per_week(&self) -> u8 {
        self.available_days.count_ones() as u8
    }
}

/// Scheduler for training plans.
#[derive(Debug)]
pub struct PlanScheduler {
    config: ScheduleConfig,
}

impl PlanScheduler {
    /// Create a new scheduler with the given config.
    pub fn new(config: ScheduleConfig) -> Self {
        Self { config }
    }

    /// Schedule all workouts in a plan.
    pub fn schedule_plan(&self, plan: &TrainingPlan) -> Vec<ScheduledWorkout> {
        let mut scheduled = Vec::new();

        for week in &plan.weeks {
            let week_start = self.week_start_date(week.week_number);

            for workout in &week.workouts {
                // Calculate the actual date for this workout
                let workout_date = self.date_for_day(week_start, workout.day_of_week);

                // Check if this day is available for training
                let actual_date = if self.is_day_available(workout_date) {
                    workout_date
                } else {
                    // Find the nearest available day
                    self.find_nearest_available_day(workout_date)
                        .unwrap_or(workout_date)
                };

                let scheduled_workout = ScheduledWorkout::new(
                    plan.id,
                    week.week_number,
                    workout.day_of_week,
                    actual_date,
                    &workout.workout_name,
                    workout.duration_minutes,
                )
                .with_tss(workout.estimated_tss)
                .with_type(workout.workout_type)
                .with_description(&workout.description);

                let scheduled_workout = if workout.is_optional {
                    scheduled_workout.optional()
                } else {
                    scheduled_workout
                };

                let scheduled_workout = if let Some(id) = workout.workout_id {
                    scheduled_workout.with_workout_id(id)
                } else {
                    scheduled_workout
                };

                scheduled.push(scheduled_workout);
            }
        }

        // Sort by date
        scheduled.sort_by(|a, b| a.scheduled_date.cmp(&b.scheduled_date));

        scheduled
    }

    /// Get the start date for a specific week.
    fn week_start_date(&self, week_number: u8) -> NaiveDate {
        // Week 1 starts on the plan start date (adjusted to Monday)
        let start = self.config.start_date;

        // Find the Monday of the start week
        let days_since_monday = start.weekday().num_days_from_monday();
        let monday = start - Duration::days(days_since_monday as i64);

        // Add weeks
        monday + Duration::weeks((week_number as i64) - 1)
    }

    /// Get the date for a specific day of the week.
    fn date_for_day(&self, week_start: NaiveDate, day_of_week: u8) -> NaiveDate {
        // day_of_week is 1-7 (Mon-Sun)
        // week_start is a Monday
        let days_to_add = (day_of_week.saturating_sub(1)) as i64;
        week_start + Duration::days(days_to_add)
    }

    /// Check if a date falls on an available training day.
    fn is_day_available(&self, date: NaiveDate) -> bool {
        self.config.is_day_available(date.weekday())
    }

    /// Find the nearest available training day.
    fn find_nearest_available_day(&self, date: NaiveDate) -> Option<NaiveDate> {
        if self.config.available_days == 0 {
            return None;
        }

        // Search up to 3 days forward and backward
        for offset in 1..=3 {
            // Try forward
            let forward = date + Duration::days(offset);
            if self.is_day_available(forward) {
                return Some(forward);
            }

            // Try backward
            let backward = date - Duration::days(offset);
            if self.is_day_available(backward) {
                return Some(backward);
            }
        }

        // If not found within 3 days, just return the next available day
        for offset in 1..=7 {
            let candidate = date + Duration::days(offset);
            if self.is_day_available(candidate) {
                return Some(candidate);
            }
        }

        None
    }

    /// Get the week number for a given date.
    pub fn week_for_date(&self, date: NaiveDate) -> u8 {
        let start = self.config.start_date;
        let days_since_monday = start.weekday().num_days_from_monday();
        let plan_start_monday = start - Duration::days(days_since_monday as i64);

        let days_diff = (date - plan_start_monday).num_days();
        if days_diff < 0 {
            0
        } else {
            ((days_diff / 7) + 1) as u8
        }
    }
}

/// Reschedule workouts for a single week.
#[derive(Debug)]
#[allow(dead_code)]
pub struct WeekRescheduler {
    week_number: u8,
    week_start: NaiveDate,
    available_days: u8,
}

#[allow(dead_code)]
impl WeekRescheduler {
    /// Create a new week rescheduler.
    pub fn new(week_number: u8, week_start: NaiveDate, available_days: u8) -> Self {
        Self {
            week_number,
            week_start,
            available_days,
        }
    }

    /// Redistribute workouts across available days.
    pub fn redistribute(&self, mut workouts: Vec<ScheduledWorkout>) -> Vec<ScheduledWorkout> {
        let available_days = self.get_available_days();
        if available_days.is_empty() || workouts.is_empty() {
            return workouts;
        }

        // Sort workouts by their original day
        workouts.sort_by(|a, b| a.day_of_week.cmp(&b.day_of_week));

        // Distribute evenly across available days
        let workouts_per_day = workouts.len() / available_days.len();
        let extra_workouts = workouts.len() % available_days.len();

        let mut result = Vec::new();
        let mut workout_idx = 0;

        for (day_idx, &(day_of_week, date)) in available_days.iter().enumerate() {
            let count = workouts_per_day + if day_idx < extra_workouts { 1 } else { 0 };

            for _ in 0..count {
                if workout_idx < workouts.len() {
                    let mut workout = workouts[workout_idx].clone();
                    workout.day_of_week = day_of_week;
                    workout.scheduled_date = date;
                    result.push(workout);
                    workout_idx += 1;
                }
            }
        }

        result
    }

    /// Get available days as (day_of_week, date) pairs.
    fn get_available_days(&self) -> Vec<(u8, NaiveDate)> {
        let mut days = Vec::new();
        for day in 1..=7u8 {
            let mask = 1 << (day - 1);
            if (self.available_days & mask) != 0 {
                let date = self.week_start + Duration::days((day as i64) - 1);
                days.push((day, date));
            }
        }
        days
    }
}

/// Schedule analysis and suggestions.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ScheduleAnalysis {
    /// Total scheduled hours.
    pub total_hours: f32,
    /// Workouts per week.
    pub workouts_per_week: f32,
    /// Most common workout types.
    pub workout_type_distribution: Vec<(String, usize)>,
    /// Weeks with highest load.
    pub peak_weeks: Vec<u8>,
    /// Suggested rest days.
    pub suggested_rest_days: Vec<NaiveDate>,
}

#[allow(dead_code)]
impl ScheduleAnalysis {
    /// Create an analysis from scheduled workouts.
    pub fn from_schedule(workouts: &[ScheduledWorkout], plan: &TrainingPlan) -> Self {
        let total_hours: f32 = workouts
            .iter()
            .map(|w| w.duration_minutes as f32 / 60.0)
            .sum();

        let workouts_per_week = if plan.duration_weeks > 0 {
            workouts.len() as f32 / plan.duration_weeks as f32
        } else {
            0.0
        };

        // Count workout types
        let mut type_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for workout in workouts {
            let type_name = workout.workout_type.display_name().to_string();
            *type_counts.entry(type_name).or_default() += 1;
        }
        let mut workout_type_distribution: Vec<_> = type_counts.into_iter().collect();
        workout_type_distribution.sort_by(|a, b| b.1.cmp(&a.1));

        // Find peak weeks (by TSS)
        let mut week_tss: std::collections::HashMap<u8, f32> = std::collections::HashMap::new();
        for workout in workouts {
            *week_tss.entry(workout.week_number).or_default() += workout.estimated_tss;
        }
        let avg_tss: f32 = if !week_tss.is_empty() {
            week_tss.values().sum::<f32>() / week_tss.len() as f32
        } else {
            0.0
        };
        let peak_weeks: Vec<_> = week_tss
            .into_iter()
            .filter(|(_, tss)| *tss > avg_tss * 1.1)
            .map(|(week, _)| week)
            .collect();

        // Suggest rest days based on recovery weeks
        let suggested_rest_days = plan
            .weeks
            .iter()
            .filter(|w| matches!(w.phase, TrainingPhase::Recovery))
            .flat_map(|_w| Vec::<NaiveDate>::new()) // Simplified - would need actual dates
            .collect();

        Self {
            total_hours,
            workouts_per_week,
            workout_type_distribution,
            peak_weeks,
            suggested_rest_days,
        }
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "{:.1} hours total, ~{:.1} workouts/week",
            self.total_hours, self.workouts_per_week
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training_plans::plan::{PlanWeek, PlanWorkout, WorkoutType};
    use crate::training_plans::{DifficultyLevel, Discipline};

    fn create_test_plan() -> TrainingPlan {
        let mut plan = TrainingPlan::new(
            uuid::Uuid::new_v4(),
            "Test Plan",
            Discipline::Road,
            DifficultyLevel::Beginner,
            "Test",
        );

        let week1 = PlanWeek::new(1, "Week 1", TrainingPhase::Base).with_workouts(vec![
            PlanWorkout::new(1, "Monday Workout", 60),    // Monday
            PlanWorkout::new(3, "Wednesday Workout", 60), // Wednesday
            PlanWorkout::new(6, "Saturday Workout", 90),  // Saturday
        ]);

        let week2 = PlanWeek::new(2, "Week 2", TrainingPhase::Build).with_workouts(vec![
            PlanWorkout::new(2, "Tuesday Workout", 60),  // Tuesday
            PlanWorkout::new(4, "Thursday Workout", 60), // Thursday
        ]);

        plan.add_week(week1);
        plan.add_week(week2);
        plan
    }

    #[test]
    fn test_schedule_config() {
        let config = ScheduleConfig::new(NaiveDate::from_ymd_opt(2025, 1, 6).unwrap())
            .with_available_days(0b0111110); // Tue-Sat

        assert!(!config.is_day_available(Weekday::Mon));
        assert!(config.is_day_available(Weekday::Tue));
        assert!(config.is_day_available(Weekday::Sat));
        assert!(!config.is_day_available(Weekday::Sun));
        assert_eq!(config.days_per_week(), 5);
    }

    #[test]
    fn test_schedule_plan() {
        let plan = create_test_plan();
        let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(); // A Monday

        let config = ScheduleConfig::new(start);
        let scheduler = PlanScheduler::new(config);
        let scheduled = scheduler.schedule_plan(&plan);

        assert_eq!(scheduled.len(), 5);

        // First workout should be on Monday, Jan 6
        assert_eq!(scheduled[0].scheduled_date, start);
        assert_eq!(scheduled[0].workout_name, "Monday Workout");
    }

    #[test]
    fn test_week_start_date() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(); // Monday
        let config = ScheduleConfig::new(start);
        let scheduler = PlanScheduler::new(config);

        assert_eq!(scheduler.week_start_date(1), start);
        assert_eq!(
            scheduler.week_start_date(2),
            NaiveDate::from_ymd_opt(2025, 1, 13).unwrap()
        );
    }

    #[test]
    fn test_week_for_date() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        let config = ScheduleConfig::new(start);
        let scheduler = PlanScheduler::new(config);

        assert_eq!(scheduler.week_for_date(start), 1);
        assert_eq!(
            scheduler.week_for_date(NaiveDate::from_ymd_opt(2025, 1, 13).unwrap()),
            2
        );
        assert_eq!(
            scheduler.week_for_date(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
            3
        );
    }

    #[test]
    fn test_find_nearest_available_day() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        let config = ScheduleConfig::new(start).with_available_days(0b0100010); // Tue and Sat only

        let scheduler = PlanScheduler::new(config);

        // Monday should find Tuesday
        let monday = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        let nearest = scheduler.find_nearest_available_day(monday);
        assert_eq!(nearest, Some(NaiveDate::from_ymd_opt(2025, 1, 7).unwrap()));
    }

    #[test]
    fn test_schedule_analysis() {
        let plan = create_test_plan();
        let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        let config = ScheduleConfig::new(start);
        let scheduler = PlanScheduler::new(config);
        let scheduled = scheduler.schedule_plan(&plan);

        let analysis = ScheduleAnalysis::from_schedule(&scheduled, &plan);

        assert!(analysis.total_hours > 0.0);
        assert!(analysis.workouts_per_week > 0.0);
    }
}
