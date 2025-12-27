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
#[serde(default)]
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
        // Helper to calculate watts from percentage, using round() to avoid
        // floating-point truncation issues (e.g., 209.9999 -> 210, not 209)
        let calc_watts = |percent: f32| -> u16 { (ftp as f32 * percent).round() as u16 };

        Self {
            z1_recovery: ZoneRange {
                zone: 1,
                min_percent: 0,
                max_percent: 55,
                min_watts: 0,
                max_watts: calc_watts(0.55),
                color: POWER_ZONE_COLORS[0],
                name: "Active Recovery".to_string(),
            },
            z2_endurance: ZoneRange {
                zone: 2,
                min_percent: 56,
                max_percent: 75,
                min_watts: calc_watts(0.56),
                max_watts: calc_watts(0.75),
                color: POWER_ZONE_COLORS[1],
                name: "Endurance".to_string(),
            },
            z3_tempo: ZoneRange {
                zone: 3,
                min_percent: 76,
                max_percent: 90,
                min_watts: calc_watts(0.76),
                max_watts: calc_watts(0.90),
                color: POWER_ZONE_COLORS[2],
                name: "Tempo".to_string(),
            },
            z4_threshold: ZoneRange {
                zone: 4,
                min_percent: 91,
                max_percent: 105,
                min_watts: calc_watts(0.91),
                max_watts: calc_watts(1.05),
                color: POWER_ZONE_COLORS[3],
                name: "Threshold".to_string(),
            },
            z5_vo2max: ZoneRange {
                zone: 5,
                min_percent: 106,
                max_percent: 120,
                min_watts: calc_watts(1.06),
                max_watts: calc_watts(1.20),
                color: POWER_ZONE_COLORS[4],
                name: "VO2max".to_string(),
            },
            z6_anaerobic: ZoneRange {
                zone: 6,
                min_percent: 121,
                max_percent: 150,
                min_watts: calc_watts(1.21),
                max_watts: calc_watts(1.50),
                color: POWER_ZONE_COLORS[5],
                name: "Anaerobic".to_string(),
            },
            z7_neuromuscular: ZoneRange {
                zone: 7,
                min_percent: 151,
                max_percent: 255,
                min_watts: calc_watts(1.51),
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

    /// Update zones based on a new FTP value.
    ///
    /// T075: Trigger zone recalculation on FTP acceptance.
    /// Returns the new zones calculated from the updated FTP.
    pub fn update_from_ftp(&mut self, new_ftp: u16) {
        *self = Self::from_ftp(new_ftp);
    }
}

impl Default for PowerZones {
    fn default() -> Self {
        // Default to 200W FTP as a reasonable starting point
        Self::from_ftp(200)
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

/// Events emitted when zones change.
#[derive(Debug, Clone)]
pub enum ZoneEvent {
    /// Power zone changed
    PowerZoneChange {
        /// Previous zone (0 if first reading)
        previous_zone: u8,
        /// New zone number (1-7)
        new_zone: u8,
        /// Zone name
        zone_name: String,
    },
    /// Heart rate zone changed
    HeartRateZoneChange {
        /// Previous zone (0 if first reading)
        previous_zone: u8,
        /// New zone number (1-5)
        new_zone: u8,
        /// Zone name
        zone_name: String,
    },
}

/// Tracks power and HR zones and detects zone changes.
///
/// T063: Zone change detection for audio alerts.
pub struct ZoneTracker {
    /// Current power zone (1-7, or 0 if not yet set)
    current_power_zone: u8,
    /// Current HR zone (1-5, or 0 if not yet set)
    current_hr_zone: u8,
    /// Power zones configuration
    power_zones: PowerZones,
    /// HR zones configuration
    hr_zones: Option<HRZones>,
    /// Pending events to be consumed
    pending_events: Vec<ZoneEvent>,
    /// Minimum time between zone change alerts (debounce)
    zone_change_debounce_secs: u32,
    /// Last power zone change time
    last_power_zone_change: Option<std::time::Instant>,
    /// Last HR zone change time
    last_hr_zone_change: Option<std::time::Instant>,
}

impl ZoneTracker {
    /// Create a new zone tracker with the given power zones.
    pub fn new(power_zones: PowerZones) -> Self {
        Self {
            current_power_zone: 0,
            current_hr_zone: 0,
            power_zones,
            hr_zones: None,
            pending_events: Vec::new(),
            zone_change_debounce_secs: 5,
            last_power_zone_change: None,
            last_hr_zone_change: None,
        }
    }

    /// Set HR zones for heart rate zone tracking.
    pub fn set_hr_zones(&mut self, hr_zones: HRZones) {
        self.hr_zones = Some(hr_zones);
    }

    /// Update power zones (e.g., when FTP changes).
    pub fn set_power_zones(&mut self, power_zones: PowerZones) {
        self.power_zones = power_zones;
    }

    /// Set the debounce time for zone change alerts.
    pub fn set_debounce_secs(&mut self, secs: u32) {
        self.zone_change_debounce_secs = secs;
    }

    /// Update with current power reading and check for zone change.
    pub fn update_power(&mut self, power: u16) {
        let new_zone = self.power_zones.get_zone(power);

        if new_zone != self.current_power_zone {
            // Check debounce
            let should_emit = self
                .last_power_zone_change
                .map(|t| t.elapsed().as_secs() >= self.zone_change_debounce_secs as u64)
                .unwrap_or(true);

            if should_emit || self.current_power_zone == 0 {
                let previous_zone = self.current_power_zone;
                self.current_power_zone = new_zone;
                self.last_power_zone_change = Some(std::time::Instant::now());

                // Get zone name
                let zone_name = self
                    .power_zones
                    .get_zone_range(new_zone)
                    .map(|z| z.name.clone())
                    .unwrap_or_else(|| format!("Zone {}", new_zone));

                // Only emit event if not initial (previous zone was set)
                if previous_zone > 0 {
                    self.pending_events.push(ZoneEvent::PowerZoneChange {
                        previous_zone,
                        new_zone,
                        zone_name,
                    });
                    tracing::debug!(
                        "Power zone changed: {} -> {} ({})",
                        previous_zone,
                        new_zone,
                        self.power_zones
                            .get_zone_range(new_zone)
                            .map(|z| z.name.as_str())
                            .unwrap_or("Unknown")
                    );
                }
            }
        }
    }

    /// Update with current heart rate and check for zone change.
    pub fn update_heart_rate(&mut self, hr: u8) {
        let hr_zones = match &self.hr_zones {
            Some(zones) => zones,
            None => return,
        };

        let new_zone = hr_zones.get_zone(hr);

        if new_zone != self.current_hr_zone && new_zone > 0 {
            // Check debounce
            let should_emit = self
                .last_hr_zone_change
                .map(|t| t.elapsed().as_secs() >= self.zone_change_debounce_secs as u64)
                .unwrap_or(true);

            if should_emit || self.current_hr_zone == 0 {
                let previous_zone = self.current_hr_zone;
                self.current_hr_zone = new_zone;
                self.last_hr_zone_change = Some(std::time::Instant::now());

                // Get zone name
                let zone_name = hr_zones
                    .get_zone_range(new_zone)
                    .map(|z| z.name.clone())
                    .unwrap_or_else(|| format!("Zone {}", new_zone));

                // Only emit event if not initial
                if previous_zone > 0 {
                    self.pending_events.push(ZoneEvent::HeartRateZoneChange {
                        previous_zone,
                        new_zone,
                        zone_name,
                    });
                    tracing::debug!(
                        "HR zone changed: {} -> {} ({})",
                        previous_zone,
                        new_zone,
                        hr_zones
                            .get_zone_range(new_zone)
                            .map(|z| z.name.as_str())
                            .unwrap_or("Unknown")
                    );
                }
            }
        }
    }

    /// Take all pending events, clearing the queue.
    pub fn take_events(&mut self) -> Vec<ZoneEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Check if there are pending events.
    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    /// Get the current power zone.
    pub fn current_power_zone(&self) -> u8 {
        self.current_power_zone
    }

    /// Get the current HR zone.
    pub fn current_hr_zone(&self) -> u8 {
        self.current_hr_zone
    }

    /// Get the name of the current power zone.
    pub fn current_power_zone_name(&self) -> Option<String> {
        self.power_zones
            .get_zone_range(self.current_power_zone)
            .map(|z| z.name.clone())
    }

    /// Get the name of the current HR zone.
    pub fn current_hr_zone_name(&self) -> Option<String> {
        self.hr_zones
            .as_ref()
            .and_then(|zones| zones.get_zone_range(self.current_hr_zone))
            .map(|z| z.name.clone())
    }

    /// Reset tracking state.
    pub fn reset(&mut self) {
        self.current_power_zone = 0;
        self.current_hr_zone = 0;
        self.pending_events.clear();
        self.last_power_zone_change = None;
        self.last_hr_zone_change = None;
    }
}

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

        assert_eq!(zones.get_zone(50), 1); // Recovery
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

    #[test]
    fn test_zone_tracker_power_zone_change() {
        let power_zones = PowerZones::from_ftp(200);
        let mut tracker = ZoneTracker::new(power_zones);
        tracker.set_debounce_secs(0); // Disable debounce for testing

        // First update sets initial zone (no event emitted)
        tracker.update_power(100); // Zone 1
        assert_eq!(tracker.current_power_zone(), 1);
        assert!(!tracker.has_pending_events());

        // Zone change should emit event
        tracker.update_power(200); // Zone 4 (Threshold)
        assert_eq!(tracker.current_power_zone(), 4);
        assert!(tracker.has_pending_events());

        let events = tracker.take_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ZoneEvent::PowerZoneChange {
                previous_zone,
                new_zone,
                zone_name,
            } => {
                assert_eq!(*previous_zone, 1);
                assert_eq!(*new_zone, 4);
                assert_eq!(zone_name, "Threshold");
            }
            _ => panic!("Expected PowerZoneChange event"),
        }
    }

    #[test]
    fn test_zone_tracker_hr_zone_change() {
        let power_zones = PowerZones::from_ftp(200);
        let hr_zones = HRZones::from_hr(180, 60);
        let mut tracker = ZoneTracker::new(power_zones);
        tracker.set_hr_zones(hr_zones);
        tracker.set_debounce_secs(0);

        // Initial HR reading
        tracker.update_heart_rate(125); // Zone 1
        assert_eq!(tracker.current_hr_zone(), 1);
        assert!(!tracker.has_pending_events());

        // Zone change
        tracker.update_heart_rate(170); // Zone 5
        assert_eq!(tracker.current_hr_zone(), 5);
        assert!(tracker.has_pending_events());

        let events = tracker.take_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ZoneEvent::HeartRateZoneChange {
                previous_zone,
                new_zone,
                zone_name,
            } => {
                assert_eq!(*previous_zone, 1);
                assert_eq!(*new_zone, 5);
                assert_eq!(zone_name, "Maximum");
            }
            _ => panic!("Expected HeartRateZoneChange event"),
        }
    }

    #[test]
    fn test_zone_tracker_no_event_for_same_zone() {
        let power_zones = PowerZones::from_ftp(200);
        let mut tracker = ZoneTracker::new(power_zones);
        tracker.set_debounce_secs(0);

        tracker.update_power(100); // Zone 1
        tracker.update_power(80); // Still Zone 1
        tracker.update_power(90); // Still Zone 1

        // No events should be emitted for staying in the same zone
        assert!(!tracker.has_pending_events());
        assert_eq!(tracker.current_power_zone(), 1);
    }

    #[test]
    fn test_zone_tracker_reset() {
        let power_zones = PowerZones::from_ftp(200);
        let mut tracker = ZoneTracker::new(power_zones);
        tracker.set_debounce_secs(0);

        tracker.update_power(200);
        tracker.update_power(300); // Zone change

        assert!(tracker.has_pending_events());
        assert!(tracker.current_power_zone() > 0);

        tracker.reset();

        assert!(!tracker.has_pending_events());
        assert_eq!(tracker.current_power_zone(), 0);
        assert_eq!(tracker.current_hr_zone(), 0);
    }
}
