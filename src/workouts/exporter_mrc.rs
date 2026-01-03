//! MRC (TrainerRoad) export functionality.
//!
//! Provides functions to export Workout structs to TrainerRoad's MRC text format.

use crate::workouts::types::{PowerTarget, SegmentType, Workout, WorkoutExportError, WorkoutSegment};
use std::path::Path;

/// A point in the MRC course data (time in minutes, power in percent FTP).
#[derive(Debug, Clone, PartialEq)]
struct CourseDataPoint {
    minutes: f32,
    power_percent: u8,
}

/// A text event entry for the MRC COURSE TEXT section.
#[derive(Debug, Clone, PartialEq)]
struct TextEventEntry {
    minutes: f32,
    text: String,
}

/// Convert a power target to percent FTP.
///
/// For PercentFtp targets, returns the percent directly.
/// For Absolute targets, converts using the optional FTP value (default 200W).
/// For Range targets, returns the start power.
fn power_to_percent(power: &PowerTarget, ftp: Option<u16>) -> u8 {
    let ftp_value = ftp.unwrap_or(200);
    match power {
        PowerTarget::PercentFtp { percent } => *percent,
        PowerTarget::Absolute { watts } => {
            let percent = (*watts as f32 / ftp_value as f32 * 100.0) as u8;
            percent.min(255) // Cap at u8 max
        }
        PowerTarget::Range { start, .. } => power_to_percent(start, ftp),
    }
}

/// Convert a list of workout segments to MRC COURSE DATA points.
///
/// Each segment is converted to time/power pairs:
/// - SteadyState, FreeRide, Intervals: Two points at the same power (start and end)
/// - Warmup, Cooldown, Ramp with Range: Two points at start and end power
/// - Warmup, Cooldown, Ramp without Range: Two points at the same power
///
/// Time accumulates correctly across segments. Power is expressed as percent FTP.
///
/// # Arguments
/// * `segments` - The workout segments to convert
/// * `ftp` - Optional FTP value for converting absolute power targets (defaults to 200W)
///
/// # Returns
/// A vector of CourseDataPoint tuples (minutes, power_percent)
fn segments_to_course_data(segments: &[WorkoutSegment], ftp: Option<u16>) -> Vec<CourseDataPoint> {
    let mut points = Vec::new();
    let mut current_time_minutes: f32 = 0.0;

    for segment in segments {
        let duration_minutes = segment.duration_seconds as f32 / 60.0;

        match segment.segment_type {
            SegmentType::SteadyState | SegmentType::FreeRide | SegmentType::Intervals => {
                // Constant power - two points at the same level
                let power = if segment.segment_type == SegmentType::FreeRide {
                    0 // FreeRide means no ERG target, typically 0 or low power
                } else {
                    power_to_percent(&segment.power_target, ftp)
                };

                points.push(CourseDataPoint {
                    minutes: current_time_minutes,
                    power_percent: power,
                });
                points.push(CourseDataPoint {
                    minutes: current_time_minutes + duration_minutes,
                    power_percent: power,
                });
            }
            SegmentType::Warmup | SegmentType::Cooldown | SegmentType::Ramp => {
                // Check if power target is a range
                match &segment.power_target {
                    PowerTarget::Range { start, end } => {
                        // Ramp with different start/end power
                        let start_power = power_to_percent(start, ftp);
                        let end_power = power_to_percent(end, ftp);

                        points.push(CourseDataPoint {
                            minutes: current_time_minutes,
                            power_percent: start_power,
                        });
                        points.push(CourseDataPoint {
                            minutes: current_time_minutes + duration_minutes,
                            power_percent: end_power,
                        });
                    }
                    _ => {
                        // Constant power even though segment type suggests ramp
                        let power = power_to_percent(&segment.power_target, ftp);
                        points.push(CourseDataPoint {
                            minutes: current_time_minutes,
                            power_percent: power,
                        });
                        points.push(CourseDataPoint {
                            minutes: current_time_minutes + duration_minutes,
                            power_percent: power,
                        });
                    }
                }
            }
        }

        current_time_minutes += duration_minutes;
    }

    points
}

/// Extract text events from workout segments for the MRC COURSE TEXT section.
///
/// Each segment with a text_event is converted to a TextEventEntry with:
/// - The start time of the segment (in minutes)
/// - The text event message
///
/// Segments without text events are skipped.
///
/// # Arguments
/// * `segments` - The workout segments to extract text events from
///
/// # Returns
/// A vector of TextEventEntry with time and text for each text event
fn extract_text_events(segments: &[WorkoutSegment]) -> Vec<TextEventEntry> {
    let mut events = Vec::new();
    let mut current_time_minutes: f32 = 0.0;

    for segment in segments {
        // If segment has a text event, add it with the current start time
        if let Some(text) = &segment.text_event {
            events.push(TextEventEntry {
                minutes: current_time_minutes,
                text: text.clone(),
            });
        }

        // Accumulate time for next segment
        current_time_minutes += segment.duration_seconds as f32 / 60.0;
    }

    events
}

