//! Unit tests for power profile module.
//!
//! T058: Add unit tests for profile calculations.

use chrono::Utc;
use rustride::power_profile::{
    duration_label, is_standard_duration,
    PowerProfile, PowerProfilePoint, ProfileAnalysis, ProfileType,
    RiderClassification, RiderType,
    EnergySystem, StrengthLevel,
    ProfileComparer, ReferenceLevel,
    male_reference_wpk, female_reference_wpk,
    PowerProfileManager, PowerProfileManagerBuilder,
    MmpAdapter, PROFILE_DURATIONS,
    LifetimeBestTracker,
};
use uuid::Uuid;

// =============================================================================
// Profile Type Tests
// =============================================================================

#[test]
fn test_profile_durations_count() {
    assert_eq!(PROFILE_DURATIONS.len(), 9);
}

#[test]
fn test_standard_durations() {
    assert!(is_standard_duration(5));
    assert!(is_standard_duration(60));
    assert!(is_standard_duration(300));
    assert!(is_standard_duration(1200));
    assert!(is_standard_duration(3600));
    assert!(!is_standard_duration(10));
    assert!(!is_standard_duration(100));
}

#[test]
fn test_duration_labels_seconds() {
    assert_eq!(duration_label(5), "5s");
    assert_eq!(duration_label(30), "30s");
}

#[test]
fn test_duration_labels_minutes() {
    assert_eq!(duration_label(60), "1 min");
    assert_eq!(duration_label(300), "5 min");
    assert_eq!(duration_label(1200), "20 min");
}

#[test]
fn test_duration_labels_hours() {
    assert_eq!(duration_label(3600), "1 hr");
}

// =============================================================================
// PowerProfilePoint Tests
// =============================================================================

#[test]
fn test_power_profile_point_creation() {
    let point = PowerProfilePoint::new(300, 350);
    assert_eq!(point.duration_secs, 300);
    assert_eq!(point.power_watts, 350);
}

#[test]
fn test_power_profile_point_watts_per_kg() {
    let point = PowerProfilePoint::new(300, 280);
    let wpk = point.watts_per_kg(70.0);
    assert!((wpk - 4.0).abs() < 0.001);
}

#[test]
fn test_power_profile_point_zero_weight() {
    let point = PowerProfilePoint::new(300, 280);
    let wpk = point.watts_per_kg(0.0);
    assert_eq!(wpk, 0.0);
}

#[test]
fn test_power_profile_point_with_timestamp() {
    let now = Utc::now();
    let point = PowerProfilePoint::with_timestamp(300, 350, now);
    assert_eq!(point.duration_secs, 300);
    assert_eq!(point.power_watts, 350);
    assert_eq!(point.achieved_at, now);
}

#[test]
fn test_power_profile_point_with_ride() {
    let ride_id = Uuid::new_v4();
    let point = PowerProfilePoint::new(300, 350).with_ride(ride_id);
    assert_eq!(point.ride_id, Some(ride_id));
}

// =============================================================================
// PowerProfile Tests
// =============================================================================

#[test]
fn test_power_profile_creation() {
    let user_id = Uuid::new_v4();
    let profile = PowerProfile::new(user_id, ProfileType::Current);
    assert_eq!(profile.user_id, user_id);
    assert_eq!(profile.profile_type, ProfileType::Current);
    assert!(profile.points.is_empty());
}

#[test]
fn test_power_profile_update() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    // Initial update should be a PR
    let point1 = PowerProfilePoint::new(300, 280);
    let is_pr = profile.update_point(point1);
    assert!(is_pr);
    assert_eq!(profile.power_at_duration(300), Some(280));

    // Lower power should not update
    let point2 = PowerProfilePoint::new(300, 270);
    let is_pr = profile.update_point(point2);
    assert!(!is_pr);
    assert_eq!(profile.power_at_duration(300), Some(280));

    // Higher power should update
    let point3 = PowerProfilePoint::new(300, 300);
    let is_pr = profile.update_point(point3);
    assert!(is_pr);
    assert_eq!(profile.power_at_duration(300), Some(300));
}

#[test]
fn test_power_profile_estimated_ftp() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    // No 20-min power means no FTP
    assert_eq!(profile.estimated_ftp(), None);

    // Add 20-min power
    let point = PowerProfilePoint::new(1200, 280);
    profile.update_point(point);

    // FTP is 95% of 20-min power
    let ftp = profile.estimated_ftp().unwrap();
    assert_eq!(ftp, 266); // 280 * 0.95 = 266
}

