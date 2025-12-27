//! ANT+ Device Profiles
//!
//! Implements parsers for various ANT+ device profiles.

pub mod fec;
pub mod hr;
pub mod power;

use super::AntDeviceType;

/// Common trait for ANT+ profile data pages
pub trait AntProfilePage: Send + Sync {
    /// Get the page number
    fn page_number(&self) -> u8;

    /// Parse raw data into this page type
    fn parse(data: &[u8]) -> Option<Self>
    where
        Self: Sized;

    /// Get the device type this page belongs to
    fn device_type() -> AntDeviceType;
}

/// Common manufacturer information page (Page 80)
#[derive(Debug, Clone)]
pub struct ManufacturerInfo {
    pub hw_revision: u8,
    pub manufacturer_id: u16,
    pub model_number: u16,
}

impl ManufacturerInfo {
    /// Parse manufacturer info page
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 80 {
            return None;
        }

        Some(Self {
            hw_revision: data[3],
            manufacturer_id: u16::from_le_bytes([data[4], data[5]]),
            model_number: u16::from_le_bytes([data[6], data[7]]),
        })
    }
}

/// Product information page (Page 81)
#[derive(Debug, Clone)]
pub struct ProductInfo {
    pub sw_revision_supplemental: u8,
    pub sw_revision_main: u8,
    pub serial_number: u32,
}

impl ProductInfo {
    /// Parse product info page
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 81 {
            return None;
        }

        Some(Self {
            sw_revision_supplemental: data[2],
            sw_revision_main: data[3],
            serial_number: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        })
    }
}

/// Battery status page (Page 82)
#[derive(Debug, Clone)]
pub struct BatteryStatus {
    pub operating_time: u32, // in 2-second units
    pub battery_voltage: f32,
    pub battery_status: BatteryStatusValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatusValue {
    New,
    Good,
    Ok,
    Low,
    Critical,
    Invalid,
    Unknown(u8),
}

impl BatteryStatus {
    /// Parse battery status page
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 82 {
            return None;
        }

        let operating_time = u32::from_le_bytes([data[3], data[4], data[5], 0]) & 0x00FFFFFF;

        let voltage_fractional = data[6];
        let coarse_and_status = data[7];
        let voltage_coarse = coarse_and_status & 0x0F;
        let status_bits = (coarse_and_status >> 4) & 0x07;

        let battery_voltage = voltage_coarse as f32 + (voltage_fractional as f32 / 256.0);

        let battery_status = match status_bits {
            1 => BatteryStatusValue::New,
            2 => BatteryStatusValue::Good,
            3 => BatteryStatusValue::Ok,
            4 => BatteryStatusValue::Low,
            5 => BatteryStatusValue::Critical,
            7 => BatteryStatusValue::Invalid,
            x => BatteryStatusValue::Unknown(x),
        };

        Some(Self {
            operating_time,
            battery_voltage,
            battery_status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manufacturer_info_parse() {
        let data = [80, 0xFF, 0xFF, 0x01, 0x20, 0x00, 0x64, 0x00];
        let info = ManufacturerInfo::parse(&data).unwrap();
        assert_eq!(info.hw_revision, 1);
        assert_eq!(info.manufacturer_id, 32); // Wahoo
        assert_eq!(info.model_number, 100);
    }

    #[test]
    fn test_product_info_parse() {
        let data = [81, 0xFF, 0x01, 0x02, 0x78, 0x56, 0x34, 0x12];
        let info = ProductInfo::parse(&data).unwrap();
        assert_eq!(info.sw_revision_supplemental, 1);
        assert_eq!(info.sw_revision_main, 2);
        assert_eq!(info.serial_number, 0x12345678);
    }
}
