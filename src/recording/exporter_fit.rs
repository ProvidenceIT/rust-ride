//! FIT file export functionality for ride data.
//!
//! T050: Update FIT export to include cycling dynamics fields.
//!
//! Implements FIT (Flexible and Interoperable Data Transfer) binary format export
//! according to the ANT+ FIT SDK specification. Includes support for:
//! - File header and CRC
//! - Activity, session, lap, and record messages
//! - Cycling dynamics (L/R balance, torque effectiveness, pedal smoothness)

use crate::recording::types::{ExportError, Ride, RideSample};
use chrono::{DateTime, Duration, Utc};
use std::io::{Cursor, Write};

/// FIT epoch offset: FIT timestamps are seconds since 1989-12-31 00:00:00 UTC
const FIT_EPOCH_OFFSET: i64 = 631065600;

/// FIT file header size (14 bytes for header + protocol version)
const FIT_HEADER_SIZE: u8 = 14;

/// FIT protocol version
const FIT_PROTOCOL_VERSION: u8 = 0x20; // 2.0

/// FIT profile version (21.00)
const FIT_PROFILE_VERSION: u16 = 2100;

/// FIT message types
mod message_type {
    pub const FILE_ID: u16 = 0;
    pub const FILE_CREATOR: u16 = 49;
    pub const ACTIVITY: u16 = 34;
    pub const SESSION: u16 = 18;
    pub const LAP: u16 = 19;
    pub const RECORD: u16 = 20;
    pub const EVENT: u16 = 21;
}

/// FIT field types
mod field_type {
    pub const TIMESTAMP: u8 = 253;
    pub const POWER: u8 = 7;
    pub const HEART_RATE: u8 = 3;
    pub const CADENCE: u8 = 4;
    pub const DISTANCE: u8 = 5;
    pub const SPEED: u8 = 6;
    pub const LEFT_RIGHT_BALANCE: u8 = 30;
    pub const LEFT_TORQUE_EFFECTIVENESS: u8 = 57;
    pub const RIGHT_TORQUE_EFFECTIVENESS: u8 = 58;
    pub const LEFT_PEDAL_SMOOTHNESS: u8 = 59;
    pub const RIGHT_PEDAL_SMOOTHNESS: u8 = 60;
    pub const _CALORIES: u8 = 33; // Reserved for future use
                                  // T130: Power phase fields for extended pedal metrics
    pub const LEFT_POWER_PHASE: u8 = 69; // Start/end as 2x u16 (degrees * 256 / 360)
    pub const LEFT_POWER_PHASE_PEAK: u8 = 70; // Peak as 2x u16
    pub const RIGHT_POWER_PHASE: u8 = 71;
    pub const RIGHT_POWER_PHASE_PEAK: u8 = 72;
}

/// FIT base types
mod base_type {
    pub const UINT8: u8 = 0x00;
    pub const _SINT8: u8 = 0x01; // Reserved for future use
    pub const UINT16: u8 = 0x84;
    pub const _SINT16: u8 = 0x83; // Reserved for future use
    pub const UINT32: u8 = 0x86;
    pub const _SINT32: u8 = 0x85;
    pub const _STRING: u8 = 0x07; // Reserved for future use
    pub const ENUM: u8 = 0x00;
}

/// FIT file writer
struct FitWriter {
    buffer: Cursor<Vec<u8>>,
    data_size: u32,
}

impl FitWriter {
    fn new() -> Self {
        Self {
            buffer: Cursor::new(Vec::new()),
            data_size: 0,
        }
    }

