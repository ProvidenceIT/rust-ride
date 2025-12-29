//! Power profile type definitions.

use serde::{Deserialize, Serialize};

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

/// Profile type: current (rolling 90-day) or lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfileType {
    /// Rolling 90-day window for current fitness
    Current,
    /// All-time personal bests
    Lifetime,
}

impl ProfileType {
    /// Get display name for the profile type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Current => "Current (90-day)",
            Self::Lifetime => "Lifetime Best",
        }
    }
}

/// Get a human-readable label for a duration.
pub fn duration_label(duration_secs: u32) -> String {
    if duration_secs < 60 {
        format!("{}s", duration_secs)
    } else if duration_secs < 3600 {
        let mins = duration_secs / 60;
        format!("{} min", mins)
    } else {
        let hours = duration_secs / 3600;
        format!("{} hr", hours)
    }
}

/// Get the duration bucket for a given duration in seconds.
/// Returns the standard duration that best represents this effort.
pub fn get_duration_bucket(duration_secs: u32) -> Option<u32> {
    // Find the closest standard duration
    PROFILE_DURATIONS
        .iter()
        .copied()
        .find(|&d| duration_secs <= d)
}

/// Check if a duration is a standard profile duration.
pub fn is_standard_duration(duration_secs: u32) -> bool {
    PROFILE_DURATIONS.contains(&duration_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_durations() {
        assert_eq!(PROFILE_DURATIONS.len(), 9);
        assert_eq!(PROFILE_DURATIONS[0], 5);
        assert_eq!(PROFILE_DURATIONS[8], 3600);
    }

    #[test]
    fn test_duration_labels() {
        assert_eq!(duration_label(5), "5s");
        assert_eq!(duration_label(30), "30s");
        assert_eq!(duration_label(60), "1 min");
        assert_eq!(duration_label(300), "5 min");
        assert_eq!(duration_label(3600), "1 hr");
    }

    #[test]
    fn test_duration_bucket() {
        assert_eq!(get_duration_bucket(3), Some(5));
        assert_eq!(get_duration_bucket(5), Some(5));
        assert_eq!(get_duration_bucket(10), Some(15));
        assert_eq!(get_duration_bucket(60), Some(60));
    }

    #[test]
    fn test_is_standard_duration() {
        assert!(is_standard_duration(5));
        assert!(is_standard_duration(300));
        assert!(!is_standard_duration(45));
        assert!(!is_standard_duration(120));
    }
}
