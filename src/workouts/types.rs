//! Workout types and enums.
//!
//! T013: Define PowerTarget, SegmentType, WorkoutStatus enums
//! T022: Define WorkoutError, WorkoutParseError enums
//! T058: Define Workout, WorkoutSegment structs
//! T059: Define WorkoutState, SegmentProgress structs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Type of workout segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentType {
    /// Gradual power increase
    Warmup,
    /// Gradual power decrease
    Cooldown,
    /// Constant power
    SteadyState,
    /// Repeating on/off blocks
    Intervals,
    /// No ERG target (resistance mode)
    FreeRide,
    /// Linear power change
    Ramp,
}

impl std::fmt::Display for SegmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentType::Warmup => write!(f, "Warmup"),
            SegmentType::Cooldown => write!(f, "Cooldown"),
            SegmentType::SteadyState => write!(f, "Steady State"),
            SegmentType::Intervals => write!(f, "Intervals"),
            SegmentType::FreeRide => write!(f, "Free Ride"),
            SegmentType::Ramp => write!(f, "Ramp"),
        }
    }
}

/// Power target specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerTarget {
    /// Fixed wattage target
    Absolute { watts: u16 },
    /// Percentage of user's FTP
    PercentFtp { percent: u8 },
    /// Range for ramps (start to end)
    Range {
        start: Box<PowerTarget>,
        end: Box<PowerTarget>,
    },
}

impl PowerTarget {
    /// Create an absolute power target.
    pub fn absolute(watts: u16) -> Self {
        PowerTarget::Absolute { watts }
    }

    /// Create a percent FTP target.
    pub fn percent_ftp(percent: u8) -> Self {
        PowerTarget::PercentFtp { percent }
    }

    /// Create a range target for ramps.
    pub fn range(start: PowerTarget, end: PowerTarget) -> Self {
        PowerTarget::Range {
            start: Box::new(start),
            end: Box::new(end),
        }
    }

    /// Calculate the actual wattage for a given FTP.
    pub fn to_watts(&self, ftp: u16) -> u16 {
        match self {
            PowerTarget::Absolute { watts } => *watts,
            PowerTarget::PercentFtp { percent } => (ftp as f32 * *percent as f32 / 100.0) as u16,
            PowerTarget::Range { start, .. } => start.to_watts(ftp),
        }
    }

    /// Calculate the actual wattage at a point in a range (0.0 to 1.0 progress).
    pub fn to_watts_at(&self, ftp: u16, progress: f32) -> u16 {
        match self {
            PowerTarget::Absolute { watts } => *watts,
            PowerTarget::PercentFtp { percent } => (ftp as f32 * *percent as f32 / 100.0) as u16,
            PowerTarget::Range { start, end } => {
                let start_watts = start.to_watts(ftp) as f32;
                let end_watts = end.to_watts(ftp) as f32;
                let progress = progress.clamp(0.0, 1.0);
                (start_watts + (end_watts - start_watts) * progress) as u16
            }
        }
    }
}

/// Cadence target specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadenceTarget {
    /// Minimum target cadence in RPM
    pub min_rpm: u8,
    /// Maximum target cadence in RPM
    pub max_rpm: u8,
}

/// Current status of workout execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkoutStatus {
    /// Workout not started
    #[default]
    NotStarted,
    /// Workout in progress
    InProgress,
    /// Workout paused
    Paused,
    /// Workout completed successfully
    Completed,
    /// Workout stopped early
    Stopped,
    /// Trainer disconnected - workout paused waiting for reconnection
    TrainerDisconnected,
}

/// Source format of imported workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkoutFormat {
    /// Zwift workout format (.zwo)
    Zwo,
    /// TrainerRoad/generic format (.mrc, .erg)
    Mrc,
    /// Garmin workout format (.fit)
    Fit,
    /// RustRide native format (JSON)
    Native,
}

/// A single segment within a workout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutSegment {
    /// Type of segment
    pub segment_type: SegmentType,
    /// Duration in seconds
    pub duration_seconds: u32,
    /// Power target specification
    pub power_target: PowerTarget,
    /// Optional cadence target
    pub cadence_target: Option<CadenceTarget>,
    /// Optional on-screen text message
    pub text_event: Option<String>,
}

