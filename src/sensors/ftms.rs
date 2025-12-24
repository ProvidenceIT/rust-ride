//! FTMS (Fitness Machine Service) protocol implementation.
//!
//! T036: Implement Indoor Bike Data (0x2AD2) notification parsing
//! T037: Implement FTMS service/characteristic UUID constants
//! T038: Extract power, cadence, speed from FTMS Indoor Bike Data

use uuid::Uuid;

/// FTMS Service UUID (0x1826)
pub const FTMS_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000_1826_0000_1000_8000_0080_5f9b_34fb);

/// Indoor Bike Data Characteristic UUID (0x2AD2)
pub const INDOOR_BIKE_DATA_UUID: Uuid = Uuid::from_u128(0x0000_2ad2_0000_1000_8000_0080_5f9b_34fb);

/// Fitness Machine Control Point UUID (0x2AD9)
pub const FTMS_CONTROL_POINT_UUID: Uuid =
    Uuid::from_u128(0x0000_2ad9_0000_1000_8000_0080_5f9b_34fb);

/// Fitness Machine Feature UUID (0x2ACC)
pub const FTMS_FEATURE_UUID: Uuid = Uuid::from_u128(0x0000_2acc_0000_1000_8000_0080_5f9b_34fb);

/// Training Status UUID (0x2AD3)
pub const TRAINING_STATUS_UUID: Uuid = Uuid::from_u128(0x0000_2ad3_0000_1000_8000_0080_5f9b_34fb);

/// Cycling Power Service UUID (0x1818)
pub const CYCLING_POWER_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x0000_1818_0000_1000_8000_0080_5f9b_34fb);

/// Cycling Power Measurement UUID (0x2A63)
pub const CYCLING_POWER_MEASUREMENT_UUID: Uuid =
    Uuid::from_u128(0x0000_2a63_0000_1000_8000_0080_5f9b_34fb);

/// Heart Rate Service UUID (0x180D)
pub const HEART_RATE_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);

/// Heart Rate Measurement UUID (0x2A37)
pub const HEART_RATE_MEASUREMENT_UUID: Uuid =
    Uuid::from_u128(0x0000_2a37_0000_1000_8000_0080_5f9b_34fb);

/// Cycling Speed and Cadence Service UUID (0x1816)
pub const CSC_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000_1816_0000_1000_8000_0080_5f9b_34fb);

/// Parsed data from Indoor Bike Data characteristic.
#[derive(Debug, Clone, Default)]
pub struct IndoorBikeData {
    /// Instantaneous speed in km/h (if present)
    pub speed_kmh: Option<f32>,
    /// Average speed in km/h (if present)
    pub avg_speed_kmh: Option<f32>,
    /// Instantaneous cadence in RPM (if present)
    pub cadence_rpm: Option<u16>,
    /// Average cadence in RPM (if present)
    pub avg_cadence_rpm: Option<u16>,
    /// Total distance in meters (if present)
    pub total_distance_m: Option<u32>,
    /// Resistance level (if present)
    pub resistance_level: Option<i16>,
    /// Instantaneous power in watts (if present)
    pub power_watts: Option<i16>,
    /// Average power in watts (if present)
    pub avg_power_watts: Option<i16>,
    /// Expended energy in kCal (if present)
    pub energy_kcal: Option<u16>,
    /// Heart rate in BPM (if present)
    pub heart_rate_bpm: Option<u8>,
    /// Metabolic equivalent (if present)
    pub metabolic_equivalent: Option<u8>,
    /// Elapsed time in seconds (if present)
    pub elapsed_time_s: Option<u16>,
    /// Remaining time in seconds (if present)
    pub remaining_time_s: Option<u16>,
}

/// Indoor Bike Data flags (first 2 bytes).
#[derive(Debug, Clone, Copy)]
struct IndoorBikeDataFlags {
    /// More data available (bit 0)
    more_data: bool,
    /// Average speed present (bit 1)
    avg_speed_present: bool,
    /// Instantaneous cadence present (bit 2)
    inst_cadence_present: bool,
    /// Average cadence present (bit 3)
    avg_cadence_present: bool,
    /// Total distance present (bit 4)
    total_distance_present: bool,
    /// Resistance level present (bit 5)
    resistance_level_present: bool,
    /// Instantaneous power present (bit 6)
    inst_power_present: bool,
    /// Average power present (bit 7)
    avg_power_present: bool,
    /// Expended energy present (bit 8)
    expended_energy_present: bool,
    /// Heart rate present (bit 9)
    heart_rate_present: bool,
    /// Metabolic equivalent present (bit 10)
    metabolic_equivalent_present: bool,
    /// Elapsed time present (bit 11)
    elapsed_time_present: bool,
    /// Remaining time present (bit 12)
    remaining_time_present: bool,
}

