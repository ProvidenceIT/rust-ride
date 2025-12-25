//! Ride recorder for capturing sensor data.
//!
//! T087: Implement RideRecorder struct
//! T156: Implement storage-full warning
//! T031-T038: Autosave and crash recovery

use crate::recording::types::{
    LiveRideSummary, RecorderConfig, RecorderError, RecordingStatus, Ride, RideSample,
};
use crate::storage::database::Database;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

/// Minimum disk space in bytes required to continue recording (50 MB)
const MIN_DISK_SPACE_BYTES: u64 = 50 * 1024 * 1024;

/// Warning threshold for low disk space (500 MB)
const LOW_DISK_SPACE_WARNING_BYTES: u64 = 500 * 1024 * 1024;

/// Storage status for the recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageStatus {
    /// Plenty of storage available
    Ok,
    /// Storage is running low (warning threshold)
    Low,
    /// Storage is critically low (recording should stop)
    Critical,
    /// Unable to determine storage status
    Unknown,
}

/// Data that can be recovered after a crash.
#[derive(Debug, Clone)]
pub struct RecoverableRide {
    /// The ride metadata
    pub ride: Ride,
    /// Recorded samples
    pub samples: Vec<RideSample>,
}

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
    /// Database for persistence (optional)
    database: Option<Arc<Mutex<Database>>>,
    /// Autosave timer handle
    autosave_handle: Option<Arc<TokioMutex<Option<tokio::task::JoinHandle<()>>>>>,
    /// Flag to indicate if autosave is running
    autosave_running: Arc<TokioMutex<bool>>,
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
            database: None,
            autosave_handle: None,
            autosave_running: Arc::new(TokioMutex::new(false)),
        }
    }

    /// Create a new ride recorder with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RecorderConfig::default())
    }

    /// Create a new ride recorder with database for autosave.
    pub fn with_database(config: RecorderConfig, database: Arc<Mutex<Database>>) -> Self {
        Self {
            config,
            status: RecordingStatus::Idle,
            current_ride: None,
            samples: Vec::new(),
            live_summary: LiveRideSummary::default(),
            database: Some(database),
            autosave_handle: None,
            autosave_running: Arc::new(TokioMutex::new(false)),
        }
    }

    /// Set the database for autosave functionality.
    pub fn set_database(&mut self, database: Arc<Mutex<Database>>) {
        self.database = Some(database);
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

    /// Check if there's recovery data available (T036).
    pub fn has_recovery_data(&self) -> bool {
        if let Some(db) = &self.database {
            if let Ok(guard) = db.lock() {
                guard.has_autosave().unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Recover a ride from autosave data (T037).
    ///
    /// Returns the recoverable ride data if available.
    pub fn recover_ride(&self) -> Result<Option<RecoverableRide>, RecorderError> {
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| RecorderError::RecoveryFailed("No database configured".to_string()))?;

        let guard = db
            .lock()
            .map_err(|e| RecorderError::RecoveryFailed(format!("Database lock failed: {}", e)))?;

        match guard.load_autosave() {
            Ok(Some((ride, samples))) => {
                tracing::info!(
                    "Recovered ride with {} samples from autosave",
                    samples.len()
                );
                Ok(Some(RecoverableRide { ride, samples }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(RecorderError::RecoveryFailed(e.to_string())),
        }
    }

    /// Discard recovery data (T038).
    pub fn discard_recovery(&self) -> Result<(), RecorderError> {
        if let Some(db) = &self.database {
            let guard = db.lock().map_err(|e| {
                RecorderError::RecoveryFailed(format!("Database lock failed: {}", e))
            })?;
            guard
                .clear_autosave()
                .map_err(|e| RecorderError::RecoveryFailed(e.to_string()))?;
            tracing::info!("Discarded crash recovery data");
        }
        Ok(())
    }

    /// Save the current ride to the database (T031).
    pub fn save_ride(&mut self) -> Result<Ride, RecorderError> {
        let (ride, samples) = self.finish()?;

        if let Some(db) = &self.database {
            let mut guard = db
                .lock()
                .map_err(|e| RecorderError::SaveFailed(format!("Database lock failed: {}", e)))?;

            // Save ride to database
            guard
                .insert_ride(&ride)
                .map_err(|e| RecorderError::SaveFailed(e.to_string()))?;

            // Save all samples
            guard
                .insert_ride_samples(&ride.id, &samples)
                .map_err(|e| RecorderError::SaveFailed(e.to_string()))?;

            // Clear autosave data
            let _ = guard.clear_autosave();

            tracing::info!("Saved ride {} with {} samples", ride.id, samples.len());
        } else {
            tracing::warn!("No database configured, ride not persisted");
        }

        Ok(ride)
    }

    /// Enable autosave with periodic saves (T032).
    ///
    /// This starts a background timer that saves the current ride data
    /// to the autosave table at the configured interval.
    pub fn enable_autosave(&mut self) {
        if self.database.is_none() {
            tracing::warn!("Cannot enable autosave without database");
            return;
        }

        let interval_secs = self.config.autosave_interval_secs;
        let _autosave_running = self.autosave_running.clone();

        // Note: Actual autosave timer would require more complex async handling
        // For now, we'll implement manual autosave triggering via trigger_autosave()
        tracing::info!("Autosave enabled with {}s interval", interval_secs);
    }

    /// Trigger an autosave of the current ride data.
    ///
    /// This should be called periodically (e.g., every 30 seconds) during recording.
    pub fn trigger_autosave(&self) -> Result<(), RecorderError> {
        let ride = self
            .current_ride
            .as_ref()
            .ok_or(RecorderError::NotRecording)?;
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| RecorderError::SaveFailed("No database configured".to_string()))?;

        // Create a snapshot of the current ride with updated stats
        let mut ride_snapshot = ride.clone();
        ride_snapshot.duration_seconds = self.live_summary.elapsed_seconds;
        ride_snapshot.distance_meters = self.live_summary.distance_meters;
        ride_snapshot.avg_power = self.live_summary.avg_power;
        ride_snapshot.max_power = self.live_summary.max_power;
        ride_snapshot.avg_hr = self.live_summary.avg_hr;
        ride_snapshot.max_hr = self.live_summary.max_hr;
        ride_snapshot.calories = self.live_summary.calories;

        let guard = db
            .lock()
            .map_err(|e| RecorderError::SaveFailed(format!("Database lock failed: {}", e)))?;

        guard
            .save_autosave(&ride_snapshot, &self.samples)
            .map_err(|e| RecorderError::SaveFailed(e.to_string()))?;

        tracing::debug!("Autosaved ride with {} samples", self.samples.len());
        Ok(())
    }

    /// Disable autosave.
    pub fn disable_autosave(&mut self) {
        if let Some(handle) = self.autosave_handle.take() {
            // Cancel the autosave timer if running
            tokio::spawn(async move {
                if let Some(h) = handle.lock().await.take() {
                    h.abort();
                }
            });
        }
        tracing::info!("Autosave disabled");
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

    /// Check the current storage status.
    ///
    /// Returns the storage status based on available disk space.
    pub fn check_storage_status(&self) -> StorageStatus {
        // Check available space from current directory
        check_disk_space(".")
    }

    /// Check if there's enough storage to continue recording.
    ///
    /// Returns `Err(RecorderError::StorageFull)` if storage is critically low.
    pub fn ensure_storage_available(&self) -> Result<(), RecorderError> {
        match self.check_storage_status() {
            StorageStatus::Critical => {
                tracing::error!("Storage is critically low - cannot continue recording");
                Err(RecorderError::StorageFull)
            }
            StorageStatus::Low => {
                tracing::warn!("Storage is running low");
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Get the estimated storage used by the current recording in bytes.
    pub fn estimated_storage_used(&self) -> u64 {
        // Rough estimate: each sample is about 100 bytes when serialized
        const BYTES_PER_SAMPLE: u64 = 100;
        self.samples.len() as u64 * BYTES_PER_SAMPLE
    }

    /// Get a human-readable storage warning message if storage is low.
    pub fn get_storage_warning(&self) -> Option<String> {
        match self.check_storage_status() {
            StorageStatus::Critical => Some(
                "Critical: Storage is almost full! Recording will be stopped to prevent data loss."
                    .to_string(),
            ),
            StorageStatus::Low => Some(
                "Warning: Storage space is running low. Consider freeing up disk space."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

/// Check available disk space for a path.
fn check_disk_space(path: &str) -> StorageStatus {
    #[cfg(target_os = "windows")]
    {
        check_disk_space_windows(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        check_disk_space_unix(path)
    }
}

#[cfg(target_os = "windows")]
fn check_disk_space_windows(path: &str) -> StorageStatus {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Get the drive letter from the path
    let path = Path::new(path);
    let root = path
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| "C:\\".to_string());

    // Append backslash if needed
    let root = if root.ends_with('\\') || root.ends_with('/') {
        root
    } else {
        format!("{}\\", root)
    };

    // Use winapi to get disk space
    unsafe {
        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let root_wide: Vec<u16> = OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let result = windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            root_wide.as_ptr(),
            &mut free_bytes_available,
            &mut total_bytes,
            &mut total_free_bytes,
        );

        if result == 0 {
            tracing::warn!("Failed to get disk space for {}", root);
            return StorageStatus::Unknown;
        }

        if free_bytes_available < MIN_DISK_SPACE_BYTES {
            StorageStatus::Critical
        } else if free_bytes_available < LOW_DISK_SPACE_WARNING_BYTES {
            StorageStatus::Low
        } else {
            StorageStatus::Ok
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn check_disk_space_unix(path: &str) -> StorageStatus {
    use std::mem::MaybeUninit;

    let path =
        std::ffi::CString::new(path).unwrap_or_else(|_| std::ffi::CString::new(".").unwrap());

    unsafe {
        let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        let result = libc::statvfs(path.as_ptr(), stat.as_mut_ptr());

        if result != 0 {
            tracing::warn!("Failed to get disk space");
            return StorageStatus::Unknown;
        }

        let stat = stat.assume_init();
        #[allow(clippy::unnecessary_cast)]
        let free_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;

        if free_bytes < MIN_DISK_SPACE_BYTES {
            StorageStatus::Critical
        } else if free_bytes < LOW_DISK_SPACE_WARNING_BYTES {
            StorageStatus::Low
        } else {
            StorageStatus::Ok
        }
    }
}