#[test]
fn test_power_profile_max_power() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    profile.update_point(PowerProfilePoint::new(5, 800));
    profile.update_point(PowerProfilePoint::new(60, 400));
    profile.update_point(PowerProfilePoint::new(300, 320));

    assert_eq!(profile.max_power(), Some(800));
}

// =============================================================================
// ProfileAnalysis Tests
// =============================================================================

#[test]
fn test_profile_analysis_from_profile() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);

    // Add points for analysis
    profile.update_point(PowerProfilePoint::new(5, 800));
    profile.update_point(PowerProfilePoint::new(60, 400));
    profile.update_point(PowerProfilePoint::new(300, 320));
    profile.update_point(PowerProfilePoint::new(1200, 280));

    let analysis = ProfileAnalysis::from_profile(&profile, Some(70.0));

    // Analysis should have duration analyses
    assert!(!analysis.duration_analyses.is_empty());
}

#[test]
fn test_strength_level_from_deviation() {
    assert_eq!(StrengthLevel::from_deviation(0.0), StrengthLevel::Average);
    assert_eq!(StrengthLevel::from_deviation(-3.0), StrengthLevel::Average);
    assert_eq!(StrengthLevel::from_deviation(3.0), StrengthLevel::Average);
    assert_eq!(StrengthLevel::from_deviation(-10.0), StrengthLevel::Weak);
    assert_eq!(StrengthLevel::from_deviation(10.0), StrengthLevel::Strong);
    assert_eq!(StrengthLevel::from_deviation(-20.0), StrengthLevel::VeryWeak);
    assert_eq!(StrengthLevel::from_deviation(20.0), StrengthLevel::VeryStrong);
}

#[test]
fn test_energy_system_from_duration() {
    assert_eq!(EnergySystem::from_duration(5), EnergySystem::Neuromuscular);
    assert_eq!(EnergySystem::from_duration(15), EnergySystem::Neuromuscular);
    assert_eq!(EnergySystem::from_duration(30), EnergySystem::Anaerobic);
    assert_eq!(EnergySystem::from_duration(60), EnergySystem::Anaerobic);
    assert_eq!(EnergySystem::from_duration(180), EnergySystem::Aerobic);
    assert_eq!(EnergySystem::from_duration(300), EnergySystem::Aerobic);
    assert_eq!(EnergySystem::from_duration(600), EnergySystem::Threshold);
    assert_eq!(EnergySystem::from_duration(1200), EnergySystem::Threshold);
    assert_eq!(EnergySystem::from_duration(3600), EnergySystem::Threshold);
}

// =============================================================================
// RiderType Tests
// =============================================================================

#[test]
fn test_rider_type_display_names() {
    assert_eq!(RiderType::Sprinter.display_name(), "Sprinter");
    assert_eq!(RiderType::Puncher.display_name(), "Puncher");
    assert_eq!(RiderType::Rouleur.display_name(), "Rouleur (Time Trialist)");
    assert_eq!(RiderType::Climber.display_name(), "Climber");
    assert_eq!(RiderType::AllRounder.display_name(), "All-Rounder");
    assert_eq!(RiderType::Unknown.display_name(), "Unclassified");
}

// =============================================================================
// LifetimeBest Tests
// =============================================================================

#[test]
fn test_lifetime_tracker_new_bests() {
    let user_id = Uuid::new_v4();
    let mut tracker = LifetimeBestTracker::new(user_id);
    let ride_id = Uuid::new_v4();
    let now = Utc::now();

    // First ride with 5min PR
    let result = tracker.check_ride(ride_id, now, &[(300, 350)]);
    assert!(!result.new_bests.is_empty());

    // Same power - not a PR
    let result = tracker.check_ride(ride_id, now, &[(300, 350)]);
    assert!(result.new_bests.is_empty());

    // Lower power - not a PR
    let result = tracker.check_ride(ride_id, now, &[(300, 340)]);
    assert!(result.new_bests.is_empty());

    // Higher power - new PR
    let result = tracker.check_ride(ride_id, now, &[(300, 360)]);
    assert!(!result.new_bests.is_empty());
}

// =============================================================================
// ProfileComparer Tests
// =============================================================================

