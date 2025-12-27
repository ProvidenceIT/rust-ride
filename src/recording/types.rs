//! Recording types for ride data capture and export.
//!
//! T014: Define RideSample, RecordingStatus structs
//! T023: Define RecorderError, ExportError enums
//! T084: Define Ride struct with summary fields
//! T085: Define RecorderConfig struct
//! T086: Define LiveRideSummary struct

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Status of the ride recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecordingStatus {
    /// Not recording
    #[default]
    Idle,
    /// Actively recording
    Recording,
    /// Recording paused
    Paused,
    /// Finishing up (saving data)
    Finishing,
}

/// A single data point during a ride (1-second resolution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RideSample {
    /// Seconds since ride start
    pub elapsed_seconds: u32,
    /// Instantaneous power in watts
    pub power_watts: Option<u16>,
    /// Cadence in RPM
    pub cadence_rpm: Option<u8>,
    /// Heart rate in BPM
    pub heart_rate_bpm: Option<u8>,
    /// Speed in km/h
    pub speed_kmh: Option<f32>,
    /// Cumulative distance in meters
    pub distance_meters: f64,
    /// Cumulative calories
    pub calories: u32,
    /// Trainer resistance level (0-100)
    pub resistance_level: Option<u8>,
    /// ERG target power
    pub target_power: Option<u16>,
    /// Simulated gradient percentage
    pub trainer_grade: Option<f32>,
    /// T049: Left/right power balance (left percentage)
    pub left_right_balance: Option<f32>,
    /// T049: Left pedal torque effectiveness percentage
    pub left_torque_effectiveness: Option<f32>,
    /// T049: Right pedal torque effectiveness percentage
    pub right_torque_effectiveness: Option<f32>,
    /// T049: Left pedal smoothness percentage
    pub left_pedal_smoothness: Option<f32>,
    /// T049: Right pedal smoothness percentage
    pub right_pedal_smoothness: Option<f32>,
    /// T130: Left power phase start angle (degrees)
    pub left_power_phase_start: Option<f32>,
    /// T130: Left power phase end angle (degrees)
    pub left_power_phase_end: Option<f32>,
    /// T130: Left power phase peak angle (degrees)
    pub left_power_phase_peak: Option<f32>,
    /// T130: Right power phase start angle (degrees)
    pub right_power_phase_start: Option<f32>,
    /// T130: Right power phase end angle (degrees)
    pub right_power_phase_end: Option<f32>,
    /// T130: Right power phase peak angle (degrees)
    pub right_power_phase_peak: Option<f32>,
}

impl RideSample {
    /// Create a new ride sample at the given elapsed time.
    pub fn new(elapsed_seconds: u32) -> Self {
        Self {
            elapsed_seconds,
            power_watts: None,
            cadence_rpm: None,
            heart_rate_bpm: None,
            speed_kmh: None,
            distance_meters: 0.0,
            calories: 0,
            resistance_level: None,
            target_power: None,
            trainer_grade: None,
            left_right_balance: None,
            left_torque_effectiveness: None,
            right_torque_effectiveness: None,
            left_pedal_smoothness: None,
            right_pedal_smoothness: None,
            left_power_phase_start: None,
            left_power_phase_end: None,
            left_power_phase_peak: None,
            right_power_phase_start: None,
            right_power_phase_end: None,
            right_power_phase_peak: None,
        }
    }

    /// Apply cycling dynamics data to this sample.
    pub fn with_dynamics(mut self, dynamics: &crate::sensors::CyclingDynamicsData) -> Self {
        self.left_right_balance = Some(dynamics.balance.left_percent);
        self.left_torque_effectiveness = Some(dynamics.torque_effectiveness.left_percent);
        self.right_torque_effectiveness = Some(dynamics.torque_effectiveness.right_percent);
        self.left_pedal_smoothness = Some(dynamics.smoothness.left_percent);
        self.right_pedal_smoothness = Some(dynamics.smoothness.right_percent);
        // T130: Include power phase data
        if let Some(ref phase) = dynamics.left_power_phase {
            self.left_power_phase_start = Some(phase.start_angle);
            self.left_power_phase_end = Some(phase.end_angle);
            self.left_power_phase_peak = phase.peak_angle;
        }
        if let Some(ref phase) = dynamics.right_power_phase {
            self.right_power_phase_start = Some(phase.start_angle);
            self.right_power_phase_end = Some(phase.end_angle);
            self.right_power_phase_peak = phase.peak_angle;
        }
        self
    }

