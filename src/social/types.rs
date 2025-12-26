//! Core types for social features.
//!
//! Defines rider profiles, badges, goals, and related enums.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Rider profile for social features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiderProfile {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_id: Option<String>,
    pub bio: Option<String>,
    pub ftp: Option<u16>,
    pub total_distance_km: f64,
    pub total_time_hours: f64,
    pub total_rides: u32,
    pub sharing_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RiderProfile {
    /// Create a new rider profile with default values.
    pub fn new(id: Uuid, display_name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            display_name,
            avatar_id: None,
            bio: None,
            ftp: None,
            total_distance_km: 0.0,
            total_time_hours: 0.0,
            total_rides: 0,
            sharing_enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Badge category for achievements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeCategory {
    /// Total distance milestones
    Distance,
    /// FTP improvement milestones
    Ftp,
    /// Streak-based achievements
    Consistency,
    /// Event or unique achievements
    Special,
}

impl BadgeCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            BadgeCategory::Distance => "distance",
            BadgeCategory::Ftp => "ftp",
            BadgeCategory::Consistency => "consistency",
            BadgeCategory::Special => "special",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "distance" => Some(BadgeCategory::Distance),
            "ftp" => Some(BadgeCategory::Ftp),
            "consistency" => Some(BadgeCategory::Consistency),
            "special" => Some(BadgeCategory::Special),
            _ => None,
        }
    }
}

/// Badge criteria type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriteriaType {
    /// Total distance in kilometers
    TotalDistanceKm,
    /// FTP increase in watts
    FtpIncrease,
    /// Consecutive days riding
    ConsecutiveDays,
    /// Total workouts completed
    WorkoutsCompleted,
}

impl CriteriaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CriteriaType::TotalDistanceKm => "total_distance_km",
            CriteriaType::FtpIncrease => "ftp_increase",
            CriteriaType::ConsecutiveDays => "consecutive_days",
            CriteriaType::WorkoutsCompleted => "workouts_completed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "total_distance_km" => Some(CriteriaType::TotalDistanceKm),
            "ftp_increase" => Some(CriteriaType::FtpIncrease),
            "consecutive_days" => Some(CriteriaType::ConsecutiveDays),
            "workouts_completed" => Some(CriteriaType::WorkoutsCompleted),
            _ => None,
        }
    }
}

/// Badge definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: BadgeCategory,
    pub criteria_type: CriteriaType,
    pub criteria_value: f64,
    /// Whether the badge has been earned.
    #[serde(default)]
    pub earned: bool,
    /// Current progress towards the badge.
    #[serde(default)]
    pub progress: f64,
    /// Target value for the badge (same as criteria_value, for display).
    #[serde(default)]
    pub target: f64,
}

/// Earned badge record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarnedBadge {
    pub badge: Badge,
    pub unlocked_at: DateTime<Utc>,
}

/// Goal type for challenges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
    /// Ride X km total
    TotalDistanceKm,
    /// Ride X hours total
    TotalTimeHours,
    /// Accumulate X TSS
    TotalTss,
    /// Complete X workouts
    WorkoutCount,
    /// Complete X workouts of specific type
    WorkoutTypeCount,
}

impl GoalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalType::TotalDistanceKm => "total_distance_km",
            GoalType::TotalTimeHours => "total_time_hours",
            GoalType::TotalTss => "total_tss",
            GoalType::WorkoutCount => "workout_count",
            GoalType::WorkoutTypeCount => "workout_type_count",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "total_distance_km" => Some(GoalType::TotalDistanceKm),
            "total_time_hours" => Some(GoalType::TotalTimeHours),
            "total_tss" => Some(GoalType::TotalTss),
            "workout_count" => Some(GoalType::WorkoutCount),
            "workout_type_count" => Some(GoalType::WorkoutTypeCount),
            _ => None,
        }
    }
}

/// Challenge definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub goal_type: GoalType,
    pub goal_value: f64,
    pub duration_days: u16,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub created_by_rider_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Challenge progress for a rider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeProgress {
    pub challenge_id: Uuid,
    pub rider_id: Uuid,
    pub current_value: f64,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
}

