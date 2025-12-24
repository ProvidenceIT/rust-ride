# Contract: Ride Recorder

**Module**: `src/recording/`
**Responsibility**: Real-time ride data capture, auto-save, and file export

## Public Interface

### RideRecorder

Manages active ride recording and persistence.

```rust
pub struct RideRecorder {
    // Internal state
}

impl RideRecorder {
    /// Create a new ride recorder
    pub fn new(config: RecorderConfig, db: Database) -> Self;

    /// Start a new ride recording
    pub fn start(&mut self, user: &UserProfile, workout: Option<&Workout>) -> Result<RideId, RecorderError>;

    /// Record a data sample
    /// Should be called approximately once per second
    pub fn record_sample(&mut self, reading: AggregatedMetrics) -> Result<(), RecorderError>;

    /// Pause recording (stops timer, continues accepting samples)
    pub fn pause(&mut self) -> Result<(), RecorderError>;

    /// Resume recording
    pub fn resume(&mut self) -> Result<(), RecorderError>;

    /// End the ride and save to database
    pub fn finish(&mut self) -> Result<Ride, RecorderError>;

    /// Discard the current ride without saving
    pub fn discard(&mut self) -> Result<(), RecorderError>;

    /// Get current ride state
    pub fn state(&self) -> &RecorderState;

    /// Get current ride summary (for display during ride)
    pub fn get_live_summary(&self) -> LiveRideSummary;

    /// Force an immediate auto-save
    pub fn force_save(&mut self) -> Result<(), RecorderError>;

    /// Check if there's a recoverable ride from crash
    pub fn has_recovery_data(&self) -> bool;

    /// Recover ride from auto-save after crash
    pub fn recover(&mut self) -> Result<RecoveryResult, RecorderError>;

    /// Clear recovery data
    pub fn clear_recovery(&mut self) -> Result<(), RecorderError>;
}
```

### Configuration

```rust
pub struct RecorderConfig {
    /// Auto-save interval (default: 30 seconds)
    pub autosave_interval: Duration,

    /// Maximum samples to buffer before forcing write
    pub max_buffer_size: usize,

    /// Whether to filter noise from power readings
    pub filter_noise: bool,

    /// Maximum valid power value (readings above are discarded)
    pub max_valid_power: u16,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            autosave_interval: Duration::from_secs(30),
            max_buffer_size: 1000,
            filter_noise: true,
            max_valid_power: 2000,
        }
    }
}
```

### State Types

```rust
pub struct RecorderState {
    /// Current status
    pub status: RecordingStatus,

    /// Ride ID (if recording)
    pub ride_id: Option<RideId>,

    /// Recording start time
    pub started_at: Option<DateTime<Utc>>,

    /// Total samples recorded
    pub sample_count: usize,

    /// Time since last auto-save
    pub since_last_save: Duration,
}

pub enum RecordingStatus {
    Idle,
    Recording,
    Paused,
}

pub struct LiveRideSummary {
    /// Elapsed time (excluding pauses)
    pub elapsed_time: Duration,

    /// Total distance
    pub distance_meters: f64,

    /// Current power (instantaneous)
    pub current_power: Option<u16>,

    /// Average power
    pub avg_power: Option<u16>,

    /// Normalized power
    pub normalized_power: Option<u16>,

    /// Current heart rate
    pub current_hr: Option<u8>,

    /// Average heart rate
    pub avg_hr: Option<u8>,

    /// Current cadence
    pub current_cadence: Option<u8>,

    /// Average cadence
    pub avg_cadence: Option<u8>,

    /// Current speed
    pub current_speed: Option<f32>,

    /// Calories burned
    pub calories: u32,

    /// TSS so far
    pub tss: Option<f32>,

    /// Intensity Factor so far
    pub intensity_factor: Option<f32>,
}

pub struct RecoveryResult {
    /// Recovered ride data
    pub ride: Ride,

    /// Number of samples recovered
    pub samples_recovered: usize,

    /// Time of last auto-save
    pub saved_at: DateTime<Utc>,
}
```

### Export Functions

```rust
/// Export a ride to FIT format
pub fn export_fit(ride: &Ride, samples: &[RideSample]) -> Result<Vec<u8>, ExportError>;

/// Export a ride to TCX format
pub fn export_tcx(ride: &Ride, samples: &[RideSample]) -> Result<String, ExportError>;

/// Export a ride to GPX format (with power/HR extensions)
pub fn export_gpx(ride: &Ride, samples: &[RideSample]) -> Result<String, ExportError>;

/// Export a ride to CSV format
pub fn export_csv(ride: &Ride, samples: &[RideSample]) -> Result<String, ExportError>;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("No samples to export")]
    NoSamples,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("No ride in progress")]
    NoRideInProgress,

    #[error("Ride already in progress")]
    RideAlreadyInProgress,

    #[error("Ride is paused")]
    RidePaused,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Auto-save failed: {0}")]
    AutoSaveFailed(String),

    #[error("No recovery data available")]
    NoRecoveryData,

    #[error("Recovery data corrupted")]
    RecoveryCorrupted,
}

pub type RideId = uuid::Uuid;
```

## Auto-Save Behavior