    /// Check if this sample has cycling dynamics data.
    pub fn has_dynamics(&self) -> bool {
        self.left_right_balance.is_some()
    }
}

/// A completed or in-progress ride.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ride {
    /// Unique identifier
    pub id: Uuid,
    /// User who completed the ride
    pub user_id: Uuid,
    /// Associated workout (if structured workout)
    pub workout_id: Option<Uuid>,
    /// Ride start timestamp
    pub started_at: DateTime<Utc>,
    /// Ride end timestamp
    pub ended_at: Option<DateTime<Utc>>,
    /// Active riding time in seconds
    pub duration_seconds: u32,
    /// Total distance in meters
    pub distance_meters: f64,
    /// Average power in watts
    pub avg_power: Option<u16>,
    /// Maximum power in watts
    pub max_power: Option<u16>,
    /// Normalized Power
    pub normalized_power: Option<u16>,
    /// Intensity Factor (NP / FTP)
    pub intensity_factor: Option<f32>,
    /// Training Stress Score
    pub tss: Option<f32>,
    /// Average heart rate
    pub avg_hr: Option<u8>,
    /// Maximum heart rate
    pub max_hr: Option<u8>,
    /// Average cadence
    pub avg_cadence: Option<u8>,
    /// Estimated calories burned
    pub calories: u32,
    /// User's FTP at time of ride
    pub ftp_at_ride: u16,
    /// User notes
    pub notes: Option<String>,
    /// Record creation timestamp
    pub created_at: DateTime<Utc>,
    /// T049: Average left/right balance (left percentage)
    pub avg_left_balance: Option<f32>,
    /// T049: Average left torque effectiveness
    pub avg_left_torque_eff: Option<f32>,
    /// T049: Average right torque effectiveness
    pub avg_right_torque_eff: Option<f32>,
    /// T049: Average left pedal smoothness
    pub avg_left_smoothness: Option<f32>,
    /// T049: Average right pedal smoothness
    pub avg_right_smoothness: Option<f32>,
}

impl Ride {
    /// Create a new ride for a user.
    pub fn new(user_id: Uuid, ftp: u16) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            workout_id: None,
            started_at: now,
            ended_at: None,
            duration_seconds: 0,
            distance_meters: 0.0,
            avg_power: None,
            max_power: None,
            normalized_power: None,
            intensity_factor: None,
            tss: None,
            avg_hr: None,
            max_hr: None,
            avg_cadence: None,
            calories: 0,
            ftp_at_ride: ftp,
            notes: None,
            created_at: now,
            avg_left_balance: None,
            avg_left_torque_eff: None,
            avg_right_torque_eff: None,
            avg_left_smoothness: None,
            avg_right_smoothness: None,
        }
    }

    /// Apply dynamics averages to this ride.
    pub fn with_dynamics_averages(mut self, averages: &crate::sensors::DynamicsAverages) -> Self {
        if averages.sample_count > 0 {
            self.avg_left_balance = Some(averages.avg_left_balance);
            self.avg_left_torque_eff = Some(averages.avg_left_torque_eff);
            self.avg_right_torque_eff = Some(averages.avg_right_torque_eff);
            self.avg_left_smoothness = Some(averages.avg_left_smoothness);
            self.avg_right_smoothness = Some(averages.avg_right_smoothness);
        }
        self
    }
}

/// Configuration for the ride recorder.
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    /// Auto-save interval in seconds
    pub autosave_interval_secs: u32,
    /// Sample rate (samples per second)
    pub sample_rate_hz: u32,
    /// Maximum power value before filtering as noise
    pub max_power_filter: u16,
    /// Whether to record zero-power samples
    pub record_zeros: bool,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            autosave_interval_secs: 30,
            sample_rate_hz: 1,
            max_power_filter: 2000,
            record_zeros: true,
        }
    }
}

