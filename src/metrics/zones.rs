//! Power and heart rate zone calculations.
//!
//! T015: Define PowerZones, HRZones, ZoneRange structs
//! T017: Implement Coggan 7-zone power zone calculation from FTP
//! T018: Implement Karvonen HR zone calculation
//! T107: Add zone colors

use serde::{Deserialize, Serialize};

/// RGB color representation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to egui color format.
    pub fn to_egui(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

/// A power zone range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneRange {
    /// Zone number (1-7 for power, 1-5 for HR)
    pub zone: u8,
    /// Minimum percentage of FTP
    pub min_percent: u8,
    /// Maximum percentage of FTP (255 = no upper limit)
    pub max_percent: u8,
    /// Minimum watts for this zone
    pub min_watts: u16,
    /// Maximum watts for this zone
    pub max_watts: u16,
    /// Display color
    pub color: Color,
    /// Zone name
    pub name: String,
}

/// A heart rate zone range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HRZoneRange {
    /// Zone number (1-5)
    pub zone: u8,
    /// Minimum BPM
    pub min_bpm: u8,
    /// Maximum BPM
    pub max_bpm: u8,
    /// Display color
    pub color: Color,
    /// Zone name
    pub name: String,
}

/// Coggan 7-zone power zones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZones {
    /// Zone 1: Active Recovery (0-55% FTP)
    pub z1_recovery: ZoneRange,
    /// Zone 2: Endurance (56-75% FTP)
    pub z2_endurance: ZoneRange,
    /// Zone 3: Tempo (76-90% FTP)
    pub z3_tempo: ZoneRange,
    /// Zone 4: Threshold (91-105% FTP)
    pub z4_threshold: ZoneRange,
    /// Zone 5: VO2max (106-120% FTP)
    pub z5_vo2max: ZoneRange,
    /// Zone 6: Anaerobic (121-150% FTP)
    pub z6_anaerobic: ZoneRange,
    /// Zone 7: Neuromuscular (>150% FTP)
    pub z7_neuromuscular: ZoneRange,
    /// Whether zones are user-customized
    pub custom: bool,
}

impl PowerZones {
    /// Calculate power zones from FTP using Coggan 7-zone model.
    pub fn from_ftp(ftp: u16) -> Self {
        Self {
            z1_recovery: ZoneRange {
                zone: 1,
                min_percent: 0,
                max_percent: 55,
                min_watts: 0,
                max_watts: (ftp as f32 * 0.55) as u16,
                color: POWER_ZONE_COLORS[0],
                name: "Active Recovery".to_string(),
            },
            z2_endurance: ZoneRange {
                zone: 2,
                min_percent: 56,
                max_percent: 75,
                min_watts: (ftp as f32 * 0.56) as u16,
                max_watts: (ftp as f32 * 0.75) as u16,
                color: POWER_ZONE_COLORS[1],
                name: "Endurance".to_string(),
            },
            z3_tempo: ZoneRange {
                zone: 3,
                min_percent: 76,
                max_percent: 90,
                min_watts: (ftp as f32 * 0.76) as u16,
                max_watts: (ftp as f32 * 0.90) as u16,
                color: POWER_ZONE_COLORS[2],
                name: "Tempo".to_string(),
            },
            z4_threshold: ZoneRange {
                zone: 4,
                min_percent: 91,
                max_percent: 105,
                min_watts: (ftp as f32 * 0.91) as u16,
                max_watts: (ftp as f32 * 1.05) as u16,
                color: POWER_ZONE_COLORS[3],
                name: "Threshold".to_string(),
            },
            z5_vo2max: ZoneRange {
                zone: 5,
                min_percent: 106,
                max_percent: 120,
                min_watts: (ftp as f32 * 1.06) as u16,
                max_watts: (ftp as f32 * 1.20) as u16,
                color: POWER_ZONE_COLORS[4],
                name: "VO2max".to_string(),
            },
            z6_anaerobic: ZoneRange {
                zone: 6,
                min_percent: 121,
                max_percent: 150,
                min_watts: (ftp as f32 * 1.21) as u16,
                max_watts: (ftp as f32 * 1.50) as u16,
                color: POWER_ZONE_COLORS[5],
                name: "Anaerobic".to_string(),
            },
            z7_neuromuscular: ZoneRange {
                zone: 7,
                min_percent: 151,
                max_percent: 255,
                min_watts: (ftp as f32 * 1.51) as u16,
                max_watts: u16::MAX,
                color: POWER_ZONE_COLORS[6],
                name: "Neuromuscular".to_string(),
            },
            custom: false,
        }
    }