    /// Write the FIT file header
    fn write_header(&mut self) -> Result<(), ExportError> {
        // Header size (14 bytes)
        self.buffer
            .write_all(&[FIT_HEADER_SIZE])
            .map_err(ExportError::IoError)?;

        // Protocol version
        self.buffer
            .write_all(&[FIT_PROTOCOL_VERSION])
            .map_err(ExportError::IoError)?;

        // Profile version (little endian)
        self.buffer
            .write_all(&FIT_PROFILE_VERSION.to_le_bytes())
            .map_err(ExportError::IoError)?;

        // Data size placeholder (will be updated later)
        self.buffer
            .write_all(&0u32.to_le_bytes())
            .map_err(ExportError::IoError)?;

        // Data type signature ".FIT"
        self.buffer
            .write_all(b".FIT")
            .map_err(ExportError::IoError)?;

        // Header CRC (for 14-byte header)
        let header_crc = calculate_crc(&self.buffer.get_ref()[0..12]);
        self.buffer
            .write_all(&header_crc.to_le_bytes())
            .map_err(ExportError::IoError)?;

        Ok(())
    }

    /// Write a definition message
    fn write_definition(
        &mut self,
        local_mesg_num: u8,
        global_mesg_num: u16,
        fields: &[(u8, u8, u8)], // (field_def_num, size, base_type)
    ) -> Result<(), ExportError> {
        // Record header: definition message (bit 6 set), local message num in bits 0-3
        let header = 0x40 | (local_mesg_num & 0x0F);
        self.write_byte(header)?;

        // Reserved byte
        self.write_byte(0)?;

        // Architecture: 0 = little endian
        self.write_byte(0)?;

        // Global message number (little endian)
        self.write_u16(global_mesg_num)?;

        // Number of fields
        self.write_byte(fields.len() as u8)?;

        // Field definitions
        for (field_num, size, base_type) in fields {
            self.write_byte(*field_num)?;
            self.write_byte(*size)?;
            self.write_byte(*base_type)?;
        }

        Ok(())
    }

    /// Write a data message header
    fn write_data_header(&mut self, local_mesg_num: u8) -> Result<(), ExportError> {
        // Record header: data message (bit 6 clear), local message num in bits 0-3
        let header = local_mesg_num & 0x0F;
        self.write_byte(header)?;
        Ok(())
    }