1. **Trigger conditions**:
   - Every `autosave_interval` (default 30 seconds)
   - When buffer reaches `max_buffer_size`
   - On explicit `force_save()` call

2. **What's saved**:
   - Current ride metadata (as JSON)
   - All samples since last save (appended to autosave table)

3. **Recovery process**:
   - On startup, check `autosave` table for data
   - If found, prompt user to recover or discard
   - Recovery merges autosave data into a new ride record

## Sample Processing

```rust
fn process_sample(&mut self, metrics: AggregatedMetrics) -> Option<RideSample> {
    // 1. Noise filtering
    let power = metrics.power_instant
        .filter(|&p| p <= self.config.max_valid_power);

    // 2. Distance calculation (from speed if not provided)
    let distance_delta = metrics.speed
        .map(|s| s / 3.6) // km/h to m/s
        .unwrap_or(0.0);

    // 3. Calorie calculation (power-based)
    let calorie_delta = power
        .map(|p| (p as f64 * 1.0 / 4184.0)) // Joules to kcal
        .unwrap_or(0.0);

    Some(RideSample {
        elapsed_seconds: self.elapsed_seconds(),
        power_watts: power,
        cadence_rpm: metrics.cadence,
        heart_rate_bpm: metrics.heart_rate,
        speed_kmh: metrics.speed,
        distance_meters: self.total_distance + distance_delta,
        calories: self.total_calories + calorie_delta as u32,
        resistance_level: metrics.resistance_level,
        target_power: metrics.target_power,
        trainer_grade: metrics.trainer_grade,
    })
}
```

## Summary Calculations

Performed when `finish()` is called:

```rust
fn calculate_summary(&self, samples: &[RideSample]) -> RideSummary {
    // Average power (simple mean of non-zero values)
    let avg_power = samples.iter()
        .filter_map(|s| s.power_watts)
        .map(|p| p as f64)
        .sum::<f64>() / sample_count;

    // Normalized Power (30s rolling average, 4th power)
    let normalized_power = calculate_np(samples);

    // Intensity Factor
    let intensity_factor = normalized_power / self.user_ftp as f64;

    // TSS = (duration_hours × IF² × 100)
    let duration_hours = self.elapsed_seconds as f64 / 3600.0;
    let tss = duration_hours * intensity_factor.powi(2) * 100.0;

    RideSummary {
        duration_seconds: self.elapsed_seconds,
        distance_meters: samples.last().map(|s| s.distance_meters).unwrap_or(0.0),
        avg_power: Some(avg_power as u16),
        max_power: samples.iter().filter_map(|s| s.power_watts).max(),
        normalized_power: Some(normalized_power as u16),
        intensity_factor: Some(intensity_factor as f32),
        tss: Some(tss as f32),
        avg_hr: calculate_avg(samples.iter().filter_map(|s| s.heart_rate_bpm)),
        max_hr: samples.iter().filter_map(|s| s.heart_rate_bpm).max(),
        avg_cadence: calculate_avg(samples.iter().filter_map(|s| s.cadence_rpm)),
        calories: samples.last().map(|s| s.calories).unwrap_or(0),
    }
}
```

## Threading Model

```
┌──────────────────────┐
│    UI Thread         │
│  (calls record_sample)│
└──────────┬───────────┘
           │
           │ samples
           ▼
┌──────────────────────┐
│   Sample Buffer      │
│   (Vec<RideSample>)  │
└──────────┬───────────┘
           │
           │ on timer / buffer full
           ▼
┌──────────────────────┐
│  Background Writer   │
│  (tokio task)        │
│  - SQLite inserts    │
│  - Autosave updates  │
└──────────────────────┘
```

## TCX Export Format

```xml
<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities>
    <Activity Sport="Biking">
      <Id>2024-12-24T10:00:00Z</Id>
      <Lap StartTime="2024-12-24T10:00:00Z">
        <TotalTimeSeconds>3600.0</TotalTimeSeconds>
        <DistanceMeters>30000.0</DistanceMeters>
        <MaximumSpeed>12.5</MaximumSpeed>
        <Calories>500</Calories>
        <AverageHeartRateBpm><Value>150</Value></AverageHeartRateBpm>
        <MaximumHeartRateBpm><Value>175</Value></MaximumHeartRateBpm>
        <Intensity>Active</Intensity>
        <Cadence>90</Cadence>
        <TriggerMethod>Manual</TriggerMethod>
        <Track>
          <Trackpoint>
            <Time>2024-12-24T10:00:00Z</Time>
            <DistanceMeters>0.0</DistanceMeters>
            <HeartRateBpm><Value>145</Value></HeartRateBpm>
            <Cadence>90</Cadence>
            <Extensions>
              <TPX xmlns="http://www.garmin.com/xmlschemas/ActivityExtension/v2">
                <Watts>250</Watts>
                <Speed>8.3</Speed>
              </TPX>
            </Extensions>
          </Trackpoint>
          <!-- More trackpoints... -->
        </Track>
      </Lap>
      <Creator xsi:type="Device_t">
        <Name>RustRide</Name>
        <Version>
          <VersionMajor>1</VersionMajor>
          <VersionMinor>0</VersionMinor>
        </Version>
      </Creator>
    </Activity>
  </Activities>
</TrainingCenterDatabase>
```
