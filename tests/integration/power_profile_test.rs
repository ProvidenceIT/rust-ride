//! Integration tests for power profile updates.
//!
//! T085: Integration test for power profile updates.

use rustride::power_profile::{
    DurationStrength, EnergySystem, LifetimeBestTracker, PowerProfile, PowerProfilePoint,
    ProfileAnalysis, ProfileType, RiderType, StrengthLevel,
};
use uuid::Uuid;

#[test]
fn test_power_profile_creation() {
    let user_id = Uuid::new_v4();
    let profile = PowerProfile::new(user_id, ProfileType::Current);
    assert!(profile.points.is_empty());
}

#[test]
fn test_power_profile_point() {
    let point = PowerProfilePoint::new(300, 350);

    assert_eq!(point.duration_secs, 300);
    assert_eq!(point.power_watts, 350);
}

#[test]
fn test_profile_type() {
    assert_eq!(ProfileType::Current.display_name(), "Current (90-day)");
    assert_eq!(ProfileType::Lifetime.display_name(), "Lifetime Best");
}

#[test]
fn test_rider_type_display() {
    assert_eq!(RiderType::Sprinter.display_name(), "Sprinter");
    assert_eq!(RiderType::Puncher.display_name(), "Puncher");
    assert_eq!(RiderType::Rouleur.display_name(), "Rouleur (Time Trialist)");
    assert_eq!(RiderType::AllRounder.display_name(), "All-Rounder");
    assert_eq!(RiderType::Climber.display_name(), "Climber");
}

#[test]
fn test_energy_system_classification() {
    // Test energy system display names
    assert_eq!(
        EnergySystem::Neuromuscular.display_name(),
        "Neuromuscular (Sprint)"
    );
    assert_eq!(EnergySystem::Anaerobic.display_name(), "Anaerobic Capacity");
    assert_eq!(EnergySystem::Aerobic.display_name(), "VO2max (Aerobic)");
    assert_eq!(
        EnergySystem::Threshold.display_name(),
        "Threshold (Endurance)"
    );
}

#[test]
fn test_energy_system_from_duration() {
    // Short durations are neuromuscular
    assert_eq!(EnergySystem::from_duration(5), EnergySystem::Neuromuscular);
    assert_eq!(EnergySystem::from_duration(15), EnergySystem::Neuromuscular);

    // 30-60 seconds is anaerobic
    assert_eq!(EnergySystem::from_duration(30), EnergySystem::Anaerobic);
    assert_eq!(EnergySystem::from_duration(60), EnergySystem::Anaerobic);

    // 3-5 minutes is aerobic
    assert_eq!(EnergySystem::from_duration(180), EnergySystem::Aerobic);
    assert_eq!(EnergySystem::from_duration(300), EnergySystem::Aerobic);

    // 20+ minutes is threshold
    assert_eq!(EnergySystem::from_duration(1200), EnergySystem::Threshold);
}

#[test]
fn test_strength_level_from_deviation() {
    assert_eq!(
        StrengthLevel::from_deviation(-20.0),
        StrengthLevel::VeryWeak
    );
    assert_eq!(StrengthLevel::from_deviation(-10.0), StrengthLevel::Weak);
    assert_eq!(StrengthLevel::from_deviation(0.0), StrengthLevel::Average);
    assert_eq!(StrengthLevel::from_deviation(10.0), StrengthLevel::Strong);
    assert_eq!(
        StrengthLevel::from_deviation(20.0),
        StrengthLevel::VeryStrong
    );
}

#[test]
fn test_strength_level_display() {
    assert_eq!(StrengthLevel::VeryWeak.display_name(), "Very Weak");
    assert_eq!(StrengthLevel::Weak.display_name(), "Weak");
    assert_eq!(StrengthLevel::Average.display_name(), "Average");
    assert_eq!(StrengthLevel::Strong.display_name(), "Strong");
    assert_eq!(StrengthLevel::VeryStrong.display_name(), "Very Strong");
}

