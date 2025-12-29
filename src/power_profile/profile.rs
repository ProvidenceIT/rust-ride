//! Power profile structures for multi-duration power analysis.
//!
//! T046: Create PowerProfile and PowerProfilePoint structs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{duration_label, ProfileType, PROFILE_DURATIONS};

/// A single power data point at a specific duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerProfilePoint {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Peak power achieved at this duration.
    pub power_watts: u16,
    /// When this power was achieved.
    pub achieved_at: DateTime<Utc>,
    /// Which ride this came from (if tracked).
    pub ride_id: Option<Uuid>,
}

impl PowerProfilePoint {
    /// Create a new power profile point.
    pub fn new(duration_secs: u32, power_watts: u16) -> Self {
        Self {
            duration_secs,
            power_watts,
            achieved_at: Utc::now(),
            ride_id: None,
        }
    }

    /// Create with specific timestamp.
    pub fn with_timestamp(duration_secs: u32, power_watts: u16, achieved_at: DateTime<Utc>) -> Self {
        Self {
            duration_secs,
            power_watts,
            achieved_at,
            ride_id: None,
        }
    }

    /// Associate with a ride.
    pub fn with_ride(mut self, ride_id: Uuid) -> Self {
        self.ride_id = Some(ride_id);
        self
    }

    /// Get human-readable duration label.
    pub fn duration_label(&self) -> String {
        duration_label(self.duration_secs)
    }

    /// Calculate watts per kg given rider weight.
    pub fn watts_per_kg(&self, weight_kg: f64) -> f64 {
        if weight_kg > 0.0 {
            self.power_watts as f64 / weight_kg
        } else {
            0.0
        }
    }
}

/// A complete power profile with multiple duration points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerProfile {
    /// Unique identifier for this profile.
    pub id: Uuid,
    /// User this profile belongs to.
    pub user_id: Uuid,
    /// Profile type (current 90-day or lifetime).
    pub profile_type: ProfileType,
    /// When this profile was last updated.
    pub updated_at: DateTime<Utc>,
    /// Power points at standard durations.
    pub points: Vec<PowerProfilePoint>,
    /// Whether this is the current active profile.
    pub is_current: bool,
}

impl PowerProfile {
    /// Create a new empty profile.
    pub fn new(user_id: Uuid, profile_type: ProfileType) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            profile_type,
            updated_at: Utc::now(),
            points: Vec::new(),
            is_current: true,
        }
    }

    /// Create a profile with initial points.
    pub fn with_points(user_id: Uuid, profile_type: ProfileType, points: Vec<PowerProfilePoint>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            profile_type,
            updated_at: Utc::now(),
            points,
            is_current: true,
        }
    }

    /// Get power at a specific duration.
    pub fn power_at_duration(&self, duration_secs: u32) -> Option<u16> {
        self.points
            .iter()
            .find(|p| p.duration_secs == duration_secs)
            .map(|p| p.power_watts)
    }

    /// Get the point at a specific duration.
    pub fn point_at_duration(&self, duration_secs: u32) -> Option<&PowerProfilePoint> {
        self.points
            .iter()
            .find(|p| p.duration_secs == duration_secs)
    }

    /// Update or add a power point.
    /// Returns true if this was a new PR at this duration.
    pub fn update_point(&mut self, point: PowerProfilePoint) -> bool {
        let existing = self.points.iter_mut().find(|p| p.duration_secs == point.duration_secs);

        match existing {
            Some(existing_point) => {
                if point.power_watts > existing_point.power_watts {
                    *existing_point = point;
                    self.updated_at = Utc::now();
                    true
                } else {
                    false
                }
            }
            None => {
                self.points.push(point);
                self.updated_at = Utc::now();
                true
            }
        }
    }

    /// Check if this profile has data for all standard durations.
    pub fn is_complete(&self) -> bool {
        PROFILE_DURATIONS.iter().all(|&d|
            self.points.iter().any(|p| p.duration_secs == d)
        )
    }

    /// Get the number of duration points recorded.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Get estimated FTP from 20-minute power (95% of 20min power).
    pub fn estimated_ftp(&self) -> Option<u16> {
        self.power_at_duration(1200).map(|p20| (p20 as f64 * 0.95) as u16)
    }

    /// Get the maximum power across all durations.
    pub fn max_power(&self) -> Option<u16> {
        self.points.iter().map(|p| p.power_watts).max()
    }

    /// Get watts per kg for all points.
    pub fn watts_per_kg_profile(&self, weight_kg: f64) -> Vec<(u32, f64)> {
        self.points
            .iter()
            .map(|p| (p.duration_secs, p.watts_per_kg(weight_kg)))
            .collect()
    }

    /// Get the age of the most recent data point.
    pub fn most_recent_point(&self) -> Option<&PowerProfilePoint> {
        self.points.iter().max_by_key(|p| p.achieved_at)
    }

    /// Filter points to only include those within a date range.
    pub fn filter_by_date_range(&self, start: NaiveDate, end: NaiveDate) -> Vec<&PowerProfilePoint> {
        self.points
            .iter()
            .filter(|p| {
                let date = p.achieved_at.date_naive();
                date >= start && date <= end
            })
            .collect()
    }
}

impl Default for PowerProfile {
    fn default() -> Self {
        Self::new(Uuid::nil(), ProfileType::Current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_profile_point() {
        let point = PowerProfilePoint::new(300, 350);
        assert_eq!(point.duration_secs, 300);
        assert_eq!(point.power_watts, 350);
        assert_eq!(point.duration_label(), "5 min");
    }

    #[test]
    fn test_watts_per_kg() {
        let point = PowerProfilePoint::new(1200, 300);
        let wpk = point.watts_per_kg(75.0);
        assert!((wpk - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_profile_update_pr() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        // First point
        let point1 = PowerProfilePoint::new(300, 300);
        assert!(profile.update_point(point1));
        assert_eq!(profile.power_at_duration(300), Some(300));

        // Better PR
        let point2 = PowerProfilePoint::new(300, 350);
        assert!(profile.update_point(point2));
        assert_eq!(profile.power_at_duration(300), Some(350));

        // Worse power - no update
        let point3 = PowerProfilePoint::new(300, 320);
        assert!(!profile.update_point(point3));
        assert_eq!(profile.power_at_duration(300), Some(350));
    }

    #[test]
    fn test_estimated_ftp() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        // 20-minute power of 300W = FTP of ~285W
        let point = PowerProfilePoint::new(1200, 300);
        profile.update_point(point);

        let ftp = profile.estimated_ftp();
        assert_eq!(ftp, Some(285));
    }

    #[test]
    fn test_profile_completeness() {
        let user_id = Uuid::new_v4();
        let mut profile = PowerProfile::new(user_id, ProfileType::Current);

        assert!(!profile.is_complete());

        // Add all standard durations
        for &duration in &PROFILE_DURATIONS {
            let point = PowerProfilePoint::new(duration, 200);
            profile.update_point(point);
        }

        assert!(profile.is_complete());
    }
}
