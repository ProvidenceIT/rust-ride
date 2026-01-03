//! Cue Templates and Message Building
//!
//! Provides templated messages for audio cues that can include dynamic data.

use super::alerts::{AlertContext, AlertData, AlertType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template for a cue message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CueTemplate {
    /// Template string with placeholders like {zone_name}, {power}, etc.
    pub template: String,
    /// Alternative templates for variety
    pub alternatives: Vec<String>,
    /// Whether to randomly select from alternatives
    pub use_random: bool,
}

impl CueTemplate {
    /// Create a simple template with no alternatives
    pub fn simple(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            alternatives: Vec::new(),
            use_random: false,
        }
    }

    /// Create a template with alternatives
    pub fn with_alternatives(template: impl Into<String>, alts: Vec<String>) -> Self {
        Self {
            template: template.into(),
            alternatives: alts,
            use_random: true,
        }
    }

    /// Get a template (potentially random)
    pub fn get_template(&self) -> &str {
        if self.use_random && !self.alternatives.is_empty() {
            // Simple random selection based on current time
            let idx = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as usize % (self.alternatives.len() + 1))
                .unwrap_or(0);

            if idx == 0 {
                &self.template
            } else {
                &self.alternatives[idx - 1]
            }
        } else {
            &self.template
        }
    }
}

/// Default templates for each alert type
pub fn default_templates() -> HashMap<AlertType, CueTemplate> {
    let mut templates = HashMap::new();

    // Workout alerts
    templates.insert(
        AlertType::WorkoutStart,
        CueTemplate::with_alternatives(
            "Starting workout".to_string(),
            vec![
                "Let's begin".to_string(),
                "Workout starting now".to_string(),
            ],
        ),
    );

    templates.insert(
        AlertType::IntervalChange,
        CueTemplate::simple("{interval_name} interval, {power}, {duration}"),
    );

    templates.insert(
        AlertType::IntervalCountdown,
        CueTemplate::simple("{countdown}"),
    );

    templates.insert(
        AlertType::WorkoutComplete,
        CueTemplate::with_alternatives(
            "Workout complete. Great job!".to_string(),
            vec![
                "Workout finished. Well done!".to_string(),
                "You did it! Workout complete.".to_string(),
            ],
        ),
    );

    templates.insert(
        AlertType::RecoveryStart,
        CueTemplate::simple("Recovery. Take it easy."),
    );

    // Motivational messages for high-intensity intervals
    templates.insert(
        AlertType::MotivationalHighIntensity,
        CueTemplate::with_alternatives(
            "You're doing great!".to_string(),
            vec![
                "Keep pushing!".to_string(),
                "Stay strong!".to_string(),
                "You've got this!".to_string(),
                "Keep it up!".to_string(),
                "Great effort!".to_string(),
                "Push through!".to_string(),
            ],
        ),
    );

    // Motivational messages for recovery intervals
    templates.insert(
        AlertType::MotivationalRecovery,
        CueTemplate::with_alternatives(
            "Nice work, catch your breath".to_string(),
            vec![
                "Great job, take it easy".to_string(),
                "Well done, recover well".to_string(),
                "Excellent effort, rest up".to_string(),
                "Good work, relax and recover".to_string(),
            ],
        ),
    );

    // Power zone alerts
    templates.insert(
        AlertType::PowerZoneChange,
        CueTemplate::simple("Zone {zone_number}, {zone_name}"),
    );

    templates.insert(
        AlertType::PowerTooHigh,
        CueTemplate::with_alternatives(
            "Power too high. Ease off.".to_string(),
            vec!["Back off a bit".to_string(), "Reduce power".to_string()],
        ),
    );

    templates.insert(
        AlertType::PowerTooLow,
        CueTemplate::with_alternatives(
            "Power too low. Push harder.".to_string(),
            vec!["Pick it up".to_string(), "More power needed".to_string()],
        ),
    );

    templates.insert(AlertType::PowerOnTarget, CueTemplate::simple("On target"));

    // Heart rate alerts
    templates.insert(
        AlertType::HeartRateZoneChange,
        CueTemplate::simple("Heart rate zone {zone_number}"),
    );

    templates.insert(
        AlertType::HeartRateTooHigh,
        CueTemplate::simple("Heart rate high. Slow down."),
    );

    templates.insert(
        AlertType::HeartRateTooLow,
        CueTemplate::simple("Heart rate low. Pick up the pace."),
    );

    // Cadence alerts
    templates.insert(AlertType::CadenceTooLow, CueTemplate::simple("Spin faster"));

    templates.insert(
        AlertType::CadenceTooHigh,
        CueTemplate::simple("Slow your cadence"),
    );

    // Milestone alerts
    templates.insert(
        AlertType::DistanceMilestone,
        CueTemplate::simple("{value} {unit}"),
    );

    templates.insert(
        AlertType::TimeMilestone,
        CueTemplate::simple("{value} minutes"),
    );

    // Sensor alerts
    templates.insert(
        AlertType::SensorConnected,
        CueTemplate::simple("{sensor_name} connected"),
    );

    templates.insert(
        AlertType::SensorDisconnected,
        CueTemplate::simple("{sensor_name} disconnected"),
    );

    templates.insert(
        AlertType::SensorLowBattery,
        CueTemplate::simple("{sensor_name} battery low"),
    );

    // Achievement alerts
    templates.insert(
        AlertType::PersonalRecord,
        CueTemplate::simple("New personal record! {record_type}, {value} {unit}"),
    );

    templates.insert(
        AlertType::AchievementUnlocked,
        CueTemplate::with_alternatives(
            "Achievement unlocked! {achievement_name}".to_string(),
            vec![
                "New achievement! {achievement_name}".to_string(),
                "You earned {achievement_name}!".to_string(),
            ],
        ),
    );

    // General alerts
    templates.insert(AlertType::LapMarker, CueTemplate::simple("Lap"));
    templates.insert(AlertType::RidePaused, CueTemplate::simple("Paused"));
    templates.insert(AlertType::RideResumed, CueTemplate::simple("Resumed"));

    templates
}

