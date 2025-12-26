//! Training goal type definitions.
//!
//! T016: Create goal types for training objectives

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A training goal set by the rider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingGoal {
    /// Unique identifier
    pub id: Uuid,
    /// User who owns this goal
    pub user_id: Uuid,
    /// Type of goal
    pub goal_type: GoalType,
    /// Display title
    pub title: String,
    /// Optional detailed description
    pub description: Option<String>,
    /// Target date for event goals
    pub target_date: Option<NaiveDate>,
    /// Target metric to achieve
    pub target_metric: Option<TargetMetric>,
    /// Priority (1 = highest, unique per user)
    pub priority: u8,
    /// Current status
    pub status: GoalStatus,
    /// When the goal was created
    pub created_at: DateTime<Utc>,
    /// When the goal was last updated
    pub updated_at: DateTime<Utc>,
}

impl TrainingGoal {
    /// Create a new training goal.
    pub fn new(user_id: Uuid, goal_type: GoalType, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            goal_type,
            title,
            description: None,
            target_date: None,
            target_metric: None,
            priority: 1,
            status: GoalStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this is an event-focused goal.
    pub fn is_event_goal(&self) -> bool {
        matches!(
            self.goal_type,
            GoalType::Race { .. }
                | GoalType::CenturyRide
                | GoalType::GranFondo
                | GoalType::TimeTrial
        )
    }

    /// Check if this goal has a target date.
    pub fn has_target_date(&self) -> bool {
        self.target_date.is_some()
    }

    /// Get days until target date (None if no target or in past).
    pub fn days_until_target(&self) -> Option<i64> {
        self.target_date.map(|date| {
            let today = Utc::now().date_naive();
            (date - today).num_days()
        })
    }

    /// Check if target date has passed.
    pub fn is_past_target(&self) -> bool {
        self.days_until_target().map(|d| d < 0).unwrap_or(false)
    }

    /// Get the primary energy system this goal targets.
    pub fn primary_energy_system(&self) -> Option<&'static str> {
        match &self.goal_type {
            GoalType::ImproveEndurance => Some("endurance"),
            GoalType::ImproveVo2max => Some("vo2max"),
            GoalType::BuildThreshold => Some("threshold"),
            GoalType::DevelopSprint => Some("sprint"),
            GoalType::Race { event_type } => match event_type {
                EventType::Criterium => Some("anaerobic"),
                EventType::TimeTrial => Some("threshold"),
                EventType::RoadRace | EventType::GranFondo => Some("endurance"),
                _ => None,
            },
            GoalType::CenturyRide | GoalType::GranFondo => Some("endurance"),
            GoalType::TimeTrial => Some("threshold"),
            _ => None,
        }
    }
}

/// Type of training goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalType {
    // General fitness goals
    /// Improve overall endurance
    ImproveEndurance,
    /// Lose weight while maintaining fitness
    LoseWeight,
    /// General performance improvement
    GetFaster,

    // Event-focused goals
    /// Prepare for a race
    Race { event_type: EventType },
    /// Complete a century ride (100 miles)
    CenturyRide,
    /// Complete a gran fondo event
    GranFondo,
    /// Compete in a time trial
    TimeTrial,

    // Energy system goals
    /// Improve VO2max capacity
    ImproveVo2max,
    /// Build threshold power
    BuildThreshold,
    /// Develop sprint power
    DevelopSprint,
}

impl GoalType {
    /// Get display name for the goal type.
    pub fn display_name(&self) -> &str {
        match self {
            GoalType::ImproveEndurance => "Improve Endurance",
            GoalType::LoseWeight => "Lose Weight",
            GoalType::GetFaster => "Get Faster",
            GoalType::Race { .. } => "Race Preparation",
            GoalType::CenturyRide => "Century Ride",
            GoalType::GranFondo => "Gran Fondo",
            GoalType::TimeTrial => "Time Trial",
            GoalType::ImproveVo2max => "Improve VO2max",
            GoalType::BuildThreshold => "Build Threshold",
            GoalType::DevelopSprint => "Develop Sprint",
        }
    }

    /// Get description of what this goal involves.
    pub fn description(&self) -> &str {
        match self {
            GoalType::ImproveEndurance => "Build aerobic base and sustain longer efforts",
            GoalType::LoseWeight => "Optimize training for fat burning while maintaining power",
            GoalType::GetFaster => "General performance improvement across all areas",
            GoalType::Race { .. } => "Peak fitness for race day",
            GoalType::CenturyRide => "Build endurance for 100+ mile rides",
            GoalType::GranFondo => "Prepare for a challenging mass participation event",
            GoalType::TimeTrial => "Maximize sustained power output",
            GoalType::ImproveVo2max => "Increase maximum oxygen uptake capacity",
            GoalType::BuildThreshold => "Raise functional threshold power",
            GoalType::DevelopSprint => "Build peak short-duration power",
        }
    }

