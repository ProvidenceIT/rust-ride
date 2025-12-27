//! Unit tests for zone calculations.
//!
//! T104: Unit test for zone determination from power/HR
//! T113: Unit test for zone calculation from FTP
//! T114: Unit test for HR zone calculation

use rustride::metrics::zones::{HRZones, PowerZones, HR_ZONE_COLORS, POWER_ZONE_COLORS};

#[test]
fn test_power_zones_from_ftp_200() {
    let zones = PowerZones::from_ftp(200);

    // Verify zone boundaries
    // Z1: 0-55% = 0-110W
    assert_eq!(zones.z1_recovery.min_watts, 0);
    assert_eq!(zones.z1_recovery.max_watts, 110);
    assert_eq!(zones.z1_recovery.zone, 1);

    // Z2: 56-75% = 112-150W
    assert_eq!(zones.z2_endurance.min_watts, 112);
    assert_eq!(zones.z2_endurance.max_watts, 150);
    assert_eq!(zones.z2_endurance.zone, 2);

    // Z3: 76-90% = 152-180W
    assert_eq!(zones.z3_tempo.min_watts, 152);
    assert_eq!(zones.z3_tempo.max_watts, 180);

    // Z4: 91-105% = 182-210W (threshold = FTP)
    assert_eq!(zones.z4_threshold.min_watts, 182);
    assert_eq!(zones.z4_threshold.max_watts, 210);

    // Z5: 106-120% = 212-240W
    assert_eq!(zones.z5_vo2max.min_watts, 212);
    assert_eq!(zones.z5_vo2max.max_watts, 240);

    // Z6: 121-150% = 242-300W
    assert_eq!(zones.z6_anaerobic.min_watts, 242);
    assert_eq!(zones.z6_anaerobic.max_watts, 300);

    // Z7: >150% = >302W
    assert_eq!(zones.z7_neuromuscular.min_watts, 302);
}

#[test]
fn test_power_zones_from_ftp_300() {
    let zones = PowerZones::from_ftp(300);

    // Z4 should span 91-105% = 273-315W
    assert_eq!(zones.z4_threshold.min_watts, 273);
    assert_eq!(zones.z4_threshold.max_watts, 315);

    // Z5: 106-120% = 318-360W
    assert_eq!(zones.z5_vo2max.min_watts, 318);
    assert_eq!(zones.z5_vo2max.max_watts, 360);
}

#[test]
fn test_power_zone_lookup() {
    let zones = PowerZones::from_ftp(200);

    // Test zone boundaries
    assert_eq!(zones.get_zone(0), 1);
    assert_eq!(zones.get_zone(50), 1);
    assert_eq!(zones.get_zone(110), 1);

    assert_eq!(zones.get_zone(111), 2);
    assert_eq!(zones.get_zone(130), 2);
    assert_eq!(zones.get_zone(150), 2);

    assert_eq!(zones.get_zone(151), 3);
    assert_eq!(zones.get_zone(170), 3);
    assert_eq!(zones.get_zone(180), 3);

    assert_eq!(zones.get_zone(181), 4);
    assert_eq!(zones.get_zone(200), 4); // At FTP
    assert_eq!(zones.get_zone(210), 4);

    assert_eq!(zones.get_zone(211), 5);
    assert_eq!(zones.get_zone(220), 5);
    assert_eq!(zones.get_zone(240), 5);

    assert_eq!(zones.get_zone(241), 6);
    assert_eq!(zones.get_zone(280), 6);
    assert_eq!(zones.get_zone(300), 6);

    assert_eq!(zones.get_zone(301), 7);
    assert_eq!(zones.get_zone(400), 7);
    assert_eq!(zones.get_zone(1000), 7);
}

#[test]
fn test_power_zone_names() {
    let zones = PowerZones::from_ftp(200);

    assert_eq!(zones.z1_recovery.name, "Active Recovery");
    assert_eq!(zones.z2_endurance.name, "Endurance");
    assert_eq!(zones.z3_tempo.name, "Tempo");
    assert_eq!(zones.z4_threshold.name, "Threshold");
    assert_eq!(zones.z5_vo2max.name, "VO2max");
    assert_eq!(zones.z6_anaerobic.name, "Anaerobic");
    assert_eq!(zones.z7_neuromuscular.name, "Neuromuscular");
}

#[test]
fn test_power_zone_colors_defined() {
    assert_eq!(POWER_ZONE_COLORS.len(), 7);

    // Verify all colors are different
    for i in 0..7 {
        for j in (i + 1)..7 {
            let c1 = &POWER_ZONE_COLORS[i];
            let c2 = &POWER_ZONE_COLORS[j];
            assert!(
                c1.r != c2.r || c1.g != c2.g || c1.b != c2.b,
                "Zone {} and {} should have different colors",
                i + 1,
                j + 1
            );
        }
    }
}

#[test]
fn test_power_zone_range_lookup() {
    let zones = PowerZones::from_ftp(200);

    assert!(zones.get_zone_range(0).is_none());
    assert!(zones.get_zone_range(8).is_none());

    assert_eq!(zones.get_zone_range(1).unwrap().name, "Active Recovery");
    assert_eq!(zones.get_zone_range(4).unwrap().name, "Threshold");
    assert_eq!(zones.get_zone_range(7).unwrap().name, "Neuromuscular");
}

