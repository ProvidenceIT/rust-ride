//! Segment timing and leaderboard system.

pub mod leaderboard;
pub mod timing;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Climbing category (Tour de France style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentCategory {
    /// Hors Categorie (beyond category)
    HC,
    /// Category 1 - hardest regular climb
    Cat1,
    /// Category 2
    Cat2,
    /// Category 3
    Cat3,
    /// Category 4 - easiest climb
    Cat4,
    /// Sprint segment (flat/downhill)
    Sprint,
}

impl SegmentCategory {
    /// Determine category from elevation gain and length
    pub fn from_profile(elevation_gain: f32, length_meters: f64) -> Option<Self> {
        let avg_gradient = (elevation_gain as f64 / length_meters * 100.0) as f32;
        let climb_score = elevation_gain * avg_gradient / 100.0;

        if avg_gradient < 1.0 {
            Some(Self::Sprint)
        } else if climb_score > 800.0 {
            Some(Self::HC)
        } else if climb_score > 400.0 {
            Some(Self::Cat1)
        } else if climb_score > 200.0 {
            Some(Self::Cat2)
        } else if climb_score > 100.0 {
            Some(Self::Cat3)
        } else if climb_score > 50.0 {
            Some(Self::Cat4)
        } else {
            None
        }
    }
}

/// A timed segment on a route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique identifier
    pub id: Uuid,
    /// Route this segment belongs to
    pub route_id: Uuid,
    /// Display name
    pub name: String,
    /// Start distance from route start (meters)
    pub start_distance_meters: f64,
    /// End distance from route start (meters)
    pub end_distance_meters: f64,
    /// Segment length in meters
    pub length_meters: f64,
    /// Elevation gain over segment
    pub elevation_gain_meters: f32,
    /// Average gradient over segment
    pub avg_gradient_percent: f32,
    /// Category (HC, 1, 2, 3, 4, Sprint, None)
    pub category: Option<SegmentCategory>,
    /// When created
    pub created_at: DateTime<Utc>,
}

impl Segment {
    /// Create a new segment
    pub fn new(route_id: Uuid, name: String, start: f64, end: f64, elevation_gain: f32) -> Self {
        let length = end - start;
        let avg_gradient = if length > 0.0 {
            (elevation_gain as f64 / length * 100.0) as f32
        } else {
            0.0
        };

        Self {
            id: Uuid::new_v4(),
            route_id,
            name,
            start_distance_meters: start,
            end_distance_meters: end,
            length_meters: length,
            elevation_gain_meters: elevation_gain,
            avg_gradient_percent: avg_gradient,
            category: SegmentCategory::from_profile(elevation_gain, length),
            created_at: Utc::now(),
        }
    }
}

/// A user's recorded time on a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentTime {
    /// Unique identifier
    pub id: Uuid,
    /// Segment this time is for
    pub segment_id: Uuid,
    /// User who recorded this time
    pub user_id: Uuid,
    /// Ride during which this was recorded
    pub ride_id: Uuid,
    /// Time in seconds (to 0.1s precision)
    pub time_seconds: f64,
    /// Average power during segment (if available)
    pub avg_power_watts: Option<u16>,
    /// Average heart rate during segment
    pub avg_heart_rate: Option<u8>,
    /// FTP at time of effort (for relative comparison)
    pub ftp_at_effort: u16,
    /// Whether this is user's personal best
    pub is_personal_best: bool,
    /// When recorded
    pub recorded_at: DateTime<Utc>,
}

impl SegmentTime {
    /// Create a new segment time
    pub fn new(
        segment_id: Uuid,
        user_id: Uuid,
        ride_id: Uuid,
        time_seconds: f64,
        ftp: u16,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            segment_id,
            user_id,
            ride_id,
            time_seconds,
            avg_power_watts: None,
            avg_heart_rate: None,
            ftp_at_effort: ftp,
            is_personal_best: false,
            recorded_at: Utc::now(),
        }
    }

    /// Set power and HR data
    pub fn with_metrics(mut self, power: Option<u16>, hr: Option<u8>) -> Self {
        self.avg_power_watts = power;
        self.avg_heart_rate = hr;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_category_sprint() {
        let cat = SegmentCategory::from_profile(10.0, 2000.0);
        assert_eq!(cat, Some(SegmentCategory::Sprint));
    }

    #[test]
    fn test_segment_category_climb() {
        // Use a steeper climb that will clearly score as Cat2+: 1000m over 4km = 25% avg gradient
        // climb_score = 1000 * 25 / 100 = 250, which is > 200 (Cat2)
        let cat = SegmentCategory::from_profile(1000.0, 4000.0);
        assert!(cat.is_some());
        // Cat3 or higher climbs
        assert!(matches!(
            cat,
            Some(SegmentCategory::Cat1)
                | Some(SegmentCategory::Cat2)
                | Some(SegmentCategory::Cat3)
                | Some(SegmentCategory::HC)
        ));
    }

    #[test]
    fn test_segment_new() {
        let segment = Segment::new(
            Uuid::new_v4(),
            "Test Climb".to_string(),
            1000.0,
            3000.0,
            200.0,
        );
        assert_eq!(segment.length_meters, 2000.0);
        assert!((segment.avg_gradient_percent - 10.0).abs() < 0.1);
    }
}