/// Activity summary for sharing on LAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub id: Uuid,
    pub ride_id: Option<Uuid>,
    pub rider_id: Uuid,
    pub rider_name: String,
    pub distance_km: f64,
    pub duration_minutes: u32,
    pub avg_power_watts: Option<u16>,
    pub elevation_gain_m: f64,
    pub world_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub shared: bool,
}

/// Peer rider information (received via LAN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRider {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_id: Option<String>,
    pub address: std::net::SocketAddr,
    pub last_seen: DateTime<Utc>,
}

/// Default badge definitions.
pub fn default_badges() -> Vec<Badge> {
    vec![
        Badge {
            id: "first_ride".to_string(),
            name: "First Ride".to_string(),
            description: "Complete your first ride".to_string(),
            icon: "🚴".to_string(),
            category: BadgeCategory::Special,
            criteria_type: CriteriaType::WorkoutsCompleted,
            criteria_value: 1.0,
            earned: false,
            progress: 0.0,
            target: 1.0,
        },
        Badge {
            id: "century".to_string(),
            name: "Century Rider".to_string(),
            description: "Ride 100 km total".to_string(),
            icon: "💯".to_string(),
            category: BadgeCategory::Distance,
            criteria_type: CriteriaType::TotalDistanceKm,
            criteria_value: 100.0,
            earned: false,
            progress: 0.0,
            target: 100.0,
        },
        Badge {
            id: "millennium".to_string(),
            name: "Millennium Rider".to_string(),
            description: "Ride 1000 km total".to_string(),
            icon: "🏆".to_string(),
            category: BadgeCategory::Distance,
            criteria_type: CriteriaType::TotalDistanceKm,
            criteria_value: 1000.0,
            earned: false,
            progress: 0.0,
            target: 1000.0,
        },
        Badge {
            id: "week_streak".to_string(),
            name: "Week Warrior".to_string(),
            description: "Ride 7 consecutive days".to_string(),
            icon: "📅".to_string(),
            category: BadgeCategory::Consistency,
            criteria_type: CriteriaType::ConsecutiveDays,
            criteria_value: 7.0,
            earned: false,
            progress: 0.0,
            target: 7.0,
        },
        Badge {
            id: "month_streak".to_string(),
            name: "Month Master".to_string(),
            description: "Ride 30 consecutive days".to_string(),
            icon: "🌟".to_string(),
            category: BadgeCategory::Consistency,
            criteria_type: CriteriaType::ConsecutiveDays,
            criteria_value: 30.0,
            earned: false,
            progress: 0.0,
            target: 30.0,
        },
        Badge {
            id: "ftp_10".to_string(),
            name: "Power Up".to_string(),
            description: "Increase FTP by 10 watts".to_string(),
            icon: "⚡".to_string(),
            category: BadgeCategory::Ftp,
            criteria_type: CriteriaType::FtpIncrease,
            criteria_value: 10.0,
            earned: false,
            progress: 0.0,
            target: 10.0,
        },
        Badge {
            id: "ftp_25".to_string(),
            name: "Power Surge".to_string(),
            description: "Increase FTP by 25 watts".to_string(),
            icon: "💪".to_string(),
            category: BadgeCategory::Ftp,
            criteria_type: CriteriaType::FtpIncrease,
            criteria_value: 25.0,
            earned: false,
            progress: 0.0,
            target: 25.0,
        },
        Badge {
            id: "workouts_10".to_string(),
            name: "Workout Enthusiast".to_string(),
            description: "Complete 10 structured workouts".to_string(),
            icon: "🎯".to_string(),
            category: BadgeCategory::Special,
            criteria_type: CriteriaType::WorkoutsCompleted,
            criteria_value: 10.0,
            earned: false,
            progress: 0.0,
            target: 10.0,
        },
        Badge {
            id: "workouts_50".to_string(),
            name: "Workout Pro".to_string(),
            description: "Complete 50 structured workouts".to_string(),
            icon: "👑".to_string(),
            category: BadgeCategory::Special,
            criteria_type: CriteriaType::WorkoutsCompleted,
            criteria_value: 50.0,
            earned: false,
            progress: 0.0,
            target: 50.0,
        },
    ]
}
