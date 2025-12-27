//! ANT+ FE-C (Fitness Equipment Control) Profile
//!
//! Implements the ANT+ FE-C device profile for smart trainers.
//! Device Type: 17

use super::AntProfilePage;
use crate::sensors::ant::AntDeviceType;

/// General FE data page (Page 16 / 0x10)
#[derive(Debug, Clone)]
pub struct GeneralFePage {
    /// Equipment type
    pub equipment_type: EquipmentType,
    /// Elapsed time in 0.25 second units
    pub elapsed_time: u8,
    /// Distance traveled in meters
    pub distance: u8,
    /// Speed in 0.001 m/s units (0-65.534 m/s)
    pub speed: u16,
    /// Heart rate if from equipment
    pub heart_rate: Option<u8>,
    /// FE state
    pub fe_state: FeState,
}

/// Equipment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentType {
    General,
    Treadmill,
    Elliptical,
    Rower,
    Climber,
    NordicSkier,
    Trainer,
    Unknown(u8),
}

impl From<u8> for EquipmentType {
    fn from(value: u8) -> Self {
        match value & 0x1F {
            0 => EquipmentType::General,
            19 => EquipmentType::Treadmill,
            20 => EquipmentType::Elliptical,
            22 => EquipmentType::Rower,
            23 => EquipmentType::Climber,
            24 => EquipmentType::NordicSkier,
            25 => EquipmentType::Trainer,
            x => EquipmentType::Unknown(x),
        }
    }
}