#[test]
fn test_reference_level_ordering() {
    // Test that levels are in the correct order
    assert_eq!(ReferenceLevel::Untrained.next_level(), Some(ReferenceLevel::Recreational));
    assert_eq!(ReferenceLevel::Recreational.next_level(), Some(ReferenceLevel::Trained));
    assert_eq!(ReferenceLevel::Trained.next_level(), Some(ReferenceLevel::Competitive));
    assert_eq!(ReferenceLevel::Competitive.next_level(), Some(ReferenceLevel::Elite));
    assert_eq!(ReferenceLevel::Elite.next_level(), Some(ReferenceLevel::WorldClass));
    assert_eq!(ReferenceLevel::WorldClass.next_level(), None);
}

#[test]
fn test_male_vs_female_reference() {
    let male = male_reference_wpk(ReferenceLevel::Trained);
    let female = female_reference_wpk(ReferenceLevel::Trained);

    // Female values should be lower than male
    assert!(female.wpk_5s < male.wpk_5s);
    assert!(female.wpk_60s < male.wpk_60s);
    assert!(female.wpk_300s < male.wpk_300s);
    assert!(female.wpk_1200s < male.wpk_1200s);
}

#[test]
fn test_reference_curve_interpolation() {
    let curve = male_reference_wpk(ReferenceLevel::Trained);

    // Value at 30s should be between 5s and 60s
    let wpk_30 = curve.wpk_at(30);
    assert!(wpk_30 < curve.wpk_5s);
    assert!(wpk_30 > curve.wpk_60s);

    // Value at 180s should be between 60s and 300s
    let wpk_180 = curve.wpk_at(180);
    assert!(wpk_180 < curve.wpk_60s);
    assert!(wpk_180 > curve.wpk_300s);
}

#[test]
fn test_profile_comparison() {
    let user_id = Uuid::new_v4();
    let mut profile = PowerProfile::new(user_id, ProfileType::Current);
    profile.update_point(PowerProfilePoint::new(300, 280));  // 5-min at 280W

    let comparer = ProfileComparer::new(70.0, false);
    let comparison = comparer.compare(&profile);

    // Should have comparison for 5-min duration
    let five_min = comparison.duration_comparisons.iter()
        .find(|c| c.duration_secs == 300);
    assert!(five_min.is_some());

    let five_min = five_min.unwrap();
    assert_eq!(five_min.actual_power, 280);
    assert!((five_min.actual_wpk - 4.0).abs() < 0.001);
}

// =============================================================================
// PowerProfileManager Tests
// =============================================================================

#[test]
fn test_manager_creation() {
    let user_id = Uuid::new_v4();
    let manager = PowerProfileManager::new(user_id);

    assert!(!manager.has_sufficient_data());
    assert_eq!(manager.ride_count(), 0);
}

#[test]
fn test_manager_set_weight() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(75.0);

    // Weight is stored internally
    // We can test it by checking W/kg calculations work
    let ride_id = Uuid::new_v4();
    let now = Utc::now();
    manager.process_ride(ride_id, now, vec![(1200, 300)]);

    let wpk = manager.watts_per_kg_ftp();
    assert!(wpk.is_some());
    // 300 * 0.95 / 75 = 3.8 W/kg
    assert!((wpk.unwrap() - 3.8).abs() < 0.1);
}

#[test]
fn test_manager_process_ride() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let ride_id = Uuid::new_v4();
    let now = Utc::now();

    // Process a ride with MMP values
    let mmp_values = vec![
        (5, 800),
        (60, 400),
        (300, 320),
        (1200, 280),
    ];

    let result = manager.process_ride(ride_id, now, mmp_values);

    // First ride should have PRs
    assert!(result.has_new_prs());
    assert!(!result.rolling_prs.is_empty());
}

#[test]
fn test_manager_ftp_estimation() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let ride_id = Uuid::new_v4();
    let now = Utc::now();

    // Process a ride with 20-min power
    let mmp_values = vec![
        (1200, 300),  // 300W for 20 min
    ];

    manager.process_ride(ride_id, now, mmp_values);

    // FTP should be 95% of 20-min power
    let ftp = manager.estimated_ftp_rolling();
    assert!(ftp.is_some());
    assert_eq!(ftp.unwrap(), 285); // 300 * 0.95 = 285
}

#[test]
fn test_manager_classification() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let now = Utc::now();

    // Add a "sprinter" profile
    manager.process_ride(
        Uuid::new_v4(),
        now,
        vec![
            (5, 1200),   // Very strong sprint
            (15, 900),
            (30, 600),
            (60, 450),
            (180, 350),
            (300, 300),
            (600, 270),
            (1200, 250),
            (3600, 200),
        ],
    );

    // Should classify as Sprinter
    assert!(manager.classification().is_some());
    assert_eq!(manager.rider_type(), RiderType::Sprinter);
}

