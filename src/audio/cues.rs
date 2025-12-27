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
        CueTemplate::simple("{interval_name}, {duration}"),
    );

    templates.insert(
        AlertType::IntervalCountdown,
        CueTemplate::simple("{seconds} seconds"),
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
        CueTemplate::simple("Achievement unlocked"),
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
                result = result.replace(
                    "{power}",
                    &target_power
                        .map(|p| format!("{} watts", p))
                        .unwrap_or_default(),
                );
                result = result.replace("{duration}", &format_duration(*duration_secs));
            }
            AlertData::Countdown { seconds_remaining } => {
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
        let context = AlertContext::interval_change("Threshold", Some(250), 300);

        let message = builder.build(AlertType::IntervalChange, &context);
        assert!(message.contains("Threshold"));
        assert!(message.contains("5 minutes"));
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
}
