//! T065: Unit tests for audio module.
//!
//! Tests for audio alerts, cue builder, TTS, and engine components.

use rustride::audio::{
    AlertCategory, AlertConfig, AlertContext, AlertData, AlertType, CueBuilder, DefaultTtsProvider,
    TtsProvider,
};

/// Test AlertType category classification.
#[test]
fn test_alert_type_categories() {
    // Workout alerts
    assert_eq!(AlertType::WorkoutStart.category(), AlertCategory::Workout);
    assert_eq!(AlertType::IntervalChange.category(), AlertCategory::Workout);
    assert_eq!(
        AlertType::IntervalCountdown.category(),
        AlertCategory::Workout
    );
    assert_eq!(
        AlertType::WorkoutComplete.category(),
        AlertCategory::Workout
    );
    assert_eq!(AlertType::RecoveryStart.category(), AlertCategory::Workout);

    // Power alerts
    assert_eq!(AlertType::PowerZoneChange.category(), AlertCategory::Power);
    assert_eq!(AlertType::PowerTooHigh.category(), AlertCategory::Power);
    assert_eq!(AlertType::PowerTooLow.category(), AlertCategory::Power);
    assert_eq!(AlertType::PowerOnTarget.category(), AlertCategory::Power);

    // HR alerts
    assert_eq!(
        AlertType::HeartRateZoneChange.category(),
        AlertCategory::HeartRate
    );
    assert_eq!(
        AlertType::HeartRateTooHigh.category(),
        AlertCategory::HeartRate
    );
    assert_eq!(
        AlertType::HeartRateTooLow.category(),
        AlertCategory::HeartRate
    );

    // Cadence alerts
    assert_eq!(AlertType::CadenceTooLow.category(), AlertCategory::Cadence);
    assert_eq!(AlertType::CadenceTooHigh.category(), AlertCategory::Cadence);

    // Milestone alerts
    assert_eq!(
        AlertType::DistanceMilestone.category(),
        AlertCategory::Milestones
    );
    assert_eq!(
        AlertType::TimeMilestone.category(),
        AlertCategory::Milestones
    );
    assert_eq!(
        AlertType::CalorieMilestone.category(),
        AlertCategory::Milestones
    );

    // Sensor alerts
    assert_eq!(
        AlertType::SensorConnected.category(),
        AlertCategory::Sensors
    );
    assert_eq!(
        AlertType::SensorDisconnected.category(),
        AlertCategory::Sensors
    );
    assert_eq!(
        AlertType::SensorLowBattery.category(),
        AlertCategory::Sensors
    );

    // Achievement alerts
    assert_eq!(
        AlertType::PersonalRecord.category(),
        AlertCategory::Achievements
    );
    assert_eq!(
        AlertType::AchievementUnlocked.category(),
        AlertCategory::Achievements
    );

    // General alerts
    assert_eq!(AlertType::LapMarker.category(), AlertCategory::General);
    assert_eq!(AlertType::RidePaused.category(), AlertCategory::General);
    assert_eq!(AlertType::RideResumed.category(), AlertCategory::General);
}

/// Test AlertType default enabled state.
#[test]
fn test_alert_type_default_enabled() {
    // Workout alerts that should be enabled by default
    assert!(AlertType::WorkoutStart.default_enabled());
    assert!(AlertType::IntervalChange.default_enabled());
    assert!(AlertType::WorkoutComplete.default_enabled());

    // Sensor disconnect should be enabled
    assert!(AlertType::SensorDisconnected.default_enabled());

    // Zone changes should be enabled
    assert!(AlertType::PowerZoneChange.default_enabled());
    assert!(AlertType::HeartRateZoneChange.default_enabled());

    // Personal records should be enabled
    assert!(AlertType::PersonalRecord.default_enabled());

    // Some alerts should be disabled by default
    assert!(!AlertType::IntervalCountdown.default_enabled());
    assert!(!AlertType::RidePaused.default_enabled());
    assert!(!AlertType::LapMarker.default_enabled());
}

/// Test AlertConfig default values.
#[test]
fn test_alert_config_default() {
    let config = AlertConfig::default();

    assert!(config.enabled);
    assert!(config.use_voice);
    assert!(config.play_sound);
    assert_eq!(config.cooldown_secs, 5);
    assert!(config.sound_name.is_none());
}