    /// Whether this goal type requires a target date.
    pub fn requires_target_date(&self) -> bool {
        matches!(
            self,
            GoalType::Race { .. }
                | GoalType::CenturyRide
                | GoalType::GranFondo
                | GoalType::TimeTrial
        )
    }
}

impl std::fmt::Display for GoalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Type of racing event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    /// Road race
    RoadRace,
    /// Criterium (short circuit race)
    Criterium,
    /// Mass participation event
    GranFondo,
    /// Time trial
    TimeTrial,
    /// Triathlon cycling leg
    Triathlon,
    /// Custom event type
    Other(String),
}

impl EventType {
    /// Get display name.
    pub fn display_name(&self) -> &str {
        match self {
            EventType::RoadRace => "Road Race",
            EventType::Criterium => "Criterium",
            EventType::GranFondo => "Gran Fondo",
            EventType::TimeTrial => "Time Trial",
            EventType::Triathlon => "Triathlon",
            EventType::Other(name) => name,
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Target metric for a goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetric {
    /// Type of metric being targeted
    pub metric_type: MetricType,
    /// Target value to achieve
    pub target_value: f32,
    /// Current value (if known)
    pub current_value: Option<f32>,
}

impl TargetMetric {
    /// Create a new target metric.
    pub fn new(metric_type: MetricType, target_value: f32) -> Self {
        Self {
            metric_type,
            target_value,
            current_value: None,
        }
    }

    /// Update current value.
    pub fn update_current(&mut self, value: f32) {
        self.current_value = Some(value);
    }

    /// Get progress percentage (0-100).
    pub fn progress_percent(&self) -> Option<f32> {
        self.current_value
            .map(|current| (current / self.target_value * 100.0).min(100.0))
    }

    /// Get gap to target (positive = still need to improve).
    pub fn gap(&self) -> Option<f32> {
        self.current_value
            .map(|current| self.target_value - current)
    }
}

/// Type of measurable metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Chronic Training Load
    Ctl,
    /// Functional Threshold Power
    Ftp,
    /// VO2max estimate
    Vo2max,
    /// Body weight
    Weight,
}

impl MetricType {
    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            MetricType::Ctl => "CTL (Fitness)",
            MetricType::Ftp => "FTP",
            MetricType::Vo2max => "VO2max",
            MetricType::Weight => "Weight",
        }
    }

    /// Get unit of measurement.
    pub fn unit(&self) -> &'static str {
        match self {
            MetricType::Ctl => "TSS/day",
            MetricType::Ftp => "W",
            MetricType::Vo2max => "ml/kg/min",
            MetricType::Weight => "kg",
        }
    }
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status of a training goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    /// Goal is active and being tracked
    Active,
    /// Goal has been achieved
    Completed,
    /// Goal was abandoned
    Abandoned,
    /// Goal is temporarily on hold
    OnHold,
}

impl GoalStatus {
    /// Whether the goal is still being actively tracked.
    pub fn is_active(&self) -> bool {
        matches!(self, GoalStatus::Active)
    }

    /// Get display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            GoalStatus::Active => "Active",
            GoalStatus::Completed => "Completed",
            GoalStatus::Abandoned => "Abandoned",
            GoalStatus::OnHold => "On Hold",
        }
    }
}

impl std::fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_goal_creation() {
        let goal = TrainingGoal::new(
            Uuid::new_v4(),
            GoalType::ImproveVo2max,
            "Boost VO2max for summer races".to_string(),
        );

        assert!(goal.status.is_active());
        assert!(!goal.is_event_goal());
        assert_eq!(goal.primary_energy_system(), Some("vo2max"));
    }

    #[test]
    fn test_event_goal_detection() {
        let event_goal = TrainingGoal::new(
            Uuid::new_v4(),
            GoalType::Race {
                event_type: EventType::RoadRace,
            },
            "Local road race".to_string(),
        );

        assert!(event_goal.is_event_goal());
        assert!(GoalType::Race {
            event_type: EventType::RoadRace
        }
        .requires_target_date());
    }

    #[test]
    fn test_target_metric_progress() {
        let mut metric = TargetMetric::new(MetricType::Ftp, 300.0);
        assert!(metric.progress_percent().is_none());

        metric.update_current(270.0);
        assert_eq!(metric.progress_percent(), Some(90.0));
        assert_eq!(metric.gap(), Some(30.0));
    }

    #[test]
    fn test_goal_type_energy_systems() {
        let endurance_goal = TrainingGoal::new(
            Uuid::new_v4(),
            GoalType::ImproveEndurance,
            "Endurance goal".to_string(),
        );
        assert_eq!(endurance_goal.primary_energy_system(), Some("endurance"));

        let sprint_goal = TrainingGoal::new(
            Uuid::new_v4(),
            GoalType::DevelopSprint,
            "Sprint goal".to_string(),
        );
        assert_eq!(sprint_goal.primary_energy_system(), Some("sprint"));

        let weight_goal = TrainingGoal::new(
            Uuid::new_v4(),
            GoalType::LoseWeight,
            "Weight goal".to_string(),
        );
        assert_eq!(weight_goal.primary_energy_system(), None);
    }
}