/// Builds cue messages from templates and context
pub struct CueBuilder {
    templates: HashMap<AlertType, CueTemplate>,
}

impl Default for CueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CueBuilder {
    /// Create a new cue builder with default templates
    pub fn new() -> Self {
        Self {
            templates: default_templates(),
        }
    }

    /// Set a custom template for an alert type
    pub fn set_template(&mut self, alert_type: AlertType, template: CueTemplate) {
        self.templates.insert(alert_type, template);
    }

    /// Build a message from alert type and context
    pub fn build(&self, alert_type: AlertType, context: &AlertContext) -> String {
        let template = self
            .templates
            .get(&alert_type)
            .map(|t| t.get_template())
            .unwrap_or("Alert");

        self.expand_template(template, &context.data)
    }

    /// Expand placeholders in a template
    fn expand_template(&self, template: &str, data: &AlertData) -> String {
        let mut result = template.to_string();

        match data {
            AlertData::None => {}
            AlertData::IntervalChange {
                new_interval_name,
                target_power,
                duration_secs,
            } => {
                result = result.replace("{interval_name}", new_interval_name);
                // Handle power with proper formatting for optional values
                if let Some(power) = target_power {
                    result = result.replace("{power}", &format!("{} watts", power));
                } else {
                    // Remove "{power}" and clean up any surrounding commas/spaces
                    result = result.replace(", {power}", "");
                    result = result.replace("{power}, ", "");
                    result = result.replace("{power}", "");
                }
                result = result.replace("{duration}", &format_duration(*duration_secs));
            }
            AlertData::Countdown { seconds_remaining } => {
                result = result.replace("{countdown}", &format_countdown(*seconds_remaining));
                // Also support legacy {seconds} placeholder
                result = result.replace("{seconds}", &seconds_remaining.to_string());
            }
            AlertData::ZoneChange {
                zone_name,
                zone_number,
            } => {
                result = result.replace("{zone_name}", zone_name);
                result = result.replace("{zone_number}", &zone_number.to_string());
            }
            AlertData::Milestone {
                metric_name,
                value,
                unit,
            } => {
                result = result.replace("{metric}", metric_name);
                result = result.replace("{value}", &format!("{:.0}", value));
                result = result.replace("{unit}", unit);
            }
            AlertData::Sensor {
                sensor_name,
                sensor_type,
            } => {
                result = result.replace("{sensor_name}", sensor_name);
                result = result.replace("{sensor_type}", sensor_type);
            }
            AlertData::PersonalRecord {
                record_type,
                value,
                unit,
                previous_value,
            } => {
                result = result.replace("{record_type}", record_type);
                result = result.replace("{value}", &format!("{:.1}", value));
                result = result.replace("{unit}", unit);
                if let Some(prev) = previous_value {
                    result = result.replace("{previous}", &format!("{:.1}", prev));
                }
            }
            AlertData::Achievement { achievement_name } => {
                result = result.replace("{achievement_name}", achievement_name);
            }
            AlertData::Custom { message } => {
                // For custom messages, replace the entire template with the message
                result = message.clone();
            }
        }

        result
    }
}

