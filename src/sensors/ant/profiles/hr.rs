//! ANT+ Heart Rate Profile
//!
//! Implements the ANT+ Heart Rate Monitor device profile.
//! Device Type: 120

use super::AntProfilePage;
use crate::sensors::ant::AntDeviceType;

/// Heart rate data page (Page 0 - Legacy format)
#[derive(Debug, Clone)]
pub struct HeartRatePageLegacy {
    /// Heart beat event time (1/1024 second resolution)
    pub beat_time: u16,
    /// Heart beat count (wraps at 255)
    pub beat_count: u8,
    /// Computed heart rate in BPM
    pub heart_rate: u8,
}

impl AntProfilePage for HeartRatePageLegacy {
    fn page_number(&self) -> u8 {
        0
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        // Page 0 has no page number indicator in first byte
        // Data starts from byte 4
        Some(Self {
            beat_time: u16::from_le_bytes([data[4], data[5]]),
            beat_count: data[6],
            heart_rate: data[7],
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::HeartRate
    }
}

/// Heart rate data page (Page 4 - with manufacturer info)
#[derive(Debug, Clone)]
pub struct HeartRatePage4 {
    /// Manufacturer ID
    pub manufacturer_id: u8,
    /// Serial number (lower 16 bits)
    pub serial_number: u16,
    /// Heart beat event time (1/1024 second resolution)
    pub beat_time: u16,
    /// Heart beat count
    pub beat_count: u8,
    /// Computed heart rate in BPM
    pub heart_rate: u8,
}

impl AntProfilePage for HeartRatePage4 {
    fn page_number(&self) -> u8 {
        4
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || (data[0] & 0x7F) != 4 {
            return None;
        }

        Some(Self {
            manufacturer_id: data[1],
            serial_number: u16::from_le_bytes([data[2], data[3]]),
            beat_time: u16::from_le_bytes([data[4], data[5]]),
            beat_count: data[6],
            heart_rate: data[7],
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::HeartRate
    }
}

/// Heart rate data page (Page 2 - with cumulative operating time)
#[derive(Debug, Clone)]
pub struct HeartRatePage2 {
    /// Cumulative operating time in 2-second units
    pub operating_time: u32,
    /// Heart beat event time
    pub beat_time: u16,
    /// Heart beat count
    pub beat_count: u8,
    /// Computed heart rate in BPM
    pub heart_rate: u8,
}

impl AntProfilePage for HeartRatePage2 {
    fn page_number(&self) -> u8 {
        2
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || (data[0] & 0x7F) != 2 {
            return None;
        }

        let operating_time = u32::from_le_bytes([data[1], data[2], data[3], 0]) & 0x00FFFFFF;

        Some(Self {
            operating_time,
            beat_time: u16::from_le_bytes([data[4], data[5]]),
            beat_count: data[6],
            heart_rate: data[7],
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::HeartRate
    }
}

/// Heart rate data page (Page 3 - with software version)
#[derive(Debug, Clone)]
pub struct HeartRatePage3 {
    /// Hardware version
    pub hardware_version: u8,
    /// Software version
    pub software_version: u8,
    /// Model number
    pub model_number: u8,
    /// Heart beat event time
    pub beat_time: u16,
    /// Heart beat count
    pub beat_count: u8,
    /// Computed heart rate in BPM
    pub heart_rate: u8,
}

impl AntProfilePage for HeartRatePage3 {
    fn page_number(&self) -> u8 {
        3
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || (data[0] & 0x7F) != 3 {
            return None;
        }

        Some(Self {
            hardware_version: data[1],
            software_version: data[2],
            model_number: data[3],
            beat_time: u16::from_le_bytes([data[4], data[5]]),
            beat_count: data[6],
            heart_rate: data[7],
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::HeartRate
    }
}

/// Generic heart rate page for any page number
#[derive(Debug, Clone)]
pub struct HeartRateData {
    /// Page number (0-7)
    pub page_number: u8,
    /// Whether page change toggle bit is set
    pub page_change_toggle: bool,
    /// Heart beat event time (1/1024 second resolution)
    pub beat_time: u16,
    /// Heart beat count (wraps at 255)
    pub beat_count: u8,
    /// Computed heart rate in BPM
    pub heart_rate: u8,
}

impl HeartRateData {
    /// Parse any heart rate page
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let page_byte = data[0];
        let page_number = page_byte & 0x7F;
        let page_change_toggle = (page_byte & 0x80) != 0;

        Some(Self {
            page_number,
            page_change_toggle,
            beat_time: u16::from_le_bytes([data[4], data[5]]),
            beat_count: data[6],
            heart_rate: data[7],
        })
    }
}

/// Heart rate monitor state for RR interval calculation
#[derive(Debug, Default)]
pub struct HeartRateState {
    last_beat_time: u16,
    last_beat_count: u8,
    rr_intervals: Vec<u16>,
}

impl HeartRateState {
    /// Update state with new data and calculate RR interval
    pub fn update(&mut self, data: &HeartRateData) -> Option<HeartRateUpdate> {
        // Check for new beat
        let beat_delta = data.beat_count.wrapping_sub(self.last_beat_count);

        if beat_delta == 0 {
            // No new beat, just return current HR
            return Some(HeartRateUpdate {
                heart_rate: data.heart_rate,
                rr_interval_ms: None,
            });
        }

        // Calculate RR interval (time between beats)
        let time_delta = data.beat_time.wrapping_sub(self.last_beat_time);
        // Convert from 1/1024 seconds to milliseconds
        let rr_ms = (time_delta as u32 * 1000 / 1024) as u16;

        self.last_beat_time = data.beat_time;
        self.last_beat_count = data.beat_count;

        // Store RR interval for HRV analysis
        self.rr_intervals.push(rr_ms);
        if self.rr_intervals.len() > 60 {
            self.rr_intervals.remove(0);
        }

        Some(HeartRateUpdate {
            heart_rate: data.heart_rate,
            rr_interval_ms: Some(rr_ms),
        })
    }

    /// Get recent RR intervals for HRV analysis
    pub fn recent_rr_intervals(&self) -> &[u16] {
        &self.rr_intervals
    }

    /// Calculate RMSSD (HRV metric)
    pub fn calculate_rmssd(&self) -> Option<f32> {
        if self.rr_intervals.len() < 2 {
            return None;
        }

        let mut sum_squared_diff = 0.0_f64;
        let mut count = 0;

        for window in self.rr_intervals.windows(2) {
            let diff = (window[1] as f64) - (window[0] as f64);
            sum_squared_diff += diff * diff;
            count += 1;
        }

        if count == 0 {
            return None;
        }

        Some((sum_squared_diff / count as f64).sqrt() as f32)
    }
}

/// Heart rate update result
#[derive(Debug, Clone)]
pub struct HeartRateUpdate {
    /// Current heart rate in BPM
    pub heart_rate: u8,
    /// RR interval in milliseconds (if new beat detected)
    pub rr_interval_ms: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heart_rate_data_parse() {
        // Example heart rate page with toggle bit set
        let data = [0x84, 0x01, 0x12, 0x34, 0xE8, 0x03, 0x0A, 0x48];
        let hr = HeartRateData::parse(&data).unwrap();

        assert_eq!(hr.page_number, 4);
        assert!(hr.page_change_toggle);
        assert_eq!(hr.heart_rate, 72);
        assert_eq!(hr.beat_count, 10);
    }

    #[test]
    fn test_heart_rate_page4_parse() {
        let data = [0x04, 0x01, 0xAB, 0xCD, 0x00, 0x04, 0x15, 0x50];
        let page = HeartRatePage4::parse(&data).unwrap();

        assert_eq!(page.manufacturer_id, 1); // Garmin
        assert_eq!(page.serial_number, 0xCDAB);
        assert_eq!(page.heart_rate, 80);
    }

    #[test]
    fn test_heart_rate_state_update() {
        let mut state = HeartRateState::default();

        let data1 = HeartRateData {
            page_number: 0,
            page_change_toggle: false,
            beat_time: 1024, // 1 second
            beat_count: 1,
            heart_rate: 60,
        };
        state.update(&data1);

        let data2 = HeartRateData {
            page_number: 0,
            page_change_toggle: false,
            beat_time: 2048, // 2 seconds
            beat_count: 2,
            heart_rate: 60,
        };
        let update = state.update(&data2).unwrap();

        assert_eq!(update.heart_rate, 60);
        assert!(update.rr_interval_ms.is_some());
        // 1024 ticks = 1000ms at 60bpm
        assert_eq!(update.rr_interval_ms.unwrap(), 1000);
    }

    #[test]
    fn test_rmssd_calculation() {
        let mut state = HeartRateState {
            last_beat_time: 0,
            last_beat_count: 0,
            rr_intervals: vec![800, 810, 790, 820, 780],
        };

        let rmssd = state.calculate_rmssd().unwrap();
        assert!(rmssd > 0.0);
        assert!(rmssd < 50.0); // Reasonable RMSSD range
    }
}
