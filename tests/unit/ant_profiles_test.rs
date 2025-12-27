//! Unit tests for ANT+ profile parsers.
//!
//! Tests cover heart rate, power meter, and FE-C (trainer) profiles.

use rustride::sensors::ant::profiles::fec::{
    commands, EquipmentType, GeneralFePage, TargetPowerLimit, TrainerDataPage,
};
use rustride::sensors::ant::profiles::hr::{HeartRateData, HeartRatePage4, HeartRateState};
use rustride::sensors::ant::profiles::power::{
    PowerMeterState, PowerOnlyPage, TorqueEffectivenessPage,
};
use rustride::sensors::ant::profiles::{
    AntProfilePage, BatteryStatus, BatteryStatusValue, ManufacturerInfo, ProductInfo,
};

// =============================================================================
// Heart Rate Profile Tests
// =============================================================================

#[test]
fn test_hr_legacy_page_parse() {
    // Legacy page 0 format (no page number in first byte)
    let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x0A, 0x48];
    let hr = HeartRateData::parse(&data).unwrap();

    assert_eq!(hr.page_number, 0);
    assert!(!hr.page_change_toggle);
    assert_eq!(hr.heart_rate, 72); // 0x48 = 72 BPM
    assert_eq!(hr.beat_count, 10);
}

#[test]
fn test_hr_page_toggle_bit() {
    // Page with toggle bit set (0x80)
    let data = [0x84, 0x01, 0x12, 0x34, 0xE8, 0x03, 0x0A, 0x3C];
    let hr = HeartRateData::parse(&data).unwrap();

    assert_eq!(hr.page_number, 4);
    assert!(hr.page_change_toggle);
    assert_eq!(hr.heart_rate, 60);
}

#[test]
fn test_hr_page4_manufacturer_info() {
    // Page 4 with manufacturer info
    let data = [0x04, 0x01, 0xAB, 0xCD, 0x00, 0x04, 0x15, 0x50];
    let page = HeartRatePage4::parse(&data).unwrap();

    assert_eq!(page.manufacturer_id, 1); // Garmin
    assert_eq!(page.serial_number, 0xCDAB);
    assert_eq!(page.heart_rate, 80);
    assert_eq!(page.beat_count, 21);
}

#[test]
fn test_hr_state_rr_interval_calculation() {
    let mut state = HeartRateState::default();

    // First beat at 1 second
    let data1 = HeartRateData {
        page_number: 0,
        page_change_toggle: false,
        beat_time: 1024, // 1 second in 1/1024 units
        beat_count: 1,
        heart_rate: 60,
    };
    let update1 = state.update(&data1).unwrap();
    assert_eq!(update1.heart_rate, 60);

    // Second beat at 2 seconds (1000ms RR interval = 60 BPM)
    let data2 = HeartRateData {
        page_number: 0,
        page_change_toggle: false,
        beat_time: 2048, // 2 seconds
        beat_count: 2,
        heart_rate: 60,
    };
    let update2 = state.update(&data2).unwrap();

    assert_eq!(update2.heart_rate, 60);
    assert!(update2.rr_interval_ms.is_some());
    assert_eq!(update2.rr_interval_ms.unwrap(), 1000);
}

#[test]
fn test_hr_state_no_new_beat() {
    let mut state = HeartRateState::default();

    let data = HeartRateData {
        page_number: 0,
        page_change_toggle: false,
        beat_time: 1024,
        beat_count: 5,
        heart_rate: 75,
    };
    state.update(&data);

    // Same beat count means no new beat
    let data2 = HeartRateData {
        page_number: 0,
        page_change_toggle: true, // Toggle changed but count same
        beat_time: 1100,
        beat_count: 5,
        heart_rate: 75,
    };
    let update = state.update(&data2).unwrap();

    assert_eq!(update.heart_rate, 75);
    assert!(update.rr_interval_ms.is_none()); // No new RR interval
}

#[test]
fn test_hr_rmssd_calculation() {
    let mut state = HeartRateState::default();

    // Simulate several beats with varying intervals
    let intervals = vec![800, 850, 790, 820, 810, 800, 830, 780];
    for (i, &interval) in intervals.iter().enumerate() {
        let beat_time = intervals[..=i]
            .iter()
            .map(|&x| (x * 1024 / 1000) as u16)
            .sum();
        let data = HeartRateData {
            page_number: 0,
            page_change_toggle: i % 2 == 0,
            beat_time,
            beat_count: (i + 1) as u8,
            heart_rate: 75,
        };
        state.update(&data);
    }

    let rmssd = state.calculate_rmssd();
    assert!(rmssd.is_some());
    let rmssd_value = rmssd.unwrap();
    assert!(rmssd_value > 0.0);
    assert!(rmssd_value < 100.0); // Reasonable HRV range
}

// =============================================================================
// Power Meter Profile Tests
// =============================================================================