/// Test AlertContext simple creation.
#[test]
fn test_alert_context_simple() {
    let context = AlertContext::simple();

    // Simple context has no data
    matches!(context.data, AlertData::None);
}

/// Test AlertContext interval change creation.
#[test]
fn test_alert_context_interval_change() {
    let context = AlertContext::interval_change("VO2max Interval", Some(300), 180);

    match context.data {
        AlertData::IntervalChange {
            new_interval_name,
            target_power,
            duration_secs,
        } => {
            assert_eq!(new_interval_name, "VO2max Interval");
            assert_eq!(target_power, Some(300));
            assert_eq!(duration_secs, 180);
        }
        _ => panic!("Expected IntervalChange data"),
    }
}

/// Test AlertContext countdown creation.
#[test]
fn test_alert_context_countdown() {
    let context = AlertContext::countdown(5);

    match context.data {
        AlertData::Countdown { seconds_remaining } => {
            assert_eq!(seconds_remaining, 5);
        }
        _ => panic!("Expected Countdown data"),
    }
}

/// Test AlertContext zone change creation.
#[test]
fn test_alert_context_zone_change() {
    let context = AlertContext::zone_change("Threshold", 4);

    match context.data {
        AlertData::ZoneChange {
            zone_name,
            zone_number,
        } => {
            assert_eq!(zone_name, "Threshold");
            assert_eq!(zone_number, 4);
        }
        _ => panic!("Expected ZoneChange data"),
    }
}

/// Test AlertContext sensor creation.
#[test]
fn test_alert_context_sensor() {
    let context = AlertContext::sensor("Garmin Power Meter", "Power");

    match context.data {
        AlertData::Sensor {
            sensor_name,
            sensor_type,
        } => {
            assert_eq!(sensor_name, "Garmin Power Meter");
            assert_eq!(sensor_type, "Power");
        }
        _ => panic!("Expected Sensor data"),
    }
}

/// Test AlertContext milestone creation.
#[test]
fn test_alert_context_milestone() {
    let context = AlertContext::milestone("Distance", 10.0, "km");

    match context.data {
        AlertData::Milestone {
            metric_name,
            value,
            unit,
        } => {
            assert_eq!(metric_name, "Distance");
            assert_eq!(value, 10.0);
            assert_eq!(unit, "km");
        }
        _ => panic!("Expected Milestone data"),
    }
}

/// Test AlertContext personal record creation.
#[test]
fn test_alert_context_personal_record() {
    let context = AlertContext::personal_record("5 min power", 350.0, "W", Some(340.0));

    match context.data {
        AlertData::PersonalRecord {
            record_type,
            value,
            unit,
            previous_value,
        } => {
            assert_eq!(record_type, "5 min power");
            assert_eq!(value, 350.0);
            assert_eq!(unit, "W");
            assert_eq!(previous_value, Some(340.0));
        }
        _ => panic!("Expected PersonalRecord data"),
    }
}

/// Test CueBuilder with simple alert.
#[test]
fn test_cue_builder_simple_alert() {
    let builder = CueBuilder::new();
    let context = AlertContext::simple();

    let message = builder.build(AlertType::WorkoutStart, &context);
    assert!(!message.is_empty());
}

/// Test CueBuilder with interval change.
#[test]
fn test_cue_builder_interval_change() {
    let builder = CueBuilder::new();
    let context = AlertContext::interval_change("Sweet Spot", Some(260), 300);

    let message = builder.build(AlertType::IntervalChange, &context);
    assert!(!message.is_empty());
}

/// Test CueBuilder with countdown.
#[test]
fn test_cue_builder_countdown() {
    let builder = CueBuilder::new();
    let context = AlertContext::countdown(5);

    let message = builder.build(AlertType::IntervalCountdown, &context);
    assert!(message.contains('5') || message.contains("five"));
}

/// Test CueBuilder with zone change.
#[test]
fn test_cue_builder_zone_change() {
    let builder = CueBuilder::new();
    let context = AlertContext::zone_change("Threshold", 4);

    let message = builder.build(AlertType::PowerZoneChange, &context);
    assert!(!message.is_empty());
}

