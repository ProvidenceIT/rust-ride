//! MRC/ERG workout file parser.
//!
//! T061: Implement .mrc/.erg text parser
//!
//! MRC files are simple text-based workout definitions used by TrainerRoad
//! and other training applications. They define power targets as percentage
//! of FTP over time.

use crate::workouts::types::{
    PowerTarget, SegmentType, Workout, WorkoutFormat, WorkoutParseError, WorkoutSegment,
};

/// A point in the MRC course data (time -> power percentage).
#[derive(Debug, Clone)]
struct CoursePoint {
    minutes: f32,
    power_percent: u8,
}

/// A text event at a specific time.
#[derive(Debug, Clone)]
struct TextEvent {
    minutes: f32,
    text: String,
}

/// Parse an MRC workout from text content.
pub fn parse_mrc(content: &str) -> Result<Workout, WorkoutParseError> {
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut course_points: Vec<CoursePoint> = Vec::new();
    let mut text_events: Vec<TextEvent> = Vec::new();

    // Track which section we're in
    let mut in_header = false;
    let mut in_course_data = false;
    let mut in_course_text = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Section markers
        match line {
            "[COURSE HEADER]" => {
                in_header = true;
                in_course_data = false;
                in_course_text = false;
                continue;
            }
            "[END COURSE HEADER]" => {
                in_header = false;
                continue;
            }
            "[COURSE DATA]" => {
                in_course_data = true;
                in_header = false;
                in_course_text = false;
                continue;
            }
            "[END COURSE DATA]" => {
                in_course_data = false;
                continue;
            }
            "[COURSE TEXT]" => {
                in_course_text = true;
                in_header = false;
                in_course_data = false;
                continue;
            }
            "[END COURSE TEXT]" => {
                in_course_text = false;
                continue;
            }
            _ => {}
        }

        // Parse header fields
        if in_header {
            if let Some((key, value)) = parse_header_line(line) {
                match key.to_uppercase().as_str() {
                    "FILE NAME" => name = Some(value),
                    "DESCRIPTION" => description = Some(value),
                    _ => {}
                }
            }
        }

        // Parse course data (time power pairs)
        if in_course_data {
            if let Some(point) = parse_course_point(line) {
                course_points.push(point);
            }
        }

        // Parse course text (time "message" pairs)
        if in_course_text {
            if let Some(event) = parse_text_event(line) {
                text_events.push(event);
            }
        }
    }

    // Validate course data
    if course_points.len() < 2 {
        return Err(WorkoutParseError::EmptyWorkout);
    }

    // Convert course points to segments
    let segments = build_segments(&course_points, &text_events);

    if segments.is_empty() {
        return Err(WorkoutParseError::EmptyWorkout);
    }

    let workout_name = name.unwrap_or_else(|| "Unnamed MRC Workout".to_string());
    let mut workout = Workout::new(workout_name, segments);
    workout.description = description;
    workout.source_format = Some(WorkoutFormat::Mrc);

    Ok(workout)
}

/// Parse a header line like "KEY = value".
fn parse_header_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

/// Parse a course data point like "5.00    75".
fn parse_course_point(line: &str) -> Option<CoursePoint> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let minutes = parts[0].parse().ok()?;
        let power_percent = parts[1].parse().ok()?;
        Some(CoursePoint {
            minutes,
            power_percent,
        })
    } else {
        None
    }
}

/// Parse a text event line like `5.00    "Zone 3"`.
fn parse_text_event(line: &str) -> Option<TextEvent> {
    // Find the first whitespace to split time from text
    let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
    if parts.len() >= 2 {
        let minutes: f32 = parts[0].trim().parse().ok()?;
        // Remove quotes from text
        let text = parts[1]
            .trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string();
        Some(TextEvent { minutes, text })
    } else {
        None
    }
}