    /// Get the zone for a given power value.
    pub fn get_zone(&self, power: u16) -> u8 {
        if power <= self.z1_recovery.max_watts {
            1
        } else if power <= self.z2_endurance.max_watts {
            2
        } else if power <= self.z3_tempo.max_watts {
            3
        } else if power <= self.z4_threshold.max_watts {
            4
        } else if power <= self.z5_vo2max.max_watts {
            5
        } else if power <= self.z6_anaerobic.max_watts {
            6
        } else {
            7
        }
    }

    /// Get the zone range for a given zone number (1-7).
    pub fn get_zone_range(&self, zone: u8) -> Option<&ZoneRange> {
        match zone {
            1 => Some(&self.z1_recovery),
            2 => Some(&self.z2_endurance),
            3 => Some(&self.z3_tempo),
            4 => Some(&self.z4_threshold),
            5 => Some(&self.z5_vo2max),
            6 => Some(&self.z6_anaerobic),
            7 => Some(&self.z7_neuromuscular),
            _ => None,
        }
    }

    /// Get all zones as a vector.
    pub fn all_zones(&self) -> Vec<&ZoneRange> {
        vec![
            &self.z1_recovery,
            &self.z2_endurance,
            &self.z3_tempo,
            &self.z4_threshold,
            &self.z5_vo2max,
            &self.z6_anaerobic,
            &self.z7_neuromuscular,
        ]
    }
}

/// Karvonen 5-zone heart rate zones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HRZones {
    /// Zone 1: Recovery (50-60% HRR)
    pub z1_recovery: HRZoneRange,
    /// Zone 2: Aerobic (60-70% HRR)
    pub z2_aerobic: HRZoneRange,
    /// Zone 3: Tempo (70-80% HRR)
    pub z3_tempo: HRZoneRange,
    /// Zone 4: Threshold (80-90% HRR)
    pub z4_threshold: HRZoneRange,
    /// Zone 5: Maximum (90-100% HRR)
    pub z5_maximum: HRZoneRange,
    /// Whether zones are user-customized
    pub custom: bool,
}

impl HRZones {
    /// Calculate heart rate zones using Karvonen formula.
    ///
    /// Karvonen formula: Target HR = ((max_hr - resting_hr) × %intensity) + resting_hr
    /// This uses Heart Rate Reserve (HRR) = max_hr - resting_hr
    pub fn from_hr(max_hr: u8, resting_hr: u8) -> Self {
        let hrr = max_hr.saturating_sub(resting_hr) as f32;

        let calc_hr = |percent: f32| -> u8 {
            let hr = (hrr * percent) + resting_hr as f32;
            hr.round().clamp(0.0, 255.0) as u8
        };

        Self {
            z1_recovery: HRZoneRange {
                zone: 1,
                min_bpm: calc_hr(0.50),
                max_bpm: calc_hr(0.60),
                color: HR_ZONE_COLORS[0],
                name: "Recovery".to_string(),
            },
            z2_aerobic: HRZoneRange {
                zone: 2,
                min_bpm: calc_hr(0.60),
                max_bpm: calc_hr(0.70),
                color: HR_ZONE_COLORS[1],
                name: "Aerobic".to_string(),
            },
            z3_tempo: HRZoneRange {
                zone: 3,
                min_bpm: calc_hr(0.70),
                max_bpm: calc_hr(0.80),
                color: HR_ZONE_COLORS[2],
                name: "Tempo".to_string(),
            },
            z4_threshold: HRZoneRange {
                zone: 4,
                min_bpm: calc_hr(0.80),
                max_bpm: calc_hr(0.90),
                color: HR_ZONE_COLORS[3],
                name: "Threshold".to_string(),
            },
            z5_maximum: HRZoneRange {
                zone: 5,
                min_bpm: calc_hr(0.90),
                max_bpm: max_hr,
                color: HR_ZONE_COLORS[4],
                name: "Maximum".to_string(),
            },
            custom: false,
        }
    }

    /// Get the zone for a given heart rate value.
    pub fn get_zone(&self, hr: u8) -> u8 {
        if hr < self.z1_recovery.min_bpm {
            0 // Below zone 1
        } else if hr <= self.z1_recovery.max_bpm {
            1
        } else if hr <= self.z2_aerobic.max_bpm {
            2
        } else if hr <= self.z3_tempo.max_bpm {
            3
        } else if hr <= self.z4_threshold.max_bpm {
            4
        } else {
            5
        }
    }