#[test]
fn test_power_only_page_complete_data() {
    let data = [0x10, 0x32, 0x5A, 0x0A, 0xE8, 0x03, 0x64, 0x00];
    let page = PowerOnlyPage::parse(&data).unwrap();

    assert_eq!(page.event_count, 10);
    assert!(page.pedal_balance.is_some());
    let balance = page.pedal_balance.unwrap();
    assert_eq!(balance.right_percent, 50); // 0x32 & 0x7F = 50
    assert_eq!(page.cadence, Some(90)); // 0x5A = 90
    assert_eq!(page.accumulated_power, 1000); // 0x03E8
    assert_eq!(page.instantaneous_power, 100); // 0x0064
}

#[test]
fn test_power_only_page_invalid_balance() {
    let data = [0x10, 0xFF, 0x5A, 0x0A, 0xE8, 0x03, 0xC8, 0x00];
    let page = PowerOnlyPage::parse(&data).unwrap();

    assert!(page.pedal_balance.is_none());
    assert_eq!(page.cadence, Some(90));
    assert_eq!(page.instantaneous_power, 200);
}

#[test]
fn test_power_only_page_no_cadence() {
    let data = [0x10, 0x32, 0xFF, 0x0A, 0xE8, 0x03, 0x2C, 0x01];
    let page = PowerOnlyPage::parse(&data).unwrap();

    assert!(page.cadence.is_none());
    assert_eq!(page.instantaneous_power, 300);
}

#[test]
fn test_power_meter_state_average_calculation() {
    let mut state = PowerMeterState::default();

    // First page
    let page1 = PowerOnlyPage {
        event_count: 1,
        pedal_balance: None,
        cadence: Some(90),
        accumulated_power: 200,
        instantaneous_power: 200,
    };
    state.update(&page1);

    // Second page with more accumulated power
    let page2 = PowerOnlyPage {
        event_count: 2,
        pedal_balance: None,
        cadence: Some(90),
        accumulated_power: 400,
        instantaneous_power: 200,
    };
    let avg = state.update(&page2);

    assert_eq!(avg, Some(200)); // (400 - 200) / 1 = 200
}

#[test]
fn test_power_meter_state_rollover() {
    let mut state = PowerMeterState::default();

    // Near rollover
    let page1 = PowerOnlyPage {
        event_count: 254,
        pedal_balance: None,
        cadence: Some(90),
        accumulated_power: 65500,
        instantaneous_power: 250,
    };
    state.update(&page1);

    // After rollover
    let page2 = PowerOnlyPage {
        event_count: 1, // Rolled over from 255 to 0 to 1
        pedal_balance: None,
        cadence: Some(90),
        accumulated_power: 250, // Rolled over and added 250
        instantaneous_power: 250,
    };

    // This should handle the rollover correctly
    let avg = state.update(&page2);
    assert!(avg.is_some());
}

#[test]
fn test_torque_effectiveness_page() {
    let data = [0x13, 0x01, 0x64, 0x64, 0x50, 0x50, 0x00, 0x00];
    let page = TorqueEffectivenessPage::parse(&data).unwrap();

    assert_eq!(page.left_torque_effectiveness, Some(50.0)); // 0x64 / 2 = 50
    assert_eq!(page.right_torque_effectiveness, Some(50.0));
    assert_eq!(page.left_pedal_smoothness, Some(40.0)); // 0x50 / 2 = 40
    assert_eq!(page.right_pedal_smoothness, Some(40.0));
}

#[test]
fn test_torque_effectiveness_page_invalid_data() {
    let data = [0x13, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00];
    let page = TorqueEffectivenessPage::parse(&data).unwrap();

    assert!(page.left_torque_effectiveness.is_none());
    assert!(page.right_torque_effectiveness.is_none());
    assert!(page.left_pedal_smoothness.is_none());
    assert!(page.right_pedal_smoothness.is_none());
}

// =============================================================================
// FE-C (Smart Trainer) Profile Tests
// =============================================================================

#[test]
fn test_general_fe_page_trainer() {
    let data = [0x10, 25, 120, 50, 0x88, 0x13, 0xFF, 0x30];
    let page = GeneralFePage::parse(&data).unwrap();

    assert!(matches!(page.equipment_type, EquipmentType::Trainer));
    assert_eq!(page.elapsed_time, 120);
    assert_eq!(page.distance, 50);
    assert_eq!(page.speed, 5000); // 5.0 m/s = 18 km/h
    assert!(page.heart_rate.is_none());
}

#[test]
fn test_general_fe_page_with_heart_rate() {
    let data = [0x10, 25, 100, 25, 0x50, 0x0A, 0x4B, 0x20];
    let page = GeneralFePage::parse(&data).unwrap();

    assert_eq!(page.heart_rate, Some(75)); // 0x4B = 75
}

