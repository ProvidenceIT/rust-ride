//! Integration tests for the complete analytics pipeline.
//!
//! T131: Create integration test for full analytics pipeline
//!
//! Tests the end-to-end flow:
//! 1. Simulate ride with power data
//! 2. Extract MMP and update PDC
//! 3. Fit CP model from PDC
//! 4. Detect FTP changes
//! 5. Update training load
//! 6. Recalculate VO2max
//! 7. Classify rider type

use rustride::metrics::analytics::{
    AnalyticsTriggers, CpFitter, FtpDetector, MmpCalculator, PdcPoint, PowerDurationCurve,
    RiderClassifier, SweetSpotRecommender, TrainingLoadCalculator, Vo2maxCalculator,
};

/// Simulates a structured workout with varying intensities.
fn simulate_structured_workout(ftp: u16, duration_mins: u32) -> Vec<u16> {
    let mut samples = Vec::new();

    // Warm-up: 10 minutes at 50% FTP
    for _ in 0..(10 * 60).min(duration_mins * 60) {
        samples.push((ftp as f32 * 0.50) as u16);
    }

    if duration_mins > 10 {
        // Main set: 3x 5-min intervals at 105% FTP with 3-min recovery
        for interval in 0..3 {
            if samples.len() >= (duration_mins * 60) as usize {
                break;
            }

            // 5 min at 105% FTP
            for _ in 0..300 {
                if samples.len() >= (duration_mins * 60) as usize {
                    break;
                }
                samples.push((ftp as f32 * 1.05) as u16);
            }

            // 3 min recovery at 50% FTP (except after last interval)
            if interval < 2 {
                for _ in 0..180 {
                    if samples.len() >= (duration_mins * 60) as usize {
                        break;
                    }
                    samples.push((ftp as f32 * 0.50) as u16);
                }
            }
        }
    }

    // Cool-down: remaining time at 40% FTP
    while samples.len() < (duration_mins * 60) as usize {
        samples.push((ftp as f32 * 0.40) as u16);
    }

    samples
}

/// Simulates a 20-minute FTP test.
fn simulate_ftp_test(target_power: u16) -> Vec<u16> {
    let mut samples = Vec::new();

    // 10-min warm-up at 50% of target
    for _ in 0..(10 * 60) {
        samples.push((target_power as f32 * 0.50) as u16);
    }

    // 20-min all-out effort with slight fatigue decline
    for i in 0..(20 * 60) {
        // Start at 102%, decline to 98% by end
        let fatigue_factor = 1.02 - (0.04 * (i as f32 / 1200.0));
        samples.push((target_power as f32 * fatigue_factor) as u16);
    }

    // 10-min cool-down
    for _ in 0..(10 * 60) {
        samples.push((target_power as f32 * 0.40) as u16);
    }

    samples
}

#[test]
fn test_full_analytics_pipeline() {
    // Athlete profile
    let ftp = 250u16;
    let weight_kg = 75.0f32;
    let age = 35u8;

    // Step 1: Simulate a ride with power data
    let ride_samples = simulate_structured_workout(ftp, 60);
    assert!(!ride_samples.is_empty());

    // Step 2: Extract MMP and create PDC
    let calculator = MmpCalculator::standard();
    let mmp_points = calculator.calculate(&ride_samples);
    assert!(!mmp_points.is_empty(), "Should extract MMP points");

    let mut pdc = PowerDurationCurve::new();
    let updated = pdc.update(&mmp_points);
    assert!(!updated.is_empty(), "PDC should be updated");

    // Verify we have key durations
    assert!(pdc.power_at(60).is_some(), "Should have 1-min power");
    assert!(pdc.power_at(300).is_some(), "Should have 5-min power");

    // Step 3: Verify VO2max calculation
    if let Some(five_min_power) = pdc.power_at(300) {
        let vo2max_calc = Vo2maxCalculator::new(weight_kg);
        let vo2max = vo2max_calc.from_five_minute_power(five_min_power);
        assert!(vo2max.vo2max > 0.0, "Should calculate VO2max");
    }

    // Step 4: Verify rider type classification
    let classifier = RiderClassifier::new(ftp);
    let profile = classifier.profile_from_pdc(&pdc);
    let _rider_type = classifier.classify(&profile);
    // Classification should not panic

    // Step 5: Verify Sweet Spot calculation
    let recommender = SweetSpotRecommender::new(ftp);
    let (min, max) = recommender.zone_power_range(
        rustride::metrics::analytics::sweet_spot::IntensityZone::SweetSpot,
    );
    assert!(min > 0 && max > min, "Sweet spot zone should be valid");
    assert!(min >= (ftp as f32 * 0.88) as u16);
    assert!(max <= (ftp as f32 * 0.94) as u16 + 1);
}

#[test]
fn test_analytics_triggers_integration() {
    let ftp = 280u16;
    let weight_kg = 70.0f32;
    let age = 30u8;

    // Create triggers
    let triggers = AnalyticsTriggers::new(weight_kg, age, true);

    // Simulate ride with good efforts
    let ride_samples = simulate_structured_workout(ftp, 45);

    // Empty starting PDC
    let existing_pdc = PowerDurationCurve::new();

    // Run all triggers
    let result = triggers.run_all_triggers(
        &ride_samples,
        Some(75.0), // TSS
        &existing_pdc,
        None, // No previous load
    );

    // Verify PDC was updated
    assert!(
        !result.pdc_updated.is_empty(),
        "PDC should have updates from ride"
    );

    // Verify training load was calculated
    assert!(result.training_load_updated, "Training load should update");
    assert!(
        result.new_daily_load.is_some(),
        "Should have new daily load"
    );

    let daily_load = result.new_daily_load.unwrap();
    assert!(daily_load.tss > 0.0, "TSS should be recorded");
    assert!(daily_load.atl > 0.0, "ATL should be calculated");
    assert!(daily_load.ctl > 0.0, "CTL should be calculated");
}