    fn write_byte(&mut self, value: u8) -> Result<(), ExportError> {
        self.buffer
            .write_all(&[value])
            .map_err(ExportError::IoError)?;
        self.data_size += 1;
        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<(), ExportError> {
        self.buffer
            .write_all(&value.to_le_bytes())
            .map_err(ExportError::IoError)?;
        self.data_size += 2;
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<(), ExportError> {
        self.buffer
            .write_all(&value.to_le_bytes())
            .map_err(ExportError::IoError)?;
        self.data_size += 4;
        Ok(())
    }

    #[allow(dead_code)]
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ExportError> {
        self.buffer.write_all(bytes).map_err(ExportError::IoError)?;
        self.data_size += bytes.len() as u32;
        Ok(())
    }

    /// Convert DateTime to FIT timestamp
    fn datetime_to_fit_timestamp(dt: DateTime<Utc>) -> u32 {
        (dt.timestamp() - FIT_EPOCH_OFFSET) as u32
    }

    /// Finalize the file (update data size and append CRC)
    fn finalize(self) -> Result<Vec<u8>, ExportError> {
        let data_size = self.data_size;
        let mut data = self.buffer.into_inner();

        // Update data size in header (bytes 4-7)
        let data_size_bytes = data_size.to_le_bytes();
        data[4..8].copy_from_slice(&data_size_bytes);

        // Calculate CRC for the entire file (excluding the final CRC itself)
        let file_crc = calculate_crc(&data[..]);
        data.extend_from_slice(&file_crc.to_le_bytes());

        Ok(data)
    }
}

/// Calculate CRC-16 for FIT file (standalone function)
fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    let crc_table: [u16; 16] = [
        0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800,
        0xB401, 0x5000, 0x9C01, 0x8801, 0x4400,
    ];

    for byte in data {
        let tmp = crc_table[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ crc_table[(*byte & 0xF) as usize];

        let tmp = crc_table[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ crc_table[((*byte >> 4) & 0xF) as usize];
    }

    crc
}

/// Export a ride to FIT format with cycling dynamics support.
pub fn export_fit(ride: &Ride, samples: &[RideSample]) -> Result<Vec<u8>, ExportError> {
    if samples.is_empty() {
        return Err(ExportError::NoData);
    }

    let mut writer = FitWriter::new();

    // Write FIT header
    writer.write_header()?;

    // Write File ID message
    write_file_id(&mut writer, ride)?;

    // Write File Creator message
    write_file_creator(&mut writer)?;

    // Write Event message (timer start)
    write_event(&mut writer, ride.started_at, 0, 0)?; // timer start

    // Check if any sample has dynamics data
    let has_dynamics = samples.iter().any(|s| {
        s.left_right_balance.is_some()
            || s.left_torque_effectiveness.is_some()
            || s.right_torque_effectiveness.is_some()
            || s.left_pedal_smoothness.is_some()
            || s.right_pedal_smoothness.is_some()
    });

    // T130: Check if any sample has power phase data
    let has_power_phase = samples
        .iter()
        .any(|s| s.left_power_phase_start.is_some() || s.right_power_phase_start.is_some());

    // Write Record messages for each sample
    write_records(
        &mut writer,
        ride.started_at,
        samples,
        has_dynamics,
        has_power_phase,
    )?;

    // Write Event message (timer stop)
    let end_time = ride.ended_at.unwrap_or(ride.started_at);
    write_event(&mut writer, end_time, 0, 1)?; // timer stop

    // Write Lap message
    write_lap(&mut writer, ride)?;

    // Write Session message
    write_session(&mut writer, ride)?;

    // Write Activity message
    write_activity(&mut writer, ride)?;

    // Finalize and return
    writer.finalize()
}

/// Write File ID message
fn write_file_id(writer: &mut FitWriter, ride: &Ride) -> Result<(), ExportError> {
    // Define File ID message (local message 0)
    let fields = [
        (0, 1, base_type::ENUM),   // type (activity = 4)
        (1, 2, base_type::UINT16), // manufacturer
        (2, 2, base_type::UINT16), // product
        (3, 4, base_type::UINT32), // serial_number
        (4, 4, base_type::UINT32), // time_created
    ];
    writer.write_definition(0, message_type::FILE_ID, &fields)?;

    // Write File ID data
    writer.write_data_header(0)?;
    writer.write_byte(4)?; // type = activity
    writer.write_u16(1)?; // manufacturer = Garmin (for compatibility)
    writer.write_u16(1)?; // product
    writer.write_u32(1)?; // serial_number
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(ride.started_at))?;

    Ok(())
}

/// Write File Creator message
fn write_file_creator(writer: &mut FitWriter) -> Result<(), ExportError> {
    let fields = [
        (0, 2, base_type::UINT16), // software_version
    ];
    writer.write_definition(1, message_type::FILE_CREATOR, &fields)?;

    writer.write_data_header(1)?;
    writer.write_u16(100)?; // software_version = 1.00

    Ok(())
}

/// Write Event message
fn write_event(
    writer: &mut FitWriter,
    timestamp: DateTime<Utc>,
    event: u8,
    event_type: u8,
) -> Result<(), ExportError> {
    let fields = [
        (field_type::TIMESTAMP, 4, base_type::UINT32), // timestamp
        (0, 1, base_type::ENUM),                       // event
        (1, 1, base_type::ENUM),                       // event_type
    ];
    writer.write_definition(2, message_type::EVENT, &fields)?;

    writer.write_data_header(2)?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(timestamp))?;
    writer.write_byte(event)?; // event = timer (0)
    writer.write_byte(event_type)?; // event_type = start(0) or stop(1)

    Ok(())
}

/// Write Record messages with cycling dynamics
fn write_records(
    writer: &mut FitWriter,
    start_time: DateTime<Utc>,
    samples: &[RideSample],
    has_dynamics: bool,
    has_power_phase: bool,
) -> Result<(), ExportError> {
    // Define Record message with or without dynamics fields
    let mut fields = vec![
        (field_type::TIMESTAMP, 4, base_type::UINT32), // timestamp
        (field_type::POWER, 2, base_type::UINT16),     // power (watts)
        (field_type::HEART_RATE, 1, base_type::UINT8), // heart_rate (bpm)
        (field_type::CADENCE, 1, base_type::UINT8),    // cadence (rpm)
        (field_type::DISTANCE, 4, base_type::UINT32),  // distance (scaled, 100 * m)
        (field_type::SPEED, 2, base_type::UINT16),     // speed (scaled, 1000 * m/s)
    ];

    if has_dynamics {
        fields.extend_from_slice(&[
            (field_type::LEFT_RIGHT_BALANCE, 1, base_type::UINT8), // left_right_balance (scaled)
            (field_type::LEFT_TORQUE_EFFECTIVENESS, 1, base_type::UINT8), // left_torque_eff (0.5%)
            (field_type::RIGHT_TORQUE_EFFECTIVENESS, 1, base_type::UINT8), // right_torque_eff (0.5%)
            (field_type::LEFT_PEDAL_SMOOTHNESS, 1, base_type::UINT8), // left_pedal_smoothness (0.5%)
            (field_type::RIGHT_PEDAL_SMOOTHNESS, 1, base_type::UINT8), // right_pedal_smoothness (0.5%)
        ]);
    }

    // T130: Add power phase fields
    if has_power_phase {
        fields.extend_from_slice(&[
            (field_type::LEFT_POWER_PHASE, 4, base_type::UINT16), // start, end (2x u16)
            (field_type::LEFT_POWER_PHASE_PEAK, 4, base_type::UINT16), // peak start, peak end
            (field_type::RIGHT_POWER_PHASE, 4, base_type::UINT16),
            (field_type::RIGHT_POWER_PHASE_PEAK, 4, base_type::UINT16),
        ]);
    }

    writer.write_definition(3, message_type::RECORD, &fields)?;

    // Write each sample
    for sample in samples {
        writer.write_data_header(3)?;

        // Timestamp
        let sample_time = start_time + Duration::seconds(sample.elapsed_seconds as i64);
        writer.write_u32(FitWriter::datetime_to_fit_timestamp(sample_time))?;

        // Power (watts, 0xFFFF = invalid)
        writer.write_u16(sample.power_watts.unwrap_or(0xFFFF))?;

        // Heart rate (bpm, 0xFF = invalid)
        writer.write_byte(sample.heart_rate_bpm.unwrap_or(0xFF))?;

        // Cadence (rpm, 0xFF = invalid)
        writer.write_byte(sample.cadence_rpm.unwrap_or(0xFF))?;

        // Distance (100 * meters for scaling)
        writer.write_u32((sample.distance_meters * 100.0) as u32)?;

        // Speed (1000 * m/s, convert from km/h)
        let speed_ms = sample.speed_kmh.map(|s| (s / 3.6 * 1000.0) as u16);
        writer.write_u16(speed_ms.unwrap_or(0xFFFF))?;

        if has_dynamics {
            // Left/Right balance: 0-100 for left %, bit 7 set = left is reference
            // FIT format: bit 7 = which side is reference, bits 0-6 = percentage
            let balance = sample.left_right_balance.map(|b| {
                let pct = (b.clamp(0.0, 100.0) * 1.27) as u8; // Scale 0-100 to 0-127
                pct | 0x80 // Set bit 7 to indicate left is reference
            });
            writer.write_byte(balance.unwrap_or(0xFF))?;

            // Torque effectiveness: 0.5% per bit (0-100% = 0-200)
            let left_te = sample
                .left_torque_effectiveness
                .map(|t| (t.clamp(0.0, 100.0) * 2.0) as u8);
            writer.write_byte(left_te.unwrap_or(0xFF))?;

            let right_te = sample
                .right_torque_effectiveness
                .map(|t| (t.clamp(0.0, 100.0) * 2.0) as u8);
            writer.write_byte(right_te.unwrap_or(0xFF))?;

            // Pedal smoothness: 0.5% per bit (0-100% = 0-200)
            let left_ps = sample
                .left_pedal_smoothness
                .map(|s| (s.clamp(0.0, 100.0) * 2.0) as u8);
            writer.write_byte(left_ps.unwrap_or(0xFF))?;

            let right_ps = sample
                .right_pedal_smoothness
                .map(|s| (s.clamp(0.0, 100.0) * 2.0) as u8);
            writer.write_byte(right_ps.unwrap_or(0xFF))?;
        }

        // T130: Write power phase data
        if has_power_phase {
            // Convert degrees to FIT format: degrees * 256 / 360 (0.7111 factor)
            fn degrees_to_fit(deg: f32) -> u16 {
                ((deg.clamp(0.0, 360.0) * 256.0 / 360.0) as u16).min(255)
            }

            // Left power phase (start, end)
            let left_start = sample
                .left_power_phase_start
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            let left_end = sample
                .left_power_phase_end
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            writer.write_u16(left_start)?;
            writer.write_u16(left_end)?;

            // Left power phase peak (peak start, peak end - we use same value for both)
            let left_peak = sample
                .left_power_phase_peak
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            writer.write_u16(left_peak)?;
            writer.write_u16(left_peak)?;

            // Right power phase (start, end)
            let right_start = sample
                .right_power_phase_start
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            let right_end = sample
                .right_power_phase_end
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            writer.write_u16(right_start)?;
            writer.write_u16(right_end)?;

            // Right power phase peak
            let right_peak = sample
                .right_power_phase_peak
                .map(degrees_to_fit)
                .unwrap_or(0xFFFF);
            writer.write_u16(right_peak)?;
            writer.write_u16(right_peak)?;
        }
    }

    Ok(())
}

/// Write Lap message
fn write_lap(writer: &mut FitWriter, ride: &Ride) -> Result<(), ExportError> {
    let fields = [
        (field_type::TIMESTAMP, 4, base_type::UINT32), // timestamp
        (2, 4, base_type::UINT32),                     // start_time
        (7, 4, base_type::UINT32),                     // total_elapsed_time (1000 * s)
        (8, 4, base_type::UINT32),                     // total_timer_time (1000 * s)
        (9, 4, base_type::UINT32),                     // total_distance (100 * m)
        (11, 2, base_type::UINT16),                    // total_calories
        (13, 1, base_type::UINT8),                     // avg_heart_rate
        (14, 1, base_type::UINT8),                     // max_heart_rate
        (15, 1, base_type::UINT8),                     // avg_cadence
        (19, 2, base_type::UINT16),                    // avg_power
        (20, 2, base_type::UINT16),                    // max_power
        (25, 1, base_type::ENUM),                      // event (lap = 9)
        (26, 1, base_type::ENUM),                      // event_type (stop = 1)
    ];
    writer.write_definition(4, message_type::LAP, &fields)?;

    let end_time = ride.ended_at.unwrap_or(ride.started_at);
    let total_time_ms = ride.duration_seconds * 1000;
    let total_distance_scaled = (ride.distance_meters * 100.0) as u32;

    writer.write_data_header(4)?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(end_time))?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(ride.started_at))?;
    writer.write_u32(total_time_ms)?;
    writer.write_u32(total_time_ms)?;
    writer.write_u32(total_distance_scaled)?;
    writer.write_u16(ride.calories as u16)?;
    writer.write_byte(ride.avg_hr.unwrap_or(0xFF))?;
    writer.write_byte(ride.max_hr.unwrap_or(0xFF))?;
    writer.write_byte(ride.avg_cadence.unwrap_or(0xFF))?;
    writer.write_u16(ride.avg_power.unwrap_or(0xFFFF))?;
    writer.write_u16(ride.max_power.unwrap_or(0xFFFF))?;
    writer.write_byte(9)?; // event = lap
    writer.write_byte(1)?; // event_type = stop

    Ok(())
}