/// Live summary statistics during a ride.
#[derive(Debug, Clone, Default)]
pub struct LiveRideSummary {
    /// Total elapsed time in seconds
    pub elapsed_seconds: u32,
    /// Total distance in meters
    pub distance_meters: f64,
    /// Current power in watts
    pub current_power: Option<u16>,
    /// 3-second average power
    pub power_3s_avg: Option<u16>,
    /// 30-second average power (for NP)
    pub power_30s_avg: Option<u16>,
    /// Average power for entire ride
    pub avg_power: Option<u16>,
    /// Maximum power recorded
    pub max_power: Option<u16>,
    /// Running Normalized Power
    pub normalized_power: Option<u16>,
    /// Running TSS
    pub tss: Option<f32>,
    /// Running Intensity Factor
    pub intensity_factor: Option<f32>,
    /// Current heart rate
    pub current_hr: Option<u8>,
    /// Average heart rate
    pub avg_hr: Option<u8>,
    /// Maximum heart rate
    pub max_hr: Option<u8>,
    /// Current cadence
    pub current_cadence: Option<u8>,
    /// Average cadence
    pub avg_cadence: Option<u8>,
    /// Current speed in km/h
    pub current_speed: Option<f32>,
    /// Average speed in km/h
    pub avg_speed: Option<f32>,
    /// Estimated calories burned
    pub calories: u32,
    /// Current power zone (1-7)
    pub power_zone: Option<u8>,
    /// Current HR zone (1-5)
    pub hr_zone: Option<u8>,
    /// T049: Current left/right balance (left percentage)
    pub current_left_balance: Option<f32>,
    /// T049: Average left balance for the ride
    pub avg_left_balance: Option<f32>,
    /// T049: Current left pedal smoothness
    pub current_left_smoothness: Option<f32>,
    /// T049: Current right pedal smoothness
    pub current_right_smoothness: Option<f32>,
    /// T049: Current left torque effectiveness
    pub current_left_torque_eff: Option<f32>,
    /// T049: Current right torque effectiveness
    pub current_right_torque_eff: Option<f32>,
}

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// TCX format (XML, Strava/Garmin compatible)
    Tcx,
    /// FIT format (binary, Garmin native)
    Fit,
    /// CSV format (spreadsheet compatible)
    Csv,
}

/// T036: Export configuration with unit preference support.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,
    /// Unit preference for export metadata and human-readable values
    pub units: crate::storage::config::Units,
    /// Include cycling dynamics data if available
    pub include_dynamics: bool,
    /// Include power phase data if available
    pub include_power_phase: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Tcx,
            units: crate::storage::config::Units::Metric,
            include_dynamics: true,
            include_power_phase: true,
        }
    }
}

impl ExportConfig {
    /// Create a new export config with the specified format and units.
    pub fn new(format: ExportFormat, units: crate::storage::config::Units) -> Self {
        Self {
            format,
            units,
            include_dynamics: true,
            include_power_phase: true,
        }
    }

    /// Create a config for TCX export.
    pub fn tcx(units: crate::storage::config::Units) -> Self {
        Self::new(ExportFormat::Tcx, units)
    }

    /// Create a config for FIT export.
    pub fn fit(units: crate::storage::config::Units) -> Self {
        Self::new(ExportFormat::Fit, units)
    }

    /// Create a config for CSV export.
    pub fn csv(units: crate::storage::config::Units) -> Self {
        Self::new(ExportFormat::Csv, units)
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Tcx => write!(f, "TCX"),
            ExportFormat::Fit => write!(f, "FIT"),
            ExportFormat::Csv => write!(f, "CSV"),
        }
    }
}

/// Errors from the ride recorder.
#[derive(Debug, Error)]
pub enum RecorderError {
    /// Already recording
    #[error("Recording already in progress")]
    AlreadyRecording,

    /// Not currently recording
    #[error("Not currently recording")]
    NotRecording,

    /// Failed to save ride data
    #[error("Failed to save ride: {0}")]
    SaveFailed(String),

    /// Failed to load recovery data
    #[error("Failed to recover ride: {0}")]
    RecoveryFailed(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// No samples recorded
    #[error("No data recorded")]
    NoData,

    /// Sample buffer overflow
    #[error("Sample buffer overflow")]
    BufferOverflow,

    /// Storage is full or critically low
    #[error("Storage is full - cannot continue recording")]
    StorageFull,
}

/// Errors during ride export.
#[derive(Debug, Error)]
pub enum ExportError {
    /// Ride not found
    #[error("Ride not found: {0}")]
    RideNotFound(String),

    /// No samples to export
    #[error("Ride has no data to export")]
    NoData,

    /// Failed to create export file
    #[error("Failed to create file: {0}")]
    FileCreationFailed(String),

    /// Failed to write export data
    #[error("Failed to write data: {0}")]
    WriteFailed(String),

    /// Unsupported export format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// XML generation error
    #[error("XML error: {0}")]
    XmlError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
