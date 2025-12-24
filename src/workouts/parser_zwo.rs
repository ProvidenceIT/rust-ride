//! Zwift workout (.zwo) file parser.
//!
//! T060: Implement .zwo XML parser with quick-xml
//!
//! ZWO files are XML-based workout definitions used by Zwift.
//! They contain structured workouts with various segment types.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::workouts::types::{
    CadenceTarget, PowerTarget, SegmentType, Workout, WorkoutFormat, WorkoutParseError,
    WorkoutSegment,
};

/// Parse a ZWO workout from XML content.
pub fn parse_zwo(content: &str) -> Result<Workout, WorkoutParseError> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut workout_name: Option<String> = None;
    let mut workout_author: Option<String> = None;
    let mut workout_description: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut segments: Vec<WorkoutSegment> = Vec::new();

    let mut in_workout = false;
    let mut current_element: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match name.as_str() {
                    "name" | "author" | "description" => {
                        current_element = Some(name);
                    }
                    "workout" => {
                        in_workout = true;
                    }
                    "tag" => {
                        // Extract tag name attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                let tag_value =
                                    String::from_utf8_lossy(&attr.value).to_string();
                                tags.push(tag_value);
                            }
                        }
                    }
                    _ if in_workout => {
                        // Parse workout segment
                        if let Some(segment) = parse_segment(&name, e)? {
                            segments.push(segment);
                        } else if let Some(interval_segments) = parse_intervals(&name, e)? {
                            segments.extend(interval_segments);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                if in_workout {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if let Some(segment) = parse_segment(&name, e)? {
                        segments.push(segment);
                    } else if let Some(interval_segments) = parse_intervals(&name, e)? {
                        segments.extend(interval_segments);
                    }
                } else if e.name().as_ref() == b"tag" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            let tag_value = String::from_utf8_lossy(&attr.value).to_string();
                            tags.push(tag_value);
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref elem) = current_element {
                    let text = e.unescape().map_err(|e| {
                        WorkoutParseError::InvalidXml(format!("Failed to unescape text: {}", e))
                    })?;
                    match elem.as_str() {
                        "name" => workout_name = Some(text.to_string()),
                        "author" => workout_author = Some(text.to_string()),
                        "description" => workout_description = Some(text.to_string()),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "workout" {
                    in_workout = false;
                }
                current_element = None;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(WorkoutParseError::InvalidXml(format!(
                    "XML parsing error: {}",
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    // Validate workout
    if segments.is_empty() {
        return Err(WorkoutParseError::EmptyWorkout);
    }

    let name = workout_name.unwrap_or_else(|| "Unnamed Workout".to_string());
    let mut workout = Workout::new(name, segments);
    workout.author = workout_author;
    workout.description = workout_description;
    workout.source_format = Some(WorkoutFormat::Zwo);
    workout.tags = tags;

    Ok(workout)
}

/// Parse a single workout segment from XML attributes.
fn parse_segment<'a>(
    element_name: &str,
    event: &quick_xml::events::BytesStart<'a>,
) -> Result<Option<WorkoutSegment>, WorkoutParseError> {
    let segment_type = match element_name {
        "Warmup" => SegmentType::Warmup,
        "Cooldown" => SegmentType::Cooldown,
        "SteadyState" => SegmentType::SteadyState,
        "FreeRide" => SegmentType::FreeRide,
        "Ramp" => SegmentType::Ramp,
        _ => return Ok(None),
    };

    let mut duration_seconds: Option<u32> = None;
    let mut power: Option<f32> = None;
    let mut power_low: Option<f32> = None;
    let mut power_high: Option<f32> = None;
    let mut cadence: Option<u8> = None;
    let mut cadence_low: Option<u8> = None;
    let mut cadence_high: Option<u8> = None;

    for attr in event.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let value = String::from_utf8_lossy(&attr.value);

        match key.as_ref() {
            "Duration" => {
                duration_seconds = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "Duration".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "Power" => {
                power = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "Power".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "PowerLow" => {
                power_low = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "PowerLow".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "PowerHigh" => {
                power_high = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "PowerHigh".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "Cadence" => {
                cadence = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "Cadence".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "CadenceLow" => {
                cadence_low = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "CadenceLow".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "CadenceHigh" => {
                cadence_high = Some(value.parse().map_err(|_| {
                    WorkoutParseError::InvalidValue {
                        field: "CadenceHigh".to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            _ => {}
        }
    }

    let duration = duration_seconds.ok_or(WorkoutParseError::MissingField("Duration".to_string()))?;

    // Build power target
    let power_target = if let (Some(low), Some(high)) = (power_low, power_high) {
        // Ramp or warmup/cooldown with range
        PowerTarget::range(
            PowerTarget::percent_ftp((low * 100.0) as u8),
            PowerTarget::percent_ftp((high * 100.0) as u8),
        )
    } else if let Some(p) = power {
        PowerTarget::percent_ftp((p * 100.0) as u8)
    } else if segment_type == SegmentType::FreeRide {
        // Free ride segments don't need power target
        PowerTarget::percent_ftp(0)
    } else {
        return Err(WorkoutParseError::MissingField("Power".to_string()));
    };

    // Build cadence target if specified
    let cadence_target = if let (Some(low), Some(high)) = (cadence_low, cadence_high) {
        Some(CadenceTarget {
            min_rpm: low,
            max_rpm: high,
        })
    } else if let Some(c) = cadence {
        Some(CadenceTarget {
            min_rpm: c.saturating_sub(5),
            max_rpm: c.saturating_add(5),
        })
    } else {
        None
    };

    Ok(Some(WorkoutSegment {
        segment_type,
        duration_seconds: duration,
        power_target,
        cadence_target,
        text_event: None,
    }))
}

/// Parse interval blocks (IntervalsT) into multiple segments.
fn parse_intervals<'a>(
    element_name: &str,
    event: &quick_xml::events::BytesStart<'a>,
) -> Result<Option<Vec<WorkoutSegment>>, WorkoutParseError> {
    if element_name != "IntervalsT" {
        return Ok(None);
    }

    let mut repeat: u32 = 1;
    let mut on_duration: u32 = 0;
    let mut off_duration: u32 = 0;
    let mut on_power: f32 = 1.0;
    let mut off_power: f32 = 0.5;
    let mut on_cadence: Option<u8> = None;
    let mut off_cadence: Option<u8> = None;

    for attr in event.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let value = String::from_utf8_lossy(&attr.value);

        match key.as_ref() {
            "Repeat" => {
                repeat = value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "Repeat".to_string(),
                    value: value.to_string(),
                })?;
            }
            "OnDuration" => {
                on_duration = value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OnDuration".to_string(),
                    value: value.to_string(),
                })?;
            }
            "OffDuration" => {
                off_duration = value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OffDuration".to_string(),
                    value: value.to_string(),
                })?;
            }
            "OnPower" => {
                on_power = value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OnPower".to_string(),
                    value: value.to_string(),
                })?;
            }
            "OffPower" => {
                off_power = value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OffPower".to_string(),
                    value: value.to_string(),
                })?;
            }
            "Cadence" | "OnCadence" => {
                on_cadence = Some(value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OnCadence".to_string(),
                    value: value.to_string(),
                })?);
            }
            "CadenceResting" | "OffCadence" => {
                off_cadence = Some(value.parse().map_err(|_| WorkoutParseError::InvalidValue {
                    field: "OffCadence".to_string(),
                    value: value.to_string(),
                })?);
            }
            _ => {}
        }
    }

    if on_duration == 0 && off_duration == 0 {
        return Err(WorkoutParseError::MissingField("OnDuration/OffDuration".to_string()));
    }

    let mut segments = Vec::new();

    for _ in 0..repeat {
        // "On" interval (high intensity)
        if on_duration > 0 {
            segments.push(WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: on_duration,
                power_target: PowerTarget::percent_ftp((on_power * 100.0) as u8),
                cadence_target: on_cadence.map(|c| CadenceTarget {
                    min_rpm: c.saturating_sub(5),
                    max_rpm: c.saturating_add(5),
                }),
                text_event: None,
            });
        }

        // "Off" interval (recovery)
        if off_duration > 0 {
            segments.push(WorkoutSegment {
                segment_type: SegmentType::Intervals,
                duration_seconds: off_duration,
                power_target: PowerTarget::percent_ftp((off_power * 100.0) as u8),
                cadence_target: off_cadence.map(|c| CadenceTarget {
                    min_rpm: c.saturating_sub(5),
                    max_rpm: c.saturating_add(5),
                }),
                text_event: None,
            });
        }
    }

    Ok(Some(segments))
}