#[test]
fn test_power_zones_all_zones() {
    let zones = PowerZones::from_ftp(200);
    let all = zones.all_zones();

    assert_eq!(all.len(), 7);
    assert_eq!(all[0].zone, 1);
    assert_eq!(all[6].zone, 7);
}

// ========== Heart Rate Zone Tests ==========

#[test]
fn test_hr_zones_from_karvonen() {
    // Max HR 180, Resting HR 60 => HRR = 120
    let zones = HRZones::from_hr(180, 60);

    // Z1: 50-60% HRR = 60 + (120 * 0.5-0.6) = 120-132 bpm
    assert_eq!(zones.z1_recovery.min_bpm, 120);
    assert_eq!(zones.z1_recovery.max_bpm, 132);
    assert_eq!(zones.z1_recovery.zone, 1);

    // Z2: 60-70% HRR = 60 + (120 * 0.6-0.7) = 132-144 bpm
    assert_eq!(zones.z2_aerobic.min_bpm, 132);
    assert_eq!(zones.z2_aerobic.max_bpm, 144);

    // Z3: 70-80% HRR = 60 + (120 * 0.7-0.8) = 144-156 bpm
    assert_eq!(zones.z3_tempo.min_bpm, 144);
    assert_eq!(zones.z3_tempo.max_bpm, 156);

    // Z4: 80-90% HRR = 60 + (120 * 0.8-0.9) = 156-168 bpm
    assert_eq!(zones.z4_threshold.min_bpm, 156);
    assert_eq!(zones.z4_threshold.max_bpm, 168);

    // Z5: 90-100% HRR = 60 + (120 * 0.9-1.0) = 168-180 bpm
    assert_eq!(zones.z5_maximum.min_bpm, 168);
    assert_eq!(zones.z5_maximum.max_bpm, 180);
}

#[test]
fn test_hr_zones_different_hrr() {
    // Max HR 190, Resting HR 50 => HRR = 140
    let zones = HRZones::from_hr(190, 50);

    // Z1: 50-60% HRR = 50 + (140 * 0.5-0.6) = 120-134 bpm
    assert_eq!(zones.z1_recovery.min_bpm, 120);
    assert_eq!(zones.z1_recovery.max_bpm, 134);

    // Z5 max should be max_hr
    assert_eq!(zones.z5_maximum.max_bpm, 190);
}

#[test]
fn test_hr_zone_lookup() {
    let zones = HRZones::from_hr(180, 60);

    // Below Z1
    assert_eq!(zones.get_zone(100), 0);
    assert_eq!(zones.get_zone(119), 0);

    // Z1
    assert_eq!(zones.get_zone(120), 1);
    assert_eq!(zones.get_zone(125), 1);
    assert_eq!(zones.get_zone(132), 1);

    // Z2
    assert_eq!(zones.get_zone(133), 2);
    assert_eq!(zones.get_zone(140), 2);
    assert_eq!(zones.get_zone(144), 2);

    // Z3
    assert_eq!(zones.get_zone(145), 3);
    assert_eq!(zones.get_zone(150), 3);
    assert_eq!(zones.get_zone(156), 3);

    // Z4
    assert_eq!(zones.get_zone(157), 4);
    assert_eq!(zones.get_zone(165), 4);
    assert_eq!(zones.get_zone(168), 4);

    // Z5
    assert_eq!(zones.get_zone(169), 5);
    assert_eq!(zones.get_zone(175), 5);
    assert_eq!(zones.get_zone(180), 5);
}

#[test]
fn test_hr_zone_names() {
    let zones = HRZones::from_hr(180, 60);

    assert_eq!(zones.z1_recovery.name, "Recovery");
    assert_eq!(zones.z2_aerobic.name, "Aerobic");
    assert_eq!(zones.z3_tempo.name, "Tempo");
    assert_eq!(zones.z4_threshold.name, "Threshold");
    assert_eq!(zones.z5_maximum.name, "Maximum");
}

#[test]
fn test_hr_zone_colors_defined() {
    assert_eq!(HR_ZONE_COLORS.len(), 5);

    // Verify colors are defined
    for color in &HR_ZONE_COLORS {
        // At least one component should be non-zero
        assert!(color.r > 0 || color.g > 0 || color.b > 0);
    }
}

#[test]
fn test_hr_zone_range_lookup() {
    let zones = HRZones::from_hr(180, 60);

    assert!(zones.get_zone_range(0).is_none());
    assert!(zones.get_zone_range(6).is_none());

    assert_eq!(zones.get_zone_range(1).unwrap().name, "Recovery");
    assert_eq!(zones.get_zone_range(3).unwrap().name, "Tempo");
    assert_eq!(zones.get_zone_range(5).unwrap().name, "Maximum");
}

#[test]
fn test_hr_zones_all_zones() {
    let zones = HRZones::from_hr(180, 60);
    let all = zones.all_zones();

    assert_eq!(all.len(), 5);
    assert_eq!(all[0].zone, 1);
    assert_eq!(all[4].zone, 5);
}

#[test]
fn test_zones_not_custom_by_default() {
    let power_zones = PowerZones::from_ftp(200);
    let hr_zones = HRZones::from_hr(180, 60);

    assert!(!power_zones.custom);
    assert!(!hr_zones.custom);
}
