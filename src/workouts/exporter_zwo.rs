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

    let mut xml = String::new();

    // XML declaration
    xml.push_str("<?xml version=\"1.0\"?>\n");

    // Root element
    xml.push_str("<workout_file>\n");

    // Metadata elements
    xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&workout.name)));

    if let Some(ref author) = workout.author {
        xml.push_str(&format!("    <author>{}</author>\n", escape_xml(author)));
    }

    if let Some(ref description) = workout.description {
        xml.push_str(&format!(
            "    <description>{}</description>\n",
            escape_xml(description)
        ));
    }

    // Tags
    for tag in &workout.tags {
        xml.push_str(&format!("    <tag name=\"{}\"/>\n", escape_xml_attr(tag)));
    }

    // Workout element with segments
    xml.push_str("    <workout>\n");
    for segment in &workout.segments {
        // Indent segment XML
        let segment_xml = segment_to_xml(segment);
        xml.push_str(&format!("        {}", segment_xml));
    }
    xml.push_str("    </workout>\n");

    // Close root element
    xml.push_str("</workout_file>\n");

    Ok(xml)
}

/// Escape special XML characters in text content.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escape special XML characters in attribute values.
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

    // export_zwo tests

    #[test]
    fn test_export_zwo_simple_workout() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("Simple Test".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        assert!(result.starts_with("<?xml version=\"1.0\"?>"));
        assert!(result.contains("<workout_file>"));
        assert!(result.contains("<name>Simple Test</name>"));
        assert!(result.contains("<workout>"));
        assert!(result.contains("<SteadyState Duration=\"300\" Power=\"0.75\"/>"));
        assert!(result.contains("</workout>"));
        assert!(result.contains("</workout_file>"));
    }

    #[test]
    fn test_export_zwo_with_metadata() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(80),
            cadence_target: None,
            text_event: None,
        }];
        let mut workout = Workout::new("Full Metadata Test".to_string(), segments);
        workout.author = Some("Test Author".to_string());
        workout.description = Some("Test Description".to_string());
        workout.tags = vec!["Tag1".to_string(), "Tag2".to_string()];

        let result = export_zwo(&workout).unwrap();

        assert!(result.contains("<name>Full Metadata Test</name>"));
        assert!(result.contains("<author>Test Author</author>"));
        assert!(result.contains("<description>Test Description</description>"));
        assert!(result.contains("<tag name=\"Tag1\"/>"));
        assert!(result.contains("<tag name=\"Tag2\"/>"));
    }

    #[test]
    fn test_export_zwo_escapes_special_chars() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(100),
            cadence_target: None,
            text_event: None,
        }];
        let mut workout = Workout::new("Test & <Name>".to_string(), segments);
        workout.author = Some("Author \"Quote\"".to_string());
        workout.tags = vec!["Tag & Special".to_string()];

        let result = export_zwo(&workout).unwrap();

        assert!(result.contains("<name>Test &amp; &lt;Name&gt;</name>"));
        assert!(result.contains("<author>Author \"Quote\"</author>")); // quotes only escaped in attrs
        assert!(result.contains("<tag name=\"Tag &amp; Special\"/>"));
    }

    #[test]
    fn test_export_zwo_multiple_segments() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(60),
                ),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600,
                power_target: PowerTarget::percent_ftp(85),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::Cooldown,
                duration_seconds: 300,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(60),
                    PowerTarget::percent_ftp(40),
                ),
                cadence_target: None,
                text_event: None,
            },
        ];
        let workout = Workout::new("Multi-Segment".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        assert!(result.contains("<Warmup Duration=\"300\" PowerLow=\"0.40\" PowerHigh=\"0.60\"/>"));
        assert!(result.contains("<SteadyState Duration=\"600\" Power=\"0.85\"/>"));
        assert!(result.contains("<Cooldown Duration=\"300\" PowerLow=\"0.60\" PowerHigh=\"0.40\"/>"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Hello World"), "Hello World");
        assert_eq!(escape_xml("A & B"), "A &amp; B");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("A < B > C & D"), "A &lt; B &gt; C &amp; D");
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("Hello World"), "Hello World");
        assert_eq!(escape_xml_attr("A & B"), "A &amp; B");
        assert_eq!(escape_xml_attr("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml_attr("Say \"Hello\""), "Say &quot;Hello&quot;");
    }

    // Round-trip tests - verify export can be parsed back

    #[test]
    fn test_round_trip_simple_steady_state() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        }];
        let original = Workout::new("Round Trip Test".to_string(), segments);

        // Export to ZWO
        let zwo_xml = export_zwo(&original).unwrap();

        // Parse it back
        let parsed = parse_zwo(&zwo_xml).unwrap();

        // Verify key properties match
        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.segments.len(), original.segments.len());
        assert_eq!(parsed.segments[0].duration_seconds, original.segments[0].duration_seconds);
        assert_eq!(parsed.segments[0].segment_type, original.segments[0].segment_type);

        // Power target should be equivalent
        match &parsed.segments[0].power_target {
            PowerTarget::PercentFtp { percent } => assert_eq!(*percent, 75),
            _ => panic!("Expected PercentFtp power target"),
        }
    }

    #[test]
    fn test_round_trip_warmup_with_power_range() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 600,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(70),
            ),
            cadence_target: None,
            text_event: None,
        }];
        let original = Workout::new("Warmup Round Trip".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.segments[0].segment_type, SegmentType::Warmup);
        assert_eq!(parsed.segments[0].duration_seconds, 600);

        // Verify power range
        match &parsed.segments[0].power_target {
            PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        PowerTarget::PercentFtp { percent: start_pct },
                        PowerTarget::PercentFtp { percent: end_pct },
                    ) => {
                        assert_eq!(*start_pct, 40);
                        assert_eq!(*end_pct, 70);
                    }
                    _ => panic!("Expected PercentFtp in range"),
                }
            }
            _ => panic!("Expected Range power target"),
        }
    }

    #[test]
    fn test_round_trip_cooldown_with_power_range() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 300,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        }];
        let original = Workout::new("Cooldown Round Trip".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.segments[0].segment_type, SegmentType::Cooldown);

        match &parsed.segments[0].power_target {
            PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        PowerTarget::PercentFtp { percent: start_pct },
                        PowerTarget::PercentFtp { percent: end_pct },
                    ) => {
                        assert_eq!(*start_pct, 60);
                        assert_eq!(*end_pct, 40);
                    }
                    _ => panic!("Expected PercentFtp in range"),
                }
            }
            _ => panic!("Expected Range power target"),
        }
    }

    #[test]
    fn test_round_trip_with_metadata() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 120,
            power_target: PowerTarget::percent_ftp(90),
            cadence_target: None,
            text_event: None,
        }];
        let mut original = Workout::new("Metadata Round Trip".to_string(), segments);
        original.author = Some("Test Author".to_string());
        original.description = Some("A test workout description".to_string());
        original.tags = vec!["Endurance".to_string(), "Sweet Spot".to_string()];

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.name, "Metadata Round Trip");
        assert_eq!(parsed.author, Some("Test Author".to_string()));
        assert_eq!(parsed.description, Some("A test workout description".to_string()));
        assert_eq!(parsed.tags.len(), 2);
        assert!(parsed.tags.contains(&"Endurance".to_string()));
        assert!(parsed.tags.contains(&"Sweet Spot".to_string()));
    }

    #[test]
    fn test_round_trip_complex_workout() {
        use crate::workouts::parser_zwo::parse_zwo;

        // Create a complex workout with multiple segment types
        let segments = vec![
            // Warmup
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 600,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(60),
                ),
                cadence_target: None,
                text_event: None,
            },
            // Main set - SteadyState
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 900,
                power_target: PowerTarget::percent_ftp(88),
                cadence_target: Some(CadenceTarget { min_rpm: 85, max_rpm: 95 }),
                text_event: None,
            },
            // Ramp
            WorkoutSegment {
                segment_type: SegmentType::Ramp,
                duration_seconds: 180,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(80),
                    PowerTarget::percent_ftp(100),
                ),
                cadence_target: None,
                text_event: None,
            },
            // FreeRide
            WorkoutSegment {
                segment_type: SegmentType::FreeRide,
                duration_seconds: 300,
                power_target: PowerTarget::percent_ftp(0),
                cadence_target: None,
                text_event: None,
            },
            // Cooldown
            WorkoutSegment {
                segment_type: SegmentType::Cooldown,
                duration_seconds: 600,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(60),
                    PowerTarget::percent_ftp(40),
                ),
                cadence_target: None,
                text_event: None,
            },
        ];
        let mut original = Workout::new("Complex Workout".to_string(), segments);
        original.author = Some("RustRide".to_string());
        original.description = Some("A complex test workout with multiple segment types".to_string());

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        // Verify overall structure
        assert_eq!(parsed.name, "Complex Workout");
        assert_eq!(parsed.segments.len(), 5);
        assert_eq!(parsed.total_duration_seconds, 600 + 900 + 180 + 300 + 600);

        // Verify segment types preserved
        assert_eq!(parsed.segments[0].segment_type, SegmentType::Warmup);
        assert_eq!(parsed.segments[1].segment_type, SegmentType::SteadyState);
        assert_eq!(parsed.segments[2].segment_type, SegmentType::Ramp);
        assert_eq!(parsed.segments[3].segment_type, SegmentType::FreeRide);
        assert_eq!(parsed.segments[4].segment_type, SegmentType::Cooldown);

        // Verify durations
        assert_eq!(parsed.segments[0].duration_seconds, 600);
        assert_eq!(parsed.segments[1].duration_seconds, 900);
        assert_eq!(parsed.segments[2].duration_seconds, 180);
        assert_eq!(parsed.segments[3].duration_seconds, 300);
        assert_eq!(parsed.segments[4].duration_seconds, 600);
    }

    #[test]
    fn test_round_trip_with_cadence_range() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(85),
            cadence_target: Some(CadenceTarget { min_rpm: 80, max_rpm: 100 }),
            text_event: None,
        }];
        let original = Workout::new("Cadence Test".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        // Verify cadence is preserved
        let cadence = parsed.segments[0].cadence_target.as_ref().expect("Should have cadence");
        assert_eq!(cadence.min_rpm, 80);
        assert_eq!(cadence.max_rpm, 100);
    }

    #[test]
    fn test_round_trip_with_single_cadence() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(90),
            cadence_target: Some(CadenceTarget { min_rpm: 90, max_rpm: 90 }),
            text_event: None,
        }];
        let original = Workout::new("Single Cadence Test".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();

        // Verify XML contains single cadence format
        assert!(zwo_xml.contains("Cadence=\"90\""));
        assert!(!zwo_xml.contains("CadenceLow"));
        assert!(!zwo_xml.contains("CadenceHigh"));

        // Parse it back - note: parser may add a range around single cadence
        let parsed = parse_zwo(&zwo_xml).unwrap();
        let cadence = parsed.segments[0].cadence_target.as_ref().expect("Should have cadence");
        // Parser adds ±5 RPM tolerance to single cadence values
        assert!(cadence.min_rpm <= 90 && cadence.max_rpm >= 90);
    }

    #[test]
    fn test_round_trip_freeride() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::FreeRide,
            duration_seconds: 900,
            power_target: PowerTarget::percent_ftp(0),
            cadence_target: Some(CadenceTarget { min_rpm: 80, max_rpm: 100 }),
            text_event: None,
        }];
        let original = Workout::new("FreeRide Test".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.segments[0].segment_type, SegmentType::FreeRide);
        assert_eq!(parsed.segments[0].duration_seconds, 900);
    }

    #[test]
    fn test_round_trip_ramp() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 240,
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(50),
                PowerTarget::percent_ftp(100),
            ),
            cadence_target: None,
            text_event: None,
        }];
        let original = Workout::new("Ramp Test".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.segments[0].segment_type, SegmentType::Ramp);
        assert_eq!(parsed.segments[0].duration_seconds, 240);

        match &parsed.segments[0].power_target {
            PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (
                        PowerTarget::PercentFtp { percent: start_pct },
                        PowerTarget::PercentFtp { percent: end_pct },
                    ) => {
                        assert_eq!(*start_pct, 50);
                        assert_eq!(*end_pct, 100);
                    }
                    _ => panic!("Expected PercentFtp in range"),
                }
            }
            _ => panic!("Expected Range power target for Ramp"),
        }
    }

    #[test]
    fn test_round_trip_special_characters_in_metadata() {
        use crate::workouts::parser_zwo::parse_zwo;

        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 60,
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        }];
        let mut original = Workout::new("Test & <Special> Chars".to_string(), segments);
        original.author = Some("Author with & <chars>".to_string());
        original.description = Some("Description with \"quotes\" and <angle> brackets & ampersands".to_string());
        original.tags = vec!["Tag & More".to_string()];

        let zwo_xml = export_zwo(&original).unwrap();
        let parsed = parse_zwo(&zwo_xml).unwrap();

        assert_eq!(parsed.name, "Test & <Special> Chars");
        assert_eq!(parsed.author, Some("Author with & <chars>".to_string()));
        assert_eq!(
            parsed.description,
            Some("Description with \"quotes\" and <angle> brackets & ampersands".to_string())
        );
        assert!(parsed.tags.contains(&"Tag & More".to_string()));
    }

    // Edge case tests

    #[test]
    fn test_export_very_long_duration() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 7200, // 2 hours
            power_target: PowerTarget::percent_ftp(65),
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("Long Endurance Ride".to_string(), segments);

        let result = export_zwo(&workout).unwrap();
        assert!(result.contains("Duration=\"7200\""));
    }

    #[test]
    fn test_export_high_power_value() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(200), // VO2 max sprint
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("Sprint Test".to_string(), segments);

        let result = export_zwo(&workout).unwrap();
        assert!(result.contains("Power=\"2.00\""));
    }

    #[test]
    fn test_export_low_power_value() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(25), // Very easy recovery
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("Recovery Spin".to_string(), segments);

        let result = export_zwo(&workout).unwrap();
        assert!(result.contains("Power=\"0.25\""));
    }

    #[test]
    fn test_export_many_segments() {
        let mut segments = Vec::new();
        for i in 0..50 {
            segments.push(WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 60,
                power_target: PowerTarget::percent_ftp(50 + (i % 50) as u8),
                cadence_target: None,
                text_event: None,
            });
        }
        let workout = Workout::new("Many Segments".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        // Should have 50 SteadyState elements
        let count = result.matches("<SteadyState").count();
        assert_eq!(count, 50);
    }

    #[test]
    fn test_export_workout_no_author() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(80),
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("No Author".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        assert!(!result.contains("<author>"));
    }

    #[test]
    fn test_export_workout_no_description() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(80),
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("No Description".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        assert!(!result.contains("<description>"));
    }

    #[test]
    fn test_export_workout_no_tags() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(80),
            cadence_target: None,
            text_event: None,
        }];
        let workout = Workout::new("No Tags".to_string(), segments);

        let result = export_zwo(&workout).unwrap();

        assert!(!result.contains("<tag"));
    }

    #[test]
    fn test_export_xml_structure_valid() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(60),
                ),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600,
                power_target: PowerTarget::percent_ftp(85),
                cadence_target: None,
                text_event: None,
            },
        ];
        let mut workout = Workout::new("Structure Test".to_string(), segments);
        workout.author = Some("Test".to_string());
        workout.description = Some("Test description".to_string());
        workout.tags = vec!["Tag1".to_string()];

        let result = export_zwo(&workout).unwrap();

        // Verify basic XML structure
        assert!(result.starts_with("<?xml version=\"1.0\"?>"));
        assert!(result.contains("<workout_file>"));
        assert!(result.contains("</workout_file>"));
        assert!(result.contains("<workout>"));
        assert!(result.contains("</workout>"));

        // Verify element order (metadata before workout)
        let name_pos = result.find("<name>").unwrap();
        let workout_pos = result.find("<workout>").unwrap();
        assert!(name_pos < workout_pos);
    }

    #[test]
    fn test_intervals_exported_as_steady_state_round_trip() {
        use crate::workouts::parser_zwo::parse_zwo;

        // Create a workout with an Intervals segment
        // This simulates what we get when we have expanded intervals
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: 30,
                power_target: PowerTarget::percent_ftp(120),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: 30,
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: None,
            },
        ];
        let original = Workout::new("Intervals as SteadyState".to_string(), segments);

        let zwo_xml = export_zwo(&original).unwrap();

        // Verify exported as SteadyState (since Intervals is for internal representation)
        assert!(zwo_xml.contains("<SteadyState Duration=\"30\" Power=\"1.20\"/>"));
        assert!(zwo_xml.contains("<SteadyState Duration=\"30\" Power=\"0.50\"/>"));

        // Parse it back - should come back as SteadyState
        let parsed = parse_zwo(&zwo_xml).unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[0].segment_type, SegmentType::SteadyState);
        assert_eq!(parsed.segments[1].segment_type, SegmentType::SteadyState);
    }
}