/// Write Session message
fn write_session(writer: &mut FitWriter, ride: &Ride) -> Result<(), ExportError> {
    let fields = [
        (field_type::TIMESTAMP, 4, base_type::UINT32), // timestamp
        (2, 4, base_type::UINT32),                     // start_time
        (5, 1, base_type::ENUM),                       // sport (cycling = 2)
        (6, 1, base_type::ENUM),                       // sub_sport (indoor_cycling = 6)
        (7, 4, base_type::UINT32),                     // total_elapsed_time
        (8, 4, base_type::UINT32),                     // total_timer_time
        (9, 4, base_type::UINT32),                     // total_distance
        (11, 2, base_type::UINT16),                    // total_calories
        (16, 1, base_type::UINT8),                     // avg_heart_rate
        (17, 1, base_type::UINT8),                     // max_heart_rate
        (18, 1, base_type::UINT8),                     // avg_cadence
        (20, 2, base_type::UINT16),                    // avg_power
        (21, 2, base_type::UINT16),                    // max_power
        (25, 1, base_type::ENUM),                      // event (session = 8)
        (26, 1, base_type::ENUM),                      // event_type (stop = 1)
        (28, 2, base_type::UINT16),                    // num_laps
    ];
    writer.write_definition(5, message_type::SESSION, &fields)?;

    let end_time = ride.ended_at.unwrap_or(ride.started_at);
    let total_time_ms = ride.duration_seconds * 1000;
    let total_distance_scaled = (ride.distance_meters * 100.0) as u32;

    writer.write_data_header(5)?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(end_time))?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(ride.started_at))?;
    writer.write_byte(2)?; // sport = cycling
    writer.write_byte(6)?; // sub_sport = indoor_cycling
    writer.write_u32(total_time_ms)?;
    writer.write_u32(total_time_ms)?;
    writer.write_u32(total_distance_scaled)?;
    writer.write_u16(ride.calories as u16)?;
    writer.write_byte(ride.avg_hr.unwrap_or(0xFF))?;
    writer.write_byte(ride.max_hr.unwrap_or(0xFF))?;
    writer.write_byte(ride.avg_cadence.unwrap_or(0xFF))?;
    writer.write_u16(ride.avg_power.unwrap_or(0xFFFF))?;
    writer.write_u16(ride.max_power.unwrap_or(0xFFFF))?;
    writer.write_byte(8)?; // event = session
    writer.write_byte(1)?; // event_type = stop
    writer.write_u16(1)?; // num_laps = 1

    Ok(())
}