/// Build workout segments from course points.
fn build_segments(points: &[CoursePoint], text_events: &[TextEvent]) -> Vec<WorkoutSegment> {
    let mut segments = Vec::new();

    for i in 0..points.len() - 1 {
        let start = &points[i];
        let end = &points[i + 1];

        let start_seconds = (start.minutes * 60.0) as u32;
        let end_seconds = (end.minutes * 60.0) as u32;
        let duration = end_seconds.saturating_sub(start_seconds);

        if duration == 0 {
            continue;
        }

        // Determine segment type and power target
        let (segment_type, power_target) = if start.power_percent == end.power_percent {
            // Constant power - steady state
            (
                SegmentType::SteadyState,
                PowerTarget::percent_ftp(start.power_percent),
            )
        } else if start.power_percent < end.power_percent {
            // Increasing power - warmup or ramp
            (
                SegmentType::Warmup,
                PowerTarget::range(
                    PowerTarget::percent_ftp(start.power_percent),
                    PowerTarget::percent_ftp(end.power_percent),
                ),
            )
        } else {
            // Decreasing power - cooldown
            (
                SegmentType::Cooldown,
                PowerTarget::range(
                    PowerTarget::percent_ftp(start.power_percent),
                    PowerTarget::percent_ftp(end.power_percent),
                ),
            )
        };

        // Find text event for this segment (at segment start time)
        let text_event = text_events
            .iter()
            .find(|e| (e.minutes - start.minutes).abs() < 0.01)
            .map(|e| e.text.clone());

        segments.push(WorkoutSegment {
            segment_type,
            duration_seconds: duration,
            power_target,
            cadence_target: None,
            text_event,
        });
    }

    segments
}

/// Parse an MRC file from disk.
pub fn parse_mrc_file(path: &std::path::Path) -> Result<Workout, WorkoutParseError> {
    let content =
        std::fs::read_to_string(path).map_err(|e| WorkoutParseError::IoError(e.to_string()))?;

    let mut workout = parse_mrc(&content)?;
    workout.source_file = Some(path.display().to_string());

    Ok(workout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_mrc() {
        let mrc = r#"[COURSE HEADER]
VERSION = 2
FILE NAME = simple_test
[END COURSE HEADER]
[COURSE DATA]
0.00    50
5.00    50
5.00    75
10.00   75
[END COURSE DATA]"#;

        let workout = parse_mrc(mrc).unwrap();
        assert_eq!(workout.name, "simple_test");
        assert_eq!(workout.segments.len(), 2);
    }

    #[test]
    fn test_parse_mrc_durations() {
        let mrc = r#"[COURSE HEADER]
FILE NAME = duration_test
[END COURSE HEADER]
[COURSE DATA]
0.00    50
5.00    50
10.00   75
[END COURSE DATA]"#;

        let workout = parse_mrc(mrc).unwrap();
        assert_eq!(workout.segments[0].duration_seconds, 300); // 5 minutes
        assert_eq!(workout.segments[1].duration_seconds, 300); // 5 minutes
        assert_eq!(workout.total_duration_seconds, 600); // 10 minutes
    }

    #[test]
    fn test_parse_mrc_with_ramp() {
        let mrc = r#"[COURSE HEADER]
FILE NAME = ramp_test
[END COURSE HEADER]
[COURSE DATA]
0.00    50
5.00    100
[END COURSE DATA]"#;

        let workout = parse_mrc(mrc).unwrap();
        let segment = &workout.segments[0];

        // Should be warmup type with range
        assert_eq!(segment.segment_type, SegmentType::Warmup);
        match &segment.power_target {
            PowerTarget::Range { start, end } => match (start.as_ref(), end.as_ref()) {
                (
                    PowerTarget::PercentFtp { percent: s },
                    PowerTarget::PercentFtp { percent: e },
                ) => {
                    assert_eq!(*s, 50);
                    assert_eq!(*e, 100);
                }
                _ => panic!("Expected PercentFtp"),
            },
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_parse_mrc_text_events() {
        let mrc = r#"[COURSE HEADER]
FILE NAME = text_test
[END COURSE HEADER]
[COURSE DATA]
0.00    50
5.00    50
[END COURSE DATA]
[COURSE TEXT]
0.00    "Warmup zone"
[END COURSE TEXT]"#;

        let workout = parse_mrc(mrc).unwrap();
        assert_eq!(
            workout.segments[0].text_event.as_deref(),
            Some("Warmup zone")
        );
    }

    #[test]
    fn test_parse_mrc_empty() {
        let mrc = r#"[COURSE HEADER]
[END COURSE HEADER]
[COURSE DATA]
[END COURSE DATA]"#;

        let result = parse_mrc(mrc);
        assert!(result.is_err());
    }
}