/// FE state bits
#[derive(Debug, Clone, Copy)]
pub struct FeState {
    pub lap_toggle: bool,
    pub fe_state: FeStateValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeStateValue {
    Asleep,
    Ready,
    InUse,
    Finished,
    Unknown(u8),
}

impl From<u8> for FeState {
    fn from(value: u8) -> Self {
        let state_bits = (value >> 4) & 0x07;
        Self {
            lap_toggle: (value & 0x80) != 0,
            fe_state: match state_bits {
                1 => FeStateValue::Asleep,
                2 => FeStateValue::Ready,
                3 => FeStateValue::InUse,
                4 => FeStateValue::Finished,
                x => FeStateValue::Unknown(x),
            },
        }
    }
}

impl AntProfilePage for GeneralFePage {
    fn page_number(&self) -> u8 {
        0x10
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x10 {
            return None;
        }

        let heart_rate = if data[6] == 0xFF { None } else { Some(data[6]) };

        Some(Self {
            equipment_type: data[1].into(),
            elapsed_time: data[2],
            distance: data[3],
            speed: u16::from_le_bytes([data[4], data[5]]),
            heart_rate,
            fe_state: data[7].into(),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::FitnessEquipment
    }
}

/// General settings page (Page 17 / 0x11)
#[derive(Debug, Clone)]
pub struct GeneralSettingsPage {
    /// Cycle length in 0.01 meter units
    pub cycle_length: u8,
    /// Incline in 0.01% units (-100% to +100%)
    pub incline: i16,
    /// Resistance level (0-200)
    pub resistance: u8,
}

impl AntProfilePage for GeneralSettingsPage {
    fn page_number(&self) -> u8 {
        0x11
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x11 {
            return None;
        }

        let incline = if data[3] == 0xFF && data[4] == 0x7F {
            0 // Invalid
        } else {
            i16::from_le_bytes([data[3], data[4]])
        };

        Some(Self {
            cycle_length: data[2],
            incline,
            resistance: data[5],
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::FitnessEquipment
    }
}

/// Trainer data page (Page 25 / 0x19)
#[derive(Debug, Clone)]
pub struct TrainerDataPage {
    /// Update event count
    pub event_count: u8,
    /// Instantaneous cadence
    pub cadence: Option<u8>,
    /// Accumulated power in watts
    pub accumulated_power: u16,
    /// Instantaneous power in watts (0-4096)
    pub instantaneous_power: u16,
    /// Trainer status flags
    pub trainer_status: TrainerStatus,
    /// Target power flags
    pub target_power_flags: TargetPowerFlags,
}

#[derive(Debug, Clone, Copy)]
pub struct TrainerStatus {
    pub power_calibration_required: bool,
    pub resistance_calibration_required: bool,
    pub user_config_required: bool,
}

impl From<u8> for TrainerStatus {
    fn from(value: u8) -> Self {
        Self {
            power_calibration_required: (value & 0x10) != 0,
            resistance_calibration_required: (value & 0x20) != 0,
            user_config_required: (value & 0x40) != 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TargetPowerFlags {
    pub target_power_limits: TargetPowerLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPowerLimit {
    AtTarget,
    TooLow,
    TooHigh,
    Undetermined,
}

impl From<u8> for TargetPowerFlags {
    fn from(value: u8) -> Self {
        let limit = match value & 0x03 {
            0 => TargetPowerLimit::AtTarget,
            1 => TargetPowerLimit::TooLow,
            2 => TargetPowerLimit::TooHigh,
            _ => TargetPowerLimit::Undetermined,
        };
        Self {
            target_power_limits: limit,
        }
    }
}

impl AntProfilePage for TrainerDataPage {
    fn page_number(&self) -> u8 {
        0x19
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x19 {
            return None;
        }

        let cadence = if data[2] == 0xFF { None } else { Some(data[2]) };

        // Power is 12 bits, status in upper 4 bits of byte 6
        let power_and_status = u16::from_le_bytes([data[5], data[6]]);
        let instantaneous_power = power_and_status & 0x0FFF;

        Some(Self {
            event_count: data[1],
            cadence,
            accumulated_power: u16::from_le_bytes([data[3], data[4]]),
            instantaneous_power,
            trainer_status: ((power_and_status >> 8) as u8).into(),
            target_power_flags: data[7].into(),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::FitnessEquipment
    }
}

/// FE-C control commands
pub mod commands {
    /// Basic resistance control (Page 48 / 0x30)
    pub fn set_basic_resistance(resistance_percent: f32) -> [u8; 8] {
        let resistance = ((resistance_percent / 0.5).round() as u8).min(200);
        [0x30, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, resistance]
    }

    /// Target power control (Page 49 / 0x31)
    pub fn set_target_power(power_watts: u16) -> [u8; 8] {
        let power_quarter_watts = power_watts.saturating_mul(4);
        let bytes = power_quarter_watts.to_le_bytes();
        [0x31, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, bytes[0], bytes[1]]
    }

    /// Wind resistance simulation (Page 50 / 0x32)
    pub fn set_wind_resistance(
        wind_resistance_coefficient: f32,
        wind_speed_kmh: i8,
        drafting_factor: f32,
    ) -> [u8; 8] {
        let wrc = ((wind_resistance_coefficient / 0.01).round() as u8).min(255);
        let df = ((drafting_factor / 0.01).round() as u8).min(255);

        [0x32, 0xFF, 0xFF, 0xFF, 0xFF, wrc, wind_speed_kmh as u8, df]
    }

    /// Track resistance (grade/slope) simulation (Page 51 / 0x33)
    pub fn set_track_resistance(grade_percent: f32, rolling_resistance: f32) -> [u8; 8] {
        // Grade: -200% to +200% with 0.01% resolution
        // Offset by 200% (0x4E20 = 20000 = 0%)
        let grade_encoded = ((grade_percent + 200.0) / 0.01).round() as u16;
        let grade_bytes = grade_encoded.to_le_bytes();

        let rr = ((rolling_resistance / 0.00005).round() as u8).min(255);

        [
            0x33,
            0xFF,
            0xFF,
            0xFF,
            0xFF,
            grade_bytes[0],
            grade_bytes[1],
            rr,
        ]
    }

    /// User configuration (Page 55 / 0x37)
    pub fn set_user_config(
        user_weight_kg: f32,
        bike_weight_kg: f32,
        wheel_diameter_mm: u16,
    ) -> [u8; 8] {
        let user_weight = (user_weight_kg / 0.01).round() as u16;
        let bike_weight = (bike_weight_kg / 0.05).round() as u8;
        let wheel_offset = 0u8; // Offset from 700mm

        let uw_bytes = user_weight.to_le_bytes();

        [
            0x37,
            uw_bytes[0],
            uw_bytes[1],
            0xFF, // Reserved
            bike_weight,
            wheel_offset,
            (wheel_diameter_mm & 0xFF) as u8,
            ((wheel_diameter_mm >> 8) & 0x0F) as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_fe_page_parse() {
        let data = [0x10, 25, 120, 50, 0x88, 0x13, 0xFF, 0x30];
        let page = GeneralFePage::parse(&data).unwrap();

        assert!(matches!(page.equipment_type, EquipmentType::Trainer));
        assert_eq!(page.elapsed_time, 120);
        assert_eq!(page.distance, 50);
        // Speed: 0x1388 = 5000 = 5.0 m/s = 18 km/h
        assert_eq!(page.speed, 5000);
        assert!(page.heart_rate.is_none());
    }

    #[test]
    fn test_trainer_data_page_parse() {
        let data = [0x19, 0x0A, 0x5A, 0xE8, 0x03, 0x64, 0x00, 0x00];
        let page = TrainerDataPage::parse(&data).unwrap();

        assert_eq!(page.event_count, 10);
        assert_eq!(page.cadence, Some(90));
        assert_eq!(page.accumulated_power, 1000);
        assert_eq!(page.instantaneous_power, 100);
    }

    #[test]
    fn test_set_target_power_command() {
        let cmd = commands::set_target_power(200);
        assert_eq!(cmd[0], 0x31);
        // 200W * 4 = 800 = 0x0320
        assert_eq!(cmd[6], 0x20);
        assert_eq!(cmd[7], 0x03);
    }

    #[test]
    fn test_set_track_resistance_command() {
        // 5% grade
        let cmd = commands::set_track_resistance(5.0, 0.004);
        assert_eq!(cmd[0], 0x33);
        // 5% + 200% offset = 205%, / 0.01 = 20500 = 0x5014
        let grade = u16::from_le_bytes([cmd[5], cmd[6]]);
        assert_eq!(grade, 20500);
    }

    #[test]
    fn test_set_basic_resistance_command() {
        let cmd = commands::set_basic_resistance(50.0);
        assert_eq!(cmd[0], 0x30);
        assert_eq!(cmd[7], 100); // 50% / 0.5 = 100
    }
}
