//! ZWO (Zwift workout) export functionality.
//!
//! Provides functions to export Workout structs to Zwift's ZWO XML format.

use crate::workouts::types::{PowerTarget, SegmentType, Workout, WorkoutExportError, WorkoutSegment};
use std::path::Path;

/// Export a workout to ZWO XML format.
///
/// Returns the workout as a ZWO-formatted XML string.
///
/// # Errors
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_zwo(workout: &Workout) -> Result<String, WorkoutExportError> {
    if workout.segments.is_empty() {
        return Err(WorkoutExportError::EmptyWorkout);
    }

    // TODO: Implement ZWO XML generation in phase 2.2
    todo!("ZWO export implementation")
}

/// Convert a power target to ZWO decimal format (0.75 = 75% FTP).
///
/// For Absolute power targets, we convert to FTP percentage assuming 100% FTP = 200W
/// as a reasonable default since we don't have the user's FTP available.
fn power_to_decimal(power: &PowerTarget) -> f32 {
    match power {
        PowerTarget::PercentFtp { percent } => *percent as f32 / 100.0,
        PowerTarget::Absolute { watts } => {
            // Convert absolute watts to percentage assuming 200W FTP as default
            // This is a reasonable default for export purposes
            (*watts as f32 / 200.0).min(3.0)
        }
        PowerTarget::Range { start, .. } => power_to_decimal(start),
    }
}

/// Convert a WorkoutSegment to a ZWO XML element string.
///
/// Handles all segment types:
/// - Warmup: Exported with PowerLow/PowerHigh for ranges, or Power for constant
/// - Cooldown: Exported with PowerLow/PowerHigh for ranges, or Power for constant
/// - SteadyState: Exported with Power attribute
/// - Intervals: Exported as SteadyState elements (intervals are expanded during parsing)
/// - FreeRide: Exported as FreeRide element
/// - Ramp: Exported as Ramp element with PowerLow/PowerHigh
///
/// Duration is always included in seconds.
/// Cadence attributes are included when cadence_target is present.
fn segment_to_xml(segment: &WorkoutSegment) -> String {
    let duration = segment.duration_seconds;

    // Build cadence attributes if present
    let cadence_attrs = if let Some(ref cadence) = segment.cadence_target {
        if cadence.min_rpm == cadence.max_rpm {
            // Single cadence value
            format!(" Cadence=\"{}\"", cadence.min_rpm)
        } else {
            // Cadence range
            format!(" CadenceLow=\"{}\" CadenceHigh=\"{}\"", cadence.min_rpm, cadence.max_rpm)
        }
    } else {
        String::new()
    };

    match segment.segment_type {
        SegmentType::Warmup => {
            match &segment.power_target {
                PowerTarget::Range { start, end } => {
                    let low = power_to_decimal(start);
                    let high = power_to_decimal(end);
                    format!(
                        "<Warmup Duration=\"{}\" PowerLow=\"{:.2}\" PowerHigh=\"{:.2}\"{}/>\n",
                        duration, low, high, cadence_attrs
                    )
                }
                _ => {
                    let power = power_to_decimal(&segment.power_target);
                    format!(
                        "<Warmup Duration=\"{}\" Power=\"{:.2}\"{}/>\n",
                        duration, power, cadence_attrs
                    )
                }
            }
        }
        SegmentType::Cooldown => {
            match &segment.power_target {
                PowerTarget::Range { start, end } => {
                    let low = power_to_decimal(start);
                    let high = power_to_decimal(end);
                    format!(
                        "<Cooldown Duration=\"{}\" PowerLow=\"{:.2}\" PowerHigh=\"{:.2}\"{}/>\n",
                        duration, low, high, cadence_attrs
                    )
                }
                _ => {
                    let power = power_to_decimal(&segment.power_target);
                    format!(
                        "<Cooldown Duration=\"{}\" Power=\"{:.2}\"{}/>\n",
                        duration, power, cadence_attrs
                    )
                }
            }
        }
        SegmentType::SteadyState => {
            let power = power_to_decimal(&segment.power_target);
            format!(
                "<SteadyState Duration=\"{}\" Power=\"{:.2}\"{}/>\n",
                duration, power, cadence_attrs
            )
        }
        SegmentType::Intervals => {
            // Intervals are expanded during parsing, so we export each as SteadyState
            let power = power_to_decimal(&segment.power_target);
            format!(
                "<SteadyState Duration=\"{}\" Power=\"{:.2}\"{}/>\n",
                duration, power, cadence_attrs
            )
        }
        SegmentType::FreeRide => {
            format!(
                "<FreeRide Duration=\"{}\"{}/>\n",
                duration, cadence_attrs
            )
        }
        SegmentType::Ramp => {
            match &segment.power_target {
                PowerTarget::Range { start, end } => {
                    let low = power_to_decimal(start);
                    let high = power_to_decimal(end);
                    format!(
                        "<Ramp Duration=\"{}\" PowerLow=\"{:.2}\" PowerHigh=\"{:.2}\"{}/>\n",
                        duration, low, high, cadence_attrs
                    )
                }
                _ => {
                    // Ramp without range - use same power for both
                    let power = power_to_decimal(&segment.power_target);
                    format!(
                        "<Ramp Duration=\"{}\" PowerLow=\"{:.2}\" PowerHigh=\"{:.2}\"{}/>\n",
                        duration, power, power, cadence_attrs
                    )
                }
            }
        }
    }
}