impl IndoorBikeDataFlags {
    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let flags = u16::from_le_bytes([data[0], data[1]]);

        Some(Self {
            more_data: (flags & 0x0001) != 0,
            avg_speed_present: (flags & 0x0002) != 0,
            inst_cadence_present: (flags & 0x0004) != 0,
            avg_cadence_present: (flags & 0x0008) != 0,
            total_distance_present: (flags & 0x0010) != 0,
            resistance_level_present: (flags & 0x0020) != 0,
            inst_power_present: (flags & 0x0040) != 0,
            avg_power_present: (flags & 0x0080) != 0,
            expended_energy_present: (flags & 0x0100) != 0,
            heart_rate_present: (flags & 0x0200) != 0,
            metabolic_equivalent_present: (flags & 0x0400) != 0,
            elapsed_time_present: (flags & 0x0800) != 0,
            remaining_time_present: (flags & 0x1000) != 0,
        })
    }
}

/// Parse Indoor Bike Data notification.
///
/// The data format is:
/// - Bytes 0-1: Flags (indicates which fields are present)
/// - Remaining bytes: Data fields in order based on flags
///
/// Note: If "More Data" flag (bit 0) is NOT set, instantaneous speed is present.
pub fn parse_indoor_bike_data(data: &[u8]) -> Option<IndoorBikeData> {
    let flags = IndoorBikeDataFlags::from_bytes(data)?;
    let mut result = IndoorBikeData::default();
    let mut offset = 2usize;

    // Instantaneous speed (present if More Data flag is 0)
    if !flags.more_data {
        if offset + 2 > data.len() {
            return None;
        }
        // Speed is in 0.01 km/h units
        let speed_raw = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.speed_kmh = Some(speed_raw as f32 / 100.0);
        offset += 2;
    }

    // Average speed
    if flags.avg_speed_present {
        if offset + 2 > data.len() {
            return None;
        }
        let speed_raw = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.avg_speed_kmh = Some(speed_raw as f32 / 100.0);
        offset += 2;
    }

    // Instantaneous cadence
    if flags.inst_cadence_present {
        if offset + 2 > data.len() {
            return None;
        }
        // Cadence is in 0.5 RPM units
        let cadence_raw = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.cadence_rpm = Some(cadence_raw / 2);
        offset += 2;
    }

    // Average cadence
    if flags.avg_cadence_present {
        if offset + 2 > data.len() {
            return None;
        }
        let cadence_raw = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.avg_cadence_rpm = Some(cadence_raw / 2);
        offset += 2;
    }

    // Total distance (3 bytes)
    if flags.total_distance_present {
        if offset + 3 > data.len() {
            return None;
        }
        let distance = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], 0]);
        result.total_distance_m = Some(distance);
        offset += 3;
    }

    // Resistance level
    if flags.resistance_level_present {
        if offset + 2 > data.len() {
            return None;
        }
        let resistance = i16::from_le_bytes([data[offset], data[offset + 1]]);
        result.resistance_level = Some(resistance);
        offset += 2;
    }

    // Instantaneous power
    if flags.inst_power_present {
        if offset + 2 > data.len() {
            return None;
        }
        let power = i16::from_le_bytes([data[offset], data[offset + 1]]);
        result.power_watts = Some(power);
        offset += 2;
    }

    // Average power
    if flags.avg_power_present {
        if offset + 2 > data.len() {
            return None;
        }
        let power = i16::from_le_bytes([data[offset], data[offset + 1]]);
        result.avg_power_watts = Some(power);
        offset += 2;
    }

    // Expended energy (3 fields: total, per hour, per minute)
    if flags.expended_energy_present {
        if offset + 5 > data.len() {
            return None;
        }
        let total_energy = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.energy_kcal = Some(total_energy);
        // Skip energy per hour (2 bytes) and energy per minute (1 byte)
        offset += 5;
    }

    // Heart rate
    if flags.heart_rate_present {
        if offset + 1 > data.len() {
            return None;
        }
        result.heart_rate_bpm = Some(data[offset]);
        offset += 1;
    }

    // Metabolic equivalent
    if flags.metabolic_equivalent_present {
        if offset + 1 > data.len() {
            return None;
        }
        result.metabolic_equivalent = Some(data[offset]);
        offset += 1;
    }

    // Elapsed time
    if flags.elapsed_time_present {
        if offset + 2 > data.len() {
            return None;
        }
        let time = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.elapsed_time_s = Some(time);
        offset += 2;
    }

    // Remaining time
    if flags.remaining_time_present {
        if offset + 2 > data.len() {
            return None;
        }
        let time = u16::from_le_bytes([data[offset], data[offset + 1]]);
        result.remaining_time_s = Some(time);
        // offset += 2; // Not needed, last field
    }

    Some(result)
}