#[test]
fn test_lifetime_best_tracker_creation() {
    let user_id = Uuid::new_v4();
    let tracker = LifetimeBestTracker::new(user_id);

    // New tracker should have no bests
    assert!(tracker.best_at(300).is_none());
    assert!(tracker.best_at(1200).is_none());
}

#[test]
fn test_lifetime_best_check_ride() {
    let user_id = Uuid::new_v4();
    let ride_id = Uuid::new_v4();
    let mut tracker = LifetimeBestTracker::new(user_id);
    let now = chrono::Utc::now();

    // Check a ride with power values
    let mmp_values: Vec<(u32, u16)> = vec![(5, 900), (60, 450), (300, 350), (1200, 280)];

    let result = tracker.check_ride(ride_id, now, &mmp_values);

    // All values should be new bests (first ride)
    assert!(result.has_new_bests());
    assert_eq!(result.new_pr_count(), 4);

    // Verify bests were recorded
    assert_eq!(tracker.best_at(5), Some(900));
    assert_eq!(tracker.best_at(60), Some(450));
    assert_eq!(tracker.best_at(300), Some(350));
    assert_eq!(tracker.best_at(1200), Some(280));
}

#[test]
fn test_lifetime_best_only_improves() {
    let user_id = Uuid::new_v4();
    let mut tracker = LifetimeBestTracker::new(user_id);
    let now = chrono::Utc::now();

    // First ride with initial values
    let ride1_id = Uuid::new_v4();
    let mmp1: Vec<(u32, u16)> = vec![(300, 350)];
    tracker.check_ride(ride1_id, now, &mmp1);
    assert_eq!(tracker.best_at(300), Some(350));

    // Second ride with lower value (should not update)
    let ride2_id = Uuid::new_v4();
    let mmp2: Vec<(u32, u16)> = vec![(300, 340)];
    let result2 = tracker.check_ride(ride2_id, now, &mmp2);
    assert!(!result2.has_new_bests());
    assert_eq!(tracker.best_at(300), Some(350)); // Still the first value

    // Third ride with higher value (should update)
    let ride3_id = Uuid::new_v4();
    let mmp3: Vec<(u32, u16)> = vec![(300, 380)];
    let result3 = tracker.check_ride(ride3_id, now, &mmp3);
    assert!(result3.has_new_bests());
    assert_eq!(tracker.best_at(300), Some(380)); // Updated
}

#[test]
fn test_power_profile_with_points() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    // Add points
    profile.update_point(PowerProfilePoint::new(5, 900));
    profile.update_point(PowerProfilePoint::new(60, 450));
    profile.update_point(PowerProfilePoint::new(300, 350));

    assert_eq!(profile.points.len(), 3);
    assert_eq!(profile.power_at_duration(5), Some(900));
    assert_eq!(profile.power_at_duration(300), Some(350));
}

#[test]
fn test_profile_analysis_from_profile() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    // Add points representing a balanced rider
    profile.update_point(PowerProfilePoint::new(5, 900));
    profile.update_point(PowerProfilePoint::new(60, 450));
    profile.update_point(PowerProfilePoint::new(300, 350));
    profile.update_point(PowerProfilePoint::new(1200, 280));

    let analysis = ProfileAnalysis::from_profile(&profile, Some(75.0));

    // Should have analysis for the durations
    assert!(!analysis.duration_analyses.is_empty());
    assert!(analysis.estimated_ftp.is_some());
}

#[test]
fn test_profile_durations_standard() {
    use rustride::power_profile::PROFILE_DURATIONS;

    // Should have standard durations for power profiling
    assert!(PROFILE_DURATIONS.contains(&5));
    assert!(PROFILE_DURATIONS.contains(&60));
    assert!(PROFILE_DURATIONS.contains(&300));
    assert!(PROFILE_DURATIONS.contains(&1200));
}