    /// Get the zone range for a given zone number (1-5).
    pub fn get_zone_range(&self, zone: u8) -> Option<&HRZoneRange> {
        match zone {
            1 => Some(&self.z1_recovery),
            2 => Some(&self.z2_aerobic),
            3 => Some(&self.z3_tempo),
            4 => Some(&self.z4_threshold),
            5 => Some(&self.z5_maximum),
            _ => None,
        }
    }

    /// Get all zones as a vector.
    pub fn all_zones(&self) -> Vec<&HRZoneRange> {
        vec![
            &self.z1_recovery,
            &self.z2_aerobic,
            &self.z3_tempo,
            &self.z4_threshold,
            &self.z5_maximum,
        ]
    }
}

/// Default power zone colors (Coggan standard)
pub const POWER_ZONE_COLORS: [Color; 7] = [
    Color::new(128, 128, 128), // Z1: Gray (Active Recovery)
    Color::new(0, 128, 255),   // Z2: Blue (Endurance)
    Color::new(0, 200, 100),   // Z3: Green (Tempo)
    Color::new(255, 200, 0),   // Z4: Yellow (Threshold)
    Color::new(255, 128, 0),   // Z5: Orange (VO2max)
    Color::new(255, 50, 50),   // Z6: Red (Anaerobic)
    Color::new(180, 0, 180),   // Z7: Purple (Neuromuscular)
];

/// Default heart rate zone colors
pub const HR_ZONE_COLORS: [Color; 5] = [
    Color::new(128, 128, 128), // Z1: Gray (Recovery)
    Color::new(0, 128, 255),   // Z2: Blue (Aerobic)
    Color::new(0, 200, 100),   // Z3: Green (Tempo)
    Color::new(255, 200, 0),   // Z4: Yellow (Threshold)
    Color::new(255, 50, 50),   // Z5: Red (Maximum)
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_zones_from_ftp() {
        let zones = PowerZones::from_ftp(200);

        // Z1: 0-55% = 0-110W
        assert_eq!(zones.z1_recovery.max_watts, 110);

        // Z2: 56-75% = 112-150W
        assert_eq!(zones.z2_endurance.min_watts, 112);
        assert_eq!(zones.z2_endurance.max_watts, 150);

        // Z3: 76-90% = 152-180W
        assert_eq!(zones.z3_tempo.min_watts, 152);
        assert_eq!(zones.z3_tempo.max_watts, 180);

        // Z4: 91-105% = 182-210W
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
    fn test_power_zone_lookup() {
        let zones = PowerZones::from_ftp(200);

        assert_eq!(zones.get_zone(50), 1);  // Recovery
        assert_eq!(zones.get_zone(130), 2); // Endurance
        assert_eq!(zones.get_zone(170), 3); // Tempo
        assert_eq!(zones.get_zone(200), 4); // Threshold (at FTP)
        assert_eq!(zones.get_zone(220), 5); // VO2max
        assert_eq!(zones.get_zone(280), 6); // Anaerobic
        assert_eq!(zones.get_zone(350), 7); // Neuromuscular
    }

    #[test]
    fn test_hr_zones_from_hr() {
        // Max HR 180, Resting HR 60 => HRR = 120
        let zones = HRZones::from_hr(180, 60);

        // Z1: 50-60% HRR = 60 + (120 * 0.5-0.6) = 120-132 bpm
        assert_eq!(zones.z1_recovery.min_bpm, 120);
        assert_eq!(zones.z1_recovery.max_bpm, 132);

        // Z2: 60-70% HRR = 60 + (120 * 0.6-0.7) = 132-144 bpm
        assert_eq!(zones.z2_aerobic.min_bpm, 132);
        assert_eq!(zones.z2_aerobic.max_bpm, 144);

        // Z5: 90-100% HRR = 60 + (120 * 0.9-1.0) = 168-180 bpm
        assert_eq!(zones.z5_maximum.min_bpm, 168);
        assert_eq!(zones.z5_maximum.max_bpm, 180);
    }

    #[test]
    fn test_hr_zone_lookup() {
        let zones = HRZones::from_hr(180, 60);

        assert_eq!(zones.get_zone(100), 0); // Below Z1
        assert_eq!(zones.get_zone(125), 1); // Recovery
        assert_eq!(zones.get_zone(140), 2); // Aerobic
        assert_eq!(zones.get_zone(150), 3); // Tempo
        assert_eq!(zones.get_zone(165), 4); // Threshold
        assert_eq!(zones.get_zone(175), 5); // Maximum
    }
}