/// Write Activity message
fn write_activity(writer: &mut FitWriter, ride: &Ride) -> Result<(), ExportError> {
    let fields = [
        (field_type::TIMESTAMP, 4, base_type::UINT32), // timestamp
        (0, 4, base_type::UINT32),                     // total_timer_time
        (1, 2, base_type::UINT16),                     // num_sessions
        (2, 1, base_type::ENUM),                       // type (manual = 0)
        (3, 1, base_type::ENUM),                       // event (activity = 26)
        (4, 1, base_type::ENUM),                       // event_type (stop = 1)
        (5, 4, base_type::UINT32),                     // local_timestamp
    ];
    writer.write_definition(6, message_type::ACTIVITY, &fields)?;

    let end_time = ride.ended_at.unwrap_or(ride.started_at);
    let total_time_ms = ride.duration_seconds * 1000;

    writer.write_data_header(6)?;
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(end_time))?;
    writer.write_u32(total_time_ms)?;
    writer.write_u16(1)?; // num_sessions = 1
    writer.write_byte(0)?; // type = manual
    writer.write_byte(26)?; // event = activity
    writer.write_byte(1)?; // event_type = stop
    writer.write_u32(FitWriter::datetime_to_fit_timestamp(end_time))?;

    Ok(())
}

/// Export a ride to FIT and write to a file.
pub fn export_fit_to_file(
    ride: &Ride,
    samples: &[RideSample],
    path: &std::path::Path,
) -> Result<(), ExportError> {
    let content = export_fit(ride, samples)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a FIT ride export.
pub fn generate_fit_filename(ride: &Ride) -> String {
    let timestamp = ride.started_at.format("%Y%m%d_%H%M%S");
    format!("RustRide_{}.fit", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_ride() -> Ride {
        let user_id = Uuid::new_v4();
        let mut ride = Ride::new(user_id, 200);
        ride.started_at = Utc::now();
        ride.ended_at = Some(Utc::now());
        ride.duration_seconds = 3600;
        ride.distance_meters = 30000.0;
        ride.avg_power = Some(180);
        ride.max_power = Some(350);
        ride.avg_hr = Some(145);
        ride.max_hr = Some(175);
        ride.avg_cadence = Some(85);
        ride.calories = 720;
        ride
    }

    fn create_test_samples(count: usize) -> Vec<RideSample> {
        (0..count)
            .map(|i| RideSample {
                elapsed_seconds: i as u32,
                power_watts: Some(180 + (i % 30) as u16),
                cadence_rpm: Some(85),
                heart_rate_bpm: Some(145),
                speed_kmh: Some(30.0),
                distance_meters: i as f64 * 8.33,
                calories: (i as f64 * 0.2) as u32,
                resistance_level: None,
                target_power: None,
                trainer_grade: None,
                left_right_balance: None,
                left_torque_effectiveness: None,
                right_torque_effectiveness: None,
                left_pedal_smoothness: None,
                right_pedal_smoothness: None,
                left_power_phase_start: None,
                left_power_phase_end: None,
                left_power_phase_peak: None,
                right_power_phase_start: None,
                right_power_phase_end: None,
                right_power_phase_peak: None,
            })
            .collect()
    }

    fn create_test_samples_with_dynamics(count: usize) -> Vec<RideSample> {
        (0..count)
            .map(|i| RideSample {
                elapsed_seconds: i as u32,
                power_watts: Some(180 + (i % 30) as u16),
                cadence_rpm: Some(85),
                heart_rate_bpm: Some(145),
                speed_kmh: Some(30.0),
                distance_meters: i as f64 * 8.33,
                calories: (i as f64 * 0.2) as u32,
                resistance_level: None,
                target_power: None,
                trainer_grade: None,
                left_right_balance: Some(52.0),
                left_torque_effectiveness: Some(75.0),
                right_torque_effectiveness: Some(72.0),
                left_pedal_smoothness: Some(22.0),
                right_pedal_smoothness: Some(24.0),
                left_power_phase_start: None,
                left_power_phase_end: None,
                left_power_phase_peak: None,
                right_power_phase_start: None,
                right_power_phase_end: None,
                right_power_phase_peak: None,
            })
            .collect()
    }

    #[test]
    fn test_export_fit_generates_valid_header() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let result = export_fit(&ride, &samples);
        assert!(result.is_ok());

        let data = result.unwrap();
        // Check header size
        assert_eq!(data[0], 14);
        // Check protocol version
        assert_eq!(data[1], 0x20);
        // Check .FIT signature
        assert_eq!(&data[8..12], b".FIT");
    }

    #[test]
    fn test_export_fit_empty_samples_error() {
        let ride = create_test_ride();
        let samples: Vec<RideSample> = vec![];

        let result = export_fit(&ride, &samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_fit_with_dynamics() {
        let ride = create_test_ride();
        let samples = create_test_samples_with_dynamics(10);

        let result = export_fit(&ride, &samples);
        assert!(result.is_ok());

        let data = result.unwrap();
        // File should be larger with dynamics fields
        assert!(data.len() > 100);
    }

    #[test]
    fn test_export_fit_without_dynamics() {
        let ride = create_test_ride();
        let samples = create_test_samples(10);

        let result = export_fit(&ride, &samples);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_fit_filename() {
        let ride = create_test_ride();
        let filename = generate_fit_filename(&ride);

        assert!(filename.starts_with("RustRide_"));
        assert!(filename.ends_with(".fit"));
    }

    #[test]
    fn test_datetime_to_fit_timestamp() {
        use chrono::TimeZone;
        // FIT epoch is 1989-12-31 00:00:00 UTC
        let dt = Utc.with_ymd_and_hms(1989, 12, 31, 0, 0, 0).unwrap();
        assert_eq!(FitWriter::datetime_to_fit_timestamp(dt), 0);

        // One day later
        let dt = Utc.with_ymd_and_hms(1990, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(FitWriter::datetime_to_fit_timestamp(dt), 86400);
    }

    #[test]
    fn test_fit_crc_calculation() {
        // Test with known data
        let data = b"test";
        let crc = calculate_crc(data);
        // CRC should be non-zero for non-empty data
        assert_ne!(crc, 0);
    }
}