/// FTMS Control Point opcodes.
#[repr(u8)]
pub enum FtmsControlOpcode {
    /// Request control of the fitness machine
    RequestControl = 0x00,
    /// Reset the fitness machine
    Reset = 0x01,
    /// Set target speed
    SetTargetSpeed = 0x02,
    /// Set target inclination
    SetTargetInclination = 0x03,
    /// Set target resistance level
    SetTargetResistanceLevel = 0x04,
    /// Set target power
    SetTargetPower = 0x05,
    /// Set target heart rate
    SetTargetHeartRate = 0x06,
    /// Start or resume training
    StartOrResume = 0x07,
    /// Stop or pause training
    StopOrPause = 0x08,
    /// Set indoor bike simulation parameters
    SetIndoorBikeSimulation = 0x11,
    /// Spin down control
    SpinDownControl = 0x13,
    /// Set targeted cadence
    SetTargetedCadence = 0x14,
}

/// Build a control point command to request control.
pub fn build_request_control() -> Vec<u8> {
    vec![FtmsControlOpcode::RequestControl as u8]
}

/// Build a control point command to start training.
pub fn build_start_training() -> Vec<u8> {
    vec![FtmsControlOpcode::StartOrResume as u8]
}

/// Build a control point command to stop training.
///
/// `pause` - true to pause, false to stop
pub fn build_stop_training(pause: bool) -> Vec<u8> {
    vec![
        FtmsControlOpcode::StopOrPause as u8,
        if pause { 0x02 } else { 0x01 },
    ]
}

/// Build a control point command to set target power (ERG mode).
///
/// `target_watts` - Target power in watts
pub fn build_set_target_power(target_watts: u16) -> Vec<u8> {
    let mut cmd = vec![FtmsControlOpcode::SetTargetPower as u8];
    cmd.extend_from_slice(&target_watts.to_le_bytes());
    cmd
}

/// Build a control point command to set target resistance level.
///
/// `level` - Resistance level (0.1 resolution, so 100 = 10.0)
pub fn build_set_target_resistance(level: i16) -> Vec<u8> {
    let mut cmd = vec![FtmsControlOpcode::SetTargetResistanceLevel as u8];
    cmd.extend_from_slice(&level.to_le_bytes());
    cmd
}

/// Build a control point command to set simulation parameters.
///
/// `wind_speed` - Wind speed in m/s (0.001 resolution)
/// `grade` - Grade in percent (0.01 resolution)
/// `crr` - Coefficient of rolling resistance (0.0001 resolution)
/// `cw` - Wind resistance coefficient (0.01 resolution)
pub fn build_set_simulation(wind_speed: i16, grade: i16, crr: u8, cw: u8) -> Vec<u8> {
    let mut cmd = vec![FtmsControlOpcode::SetIndoorBikeSimulation as u8];
    cmd.extend_from_slice(&wind_speed.to_le_bytes());
    cmd.extend_from_slice(&grade.to_le_bytes());
    cmd.push(crr);
    cmd.push(cw);
    cmd
}

/// Parse Cycling Power Measurement data.
#[derive(Debug, Clone, Default)]
pub struct CyclingPowerData {
    /// Instantaneous power in watts
    pub power_watts: i16,
    /// Pedal power balance (if present)
    pub power_balance: Option<u8>,
    /// Accumulated torque (if present)
    pub torque: Option<u16>,
    /// Crank revolution data (if present)
    pub crank_revolutions: Option<u16>,
    /// Last crank event time (if present)
    pub last_crank_event_time: Option<u16>,
}

/// Parse Cycling Power Measurement notification.
pub fn parse_cycling_power_measurement(data: &[u8]) -> Option<CyclingPowerData> {
    if data.len() < 4 {
        return None;
    }

    let flags = u16::from_le_bytes([data[0], data[1]]);
    let power = i16::from_le_bytes([data[2], data[3]]);

    let mut result = CyclingPowerData {
        power_watts: power,
        ..Default::default()
    };

    let mut offset = 4usize;

    // Pedal Power Balance (bit 0)
    if (flags & 0x0001) != 0 {
        if offset + 1 > data.len() {
            return Some(result);
        }
        result.power_balance = Some(data[offset]);
        offset += 1;
    }

    // Accumulated Torque (bit 2)
    if (flags & 0x0004) != 0 {
        if offset + 2 > data.len() {
            return Some(result);
        }
        result.torque = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
        offset += 2;
    }

    // Crank Revolution Data (bit 5)
    if (flags & 0x0020) != 0 {
        if offset + 4 > data.len() {
            return Some(result);
        }
        result.crank_revolutions = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
        result.last_crank_event_time =
            Some(u16::from_le_bytes([data[offset + 2], data[offset + 3]]));
        // offset += 4;
    }

    Some(result)
}