/// A structured training workout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workout {
    /// Unique identifier
    pub id: Uuid,
    /// Workout name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Workout creator/author
    pub author: Option<String>,
    /// Original import file path
    pub source_file: Option<String>,
    /// Original file format
    pub source_format: Option<WorkoutFormat>,
    /// Ordered list of segments
    pub segments: Vec<WorkoutSegment>,
    /// Total workout duration in seconds (calculated)
    pub total_duration_seconds: u32,
    /// Estimated Training Stress Score
    pub estimated_tss: Option<f32>,
    /// Estimated Intensity Factor
    pub estimated_if: Option<f32>,
    /// User-defined tags
    pub tags: Vec<String>,
    /// Import/creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Workout {
    /// Create a new workout with the given name and segments.
    pub fn new(name: String, segments: Vec<WorkoutSegment>) -> Self {
        let total_duration_seconds = segments.iter().map(|s| s.duration_seconds).sum();

        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            author: None,
            source_file: None,
            source_format: None,
            segments,
            total_duration_seconds,
            estimated_tss: None,
            estimated_if: None,
            tags: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Calculate estimated TSS and IF for a given FTP.
    pub fn calculate_estimates(&mut self, ftp: u16) {
        if self.segments.is_empty() || ftp == 0 {
            return;
        }

        // Calculate weighted average intensity
        let mut total_weighted_power = 0.0f64;
        let mut total_duration = 0u32;

        for segment in &self.segments {
            let duration = segment.duration_seconds;
            let avg_power = match &segment.power_target {
                PowerTarget::Absolute { watts } => *watts as f64,
                PowerTarget::PercentFtp { percent } => ftp as f64 * *percent as f64 / 100.0,
                PowerTarget::Range { start, end } => {
                    let start_watts = start.to_watts(ftp) as f64;
                    let end_watts = end.to_watts(ftp) as f64;
                    (start_watts + end_watts) / 2.0
                }
            };

            total_weighted_power += avg_power * duration as f64;
            total_duration += duration;
        }

        if total_duration > 0 {
            let avg_power = total_weighted_power / total_duration as f64;
            let intensity_factor = (avg_power / ftp as f64) as f32;
            let duration_hours = total_duration as f32 / 3600.0;
            let tss = duration_hours * intensity_factor * intensity_factor * 100.0;

            self.estimated_if = Some(intensity_factor);
            self.estimated_tss = Some(tss);
        }
    }
}

/// Progress within a workout segment.
#[derive(Debug, Clone)]
pub struct SegmentProgress {
    /// Index of current segment
    pub segment_index: usize,
    /// Elapsed time in current segment (seconds)
    pub elapsed_seconds: u32,
    /// Remaining time in current segment (seconds)
    pub remaining_seconds: u32,
    /// Progress through segment (0.0 to 1.0)
    pub progress: f32,
    /// Current target power in watts
    pub target_power: u16,
}

/// Current state of workout execution.
#[derive(Debug, Clone)]
pub struct WorkoutState {
    /// The workout being executed
    pub workout: Workout,
    /// Current execution status
    pub status: WorkoutStatus,
    /// Total elapsed time in seconds
    pub total_elapsed_seconds: u32,
    /// Current segment progress
    pub segment_progress: Option<SegmentProgress>,
    /// Power offset (manual adjustment)
    pub power_offset: i16,
    /// User's FTP for power calculations
    pub user_ftp: u16,
}

/// Errors related to workout operations.
#[derive(Debug, Error)]
pub enum WorkoutError {
    /// Workout not found
    #[error("Workout not found: {0}")]
    NotFound(String),

    /// Workout file could not be read
    #[error("Failed to read workout file: {0}")]
    FileReadError(String),

    /// Workout parsing failed
    #[error("Failed to parse workout: {0}")]
    ParseError(#[from] WorkoutParseError),

    /// Invalid workout structure
    #[error("Invalid workout: {0}")]
    InvalidWorkout(String),

    /// Workout engine error
    #[error("Workout engine error: {0}")]
    EngineError(String),

    /// No workout loaded
    #[error("No workout loaded")]
    NoWorkoutLoaded,

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Errors during workout file parsing.
#[derive(Debug, Error)]
pub enum WorkoutParseError {
    /// Invalid XML structure
    #[error("Invalid XML: {0}")]
    InvalidXml(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value
    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    /// Unsupported workout format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Empty workout (no segments)
    #[error("Workout has no segments")]
    EmptyWorkout,

    /// IO error reading file
    #[error("IO error: {0}")]
    IoError(String),
}