/// Export a workout to ZWO format and write to a file.
///
/// # Errors
/// Returns `WorkoutExportError::IoError` if the file cannot be written.
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_zwo_to_file(workout: &Workout, path: &Path) -> Result<(), WorkoutExportError> {
    let content = export_zwo(workout)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a workout ZWO export.
///
/// The filename is based on the workout name with invalid filesystem
/// characters removed and a `.zwo` extension added.
pub fn generate_zwo_filename(workout: &Workout) -> String {
    let sanitized = sanitize_filename(&workout.name);
    format!("{}.zwo", sanitized)
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workouts::types::CadenceTarget;

    #[test]
    fn test_generate_zwo_filename_simple() {
        let workout = Workout::new("Sweet Spot".to_string(), vec![]);
        let filename = generate_zwo_filename(&workout);
        assert_eq!(filename, "Sweet Spot.zwo");
    }

    #[test]
    fn test_generate_zwo_filename_sanitizes_invalid_chars() {
        let workout = Workout::new("Test/Workout:Name*Here".to_string(), vec![]);
        let filename = generate_zwo_filename(&workout);
        assert_eq!(filename, "Test_Workout_Name_Here.zwo");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Normal Name"), "Normal Name");
        assert_eq!(sanitize_filename("File/With\\Bad:Chars"), "File_With_Bad_Chars");
        assert_eq!(sanitize_filename("Has*Question?Mark"), "Has_Question_Mark");
        assert_eq!(sanitize_filename("Quotes\"and<brackets>"), "Quotes_and_brackets_");
    }

    #[test]
    fn test_export_zwo_empty_workout_error() {
        let workout = Workout::new("Empty".to_string(), vec![]);
        let result = export_zwo(&workout);
        assert!(matches!(result, Err(WorkoutExportError::EmptyWorkout)));
    }

    // segment_to_xml tests

    #[test]
    fn test_segment_to_xml_steady_state() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<SteadyState Duration=\"300\" Power=\"0.75\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_steady_state_with_cadence() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(90),
            cadence_target: Some(CadenceTarget { min_rpm: 85, max_rpm: 95 }),
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<SteadyState Duration=\"300\" Power=\"0.90\" CadenceLow=\"85\" CadenceHigh=\"95\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_steady_state_with_single_cadence() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(100),
            cadence_target: Some(CadenceTarget { min_rpm: 90, max_rpm: 90 }),
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<SteadyState Duration=\"60\" Power=\"1.00\" Cadence=\"90\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_warmup_with_range() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 600,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(70),
            ),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Warmup Duration=\"600\" PowerLow=\"0.40\" PowerHigh=\"0.70\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_warmup_constant_power() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Warmup Duration=\"300\" Power=\"0.50\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_cooldown_with_range() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 600,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Cooldown Duration=\"600\" PowerLow=\"0.60\" PowerHigh=\"0.40\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_cooldown_constant_power() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(45),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Cooldown Duration=\"300\" Power=\"0.45\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_freeride() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::FreeRide,
            duration_seconds: 900,
            power_target: PowerTarget::percent_ftp(0),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<FreeRide Duration=\"900\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_freeride_with_cadence() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::FreeRide,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(0),
            cadence_target: Some(CadenceTarget { min_rpm: 80, max_rpm: 100 }),
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<FreeRide Duration=\"300\" CadenceLow=\"80\" CadenceHigh=\"100\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_ramp_with_range() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(50),
                PowerTarget::percent_ftp(100),
            ),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Ramp Duration=\"300\" PowerLow=\"0.50\" PowerHigh=\"1.00\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_ramp_without_range() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 120,
            power_target: PowerTarget::percent_ftp(80),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<Ramp Duration=\"120\" PowerLow=\"0.80\" PowerHigh=\"0.80\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_intervals_as_steady_state() {
        let segment = WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(120),
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        // Intervals are exported as SteadyState since they're expanded during parsing
        assert_eq!(xml, "<SteadyState Duration=\"30\" Power=\"1.20\"/>\n");
    }

    #[test]
    fn test_segment_to_xml_absolute_power() {
        // Absolute power should be converted to percentage (assuming 200W FTP)
        let segment = WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::absolute(150), // 150W / 200W = 0.75
            cadence_target: None,
            text_event: None,
        };
        let xml = segment_to_xml(&segment);
        assert_eq!(xml, "<SteadyState Duration=\"60\" Power=\"0.75\"/>\n");
    }

    #[test]
    fn test_power_to_decimal_percent_ftp() {
        assert_eq!(power_to_decimal(&PowerTarget::percent_ftp(75)), 0.75);
        assert_eq!(power_to_decimal(&PowerTarget::percent_ftp(100)), 1.0);
        assert_eq!(power_to_decimal(&PowerTarget::percent_ftp(120)), 1.2);
    }

    #[test]
    fn test_power_to_decimal_absolute() {
        // Assuming 200W FTP as default
        assert_eq!(power_to_decimal(&PowerTarget::absolute(200)), 1.0);
        assert_eq!(power_to_decimal(&PowerTarget::absolute(150)), 0.75);
        assert_eq!(power_to_decimal(&PowerTarget::absolute(100)), 0.5);
    }

    #[test]
    fn test_power_to_decimal_absolute_capped() {
        // Very high absolute power should be capped at 3.0
        let high_power = PowerTarget::absolute(1000);
        assert!(power_to_decimal(&high_power) <= 3.0);
    }

    #[test]
    fn test_power_to_decimal_range() {
        // Range should return start power
        let range = PowerTarget::range(
            PowerTarget::percent_ftp(50),
            PowerTarget::percent_ftp(100),
        );
        assert_eq!(power_to_decimal(&range), 0.5);
    }
}
