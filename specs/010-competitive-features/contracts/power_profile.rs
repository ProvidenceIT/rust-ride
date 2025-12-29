//! Power Profile Contract
//!
//! Defines the interface for 4D power profiling with rolling windows.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Standard durations for power profile (seconds).
pub const PROFILE_DURATIONS: [u32; 9] = [
    5,    // Neuromuscular sprint
    15,   // Sprint
    30,   // Anaerobic capacity
    60,   // 1-minute power
    180,  // 3-minute power
    300,  // 5-minute power (VO2max)
    600,  // 10-minute power
    1200, // 20-minute power (FTP proxy)
    3600, // 60-minute power (endurance)
];

/// A power value at a specific duration.
#[derive(Debug, Clone, Copy)]
pub struct PowerPoint {
    /// Duration in seconds
    pub duration_secs: u32,
    /// Best average power (watts)
    pub power_watts: u16,
    /// Power per kilogram (if weight known)
    pub power_wkg: Option<f32>,
    /// When this power was achieved
    pub achieved_at: DateTime<Utc>,
    /// Ride where achieved (if tracked)
    pub ride_id: Option<Uuid>,
}

/// Complete power profile at standard durations.
#[derive(Debug, Clone)]
pub struct PowerProfile {
    /// Power points at each standard duration
    pub points: Vec<PowerPoint>,
    /// User's weight used for W/kg calculations
    pub weight_kg: Option<f32>,
    /// Whether this is current (90-day) or lifetime
    pub profile_type: ProfileType,
    /// When profile was last updated
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileType {
    /// Rolling 90-day window for current fitness
    Current,
    /// All-time personal bests
    Lifetime,
}

/// Rider type classification based on power curve shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiderType {
    /// Strong in <30s efforts
    Sprinter,
    /// Strong in 1-5 min efforts
    Pursuiter,
    /// Balanced across all durations
    AllRounder,
    /// Strong in 20-60 min efforts
    TimeTrialist,
    /// Strong in endurance, good W/kg
    Climber,
}

/// Analysis of strengths and weaknesses.
#[derive(Debug, Clone)]
pub struct ProfileAnalysis {
    /// Primary rider classification
    pub rider_type: RiderType,
    /// Durations where user exceeds expected (relative strength)
    pub strengths: Vec<DurationStrength>,
    /// Durations where user is below expected (relative weakness)
    pub weaknesses: Vec<DurationStrength>,
    /// Comparison ratio at each duration vs reference curve
    pub comparison_ratios: Vec<(u32, f32)>,
}

#[derive(Debug, Clone)]
pub struct DurationStrength {
    /// Duration in seconds
    pub duration_secs: u32,
    /// Human-readable duration label (e.g., "5 min")
    pub label: String,
    /// How far from expected (positive = stronger)
    pub deviation_percent: f32,
}

/// Manages power profile calculation and analysis.
pub trait PowerProfileManager: Send + Sync {
    /// Update profiles after a new ride.
    ///
    /// # Arguments
    /// * `ride_id` - UUID of the ride
    /// * `power_samples` - 1-second power samples from ride
    ///
    /// # Returns
    /// List of improved durations (for achievements)
    fn update_from_ride(
        &mut self,
        ride_id: Uuid,
        power_samples: &[u16],
    ) -> Vec<PowerPoint>;

    /// Get current (90-day rolling) power profile.
    fn get_current_profile(&self) -> Option<PowerProfile>;

    /// Get lifetime best power profile.
    fn get_lifetime_profile(&self) -> Option<PowerProfile>;

    /// Get power at a specific duration.
    ///
    /// # Arguments
    /// * `duration_secs` - Duration to query
    /// * `profile_type` - Current or lifetime
    fn get_power_at(
        &self,
        duration_secs: u32,
        profile_type: ProfileType,
    ) -> Option<PowerPoint>;

    /// Analyze the current profile for strengths/weaknesses.
    fn analyze_profile(&self) -> Option<ProfileAnalysis>;

    /// Get historical profiles for trend analysis.
    ///
    /// # Arguments
    /// * `count` - Number of historical snapshots to return
    fn get_profile_history(&self, count: usize) -> Vec<PowerProfile>;

    /// Check if sufficient data exists for meaningful analysis.
    ///
    /// Requires at least 3 rides with efforts across different durations.
    fn has_sufficient_data(&self) -> bool;

    /// Rebuild current profile from ride history.
    ///
    /// Recalculates 90-day rolling window from stored ride data.
    fn rebuild_current_profile(&mut self) -> Result<(), ProfileError>;

    /// Set user weight for W/kg calculations.
    fn set_user_weight(&mut self, weight_kg: f32);
}

/// Errors from power profile operations.
#[derive(Debug, Clone)]
pub enum ProfileError {
    /// No rides found in date range
    NoRideData,
    /// Database error
    StorageError(String),
    /// Invalid power data
    InvalidData(String),
}