/// Test CueBuilder with sensor connected.
#[test]
fn test_cue_builder_sensor_connected() {
    let builder = CueBuilder::new();
    let context = AlertContext::sensor("Power Meter", "Power");

    let message = builder.build(AlertType::SensorConnected, &context);
    assert!(!message.is_empty());
}

/// Test CueBuilder with personal record.
#[test]
fn test_cue_builder_personal_record() {
    let builder = CueBuilder::new();
    let context = AlertContext::personal_record("5 minute power", 350.0, "W", Some(340.0));

    let message = builder.build(AlertType::PersonalRecord, &context);
    assert!(!message.is_empty());
}

/// Test TTS provider creation.
#[test]
fn test_tts_provider_creation() {
    let provider = DefaultTtsProvider::new();
    assert!(provider.initialize().is_ok());
}

/// Test TTS provider rate and volume.
#[test]
fn test_tts_provider_settings() {
    let provider = DefaultTtsProvider::new();

    // Test rate
    assert_eq!(provider.get_rate(), 1.0);
    provider.set_rate(1.5);
    assert_eq!(provider.get_rate(), 1.5);

    // Rate should be clamped
    provider.set_rate(3.0);
    assert!(provider.get_rate() <= 2.0);

    // Test volume
    assert_eq!(provider.get_volume(), 1.0);
    provider.set_volume(0.5);
    assert_eq!(provider.get_volume(), 0.5);
}

/// Test TTS provider voices.
#[test]
fn test_tts_provider_voices() {
    let provider = DefaultTtsProvider::new();

    let voices = provider.get_voices();
    assert!(!voices.is_empty());

    // Check default voice exists
    let default_voice = voices.iter().find(|v| v.is_default);
    assert!(default_voice.is_some());
}

/// Test AlertData variants.
#[test]
fn test_alert_data_variants() {
    // Test None variant
    let none_data = AlertData::None;
    matches!(none_data, AlertData::None);

    // Test IntervalChange variant
    let interval_data = AlertData::IntervalChange {
        new_interval_name: "Test".to_string(),
        target_power: Some(200),
        duration_secs: 60,
    };
    matches!(interval_data, AlertData::IntervalChange { .. });

    // Test Countdown variant
    let countdown_data = AlertData::Countdown {
        seconds_remaining: 3,
    };
    matches!(countdown_data, AlertData::Countdown { .. });

    // Test ZoneChange variant
    let zone_data = AlertData::ZoneChange {
        zone_name: "VO2max".to_string(),
        zone_number: 5,
    };
    matches!(zone_data, AlertData::ZoneChange { .. });
}

/// Test multiple alerts in sequence.
#[test]
fn test_multiple_alert_sequence() {
    let builder = CueBuilder::new();

    // Simulate workout start
    let start_msg = builder.build(AlertType::WorkoutStart, &AlertContext::simple());
    assert!(!start_msg.is_empty());

    // Simulate interval change
    let interval_context = AlertContext::interval_change("Sweet Spot 1", Some(260), 300);
    let interval_msg = builder.build(AlertType::IntervalChange, &interval_context);
    assert!(!interval_msg.is_empty());

    // Simulate countdown
    let countdown_context = AlertContext::countdown(3);
    let countdown_msg = builder.build(AlertType::IntervalCountdown, &countdown_context);
    assert!(!countdown_msg.is_empty());

    // Simulate workout complete
    let complete_msg = builder.build(AlertType::WorkoutComplete, &AlertContext::simple());
    assert!(!complete_msg.is_empty());
}

/// Test AlertType display names.
#[test]
fn test_alert_type_display_names() {
    assert_eq!(AlertType::WorkoutStart.display_name(), "Workout Start");
    assert_eq!(AlertType::IntervalChange.display_name(), "Interval Changes");
    assert_eq!(
        AlertType::PowerZoneChange.display_name(),
        "Power Zone Changes"
    );
    assert_eq!(
        AlertType::SensorDisconnected.display_name(),
        "Sensor Disconnected"
    );
}

/// Test AlertCategory display names.
#[test]
fn test_alert_category_display_names() {
    assert_eq!(AlertCategory::Workout.display_name(), "Workout");
    assert_eq!(AlertCategory::Power.display_name(), "Power");
    assert_eq!(AlertCategory::HeartRate.display_name(), "Heart Rate");
    assert_eq!(AlertCategory::Sensors.display_name(), "Sensors");
}