/// Format duration in seconds to spoken form
fn format_duration(secs: u32) -> String {
    if secs < 60 {
        format!("{} seconds", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs == 0 {
            format!("{} minutes", mins)
        } else {
            format!("{} minutes {} seconds", mins, remaining_secs)
        }
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{} hours {} minutes", hours, mins)
    }
}

/// Format countdown seconds with proper singular/plural handling
fn format_countdown(seconds: u32) -> String {
    match seconds {
        1 => "1".to_string(),
        2 | 3 => seconds.to_string(),
        _ => format!("{} seconds", seconds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cue_template_simple() {
        let template = CueTemplate::simple("Hello world");
        assert_eq!(template.get_template(), "Hello world");
    }

    #[test]
    fn test_cue_builder_interval_change() {
        let builder = CueBuilder::new();
        let context = AlertContext::interval_change("Sweet Spot", Some(260), 300);

        let message = builder.build(AlertType::IntervalChange, &context);
        // Should produce "Sweet Spot interval, 260 watts, 5 minutes"
        assert!(message.contains("Sweet Spot interval"));
        assert!(message.contains("260 watts"));
        assert!(message.contains("5 minutes"));
    }

    #[test]
    fn test_cue_builder_interval_change_without_power() {
        let builder = CueBuilder::new();
        let context = AlertContext::interval_change("Recovery", None, 120);

        let message = builder.build(AlertType::IntervalChange, &context);
        // Should produce "Recovery interval, 2 minutes" (no power)
        assert!(message.contains("Recovery interval"));
        assert!(message.contains("2 minutes"));
        // Should not have double commas or "watts"
        assert!(!message.contains("watts"));
        assert!(!message.contains(", ,"));
    }

    #[test]
    fn test_cue_builder_zone_change() {
        let builder = CueBuilder::new();
        let context = AlertContext::zone_change("Tempo", 3);

        let message = builder.build(AlertType::PowerZoneChange, &context);
        assert!(message.contains("3"));
        assert!(message.contains("Tempo"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30 seconds");
        assert_eq!(format_duration(60), "1 minutes");
        assert_eq!(format_duration(90), "1 minutes 30 seconds");
        assert_eq!(format_duration(3600), "1 hours 0 minutes");
    }

    #[test]
    fn test_interval_change_various_durations() {
        let builder = CueBuilder::new();

        // Short interval (seconds)
        let context = AlertContext::interval_change("Sprint", Some(400), 30);
        let message = builder.build(AlertType::IntervalChange, &context);
        assert!(message.contains("30 seconds"));

        // Medium interval (minutes)
        let context = AlertContext::interval_change("Tempo", Some(200), 600);
        let message = builder.build(AlertType::IntervalChange, &context);
        assert!(message.contains("10 minutes"));

        // Long interval (minutes with seconds)
        let context = AlertContext::interval_change("Endurance", Some(150), 330);
        let message = builder.build(AlertType::IntervalChange, &context);
        assert!(message.contains("5 minutes 30 seconds"));
    }

    #[test]
    fn test_interval_change_natural_sounding_message() {
        let builder = CueBuilder::new();

        // Test the exact format: "Sweet Spot interval, 260 watts, 5 minutes"
        let context = AlertContext::interval_change("Sweet Spot", Some(260), 300);
        let message = builder.build(AlertType::IntervalChange, &context);
        assert_eq!(message, "Sweet Spot interval, 260 watts, 5 minutes");

        // Test with different interval name
        let context = AlertContext::interval_change("Threshold", Some(280), 120);
        let message = builder.build(AlertType::IntervalChange, &context);
        assert_eq!(message, "Threshold interval, 280 watts, 2 minutes");
    }

    #[test]
    fn test_format_countdown() {
        // Test countdown formatting for all COUNTDOWN_THRESHOLDS [10, 5, 3, 2, 1]
        assert_eq!(format_countdown(10), "10 seconds");
        assert_eq!(format_countdown(5), "5 seconds");
        assert_eq!(format_countdown(3), "3");
        assert_eq!(format_countdown(2), "2");
        assert_eq!(format_countdown(1), "1");
    }

    #[test]
    fn test_cue_builder_countdown_announcements() {
        let builder = CueBuilder::new();

        // Test 10 seconds countdown
        let context = AlertContext::countdown(10);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(message, "10 seconds");

        // Test 5 seconds countdown
        let context = AlertContext::countdown(5);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(message, "5 seconds");

        // Test 3 seconds countdown (short form for urgency)
        let context = AlertContext::countdown(3);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(message, "3");

        // Test 2 seconds countdown (short form for urgency)
        let context = AlertContext::countdown(2);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(message, "2");

        // Test 1 second countdown (short form for urgency)
        let context = AlertContext::countdown(1);
        let message = builder.build(AlertType::IntervalCountdown, &context);
        assert_eq!(message, "1");
    }

    #[test]
    fn test_motivational_high_intensity_template_exists() {
        let templates = default_templates();
        let template = templates.get(&AlertType::MotivationalHighIntensity);
        assert!(template.is_some(), "MotivationalHighIntensity template should exist");

        let template = template.unwrap();
        // Should have alternatives for variety
        assert!(!template.alternatives.is_empty(), "Should have alternatives for variety");
        assert!(template.use_random, "Should use random selection for variety");

        // Check that the main template is a motivational message
        let main = &template.template;
        assert!(
            main.contains("great") || main.contains("pushing") || main.contains("strong"),
            "Main template should be motivational"
        );
    }

    #[test]
    fn test_motivational_recovery_template_exists() {
        let templates = default_templates();
        let template = templates.get(&AlertType::MotivationalRecovery);
        assert!(template.is_some(), "MotivationalRecovery template should exist");

        let template = template.unwrap();
        // Should have alternatives for variety
        assert!(!template.alternatives.is_empty(), "Should have alternatives for variety");
        assert!(template.use_random, "Should use random selection for variety");

        // Check that the main template is a recovery message
        let main = &template.template;
        assert!(
            main.contains("work") || main.contains("breath") || main.contains("recover"),
            "Main template should be recovery-focused"
        );
    }

    #[test]
    fn test_motivational_messages_build() {
        let builder = CueBuilder::new();
        let context = AlertContext::simple();

        // Test high intensity - should produce one of the motivational messages
        let message = builder.build(AlertType::MotivationalHighIntensity, &context);
        let high_intensity_messages = [
            "You're doing great!",
            "Keep pushing!",
            "Stay strong!",
            "You've got this!",
            "Keep it up!",
            "Great effort!",
            "Push through!",
        ];
        assert!(
            high_intensity_messages.contains(&message.as_str()),
            "Message '{}' should be one of the high-intensity motivational messages",
            message
        );

        // Test recovery - should produce one of the recovery messages
        let message = builder.build(AlertType::MotivationalRecovery, &context);
        let recovery_messages = [
            "Nice work, catch your breath",
            "Great job, take it easy",
            "Well done, recover well",
            "Excellent effort, rest up",
            "Good work, relax and recover",
        ];
        assert!(
            recovery_messages.contains(&message.as_str()),
            "Message '{}' should be one of the recovery motivational messages",
            message
        );
    }

    #[test]
    fn test_achievement_unlocked_message() {
        let builder = CueBuilder::new();
        let context = AlertContext::achievement("Century Rider");

        let message = builder.build(AlertType::AchievementUnlocked, &context);

        // Should include the achievement name
        assert!(
            message.contains("Century Rider"),
            "Message '{}' should contain the achievement name",
            message
        );

        // Should be one of the valid achievement announcement patterns
        let valid_patterns = [
            "Achievement unlocked! Century Rider",
            "New achievement! Century Rider",
            "You earned Century Rider!",
        ];
        assert!(
            valid_patterns.contains(&message.as_str()),
            "Message '{}' should be one of the valid achievement patterns",
            message
        );
    }

    #[test]
    fn test_custom_message() {
        let builder = CueBuilder::new();
        let custom_message = "Level up! You are now level 10";
        let context = AlertContext::custom(custom_message);

        // Custom message should replace the entire template
        let message = builder.build(AlertType::AchievementUnlocked, &context);
        assert_eq!(message, custom_message);
    }

    #[test]
    fn test_achievement_context_creation() {
        let context = AlertContext::achievement("First Ride");
        match context.data {
            AlertData::Achievement { achievement_name } => {
                assert_eq!(achievement_name, "First Ride");
            }
            _ => panic!("Expected Achievement data"),
        }
    }

    #[test]
    fn test_custom_context_creation() {
        let context = AlertContext::custom("Custom alert message");
        match context.data {
            AlertData::Custom { message } => {
                assert_eq!(message, "Custom alert message");
            }
            _ => panic!("Expected Custom data"),
        }
    }
}