/// Format a text event entry as an MRC COURSE TEXT line.
///
/// Formats the entry as: `<minutes>    "<text>"`
/// For example: `5.00    "Zone 3"`
fn format_text_event(event: &TextEventEntry) -> String {
    format!("{:.2}\t\"{}\"", event.minutes, event.text)
}

/// Export a workout to MRC format.
///
/// Returns the workout as an MRC-formatted text string.
///
/// # Errors
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_mrc(workout: &Workout) -> Result<String, WorkoutExportError> {
    if workout.segments.is_empty() {
        return Err(WorkoutExportError::EmptyWorkout);
    }

    // TODO: Implement MRC format generation in phase 3
    todo!("MRC export implementation")
}

/// Export a workout to MRC format and write to a file.
///
/// # Errors
/// Returns `WorkoutExportError::IoError` if the file cannot be written.
/// Returns `WorkoutExportError::EmptyWorkout` if the workout has no segments.
pub fn export_mrc_to_file(workout: &Workout, path: &Path) -> Result<(), WorkoutExportError> {
    let content = export_mrc(workout)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a default filename for a workout MRC export.
///
/// The filename is based on the workout name with invalid filesystem
/// characters removed and a `.mrc` extension added.
pub fn generate_mrc_filename(workout: &Workout) -> String {
    let sanitized = sanitize_filename(&workout.name);
    format!("{}.mrc", sanitized)
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

    #[test]
    fn test_generate_mrc_filename_simple() {
        let workout = Workout::new("Sweet Spot".to_string(), vec![]);
        let filename = generate_mrc_filename(&workout);
        assert_eq!(filename, "Sweet Spot.mrc");
    }

    #[test]
    fn test_generate_mrc_filename_sanitizes_invalid_chars() {
        let workout = Workout::new("Test/Workout:Name*Here".to_string(), vec![]);
        let filename = generate_mrc_filename(&workout);
        assert_eq!(filename, "Test_Workout_Name_Here.mrc");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Normal Name"), "Normal Name");
        assert_eq!(sanitize_filename("File/With\\Bad:Chars"), "File_With_Bad_Chars");
        assert_eq!(sanitize_filename("Has*Question?Mark"), "Has_Question_Mark");
        assert_eq!(sanitize_filename("Quotes\"and<brackets>"), "Quotes_and_brackets_");
    }

    #[test]
    fn test_export_mrc_empty_workout_error() {
        let workout = Workout::new("Empty".to_string(), vec![]);
        let result = export_mrc(&workout);
        assert!(matches!(result, Err(WorkoutExportError::EmptyWorkout)));
    }

    // power_to_percent tests

    #[test]
    fn test_power_to_percent_ftp() {
        assert_eq!(power_to_percent(&PowerTarget::percent_ftp(75), None), 75);
        assert_eq!(power_to_percent(&PowerTarget::percent_ftp(100), None), 100);
        assert_eq!(power_to_percent(&PowerTarget::percent_ftp(120), None), 120);
    }

    #[test]
    fn test_power_to_percent_absolute_default_ftp() {
        // Default FTP is 200W
        assert_eq!(power_to_percent(&PowerTarget::absolute(200), None), 100);
        assert_eq!(power_to_percent(&PowerTarget::absolute(150), None), 75);
        assert_eq!(power_to_percent(&PowerTarget::absolute(100), None), 50);
    }

    #[test]
    fn test_power_to_percent_absolute_with_ftp() {
        // Using 250W FTP
        assert_eq!(power_to_percent(&PowerTarget::absolute(250), Some(250)), 100);
        assert_eq!(power_to_percent(&PowerTarget::absolute(125), Some(250)), 50);
        assert_eq!(power_to_percent(&PowerTarget::absolute(187), Some(250)), 74); // 74.8% rounds down
    }

    #[test]
    fn test_power_to_percent_range() {
        // Range returns start power
        let range = PowerTarget::range(
            PowerTarget::percent_ftp(50),
            PowerTarget::percent_ftp(100),
        );
        assert_eq!(power_to_percent(&range, None), 50);
    }

    // segments_to_course_data tests

    #[test]
    fn test_segments_to_course_data_steady_state() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300, // 5 minutes
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 75);
        assert_eq!(points[1].minutes, 5.0);
        assert_eq!(points[1].power_percent, 75);
    }

    #[test]
    fn test_segments_to_course_data_warmup_with_range() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 600, // 10 minutes
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(40),
                PowerTarget::percent_ftp(70),
            ),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 40);
        assert_eq!(points[1].minutes, 10.0);
        assert_eq!(points[1].power_percent, 70);
    }

    #[test]
    fn test_segments_to_course_data_cooldown_with_range() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Cooldown,
            duration_seconds: 300, // 5 minutes
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(60),
                PowerTarget::percent_ftp(40),
            ),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 60);
        assert_eq!(points[1].minutes, 5.0);
        assert_eq!(points[1].power_percent, 40);
    }

    #[test]
    fn test_segments_to_course_data_ramp_with_range() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Ramp,
            duration_seconds: 180, // 3 minutes
            power_target: PowerTarget::range(
                PowerTarget::percent_ftp(80),
                PowerTarget::percent_ftp(100),
            ),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 80);
        assert_eq!(points[1].minutes, 3.0);
        assert_eq!(points[1].power_percent, 100);
    }

    #[test]
    fn test_segments_to_course_data_freeride() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::FreeRide,
            duration_seconds: 600, // 10 minutes
            power_target: PowerTarget::percent_ftp(50), // Should be ignored for FreeRide
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 0); // FreeRide = 0 power
        assert_eq!(points[1].minutes, 10.0);
        assert_eq!(points[1].power_percent, 0);
    }

    #[test]
    fn test_segments_to_course_data_intervals() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Intervals,
            duration_seconds: 30,
            power_target: PowerTarget::percent_ftp(120),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 120);
        assert_eq!(points[1].minutes, 0.5); // 30 seconds = 0.5 minutes
        assert_eq!(points[1].power_percent, 120);
    }

    #[test]
    fn test_segments_to_course_data_time_accumulation() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600, // 10 minutes
                power_target: PowerTarget::percent_ftp(75),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: None,
            },
        ];

        let points = segments_to_course_data(&segments, None);

        // Each segment produces 2 points
        assert_eq!(points.len(), 6);

        // First segment: 0-5 minutes at 50%
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 50);
        assert_eq!(points[1].minutes, 5.0);
        assert_eq!(points[1].power_percent, 50);

        // Second segment: 5-15 minutes at 75%
        assert_eq!(points[2].minutes, 5.0);
        assert_eq!(points[2].power_percent, 75);
        assert_eq!(points[3].minutes, 15.0);
        assert_eq!(points[3].power_percent, 75);

        // Third segment: 15-20 minutes at 50%
        assert_eq!(points[4].minutes, 15.0);
        assert_eq!(points[4].power_percent, 50);
        assert_eq!(points[5].minutes, 20.0);
        assert_eq!(points[5].power_percent, 50);
    }

    #[test]
    fn test_segments_to_course_data_complex_workout() {
        let segments = vec![
            // Warmup: 0-10 minutes, 40% -> 70%
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 600,
                power_target: PowerTarget::range(
                    PowerTarget::percent_ftp(40),
                    PowerTarget::percent_ftp(70),
                ),
                cadence_target: None,
                text_event: None,
            },
            // Main set: 10-25 minutes at 88%
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 900,
                power_target: PowerTarget::percent_ftp(88),
                cadence_target: None,
                text_event: None,
            },
            // Cooldown: 25-35 minutes, 60% -> 40%
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

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 6);

        // Warmup
        assert_eq!(points[0].minutes, 0.0);
        assert_eq!(points[0].power_percent, 40);
        assert_eq!(points[1].minutes, 10.0);
        assert_eq!(points[1].power_percent, 70);

        // Main set
        assert_eq!(points[2].minutes, 10.0);
        assert_eq!(points[2].power_percent, 88);
        assert_eq!(points[3].minutes, 25.0);
        assert_eq!(points[3].power_percent, 88);

        // Cooldown
        assert_eq!(points[4].minutes, 25.0);
        assert_eq!(points[4].power_percent, 60);
        assert_eq!(points[5].minutes, 35.0);
        assert_eq!(points[5].power_percent, 40);
    }

    #[test]
    fn test_segments_to_course_data_absolute_power_with_ftp() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300,
            power_target: PowerTarget::absolute(225), // 225W with 300W FTP = 75%
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, Some(300));

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].power_percent, 75);
        assert_eq!(points[1].power_percent, 75);
    }

    #[test]
    fn test_segments_to_course_data_warmup_without_range() {
        // Edge case: Warmup segment without a range power target
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::Warmup,
            duration_seconds: 300,
            power_target: PowerTarget::percent_ftp(50),
            cadence_target: None,
            text_event: None,
        }];

        let points = segments_to_course_data(&segments, None);

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].power_percent, 50);
        assert_eq!(points[1].power_percent, 50);
    }

    #[test]
    fn test_segments_to_course_data_empty_segments() {
        let segments: Vec<WorkoutSegment> = vec![];
        let points = segments_to_course_data(&segments, None);
        assert!(points.is_empty());
    }

    // extract_text_events tests

    #[test]
    fn test_extract_text_events_single_segment_with_event() {
        let segments = vec![WorkoutSegment {
            segment_type: SegmentType::SteadyState,
            duration_seconds: 300, // 5 minutes
            power_target: PowerTarget::percent_ftp(75),
            cadence_target: None,
            text_event: Some("Zone 3 effort".to_string()),
        }];

        let events = extract_text_events(&segments);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].minutes, 0.0);
        assert_eq!(events[0].text, "Zone 3 effort");
    }

    #[test]
    fn test_extract_text_events_no_text_events() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300,
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: None,
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600,
                power_target: PowerTarget::percent_ftp(75),
                cadence_target: None,
                text_event: None,
            },
        ];

        let events = extract_text_events(&segments);

        assert!(events.is_empty());
    }

    #[test]
    fn test_extract_text_events_multiple_segments_mixed() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: Some("Warm up".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600, // 10 minutes
                power_target: PowerTarget::percent_ftp(75),
                cadence_target: None,
                text_event: None, // No text event
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(88),
                cadence_target: None,
                text_event: Some("Sweet spot".to_string()),
            },
        ];

        let events = extract_text_events(&segments);

        assert_eq!(events.len(), 2);
        // First event at start (0 min)
        assert_eq!(events[0].minutes, 0.0);
        assert_eq!(events[0].text, "Warm up");
        // Second event at 15 min (after warmup 5 min + steady 10 min)
        assert_eq!(events[1].minutes, 15.0);
        assert_eq!(events[1].text, "Sweet spot");
    }

    #[test]
    fn test_extract_text_events_all_segments_with_events() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Warmup,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: Some("Start easy".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 600, // 10 minutes
                power_target: PowerTarget::percent_ftp(75),
                cadence_target: None,
                text_event: Some("Main set".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::Cooldown,
                duration_seconds: 300, // 5 minutes
                power_target: PowerTarget::percent_ftp(40),
                cadence_target: None,
                text_event: Some("Cool down".to_string()),
            },
        ];

        let events = extract_text_events(&segments);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].minutes, 0.0);
        assert_eq!(events[0].text, "Start easy");
        assert_eq!(events[1].minutes, 5.0);
        assert_eq!(events[1].text, "Main set");
        assert_eq!(events[2].minutes, 15.0);
        assert_eq!(events[2].text, "Cool down");
    }

    #[test]
    fn test_extract_text_events_empty_segments() {
        let segments: Vec<WorkoutSegment> = vec![];
        let events = extract_text_events(&segments);
        assert!(events.is_empty());
    }

    #[test]
    fn test_extract_text_events_fractional_minutes() {
        let segments = vec![
            WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: 30, // 0.5 minutes
                power_target: PowerTarget::percent_ftp(120),
                cadence_target: None,
                text_event: Some("Sprint!".to_string()),
            },
            WorkoutSegment {
                segment_type: SegmentType::SteadyState,
                duration_seconds: 90, // 1.5 minutes
                power_target: PowerTarget::percent_ftp(50),
                cadence_target: None,
                text_event: Some("Recover".to_string()),
            },
        ];

        let events = extract_text_events(&segments);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].minutes, 0.0);
        assert_eq!(events[0].text, "Sprint!");
        assert_eq!(events[1].minutes, 0.5);
        assert_eq!(events[1].text, "Recover");
    }

    // format_text_event tests

    #[test]
    fn test_format_text_event_simple() {
        let event = TextEventEntry {
            minutes: 0.0,
            text: "Zone 3".to_string(),
        };

        let formatted = format_text_event(&event);

        assert_eq!(formatted, "0.00\t\"Zone 3\"");
    }

    #[test]
    fn test_format_text_event_with_time() {
        let event = TextEventEntry {
            minutes: 5.0,
            text: "Push hard!".to_string(),
        };

        let formatted = format_text_event(&event);

        assert_eq!(formatted, "5.00\t\"Push hard!\"");
    }

    #[test]
    fn test_format_text_event_fractional_minutes() {
        let event = TextEventEntry {
            minutes: 12.50,
            text: "Halfway there".to_string(),
        };

        let formatted = format_text_event(&event);

        assert_eq!(formatted, "12.50\t\"Halfway there\"");
    }

    #[test]
    fn test_format_text_event_long_text() {
        let event = TextEventEntry {
            minutes: 0.0,
            text: "This is a longer text message for the workout".to_string(),
        };

        let formatted = format_text_event(&event);

        assert_eq!(
            formatted,
            "0.00\t\"This is a longer text message for the workout\""
        );
    }
}
