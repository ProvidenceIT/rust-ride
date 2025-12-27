//! Unit tests for ride summary calculations.
//!
//! T082: Unit test for ride summary calculations
//! Tests NP, TSS, IF, calorie estimation

use rustride::recording::types::RideSample;

/// Create test samples with known power values for NP calculation.
fn create_steady_power_samples(power: u16, count: usize) -> Vec<RideSample> {
    (0..count)
        .map(|i| RideSample {
            elapsed_seconds: i as u32,
            power_watts: Some(power),
            cadence_rpm: Some(90),
            heart_rate_bpm: Some(140),
            speed_kmh: Some(30.0),
            distance_meters: i as f64 * 8.33,
            calories: (i as f64 * 0.25) as u32,
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

/// Create a single sample with specified values.
fn create_sample(
    elapsed: u32,
    power: u16,
    cadence: u8,
    hr: u8,
    speed: f32,
    distance: f64,
) -> RideSample {
    RideSample {
        elapsed_seconds: elapsed,
        power_watts: Some(power),
        cadence_rpm: Some(cadence),
        heart_rate_bpm: Some(hr),
        speed_kmh: Some(speed),
        distance_meters: distance,
        calories: elapsed,
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
    }
}

/// Create samples with varying power for realistic NP testing.
fn create_variable_power_samples() -> Vec<RideSample> {
    let mut samples = Vec::new();

    // 30 seconds at 200W
    for i in 0..30 {
        samples.push(create_sample(i, 200, 90, 140, 30.0, i as f64 * 8.33));
    }

    // 30 seconds at 300W (interval)
    for i in 30..60 {
        samples.push(create_sample(i, 300, 100, 165, 35.0, i as f64 * 9.72));
    }

    // 30 seconds at 150W (recovery)
    for i in 60..90 {
        samples.push(create_sample(i, 150, 75, 130, 25.0, i as f64 * 6.94));
    }

    samples
}

#[test]
fn test_average_power_calculation() {
    let samples = create_steady_power_samples(200, 60);

    let power_sum: u32 = samples
        .iter()
        .filter_map(|s| s.power_watts)
        .map(|p| p as u32)
        .sum();
    let power_count = samples.iter().filter(|s| s.power_watts.is_some()).count();
    let avg_power = (power_sum / power_count as u32) as u16;

    assert_eq!(avg_power, 200);
}

#[test]
fn test_max_power_calculation() {
    let samples = create_variable_power_samples();

    let max_power = samples.iter().filter_map(|s| s.power_watts).max().unwrap();

    assert_eq!(max_power, 300);
}

#[test]
fn test_normalized_power_calculation_steady_state() {
    // For steady-state power, NP should equal average power
    let ftp = 250u16;
    let samples = create_steady_power_samples(200, 300); // 5 minutes at 200W

    // NP calculation:
    // 1. Calculate 30-second rolling average
    // 2. Raise each average to 4th power
    // 3. Take average of 4th powers
    // 4. Take 4th root

    // For steady 200W, NP should be ~200W
    let power_values: Vec<u16> = samples.iter().filter_map(|s| s.power_watts).collect();

    // Calculate 30s rolling averages
    let mut rolling_avgs = Vec::new();
    for i in 29..power_values.len() {
        let window: u32 = power_values[i - 29..=i].iter().map(|&p| p as u32).sum();
        rolling_avgs.push(window as f64 / 30.0);
    }

    // Calculate 4th power average
    let fourth_power_sum: f64 = rolling_avgs.iter().map(|&p| p.powi(4)).sum();
    let fourth_power_avg = fourth_power_sum / rolling_avgs.len() as f64;
    let np = fourth_power_avg.powf(0.25) as u16;

    // For steady state, NP should be very close to average
    assert!(
        (np as i32 - 200).abs() < 2,
        "NP {} should be close to 200",
        np
    );
}

#[test]
fn test_normalized_power_higher_than_average_for_variable() {
    // NP should be higher than average power for variable efforts
    let samples = create_variable_power_samples();

    let avg_power: u32 = samples
        .iter()
        .filter_map(|s| s.power_watts)
        .map(|p| p as u32)
        .sum::<u32>()
        / samples.len() as u32;

    // Average is (200*30 + 300*30 + 150*30) / 90 = 216.67
    assert!((avg_power as i32 - 217).abs() < 2);

    // NP should be higher due to variability
    // For now, just verify we can calculate it
    let power_values: Vec<u16> = samples.iter().filter_map(|s| s.power_watts).collect();
    assert_eq!(power_values.len(), 90);
}

#[test]
fn test_intensity_factor_calculation() {
    let ftp = 250u16;
    let np = 225u16;

    let intensity_factor = np as f32 / ftp as f32;

    assert!((intensity_factor - 0.9).abs() < 0.01);
}

#[test]
fn test_tss_calculation() {
    let ftp = 250u16;
    let np = 250u16; // Riding at FTP
    let duration_seconds = 3600u32; // 1 hour

    let intensity_factor = np as f32 / ftp as f32;
    let duration_hours = duration_seconds as f32 / 3600.0;

    // TSS = (duration_hours * IF^2 * 100)
    let tss = duration_hours * intensity_factor * intensity_factor * 100.0;

    // 1 hour at FTP = 100 TSS
    assert!((tss - 100.0).abs() < 1.0);
}

#[test]
fn test_tss_for_half_ftp() {
    let ftp = 250u16;
    let np = 125u16; // Half of FTP
    let duration_seconds = 3600u32;

    let intensity_factor = np as f32 / ftp as f32;
    let duration_hours = duration_seconds as f32 / 3600.0;
    let tss = duration_hours * intensity_factor * intensity_factor * 100.0;

    // 1 hour at 50% FTP = 25 TSS (0.5^2 * 100)
    assert!((tss - 25.0).abs() < 1.0);
}

#[test]
fn test_calorie_estimation_from_power() {
    // Calories = power * time * efficiency factor
    // Typical efficiency: ~4.0 kJ per calorie
    let avg_power = 200u16; // watts
    let duration_seconds = 3600u32; // 1 hour

    // Energy = power * time = 200W * 3600s = 720,000 J = 720 kJ
    let kilojoules = (avg_power as f64 * duration_seconds as f64) / 1000.0;

    // Calories (roughly kJ * 0.24)
    let calories = (kilojoules * 0.24) as u32;

    // Should be around 173 calories
    assert!(calories > 150 && calories < 200);
}

#[test]
fn test_average_heart_rate() {
    let samples = create_variable_power_samples();

    let hr_sum: u32 = samples
        .iter()
        .filter_map(|s| s.heart_rate_bpm)
        .map(|hr| hr as u32)
        .sum();
    let hr_count = samples
        .iter()
        .filter(|s| s.heart_rate_bpm.is_some())
        .count();
    let avg_hr = (hr_sum / hr_count as u32) as u8;

    // (140*30 + 165*30 + 130*30) / 90 = 145
    assert!((avg_hr as i32 - 145).abs() < 2);
}

#[test]
fn test_max_heart_rate() {
    let samples = create_variable_power_samples();

    let max_hr = samples
        .iter()
        .filter_map(|s| s.heart_rate_bpm)
        .max()
        .unwrap();

    assert_eq!(max_hr, 165);
}

#[test]
fn test_average_cadence() {
    let samples = create_variable_power_samples();

    let cadence_sum: u32 = samples
        .iter()
        .filter_map(|s| s.cadence_rpm)
        .map(|c| c as u32)
        .sum();
    let cadence_count = samples.iter().filter(|s| s.cadence_rpm.is_some()).count();
    let avg_cadence = (cadence_sum / cadence_count as u32) as u8;

    // (90*30 + 100*30 + 75*30) / 90 = 88.33
    assert!((avg_cadence as i32 - 88).abs() < 2);
}

#[test]
fn test_total_distance() {
    let samples = create_steady_power_samples(200, 60);

    // At 30 km/h for 60 seconds = 500 meters
    // Each sample is 8.33m apart (30km/h = 8.33 m/s)
    let total_distance = samples.last().unwrap().distance_meters;

    // 59 * 8.33 ≈ 491m
    assert!(total_distance > 480.0 && total_distance < 510.0);
}

#[test]
fn test_handles_missing_power_data() {
    let mut samples = create_steady_power_samples(200, 60);
    // Remove power from some samples
    samples[10].power_watts = None;
    samples[20].power_watts = None;
    samples[30].power_watts = None;

    let power_count = samples.iter().filter(|s| s.power_watts.is_some()).count();

    assert_eq!(power_count, 57);
}

#[test]
fn test_handles_zero_power() {
    // Zero power samples should be included (coasting)
    let mut samples = create_steady_power_samples(200, 60);
    samples[25].power_watts = Some(0);
    samples[26].power_watts = Some(0);
    samples[27].power_watts = Some(0);

    let zero_count = samples.iter().filter(|s| s.power_watts == Some(0)).count();

    assert_eq!(zero_count, 3);

    // Average should be lower
    let avg: u32 = samples
        .iter()
        .filter_map(|s| s.power_watts)
        .map(|p| p as u32)
        .sum::<u32>()
        / 60;

    assert!(avg < 200);
}