#[test]
fn test_manager_builder() {
    let user_id = Uuid::new_v4();

    let manager = PowerProfileManagerBuilder::new(user_id)
        .weight_kg(75.0)
        .build();

    // Manager should be built with weight
    let ride_id = Uuid::new_v4();
    let now = Utc::now();

    // Test that weight is set by checking W/kg calculations
    let mut manager = manager;
    manager.process_ride(ride_id, now, vec![(1200, 300)]);
    let wpk = manager.watts_per_kg_ftp();
    assert!(wpk.is_some());
}

#[test]
fn test_manager_compare_rolling_to_lifetime() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let now = Utc::now();

    // Process a ride
    manager.process_ride(
        Uuid::new_v4(),
        now,
        vec![(300, 350)],
    );

    let comparison = manager.compare_rolling_to_lifetime();

    // Should have comparison data
    assert!(!comparison.is_empty());

    // Rolling should be at 100% of lifetime since same data
    for (_, pct) in &comparison {
        assert!((*pct - 100.0).abs() < 1.0);
    }
}

// =============================================================================
// MmpAdapter Tests
// =============================================================================

#[test]
fn test_mmp_adapter_profile_mmp() {
    // Generate power samples (10 minutes at various power levels)
    let samples: Vec<u16> = (0..600)
        .map(|i| {
            if i < 5 {
                900  // Sprint start
            } else if i < 60 {
                400
            } else {
                300
            }
        })
        .collect();

    let mmp = MmpAdapter::calculate_profile_mmp(&samples);

    // Should have values for standard durations that fit in 600 seconds
    assert!(!mmp.is_empty());

    // 5-second power should be high (sprint)
    let five_sec = mmp.iter().find(|(d, _)| *d == 5);
    assert!(five_sec.is_some());
}

#[test]
fn test_mmp_adapter_create_ride_data() {
    let samples: Vec<u16> = vec![300; 600];  // 10 min at 300W
    let ride_id = Uuid::new_v4();
    let now = Utc::now();

    let ride_data = MmpAdapter::create_ride_data(ride_id, now, &samples);

    assert_eq!(ride_data.ride_id, ride_id);
    assert_eq!(ride_data.ride_date, now);
    assert!(!ride_data.mmp_values.is_empty());
}

#[test]
fn test_mmp_adapter_empty_samples() {
    let samples: Vec<u16> = vec![];
    let mmp = MmpAdapter::calculate_profile_mmp(&samples);
    assert!(mmp.is_empty());
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_workflow() {
    // Create manager
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let now = Utc::now();

    // Simulate multiple rides over time
    for i in 0..5 {
        let ride_id = Uuid::new_v4();
        let ride_date = now - chrono::Duration::days(i * 7);

        // Power increases slightly each ride (simulating training)
        let base_power = 280 + i as u16 * 5;
        let mmp_values = vec![
            (5, base_power * 3),
            (60, base_power + 100),
            (300, base_power),
            (1200, (base_power as f64 * 0.9) as u16),
        ];

        manager.process_ride(ride_id, ride_date, mmp_values);
    }

    // Check that profile has been built
    assert!(manager.has_sufficient_data());

    // Check FTP is calculated
    let ftp = manager.estimated_ftp_rolling();
    assert!(ftp.is_some());
}

#[test]
fn test_profile_lifetime_vs_rolling() {
    let user_id = Uuid::new_v4();
    let mut manager = PowerProfileManager::new(user_id);
    manager.set_weight(70.0);

    let now = Utc::now();

    // Add a strong ride from the past
    let old_ride = Uuid::new_v4();
    let old_date = now - chrono::Duration::days(10);
    manager.process_ride(old_ride, old_date, vec![(300, 350)]);  // Strong 5-min

    // Both rolling and lifetime should show 350W
    assert_eq!(manager.rolling_profile().power_at_duration(300), Some(350));
    assert_eq!(manager.lifetime_profile().power_at_duration(300), Some(350));

    // Add a weaker ride (should not affect either profile)
    let new_ride = Uuid::new_v4();
    let new_date = now - chrono::Duration::days(5);
    manager.process_ride(new_ride, new_date, vec![(300, 300)]);

    // Both should still show 350W (best power)
    assert_eq!(manager.rolling_profile().power_at_duration(300), Some(350));
    assert_eq!(manager.lifetime_profile().power_at_duration(300), Some(350));
}
