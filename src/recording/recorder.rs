//! Ride recorder for capturing sensor data.
//!
//! T087: Implement RideRecorder struct
//! Placeholder for Phase 5 implementation

use crate::recording::types::{
    LiveRideSummary, RecorderConfig, RecorderError, RecordingStatus, Ride, RideSample,
};
use uuid::Uuid;

/// Records ride data from sensors.
pub struct RideRecorder {
    /// Configuration
    config: RecorderConfig,
    /// Current recording status
    status: RecordingStatus,
    /// Current ride being recorded
    current_ride: Option<Ride>,
    /// Recorded samples
    samples: Vec<RideSample>,
    /// Live summary statistics
    live_summary: LiveRideSummary,
}

impl RideRecorder {
    /// Create a new ride recorder.
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            status: RecordingStatus::Idle,
            current_ride: None,
            samples: Vec::new(),
            live_summary: LiveRideSummary::default(),
        }
    }

    /// Create a new ride recorder with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RecorderConfig::default())
    }

    /// Start recording a new ride.
    pub fn start(&mut self, user_id: Uuid, ftp: u16) -> Result<(), RecorderError> {
        if self.status == RecordingStatus::Recording {
            return Err(RecorderError::AlreadyRecording);
        }

        self.current_ride = Some(Ride::new(user_id, ftp));
        self.samples.clear();
        self.live_summary = LiveRideSummary::default();
        self.status = RecordingStatus::Recording;

        tracing::info!("Started recording ride");
        Ok(())
    }

    /// Record a sample.
    pub fn record_sample(&mut self, sample: RideSample) -> Result<(), RecorderError> {
        if self.status != RecordingStatus::Recording {
            return Err(RecorderError::NotRecording);
        }

        // Filter power spikes
        let sample = if let Some(power) = sample.power_watts {
            if power > self.config.max_power_filter {
                RideSample {
                    power_watts: None,
                    ..sample
                }
            } else {
                sample
            }
        } else {
            sample
        };

        self.samples.push(sample);
        self.update_live_summary();

        Ok(())
    }

    /// Pause recording.
    pub fn pause(&mut self) -> Result<(), RecorderError> {
        if self.status != RecordingStatus::Recording {
            return Err(RecorderError::NotRecording);
        }

        self.status = RecordingStatus::Paused;
        tracing::info!("Paused recording");
        Ok(())
    }

    /// Resume recording.
    pub fn resume(&mut self) -> Result<(), RecorderError> {
        if self.status != RecordingStatus::Paused {
            return Err(RecorderError::NotRecording);
        }

        self.status = RecordingStatus::Recording;
        tracing::info!("Resumed recording");
        Ok(())
    }

    /// Finish recording and return the ride with samples.
    pub fn finish(&mut self) -> Result<(Ride, Vec<RideSample>), RecorderError> {
        if self.status == RecordingStatus::Idle {
            return Err(RecorderError::NotRecording);
        }

        self.status = RecordingStatus::Finishing;

        let mut ride = self.current_ride.take().ok_or(RecorderError::NoData)?;

        if self.samples.is_empty() {
            return Err(RecorderError::NoData);
        }

        // Calculate summary statistics
        ride.duration_seconds = self.live_summary.elapsed_seconds;
        ride.distance_meters = self.live_summary.distance_meters;
        ride.avg_power = self.live_summary.avg_power;
        ride.max_power = self.live_summary.max_power;
        ride.normalized_power = self.live_summary.normalized_power;
        ride.avg_hr = self.live_summary.avg_hr;
        ride.max_hr = self.live_summary.max_hr;
        ride.avg_cadence = self.live_summary.avg_cadence;
        ride.calories = self.live_summary.calories;
        ride.ended_at = Some(chrono::Utc::now());

        // Calculate IF and TSS if we have NP
        if let Some(np) = ride.normalized_power {
            let intensity_factor = np as f32 / ride.ftp_at_ride as f32;
            ride.intensity_factor = Some(intensity_factor);

            let duration_hours = ride.duration_seconds as f32 / 3600.0;
            let tss = duration_hours * intensity_factor * intensity_factor * 100.0;
            ride.tss = Some(tss);
        }

        let samples = std::mem::take(&mut self.samples);
        self.status = RecordingStatus::Idle;
        self.live_summary = LiveRideSummary::default();

        tracing::info!("Finished recording ride with {} samples", samples.len());
        Ok((ride, samples))
    }

    /// Discard the current recording.
    pub fn discard(&mut self) {
        self.current_ride = None;
        self.samples.clear();
        self.live_summary = LiveRideSummary::default();
        self.status = RecordingStatus::Idle;
        tracing::info!("Discarded recording");
    }

    /// Get the current recording status.
    pub fn status(&self) -> RecordingStatus {
        self.status
    }

    /// Get the live summary statistics.
    pub fn get_live_summary(&self) -> &LiveRideSummary {
        &self.live_summary
    }

    /// Check if there's recovery data available.
    pub fn has_recovery_data(&self) -> bool {
        // TODO: Check autosave table in Phase 5 (T089)
        false
    }

    /// Update live summary from samples.
    fn update_live_summary(&mut self) {
        // TODO: Full implementation in Phase 5 (T090)
        if let Some(sample) = self.samples.last() {
            self.live_summary.elapsed_seconds = sample.elapsed_seconds;
            self.live_summary.distance_meters = sample.distance_meters;
            self.live_summary.current_power = sample.power_watts;
            self.live_summary.current_hr = sample.heart_rate_bpm;
            self.live_summary.current_cadence = sample.cadence_rpm;
            self.live_summary.current_speed = sample.speed_kmh;
            self.live_summary.calories = sample.calories;
        }
    }
}
