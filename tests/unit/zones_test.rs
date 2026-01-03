//! Unit tests for zone calculations.
//!
//! T104: Unit test for zone determination from power/HR
//! T113: Unit test for zone calculation from FTP
//! T114: Unit test for HR zone calculation

use rustride::metrics::zones::{
    CadenceZones, HRZones, PowerZones, CADENCE_ZONE_COLORS, HR_ZONE_COLORS, POWER_ZONE_COLORS,
};

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
    for (i, c1) in POWER_ZONE_COLORS.iter().enumerate().take(7) {
        for (j, c2) in POWER_ZONE_COLORS
            .iter()
            .enumerate()
            .skip(i + 1)
            .take(7 - i - 1)
        {
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
    let cadence_zones = CadenceZones::default();

    assert!(!power_zones.custom);
    assert!(!hr_zones.custom);
    assert!(!cadence_zones.custom);
}

// ========== Cadence Zone Tests ==========

#[test]
fn test_cadence_zones_default() {
    let zones = CadenceZones::default();

    // Z1: Low (0-75 RPM)
    assert_eq!(zones.z1_low.min_rpm, 0);
    assert_eq!(zones.z1_low.max_rpm, 75);
    assert_eq!(zones.z1_low.zone, 1);

    // Z2: Economy (76-85 RPM)
    assert_eq!(zones.z2_economy.min_rpm, 76);
    assert_eq!(zones.z2_economy.max_rpm, 85);
    assert_eq!(zones.z2_economy.zone, 2);

    // Z3: Natural (86-95 RPM)
    assert_eq!(zones.z3_natural.min_rpm, 86);
    assert_eq!(zones.z3_natural.max_rpm, 95);
    assert_eq!(zones.z3_natural.zone, 3);

    // Z4: Fast (96-105 RPM)
    assert_eq!(zones.z4_fast.min_rpm, 96);
    assert_eq!(zones.z4_fast.max_rpm, 105);
    assert_eq!(zones.z4_fast.zone, 4);

    // Z5: Sprint (106+ RPM)
    assert_eq!(zones.z5_sprint.min_rpm, 106);
    assert_eq!(zones.z5_sprint.max_rpm, 255);
    assert_eq!(zones.z5_sprint.zone, 5);
}

#[test]
fn test_cadence_zone_lookup() {
    let zones = CadenceZones::default();

    // Test zone 1 boundaries (Low: 0-75 RPM)
    assert_eq!(zones.get_zone(0), 1);
    assert_eq!(zones.get_zone(40), 1);
    assert_eq!(zones.get_zone(75), 1);

    // Test zone 2 boundaries (Economy: 76-85 RPM)
    assert_eq!(zones.get_zone(76), 2);
    assert_eq!(zones.get_zone(80), 2);
    assert_eq!(zones.get_zone(85), 2);

    // Test zone 3 boundaries (Natural: 86-95 RPM)
    assert_eq!(zones.get_zone(86), 3);
    assert_eq!(zones.get_zone(90), 3); // Optimal cadence for most cyclists
    assert_eq!(zones.get_zone(95), 3);

    // Test zone 4 boundaries (Fast: 96-105 RPM)
    assert_eq!(zones.get_zone(96), 4);
    assert_eq!(zones.get_zone(100), 4);
    assert_eq!(zones.get_zone(105), 4);

    // Test zone 5 boundaries (Sprint: 106+ RPM)
    assert_eq!(zones.get_zone(106), 5);
    assert_eq!(zones.get_zone(120), 5);
    assert_eq!(zones.get_zone(200), 5);
    assert_eq!(zones.get_zone(255), 5);
}

#[test]
fn test_cadence_zone_names() {
    let zones = CadenceZones::default();

    assert_eq!(zones.z1_low.name, "Low");
    assert_eq!(zones.z2_economy.name, "Economy");
    assert_eq!(zones.z3_natural.name, "Natural");
    assert_eq!(zones.z4_fast.name, "Fast");
    assert_eq!(zones.z5_sprint.name, "Sprint");
}

#[test]
fn test_cadence_zone_colors_defined() {
    assert_eq!(CADENCE_ZONE_COLORS.len(), 5);

    // Verify all colors are different
    for (i, c1) in CADENCE_ZONE_COLORS.iter().enumerate().take(5) {
        for (j, c2) in CADENCE_ZONE_COLORS
            .iter()
            .enumerate()
            .skip(i + 1)
            .take(5 - i - 1)
        {
            assert!(
                c1.r != c2.r || c1.g != c2.g || c1.b != c2.b,
                "Zone {} and {} should have different colors",
                i + 1,
                j + 1
            );
        }
    }

    // Verify zones use correct colors
    let zones = CadenceZones::default();
    assert_eq!(zones.z1_low.color, CADENCE_ZONE_COLORS[0]);
    assert_eq!(zones.z2_economy.color, CADENCE_ZONE_COLORS[1]);
    assert_eq!(zones.z3_natural.color, CADENCE_ZONE_COLORS[2]);
    assert_eq!(zones.z4_fast.color, CADENCE_ZONE_COLORS[3]);
    assert_eq!(zones.z5_sprint.color, CADENCE_ZONE_COLORS[4]);
}

#[test]
fn test_cadence_zone_range_lookup() {
    let zones = CadenceZones::default();

    assert!(zones.get_zone_range(0).is_none());
    assert!(zones.get_zone_range(6).is_none());

    assert_eq!(zones.get_zone_range(1).unwrap().name, "Low");
    assert_eq!(zones.get_zone_range(2).unwrap().name, "Economy");
    assert_eq!(zones.get_zone_range(3).unwrap().name, "Natural");
    assert_eq!(zones.get_zone_range(4).unwrap().name, "Fast");
    assert_eq!(zones.get_zone_range(5).unwrap().name, "Sprint");
}

#[test]
fn test_cadence_zones_all_zones() {
    let zones = CadenceZones::default();
    let all = zones.all_zones();

    assert_eq!(all.len(), 5);
    assert_eq!(all[0].zone, 1);
    assert_eq!(all[4].zone, 5);

    // Verify zone names in order
    assert_eq!(all[0].name, "Low");
    assert_eq!(all[1].name, "Economy");
    assert_eq!(all[2].name, "Natural");
    assert_eq!(all[3].name, "Fast");
    assert_eq!(all[4].name, "Sprint");
}
