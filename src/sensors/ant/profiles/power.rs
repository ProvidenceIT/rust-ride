//! ANT+ Cycling Power Profile
//!
//! Implements the ANT+ Cycling Power device profile for power meters.
//! Device Type: 11

use super::AntProfilePage;
use crate::sensors::ant::AntDeviceType;

/// Power-only data page (Page 0x10 / 16)
#[derive(Debug, Clone)]
pub struct PowerOnlyPage {
    /// Update event count (wraps at 255)
    pub event_count: u8,
    /// Pedal power balance (if supported)
    pub pedal_balance: Option<PedalBalance>,
    /// Instantaneous cadence (if available)
    pub cadence: Option<u8>,
    /// Accumulated power (wraps at 65535)
    pub accumulated_power: u16,
    /// Instantaneous power in watts
    pub instantaneous_power: u16,
}

/// Pedal power balance
#[derive(Debug, Clone, Copy)]
pub struct PedalBalance {
    /// Right pedal percentage (0-100)
    pub right_percent: u8,
    /// Whether balance data is valid
    pub is_valid: bool,
}

impl AntProfilePage for PowerOnlyPage {
    fn page_number(&self) -> u8 {
        0x10
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x10 {
            return None;
        }

        let balance_byte = data[1];
        let pedal_balance = if balance_byte == 0xFF {
            None
        } else {
            Some(PedalBalance {
                right_percent: balance_byte & 0x7F,
                is_valid: (balance_byte & 0x80) != 0,
            })
        };

        let cadence = if data[2] == 0xFF { None } else { Some(data[2]) };

        Some(Self {
            event_count: data[3],
            pedal_balance,
            cadence,
            accumulated_power: u16::from_le_bytes([data[4], data[5]]),
            instantaneous_power: u16::from_le_bytes([data[6], data[7]]),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::Power
    }
}

/// Wheel torque data page (Page 0x11 / 17)
#[derive(Debug, Clone)]
pub struct WheelTorquePage {
    /// Update event count
    pub event_count: u8,
    /// Wheel ticks (revolutions)
    pub wheel_ticks: u8,
    /// Instantaneous cadence
    pub cadence: Option<u8>,
    /// Accumulated wheel period (1/2048 second units)
    pub accumulated_wheel_period: u16,
    /// Accumulated torque (1/32 Nm units)
    pub accumulated_torque: u16,
}

impl AntProfilePage for WheelTorquePage {
    fn page_number(&self) -> u8 {
        0x11
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x11 {
            return None;
        }

        Some(Self {
            event_count: data[1],
            wheel_ticks: data[2],
            cadence: if data[3] == 0xFF { None } else { Some(data[3]) },
            accumulated_wheel_period: u16::from_le_bytes([data[4], data[5]]),
            accumulated_torque: u16::from_le_bytes([data[6], data[7]]),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::Power
    }
}

/// Crank torque data page (Page 0x12 / 18)
#[derive(Debug, Clone)]
pub struct CrankTorquePage {
    /// Update event count
    pub event_count: u8,
    /// Crank ticks (revolutions)
    pub crank_ticks: u8,
    /// Instantaneous cadence
    pub cadence: Option<u8>,
    /// Accumulated crank period (1/2048 second units)
    pub accumulated_crank_period: u16,
    /// Accumulated torque (1/32 Nm units)
    pub accumulated_torque: u16,
}

impl AntProfilePage for CrankTorquePage {
    fn page_number(&self) -> u8 {
        0x12
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x12 {
            return None;
        }

        Some(Self {
            event_count: data[1],
            crank_ticks: data[2],
            cadence: if data[3] == 0xFF { None } else { Some(data[3]) },
            accumulated_crank_period: u16::from_le_bytes([data[4], data[5]]),
            accumulated_torque: u16::from_le_bytes([data[6], data[7]]),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::Power
    }
}

/// Torque effectiveness and pedal smoothness page (Page 0x13 / 19)
#[derive(Debug, Clone)]
pub struct TorqueEffectivenessPage {
    /// Update event count
    pub event_count: u8,
    /// Left torque effectiveness (0-100%)
    pub left_torque_effectiveness: Option<f32>,
    /// Right torque effectiveness (0-100%)
    pub right_torque_effectiveness: Option<f32>,
    /// Left pedal smoothness (0-100%)
    pub left_pedal_smoothness: Option<f32>,
    /// Right pedal smoothness (0-100%)
    pub right_pedal_smoothness: Option<f32>,
}

impl AntProfilePage for TorqueEffectivenessPage {
    fn page_number(&self) -> u8 {
        0x13
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x13 {
            return None;
        }

        let parse_percent = |b: u8| -> Option<f32> {
            if b == 0xFF {
                None
            } else {
                Some(b as f32 / 2.0)
            }
        };

        Some(Self {
            event_count: data[1],
            left_torque_effectiveness: parse_percent(data[2]),
            right_torque_effectiveness: parse_percent(data[3]),
            left_pedal_smoothness: parse_percent(data[4]),
            right_pedal_smoothness: parse_percent(data[5]),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::Power
    }
}

/// Crank torque frequency page (Page 0x20 / 32)
#[derive(Debug, Clone)]
pub struct CrankTorqueFrequencyPage {
    /// Update event count
    pub event_count: u8,
    /// Slope (Nm/Hz)
    pub slope: u16,
    /// Time stamp (1/2000 second)
    pub time_stamp: u16,
    /// Torque ticks since last message
    pub torque_ticks: u16,
}

impl AntProfilePage for CrankTorqueFrequencyPage {
    fn page_number(&self) -> u8 {
        0x20
    }

    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x20 {
            return None;
        }

        Some(Self {
            event_count: data[1],
            slope: u16::from_le_bytes([data[2], data[3]]),
            time_stamp: u16::from_le_bytes([data[4], data[5]]),
            torque_ticks: u16::from_le_bytes([data[6], data[7]]),
        })
    }

    fn device_type() -> AntDeviceType {
        AntDeviceType::Power
    }
}

/// Power meter state for accumulating data
#[derive(Debug, Default)]
pub struct PowerMeterState {
    last_event_count: u8,
    last_accumulated_power: u16,
    average_power_samples: Vec<u16>,
}

impl PowerMeterState {
    /// Update state with new power page and calculate average power
    pub fn update(&mut self, page: &PowerOnlyPage) -> Option<u16> {
        // Detect rollover
        let event_delta = page.event_count.wrapping_sub(self.last_event_count);

        if event_delta == 0 {
            return None; // No new data
        }

        let power_delta = page
            .accumulated_power
            .wrapping_sub(self.last_accumulated_power);
        let avg_power = power_delta / event_delta as u16;

        self.last_event_count = page.event_count;
        self.last_accumulated_power = page.accumulated_power;

        // Keep recent samples for smoothing
        self.average_power_samples.push(avg_power);
        if self.average_power_samples.len() > 10 {
            self.average_power_samples.remove(0);
        }

        Some(avg_power)
    }

    /// Get smoothed power value
    pub fn smoothed_power(&self) -> Option<u16> {
        if self.average_power_samples.is_empty() {
            return None;
        }

        let sum: u32 = self.average_power_samples.iter().map(|&p| p as u32).sum();
        Some((sum / self.average_power_samples.len() as u32) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_only_page_parse() {
        // Example power-only data page
        let data = [0x10, 0x32, 0x5A, 0x0A, 0xE8, 0x03, 0x64, 0x00];
        let page = PowerOnlyPage::parse(&data).unwrap();

        assert_eq!(page.event_count, 10);
        assert!(page.pedal_balance.is_some());
        assert_eq!(page.pedal_balance.unwrap().right_percent, 50);
        assert_eq!(page.cadence, Some(90));
        assert_eq!(page.accumulated_power, 1000);
        assert_eq!(page.instantaneous_power, 100);
    }

    #[test]
    fn test_power_only_page_no_balance() {
        let data = [0x10, 0xFF, 0xFF, 0x0A, 0xE8, 0x03, 0xC8, 0x00];
        let page = PowerOnlyPage::parse(&data).unwrap();

        assert!(page.pedal_balance.is_none());
        assert!(page.cadence.is_none());
        assert_eq!(page.instantaneous_power, 200);
    }

    #[test]
    fn test_torque_effectiveness_parse() {
        let data = [0x13, 0x01, 0x64, 0x64, 0x50, 0x50, 0x00, 0x00];
        let page = TorqueEffectivenessPage::parse(&data).unwrap();

        assert_eq!(page.left_torque_effectiveness, Some(50.0));
        assert_eq!(page.right_torque_effectiveness, Some(50.0));
        assert_eq!(page.left_pedal_smoothness, Some(40.0));
        assert_eq!(page.right_pedal_smoothness, Some(40.0));
    }

    #[test]
    fn test_power_meter_state() {
        let mut state = PowerMeterState::default();

        let page1 = PowerOnlyPage {
            event_count: 1,
            pedal_balance: None,
            cadence: Some(90),
            accumulated_power: 200,
            instantaneous_power: 200,
        };
        state.update(&page1);

        let page2 = PowerOnlyPage {
            event_count: 2,
            pedal_balance: None,
            cadence: Some(90),
            accumulated_power: 400,
            instantaneous_power: 200,
        };
        let avg = state.update(&page2);
        assert_eq!(avg, Some(200));
    }
}