/// Parse a ZWO file from disk.
pub fn parse_zwo_file(path: &std::path::Path) -> Result<Workout, WorkoutParseError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| WorkoutParseError::IoError(e.to_string()))?;

    let mut workout = parse_zwo(&content)?;
    workout.source_file = Some(path.display().to_string());

    Ok(workout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_steady_state() {
        let zwo = r#"<?xml version="1.0"?>
<workout_file>
    <name>Simple Test</name>
    <workout>
        <SteadyState Duration="300" Power="0.75"/>
    </workout>
</workout_file>"#;

        let workout = parse_zwo(zwo).unwrap();
        assert_eq!(workout.name, "Simple Test");
        assert_eq!(workout.segments.len(), 1);
        assert_eq!(workout.segments[0].duration_seconds, 300);
    }

    #[test]
    fn test_parse_warmup_with_range() {
        let zwo = r#"<?xml version="1.0"?>
<workout_file>
    <name>Warmup Test</name>
    <workout>
        <Warmup Duration="600" PowerLow="0.4" PowerHigh="0.7"/>
    </workout>
</workout_file>"#;

        let workout = parse_zwo(zwo).unwrap();
        let segment = &workout.segments[0];
        assert_eq!(segment.segment_type, SegmentType::Warmup);

        match &segment.power_target {
            PowerTarget::Range { start, end } => {
                match (start.as_ref(), end.as_ref()) {
                    (PowerTarget::PercentFtp { percent: s }, PowerTarget::PercentFtp { percent: e }) => {
                        assert_eq!(*s, 40);
                        assert_eq!(*e, 70);
                    }
                    _ => panic!("Expected PercentFtp"),
                }
            }
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_parse_intervals() {
        let zwo = r#"<?xml version="1.0"?>
<workout_file>
    <name>Interval Test</name>
    <workout>
        <IntervalsT Repeat="3" OnDuration="30" OffDuration="30" OnPower="1.2" OffPower="0.5"/>
    </workout>
</workout_file>"#;

        let workout = parse_zwo(zwo).unwrap();
        // 3 repeats * 2 (on + off) = 6 segments
        assert_eq!(workout.segments.len(), 6);
        assert_eq!(workout.total_duration_seconds, 180); // 6 * 30s
    }
}