/// Parse Heart Rate Measurement notification.
#[derive(Debug, Clone, Default)]
pub struct HeartRateData {
    /// Heart rate in BPM
    pub heart_rate_bpm: u16,
    /// RR intervals (if present)
    pub rr_intervals: Vec<u16>,
    /// Energy expended in kJ (if present)
    pub energy_expended: Option<u16>,
    /// Sensor contact detected
    pub sensor_contact: bool,
}

/// Parse Heart Rate Measurement notification.
pub fn parse_heart_rate_measurement(data: &[u8]) -> Option<HeartRateData> {
    if data.is_empty() {
        return None;
    }

    let flags = data[0];
    let hr_format_u16 = (flags & 0x01) != 0;
    let sensor_contact_supported = (flags & 0x04) != 0;
    let sensor_contact = sensor_contact_supported && ((flags & 0x02) != 0);
    let energy_expended_present = (flags & 0x08) != 0;
    let rr_interval_present = (flags & 0x10) != 0;

    let mut offset = 1usize;

    // Heart rate value
    let heart_rate_bpm = if hr_format_u16 {
        if offset + 2 > data.len() {
            return None;
        }
        let hr = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        hr
    } else {
        if offset + 1 > data.len() {
            return None;
        }
        let hr = data[offset] as u16;
        offset += 1;
        hr
    };

    let mut result = HeartRateData {
        heart_rate_bpm,
        sensor_contact,
        ..Default::default()
    };

    // Energy expended
    if energy_expended_present && offset + 2 <= data.len() {
        result.energy_expended = Some(u16::from_le_bytes([data[offset], data[offset + 1]]));
        offset += 2;
    }

    // RR intervals
    if rr_interval_present {
        while offset + 2 <= data.len() {
            let rr = u16::from_le_bytes([data[offset], data[offset + 1]]);
            result.rr_intervals.push(rr);
            offset += 2;
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_indoor_bike_data_speed_only() {
        // Flags: 0x0000 (only instantaneous speed, no "more data")
        // Speed: 2500 = 25.00 km/h
        let data = [0x00, 0x00, 0xC4, 0x09];
        let result = parse_indoor_bike_data(&data).unwrap();

        assert!((result.speed_kmh.unwrap() - 25.0).abs() < 0.01);
        assert!(result.power_watts.is_none());
        assert!(result.cadence_rpm.is_none());
    }

    #[test]
    fn test_parse_indoor_bike_data_with_power_and_cadence() {
        // Flags: 0x0044 (instantaneous cadence + instantaneous power)
        // Speed: 3000 = 30.00 km/h
        // Cadence: 180 = 90 RPM (0.5 resolution)
        // Power: 250W
        let data = [0x44, 0x00, 0xB8, 0x0B, 0xB4, 0x00, 0xFA, 0x00];
        let result = parse_indoor_bike_data(&data).unwrap();

        assert!((result.speed_kmh.unwrap() - 30.0).abs() < 0.01);
        assert_eq!(result.cadence_rpm.unwrap(), 90);
        assert_eq!(result.power_watts.unwrap(), 250);
    }

    #[test]
    fn test_parse_cycling_power_measurement() {
        // Flags: 0x0000 (no optional fields)
        // Power: 200W
        let data = [0x00, 0x00, 0xC8, 0x00];
        let result = parse_cycling_power_measurement(&data).unwrap();

        assert_eq!(result.power_watts, 200);
    }

    #[test]
    fn test_parse_heart_rate_measurement_u8() {
        // Flags: 0x00 (8-bit HR)
        // HR: 145 BPM
        let data = [0x00, 0x91];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate_bpm, 145);
    }

    #[test]
    fn test_parse_heart_rate_measurement_u16() {
        // Flags: 0x01 (16-bit HR)
        // HR: 145 BPM
        let data = [0x01, 0x91, 0x00];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate_bpm, 145);
    }

    #[test]
    fn test_build_set_target_power() {
        let cmd = build_set_target_power(250);
        assert_eq!(cmd, vec![0x05, 0xFA, 0x00]);
    }

    #[test]
    fn test_build_request_control() {
        let cmd = build_request_control();
        assert_eq!(cmd, vec![0x00]);
    }

    #[test]
    fn test_build_start_training() {
        let cmd = build_start_training();
        assert_eq!(cmd, vec![0x07]);
    }
}