#[test]
fn test_trainer_data_page_complete() {
    let data = [0x19, 0x0A, 0x5A, 0xE8, 0x03, 0x64, 0x00, 0x00];
    let page = TrainerDataPage::parse(&data).unwrap();

    assert_eq!(page.event_count, 10);
    assert_eq!(page.cadence, Some(90));
    assert_eq!(page.accumulated_power, 1000);
    assert_eq!(page.instantaneous_power, 100);
    assert!(matches!(
        page.target_power_flags.target_power_limits,
        TargetPowerLimit::AtTarget
    ));
}

#[test]
fn test_trainer_data_page_no_cadence() {
    let data = [0x19, 0x0A, 0xFF, 0xE8, 0x03, 0x64, 0x00, 0x01];
    let page = TrainerDataPage::parse(&data).unwrap();

    assert!(page.cadence.is_none());
    assert!(matches!(
        page.target_power_flags.target_power_limits,
        TargetPowerLimit::TooLow
    ));
}

// =============================================================================
// FE-C Control Command Tests
// =============================================================================

#[test]
fn test_set_target_power_command() {
    let cmd = commands::set_target_power(200);
    assert_eq!(cmd[0], 0x31); // Page 49

    // 200W * 4 = 800 quarter watts = 0x0320
    let power = u16::from_le_bytes([cmd[6], cmd[7]]);
    assert_eq!(power, 800);
}

#[test]
fn test_set_target_power_high_value() {
    let cmd = commands::set_target_power(1000);

    // 1000W * 4 = 4000 quarter watts = 0x0FA0
    let power = u16::from_le_bytes([cmd[6], cmd[7]]);
    assert_eq!(power, 4000);
}

#[test]
fn test_set_track_resistance_flat() {
    let cmd = commands::set_track_resistance(0.0, 0.004);
    assert_eq!(cmd[0], 0x33); // Page 51

    // 0% + 200% offset = 200%, / 0.01 = 20000 = 0x4E20
    let grade = u16::from_le_bytes([cmd[5], cmd[6]]);
    assert_eq!(grade, 20000);
}

#[test]
fn test_set_track_resistance_uphill() {
    let cmd = commands::set_track_resistance(10.0, 0.004);

    // 10% + 200% offset = 210%, / 0.01 = 21000 = 0x5208
    let grade = u16::from_le_bytes([cmd[5], cmd[6]]);
    assert_eq!(grade, 21000);
}

#[test]
fn test_set_track_resistance_downhill() {
    let cmd = commands::set_track_resistance(-5.0, 0.004);

    // -5% + 200% offset = 195%, / 0.01 = 19500 = 0x4C2C
    let grade = u16::from_le_bytes([cmd[5], cmd[6]]);
    assert_eq!(grade, 19500);
}

#[test]
fn test_set_basic_resistance_command() {
    let cmd = commands::set_basic_resistance(50.0);
    assert_eq!(cmd[0], 0x30); // Page 48
    assert_eq!(cmd[7], 100); // 50% / 0.5 = 100
}

#[test]
fn test_set_basic_resistance_max() {
    let cmd = commands::set_basic_resistance(100.0);
    assert_eq!(cmd[7], 200); // 100% / 0.5 = 200
}

#[test]
fn test_set_basic_resistance_clamped() {
    let cmd = commands::set_basic_resistance(150.0);
    assert_eq!(cmd[7], 200); // Clamped to max 200
}

// =============================================================================
// Common Page Tests
// =============================================================================

#[test]
fn test_manufacturer_info_page() {
    let data = [80, 0xFF, 0xFF, 0x01, 0x20, 0x00, 0x64, 0x00];
    let info = ManufacturerInfo::parse(&data).unwrap();

    assert_eq!(info.hw_revision, 1);
    assert_eq!(info.manufacturer_id, 32); // Wahoo
    assert_eq!(info.model_number, 100);
}

#[test]
fn test_product_info_page() {
    let data = [81, 0xFF, 0x01, 0x02, 0x78, 0x56, 0x34, 0x12];
    let info = ProductInfo::parse(&data).unwrap();

    assert_eq!(info.sw_revision_supplemental, 1);
    assert_eq!(info.sw_revision_main, 2);
    assert_eq!(info.serial_number, 0x12345678);
}

#[test]
fn test_battery_status_page() {
    // Page 82 with good battery status
    let data = [82, 0xFF, 0xFF, 0x10, 0x00, 0x00, 0x80, 0x23];
    let status = BatteryStatus::parse(&data).unwrap();

    assert!(status.battery_voltage > 0.0);
    assert!(matches!(status.battery_status, BatteryStatusValue::Good));
}

#[test]
fn test_invalid_page_number() {
    // Wrong page number should return None
    let data = [0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let result = HeartRatePage4::parse(&data);
    assert!(result.is_none());
}

#[test]
fn test_too_short_data() {
    // Data too short should return None
    let data = [0x10, 0x00, 0x00];
    let result = PowerOnlyPage::parse(&data);
    assert!(result.is_none());
}