#[test]
fn test_duration_label() {
    use rustride::power_profile::duration_label;

    assert_eq!(duration_label(5), "5s");
    assert_eq!(duration_label(60), "1 min");
    assert_eq!(duration_label(300), "5 min");
    assert_eq!(duration_label(1200), "20 min");
}

#[test]
fn test_is_standard_duration() {
    use rustride::power_profile::is_standard_duration;

    assert!(is_standard_duration(5));
    assert!(is_standard_duration(60));
    assert!(is_standard_duration(300));
    assert!(!is_standard_duration(7));
    assert!(!is_standard_duration(45));
}

#[test]
fn test_full_profile_workflow() {
    let user_id = Uuid::new_v4();
    let mut tracker = LifetimeBestTracker::new(user_id);
    let now = chrono::Utc::now();

    // First ride establishing baseline
    let ride1_id = Uuid::new_v4();
    let mmp1: Vec<(u32, u16)> = vec![(5, 900), (60, 450), (300, 350), (1200, 280)];
    let result1 = tracker.check_ride(ride1_id, now, &mmp1);
    assert_eq!(result1.new_pr_count(), 4);

    // Second ride with some PRs
    let ride2_id = Uuid::new_v4();
    let mmp2: Vec<(u32, u16)> = vec![
        (5, 920),    // New PR!
        (60, 440),   // Not a PR
        (300, 360),  // New PR!
        (1200, 275), // Not a PR
    ];
    let result2 = tracker.check_ride(ride2_id, now, &mmp2);

    // Should have found 2 PRs
    assert_eq!(result2.new_pr_count(), 2);

    // Verify PRs were recorded
    assert_eq!(tracker.best_at(5), Some(920));
    assert_eq!(tracker.best_at(300), Some(360));

    // Non-PRs should still be original values
    assert_eq!(tracker.best_at(60), Some(450));
    assert_eq!(tracker.best_at(1200), Some(280));
}

#[test]
fn test_duration_strength_is_strength() {
    let strength = DurationStrength {
        duration_secs: 300,
        power_watts: 350,
        watts_per_kg: Some(4.67),
        deviation_percent: 10.0,
        strength_level: StrengthLevel::Strong,
        energy_system: EnergySystem::Aerobic,
    };

    assert!(strength.is_strength());
    assert!(!strength.is_weakness());
}

#[test]
fn test_duration_strength_is_weakness() {
    let weakness = DurationStrength {
        duration_secs: 5,
        power_watts: 700,
        watts_per_kg: Some(9.33),
        deviation_percent: -12.0,
        strength_level: StrengthLevel::Weak,
        energy_system: EnergySystem::Neuromuscular,
    };

    assert!(weakness.is_weakness());
    assert!(!weakness.is_strength());
}

#[test]
fn test_estimated_ftp() {
    let user_id = Uuid::new_v4();
    let mut tracker = LifetimeBestTracker::new(user_id);
    let now = chrono::Utc::now();

    // Add 20-minute best (FTP estimate = 95% of 20min power)
    let ride_id = Uuid::new_v4();
    let mmp: Vec<(u32, u16)> = vec![
        (1200, 300), // 20min power = 300W, FTP ~= 285W
    ];
    tracker.check_ride(ride_id, now, &mmp);

    // Should have an FTP estimate
    let ftp = tracker.estimated_ftp();
    assert!(ftp.is_some());
}

#[test]
fn test_rider_type_classification() {
    // Check that RiderType variants exist
    let _ = RiderType::Sprinter;
    let _ = RiderType::Puncher;
    let _ = RiderType::Rouleur;
    let _ = RiderType::Climber;
    let _ = RiderType::AllRounder;
    let _ = RiderType::Unknown;
}

#[test]
fn test_rider_type_all() {
    let all_types = RiderType::all();
    assert!(!all_types.is_empty());
    assert!(all_types.contains(&RiderType::Sprinter));
}