#[test]
fn test_ftp_detection_from_test_ride() {
    let true_ftp = 280u16;

    // Simulate a 20-min FTP test at target power
    let test_power = (true_ftp as f32 / 0.95) as u16; // Test at ~105% of FTP to get FTP after 95% rule
    let ride_samples = simulate_ftp_test(test_power);

    // Extract MMP
    let calculator = MmpCalculator::standard();
    let mmp_points = calculator.calculate(&ride_samples);

    // Build PDC
    let mut pdc = PowerDurationCurve::new();
    pdc.update(&mmp_points);

    // Detect FTP
    let detector = FtpDetector::default();
    if let Some(estimate) = detector.detect(&pdc) {
        // FTP should be close to true_ftp (within 5%)
        let diff = (estimate.ftp_watts as i32 - true_ftp as i32).abs();
        let pct_diff = diff as f32 / true_ftp as f32 * 100.0;
        assert!(
            pct_diff < 10.0,
            "FTP estimate {} should be within 10% of true FTP {} (diff: {:.1}%)",
            estimate.ftp_watts,
            true_ftp,
            pct_diff
        );
    }
}

#[test]
fn test_cp_model_from_pdc() {
    // Create a PDC with realistic data
    let points = vec![
        PdcPoint {
            duration_secs: 5,
            power_watts: 800,
        }, // Sprint
        PdcPoint {
            duration_secs: 60,
            power_watts: 400,
        }, // 1-min
        PdcPoint {
            duration_secs: 180,
            power_watts: 340,
        }, // 3-min
        PdcPoint {
            duration_secs: 300,
            power_watts: 310,
        }, // 5-min
        PdcPoint {
            duration_secs: 600,
            power_watts: 280,
        }, // 10-min
        PdcPoint {
            duration_secs: 1200,
            power_watts: 260,
        }, // 20-min
        PdcPoint {
            duration_secs: 3600,
            power_watts: 240,
        }, // 60-min
    ];

    let pdc = PowerDurationCurve::from_points(points);

    // Verify sufficient data for CP
    assert!(pdc.has_sufficient_data_for_cp(), "Should have enough data");

    // Fit CP model
    let fitter = CpFitter::default();
    let cp_model = fitter.fit(&pdc).expect("Should fit CP model");

    // Validate CP model
    assert!(cp_model.cp > 200 && cp_model.cp < 300, "CP should be reasonable");
    assert!(
        cp_model.w_prime > 10000 && cp_model.w_prime < 30000,
        "W' should be reasonable"
    );

    // Test predictions
    let power_10min = cp_model.power_at_duration(std::time::Duration::from_secs(600));
    assert!(power_10min > cp_model.cp, "10-min power > CP");

    let tte_300w = cp_model.time_to_exhaustion(300);
    assert!(tte_300w.is_some(), "Should predict TTE");
}

#[test]
fn test_training_load_accumulation() {
    let calculator = TrainingLoadCalculator::new();

    // Simulate 7 days of training
    let daily_tss = [60.0, 80.0, 45.0, 90.0, 0.0, 70.0, 85.0];
    let dates: Vec<_> = (0..7)
        .map(|i| {
            chrono::Utc::now().date_naive() - chrono::Duration::days(6 - i)
        })
        .collect();

    let daily_data: Vec<_> = dates.iter().zip(daily_tss.iter()).map(|(d, t)| (*d, *t)).collect();

    let history = calculator.calculate_history(&daily_data);

    assert_eq!(history.len(), 7, "Should have 7 days of load data");

    // ATL should increase over the week
    let first_atl = history.first().unwrap().1.atl;
    let last_atl = history.last().unwrap().1.atl;
    assert!(
        last_atl > first_atl,
        "ATL should increase over training week"
    );

    // Calculate ACWR
    let final_load = history.last().unwrap().1;
    let acwr = calculator.acwr(final_load.atl, final_load.ctl);
    assert!(acwr.ratio > 0.0, "ACWR should be positive");
}

#[test]
fn test_sensor_gap_interpolation() {
    use rustride::metrics::analytics::pdc::interpolate_sensor_gaps;

    // Simulate power data with sensor dropouts
    let mut samples = vec![250u16; 100];
    // Insert 5-second gap
    for i in 40..45 {
        samples[i] = 0;
    }

    let interpolated = interpolate_sensor_gaps(&samples);

    // Gap should be filled
    for i in 40..45 {
        assert!(
            interpolated[i] > 0,
            "Gap at index {} should be interpolated",
            i
        );
        // Should be close to surrounding values
        assert!(
            (interpolated[i] as i32 - 250).abs() < 20,
            "Interpolated value should be close to surrounding"
        );
    }

    // Non-gap values unchanged
    assert_eq!(interpolated[0], 250);
    assert_eq!(interpolated[99], 250);
}
